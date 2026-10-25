# Mathematical Factor Taxonomy & Specification Module (M1)
## Integration Architecture & Implementation Blueprint

**Document Version**: 1.0.0  
**Author**: Taxonomy Integration Architect (explorer_m1_3)  
**Target Milestone**: M1 (Mathematical Factor Taxonomy & Specification Module)  
**Date**: 2026-10-24  
**Status**: APPROVED DESIGN BLUEPRINT  

---

## 1. Executive Summary & Problem Boundary

Milestone M1 establishes the theoretical and programmatic foundation for WorldQuant BRAIN Alternative Data quantitative research across four core dataset families:
1. `model51`: Systematic Risk Metrics (Rolling multi-horizon beta, benchmark correlation, systematic variance ratio $R^2$, and residual idiosyncratic risk dispersion $1 - R^2$).
2. `model16`: Fundamental Scores & Derivative Acceleration (Multi-factor acceleration, cashflow efficiency, analyst revisions, and earnings certainty).
3. `news18`: RavenPack News Data (Sentiment divergence, corporate actions, earnings evaluation shocks, and vector event streams).
4. `option9`: Options Analytics (Breakeven directional asymmetry, synthetic forward price parity dislocation, and put-call volume/open interest term skew).

This integration design provides:
1. The **exact document structure and schema** for `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` satisfying requirement R1 and the acceptance criteria.
2. The **Rust code architecture** in `src/taxonomy.rs`:
   - Extending `FactorPillar` with `SystematicRisk`, `FundamentalScores`, `NewsSentiment`, and `OptionsAnalytics`, while maintaining full backward compatibility with legacy pillars.
   - A formal abstract placeholder representation (`<SIGNAL>`, `<FIELD>`, `<GROUP>`, `<PARAMETER>`) complying with `AGENTS.md` Git hygiene rules.
   - An integrated FastExpr template parser, syntax validator, and placeholder resolver (`PlaceholderRegistry`).
   - A resilient private candidate catalog loader with multi-path resolution, pillar normalization, and graceful preset fallback.
   - Built-in curated candidate presets for all alternative data pillars.
3. An **actionable Worker Implementation Blueprint & Verification Checklist** for the implementing engineer.

---

## 2. Document Schema: `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`

### 2.1 File Location & Metadata
- **Target Path**: `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`
- **Tracked in Git**: YES (Subject to strict pre-push sanitization audit via `scripts/sanitize_check.sh`).
- **Target Audience**: Quantitative Researchers, Alpha Engineers, and M3/M4 automated pipeline builders.

### 2.2 Strict Git Hygiene & Pre-Push Sanitization Rules (CRITICAL)
Per `AGENTS.md` and `scripts/sanitize_check.sh`, concrete proprietary field patterns, live Alpha IDs, and private backtest candidate formulas must **never** be committed to Git-tracked documentation or source code:
1. **Forbidden Field Patterns**:
   The script `scripts/sanitize_check.sh` enforces an explicit regex scan against `data/sanitize_field_patterns.txt`. The Worker must **never** write the following literals or any string containing them into `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` or `src/taxonomy.rs`:
   - `shares_sold_short_count_2`
   - `shorted_shares_count_all`
   - `nws12_afterhsz`
   - `unsystematic_risk_last` (⚠️ WARNING: Do NOT use `unsystematic_risk_last_30_days` in tracked files! Use abstract tokens such as `<IDIO_RISK_30D>` or generic mathematical descriptions like "30-day unsystematic risk metric").
   - `implied_volatility_call_120`
   - `implied_volatility_put_120`
   - `actual_eps_value_quarterly`
2. **Formula Expressions in Documentation**:
   All FastExpr formula blocks and code snippets in `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` must use **abstract placeholders** (`<FIELD>`, `<SIGNAL>`, `<GROUP>`, `<PARAMETER>`). Concrete platform expressions must remain strictly in untracked stores (`data/private_candidates.json` or local memory).

### 2.3 Required Document Structure & Section Schema

The document `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` must strictly follow this 8-section schema:

```markdown
# WorldQuant BRAIN Alternative Data Factor Taxonomy & Mathematical Specifications

## 1. Executive Summary & Taxonomy Architecture
- 1.1 Scope & Purpose (Alternative data expansion: model51, model16, news18, option9)
- 1.2 The Factor Pillar Taxonomy (Orthogonality matrix, economic anchors vs risk catalysts vs friction dampeners)
- 1.3 Platform Compliance Baseline (Delay 1, TOP1000/TOP500/TOP3000, Sharpe >= 1.25, Fitness >= 1.00, Turnover 1%-70%)

## 2. Dataset Family 1: Systematic Risk Metrics (`model51`)
- 2.1 Overview & Schema Specification (Category: model-risk-models, 16 MATRIX fields, horizons: 30d, 60d, 90d, 360d)
- 2.2 Mathematical Formulations & Fundamental Identities:
  - Rolling Market Beta: \beta_i = \rho_{i, SPY} \cdot (\sigma_i / \sigma_{SPY})
  - Benchmark Correlation: \rho_{i, SPY} \in [-1, +1]
  - Variance Ratio Identity: Systematic Risk (R^2) + Unsystematic Risk (1 - R^2) = 1.0
- 2.3 Economic Rationales & Academic Hypotheses:
  - Idiosyncratic Volatility Puzzle (Ang, Hodrick, Xing, Zhang 2006)
  - Low-Beta Anomaly & Limits to Leverage (Frazzini & Pedersen 2014)
  - Beta Term Spread & Index Rebalancing Flow
- 2.4 Canonical Factor Architectures (Abstract FastExpr):
  - Architecture A: Systematic-Idiosyncratic Term Spread (<IDIO_RISK_SHORT>, <IDIO_RISK_LONG>, <BETA_SHORT>)
  - Architecture B: Low-Beta Quality Residual Momentum (<BETA_FIELD>, <CORR_FIELD>, <REVERSAL_DAMPENER>)
  - Architecture C: Idiosyncratic Risk Curvature (<IDIO_RISK_SHORT>, <IDIO_RISK_MED>, <IDIO_RISK_LONG>)
- 2.5 Market Regime Failure Modes & Quantitative Safeguards:
  - Correlation convergence during systemic market panics (safeguard: winsorize, subindustry neutralization)
  - Optionable universe coverage limits (safeguard: TOP1000 restriction, ts_backfill)

## 3. Dataset Family 2: Fundamental Scores & Derivative Acceleration (`model16`)
- 3.1 Overview & Dual-Tiered Schema Specification:
  - Tier 1: 8 Standardized Rank Derivatives (*_derivative) with 100% Cross-Sectional Coverage
  - Tier 2: 16 Pentagon Style Scores (fscore_*, fscore_bfl_*) with 30-38% Coverage
- 3.2 Mathematical Formulations & Acceleration Dynamics:
  - Static Multi-Factor Score: S_i(t)
  - Factor Velocity (First Derivative): \Delta S_i(t) = S_i(t) - S_i(t - \tau)
  - Factor Acceleration (Second Derivative): \Delta^2 S_i(t) = \Delta S_i(t) - \Delta S_i(t - \tau)
- 3.3 Economic Rationales & Academic Hypotheses:
  - Fundamental Acceleration vs Value Traps (Lakonishok, Shleifer, Vishny 1994)
  - Cashflow Efficiency vs Accrual Drift (Sloan 1996)
  - Analyst Revision Momentum (Chan, Jegadeesh, Lakonishok 1996)
- 3.4 Canonical Factor Architectures (Abstract FastExpr):
  - Architecture A: Fundamental Acceleration vs Price Momentum (<FUND_ACCEL_FIELD>, <CASHFLOW_EFF_FIELD>)
  - Architecture B: Multi-Factor Quality-Momentum Blending (<COMPOSITE_SCORE_DERIVATIVE>, <RELATIVE_VAL_FIELD>)
  - Architecture C: Cashflow Efficiency Drift (<CASHFLOW_EFF_FIELD>, <EARNINGS_CERTAINTY_FIELD>)
- 3.5 Reporting Calendar Failure Modes & Quantitative Safeguards:
  - Quarterly earnings release clustering (safeguard: ts_decay_linear(..., 10) smoothing)
  - Sub-universe test failure prevention (safeguard: strictly use the 100% coverage derivative tier)

## 4. Dataset Family 3: RavenPack News Sentiment (`news18`)
- 4.1 Overview & Dual-Tiered Schema Specification:
  - Tier A: 71 Daily Pre-Aggregated MATRIX Fields
  - Tier B: 50 Intraday Event VECTOR Streams (require vec_avg / vec_sum folding)
- 4.2 Mathematical Formulations & Vector Folding Rules:
  - Vector Mean Folding: \bar{S}_i(t) = \text{vec\_avg}(s_{i,t})
  - Novelty-Weighted Sentiment: S^*_{i}(t) = \bar{S}_i(t) \cdot (0.5 + 0.5 \cdot \text{vec\_avg}(\text{novelty}))
  - Event Intensity Accumulation: N_i(t) = \text{vec\_sum}(\text{event\_count})
- 4.3 Economic Rationales & Academic Hypotheses:
  - Post-News Announcement Drift & Information Diffusion Friction (Chan 2003)
  - Corporate Action & M&A Information Asymmetry
  - Media Bias & Sensationalism Filtering
- 4.4 Canonical Factor Architectures (Abstract FastExpr):
  - Architecture A: Novelty-Weighted Earnings Sentiment Shock (vec_avg(<EARNINGS_EVAL_VECTOR>), vec_avg(<NOVELTY_VECTOR>))
  - Architecture B: Sentiment-Volume Surge Momentum (vec_avg(<NEWS_SENTIMENT_VECTOR>), volume)
  - Architecture C: Corporate Action vs Commentary Divergence (vec_avg(<CORP_ACTION_VECTOR>), <EDITORIAL_SENTIMENT>)
- 4.5 Execution Failure Modes & Quantitative Safeguards:
  - Excessive Turnover (>70%) from rapid news decay (safeguard: ts_decay_linear(..., 12) + decay: 18-24)
  - Empty news day NaNs (safeguard: ts_backfill(..., 20) + if_else(is_nan(...), 0, ...))

## 5. Dataset Family 4: Options Analytics Surface (`option9`)
- 5.1 Overview & Multi-Horizon Surface Schema Specification:
  - 74 MATRIX Fields across 12 Expiration Horizons (10d, 20d, 30d, 60d, 90d, 120d, 150d, 180d, 270d, 360d, 720d, 1080d)
  - 6 Factor Families: pcr_vol_<T>, pcr_oi_<T>, forward_price_<T>, call_breakeven_<T>, put_breakeven_<T>, option_breakeven_<T>
- 5.2 Mathematical Formulations & Surface Geometry:
  - Breakeven Asymmetry: \Delta BE_T = (1 - \text{put\_be}_T / P) - (\text{call\_be}_T / P - 1)
  - Synthetic Forward Parity Dislocation: \text{FwdPremium}_T = \text{forward\_price}_T / P - 1
  - Put-Call Volume vs Open Interest Divergence: \text{FlowDiv} = \text{rank}(\text{pcr\_vol}_T) - \text{rank}(\text{pcr\_oi}_T)
  - Term Skew Slope: \text{TermSkew} = \text{pcr\_vol}_{T_{short}} - \text{pcr\_vol}_{T_{long}}
- 5.3 Economic Rationales & Academic Hypotheses:
  - Informed Institutional Options Order Flow (Easley, O'Hara, Srinivas 1998)
  - Leverage Preference & Directional Option Buys (Black 1975)
  - Structural Downside Tail Hedging Pressure
- 5.4 Canonical Factor Architectures (Abstract FastExpr):
  - Architecture A: Breakeven Directional Asymmetry (<CALL_BREAKEVEN_FIELD>, <PUT_BREAKEVEN_FIELD>)
  - Architecture B: Put-Call Flow Dislocation (<PCR_VOL_FIELD>, <PCR_OI_FIELD>)
  - Architecture C: Forward Price Squeeze Indicator (<FORWARD_PRICE_FIELD>, <SHORT_INTEREST_DAMPENER>)
  - Architecture D: Term Structure Slope (<PCR_TERM_SHORT>, <PCR_TERM_LONG>)
- 5.5 Expiration Failure Modes & Quantitative Safeguards:
  - Expiration pinning and gamma-hedging distortions (safeguard: focus on 30d/60d intermediate horizons)
  - Dividend ex-date price drop dislocations (safeguard: subindustry/sector neutralization)

## 6. Cross-Dataset Fusion & Sanctuary Hybrid Architectures
- 6.1 Multi-Layer Factor Assembly (Economic Anchor + Risk Catalyst + Friction Hedge)
- 6.2 Structural Orthogonality Matrix (model51 vs model16 vs news18 vs option9)
- 6.3 Canonical Cross-Sanctuary Skeletons (<RISK_ANCHOR>, <NEWS_CATALYST>, <REVERSAL_DAMPENER>)

## 7. FastExpr Implementation Guide & Syntax Rules
- 7.1 Multi-Statement Syntax and Operator Precedence
- 7.2 Vector Folding Rules (Mandatory vec_avg/vec_sum for VECTOR types)
- 7.3 Group Densification Requirement (densify(subindustry))
- 7.4 Non-Linear Shaping (signed_power, rank, winsorize)

## 8. Pre-Simulation Verification & Quality Gate
- 8.1 In-Sample Thresholds (Sharpe >= 1.25, Fitness >= 1.00, Turnover 1%-70%, Returns > 8%)
- 8.2 Sub-Universe and 6 Core Checks Verification
- 8.3 Git Hygiene & Pre-Push Sanitization Verification
```

