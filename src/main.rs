use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use alphafind::client::BrainClient;
use alphafind::correlation::{audit_candidate, simulate_portfolio};
use alphafind::models::{AlphaSettings, PortfolioAlpha};
use alphafind::screener::MultiAccountScreener;
use alphafind::taxonomy::{get_curated_candidates, FactorPillar};

#[derive(Parser)]
#[command(name = "alphafind")]
#[command(about = "High-Performance Quantitative Alpha Mining Engine for WorldQuant BRAIN", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate all configured BRAIN accounts and sessions concurrently
    Auth,

    /// Sync active Out-of-Sample (OS) portfolio from BRAIN into portfolio_os.json
    Sync,

    /// Verify the 8 mandatory In-Sample platform submission checks for an Alpha
    Check {
        /// Alpha ID (e.g., A1b2C3d4)
        alpha_id: String,
    },

    /// Run exact 1,236-day Pearson correlation audit against all active OS Alphas
    Audit {
        /// Alpha ID (e.g., A1b2C3d4)
        alpha_id: String,

        /// Optional label / description
        #[arg(short, long)]
        label: Option<String>,
    },

    /// Simulate merged multi-alpha Out-of-Sample portfolio Sharpe and diversification metrics
    Portfolio,

    /// Submit an Alpha to Out-of-Sample (OS) after 8/8 PASS verification
    Submit {
        /// Alpha ID (e.g., A1b2C3d4)
        alpha_id: String,

        /// Custom Alpha Name
        #[arg(long)]
        name: Option<String>,

        /// Alpha Category (e.g., FUNDAMENTAL, PRICE_VOLUME)
        #[arg(long)]
        category: Option<String>,

        /// Color badge (e.g., BLUE, GREEN, YELLOW)
        #[arg(long)]
        color: Option<String>,

        /// Comma-separated tags
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,

        /// Skip pre-flight check verification (not recommended)
        #[arg(long)]
        skip_checks: bool,
    },

    /// Launch distributed 9-worker candidate screening across 3 accounts
    Screen {
        /// Target Factor Pillar (ANALYST, OPTIONS, MICRO, QUAL, RISK, SHORT)
        #[arg(short, long)]
        pillar: Option<String>,

        /// Target Universe (TOP3000, TOP1000, TOP500, TOP200)
        #[arg(short, long, default_value = "TOP1000")]
        universe: String,

        /// Minimum Fitness required to qualify (default: 1.50)
        #[arg(short, long, default_value_t = 1.50)]
        min_fitness: f64,

        /// Workers per account (default: 3 -> 9 total across 3 accounts)
        #[arg(short, long, default_value_t = 3)]
        workers: usize,

        /// Disable automatic decay parameter sweeps
        #[arg(long)]
        no_tune: bool,
    },

    /// Simulate a single Alpha expression directly on MAIN account
    Sim {
        /// FastExpr alpha formula
        #[arg(short, long)]
        expr: String,

        /// Target Universe (default: TOP1000)
        #[arg(short, long, default_value = "TOP1000")]
        universe: String,

        /// Signal decay parameter (default: 5)
        #[arg(short, long, default_value_t = 5)]
        decay: i32,

        /// Neutralization group (SUBINDUSTRY, SECTOR, MARKET, NONE)
        #[arg(short, long, default_value = "SUBINDUSTRY")]
        neutralization: String,

        /// Weight truncation limit (default: 0.05)
        #[arg(short, long, default_value_t = 0.05)]
        truncation: f64,
    },
}

fn get_main_client() -> Result<BrainClient> {
    dotenvy::dotenv().ok();
    let email = std::env::var("MAIN_ACCOUNT_EMAIL")
        .or_else(|_| std::env::var("WQ_BRAIN_EMAIL"))
        .context("Missing MAIN_ACCOUNT_EMAIL in .env")?;
    let pass = std::env::var("MAIN_ACCOUNT_PASSWORD")
        .or_else(|_| std::env::var("WQ_BRAIN_PASSWORD"))
        .context("Missing MAIN_ACCOUNT_PASSWORD in .env")?;

    BrainClient::new(email, pass)
}

