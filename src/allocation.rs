//! Portfolio Allocation, Hierarchical Risk Parity (HRP), and Admission Decision Engine.
//!
//! Provides quantitative evaluation of portfolio balance across:
//! 1. Temporal Horizon & Turnover Entropy (Shannon Entropy)
//! 2. Multi-Asset Universe & Factor Pillar Quotas
//! 3. Hierarchical Risk Parity (HRP) optimal weights vs Equal-Weight (1/N)
//! 4. Automated Candidate Admission Decision Gate (Go / No-Go)
//! 5. Trimming & Retirement recommendations for the 60-Alpha milestone

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::correlation::{pearson_correlation, simulate_portfolio};
use crate::models::PortfolioAlpha;

/// Turnover classification spectrum bucket
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnoverBucket {
    UltraSlow, // < 10%
    Core,      // 10% - 25%
    Fast,      // 25% - 45%
    VeryFast,  // 45% - 70%
}

impl TurnoverBucket {
    pub fn from_turnover(to: f64) -> Self {
        let pct = if to <= 1.0 { to * 100.0 } else { to };
        if pct < 10.0 {
            Self::UltraSlow
        } else if pct < 25.0 {
            Self::Core
        } else if pct < 45.0 {
            Self::Fast
        } else {
            Self::VeryFast
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::UltraSlow => "Ultra-Slow (<10%)",
            Self::Core => "Core Medium (10-25%)",
            Self::Fast => "Fast Tactical (25-45%)",
            Self::VeryFast => "Very Fast (45-70%)",
        }
    }
}

/// Turnover Entropy diagnostic report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnoverEntropyReport {
    pub total_alphas: usize,
    pub ultra_slow_count: usize,
    pub core_count: usize,
    pub fast_count: usize,
    pub very_fast_count: usize,
    pub ultra_slow_pct: f64,
    pub core_pct: f64,
    pub fast_pct: f64,
    pub very_fast_pct: f64,
    pub shannon_entropy: f64,
    pub max_entropy: f64,
    pub diversity_score: f64, // 0 - 100%
    pub status: String,
}

/// Evaluates Shannon Entropy over turnover distribution
pub fn evaluate_turnover_entropy(alphas: &[PortfolioAlpha]) -> TurnoverEntropyReport {
    let n = alphas.len();
    if n == 0 {
        return TurnoverEntropyReport {
            total_alphas: 0,
            ultra_slow_count: 0,
            core_count: 0,
            fast_count: 0,
            very_fast_count: 0,
            ultra_slow_pct: 0.0,
            core_pct: 0.0,
            fast_pct: 0.0,
            very_fast_pct: 0.0,
            shannon_entropy: 0.0,
            max_entropy: (4.0f64).ln(),
            diversity_score: 0.0,
            status: "EMPTY".to_string(),
        };
    }

    let mut c_slow = 0;
    let mut c_core = 0;
    let mut c_fast = 0;
    let mut c_vfast = 0;

    for a in alphas {
        let to = a.turnover.unwrap_or(0.15);
        match TurnoverBucket::from_turnover(to) {
            TurnoverBucket::UltraSlow => c_slow += 1,
            TurnoverBucket::Core => c_core += 1,
            TurnoverBucket::Fast => c_fast += 1,
            TurnoverBucket::VeryFast => c_vfast += 1,
        }
    }

    let p_slow = c_slow as f64 / n as f64;
    let p_core = c_core as f64 / n as f64;
    let p_fast = c_fast as f64 / n as f64;
    let p_vfast = c_vfast as f64 / n as f64;

    let mut entropy = 0.0;
    for &p in &[p_slow, p_core, p_fast, p_vfast] {
        if p > 1e-9 {
            entropy -= p * p.ln();
        }
    }

    let max_entropy = (4.0f64).ln();
    let diversity_score = (entropy / max_entropy * 100.0).clamp(0.0, 100.0);

    let status = if diversity_score >= 75.0 {
        "🟢 EXCELLENT DIVERSIFICATION".to_string()
    } else if diversity_score >= 50.0 {
        "🟡 MODERATE CONCENTRATION".to_string()
    } else {
        "🔴 HIGH CLUSTERING CONCENTRATION".to_string()
    };

    TurnoverEntropyReport {
        total_alphas: n,
        ultra_slow_count: c_slow,
        core_count: c_core,
        fast_count: c_fast,
        very_fast_count: c_vfast,
        ultra_slow_pct: p_slow * 100.0,
        core_pct: p_core * 100.0,
        fast_pct: p_fast * 100.0,
        very_fast_pct: p_vfast * 100.0,
        shannon_entropy: entropy,
        max_entropy,
        diversity_score,
        status,
    }
}

