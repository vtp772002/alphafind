//! Quantitative research constants and configuration thresholds for AlphaFind.
//!
//! Enforces strict separation of concerns per `AGENTS.md`:
//! - [`platform_rules`]: Verified official platform rules (WorldQuant BRAIN Learn).
//! - [`project_policies`]: Project-specific local safety thresholds.
//! - [`research_hypotheses`]: Internal research hypotheses and targets.
//! - [`math_constants`]: Standard mathematical and financial conventions.
//! - [`operational`]: Operational execution and caching parameters.

/// Official platform rules verified in BRAIN Learn / platform guidelines
pub mod platform_rules {
    /// Maximum allowable self-correlation on WorldQuant BRAIN (0.70). Alphas exceeding this fail submission checks
    /// unless the candidate's Sharpe is at least 10% higher than all correlated submissions over a 4-year rolling window.
    pub const BRAIN_MAX_CORRELATION: f64 = 0.70;

    /// Minimum Turnover threshold (1.0%). Alphas with Turnover <= 1% fail submission checks.
    pub const MIN_TURNOVER: f64 = 0.01;

    /// Maximum Turnover threshold (70.0%). Alphas with Turnover >= 70% fail submission checks.
    pub const MAX_TURNOVER: f64 = 0.70;

    /// Delay 0 minimum Sharpe ratio requirement (> 2.0).
    pub const MIN_SHARPE_DELAY_0: f64 = 2.00;

    /// Delay 1 minimum Sharpe ratio requirement (> 1.25).
    pub const MIN_SHARPE_DELAY_1: f64 = 1.25;
}

/// Project-chosen safety and sample coverage thresholds
pub mod project_policies {
    /// Minimum overlapping trading days required to compute Pearson correlation in a fail-closed manner.
    pub const MIN_CORRELATION_DAYS: usize = 30;

    /// Minimum overlapping trading days required to compute portfolio simulation metrics.
    pub const MIN_PORTFOLIO_DAYS: usize = 100;
}

/// Internal research targets and optimization hypotheses (not official BRAIN thresholds)
pub mod research_hypotheses {
    /// Local heuristic threshold for declaring true orthogonality against portfolio (< 0.15).
    pub const ORTHOGONAL_CORRELATION_THRESHOLD: f64 = 0.15;

    /// Internal research target Sharpe heuristic for screening candidate quality.
    pub const INTERNAL_TARGET_SHARPE_HEURISTIC: f64 = 1.58;
}

/// Standard mathematical and financial conventions
pub mod math_constants {
    /// Standard annual trading days in US equity markets (used for annualizing PnL and Sharpe ratio).
    pub const TRADING_DAYS_PER_YEAR: f64 = 252.0;

    /// Epsilon variance floor to avoid division by zero or jitter noise on flatline series.
    pub const EPSILON_VARIANCE: f64 = 1e-12;

    /// Epsilon volatility floor to avoid division by zero in Sharpe ratio calculation.
    pub const EPSILON_VOLATILITY: f64 = 1e-9;
}

/// Legacy and unified quant namespace re-exporting the above categories
pub mod quant {
    pub use super::math_constants::*;
    pub use super::platform_rules::*;
    pub use super::project_policies::*;
    pub use super::research_hypotheses::*;

    /// Backward-compatibility alias for internal target Sharpe
    pub const MIN_QUALIFYING_SHARPE: f64 =
        super::research_hypotheses::INTERNAL_TARGET_SHARPE_HEURISTIC;
}

/// Operational and caching thresholds
pub mod operational {
    /// Maximum acceptable age in hours before PnL cache is considered stale.
    pub const PNL_CACHE_MAX_AGE_HOURS: u64 = 24;

    /// Default concurrency limit for screening operations per account.
    pub const DEFAULT_WORKERS_PER_ACCOUNT: usize = 3;

    /// Simulation status polling delay in milliseconds.
    pub const SIMULATION_POLL_INTERVAL_MS: u64 = 3000;

    /// Maximum simulation status polling retries before timing out.
    pub const SIMULATION_POLL_MAX_ATTEMPTS: usize = 100;
}
