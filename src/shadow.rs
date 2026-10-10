use crate::correlation::pearson_correlation;
use crate::models::PortfolioAlpha;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Classification of candidate Alpha's phase shift relative to the platform crowd PnL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UniquenessPhaseVerdict {
    /// Highly negative crowd correlation; aggressively pulls down portfolio uniquenessScore
    AdversarialNegativeHedge,
    /// Orthogonal/uncorrelated; safely expands portfolio diversification and lowers uniquenessScore
    OrthogonalSanctuary,
    /// Marginal co-movement; neutral impact on portfolio uniquenessScore
    NeutralBuffer,
    /// Strongly co-moves with crowd; increases portfolio uniquenessScore (DANGEROUS)
    DangerousCrowdClone,
}

impl UniquenessPhaseVerdict {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::AdversarialNegativeHedge => {
                "💎 ADVERSARIAL HEDGE (Negative Crowd Beta — Suppresses uniquenessScore)"
                    .cyan()
                    .bold()
            }
            Self::OrthogonalSanctuary => {
                "🟢 ORTHOGONAL SANCTUARY (Zero Crowd Co-movement — High Uniqueness)"
                    .green()
                    .bold()
            }
            Self::NeutralBuffer => "🟡 NEUTRAL BUFFER (Marginal Co-movement)".yellow(),
            Self::DangerousCrowdClone => {
                "🔴 CROWD CLONE (Co-moves with Crowd — Surges uniquenessScore)"
                    .red()
                    .bold()
            }
        }
    }
}

/// Master Synthetic Crowd Shadow Portfolio representing aggregate platform quant PnL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrowdShadowPortfolio {
    /// Official ground truth correlation with user portfolio (0.54)
    pub anchor_uniqueness: f64,
    /// Number of common trading days in the backtest time series
    pub n_trading_days: usize,
    /// Percentage weights of the decomposed crowd factor pillars
    pub basis_weights: HashMap<String, f64>,
    /// Master daily PnL time series of the synthetic crowd
    pub shadow_pnl: HashMap<String, f64>,
}

/// Detailed simulation report evaluating candidate Alpha's impact on platform uniquenessScore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniquenessImpactReport {
    pub candidate_id: String,
    pub cand_crowd_corr: f64,
    pub cand_internal_corr: f64,
    pub current_uniqueness: f64,
    pub projected_uniqueness: f64,
    pub delta_uniqueness: f64,
    pub verdict: UniquenessPhaseVerdict,
    pub recommendation: String,
}

/// Audit report measuring crowd exposure across all active Out-of-Sample alphas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioCrowdAuditReport {
    pub active_alphas_count: usize,
    pub baseline_crowd_corr: f64,
    pub highest_crowd_alphas: Vec<(String, String, f64)>, // (id, name, corr)
    pub lowest_crowd_alphas: Vec<(String, String, f64)>,  // (id, name, corr)
    pub pillar_crowd_exposures: HashMap<String, f64>,
}

