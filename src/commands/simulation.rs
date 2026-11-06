//! In-sample simulation, parameter tuning, multi-account screening, and dataset radar commands.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::autotuner::FitnessAutoTuner;
use crate::commands::{
    dataset_census_key, format_count, format_snapshot_timestamp, get_cache_dir, get_main_client,
};
use crate::crowd_allocation::current_unix_time;
use crate::models::{AlphaSettings, CensusSnapshot, DatasetEntry};
use crate::screener::MultiAccountScreener;
use crate::taxonomy::{get_curated_candidates, FactorPillar};

/// Screens candidates using configured BRAIN sessions with a bounded work queue.
#[allow(clippy::too_many_arguments)]
pub async fn cmd_screen(
    pillar: Option<String>,
    universe: String,
    min_fitness: f64,
    workers: usize,
    no_tune: bool,
    single_account: bool,
    alternative: bool,
    dataset: Option<String>,
    count: usize,
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

    let candidates = if alternative {
        println!("  Generating alternative data candidate expressions via ExpressionBuilder...");
        let builder = crate::expression_builder::ExpressionBuilder::new()
            .with_universe(&universe)
            .with_delay(1)
            .with_neutralization(crate::expression_builder::NeutralizationGroup::Subindustry);

        let pillar_filter = pillar
            .as_deref()
            .and_then(crate::expression_builder::AlternativePillar::from_str);

        let generated = builder.generate_batch(dataset.as_deref(), pillar_filter, count);

        // Pre-dispatch syntax and schema validation
        let failed_count = generated.iter().filter(|g| !g.is_valid).count();
        if failed_count > 0 {
            anyhow::bail!(
                "Alternative candidate generation failed schema validation: {} invalid expressions",
                failed_count
            );
        }

        println!(
            "  Generated and verified {} alternative candidates across target datasets.",
            generated.len()
        );

        generated
            .into_iter()
            .map(|g| crate::models::CandidateAlpha {
                name: g.name,
                expression: g.expression,
                universe: g.universe,
                decay: g.decay,
                neutralization: g.neutralization,
                truncation: g.truncation,
                pillar: g.pillar,
            })
            .collect()
    } else {
        let factor_pillar = pillar.as_deref().and_then(FactorPillar::from_str);
        get_curated_candidates(factor_pillar, Some(&universe))
    };

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

/// Persistent private simulation ledger record stored in ~/.cache/alphafind/sim_history.jsonl
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SimLedgerEntry {
    pub timestamp: u64,
    pub fingerprint: String,
    pub alpha_id: String,
    pub universe: String,
    pub decay: i32,
    pub neutralization: String,
    pub truncation: f64,
    pub sharpe: f64,
    pub fitness: f64,
    pub returns: f64,
    pub turnover: f64,
    pub margin_bps: f64,
    pub drawdown: f64,
    pub expression: String,
}

/// Computes a deterministic 64-bit hex hash fingerprint of an expression and its simulation settings.
pub fn compute_sim_fingerprint(
    expr: &str,
    universe: &str,
    decay: i32,
    neutralization: &str,
    truncation: f64,
) -> String {
    let normalized = expr.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut write = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    write(normalized.as_bytes());
    write(b"|");
    write(universe.as_bytes());
    write(b"|");
    write(decay.to_string().as_bytes());
    write(b"|");
    write(neutralization.as_bytes());
    write(b"|");
    write(format!("{:.6}", truncation).as_bytes());
    format!("{:016x}", hash)
}

/// Searches ~/.cache/alphafind/sim_history.jsonl for an existing record matching `fingerprint`.
pub fn lookup_sim_ledger(fingerprint: &str) -> Option<SimLedgerEntry> {
    let ledger_path = get_cache_dir().join("sim_history.jsonl");
    if !ledger_path.exists() {
        return None;
    }
    let file = std::fs::File::open(&ledger_path).ok()?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    let mut last_match = None;
    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Ok(entry) = serde_json::from_str::<SimLedgerEntry>(trimmed) {
                if entry.fingerprint == fingerprint {
                    last_match = Some(entry);
                }
            }
        }
    }
    last_match
}

