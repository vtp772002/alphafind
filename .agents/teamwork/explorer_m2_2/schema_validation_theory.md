# Field Schema & Type Validator Theory Specification
**Milestone M2: Field Metadata Extraction & Schema Validator**  
**Document Version**: 1.0.0 (Authoritative Theoretical Specification)  
**Author**: Field Schema & Type Validator Theorist (`explorer_m2_2`)  
**Target Platform**: WorldQuant BRAIN FastExpr Engine & AlphaFind Framework  
**Integrity Mode**: Rigorous Academic & Quantitative Validation Standards (AGENTS.md Compliant)  

---

## 1. Executive Summary & Theoretical Framework

In WorldQuant BRAIN's quantitative simulation environment, candidate alpha expressions written in `FastExpr` are executed over high-dimensional tensor spaces ($[T \times N]$ for dates $\times$ instruments, or $[T \times N \times K]$ for intraday event vectors). In standard equity price-volume workflows, expressions operate almost exclusively on 2D matrix fields (`close`, `volume`, `vwap`), where implicit type promotions and homogeneous temporal grids prevent syntax and execution errors.

However, expanding the quantitative research universe into **Alternative Datasets** (`model51` Systematic Risk, `model16` Fundamental Scores & Derivative Acceleration, `news18` RavenPack News, and `option9` Options Analytics Surface) introduces substantial multidimensional complexity:
1. **Heterogeneous Tensor Geometries**: `news18` presents a dual-tiered architecture containing both 2D daily aggregates (`MATRIX`) and variable-length 3D event lists (`VECTOR`), where direct scalar/matrix arithmetic triggers fatal HTTP 400 Bad Request simulation aborts.
2. **Extreme Cross-Sectional Coverage Disparities**: While `model16` Tier 1 rank derivatives and `option9` surface fields guarantee $98\% - 100\%$ universe coverage, `model16` raw pentagons exhibit $\sim 29\% - 38\%$ coverage, creating fatal sub-universe test failures unless guarded by verified lookback backfilling.
3. **Horizon Invariants & String Syntax Rules**: Horizon representations diverge sharply between datasets (e.g. `option9` uses strict integer suffixes such as `pcr_vol_20`, whereas appending `'d'` as in `pcr_vol_20d` results in unknown field exceptions).
4. **Temporal Look-Ahead & Point-in-Time Dissemination**: Signals generated under `delay = 1` execution require formal verification that no negative time lags, future-referencing windows, or Delay 0 intraday fields are improperly ingested.

This document establishes the **Formal Field Schema and Type Validator Theory** for Milestone M2. It defines:
- The authoritative catalog and structural invariants for each alternative dataset.
- A formal type system and type lattice for `FastExpr`.
- Operational folding mechanics and illegal operator grammar rules.
- Coverage threshold validation and ancestral AST backfill verification.
- Temporal causal integrity constraints preventing look-ahead bias.
- An architectural engine design (AST lexer, parser, type checker, diagnostic engine) ready for implementation in Rust/Python.

---

## 2. Alternative Data Field Schemas & Platform Catalogs

```
+--------------------------------------------------------------------------------------------------------------------+
|                                    ALTERNATIVE DATASET SCHEMA TOPOLOGY                                             |
+---------------+------------------------+------------+------------+---------------+----------------+----------------+
| Dataset ID    | Family Name            | Category   | Subcategory| Matrix Fields | Vector Fields  | Typical Cov    |
+---------------+------------------------+------------+------------+---------------+----------------+----------------+
| model51       | Systematic Risk        | model      | risk       | 16            | 0              | 93.7% - 97.6%  |
| model16       | Fundamental Scores     | model      | valuation  | 24            | 0              | 29.5% - 100.0% |
| news18        | RavenPack News Data    | news       | sentiment  | 71 (Delay 1)  | 50 (Delay 1)   | 73.0% - 100.0% |
| option9       | Options Analytics      | option     | analytics  | 74            | 0              | 97.9% - 98.4%  |
+---------------+------------------------+------------+------------+---------------+----------------+----------------+
```

---

### 2.1 Dataset `model51`: Systematic Risk Metrics

#### 2.1.1 Overview & Parametric Space
- **Dataset ID**: `model51`
- **Category / Subcategory**: `model` / `model-risk-models`
- **Primary Asset Class**: US Equities (TOP200, TOP500, TOP1000, TOP2000, TOP3000, TOPSP500)
- **Supported Delays**: Delay 1 (primary), Delay 0
- **Total Fields**: Exactly **16 fields**, all of type `MATRIX`.
- **Benchmark Instrument**: SPY (SPDR S&P 500 ETF Trust)
- **Valid Estimation Horizons**: $\mathcal{H}_{51} = \{30, 60, 90, 360\}$ calendar days.
- **Cross-Sectional Coverage**: $C_{\text{cross}} \in [0.9370, 0.9765]$ in `TOP1000`.
- **Date Coverage**: $C_{\text{date}} = 1.0$ (Complete historical coverage from 2022 to present).

#### 2.1.2 Complete Field Schema Catalog
The 16 fields map into four canonical regression and variance metrics across the 4 horizons:

