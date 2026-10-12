//! Dataset interaction matrix, gap blueprinting, and proxy shadow analysis commands.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::{
    field_config_label, format_count, get_cache_dir, get_main_client, load_pnl_cache,
    print_cache_status, profile_field_config,
};
use crate::correlation::pearson_correlation;
use crate::crowd_allocation::{
    build_formula_allocation, cache_field, current_unix_time, field_cache_key, find_dataset_config,
    get_fresh_cached_field, load_local_field_datasets, FieldConfig, FieldUsageCache,
};
use crate::crowd_model::{
    build_usage_weighted_profile_pnl, equal_weight_pnl_merge, project_proxy_correlation,
};
use crate::forecast::{generate_inverse_blueprint, print_blueprint_report};
use crate::matrix::{analyze_formula_hyper_synergy, build_dynamic_matrix, print_dynamic_matrix};
use crate::models::{
    AlphaSettings, CensusSnapshot, CompetitionEntry, DataFieldEntry, DatasetEntry, PortfolioAlpha,
};

type DatasetAllocationKey = (String, String, String, i32, String);
type ProfileDatasetFields = Vec<(DatasetAllocationKey, Vec<String>)>;
type AllocationSets = (
    std::collections::HashSet<String>,
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
);

fn derive_profile_usage_weights(
    alpha_datasets: &HashMap<String, ProfileDatasetFields>,
    field_cache: &FieldUsageCache,
    dataset_counts: &HashMap<DatasetAllocationKey, f64>,
    now: u64,
) -> (HashMap<String, f64>, usize, usize) {
    let mut weights = HashMap::new();
    let mut field_weighted_alphas = 0;
    let mut dataset_fallback_alphas = 0;

    for (alpha_id, exposures) in alpha_datasets {
        let mut exposure_weights = Vec::new();
        let mut used_field_counts = false;
        for (key, fields) in exposures {
            let config = FieldConfig {
                instrument_type: key.0.clone(),
                region: key.1.clone(),
                universe: key.2.clone(),
                delay: key.3,
            };
            let field_counts: Vec<f64> = fields
                .iter()
                .filter_map(|field_id| {
                    let cache_key = field_cache_key(field_id, &config);
                    get_fresh_cached_field(field_cache, &cache_key, now)
                        .and_then(|field| field.alpha_count)
                        .filter(|count| *count > 0)
                        .map(|count| count as f64)
                })
                .collect();

            if !field_counts.is_empty() {
                used_field_counts = true;
                exposure_weights.push(field_counts.iter().sum::<f64>() / field_counts.len() as f64);
            } else if let Some(count) = dataset_counts.get(key).filter(|count| **count > 0.0) {
                exposure_weights.push(*count);
            }
        }

        if !exposure_weights.is_empty() {
            let weight = exposure_weights.iter().sum::<f64>() / exposure_weights.len() as f64;
            if weight.is_finite() && weight > 0.0 {
                weights.insert(alpha_id.clone(), weight);
                if used_field_counts {
                    field_weighted_alphas += 1;
                } else {
                    dataset_fallback_alphas += 1;
                }
            }
        }
    }

    (weights, field_weighted_alphas, dataset_fallback_alphas)
}

fn challenge_uniqueness_score(client_competitions: &[CompetitionEntry]) -> Option<f64> {
    client_competitions
        .iter()
        .find(|competition| {
            let name = competition.name.to_ascii_lowercase();
            name.contains("challenge") && !name.contains("iqc")
        })
        .and_then(|competition| competition.leaderboard.as_ref())
        .and_then(|leaderboard| leaderboard.uniqueness_score)
}

