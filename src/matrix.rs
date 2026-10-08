use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{CensusSnapshot, DatasetEntry};
use crate::taxonomy::FactorPillar;

/// Classification of a participating dataset's economic role in a multi-dataset alpha
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetRole {
    /// Economic Anchor: Core structural driver (50%-70% signal power)
    EconomicAnchor,
    /// Catalyst: Non-linear event/risk trigger that activates the signal
    RiskCatalyst,
    /// Friction Hedge: Microstructure timing, VWAP dislocation, or reversal hedge
    FrictionHedge,
    /// Unclassified auxiliary component
    Auxiliary,
}

impl DatasetRole {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::EconomicAnchor => "ANCHOR (Structural Driver)".cyan().bold(),
            Self::RiskCatalyst => "CATALYST (Event/Risk Multiplier)".yellow().bold(),
            Self::FrictionHedge => "HEDGE (Microstructure/Friction Buffer)".green().bold(),
            Self::Auxiliary => "AUXILIARY".normal(),
        }
    }
}

/// Fusion architecture pattern used to combine multiple datasets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionPattern {
    /// Hierarchical Conditioning: Anchor * (1 + gamma * Catalyst) - Hedge, or trade_when triggers
    HierarchicalConditioning,
    /// Naive Linear Addition: Signal = Rank(A) + Rank(B) (dilutes tails, high crowd risk)
    NaiveLinearAddition,
    /// Single Dataset: No cross-dataset mixing detected
    SingleDataset,
}

impl FusionPattern {
    pub fn badge(&self) -> colored::ColoredString {
        match self {
            Self::HierarchicalConditioning => {
                "HIERARCHICAL CONDITIONING (Institutional Gold Standard)"
                    .green()
                    .bold()
            }
            Self::NaiveLinearAddition => "NAIVE LINEAR ADDITION (Crowd Tail Dilution Risk)"
                .red()
                .bold(),
            Self::SingleDataset => "SINGLE DATASET PURITY".yellow(),
        }
    }
}

/// Interaction assessment between two factor pillars
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarInteraction {
    pub pillar_a: String,
    pub pillar_b: String,
    pub density_pct: f64,
    pub velocity_a: i64,
    pub velocity_b: i64,
    pub hybrid_uniqueness_index: f64,
    pub risk_tier: String,
}

/// Summary report of the dynamic multi-dataset co-occurrence matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMatrixReport {
    pub universe: String,
    pub total_platform_alphas: i64,
    pub pillars: Vec<String>,
    pub pillar_stats: HashMap<String, PillarCensusStat>,
    pub interactions: Vec<PillarInteraction>,
    pub migration_alerts: Vec<String>,
}

/// Dynamic census statistics for a single factor pillar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarCensusStat {
    pub primary_dataset_id: String,
    pub live_users: i64,
    pub live_alphas: i64,
    pub delta_users: i64,
    pub delta_alphas: i64,
    pub coverage_pct: f64,
    pub platform_share_pct: f64,
    pub status: String,
}

/// Comprehensive analysis of a formula combining K >= 2 datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaHybridAnalysis {
    pub formula: String,
    pub universe: String,
    pub detected_k: usize,
    pub participating_pillars: Vec<(String, DatasetRole, PillarCensusStat)>,
    pub fusion_pattern: FusionPattern,
    pub composite_uniqueness_index: f64,
    pub uniqueness_verdict: String,
    pub dynamic_migration_warnings: Vec<String>,
    pub economic_recommendations: Vec<String>,
}

