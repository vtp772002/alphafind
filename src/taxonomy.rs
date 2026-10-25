use crate::models::CandidateAlpha;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The Canonical Economic Factor Pillars of WorldQuant BRAIN,
/// including Alternative Data expansions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactorPillar {
    // --- Legacy / Core Platform Pillars ---
    AnalystConsensus,
    OptionsVrp,
    MicrostructureVwap,
    FinancialHealthQuality,
    IdiosyncraticRisk,
    ShortInterest,
    NewsEvent,
    CrossSanctuary,

    // --- Alternative Data Pillars (Milestone M1) ---
    SystematicRisk, // model51: Multi-horizon beta, benchmark correlation, systematic variance
    FundamentalScores, // model16: Fundamental rank derivatives, acceleration, pentagon scores
    NewsSentiment,  // news18: RavenPack news sentiment, corporate actions, event streams
    OptionsAnalytics, // option9: Breakeven asymmetry, synthetic forward parity, term skew
}

impl FactorPillar {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnalystConsensus => "ANALYST_CONSENSUS",
            Self::OptionsVrp => "OPTIONS_VRP",
            Self::MicrostructureVwap => "MICROSTRUCTURE_VWAP",
            Self::FinancialHealthQuality => "FINANCIAL_HEALTH_QUALITY",
            Self::IdiosyncraticRisk => "IDIOSYNCRATIC_RISK",
            Self::ShortInterest => "SHORT_INTEREST",
            Self::NewsEvent => "NEWS_EVENT",
            Self::CrossSanctuary => "CROSS_SANCTUARY",
            // Alternative Data
            Self::SystematicRisk => "SYSTEMATIC_RISK",
            Self::FundamentalScores => "FUNDAMENTAL_SCORES",
            Self::NewsSentiment => "NEWS_SENTIMENT",
            Self::OptionsAnalytics => "OPTIONS_ANALYTICS",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        let upper = s.to_uppercase();

        // 1. Alternative Data Mappings (checked first for precision)
        if upper.contains("SYSTEMATIC") || upper.contains("SYS_RISK") || upper.contains("MODEL51") {
            Some(Self::SystematicRisk)
        } else if upper.contains("FUNDAMENTAL_SCORE")
            || upper.contains("DERIVATIVE_ACCEL")
            || upper.contains("MODEL16")
            || upper.contains("FSCORE")
        {
            Some(Self::FundamentalScores)
        } else if upper.contains("NEWS_SENTIMENT")
            || upper.contains("RAVENPACK")
            || upper.contains("NEWS18")
        {
            Some(Self::NewsSentiment)
        } else if upper.contains("OPTIONS_ANALYTIC")
            || upper.contains("OPTION9")
            || upper.contains("BREAKEVEN")
        {
            Some(Self::OptionsAnalytics)
        }
        // 2. Legacy / Core Platform Mappings
        else if upper.contains("ANALYST") || upper.contains("CAPITAL_ALLOCATION") {
            Some(Self::AnalystConsensus)
        } else if upper.contains("OPT") || upper.contains("VRP") {
            Some(Self::OptionsVrp)
        } else if upper.contains("MICRO") || upper.contains("VWAP") {
            Some(Self::MicrostructureVwap)
        } else if upper.contains("QUAL") || upper.contains("FUND") || upper.contains("HEALTH") {
            Some(Self::FinancialHealthQuality)
        } else if upper.contains("RISK") || upper.contains("IDIO") {
            Some(Self::IdiosyncraticRisk)
        } else if upper.contains("CROSS") || upper.contains("SANCTUARY") || upper.contains("HYBRID")
        {
            Some(Self::CrossSanctuary)
        } else if upper.contains("SHORT") || upper.contains("SQUEEZE") {
            Some(Self::ShortInterest)
        } else if upper.contains("NEWS") || upper.contains("SENTIMENT") {
            Some(Self::NewsEvent)
        } else {
            None
        }
    }

    /// Returns a slice of all 12 factor pillars.
    pub fn all() -> &'static [FactorPillar] {
        &[
            Self::AnalystConsensus,
            Self::OptionsVrp,
            Self::MicrostructureVwap,
            Self::FinancialHealthQuality,
            Self::IdiosyncraticRisk,
            Self::ShortInterest,
            Self::NewsEvent,
            Self::CrossSanctuary,
            Self::SystematicRisk,
            Self::FundamentalScores,
            Self::NewsSentiment,
            Self::OptionsAnalytics,
        ]
    }

    /// Short economic description of the pillar.
    pub fn description(&self) -> &'static str {
        match self {
            Self::AnalystConsensus => "Consensus earnings estimate revisions and target price drift",
            Self::OptionsVrp => "Volatility risk premium and implied volatility term structure",
            Self::MicrostructureVwap => "Intraday volume-weighted average price and liquidity dynamics",
            Self::FinancialHealthQuality => "Balance sheet resilience, profitability, and capital efficiency",
            Self::IdiosyncraticRisk => "Residual non-market risk dispersion and lottery preferences",
            Self::ShortInterest => "Short-seller crowding, borrowing fees, and short squeeze dynamics",
            Self::NewsEvent => "Media headline sentiment, article velocity, and event reactions",
            Self::CrossSanctuary => "Multi-pillar hybrid combinations fusing anchors and catalysts",
            Self::SystematicRisk => "Multi-horizon rolling beta, SPY correlation, and systematic variance (model51)",
            Self::FundamentalScores => "Rank derivatives, multi-factor acceleration convexity, and cashflow quality (model16)",
            Self::NewsSentiment => "RavenPack NLP sentiment divergence, corporate actions, and PEAD shocks (news18)",
            Self::OptionsAnalytics => "Breakeven directional asymmetry, synthetic forward price parity, and term skew (option9)",
        }
    }

    /// Primary BRAIN dataset ID associated with this pillar.
    pub fn primary_dataset_id(&self) -> &'static str {
        match self {
            Self::SystematicRisk | Self::IdiosyncraticRisk => "model51",
            Self::FundamentalScores => "model16",
            Self::NewsSentiment | Self::NewsEvent => "news18",
            Self::OptionsAnalytics => "option9",
            Self::OptionsVrp => "option8",
            Self::AnalystConsensus => "analyst4",
            Self::FinancialHealthQuality => "fundamental6",
            Self::ShortInterest => "short",
            Self::MicrostructureVwap => "pv1",
            Self::CrossSanctuary => "hybrid",
        }
    }

    /// Canonical legacy fallback pillar for backward compatibility.
    pub fn canonical_legacy_pillar(&self) -> Self {
        match self {
            Self::SystematicRisk => Self::IdiosyncraticRisk,
            Self::FundamentalScores => Self::FinancialHealthQuality,
            Self::NewsSentiment => Self::NewsEvent,
            Self::OptionsAnalytics => Self::OptionsVrp,
            other => *other,
        }
    }

    /// Whether this pillar is part of the Alternative Data research expansion.
    pub fn is_alternative_data(&self) -> bool {
        matches!(
            self,
            Self::SystematicRisk
                | Self::FundamentalScores
                | Self::NewsSentiment
                | Self::OptionsAnalytics
                | Self::IdiosyncraticRisk
                | Self::OptionsVrp
                | Self::NewsEvent
        )
    }
}