/// Result of Hierarchical Risk Parity (HRP) weight calculation for an individual Alpha
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrpAlphaWeight {
    pub alpha_id: String,
    pub name: String,
    pub universe: String,
    pub hrp_weight: f64,
    pub equal_weight: f64,
    pub health_ratio: f64, // hrp_weight / equal_weight
    pub verdict: String,
}

/// Computes Hierarchical Risk Parity weights using recursive bisection on daily PnLs
pub fn compute_hrp_weights(
    alphas: &[PortfolioAlpha],
    pnl_cache: &HashMap<String, HashMap<String, f64>>,
) -> Vec<HrpAlphaWeight> {
    let n = alphas.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![HrpAlphaWeight {
            alpha_id: alphas[0].id.clone(),
            name: alphas[0]
                .name
                .clone()
                .unwrap_or_else(|| alphas[0].id.clone()),
            universe: alphas[0]
                .universe
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            hrp_weight: 1.0,
            equal_weight: 1.0,
            health_ratio: 1.0,
            verdict: "SOLO".to_string(),
        }];
    }

    // Find intersection of common trading dates across cached alphas
    let mut common_dates: Option<std::collections::BTreeSet<String>> = None;
    for a in alphas {
        if let Some(series) = pnl_cache.get(&a.id) {
            let dates: std::collections::BTreeSet<String> = series.keys().cloned().collect();
            common_dates = match common_dates {
                None => Some(dates),
                Some(acc) => Some(acc.intersection(&dates).cloned().collect()),
            };
        }
    }

    let dates: Vec<String> = common_dates.unwrap_or_default().into_iter().collect();
    if dates.len() < 30 {
        // Fall back to equal weights if insufficient overlapping history
        let eq_w = 1.0 / n as f64;
        return alphas
            .iter()
            .map(|a| HrpAlphaWeight {
                alpha_id: a.id.clone(),
                name: a.name.clone().unwrap_or_else(|| a.id.clone()),
                universe: a.universe.clone().unwrap_or_else(|| "N/A".to_string()),
                hrp_weight: eq_w,
                equal_weight: eq_w,
                health_ratio: 1.0,
                verdict: "EQUAL_FALLBACK".to_string(),
            })
            .collect();
    }

    let t = dates.len();

    // Build PnL matrix: n x t
    let mut pnl_matrix: Vec<Vec<f64>> = Vec::with_capacity(n);
    for a in alphas {
        let mut row = Vec::with_capacity(t);
        if let Some(series) = pnl_cache.get(&a.id) {
            for d in &dates {
                row.push(*series.get(d).unwrap_or(&0.0));
            }
        } else {
            row.resize(t, 0.0);
        }
        pnl_matrix.push(row);
    }

    // Compute sample covariance matrix and variances
    let mut cov = vec![vec![0.0f64; n]; n];
    let mut vars = vec![0.0f64; n];
    for i in 0..n {
        let mean_i = pnl_matrix[i].iter().sum::<f64>() / t as f64;
        let var_i = pnl_matrix[i]
            .iter()
            .map(|&x| (x - mean_i).powi(2))
            .sum::<f64>()
            / (t - 1) as f64;
        vars[i] = var_i.max(1e-8);
        cov[i][i] = vars[i];

        for j in 0..i {
            let mean_j = pnl_matrix[j].iter().sum::<f64>() / t as f64;
            let cov_ij = pnl_matrix[i]
                .iter()
                .zip(pnl_matrix[j].iter())
                .map(|(&x, &y)| (x - mean_i) * (y - mean_j))
                .sum::<f64>()
                / (t - 1) as f64;
            cov[i][j] = cov_ij;
            cov[j][i] = cov_ij;
        }
    }

    // Correlation and Distance Matrix
    let mut dist = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                dist[i][j] = 0.0;
            } else {
                let std_i = vars[i].sqrt();
                let std_j = vars[j].sqrt();
                let corr = if std_i > 1e-8 && std_j > 1e-8 {
                    (cov[i][j] / (std_i * std_j)).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                dist[i][j] = ((1.0 - corr) / 2.0).max(0.0).sqrt();
            }
        }
    }

    // Quasi-diagonalization via Agglomerative complete linkage
    let sorted_order = quasi_diagonalize(n, &dist);

    // Recursive bisection for HRP weights
    let mut weights = vec![1.0f64; n];
    recursive_bisection(&sorted_order, &cov, &mut weights);

    // Normalize weights
    let sum_w: f64 = weights.iter().sum();
    let norm_weights: Vec<f64> = if sum_w > 1e-8 {
        weights.iter().map(|&w| w / sum_w).collect()
    } else {
        vec![1.0 / n as f64; n]
    };

    let eq_w = 1.0 / n as f64;
    alphas
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let hrp_w = norm_weights[i];
            let ratio = if eq_w > 0.0 { hrp_w / eq_w } else { 1.0 };
            let verdict = if ratio >= 1.30 {
                "🌟 UNDER-ALLOCATED PILLAR".to_string()
            } else if ratio <= 0.70 {
                "⚠️ DILUTIVE OVER-ALLOCATED".to_string()
            } else {
                "🟢 BALANCED RISK".to_string()
            };

            HrpAlphaWeight {
                alpha_id: a.id.clone(),
                name: a.name.clone().unwrap_or_else(|| a.id.clone()),
                universe: a.universe.clone().unwrap_or_else(|| "N/A".to_string()),
                hrp_weight: hrp_w,
                equal_weight: eq_w,
                health_ratio: ratio,
                verdict,
            }
        })
        .collect()
}

