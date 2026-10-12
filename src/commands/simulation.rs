//! In-sample simulation, parameter tuning, multi-account screening, and dataset radar commands.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::autotuner::FitnessAutoTuner;
use crate::commands::{dataset_census_key, format_count, get_main_client};
use crate::models::{AlphaSettings, CensusSnapshot, DatasetEntry};
use crate::screener::MultiAccountScreener;
use crate::taxonomy::{get_curated_candidates, FactorPillar};

/// Screens candidates using configured BRAIN sessions with a bounded work queue.
pub async fn cmd_screen(
    pillar: Option<String>,
    universe: String,
    min_fitness: f64,
    workers: usize,
    no_tune: bool,
    single_account: bool,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Configured-Session Screener"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let screener = MultiAccountScreener::from_env(workers, single_account)?;
    let factor_pillar = pillar.as_deref().and_then(FactorPillar::from_str);

    let candidates = get_curated_candidates(factor_pillar, Some(&universe));
    println!(
        "  Loaded {} candidate hypotheses for screening.",
        candidates.len()
    );

    let qualified = screener
        .screen_batch(candidates, min_fitness, !no_tune)
        .await?;

    println!(
        "\n{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "  Screening Session Complete: {} candidates marked qualified by local checks. Verify each result in BRAIN before submission.",
        qualified.len()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    for q in qualified {
        println!(
            "    🌟 [{}] {} | Sharpe: {:.2} | Fit: {:.2} | Return: {:.1}%",
            q.alpha_id.green().bold(),
            q.name,
            q.sharpe,
            q.fitness,
            q.annual_return
        );
    }
    Ok(())
}

/// Simulates a single Alpha expression directly on MAIN account.
pub async fn cmd_sim(
    expr: String,
    universe: String,
    decay: i32,
    neutralization: String,
    truncation: f64,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Direct FastExpr Simulation"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    let settings = AlphaSettings {
        universe: universe.clone(),
        decay,
        neutralization: neutralization.clone(),
        truncation,
        ..Default::default()
    };

    println!(
        "  Settings: Universe: {} | Decay: {} | Neutralization: {} | Truncation: {}",
        universe.yellow(),
        decay,
        neutralization.yellow(),
        truncation
    );
    println!("  Expression: {}\n", expr.cyan());

    println!("  Submitting simulation to MAIN account...");
    let sim_id = client.submit_simulation(&expr, &settings).await?;
    println!(
        "  Simulation queued (ID: {}). Polling progress...",
        sim_id.yellow()
    );

    let sim_res = client.poll_simulation(&sim_id, 350, 5).await?;
    if let Some(alpha_id) = sim_res.alpha {
        println!(
            "  {} Simulation Complete! Alpha ID: {}",
            "✅".green(),
            alpha_id.green().bold()
        );
        let details = client.get_alpha_details(&alpha_id).await?;
        if let Some(st) = details.is {
            println!("\n  📊 In-Sample Statistics:");
            println!("    Sharpe:      {:.2}", st.sharpe.unwrap_or(0.0));
            println!("    Fitness:     {:.2}", st.fitness.unwrap_or(0.0));
            println!("    Annual Ret:  {:.2}%", st.returns.unwrap_or(0.0) * 100.0);
            println!(
                "    Turnover:    {:.2}%",
                st.turnover.unwrap_or(0.0) * 100.0
            );
            println!(
                "    Margin:      {:.2} bps",
                st.margin.unwrap_or(0.0) * 10000.0
            );
            println!(
                "    Drawdown:    {:.2}%",
                st.drawdown.unwrap_or(0.0) * 100.0
            );
        }
        println!(
            "\n  Platform Link: https://platform.worldquantbrain.com/alpha/{}",
            alpha_id.cyan()
        );
    } else {
        println!("  {} No Alpha ID returned from simulation.", "⚠️".yellow());
    }
    Ok(())
}