/// Formal taxonomy of abstract placeholders permitted in public / tracked code.
pub struct PlaceholderRegistry;

impl PlaceholderRegistry {
    pub const FIELD_PLACEHOLDERS: &'static [&'static str] = &[
        "<FIELD>",
        "<CORE_FIELD>",
        "<BETA_FIELD>",
        "<BETA_SHORT>",
        "<BETA_LONG>",
        "<CORR_FIELD>",
        "<IDIO_RISK_SHORT>",
        "<IDIO_RISK_MED>",
        "<IDIO_RISK_LONG>",
        "<IDIO_RISK_30D>",
        "<IDIO_RISK_60D>",
        "<IDIO_RISK_90D>",
        "<FUND_ACCEL_FIELD>",
        "<CASHFLOW_EFF_FIELD>",
        "<ANALYST_REV_FIELD>",
        "<EARNINGS_CERTAINTY_FIELD>",
        "<RELATIVE_VAL_FIELD>",
        "<COMPOSITE_SCORE_DERIVATIVE>",
        "<NEWS_SENTIMENT_VECTOR>",
        "<NEWS_SENTIMENT_FIELD>",
        "<NOVELTY_SCORE_VECTOR>",
        "<RELEVANCE_SCORE_VECTOR>",
        "<EARNINGS_EVAL_VECTOR>",
        "<CORP_ACTION_VECTOR>",
        "<CALL_BREAKEVEN_FIELD>",
        "<PUT_BREAKEVEN_FIELD>",
        "<PCR_VOL_FIELD>",
        "<PCR_OI_FIELD>",
        "<PCR_TERM_SHORT>",
        "<PCR_TERM_LONG>",
        "<FORWARD_PRICE_FIELD>",
    ];

    pub const SIGNAL_PLACEHOLDERS: &'static [&'static str] = &[
        "<SIGNAL>",
        "<CORE_SIGNAL>",
        "<CATALYST_SIGNAL>",
        "<REVERSAL_DAMPENER>",
        "<VOLATILITY_NORM>",
        "<VOLUME_SHOCK_FACTOR>",
    ];

    pub const GROUP_PLACEHOLDERS: &'static [&'static str] = &["<GROUP>", "<NEUTRALIZATION_GROUP>"];

    /// Scans an expression string and extracts all unique `<PLACEHOLDER>` tokens.
    /// Only tokens conforming to `<[A-Z0-9_]+>` are extracted.
    pub fn extract_placeholders(expr: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let bytes = expr.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'<' {
                if let Some(j) = bytes[i + 1..].iter().position(|&b| b == b'>') {
                    let end = i + 1 + j;
                    let inner = &expr[i + 1..end];
                    if !inner.is_empty()
                        && inner
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                    {
                        let token = expr[i..=end].to_string();
                        if !tokens.contains(&token) {
                            tokens.push(token);
                        }
                    }
                    i = end + 1;
                    continue;
                }
            }
            i += 1;
        }
        tokens
    }

    /// Returns true if the expression contains at least one unresolved `<...>` placeholder.
    pub fn has_unresolved_placeholders(expr: &str) -> bool {
        !Self::extract_placeholders(expr).is_empty()
    }

    /// Substitutes placeholder tokens with concrete values from `bindings`.
    /// Returns Ok(rendered_expr) or Err(missing_placeholders) if any tokens remain unbound.
    pub fn render_template(
        template: &str,
        bindings: &HashMap<&str, &str>,
    ) -> Result<String, Vec<String>> {
        let mut rendered = template.to_string();
        for (placeholder, replacement) in bindings {
            rendered = rendered.replace(placeholder, replacement);
        }

        let remaining = Self::extract_placeholders(&rendered);
        if remaining.is_empty() {
            Ok(rendered)
        } else {
            Err(remaining)
        }
    }

    /// Validates basic FastExpr structural syntax before API dispatch:
    /// 1. Balanced parentheses `()`
    /// 2. No unresolved `<...>` placeholders
    /// 3. Proper densify() wrapper for group_neutralize
    pub fn validate_fastexpr_syntax(expr: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 1. Unresolved placeholders check
        let placeholders = Self::extract_placeholders(expr);
        if !placeholders.is_empty() {
            errors.push(format!(
                "Expression contains unresolved abstract placeholders: {:?}",
                placeholders
            ));
        }

        // 2. Balanced parentheses check
        let mut depth = 0i32;
        for (idx, ch) in expr.chars().enumerate() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth < 0 {
                    errors.push(format!(
                        "Unmatched closing parenthesis ')' at char index {}",
                        idx
                    ));
                    break;
                }
            }
        }
        if depth > 0 {
            errors.push(format!("Unclosed open parentheses: {} unclosed '('", depth));
        }

        // 3. Group neutralization densify check
        if expr.contains("group_neutralize") {
            let lower = expr.to_lowercase();
            if !lower.contains("densify(") {
                errors.push(
                    "group_neutralize must wrap grouping variable in densify(...)".to_string(),
                );
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Attempts to load private candidate definitions from a local permission-restricted JSON store.
/// Per AGENTS.md, concrete candidate formulas are excluded from Git-tracked source code.
fn load_private_candidates() -> Option<Vec<CandidateAlpha>> {
    let default_path = if std::path::Path::new("/tmp/alphafind_candidates.json").exists() {
        "/tmp/alphafind_candidates.json".to_string()
    } else {
        "data/private_candidates.json".to_string()
    };
    let path = std::env::var("ALPHAFIND_CANDIDATES_PATH").unwrap_or(default_path);
    if std::path::Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                let candidates: Vec<CandidateAlpha> = values
                    .into_iter()
                    .filter_map(|v| serde_json::from_value::<CandidateAlpha>(v).ok())
                    .map(|mut c| {
                        // Canonicalize pillar name to canonical form if recognized
                        if let Some(canonical) = FactorPillar::from_str(&c.pillar) {
                            c.pillar = canonical.as_str().to_string();
                        }
                        c
                    })
                    .collect();
                if !candidates.is_empty() {
                    return Some(candidates);
                }
            }
        }
    }
    None
}

