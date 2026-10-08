use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use alphafind::autotuner::FitnessAutoTuner;
use alphafind::client::BrainClient;
use alphafind::correlation::{audit_candidate, calculate_portfolio_impact, simulate_portfolio};
use alphafind::crowd::{evaluate_crowd_risk, print_crowd_risk_audit};
use alphafind::matrix::{
    analyze_formula_hyper_synergy, build_dynamic_matrix, print_dynamic_matrix,
};
use alphafind::models::{AlphaSettings, CensusSnapshot, DatasetEntry, PortfolioAlpha};
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

    /// View live user status, rank, and official BRAIN leaderboard scores
    Score,

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

    /// Live Dynamic Crowd Census Radar querying all 150 datasets to track crowd migration
    Radar {
        /// Target Region (default: USA)
        #[arg(short, long, default_value = "USA")]
        region: String,

        /// Filter by Universe (e.g. TOP3000, TOP1000, TOP500, TOPSP500, TOP200)
        #[arg(short, long)]
        universe: Option<String>,

        /// Maximum results to show (default: 15)
        #[arg(short, long, default_value_t = 15)]
        top: usize,

        /// Keyword filter (e.g. news, short, vwap, analyst, fundamental, option)
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter only Green Sanctuaries (userCount < 300)
        #[arg(long)]
        green_only: bool,
    },

    /// Hyperparameter AutoTuner: multi-dimensional sweep (decay, exponent, neutralization) to maximize Fitness >= 1.50
    Tune {
        /// FastExpr alpha formula
        #[arg(short, long)]
        expr: String,

        /// Target Universe (default: TOP1000)
        #[arg(short, long, default_value = "TOP1000")]
        universe: String,

        /// Target Fitness to achieve (default: 1.50)
        #[arg(short, long, default_value_t = 1.50)]
        target_fitness: f64,

        /// Base decay parameter (default: 10)
        #[arg(short, long, default_value_t = 10)]
        decay: i32,

        /// Base neutralization group (default: SUBINDUSTRY)
        #[arg(short, long, default_value = "SUBINDUSTRY")]
        neutralization: String,
    },

    /// Simulate exact portfolio merge impact (Delta Sharpe, PnL, Vol, Correlation) for a candidate Alpha
    Impact {
        /// Candidate Alpha ID (e.g., vR2lzJor)
        alpha_id: String,
    },

    /// 2D Cross-Dataset Co-occurrence Matrix & Multi-Dataset Hyper-Synergy Analyzer (K >= 2)
    Matrix {
        /// Target Universe (default: TOP200)
        #[arg(short, long, default_value = "TOP200")]
        universe: String,

        /// Optional FastExpr formula to evaluate its K >= 2 multi-dataset synergy
        #[arg(short, long)]
        expr: Option<String>,

        /// Filter only pristine sanctuary interactions (HUI >= 0.65)
        #[arg(long)]
        green_only: bool,

        /// Target Region (default: USA)
        #[arg(short, long, default_value = "USA")]
        region: String,
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
        Commands::Score => cmd_score().await,
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
        Commands::Radar {
            region,
            universe,
            top,
            filter,
            green_only,
        } => cmd_radar(region, universe, top, filter, green_only).await,
        Commands::Tune {
            expr,
            universe,
            target_fitness,
            decay,
            neutralization,
        } => cmd_tune(expr, universe, target_fitness, decay, neutralization).await,
        Commands::Impact { alpha_id } => cmd_impact(alpha_id).await,
        Commands::Matrix {
            universe,
            expr,
            green_only,
            region,
        } => cmd_matrix(universe, expr, green_only, region).await,
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

    if let Ok(competitions) = client.fetch_competitions().await {
        for comp in &competitions {
            if let Some(lb) = &comp.leaderboard {
                println!(
                    "\n  {} Live Leaderboard Status ({}):",
                    "🏆".bold(),
                    comp.name.yellow()
                );
                if let Some(r) = lb.rank {
                    println!(
                        "    Rank: #{} | Score: {:.2} | isScore: {:.1} | Uniqueness: {:.2} | Days: {}/60",
                        r.to_string().bold().green(),
                        lb.score.unwrap_or(0.0),
                        lb.is_score.unwrap_or(0.0),
                        lb.uniqueness_score.unwrap_or(0.0),
                        lb.days_of_submission.unwrap_or(0)
                    );
                }
            }
        }
    }

    Ok(())
}