/// Appends a completed simulation record to ~/.cache/alphafind/sim_history.jsonl with 0o600 permissions.
pub fn append_sim_ledger(entry: &SimLedgerEntry) -> Result<()> {
    let ledger_path = get_cache_dir().join("sim_history.jsonl");
    let line = format!("{}\n", serde_json::to_string(entry)?);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger_path)
        .context("Failed to open sim_history.jsonl for appending")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&ledger_path, std::fs::Permissions::from_mode(0o600));
    }
    use std::io::Write;
    file.write_all(line.as_bytes())
        .context("Failed to append entry to sim_history.jsonl")?;
    Ok(())
}

fn get_known_event_fields() -> &'static std::collections::HashSet<String> {
    use std::sync::OnceLock;
    static EVENT_FIELDS: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    EVENT_FIELDS.get_or_init(|| {
        let mut set = std::collections::HashSet::new();
        // Mandatory known event field
        set.insert("anl4_fs_detail_rec_v4_nd_estimate".to_lowercase());

        // Scan data/ directory for fields_*.json with "type": "EVENT"
        let data_dir = std::path::Path::new("data");
        if data_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(data_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && path.extension().is_some_and(|ext| ext == "json")
                        && path
                            .file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with("fields_"))
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(fields) =
                                serde_json::from_str::<Vec<serde_json::Value>>(&content)
                            {
                                for f in fields {
                                    if let (Some(id), Some(ty)) = (
                                        f.get("id").and_then(|v| v.as_str()),
                                        f.get("type").and_then(|v| v.as_str()),
                                    ) {
                                        if ty.eq_ignore_ascii_case("EVENT") {
                                            set.insert(id.to_lowercase());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        set
    })
}

fn extract_identifier_tokens(text: &str) -> std::collections::HashSet<String> {
    let mut tokens = std::collections::HashSet::new();
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                tokens.insert(cur.clone());
                cur.clear();
            }
        }
    }
    if !cur.is_empty() {
        tokens.insert(cur);
    }
    tokens
}

