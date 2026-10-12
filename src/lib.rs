//! # AlphaFind: Quantitative Alpha Research & Portfolio Orthogonality Engine
//!
//! `alphafind` is a high-performance quantitative research toolkit and CLI designed for
//! quantitative alpha screening, crowd-correlation estimation, portfolio health diagnostics,
//! and automated hyperparameter optimization.
//!
//! ## Core Architecture Modules
//! - [`autotuner`]: Grid & coordinate search for alpha hyperparameter optimization (decay, universe, neutralization).
//! - [`client`]: Fail-closed, SSRF-safe HTTP client for BRAIN API simulation & submission checks.
//! - [`commands`]: CLI command handler modules (auth, simulation, submission, portfolio, research).
//! - [`config`]: Quant thresholds and configuration partitions (platform rules, project policies, research hypotheses).
//! - [`correlation`]: Pearson correlation engine, portfolio simulation, and orthogonality audits.
//! - [`crowd`]: Crowd overlap, submission velocity analysis, and crowd health metrics.
//! - [`crowd_allocation`]: Optimal allocation weights under crowd decay constraints.
//! - [`crowd_model`]: Synthetic crowd PnL modeling and cross-sectional crowd simulation.
//! - [`forecast`]: PnL trajectory forecasting, return modeling, and turnover projections.
//! - [`health`]: Multi-horizon decay analytics, performance degradation scoring, and health audits.
//! - [`matrix`]: Pairwise correlation matrix generation, cluster analysis, and heatmap visualization.
//! - [`models`]: Type definitions, data models, and serialization schemas.
//! - [`screener`]: Multi-threaded alpha candidate generator and expression screening engine.
//! - [`shadow`]: Shadow benchmark correlation, crowd-clone risk estimation, and portfolio safety buffer.
//! - [`taxonomy`]: Mathematical strategy classification, alpha style categorization, and expression templates.

pub mod autotuner;
pub mod client;
pub mod commands;
pub mod config;
pub mod correlation;
pub mod crowd;
pub mod crowd_allocation;
pub mod crowd_model;
pub mod forecast;
pub mod health;
pub mod matrix;
pub mod models;
pub mod screener;
pub mod shadow;
pub mod taxonomy;