/// Identifies all participating factor pillars and datasets from a FASTEXPR formula (K >= 1)
pub fn detect_all_participating_pillars(expr: &str) -> Vec<(&'static str, FactorPillar)> {
    let lower = expr.to_lowercase();
    let mut detected = Vec::new();

    // 1. Idiosyncratic Risk (model51)
    if lower.contains("unsystematic_risk")
        || lower.contains("residual_vol")
        || lower.contains("residual_momentum")
        || lower.contains("idio_")
    {
        detected.push(("model51", FactorPillar::IdiosyncraticRisk));
    }

    // 2. Short Interest (short / news12 borrow metrics)
    if lower.contains("shares_sold_short")
        || lower.contains("shorted_shares")
        || lower.contains("short_")
        || lower.contains("borrow_")
    {
        detected.push(("short", FactorPillar::ShortInterest));
    }

    // 3. News Sentiment (news12 / news18 RavenPack)
    if lower.contains("nws12")
        || lower.contains("mean_composite_sentiment")
        || lower.contains("mean_earnings_evaluation")
        || lower.contains("mean_event_novelty")
        || lower.contains("mean_entity_relevance")
        || lower.contains("headline")
        || lower.contains("sentiment")
    {
        let ds = if lower.contains("mean_composite") || lower.contains("mean_earnings") {
            "news18"
        } else {
            "news12"
        };
        detected.push((ds, FactorPillar::NewsEvent));
    }

    // 4. Options Surface & VRP (option8 / option9)
    if lower.contains("implied_volatility")
        || lower.contains("historical_volatility")
        || lower.contains("pcr_vol")
        || lower.contains("pcr_oi")
    {
        detected.push(("option8", FactorPillar::OptionsVrp));
    }

    // 5. Analyst Consensus (analyst4)
    if lower.contains("actual_eps")
        || lower.contains("actual_adj_net")
        || lower.contains("anl4_")
        || lower.contains("fwd_eps")
        || lower.contains("consensus")
    {
        detected.push(("analyst4", FactorPillar::AnalystConsensus));
    }

    // 6. Fundamental Quality (fundamental6 / fundamental2)
    if lower.contains("fnd6_")
        || lower.contains("operating_income")
        || lower.contains("cashflow_op")
        || lower.contains("actual_cashflow")
        || lower.contains("ebitda")
        || lower.contains("liabilities")
        || lower.contains("assets")
        || lower.contains("cogs")
    {
        detected.push(("fundamental6", FactorPillar::FinancialHealthQuality));
    }

    // 7. Microstructure & Price-Volume (pv1)
    if lower.contains("vwap")
        || lower.contains("volume")
        || lower.contains("ts_delta(close")
        || lower.contains("night_ret")
        || lower.contains("day_ret")
        || lower.contains("open")
        || lower.contains("close")
    {
        detected.push(("pv1", FactorPillar::MicrostructureVwap));
    }

    // Deduplicate in case multiple conditions match the same pillar
    let mut seen = std::collections::HashSet::new();
    detected.retain(|(_, p)| seen.insert(p.as_str()));

    detected
}

/// Classifies economic role of a pillar inside an expression
pub fn classify_pillar_role(
    pillar: FactorPillar,
    expr: &str,
    total_detected: usize,
) -> DatasetRole {
    if total_detected <= 1 {
        return DatasetRole::EconomicAnchor;
    }

    let lower = expr.to_lowercase();

    match pillar {
        FactorPillar::IdiosyncraticRisk
        | FactorPillar::FinancialHealthQuality
        | FactorPillar::AnalystConsensus => DatasetRole::EconomicAnchor,

        FactorPillar::ShortInterest | FactorPillar::NewsEvent | FactorPillar::OptionsVrp => {
            if lower.contains("trade_when")
                || lower.contains("* (1 +")
                || lower.contains("* (0.")
                || lower.contains("* (1.0 +")
                || lower.contains("* rank")
                || lower.contains("* catalyst")
            {
                DatasetRole::RiskCatalyst
            } else {
                DatasetRole::EconomicAnchor
            }
        }

        FactorPillar::MicrostructureVwap => {
            if lower.contains("ts_delta")
                || lower.contains("reversal")
                || lower.contains("vwap / close")
                || lower.contains("friction")
                || lower.contains("hedge")
            {
                DatasetRole::FrictionHedge
            } else {
                DatasetRole::Auxiliary
            }
        }

        _ => DatasetRole::Auxiliary,
    }
}

/// Detects whether an expression uses Hierarchical Conditioning or Naive Linear Addition
pub fn detect_fusion_pattern(expr: &str, num_datasets: usize) -> FusionPattern {
    if num_datasets <= 1 {
        return FusionPattern::SingleDataset;
    }

    let lower = expr.to_lowercase();

    // Check for hierarchical conditioning markers:
    // 1. Non-linear threshold gating: trade_when(...)
    // 2. Multiplicative event scaling: * (1 + gamma * ...)
    // 3. Friction subtraction: - reversal, - ts_delta, - hedge
    let has_trade_when = lower.contains("trade_when");
    let has_multiplicative_scale = lower.contains("* (1 +")
        || lower.contains("* (0.")
        || lower.contains("* (1.0 +")
        || lower.contains("* (0.8 +");
    let has_friction_hedge = lower.contains("- reversal")
        || lower.contains("- ts_delta")
        || lower.contains("- friction")
        || lower.contains("- hedge")
        || lower.contains("- 0.");

    if has_trade_when || has_multiplicative_scale || has_friction_hedge {
        FusionPattern::HierarchicalConditioning
    } else {
        FusionPattern::NaiveLinearAddition
    }
}

