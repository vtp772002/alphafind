//! Quantitative research constants and configuration thresholds for AlphaFind.
//!
//! Defines official platform rule boundaries, quant thresholds, and operational parameters
//! adhering to the separation standard in `AGENTS.md`.

/// Quantitative research and platform calculation thresholds
pub mod quant {
    /// Maximum allowable self-correlation on WorldQuant BRAIN (0.70). Alphas exceeding this fail submission checks.
    pub const BRAIN_MAX_CORRELATION: f64 = 0.70;

    /// Threshold for declaring true orthogonality against portfolio (< 0.15).
    pub const ORTHOGONAL_CORRELATION_THRESHOLD: f64 = 0.15;

    /// Minimum overlapping trading days required to compute Pearson correlation in a fail-closed manner.
    pub const MIN_CORRELATION_DAYS: usize = 30;

    /// Minimum overlapping trading days required to compute portfolio simulation metrics.
    pub const MIN_PORTFOLIO_DAYS: usize = 100;

    /// Standard number of annual trading days used for annualizing PnL, volatility, and Sharpe ratio.
    pub const TRADING_DAYS_PER_YEAR: f64 = 252.0;

    /// Minimum Sharpe ratio required for standard Brain competition qualification.
    pub const MIN_QUALIFYING_SHARPE: f64 = 1.58;

    /// Minimum Turnover threshold (1%).
    pub const MIN_TURNOVER: f64 = 0.01;

    /// Maximum Turnover threshold (70%).
    pub const MAX_TURNOVER: f64 = 0.70;

    /// Epsilon variance floor to avoid division by zero on flatline series.
    pub const EPSILON_VARIANCE: f64 = 1e-12;

    /// Epsilon volatility floor to avoid division by zero in Sharpe calculation.
    pub const EPSILON_VOLATILITY: f64 = 1e-9;
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
