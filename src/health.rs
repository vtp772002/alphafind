use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::correlation::{pearson_correlation, simulate_portfolio};
use crate::models::PortfolioAlpha;
use crate::shadow::CrowdShadowPortfolio;

/// Lifecycle and decay status of an Out-of-Sample Alpha
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaDecayStatus {
    /// Recent trailing performance is accelerating above historical baseline (> +15%)
    Accelerating,
    /// Stable, robust performance closely tracking historical expectations
    Robust,
    /// Moderate performance decay (-20% to -50% trailing drop)
    ModerateDecay,
    /// Severe performance decay (> -50% drop or negative trailing Sharpe)
    SevereDecay,
}

impl AlphaDecayStatus {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::Accelerating => "🚀 ACCELERATING".green().bold(),
            Self::Robust => "🟢 ROBUST".green(),
            Self::ModerateDecay => "🟡 MODERATE DECAY".yellow(),
            Self::SevereDecay => "🔴 SEVERE DECAY".red().bold(),
        }
    }
}

/// The economic role and portfolio contribution of an Alpha
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaRoleVerdict {
    /// Major portfolio pillar: significantly surges portfolio Sharpe (+0.02+)
    PillarDriver,
    /// Positive contributor: adds diversification and net positive Sharpe
    PositiveContributor,
    /// Neutral diversifier: buffers volatility with minimal Sharpe change
    NeutralDiversifier,
    /// Dilutive drag: slightly dilutes portfolio Sharpe (candidate for replacement)
    DilutiveDrag,
}

impl AlphaRoleVerdict {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::PillarDriver => "🌟 PILLAR DRIVER".cyan().bold(),
            Self::PositiveContributor => "✅ CONTRIBUTOR".green(),
            Self::NeutralDiversifier => "⚪ DIVERSIFIER".white(),
            Self::DilutiveDrag => "⚠️ DILUTIVE DRAG".yellow().bold(),
        }
    }
}

/// Detailed health diagnostic for a single Out-of-Sample Alpha
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaHealthReport {
    pub alpha_id: String,
    pub name: String,
    pub universe: String,
    pub category: String,
    pub full_sharpe: f64,
    pub trailing_1y_sharpe: f64,
    pub decay_ratio: f64,
    pub decay_status: AlphaDecayStatus,
    pub max_internal_corr: f64,
    pub most_correlated_id: String,
    pub safety_buffer_pct: f64,
    pub marginal_sharpe_delta: f64,
    pub role_verdict: AlphaRoleVerdict,
    pub crowd_co_movement: Option<f64>,
}

/// Master Portfolio Health & Alpha Half-Life Diagnostic Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPortfolioHealthReport {
    pub total_alphas: usize,
    pub common_trading_days: usize,
    pub portfolio_sharpe: f64,
    pub portfolio_annualized_pnl: f64,
    pub portfolio_annualized_vol: f64,
    pub avg_pairwise_correlation: f64,
    pub health_score: f64, // 0.0 - 100.0
    pub accelerating_count: usize,
    pub robust_count: usize,
    pub moderate_decay_count: usize,
    pub severe_decay_count: usize,
    pub tight_safety_buffer_count: usize, // buffer < 8.0%
    pub top_pillar_drivers: Vec<AlphaHealthReport>,
    pub at_risk_alphas: Vec<AlphaHealthReport>,
    pub all_alpha_diagnostics: Vec<AlphaHealthReport>,
    pub actionable_recommendations: Vec<String>,
}

/// Computes annualized Sharpe ratio for a single alpha given dates and daily PnL mapping
fn compute_series_sharpe(dates: &[String], pnl: &HashMap<String, f64>) -> f64 {
    if dates.is_empty() {
        return 0.0;
    }
    let n = dates.len() as f64;
    let sum: f64 = dates.iter().filter_map(|d| pnl.get(d)).sum();
    let mean = sum / n;

    let var: f64 = dates
        .iter()
        .filter_map(|d| pnl.get(d))
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0).max(1.0);
    let std = var.sqrt();

    let ann_pnl = mean * 252.0;
    let ann_vol = std * (252.0f64).sqrt();

    if ann_vol > 1e-9 {
        ann_pnl / ann_vol
    } else {
        0.0
    }
}