/// Pre-flight validates a FastExpr string for structural/syntax/type errors, unused variables,
/// and illegal event field inputs to time-series operators.
pub fn preflight_validate_expr(expr: &str) -> Result<()> {
    // 1. AST Schema validation
    let validator = crate::models::schema_validator::SchemaValidator::default();
    let report = validator.validate(expr);

    let fatal_errors: Vec<_> = report
        .errors
        .into_iter()
        .filter(|err| {
            !matches!(
                err.code,
                crate::models::schema_validator::DiagnosticCode::ErrUnknownField
                    | crate::models::schema_validator::DiagnosticCode::ErrMissingBackfill
            )
        })
        .collect();

    if !fatal_errors.is_empty() {
        // If the error was a vector/event type mismatch in an operator, return the specific message
        if fatal_errors.iter().any(|e| {
            e.code == crate::models::schema_validator::DiagnosticCode::ErrVecTypeMismatch
                || e.code
                    == crate::models::schema_validator::DiagnosticCode::ErrEventInputNotSupported
                || e.message.to_uppercase().contains("EVENT")
        }) {
            anyhow::bail!("Pre-flight validation failed: Operator does not support EVENT inputs");
        }
        let msg = fatal_errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("Pre-flight validation failed: {}", msg);
    }

    // 2. Unused variable check:
    // If any assigned variable `target = ...;` is not referenced as an identifier token in any
    // subsequent statement or terminal expression, bail immediately.
    let raw_stmts: Vec<&str> = expr
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    for (i, stmt) in raw_stmts.iter().enumerate() {
        if let Some(eq_pos) = stmt.find('=') {
            let before_eq = &stmt[..eq_pos];
            let after_eq = &stmt[eq_pos + 1..];
            if !before_eq.ends_with('!')
                && !before_eq.ends_with('<')
                && !before_eq.ends_with('>')
                && !after_eq.starts_with('=')
            {
                let target = before_eq.trim();
                let is_ident = !target.is_empty()
                    && (target.starts_with(|c: char| c.is_alphabetic() || c == '_'))
                    && target.chars().all(|c| c.is_alphanumeric() || c == '_');
                if is_ident {
                    let subsequent_text = raw_stmts[i + 1..].join(";");
                    let tokens = extract_identifier_tokens(&subsequent_text);
                    if !tokens.contains(target) {
                        anyhow::bail!(
                            "Pre-flight validation failed: Found unused variable \"{}\"",
                            target
                        );
                    }
                }
            }
        }
    }

    // 3. EVENT inputs check:
    // If expr passes known event fields into ts_backfill(, ts_rank(, ts_delta(, ts_decay_linear(,
    // bail immediately with: Pre-flight validation failed: Operator does not support EVENT inputs.
    let event_fields = get_known_event_fields();
    let mut event_vars: std::collections::HashSet<String> = event_fields.clone();

    // Track assigned event variables across statements
    for stmt in &raw_stmts {
        if let Some(eq_pos) = stmt.find('=') {
            let before_eq = &stmt[..eq_pos];
            let after_eq = &stmt[eq_pos + 1..];
            if !before_eq.ends_with('!')
                && !before_eq.ends_with('<')
                && !before_eq.ends_with('>')
                && !after_eq.starts_with('=')
            {
                let target = before_eq.trim().to_lowercase();
                let rhs_lower = after_eq.to_lowercase();
                let is_folded = rhs_lower.contains("vec_avg") || rhs_lower.contains("vec_sum");
                if !is_folded {
                    let rhs_tokens = extract_identifier_tokens(&rhs_lower);
                    if rhs_tokens.iter().any(|t| event_vars.contains(t)) {
                        event_vars.insert(target);
                    }
                }
            }
        }
    }

    let forbidden_ops = ["ts_backfill", "ts_rank", "ts_delta", "ts_decay_linear"];
    let expr_lower = expr.to_lowercase();

    for op in forbidden_ops {
        let op_pattern = format!("{}(", op);
        let mut search_from = 0;
        while let Some(pos) = expr_lower[search_from..].find(&op_pattern) {
            let abs_pos = search_from + pos;
            let is_boundary = abs_pos == 0 || {
                let prev_char = expr_lower[..abs_pos].chars().last().unwrap();
                !prev_char.is_alphanumeric() && prev_char != '_'
            };

            if is_boundary {
                let args_start = abs_pos + op_pattern.len();
                let mut depth = 0;
                let mut arg1_end = args_start;
                for (char_idx, c) in expr_lower[args_start..].char_indices() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            if depth == 0 {
                                arg1_end = args_start + char_idx;
                                break;
                            }
                            depth -= 1;
                        }
                        ',' if depth == 0 => {
                            arg1_end = args_start + char_idx;
                            break;
                        }
                        _ => {}
                    }
                }

                let arg1 = &expr_lower[args_start..arg1_end];
                let is_folded = arg1.contains("vec_avg") || arg1.contains("vec_sum");
                if !is_folded {
                    let arg1_tokens = extract_identifier_tokens(arg1);
                    if arg1_tokens.iter().any(|t| event_vars.contains(t)) {
                        anyhow::bail!(
                            "Pre-flight validation failed: Operator does not support EVENT inputs"
                        );
                    }
                }
            }
            search_from = abs_pos + op_pattern.len();
        }
    }

    Ok(())
}

/// Stability metrics for Neighbor Sensitivity Sweep (Plateau vs. Spike).
#[derive(Debug, Clone, PartialEq)]
pub struct PlateauMetrics {
    pub min_sharpe: f64,
    pub max_sharpe: f64,
    pub mean_sharpe: f64,
    pub std_dev_sharpe: f64,
    pub retention_ratio: f64,
    pub is_plateau: bool,
}

/// Computes plateau stability metrics across an array of neighbor Sharpe ratios.
pub fn compute_plateau_metrics(sharpes: &[f64]) -> PlateauMetrics {
    if sharpes.is_empty() {
        return PlateauMetrics {
            min_sharpe: 0.0,
            max_sharpe: 0.0,
            mean_sharpe: 0.0,
            std_dev_sharpe: 0.0,
            retention_ratio: 0.0,
            is_plateau: false,
        };
    }

    let min_sharpe = sharpes.iter().copied().fold(f64::INFINITY, f64::min);
    let max_sharpe = sharpes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean_sharpe = sharpes.iter().sum::<f64>() / (sharpes.len() as f64);

    let variance = if sharpes.len() > 1 {
        sharpes
            .iter()
            .map(|&s| (s - mean_sharpe).powi(2))
            .sum::<f64>()
            / ((sharpes.len() - 1) as f64)
    } else {
        0.0
    };
    let std_dev_sharpe = variance.sqrt();

    let retention_ratio = if max_sharpe > 0.0 {
        min_sharpe / max_sharpe
    } else {
        0.0
    };

    let is_plateau = min_sharpe >= 1.15 && retention_ratio >= 0.80;

    PlateauMetrics {
        min_sharpe,
        max_sharpe,
        mean_sharpe,
        std_dev_sharpe,
        retention_ratio,
        is_plateau,
    }
}