/// Simple quasi-diagonalization that orders items by pairwise distance similarity
fn quasi_diagonalize(n: usize, dist: &[Vec<f64>]) -> Vec<usize> {
    let mut order = Vec::with_capacity(n);
    let mut visited = vec![false; n];

    // Start with index 0
    let mut current = 0;
    order.push(current);
    visited[current] = true;

    for _ in 1..n {
        let mut min_d = f64::MAX;
        let mut next = current;
        for j in 0..n {
            if !visited[j] && dist[current][j] < min_d {
                min_d = dist[current][j];
                next = j;
            }
        }
        visited[next] = true;
        order.push(next);
        current = next;
    }

    order
}

/// Recursive bisection for Inverse-Variance cluster allocations
fn recursive_bisection(cluster: &[usize], cov: &[Vec<f64>], weights: &mut [f64]) {
    if cluster.len() <= 1 {
        return;
    }

    let mid = cluster.len() / 2;
    let left = &cluster[..mid];
    let right = &cluster[mid..];

    let v_left = cluster_variance(left, cov);
    let v_right = cluster_variance(right, cov);

    let alpha = 1.0 - (v_left / (v_left + v_right + 1e-12));

    for &idx in left {
        weights[idx] *= alpha;
    }
    for &idx in right {
        weights[idx] *= 1.0 - alpha;
    }

    recursive_bisection(left, cov, weights);
    recursive_bisection(right, cov, weights);
}

/// Computes variance of a sub-cluster using inverse-variance intra-weights
fn cluster_variance(cluster: &[usize], cov: &[Vec<f64>]) -> f64 {
    if cluster.is_empty() {
        return 1.0;
    }
    if cluster.len() == 1 {
        return cov[cluster[0]][cluster[0]].max(1e-8);
    }

    // Inverse variance weights within cluster
    let mut inv_vars: Vec<f64> = cluster.iter().map(|&i| 1.0 / cov[i][i].max(1e-8)).collect();
    let sum_inv: f64 = inv_vars.iter().sum();
    for w in &mut inv_vars {
        *w /= sum_inv.max(1e-12);
    }

    // Quadratic form: w^T * Cov * w
    let mut var = 0.0;
    for (a_idx, &i) in cluster.iter().enumerate() {
        for (b_idx, &j) in cluster.iter().enumerate() {
            var += inv_vars[a_idx] * inv_vars[b_idx] * cov[i][j];
        }
    }
    var.max(1e-8)
}

/// Comprehensive Candidate Admission Decision Verdict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionVerdict {
    StrongGo,      // Boosts Sharpe, reduces volatility, improves turnover balance
    ConditionalGo, // Compliant with buffer, but adds minimal diversification
    NoGo,          // Exceeds correlation, tight buffer, or exacerbates turnover saturation
}

impl AdmissionVerdict {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::StrongGo => "🌟 STRONG GO (RECOMMENDED FOR SUBMISSION)".green().bold(),
            Self::ConditionalGo => "🟡 CONDITIONAL GO (ACCEPTABLE BACKUP)".yellow().bold(),
            Self::NoGo => "🔴 NO GO (REJECTED / DILUTIVE)".red().bold(),
        }
    }
}