fn get_cache_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let cache_dir = std::path::PathBuf::from(home)
        .join(".cache")
        .join("alphafind");
    let _ = fs::create_dir_all(&cache_dir);
    cache_dir
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth => cmd_auth().await,
        Commands::Sync => cmd_sync().await,
        Commands::Check { alpha_id } => cmd_check(alpha_id).await,
        Commands::Audit { alpha_id, label } => cmd_audit(alpha_id, label).await,
        Commands::Portfolio => cmd_portfolio().await,
        Commands::Submit {
            alpha_id,
            name,
            category,
            color,
            tags,
            skip_checks,
        } => cmd_submit(alpha_id, name, category, color, tags, skip_checks).await,
        Commands::Screen {
            pillar,
            universe,
            min_fitness,
            workers,
            no_tune,
        } => cmd_screen(pillar, universe, min_fitness, workers, no_tune).await,
        Commands::Sim {
            expr,
            universe,
            decay,
            neutralization,
            truncation,
        } => cmd_sim(expr, universe, decay, neutralization, truncation).await,
    }
}

async fn cmd_auth() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Multi-Account Authentication"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let screener = MultiAccountScreener::from_env(3)?;
    screener.authenticate_all().await?;
    Ok(())
}

async fn cmd_sync() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Out-of-Sample (OS) Portfolio Sync"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!("  Connecting to MAIN account: {}...", client.email().cyan());
    let portfolio = client.fetch_active_portfolio().await?;

    println!(
        "  {} Found {} active OS alphas.",
        "✅".green(),
        portfolio.len()
    );

    let mut univ_counts: HashMap<String, usize> = HashMap::new();
    for p in &portfolio {
        let u = p.universe.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        *univ_counts.entry(u).or_insert(0) += 1;
    }

    println!("\n  {} Universe Diversification Breakdown:", "📊".bold());
    for (u, count) in &univ_counts {
        let bar = "█".repeat(*count * 2);
        println!("    {:<12} : {:2} alphas {}", u.yellow(), count, bar.cyan());
    }

    let json_path = "portfolio_os.json";
    let json_str = serde_json::to_string_pretty(&portfolio)?;
    if let Err(e) = fs::write(json_path, json_str) {
        eprintln!("  ⚠️  Warning: Failed to write cache: {}", e);
    } else {
        println!(
            "\n  {} Successfully updated {}",
            "💾".green(),
            json_path.bold()
        );
    }
    Ok(())
}

async fn cmd_check(alpha_id: String) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        format!(
            "  AlphaFind Quant Engine — 8/8 Submission Check [{}]",
            alpha_id
        )
        .bold()
        .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!("  Querying check results for {}...", alpha_id.yellow());

    let (passed, summary, stats) = client.check_submission(&alpha_id, 30).await?;

    println!("\n  ┌──────────────────────────────┬────────────┐");
    println!("  │ Check Name                   │ Result     │");
    println!("  ├──────────────────────────────┼────────────┤");

    for c in &stats.checks {
        let res_styled = if c.result == "PASS" {
            format!("{:<10}", "PASS".green().bold())
        } else {
            format!("{:<10}", "FAIL".red().bold())
        };
        println!("  │ {:<28} │ {} │", c.name, res_styled);
    }
    println!("  └──────────────────────────────┴────────────┘");

    if let (Some(sh), Some(fit), Some(ret), Some(turn)) =
        (stats.sharpe, stats.fitness, stats.returns, stats.turnover)
    {
        println!(
            "\n  Metrics: Sharpe: {:.2} | Fitness: {:.2} | Return: {:.1}% | Turnover: {:.1}%",
            sh,
            fit,
            ret * 100.0,
            turn * 100.0
        );
    }

    if passed {
        println!("\n  {} {}", "🌟 [PASSED]".green().bold(), summary.green());
        println!(
            "  Submission URL: https://platform.worldquantbrain.com/alpha/{}",
            alpha_id.cyan()
        );
    } else {
        println!("\n  {} {}", "❌ [FAILED]".red().bold(), summary.red());
    }
    Ok(())
}

