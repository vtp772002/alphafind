use crate::correlation::pearson_correlation;
use std::collections::{HashMap, HashSet};

/// Local profile PnL basket weighted by platform field/dataset usage counts.
/// This is not a sample of competitor PnL.
#[derive(Debug, Clone)]
pub struct UsageWeightedProfilePnl {
    pub pnl: HashMap<String, f64>,
    pub sample_alpha_count: usize,
    pub sample_dataset_count: usize,
    pub common_days: usize,
    pub pnl_weight_coverage: f64,
}

#[derive(Debug, Clone)]
pub struct ProxyCorrelationImpact {
    pub baseline_proxy_corr: f64,
    pub projected_proxy_corr: f64,
    pub correlation_delta: f64,
    pub common_days: usize,
}

/// Merges several PnL series using fixed weights. Dates with less than 75% of
/// the sample's total weight available are omitted.
pub fn weighted_pnl_merge(
    weighted_series: &[(String, f64)],
    pnls: &HashMap<String, HashMap<String, f64>>,
) -> Option<(HashMap<String, f64>, usize)> {
    let series: Vec<(&HashMap<String, f64>, f64)> = weighted_series
        .iter()
        .filter_map(|(id, weight)| {
            (weight.is_finite() && *weight > 0.0)
                .then(|| pnls.get(id).map(|pnl| (pnl, *weight)))
                .flatten()
        })
        .collect();

    if series.is_empty() {
        return None;
    }

    let total_weight: f64 = series.iter().map(|(_, weight)| weight).sum();
    if total_weight <= 0.0 {
        return None;
    }

    let mut values_by_date: HashMap<String, (f64, f64, usize)> = HashMap::new();
    for (pnl, weight) in &series {
        for (date, value) in *pnl {
            if !value.is_finite() {
                continue;
            }
            let entry = values_by_date.entry(date.clone()).or_default();
            entry.0 += value * weight;
            entry.1 += weight;
            entry.2 += 1;
        }
    }

    let merged: HashMap<String, f64> = values_by_date
        .into_iter()
        .filter_map(
            |(date, (weighted_sum, available_weight, available_series))| {
                (available_series > 0 && available_weight >= total_weight * 0.75)
                    .then_some((date, weighted_sum / available_weight))
            },
        )
        .collect();

    let days = merged.len();
    (days >= 30).then_some((merged, days))
}

/// Builds a usage-weighted PnL basket from the user's own allocated Alpha series.
/// The platform usage counts set weights, but do not provide competitor PnLs.
pub fn build_usage_weighted_profile_pnl(
    allocation_weights: &HashMap<String, f64>,
    pnls: &HashMap<String, HashMap<String, f64>>,
    total_observed_weight: f64,
    sample_dataset_count: usize,
) -> Option<UsageWeightedProfilePnl> {
    let weighted_series: Vec<(String, f64)> = allocation_weights
        .iter()
        .filter(|(id, weight)| pnls.contains_key(*id) && weight.is_finite() && **weight > 0.0)
        .map(|(id, weight)| (id.clone(), *weight))
        .collect();

    let (pnl, common_days) = weighted_pnl_merge(&weighted_series, pnls)?;
    let represented_weight: f64 = weighted_series.iter().map(|(_, weight)| weight).sum();
    let pnl_weight_coverage = if total_observed_weight > 0.0 {
        (represented_weight / total_observed_weight).clamp(0.0, 1.0)
    } else {
        0.0
    };

    Some(UsageWeightedProfilePnl {
        pnl,
        sample_alpha_count: weighted_series.len(),
        sample_dataset_count,
        common_days,
        pnl_weight_coverage,
    })
}

/// Compares a local profile merge with a usage-weighted local PnL basket.
/// This sensitivity diagnostic does not forecast cross-user Uniqueness.
pub fn project_proxy_correlation(
    baseline_pnl: &HashMap<String, f64>,
    projected_pnl: &HashMap<String, f64>,
    usage_weighted_local_pnl: &HashMap<String, f64>,
) -> Option<ProxyCorrelationImpact> {
    let common_dates: HashSet<&String> = baseline_pnl
        .keys()
        .filter(|date| {
            projected_pnl.contains_key(*date) && usage_weighted_local_pnl.contains_key(*date)
        })
        .collect();
    let common_days = common_dates.len();
    if common_days < 30 {
        return None;
    }

    let baseline_common: HashMap<String, f64> = common_dates
        .iter()
        .filter_map(|date| {
            baseline_pnl
                .get(*date)
                .map(|value| ((*date).clone(), *value))
        })
        .collect();
    let projected_common: HashMap<String, f64> = common_dates
        .iter()
        .filter_map(|date| {
            projected_pnl
                .get(*date)
                .map(|value| ((*date).clone(), *value))
        })
        .collect();
    let local_proxy_common: HashMap<String, f64> = common_dates
        .iter()
        .filter_map(|date| {
            usage_weighted_local_pnl
                .get(*date)
                .map(|value| ((*date).clone(), *value))
        })
        .collect();
    let (baseline_proxy_corr_opt, _) = pearson_correlation(&baseline_common, &local_proxy_common);
    let (projected_proxy_corr_opt, _) = pearson_correlation(&projected_common, &local_proxy_common);

    let baseline_proxy_corr = baseline_proxy_corr_opt?;
    let projected_proxy_corr = projected_proxy_corr_opt?;

    let correlation_delta = projected_proxy_corr - baseline_proxy_corr;

    Some(ProxyCorrelationImpact {
        baseline_proxy_corr,
        projected_proxy_corr,
        correlation_delta,
        common_days,
    })
}

/// Creates an equal-weight PnL series on dates shared by all included Alphas.
pub fn equal_weight_pnl_merge(
    alpha_ids: &[String],
    pnls: &HashMap<String, HashMap<String, f64>>,
) -> Option<HashMap<String, f64>> {
    let series: Vec<&HashMap<String, f64>> =
        alpha_ids.iter().filter_map(|id| pnls.get(id)).collect();
    if series.is_empty() || series.len() != alpha_ids.len() {
        return None;
    }

    let mut common_dates: HashSet<String> = series[0].keys().cloned().collect();
    for pnl in &series[1..] {
        let dates: HashSet<String> = pnl.keys().cloned().collect();
        common_dates.retain(|date| dates.contains(date));
    }
    if common_dates.len() < 30 {
        return None;
    }

    let alpha_count = series.len() as f64;
    Some(
        common_dates
            .into_iter()
            .map(|date| {
                let daily_pnl = series.iter().map(|pnl| pnl[&date]).sum::<f64>() / alpha_count;
                (date, daily_pnl)
            })
            .collect(),
    )
}