/// Comprehensive Candidate Admission Audit Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateAdmissionReport {
    pub candidate_id: String,
    pub candidate_turnover_pct: f64,
    pub candidate_bucket: TurnoverBucket,
    pub baseline_sharpe: f64,
    pub merged_sharpe: f64,
    pub sharpe_delta: f64,
    pub baseline_vol: f64,
    pub merged_vol: f64,
    pub vol_delta: f64,
    pub max_pairwise_corr: f64,
    pub most_correlated_id: String,
    pub safety_buffer_pct: f64,
    pub baseline_entropy_score: f64,
    pub merged_entropy_score: f64,
    pub entropy_delta: f64,
    pub quota_deficit_addressed: bool,
    pub verdict: AdmissionVerdict,
    pub rationale: Vec<String>,
}

/// Evaluates whether a candidate Alpha should be admitted into the Out-of-Sample portfolio
pub fn evaluate_candidate_admission(
    candidate_id: &str,
    candidate_turnover: f64,
    candidate_pnl: &HashMap<String, f64>,
    os_alphas: &[PortfolioAlpha],
    pnl_cache: &HashMap<String, HashMap<String, f64>>,
) -> CandidateAdmissionReport {
    // Baseline stats
    let base_entropy = evaluate_turnover_entropy(os_alphas);
    let mut base_pnl_refs: Vec<&HashMap<String, f64>> = Vec::new();
    for a in os_alphas {
        if let Some(pnl) = pnl_cache.get(&a.id) {
            base_pnl_refs.push(pnl);
        }
    }
    let base_sim = simulate_portfolio(&base_pnl_refs);
    let (base_sharpe, base_vol) = match base_sim {
        Some(ref m) => (m.merged_sharpe, m.annualized_vol),
        None => (0.0, 0.0),
    };

    // Pairwise correlations against baseline
    let mut max_corr = -1.0f64;
    let mut top_match = "N/A".to_string();
    for a in os_alphas {
        if let Some(os_pnl) = pnl_cache.get(&a.id) {
            if let (Some(corr), _) = pearson_correlation(candidate_pnl, os_pnl) {
                if corr > max_corr {
                    max_corr = corr;
                    top_match = a.id.clone();
                }
            }
        }
    }

    let safety_buffer = (0.70 - max_corr.max(0.0)) * 100.0;

    // Synthetic candidate for merged simulation
    let cand_alpha = PortfolioAlpha {
        id: candidate_id.to_string(),
        name: Some(format!("CANDIDATE_{}", candidate_id)),
        status: Some("CANDIDATE".to_string()),
        stage: Some("IN_SAMPLE".to_string()),
        date_submitted: Some(String::new()),
        universe: Some("TOP1000".to_string()),
        decay: Some(10),
        category: Some("HYPOTHESIS".to_string()),
        color: Some("GREEN".to_string()),
        sharpe: Some(2.0),
        fitness: Some(1.2),
        returns: Some(0.10),
        turnover: Some(candidate_turnover),
        direct_url: None,
        code: None,
        tags: Some(vec![]),
        settings: None,
    };

    let mut all_alphas = os_alphas.to_vec();
    all_alphas.push(cand_alpha);

    let mut merged_pnl_refs = base_pnl_refs.clone();
    merged_pnl_refs.push(candidate_pnl);

    let merged_entropy = evaluate_turnover_entropy(&all_alphas);
    let merged_sim = simulate_portfolio(&merged_pnl_refs);
    let (merged_sharpe, merged_vol) = match merged_sim {
        Some(ref m) => (m.merged_sharpe, m.annualized_vol),
        None => (base_sharpe, base_vol),
    };

    let sharpe_delta = merged_sharpe - base_sharpe;
    let vol_delta = merged_vol - base_vol;
    let entropy_delta = merged_entropy.diversity_score - base_entropy.diversity_score;

    let cand_bucket = TurnoverBucket::from_turnover(candidate_turnover);
    let quota_deficit_addressed =
        cand_bucket == TurnoverBucket::Fast || cand_bucket == TurnoverBucket::VeryFast;

    let mut rationale = Vec::new();

    // Decision rule logic
    let verdict = if max_corr >= 0.70 {
        rationale.push(format!(
            "FAIL local self-correlation proxy (rho = {:.2}% >= 70.0% vs {}).",
            max_corr * 100.0,
            top_match
        ));
        AdmissionVerdict::NoGo
    } else if safety_buffer < 5.0 {
        rationale.push(format!(
            "Tight safety buffer ({:.2}% < 5.0% vs {}). High risk of OOS collision.",
            safety_buffer, top_match
        ));
        AdmissionVerdict::NoGo
    } else if sharpe_delta > 0.0 && vol_delta < 0.0 && quota_deficit_addressed {
        rationale.push(format!(
            "Variance collapse (-${:.0}) with Sharpe increase (+{:.4}).",
            vol_delta.abs(),
            sharpe_delta
        ));
        rationale.push(format!(
            "Directly addresses portfolio turnover deficit bucket ({:?}).",
            cand_bucket
        ));
        rationale.push(format!(
            "Robust safety buffer: {:.2}% (Max corr: {:.2}% vs {}).",
            safety_buffer,
            max_corr * 100.0,
            top_match
        ));
        AdmissionVerdict::StrongGo
    } else if sharpe_delta > 0.0 && safety_buffer >= 8.0 {
        rationale.push(format!(
            "Net positive Sharpe delta (+{:.4}) with healthy safety buffer ({:.2}%).",
            sharpe_delta, safety_buffer
        ));
        AdmissionVerdict::ConditionalGo
    } else {
        rationale.push(format!(
            "Dilutes Sharpe or increases volatility without improving diversity (Δ Sharpe: {:.4}).",
            sharpe_delta
        ));
        AdmissionVerdict::NoGo
    };

    CandidateAdmissionReport {
        candidate_id: candidate_id.to_string(),
        candidate_turnover_pct: candidate_turnover * 100.0,
        candidate_bucket: cand_bucket,
        baseline_sharpe: base_sharpe,
        merged_sharpe,
        sharpe_delta,
        baseline_vol: base_vol,
        merged_vol,
        vol_delta,
        max_pairwise_corr: max_corr,
        most_correlated_id: top_match,
        safety_buffer_pct: safety_buffer,
        baseline_entropy_score: base_entropy.diversity_score,
        merged_entropy_score: merged_entropy.diversity_score,
        entropy_delta,
        quota_deficit_addressed,
        verdict,
        rationale,
    }
}

