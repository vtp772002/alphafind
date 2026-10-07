use anyhow::{Context, Result};
use colored::Colorize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::autotuner::FitnessAutoTuner;
use crate::client::BrainClient;
use crate::models::{CandidateAlpha, SubmittableAlphaRecord};

/// A single authenticated BRAIN account worker used for distributed screening.
pub struct AccountWorker {
    pub client: BrainClient,
    pub role: String,
    pub is_main: bool,
}

/// Coordinates distributed alpha screening across multiple authenticated BRAIN accounts.
pub struct MultiAccountScreener {
    pub main_client: BrainClient,
    pub accounts: Vec<AccountWorker>,
    pub workers_per_account: usize,
    pub transfer_lock: Arc<Mutex<()>>,
}

impl MultiAccountScreener {
    /// Creates a multi-account screener by loading credentials from environment variables.
    pub fn from_env(workers_per_account: usize) -> Result<Self> {
        dotenvy::dotenv().ok();

        let main_email = std::env::var("MAIN_ACCOUNT_EMAIL")
            .or_else(|_| std::env::var("WQ_BRAIN_EMAIL"))
            .context("Missing MAIN_ACCOUNT_EMAIL in .env")?;
        let main_pass = std::env::var("MAIN_ACCOUNT_PASSWORD")
            .or_else(|_| std::env::var("WQ_BRAIN_PASSWORD"))
            .context("Missing MAIN_ACCOUNT_PASSWORD in .env")?;

        let main_client = BrainClient::new(&main_email, &main_pass)?;
        let mut accounts = Vec::new();

        accounts.push(AccountWorker {
            client: main_client.clone(),
            role: "MAIN".to_string(),
            is_main: true,
        });

        // Load Secondary / Worker Sessions (for collaborative team research pooling)
        let mut idx = 1;
        loop {
            let s_email = std::env::var(format!("SECONDARY_ACCOUNT_{idx}_EMAIL"))
                .or_else(|_| std::env::var(format!("SCOUT_ACCOUNT_{idx}_EMAIL")))
                .or_else(|_| std::env::var(format!("WQ_BRAIN_EMAIL_ACC{}", idx + 1)));
            let s_pass = std::env::var(format!("SECONDARY_ACCOUNT_{idx}_PASSWORD"))
                .or_else(|_| std::env::var(format!("SCOUT_ACCOUNT_{idx}_PASSWORD")))
                .or_else(|_| std::env::var(format!("WQ_BRAIN_PASSWORD_ACC{}", idx + 1)));
            match (s_email, s_pass) {
                (Ok(email), Ok(pass)) => {
                    if let Ok(client) = BrainClient::new(&email, &pass) {
                        accounts.push(AccountWorker {
                            client,
                            role: format!("WORKER_{idx}"),
                            is_main: false,
                        });
                    }
                    idx += 1;
                }
                _ => break,
            }
        }

        Ok(Self {
            main_client,
            accounts,
            workers_per_account,
            transfer_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Authenticates all accounts in parallel
    pub async fn authenticate_all(&self) -> Result<()> {
        println!(
            "{}",
            "🔐 Authenticating all configured WorldQuant BRAIN accounts...".bold()
        );
        let mut tasks = Vec::new();

        for acc in &self.accounts {
            let client = acc.client.clone();
            let role = acc.role.clone();
            let email = acc.client.email().to_string();

            tasks.push(tokio::spawn(async move {
                let res = client.authenticate().await;
                (role, email, res)
            }));
        }

        for t in tasks {
            let (role, email, res) = t.await?;
            match res {
                Ok(_) => println!("  {} [{}] {}", "✅ Connected:".green(), role.cyan(), email),
                Err(e) => println!("  {} [{}] {}: {}", "❌ Failed:".red(), role.red(), email, e),
            }
        }

        Ok(())
    }

    /// Screens a batch of candidate alphas across 9 concurrent workers
    pub async fn screen_batch(
        &self,
        candidates: Vec<CandidateAlpha>,
        min_fitness: f64,
        enable_tuning: bool,
    ) -> Result<Vec<SubmittableAlphaRecord>> {
        let total_workers = self.accounts.len() * self.workers_per_account;
        println!("\n{}", "=".repeat(85));
        println!(
            "{}",
            format!(
                "🚀 DISTRIBUTED SCREENING: {} Candidates across {} Workers ({} Accounts)",
                candidates.len(),
                total_workers,
                self.accounts.len()
            )
            .bold()
            .cyan()
        );
        println!("{}", "=".repeat(85));

        let mut qualified_alphas = Vec::new();
        let mut tasks = Vec::new();

        for (idx, cand) in candidates.into_iter().enumerate() {
            // Assign candidate to an account in round-robin fashion
            let acc_idx = idx % self.accounts.len();
            let acc = &self.accounts[acc_idx];
            let client = acc.client.clone();
            let main_client = self.main_client.clone();
            let is_main = acc.is_main;
            let role = acc.role.clone();
            let transfer_lock = self.transfer_lock.clone();

            tasks.push(tokio::spawn(async move {
                let settings = cand.to_settings();
                println!(
                    "[{}] Dispatching '{}' on {}...",
                    role.cyan(),
                    cand.name,
                    settings.universe
                );

                let sim_res = client.submit_simulation(&cand.expression, &settings).await;
                let sim_id = match sim_res {
                    Ok(id) => id,
                    Err(e) => {
                        println!("  {} [{}] Failed: {}", "❌".red(), cand.name, e);
                        return None;
                    }
                };

                let poll_res = client.poll_simulation(&sim_id, 350, 5).await;
                let alpha_id = match poll_res {
                    Ok(r) => match r.alpha {
                        Some(a) => a,
                        None => return None,
                    },
                    Err(e) => {
                        println!("  {} [{}] Poll error: {}", "❌".red(), cand.name, e);
                        return None;
                    }
                };

                let details = client.get_alpha_details(&alpha_id).await.ok()?;
                let stats = details.is?;

                let sh = stats.sharpe.unwrap_or(0.0);
                let fit = stats.fitness.unwrap_or(0.0);
                let ret = stats.returns.unwrap_or(0.0);
                let turn = stats.turnover.unwrap_or(0.0);

                println!(
                    "  {} [{}] [{}] Sharpe: {:.2} | Fit: {:.2} | Ret: {:.1}% | Turn: {:.1}%",
                    "📊 Result:".bold(),
                    alpha_id.yellow(),
                    cand.name,
                    sh,
                    fit,
                    ret * 100.0,
                    turn * 100.0
                );

                // Auto-tune if promising
                let (final_aid, final_fit, final_sh, final_ret, final_turn, final_settings, final_expr) =
                    if fit < min_fitness && sh >= 1.25 && ret >= 0.05 && enable_tuning {
                        let tuner = FitnessAutoTuner::new(&client);
                        if let Ok(Some(tuned)) = tuner.tune_hyperparameters(&cand.expression, &settings, min_fitness).await {
                            let t_st = tuned.details.is.unwrap_or_default();
                            (
                                tuned.details.id,
                                t_st.fitness.unwrap_or(fit),
                                t_st.sharpe.unwrap_or(sh),
                                t_st.returns.unwrap_or(ret),
                                t_st.turnover.unwrap_or(turn),
                                tuned.settings,
                                tuned.expression,
                            )
                        } else {
                            (alpha_id, fit, sh, ret, turn, settings.clone(), cand.expression.clone())
                        }
                    } else {
                        (alpha_id, fit, sh, ret, turn, settings.clone(), cand.expression.clone())
                    };

                // Check submission criteria
                if final_fit >= min_fitness && final_sh >= 1.25 && (0.01..=0.70).contains(&final_turn) {
                    println!(
                        "{}",
                        format!(
                            "  🎯 [HIGH-CONVICTION] Alpha {} qualified! (Fit: {:.2} >= {:.2})",
                            final_aid, final_fit, min_fitness
                        )
                        .green()
                        .bold()
                    );

                    // Transfer lock to Main Account verification
                    let _lock = transfer_lock.lock().await;
                    let (main_aid, main_fit, main_sh, main_turn, main_ret) = if !is_main {
                        println!("  [*] Transferring {} to MAIN Account for official 8/8 check...", final_aid);
                        let main_sett = final_settings.clone();

                        let m_sim = main_client.submit_simulation(&final_expr, &main_sett).await.ok()?;
                        let m_poll = main_client.poll_simulation(&m_sim, 350, 5).await.ok()?;
                        let m_aid = m_poll.alpha?;
                        let m_det = main_client.get_alpha_details(&m_aid).await.ok()?;
                        let m_st = m_det.is?;
                        (
                            m_aid,
                            m_st.fitness.unwrap_or(final_fit),
                            m_st.sharpe.unwrap_or(final_sh),
                            m_st.turnover.unwrap_or(final_turn),
                            m_st.returns.unwrap_or(final_ret),
                        )
                    } else {
                        (final_aid, final_fit, final_sh, final_turn, final_ret)
                    };

                    // Run official check on Main
                    let (check_pass, check_msg, _) = main_client.check_submission(&main_aid, 45).await.ok()?;
                    if check_pass {
                        println!(
                            "{}",
                            format!(
                                "  🌟 [OFFICIAL 8/8 PASS] Alpha {} on MAIN is 100% Submittable!\n  URL: https://platform.worldquantbrain.com/alpha/{}",
                                main_aid, main_aid
                            )
                            .green()
                            .bold()
                        );

                        let sett_json = serde_json::to_string(&final_settings).unwrap_or_default();

                        return Some(SubmittableAlphaRecord {
                            alpha_id: main_aid,
                            name: cand.name.clone(),
                            universe: final_settings.universe,
                            sharpe: (main_sh * 100.0).round() / 100.0,
                            fitness: (main_fit * 100.0).round() / 100.0,
                            turnover: (main_turn * 1000.0).round() / 10.0,
                            annual_return: (main_ret * 1000.0).round() / 10.0,
                            submitted: false,
                            expression: final_expr,
                            settings: sett_json,
                        });
                    } else {
                        println!("  {} [Check Failed on Main] {}: {}", "⚠️".yellow(), main_aid, check_msg);
                    }
                }

                None
            }));
        }

        for t in tasks {
            if let Ok(Some(record)) = t.await {
                qualified_alphas.push(record);
            }
        }

        if !qualified_alphas.is_empty() {
            Self::save_to_submittable_file(&qualified_alphas)?;
        }

        Ok(qualified_alphas)
    }

    /// Appends qualified alphas to submittable_alphas.csv
    pub fn save_to_submittable_file(records: &[SubmittableAlphaRecord]) -> Result<()> {
        let csv_path = "submittable_alphas.csv";
        let file_exists = std::path::Path::new(csv_path).exists();

        let mut existing_ids = std::collections::HashSet::new();
        if file_exists {
            if let Ok(mut rdr) = csv::Reader::from_path(csv_path) {
                for r in rdr.deserialize::<SubmittableAlphaRecord>().flatten() {
                    existing_ids.insert(r.alpha_id);
                }
            }
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(csv_path)?;

        let mut wtr = if file_exists {
            csv::WriterBuilder::new()
                .has_headers(false)
                .from_writer(file)
        } else {
            csv::WriterBuilder::new()
                .has_headers(true)
                .from_writer(file)
        };

        for r in records {
            if !existing_ids.contains(&r.alpha_id) {
                wtr.serialize(r)?;
                println!("  💾 [Saved to CSV] {}", r.alpha_id.green());
            }
        }

        wtr.flush()?;
        Ok(())
    }
}