/// Fetches and displays the live dataset usage census.
pub async fn cmd_radar(
    region: String,
    universe_filter: Option<String>,
    top: usize,
    filter: Option<String>,
    green_only: bool,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind — BRAIN Dataset Usage Census".bold().cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!(
        "  Connecting to WorldQuant BRAIN API (Region: {})...",
        region.yellow()
    );

    let datasets = client.fetch_all_datasets(&region).await?;
    println!(
        "  {} Fetched {} datasets from WorldQuant BRAIN Production API.",
        "✅".green(),
        datasets.len().to_string().bold().green()
    );

    // Load previous snapshot if exists for delta tracking
    let data_dir = Path::new("data");
    let _ = fs::create_dir_all(data_dir);
    let snapshot_file = data_dir.join("census_snapshots.json");

    let prev_snapshot: Option<CensusSnapshot> = if snapshot_file.exists() {
        fs::read_to_string(&snapshot_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    };

    let mut prev_map: HashMap<String, (Option<i64>, Option<i64>)> = HashMap::new();
    if let Some(ref prev) = prev_snapshot {
        if prev.region.eq_ignore_ascii_case(&region) {
            println!(
                "  Loaded previous snapshot from {}. Showing count changes since that snapshot.",
                prev.timestamp.cyan()
            );
            for d in &prev.datasets {
                let key = dataset_census_key(d);
                prev_map.insert(key, (d.user_count, d.alpha_count));
            }
        } else {
            println!(
                "  Ignoring previous snapshot for region {}; current region is {}.",
                prev.region, region
            );
        }
    }

    // Save current snapshot
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let current_snapshot = CensusSnapshot {
        timestamp: format!("Unix Epoch {}", secs),
        region: region.clone(),
        total_datasets: datasets.len(),
        datasets: datasets.clone(),
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&current_snapshot) {
        let _ = fs::write(&snapshot_file, json_str);
    }

    // Filter datasets
    let mut filtered: Vec<&DatasetEntry> = datasets.iter().collect();

    // Filter by Universe if provided
    if let Some(ref uf) = universe_filter {
        let uf_upper = uf.to_uppercase();
        filtered.retain(|d| {
            d.universe
                .as_deref()
                .map(|u| u.to_uppercase().contains(&uf_upper))
                .unwrap_or(false)
        });
        println!(
            "  Filtered by Universe: {} ({} matching dataset entries)",
            uf.cyan().bold(),
            filtered.len().to_string().yellow()
        );
    }

    // Keyword filter
    if let Some(ref f) = filter {
        let f_lower = f.to_lowercase();
        filtered.retain(|d| {
            d.id.to_lowercase().contains(&f_lower)
                || d.name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&f_lower)
                || d.subcategory
                    .as_ref()
                    .and_then(|s| s.name.as_deref())
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&f_lower)
        });
    }

    if green_only {
        filtered.retain(|d| d.user_count.is_some_and(|count| (0..300).contains(&count)));
    }

    // Sort reported lower user counts first; unknown and invalid counts go last.
    let mut lower_usage = filtered.clone();
    lower_usage.sort_by_key(|d| match d.user_count {
        Some(count) if count >= 0 => (0, count),
        _ => (1, i64::MAX),
    });

    println!(
        "\n  DATASET USAGE COUNTS (lower reported user counts first; not a Uniqueness forecast):"
    );
    println!("  ┌──────────────┬──────────┬───────────────────────────┬──────────────┬──────────────┬──────────┬────────────┐");
    println!("  │ Dataset ID   │ Universe │ Subcategory               │ Users (API)  │ Alphas (API) │ Coverage │ Status     │");
    println!("  ├──────────────┼──────────┼───────────────────────────┼──────────────┼──────────────┼──────────┼────────────┤");

    for d in lower_usage.iter().take(top) {
        let u_cnt = d.user_count;
        let a_cnt = d.alpha_count;
        let u_name = d.universe.as_deref().unwrap_or("—");
        let cov_str = d
            .coverage
            .map(|c| format!("{:.1}%", c * 100.0))
            .unwrap_or_else(|| "—".to_string());
        let subcat = d
            .subcategory
            .as_ref()
            .and_then(|s| s.name.as_deref())
            .unwrap_or("General");
        let sub_display = if subcat.chars().count() > 25 {
            let truncated: String = subcat.chars().take(22).collect();
            format!("{}...", truncated)
        } else {
            subcat.to_string()
        };

        let status = match u_cnt {
            Some(count) if (0..=50).contains(&count) => "LOW COUNT".green().bold(),
            Some(count) if (0..=300).contains(&count) => "LOW COUNT".green(),
            Some(count) if count >= 0 => "COUNT PRESENT".yellow(),
            _ => "UNKNOWN".dimmed(),
        };

        let key = dataset_census_key(d);
        let delta_str = match (u_cnt, prev_map.get(&key).and_then(|counts| counts.0)) {
            (Some(current), Some(previous)) if current != previous => {
                format!(" ({:+})", current - previous).yellow().to_string()
            }
            _ => String::new(),
        };

        let u_str = format!("{}{}", format_count(u_cnt), delta_str);

        println!(
            "  │ {:<12} │ {:<8} │ {:<25} │ {:>12} │ {:>12} │ {:>8} │ {:<10} │",
            d.id.cyan(),
            u_name.white(),
            sub_display,
            u_str,
            format_count(a_cnt),
            cov_str,
            status
        );
    }
    println!("  └──────────────┴──────────┴───────────────────────────┴──────────────┴──────────────┴──────────┴────────────┘");

    // Display Top Red Zones if not green_only
    if !green_only {
        let mut higher_usage = filtered.clone();
        higher_usage.sort_by_key(|d| match d.user_count {
            Some(count) if count >= 0 => (0, std::cmp::Reverse(count)),
            _ => (1, std::cmp::Reverse(0)),
        });

        println!("\n  HIGHER REPORTED USER COUNTS (usage only; not a PnL-correlation or Uniqueness score):");
        println!("  ┌──────────────┬──────────┬───────────────────────────┬──────────────┬──────────────┬──────────┬────────────┐");
        println!("  │ Dataset ID   │ Universe │ Subcategory               │ Users (API)  │ Alphas (API) │ Coverage │ Status     │");
        println!("  ├──────────────┼──────────┼───────────────────────────┼──────────────┼──────────────┼──────────┼────────────┤");

        for d in higher_usage.iter().take(top.min(8)) {
            let u_cnt = d.user_count;
            let a_cnt = d.alpha_count;
            let u_name = d.universe.as_deref().unwrap_or("—");
            let cov_str = d
                .coverage
                .map(|c| format!("{:.1}%", c * 100.0))
                .unwrap_or_else(|| "—".to_string());
            let subcat = d
                .subcategory
                .as_ref()
                .and_then(|s| s.name.as_deref())
                .unwrap_or("General");
            let sub_display = if subcat.chars().count() > 25 {
                let truncated: String = subcat.chars().take(22).collect();
                format!("{}...", truncated)
            } else {
                subcat.to_string()
            };

            let status = match u_cnt {
                Some(count) if count >= 20000 => "VERY HIGH COUNT".red().bold(),
                Some(count) if count >= 2000 => "HIGH COUNT".red(),
                Some(count) if count >= 0 => "COUNT PRESENT".yellow(),
                _ => "UNKNOWN".dimmed(),
            };

            println!(
                "  │ {:<12} │ {:<8} │ {:<25} │ {:>12} │ {:>12} │ {:>8} │ {:<10} │",
                d.id.yellow(),
                u_name.white(),
                sub_display,
                format_count(u_cnt),
                format_count(a_cnt),
                cov_str,
                status
            );
        }
        println!("  └──────────────┴──────────┴───────────────────────────┴──────────────┴──────────────┴──────────┴────────────┘");
    }

    Ok(())
}

