use crate::models::{DataFieldEntry, DatasetEntry};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const FIELD_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldConfig {
    pub instrument_type: String,
    pub region: String,
    pub universe: String,
    pub delay: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldUsageCache {
    #[serde(default)]
    pub entries: HashMap<String, CachedFieldUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFieldUsage {
    pub fetched_at_unix_secs: u64,
    pub field: DataFieldEntry,
}

#[derive(Debug, Clone, Default)]
pub struct FormulaAllocation {
    pub fields: Vec<String>,
    pub by_dataset: BTreeMap<String, Vec<String>>,
    pub inferred_fields: BTreeSet<String>,
    pub unmapped_fields: usize,
}

pub fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn field_cache_key(field_id: &str, config: &FieldConfig) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        config.instrument_type.to_uppercase(),
        config.region.to_uppercase(),
        config.universe.to_uppercase(),
        config.delay,
        field_id.to_lowercase()
    )
}

pub fn get_fresh_cached_field(
    cache: &FieldUsageCache,
    key: &str,
    now: u64,
) -> Option<DataFieldEntry> {
    cache.entries.get(key).and_then(|cached| {
        (now.saturating_sub(cached.fetched_at_unix_secs) <= FIELD_CACHE_TTL_SECS)
            .then(|| cached.field.clone())
    })
}

pub fn cache_field(cache: &mut FieldUsageCache, key: String, field: DataFieldEntry, now: u64) {
    cache.entries.insert(
        key,
        CachedFieldUsage {
            fetched_at_unix_secs: now,
            field,
        },
    );
}

/// Reads the repository's downloaded field metadata only to map field IDs to datasets.
/// Counts from these files are deliberately not treated as live measurements.
pub fn load_local_field_datasets(data_dir: &Path) -> HashMap<String, BTreeSet<String>> {
    let mut result: HashMap<String, BTreeSet<String>> = HashMap::new();
    let Ok(files) = fs::read_dir(data_dir) else {
        return result;
    };

    for file in files.flatten() {
        let path = file.path();
        let is_field_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("fields_") && name.ends_with(".json"))
            .unwrap_or(false);
        if !is_field_file {
            continue;
        }

        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(&content) else {
            continue;
        };

        for row in rows {
            let Some(field_id) = row.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(dataset_id) = row
                .get("dataset")
                .and_then(|dataset| dataset.get("id"))
                .and_then(|value| value.as_str())
            else {
                continue;
            };
            result
                .entry(field_id.to_lowercase())
                .or_default()
                .insert(dataset_id.to_lowercase());
        }
    }
    result
}

/// Extracts non-function identifiers from a Fast Expression. The result is a set of
/// field candidates; callers must confirm them against BRAIN's data-fields endpoint.
pub fn extract_field_candidates(expression: &str) -> Vec<String> {
    let characters: Vec<char> = expression.chars().collect();
    let mut fields = BTreeSet::new();
    let mut index = 0;

    while index < characters.len() {
        if characters[index] == '"' || characters[index] == '\'' {
            let quote = characters[index];
            index += 1;
            while index < characters.len() {
                if characters[index] == '\\' {
                    index = (index + 2).min(characters.len());
                } else if characters[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            continue;
        }

        if !characters[index].is_ascii_alphanumeric() && characters[index] != '_' {
            index += 1;
            continue;
        }

        let start = index;
        while index < characters.len()
            && (characters[index].is_ascii_alphanumeric() || characters[index] == '_')
        {
            index += 1;
        }

        let token: String = characters[start..index].iter().collect();
        let lower = token.to_lowercase();
        if lower
            .chars()
            .all(|character| character.is_ascii_digit() || character == '_')
            || is_reserved_identifier(&lower)
        {
            continue;
        }

        let mut lookahead = index;
        while lookahead < characters.len() && characters[lookahead].is_whitespace() {
            lookahead += 1;
        }
        if lookahead < characters.len() && matches!(characters[lookahead], '(' | '=') {
            continue;
        }

        fields.insert(lower);
    }

    fields.into_iter().collect()
}

fn is_reserved_identifier(value: &str) -> bool {
    matches!(
        value,
        "true"
            | "false"
            | "nan"
            | "inf"
            | "and"
            | "or"
            | "not"
            | "if"
            | "else"
            | "market"
            | "sector"
            | "industry"
            | "subindustry"
            | "sub_industry"
            | "industry_group"
            | "country"
            | "exchange"
            | "currency"
            | "asset"
            | "cap"
            | "none"
            | "na"
    )
}

pub fn map_field_to_dataset(
    field_id: &str,
    local_field_datasets: &HashMap<String, BTreeSet<String>>,
    live_field: Option<&DataFieldEntry>,
) -> Option<String> {
    if let Some(dataset_id) = live_field
        .and_then(|field| field.dataset.as_ref())
        .and_then(|dataset| dataset.id.as_deref())
    {
        return Some(dataset_id.to_lowercase());
    }

    if let Some(datasets) = local_field_datasets.get(&field_id.to_lowercase()) {
        if datasets.len() == 1 {
            return datasets.iter().next().cloned();
        }
    }

    None
}

fn infer_dataset_from_local_prefix(
    field_id: &str,
    local_field_datasets: &HashMap<String, BTreeSet<String>>,
) -> Option<String> {
    let field = field_id.to_lowercase();
    if local_field_datasets
        .get(&field)
        .is_some_and(|datasets| datasets.len() != 1)
    {
        return None;
    }
    let prefix = field.split_once('_')?.0;
    let mut matching_datasets = BTreeSet::new();
    for (known_field, datasets) in local_field_datasets {
        if known_field
            .split_once('_')
            .map(|(known_prefix, _)| known_prefix)
            == Some(prefix)
        {
            matching_datasets.extend(datasets.iter().cloned());
        }
    }
    (matching_datasets.len() == 1)
        .then(|| matching_datasets.into_iter().next())
        .flatten()
}

pub fn build_formula_allocation(
    expression: &str,
    local_field_datasets: &HashMap<String, BTreeSet<String>>,
    live_fields: &HashMap<String, DataFieldEntry>,
) -> FormulaAllocation {
    let fields = extract_field_candidates(expression);
    let mut by_dataset: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut inferred_fields = BTreeSet::new();
    let mut unmapped_fields = 0;

    for field_id in &fields {
        let live_field = live_fields.get(field_id);
        if let Some(dataset_id) = map_field_to_dataset(field_id, local_field_datasets, live_field) {
            by_dataset
                .entry(dataset_id)
                .or_default()
                .push(field_id.clone());
        } else if let Some(dataset_id) =
            infer_dataset_from_local_prefix(field_id, local_field_datasets)
        {
            by_dataset
                .entry(dataset_id)
                .or_default()
                .push(field_id.clone());
            inferred_fields.insert(field_id.clone());
        } else {
            unmapped_fields += 1;
        }
    }

    FormulaAllocation {
        fields,
        by_dataset,
        inferred_fields,
        unmapped_fields,
    }
}

pub fn find_dataset_config<'a>(
    datasets: &'a [DatasetEntry],
    dataset_id: &str,
    config: &FieldConfig,
) -> Option<&'a DatasetEntry> {
    datasets.iter().find(|dataset| {
        dataset.id.eq_ignore_ascii_case(dataset_id)
            && dataset
                .region
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(&config.region))
                .unwrap_or(false)
            && dataset
                .universe
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case(&config.universe))
                .unwrap_or(false)
            && dataset.delay == Some(config.delay)
    })
}
