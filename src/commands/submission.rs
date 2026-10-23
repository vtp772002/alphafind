//! Submission verification and alpha dispatch command handlers.

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::client::{SubmitStatus, AMBIGUOUS_SUBMISSION_NOTE};
use crate::commands::{
    cache_age_label, get_main_client, load_pnl_cache, load_portfolio_os, write_private_file,
};
use crate::correlation::audit_candidate;
use crate::models::SubmittableAlphaRecord;

/// Updates `submitted` to `true` for `alpha_id` in a `submittable_alphas.csv` ledger.
///
/// Uses structured CSV deserialization/serialization first (where Rust `bool` serializes as
/// lowercase `false`/`true`), and falls back to case-insensitive token replacement if the file
/// has custom columns. Restricts file permissions to `0600` on Unix.
pub fn mark_alpha_submitted_in_csv(csv_path: &Path, alpha_id: &str) -> Result<bool> {
    if !csv_path.exists() {
        return Ok(false);
    }

    if let Ok(mut rdr) = csv::Reader::from_path(csv_path) {
        let parsed: std::result::Result<Vec<SubmittableAlphaRecord>, _> =
            rdr.deserialize::<SubmittableAlphaRecord>().collect();
        if let Ok(mut records) = parsed {
            let mut matched = false;
            for rec in &mut records {
                if rec.alpha_id == alpha_id {
                    rec.submitted = true;
                    matched = true;
                }
            }
            if matched {
                let mut wtr = csv::WriterBuilder::new()
                    .has_headers(true)
                    .from_writer(Vec::new());
                for rec in &records {
                    wtr.serialize(rec)?;
                }
                wtr.flush()?;
                let bytes = wtr.into_inner()?;
                write_private_file(csv_path, bytes)?;
                return Ok(true);
            }
        }
    }

    let content = fs::read_to_string(csv_path)?;
    let mut matched = false;
    let prefix = format!("{},", alpha_id);
    let updated = content
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                let replaced = line
                    .replace(",false,", ",true,")
                    .replace(",False,", ",True,");
                if replaced != line {
                    matched = true;
                }
                replaced
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if matched {
        write_private_file(csv_path, format!("{updated}\n"))?;
    }
    Ok(matched)
}

/// Environment variable that must be set for `--skip-checks` to be honored.
///
/// The platform check gate is the last local safety net before an external,
/// non-reversible submission, so the bypass is opt-in and conspicuous.
pub const SKIP_CHECKS_ENV: &str = "ALPHAFIND_ALLOW_SKIP_CHECKS";

/// Returns whether a raw environment value permits bypassing pre-flight checks.
///
/// Only `1` and `true` (case-insensitive) are accepted; anything else, including a
/// missing variable, keeps the gate closed.
pub fn skip_checks_permitted(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        Some(v) => v == "1" || v.eq_ignore_ascii_case("true"),
        None => false,
    }
}

/// Reads the process environment to decide whether `--skip-checks` may run.
fn skip_checks_allowed() -> bool {
    skip_checks_permitted(std::env::var(SKIP_CHECKS_ENV).ok().as_deref())
}

/// How a platform submission check should be displayed.
///
/// This is presentation only: the pass/fail decision always comes from
/// [`crate::client::evaluate_submission_checks`], which fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckDisplay {
    /// The platform reported an explicit `PASS`.
    Pass,
    /// The platform is still calculating this check.
    Pending,
    /// The platform reported an explicit `FAIL`.
    Fail,
    /// The platform reported an execution error for this check.
    Error,
    /// A status this client does not recognize; the gate fails closed.
    Unknown,
}

impl CheckDisplay {
    /// Classifies a raw platform result string.
    pub fn from_result(result: &str) -> Self {
        match result.trim().to_ascii_uppercase().as_str() {
            "PASS" => Self::Pass,
            "PENDING" => Self::Pending,
            "FAIL" => Self::Fail,
            "ERROR" => Self::Error,
            _ => Self::Unknown,
        }
    }

    /// Label shown in the report table.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Pending => "PENDING",
            Self::Fail => "FAIL",
            Self::Error => "ERROR",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Whether this status alone would satisfy the gate.
    pub fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }
}

/// Renders one row of the check report.
///
/// The label is padded before colors are applied so column alignment survives the
/// ANSI escape sequences.
pub fn format_check_table_row(name: &str, result: &str) -> String {
    let display = CheckDisplay::from_result(result);
    let padded = format!("{:<10}", display.label());
    let styled = match display {
        CheckDisplay::Pass => padded.green().bold().to_string(),
        CheckDisplay::Pending => padded.yellow().bold().to_string(),
        CheckDisplay::Fail | CheckDisplay::Error | CheckDisplay::Unknown => {
            padded.red().bold().to_string()
        }
    };
    format!("  │ {:<28} │ {} │", name, styled)
}