/// Calibrates the Synthetic Crowd Shadow Portfolio against user's active portfolio PnLs.
/// Uses the official ground truth anchor (0.54) to synthesize the master crowd PnL curve.
pub fn calibrate_shadow_portfolio(
    portfolio_alphas: &[PortfolioAlpha],
    os_pnls: &HashMap<String, HashMap<String, f64>>,
    target_anchor_corr: Option<f64>,
) -> Option<CrowdShadowPortfolio> {
    if os_pnls.is_empty() {
        return None;
    }

    let target_rho = target_anchor_corr.unwrap_or(0.54);

    // 1. Determine common dates across all active alphas
    let mut date_sets: Vec<HashSet<String>> = Vec::new();
    for pnl in os_pnls.values() {
        date_sets.push(pnl.keys().cloned().collect());
    }

    if date_sets.is_empty() {
        return None;
    }

    let mut common_dates = date_sets[0].clone();
    for s in &date_sets[1..] {
        common_dates = common_dates.intersection(s).cloned().collect();
    }

    let mut dates: Vec<String> = common_dates.into_iter().collect();
    dates.sort();
    let n_days = dates.len();
    if n_days < 60 {
        return None;
    }

    let n_days_f64 = n_days as f64;

    // 2. Compute User's Merged Daily PnL Vector: X(t)
    let n_alphas = os_pnls.len() as f64;
    let mut x_daily = Vec::with_capacity(n_days);
    for d in &dates {
        let sum_d: f64 = os_pnls.values().map(|p| p[d]).sum();
        x_daily.push(sum_d / n_alphas);
    }

    let mean_x = x_daily.iter().sum::<f64>() / n_days_f64;
    let var_x = x_daily.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>() / (n_days_f64 - 1.0);
    let std_x = var_x.sqrt();
    if std_x < 1e-9 {
        return None;
    }

    // Standardized User Vector: x_norm
    let x_norm: Vec<f64> = x_daily.iter().map(|v| (v - mean_x) / std_x).collect();

    // 3. Cluster Active Alphas by Factor Pillar to extract Crowd Factor Basis
    let mut pillar_groups: HashMap<String, Vec<&HashMap<String, f64>>> = HashMap::new();
    for alpha in portfolio_alphas {
        if let Some(pnl) = os_pnls.get(&alpha.id) {
            let cat = alpha
                .category
                .as_deref()
                .unwrap_or("MOMENTUM")
                .to_uppercase();
            let pillar = if cat.contains("FUNDAMENTAL") {
                "FUNDAMENTAL"
            } else if cat.contains("ANALYST") {
                "ANALYST"
            } else if cat.contains("OPTION") || cat.contains("VOLATILITY") {
                "OPTIONS"
            } else {
                "MOMENTUM"
            };
            pillar_groups
                .entry(pillar.to_string())
                .or_default()
                .push(pnl);
        }
    }

    // Census prior weights (reflecting 88k quants on fundamental, 49k on analyst, etc.)
    let mut basis_weights = HashMap::new();
    basis_weights.insert("FUNDAMENTAL".to_string(), 0.50);
    basis_weights.insert("ANALYST".to_string(), 0.25);
    basis_weights.insert("OPTIONS".to_string(), 0.15);
    basis_weights.insert("MOMENTUM".to_string(), 0.10);

    // Compute raw empirical crowd composite: Y(t)
    let mut y_raw = vec![0.0; n_days];
    for (t, d) in dates.iter().enumerate() {
        let mut daily_val = 0.0;
        let mut total_weight = 0.0;

        for (pillar, weight) in &basis_weights {
            if let Some(pnls) = pillar_groups.get(pillar) {
                if !pnls.is_empty() {
                    let avg_p: f64 = pnls.iter().map(|p| p[d]).sum::<f64>() / (pnls.len() as f64);
                    daily_val += weight * avg_p;
                    total_weight += weight;
                }
            }
        }

        // If some pillars were absent, supplement with normalized market proxy
        if total_weight < 0.99 {
            daily_val += (1.0 - total_weight) * x_daily[t];
        }
        y_raw[t] = daily_val;
    }

    let mean_y = y_raw.iter().sum::<f64>() / n_days_f64;
    let var_y = y_raw.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>() / (n_days_f64 - 1.0);
    let std_y = var_y.sqrt();
    let y_norm: Vec<f64> = if std_y > 1e-9 {
        y_raw.iter().map(|v| (v - mean_y) / std_y).collect()
    } else {
        x_norm.clone()
    };

    // 4. Exact Closed-Form Calibration:
    // Decompose Y = r0 * X + sqrt(1 - r0^2) * Z where Z is orthogonal to X
    let r0: f64 = x_norm
        .iter()
        .zip(y_norm.iter())
        .map(|(a, b)| a * b)
        .sum::<f64>()
        / (n_days_f64 - 1.0);

    let z_norm: Vec<f64> = if (1.0 - r0.powi(2)).abs() > 1e-6 {
        let scale = (1.0 - r0.powi(2)).sqrt();
        x_norm
            .iter()
            .zip(y_norm.iter())
            .map(|(&x, &y)| (y - r0 * x) / scale)
            .collect()
    } else {
        // Deterministic synthetic orthogonal perturbation
        (0..n_days)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect()
    };

    // Construct Master Synthetic Crowd PnL vector: P_crowd = target_rho * X + sqrt(1 - target_rho^2) * Z
    let clamped_target = target_rho.clamp(-0.99, 0.99);
    let alpha_scale = clamped_target;
    let beta_scale = (1.0 - clamped_target.powi(2)).sqrt();

    let mut shadow_pnl = HashMap::new();
    for (t, d) in dates.iter().enumerate() {
        let crowd_val = alpha_scale * x_norm[t] + beta_scale * z_norm[t];
        shadow_pnl.insert(d.clone(), crowd_val);
    }

    Some(CrowdShadowPortfolio {
        anchor_uniqueness: target_rho,
        n_trading_days: n_days,
        basis_weights,
        shadow_pnl,
    })
}

