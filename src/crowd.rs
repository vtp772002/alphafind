//! [DEPRECATED] Legacy heuristic crowd risk models.
//!
//! Superseded by `crowd_allocation`, `crowd_model`, and `shadow` modules
//! which provide usage-weighted profile PnL synthesis and proxy sensitivity.

use crate::models::DatasetEntry;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrowdTier {
    Pristine, // <= 50 users
    Safe,     // 51 - 300 users
    Moderate, // 301 - 2,000 users
    Crowded,  // 2,001 - 15,000 users
    Danger,   // > 15,000 users
}

impl CrowdTier {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::Pristine => "LOW OBSERVED USAGE (<=50 users)".green().bold(),
            Self::Safe => "LOW OBSERVED USAGE (<=300 users)".green(),
            Self::Moderate => "MODERATE OBSERVED USAGE (300-2k users)".yellow(),
            Self::Crowded => "HIGH OBSERVED USAGE (2k-15k users)".red(),
            Self::Danger => "VERY HIGH OBSERVED USAGE (>15k users)".red().bold(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeutralizationShield {
    Strong,   // SUBINDUSTRY
    Moderate, // INDUSTRY
    Weak,     // SECTOR
    Exposed,  // MARKET or NONE
}

impl NeutralizationShield {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::Strong => "SUBINDUSTRY SETTING (effect on crowd PnL not measured)"
                .green()
                .bold(),
            Self::Moderate => "MODERATE (INDUSTRY)".yellow(),
            Self::Weak => "WEAK (SECTOR)".yellow(),
            Self::Exposed => "MARKET/NONE SETTING (effect on crowd PnL not measured)"
                .red()
                .bold(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UniquenessRisk {
    Low,
    Moderate,
    High,
    Critical,
}

impl UniquenessRisk {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::Low => "🟢 LOW HEURISTIC USAGE RISK (not official Uniqueness)"
                .green()
                .bold(),
            Self::Moderate => "🟡 MODERATE HEURISTIC USAGE RISK (not official Uniqueness)"
                .yellow()
                .bold(),
            Self::High => "🟠 HIGH HEURISTIC USAGE RISK (not official Uniqueness)"
                .truecolor(255, 140, 0)
                .bold(),
            Self::Critical => "🔴 VERY HIGH HEURISTIC USAGE RISK (not official Uniqueness)"
                .red()
                .bold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrowdRiskReport {
    pub detected_dataset: String,
    pub detected_pillar: String,
    pub universe: String,
    pub neutralization: String,
    pub user_count: i64,
    pub alpha_count: i64,
    pub crowd_tier: CrowdTier,
    pub neutralization_shield: NeutralizationShield,
    pub overall_uniqueness_risk: UniquenessRisk,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Identifies likely dataset ID and factor pillar from alpha formula expression
pub fn detect_dataset_and_pillar(expr: &str) -> (&'static str, &'static str) {
    let lower = expr.to_lowercase();

    if lower.contains("implied_vol")
        || lower.contains("iv_call")
        || lower.contains("iv_put")
        || lower.contains("option")
        || lower.contains("put_call")
    {
        ("option8", "OPTIONS_VRP")
    } else if lower.contains("unsystematic_risk") || lower.contains("residual_vol") {
        ("model51", "IDIOSYNCRATIC_RISK")
    } else if lower.contains("shares_sold_short")
        || lower.contains("shorted_shares")
        || lower.contains("short_")
        || lower.contains("borrow_")
    {
        ("short", "SHORT_INTEREST")
    } else if lower.contains("nws")
        || lower.contains("news")
        || lower.contains("headline")
        || lower.contains("sentiment")
    {
        ("news12", "NEWS_EVENT")
    } else if lower.contains("actual_eps")
        || lower.contains("actual_adj_net")
        || lower.contains("est_")
        || lower.contains("consensus")
    {
        ("analyst4", "ANALYST_CONSENSUS")
    } else if lower.contains("fnd6_")
        || lower.contains("ebit")
        || lower.contains("assets")
        || lower.contains("liabilities")
        || lower.contains("debt")
        || lower.contains("cash_flow")
    {
        ("fundamental6", "FINANCIAL_HEALTH_QUALITY")
    } else {
        ("pv1", "MICROSTRUCTURE_VWAP")
    }
}

/// Loads cached dataset entries from data/census_snapshots.json if available
pub fn load_cached_census() -> Vec<DatasetEntry> {
    let path = Path::new("data").join("census_snapshots.json");
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<crate::models::CensusSnapshot>(&content) {
            return parsed.datasets;
        }
    }
    Vec::new()
}

/// Legacy heuristic based on cached census rows and static fallbacks.
/// This does not estimate crowd PnL or official BRAIN Uniqueness.
#[deprecated(note = "Use the live dataset/field usage report from `alphafind shadow`.")]
pub fn evaluate_crowd_risk(
    expression: &str,
    universe: &str,
    neutralization: &str,
    census_override: Option<&[DatasetEntry]>,
) -> CrowdRiskReport {
    let (dataset_id, pillar) = detect_dataset_and_pillar(expression);
    let cached_census = if census_override.is_none() {
        load_cached_census()
    } else {
        Vec::new()
    };
    let census = census_override.unwrap_or(&cached_census);

    let u_clean = universe.trim().to_uppercase();
    let neut_clean = neutralization.trim().to_uppercase();

    // Look for dataset x universe match in census
    let mut matched_entry: Option<&DatasetEntry> = None;
    for d in census {
        if d.id.eq_ignore_ascii_case(dataset_id) {
            if let Some(ref d_u) = d.universe {
                if d_u.to_uppercase() == u_clean {
                    matched_entry = Some(d);
                    break;
                }
            }
        }
    }

    // Fallback: match dataset ID only
    if matched_entry.is_none() {
        for d in census {
            if d.id.eq_ignore_ascii_case(dataset_id) {
                matched_entry = Some(d);
                break;
            }
        }
    }

    // Known default platform statistics if not in census snapshot
    let (users, alphas) = match matched_entry {
        Some(entry) => (
            entry.user_count.unwrap_or(100),
            entry.alpha_count.unwrap_or(200),
        ),
        None => match (dataset_id, u_clean.as_str()) {
            ("option8", "TOP3000") => (34103, 190405),
            ("option8", "TOP1000") => (7574, 13327),
            ("option8", "TOP500") | ("option8", "TOPSP500") => (2758, 5151),
            ("analyst4", "TOP3000") => (49471, 760206),
            ("analyst4", "TOP500") | ("analyst4", "TOPSP500") => (2789, 5689),
            ("fundamental6", _) => (88698, 850369),
            ("pv1", _) => (234, 388),
            ("news12", _) => (10, 12),
            ("news18", _) => (193, 637),
            ("model51", _) => (210, 420),
            ("short", _) => (150, 310),
            _ => (150, 300),
        },
    };

    let crowd_tier = if users <= 50 {
        CrowdTier::Pristine
    } else if users <= 300 {
        CrowdTier::Safe
    } else if users <= 2000 {
        CrowdTier::Moderate
    } else if users <= 15000 {
        CrowdTier::Crowded
    } else {
        CrowdTier::Danger
    };

    let shield = if neut_clean == "SUBINDUSTRY" {
        NeutralizationShield::Strong
    } else if neut_clean == "INDUSTRY" {
        NeutralizationShield::Moderate
    } else if neut_clean == "SECTOR" {
        NeutralizationShield::Weak
    } else {
        NeutralizationShield::Exposed
    };

    // Calculate Uniqueness Risk
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();

    let uniqueness_risk = match (crowd_tier, shield) {
        (CrowdTier::Danger, NeutralizationShield::Exposed) => {
            warnings.push(format!(
                "🔴 HIGH OBSERVED DATASET USAGE: '{}' on {} has {} users ({} alphas); this does not measure PnL correlation.",
                dataset_id, u_clean, users, alphas
            ));
            warnings.push(format!(
                "Neutralization '{}' is a setting; its effect on other users' PnL correlation is unknown.",
                neut_clean
            ));
            recommendations.push(
                "Compare neutralization variants by simulation; usage counts alone do not establish crowd co-movement."
                    .to_string(),
            );
            if u_clean == "TOP3000" && dataset_id == "option8" {
                recommendations.push("Avoid TOP3000 for options! Shift to TOP500 or TOP1000 where crowd is 12x-37x smaller.".to_string());
            }
            UniquenessRisk::Critical
        }
        (CrowdTier::Danger, _) => {
            warnings.push(format!(
                "🟠 HIGH OBSERVED DATASET USAGE: '{}' has {} users; {} neutralization has no measured protection in this report.",
                dataset_id, users, neut_clean
            ));
            recommendations.push("Evaluate any additional data source with simulations; low usage does not guarantee diversification.".to_string());
            UniquenessRisk::High
        }
        (CrowdTier::Crowded, NeutralizationShield::Exposed) => {
            warnings.push(format!(
                "🟠 HIGH OBSERVED DATASET USAGE: {} users in '{}' with {} neutralization; PnL relationship is unknown.",
                users, dataset_id, neut_clean
            ));
            recommendations.push(
                "Compare neutralization variants by simulation; this count cannot predict platform PnL co-movement."
                    .to_string(),
            );
            UniquenessRisk::High
        }
        (CrowdTier::Crowded, _) => {
            recommendations
                .push("Use direct simulation results to choose neutralization.".to_string());
            UniquenessRisk::Moderate
        }
        (CrowdTier::Moderate, NeutralizationShield::Exposed) => {
            warnings.push(format!(
                "🟡 Neutralization is {}; this usage count does not reveal peers' positions or PnL.",
                neut_clean
            ));
            recommendations
                .push("Compare settings on the same expression and evaluation period.".to_string());
            UniquenessRisk::Moderate
        }
        (CrowdTier::Moderate, _) => UniquenessRisk::Low,
        (CrowdTier::Safe | CrowdTier::Pristine, _) => {
            recommendations.push("Observed usage is low; this does not establish high Uniqueness or low PnL correlation.".to_string());
            UniquenessRisk::Low
        }
    };

    CrowdRiskReport {
        detected_dataset: dataset_id.to_string(),
        detected_pillar: pillar.to_string(),
        universe: u_clean,
        neutralization: neut_clean,
        user_count: users,
        alpha_count: alphas,
        crowd_tier,
        neutralization_shield: shield,
        overall_uniqueness_risk: uniqueness_risk,
        warnings,
        recommendations,
    }
}

/// Prints the legacy crowd usage heuristic. Counts may be stale and are not Uniqueness.
pub fn print_crowd_risk_audit(report: &CrowdRiskReport) {
    println!("\n  📊 LEGACY DATASET USAGE HEURISTIC (may be stale; not official Uniqueness)");
    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!(
        "    Economic Factor Pillar:     {}",
        report.detected_pillar.cyan().bold()
    );
    println!(
        "    Underlying Dataset:         {} ({})",
        report.detected_dataset.yellow(),
        report.universe.white()
    );
    println!(
        "    Historical Crowd Usage:     {} ({} users | {} alphas)",
        report.crowd_tier.badge(),
        report.user_count,
        report.alpha_count
    );
    println!(
        "    Neutralization Setting:     {}",
        report.neutralization_shield.badge()
    );
    println!("  ─────────────────────────────────────────────────────────────────────────");
    println!(
        "    HEURISTIC USAGE LABEL:      {}",
        report.overall_uniqueness_risk.badge()
    );

    if !report.warnings.is_empty() {
        println!("\n    Crowd Warnings:");
        for w in &report.warnings {
            println!("      {}", w);
        }
    }

    if !report.recommendations.is_empty() {
        println!("\n    Strategic Recommendations:");
        for r in &report.recommendations {
            println!("      👉 {}", r.green());
        }
    }
    println!("  ─────────────────────────────────────────────────────────────────────────");
}
