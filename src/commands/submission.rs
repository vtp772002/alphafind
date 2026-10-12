//! Submission verification and alpha dispatch command handlers.

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::commands::get_main_client;

/// Verifies the mandatory In-Sample platform submission checks for an Alpha.
pub async fn cmd_check(alpha_id: String) -> Result<()> {
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