/// Inspects data usage and local profile PnL sensitivity.
pub async fn cmd_shadow(
    candidate: Option<String>,
    expression: Option<String>,
    region: Option<String>,
    universe: Option<String>,
    delay: Option<i32>,
    simulate_expression: bool,
) -> Result<()> {
    if expression.is_some() && candidate.is_some() {
        anyhow::bail!("Use either --candidate or --expr, not both.");
    }
    if expression.is_none() && (region.is_some() || universe.is_some() || delay.is_some()) {
        anyhow::bail!("--region, --universe, and --delay apply only when using --expr.");
    }

    let client = get_main_client()?;
    let requested_candidate_id = candidate.clone();
    let candidate_source: Option<(String, FieldConfig)> = if let Some(alpha_id) = candidate.as_ref()
    {
        let details = client.get_alpha_details(alpha_id).await?;
        let code = details
            .get_code()
            .context("BRAIN did not return a formula for this Alpha")?
            .to_string();
        let settings = details.settings.unwrap_or_default();
        Some((
            code,
            FieldConfig {
                instrument_type: settings.instrument_type,
                region: settings.region,
                universe: settings.universe,
                delay: settings.delay,
            },
        ))
    } else {
        expression.map(|code| {
            (
                code,
                FieldConfig {
                    instrument_type: "EQUITY".to_string(),
                    region: region.unwrap_or_else(|| "USA".to_string()),
                    universe: universe.unwrap_or_else(|| "TOP1000".to_string()),
                    delay: delay.unwrap_or(1),
                },
            )
        })
    };

    let mut candidate_pnl: Option<HashMap<String, f64>> = None;
    let mut candidate_alpha_id = requested_candidate_id;
    if candidate_alpha_id.is_none() && simulate_expression {
        let (formula, config) = candidate_source
            .as_ref()
            .context("--simulate requires an expression")?;
        let settings = AlphaSettings {
            instrument_type: config.instrument_type.clone(),
            region: config.region.clone(),
            universe: config.universe.clone(),
            delay: config.delay,
            ..Default::default()
        };
        println!("  Running one private BRAIN simulation to obtain candidate PnL...");
        let simulation_url = client.submit_simulation(formula, &settings).await?;
        let result = client
            .poll_simulation(&simulation_url, 1800, 8)
            .await
            .context("BRAIN candidate simulation did not complete")?;
        let alpha_ref = result
            .alpha
            .as_deref()
            .context("BRAIN completed the simulation without returning an Alpha reference")?;
        let alpha_id = alpha_ref
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(alpha_ref)
            .to_string();
        candidate_pnl = Some(client.fetch_daily_pnl(&alpha_id).await?);
        candidate_alpha_id = Some(alpha_id);
    }

    let portfolio_path = Path::new("portfolio_os.json");
    let (portfolio, portfolio_available): (Vec<PortfolioAlpha>, bool) = if portfolio_path.exists() {
        let data = fs::read_to_string(portfolio_path)?;
        (serde_json::from_str(&data)?, true)
    } else {
        (Vec::new(), false)
    };

    let data_dir = Path::new("data");
    let local_field_datasets = load_local_field_datasets(data_dir);
    let mut field_cache: FieldUsageCache =
        fs::read_to_string(get_cache_dir().join("data_field_usage.json"))
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
    let now = current_unix_time();
    let mut live_fields = HashMap::new();
    let mut field_source = HashMap::new();

    let mut profile_lookups = 0usize;
    'profile_lookup: for alpha in &portfolio {
        let config = profile_field_config(alpha);
        let formula_fields = build_formula_allocation(
            alpha.code.as_deref().unwrap_or_default(),
            &local_field_datasets,
            &HashMap::new(),
        )
        .fields;
        for field_id in formula_fields.iter().take(3) {
            if profile_lookups >= 20 {
                break 'profile_lookup;
            }
            let key = field_cache_key(field_id, &config);
            if get_fresh_cached_field(&field_cache, &key, now).is_some() {
                continue;
            }
            profile_lookups += 1;
            match client
                .fetch_data_field(
                    field_id,
                    &config.instrument_type,
                    &config.region,
                    &config.universe,
                    config.delay,
                )
                .await
            {
                Ok(Some(field)) => {
                    cache_field(&mut field_cache, key, field, now);
                }
                Ok(None) => {}
                Err(error) => {
                    println!(
                        "  Field metadata lookup paused ({}); continuing with available cache and census.",
                        error
                    );
                    break 'profile_lookup;
                }
            }
        }
    }

    if let Some((formula, config)) = candidate_source.as_ref() {
        let field_ids =
            build_formula_allocation(formula, &local_field_datasets, &HashMap::new()).fields;
        for field_id in field_ids {
            let key = field_cache_key(&field_id, config);
            if let Some(field) = get_fresh_cached_field(&field_cache, &key, now) {
                let fetched_at = field_cache
                    .entries
                    .get(&key)
                    .map(|entry| entry.fetched_at_unix_secs)
                    .unwrap_or(now);
                let age_hours = now.saturating_sub(fetched_at) / 3600;
                field_source.insert(field_id.clone(), format!("cache, {}h old", age_hours));
                live_fields.insert(field_id, field);
                continue;
            }

            match client
                .fetch_data_field(
                    &field_id,
                    &config.instrument_type,
                    &config.region,
                    &config.universe,
                    config.delay,
                )
                .await
            {
                Ok(Some(field)) => {
                    cache_field(&mut field_cache, key, field.clone(), now);
                    field_source.insert(field_id.clone(), "BRAIN API (live)".to_string());
                    live_fields.insert(field_id, field);
                }
                Ok(None) => {
                    field_source.insert(field_id, "not returned for this config".to_string());
                }
                Err(_) => {
                    if let Some(cached) = field_cache.entries.get(&key) {
                        field_source.insert(
                            field_id.clone(),
                            "stale cache (API unavailable)".to_string(),
                        );
                        live_fields.insert(field_id, cached.field.clone());
                    } else {
                        field_source.insert(field_id, "unavailable".to_string());
                    }
                }
            }
        }
    }

    let cache_path = get_cache_dir().join("data_field_usage.json");
    if let Ok(contents) = serde_json::to_vec_pretty(&field_cache) {
        let _ = fs::write(cache_path, contents);
    }

    let mut cached_mapping_fields: HashMap<String, DataFieldEntry> = HashMap::new();
    let mut ambiguous_cached_ids = std::collections::HashSet::new();
    for cached in field_cache.entries.values() {
        let field = &cached.field;
        let Some(dataset_id) = field
            .dataset
            .as_ref()
            .and_then(|dataset| dataset.id.as_deref())
        else {
            continue;
        };
        let field_id = field.id.to_lowercase();
        if ambiguous_cached_ids.contains(&field_id) {
            continue;
        }
        if let Some(existing) = cached_mapping_fields.get(&field_id) {
            let existing_dataset = existing
                .dataset
                .as_ref()
                .and_then(|dataset| dataset.id.as_deref());
            if !existing_dataset
                .is_some_and(|existing_id| existing_id.eq_ignore_ascii_case(dataset_id))
            {
                cached_mapping_fields.remove(&field_id);
                ambiguous_cached_ids.insert(field_id);
            }
        } else {
            cached_mapping_fields.insert(field_id, field.clone());
        }
    }

    let mut portfolio_allocations: std::collections::BTreeMap<
        DatasetAllocationKey,
        AllocationSets,
    > = std::collections::BTreeMap::new();
    let mut alpha_dataset_fields: HashMap<String, ProfileDatasetFields> = HashMap::new();
    let mut portfolio_field_count = 0usize;
    let mut portfolio_inferred_count = 0usize;
    let mut portfolio_unmapped_count = 0usize;
    for alpha in &portfolio {
        let config = profile_field_config(alpha);
        let allocation = build_formula_allocation(
            alpha.code.as_deref().unwrap_or_default(),
            &local_field_datasets,
            &cached_mapping_fields,
        );
        portfolio_field_count += allocation.fields.len();
        portfolio_inferred_count += allocation.inferred_fields.len();
        portfolio_unmapped_count += allocation.unmapped_fields;
        for (dataset_id, fields) in &allocation.by_dataset {
            let key = (
                config.instrument_type.to_uppercase(),
                config.region.to_uppercase(),
                config.universe.to_uppercase(),
                config.delay,
                dataset_id.to_uppercase(),
            );
            alpha_dataset_fields
                .entry(alpha.id.clone())
                .or_default()
                .push((key.clone(), fields.clone()));
            let usage = portfolio_allocations.entry(key).or_default();
            usage.0.insert(alpha.id.clone());
            usage.2.extend(
                fields
                    .iter()
                    .filter(|field| allocation.inferred_fields.contains(*field))
                    .cloned(),
            );
            usage.1.extend(fields.iter().cloned());
        }
    }
    if profile_lookups > 0 {
        println!(
            "  Refreshed profile field allocation metadata with {} API lookups.",
            profile_lookups
        );
    }

    let mut candidate_mapping_fields = cached_mapping_fields;
    candidate_mapping_fields.extend(live_fields.clone());
    let candidate_allocation = candidate_source.as_ref().map(|(formula, _)| {
        build_formula_allocation(formula, &local_field_datasets, &candidate_mapping_fields)
    });

    let mut configs_by_region: HashMap<String, Vec<FieldConfig>> = HashMap::new();
    for alpha in &portfolio {
        let config = profile_field_config(alpha);
        let entries = configs_by_region.entry(config.region.clone()).or_default();
        if !entries.contains(&config) {
            entries.push(config);
        }
    }
    if let Some((_, config)) = candidate_source.as_ref() {
        let entries = configs_by_region.entry(config.region.clone()).or_default();
        if !entries.contains(config) {
            entries.push(config.clone());
        }
    }

    let mut datasets_by_region: HashMap<String, Vec<DatasetEntry>> = HashMap::new();
    for region_name in configs_by_region.keys() {
        match client.fetch_all_datasets(region_name).await {
            Ok(datasets) => {
                datasets_by_region.insert(region_name.clone(), datasets);
            }
            Err(_) => {
                println!("  Dataset census unavailable for region {}.", region_name);
            }
        }
    }

    let mut dataset_counts_by_key: HashMap<DatasetAllocationKey, f64> = HashMap::new();
    let mut observed_dataset_count = 0usize;
    for key in portfolio_allocations.keys() {
        let config = FieldConfig {
            instrument_type: key.0.clone(),
            region: key.1.clone(),
            universe: key.2.clone(),
            delay: key.3,
        };
        if let Some(count) = datasets_by_region
            .get(&config.region)
            .and_then(|datasets| find_dataset_config(datasets, &key.4, &config))
            .and_then(|dataset| dataset.alpha_count)
            .filter(|count| *count > 0)
        {
            let weight = count as f64;
            dataset_counts_by_key.insert(key.clone(), weight);
            observed_dataset_count += 1;
        }
    }
    let (allocation_weights, field_weighted_alpha_count, dataset_fallback_alpha_count) =
        derive_profile_usage_weights(
            &alpha_dataset_fields,
            &field_cache,
            &dataset_counts_by_key,
            now,
        );
    let total_sample_weight: f64 = allocation_weights.values().sum();

    let (mut pnl_cache, cache_age) = load_pnl_cache();
    let pnl_cache_path = get_cache_dir().join("portfolio_pnl.json");
    print_cache_status(cache_age, pnl_cache.len());

    if let (Some(alpha_id), Some(pnl)) = (candidate_alpha_id.as_ref(), candidate_pnl.as_ref()) {
        pnl_cache.insert(alpha_id.clone(), pnl.clone());
    }
    let mut pnl_ids: std::collections::BTreeSet<String> =
        portfolio.iter().map(|alpha| alpha.id.clone()).collect();
    if let Some(alpha_id) = candidate_alpha_id.as_ref() {
        pnl_ids.insert(alpha_id.clone());
    }
    let refresh_pnl_ids: Vec<String> = pnl_ids
        .into_iter()
        .filter(|alpha_id| {
            !pnl_cache.contains_key(alpha_id)
                && !(candidate_alpha_id.as_deref() == Some(alpha_id.as_str())
                    && candidate_pnl.is_some())
        })
        .collect();
    let mut stale_pnl_count = 0usize;
    for (index, alpha_id) in refresh_pnl_ids.iter().enumerate() {
        if index == 0 {
            println!("  Refreshing allocated Alpha PnL from the main account...");
        }
        println!("  PnL refresh {}/{}...", index + 1, refresh_pnl_ids.len());
        let previous = pnl_cache.get(alpha_id).cloned();
        match client.fetch_daily_pnl(alpha_id).await {
            Ok(pnl) => {
                pnl_cache.insert(alpha_id.clone(), pnl);
            }
            Err(error) => {
                if previous.is_some() {
                    stale_pnl_count += 1;
                }
                if candidate_alpha_id.as_deref() == Some(alpha_id.as_str()) {
                    println!("  Candidate PnL unavailable: {}", error);
                }
            }
        }
    }
    if let Some(alpha_id) = candidate_alpha_id.as_ref() {
        if candidate_pnl.is_none() {
            candidate_pnl = pnl_cache.get(alpha_id).cloned();
        }
        if let Some(pnl) = candidate_pnl.as_ref() {
            pnl_cache.insert(alpha_id.clone(), pnl.clone());
        }
    }
    if let Ok(contents) = serde_json::to_vec(&pnl_cache) {
        let _ = fs::write(&pnl_cache_path, contents);
    }
    if stale_pnl_count > 0 {
        println!(
            "  Used cached PnL for {} Alpha(s) whose live refresh failed.",
            stale_pnl_count
        );
    }

    println!("\n  Crowd usage and local PnL sensitivity (not projected Uniqueness)");
    if portfolio_available {
        println!(
            "  Local profile: {} stored OS alphas; {} formula fields parsed; {} prefix-inferred; {} unmapped.",
            portfolio.len(), portfolio_field_count, portfolio_inferred_count, portfolio_unmapped_count
        );
    } else {
        println!("  Local profile: portfolio_os.json not found; profile allocation omitted.");
    }
    println!("  Census counts come from /data-sets; field counts come from /data-fields (live or labeled cache).\n");

    if !portfolio_allocations.is_empty() || candidate_allocation.is_some() {
        println!(
            "  {:<18} {:<31} {:>10} {:>14} {:>10} {:>17} {:>12} {:>12}",
            "Dataset",
            "Config",
            "Profile α",
            "Mapped fields",
            "Inferred",
            "Candidate fields",
            "Users",
            "Alphas"
        );
        let mut report_rows = std::collections::BTreeSet::new();
        for key in portfolio_allocations.keys() {
            report_rows.insert(key.clone());
        }
        if let (Some(allocation), Some((_, config))) =
            (candidate_allocation.as_ref(), candidate_source.as_ref())
        {
            for dataset_id in allocation.by_dataset.keys() {
                report_rows.insert((
                    config.instrument_type.to_uppercase(),
                    config.region.to_uppercase(),
                    config.universe.to_uppercase(),
                    config.delay,
                    dataset_id.to_uppercase(),
                ));
            }
        }

        for key in report_rows {
            let config = FieldConfig {
                instrument_type: key.0.clone(),
                region: key.1.clone(),
                universe: key.2.clone(),
                delay: key.3,
            };
            let profile = portfolio_allocations.get(&key);
            let profile_alpha_count = profile.map(|usage| usage.0.len()).unwrap_or(0);
            let profile_fields = profile.map(|usage| usage.1.len()).unwrap_or(0);
            let profile_inferred_fields = profile.map(|usage| usage.2.len()).unwrap_or(0);
            let candidate_fields = candidate_allocation
                .as_ref()
                .filter(|_| {
                    candidate_source
                        .as_ref()
                        .is_some_and(|(_, candidate_config)| candidate_config == &config)
                })
                .and_then(|allocation| allocation.by_dataset.get(&key.4.to_lowercase()))
                .map(Vec::len)
                .unwrap_or(0);
            let census = datasets_by_region
                .get(&config.region)
                .and_then(|datasets| find_dataset_config(datasets, &key.4, &config));
            let (users, alphas) = census
                .map(|dataset| {
                    (
                        format_count(dataset.user_count),
                        format_count(dataset.alpha_count),
                    )
                })
                .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));
            println!(
                "  {:<18} {:<31} {:>10} {:>14} {:>10} {:>17} {:>12} {:>12}",
                key.4,
                field_config_label(&config),
                profile_alpha_count,
                profile_fields,
                profile_inferred_fields,
                candidate_fields,
                users,
                alphas
            );
        }
    } else {
        println!("  No mappable dataset allocations found in the local profile or candidate.");
    }

    if let (Some(allocation), Some((_, config))) =
        (candidate_allocation.as_ref(), candidate_source.as_ref())
    {
        println!(
            "\n  Candidate field-level usage ({})",
            field_config_label(config)
        );
        if allocation.fields.is_empty() {
            println!("  No field identifiers were extracted from the formula.");
        } else {
            println!(
                "  {:<30} {:<18} {:>12} {:>12}  Source",
                "Field", "Dataset", "Users", "Alphas"
            );
            for field_id in &allocation.fields {
                let field = live_fields.get(field_id);
                let dataset = allocation
                    .by_dataset
                    .iter()
                    .find(|(_, fields)| fields.iter().any(|field| field == field_id))
                    .map(|(dataset, _)| dataset.as_str())
                    .unwrap_or("unknown");
                let source = field_source
                    .get(field_id)
                    .map(String::as_str)
                    .unwrap_or_else(|| {
                        if allocation.inferred_fields.contains(field_id) {
                            "prefix inferred from local schema"
                        } else {
                            "local schema mapping"
                        }
                    });
                println!(
                    "  {:<30} {:<18} {:>12} {:>12}  {}",
                    field_id,
                    dataset,
                    format_count(field.and_then(|entry| entry.user_count)),
                    format_count(field.and_then(|entry| entry.alpha_count)),
                    source
                );
            }

            let mut measured_fields = 0usize;
            let mut alpha_penetration_sum = 0.0;
            let mut alpha_penetration_count = 0usize;
            let mut user_penetration_sum = 0.0;
            let mut user_penetration_count = 0usize;
            for (dataset_id, fields) in &allocation.by_dataset {
                let dataset = datasets_by_region
                    .get(&config.region)
                    .and_then(|datasets| find_dataset_config(datasets, dataset_id, config));
                for field_id in fields {
                    if field_source
                        .get(field_id)
                        .is_some_and(|source| source.starts_with("stale cache"))
                    {
                        continue;
                    }
                    let Some(field) = live_fields.get(field_id) else {
                        continue;
                    };
                    let Some(dataset) = dataset else {
                        continue;
                    };
                    let mut field_has_comparable_count = false;
                    if let (Some(field_count), Some(dataset_count)) =
                        (field.alpha_count, dataset.alpha_count)
                    {
                        if field_count >= 0 && dataset_count > 0 {
                            alpha_penetration_sum += field_count as f64 / dataset_count as f64;
                            alpha_penetration_count += 1;
                            field_has_comparable_count = true;
                        }
                    }
                    if let (Some(field_count), Some(dataset_count)) =
                        (field.user_count, dataset.user_count)
                    {
                        if field_count >= 0 && dataset_count > 0 {
                            user_penetration_sum += field_count as f64 / dataset_count as f64;
                            user_penetration_count += 1;
                            field_has_comparable_count = true;
                        }
                    }
                    if field_has_comparable_count {
                        measured_fields += 1;
                    }
                }
            }
            if alpha_penetration_count > 0 || user_penetration_count > 0 {
                let alpha_share = if alpha_penetration_count > 0 {
                    format!(
                        "{:.1}%",
                        100.0 * alpha_penetration_sum / alpha_penetration_count as f64
                    )
                } else {
                    "unavailable".to_string()
                };
                let user_share = if user_penetration_count > 0 {
                    format!(
                        "{:.1}%",
                        100.0 * user_penetration_sum / user_penetration_count as f64
                    )
                } else {
                    "unavailable".to_string()
                };
                println!(
                    "  Usage penetration proxy: mean field share of dataset Alphas = {}; mean share of dataset users = {} ({} of {} fields measured).",
                    alpha_share,
                    user_share,
                    measured_fields,
                    allocation.fields.len()
                );
                println!("  This reports popularity of the formula's fields; it is not a PnL-correlation or Uniqueness estimate.");
            } else {
                println!("  Usage penetration proxy unavailable: matching field and dataset counts were not available.");
            }
            if allocation.unmapped_fields > 0 {
                println!(
                    "  {} field identifier(s) could not be mapped to a dataset.",
                    allocation.unmapped_fields
                );
            }
        }
    }

    let sampled_datasets: std::collections::HashSet<DatasetAllocationKey> = portfolio_allocations
        .iter()
        .filter(|(_, usage)| {
            usage
                .0
                .iter()
                .any(|alpha_id| pnl_cache.contains_key(alpha_id))
        })
        .filter_map(|(key, _)| {
            dataset_counts_by_key
                .contains_key(key)
                .then_some(key.clone())
        })
        .collect();
    let usage_weighted_pnl = build_usage_weighted_profile_pnl(
        &allocation_weights,
        &pnl_cache,
        total_sample_weight,
        sampled_datasets.len(),
    );
    let competitions = client.fetch_competitions().await.unwrap_or_default();
    let official_score = challenge_uniqueness_score(&competitions);
    let profile_alpha_ids: Vec<String> = portfolio
        .iter()
        .filter(|alpha| pnl_cache.contains_key(&alpha.id))
        .map(|alpha| alpha.id.clone())
        .collect();
    let profile_pnl = equal_weight_pnl_merge(&profile_alpha_ids, &pnl_cache);

    println!("\n  Local PnL sensitivity (not competitor PnL)");
    println!(
        "  Local PnL series: {} of {} active Alphas; field-count weights for {} Alphas; {} dataset configurations with counts.",
        usage_weighted_pnl.as_ref().map(|proxy| proxy.sample_alpha_count).unwrap_or(0),
        portfolio.len(),
        field_weighted_alpha_count,
        observed_dataset_count
    );
    println!(
        "  Dataset fallback weights used for {} Alphas; local PnL weight coverage: {:.1}%.",
        dataset_fallback_alpha_count,
        usage_weighted_pnl
            .as_ref()
            .map(|proxy| proxy.pnl_weight_coverage * 100.0)
            .unwrap_or(0.0)
    );

    if let Some(score) = official_score {
        println!(
            "  Current official BRAIN Uniqueness: {:.3} (current value only).",
            score
        );
    } else {
        println!("  Current official BRAIN Uniqueness was not available from the account API.");
    }

    match (usage_weighted_pnl.as_ref(), profile_pnl.as_ref()) {
        (Some(proxy), Some(profile_series)) => {
            let (baseline_proxy_corr_opt, common_days) =
                pearson_correlation(profile_series, &proxy.pnl);
            println!(
                "  Usage-weighted local PnL basket: {} Alpha samples across {} dataset configurations; {} common days.",
                proxy.sample_alpha_count, proxy.sample_dataset_count, common_days
            );
            let baseline_corr_str = match baseline_proxy_corr_opt {
                Some(c) => format!("{:.3}", c),
                None => "N/A (insufficient data)".to_string(),
            };
            println!(
                "  Equal-weight profile vs usage-weighted local basket: {} correlation.",
                baseline_corr_str
            );

            if let Some(alpha_id) = candidate_alpha_id.as_ref() {
                if portfolio.iter().any(|alpha| alpha.id.eq_ignore_ascii_case(alpha_id)) {
                    println!("  Candidate is already active in this profile; no add-on projection was made.");
                } else if candidate_pnl.is_some() {
                    let mut projected_ids = profile_alpha_ids.clone();
                    projected_ids.push(alpha_id.clone());
                    if let Some(projected_pnl) = equal_weight_pnl_merge(&projected_ids, &pnl_cache) {
                        if let Some(projection) = project_proxy_correlation(
                            profile_series,
                            &projected_pnl,
                            &proxy.pnl,
                        ) {
                            println!(
                                "  Candidate impact on local basket correlation: {:.3} -> {:.3} ({:+.3}).",
                                projection.baseline_proxy_corr,
                                projection.projected_proxy_corr,
                                projection.correlation_delta
                            );
                        } else {
                            println!("  Not enough overlapping PnL dates to estimate candidate impact.");
                        }
                    } else {
                        println!("  Could not merge candidate and profile PnL over common dates.");
                    }
                } else {
                    println!("  Candidate PnL is unavailable, so local PnL sensitivity was not computed.");
                }
            } else if candidate_source.is_some() {
                println!("  An expression has no PnL yet. Use --simulate with --expr for local portfolio sensitivity only.");
            }

            if proxy.pnl_weight_coverage < 0.70 || proxy.sample_alpha_count < 5 {
                println!("  Data coverage: LOW — few local Alpha PnLs or limited weight coverage.");
            } else {
                println!("  Data coverage: adequate for this local diagnostic only; it says nothing quantitative about competitor PnL.");
            }
        }
        _ => println!("  No local sensitivity: need mapped active Alphas, their PnL series, and at least 30 overlapping days."),
    }

    println!("\n  Weight method: average fresh field alphaCount within each mapped dataset, then average across that Alpha's datasets; use dataset alphaCount where field counts are unavailable.");
    println!("  This reweights only your local profile PnLs. BRAIN does not return the other competitors' formulas or merged PnLs through these endpoints.");
    println!("  Therefore only the current official Uniqueness is known; this command does not forecast a future Uniqueness score.");
    Ok(())
}