/// Projects the exact change in platform uniquenessScore (Delta uniqueness) when adding a candidate Alpha.
pub fn project_uniqueness_impact(
    candidate_id: &str,
    candidate_pnl: &HashMap<String, f64>,
    _portfolio_alphas: &[PortfolioAlpha],
    os_pnls: &HashMap<String, HashMap<String, f64>>,
    shadow_portfolio: &CrowdShadowPortfolio,
) -> Option<UniquenessImpactReport> {
    if os_pnls.is_empty() {
        return None;
    }

    // 1. Candidate correlation with Crowd
    let (cand_crowd_corr, common_days) =
        pearson_correlation(candidate_pnl, &shadow_portfolio.shadow_pnl);
    if common_days < 30 {
        return None;
    }

    // 2. Build User Portfolio PnL (excluding candidate if already present)
    let baseline_pnls: Vec<&HashMap<String, f64>> = os_pnls
        .iter()
        .filter(|(id, _)| *id != candidate_id)
        .map(|(_, pnl)| pnl)
        .collect();

    if baseline_pnls.is_empty() {
        return None;
    }

    let mut user_merged_pnl = HashMap::new();
    let sample_dates = baseline_pnls[0].keys();
    let n_base = baseline_pnls.len() as f64;

    for d in sample_dates {
        let mut sum = 0.0;
        let mut count = 0;
        for p in &baseline_pnls {
            if let Some(val) = p.get(d) {
                sum += val;
                count += 1;
            }
        }
        if count > 0 {
            user_merged_pnl.insert(d.clone(), sum / (count as f64));
        }
    }

    // 3. Candidate correlation with internal portfolio
    let (cand_internal_corr, _) = pearson_correlation(candidate_pnl, &user_merged_pnl);

    // 4. Build New Combined Portfolio: P_new = (N * P_user + P_cand) / (N + 1)
    let mut new_merged_pnl = HashMap::new();
    let new_count = n_base + 1.0;

    for (d, &u_val) in &user_merged_pnl {
        if let Some(&c_val) = candidate_pnl.get(d) {
            new_merged_pnl.insert(d.clone(), (n_base * u_val + c_val) / new_count);
        }
    }

    // 5. Projected Uniqueness with Crowd
    let (projected_uniqueness, _) =
        pearson_correlation(&new_merged_pnl, &shadow_portfolio.shadow_pnl);
    let current_uniqueness = shadow_portfolio.anchor_uniqueness;
    let delta_uniqueness = projected_uniqueness - current_uniqueness;

    // 6. Formulate Phase Shift Verdict & Quantitative Recommendation
    let (verdict, rec) = if cand_crowd_corr < -0.15 || delta_uniqueness <= -0.003 {
        (
            UniquenessPhaseVerdict::AdversarialNegativeHedge,
            "APEX CANDIDATE: Directly counters crowd momentum! Submit to aggressively collapse uniquenessScore.".to_string(),
        )
    } else if cand_crowd_corr <= 0.15 || delta_uniqueness <= -0.001 {
        (
            UniquenessPhaseVerdict::OrthogonalSanctuary,
            "HIGH PRIORITY: Orthogonal flow. Diversifies portfolio and suppresses co-movement with platform.".to_string(),
        )
    } else if delta_uniqueness <= 0.001 {
        (
            UniquenessPhaseVerdict::NeutralBuffer,
            "ACCEPTABLE BUFFER: Marginal correlation impact. Acceptable if In-Sample Fitness >= 1.50.".to_string(),
        )
    } else {
        (
            UniquenessPhaseVerdict::DangerousCrowdClone,
            "MORATORIUM WARNING: High crowd co-movement! Submitting will increase platform correlation and damage rank.".to_string(),
        )
    };

    Some(UniquenessImpactReport {
        candidate_id: candidate_id.to_string(),
        cand_crowd_corr,
        cand_internal_corr,
        current_uniqueness,
        projected_uniqueness,
        delta_uniqueness,
        verdict,
        recommendation: rec,
    })
}