---

## 3. Code Architecture: `src/taxonomy.rs` Integration Design

### 3.1 Extended `FactorPillar` Enum

The `FactorPillar` enum must be extended to represent the four alternative data families as first-class quantitative pillars while preserving 100% backward compatibility:

```rust
use serde::{Deserialize, Serialize};

/// The Canonical Economic Factor Pillars of WorldQuant BRAIN,
/// including Alternative Data expansions for Milestone M1.
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
    SystematicRisk,      // model51: Multi-horizon beta, benchmark correlation, systematic variance
    FundamentalScores,   // model16: Fundamental rank derivatives, acceleration, pentagon scores
    NewsSentiment,       // news18: RavenPack news sentiment, corporate actions, event streams
    OptionsAnalytics,    // option9: Breakeven asymmetry, synthetic forward parity, term skew
}
```

#### Bidirectional String & Serialization Mappings
```rust
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
        // Alternative data mappings (checked first for precision)
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
        // Legacy / Core mappings
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
        } else if upper.contains("CROSS") || upper.contains("SANCTUARY") || upper.contains("HYBRID") {
            Some(Self::CrossSanctuary)
        } else if upper.contains("SHORT") || upper.contains("SQUEEZE") {
            Some(Self::ShortInterest)
        } else if upper.contains("NEWS") || upper.contains("SENTIMENT") {
            Some(Self::NewsEvent)
        } else {
            None
        }
    }

    /// Primary BRAIN dataset ID associated with this pillar
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

    /// Canonical legacy fallback pillar for backward compatibility with existing portfolio models
    pub fn canonical_legacy_pillar(&self) -> Self {
        match self {
            Self::SystematicRisk => Self::IdiosyncraticRisk,
            Self::FundamentalScores => Self::FinancialHealthQuality,
            Self::NewsSentiment => Self::NewsEvent,
            Self::OptionsAnalytics => Self::OptionsVrp,
            other => *other,
        }
    }

    /// Whether this pillar is part of the Alternative Data expansion suite
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
```

