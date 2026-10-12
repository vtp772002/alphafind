//! Command handlers for the AlphaFind CLI.
//!
//! Subcommands are partitioned into domain-specific modules:
//! - [`auth`]: Authentication, synchronization, and score status
//! - [`submission`]: Pre-flight check verification and alpha submission gates
//! - [`simulation`]: In-sample simulation, parameter tuning, multi-account screening, and radar
//! - [`portfolio`]: Out-of-sample portfolio audit, merge simulation, impact analysis, and health
//! - [`research`]: Dataset interaction matrix, gap blueprinting, and proxy shadow analysis

pub mod auth;
pub mod portfolio;
pub mod research;
pub mod simulation;
pub mod submission;

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::client::BrainClient;
use crate::config::operational::PNL_CACHE_MAX_AGE_HOURS;

/// Instantiates an authenticated `BrainClient` using credentials from `.env`.
pub fn get_main_client() -> Result<BrainClient> {
    dotenvy::dotenv().ok();
    let email = std::env::var("MAIN_ACCOUNT_EMAIL")
        .or_else(|_| std::env::var("WQ_BRAIN_EMAIL"))
        .context("Missing MAIN_ACCOUNT_EMAIL in .env")?;
    let pass = std::env::var("MAIN_ACCOUNT_PASSWORD")
        .or_else(|_| std::env::var("WQ_BRAIN_PASSWORD"))
        .context("Missing MAIN_ACCOUNT_PASSWORD in .env")?;

    BrainClient::new(email, pass)
}

/// Returns the path to the AlphaFind local cache directory (~/.cache/alphafind).
pub fn get_cache_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let cache_dir = PathBuf::from(home).join(".cache").join("alphafind");
    let _ = fs::create_dir_all(&cache_dir);
    cache_dir
}

/// Loads the local PnL cache along with its age and freshness information.
pub fn load_pnl_cache() -> (
    HashMap<String, HashMap<String, f64>>,
    Option<std::time::Duration>,
) {
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    if !cache_file.exists() {
        return (HashMap::new(), None);
    }

    let age = fs::metadata(&cache_file)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| std::time::SystemTime::now().duration_since(t).ok());

    let pnl_cache = fs::read_to_string(&cache_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    (pnl_cache, age)
}

/// Prints cache age and alerts if cache is stale (>24h per config).
pub fn print_cache_status(age: Option<std::time::Duration>, count: usize) {
    match age {
        Some(d) => {
            let secs = d.as_secs();
            let age_str = if secs < 60 {
                format!("{}s ago", secs)
            } else if secs < 3600 {
                format!("{:.1}m ago", secs as f64 / 60.0)
            } else {
                format!("{:.1}h ago", secs as f64 / 3600.0)
            };

            if secs > PNL_CACHE_MAX_AGE_HOURS * 3600 {
                println!(
                    "  {} Local PnL cache was updated {} ({} alphas cached, exceeds {}h freshness recommended in AGENTS.md). Run 'alphafind portfolio' to refresh.",
                    "⚠️  [CACHE STALE]".yellow().bold(),
                    age_str.yellow(),
                    count,
                    PNL_CACHE_MAX_AGE_HOURS
                );
            } else {
                println!(
                    "  ℹ️  Local PnL cache: {} alphas loaded (cached {})",
                    count,
                    age_str.cyan()
                );
            }
        }
        None => {
            if count > 0 {
                println!("  ℹ️  Local PnL cache: {} alphas loaded", count);
            }
        }
    }
}

/// Helper to format file paths for console display
pub fn json_path_display(p: &str) -> colored::ColoredString {
    p.bold()
}

/// Helper to format counts for console display
pub fn format_count(value: Option<i64>) -> String {
    value
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Helper to generate a unique key for dataset census snapshots
pub fn dataset_census_key(dataset: &crate::models::DatasetEntry) -> String {
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

/// Helper to format field config label
pub fn field_config_label(config: &crate::crowd_allocation::FieldConfig) -> String {
    format!(
        "{}/{}/{}/D{}",
        config.instrument_type, config.region, config.universe, config.delay
    )
}

/// Helper to derive field configuration from a portfolio alpha
pub fn profile_field_config(
    alpha: &crate::models::PortfolioAlpha,
) -> crate::crowd_allocation::FieldConfig {
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

    crate::crowd_allocation::FieldConfig {
        instrument_type: string_setting("instrumentType", "instrument_type")
            .unwrap_or_else(|| "EQUITY".to_string()),
        region: string_setting("region", "region").unwrap_or_else(|| "USA".to_string()),
        universe: string_setting("universe", "universe")
            .or_else(|| alpha.universe.clone())
            .unwrap_or_else(|| "TOP1000".to_string()),
        delay: integer_setting("delay", "delay").unwrap_or(1),
    }
}