| # | Field Identifier | Tensor Type | Horizon | Coverage | Range / Domain | Theoretical Economic Definition |
|---|------------------|-------------|---------|----------|----------------|----------------------------------|
| 1 | `beta_last_30_days_spy` | `MATRIX` | 30d | 0.9765 | $\mathbb{R}$ | Rolling OLS slope $\beta_{30} = \frac{\text{Cov}_{30}(R_i, R_{\text{SPY}})}{\text{Var}_{30}(R_{\text{SPY}})}$ |
| 2 | `beta_last_60_days_spy` | `MATRIX` | 60d | 0.9752 | $\mathbb{R}$ | Rolling OLS slope $\beta_{60}$ relative to SPY |
| 3 | `beta_last_90_days_spy` | `MATRIX` | 90d | 0.9734 | $\mathbb{R}$ | Rolling OLS slope $\beta_{90}$ relative to SPY |
| 4 | `beta_last_360_days_spy` | `MATRIX` | 360d | 0.9482 | $\mathbb{R}$ | Rolling structural OLS slope $\beta_{360}$ relative to SPY |
| 5 | `correlation_last_30_days_spy` | `MATRIX` | 30d | 0.9765 | $[-1.0, +1.0]$ | Rolling Pearson correlation $\rho_{30}(R_i, R_{\text{SPY}})$ |
| 6 | `correlation_last_60_days_spy` | `MATRIX` | 60d | 0.9752 | $[-1.0, +1.0]$ | Rolling Pearson correlation $\rho_{60}(R_i, R_{\text{SPY}})$ |
| 7 | `correlation_last_90_days_spy` | `MATRIX` | 90d | 0.9734 | $[-1.0, +1.0]$ | Rolling Pearson correlation $\rho_{90}(R_i, R_{\text{SPY}})$ |
| 8 | `correlation_last_360_days_spy` | `MATRIX` | 360d | 0.9482 | $[-1.0, +1.0]$ | Rolling Pearson correlation $\rho_{360}(R_i, R_{\text{SPY}})$ |
| 9 | `systematic_risk_last_30_days` | `MATRIX` | 30d | 0.9650 | $[0.0, 1.0]$ | Proportion of variance explained by SPY ($R^2_{30}$) |
| 10 | `systematic_risk_last_60_days` | `MATRIX` | 60d | 0.9637 | $[0.0, 1.0]$ | Proportion of variance explained by SPY ($R^2_{60}$) |
| 11 | `systematic_risk_last_90_days` | `MATRIX` | 90d | 0.9619 | $[0.0, 1.0]$ | Proportion of variance explained by SPY ($R^2_{90}$) |
| 12 | `systematic_risk_last_360_days` | `MATRIX` | 360d | 0.9370 | $[0.0, 1.0]$ | Proportion of variance explained by SPY ($R^2_{360}$) |
| 13 | `unsystematic_risk_last_30_days` | `MATRIX` | 30d | 0.9650 | $[0.0, 1.0]$ | Idiosyncratic risk ratio: $1 - R^2_{30} = \frac{\sigma_{\epsilon, 30}^2}{\sigma_{i, 30}^2}$ |
| 14 | `unsystematic_risk_last_60_days` | `MATRIX` | 60d | 0.9637 | $[0.0, 1.0]$ | Idiosyncratic risk ratio: $1 - R^2_{60} = \frac{\sigma_{\epsilon, 60}^2}{\sigma_{i, 60}^2}$ |
| 15 | `unsystematic_risk_last_90_days` | `MATRIX` | 90d | 0.9619 | $[0.0, 1.0]$ | Idiosyncratic risk ratio: $1 - R^2_{90} = \frac{\sigma_{\epsilon, 90}^2}{\sigma_{i, 90}^2}$ |
| 16 | `unsystematic_risk_last_360_days` | `MATRIX` | 360d | 0.9370 | $[0.0, 1.0]$ | Idiosyncratic risk ratio: $1 - R^2_{360} = \frac{\sigma_{\epsilon, 360}^2}{\sigma_{i, 360}^2}$ |

#### 2.1.3 Mathematical Constraints & Validation Invariants
1. **Variance Conservation Invariant**:
   $$\forall T \in \mathcal{H}_{51}, \quad \text{systematic\_risk\_last\_}T\text{\_days} + \text{unsystematic\_risk\_last\_}T\text{\_days} \equiv 1.0$$
2. **Correlation-Variance Parity**:
   $$\forall T \in \mathcal{H}_{51}, \quad \text{systematic\_risk\_last\_}T\text{\_days} \equiv (\text{correlation\_last\_}T\text{\_days\_spy})^2$$
3. **Horizon Syntax Enforcement**:
   Any identifier matching `beta_last_*`, `correlation_last_*`, `systematic_risk_last_*`, or `unsystematic_risk_last_*` where the horizon $T \notin \{30, 60, 90, 360\}$ is flagged as an invalid field token.

---

### 2.2 Dataset `model16`: Fundamental Scores & Derivative Acceleration

#### 2.2.1 Overview & Dual-Tiered Topology
- **Dataset ID**: `model16`
- **Category / Subcategory**: `model` / `model-valuation-models` (or `model-valuation`)
- **Primary Asset Class**: US Equities (TOP200 through TOP3000)
- **Supported Delays**: Delay 1
- **Total Fields**: Exactly **24 fields**, all of type `MATRIX`.
- **Architectural Bifurcation**:
  - **Tier 1 (Rank Derivatives)**: 8 fields with **100.0% coverage** ($C_{\text{cross}} = 1.0000$).
  - **Tier 2 (Backfilled Style Pentagons)**: 8 fields with **$37.26\% - 38.28\%$ coverage**.
  - **Tier 3 (Raw Style Pentagons)**: 8 fields with **$29.46\% - 29.93\%$ coverage**.

#### 2.2.2 Complete Field Schema Catalog