/// Evaluates the complete health and decay diagnostics across all active Out-of-Sample Alphas
pub fn evaluate_portfolio_health(
    portfolio_alphas: &[PortfolioAlpha],
    os_pnls: &HashMap<String, HashMap<String, f64>>,
    shadow_opt: Option<&CrowdShadowPortfolio>,
) -> Option<MasterPortfolioHealthReport> {
    if portfolio_alphas.is_empty() || os_pnls.is_empty() {
        return None;
    }

    // 1. Determine common dates
    let mut date_sets: Vec<HashSet<String>> = Vec::new();
    for alpha in portfolio_alphas {
        if let Some(pnl) = os_pnls.get(&alpha.id) {
            date_sets.push(pnl.keys().cloned().collect());
        }
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

    // Baseline Full Portfolio Simulation
    let active_pnls: Vec<&HashMap<String, f64>> = portfolio_alphas
        .iter()
        .filter_map(|a| os_pnls.get(&a.id))
        .collect();

    let baseline_metrics = simulate_portfolio(&active_pnls)?;
    let baseline_sharpe = baseline_metrics.merged_sharpe;

    // Trailing 1-year window (252 trading days)
    let trailing_1y_start = if n_days > 252 {
        n_days - 252
    } else {
        n_days / 2
    };
    let trailing_dates = &dates[trailing_1y_start..];

    let mut all_reports = Vec::new();
    let mut accelerating_count = 0;
    let mut robust_count = 0;
    let mut moderate_decay_count = 0;
    let mut severe_decay_count = 0;
    let mut tight_buffer_count = 0;

    for alpha in portfolio_alphas {
        if let Some(pnl) = os_pnls.get(&alpha.id) {
            let full_sh = compute_series_sharpe(&dates, pnl);
            let trailing_sh = compute_series_sharpe(trailing_dates, pnl);

            let decay_ratio = if full_sh.abs() > 0.05 {
                trailing_sh / full_sh
            } else {
                1.0
            };

            let decay_status = if trailing_sh > full_sh * 1.15 && trailing_sh > 1.25 {
                accelerating_count += 1;
                AlphaDecayStatus::Accelerating
            } else if decay_ratio >= 0.75 {
                robust_count += 1;
                AlphaDecayStatus::Robust
            } else if decay_ratio >= 0.40 {
                moderate_decay_count += 1;
                AlphaDecayStatus::ModerateDecay
            } else {
                severe_decay_count += 1;
                AlphaDecayStatus::SevereDecay
            };

            // Pairwise self-correlation audit
            let mut max_corr = -1.0;
            let mut most_corr_id = String::new();
            for other in portfolio_alphas {
                if other.id == alpha.id {
                    continue;
                }
                if let Some(other_pnl) = os_pnls.get(&other.id) {
                    if let (Some(r), _) = pearson_correlation(pnl, other_pnl) {
                        if r > max_corr {
                            max_corr = r;
                            most_corr_id = other.id.clone();
                        }
                    }
                }
            }
            let safety_buffer = (0.70 - max_corr) * 100.0;
            if safety_buffer < 8.0 {
                tight_buffer_count += 1;
            }

            // Leave-One-Out Marginal Portfolio Impact
            let leave_one_out: Vec<&HashMap<String, f64>> = portfolio_alphas
                .iter()
                .filter(|a| a.id != alpha.id)
                .filter_map(|a| os_pnls.get(&a.id))
                .collect();

            let marginal_delta = if let Some(loo_metrics) = simulate_portfolio(&leave_one_out) {
                baseline_sharpe - loo_metrics.merged_sharpe
            } else {
                0.0
            };

            let role_verdict = if marginal_delta >= 0.020 {
                AlphaRoleVerdict::PillarDriver
            } else if marginal_delta > 0.001 {
                AlphaRoleVerdict::PositiveContributor
            } else if marginal_delta >= -0.010 {
                AlphaRoleVerdict::NeutralDiversifier
            } else {
                AlphaRoleVerdict::DilutiveDrag
            };

            // Crowd co-movement from shadow portfolio if available
            let crowd_corr = shadow_opt.and_then(|shadow| {
                pearson_correlation(pnl, &shadow.shadow_pnl).0
            });

            all_reports.push(AlphaHealthReport {
                alpha_id: alpha.id.clone(),
                name: alpha
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unknown Alpha".to_string()),
                universe: alpha
                    .universe
                    .clone()
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                category: alpha
                    .category
                    .clone()
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                full_sharpe: full_sh,
                trailing_1y_sharpe: trailing_sh,
                decay_ratio,
                decay_status,
                max_internal_corr: max_corr,
                most_correlated_id: most_corr_id,
                safety_buffer_pct: safety_buffer,
                marginal_sharpe_delta: marginal_delta,
                role_verdict,
                crowd_co_movement: crowd_corr,
            });
        }
    }

    // Top Pillar Drivers (highest marginal Sharpe contribution)
    let mut drivers = all_reports.clone();
    drivers.sort_by(|a, b| {
        b.marginal_sharpe_delta
            .partial_cmp(&a.marginal_sharpe_delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_drivers = drivers.into_iter().take(5).collect();

    // At-Risk Alphas (highest decay or lowest marginal contribution)
    let mut at_risk = all_reports.clone();
    at_risk.sort_by(|a, b| {
        a.marginal_sharpe_delta
            .partial_cmp(&b.marginal_sharpe_delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_drags = at_risk
        .into_iter()
        .filter(|a| {
            a.role_verdict == AlphaRoleVerdict::DilutiveDrag
                || a.decay_status == AlphaDecayStatus::SevereDecay
                || a.safety_buffer_pct < 8.0
        })
        .take(5)
        .collect();

    // Institutional Portfolio Health Score (0 - 100)
    let n_total = all_reports.len() as f64;
    let healthy_ratio = (robust_count + accelerating_count) as f64 / n_total;
    let driver_ratio = all_reports
        .iter()
        .filter(|a| {
            a.role_verdict == AlphaRoleVerdict::PillarDriver
                || a.role_verdict == AlphaRoleVerdict::PositiveContributor
        })
        .count() as f64
        / n_total;
    let corr_penalty = (baseline_metrics.avg_pairwise_corr / 0.35).min(1.0);
    let collision_penalty = (tight_buffer_count as f64 / n_total).min(1.0);

    let raw_score = 45.0 * healthy_ratio + 35.0 * driver_ratio + 20.0 * (1.0 - corr_penalty)
        - 15.0 * collision_penalty;
    let health_score = raw_score.clamp(0.0, 100.0);

    // Formulate actionable recommendations
    let mut recs = Vec::new();
    if tight_buffer_count > 0 {
        recs.push(format!(
            "CẢNH BÁO TƯƠNG QUAN: Có {} Alpha đang có đệm an toàn dưới 8% so với trần 70%. Ưu tiên khai thác các Factor Pillar hoàn toàn trực giao.",
            tight_buffer_count
        ));
    }
    if severe_decay_count > 0 {
        recs.push(format!(
            "ALPHA DECAY: Phát hiện {} Alpha có dấu hiệu bán rã hiệu năng (Trailing 1Y Sharpe suy giảm mạnh). Khi danh mục chạm 60 Alphas, xem xét thanh lọc nhóm này.",
            severe_decay_count
        ));
    }
    if baseline_metrics.avg_pairwise_corr < 0.22 {
        recs.push(format!(
            "TỔNG THỂ VỮNG CHẮC: Tương quan chéo bình quân danh mục duy trì ở mức tối ưu ({:.2}%). Tiếp tục phát huy chiến lược phân tầng Green Sanctuary.",
            baseline_metrics.avg_pairwise_corr * 100.0
        ));
    }

    Some(MasterPortfolioHealthReport {
        total_alphas: all_reports.len(),
        common_trading_days: n_days,
        portfolio_sharpe: baseline_sharpe,
        portfolio_annualized_pnl: baseline_metrics.annualized_pnl,
        portfolio_annualized_vol: baseline_metrics.annualized_vol,
        avg_pairwise_correlation: baseline_metrics.avg_pairwise_corr,
        health_score,
        accelerating_count,
        robust_count,
        moderate_decay_count,
        severe_decay_count,
        tight_safety_buffer_count: tight_buffer_count,
        top_pillar_drivers: top_drivers,
        at_risk_alphas: top_drags,
        all_alpha_diagnostics: all_reports,
        actionable_recommendations: recs,
    })
}

/// Prints the master portfolio health diagnostic report with terminal styling
pub fn print_health_report(report: &MasterPortfolioHealthReport) {
    println!(
        "\n{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Portfolio Health & Alpha Half-Life Diagnostic"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let score_styled = if report.health_score >= 85.0 {
        format!("{:.1} / 100 (EXCELLENT HEALTH ⭐)", report.health_score)
            .green()
            .bold()
    } else if report.health_score >= 70.0 {
        format!("{:.1} / 100 (ROBUST HEALTH 🟢)", report.health_score)
            .green()
            .bold()
    } else if report.health_score >= 50.0 {
        format!(
            "{:.1} / 100 (MODERATE VULNERABILITY 🟡)",
            report.health_score
        )
        .yellow()
    } else {
        format!("{:.1} / 100 (HIGH DECAY RISK 🔴)", report.health_score)
            .red()
            .bold()
    };

    println!("\n  🩺 Overall Portfolio Health Score: {}", score_styled);
    println!(
        "  Active Alphas: {} | Common Trading Days: {} | Merged Sharpe: {}",
        report.total_alphas.to_string().yellow().bold(),
        report.common_trading_days.to_string().cyan(),
        format!("{:.2}", report.portfolio_sharpe).green().bold()
    );
    println!(
        "  Annualized Merged PnL: ${:.2} | Volatility: ${:.2} | Avg Corr: {:.2}%",
        report.portfolio_annualized_pnl,
        report.portfolio_annualized_vol,
        report.avg_pairwise_correlation * 100.0
    );

    println!("\n  📊 Alpha Lifecycle & Decay Spectrum:");
    println!(
        "    • 🚀 Accelerating Momentum : {:2} alphas ({:.1}%)",
        report.accelerating_count,
        report.accelerating_count as f64 * 100.0 / report.total_alphas as f64
    );
    println!(
        "    • 🟢 Robust / Stable       : {:2} alphas ({:.1}%)",
        report.robust_count,
        report.robust_count as f64 * 100.0 / report.total_alphas as f64
    );
    println!(
        "    • 🟡 Moderate Decay        : {:2} alphas ({:.1}%)",
        report.moderate_decay_count,
        report.moderate_decay_count as f64 * 100.0 / report.total_alphas as f64
    );
    println!(
        "    • 🔴 Severe Decay          : {:2} alphas ({:.1}%)",
        report.severe_decay_count,
        report.severe_decay_count as f64 * 100.0 / report.total_alphas as f64
    );

    println!(
        "\n  {}",
        "🌟 TOP 5 PILLAR DRIVERS (Highest Marginal Sharpe Contribution):".bold()
    );
    println!("  ┌──────────┬──────────┬─────────────┬─────────────┬──────────────┬────────────┬────────────────────┐");
    println!("  │ Alpha ID │ Universe │ Full Sharpe │ 1Y Trailing │ Decay Status │ Δ Sharpe   │ Role Verdict       │");
    println!("  ├──────────┼──────────┼─────────────┼─────────────┼──────────────┼────────────┼────────────────────┤");

    for a in &report.top_pillar_drivers {
        println!(
            "  │ {:<8} │ {:<8} │ {:>11.2} │ {:>11.2} │ {:<20} │ {:>+10.4} │ {:<18} │",
            a.alpha_id.cyan().bold(),
            a.universe.yellow(),
            a.full_sharpe,
            a.trailing_1y_sharpe,
            a.decay_status.badge(),
            a.marginal_sharpe_delta,
            a.role_verdict.badge()
        );
    }
    println!("  └──────────┴──────────┴─────────────┴─────────────┴──────────────┴────────────┴────────────────────┘");

    if !report.at_risk_alphas.is_empty() {
        println!(
            "\n  {}",
            "⚠️ AT-RISK & DECAYING ALPHAS (Dilutive / Fragile / High Decay):"
                .red()
                .bold()
        );
        println!("  ┌──────────┬──────────┬─────────────┬─────────────┬──────────────┬────────────┬────────────────────┐");
        println!("  │ Alpha ID │ Universe │ Full Sharpe │ 1Y Trailing │ Decay Status │ Safety Buf │ Role Verdict       │");
        println!("  ├──────────┼──────────┼─────────────┼─────────────┼──────────────┼────────────┼────────────────────┤");

        for a in &report.at_risk_alphas {
            let buf_str = format!("{:.1}%", a.safety_buffer_pct);
            let buf_styled = if a.safety_buffer_pct < 8.0 {
                buf_str.red().bold()
            } else {
                buf_str.yellow()
            };

            println!(
                "  │ {:<8} │ {:<8} │ {:>11.2} │ {:>11.2} │ {:<20} │ {:>10} │ {:<18} │",
                a.alpha_id.yellow(),
                a.universe.white(),
                a.full_sharpe,
                a.trailing_1y_sharpe,
                a.decay_status.badge(),
                buf_styled,
                a.role_verdict.badge()
            );
        }
        println!("  └──────────┴──────────┴─────────────┴─────────────┴──────────────┴────────────┴────────────────────┘");
    }

    if !report.actionable_recommendations.is_empty() {
        println!(
            "\n  {}",
            "💡 Strategic Prescriptions & Directives:".green().bold()
        );
        for rec in &report.actionable_recommendations {
            println!("    👉 {}", rec.white());
        }
    }
    println!();
}

/// Prints complete tabular diagnostic across all active Out-of-Sample Alphas
pub fn print_detailed_alpha_table(alphas: &[AlphaHealthReport]) {
    println!(
        "\n{}",
        "─────────────────────────────────────────────────────────────────────────────────────────────────────────────".cyan()
    );
    println!(
        "{}",
        "  Active Portfolio Alpha Fleet Diagnostic (Full Inventory)"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "─────────────────────────────────────────────────────────────────────────────────────────────────────────────".cyan()
    );
    println!("  ┌──────────┬──────────┬──────────────┬─────────────┬─────────────┬──────────────┬──────────────────┬─────────────┬────────────────────┐");
    println!("  │ Alpha ID │ Universe │ Category     │ Full Sharpe │ 1Y Trailing │ Decay Status │ Max Corr (Peer)  │ Δ Sharpe    │ Role Verdict       │");
    println!("  ├──────────┼──────────┼──────────────┼─────────────┼─────────────┼──────────────┼──────────────────┼─────────────┼────────────────────┤");

    for a in alphas {
        let max_corr_str = format!(
            "{:.1}% ({})",
            a.max_internal_corr * 100.0,
            &a.most_correlated_id[..a.most_correlated_id.len().min(8)]
        );
        println!(
            "  │ {:<8} │ {:<8} │ {:<12} │ {:>11.2} │ {:>11.2} │ {:<20} │ {:>16} │ {:>+11.4} │ {:<18} │",
            a.alpha_id.cyan().bold(),
            a.universe.yellow(),
            &a.category[..a.category.len().min(12)],
            a.full_sharpe,
            a.trailing_1y_sharpe,
            a.decay_status.badge(),
            max_corr_str,
            a.marginal_sharpe_delta,
            a.role_verdict.badge()
        );
    }
    println!("  └──────────┴──────────┴──────────────┴─────────────┴─────────────┴──────────────┴──────────────────┴─────────────┴────────────────────┘\n");
}

/// Prints a focused diagnostic deep-dive for a single Alpha
pub fn print_single_alpha_diagnostic(a: &AlphaHealthReport) {
    println!(
        "\n{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "  Alpha Deep-Dive Health Diagnostic: {}",
        a.alpha_id.yellow().bold()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!("  Name:                 {}", a.name.cyan());
    println!(
        "  Universe / Category:  {} / {}",
        a.universe.yellow(),
        a.category.white()
    );
    println!("  Historical Sharpe:    {:.2}", a.full_sharpe);
    println!("  Trailing 1Y Sharpe:   {:.2}", a.trailing_1y_sharpe);
    println!("  Decay Ratio:          {:.1}%", a.decay_ratio * 100.0);
    println!("  Decay Status:         {}", a.decay_status.badge());
    println!(
        "  Max Internal Corr:    {:.2}% (with {})",
        a.max_internal_corr * 100.0,
        a.most_correlated_id.yellow()
    );
    println!(
        "  Self-Corr Safety Buf: {:.1}% to 70% limit",
        a.safety_buffer_pct
    );
    println!("  Marginal Delta Sharpe:{:>+8.4}", a.marginal_sharpe_delta);
    println!("  Portfolio Role:       {}", a.role_verdict.badge());
    if let Some(crowd_corr) = a.crowd_co_movement {
        println!(
            "  Synthetic Proxy Corr: {:.2}% (not platform PnL/Uniqueness)",
            crowd_corr * 100.0
        );
    }
    println!();
}