/// Simulates a single Alpha expression or a batch file directly on MAIN account.
pub async fn cmd_sim(
    expr: Option<String>,
    batch: Option<std::path::PathBuf>,
    universe: String,
    decay: i32,
    neutralization: String,
    truncation: f64,
    workers: usize,
) -> Result<()> {
    // Acquire cross-process lock per AGENTS.md policy
    let _lock = crate::concurrency_lock::ProcessLock::try_acquire()?;

    let client = get_main_client()?;

    if let Some(batch_path) = batch {
        println!(
            "{}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "{}",
            "  AlphaFind Quant Engine — Batch FastExpr Simulation"
                .bold()
                .cyan()
        );
        println!(
            "{}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "  Batch File:   {}",
            batch_path.display().to_string().cyan()
        );
        println!(
            "  Settings:     Universe: {} | Decay: {} | Neutralization: {} | Truncation: {}",
            universe.yellow(),
            decay,
            neutralization.yellow(),
            truncation
        );
        let max_workers = workers.clamp(1, 3);
        println!(
            "  Concurrency:  {} worker(s) in-process (capped at 3)\n",
            max_workers
        );

        let content = std::fs::read_to_string(&batch_path)
            .with_context(|| format!("Failed to read batch file at '{}'", batch_path.display()))?;

        let expressions: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(ToString::to_string)
            .collect();

        if expressions.is_empty() {
            anyhow::bail!(
                "No valid expressions found in batch file '{}'",
                batch_path.display()
            );
        }

        println!(
            "  Pre-flight validating {} expressions...",
            expressions.len()
        );
        for (idx, e) in expressions.iter().enumerate() {
            preflight_validate_expr(e).with_context(|| {
                format!(
                    "Pre-flight validation failed for expression #{} in batch file",
                    idx + 1
                )
            })?;
        }
        println!(
            "  {} All expressions passed pre-flight validation.\n",
            "✅".green()
        );

        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(max_workers));
        let mut tasks = Vec::new();

        for (idx, expr_str) in expressions.into_iter().enumerate() {
            let sem_clone = sem.clone();
            let client_clone = client.clone();
            let universe_clone = universe.clone();
            let neut_clone = neutralization.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = sem_clone.acquire_owned().await.unwrap();
                let fp = compute_sim_fingerprint(&expr_str, &universe_clone, decay, &neut_clone, truncation);
                if let Some(prev) = lookup_sim_ledger(&fp) {
                    println!(
                        "  {}",
                        format!(
                            "ℹ️  [SIM LEDGER] Expression fingerprint already simulated previously -> Alpha ID: {} (Sharpe: {:.2}, Fitness: {:.2}, Turn: {:.1}%)",
                            prev.alpha_id, prev.sharpe, prev.fitness, prev.turnover * 100.0
                        ).yellow()
                    );
                }

                let settings = AlphaSettings {
                    universe: universe_clone.clone(),
                    decay,
                    neutralization: neut_clone.clone(),
                    truncation,
                    ..Default::default()
                };

                println!("  [#{}] Submitting simulation to MAIN account...", idx + 1);
                let sim_id = match client_clone.submit_simulation(&expr_str, &settings).await {
                    Ok(id) => id,
                    Err(e) => return (idx, expr_str, Err(e)),
                };
                println!("  [#{}] Queued (ID: {}). Polling progress...", idx + 1, sim_id.yellow());

                let sim_res = match client_clone.poll_simulation(&sim_id, 350, 5).await {
                    Ok(res) => res,
                    Err(e) => return (idx, expr_str, Err(e)),
                };

                if let Some(alpha_id) = sim_res.alpha {
                    println!("  [#{}] {} Complete! Alpha ID: {}", idx + 1, "✅".green(), alpha_id.green().bold());
                    let details = match client_clone.get_alpha_details(&alpha_id).await {
                        Ok(d) => d,
                        Err(e) => return (idx, expr_str, Err(e)),
                    };
                    if let Some(st) = details.is {
                        let entry = SimLedgerEntry {
                            timestamp: current_unix_time(),
                            fingerprint: fp,
                            alpha_id: alpha_id.clone(),
                            universe: universe_clone,
                            decay,
                            neutralization: neut_clone,
                            truncation,
                            sharpe: st.sharpe.unwrap_or(0.0),
                            fitness: st.fitness.unwrap_or(0.0),
                            returns: st.returns.unwrap_or(0.0),
                            turnover: st.turnover.unwrap_or(0.0),
                            margin_bps: st.margin.unwrap_or(0.0) * 10000.0,
                            drawdown: st.drawdown.unwrap_or(0.0),
                            expression: expr_str.clone(),
                        };
                        let _ = append_sim_ledger(&entry);
                        return (idx, expr_str, Ok(Some(entry)));
                    }
                    (idx, expr_str, Ok(None))
                } else {
                    (idx, expr_str, Ok(None))
                }
            }));
        }

        let mut results = Vec::new();
        for t in tasks {
            results.push(t.await?);
        }
        results.sort_by_key(|r| r.0);

        println!(
            "\n{}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!("{}", "  BATCH SIMULATION SUMMARY".bold().cyan());
        println!(
            "{}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "  ┌─────┬────────────┬──────────┬──────────┬────────────┬────────────┬────────────┬────────────┐"
        );
        println!(
            "  │ {:<3} │ {:<10} │ {:>8} │ {:>8} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │",
            "#", "Alpha ID", "Sharpe", "Fitness", "Return", "Turnover", "Margin", "Drawdown"
        );
        println!(
            "  ├─────┼────────────┼──────────┼──────────┼────────────┼────────────┼────────────┼────────────┤"
        );

        for (idx, _expr, res) in results {
            match res {
                Ok(Some(entry)) => {
                    println!(
                        "  │ {:<3} │ {:<10} │ {:>8.2} │ {:>8.2} │ {:>9.1}% │ {:>9.1}% │ {:>6.1} bps │ {:>9.1}% │",
                        idx + 1,
                        entry.alpha_id.green(),
                        entry.sharpe,
                        entry.fitness,
                        entry.returns * 100.0,
                        entry.turnover * 100.0,
                        entry.margin_bps,
                        entry.drawdown * 100.0
                    );
                }
                Ok(None) => {
                    println!(
                        "  │ {:<3} │ {:<10} │ {:>8} │ {:>8} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │",
                        idx + 1,
                        "NO ALPHA".yellow(),
                        "—",
                        "—",
                        "—",
                        "—",
                        "—",
                        "—"
                    );
                }
                Err(e) => {
                    println!(
                        "  │ {:<3} │ {:<10} │ {:>8} │ {:>8} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │",
                        idx + 1,
                        "FAILED".red(),
                        "—",
                        "—",
                        "—",
                        "—",
                        "—",
                        format!("{:.8}", e)
                    );
                }
            }
        }
        println!(
            "  └─────┴────────────┴──────────┴──────────┴────────────┴────────────┴────────────┴────────────┘"
        );

        return Ok(());
    }

    // Single expression simulation
    let expr =
        expr.ok_or_else(|| anyhow::anyhow!("Either --expr (-e) or --batch (-b) must be provided"))?;

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

    // Pre-flight validation
    preflight_validate_expr(&expr)?;

    let fp = compute_sim_fingerprint(&expr, &universe, decay, &neutralization, truncation);
    if let Some(prev) = lookup_sim_ledger(&fp) {
        println!(
            "  {}",
            format!(
                "ℹ️  [SIM LEDGER] Expression fingerprint already simulated previously -> Alpha ID: {} (Sharpe: {:.2}, Fitness: {:.2}, Turn: {:.1}%)",
                prev.alpha_id, prev.sharpe, prev.fitness, prev.turnover * 100.0
            ).yellow()
        );
    }

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
            let entry = SimLedgerEntry {
                timestamp: current_unix_time(),
                fingerprint: fp,
                alpha_id: alpha_id.clone(),
                universe,
                decay,
                neutralization,
                truncation,
                sharpe: st.sharpe.unwrap_or(0.0),
                fitness: st.fitness.unwrap_or(0.0),
                returns: st.returns.unwrap_or(0.0),
                turnover: st.turnover.unwrap_or(0.0),
                margin_bps: st.margin.unwrap_or(0.0) * 10000.0,
                drawdown: st.drawdown.unwrap_or(0.0),
                expression: expr,
            };
            let _ = append_sim_ledger(&entry);

            println!("\n  📊 In-Sample Statistics:");
            println!("    Sharpe:      {:.2}", entry.sharpe);
            println!("    Fitness:     {:.2}", entry.fitness);
            println!("    Annual Ret:  {:.2}%", entry.returns * 100.0);
            println!("    Turnover:    {:.2}%", entry.turnover * 100.0);
            println!("    Margin:      {:.2} bps", entry.margin_bps);
            println!("    Drawdown:    {:.2}%", entry.drawdown * 100.0);
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
                format_snapshot_timestamp(&prev.timestamp, current_unix_time()).cyan()
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