/// Builds dynamic 2D co-occurrence matrix from live BRAIN dataset census and previous snapshot
pub fn build_dynamic_matrix(
    datasets: &[DatasetEntry],
    prev_snapshot: Option<&CensusSnapshot>,
    universe: &str,
) -> DynamicMatrixReport {
    let u_clean = universe.to_uppercase();

    // Map previous snapshot for delta migration tracking
    let mut prev_map: HashMap<String, (i64, i64)> = HashMap::new();
    if let Some(prev) = prev_snapshot {
        for d in &prev.datasets {
            let u_str = d.universe.as_deref().unwrap_or("").to_uppercase();
            let key = format!("{}:{}", d.id.to_lowercase(), u_str);
            prev_map.insert(key, (d.user_count.unwrap_or(0), d.alpha_count.unwrap_or(0)));
        }
    }

    // 7 Pillars
    let pillar_defs = [
        ("MICRO", "pv1", "Price-Volume / Microstructure VWAP"),
        ("FND", "fundamental6", "Fundamental Health & Quality"),
        ("ANL", "analyst4", "Analyst Estimates Consensus"),
        ("OPT", "option8", "Options Implied Volatility & VRP"),
        ("NEWS", "news18", "News Event Sentiment & RavenPack"),
        ("RISK", "model51", "Idiosyncratic / Unsystematic Risk"),
        ("SHORT", "short", "Short Interest & Borrow Dynamics"),
    ];

    let mut pillar_stats = HashMap::new();
    let mut total_platform_alphas = 0i64;
    let mut migration_alerts = Vec::new();

    for (p_key, ds_id, _desc) in &pillar_defs {
        // Find best match in datasets for this universe
        let mut matched: Option<&DatasetEntry> = None;
        for d in datasets {
            if d.id.eq_ignore_ascii_case(ds_id) {
                if let Some(ref du) = d.universe {
                    if du.to_uppercase() == u_clean {
                        matched = Some(d);
                        break;
                    }
                }
            }
        }

        // Fallback: match dataset ID only
        if matched.is_none() {
            for d in datasets {
                if d.id.eq_ignore_ascii_case(ds_id) {
                    matched = Some(d);
                    break;
                }
            }
        }

        let (live_u, live_a, cov) = match matched {
            Some(entry) => (
                entry.user_count.unwrap_or(150),
                entry.alpha_count.unwrap_or(300),
                entry.coverage.unwrap_or(0.95),
            ),
            None => match (*ds_id, u_clean.as_str()) {
                ("pv1", "TOP3000") => (92205, 2165639, 1.00),
                ("pv1", "TOP1000") => (22769, 90031, 1.00),
                ("pv1", "TOP200") => (205, 367, 1.00),
                ("fundamental6", "TOP3000") => (88698, 850369, 0.50),
                ("fundamental6", "TOP200") => (98, 210, 0.50),
                ("analyst4", "TOP3000") => (49471, 760206, 0.726),
                ("analyst4", "TOP200") => (10, 11, 0.666),
                ("option8", "TOP3000") => (34103, 190405, 0.97),
                ("option8", "TOP200") => (39, 46, 0.97),
                ("news18", "TOP200") => (193, 637, 0.975),
                ("model51", "TOP200") => (378, 796, 0.963),
                ("model51", "TOP3000") => (10016, 43367, 0.963),
                ("short", "TOP200") => (150, 310, 0.95),
                _ => (150, 300, 0.90),
            },
        };

        total_platform_alphas += live_a;

        // Velocity tracking
        let prev_key = format!("{}:{}", ds_id.to_lowercase(), u_clean);
        let (prev_u, prev_a) = prev_map.get(&prev_key).copied().unwrap_or((live_u, live_a));
        let delta_u = live_u - prev_u;
        let delta_a = live_a - prev_a;

        if delta_u >= 50 {
            migration_alerts.push(format!(
                "⚠️ CROWD INFLUX ALERT: Pillar '{}' on {} saw +{} quants recently! Crowding velocity accelerating.",
                p_key, u_clean, delta_u
            ));
        }

        let status_str = if live_u <= 50 && delta_u <= 5 {
            "PRISTINE"
        } else if live_u <= 300 && delta_u <= 20 {
            "SAFE"
        } else if live_u <= 2000 {
            "MODERATE"
        } else if live_u <= 15000 {
            "CROWDED"
        } else {
            "DANGER"
        };

        pillar_stats.insert(
            p_key.to_string(),
            PillarCensusStat {
                primary_dataset_id: ds_id.to_string(),
                live_users: live_u,
                live_alphas: live_a,
                delta_users: delta_u,
                delta_alphas: delta_a,
                coverage_pct: cov * 100.0,
                platform_share_pct: 0.0, // calculated below
                status: status_str.to_string(),
            },
        );
    }

    // Normalize platform share
    if total_platform_alphas > 0 {
        for stat in pillar_stats.values_mut() {
            stat.platform_share_pct =
                (stat.live_alphas as f64 / total_platform_alphas as f64) * 100.0;
        }
    }

    // Build pairwise co-occurrence matrix (7 x 7)
    let p_names: Vec<String> = pillar_defs.iter().map(|(p, _, _)| p.to_string()).collect();
    let mut interactions = Vec::new();

    for i in 0..p_names.len() {
        for j in (i + 1)..p_names.len() {
            let p_a = &p_names[i];
            let p_b = &p_names[j];

            let stat_a = pillar_stats.get(p_a).unwrap();
            let stat_b = pillar_stats.get(p_b).unwrap();

            // Empirical affinity multiplier between factors:
            // Fundamental x Micro has massive natural affinity (crowd template 1)
            // IdioRisk x Short has near-zero historical affinity
            let affinity = match (p_a.as_str(), p_b.as_str()) {
                ("MICRO", "FND") | ("FND", "MICRO") => 2.2,
                ("MICRO", "ANL") | ("ANL", "MICRO") => 1.8,
                ("FND", "ANL") | ("ANL", "FND") => 1.5,
                ("MICRO", "OPT") | ("OPT", "MICRO") => 1.1,
                ("MICRO", "NEWS") | ("NEWS", "MICRO") => 1.0,
                ("RISK", "SHORT") | ("SHORT", "RISK") => 0.15,
                ("RISK", "NEWS") | ("NEWS", "RISK") => 0.20,
                ("SHORT", "NEWS") | ("NEWS", "SHORT") => 0.25,
                ("RISK", "ANL") | ("ANL", "RISK") => 0.30,
                _ => 0.60,
            };

            // Joint probability estimation (% of alphas mixing A and B)
            let base_joint =
                (stat_a.platform_share_pct * stat_b.platform_share_pct / 100.0) * affinity;
            let density_pct = base_joint.clamp(0.005, 50.0);

            // Dynamic Hybrid Uniqueness Index: penalized by joint density and positive migration velocity
            let vel_penalty =
                ((stat_a.delta_users.max(0) + stat_b.delta_users.max(0)) as f64 * 0.001).min(0.20);
            let hui = (1.0 - (density_pct / 15.0).min(0.95) - vel_penalty).clamp(0.05, 0.99);

            let risk_tier = if hui >= 0.85 {
                "🟢 PRISTINE HYBRID"
            } else if hui >= 0.65 {
                "🟡 SAFE CORRIDOR"
            } else if hui >= 0.40 {
                "🟠 CONTESTED"
            } else {
                "🔴 CROWD DEATH ZONE"
            };

            interactions.push(PillarInteraction {
                pillar_a: p_a.clone(),
                pillar_b: p_b.clone(),
                density_pct,
                velocity_a: stat_a.delta_users,
                velocity_b: stat_b.delta_users,
                hybrid_uniqueness_index: hui,
                risk_tier: risk_tier.to_string(),
            });
        }
    }

    DynamicMatrixReport {
        universe: u_clean,
        total_platform_alphas,
        pillars: p_names,
        pillar_stats,
        interactions,
        migration_alerts,
    }
}