/// Sweeps configured decay, exponent, and neutralization values for a candidate.
pub async fn cmd_tune(
    expr: String,
    universe: String,
    target_fitness: f64,
    decay: i32,
    neutralization: String,
) -> Result<()> {
    let client = get_main_client()?;
    let settings = AlphaSettings {
        universe,
        decay,
        neutralization,
        truncation: 0.065,
        ..Default::default()
    };
    let tuner = FitnessAutoTuner::new(&client);
    let res = tuner
        .tune_hyperparameters(&expr, &settings, target_fitness)
        .await?;

    match res {
        Some(tuned) => {
            println!("\n  🎉 TUNING SUCCESS!");
            println!(
                "    Tuned Strategy: {}",
                tuned.tuning_strategy.green().bold()
            );
            println!("    Alpha ID:       {}", tuned.details.id.yellow().bold());
            println!(
                "    Fitness:        {:.2} -> {:.2}",
                tuned.original_fitness, tuned.tuned_fitness
            );
            if let Some(st) = tuned.details.is.as_ref() {
                println!("    Sharpe:         {:.2}", st.sharpe.unwrap_or(0.0));
                println!(
                    "    Turnover:       {:.1}%",
                    st.turnover.unwrap_or(0.0) * 100.0
                );
            }
            println!(
                "    Settings:       Decay={}, Neut={}",
                tuned.settings.decay, tuned.settings.neutralization
            );
            println!("    Formula:        {}", tuned.expression.cyan());
            println!(
                "    URL:            https://platform.worldquantbrain.com/alpha/{}",
                tuned.details.id
            );
        }
        None => {
            println!(
                "\n  ⚠️ AutoTuner completed: Could not achieve target fitness >= {:.2}.",
                target_fitness
            );
        }
    }
    Ok(())
}