/// Runs a neighbor sensitivity sweep over a template expression to verify Plateau vs. Spike stability.
#[allow(clippy::too_many_arguments)]
pub async fn cmd_sweep(
    template: String,
    values: Vec<String>,
    placeholder: String,
    universe: String,
    decay: i32,
    neutralization: String,
    truncation: f64,
    workers: usize,
) -> Result<()> {
    // 1. Verify placeholder exists in template
    let target_placeholder = format!("{{{}}}", placeholder);
    if !template.contains(&target_placeholder) {
        anyhow::bail!(
            "Template expression does not contain placeholder \"{}\"",
            target_placeholder
        );
    }

    if values.is_empty() {
        anyhow::bail!("No parameter values provided for sensitivity sweep.");
    }

    // 2. Expand template and pre-flight validate all expressions upfront
    let mut expanded_exprs = Vec::with_capacity(values.len());
    for val in &values {
        let expanded = template.replace(&target_placeholder, val.trim());
        preflight_validate_expr(&expanded).with_context(|| {
            format!(
                "Pre-flight validation failed for expanded value \"{}\"",
                val.trim()
            )
        })?;
        expanded_exprs.push((val.clone(), expanded));
    }

    // 3. Acquire cross-process lock per AGENTS.md policy
    let _lock = crate::concurrency_lock::ProcessLock::try_acquire()?;

    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Neighbor Sensitivity Sweep (Plateau vs Spike)"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!("  Template:     {}", template.cyan());
    println!("  Placeholder:  {}", target_placeholder.yellow());
    println!("  Values ({}):   {:?}", values.len(), values);
    println!(
        "  Settings:     Universe: {} | Decay: {} | Neutralization: {} | Truncation: {}",
        universe.yellow(),
        decay,
        neutralization.yellow(),
        truncation
    );
    let max_workers = workers.clamp(1, 3);
    println!(
        "  Workers:      {} (in-process concurrency, max 3)\n",
        max_workers
    );

    let client = get_main_client()?;
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(max_workers));
    let mut tasks = Vec::new();

    for (idx, (val, expr_str)) in expanded_exprs.into_iter().enumerate() {
        let sem_clone = sem.clone();
        let client_clone = client.clone();
        let universe_clone = universe.clone();
        let neut_clone = neutralization.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire_owned().await.unwrap();
            let fp = compute_sim_fingerprint(&expr_str, &universe_clone, decay, &neut_clone, truncation);
            if let Some(prev) = lookup_sim_ledger(&fp) {
                println!(
                    "  {}",
                    format!(
                        "ℹ️  [SIM LEDGER] Expression fingerprint already simulated previously -> Alpha ID: {} (Sharpe: {:.2}, Fitness: {:.2}, Turn: {:.1}%)",
                        prev.alpha_id, prev.sharpe, prev.fitness, prev.turnover * 100.0
                    ).yellow()
                );
            }

            let settings = AlphaSettings {
                universe: universe_clone.clone(),
                decay,
                neutralization: neut_clone.clone(),
                truncation,
                ..Default::default()
            };

            println!("  [Param: {}] Submitting simulation...", val.yellow());
            let sim_id = match client_clone.submit_simulation(&expr_str, &settings).await {
                Ok(id) => id,
                Err(e) => return (idx, val, expr_str, Err(e)),
            };

            let sim_res = match client_clone.poll_simulation(&sim_id, 350, 5).await {
                Ok(res) => res,
                Err(e) => return (idx, val, expr_str, Err(e)),
            };

            if let Some(alpha_id) = sim_res.alpha {
                println!("  [Param: {}] Complete! Alpha ID: {}", val.yellow(), alpha_id.green().bold());
                let details = match client_clone.get_alpha_details(&alpha_id).await {
                    Ok(d) => d,
                    Err(e) => return (idx, val, expr_str, Err(e)),
                };
                if let Some(st) = details.is {
                    let entry = SimLedgerEntry {
                        timestamp: current_unix_time(),
                        fingerprint: fp,
                        alpha_id: alpha_id.clone(),
                        universe: universe_clone,
                        decay,
                        neutralization: neut_clone,
                        truncation,
                        sharpe: st.sharpe.unwrap_or(0.0),
                        fitness: st.fitness.unwrap_or(0.0),
                        returns: st.returns.unwrap_or(0.0),
                        turnover: st.turnover.unwrap_or(0.0),
                        margin_bps: st.margin.unwrap_or(0.0) * 10000.0,
                        drawdown: st.drawdown.unwrap_or(0.0),
                        expression: expr_str.clone(),
                    };
                    let _ = append_sim_ledger(&entry);
                    return (idx, val, expr_str, Ok(Some(entry)));
                }
                (idx, val, expr_str, Ok(None))
            } else {
                (idx, val, expr_str, Ok(None))
            }
        }));
    }

    let mut sweep_results = Vec::new();
    for t in tasks {
        sweep_results.push(t.await?);
    }
    // Preserve order of values
    sweep_results.sort_by_key(|r| r.0);

    println!(
        "\n{}",
        "  NEIGHBOR SENSITIVITY SWEEP (Plateau vs. Spike)"
            .bold()
            .cyan()
    );
    println!(
        "  ┌──────────────┬────────────┬──────────┬──────────┬────────────┬────────────┬────────────┬────────────┐"
    );
    println!(
        "  │ {:<12} │ {:<10} │ {:>8} │ {:>8} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │",
        "Param", "Alpha ID", "Sharpe", "Fitness", "Return", "Turnover", "Margin", "Drawdown"
    );
    println!(
        "  ├──────────────┼────────────┼──────────┼──────────┼────────────┼────────────┼────────────┼────────────┤"
    );

    let mut sharpes = Vec::new();

    for (_idx, val, _expr, res) in &sweep_results {
        match res {
            Ok(Some(entry)) => {
                sharpes.push(entry.sharpe);
                println!(
                    "  │ {:<12} │ {:<10} │ {:>8.2} │ {:>8.2} │ {:>9.1}% │ {:>9.1}% │ {:>6.1} bps │ {:>9.1}% │",
                    val.yellow(),
                    entry.alpha_id.green(),
                    entry.sharpe,
                    entry.fitness,
                    entry.returns * 100.0,
                    entry.turnover * 100.0,
                    entry.margin_bps,
                    entry.drawdown * 100.0
                );
            }
            Ok(None) => {
                println!(
                    "  │ {:<12} │ {:<10} │ {:>8} │ {:>8} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │",
                    val.yellow(),
                    "NO ALPHA",
                    "—",
                    "—",
                    "—",
                    "—",
                    "—",
                    "—"
                );
            }
            Err(e) => {
                println!(
                    "  │ {:<12} │ {:<10} │ {:>8} │ {:>8} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │",
                    val.yellow(),
                    "FAILED".red(),
                    "—",
                    "—",
                    "—",
                    "—",
                    "—",
                    format!("{:.8}", e)
                );
            }
        }
    }
    println!(
        "  └──────────────┴────────────┴──────────┴──────────┴────────────┴────────────┴────────────┴────────────┘"
    );

    // Compute Plateau Stability Diagnostics
    let metrics = compute_plateau_metrics(&sharpes);

    println!("\n  PLATEAU STABILITY DIAGNOSTICS:");
    println!("    Min Sharpe:            {:.2}", metrics.min_sharpe);
    println!("    Max Sharpe:            {:.2}", metrics.max_sharpe);
    println!("    Mean Sharpe:           {:.2}", metrics.mean_sharpe);
    println!("    StdDev Sharpe:         {:.3}", metrics.std_dev_sharpe);
    println!(
        "    Retention Ratio:       {:.2}% (Min / Max)",
        metrics.retention_ratio * 100.0
    );

    println!("\n  VERDICT:");
    if metrics.is_plateau {
        println!(
            "  {}",
            "✅ [PLATEAU CONFIRMED] Neighbor parameters form a stable plateau (low overfit risk)."
                .green()
                .bold()
        );
    } else {
        println!(
            "  {}",
            "⚠️ [SPIKE / OVERFIT WARNING] Sharpe drops significantly at neighbor values; inspect parameter sensitivity."
                .yellow()
                .bold()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preflight_validate_expr_valid_single() {
        assert!(preflight_validate_expr("ts_rank(close, 10)").is_ok());
    }

    #[test]
    fn test_preflight_validate_expr_valid_multi() {
        assert!(preflight_validate_expr("x = ts_rank(close, 10); ts_decay_linear(x, 5)").is_ok());
    }

    #[test]
    fn test_preflight_validate_expr_unused_variable() {
        let err = preflight_validate_expr("x = ts_rank(close, 10); ts_decay_linear(close, 5)")
            .unwrap_err();
        assert!(err.to_string().contains("Found unused variable \"x\""));
    }

    #[test]
    fn test_preflight_validate_expr_event_in_ts_rank() {
        let err =
            preflight_validate_expr("ts_rank(anl4_fs_detail_rec_v4_nd_estimate, 10)").unwrap_err();
        assert!(err
            .to_string()
            .contains("Operator does not support EVENT inputs"));
    }

    #[test]
    fn test_preflight_validate_expr_event_in_ts_backfill() {
        let err = preflight_validate_expr("ts_backfill(anl4_fs_detail_rec_v4_nd_estimate, 5)")
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("Operator does not support EVENT inputs"));
    }

    #[test]
    fn test_preflight_validate_expr_event_in_ts_delta() {
        let err =
            preflight_validate_expr("ts_delta(anl4_fs_detail_rec_v4_nd_estimate, 5)").unwrap_err();
        assert!(err
            .to_string()
            .contains("Operator does not support EVENT inputs"));
    }

    #[test]
    fn test_preflight_validate_expr_event_in_ts_decay_linear() {
        let err = preflight_validate_expr("ts_decay_linear(anl4_fs_detail_rec_v4_nd_estimate, 5)")
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("Operator does not support EVENT inputs"));
    }

    #[test]
    fn test_preflight_validate_expr_event_alias() {
        let err =
            preflight_validate_expr("x = anl4_fs_detail_rec_v4_nd_estimate; ts_decay_linear(x, 5)")
                .unwrap_err();
        assert!(err
            .to_string()
            .contains("Operator does not support EVENT inputs"));
    }

    #[test]
    fn test_compute_sim_fingerprint_determinism_and_normalization() {
        let fp1 =
            compute_sim_fingerprint("  ts_rank(close,  10)  ", "TOP1000", 5, "SUBINDUSTRY", 0.05);
        let fp2 = compute_sim_fingerprint("ts_rank(close, 10)", "TOP1000", 5, "SUBINDUSTRY", 0.05);
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 16);

        let fp3 = compute_sim_fingerprint("ts_rank(close, 10)", "TOP200", 5, "SUBINDUSTRY", 0.05);
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn test_template_expansion_and_validation() {
        let template = "ts_rank(close, {H})";
        let placeholder = "H";
        let values = vec!["10".to_string(), "20".to_string()];
        let target_ph = format!("{{{}}}", placeholder);
        assert!(template.contains(&target_ph));

        for v in values {
            let expanded = template.replace(&target_ph, &v);
            assert!(preflight_validate_expr(&expanded).is_ok());
        }
    }

    #[test]
    fn test_compute_plateau_metrics_plateau() {
        let sharpes = vec![1.40, 1.45, 1.50, 1.42];
        let m = compute_plateau_metrics(&sharpes);
        assert_eq!(m.min_sharpe, 1.40);
        assert_eq!(m.max_sharpe, 1.50);
        assert!((m.retention_ratio - (1.40 / 1.50)).abs() < 1e-4);
        assert!(m.is_plateau);
    }

    #[test]
    fn test_compute_plateau_metrics_spike() {
        let sharpes = vec![0.80, 1.80, 0.90];
        let m = compute_plateau_metrics(&sharpes);
        assert_eq!(m.min_sharpe, 0.80);
        assert_eq!(m.max_sharpe, 1.80);
        assert!(!m.is_plateau);
    }

    #[test]
    fn test_compute_plateau_metrics_empty() {
        let m = compute_plateau_metrics(&[]);
        assert_eq!(m.min_sharpe, 0.0);
        assert!(!m.is_plateau);
    }
}