/// Evaluates any FASTEXPR alpha formula containing K >= 2 datasets
pub fn analyze_formula_hyper_synergy(
    expr: &str,
    universe: &str,
    datasets: &[DatasetEntry],
    prev_snapshot: Option<&CensusSnapshot>,
) -> FormulaHybridAnalysis {
    let matrix_report = build_dynamic_matrix(datasets, prev_snapshot, universe);
    let detected_raw = detect_all_participating_pillars(expr);
    let detected_k = detected_raw.len();

    let mut participating_pillars = Vec::new();
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();

    for (_ds_id, pillar) in &detected_raw {
        let p_str = match pillar {
            FactorPillar::IdiosyncraticRisk => "RISK",
            FactorPillar::ShortInterest => "SHORT",
            FactorPillar::NewsEvent => "NEWS",
            FactorPillar::OptionsVrp => "OPT",
            FactorPillar::AnalystConsensus => "ANL",
            FactorPillar::FinancialHealthQuality => "FND",
            FactorPillar::MicrostructureVwap => "MICRO",
            _ => "OTHER",
        };

        let role = classify_pillar_role(*pillar, expr, detected_k);
        let stat = matrix_report
            .pillar_stats
            .get(p_str)
            .cloned()
            .unwrap_or(PillarCensusStat {
                primary_dataset_id: "unknown".to_string(),
                live_users: 100,
                live_alphas: 200,
                delta_users: 0,
                delta_alphas: 0,
                coverage_pct: 95.0,
                platform_share_pct: 1.0,
                status: "SAFE".to_string(),
            });

        if stat.delta_users > 20 {
            warnings.push(format!(
                "⚠️ DYNAMIC MIGRATION: Pillar '{}' has incoming crowd velocity (+{} quants).",
                p_str, stat.delta_users
            ));
        }

        participating_pillars.push((p_str.to_string(), role, stat));
    }

    let fusion_pattern = detect_fusion_pattern(expr, detected_k);

    if fusion_pattern == FusionPattern::NaiveLinearAddition && detected_k >= 2 {
        warnings.push("🔴 NAIVE LINEAR ADDITION DETECTED: Combining signals via linear '+' dilutes tail distribution and boosts internal turnover.".to_string());
        recommendations.push("Refactor to Hierarchical Conditioning: Anchor * (1 + gamma * Catalyst) - FrictionHedge, or apply trade_when() gating.".to_string());
    }

    // Calculate Composite Hybrid Uniqueness Index (CHUI)
    let chui = if detected_k <= 1 {
        0.50
    } else {
        // Average HUI across all participating pairs
        let mut sum_hui = 0.0;
        let mut count = 0;
        for i in 0..participating_pillars.len() {
            for j in (i + 1)..participating_pillars.len() {
                let p_a = &participating_pillars[i].0;
                let p_b = &participating_pillars[j].0;

                for inter in &matrix_report.interactions {
                    if (inter.pillar_a == *p_a && inter.pillar_b == *p_b)
                        || (inter.pillar_a == *p_b && inter.pillar_b == *p_a)
                    {
                        sum_hui += inter.hybrid_uniqueness_index;
                        count += 1;
                        break;
                    }
                }
            }
        }

        let base_chui = if count > 0 {
            sum_hui / count as f64
        } else {
            0.70
        };

        // Bonus for hierarchical conditioning; penalty for naive addition
        match fusion_pattern {
            FusionPattern::HierarchicalConditioning => (base_chui + 0.08).min(0.99),
            FusionPattern::NaiveLinearAddition => (base_chui - 0.15).max(0.10),
            FusionPattern::SingleDataset => base_chui,
        }
    };

    let verdict = if chui >= 0.85 {
        "🌟 SPECTACULAR ORTHOGONAL HYBRID (Maximum Uniqueness Score Suppression)".to_string()
    } else if chui >= 0.70 {
        "🟢 SAFE INSTITUTIONAL HYBRID (High Diversification / Low Crowd Overlap)".to_string()
    } else if chui >= 0.50 {
        "🟡 MODERATE HYBRID (Acceptable if Subindustry Neutralized)".to_string()
    } else {
        "🔴 CROWD TRAP HYBRID (High platform PnL correlation; risks raising uniquenessScore)"
            .to_string()
    };

    FormulaHybridAnalysis {
        formula: expr.to_string(),
        universe: universe.to_string(),
        detected_k,
        participating_pillars,
        fusion_pattern,
        composite_uniqueness_index: chui,
        uniqueness_verdict: verdict,
        dynamic_migration_warnings: warnings,
        economic_recommendations: recommendations,
    }
}

