use anyhow::{anyhow, Context, Result};
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::models::{
    AlphaDetails, AlphaSettings, DailyPnlResponse, InSampleStats, PortfolioAlpha, SimulationPayload,
    SimulationResponse,
};

pub const BASE_URL: &str = "https://api.worldquantbrain.com";

#[derive(Clone)]
pub struct BrainClient {
    client: reqwest::Client,
    email: String,
    password: String,
    authenticated: Arc<AtomicBool>,
}

impl BrainClient {
    pub fn new(email: impl Into<String>, password: impl Into<String>) -> Result<Self> {
        let email = email.into();
        let password = password.into();

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .gzip(true)
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to build reqwest HTTP client")?;

        Ok(Self {
            client,
            email,
            password,
            authenticated: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    fn auth_header(&self) -> String {
        let creds = format!("{}:{}", self.email, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
        format!("Basic {}", encoded)
    }

    pub async fn authenticate(&self) -> Result<()> {
        if self.authenticated.load(Ordering::SeqCst) {
            return Ok(());
        }

        let url = format!("{}/authentication", BASE_URL);
        let auth = self.auth_header();

        for attempt in 0..4 {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

            let resp = self.client.post(&url).headers(headers).send().await;

            match resp {
                Ok(res) => {
                    let status = res.status();
                    if status.is_success() {
                        self.authenticated.store(true, Ordering::SeqCst);
                        return Ok(());
                    }

                    if status.as_u16() == 429 && attempt < 3 {
                        let wait = res
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(2.0f64.powi(attempt as i32) + 1.5);
                        println!(
                            "[*] [Auth 429] {} Sleeping {:.1}s...",
                            self.email, wait
                        );
                        sleep(Duration::from_secs_f64(wait)).await;
                        continue;
                    }

                    let err_text = res.text().await.unwrap_or_default();
                    return Err(anyhow!(
                        "Auth failed for {} (HTTP {}): {}",
                        self.email,
                        status,
                        err_text
                    ));
                }
                Err(e) => {
                    if attempt < 3 {
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(anyhow!("Connection error during auth for {}: {}", self.email, e));
                }
            }
        }

        Err(anyhow!("Exceeded max auth retries for {}", self.email))
    }

    pub async fn submit_simulation(
        &self,
        expression: &str,
        settings: &AlphaSettings,
    ) -> Result<String> {
        self.authenticate().await?;

        let url = format!("{}/simulations", BASE_URL);
        let payload = SimulationPayload {
            sim_type: "REGULAR",
            settings,
            regular: expression,
        };

        let auth = self.auth_header();

        for retry in 0..8 {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

            let res = self
                .client
                .post(&url)
                .headers(headers)
                .json(&payload)
                .send()
                .await;

            match res {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        if let Some(loc) = resp.headers().get("Location") {
                            return Ok(loc.to_str()?.to_string());
                        }
                        let body: serde_json::Value = resp.json().await?;
                        if let Some(id) = body.get("id").and_then(|v| v.as_str()) {
                            return Ok(id.to_string());
                        }
                        if let Some(loc) = body.get("location").and_then(|v| v.as_str()) {
                            return Ok(loc.to_string());
                        }
                        return Err(anyhow!("Simulation succeeded but no ID/Location returned: {:?}", body));
                    } else if status.as_u16() == 429 {
                        let wait = resp
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(2.0f64.powi(retry.min(5) as i32) + 1.2);
                        sleep(Duration::from_secs_f64(wait)).await;
                        continue;
                    } else {
                        let err_body = resp.text().await.unwrap_or_default();
                        return Err(anyhow!("Simulation post failed (HTTP {}): {}", status, err_body));
                    }
                }
                Err(e) => {
                    if retry < 7 {
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(anyhow!("Network error during simulation post: {}", e));
                }
            }
        }

        Err(anyhow!("Exceeded max simulation post retries"))
    }

    pub async fn poll_simulation(
        &self,
        sim_url_or_id: &str,
        timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> Result<SimulationResponse> {
        let url = if sim_url_or_id.starts_with("http") {
            sim_url_or_id.to_string()
        } else {
            format!("{}/simulations/{}", BASE_URL, sim_url_or_id)
        };

        let auth = self.auth_header();
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

            let res = self.client.get(&url).headers(headers).send().await;

            match res {
                Ok(resp) => {
                    if resp.status().as_u16() == 429 {
                        sleep(Duration::from_secs(8)).await;
                        continue;
                    }

                    if let Ok(sim_resp) = resp.json::<SimulationResponse>().await {
                        if let Some(ref st) = sim_resp.status {
                            if st == "COMPLETE" || (st == "WARNING" && sim_resp.alpha.is_some()) {
                                return Ok(sim_resp);
                            }
                            if st == "ERROR" {
                                let msg = sim_resp
                                    .message
                                    .clone()
                                    .or(sim_resp.error.clone())
                                    .unwrap_or_else(|| "Simulation failed".to_string());
                                return Err(anyhow!("Simulation ERROR: {}", msg));
                            }
                        }
                    }
                }
                Err(_) => {
                    // transient connection hiccups
                }
            }

            sleep(Duration::from_secs(poll_interval_secs)).await;
        }

        Err(anyhow!("Simulation timed out after {}s", timeout_secs))
    }

    pub async fn get_alpha_details(&self, alpha_id: &str) -> Result<AlphaDetails> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}", BASE_URL, alpha_id);
        let auth = self.auth_header();

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let res = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?
            .error_for_status()?;

        let details = res.json::<AlphaDetails>().await?;
        Ok(details)
    }

    pub async fn check_submission(
        &self,
        alpha_id: &str,
        max_attempts: usize,
    ) -> Result<(bool, String, InSampleStats)> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}/check", BASE_URL, alpha_id);
        let auth = self.auth_header();

        for _attempt in 0..max_attempts {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

            let resp = self.client.get(&url).headers(headers).send().await?;

            if resp.status().as_u16() == 429 {
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let retry_after = resp
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok());

            let text = resp.text().await?;
            if text.trim().is_empty() {
                let wait = retry_after.unwrap_or(2.0);
                sleep(Duration::from_secs_f64(wait)).await;
                continue;
            }

            let val: serde_json::Value = serde_json::from_str(&text)?;
            let is_data = val.get("is").cloned().unwrap_or(val.clone());
            let stats: InSampleStats = serde_json::from_value(is_data)?;

            let mut failed = Vec::new();
            for c in &stats.checks {
                if c.result == "FAIL" {
                    if c.name == "SELF_CORRELATION" {
                        let max_corr = stats
                            .self_correlated
                            .as_ref()
                            .and_then(|sc| sc.max)
                            .unwrap_or(0.70);
                        let partner = stats
                            .self_correlated
                            .as_ref()
                            .and_then(|sc| sc.records.as_ref())
                            .and_then(|r| r.first())
                            .and_then(|first| first.first())
                            .and_then(|v| v.as_str())
                            .unwrap_or("active OS alpha");
                        failed.push(format!(
                            "SELF_CORRELATION: {:.1}% (>70%) vs {}",
                            max_corr * 100.0,
                            partner
                        ));
                    } else {
                        failed.push(c.name.clone());
                    }
                }
            }

            if failed.is_empty() {
                return Ok((
                    true,
                    "ALL SUBMISSION CHECKS PASSED (8/8 PASS)".to_string(),
                    stats,
                ));
            } else {
                return Ok((
                    false,
                    format!("FAILED CHECKS: {}", failed.join("; ")),
                    stats,
                ));
            }
        }

        Err(anyhow!("Submission check timed out waiting for correlation calculation"))
    }

