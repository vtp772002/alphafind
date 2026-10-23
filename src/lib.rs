//! # AlphaFind: Quantitative Alpha Research & Portfolio Orthogonality Engine
//!
//! `alphafind` is a high-performance quantitative research toolkit and CLI designed for
//! quantitative alpha screening, crowd-correlation estimation, portfolio health diagnostics,
//! and automated hyperparameter optimization.
//!
//! ## Core Architecture Modules
//! - [`allocation`]: Turnover Shannon entropy, Hierarchical Risk Parity (HRP) weights, and fail-closed candidate admission gate.
//! - [`autotuner`]: Delay, exponent, and neutralization parameter sweeps with delay-aware platform thresholds.
//! - [`client`]: Fail-closed, SSRF-safe HTTP client for BRAIN API simulation & submission checks.
//! - [`commands`]: CLI command handler modules (auth, simulation, submission, portfolio, research).
//! - [`config`]: Quant thresholds and configuration partitions (platform rules, project policies, research hypotheses).
//! - [`correlation`]: Fail-closed Pearson correlation engine, portfolio merge simulation, and orthogonality audits.
//! - [`crowd`]: Live dataset census analysis (`radar`) and deprecated static crowd heuristics.
//! - [`crowd_allocation`]: Formula field extraction, dataset mapping, and field-usage cache management.
//! - [`crowd_model`]: Usage-weighted local profile PnL basket sensitivity diagnostics.
//! - [`forecast`]: Portfolio factor/universe deficit audit and rule-based hypothesis archetypes (`blueprint`).
//! - [`health`]: Multi-horizon decay analytics, performance degradation scoring, and health audits.
//! - [`matrix`]: Configured dataset-interaction heuristic matrix and multi-dataset formula analysis.
//! - [`models`]: Type definitions, data models, and serialization schemas.
//! - [`screener`]: Bounded worker-queue alpha candidate screener with per-account semaphore concurrency control.
//! - [`shadow`]: Legacy synthetic-proxy correlation helpers (not used by the `shadow` CLI command).
//! - [`taxonomy`]: Strategy pillar classification and candidate template loading from local ignored storage.

pub mod allocation;
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