async fn cmd_score() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — WorldQuant BRAIN Live Leaderboard & Status"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!("  Connecting to MAIN account: {}...", client.email().cyan());

    let user_profile = client.fetch_user_profile().await.ok();
    if let Some(ref profile) = user_profile {
        let name_str = profile.full_name.as_deref().unwrap_or("Quant Researcher");
        let level_str = profile.level.as_deref().unwrap_or("MEMBER");
        println!(
            "  👤 User: {} ({}) | Email: {} | Tier: {}",
            name_str.bold().white(),
            profile.id.yellow(),
            profile.email.cyan(),
            level_str.yellow().bold()
        );
    }

    let competitions = client.fetch_competitions().await?;
    let mut found_board = false;

    for comp in &competitions {
        if let Some(lb) = &comp.leaderboard {
            found_board = true;
            println!(
                "\n  🏆 Competition: {} [{}]",
                comp.name.bold().yellow(),
                comp.id.cyan()
            );
            println!("  ─────────────────────────────────────────────────────────────────────────");
            if let Some(r) = lb.rank {
                let rank_str = format!("#{}", r);
                let rank_display = if r == 1 {
                    rank_str.bold().green()
                } else if r <= 3 {
                    rank_str.bold().yellow()
                } else {
                    rank_str.bold().white()
                };
                println!("    Leaderboard Rank:          {}", rank_display);
            }
            if let Some(sc) = lb.score {
                println!(
                    "    Total Leaderboard Score:   {}",
                    format!("{:.2}", sc).bold().green()
                );
            }
            if let Some(is) = lb.is_score {
                println!(
                    "    In-Sample Score (isScore): {}",
                    format!("{:.1}", is).bold().yellow()
                );
            }
            if let Some(uniq) = lb.uniqueness_score {
                println!(
                    "    Uniqueness Score:          {}",
                    format!("{:.2}", uniq).cyan()
                );
            }
            if let Some(days) = lb.days_of_submission {
                println!(
                    "    Days of Submission:        {} / 60",
                    days.to_string().cyan()
                );
            }
            if let Some(uni) = &lb.university {
                println!("    Affiliation / University:  {}", uni.white());
            }
            println!("  ─────────────────────────────────────────────────────────────────────────");

            // Fetch Top Podium Competitors
            if let Ok(board) = client.fetch_competition_leaderboard(&comp.id, 5).await {
                println!(
                    "\n  🏅 National Podium Standings (Top {} / {} Competitors):",
                    board.results.len().min(5),
                    board.count
                );
                println!("  ┌──────┬────────────────────────┬─────────┬───────────┬────────────┬──────┬────────────────────────────┐");
                println!("  │ Rank │ Competitor             │ Score   │ isScore   │ Uniqueness │ Days │ University                 │");
                println!("  ├──────┼────────────────────────┼─────────┼───────────┼────────────┼──────┼────────────────────────────┤");

                for entry in board.results.iter().take(5) {
                    let is_me = if let Some(ref prof) = user_profile {
                        entry.user.id() == prof.id
                    } else {
                        false
                    };

                    let rank_badge = match entry.rank {
                        1 => "🥇 #1".yellow().bold(),
                        2 => "🥈 #2".bright_white().bold(),
                        3 => "🥉 #3".bright_yellow().bold(),
                        r => format!("   #{}", r).normal(),
                    };

                    let comp_name = if is_me {
                        format!("{} (YOU)", entry.user.display_name())
                            .green()
                            .bold()
                    } else {
                        entry.user.display_name().normal()
                    };

                    let uni_short = entry.university.as_deref().unwrap_or("—");
                    let uni_display = if uni_short.len() > 26 {
                        format!("{}...", &uni_short[..23])
                    } else {
                        uni_short.to_string()
                    };

                    println!(
                        "  │ {:<4} │ {:<22} │ {:>7.2} │ {:>9.1} │ {:>10.2} │ {:>4} │ {:<26} │",
                        rank_badge,
                        comp_name,
                        entry.score,
                        entry.is_score,
                        entry.uniqueness_score,
                        entry.days_of_submission,
                        uni_display
                    );
                }
                println!("  └──────┴────────────────────────┴─────────┴───────────┴────────────┴──────┴────────────────────────────┘");
            }
        }
    }

    if !found_board {
        println!("  ℹ️  No active competition leaderboard entry found.");
    }

    // Also display portfolio summary if portfolio_os.json exists
    let port_path = "portfolio_os.json";
    if Path::new(port_path).exists() {
        if let Ok(port_data) = fs::read_to_string(port_path) {
            if let Ok(os_alphas) = serde_json::from_str::<Vec<PortfolioAlpha>>(&port_data) {
                println!(
                    "\n  📊 Active OS Portfolio: {} Alphas tracked in {}",
                    os_alphas.len().to_string().bold().green(),
                    json_path_display(port_path)
                );
            }
        }
    }

    Ok(())
}