/// Verifies the platform In-Sample submission checks for an Alpha.
///
/// The verdict comes from the fail-closed evaluator: every expected core check must
/// be present and every returned check must be an explicit `PASS`.
pub async fn cmd_check(alpha_id: String) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        format!("  AlphaFind Quant Engine — Submission Check [{}]", alpha_id)
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

    if stats.checks.is_empty() {
        println!(
            "\n  {} No checks were returned by the platform. The gate fails closed; nothing can be submitted from this response.",
            "❌".red()
        );
    } else {
        println!("\n  ┌──────────────────────────────┬────────────┐");
        println!("  │ Check Name                   │ Result     │");
        println!("  ├──────────────────────────────┼────────────┤");
        for c in &stats.checks {
            println!("{}", format_check_table_row(&c.name, &c.result));
        }
        println!("  └──────────────────────────────┴────────────┘");
    }

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
        println!("\n  {} {}", "❌ [FAILED]".red(), summary.red());
    }
    Ok(())
}

/// Submits an Alpha to Out-of-Sample (OS) after pre-flight verification.
pub async fn cmd_submit(
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

    let mut checks_confirmed = false;
    if skip_checks {
        if !skip_checks_allowed() {
            anyhow::bail!(
                "--skip-checks is disabled by default: it bypasses the platform pre-flight gate.\n  \
                 If you deliberately want to bypass it, review the alpha in the BRAIN UI/API first and \
                 re-run with {}=1.",
                SKIP_CHECKS_ENV
            );
        }
        println!(
            "  {} {}",
            "⚠️  [CHECKS BYPASSED]".yellow().bold(),
            format!(
                "--skip-checks is active ({}={}); this submission is NOT check-verified.",
                SKIP_CHECKS_ENV,
                std::env::var(SKIP_CHECKS_ENV).unwrap_or_default()
            )
            .yellow()
        );
    } else {
        println!("  Verifying submission checks on MAIN account...");
        let (passed, msg, _) = client.check_submission(&alpha_id, 30).await?;
        if !passed {
            println!("  {} Cannot submit: {}", "❌".red(), msg.red());
            anyhow::bail!("Alpha failed submission checks");
        }
        checks_confirmed = true;
        println!("  {} Submission checks confirmed!", "✅".green());

        // Separate Local Portfolio-Policy Gate (self-correlation, staleness, overlap)
        if Path::new("portfolio_os.json").exists() {
            println!("  Evaluating local portfolio-policy gate (separate from platform check)...");
            if let Ok(os_alphas) = load_portfolio_os() {
                if !os_alphas.is_empty() {
                    let (pnl_cache, cache_age) = load_pnl_cache();
                    let (age_label, is_stale) = cache_age_label(cache_age);
                    if is_stale {
                        println!(
                            "  {} Local PnL cache is {} (stale or missing). Run 'alphafind portfolio --refresh' to keep local policy checks fresh.",
                            "⚠️  [LOCAL POLICY WARNING]".yellow().bold(),
                            age_label
                        );
                    }
                    if !pnl_cache.is_empty() {
                        if let Ok(cand_pnl) = client.fetch_daily_pnl(&alpha_id).await {
                            let audit = audit_candidate(&alpha_id, &cand_pnl, &pnl_cache);
                            if !audit.is_within_correlation_proxy {
                                anyhow::bail!(
                                    "Local portfolio-policy gate rejected {}: max local correlation ({:?} vs {:?}, valid pairs: {}) failed the < 0.70 fail-closed local check.",
                                    alpha_id,
                                    audit.max_correlation,
                                    audit.most_correlated_id,
                                    audit.valid_comparisons
                                );
                            }
                            println!(
                                "  {} Local portfolio-policy gate passed (max local rho: {:+.2}% across {} active OS alphas).",
                                "✅".green(),
                                audit.max_correlation.unwrap_or(0.0) * 100.0,
                                audit.valid_comparisons
                            );
                        }
                    }
                }
            }
        }
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
    let (status, resp_msg) = client.submit_alpha(&alpha_id).await?;

    match status {
        SubmitStatus::Accepted => {
            println!(
                "  {} {}",
                "🌟 [SUBMITTED SUCCESSFULLY]".green().bold(),
                resp_msg.green()
            );
            if checks_confirmed {
                println!("  Pre-flight checks: verified locally before dispatch.");
            } else {
                println!("  Pre-flight checks: BYPASSED (--skip-checks). Confirm in BRAIN.");
            }
            println!(
                "  Out-of-Sample Tracking URL: https://platform.worldquantbrain.com/alpha/{}",
                alpha_id.cyan()
            );

            // Update submittable_alphas.csv
            let csv_path = Path::new("submittable_alphas.csv");
            if csv_path.exists() {
                match mark_alpha_submitted_in_csv(csv_path, &alpha_id) {
                    Ok(true) => {
                        println!(
                            "  💾 Marked {} as submitted=true in {}",
                            alpha_id.green(),
                            csv_path.display()
                        );
                    }
                    Ok(false) => {}
                    Err(e) => {
                        eprintln!(
                            "  ⚠️  Warning: Failed to update {}: {}",
                            csv_path.display(),
                            e
                        );
                    }
                }
            }
        }
        SubmitStatus::Rejected => {
            println!(
                "  {} Submission dispatch failed: {}",
                "❌".red(),
                resp_msg.red()
            );
        }
        SubmitStatus::Unknown => {
            println!(
                "  {} {}",
                "⚠️  [UNKNOWN OUTCOME]".yellow().bold(),
                resp_msg.yellow()
            );
            println!("  {}", AMBIGUOUS_SUBMISSION_NOTE.yellow());
            anyhow::bail!(
                "Submission outcome unknown; verify the alpha stage in BRAIN before retrying"
            );
        }
    }
    Ok(())
}