/// Generates a default multi-pillar curated batch for broad universe screening.
fn get_default_curated_batch(u: &str) -> Vec<CandidateAlpha> {
    vec![
        CandidateAlpha {
            name: format!("US_D1_{}_SysIdio_TermSpread_Dec16", u),
            expression: "idio_spread = ts_backfill(<IDIO_RISK_SHORT>, 10) / (ts_backfill(<IDIO_RISK_LONG>, 15) + 0.001); beta_spread = ts_backfill(<BETA_SHORT>, 10) - ts_backfill(<BETA_LONG>, 15); raw_sig = 0.65 * signed_power(0.5 - rank(idio_spread), 1.4) - 0.35 * signed_power(rank(beta_spread) - 0.5, 1.2); group_neutralize(signed_power(raw_sig, 3.5), densify(subindustry))".to_string(),
            universe: u.to_string(),
            decay: 16,
            neutralization: "SUBINDUSTRY".to_string(),
            truncation: 0.065,
            pillar: "SYSTEMATIC_RISK".to_string(),
        },
        CandidateAlpha {
            name: format!("US_D1_{}_FundAccel_PriceRev_Dec20", u),
            expression: "accel = rank(<FUND_ACCEL_FIELD>) - 0.5; cashflow = rank(<CASHFLOW_EFF_FIELD>) - 0.5; rev = rank(<ANALYST_REV_FIELD>) - 0.5; core_fund = 0.50 * accel + 0.30 * cashflow + 0.20 * rev; price_mom = 0.10 * (rank(ts_delta(close, 5)) - 0.5); sig = signed_power(core_fund, 1.5) - price_mom; group_neutralize(signed_power(sig, 3.6), densify(subindustry))".to_string(),
            universe: u.to_string(),
            decay: 20,
            neutralization: "SUBINDUSTRY".to_string(),
            truncation: 0.07,
            pillar: "FUNDAMENTAL_SCORES".to_string(),
        },
        CandidateAlpha {
            name: format!("US_D1_{}_Options_BreakevenAsym_Dec18", u),
            expression: "call_be = ts_backfill(<CALL_BREAKEVEN_FIELD> / close - 1, 10); put_be = ts_backfill(1 - <PUT_BREAKEVEN_FIELD> / close, 10); be_asym = rank(put_be - call_be) - 0.5; rev = 0.06 * (rank(ts_delta(close, 3)) - 0.5); sig = signed_power(be_asym, 1.4) - rev; group_neutralize(signed_power(sig, 4.0), densify(subindustry))".to_string(),
            universe: u.to_string(),
            decay: 18,
            neutralization: "SUBINDUSTRY".to_string(),
            truncation: 0.065,
            pillar: "OPTIONS_ANALYTICS".to_string(),
        },
        CandidateAlpha {
            name: format!("US_D1_{}_News_EarningsShock_Dec22", u),
            expression: "earnings_sent = ts_backfill(vec_avg(<EARNINGS_EVAL_VECTOR>), 20); rel = ts_backfill(vec_avg(<RELEVANCE_SCORE_VECTOR>), 20); f_sent = if_else(is_nan(earnings_sent), 0, rank(earnings_sent) - 0.5); f_rel = if_else(is_nan(rel), 0.5, rank(rel)); sig = signed_power(f_sent, 1.5) * (0.4 + 0.6 * f_rel); group_neutralize(signed_power(sig, 4.2), densify(subindustry))".to_string(),
            universe: u.to_string(),
            decay: 22,
            neutralization: "SUBINDUSTRY".to_string(),
            truncation: 0.07,
            pillar: "NEWS_SENTIMENT".to_string(),
        },
        CandidateAlpha {
            name: "US_D1_TOP500_Analyst_EPS_Delta5_Dec32".to_string(),
            expression: "group_neutralize(signed_power(rank(ts_backfill(<ANALYST_REVISION_FIELD>, 20) / close) - 0.5, 3.2) - 0.6 * rank(ts_delta(close, 5)), densify(sector))".to_string(),
            universe: "TOP500".to_string(),
            decay: 32,
            neutralization: "SECTOR".to_string(),
            truncation: 0.07,
            pillar: "ANALYST_CONSENSUS".to_string(),
        },
        CandidateAlpha {
            name: format!("US_D1_{}_IdioCurv_30_60_90_Dec14", u),
            expression: "group_neutralize(signed_power(rank(ts_backfill(<IDIO_RISK_SHORT>, 10) - 2.0 * ts_backfill(<IDIO_RISK_MED>, 10) + ts_backfill(<IDIO_RISK_LONG>, 10)) - 0.5, 2.5), densify(subindustry))".to_string(),
            universe: u.to_string(),
            decay: 14,
            neutralization: "SUBINDUSTRY".to_string(),
            truncation: 0.065,
            pillar: "IDIOSYNCRATIC_RISK".to_string(),
        },
        CandidateAlpha {
            name: format!("US_D1_{}_Micro_VWAP_Burst_Dec40", u),
            expression: "group_neutralize(signed_power(rank(vwap / close - 1) - 0.5, 3.8) * (1.0 + 0.5 * rank(volume / ts_mean(volume, 40))), densify(market))".to_string(),
            universe: u.to_string(),
            decay: 40,
            neutralization: "MARKET".to_string(),
            truncation: 0.065,
            pillar: "MICROSTRUCTURE_VWAP".to_string(),
        },
    ]
}