| # | Field Identifier | Tensor Type | Tier / Class | Coverage | Range | Structural Meaning |
|---|------------------|-------------|--------------|----------|-------|--------------------|
| 1 | `analyst_revision_rank_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Revision momentum in consensus earnings estimates |
| 2 | `cashflow_efficiency_rank_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Operating cash flow generation normalized by assets |
| 3 | `composite_factor_score_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Blended multi-factor revision velocity |
| 4 | `earnings_certainty_rank_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Sustainability and low dispersion of earnings quality |
| 5 | `growth_potential_rank_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Percentile rank of medium-term growth potential |
| 6 | `multi_factor_acceleration_score_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Convexity: $\frac{\partial^2 S}{\partial t^2}$ of composite score |
| 7 | `multi_factor_static_score_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Velocity: $\frac{\partial S}{\partial t}$ of composite score |
| 8 | `relative_valuation_rank_derivative` | `MATRIX` | Tier 1 (Rank Deriv) | 1.0000 | $[0.0, 1.0]$ | Percentile rank of composite valuation multiples |
| 9 | `fscore_bfl_growth` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3807 | $[0, 100]$ | Backfilled growth style composite |
| 10 | `fscore_bfl_momentum` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3774 | $[0, 100]$ | Backfilled price-analyst momentum score |
| 11 | `fscore_bfl_profitability` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3797 | $[0, 100]$ | Backfilled profitability & cash generation |
| 12 | `fscore_bfl_quality` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3828 | $[0, 100]$ | Backfilled balance-sheet quality score |
| 13 | `fscore_bfl_surface` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3740 | $[0, 100]$ | Backfilled style pentagon geometric surface area |
| 14 | `fscore_bfl_surface_accel` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3726 | $[-100, 100]$| Backfilled acceleration of style pentagon surface |
| 15 | `fscore_bfl_total` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3726 | $[0, 100]$ | Backfilled composite Piotroski-style M-score |
| 16 | `fscore_bfl_value` | `MATRIX` | Tier 2 (Bfl Pentagon) | 0.3808 | $[0, 100]$ | Backfilled valuation composite (higher = cheaper) |
| 17 | `fscore_growth` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2987 | $[0, 100]$ | Raw unbackfilled growth score |
| 18 | `fscore_momentum` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2973 | $[0, 100]$ | Raw unbackfilled momentum score |
| 19 | `fscore_profitability` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2981 | $[0, 100]$ | Raw unbackfilled profitability score |
| 20 | `fscore_quality` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2993 | $[0, 100]$ | Raw unbackfilled quality score |
| 21 | `fscore_surface` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2957 | $[0, 100]$ | Raw unbackfilled pentagon area |
| 22 | `fscore_surface_accel` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2946 | $[-100, 100]$| Raw unbackfilled pentagon acceleration |
| 23 | `fscore_total` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2946 | $[0, 100]$ | Raw unbackfilled composite total score |
| 24 | `fscore_value` | `MATRIX` | Tier 3 (Raw Pentagon) | 0.2981 | $[0, 100]$ | Raw unbackfilled valuation score |

#### 2.2.3 Tier Validation Rules & Sub-Universe Risks
1. **Tier 1 Exemption**: Tier 1 fields require no backfilling ($C_{\text{cross}} = 1.0000$). They are guaranteed to pass BRAIN's Sub-universe Consistency check (`subuniverse_sharpe`).
2. **Tier 2/3 Mandatory Backfill**: Because Tier 2/3 fields have $C_{\text{cross}} < 0.50$ ($29\% - 38\%$), using any `fscore_*` or `fscore_bfl_*` field without wrapping it in `ts_backfill(..., d)` ($d \ge 10$) triggers a **FATAL Validation Error**: `MissingMandatoryBackfill`.
3. **Sub-Universe Inconsistency Warning**: Even when backfilled, the extreme sparsity of Tier 2/3 fields in small capitalization deciles creates substantial risk of failing the platform's subuniverse check. The validator must emit a `Severity::Warning` recommending that production candidates utilize the Tier 1 rank derivative equivalents.

---

### 2.3 Dataset `news18`: RavenPack News Data

#### 2.3.1 Overview & Tensor Bifurcation
- **Dataset ID**: `news18`
- **Category / Subcategory**: `news` / `news-news-sentiment`
- **Primary Asset Class**: Global & US Equities
- **Supported Delays**: Delay 1 (121 fields), Delay 0 (24 fields)
- **Total Fields (Delay 1)**: Exactly **121 fields**:
  - **71 MATRIX fields**: Daily cross-sectional summaries ($\mathbf{M} \in \mathbb{R}^{T \times N}$).
  - **50 VECTOR fields**: Intraday granular event streams ($\mathbf{V}_{i, t} \in \mathbb{R}^{K_{i, t}}$).

#### 2.3.2 Representative Field Catalog (Delay 1)

##### Tier A: 71 Daily Pre-Aggregated MATRIX Fields
These fields represent pre-computed daily arithmetic means or entity scores. They evaluate directly within standard matrix operations:

| Field Identifier | Tensor Type | Coverage | Range | Aggregation Semantic |
|------------------|-------------|----------|-------|----------------------|
| `mean_composite_sentiment_score` | `MATRIX` | 1.0000 | $[-1.0, +1.0]$ | Daily average sentiment across all published stories |
| `mean_earnings_evaluation_sentiment` | `MATRIX` | 1.0000 | $[-1.0, +1.0]$ | Daily average sentiment on earnings releases & guidance |
| `mean_corporate_action_sentiment` | `MATRIX` | 1.0000 | $[-1.0, +1.0]$ | Daily average sentiment on dividends, buybacks, M&A |
| `mean_merger_acquisition_sentiment` | `MATRIX` | 1.0000 | $[-1.0, +1.0]$ | Daily average sentiment on acquisitions & takeovers |
| `mean_event_novelty_score` | `MATRIX` | 1.0000 | $[0, 100]$ | Daily average novelty (freshness vs reprints) |
| `mean_entity_relevance_score` | `MATRIX` | 1.0000 | $[0, 100]$ | Centrality of company in the reported narrative |
| `mean_news_impact_projection` | `MATRIX` | 1.0000 | $\mathbb{R}^+$ | Model-projected price volatility impact magnitude |
| `mean_editorial_commentary_sentiment` | `MATRIX` | 1.0000 | $[-1.0, +1.0]$ | Sentiment extracted from opinion pieces & columns |
| `rp_css_earnings` | `MATRIX` | 0.5000 | $[-1.0, +1.0]$ | Topic-filtered earnings sentiment |
| `rp_css_dividends` | `MATRIX` | 0.5000 | $[-1.0, +1.0]$ | Topic-filtered dividend action sentiment |
| `rp_css_credit` | `MATRIX` | 0.5000 | $[-1.0, +1.0]$ | Topic-filtered debt and credit risk sentiment |
| `rp_css_insider` | `MATRIX` | 0.5000 | $[-1.0, +1.0]$ | Topic-filtered executive and insider transaction sentiment |

##### Tier B: 50 Intraday VECTOR Event Fields
These fields represent variable-length 1D arrays of event records for equity $i$ on date $t$. **They MUST be collapsed via vector folding operators before any downstream computation**:

| Field Identifier | Tensor Type | Coverage | Required Folding Operator | Description |
|------------------|-------------|----------|---------------------------|-------------|
| `composite_sentiment_score_2` | `VECTOR` | 1.0000 | `vec_avg(...)` | Granular story-level composite sentiment |
| `earnings_evaluation_sentiment` | `VECTOR` | 1.0000 | `vec_avg(...)` | Story-level earnings sentiment polarity |
| `event_novelty_score_2` | `VECTOR` | 1.0000 | `vec_avg(...)` | Story-level novelty index |
| `news_impact_projection_score` | `VECTOR` | 1.0000 | `vec_avg(...)` | Story-level projected volatility impact |
| `nws18_acb` | `VECTOR` | 0.9030 | `vec_avg(...)` | Corporate action announcement score |
| `nws18_bam` | `VECTOR` | 1.0000 | `vec_avg(...)` | M&A indicator score ($-1, 0, +1$) |
| `nws18_bee` | `VECTOR` | 1.0000 | `vec_avg(...)` | Earnings evaluation score ($-1, 0, +1$) |
| `nws18_ber` | `VECTOR` | 1.0000 | `vec_sum(...)` | Earnings release binary occurrence flag |
| `nws18_nip` | `VECTOR` | 0.9030 | `vec_avg(...)` | Narrative impact projection score |
| `nws18_ssc` | `VECTOR` | 1.0000 | `vec_avg(...)` | Continuous story sentiment $[-1.0, +1.0]$ |
| `story_event_record_count` | `VECTOR` | 0.9030 | `vec_sum(...)` | Story event record counter (event volume) |

---

### 2.4 Dataset `option9`: Options Analytics Surface

#### 2.4.1 Overview & Surface Parameterization
- **Dataset ID**: `option9`
- **Category / Subcategory**: `option` / `option-option-analytics`
- **Primary Asset Class**: Optionable US Equities (>2,500 liquid instruments)
- **Supported Delays**: Delay 1
- **Total Fields**: Exactly **74 fields**, all of type `MATRIX`.
- **Cross-Sectional Coverage**: $C_{\text{cross}} \in [0.9795, 0.9838]$ (High across all market cap tiers).
- **Date Coverage**: $C_{\text{date}} = 1.0$ (Full history).
- **Constant Expiration Horizons**:
  $$\mathcal{T} = \{10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080\} \text{ calendar days}$$

#### 2.4.2 The 6 Factor Families (74 Fields Exact Decomposition)
The 74 fields decompose across 6 canonical families:
1. **Put-Call Volume Ratio (`pcr_vol_`)**: 12 constant horizons in $\mathcal{T}$ plus aggregate `pcr_vol_all` $\implies$ **13 fields**.
2. **Put-Call Open Interest Ratio (`pcr_oi_`)**: 12 constant horizons in $\mathcal{T}$ plus aggregate `pcr_oi_all` $\implies$ **13 fields**.
3. **Synthetic Forward Price (`forward_price_`)**: 12 constant horizons in $\mathcal{T}$ $\implies$ **12 fields**.
4. **Call Breakeven Price (`call_breakeven_`)**: 12 constant horizons in $\mathcal{T}$ $\implies$ **12 fields**.
5. **Put Breakeven Price (`put_breakeven_`)**: 12 constant horizons in $\mathcal{T}$ $\implies$ **12 fields**.
6. **Total Option Breakeven Price (`option_breakeven_`)**: 12 constant horizons in $\mathcal{T}$ $\implies$ **12 fields**.

$$\text{Total Fields} = 13 + 13 + 12 + 12 + 12 + 12 \equiv 74$$

#### 2.4.3 Critical Syntax Invariants for `option9`
1. **Integer Horizon Suffix Invariant**:
   Field names must end with the raw integer horizon $T$, **strictly without the `'d'` suffix**:
   - Valid: `pcr_vol_20`, `pcr_oi_30`, `forward_price_60`, `call_breakeven_90`.
   - Invalid (Fatal Error): `pcr_vol_20d`, `pcr_oi_30d`, `call_breakeven_90d`.
2. **Horizon Membership Invariant**:
   Any horizon not belonging to $\mathcal{T} \cup \{\text{"all"}\}$ (e.g. `pcr_vol_15`, `forward_price_45`, `call_breakeven_250`) does not exist on BRAIN and triggers an HTTP 400 Bad Request.
3. **Price Level Normalization Invariant**:
   The fields `forward_price_<T>`, `call_breakeven_<T>`, `put_breakeven_<T>`, and `option_breakeven_<T>` represent raw dollar price levels.
   - Passing raw price levels into `rank()` or `group_neutralize()` creates an unintended high-price stock bias.
   - **Validation Requirement**: The validator emits a `Severity::Warning` if a breakeven or forward price field is passed into `rank(...)` or `group_neutralize(...)` without first being normalized by `close` (e.g. `call_breakeven_30 / close - 1`).

---

## 3. Formal FastExpr Type System & Type Lattice

FastExpr operates with dynamic type dispatch on the BRAIN execution engine. To guarantee compile-time safety and eliminate runtime simulation failures, we define a formal static type system for FastExpr.

### 3.1 Formal Type Universe
Let the type universe $\mathbb{T}$ be defined as:
$$\mathbb{T} = \{ \text{Scalar}, \text{Matrix}, \text{Vector}, \text{Group}, \text{DensifiedGroup}, \text{BooleanMatrix}, \bot \}$$

Where:
- $\text{Scalar}$: A 0D scalar constant ($c \in \mathbb{R}$, e.g. `0.5`, `1.5`, `10`).
- $\text{Matrix}$: A 2D cross-sectional time-series tensor $[T \times N]$ where each cell contains a real scalar $x_{i, t} \in \mathbb{R} \cup \{\text{NaN}\}$.
- $\text{Vector}$: A 3D ragged tensor $[T \times N \times K_{i, t}]$ where each equity-date cell contains a 1D sequence of event attributes.
- $\text{Group}$: A 2D categorical tensor of sparse, non-contiguous integer IDs (e.g. `subindustry`, `industry`, `sector`).
- $\text{DensifiedGroup}$: A 2D categorical tensor mapped to contiguous compact integer indices $[0, G-1]$ via `densify()`.
- $\text{BooleanMatrix}$: A 2D matrix where values are strictly $\{0.0, 1.0\}$.
- $\bot$ (Bottom): Type error state representing illegal expressions or incompatible operations.

### 3.2 Operator Signature Taxonomy

```
+---------------------------------------------------------------------------------------------------------+
|                                  OPERATOR TYPE SIGNATURE MATRIX                                         |
+----------------------+--------------------------------------------------+-------------------------------+
| Operator Class       | Signature Rule                                   | Return Type                   |
+----------------------+--------------------------------------------------+-------------------------------+
| Vector Folding       | vec_avg(Vector), vec_sum(Vector)                 | Matrix                        |
| Binary Arithmetic    | Matrix ⊗ Matrix, Matrix ⊗ Scalar, Scalar ⊗ Matrix | Matrix                        |
| Unary Arithmetic     | abs(Matrix), sign(Matrix), log(Matrix), etc.     | Matrix                        |
| Power Transform      | signed_power(Matrix, Scalar)                     | Matrix                        |
| Cross-Sectional      | rank(Matrix), zscore(Matrix), winsorize(Matrix)  | Matrix                        |
| Time-Series          | ts_*(Matrix, Scalar (lookback d >= 1))           | Matrix                        |
| Group Densify        | densify(Group)                                   | DensifiedGroup                |
| Group Neutralization | group_neutralize(Matrix, DensifiedGroup)         | Matrix                        |
| Logical Ternary      | if_else(BooleanMatrix, Matrix, Matrix)           | Matrix                        |
| NaN Guard            | is_nan(Matrix)                                   | BooleanMatrix                 |
+----------------------+--------------------------------------------------+-------------------------------+
```

### 3.3 Vector Folding Mechanics & Syntax Rules

#### Rule 3.3.1: Strict Vector Incompatibility
A `Vector` tensor cannot participate in any binary arithmetic, unary transform, cross-sectional ranking, or time-series windowing:
$$\forall \tau \in \mathbb{T} \setminus \{\emptyset\}, \quad \text{Vector} + \tau \implies \bot$$
$$\text{rank}(\text{Vector}) \implies \bot$$
$$\text{ts\_delta}(\text{Vector}, 5) \implies \bot$$
$$\text{group\_neutralize}(\text{Vector}, \text{DensifiedGroup}) \implies \bot$$

#### Rule 3.3.2: Folding Morphisms
The only legal operations on a `Vector` tensor are the vector folding operators:
$$\text{vec\_avg}: \text{Vector} \longrightarrow \text{Matrix}$$
$$\text{vec\_sum}: \text{Vector} \longrightarrow \text{Matrix}$$

- **Semantic Choice**:
  - `vec_avg`: Computes $\frac{1}{K_{i, t}} \sum_{k=1}^{K_{i, t}} v_{i, t, k}$. Appropriate for intensive properties (sentiment score, novelty, relevance, impact).
  - `vec_sum`: Computes $\sum_{k=1}^{K_{i, t}} v_{i, t, k}$. Appropriate for extensive properties (story counts, event flags like `nws18_ber`, total records).

#### Rule 3.3.3: Illegal Redundant Folding
Applying `vec_avg` or `vec_sum` to a `Matrix` or `Scalar` is a type violation:
$$\text{vec\_avg}(\text{Matrix}) \implies \bot \quad (\text{InvalidVectorFolding})$$
$$\text{vec\_sum}(\text{Scalar}) \implies \bot \quad (\text{InvalidVectorFolding})$$

---

### 3.4 Group Neutralization & Densification Rules

#### Rule 3.4.1: The Group Densification Invariant
WorldQuant BRAIN requires that grouping variables in `group_neutralize`, `group_rank`, `group_mean`, `group_scale`, `group_zscore`, and `group_backfill` be densified:
$$\text{group\_neutralize}: (\text{Matrix}, \text{DensifiedGroup}) \longrightarrow \text{Matrix}$$
If the second argument is a raw `Group` without `densify(...)`:
$$\text{group\_neutralize}(\text{Matrix}, \text{Group}) \implies \text{Error}(\text{UndensifiedGroupException})$$

#### Rule 3.4.2: False-Positive Comment & Substring Immunity
As discovered during M1 adversarial fuzzing, naive substring search (`expr.contains("densify(")`) can be evaded if `"densify("` appears inside comments or unrelated variable names.
The Schema Validator AST must explicitly inspect the **second argument node** of the `group_neutralize` AST function call and verify that the callee of the argument node is `densify`.

---

## 4. Coverage Thresholds & Missing Data Imputation Theory

### 4.1 Coverage Metrics Definitions
For any data field $F$, the platform API reports two primary coverage indices:
1. **Cross-Sectional Coverage ($C_{\text{cross}}(F, U)$)**:
   $$C_{\text{cross}}(F, U) = \frac{1}{T} \sum_{t=1}^T \frac{\sum_{i \in U_t} \mathbb{I}(F_{i, t} \ne \text{NaN})}{|U_t|}$$
   Measures the average fraction of universe instruments having non-null data on any given trading day.
2. **Date Coverage ($C_{\text{date}}(F)$)**:
   $$C_{\text{date}}(F) = \frac{\text{Valid Trading Days with Field Data}}{\text{Total Historical Trading Days in Simulation Sample}}$$
   Measures historical temporal depth.

---

### 4.2 Coverage Threshold Policy

```
+------------------------------------------------------------------------------------------------------------------------+
|                                    COVERAGE THRESHOLD ACTION MATRIX                                                    |
+--------------------------+-----------------------+-----------------------------+---------------------------------------+
| Metric                   | Value Range           | Validator Classification    | Mandatory AST Requirement             |
+--------------------------+-----------------------+-----------------------------+---------------------------------------+
| Cross-Sectional Coverage | C_cross >= 0.90       | High Coverage               | Direct usage permitted                |
| Cross-Sectional Coverage | 0.50 <= C_cross < 0.90| Moderate Coverage           | ts_backfill recommended (Warning)     |
| Cross-Sectional Coverage | C_cross < 0.50        | Critically Sparse Coverage  | ts_backfill MANDATORY (Fatal Error)   |
| Date Coverage            | C_date >= 0.90        | Full Historical Depth       | Direct usage permitted                |
| Date Coverage            | C_date < 0.90         | Truncated History           | Date Truncation Warning emitted       |
+--------------------------+-----------------------+-----------------------------+---------------------------------------+
```

---

### 4.3 Ancestral AST Path Tracing for Backfill Enforcement

When an expression incorporates a sparse field ($C_{\text{cross}} < 0.50$, such as `fscore_growth` or `rp_css_earnings`), the validator executes an **Ancestral AST Path Trace**:

```
      [group_neutralize]
             │
      [signed_power]
             │
          [rank]
             │
      [ts_backfill] <── Mandatory Ancestor Node (lookback d in [5, 60])
             │
     [fscore_growth] <── Sparse Field Node (C_cross = 0.2987 < 0.50)
