//! Out-of-sample portfolio audit, merge simulation, impact analysis, and health commands.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::{get_cache_dir, get_main_client, load_pnl_cache, print_cache_status};
use crate::correlation::{audit_candidate, calculate_portfolio_impact, simulate_portfolio};
use crate::health::{
    evaluate_portfolio_health, print_detailed_alpha_table, print_health_report,
    print_single_alpha_diagnostic,
};
use crate::models::PortfolioAlpha;

/// Audits Pearson correlation over overlapping local PnL dates against active OS Alphas.
pub async fn cmd_audit(alpha_id: String, label: Option<String>) -> Result<()> {
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
    let (mut pnl_cache, cache_age) = load_pnl_cache();
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    print_cache_status(cache_age, pnl_cache.len());

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
    let max_str = match report.max_correlation {
        Some(m) => format!("{:+.4} ({:+.2}%)", m, m * 100.0),
        None => "N/A (insufficient overlap)".to_string(),
    };
    let avg_str = match report.avg_correlation {
        Some(a) => format!("{:+.4} ({:+.2}%)", a, a * 100.0),
        None => "N/A".to_string(),
    };
    let partner = report.most_correlated_id.as_deref().unwrap_or("none");
    println!(
        "  MAX INTERNAL CORRELATION: {} vs {}",
        max_str,
        partner.yellow()
    );
    println!("  AVERAGE INTERNAL CORR:   {}", avg_str);
    if report.insufficient_overlap_count > 0 {
        println!(
            "  INSUFFICIENT OVERLAP:    {} alphas skipped (<30 common days)",
            report.insufficient_overlap_count.to_string().yellow()
        );
    }

    if report.valid_comparisons == 0 {
        println!(
            "\n  {} No active OS alphas had >= 30 common trading days. Compliance cannot be confirmed (fail-closed).",
            "⚠️  [INSUFFICIENT DATA]".yellow().bold()
        );
    } else if report.is_true_orthogonal {
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

/// Simulates merged multi-alpha Out-of-Sample portfolio Sharpe and diversification metrics.
pub async fn cmd_portfolio() -> Result<()> {
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

    let (mut pnl_cache, cache_age) = load_pnl_cache();
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    print_cache_status(cache_age, pnl_cache.len());

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

/// Simulates exact portfolio merge impact for a candidate Alpha.
pub async fn cmd_impact(alpha_id: String) -> Result<()> {
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
    let (pnl_cache, cache_age) = load_pnl_cache();
    if pnl_cache.is_empty() {
        anyhow::bail!("portfolio_pnl.json not found or empty! Run 'alphafind portfolio' first.");
    }
    print_cache_status(cache_age, pnl_cache.len());

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

/// Diagnoses active OS portfolio health, alpha decay ratios, half-life, and marginal Sharpe impact.
pub async fn cmd_health(alpha_id: Option<String>, export: Option<String>, all: bool) -> Result<()> {
    let port_path = "portfolio_os.json";
    if !Path::new(port_path).exists() {
        anyhow::bail!("portfolio_os.json not found! Run 'alphafind sync' first.");
    }
    let port_data = fs::read_to_string(port_path)?;
    let os_alphas: Vec<PortfolioAlpha> = serde_json::from_str(&port_data)?;

    let (mut pnl_cache, cache_age) = load_pnl_cache();
    let cache_file = get_cache_dir().join("portfolio_pnl.json");
    print_cache_status(cache_age, pnl_cache.len());

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
