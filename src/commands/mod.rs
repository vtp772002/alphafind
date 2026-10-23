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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o700));
    }
    cache_dir
}

/// Writes confidential research or portfolio data to disk and restricts permissions to `0600` on Unix.
pub fn write_private_file<P: AsRef<std::path::Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> Result<()> {
    let path_ref = path.as_ref();
    fs::write(path_ref, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path_ref, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Explicit active/Out-of-Sample classification for a portfolio alpha record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaClassification {
    /// Confirmed Out-of-Sample and Active (`stage == "OS"` and `status == "ACTIVE"`).
    ActiveOs,
    /// Submitted or pending OOS processing (`status` is `SUBMITTED` or `UNPROCESSED`, or `stage == "OS"`).
    PendingOrSubmittedOs,
    /// Decommissioned, inactive, or explicitly In-Sample (`stage == "IS"`).
    InactiveOrNonOs,
    /// Stage or status fields are missing or unrecognized in the local snapshot.
    Unclassified,
}

/// Classifies a [`crate::models::PortfolioAlpha`] by its explicit `stage` and `status` metadata.
pub fn classify_portfolio_alpha(alpha: &crate::models::PortfolioAlpha) -> AlphaClassification {
    let stage = alpha.stage.as_deref().map(str::trim).unwrap_or("");
    let status = alpha.status.as_deref().map(str::trim).unwrap_or("");

    if status.eq_ignore_ascii_case("DECOMMISSIONED")
        || status.eq_ignore_ascii_case("INACTIVE")
        || status.eq_ignore_ascii_case("RETIRED")
        || stage.eq_ignore_ascii_case("IS")
        || stage.eq_ignore_ascii_case("IN_SAMPLE")
    {
        return AlphaClassification::InactiveOrNonOs;
    }

    let is_os_stage =
        stage.eq_ignore_ascii_case("OS") || stage.eq_ignore_ascii_case("OUT_OF_SAMPLE");
    let is_active_status = status.eq_ignore_ascii_case("ACTIVE");

    if is_os_stage && is_active_status {
        AlphaClassification::ActiveOs
    } else if is_os_stage
        || is_active_status
        || status.eq_ignore_ascii_case("SUBMITTED")
        || status.eq_ignore_ascii_case("UNPROCESSED")
        || alpha
            .date_submitted
            .as_deref()
            .is_some_and(|d| !d.trim().is_empty())
    {
        AlphaClassification::PendingOrSubmittedOs
    } else {
        AlphaClassification::Unclassified
    }
}

/// Loads `portfolio_os.json`, reports its cache age and explicit active/OOS classification breakdown,
/// and filters out explicitly inactive/non-OS records.
pub fn load_portfolio_os() -> Result<Vec<crate::models::PortfolioAlpha>> {
    let port_path = std::path::Path::new("portfolio_os.json");
    if !port_path.exists() {
        anyhow::bail!("portfolio_os.json not found! Run 'alphafind sync' first.");
    }
    let age = fs::metadata(port_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| std::time::SystemTime::now().duration_since(t).ok());
    let (age_label, stale) = cache_age_label(age);

    let port_data = fs::read_to_string(port_path)?;
    let raw_alphas: Vec<crate::models::PortfolioAlpha> = serde_json::from_str(&port_data)?;

    let mut active_os = 0usize;
    let mut pending_os = 0usize;
    let mut inactive = 0usize;
    let mut unclassified = 0usize;
    let mut retained = Vec::with_capacity(raw_alphas.len());

    for alpha in raw_alphas {
        match classify_portfolio_alpha(&alpha) {
            AlphaClassification::ActiveOs => {
                active_os += 1;
                retained.push(alpha);
            }
            AlphaClassification::PendingOrSubmittedOs => {
                pending_os += 1;
                retained.push(alpha);
            }
            AlphaClassification::Unclassified => {
                unclassified += 1;
                retained.push(alpha);
            }
            AlphaClassification::InactiveOrNonOs => {
                inactive += 1;
            }
        }
    }

    if stale {
        println!(
            "  {} portfolio_os.json was updated {} (exceeds {}h freshness policy). Run 'alphafind sync' to refresh.",
            "⚠️  [PORTFOLIO SNAPSHOT STALE]".yellow().bold(),
            age_label.yellow(),
            PNL_CACHE_MAX_AGE_HOURS
        );
    } else {
        println!(
            "  ℹ️  Portfolio snapshot (portfolio_os.json): synced {} | Active OS: {} | Submitted/Pending OS: {} | Unclassified: {} | Excluded Non-OS/Inactive: {}",
            age_label.cyan(),
            active_os,
            pending_os,
            unclassified,
            inactive
        );
    }

    Ok(retained)
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

/// Formats a cache age and reports whether it exceeds the freshness policy.
///
/// Returns `(label, is_stale)`. `None` means the age is unknown (for example a
/// platform response that was not written to disk), which is never treated as fresh.
pub fn cache_age_label(age: Option<std::time::Duration>) -> (String, bool) {
    match age {
        Some(d) => {
            let secs = d.as_secs();
            let label = if secs < 60 {
                format!("{}s ago", secs)
            } else if secs < 3600 {
                format!("{:.1}m ago", secs as f64 / 60.0)
            } else {
                format!("{:.1}h ago", secs as f64 / 3600.0)
            };
            (label, secs > PNL_CACHE_MAX_AGE_HOURS * 3600)
        }
        None => ("age unknown".to_string(), true),
    }
}

/// Prints cache age and alerts if cache is stale (>24h per config).
pub fn print_cache_status(age: Option<std::time::Duration>, count: usize) {
    match age {
        Some(d) => {
            let (age_str, stale) = cache_age_label(Some(d));
            if stale {
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
                println!(
                    "  {} Local PnL cache: {} alphas loaded, but the cache age could not be read. Treat freshness as unknown.",
                    "⚠️".yellow().bold(),
                    count
                );
            }
        }
    }
}

/// Renders Unix seconds as a UTC `YYYY-MM-DD HH:MM` timestamp.
///
/// Uses the civil-from-days algorithm so no date crate is required. Deterministic
/// for every input, including leap years.
pub fn format_epoch_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hour, minute) = (rem / 3_600, (rem % 3_600) / 60);

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year, month, day, hour, minute
    )
}

/// Formats a stored snapshot timestamp for display.
///
/// Snapshots persist `Unix Epoch <secs>`; that value is rendered as a readable UTC
/// date plus its age instead of a raw epoch number. Any other text is returned
/// unchanged so older or hand-edited files still display.
pub fn format_snapshot_timestamp(raw: &str, now_secs: u64) -> String {
    let Some(secs) = raw
        .trim()
        .strip_prefix("Unix Epoch ")
        .and_then(|value| value.trim().parse::<u64>().ok())
    else {
        return raw.to_string();
    };

    let age = now_secs.saturating_sub(secs);
    let age_str = if age < 3_600 {
        format!("{}m old", age / 60)
    } else if age < 86_400 {
        format!("{:.1}h old", age as f64 / 3_600.0)
    } else {
        format!("{:.1}d old", age as f64 / 86_400.0)
    };

    format!("{} UTC ({})", format_epoch_utc(secs), age_str)
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