```

#### Formal Verification Condition:
Let $\text{Ancestors}(N)$ be the set of AST nodes on the path from the root expression down to node $N$.
For any field token $F$ with $C_{\text{cross}}(F) < 0.50$:
$$\exists A \in \text{Ancestors}(F) \quad \text{such that} \quad A = \text{CallNode}(\text{"ts\_backfill"}, [x, d]) \quad \land \quad d \ge 5$$
If this condition evaluates to false, the validator rejects the expression with:
`Error::MissingMandatoryBackfill(field_id, coverage, required_min_lookback=5)`.

---

### 4.4 NaN Imputation Patterns for Sparse Datasets

Even after applying `ts_backfill(x, 15)`, event-driven fields (`news18`) or newly listed stocks may evaluate to `NaN` during initial warmup intervals.
If simulation execution settings enforce `nanHandling: "OFF"`, any instrument with a final `NaN` weight will cause position cancellation or simulation failure.

The validator verifies that expressions based on event-driven fields incorporate an explicit ternary NaN guard:
```fastexpr
clean_signal = if_else(is_nan(raw_sig), 0.0, raw_sig);
```
Or rank centering with fallback:
```fastexpr
sig = if_else(is_nan(raw_sig), 0.0, rank(raw_sig) - 0.5);
```

---

## 5. Temporal Integrity & Look-Ahead Bias Prevention

### 5.1 Delay Alignment Theory
WorldQuant BRAIN models trade execution under discrete delay conventions:
- **`delay = 0`**: Alphas trade at day $t$ Open/VWAP based on information available at or before market open on day $t$.
- **`delay = 1`**: Alphas calculate target positions after day $t$ Close and trade on day $t+1$.

#### Delay Compatibility Invariant
Each field $F$ in `/data-fields` has a certified dissemination delay:
1. If `settings.delay == 1`:
   - Delay 1 fields are **valid**.
   - Delay 0 fields are **valid** (since information available at $t$ is strictly available at $t+1$).
2. If `settings.delay == 0`:
   - Delay 1 fields are **STRICTLY FORBIDDEN**.
   - Using a Delay 1-only field in a `delay = 0` simulation constitutes severe look-ahead bias and triggers an immediate `Error::LookAheadDelayMismatch`.

---

### 5.2 Time-Series Causal Directionality Invariants

All time-series operators in FastExpr take a window or lag parameter $d$.
$$\text{ts\_delay}(x, d), \quad \text{ts\_delta}(x, d), \quad \text{ts\_mean}(x, d), \quad \text{ts\_decay\_linear}(x, d)$$

#### Formal Invariant: Positive Causal Lookback
$$\forall \text{CallNode}(\text{"ts\_*"}, [\dots, d]), \quad d \in \mathbb{Z}^+ \quad (d \ge 1)$$

- If $d \le 0$: A negative lag ($d < 0$) references future timestamps ($t + |d|$), representing an illegal non-causal look-ahead.
- The validator enforces that the lookback argument is a literal integer or scalar resolving to $d \ge 1$.

---

### 5.3 Asynchronous Corporate Filing & Earnings Reporting Lags
Fundamental scores (`model16`) reflect corporate financial reports. Under Delay 1, BRAIN guarantees point-in-time publication timestamps (SEC 10-Q/10-K filings and press releases).
However, point-in-time updates occur in step-function jumps. Unfiltered fundamental derivatives produce spikes in annualized turnover ($> 70\%$).
- The validator checks that fundamental signals incorporate turnover smoothing:
  - Inside formula: `ts_decay_linear(..., d)` where $d \in [10, 25]$, or
  - Simulation settings: `settings.decay >= 16`.

---

### 5.4 Event-Driven Trading Logic (`trade_when`) Causal Safety
FastExpr's `trade_when(trigger, alpha, exit)` operator provides event-driven position updating.
- The `trigger` condition must be a causal event indicator (e.g. `nws18_ber > 0` or `ts_delta(close, 1) > 0.05`).
- The validator verifies that `exit` is either a valid causal boolean matrix or `-1` (disabled). Passing forward-looking filters into `exit` is rejected.

---

## 6. Schema Validator Engine Architectural Design

```
+───────────────────────────────────────────────────────────────────────────────────+
|                          SCHEMA VALIDATOR ENGINE PIPELINE                         |
+───────────────────────────────────────────────────────────────────────────────────+
                                          │
                                 [FastExpr String]
                                          │
                                          ▼
                                   [1. LEXER / TOKENIZER]
                                          │ (Tokens: Ident, Kw, Op, Lit)
                                          ▼
                                    [2. AST PARSER]
                                          │ (Multi-statement AST nodes)
                                          ▼
                              [3. FIELD CATALOG RESOLVER]
                                          │ (Metadata: type, cov, delay)
                                          ▼
                                  [4. TYPE INFERENCE ENGINE]
                                          │ (Lattice type propagation)
                                          ▼
                           [5. POLICY & INTEGRITY VALIDATOR]
                                  ├─ Type Compatibility (Vector vs Matrix)
                                  ├─ Coverage & ts_backfill Enforcement
                                  ├─ Delay & Look-Ahead Verification
                                  └─ Group Densify Compliance
                                          │
                                          ▼
                            [6. VALIDATION REPORT / DIAGNOSTICS]
                                  ├─ IsValid: bool
                                  ├─ Errors: Vec<ValidationError>
                                  └─ Warnings: Vec<ValidationWarning>