/// Alpha candidate recommended for retirement / trimming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementCandidate {
    pub alpha_id: String,
    pub name: String,
    pub universe: String,
    pub hrp_weight_pct: f64,
    pub health_ratio: f64,
    pub turnover_pct: f64,
    pub reason: String,
}

/// Identifies bottom candidates for retirement when approaching the 60-Alpha milestone
pub fn identify_retirement_candidates(
    alphas: &[PortfolioAlpha],
    hrp_weights: &[HrpAlphaWeight],
) -> Vec<RetirementCandidate> {
    let mut candidates = Vec::new();
    let weight_map: HashMap<String, &HrpAlphaWeight> = hrp_weights
        .iter()
        .map(|w| (w.alpha_id.clone(), w))
        .collect();

    for a in alphas {
        if let Some(w) = weight_map.get(&a.id) {
            let to_pct = a.turnover.unwrap_or(0.15) * 100.0;
            // Criteria: Low HRP weight (Health Ratio < 0.65)
            if w.health_ratio < 0.70 {
                let reason = format!(
                    "Dilutive risk drag: HRP model requests only {:.3}% weight (HR: {:.2}x vs 1/N).",
                    w.hrp_weight * 100.0,
                    w.health_ratio
                );
                candidates.push(RetirementCandidate {
                    alpha_id: a.id.clone(),
                    name: a.name.clone().unwrap_or_else(|| a.id.clone()),
                    universe: a.universe.clone().unwrap_or_else(|| "N/A".to_string()),
                    hrp_weight_pct: w.hrp_weight * 100.0,
                    health_ratio: w.health_ratio,
                    turnover_pct: to_pct,
                    reason,
                });
            }
        }
    }

    // Sort by lowest health ratio first
    candidates.sort_by(|a, b| {
        a.health_ratio
            .partial_cmp(&b.health_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

/// Simulated impact of swapping a retired alpha with a new candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapSimulationReport {
    pub replaced_alpha_id: String,
    pub candidate_id: String,
    pub baseline_sharpe: f64,
    pub swapped_sharpe: f64,
    pub sharpe_delta: f64,
    pub baseline_vol: f64,
    pub swapped_vol: f64,
    pub vol_delta: f64,
    pub is_net_improvement: bool,
}

/// Simulates swapping an underperforming / dilutive alpha with a new candidate
pub fn evaluate_candidate_swap(
    candidate_id: &str,
    candidate_pnl: &HashMap<String, f64>,
    os_alphas: &[PortfolioAlpha],
    pnl_cache: &HashMap<String, HashMap<String, f64>>,
    worst_alpha_id: &str,
) -> Option<SwapSimulationReport> {
    let mut base_pnl_refs: Vec<&HashMap<String, f64>> = Vec::new();
    let mut swapped_pnl_refs: Vec<&HashMap<String, f64>> = Vec::new();

    for a in os_alphas {
        if let Some(pnl) = pnl_cache.get(&a.id) {
            base_pnl_refs.push(pnl);
            if a.id != worst_alpha_id {
                swapped_pnl_refs.push(pnl);
            }
        }
    }
    swapped_pnl_refs.push(candidate_pnl);

    let base_sim = simulate_portfolio(&base_pnl_refs)?;
    let swapped_sim = simulate_portfolio(&swapped_pnl_refs)?;

    let sharpe_delta = swapped_sim.merged_sharpe - base_sim.merged_sharpe;
    let vol_delta = swapped_sim.annualized_vol - base_sim.annualized_vol;

    Some(SwapSimulationReport {
        replaced_alpha_id: worst_alpha_id.to_string(),
        candidate_id: candidate_id.to_string(),
        baseline_sharpe: base_sim.merged_sharpe,
        swapped_sharpe: swapped_sim.merged_sharpe,
        sharpe_delta,
        baseline_vol: base_sim.annualized_vol,
        swapped_vol: swapped_sim.annualized_vol,
        vol_delta,
        is_net_improvement: sharpe_delta > 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turnover_bucket_classification() {
        assert_eq!(
            TurnoverBucket::from_turnover(0.08),
            TurnoverBucket::UltraSlow
        );
        assert_eq!(TurnoverBucket::from_turnover(0.18), TurnoverBucket::Core);
        assert_eq!(TurnoverBucket::from_turnover(0.35), TurnoverBucket::Fast);
        assert_eq!(
            TurnoverBucket::from_turnover(0.55),
            TurnoverBucket::VeryFast
        );
        assert_eq!(TurnoverBucket::from_turnover(35.0), TurnoverBucket::Fast);
    }

    #[test]
    fn test_turnover_entropy_calculation() {
        let mut alphas = Vec::new();
        for _ in 0..10 {
            alphas.push(PortfolioAlpha {
                id: "A1".to_string(),
                name: Some("Alpha".to_string()),
                status: Some("ACTIVE".to_string()),
                stage: Some("OS".to_string()),
                date_submitted: Some(String::new()),
                universe: Some("TOP1000".to_string()),
                decay: Some(10),
                category: Some("PV".to_string()),
                color: Some("BLUE".to_string()),
                sharpe: Some(2.0),
                fitness: Some(1.5),
                returns: Some(0.1),
                turnover: Some(0.15), // All in core bucket
                direct_url: None,
                code: None,
                tags: Some(vec![]),
                settings: None,
            });
        }
        let report = evaluate_turnover_entropy(&alphas);
        // All 10 in Core -> Entropy should be 0.0
        assert_eq!(report.shannon_entropy, 0.0);
        assert_eq!(report.diversity_score, 0.0);
    }

    #[test]
    fn test_turnover_entropy_balanced() {
        let mut alphas = Vec::new();
        // 1 in each of 4 buckets
        for &to in &[0.05, 0.15, 0.35, 0.55] {
            alphas.push(PortfolioAlpha {
                id: "A".to_string(),
                name: Some("A".to_string()),
                status: Some("ACTIVE".to_string()),
                stage: Some("OS".to_string()),
                date_submitted: Some(String::new()),
                universe: Some("TOP1000".to_string()),
                decay: Some(10),
                category: Some("PV".to_string()),
                color: Some("BLUE".to_string()),
                sharpe: Some(2.0),
                fitness: Some(1.5),
                returns: Some(0.1),
                turnover: Some(to),
                direct_url: None,
                code: None,
                tags: Some(vec![]),
                settings: None,
            });
        }
        let report = evaluate_turnover_entropy(&alphas);
        // Perfectly balanced across 4 buckets -> Diversity score should be 100%
        assert!((report.diversity_score - 100.0).abs() < 1e-4);
    }
}