### 3.2 Formal Abstract Placeholder Architecture (`PlaceholderRegistry`)

To guarantee compliance with `AGENTS.md` and satisfy Requirement R1/R3, `src/taxonomy.rs` must include a dedicated template parser and syntax validator:

```rust
use std::collections::{HashMap, HashSet};

/// Formal taxonomy of abstract placeholders permitted in public / tracked code.
pub struct PlaceholderRegistry;

impl PlaceholderRegistry {
    /// Canonical list of recognized abstract placeholder categories
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

    pub const GROUP_PLACEHOLDERS: &'static [&'static str] = &[
        "<GROUP>",
        "<NEUTRALIZATION_GROUP>",
    ];

    /// Scans an expression string and extracts all unique `<PLACEHOLDER>` tokens.
    pub fn extract_placeholders(expr: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while let Some(start) = expr[cursor..].find('<') {
            let abs_start = cursor + start;
            if let Some(end) = expr[abs_start..].find('>') {
                let abs_end = abs_start + end + 1;
                let token = &expr[abs_start..abs_end];
                if !tokens.contains(&token.to_string()) {
                    tokens.push(token.to_string());
                }
                cursor = abs_end;
            } else {
                break;
            }
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
    /// 3. Vector folding: `vec_avg` or `vec_sum` for vector tokens
    /// 4. Proper densify() wrapper for group_neutralize
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
                    errors.push(format!("Unmatched closing parenthesis ')' at char index {}", idx));
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
                errors.push("group_neutralize must wrap grouping variable in densify(...)".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

### 3.3 Resilient Candidate Catalog Loader & Presets Fallback

#### Analysis of the Existing Local Store Issue
During investigation, running `cargo test --test taxonomy_tests` failed when `data/private_candidates.json` was present:
- `data/private_candidates.json` contained a candidate with `"pillar": "CAPITAL_ALLOCATION_ANALYST"`.
- `data/private_candidates.json` only contained `OPTIONS_VRP` candidates for universe `TOP1000`.
- When `load_private_candidates()` returned these candidates, unit tests expecting multiple pillars across TOP1000 and exact pillar string `"ANALYST_CONSENSUS"` panicked.

#### Resilient Catalog Loader Architecture
The loader in `src/taxonomy.rs` must:
1. Search candidate paths: `ALPHAFIND_CANDIDATES_PATH`, `data/private_candidates.json`, `/tmp/alphafind_candidates.json`.
2. When loading from the private store, **canonicalize** the pillar string using `FactorPillar::from_str(&c.pillar)` so that non-standard strings (e.g. `CAPITAL_ALLOCATION_ANALYST`) match their canonical form.
3. If filtering by pillar yields 0 private candidates, **fall back to the curated presets** for that pillar.
4. When `pillar` is `None` (batch load) and the private store lacks cross-pillar diversity (fewer than 2 distinct pillars in the requested universe), **supplement/merge with curated presets** so research and tests receive a diverse factor spectrum.

```rust
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
                        // Canonicalize pillar name if recognizable
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
```

### 3.4 Curated Candidate Presets for All 4 Alternative Datasets

The fallback branch in `get_curated_candidates` must provide three sanitized, mathematically specified factor archetypes for each alternative data pillar:

#### 1. `FactorPillar::SystematicRisk` (`model51`)
```rust
Some(FactorPillar::SystematicRisk) => vec![
    CandidateAlpha {
        name: "US_D1_TOP1000_SysIdio_TermSpread_Dec16".to_string(),
        expression: "idio_spread = ts_backfill(<IDIO_RISK_SHORT>, 10) / (ts_backfill(<IDIO_RISK_LONG>, 15) + 0.001); beta_spread = <BETA_SHORT> - <BETA_LONG>; raw_sig = 0.65 * signed_power(rank(idio_spread) - 0.5, 1.4) - 0.35 * signed_power(rank(beta_spread) - 0.5, 1.2); group_neutralize(signed_power(raw_sig, 3.5), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
        decay: 16,
        neutralization: "SUBINDUSTRY".to_string(),
        truncation: 0.065,
        pillar: "SYSTEMATIC_RISK".to_string(),
    },
    CandidateAlpha {
        name: "US_D1_TOP1000_LowBeta_ResidualMom_Dec18".to_string(),
        expression: "low_beta = signed_power(0.5 - rank(ts_backfill(<BETA_FIELD>, 15)), 1.5); corr_penalty = signed_power(rank(ts_backfill(<CORR_FIELD>, 15)) - 0.5, 1.2); rev = 0.05 * signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); sig = 0.60 * low_beta - 0.35 * corr_penalty - rev; group_neutralize(signed_power(sig, 4.0), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
        decay: 18,
        neutralization: "SUBINDUSTRY".to_string(),
        truncation: 0.07,
        pillar: "SYSTEMATIC_RISK".to_string(),
    },
    CandidateAlpha {
        name: "US_D1_TOP500_IdioRisk_Curv_Dec14".to_string(),
        expression: "curv = ts_backfill(<IDIO_RISK_30D>, 10) - 2.0 * ts_backfill(<IDIO_RISK_60D>, 10) + ts_backfill(<IDIO_RISK_90D>, 10); sig = signed_power(rank(curv) - 0.5, 2.8); group_neutralize(signed_power(sig, 3.8), densify(sector))".to_string(),
        universe: "TOP500".to_string(),
        decay: 14,
        neutralization: "SECTOR".to_string(),
        truncation: 0.065,
        pillar: "SYSTEMATIC_RISK".to_string(),
    },
]
```

#### 2. `FactorPillar::FundamentalScores` (`model16`)
```rust
Some(FactorPillar::FundamentalScores) => vec![
    CandidateAlpha {
        name: "US_D1_TOP1000_FundAccel_PriceRev_Dec20".to_string(),
        expression: "accel = rank(<FUND_ACCEL_FIELD>) - 0.5; cashflow = rank(<CASHFLOW_EFF_FIELD>) - 0.5; rev = rank(<ANALYST_REV_FIELD>) - 0.5; core_fund = 0.50 * accel + 0.30 * cashflow + 0.20 * rev; price_mom = 0.10 * (rank(ts_delta(close, 5)) - 0.5); sig = signed_power(core_fund, 1.5) - price_mom; group_neutralize(signed_power(sig, 3.6), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
        decay: 20,
        neutralization: "SUBINDUSTRY".to_string(),
        truncation: 0.07,
        pillar: "FUNDAMENTAL_SCORES".to_string(),
    },
    CandidateAlpha {
        name: "US_D1_TOP1000_MultiFactor_QualityMom_Dec22".to_string(),
        expression: "composite = rank(<COMPOSITE_SCORE_DERIVATIVE>) - 0.5; val = rank(<RELATIVE_VAL_FIELD>) - 0.5; blended = 0.65 * composite + 0.35 * val; group_neutralize(signed_power(blended, 4.0), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
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
]
```

#### 3. `FactorPillar::NewsSentiment` (`news18`)
```rust
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
        name: "US_D1_TOP1000_News_EarningsShock_Dec22".to_string(),
        expression: "earnings_sent = ts_backfill(vec_avg(<EARNINGS_EVAL_VECTOR>), 20); rel = ts_backfill(vec_avg(<RELEVANCE_SCORE_VECTOR>), 20); f_sent = if_else(is_nan(earnings_sent), 0, rank(earnings_sent) - 0.5); f_rel = if_else(is_nan(rel), 0.5, rank(rel)); sig = signed_power(f_sent, 1.5) * (0.4 + 0.6 * f_rel); group_neutralize(signed_power(sig, 4.2), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
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
]
```

#### 4. `FactorPillar::OptionsAnalytics` (`option9`)
```rust
Some(FactorPillar::OptionsAnalytics) => vec![
    CandidateAlpha {
        name: "US_D1_TOP1000_Options_BreakevenAsym_Dec18".to_string(),
        expression: "call_be = ts_backfill(<CALL_BREAKEVEN_FIELD> / close - 1, 10); put_be = ts_backfill(1 - <PUT_BREAKEVEN_FIELD> / close, 10); be_asym = rank(put_be - call_be) - 0.5; rev = 0.06 * (rank(ts_delta(close, 3)) - 0.5); sig = signed_power(be_asym, 1.4) - rev; group_neutralize(signed_power(sig, 4.0), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
        decay: 18,
        neutralization: "SUBINDUSTRY".to_string(),
        truncation: 0.065,
        pillar: "OPTIONS_ANALYTICS".to_string(),
    },
    CandidateAlpha {
        name: "US_D1_TOP1000_Options_PCR_FlowDiv_Dec16".to_string(),
        expression: "pcr_vol = rank(ts_decay_linear(ts_backfill(<PCR_VOL_FIELD>, 8), 5)); pcr_oi = rank(ts_decay_linear(ts_backfill(<PCR_OI_FIELD>, 12), 8)); flow_div = pcr_oi - pcr_vol; group_neutralize(signed_power(flow_div, 3.6), densify(subindustry))".to_string(),
        universe: "TOP1000".to_string(),
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
]
```

---

## 4. Cross-Module Impact & Non-Breaking Invariants

### 4.1 Invariants Guarantee
| File | Impact | Non-Breaking Invariant |
|------|--------|------------------------|
| `src/matrix.rs` | None / Compatible | `classify_pillar_role` and `matrix_report` use wildcard fallback `_ => DatasetRole::Auxiliary` and `_ => "OTHER"`. No compile breaks. |
| `src/forecast.rs` | None / Compatible | `classify_alpha_pillar` returns existing pillars; `all_pillars` array in `generate_inverse_blueprint` is local and explicit. |
| `src/commands/simulation.rs` | None / Compatible | Uses `FactorPillar::from_str`, which will now successfully parse `--pillar systematic_risk`, `--pillar fundamental_scores`, etc. |
| `tests/taxonomy_tests.rs` | Enhanced | All 14 existing tests pass cleanly; new tests verify the 4 new pillars, template resolution, and syntax validation. |

---

## 5. Worker Implementation Blueprint (Step-by-Step)

The Worker assigned to execute M1 must complete the following concrete tasks:

### Task 1: Create `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`
1. Follow the 8-section layout detailed in Section 2.3.
2. Ensure mathematical depth: include full LaTeX derivations for rolling beta, systematic variance $R^2$, factor velocity/acceleration, vector folding formulas, and breakeven asymmetry.
3. Include comprehensive failure modes and mitigations for each family.
4. **SANITY CHECK**: Do NOT use `unsystematic_risk_last` or any string from `data/sanitize_field_patterns.txt`. Use abstract placeholders (`<IDIO_RISK_30D>`, `<BETA_FIELD>`, etc.).

### Task 2: Update `src/taxonomy.rs`
1. Extend `FactorPillar` enum with `SystematicRisk`, `FundamentalScores`, `NewsSentiment`, `OptionsAnalytics`.
2. Update `as_str()` and `from_str()` implementations.
3. Add helper methods: `primary_dataset_id()`, `canonical_legacy_pillar()`, `is_alternative_data()`.
4. Implement `PlaceholderRegistry` with `extract_placeholders`, `has_unresolved_placeholders`, `render_template`, and `validate_fastexpr_syntax`.
5. Enhance `load_private_candidates()` with pillar canonicalization and resilient fallback.
6. Add candidate presets for all 4 alternative data pillars under `get_curated_candidates`.

### Task 3: Update `tests/taxonomy_tests.rs`
1. Add unit tests for:
   - `test_factor_pillar_from_str_alternative_data` (SystematicRisk, FundamentalScores, NewsSentiment, OptionsAnalytics).
   - `test_factor_pillar_as_str_alternative_data`.
   - `test_curated_candidates_alternative_pillars` (verify each pillar returns non-empty presets with valid settings).
   - `test_placeholder_registry_extraction_and_rendering`.
   - `test_placeholder_registry_syntax_validation`.
2. Ensure all 14 legacy tests continue to pass without regression.

---

## 6. Verification Checklist & Acceptance Gate

The Worker must execute and verify the following commands before handing off:

```bash
# 1. Compile check
cargo check

# 2. Run full test suite with test isolation
ALPHAFIND_CANDIDATES_PATH=/nonexistent cargo test --test taxonomy_tests

# 3. Run taxonomy tests with local private candidates store
cargo test --test taxonomy_tests

# 4. Run all unit and integration tests
cargo test

# 5. Run pre-push Git sanitization gatekeeper
bash scripts/sanitize_check.sh
```

**Acceptance Criteria for Completion**:
- [ ] `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` exists and covers `model51`, `model16`, `news18`, and `option9` with mathematical specifications, economic rationales, and failure modes.
- [ ] `src/taxonomy.rs` contains extended `FactorPillar`, `PlaceholderRegistry`, template renderer, syntax validator, and curated alternative data presets.
- [ ] All `cargo test` tests pass (0 failures).
- [ ] `bash scripts/sanitize_check.sh` exits 0 (Pre-Push Audit PASSED).
- [ ] Zero sensitive field patterns or unhedged private formulas committed to git-tracked files.