/// Generates curated candidate hypotheses for screening.
///
/// If a local private store (`data/private_candidates.json` or `ALPHAFIND_CANDIDATES_PATH`)
/// is present, candidates are loaded dynamically from that store.
/// Otherwise, canonical factor archetypes with abstract placeholders (`<FIELD>`, `<SIGNAL>`)
/// are returned per the AGENTS.md research hygiene policy.
pub fn get_curated_candidates(
    pillar: Option<FactorPillar>,
    universe: Option<&str>,
) -> Vec<CandidateAlpha> {
    let u = universe.unwrap_or("TOP1000");

    // 1. Attempt dynamic load from untracked private store
    if let Some(private_candidates) = load_private_candidates() {
        if let Some(p) = pillar {
            let filtered: Vec<CandidateAlpha> = private_candidates
                .into_iter()
                .filter(|c| {
                    if !c.pillar.eq_ignore_ascii_case(p.as_str())
                        && FactorPillar::from_str(&c.pillar) != Some(p)
                    {
                        return false;
                    }
                    if let Some(target_u) = universe {
                        if !target_u.eq_ignore_ascii_case("ALL")
                            && !c.universe.eq_ignore_ascii_case(target_u)
                        {
                            return false;
                        }
                    }
                    true
                })
                .collect();
            if !filtered.is_empty() {
                return filtered;
            }
            // If private store has no candidates for this pillar, fall through to curated presets for this pillar!
        } else {
            // When pillar is None (batch load), filter by universe
            let mut filtered: Vec<CandidateAlpha> = private_candidates
                .into_iter()
                .filter(|c| {
                    if let Some(target_u) = universe {
                        if !target_u.eq_ignore_ascii_case("ALL")
                            && !c.universe.eq_ignore_ascii_case(target_u)
                        {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            let distinct_pillars: std::collections::HashSet<&str> =
                filtered.iter().map(|c| c.pillar.as_str()).collect();

            // If private store has diverse coverage (>= 3 candidates across >= 2 pillars), return it
            if filtered.len() >= 3 && distinct_pillars.len() >= 2 {
                return filtered;
            }

            // Otherwise, supplement with curated default batch to guarantee multi-pillar diversity
            let default_curated = get_default_curated_batch(u);
            for c in default_curated {
                if !filtered.iter().any(|existing| existing.name == c.name) {
                    filtered.push(c);
                }
            }
            return filtered;
        }
    }

    // 2. Fall back to sanitized canonical factor skeletons with abstract placeholders
    match pillar {
        Some(FactorPillar::AnalystConsensus) => vec![
            CandidateAlpha {
                name: "US_D1_TOP500_Analyst_EPS_Delta5_Dec32".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(<ANALYST_REVISION_FIELD>, 20) / close) - 0.5, 3.2) - 0.6 * rank(ts_delta(close, 5)), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 32,
                neutralization: "SECTOR".to_string(),
                truncation: 0.07,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_Dual_Consensus_Yield_Dec40".to_string(),
                expression: "group_neutralize(signed_power(0.6 * rank(ts_backfill(<ANALYST_SURPRISE_FIELD>, 60) / close) + 0.4 * rank(ts_backfill(<CONSENSUS_NET_INCOME_FIELD>, 60) / cap) - 0.5, 4.0), densify(market))".to_string(),
                universe: "TOP500".to_string(),
                decay: 40,
                neutralization: "MARKET".to_string(),
                truncation: 0.065,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_PureAnalyst_Apex_Dec50".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_decay_linear(ts_backfill(<ANALYST_EPS_FIELD> / close, 126), 20)) - 0.5, 4.4), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 50,
                neutralization: "SECTOR".to_string(),
                truncation: 0.065,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
        ],
        Some(FactorPillar::IdiosyncraticRisk) => vec![
            CandidateAlpha {
                name: "US_D1_TOP1000_IdioCurv_30_60_90_Dec14".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(<IDIO_RISK_SHORT>, 10) - 2.0 * ts_backfill(<IDIO_RISK_MED>, 10) + ts_backfill(<IDIO_RISK_LONG>, 10)) - 0.5, 2.5), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 14,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_IdioVol_60d_Delta3_Dec16".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(<IDIO_RISK_MED>, 15)) - 0.5, 2.8) - 0.5 * rank(ts_delta(close, 3)), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 16,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_IdioRisk_Spread_Dec12".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(<IDIO_RISK_SHORT> / <IDIO_RISK_LONG>, 10)) - 0.5, 3.0), densify(subindustry))".to_string(),
                universe: "TOP500".to_string(),
                decay: 12,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
        ],
        Some(FactorPillar::MicrostructureVwap) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_NightDay_Spread_SubNeut_Dec22".to_string(),
                expression: "night_ret = winsorize(<OVERNIGHT_RETURN_GAP>); day_ret = winsorize(<INTRADAY_RETURN_DRIFT>); spread = ts_mean(night_ret, 10) - ts_mean(day_ret, 10); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_ATR_NightDay_Spread_Dec20".to_string(),
                expression: "vol_norm = <VOLATILITY_NORM>; night_gap = winsorize(<OVERNIGHT_RETURN_GAP> / vol_norm); day_drift = winsorize(<INTRADAY_RETURN_DRIFT> / vol_norm); spread = ts_mean(night_gap, 10) - ts_mean(day_drift, 10); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 20,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_Micro_VWAP_Burst_Dec38".to_string(),
                expression: "vwap_dev = signed_power(rank(<VWAP_PRICE_DISLOCATION>) - 0.5, 1.4); vol_burst = signed_power(rank(<VOLUME_SHOCK_FACTOR>) - 0.5, 1.3); reversal = signed_power(rank(<REVERSAL_DAMPENER>) - 0.5, 1.5); signal = vwap_dev * (1 + 0.70 * vol_burst) - 0.04 * reversal; group_neutralize(signed_power(signal, 3.8), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 38,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_NightDay_Spread_Dec22".to_string(),
                expression: "night_ret = winsorize(<OVERNIGHT_RETURN_GAP>); day_ret = winsorize(<INTRADAY_RETURN_DRIFT>); spread = ts_mean(night_ret, 10) - ts_mean(day_ret, 10); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_NightDay_VolWeight_Dec24".to_string(),
                expression: "night_ret = winsorize(<OVERNIGHT_RETURN_GAP>); day_ret = winsorize(<INTRADAY_RETURN_DRIFT>); spread = (ts_mean(night_ret, 10) - ts_mean(day_ret, 10)) * (1 + 0.5 * rank(<VOLUME_SHOCK_FACTOR>)); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.0), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 24,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_Micro_VWAP_Burst_Dec40".to_string(),
                expression: "vwap_dev = signed_power(rank(<VWAP_PRICE_DISLOCATION>) - 0.5, 1.4); vol_burst = signed_power(rank(<VOLUME_SHOCK_FACTOR>) - 0.5, 1.3); reversal = signed_power(rank(<REVERSAL_DAMPENER>) - 0.5, 1.5); signal = vwap_dev * (1 + 0.70 * vol_burst) - 0.04 * reversal; group_neutralize(signed_power(signal, 3.8), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 40,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
        ],
        Some(FactorPillar::NewsEvent) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_News_Event_FastRev_Dec20".to_string(),
                expression: "news = rank(ts_decay_linear(winsorize(ts_backfill(vec_avg(<NEWS_SENTIMENT_FIELD>), 30)), 20)); rev = signed_power(rank(ts_decay_linear(ts_delta(close, 5), 3)) - 0.5, 1.5); signal = signed_power(news - 0.5, 1.5) - 0.04 * rev; group_neutralize(signed_power(signal, 4.0), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 20,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_EVENT".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_News_VolSurge_Dec24".to_string(),
                expression: "news = ts_backfill(vec_avg(<NEWS_SENTIMENT_FIELD>), 20); f_news = if_else(is_nan(news), 0, rank(news) - 0.5); vol_ratio = rank(volume / ts_mean(volume, 30)) - 0.5; signal = signed_power(f_news, 1.5) * (1 + 0.65 * signed_power(vol_ratio, 1.2)); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 24,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_EVENT".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_News_Afterhours_Dec22".to_string(),
                expression: "news = rank(ts_decay_linear(winsorize(ts_backfill(vec_avg(<AFTERHOURS_NEWS_FIELD>), 25)), 15)); rev = signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); signal = signed_power(news - 0.5, 1.6) - 0.03 * rev; group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_EVENT".to_string(),
            },
        ],
        Some(FactorPillar::ShortInterest) => vec![
            CandidateAlpha {
                name: "US_D1_TOP1000_Short_Dual_SubNeut_Dec28".to_string(),
                expression: "short_cov = ts_backfill(vec_avg(<SHORT_COVER_FIELD>), 45); short_all = ts_backfill(vec_avg(<SHORT_INTEREST_ALL_FIELD>), 45); f_cov = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); f_all = if_else(is_nan(short_all), 0, rank(short_all) - 0.5); dual_short = 0.60 * f_cov + 0.40 * f_all; vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); reversal = signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); signal = 0.65 * signed_power(dual_short, 1.5) + 0.20 * vol_burst - 0.04 * reversal; group_neutralize(signed_power(signal, 4.0), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 28,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "SHORT_INTEREST".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_Short_Accel_SubNeut_Dec22".to_string(),
                expression: "short_val = ts_backfill(vec_avg(<SHORT_INTEREST_FIELD>), 60); short_factor = if_else(is_nan(short_val), 0, rank(short_val) - 0.5); short_delta = ts_delta(short_val, 20); short_accel = if_else(is_nan(short_delta), 0, rank(short_delta) - 0.5); vol_factor = rank(volume / ts_mean(volume, 40)) - 0.5; signal = 0.70 * signed_power(short_factor, 1.5) + 0.30 * signed_power(short_accel, 1.3) + 0.25 * signed_power(vol_factor, 1.2); delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); signal_clean = rank(signal - delta_damp) - 0.5; group_neutralize(signed_power(signal_clean, 3.8), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "SHORT_INTEREST".to_string(),
            },
        ],
        Some(FactorPillar::OptionsVrp) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_VRP120d_Hedge_Dec26".to_string(),
                expression: "vrp = rank(ts_backfill(<IMPLIED_VOL_CALL_120D> / <HISTORICAL_VOL_120D>, 20)) - rank(ts_backfill(<IMPLIED_VOL_PUT_120D> / <HISTORICAL_VOL_120D>, 20)); signal = signed_power(vrp, 1.3) - 0.04 * signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 26,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "OPTIONS_VRP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_VRP60d_Delta3_Dec18".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(<IMPLIED_VOL_CALL_60D> / <HISTORICAL_VOL_60D>, 15)) - rank(ts_backfill(<IMPLIED_VOL_PUT_60D> / <HISTORICAL_VOL_60D>, 15)) - 0.5, 3.8), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 18,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "OPTIONS_VRP".to_string(),
            },
        ],
        Some(FactorPillar::CrossSanctuary) => vec![
            CandidateAlpha {
                name: format!("US_D1_{}_News_Short_Hybrid_SubNeut_Dec28", u),
                expression: "news_sl = ts_backfill(vec_avg(<NEWS_SENTIMENT_FIELD>), 20); short_cov = ts_backfill(vec_avg(<SHORT_COVER_FIELD>), 40); f_news = if_else(is_nan(news_sl), 0, rank(news_sl) - 0.5); f_short = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); signal = 0.50 * signed_power(f_news, 1.5) + 0.40 * signed_power(f_short, 1.4) + 0.20 * vol_burst; delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); group_neutralize(signed_power(signal - delta_damp, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 28,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_News_DualShort_VWAP_Dec28", u),
                expression: "news_sl = ts_backfill(vec_avg(<NEWS_SENTIMENT_FIELD>), 20); short_cov = ts_backfill(vec_avg(<SHORT_COVER_FIELD>), 40); short_all = ts_backfill(vec_avg(<SHORT_INTEREST_ALL_FIELD>), 40); f_news = if_else(is_nan(news_sl), 0, rank(news_sl) - 0.5); f_cov = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); f_all = if_else(is_nan(short_all), 0, rank(short_all) - 0.5); dual_short = 0.60 * f_cov + 0.40 * f_all; vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.4); signal = 0.50 * signed_power(f_news, 1.5) + 0.40 * signed_power(dual_short, 1.4) + 0.15 * vol_burst + 0.15 * vwap_dev; delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); group_neutralize(signed_power(signal - delta_damp, 4.2), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 28,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_Short_VWAP_OrderFlow_Dec25", u),
                expression: "short_cov = ts_backfill(vec_avg(<SHORT_COVER_FIELD>), 40); f_short = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.4); vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); signal = 0.60 * signed_power(f_short, 1.4) + 0.25 * vwap_dev + 0.20 * vol_burst; delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); group_neutralize(signed_power(signal - delta_damp, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 25,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_NightDay_BorrowCost_Dec22", u),
                expression: "night_ret = winsorize((open - ts_delay(close, 1)) / ts_delay(close, 1)); day_ret = winsorize((close - open) / open); spread = ts_mean(night_ret, 10) - ts_mean(day_ret, 10); short_cov = ts_backfill(vec_avg(<SHORT_COVER_FIELD>), 40); f_short = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); signal = 0.65 * signed_power(rank(spread) - 0.5, 1.8) + 0.35 * signed_power(f_short, 1.4); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
        ],
        Some(FactorPillar::SystematicRisk) => vec![
            CandidateAlpha {
                name: format!("US_D1_{}_SysIdio_TermSpread_Dec16", u),
                expression: "idio_spread = ts_backfill(<IDIO_RISK_SHORT>, 10) / (ts_backfill(<IDIO_RISK_LONG>, 15) + 0.001); beta_spread = ts_backfill(<BETA_SHORT>, 10) - ts_backfill(<BETA_LONG>, 15); raw_sig = 0.65 * signed_power(0.5 - rank(idio_spread), 1.4) - 0.35 * signed_power(rank(beta_spread) - 0.5, 1.2); group_neutralize(signed_power(raw_sig, 3.5), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 16,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "SYSTEMATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_LowBeta_ResidualMom_Dec18", u),
                expression: "low_beta = signed_power(0.5 - rank(ts_backfill(<BETA_FIELD>, 15)), 1.5); corr_penalty = signed_power(rank(ts_backfill(<CORR_FIELD>, 15)) - 0.5, 1.2); rev = 0.05 * signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); sig = 0.60 * low_beta - 0.35 * corr_penalty - rev; group_neutralize(signed_power(sig, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 18,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "SYSTEMATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_IdioRisk_Curv_Dec14".to_string(),
                expression: "curv = ts_backfill(<IDIO_RISK_30D>, 10) - 2.0 * ts_backfill(<IDIO_RISK_60D>, 10) + ts_backfill(<IDIO_RISK_90D>, 10); sig = -1.0 * signed_power(rank(curv) - 0.5, 2.5); group_neutralize(signed_power(sig, 3.8), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 14,
                neutralization: "SECTOR".to_string(),
                truncation: 0.065,
                pillar: "SYSTEMATIC_RISK".to_string(),
            },
        ],
        Some(FactorPillar::FundamentalScores) => vec![
            CandidateAlpha {
                name: format!("US_D1_{}_FundAccel_PriceRev_Dec20", u),
                expression: "accel = rank(<FUND_ACCEL_FIELD>) - 0.5; cashflow = rank(<CASHFLOW_EFF_FIELD>) - 0.5; rev = rank(<ANALYST_REV_FIELD>) - 0.5; core_fund = 0.50 * accel + 0.30 * cashflow + 0.20 * rev; price_mom = 0.10 * (rank(ts_delta(close, 5)) - 0.5); sig = signed_power(core_fund, 1.5) - price_mom; group_neutralize(signed_power(sig, 3.6), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 20,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "FUNDAMENTAL_SCORES".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_MultiFactor_QualityMom_Dec22", u),
                expression: "composite = rank(<COMPOSITE_SCORE_DERIVATIVE>) - 0.5; val = rank(<RELATIVE_VAL_FIELD>) - 0.5; blended = 0.65 * composite + 0.35 * val; group_neutralize(signed_power(blended, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "FUNDAMENTAL_SCORES".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_Cashflow_Certainty_Dec18".to_string(),
                expression: "cf = rank(<CASHFLOW_EFF_FIELD>) - 0.5; cert = rank(<EARNINGS_CERTAINTY_FIELD>) - 0.5; sig = 0.55 * cf + 0.45 * cert; group_neutralize(signed_power(sig, 3.8), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 18,
                neutralization: "SECTOR".to_string(),
                truncation: 0.065,
                pillar: "FUNDAMENTAL_SCORES".to_string(),
            },
        ],
        Some(FactorPillar::NewsSentiment) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_News_NoveltyDrift_Dec20".to_string(),
                expression: "raw_sent = ts_backfill(vec_avg(<NEWS_SENTIMENT_VECTOR>), 15); novelty = ts_backfill(vec_avg(<NOVELTY_SCORE_VECTOR>), 15); sent_clean = if_else(is_nan(raw_sent), 0, rank(raw_sent) - 0.5); nov_wt = if_else(is_nan(novelty), 0.5, rank(novelty)); sig = signed_power(sent_clean, 1.4) * (0.5 + 0.5 * nov_wt) - 0.06 * (rank(ts_delta(close, 3)) - 0.5); group_neutralize(signed_power(sig, 4.0), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 20,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_SENTIMENT".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_News_EarningsShock_Dec22", u),
                expression: "earnings_sent = ts_backfill(vec_avg(<EARNINGS_EVAL_VECTOR>), 20); rel = ts_backfill(vec_avg(<RELEVANCE_SCORE_VECTOR>), 20); f_sent = if_else(is_nan(earnings_sent), 0, rank(earnings_sent) - 0.5); f_rel = if_else(is_nan(rel), 0.5, rank(rel)); sig = signed_power(f_sent, 1.5) * (0.4 + 0.6 * f_rel); group_neutralize(signed_power(sig, 4.2), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_SENTIMENT".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_News_CorpAction_Surge_Dec24".to_string(),
                expression: "corp = ts_backfill(vec_avg(<CORP_ACTION_VECTOR>), 25); f_corp = if_else(is_nan(corp), 0, rank(corp) - 0.5); vol_ratio = rank(volume / ts_mean(volume, 30)) - 0.5; sig = 0.65 * signed_power(f_corp, 1.5) + 0.35 * signed_power(vol_ratio, 1.2); group_neutralize(signed_power(sig, 3.8), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 24,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_SENTIMENT".to_string(),
            },
        ],
        Some(FactorPillar::OptionsAnalytics) => vec![
            CandidateAlpha {
                name: format!("US_D1_{}_Options_BreakevenAsym_Dec18", u),
                expression: "call_be = ts_backfill(<CALL_BREAKEVEN_FIELD> / close - 1, 10); put_be = ts_backfill(1 - <PUT_BREAKEVEN_FIELD> / close, 10); be_asym = rank(put_be - call_be) - 0.5; rev = 0.06 * (rank(ts_delta(close, 3)) - 0.5); sig = signed_power(be_asym, 1.4) - rev; group_neutralize(signed_power(sig, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 18,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "OPTIONS_ANALYTICS".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_Options_PCR_FlowDiv_Dec16", u),
                expression: "pcr_vol = rank(ts_decay_linear(ts_backfill(<PCR_VOL_FIELD>, 8), 5)); pcr_oi = rank(ts_decay_linear(ts_backfill(<PCR_OI_FIELD>, 12), 8)); flow_div = pcr_oi - pcr_vol; group_neutralize(signed_power(flow_div, 3.6), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 16,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "OPTIONS_ANALYTICS".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_Options_FwdParity_Skew_Dec14".to_string(),
                expression: "fwd_premium = ts_backfill(<FORWARD_PRICE_FIELD> / close - 1, 10); term_skew = ts_backfill(<PCR_TERM_SHORT> - <PCR_TERM_LONG>, 10); sig = 0.60 * signed_power(rank(fwd_premium) - 0.5, 1.4) - 0.40 * signed_power(rank(term_skew) - 0.5, 1.3); group_neutralize(signed_power(sig, 4.2), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 14,
                neutralization: "SECTOR".to_string(),
                truncation: 0.065,
                pillar: "OPTIONS_ANALYTICS".to_string(),
            },
        ],
        _ => get_default_curated_batch(u),
    }
}