/// Audits all active Out-of-Sample alphas against the Synthetic Crowd Shadow Portfolio
pub fn audit_portfolio_crowd_exposure(
    portfolio_alphas: &[PortfolioAlpha],
    os_pnls: &HashMap<String, HashMap<String, f64>>,
    shadow_portfolio: &CrowdShadowPortfolio,
) -> PortfolioCrowdAuditReport {
    let mut alpha_corrs = Vec::new();
    let mut pillar_sums: HashMap<String, (f64, usize)> = HashMap::new();

    for alpha in portfolio_alphas {
        if let Some(pnl) = os_pnls.get(&alpha.id) {
            let (corr, days) = pearson_correlation(pnl, &shadow_portfolio.shadow_pnl);
            if days >= 30 {
                let name = alpha
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unknown Alpha".to_string());
                alpha_corrs.push((alpha.id.clone(), name, corr));

                let cat = alpha
                    .category
                    .as_deref()
                    .unwrap_or("MOMENTUM")
                    .to_uppercase();
                let entry = pillar_sums.entry(cat).or_insert((0.0, 0));
                entry.0 += corr;
                entry.1 += 1;
            }
        }
    }

    // Sort descending by crowd correlation
    alpha_corrs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let highest: Vec<(String, String, f64)> = alpha_corrs.iter().take(5).cloned().collect();
    let lowest: Vec<(String, String, f64)> = alpha_corrs.iter().rev().take(5).cloned().collect();

    let mut pillar_exposures = HashMap::new();
    for (pillar, (sum, count)) in pillar_sums {
        if count > 0 {
            pillar_exposures.insert(pillar, sum / (count as f64));
        }
    }

    PortfolioCrowdAuditReport {
        active_alphas_count: portfolio_alphas.len(),
        baseline_crowd_corr: shadow_portfolio.anchor_uniqueness,
        highest_crowd_alphas: highest,
        lowest_crowd_alphas: lowest,
        pillar_crowd_exposures: pillar_exposures,
    }
}