    pub async fn fetch_daily_pnl(&self, alpha_id: &str) -> Result<HashMap<String, f64>> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}/recordsets/daily-pnl", BASE_URL, alpha_id);
        let auth = self.auth_header();

        for attempt in 0..5 {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

            let resp = self.client.get(&url).headers(headers).send().await;

            match resp {
                Ok(r) => {
                    if r.status().as_u16() == 429 {
                        sleep(Duration::from_secs(3 * (attempt + 1))).await;
                        continue;
                    }
                    if r.status().is_success() {
                        let text = r.text().await.unwrap_or_default();
                        if text.trim().is_empty() {
                            sleep(Duration::from_secs(2)).await;
                            continue;
                        }
                        if let Ok(data) = serde_json::from_str::<DailyPnlResponse>(&text) {
                            let mut map = HashMap::new();
                            for row in data.records {
                                if row.len() >= 2 {
                                    if let (Some(d), Some(val)) = (row[0].as_str(), row[1].as_f64()) {
                                        map.insert(d.to_string(), val);
                                    }
                                }
                            }
                            if !map.is_empty() {
                                return Ok(map);
                            }
                        }
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                }
                Err(_) => {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        Err(anyhow!("Could not fetch daily-pnl for {}", alpha_id))
    }

    pub async fn submit_alpha(&self, alpha_id: &str) -> Result<(bool, String)> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}/submit", BASE_URL, alpha_id);
        let auth = self.auth_header();

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .body("{}")
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if status.is_success() {
            Ok((true, "Submitted successfully (201 Created)".to_string()))
        } else {
            Ok((false, format!("HTTP {}: {}", status, body)))
        }
    }

    pub async fn update_metadata(
        &self,
        alpha_id: &str,
        name: Option<&str>,
        category: Option<&str>,
        color: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<bool> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}", BASE_URL, alpha_id);
        let auth = self.auth_header();

        let mut payload = serde_json::Map::new();
        if let Some(n) = name {
            payload.insert("name".to_string(), serde_json::json!(n));
        }
        if let Some(c) = category {
            payload.insert("category".to_string(), serde_json::json!(c));
        }
        if let Some(col) = color {
            payload.insert("color".to_string(), serde_json::json!(col));
        }
        if let Some(t) = tags {
            payload.insert("tags".to_string(), serde_json::json!(t));
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let resp = self
            .client
            .patch(&url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        Ok(resp.status().is_success())
    }

    pub async fn fetch_user_alphas(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        self.authenticate().await?;

        let url = format!("{}/users/self/alphas?{}", BASE_URL, query);
        let auth = self.auth_header();

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let resp = self.client.get(&url).headers(headers).send().await?.error_for_status()?;
        let data: serde_json::Value = resp.json().await?;

        if let Some(arr) = data.as_array() {
            Ok(arr.clone())
        } else if let Some(arr) = data.get("results").and_then(|r| r.as_array()) {
            Ok(arr.clone())
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn fetch_active_portfolio(&self) -> Result<Vec<PortfolioAlpha>> {
        let queries = [
            "stage=OS&limit=100",
            "status=ACTIVE&limit=100",
            "status=SUBMITTED&limit=100",
            "status=UNPROCESSED&limit=100",
        ];

        let mut seen = std::collections::HashSet::new();
        let mut portfolio = Vec::new();

        for q in queries {
            if let Ok(alphas) = self.fetch_user_alphas(q).await {
                for item in alphas {
                    if let Some(aid) = item.get("id").and_then(|v| v.as_str()) {
                        let is_os = item.get("stage").and_then(|s| s.as_str()) == Some("OS")
                            || matches!(item.get("status").and_then(|s| s.as_str()), Some("ACTIVE") | Some("SUBMITTED") | Some("UNPROCESSED"))
                            || item.get("dateSubmitted").is_some();

                        if is_os && seen.insert(aid.to_string()) {
                            let st = item.get("is");
                            let sett = item.get("settings");
                            let code = item.get("regular")
                                .and_then(|r| r.get("code"))
                                .and_then(|c| c.as_str())
                                .map(|s| s.trim().to_string())
                                .or_else(|| item.get("code").and_then(|c| c.as_str()).map(|s| s.to_string()));

                            portfolio.push(PortfolioAlpha {
                                id: aid.to_string(),
                                name: item.get("name").and_then(|v| v.as_str()).map(String::from),
                                status: item.get("status").and_then(|v| v.as_str()).map(String::from),
                                stage: item.get("stage").and_then(|v| v.as_str()).map(String::from),
                                date_submitted: item.get("dateSubmitted").and_then(|v| v.as_str()).map(String::from),
                                universe: sett.and_then(|s| s.get("universe")).and_then(|v| v.as_str()).map(String::from),
                                decay: sett.and_then(|s| s.get("decay")).and_then(|v| v.as_i64()).map(|d| d as i32),
                                category: item.get("category").and_then(|v| v.as_str()).map(String::from),
                                color: item.get("color").and_then(|v| v.as_str()).map(String::from),
                                sharpe: st.and_then(|s| s.get("sharpe")).and_then(|v| v.as_f64()),
                                fitness: st.and_then(|s| s.get("fitness")).and_then(|v| v.as_f64()),
                                returns: st.and_then(|s| s.get("returns")).and_then(|v| v.as_f64()),
                                turnover: st.and_then(|s| s.get("turnover")).and_then(|v| v.as_f64()),
                                direct_url: Some(format!("https://platform.worldquantbrain.com/alpha/{}", aid)),
                                code,
                                tags: item.get("tags").and_then(|t| t.as_array()).map(|arr| {
                                    arr.iter().filter_map(|x| x.as_str().map(String::from)).collect()
                                }),
                                settings: sett.cloned(),
                            });
                        }
                    }
                }
            }
        }

        Ok(portfolio)
    }
}
