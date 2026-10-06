use anyhow::{anyhow, Context, Result};
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::models::{
    AlphaDetails, AlphaSettings, CompetitionEntry, DailyPnlResponse, DatasetEntry,
    DatasetListResponse, InSampleStats, LeaderboardResponse, PortfolioAlpha, SimulationPayload,
    SimulationResponse, UserProfile,
};

pub const BASE_URL: &str = "https://api.worldquantbrain.com";

#[derive(Clone)]
pub struct BrainClient {
    client: reqwest::Client,
    email: String,
    auth_value: String,
    authenticated: Arc<AtomicBool>,
}

impl BrainClient {
    /// Creates a new BRAIN API client with the given credentials.
    pub fn new(email: impl Into<String>, password: impl Into<String>) -> Result<Self> {
        let email = email.into();
        let password = password.into();

        let creds = format!("{}:{}", email, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
        let auth_value = format!("Basic {}", encoded);

        let mut default_headers = HeaderMap::new();
        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .gzip(true)
            .timeout(Duration::from_secs(60))
            .default_headers(default_headers)
            .build()
            .context("Failed to build reqwest HTTP client")?;

        Ok(Self {
            client,
            email,
            auth_value,
            authenticated: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Returns the email address for this client session.
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Returns headers with Authorization for GET requests (Accept is already default).
    fn auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&self.auth_value)?);
        Ok(headers)
    }

    /// Returns headers with Authorization + Content-Type for POST/PATCH requests.
    fn auth_headers_json(&self) -> Result<HeaderMap> {
        let mut headers = self.auth_headers()?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    /// Authenticates with the BRAIN API using Basic auth and establishes a session cookie.
    pub async fn authenticate(&self) -> Result<()> {
        if self.authenticated.load(Ordering::SeqCst) {
            return Ok(());
        }

        let url = format!("{}/authentication", BASE_URL);

        for attempt in 0..4 {
            let headers = self.auth_headers_json()?;

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
                            .unwrap_or(2.0f64.powi(attempt) + 1.5);
                        println!("[*] [Auth 429] {} Sleeping {:.1}s...", self.email, wait);
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
                    return Err(anyhow!(
                        "Connection error during auth for {}: {}",
                        self.email,
                        e
                    ));
                }
            }
        }

        Err(anyhow!("Exceeded max auth retries for {}", self.email))
    }

    /// Submits a FastExpr alpha simulation and returns the simulation ID/URL.
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

        for retry in 0..8 {
            let headers = self.auth_headers_json()?;

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
                        return Err(anyhow!(
                            "Simulation succeeded but no ID/Location returned: {:?}",
                            body
                        ));
                    } else if status.as_u16() == 429 {
                        let wait = resp
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(2.0f64.powi(retry.min(5)) + 1.2);
                        sleep(Duration::from_secs_f64(wait)).await;
                        continue;
                    } else {
                        let err_body = resp.text().await.unwrap_or_default();
                        return Err(anyhow!(
                            "Simulation post failed (HTTP {}): {}",
                            status,
                            err_body
                        ));
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

    /// Polls a running simulation until completion, error, or timeout.
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

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            let headers = self.auth_headers()?;

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

    /// Fetches complete alpha details including In-Sample statistics.
    pub async fn get_alpha_details(&self, alpha_id: &str) -> Result<AlphaDetails> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}", BASE_URL, alpha_id);
        let headers = self.auth_headers()?;

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

