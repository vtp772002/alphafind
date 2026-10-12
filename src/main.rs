use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use alphafind::autotuner::FitnessAutoTuner;
use alphafind::client::BrainClient;
use alphafind::correlation::{
    audit_candidate, calculate_portfolio_impact, pearson_correlation, simulate_portfolio,
};
use alphafind::crowd_allocation::{
    build_formula_allocation, cache_field, current_unix_time, field_cache_key, find_dataset_config,
    get_fresh_cached_field, load_local_field_datasets, FieldConfig, FieldUsageCache,
};
use alphafind::crowd_model::{
    build_usage_weighted_profile_pnl, equal_weight_pnl_merge, project_proxy_correlation,
};
use alphafind::forecast::{generate_inverse_blueprint, print_blueprint_report};
use alphafind::health::{
    evaluate_portfolio_health, print_detailed_alpha_table, print_health_report,
    print_single_alpha_diagnostic,
};
use alphafind::matrix::{
    analyze_formula_hyper_synergy, build_dynamic_matrix, print_dynamic_matrix,
};
use alphafind::models::{
    AlphaSettings, CensusSnapshot, DataFieldEntry, DatasetEntry, PortfolioAlpha,
};
use alphafind::screener::MultiAccountScreener;
use alphafind::taxonomy::{get_curated_candidates, FactorPillar};

type DatasetAllocationKey = (String, String, String, i32, String);
type ProfileDatasetFields = Vec<(DatasetAllocationKey, Vec<String>)>;
type AllocationSets = (
    std::collections::HashSet<String>,
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
);

fn derive_profile_usage_weights(
    alpha_datasets: &HashMap<String, ProfileDatasetFields>,
    field_cache: &FieldUsageCache,
    dataset_counts: &HashMap<DatasetAllocationKey, f64>,
    now: u64,
) -> (HashMap<String, f64>, usize, usize) {
    let mut weights = HashMap::new();
    let mut field_weighted_alphas = 0;
    let mut dataset_fallback_alphas = 0;

    for (alpha_id, exposures) in alpha_datasets {
        let mut exposure_weights = Vec::new();
        let mut used_field_counts = false;
        for (key, fields) in exposures {
            let config = FieldConfig {
                instrument_type: key.0.clone(),
                region: key.1.clone(),
                universe: key.2.clone(),
                delay: key.3,
            };
            let field_counts: Vec<f64> = fields
                .iter()
                .filter_map(|field_id| {
                    let cache_key = field_cache_key(field_id, &config);
                    get_fresh_cached_field(field_cache, &cache_key, now)
                        .and_then(|field| field.alpha_count)
                        .filter(|count| *count > 0)
                        .map(|count| count as f64)
                })
                .collect();

            if !field_counts.is_empty() {
                used_field_counts = true;
                exposure_weights.push(field_counts.iter().sum::<f64>() / field_counts.len() as f64);
            } else if let Some(count) = dataset_counts.get(key).filter(|count| **count > 0.0) {
                exposure_weights.push(*count);
            }
        }

        if !exposure_weights.is_empty() {
            let weight = exposure_weights.iter().sum::<f64>() / exposure_weights.len() as f64;
            if weight.is_finite() && weight > 0.0 {
                weights.insert(alpha_id.clone(), weight);
                if used_field_counts {
                    field_weighted_alphas += 1;
                } else {
                    dataset_fallback_alphas += 1;
                }
            }
        }
    }

    (weights, field_weighted_alphas, dataset_fallback_alphas)
}