fn json_path_display(p: &str) -> colored::ColoredString {
    p.bold()
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
        "\n  📊 TIER 1: INTERNAL PORTFOLIO CORRELATION AUDIT (vs {} OS Alphas):",
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
        "  MAX INTERNAL CORRELATION: {:+.4} ({:+.2}%) vs {}",
        report.max_correlation,
        report.max_correlation * 100.0,
        report.most_correlated_id.yellow()
    );
    println!(
        "  AVERAGE INTERNAL CORR:   {:+.4} ({:+.2}%)",
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

    // Tier 2: Platform Crowd & Uniqueness Risk Estimator
    if let Ok(details) = client.get_alpha_details(&alpha_id).await {
        let u = details
            .settings
            .as_ref()
            .map(|s| s.universe.as_str())
            .unwrap_or("TOP1000");
        let neut = details
            .settings
            .as_ref()
            .map(|s| s.neutralization.as_str())
            .unwrap_or("SUBINDUSTRY");
        let code = details.get_code().unwrap_or("");
        let crowd_report = evaluate_crowd_risk(code, u, neut, None);
        print_crowd_risk_audit(&crowd_report);
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

async fn cmd_radar(
    region: String,
    universe_filter: Option<String>,
    top: usize,
    filter: Option<String>,
    green_only: bool,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Live Dynamic Crowd Census Radar (2D Matrix)"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!(
        "  Connecting to WorldQuant BRAIN API (Region: {})...",
        region.yellow()
    );

    let datasets = client.fetch_all_datasets(&region).await?;
    println!(
        "  {} Fetched {} datasets from WorldQuant BRAIN Production API.",
        "✅".green(),
        datasets.len().to_string().bold().green()
    );

    // Load previous snapshot if exists for delta tracking
    let data_dir = Path::new("data");
    let _ = fs::create_dir_all(data_dir);
    let snapshot_file = data_dir.join("census_snapshots.json");

    let prev_snapshot: Option<CensusSnapshot> = if snapshot_file.exists() {
        fs::read_to_string(&snapshot_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    };

    let mut prev_map: HashMap<String, (i64, i64)> = HashMap::new();
    if let Some(ref prev) = prev_snapshot {
        println!(
            "  Loaded previous snapshot from {}. Calculating crowd migration velocity...",
            prev.timestamp.cyan()
        );
        for d in &prev.datasets {
            let sub_name = d
                .subcategory
                .as_ref()
                .and_then(|s| s.name.as_deref())
                .unwrap_or("");
            let u_name = d.universe.as_deref().unwrap_or("");
            let key = format!("{}:{}:{}", d.id, u_name, sub_name);
            prev_map.insert(key, (d.user_count.unwrap_or(0), d.alpha_count.unwrap_or(0)));
        }
    }

    // Save current snapshot
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let current_snapshot = CensusSnapshot {
        timestamp: format!("Unix Epoch {}", secs),
        region: region.clone(),
        total_datasets: datasets.len(),
        datasets: datasets.clone(),
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&current_snapshot) {
        let _ = fs::write(&snapshot_file, json_str);
    }

    // Filter datasets
    let mut filtered: Vec<&DatasetEntry> = datasets.iter().collect();

    // Filter by Universe if provided
    if let Some(ref uf) = universe_filter {
        let uf_upper = uf.to_uppercase();
        filtered.retain(|d| {
            d.universe
                .as_deref()
                .map(|u| u.to_uppercase().contains(&uf_upper))
                .unwrap_or(false)
        });
        println!(
            "  Filtered by Universe: {} ({} matching dataset entries)",
            uf.cyan().bold(),
            filtered.len().to_string().yellow()
        );
    }

    // Keyword filter
    if let Some(ref f) = filter {
        let f_lower = f.to_lowercase();
        filtered.retain(|d| {
            d.id.to_lowercase().contains(&f_lower)
                || d.name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&f_lower)
                || d.subcategory
                    .as_ref()
                    .and_then(|s| s.name.as_deref())
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&f_lower)
        });
    }

    if green_only {
        filtered.retain(|d| d.user_count.unwrap_or(0) < 300);
    }

    // Sort Green Sanctuaries (lowest userCount first)
    let mut green_sanctuaries = filtered.clone();
    green_sanctuaries.sort_by_key(|d| d.user_count.unwrap_or(0));

    println!("\n  🟢 GREEN SANCTUARIES (Uncrowded / High Uniqueness Potential):");
    println!("  ┌──────────────┬──────────┬───────────────────────────┬──────────────┬──────────────┬──────────┬────────────┐");
    println!("  │ Dataset ID   │ Universe │ Subcategory               │ Users (Live) │ Alphas (Live)│ Coverage │ Status     │");
    println!("  ├──────────────┼──────────┼───────────────────────────┼──────────────┼──────────────┼──────────┼────────────┤");

    for d in green_sanctuaries.iter().take(top) {
        let u_cnt = d.user_count.unwrap_or(0);
        let a_cnt = d.alpha_count.unwrap_or(0);
        let u_name = d.universe.as_deref().unwrap_or("—");
        let cov_str = d
            .coverage
            .map(|c| format!("{:.1}%", c * 100.0))
            .unwrap_or_else(|| "—".to_string());
        let subcat = d
            .subcategory
            .as_ref()
            .and_then(|s| s.name.as_deref())
            .unwrap_or("General");
        let sub_display = if subcat.len() > 25 {
            format!("{}...", &subcat[..22])
        } else {
            subcat.to_string()
        };

        let status = if u_cnt <= 50 {
            "PRISTINE".green().bold()
        } else if u_cnt <= 300 {
            "SAFE".green()
        } else {
            "MODERATE".yellow()
        };

        let key = format!("{}:{}:{}", d.id, u_name, subcat);
        let delta_str = if let Some(&(p_u, _)) = prev_map.get(&key) {
            let du = u_cnt - p_u;
            if du > 0 {
                format!(" (+{})", du).yellow().to_string()
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        let u_str = format!("{}{}", u_cnt, delta_str);

        println!(
            "  │ {:<12} │ {:<8} │ {:<25} │ {:>12} │ {:>12} │ {:>8} │ {:<10} │",
            d.id.cyan(),
            u_name.white(),
            sub_display,
            u_str,
            a_cnt,
            cov_str,
            status
        );
    }
    println!("  └──────────────┴──────────┴───────────────────────────┴──────────────┴──────────────┴──────────┴────────────┘");

    // Display Top Red Zones if not green_only
    if !green_only {
        let mut red_zones = filtered.clone();
        red_zones.sort_by_key(|d| std::cmp::Reverse(d.user_count.unwrap_or(0)));

        println!("\n  🔴 HIGH-CROWD RED ZONES (Crowded / Diluted Uniqueness - AVOID):");
        println!("  ┌──────────────┬──────────┬───────────────────────────┬──────────────┬──────────────┬──────────┬────────────┐");
        println!("  │ Dataset ID   │ Universe │ Subcategory               │ Users (Live) │ Alphas (Live)│ Coverage │ Status     │");
        println!("  ├──────────────┼──────────┼───────────────────────────┼──────────────┼──────────────┼──────────┼────────────┤");

        for d in red_zones.iter().take(top.min(8)) {
            let u_cnt = d.user_count.unwrap_or(0);
            let a_cnt = d.alpha_count.unwrap_or(0);
            let u_name = d.universe.as_deref().unwrap_or("—");
            let cov_str = d
                .coverage
                .map(|c| format!("{:.1}%", c * 100.0))
                .unwrap_or_else(|| "—".to_string());
            let subcat = d
                .subcategory
                .as_ref()
                .and_then(|s| s.name.as_deref())
                .unwrap_or("General");
            let sub_display = if subcat.len() > 25 {
                format!("{}...", &subcat[..22])
            } else {
                subcat.to_string()
            };

            let status = if u_cnt >= 20000 {
                "DANGER".red().bold()
            } else if u_cnt >= 2000 {
                "CROWDED".red()
            } else {
                "MODERATE".yellow()
            };

            println!(
                "  │ {:<12} │ {:<8} │ {:<25} │ {:>12} │ {:>12} │ {:>8} │ {:<10} │",
                d.id.yellow(),
                u_name.white(),
                sub_display,
                u_cnt,
                a_cnt,
                cov_str,
                status
            );
        }
        println!("  └──────────────┴──────────┴───────────────────────────┴──────────────┴──────────────┴──────────┴────────────┘");
    }

    Ok(())
}

async fn cmd_tune(
    expr: String,
    universe: String,
    target_fitness: f64,
    decay: i32,
    neutralization: String,
) -> Result<()> {
    let client = get_main_client()?;
    let settings = AlphaSettings {
        universe,
        decay,
        neutralization,
        truncation: 0.065,
        ..Default::default()
    };
    let tuner = FitnessAutoTuner::new(&client);
    let res = tuner
        .tune_hyperparameters(&expr, &settings, target_fitness)
        .await?;

    match res {
        Some(tuned) => {
            let st = tuned.details.is.as_ref().unwrap();
            println!("\n  🎉 TUNING SUCCESS!");
            println!(
                "    Tuned Strategy: {}",
                tuned.tuning_strategy.green().bold()
            );
            println!("    Alpha ID:       {}", tuned.details.id.yellow().bold());
            println!(
                "    Fitness:        {:.2} -> {:.2}",
                tuned.original_fitness, tuned.tuned_fitness
            );
            println!("    Sharpe:         {:.2}", st.sharpe.unwrap_or(0.0));
            println!(
                "    Turnover:       {:.1}%",
                st.turnover.unwrap_or(0.0) * 100.0
            );
            println!(
                "    Settings:       Decay={}, Neut={}",
                tuned.settings.decay, tuned.settings.neutralization
            );
            println!("    Formula:        {}", tuned.expression.cyan());
            println!(
                "    URL:            https://platform.worldquantbrain.com/alpha/{}",
                tuned.details.id
            );
        }
        None => {
            println!(
                "\n  ⚠️ AutoTuner completed: Could not achieve target fitness >= {:.2}.",
                target_fitness
            );
        }
    }
    Ok(())
}

async fn cmd_impact(alpha_id: String) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        format!(
            "  AlphaFind Quant Engine — Portfolio Delta Impact [{}]",
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
    println!(
        "  Fetching candidate daily PnL for {}...",
        alpha_id.yellow()
    );
    let cand_pnl = client.fetch_daily_pnl(&alpha_id).await?;
    println!("  Candidate PnL records: {} trading days.", cand_pnl.len());

    // Load active OS alphas PnL cache
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    if !cache_file.exists() {
        anyhow::bail!("portfolio_pnl.json not found! Run 'alphafind portfolio' first.");
    }
    let pnl_data = fs::read_to_string(&cache_file)?;
    let pnl_cache: HashMap<String, HashMap<String, f64>> = serde_json::from_str(&pnl_data)?;

    let impact = calculate_portfolio_impact(&alpha_id, &cand_pnl, &pnl_cache)
        .context("Failed to compute portfolio merge impact (insufficient overlapping days)")?;

    println!("\n  📊 PORTFOLIO MERGE SIMULATION IMPACT (1,236 Days):");
    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!(
        "    Active Alphas Count:       {} -> {} (+1 Alpha)",
        impact.baseline_alphas, impact.new_alphas
    );

    let d_sharpe_styled = if impact.delta_sharpe >= 0.0 {
        format!("+{:.4}", impact.delta_sharpe)
            .green()
            .bold()
            .to_string()
    } else {
        format!("{:.4}", impact.delta_sharpe)
            .red()
            .bold()
            .to_string()
    };
    println!(
        "    Merged Portfolio Sharpe:   {:.4} -> {:.4} (Delta: {})",
        impact.baseline_sharpe, impact.new_sharpe, d_sharpe_styled
    );

    let d_pnl_styled = if impact.delta_pnl >= 0.0 {
        format!("+${:.2}", impact.delta_pnl).green().to_string()
    } else {
        format!("-${:.2}", impact.delta_pnl.abs()).red().to_string()
    };
    println!(
        "    Annualized Merged PnL:     ${:.2} -> ${:.2} (Delta: {})",
        impact.baseline_pnl, impact.new_pnl, d_pnl_styled
    );

    let d_vol_styled = if impact.delta_vol <= 0.0 {
        format!("-${:.2} (Variance Collapsed)", impact.delta_vol.abs())
            .green()
            .to_string()
    } else {
        format!("+${:.2}", impact.delta_vol).yellow().to_string()
    };
    println!(
        "    Annualized Volatility:     ${:.2} -> ${:.2} (Delta: {})",
        impact.baseline_vol, impact.new_vol, d_vol_styled
    );

    let d_corr_styled = if impact.delta_avg_corr <= 0.0 {
        format!("{:.2}%", impact.delta_avg_corr * 100.0)
            .green()
            .to_string()
    } else {
        format!("+{:.2}%", impact.delta_avg_corr * 100.0)
            .yellow()
            .to_string()
    };
    println!(
        "    Average Pairwise Corr:     {:.2}% -> {:.2}% (Delta: {})",
        impact.baseline_avg_corr * 100.0,
        impact.new_avg_corr * 100.0,
        d_corr_styled
    );

    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!(
        "    Max Pairwise Correlation:  {:+.2}% vs {}",
        impact.max_pairwise_corr * 100.0,
        impact.most_correlated_id.yellow()
    );
    println!(
        "    Direct Avg Corr vs OS:     {:+.2}%",
        impact.avg_pairwise_corr_vs_os * 100.0
    );

    let buffer_styled = if impact.safety_buffer_pct >= 10.0 {
        format!("{:.2}% (HIGH SAFETY)", impact.safety_buffer_pct)
            .green()
            .bold()
            .to_string()
    } else if impact.safety_buffer_pct > 0.0 {
        format!("{:.2}% (TIGHT SAFETY)", impact.safety_buffer_pct)
            .yellow()
            .bold()
            .to_string()
    } else {
        format!("{:.2}% (VIOLATION)", impact.safety_buffer_pct)
            .red()
            .bold()
            .to_string()
    };
    println!("    Safety Buffer to 70% Limit: {}", buffer_styled);
    println!("  ─────────────────────────────────────────────────────────────────────────");

    if impact.max_pairwise_corr > 0.70 {
        println!(
            "\n  {} REJECTED: Candidate violates 70% self-correlation ceiling.",
            "❌ [UNSUBMITTABLE]".red().bold()
        );
    } else if impact.delta_sharpe > 0.0 && impact.safety_buffer_pct >= 8.0 {
        println!(
            "\n  {} HIGH CONVICTION: Boosts portfolio Sharpe and preserves safety buffer!",
            "🌟 [RECOMMENDED]".green().bold()
        );
    } else if impact.delta_sharpe > 0.0 {
        println!(
            "\n  {} ACCEPTABLE: Boosts portfolio Sharpe but safety buffer is narrow.",
            "🟡 [CAUTION]".yellow().bold()
        );
    } else {
        println!(
            "\n  {} DILUTIVE: Candidate reduces portfolio Sharpe.",
            "⚠️ [SUBOPTIMAL]".yellow()
        );
    }

    Ok(())
}

async fn cmd_matrix(
    universe: String,
    expr: Option<String>,
    green_only: bool,
    region: String,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — 2D Dynamic Cross-Dataset Co-occurrence Matrix"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!(
        "  Fetching dynamic platform dataset census from BRAIN API (Region: {})...",
        region.cyan()
    );

    let datasets = match client.fetch_all_datasets(&region).await {
        Ok(ds) => ds,
        Err(e) => {
            println!(
                "  ⚠️ Warning: Live API fetch failed ({}), falling back to cached snapshot.",
                e
            );
            let snapshot_file = Path::new("data").join("census_snapshots.json");
            if let Ok(content) = fs::read_to_string(&snapshot_file) {
                if let Ok(snap) = serde_json::from_str::<CensusSnapshot>(&content) {
                    snap.datasets
                } else {
                    anyhow::bail!("No cached census snapshot available");
                }
            } else {
                anyhow::bail!("No cached census snapshot available");
            }
        }
    };

    println!(
        "  {} Analyzed {} platform dataset configurations.",
        "✅".green(),
        datasets.len().to_string().bold().green()
    );

    let data_dir = Path::new("data");
    let snapshot_file = data_dir.join("census_snapshots.json");
    let prev_snapshot: Option<CensusSnapshot> = if snapshot_file.exists() {
        fs::read_to_string(&snapshot_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    };

    let report = build_dynamic_matrix(&datasets, prev_snapshot.as_ref(), &universe);
    print_dynamic_matrix(&report, green_only);

    if let Some(ref formula) = expr {
        println!(
            "\n  {}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "  {}",
            "  Multi-Dataset Formula Hyper-Synergy Analysis (K >= 2)"
                .bold()
                .cyan()
        );
        println!(
            "  {}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );

        let analysis =
            analyze_formula_hyper_synergy(formula, &universe, &datasets, prev_snapshot.as_ref());

        println!("\n  {} Formula Architecture Assessment:", "🧬".bold());
        println!(
            "    Target Universe:            {}",
            analysis.universe.yellow().bold()
        );
        println!(
            "    Detected Datasets Count:    {} Datasets",
            analysis.detected_k.to_string().cyan().bold()
        );
        println!(
            "    Fusion Architecture Pattern: {}",
            analysis.fusion_pattern.badge()
        );

        println!(
            "\n  {} Participating Factor Pillars Breakdown (K = {}):",
            "🏛️".bold(),
            analysis.detected_k
        );
        println!("  ┌────────┬──────────────────────────────────────────┬──────────────┬──────────────┬────────────┐");
        println!("  │ Pillar │ Economic Architecture Role               │ Live Quants  │ Live Alphas  │ Crowd Tier │");
        println!("  ├────────┼──────────────────────────────────────────┼──────────────┼──────────────┼────────────┤");

        for (pillar, role, stat) in &analysis.participating_pillars {
            println!(
                "  │ {:<6} │ {:<40} │ {:>12} │ {:>12} │ {:<10} │",
                pillar.cyan().bold(),
                role.badge(),
                stat.live_users,
                stat.live_alphas,
                stat.status
            );
        }
        println!("  └────────┴──────────────────────────────────────────┴──────────────┴──────────────┴────────────┘");

        let chui_styled = if analysis.composite_uniqueness_index >= 0.85 {
            format!("{:.3}", analysis.composite_uniqueness_index)
                .green()
                .bold()
        } else if analysis.composite_uniqueness_index >= 0.70 {
            format!("{:.3}", analysis.composite_uniqueness_index).green()
        } else if analysis.composite_uniqueness_index >= 0.50 {
            format!("{:.3}", analysis.composite_uniqueness_index).yellow()
        } else {
            format!("{:.3}", analysis.composite_uniqueness_index)
                .red()
                .bold()
        };

        println!(
            "\n  {} Dynamic Composite Hybrid Uniqueness Index (CHUI): {}",
            "🎯".bold(),
            chui_styled
        );
        println!(
            "  {} Platform Uniqueness Verdict: {}",
            "⚖️".bold(),
            analysis.uniqueness_verdict
        );

        if !analysis.dynamic_migration_warnings.is_empty() {
            println!(
                "\n  {} Dynamic Crowd Migration Warnings:",
                "🚨".red().bold()
            );
            for w in &analysis.dynamic_migration_warnings {
                println!("    {}", w.yellow());
            }
        }

        if !analysis.economic_recommendations.is_empty() {
            println!("\n  {} Economic Architecture Recommendations:", "💡".bold());
            for r in &analysis.economic_recommendations {
                println!("    👉 {}", r.green());
            }
        }
    } else {
        println!(
            "\n  {} Top Pristine Multi-Dataset Synergy Pathways for {}:",
            "🌟".bold(),
            universe.yellow().bold()
        );
        println!(
            "    1. RISK x SHORT x MICRO  -> (model51 Anchor x Short Squeeze Catalyst x VWAP Dislocation)"
        );
        println!(
            "    2. NEWS18 x SHORT x MICRO -> (RavenPack Novelty x Borrow Squeeze x Reversal Dampening)"
        );
        println!(
            "    3. RISK x ANL x MICRO    -> (Idiosyncratic Risk x Consensus Revision Drift x Liquidity Shock)"
        );
        println!(
            "\n  💡 Tip: Run with --expr \"<fastexpr>\" to evaluate any multi-dataset alpha formula!"
        );
    }

    Ok(())
}