/// Formats and displays the dynamic co-occurrence heatmap matrix in the terminal
pub fn print_dynamic_matrix(report: &DynamicMatrixReport, green_only: bool) {
    println!(
        "\n  {} Target Universe: {} | Platform Tracked Alphas: {}",
        "📊".bold(),
        report.universe.yellow().bold(),
        report.total_platform_alphas.to_string().cyan()
    );

    if !report.migration_alerts.is_empty() {
        println!(
            "\n  {} Dynamic Migration Velocity Alerts:",
            "🚨".red().bold()
        );
        for alert in &report.migration_alerts {
            println!("    {}", alert.yellow());
        }
    }

    println!(
        "\n  {} Factor Pillars Dynamic Census Baseline:",
        "🏛️".bold()
    );
    println!("  ┌────────┬──────────────────────┬──────────────┬──────────────┬──────────────┬──────────┬────────────┐");
    println!("  │ Pillar │ Primary Dataset      │ Users (Live) │ Alphas (Live)│ Δ Users (Vel)│ Coverage │ Status     │");
    println!("  ├────────┼──────────────────────┼──────────────┼──────────────┼──────────────┼──────────┼────────────┤");

    for p in &report.pillars {
        if let Some(stat) = report.pillar_stats.get(p) {
            let vel_styled = if stat.delta_users > 0 {
                format!("+{:>4} ⚠️", stat.delta_users).red()
            } else if stat.delta_users < 0 {
                format!("{:>5} 📉", stat.delta_users).green()
            } else {
                format!("{:>5}   ", stat.delta_users).normal()
            };

            let status_styled = match stat.status.as_str() {
                "PRISTINE" => "PRISTINE".green().bold(),
                "SAFE" => "SAFE".green(),
                "MODERATE" => "MODERATE".yellow(),
                "CROWDED" => "CROWDED".red(),
                _ => "DANGER".red().bold(),
            };

            println!(
                "  │ {:<6} │ {:<20} │ {:>12} │ {:>12} │ {} │ {:>7.1}% │ {:<10} │",
                p.cyan().bold(),
                stat.primary_dataset_id,
                stat.live_users,
                stat.live_alphas,
                vel_styled,
                stat.coverage_pct,
                status_styled
            );
        }
    }
    println!("  └────────┴──────────────────────┴──────────────┴──────────────┴──────────────┴──────────┴────────────┘");

    println!(
        "\n  {} 2D Dynamic Cross-Dataset Co-occurrence Interaction Heatmap:",
        "🗺️".bold()
    );
    println!("  ┌──────────────┬──────────────┬──────────────┬────────────┬──────────┬─────────────────────────────┐");
    println!("  │ Factor A     │ Factor B     │ Est. Density │ HUI Score  │ Δ Vel A  │ Risk Tier Status            │");
    println!("  ├──────────────┼──────────────┼──────────────┼────────────┼──────────┼─────────────────────────────┤");

    for inter in &report.interactions {
        if green_only && inter.hybrid_uniqueness_index < 0.65 {
            continue;
        }

        let hui_styled = if inter.hybrid_uniqueness_index >= 0.85 {
            format!("{:.2}", inter.hybrid_uniqueness_index)
                .green()
                .bold()
        } else if inter.hybrid_uniqueness_index >= 0.65 {
            format!("{:.2}", inter.hybrid_uniqueness_index).green()
        } else if inter.hybrid_uniqueness_index >= 0.40 {
            format!("{:.2}", inter.hybrid_uniqueness_index).yellow()
        } else {
            format!("{:.2}", inter.hybrid_uniqueness_index).red().bold()
        };

        println!(
            "  │ {:<12} │ {:<12} │ {:>10.3}% │ {:>10} │ {:>+4} / {:>+4} │ {:<27} │",
            inter.pillar_a.cyan(),
            inter.pillar_b.cyan(),
            inter.density_pct,
            hui_styled,
            inter.velocity_a,
            inter.velocity_b,
            inter.risk_tier
        );
    }
    println!("  └──────────────┴──────────────┴──────────────┴────────────┴──────────┴─────────────────────────────┘");
}