fn challenge_uniqueness_score(
    client_competitions: &[alphafind::models::CompetitionEntry],
) -> Option<f64> {
    client_competitions
        .iter()
        .find(|competition| {
            let name = competition.name.to_ascii_lowercase();
            name.contains("challenge") && !name.contains("iqc")
        })
        .and_then(|competition| competition.leaderboard.as_ref())
        .and_then(|leaderboard| leaderboard.uniqueness_score)
}

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

    /// Audit Pearson correlation over overlapping local PnL dates against active OS Alphas
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

    /// Screen candidates using configured BRAIN sessions (concurrency is not bounded)
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

        /// Displayed workers per session (default: 3; does not bound task concurrency)
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

    /// Fetch and display the live dataset usage census
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

        /// Keep datasets with a reported userCount below 300 (local usage filter)
        #[arg(long)]
        green_only: bool,
    },

    /// Sweep configured decay, exponent, and neutralization values for a candidate
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

    /// Configured dataset-interaction heuristic and multi-dataset formula analysis
    Matrix {
        /// Target Universe (default: TOP200)
        #[arg(short, long, default_value = "TOP200")]
        universe: String,

        /// Optional FastExpr formula to evaluate its K >= 2 multi-dataset synergy
        #[arg(short, long)]
        expr: Option<String>,

        /// Keep only configured-affinity pairs with score >= 0.65
        #[arg(long)]
        green_only: bool,

        /// Target Region (default: USA)
        #[arg(short, long, default_value = "USA")]
        region: String,
    },

    /// Generate rule-based portfolio gap and candidate suggestions
    Blueprint {
        /// Target Universe filter (optional: TOP3000, TOP1000, TOP500, TOP200)
        #[arg(short, long)]
        universe: Option<String>,

        /// Export generated FastExpr candidate templates to a file
        #[arg(long)]
        export: Option<String>,
    },

    /// Inspect data usage and local profile PnL sensitivity; does not forecast Uniqueness
    Shadow {
        /// Inspect a candidate Alpha's saved formula and settings
        #[arg(short, long, conflicts_with = "expr")]
        candidate: Option<String>,

        /// Inspect a private Fast Expression directly (kept in memory only)
        #[arg(short, long, conflicts_with = "candidate")]
        expr: Option<String>,

        /// Region for --expr (default: USA)
        #[arg(short, long)]
        region: Option<String>,

        /// Universe for --expr (default: TOP1000)
        #[arg(short, long)]
        universe: Option<String>,

        /// Delay for --expr (default: 1)
        #[arg(long)]
        delay: Option<i32>,

        /// Simulate --expr once on the main account to obtain candidate PnL for the estimate
        #[arg(long, requires = "expr")]
        simulate: bool,
    },

    /// Diagnose active OS portfolio health, alpha decay ratios, half-life, and marginal Sharpe impact
    Health {
        /// Optional specific Alpha ID to inspect in depth
        #[arg(short, long)]
        alpha_id: Option<String>,

        /// Export full diagnostic JSON report to a file
        #[arg(long)]
        export: Option<String>,

        /// Show full table of all active alphas
        #[arg(short, long)]
        all: bool,
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
        Commands::Blueprint { universe, export } => cmd_blueprint(universe, export).await,
        Commands::Shadow {
            candidate,
            expr,
            region,
            universe,
            delay,
            simulate,
        } => cmd_shadow(candidate, expr, region, universe, delay, simulate).await,
        Commands::Health {
            alpha_id,
            export,
            all,
        } => cmd_health(alpha_id, export, all).await,
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
            "  AlphaFind Quant Engine — Local Overlapping-Date Pearson Audit [{}]",
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

    println!(
        "\n  Dataset/field usage is available via `alphafind shadow`; it measures participation, not official Uniqueness."
    );

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
        "  AlphaFind Quant Engine — Configured-Session Screener"
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
        "  Loaded {} candidate hypotheses for screening.",
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
        "  Screening Session Complete: {} candidates marked qualified by local checks. Verify each result in BRAIN before submission.",
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
        "  AlphaFind — BRAIN Dataset Usage Census".bold().cyan()
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

    let mut prev_map: HashMap<String, (Option<i64>, Option<i64>)> = HashMap::new();
    if let Some(ref prev) = prev_snapshot {
        if prev.region.eq_ignore_ascii_case(&region) {
            println!(
                "  Loaded previous snapshot from {}. Showing count changes since that snapshot.",
                prev.timestamp.cyan()
            );
            for d in &prev.datasets {
                let key = dataset_census_key(d);
                prev_map.insert(key, (d.user_count, d.alpha_count));
            }
        } else {
            println!(
                "  Ignoring previous snapshot for region {}; current region is {}.",
                prev.region, region
            );
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
        filtered.retain(|d| d.user_count.is_some_and(|count| (0..300).contains(&count)));
    }

    // Sort reported lower user counts first; unknown and invalid counts go last.
    let mut lower_usage = filtered.clone();
    lower_usage.sort_by_key(|d| match d.user_count {
        Some(count) if count >= 0 => (0, count),
        _ => (1, i64::MAX),
    });

    println!(
        "\n  DATASET USAGE COUNTS (lower reported user counts first; not a Uniqueness forecast):"
    );
    println!("  ┌──────────────┬──────────┬───────────────────────────┬──────────────┬──────────────┬──────────┬────────────┐");
    println!("  │ Dataset ID   │ Universe │ Subcategory               │ Users (API)  │ Alphas (API) │ Coverage │ Status     │");
    println!("  ├──────────────┼──────────┼───────────────────────────┼──────────────┼──────────────┼──────────┼────────────┤");

    for d in lower_usage.iter().take(top) {
        let u_cnt = d.user_count;
        let a_cnt = d.alpha_count;
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

        let status = match u_cnt {
            Some(count) if (0..=50).contains(&count) => "LOW COUNT".green().bold(),
            Some(count) if (0..=300).contains(&count) => "LOW COUNT".green(),
            Some(count) if count >= 0 => "COUNT PRESENT".yellow(),
            _ => "UNKNOWN".dimmed(),
        };

        let key = dataset_census_key(d);
        let delta_str = match (u_cnt, prev_map.get(&key).and_then(|counts| counts.0)) {
            (Some(current), Some(previous)) if current != previous => {
                format!(" ({:+})", current - previous).yellow().to_string()
            }
            _ => String::new(),
        };

        let u_str = format!("{}{}", format_count(u_cnt), delta_str);

        println!(
            "  │ {:<12} │ {:<8} │ {:<25} │ {:>12} │ {:>12} │ {:>8} │ {:<10} │",
            d.id.cyan(),
            u_name.white(),
            sub_display,
            u_str,
            format_count(a_cnt),
            cov_str,
            status
        );
    }
    println!("  └──────────────┴──────────┴───────────────────────────┴──────────────┴──────────────┴──────────┴────────────┘");

    // Display Top Red Zones if not green_only
    if !green_only {
        let mut higher_usage = filtered.clone();
        higher_usage.sort_by_key(|d| match d.user_count {
            Some(count) if count >= 0 => (0, std::cmp::Reverse(count)),
            _ => (1, std::cmp::Reverse(0)),
        });

        println!("\n  HIGHER REPORTED USER COUNTS (usage only; not a PnL-correlation or Uniqueness score):");
        println!("  ┌──────────────┬──────────┬───────────────────────────┬──────────────┬──────────────┬──────────┬────────────┐");
        println!("  │ Dataset ID   │ Universe │ Subcategory               │ Users (API)  │ Alphas (API) │ Coverage │ Status     │");
        println!("  ├──────────────┼──────────┼───────────────────────────┼──────────────┼──────────────┼──────────┼────────────┤");

        for d in higher_usage.iter().take(top.min(8)) {
            let u_cnt = d.user_count;
            let a_cnt = d.alpha_count;
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

            let status = match u_cnt {
                Some(count) if count >= 20000 => "VERY HIGH COUNT".red().bold(),
                Some(count) if count >= 2000 => "HIGH COUNT".red(),
                Some(count) if count >= 0 => "COUNT PRESENT".yellow(),
                _ => "UNKNOWN".dimmed(),
            };

            println!(
                "  │ {:<12} │ {:<8} │ {:<25} │ {:>12} │ {:>12} │ {:>8} │ {:<10} │",
                d.id.yellow(),
                u_name.white(),
                sub_display,
                format_count(u_cnt),
                format_count(a_cnt),
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

    println!("\n  📊 PORTFOLIO MERGE SIMULATION IMPACT (overlapping PnL dates):");
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

fn profile_field_config(alpha: &PortfolioAlpha) -> FieldConfig {
    let settings = alpha.settings.as_ref();
    let string_setting = |camel: &str, snake: &str| {
        settings
            .and_then(|value| value.get(camel).or_else(|| value.get(snake)))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };
    let integer_setting = |camel: &str, snake: &str| {
        settings
            .and_then(|value| value.get(camel).or_else(|| value.get(snake)))
            .and_then(serde_json::Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
    };

    FieldConfig {
        instrument_type: string_setting("instrumentType", "instrument_type")
            .unwrap_or_else(|| "EQUITY".to_string()),
        region: string_setting("region", "region").unwrap_or_else(|| "USA".to_string()),
        universe: string_setting("universe", "universe")
            .or_else(|| alpha.universe.clone())
            .unwrap_or_else(|| "TOP1000".to_string()),
        delay: integer_setting("delay", "delay").unwrap_or(1),
    }
}

fn format_count(value: Option<i64>) -> String {
    value
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn field_config_label(config: &FieldConfig) -> String {
    format!(
        "{}/{}/{}/D{}",
        config.instrument_type, config.region, config.universe, config.delay
    )
}

fn dataset_census_key(dataset: &DatasetEntry) -> String {
    format!(
        "{}:{}:{}:{}",
        dataset.region.as_deref().unwrap_or("").to_ascii_uppercase(),
        dataset.id.to_ascii_lowercase(),
        dataset
            .universe
            .as_deref()
            .unwrap_or("")
            .to_ascii_uppercase(),
        dataset.delay.unwrap_or(-1)
    )
}

async fn cmd_shadow(
    candidate: Option<String>,
    expression: Option<String>,
    region: Option<String>,
    universe: Option<String>,
    delay: Option<i32>,
    simulate_expression: bool,
) -> Result<()> {
    if expression.is_some() && candidate.is_some() {
        anyhow::bail!("Use either --candidate or --expr, not both.");
    }
    if expression.is_none() && (region.is_some() || universe.is_some() || delay.is_some()) {
        anyhow::bail!("--region, --universe, and --delay apply only when using --expr.");
    }

    let client = get_main_client()?;
    let requested_candidate_id = candidate.clone();
    let candidate_source: Option<(String, FieldConfig)> = if let Some(alpha_id) = candidate.as_ref()
    {
        let details = client.get_alpha_details(alpha_id).await?;
        let code = details
            .get_code()
            .context("BRAIN did not return a formula for this Alpha")?
            .to_string();
        let settings = details.settings.unwrap_or_default();
        Some((
            code,
            FieldConfig {
                instrument_type: settings.instrument_type,
                region: settings.region,
                universe: settings.universe,
                delay: settings.delay,
            },
        ))
    } else {
        expression.map(|code| {
            (
                code,
                FieldConfig {
                    instrument_type: "EQUITY".to_string(),
                    region: region.unwrap_or_else(|| "USA".to_string()),
                    universe: universe.unwrap_or_else(|| "TOP1000".to_string()),
                    delay: delay.unwrap_or(1),
                },
            )
        })
    };

    let mut candidate_pnl: Option<HashMap<String, f64>> = None;
    let mut candidate_alpha_id = requested_candidate_id;
    if candidate_alpha_id.is_none() && simulate_expression {
        let (formula, config) = candidate_source
            .as_ref()
            .context("--simulate requires an expression")?;
        let settings = AlphaSettings {
            instrument_type: config.instrument_type.clone(),
            region: config.region.clone(),
            universe: config.universe.clone(),
            delay: config.delay,
            ..Default::default()
        };
        println!("  Running one private BRAIN simulation to obtain candidate PnL...");
        let simulation_url = client.submit_simulation(formula, &settings).await?;
        let result = client
            .poll_simulation(&simulation_url, 1800, 8)
            .await
            .context("BRAIN candidate simulation did not complete")?;
        let alpha_ref = result
            .alpha
            .as_deref()
            .context("BRAIN completed the simulation without returning an Alpha reference")?;
        let alpha_id = alpha_ref
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(alpha_ref)
            .to_string();
        candidate_pnl = Some(client.fetch_daily_pnl(&alpha_id).await?);
        candidate_alpha_id = Some(alpha_id);
    }

    let portfolio_path = Path::new("portfolio_os.json");
    let (portfolio, portfolio_available): (Vec<PortfolioAlpha>, bool) = if portfolio_path.exists() {
        let data = fs::read_to_string(portfolio_path)?;
        (serde_json::from_str(&data)?, true)
    } else {
        (Vec::new(), false)
    };

    let data_dir = Path::new("data");
    let local_field_datasets = load_local_field_datasets(data_dir);
    let mut field_cache: FieldUsageCache =
        fs::read_to_string(get_cache_dir().join("data_field_usage.json"))
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
    let now = current_unix_time();
    let mut live_fields = HashMap::new();
    let mut field_source = HashMap::new();

    let mut profile_lookups = 0usize;
    'profile_lookup: for alpha in &portfolio {
        let config = profile_field_config(alpha);
        let formula_fields = build_formula_allocation(
            alpha.code.as_deref().unwrap_or_default(),
            &local_field_datasets,
            &HashMap::new(),
        )
        .fields;
        for field_id in formula_fields.iter().take(3) {
            if profile_lookups >= 120 {
                break 'profile_lookup;
            }
            let key = field_cache_key(field_id, &config);
            if get_fresh_cached_field(&field_cache, &key, now).is_some() {
                continue;
            }
            profile_lookups += 1;
            if let Ok(Some(field)) = client
                .fetch_data_field(
                    field_id,
                    &config.instrument_type,
                    &config.region,
                    &config.universe,
                    config.delay,
                )
                .await
            {
                cache_field(&mut field_cache, key, field, now);
            }
        }
    }

    if let Some((formula, config)) = candidate_source.as_ref() {
        let field_ids =
            build_formula_allocation(formula, &local_field_datasets, &HashMap::new()).fields;
        for field_id in field_ids {
            let key = field_cache_key(&field_id, config);
            if let Some(field) = get_fresh_cached_field(&field_cache, &key, now) {
                let fetched_at = field_cache
                    .entries
                    .get(&key)
                    .map(|entry| entry.fetched_at_unix_secs)
                    .unwrap_or(now);
                let age_hours = now.saturating_sub(fetched_at) / 3600;
                field_source.insert(field_id.clone(), format!("cache, {}h old", age_hours));
                live_fields.insert(field_id, field);
                continue;
            }

            match client
                .fetch_data_field(
                    &field_id,
                    &config.instrument_type,
                    &config.region,
                    &config.universe,
                    config.delay,
                )
                .await
            {
                Ok(Some(field)) => {
                    cache_field(&mut field_cache, key, field.clone(), now);
                    field_source.insert(field_id.clone(), "BRAIN API (live)".to_string());
                    live_fields.insert(field_id, field);
                }
                Ok(None) => {
                    field_source.insert(field_id, "not returned for this config".to_string());
                }
                Err(_) => {
                    if let Some(cached) = field_cache.entries.get(&key) {
                        field_source.insert(
                            field_id.clone(),
                            "stale cache (API unavailable)".to_string(),
                        );
                        live_fields.insert(field_id, cached.field.clone());
                    } else {
                        field_source.insert(field_id, "unavailable".to_string());
                    }
                }
            }
        }
    }

    let cache_path = get_cache_dir().join("data_field_usage.json");
    if let Ok(contents) = serde_json::to_vec_pretty(&field_cache) {
        let _ = fs::write(cache_path, contents);
    }

    let mut cached_mapping_fields: HashMap<String, DataFieldEntry> = HashMap::new();
    let mut ambiguous_cached_ids = std::collections::HashSet::new();
    for cached in field_cache.entries.values() {
        let field = &cached.field;
        let Some(dataset_id) = field
            .dataset
            .as_ref()
            .and_then(|dataset| dataset.id.as_deref())
        else {
            continue;
        };
        let field_id = field.id.to_lowercase();
        if ambiguous_cached_ids.contains(&field_id) {
            continue;
        }
        if let Some(existing) = cached_mapping_fields.get(&field_id) {
            let existing_dataset = existing
                .dataset
                .as_ref()
                .and_then(|dataset| dataset.id.as_deref());
            if !existing_dataset
                .is_some_and(|existing_id| existing_id.eq_ignore_ascii_case(dataset_id))
            {
                cached_mapping_fields.remove(&field_id);
                ambiguous_cached_ids.insert(field_id);
            }
        } else {
            cached_mapping_fields.insert(field_id, field.clone());
        }
    }

    let mut portfolio_allocations: std::collections::BTreeMap<
        DatasetAllocationKey,
        AllocationSets,
    > = std::collections::BTreeMap::new();
    let mut alpha_dataset_fields: HashMap<String, ProfileDatasetFields> = HashMap::new();
    let mut portfolio_field_count = 0usize;
    let mut portfolio_inferred_count = 0usize;
    let mut portfolio_unmapped_count = 0usize;
    for alpha in &portfolio {
        let config = profile_field_config(alpha);
        let allocation = build_formula_allocation(
            alpha.code.as_deref().unwrap_or_default(),
            &local_field_datasets,
            &cached_mapping_fields,
        );
        portfolio_field_count += allocation.fields.len();
        portfolio_inferred_count += allocation.inferred_fields.len();
        portfolio_unmapped_count += allocation.unmapped_fields;
        for (dataset_id, fields) in &allocation.by_dataset {
            let key = (
                config.instrument_type.to_uppercase(),
                config.region.to_uppercase(),
                config.universe.to_uppercase(),
                config.delay,
                dataset_id.to_uppercase(),
            );
            alpha_dataset_fields
                .entry(alpha.id.clone())
                .or_default()
                .push((key.clone(), fields.clone()));
            let usage = portfolio_allocations.entry(key).or_default();
            usage.0.insert(alpha.id.clone());
            usage.2.extend(
                fields
                    .iter()
                    .filter(|field| allocation.inferred_fields.contains(*field))
                    .cloned(),
            );
            usage.1.extend(fields.iter().cloned());
        }
    }
    if profile_lookups > 0 {
        println!(
            "  Refreshed profile field allocation metadata with {} API lookups.",
            profile_lookups
        );
    }

    let mut candidate_mapping_fields = cached_mapping_fields;
    candidate_mapping_fields.extend(live_fields.clone());
    let candidate_allocation = candidate_source.as_ref().map(|(formula, _)| {
        build_formula_allocation(formula, &local_field_datasets, &candidate_mapping_fields)
    });

    let mut configs_by_region: HashMap<String, Vec<FieldConfig>> = HashMap::new();
    for alpha in &portfolio {
        let config = profile_field_config(alpha);
        let entries = configs_by_region.entry(config.region.clone()).or_default();
        if !entries.contains(&config) {
            entries.push(config);
        }
    }
    if let Some((_, config)) = candidate_source.as_ref() {
        let entries = configs_by_region.entry(config.region.clone()).or_default();
        if !entries.contains(config) {
            entries.push(config.clone());
        }
    }

    let mut datasets_by_region: HashMap<String, Vec<DatasetEntry>> = HashMap::new();
    for region_name in configs_by_region.keys() {
        match client.fetch_all_datasets(region_name).await {
            Ok(datasets) => {
                datasets_by_region.insert(region_name.clone(), datasets);
            }
            Err(_) => {
                println!("  Dataset census unavailable for region {}.", region_name);
            }
        }
    }

    let mut dataset_counts_by_key: HashMap<DatasetAllocationKey, f64> = HashMap::new();
    let mut observed_dataset_count = 0usize;
    for key in portfolio_allocations.keys() {
        let config = FieldConfig {
            instrument_type: key.0.clone(),
            region: key.1.clone(),
            universe: key.2.clone(),
            delay: key.3,
        };
        if let Some(count) = datasets_by_region
            .get(&config.region)
            .and_then(|datasets| find_dataset_config(datasets, &key.4, &config))
            .and_then(|dataset| dataset.alpha_count)
            .filter(|count| *count > 0)
        {
            let weight = count as f64;
            dataset_counts_by_key.insert(key.clone(), weight);
            observed_dataset_count += 1;
        }
    }
    let (allocation_weights, field_weighted_alpha_count, dataset_fallback_alpha_count) =
        derive_profile_usage_weights(
            &alpha_dataset_fields,
            &field_cache,
            &dataset_counts_by_key,
            now,
        );
    let total_sample_weight: f64 = allocation_weights.values().sum();

    let pnl_cache_path = get_cache_dir().join("portfolio_pnl.json");
    let mut pnl_cache: HashMap<String, HashMap<String, f64>> = fs::read_to_string(&pnl_cache_path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default();
    if let (Some(alpha_id), Some(pnl)) = (candidate_alpha_id.as_ref(), candidate_pnl.as_ref()) {
        pnl_cache.insert(alpha_id.clone(), pnl.clone());
    }
    let mut pnl_ids: std::collections::BTreeSet<String> =
        portfolio.iter().map(|alpha| alpha.id.clone()).collect();
    if let Some(alpha_id) = candidate_alpha_id.as_ref() {
        pnl_ids.insert(alpha_id.clone());
    }
    let refresh_pnl_ids: Vec<String> = pnl_ids
        .into_iter()
        .filter(|alpha_id| {
            !(candidate_alpha_id.as_deref() == Some(alpha_id.as_str()) && candidate_pnl.is_some())
        })
        .collect();
    let mut stale_pnl_count = 0usize;
    for (index, alpha_id) in refresh_pnl_ids.iter().enumerate() {
        if index == 0 {
            println!("  Refreshing allocated Alpha PnL from the main account...");
        }
        println!("  PnL refresh {}/{}...", index + 1, refresh_pnl_ids.len());
        let previous = pnl_cache.get(alpha_id).cloned();
        match client.fetch_daily_pnl(alpha_id).await {
            Ok(pnl) => {
                pnl_cache.insert(alpha_id.clone(), pnl);
            }
            Err(error) => {
                if previous.is_some() {
                    stale_pnl_count += 1;
                }
                if candidate_alpha_id.as_deref() == Some(alpha_id.as_str()) {
                    println!("  Candidate PnL unavailable: {}", error);
                }
            }
        }
    }
    if let Some(alpha_id) = candidate_alpha_id.as_ref() {
        if candidate_pnl.is_none() {
            candidate_pnl = pnl_cache.get(alpha_id).cloned();
        }
        if let Some(pnl) = candidate_pnl.as_ref() {
            pnl_cache.insert(alpha_id.clone(), pnl.clone());
        }
    }
    if let Ok(contents) = serde_json::to_vec(&pnl_cache) {
        let _ = fs::write(&pnl_cache_path, contents);
    }
    if stale_pnl_count > 0 {
        println!(
            "  Used cached PnL for {} Alpha(s) whose live refresh failed.",
            stale_pnl_count
        );
    }

    println!("\n  Crowd usage and local PnL sensitivity (not projected Uniqueness)");
    if portfolio_available {
        println!(
            "  Local profile: {} stored OS alphas; {} formula fields parsed; {} prefix-inferred; {} unmapped.",
            portfolio.len(), portfolio_field_count, portfolio_inferred_count, portfolio_unmapped_count
        );
    } else {
        println!("  Local profile: portfolio_os.json not found; profile allocation omitted.");
    }
    println!("  Census counts come from /data-sets; field counts come from /data-fields (live or labeled cache).\n");

    if !portfolio_allocations.is_empty() || candidate_allocation.is_some() {
        println!(
            "  {:<18} {:<31} {:>10} {:>14} {:>10} {:>17} {:>12} {:>12}",
            "Dataset",
            "Config",
            "Profile α",
            "Mapped fields",
            "Inferred",
            "Candidate fields",
            "Users",
            "Alphas"
        );
        let mut report_rows = std::collections::BTreeSet::new();
        for key in portfolio_allocations.keys() {
            report_rows.insert(key.clone());
        }
        if let (Some(allocation), Some((_, config))) =
            (candidate_allocation.as_ref(), candidate_source.as_ref())
        {
            for dataset_id in allocation.by_dataset.keys() {
                report_rows.insert((
                    config.instrument_type.to_uppercase(),
                    config.region.to_uppercase(),
                    config.universe.to_uppercase(),
                    config.delay,
                    dataset_id.to_uppercase(),
                ));
            }
        }

        for key in report_rows {
            let config = FieldConfig {
                instrument_type: key.0.clone(),
                region: key.1.clone(),
                universe: key.2.clone(),
                delay: key.3,
            };
            let profile = portfolio_allocations.get(&key);
            let profile_alpha_count = profile.map(|usage| usage.0.len()).unwrap_or(0);
            let profile_fields = profile.map(|usage| usage.1.len()).unwrap_or(0);
            let profile_inferred_fields = profile.map(|usage| usage.2.len()).unwrap_or(0);
            let candidate_fields = candidate_allocation
                .as_ref()
                .filter(|_| {
                    candidate_source
                        .as_ref()
                        .is_some_and(|(_, candidate_config)| candidate_config == &config)
                })
                .and_then(|allocation| allocation.by_dataset.get(&key.4.to_lowercase()))
                .map(Vec::len)
                .unwrap_or(0);
            let census = datasets_by_region
                .get(&config.region)
                .and_then(|datasets| find_dataset_config(datasets, &key.4, &config));
            let (users, alphas) = census
                .map(|dataset| {
                    (
                        format_count(dataset.user_count),
                        format_count(dataset.alpha_count),
                    )
                })
                .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));
            println!(
                "  {:<18} {:<31} {:>10} {:>14} {:>10} {:>17} {:>12} {:>12}",
                key.4,
                field_config_label(&config),
                profile_alpha_count,
                profile_fields,
                profile_inferred_fields,
                candidate_fields,
                users,
                alphas
            );
        }
    } else {
        println!("  No mappable dataset allocations found in the local profile or candidate.");
    }

    if let (Some(allocation), Some((_, config))) =
        (candidate_allocation.as_ref(), candidate_source.as_ref())
    {
        println!(
            "\n  Candidate field-level usage ({})",
            field_config_label(config)
        );
        if allocation.fields.is_empty() {
            println!("  No field identifiers were extracted from the formula.");
        } else {
            println!(
                "  {:<30} {:<18} {:>12} {:>12}  Source",
                "Field", "Dataset", "Users", "Alphas"
            );
            for field_id in &allocation.fields {
                let field = live_fields.get(field_id);
                let dataset = allocation
                    .by_dataset
                    .iter()
                    .find(|(_, fields)| fields.iter().any(|field| field == field_id))
                    .map(|(dataset, _)| dataset.as_str())
                    .unwrap_or("unknown");
                let source = field_source
                    .get(field_id)
                    .map(String::as_str)
                    .unwrap_or_else(|| {
                        if allocation.inferred_fields.contains(field_id) {
                            "prefix inferred from local schema"
                        } else {
                            "local schema mapping"
                        }
                    });
                println!(
                    "  {:<30} {:<18} {:>12} {:>12}  {}",
                    field_id,
                    dataset,
                    format_count(field.and_then(|entry| entry.user_count)),
                    format_count(field.and_then(|entry| entry.alpha_count)),
                    source
                );
            }

            let mut measured_fields = 0usize;
            let mut alpha_penetration_sum = 0.0;
            let mut alpha_penetration_count = 0usize;
            let mut user_penetration_sum = 0.0;
            let mut user_penetration_count = 0usize;
            for (dataset_id, fields) in &allocation.by_dataset {
                let dataset = datasets_by_region
                    .get(&config.region)
                    .and_then(|datasets| find_dataset_config(datasets, dataset_id, config));
                for field_id in fields {
                    if field_source
                        .get(field_id)
                        .is_some_and(|source| source.starts_with("stale cache"))
                    {
                        continue;
                    }
                    let Some(field) = live_fields.get(field_id) else {
                        continue;
                    };
                    let Some(dataset) = dataset else {
                        continue;
                    };
                    let mut field_has_comparable_count = false;
                    if let (Some(field_count), Some(dataset_count)) =
                        (field.alpha_count, dataset.alpha_count)
                    {
                        if field_count >= 0 && dataset_count > 0 {
                            alpha_penetration_sum += field_count as f64 / dataset_count as f64;
                            alpha_penetration_count += 1;
                            field_has_comparable_count = true;
                        }
                    }
                    if let (Some(field_count), Some(dataset_count)) =
                        (field.user_count, dataset.user_count)
                    {
                        if field_count >= 0 && dataset_count > 0 {
                            user_penetration_sum += field_count as f64 / dataset_count as f64;
                            user_penetration_count += 1;
                            field_has_comparable_count = true;
                        }
                    }
                    if field_has_comparable_count {
                        measured_fields += 1;
                    }
                }
            }
            if alpha_penetration_count > 0 || user_penetration_count > 0 {
                let alpha_share = if alpha_penetration_count > 0 {
                    format!(
                        "{:.1}%",
                        100.0 * alpha_penetration_sum / alpha_penetration_count as f64
                    )
                } else {
                    "unavailable".to_string()
                };
                let user_share = if user_penetration_count > 0 {
                    format!(
                        "{:.1}%",
                        100.0 * user_penetration_sum / user_penetration_count as f64
                    )
                } else {
                    "unavailable".to_string()
                };
                println!(
                    "  Usage penetration proxy: mean field share of dataset Alphas = {}; mean share of dataset users = {} ({} of {} fields measured).",
                    alpha_share,
                    user_share,
                    measured_fields,
                    allocation.fields.len()
                );
                println!("  This reports popularity of the formula's fields; it is not a PnL-correlation or Uniqueness estimate.");
            } else {
                println!("  Usage penetration proxy unavailable: matching field and dataset counts were not available.");
            }
            if allocation.unmapped_fields > 0 {
                println!(
                    "  {} field identifier(s) could not be mapped to a dataset.",
                    allocation.unmapped_fields
                );
            }
        }
    }

    let sampled_datasets: std::collections::HashSet<DatasetAllocationKey> = portfolio_allocations
        .iter()
        .filter(|(_, usage)| {
            usage
                .0
                .iter()
                .any(|alpha_id| pnl_cache.contains_key(alpha_id))
        })
        .filter_map(|(key, _)| {
            dataset_counts_by_key
                .contains_key(key)
                .then_some(key.clone())
        })
        .collect();
    let usage_weighted_pnl = build_usage_weighted_profile_pnl(
        &allocation_weights,
        &pnl_cache,
        total_sample_weight,
        sampled_datasets.len(),
    );
    let competitions = client.fetch_competitions().await.unwrap_or_default();
    let official_score = challenge_uniqueness_score(&competitions);
    let profile_alpha_ids: Vec<String> = portfolio
        .iter()
        .filter(|alpha| pnl_cache.contains_key(&alpha.id))
        .map(|alpha| alpha.id.clone())
        .collect();
    let profile_pnl = equal_weight_pnl_merge(&profile_alpha_ids, &pnl_cache);

    println!("\n  Local PnL sensitivity (not competitor PnL)");
    println!(
        "  Local PnL series: {} of {} active Alphas; field-count weights for {} Alphas; {} dataset configurations with counts.",
        usage_weighted_pnl.as_ref().map(|proxy| proxy.sample_alpha_count).unwrap_or(0),
        portfolio.len(),
        field_weighted_alpha_count,
        observed_dataset_count
    );
    println!(
        "  Dataset fallback weights used for {} Alphas; local PnL weight coverage: {:.1}%.",
        dataset_fallback_alpha_count,
        usage_weighted_pnl
            .as_ref()
            .map(|proxy| proxy.pnl_weight_coverage * 100.0)
            .unwrap_or(0.0)
    );

    if let Some(score) = official_score {
        println!(
            "  Current official BRAIN Uniqueness: {:.3} (current value only).",
            score
        );
    } else {
        println!("  Current official BRAIN Uniqueness was not available from the account API.");
    }

    match (usage_weighted_pnl.as_ref(), profile_pnl.as_ref()) {
        (Some(proxy), Some(profile_series)) => {
            let (baseline_proxy_corr, common_days) =
                pearson_correlation(profile_series, &proxy.pnl);
            println!(
                "  Usage-weighted local PnL basket: {} Alpha samples across {} dataset configurations; {} common days.",
                proxy.sample_alpha_count, proxy.sample_dataset_count, common_days
            );
            println!(
                "  Equal-weight profile vs usage-weighted local basket: {:.3} correlation.",
                baseline_proxy_corr
            );

            if let Some(alpha_id) = candidate_alpha_id.as_ref() {
                if portfolio.iter().any(|alpha| alpha.id.eq_ignore_ascii_case(alpha_id)) {
                    println!("  Candidate is already active in this profile; no add-on projection was made.");
                } else if candidate_pnl.is_some() {
                    let mut projected_ids = profile_alpha_ids.clone();
                    projected_ids.push(alpha_id.clone());
                    if let Some(projected_pnl) = equal_weight_pnl_merge(&projected_ids, &pnl_cache) {
                        if let Some(projection) = project_proxy_correlation(
                            profile_series,
                            &projected_pnl,
                            &proxy.pnl,
                        ) {
                            println!(
                                "  Candidate impact on local basket correlation: {:.3} -> {:.3} ({:+.3}).",
                                projection.baseline_proxy_corr,
                                projection.projected_proxy_corr,
                                projection.correlation_delta
                            );
                        } else {
                            println!("  Not enough overlapping PnL dates to estimate candidate impact.");
                        }
                    } else {
                        println!("  Could not merge candidate and profile PnL over common dates.");
                    }
                } else {
                    println!("  Candidate PnL is unavailable, so local PnL sensitivity was not computed.");
                }
            } else if candidate_source.is_some() {
                println!("  An expression has no PnL yet. Use --simulate with --expr for local portfolio sensitivity only.");
            }

            if proxy.pnl_weight_coverage < 0.70 || proxy.sample_alpha_count < 5 {
                println!("  Data coverage: LOW — few local Alpha PnLs or limited weight coverage.");
            } else {
                println!("  Data coverage: adequate for this local diagnostic only; it says nothing quantitative about competitor PnL.");
            }
        }
        _ => println!("  No local sensitivity: need mapped active Alphas, their PnL series, and at least 30 overlapping days."),
    }

    println!("\n  Weight method: average fresh field alphaCount within each mapped dataset, then average across that Alpha's datasets; use dataset alphaCount where field counts are unavailable.");
    println!("  This reweights only your local profile PnLs. BRAIN does not return the other competitors' formulas or merged PnLs through these endpoints.");
    println!("  Therefore only the current official Uniqueness is known; this command does not forecast a future Uniqueness score.");
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
        "  AlphaFind — Configured Dataset-Interaction Heuristic"
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
                    if snap.region.eq_ignore_ascii_case(&region) {
                        snap.datasets
                    } else {
                        anyhow::bail!(
                            "Cached census is for region {}, requested {}.",
                            snap.region,
                            region
                        );
                    }
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
            .filter(|snapshot: &CensusSnapshot| snapshot.region.eq_ignore_ascii_case(&region))
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
        println!("  │ Pillar │ Economic Architecture Role               │ Users (data) │ Alphas (data)│ Count band │");
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
            "\n  {} Composite Configured-Interaction Heuristic (CHUI): {}",
            "🎯".bold(),
            chui_styled
        );
        println!(
            "  {} Heuristic interpretation (not platform Uniqueness): {}",
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
            "\n  {} Rule-Based Multi-Dataset Hypotheses for {} (not validated forecasts):",
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

async fn cmd_blueprint(universe: Option<String>, export: Option<String>) -> Result<()> {
    // 1. Load active OS portfolio
    let port_path = "portfolio_os.json";
    if !Path::new(port_path).exists() {
        anyhow::bail!("portfolio_os.json not found! Run 'alphafind sync' first.");
    }
    let port_data = fs::read_to_string(port_path)?;
    let os_alphas: Vec<PortfolioAlpha> = serde_json::from_str(&port_data)?;

    // 2. Load cached baseline PnLs
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    let pnl_cache: HashMap<String, HashMap<String, f64>> = if cache_file.exists() {
        fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // 3. Generate inverse blueprint report
    let report = generate_inverse_blueprint(&os_alphas, &pnl_cache, universe.as_deref());

    // 4. Print master report
    print_blueprint_report(&report);

    // 5. Optional file export
    if let Some(export_path) = export {
        let mut out = String::new();
        out.push_str("# AlphaFind Inverse Portfolio Optimization & Forecast Blueprint\n\n");
        out.push_str(&format!(
            "Generated for: {} Out-of-Sample Alphas\n\n",
            report.active_alpha_count
        ));
        for (i, arch) in report.top_archetypes.iter().enumerate() {
            out.push_str(&format!(
                "## Archetype #{}: {} [{}]\n",
                i + 1,
                arch.name,
                arch.target_universe
            ));
            out.push_str(&format!(
                "- Primary Factor: {} x {}\n",
                arch.primary_pillar, arch.secondary_catalyst
            ));
            out.push_str(&format!(
                "- Datasets: Anchor: {} | Catalyst: {}\n",
                arch.anchor_dataset, arch.catalyst_dataset
            ));
            out.push_str(&format!("- Optimal Settings: Universe: {}, Decay: {}, Neutralization: {}, Power: {:.1}, Truncation: {:.3}\n",
                arch.target_universe, arch.recommended_decay, arch.recommended_neutralization, arch.recommended_power, arch.recommended_truncation));
            out.push_str(&format!(
                "- Economic Rationale: {}\n",
                arch.economic_rationale
            ));
            out.push_str("```python\n");
            out.push_str(&arch.fast_expr_skeleton);
            out.push_str("\n```\n\n");
        }
        fs::write(&export_path, out)?;
        println!(
            "  💾 Successfully exported blueprint skeletons to {}\n",
            export_path.green().bold()
        );
    }

    Ok(())
}

async fn cmd_health(alpha_id: Option<String>, export: Option<String>, all: bool) -> Result<()> {
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
        if let Ok(client) = get_main_client() {
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
                let _ = fs::write(&cache_file, &serialized);
            }
        }
    }

    let report = evaluate_portfolio_health(&os_alphas, &pnl_cache, None).context(
        "Failed to evaluate portfolio health (insufficient data or overlapping trading days < 60)",
    )?;

    print_health_report(&report);

    if all {
        print_detailed_alpha_table(&report.all_alpha_diagnostics);
    }

    if let Some(ref target_id) = alpha_id {
        if let Some(target_diag) = report
            .all_alpha_diagnostics
            .iter()
            .find(|a| a.alpha_id.eq_ignore_ascii_case(target_id))
        {
            print_single_alpha_diagnostic(target_diag);
        } else {
            println!(
                "  ⚠️  Alpha ID '{}' not found in active Out-of-Sample portfolio.",
                target_id.yellow()
            );
        }
    }

    if let Some(ref export_path) = export {
        let serialized = serde_json::to_string_pretty(&report)?;
        fs::write(export_path, serialized)?;
        println!(
            "  💾 Full Master Portfolio Health Report exported to: {}\n",
            export_path.green().bold()
        );
    }

    Ok(())
}