/// Evaluates dataset-interaction heuristics and formula synergy.
pub async fn cmd_matrix(
    universe: String,
    expr: Option<String>,
    green_only: bool,
    region: String,
) -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind — Configured Dataset-Interaction Heuristic"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!(
        "  Fetching dynamic platform dataset census from BRAIN API (Region: {})...",
        region.cyan()
    );

    let datasets = match client.fetch_all_datasets(&region).await {
        Ok(ds) => ds,
        Err(e) => {
            println!(
                "  ⚠️ Warning: Live API fetch failed ({}), falling back to cached snapshot.",
                e
            );
            let snapshot_file = Path::new("data").join("census_snapshots.json");
            if let Ok(content) = fs::read_to_string(&snapshot_file) {
                if let Ok(snap) = serde_json::from_str::<CensusSnapshot>(&content) {
                    if snap.region.eq_ignore_ascii_case(&region) {
                        snap.datasets
                    } else {
                        anyhow::bail!(
                            "Cached census is for region {}, requested {}.",
                            snap.region,
                            region
                        );
                    }
                } else {
                    anyhow::bail!("No cached census snapshot available");
                }
            } else {
                anyhow::bail!("No cached census snapshot available");
            }
        }
    };

    println!(
        "  {} Analyzed {} platform dataset configurations.",
        "✅".green(),
        datasets.len().to_string().bold().green()
    );

    let data_dir = Path::new("data");
    let snapshot_file = data_dir.join("census_snapshots.json");
    let prev_snapshot: Option<CensusSnapshot> = if snapshot_file.exists() {
        fs::read_to_string(&snapshot_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .filter(|snapshot: &CensusSnapshot| snapshot.region.eq_ignore_ascii_case(&region))
    } else {
        None
    };

    let report = build_dynamic_matrix(&datasets, prev_snapshot.as_ref(), &universe);
    print_dynamic_matrix(&report, green_only);

    if let Some(ref formula) = expr {
        println!(
            "\n  {}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "  {}",
            "  Multi-Dataset Formula Hyper-Synergy Analysis (K >= 2)"
                .bold()
                .cyan()
        );
        println!(
            "  {}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );

        let analysis =
            analyze_formula_hyper_synergy(formula, &universe, &datasets, prev_snapshot.as_ref());

        println!("\n  {} Formula Architecture Assessment:", "🧬".bold());
        println!(
            "    Target Universe:            {}",
            analysis.universe.yellow().bold()
        );
        println!(
            "    Detected Datasets Count:    {} Datasets",
            analysis.detected_k.to_string().cyan().bold()
        );
        println!(
            "    Fusion Architecture Pattern: {}",
            analysis.fusion_pattern.badge()
        );

        println!(
            "\n  {} Participating Factor Pillars Breakdown (K = {}):",
            "🏛️".bold(),
            analysis.detected_k
        );
        println!("  ┌────────┬──────────────────────────────────────────┬──────────────┬──────────────┬────────────┐");
        println!("  │ Pillar │ Economic Architecture Role               │ Users (data) │ Alphas (data)│ Count band │");
        println!("  ├────────┼──────────────────────────────────────────┼──────────────┼──────────────┼────────────┤");

        for (pillar, role, stat) in &analysis.participating_pillars {
            println!(
                "  │ {:<6} │ {:<40} │ {:>12} │ {:>12} │ {:<10} │",
                pillar.cyan().bold(),
                role.badge(),
                stat.live_users,
                stat.live_alphas,
                stat.status
            );
        }
        println!("  └────────┴──────────────────────────────────────────┴──────────────┴──────────────┴────────────┘");

        let chui_styled = if analysis.composite_uniqueness_index >= 0.85 {
            format!("{:.3}", analysis.composite_uniqueness_index)
                .green()
                .bold()
        } else if analysis.composite_uniqueness_index >= 0.70 {
            format!("{:.3}", analysis.composite_uniqueness_index).green()
        } else if analysis.composite_uniqueness_index >= 0.50 {
            format!("{:.3}", analysis.composite_uniqueness_index).yellow()
        } else {
            format!("{:.3}", analysis.composite_uniqueness_index)
                .red()
                .bold()
        };

        println!(
            "\n  {} Composite Configured-Interaction Heuristic (CHUI): {}",
            "🎯".bold(),
            chui_styled
        );
        println!(
            "  {} Heuristic interpretation (not platform Uniqueness): {}",
            "⚖️".bold(),
            analysis.uniqueness_verdict
        );

        if !analysis.dynamic_migration_warnings.is_empty() {
            println!(
                "\n  {} Dynamic Crowd Migration Warnings:",
                "🚨".red().bold()
            );
            for w in &analysis.dynamic_migration_warnings {
                println!("    {}", w.yellow());
            }
        }

        if !analysis.economic_recommendations.is_empty() {
            println!("\n  {} Economic Architecture Recommendations:", "💡".bold());
            for r in &analysis.economic_recommendations {
                println!("    👉 {}", r.green());
            }
        }
    } else {
        println!(
            "\n  {} Rule-Based Multi-Dataset Hypotheses for {} (not validated forecasts):",
            "🌟".bold(),
            universe.yellow().bold()
        );
        println!(
            "    1. RISK x SHORT x MICRO  -> (model51 Anchor x Short Squeeze Catalyst x VWAP Dislocation)"
        );
        println!(
            "    2. NEWS18 x SHORT x MICRO -> (RavenPack Novelty x Borrow Squeeze x Reversal Dampening)"
        );
        println!(
            "    3. RISK x ANL x MICRO    -> (Idiosyncratic Risk x Consensus Revision Drift x Liquidity Shock)"
        );
        println!(
            "\n  💡 Tip: Run with --expr \"<fastexpr>\" to evaluate any multi-dataset alpha formula!"
        );
    }

    Ok(())
}

/// Generates rule-based portfolio gap and candidate suggestions.
pub async fn cmd_blueprint(universe: Option<String>, export: Option<String>) -> Result<()> {
    // 1. Load active OS portfolio
    let port_path = "portfolio_os.json";
    if !Path::new(port_path).exists() {
        anyhow::bail!("portfolio_os.json not found! Run 'alphafind sync' first.");
    }
    let port_data = fs::read_to_string(port_path)?;
    let os_alphas: Vec<PortfolioAlpha> = serde_json::from_str(&port_data)?;

    // 2. Load cached baseline PnLs
    let (pnl_cache, cache_age) = load_pnl_cache();
    print_cache_status(cache_age, pnl_cache.len());

    // 3. Generate inverse blueprint report
    let report = generate_inverse_blueprint(&os_alphas, &pnl_cache, universe.as_deref());

    // 4. Print master report
    print_blueprint_report(&report);

    // 5. Optional file export
    if let Some(export_path) = export {
        let mut out = String::new();
        out.push_str("# AlphaFind Inverse Portfolio Optimization & Forecast Blueprint\n\n");
        out.push_str(&format!(
            "Generated for: {} Out-of-Sample Alphas\n\n",
            report.active_alpha_count
        ));
        for (i, arch) in report.top_archetypes.iter().enumerate() {
            out.push_str(&format!(
                "## Archetype #{}: {} [{}]\n",
                i + 1,
                arch.name,
                arch.target_universe
            ));
            out.push_str(&format!(
                "- Primary Factor: {} x {}\n",
                arch.primary_pillar, arch.secondary_catalyst
            ));
            out.push_str(&format!(
                "- Datasets: Anchor: {} | Catalyst: {}\n",
                arch.anchor_dataset, arch.catalyst_dataset
            ));
            out.push_str(&format!("- Optimal Settings: Universe: {}, Decay: {}, Neutralization: {}, Power: {:.1}, Truncation: {:.3}\n",
                arch.target_universe, arch.recommended_decay, arch.recommended_neutralization, arch.recommended_power, arch.recommended_truncation));
            out.push_str(&format!(
                "- Economic Rationale: {}\n",
                arch.economic_rationale
            ));
            out.push_str("```python\n");
            out.push_str(&arch.fast_expr_skeleton);
            out.push_str("\n```\n\n");
        }
        fs::write(&export_path, out)?;
        println!(
            "  💾 Successfully exported blueprint skeletons to {}\n",
            export_path.green().bold()
        );
    }

    Ok(())
}
