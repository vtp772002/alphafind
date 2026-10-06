use crate::models::PortfolioImpactResult;
use std::collections::{HashMap, HashSet};

/// Computes the exact Pearson correlation between two daily PnL series over their shared trading days.
pub fn pearson_correlation(
    x_dict: &HashMap<String, f64>,
    y_dict: &HashMap<String, f64>,
) -> (f64, usize) {
    let x_keys: HashSet<&String> = x_dict.keys().collect();
    let y_keys: HashSet<&String> = y_dict.keys().collect();
    let common: Vec<&&String> = x_keys.intersection(&y_keys).collect();
    let n = common.len();

    if n < 30 {
        return (0.0, n);
    }

    let n_f64 = n as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;

    for &&d in &common {
        sum_x += x_dict[d];
        sum_y += y_dict[d];
    }

    let mean_x = sum_x / n_f64;
    let mean_y = sum_y / n_f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for &&d in &common {
        let dx = x_dict[d] - mean_x;
        let dy = y_dict[d] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    let corr = if denom > 1e-12 { cov / denom } else { 0.0 };

    (corr, n)
}

/// Audit report against Out-of-Sample portfolio
#[derive(Debug, Clone)]
pub struct CorrelationAuditReport {
    pub candidate_id: String,
    pub max_correlation: f64,
    pub avg_correlation: f64,
    pub most_correlated_id: String,
    pub pairwise_results: Vec<(String, f64, usize)>, // (alpha_id, corr, common_days)
    pub is_brain_compliant: bool,                    // <= 0.70
    pub is_true_orthogonal: bool,                    // <= 0.15
}

/// Audits a candidate Alpha's PnL against an entire portfolio of active OS Alphas
pub fn audit_candidate(
    candidate_id: &str,
    cand_pnl: &HashMap<String, f64>,
    os_pnls: &HashMap<String, HashMap<String, f64>>,
) -> CorrelationAuditReport {
    let mut max_corr = -1.0;
    let mut most_corr_id = String::new();
    let mut sum_corr = 0.0;
    let mut valid_comparisons = 0;
    let mut pairwise = Vec::new();

    for (os_id, os_pnl) in os_pnls {
        if os_id == candidate_id {
            continue;
        }
        let (corr, days) = pearson_correlation(cand_pnl, os_pnl);
        if days >= 30 {
            if corr > max_corr {
                max_corr = corr;
                most_corr_id = os_id.clone();
            }
            sum_corr += corr;
            valid_comparisons += 1;
            pairwise.push((os_id.clone(), corr, days));
        }
    }

    pairwise.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let avg_corr = if valid_comparisons > 0 {
        sum_corr / (valid_comparisons as f64)
    } else {
        0.0
    };

    CorrelationAuditReport {
        candidate_id: candidate_id.to_string(),
        max_correlation: max_corr,
        avg_correlation: avg_corr,
        most_correlated_id: most_corr_id,
        pairwise_results: pairwise,
        is_brain_compliant: max_corr <= 0.70,
        is_true_orthogonal: max_corr <= 0.15,
    }
}

/// Multi-Alpha portfolio merge metrics
#[derive(Debug, Clone)]
pub struct PortfolioMergeMetrics {
    pub n_alphas: usize,
    pub n_trading_days: usize,
    pub annualized_pnl: f64,
    pub annualized_vol: f64,
    pub merged_sharpe: f64,
    pub avg_pairwise_corr: f64,
}

/// Simulates the exact equal-weighted multi-alpha portfolio PnL across common days
pub fn simulate_portfolio(pnls: &[&HashMap<String, f64>]) -> Option<PortfolioMergeMetrics> {
    if pnls.is_empty() {
        return None;
    }

    // Find common dates across all alphas
    let mut common_dates: HashSet<String> = pnls[0].keys().cloned().collect();
    for p in &pnls[1..] {
        let current_keys: HashSet<String> = p.keys().cloned().collect();
        common_dates = common_dates.intersection(&current_keys).cloned().collect();
    }

    let mut dates_vec: Vec<String> = common_dates.into_iter().collect();
    dates_vec.sort();

    let n_days = dates_vec.len();
    if n_days < 100 {
        return None;
    }

    let n_alphas = pnls.len() as f64;
    let mut portfolio_daily = Vec::with_capacity(n_days);

    for d in &dates_vec {
        let sum_day: f64 = pnls.iter().map(|p| p[d]).sum();
        portfolio_daily.push(sum_day / n_alphas);
    }

    let n_days_f64 = n_days as f64;
    let sum_pnl: f64 = portfolio_daily.iter().sum();
    let mean_daily = sum_pnl / n_days_f64;

    let var_daily: f64 = portfolio_daily
        .iter()
        .map(|x| (x - mean_daily).powi(2))
        .sum::<f64>()
        / (n_days_f64 - 1.0);
    let std_daily = var_daily.sqrt();

    let ann_pnl = mean_daily * 252.0;
    let ann_vol = std_daily * (252.0f64).sqrt();
    let merged_sharpe = if ann_vol > 1e-9 {
        ann_pnl / ann_vol
    } else {
        0.0
    };

    // Average pairwise correlation
    let mut sum_rho = 0.0;
    let mut count_pairs = 0;
    for i in 0..pnls.len() {
        for j in (i + 1)..pnls.len() {
            let (r, _) = pearson_correlation(pnls[i], pnls[j]);
            sum_rho += r;
            count_pairs += 1;
        }
    }
    let rho_bar = if count_pairs > 0 {
        sum_rho / (count_pairs as f64)
    } else {
        0.0
    };

    Some(PortfolioMergeMetrics {
        n_alphas: pnls.len(),
        n_trading_days: n_days,
        annualized_pnl: ann_pnl,
        annualized_vol: ann_vol,
        merged_sharpe,
        avg_pairwise_corr: rho_bar,
    })
}

/// Simulates the exact portfolio impact of adding a candidate Alpha to an existing portfolio.
pub fn calculate_portfolio_impact(
    candidate_id: &str,
    candidate_pnl: &HashMap<String, f64>,
    os_pnls: &HashMap<String, HashMap<String, f64>>,
) -> Option<PortfolioImpactResult> {
    if os_pnls.is_empty() {
        return None;
    }

    // Build baseline excluding candidate if it is already present in os_pnls
    let mut baseline_refs: Vec<&HashMap<String, f64>> = Vec::new();
    for (os_id, pnl) in os_pnls {
        if os_id != candidate_id {
            baseline_refs.push(pnl);
        }
    }

    if baseline_refs.is_empty() {
        return None;
    }

    let baseline = simulate_portfolio(&baseline_refs)?;

    let mut combined_refs = baseline_refs.clone();
    combined_refs.push(candidate_pnl);
    let new_metrics = simulate_portfolio(&combined_refs)?;

    // Calculate candidate correlation vs active OS alphas (skipping itself)
    let mut max_corr = -1.0;
    let mut most_corr_id = String::new();
    let mut sum_corr = 0.0;
    let mut valid_pairs = 0;

    for (os_id, pnl) in os_pnls {
        if os_id == candidate_id {
            continue;
        }
        let (corr, days) = pearson_correlation(candidate_pnl, pnl);
        if days >= 30 {
            if corr > max_corr {
                max_corr = corr;
                most_corr_id = os_id.clone();
            }
            sum_corr += corr;
            valid_pairs += 1;
        }
    }

    let avg_corr_vs_os = if valid_pairs > 0 {
        sum_corr / (valid_pairs as f64)
    } else {
        0.0
    };

    let safety_buffer = (0.70 - max_corr).max(0.0) * 100.0;

    Some(PortfolioImpactResult {
        candidate_id: candidate_id.to_string(),
        baseline_alphas: baseline.n_alphas,
        baseline_sharpe: baseline.merged_sharpe,
        baseline_pnl: baseline.annualized_pnl,
        baseline_vol: baseline.annualized_vol,
        baseline_avg_corr: baseline.avg_pairwise_corr,

        new_alphas: new_metrics.n_alphas,
        new_sharpe: new_metrics.merged_sharpe,
        new_pnl: new_metrics.annualized_pnl,
        new_vol: new_metrics.annualized_vol,
        new_avg_corr: new_metrics.avg_pairwise_corr,

        delta_sharpe: new_metrics.merged_sharpe - baseline.merged_sharpe,
        delta_pnl: new_metrics.annualized_pnl - baseline.annualized_pnl,
        delta_vol: new_metrics.annualized_vol - baseline.annualized_vol,
        delta_avg_corr: new_metrics.avg_pairwise_corr - baseline.avg_pairwise_corr,

        max_pairwise_corr: max_corr,
        most_correlated_id: most_corr_id,
        avg_pairwise_corr_vs_os: avg_corr_vs_os,
        safety_buffer_pct: safety_buffer,
    })
}