```

---

### 6.1 Data Structures in Rust (`src/models.rs` & `src/schema_validator.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tensor Type reported by BRAIN /data-fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FieldTensorType {
    Matrix,
    Vector,
}

/// Category and subcategory descriptor
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryInfo {
    pub id: String,
    pub name: String,
}

/// Enriched DataFieldEntry matching BRAIN API /data-fields schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFieldMetadata {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub dataset: CategoryInfo,
    #[serde(default)]
    pub category: Option<CategoryInfo>,
    #[serde(default)]
    pub subcategory: Option<CategoryInfo>,
    #[serde(default = "default_region")]
    pub region: String,
    pub delay: i32,
    #[serde(default = "default_universe")]
    pub universe: String,
    #[serde(rename = "type")]
    pub tensor_type: FieldTensorType,
    pub coverage: f64,
    pub date_coverage: f64,
    #[serde(default)]
    pub user_count: Option<i64>,
    #[serde(default)]
    pub alpha_count: Option<i64>,
}

fn default_region() -> String { "USA".to_string() }
fn default_universe() -> String { "TOP1000".to_string() }

/// Local cache catalog storing metadata for an entire dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetFieldCatalog {
    pub dataset_id: String,
    pub fetched_at: String,
    pub count: usize,
    pub fields: HashMap<String, DataFieldMetadata>,
}
```

---

### 6.2 Type Lattice & AST Nodes

```rust
/// Internal Type Lattice for FastExpr AST evaluation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprType {
    Scalar,
    Matrix,
    Vector,
    Group,
    DensifiedGroup,
    BooleanMatrix,
    Unknown,
    Error(String),
}