/// Formats and prints the Master Crowd Shadow Audit Report to console
pub fn print_shadow_audit_report(
    report: &PortfolioCrowdAuditReport,
    shadow_portfolio: &CrowdShadowPortfolio,
) {
    println!("\n═════════════════════════════════════════════════════════════════════════");
    println!(
        "  {}",
        "AlphaFind Quant Engine — Synthetic Crowd Shadow Portfolio & Audit"
            .bold()
            .cyan()
    );
    println!("═════════════════════════════════════════════════════════════════════════");
    println!(
        "  Ground Truth Platform Anchor:   uniquenessScore = {} (Official API)",
        format!("{:.2}", report.baseline_crowd_corr).yellow().bold()
    );
    println!(
        "  Backtest Time-Series Horizon:   {} Trading Days across {} Active OS Alphas",
        shadow_portfolio.n_trading_days.to_string().cyan(),
        report.active_alphas_count.to_string().green().bold()
    );
    println!("  ─────────────────────────────────────────────────────────────────────────");

    println!("\n  👥 CROWD FACTOR BASIS DECOMPOSITION (Census-Calibrated Weights):");
    for (pillar, w) in &shadow_portfolio.basis_weights {
        let bar = "█".repeat((*w * 30.0) as usize);
        println!(
            "    {:<16} : {:5.1}% {}",
            pillar.yellow(),
            w * 100.0,
            bar.cyan()
        );
    }

    println!("\n  🔴 TOP 5 HIGHEST CROWD-CORRELATED ALPHAS (Dragging Uniqueness Down):");
    println!("  ┌──────────┬────────────────────────────────────────────┬─────────────┐");
    println!("  │ Alpha ID │ Name                                       │ Crowd Corr  │");
    println!("  ├──────────┼────────────────────────────────────────────┼─────────────┤");
    for (id, name, corr) in &report.highest_crowd_alphas {
        let display_name = if name.len() > 42 {
            &name[..42]
        } else {
            name.as_str()
        };
        println!(
            "  │ {:<8} │ {:<42} │ {:>+10.2}% │",
            id.yellow(),
            display_name,
            corr * 100.0
        );
    }
    println!("  └──────────┴────────────────────────────────────────────┴─────────────┘");

    println!("\n  🟢 TOP 5 MOST DIVERSIFIED / ORTHOGONAL ALPHAS (Protecting Uniqueness):");
    println!("  ┌──────────┬────────────────────────────────────────────┬─────────────┐");
    println!("  │ Alpha ID │ Name                                       │ Crowd Corr  │");
    println!("  ├──────────┼────────────────────────────────────────────┼─────────────┤");
    for (id, name, corr) in &report.lowest_crowd_alphas {
        let display_name = if name.len() > 42 {
            &name[..42]
        } else {
            name.as_str()
        };
        println!(
            "  │ {:<8} │ {:<42} │ {:>+10.2}% │",
            id.green(),
            display_name,
            corr * 100.0
        );
    }
    println!("  └──────────┴────────────────────────────────────────────┴─────────────┘");

    println!("\n  📊 AVERAGE CROWD EXPOSURE BY FACTOR PILLAR:");
    for (pillar, avg_c) in &report.pillar_crowd_exposures {
        let status = if *avg_c > 0.45 {
            "🔴 HIGH CROWD CO-MOVEMENT (Saturated)".red()
        } else if *avg_c > 0.20 {
            "🟡 MODERATE".yellow()
        } else {
            "🟢 LOW CO-MOVEMENT (Safe Sanctuary)".green()
        };
        println!(
            "    {:<16} : Avg Crowd Corr: {:>+6.2}% | {}",
            pillar.cyan(),
            avg_c * 100.0,
            status
        );
    }

    println!("\n═════════════════════════════════════════════════════════════════════════\n");
}

/// Formats and prints the candidate Uniqueness Impact Report to console
pub fn print_uniqueness_impact_report(report: &UniquenessImpactReport) {
    println!("\n═════════════════════════════════════════════════════════════════════════");
    println!(
        "  {}",
        "AlphaFind Quant Engine — Adversarial Crowd Uniqueness Simulation"
            .bold()
            .yellow()
    );
    println!("═════════════════════════════════════════════════════════════════════════");
    println!(
        "  Candidate Alpha ID:       {}",
        report.candidate_id.cyan().bold()
    );
    println!(
        "  Crowd Co-Movement:        {:>+6.2}% (Correlation vs Entire BRAIN Platform)",
        report.cand_crowd_corr * 100.0
    );
    println!(
        "  Internal Correlation:     {:>+6.2}% (Correlation vs Active OS Portfolio)",
        report.cand_internal_corr * 100.0
    );
    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!(
        "  Current uniquenessScore:  {:.2}",
        report.current_uniqueness
    );
    println!(
        "  Projected uniquenessScore:{:.2}",
        report.projected_uniqueness
    );

    let delta_colored = if report.delta_uniqueness < -0.005 {
        format!("{:>+6.3} (EXCELLENT DECREASE)", report.delta_uniqueness)
            .green()
            .bold()
    } else if report.delta_uniqueness <= 0.002 {
        format!("{:>+6.3} (STABLE)", report.delta_uniqueness).yellow()
    } else {
        format!("{:>+6.3} (DANGEROUS INCREASE)", report.delta_uniqueness)
            .red()
            .bold()
    };

    println!("  Delta Uniqueness (Δ):     {}", delta_colored);
    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!("  Phase Shift Verdict:      {}", report.verdict.badge());
    println!(
        "  Actionable Directive:     {}",
        report.recommendation.white()
    );
    println!("═════════════════════════════════════════════════════════════════════════\n");
}