async fn cmd_audit(alpha_id: String, label: Option<String>) -> Result<()> {
    let label_str = label.as_deref().unwrap_or(&alpha_id);
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        format!(
            "  AlphaFind Quant Engine — 1,236-Day Pearson Correlation Audit [{}]",
            label_str
        )
        .bold()
        .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!(
        "  Fetching candidate daily PnL for {}...",
        alpha_id.yellow()
    );
    let cand_pnl = client.fetch_daily_pnl(&alpha_id).await?;
    println!("  Candidate PnL records: {} trading days.", cand_pnl.len());

    // Load OS Portfolio
    let port_path = "portfolio_os.json";
    if !Path::new(port_path).exists() {
        anyhow::bail!("portfolio_os.json not found! Run 'alphafind sync' first.");
    }
    let port_data = fs::read_to_string(port_path)?;
    let os_alphas: Vec<PortfolioAlpha> = serde_json::from_str(&port_data)?;

    // Load or fetch cached baseline PnLs
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    let mut pnl_cache: HashMap<String, HashMap<String, f64>> = if cache_file.exists() {
        fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut missing_ids = Vec::new();
    for a in &os_alphas {
        if !pnl_cache.contains_key(&a.id) {
            missing_ids.push(a.id.clone());
        }
    }

    if !missing_ids.is_empty() {
        println!(
            "  Fetching PnL for {} active OS alphas not in cache...",
            missing_ids.len()
        );
        for id in missing_ids {
            if let Ok(pnl) = client.fetch_daily_pnl(&id).await {
                pnl_cache.insert(id, pnl);
            }
        }
        // Save updated cache
        if let Ok(serialized) = serde_json::to_string(&pnl_cache) {
            if let Err(e) = fs::write(&cache_file, &serialized) {
                eprintln!("  ⚠️  Warning: Failed to write cache: {}", e);
            }
        }
    }

    // Run Audit
    let report = audit_candidate(&alpha_id, &cand_pnl, &pnl_cache);

    println!(
        "\n  Pairwise Correlation Audit vs {} OS Alphas:",
        pnl_cache.len()
    );
    println!("  ─────────────────────────────────────────────────────────────────────────");
    for (idx, (os_id, corr, days)) in report.pairwise_results.iter().take(8).enumerate() {
        let status_icon = if *corr <= 0.15 {
            "🟢 TRUE ORTHOGONAL".green()
        } else if *corr <= 0.70 {
            "🟡 COMPLIANT (<0.70)".yellow()
        } else {
            "🔴 COLLISION (>0.70)".red()
        };

        println!(
            "    #{:<2} vs {:<10} : rho = {:+7.4} ({:+6.2}%) | {:4} days -> {}",
            idx + 1,
            os_id.yellow(),
            corr,
            corr * 100.0,
            days,
            status_icon
        );
    }

    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!(
        "  MAX CORRELATION: {:+.4} ({:+.2}%) vs {}",
        report.max_correlation,
        report.max_correlation * 100.0,
        report.most_correlated_id.yellow()
    );
    println!(
        "  AVERAGE CORRELATION: {:+.4} ({:+.2}%)",
        report.avg_correlation,
        report.avg_correlation * 100.0
    );

    if report.is_true_orthogonal {
        println!("\n  {} Candidate is 100% TRUE ORTHOGONAL (rho <= 0.15). Maximum portfolio diversification value!", "🌟 [EXCELLENT]".green().bold());
    } else if report.is_brain_compliant {
        println!(
            "\n  {} Candidate satisfies WorldQuant BRAIN self-correlation limit (rho <= 0.70).",
            "✅ [COMPLIANT]".green()
        );
    } else {
        println!(
            "\n  {} Candidate violates self-correlation limit (rho > 0.70). Unsubmittable.",
            "❌ [REJECTED]".red().bold()
        );
    }
    Ok(())
}

async fn cmd_portfolio() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Multi-Alpha Portfolio Merge Simulation"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let port_path = "portfolio_os.json";
    if !Path::new(port_path).exists() {
        anyhow::bail!("portfolio_os.json not found! Run 'alphafind sync' first.");
    }
    let port_data = fs::read_to_string(port_path)?;
    let os_alphas: Vec<PortfolioAlpha> = serde_json::from_str(&port_data)?;

    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    let mut pnl_cache: HashMap<String, HashMap<String, f64>> = if cache_file.exists() {
        fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut missing_ids = Vec::new();
    for a in &os_alphas {
        if !pnl_cache.contains_key(&a.id) {
            missing_ids.push(a.id.clone());
        }
    }

    if !missing_ids.is_empty() {
        let client = get_main_client()?;
        println!(
            "  Fetching PnL for {} active OS alphas not in cache...",
            missing_ids.len()
        );
        for id in missing_ids {
            if let Ok(pnl) = client.fetch_daily_pnl(&id).await {
                pnl_cache.insert(id, pnl);
            }
        }
        if let Ok(serialized) = serde_json::to_string(&pnl_cache) {
            if let Err(e) = fs::write(&cache_file, &serialized) {
                eprintln!("  ⚠️  Warning: Failed to write cache: {}", e);
            }
        }
    }

    let pnl_refs: Vec<&HashMap<String, f64>> = os_alphas
        .iter()
        .filter_map(|a| pnl_cache.get(&a.id))
        .collect();

    if let Some(metrics) = simulate_portfolio(&pnl_refs) {
        println!(
            "\n  Active Portfolio Aggregate Performance ({} Alphas):",
            metrics.n_alphas
        );
        println!("  ─────────────────────────────────────────────────────────────────────────");
        println!(
            "    Common Backtest Trading Days:  {}",
            metrics.n_trading_days
        );
        println!(
            "    Annualized Merged PnL:         ${:.2}",
            metrics.annualized_pnl
        );
        println!(
            "    Annualized Volatility:         ${:.2}",
            metrics.annualized_vol
        );
        println!(
            "    MERGED PORTFOLIO SHARPE:       {}",
            format!("{:.2}", metrics.merged_sharpe).green().bold()
        );
        println!(
            "    Average Pairwise Correlation:  {}",
            format!(
                "{:.4} ({:.2}%)",
                metrics.avg_pairwise_corr,
                metrics.avg_pairwise_corr * 100.0
            )
            .cyan()
        );
        println!("  ─────────────────────────────────────────────────────────────────────────");
    } else {
        println!(
            "  {} Could not simulate portfolio (insufficient common trading days).",
            "⚠️".yellow()
        );
    }
    Ok(())
}

async fn cmd_submit(
    alpha_id: String,
    name: Option<String>,
    category: Option<String>,
    color: Option<String>,
    tags: Option<Vec<String>>,
    skip_checks: bool,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        format!("  AlphaFind Quant Engine — Alpha Submission [{}]", alpha_id)
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;

    if !skip_checks {
        println!("  Verifying 8/8 submission checks on MAIN account...");
        let (passed, msg, _) = client.check_submission(&alpha_id, 30).await?;
        if !passed {
            println!("  {} Cannot submit: {}", "❌".red(), msg.red());
            anyhow::bail!("Alpha failed submission checks");
        }
        println!("  {} 8/8 Submission Checks Confirmed!", "✅".green());
    }

    // Update metadata if specified
    if name.is_some() || category.is_some() || color.is_some() || tags.is_some() {
        println!("  Updating Alpha metadata...");
        let _ = client
            .update_metadata(
                &alpha_id,
                name.as_deref(),
                category.as_deref(),
                color.as_deref(),
                tags.as_deref(),
            )
            .await;
    }

    println!("  Dispatching submission to WorldQuant BRAIN API...");
    let (success, resp_msg) = client.submit_alpha(&alpha_id).await?;

    if success {
        println!(
            "  {} {}",
            "🌟 [SUBMITTED SUCCESSFULLY]".green().bold(),
            resp_msg.green()
        );
        println!(
            "  Out-of-Sample Tracking URL: https://platform.worldquantbrain.com/alpha/{}",
            alpha_id.cyan()
        );

        // Update submittable_alphas.csv
        let csv_path = "submittable_alphas.csv";
        if Path::new(csv_path).exists() {
            if let Ok(content) = fs::read_to_string(csv_path) {
                let updated = content
                    .lines()
                    .map(|line| {
                        if line.starts_with(&format!("{},", alpha_id)) {
                            line.replace(",False,", ",True,")
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Err(e) = fs::write(csv_path, &updated) {
                    eprintln!("  ⚠️  Warning: Failed to write cache: {}", e);
                } else {
                    println!(
                        "  💾 Marked {} as Submitted=True in {}",
                        alpha_id.green(),
                        csv_path
                    );
                }
            }
        }
    } else {
        println!(
            "  {} Submission dispatch failed: {}",
            "❌".red(),
            resp_msg.red()
        );
    }
    Ok(())
}

async fn cmd_screen(
    pillar: Option<String>,
    universe: String,
    min_fitness: f64,
    workers: usize,
    no_tune: bool,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — 9-Worker Distributed Screener"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let screener = MultiAccountScreener::from_env(workers)?;
    let factor_pillar = pillar.as_deref().and_then(FactorPillar::from_str);

    let candidates = get_curated_candidates(factor_pillar, Some(&universe));
    println!(
        "  Loaded {} orthogonal candidate hypotheses for screening.",
        candidates.len()
    );

    let qualified = screener
        .screen_batch(candidates, min_fitness, !no_tune)
        .await?;

    println!(
        "\n{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "  🎉 Screening Session Complete! {} Alphas qualified 8/8 PASS.",
        qualified.len()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    for q in qualified {
        println!(
            "    🌟 [{}] {} | Sharpe: {:.2} | Fit: {:.2} | Return: {:.1}%",
            q.alpha_id.green().bold(),
            q.name,
            q.sharpe,
            q.fitness,
            q.annual_return
        );
    }
    Ok(())
}

async fn cmd_sim(
    expr: String,
    universe: String,
    decay: i32,
    neutralization: String,
    truncation: f64,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Direct FastExpr Simulation"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    let settings = AlphaSettings {
        universe: universe.clone(),
        decay,
        neutralization: neutralization.clone(),
        truncation,
        ..Default::default()
    };

    println!(
        "  Settings: Universe: {} | Decay: {} | Neutralization: {} | Truncation: {}",
        universe.yellow(),
        decay,
        neutralization.yellow(),
        truncation
    );
    println!("  Expression: {}\n", expr.cyan());

    println!("  Submitting simulation to MAIN account...");
    let sim_id = client.submit_simulation(&expr, &settings).await?;
    println!(
        "  Simulation queued (ID: {}). Polling progress...",
        sim_id.yellow()
    );

    let sim_res = client.poll_simulation(&sim_id, 350, 5).await?;
    if let Some(alpha_id) = sim_res.alpha {
        println!(
            "  {} Simulation Complete! Alpha ID: {}",
            "✅".green(),
            alpha_id.green().bold()
        );
        let details = client.get_alpha_details(&alpha_id).await?;
        if let Some(st) = details.is {
            println!("\n  📊 In-Sample Statistics:");
            println!("    Sharpe:      {:.2}", st.sharpe.unwrap_or(0.0));
            println!("    Fitness:     {:.2}", st.fitness.unwrap_or(0.0));
            println!("    Annual Ret:  {:.2}%", st.returns.unwrap_or(0.0) * 100.0);
            println!(
                "    Turnover:    {:.2}%",
                st.turnover.unwrap_or(0.0) * 100.0
            );
            println!(
                "    Margin:      {:.2} bps",
                st.margin.unwrap_or(0.0) * 10000.0
            );
            println!(
                "    Drawdown:    {:.2}%",
                st.drawdown.unwrap_or(0.0) * 100.0
            );
        }
        println!(
            "\n  Platform Link: https://platform.worldquantbrain.com/alpha/{}",
            alpha_id.cyan()
        );
    } else {
        println!("  {} No Alpha ID returned from simulation.", "⚠️".yellow());
    }
    Ok(())
}