/// Abstract Syntax Tree Node for FastExpr
#[derive(Debug, Clone)]
pub enum AstNode {
    LiteralScalar(f64),
    Identifier(String),
    Assignment {
        var_name: String,
        value: Box<AstNode>,
    },
    FunctionCall {
        func_name: String,
        args: Vec<AstNode>,
    },
    BinaryOp {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryOp {
        op: String,
        operand: Box<AstNode>,
    },
    Block(Vec<AstNode>),
}
```

---

### 6.3 Validation Diagnostics & Error Taxonomy

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Fatal,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct ValidationDiagnostic {
    pub severity: ValidationSeverity,
    pub code: &'static str,
    pub message: String,
    pub token: Option<String>,
    pub recommendation: Option<String>,
}

pub struct ValidationReport {
    pub is_valid: bool,
    pub diagnostics: Vec<ValidationDiagnostic>,
}
```

#### Diagnostic Error Taxonomy:
1. `ERR_VEC_TYPE_MISMATCH`: A `VECTOR` field was used directly in arithmetic or non-folding operator.
   - *Message*: `Field '<field>' is of type VECTOR and cannot be passed to '<operator>'. Wrap in vec_avg(...) or vec_sum(...) first.`
2. `ERR_MISSING_BACKFILL`: A field with $C_{\text{cross}} < 0.50$ is not enclosed in `ts_backfill(..., d)` with $d \ge 5$.
   - *Message*: `Field '<field>' has low cross-sectional coverage (<cov>). Mandatory ts_backfill(..., d) is missing.`
3. `ERR_LOOKAHEAD_DELAY`: Field requires `delay = 1` but simulation settings specify `delay = 0`.
   - *Message*: `Field '<field>' is Delay 1-only and cannot be simulated under delay=0.`
4. `ERR_NEGATIVE_LOOKBACK`: Time-series lookback parameter $d \le 0$.
   - *Message*: `Operator '<op>' received invalid lookback d=<d>. Lookback must be an integer >= 1 to prevent non-causal look-ahead bias.`
5. `ERR_UNDENSIFIED_GROUP`: `group_neutralize` second argument lacks `densify(...)`.
   - *Message*: `group_neutralize grouping variable must be wrapped in densify(...) to prevent simulation gateway timeout.`
6. `ERR_UNKNOWN_FIELD`: Field identifier does not exist in the platform metadata catalog.
   - *Message*: `Field '<field>' not recognized in dataset '<dataset>' catalog.`
7. `WARN_HORIZON_SYNTAX`: Option9 field contains trailing `'d'` (e.g. `pcr_vol_20d`).
   - *Message*: `Field '<field>' invalid. option9 horizons must be exact integers without trailing 'd' (e.g., pcr_vol_20).`
8. `WARN_SUBUNIVERSE_RISK`: Low-coverage field ($C_{\text{cross}} < 0.40$) used in broad universe without rank derivative anchor.
   - *Message*: `Field '<field>' has ~<cov>% coverage. High risk of failing subuniverse consistency test. Prefer Tier 1 rank derivatives.`
9. `WARN_UNNORMALIZED_PRICE`: Raw dollar breakeven or forward price field passed into `rank()` without `close` normalization.
   - *Message*: `Breakeven field '<field>' should be normalized by 'close' (e.g. '<field> / close - 1') before ranking.`

---

## 7. Concrete Verification Scenarios & Test Suite Specifications

### Scenario 7.1: Valid Multi-Pillar Alternative Expression (Expected: Pass)
```fastexpr
// model51 anchor + news18 folded catalyst + option9 breakeven asymmetry
anchor_beta = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 15));
news_sent = ts_backfill(vec_avg(composite_sentiment_score_2), 15);
clean_news = if_else(is_nan(news_sent), 0.0, rank(news_sent) - 0.5);
call_be = ts_backfill(call_breakeven_30 / close - 1.0, 10);
put_be = ts_backfill(1.0 - put_breakeven_30 / close, 10);
opt_asym = rank(put_be - call_be) - 0.5;
composite = 0.50 * anchor_beta + 0.25 * clean_news + 0.25 * opt_asym;
rev_damp = 0.06 * (rank(ts_delta(close, 3)) - 0.5);
final_alpha = signed_power(composite, 1.5) - rev_damp;
group_neutralize(signed_power(final_alpha, 4.0), densify(subindustry))
```
- **Validator Evaluation**:
  - `beta_last_60_days_spy`: MATRIX, coverage $0.9752$, Delay 1. Pass.
  - `composite_sentiment_score_2`: VECTOR, wrapped in `vec_avg(...)`, backfilled ($15 \ge 5$). Pass.
  - `call_breakeven_30`, `put_breakeven_30`: MATRIX, exact integer horizon $30 \in \mathcal{T}$, normalized by `close`, backfilled. Pass.
  - `group_neutralize`: Second argument is `densify(subindustry)`. Pass.
  - Result: **VALID (0 Errors, 0 Warnings)**.

---

### Scenario 7.2: Vector Type Mismatch (Expected: Fatal Error)
```fastexpr
// Illegal direct arithmetic on VECTOR nws18_ssc
raw_signal = nws18_ssc + 1.0;
group_neutralize(rank(raw_signal), densify(subindustry))
```
- **Validator Evaluation**:
  - Identifier `nws18_ssc` resolves to `FieldTensorType::Vector`.
  - Operator `+` applied between `Vector` and `LiteralScalar(1.0)`.
  - Diagnostic: `ERR_VEC_TYPE_MISMATCH` (Fatal).
  - Result: **INVALID (Rejected before simulation)**.

---

### Scenario 7.3: Missing Backfill on Low-Coverage Pentagon (Expected: Fatal Error)
```fastexpr
// model16 raw pentagon fscore_growth has coverage 0.2987 (< 0.50) without ts_backfill
sig = rank(fscore_growth) - 0.5;
group_neutralize(signed_power(sig, 3.5), densify(subindustry))
```
- **Validator Evaluation**:
  - `fscore_growth`: MATRIX, coverage $0.2987 < 0.50$.
  - Ancestor traversal: `[rank, signed_power, group_neutralize]`. No `ts_backfill` found.
  - Diagnostic: `ERR_MISSING_BACKFILL` (Fatal) + `WARN_SUBUNIVERSE_RISK` (Warning).
  - Result: **INVALID (Rejected before simulation)**.

---

### Scenario 7.4: Look-Ahead Lag Parameter (Expected: Fatal Error)
```fastexpr
// Negative lookback in ts_delay references future data
sig = ts_delay(close, -5);
group_neutralize(rank(sig), densify(subindustry))
```
- **Validator Evaluation**:
  - `ts_delay` argument $d = -5 < 1$.
  - Diagnostic: `ERR_NEGATIVE_LOOKBACK` (Fatal).
  - Result: **INVALID (Rejected before simulation)**.

---

### Scenario 7.5: Option9 Trailing 'd' Syntax Error (Expected: Fatal Error / Warning)
```fastexpr
// Illegal trailing 'd' on option9 horizon
flow = rank(pcr_vol_20d) - 0.5;
group_neutralize(flow, densify(subindustry))
```
- **Validator Evaluation**:
  - Identifier `pcr_vol_20d` does not exist in `option9` field catalog.
  - Matcher detects pattern `pcr_vol_<T>d` and identifies illegal trailing `'d'`.
  - Diagnostic: `ERR_UNKNOWN_FIELD` (Fatal) with recommendation: `Did you mean 'pcr_vol_20'?`.
  - Result: **INVALID (Rejected before simulation)**.

---

## 8. Summary of Theoretical Invariants for Worker Implementation

| Component | Target File | Core Invariant to Implement |
|-----------|-------------|-----------------------------|
| Field Metadata Model | `src/models.rs` | Add `FieldTensorType { Matrix, Vector }` and enrich `DataFieldEntry`. |
| Field Catalog Cache | `src/schema_validator.rs` | Local JSON loader for `data/fields_{dataset}.json` with fail-closed schema checks. |
| FastExpr AST Parser | `src/schema_validator.rs` | Recursive descent or Pratt parser extracting function calls and identifiers. |
| Type Checker | `src/schema_validator.rs` | Enforce `Vector` $\to$ `vec_avg`/`vec_sum` folding before scalar/matrix arithmetic. |
| Backfill Gatekeeper | `src/schema_validator.rs` | Require ancestral `ts_backfill(..., d >= 5)` for any field with $C_{\text{cross}} < 0.50$. |
| Delay Gatekeeper | `src/schema_validator.rs` | Verify Delay 1 fields only execute under `settings.delay == 1`. |
| Causal Guard | `src/schema_validator.rs` | Require positive lookback $d \ge 1$ for all `ts_*` time-series operators. |
| Densify Enforcer | `src/schema_validator.rs` | Verify second argument of `group_neutralize` is strictly `densify(group)`. |

This formal theory provides complete mathematical and computational specifications for Milestone M2 and establishes a zero-error simulation pipeline for Milestone M3 and M4.
