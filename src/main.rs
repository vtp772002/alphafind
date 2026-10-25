use anyhow::Result;
use clap::{Parser, Subcommand};

use alphafind::commands::auth::{cmd_auth, cmd_score, cmd_sync};
use alphafind::commands::portfolio::{
    cmd_allocate, cmd_audit, cmd_health, cmd_impact, cmd_portfolio,
};
use alphafind::commands::research::{
    cmd_blueprint, cmd_fields, cmd_generate, cmd_matrix, cmd_shadow,
};
use alphafind::commands::simulation::{cmd_radar, cmd_screen, cmd_sim, cmd_tune};
use alphafind::commands::submission::{cmd_check, cmd_submit};

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

    /// Verify the mandatory In-Sample platform submission checks for an Alpha
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

        /// Force live re-fetch of all active OS Alpha PnLs before auditing
        #[arg(long)]
        refresh: bool,
    },

    /// Simulate merged multi-alpha Out-of-Sample portfolio Sharpe and diversification metrics
    Portfolio {
        /// Force live re-fetch of all active OS Alpha PnLs before simulating
        #[arg(long)]
        refresh: bool,
    },

    /// Submit an Alpha to Out-of-Sample (OS) after pre-flight check verification
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

    /// Screen candidates using configured BRAIN sessions with a bounded work queue
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

        /// Workers per session (concurrency is bounded by semaphore)
        #[arg(short, long, default_value_t = 3)]
        workers: usize,

        /// Disable automatic decay parameter sweeps
        #[arg(long)]
        no_tune: bool,

        /// Run strictly on MAIN account only (single-account mode)
        #[arg(long, short = 's')]
        single_account: bool,

        /// Screen alternative data candidate expressions (M4 feature)
        #[arg(long)]
        alternative: bool,

        /// Specific alternative dataset for generation (model51, model16, news18, option9, cross_sanctuary)
        #[arg(long)]
        dataset: Option<String>,

        /// Candidate count for alternative generation (default: 10)
        #[arg(long, default_value_t = 10)]
        count: usize,
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

    /// Simulate exact portfolio merge impact for a candidate Alpha
    Impact {
        /// Candidate Alpha ID (e.g., A1b2C3d4)
        alpha_id: String,

        /// Force live re-fetch of all active OS Alpha PnLs before simulating impact
        #[arg(long)]
        refresh: bool,
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
        #[arg(short = 'i', long)]
        alpha_id: Option<String>,

        /// Export full diagnostic JSON report to a file
        #[arg(long)]
        export: Option<String>,

        /// Show full table of all active alphas
        #[arg(short, long)]
        all: bool,

        /// Force live re-fetch of all active OS Alpha PnLs before evaluating health
        #[arg(long)]
        refresh: bool,
    },

    /// Diagnose temporal turnover entropy, HRP risk weights, and evaluate candidate admission decisions
    Allocate {
        /// Optional Candidate Alpha ID to test admission impact
        #[arg(short, long)]
        candidate: Option<String>,

        /// Optional candidate turnover parameter (e.g. 0.35)
        #[arg(long)]
        turnover: Option<f64>,

        /// Display retirement & trimming candidates
        #[arg(short, long)]
        trim: bool,

        /// Export allocation diagnostic JSON to file
        #[arg(long)]
        export: Option<String>,

        /// Force live re-fetch of all active OS Alpha PnLs before evaluating allocation
        #[arg(long)]
        refresh: bool,
    },

    /// Query, inspect, and validate field metadata from WorldQuant BRAIN
    #[command(alias = "metadata")]
    Fields {
        /// Target Dataset ID (e.g., model51, model16, news18, option9, or all)
        #[arg(short, long)]
        dataset: Option<String>,

        /// Force live re-fetch from BRAIN /data-fields and update disk cache
        #[arg(long)]
        refresh: bool,

        /// Validate a FastExpr expression against field schemas (type check & coverage)
        #[arg(short, long)]
        validate: Option<String>,

        /// Target Universe for metadata query and coverage check (default: TOP1000)
        #[arg(short, long, default_value = "TOP1000")]
        universe: String,

        /// Signal Delay (default: 1)
        #[arg(long, default_value_t = 1)]
        delay: i32,

        /// Target Region (default: USA)
        #[arg(short, long, default_value = "USA")]
        region: String,

        /// Show high-level summary of datasets and field type breakdown
        #[arg(short, long)]
        summary: bool,

        /// Flag low coverage fields (<50% cross-sectional, <90% date coverage)
        #[arg(long)]
        check_coverage: bool,

        /// Export metadata to JSON file
        #[arg(long)]
        export: Option<String>,
    },

    /// Generate verified FastExpr candidate expressions across alternative data pillars
    Generate {
        /// Target Dataset ID (model51, model16, news18, option9, cross_sanctuary, or all)
        #[arg(short, long)]
        dataset: Option<String>,

        /// Target Factor Pillar (SYSTEMATIC_RISK, FUNDAMENTAL_SCORES, NEWS_SENTIMENT, OPTIONS_ANALYTICS, CROSS_SANCTUARY, all)
        #[arg(short, long)]
        pillar: Option<String>,

        /// Maximum number of expressions to generate
        #[arg(short = 'n', long = "count", default_value_t = 10)]
        count: usize,

        /// Target Universe (default: TOP1000)
        #[arg(short, long, default_value = "TOP1000")]
        universe: String,

        /// Signal Delay (default: 1)
        #[arg(long, default_value_t = 1)]
        delay: i32,

        /// Neutralization group (SUBINDUSTRY, INDUSTRY, SECTOR)
        #[arg(long, default_value = "SUBINDUSTRY")]
        neutralization: String,

        /// Validate all generated expressions against SchemaValidator
        #[arg(long, default_value_t = true)]
        validate: bool,

        /// Export generated candidates to JSON file
        #[arg(long)]
        export: Option<String>,

        /// Output format (table, json, expr-only)
        #[arg(long, default_value = "table")]
        format: String,

        /// Shorthand flag for JSON output
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth => cmd_auth().await,
        Commands::Sync => cmd_sync().await,
        Commands::Score => cmd_score().await,
        Commands::Check { alpha_id } => cmd_check(alpha_id).await,
        Commands::Audit {
            alpha_id,
            label,
            refresh,
        } => cmd_audit(alpha_id, label, refresh).await,
        Commands::Portfolio { refresh } => cmd_portfolio(refresh).await,
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
            single_account,
            alternative,
            dataset,
            count,
        } => {
            cmd_screen(
                pillar,
                universe,
                min_fitness,
                workers,
                no_tune,
                single_account,
                alternative,
                dataset,
                count,
            )
            .await
        }
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
        Commands::Impact { alpha_id, refresh } => cmd_impact(alpha_id, refresh).await,
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
            refresh,
        } => cmd_health(alpha_id, export, all, refresh).await,
        Commands::Allocate {
            candidate,
            turnover,
            trim,
            export,
            refresh,
        } => cmd_allocate(candidate, turnover, trim, export, refresh).await,
        Commands::Fields {
            dataset,
            refresh,
            validate,
            universe,
            delay,
            region,
            summary,
            check_coverage,
            export,
        } => {
            cmd_fields(
                dataset,
                refresh,
                validate,
                universe,
                delay,
                region,
                summary,
                check_coverage,
                export,
            )
            .await
        }
        Commands::Generate {
            dataset,
            pillar,
            count,
            universe,
            delay,
            neutralization,
            validate,
            export,
            format,
            json,
        } => {
            cmd_generate(
                dataset,
                pillar,
                count,
                universe,
                delay,
                neutralization,
                validate,
                export,
                format,
                json,
            )
            .await
        }
    }
}