    /// Verifies the 8 mandatory submission checks for an alpha.
    pub async fn check_submission(
        &self,
        alpha_id: &str,
        max_attempts: usize,
    ) -> Result<(bool, String, InSampleStats)> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}/check", BASE_URL, alpha_id);

        for _attempt in 0..max_attempts {
            let headers = self.auth_headers()?;

            let resp = self.client.get(&url).headers(headers).send().await?;

            let retry_after = resp
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok());

            let status = resp.status();
            if status.as_u16() == 429 || status.is_server_error() {
                let wait = retry_after.unwrap_or(3.0);
                sleep(Duration::from_secs_f64(wait)).await;
                continue;
            }

            if !status.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                return Err(anyhow!(
                    "API check returned HTTP {}: {}",
                    status,
                    err_text.chars().take(200).collect::<String>()
                ));
            }

            let text = resp.text().await?;
            if text.trim().is_empty() {
                let wait = retry_after.unwrap_or(2.0);
                sleep(Duration::from_secs_f64(wait)).await;
                continue;
            }

            let val: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    if text.trim().starts_with('<') {
                        // Gateway or proxy HTML returned instead of JSON
                        let wait = retry_after.unwrap_or(3.0);
                        sleep(Duration::from_secs_f64(wait)).await;
                        continue;
                    }
                    return Err(anyhow!(
                        "Failed to parse check response JSON: {}. Body: {}",
                        e,
                        text.chars().take(200).collect::<String>()
                    ));
                }
            };
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

        Err(anyhow!(
            "Submission check timed out waiting for correlation calculation"
        ))
    }

    /// Fetches the daily PnL time series for a given alpha.
    pub async fn fetch_daily_pnl(&self, alpha_id: &str) -> Result<HashMap<String, f64>> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}/recordsets/daily-pnl", BASE_URL, alpha_id);

        for attempt in 0..5 {
            let headers = self.auth_headers()?;

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
                                    if let (Some(d), Some(val)) = (row[0].as_str(), row[1].as_f64())
                                    {
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

    /// Submits an alpha to Out-of-Sample tracking.
    pub async fn submit_alpha(&self, alpha_id: &str) -> Result<(bool, String)> {
        self.authenticate().await?;

        let url = format!("{}/alphas/{}/submit", BASE_URL, alpha_id);
        let headers = self.auth_headers_json()?;

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

    /// Updates alpha metadata (name, category, color, tags).
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

        let headers = self.auth_headers_json()?;

        let resp = self
            .client
            .patch(&url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        Ok(resp.status().is_success())
    }

    /// Fetches user's alphas with the given query parameters.
    pub async fn fetch_user_alphas(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        self.authenticate().await?;

        let url = format!("{}/users/self/alphas?{}", BASE_URL, query);
        let headers = self.auth_headers()?;

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?
            .error_for_status()?;
        let data: serde_json::Value = resp.json().await?;

        if let Some(arr) = data.as_array() {
            Ok(arr.clone())
        } else if let Some(arr) = data.get("results").and_then(|r| r.as_array()) {
            Ok(arr.clone())
        } else {
            Ok(Vec::new())
        }
    }

    /// Fetches all active Out-of-Sample alphas in the user's portfolio.
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
                            || matches!(
                                item.get("status").and_then(|s| s.as_str()),
                                Some("ACTIVE") | Some("SUBMITTED") | Some("UNPROCESSED")
                            )
                            || item.get("dateSubmitted").is_some();

                        if is_os && seen.insert(aid.to_string()) {
                            let st = item.get("is");
                            let sett = item.get("settings");
                            let code = item
                                .get("regular")
                                .and_then(|r| r.get("code"))
                                .and_then(|c| c.as_str())
                                .map(|s| s.trim().to_string())
                                .or_else(|| {
                                    item.get("code")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.to_string())
                                });

                            portfolio.push(PortfolioAlpha {
                                id: aid.to_string(),
                                name: item.get("name").and_then(|v| v.as_str()).map(String::from),
                                status: item
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                stage: item.get("stage").and_then(|v| v.as_str()).map(String::from),
                                date_submitted: item
                                    .get("dateSubmitted")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                universe: sett
                                    .and_then(|s| s.get("universe"))
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                decay: sett
                                    .and_then(|s| s.get("decay"))
                                    .and_then(|v| v.as_i64())
                                    .map(|d| d as i32),
                                category: item
                                    .get("category")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                color: item.get("color").and_then(|v| v.as_str()).map(String::from),
                                sharpe: st.and_then(|s| s.get("sharpe")).and_then(|v| v.as_f64()),
                                fitness: st.and_then(|s| s.get("fitness")).and_then(|v| v.as_f64()),
                                returns: st.and_then(|s| s.get("returns")).and_then(|v| v.as_f64()),
                                turnover: st
                                    .and_then(|s| s.get("turnover"))
                                    .and_then(|v| v.as_f64()),
                                direct_url: Some(format!(
                                    "https://platform.worldquantbrain.com/alpha/{}",
                                    aid
                                )),
                                code,
                                tags: item.get("tags").and_then(|t| t.as_array()).map(|arr| {
                                    arr.iter()
                                        .filter_map(|x| x.as_str().map(String::from))
                                        .collect()
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

    /// Fetches the user's profile information from /users/self.
    pub async fn fetch_user_profile(&self) -> Result<UserProfile> {
        self.authenticate().await?;

        let url = format!("{}/users/self", BASE_URL);
        let headers = self.auth_headers()?;

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?
            .error_for_status()?;

        let profile = resp.json::<UserProfile>().await?;
        Ok(profile)
    }

    /// Fetches the user's active competitions and leaderboard scores from /users/self/competitions.
    pub async fn fetch_competitions(&self) -> Result<Vec<CompetitionEntry>> {
        self.authenticate().await?;

        let url = format!("{}/users/self/competitions", BASE_URL);
        let headers = self.auth_headers()?;

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?
            .error_for_status()?;

        let data: serde_json::Value = resp.json().await?;
        let results = data.get("results").cloned().unwrap_or(data);
        let comps: Vec<CompetitionEntry> = serde_json::from_value(results)?;
        Ok(comps)
    }

    /// Fetches the standings leaderboard for a given competition.
    pub async fn fetch_competition_leaderboard(
        &self,
        competition_id: &str,
        limit: usize,
    ) -> Result<LeaderboardResponse> {
        self.authenticate().await?;

        let url = format!(
            "{}/competitions/{}/boards/leader?limit={}&offset=0",
            BASE_URL, competition_id, limit
        );
        let headers = self.auth_headers()?;

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?
            .error_for_status()?;

        let board = resp.json::<LeaderboardResponse>().await?;
        Ok(board)
    }

    /// Fetches all datasets with pagination (limit 50 per page) for a given region with exponential backoff on 429.
    pub async fn fetch_all_datasets(&self, region: &str) -> Result<Vec<DatasetEntry>> {
        self.authenticate().await?;

        let mut all_datasets = Vec::new();
        let mut offset = 0;
        let limit = 50;

        loop {
            let url = format!(
                "{}/data-sets?region={}&limit={}&offset={}",
                BASE_URL, region, limit, offset
            );

            let mut fetched_page = false;
            for attempt in 0..5 {
                let headers = self.auth_headers()?;
                let resp = self.client.get(&url).headers(headers).send().await;

                match resp {
                    Ok(r) => {
                        let status = r.status();
                        if status.as_u16() == 429 {
                            let wait = r
                                .headers()
                                .get(RETRY_AFTER)
                                .and_then(|v| v.to_str().ok())
                                .and_then(|v| v.parse::<f64>().ok())
                                .unwrap_or(2.0f64.powi(attempt) + 1.0);
                            sleep(Duration::from_secs_f64(wait)).await;
                            continue;
                        }
                        if status.is_success() {
                            let list: DatasetListResponse = r.json().await?;
                            let total = list.count;
                            let n_fetched = list.results.len();

                            all_datasets.extend(list.results);

                            offset += limit;
                            if offset >= total || n_fetched == 0 {
                                return Ok(all_datasets);
                            }
                            fetched_page = true;
                            break;
                        }
                    }
                    Err(_) => {
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }

            if !fetched_page {
                anyhow::bail!(
                    "Failed to fetch dataset page at offset {} after 5 retries",
                    offset
                );
            }
        }
    }
}
