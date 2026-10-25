# WorldQuant BRAIN Alternative Datasets & FastExpr Specification Survey

**Document Version**: 2.0.0 (Authoritative Platform Mine)  
**Verification Date**: 2026-10-24  
**Authoritative Sources**: WorldQuant BRAIN API (`https://api.worldquantbrain.com`), BRAIN Learn Operator Catalog, Dataset Inventory (`/data-sets`), Live Field Metadata (`/data-fields`).

---

## Executive Summary

This specification report provides comprehensive, empirically verified domain requirements and technical specifications for WorldQuant BRAIN's proprietary Alternative Data ecosystem (`model51`, `model16`, `news18`, `option9`) and the `FastExpr` grammar. All schemas, field lists, operator signatures, coverage rates, and syntax behaviors have been mined directly from signed-in API endpoints and validated against the platform's execution engine.

---

## 1. Alternative Datasets Deep-Dive Specification

WorldQuant BRAIN hosts multiple proprietary and third-party alternative datasets. The 4 target datasets represent orthogonal sources of alpha:

```
+---------------------------------------------------------------------------------------------------+
|                                  ALTERNATIVE DATASET TAXONOMY                                      |
+---------------+------------------------+------------+------------+---------------+----------------+
| Dataset ID    | Name                   | Category   | Subcategory| Matrix Fields | Vector Fields  |
+---------------+------------------------+------------+------------+---------------+----------------+
| model51       | Systematic Risk        | Model      | Risk       | 16            | 0              |
| model16       | Fundamental Scores     | Model      | Valuation  | 24            | 0              |
| news18        | RavenPack News Data    | News       | Sentiment  | 71            | 50             |
| option9       | Options Analytics      | Option     | Analytics  | 74            | 0              |
+---------------+------------------------+------------+------------+---------------+----------------+
```

---

### 1.1 Dataset `model51`: Systematic Risk Metrics

- **Platform ID**: `model51`
- **Name**: Systematic Risk Metrics
- **Category**: `model` (Model) | **Subcategory**: `model-risk-models` (Risk Models)
- **Primary Instrument**: Optionable US Equities (TOP200, TOP500, TOP1000, TOP2000, TOP3000, TOPSP500)
- **Delay**: Delay 1 (and Delay 0 where supported)
- **Cross-Sectional Coverage**: 93.70% - 97.65% (Mean: 96.26% across TOP1000)
- **Date Coverage**: 1.0 (Full historical coverage from 2022 to present)
- **Total Fields**: 16 fields (100% MATRIX type)
- **Data Structure**: Daily cross-sectional matrices indexed by `[Date, Instrument]`
- **Benchmark Reference**: SPY (SPDR S&P 500 ETF Trust)
- **Time Horizons**: 30, 60, 90, 360 calendar days

#### Complete Field Inventory for `model51`

| # | Field ID | Type | Coverage | Units / Scale | Platform Description |
|---|----------|------|----------|---------------|----------------------|
| 1 | `beta_last_30_days_spy` | MATRIX | 0.9765 | Ratio ($\beta$) | Rolling beta relative to SPY via regression over last 30 calendar days |
| 2 | `beta_last_60_days_spy` | MATRIX | 0.9752 | Ratio ($\beta$) | Rolling beta relative to SPY via regression over last 60 calendar days |
| 3 | `beta_last_90_days_spy` | MATRIX | 0.9734 | Ratio ($\beta$) | Rolling beta relative to SPY via regression over last 90 calendar days |
| 4 | `beta_last_360_days_spy` | MATRIX | 0.9482 | Ratio ($\beta$) | Rolling beta relative to SPY via regression over last 360 calendar days |
| 5 | `correlation_last_30_days_spy` | MATRIX | 0.9765 | $[-1, +1]$ | Pearson correlation of daily log returns vs SPY over 30 calendar days |
| 6 | `correlation_last_60_days_spy` | MATRIX | 0.9752 | $[-1, +1]$ | Pearson correlation of daily log returns vs SPY over 60 calendar days |
| 7 | `correlation_last_90_days_spy` | MATRIX | 0.9734 | $[-1, +1]$ | Pearson correlation of daily log returns vs SPY over 90 calendar days |
| 8 | `correlation_last_360_days_spy` | MATRIX | 0.9482 | $[-1, +1]$ | Pearson correlation of daily log returns vs SPY over 360 calendar days |
| 9 | `systematic_risk_last_30_days` | MATRIX | 0.9650 | $[0, 1]$ ($R^2$) | Portion of variance explained by SPY over 30 calendar days |
| 10 | `systematic_risk_last_60_days` | MATRIX | 0.9637 | $[0, 1]$ ($R^2$) | Portion of variance explained by SPY over 60 calendar days |
| 11 | `systematic_risk_last_90_days` | MATRIX | 0.9619 | $[0, 1]$ ($R^2$) | Portion of variance explained by SPY over 90 calendar days |
| 12 | `systematic_risk_last_360_days` | MATRIX | 0.9370 | $[0, 1]$ ($R^2$) | Portion of variance explained by SPY over 360 calendar days |
| 13 | `unsystematic_risk_last_30_days` | MATRIX | 0.9650 | $[0, 1]$ ($1 - R^2$) | Portion of variance not explained by SPY (idiosyncratic risk) over 30 days |
| 14 | `unsystematic_risk_last_60_days` | MATRIX | 0.9637 | $[0, 1]$ ($1 - R^2$) | Portion of variance not explained by SPY (idiosyncratic risk) over 60 days |
| 15 | `unsystematic_risk_last_90_days` | MATRIX | 0.9619 | $[0, 1]$ ($1 - R^2$) | Portion of variance not explained by SPY (idiosyncratic risk) over 90 days |
| 16 | `unsystematic_risk_last_360_days` | MATRIX | 0.9370 | $[0, 1]$ ($1 - R^2$) | Portion of variance not explained by SPY (idiosyncratic risk) over 360 days |

#### Mathematical Identities & Relationships
1. **Variance Decomposition**:
   $$\text{systematic\_risk\_last\_Xd} + \text{unsystematic\_risk\_last\_Xd} \equiv 1.0$$
2. **Correlation vs Systematic Variance**:
   $$\text{systematic\_risk\_last\_Xd} \equiv (\text{correlation\_last\_Xd\_spy})^2$$
3. **Beta Relationship**:
   $$\beta_i = \rho_{i, \text{SPY}} \cdot \frac{\sigma_i}{\sigma_{\text{SPY}}}$$

#### Missing Data & FastExpr Handling
- **Missingness Pattern**: Illiquid small-caps in `TOP3000` or newly IPO'd instruments have `NaN` for horizons exceeding their history.
- **Handling Strategy**:
  - `ts_backfill(unsystematic_risk_last_30_days, 10)` to prevent transient trading day gaps.
  - Zero-imputation after ranking: `if_else(is_nan(sig), 0, sig)`.

---

### 1.2 Dataset `model16`: Fundamental Scores & Derivative Acceleration

- **Platform ID**: `model16`
- **Name**: Fundamental Scores
- **Category**: `model` (Model) | **Subcategory**: `model-valuation-models` (Valuation Models)
- **Primary Instrument**: US Equities (TOP200 through TOP3000)
- **Delay**: Delay 1
- **Cross-Sectional Coverage**: Dual-tiered:
  - Standardized Derivative Ranks (`*_derivative`): **100.0% coverage** (`1.0`)
  - Raw / Backfilled Pentagon Scores (`fscore_*` / `fscore_bfl_*`): **29.46% - 38.28% coverage**
- **Total Fields**: 24 fields (100% MATRIX type)
- **Key Academic Anchor**: Research Paper 01 ("The Momentum of News") and Piotroski / Multi-factor Style Pentagons

#### Complete Field Inventory for `model16`

| # | Field ID | Type | Coverage | Category Tier | Platform Description |
|---|----------|------|----------|---------------|----------------------|
| 1 | `analyst_revision_rank_derivative` | MATRIX | 1.0000 | Rank Derivative | Daily-indicator variant of momentum capturing recent price moves and analyst estimate revisions |
| 2 | `cashflow_efficiency_rank_derivative` | MATRIX | 1.0000 | Rank Derivative | Ranks stocks by their ability to generate cash flows and operational profitability |
| 3 | `composite_factor_score_derivative` | MATRIX | 1.0000 | Rank Derivative | Momentum score based on analyst revisions; intraday variant |
| 4 | `earnings_certainty_rank_derivative` | MATRIX | 1.0000 | Rank Derivative | Measures the sustainability and certainty of earnings quality |
| 5 | `growth_potential_rank_derivative` | MATRIX | 1.0000 | Rank Derivative | Composite growth score qualifying the stock’s expected medium-term growth potential |
| 6 | `multi_factor_acceleration_score_derivative` | MATRIX | 1.0000 | Rank Derivative | Change in the acceleration of multi-factor score compared to previous period |
| 7 | `multi_factor_static_score_derivative` | MATRIX | 1.0000 | Rank Derivative | Change in static multi-factor score compared to previous period |
| 8 | `relative_valuation_rank_derivative` | MATRIX | 1.0000 | Rank Derivative | Assesses whether the stock is under- or overpriced based on standard valuation multiples |
| 9 | `fscore_bfl_growth` | MATRIX | 0.3807 | Backfilled Pentagon | Growth composite estimating expected medium-term growth potential (0–100) |
| 10 | `fscore_bfl_momentum` | MATRIX | 0.3774 | Backfilled Pentagon | Composite momentum measure reflecting recent price dynamics and analyst revisions |
| 11 | `fscore_bfl_profitability` | MATRIX | 0.3797 | Backfilled Pentagon | Profitability composite ranking firms by cash flow generation and efficiency (0–100) |
| 12 | `fscore_bfl_quality` | MATRIX | 0.3828 | Backfilled Pentagon | Composite measuring earnings quality, stability, and balance-sheet resilience |
| 13 | `fscore_bfl_surface` | MATRIX | 0.3740 | Backfilled Pentagon | Static composite “style surface” score aggregating the five primary style scores |
| 14 | `fscore_bfl_surface_accel` | MATRIX | 0.3726 | Backfilled Pentagon | Acceleration (derivative) of the pentagon surface score relative to previous month |
| 15 | `fscore_bfl_total` | MATRIX | 0.3726 | Backfilled Pentagon | Blended composite M-Score combining static pentagon surface and acceleration |
| 16 | `fscore_bfl_value` | MATRIX | 0.3808 | Backfilled Pentagon | Valuation composite indicating under/overpricing (higher is cheaper, 0–100) |
| 17 | `fscore_growth` | MATRIX | 0.2987 | Raw Pentagon | Raw composite measuring expected medium-term growth potential |
| 18 | `fscore_momentum` | MATRIX | 0.2973 | Raw Pentagon | Raw composite momentum metric focused on recent analyst estimate revisions |
| 19 | `fscore_profitability` | MATRIX | 0.2981 | Raw Pentagon | Raw composite ranking stocks by ability to generate cash flows and profitability |
| 20 | `fscore_quality` | MATRIX | 0.2993 | Raw Pentagon | Raw composite measuring sustainability and certainty of earnings |
| 21 | `fscore_surface` | MATRIX | 0.2957 | Raw Pentagon | Raw static overall style surface score combining 5 pillars |
| 22 | `fscore_surface_accel` | MATRIX | 0.2946 | Raw Pentagon | Raw acceleration of pentagon surface score vs previous month |
| 23 | `fscore_total` | MATRIX | 0.2946 | Raw Pentagon | Raw final blended score combining surface and acceleration |
| 24 | `fscore_value` | MATRIX | 0.2981 | Raw Pentagon | Raw composite assessing whether stock is under/overpriced |

#### Key Research Insights
- The `*_derivative` fields are pre-computed daily rank indicators that **guarantee 100% cross-sectional universe coverage**. This completely eliminates sub-universe coverage test failures on BRAIN (`subuniverse_sharpe` criterion).
- The `fscore_bfl_*` fields represent a multi-factor pentagon surface ($Value \times Growth \times Profitability \times Momentum \times Quality$). The surface area measures total factor strength, while `fscore_bfl_surface_accel` measures fundamental inflection velocity.

---

### 1.3 Dataset `news18`: RavenPack News Data

- **Platform ID**: `news18`
- **Name**: RavenPack News Data
- **Category**: `news` (News) | **Subcategory**: `news-news-sentiment` (News Sentiment)
- **Primary Instrument**: Global / US Equities
- **Delay**: Delay 1 (121 fields), Delay 0 (24 fields)
- **Cross-Sectional Coverage**: 73.01% (Delay 1 TOP1000) to 97.46% (Delay 0 TOP1000)
- **Total Fields**: 121 fields in Delay 1:
  - **71 MATRIX fields**: Pre-aggregated daily metrics (e.g. `mean_composite_sentiment_score`, `mean_earnings_evaluation_sentiment`)
  - **50 VECTOR fields**: Granular, intraday event-level records (e.g. `composite_sentiment_score_2`, `nws18_bee`, `nws18_bam`)
- **Key Academic Anchors**:
  - Research Paper 01: "The Momentum of News"
  - Research Paper 05: "Predictability of Industry Returns After M&A Announcements"
  - Research Paper 11: "Analyst Following Along the Supply Chain"
  - Research Paper 13: "Valuation Ratios, Surprises, Uncertainty or Sentiment: How Does Financial Machine Learning Predict Returns From Earnings Announcements?"

#### Representative Core Field Catalog for `news18`

##### Tier A: Daily Pre-Aggregated MATRIX Fields (Direct FastExpr Operators)

| Field ID | Type | Coverage | Economic Role | Description |
|----------|------|----------|---------------|-------------|
| `mean_composite_sentiment_score` | MATRIX | 1.0000 | Broad Sentiment | Average composite sentiment score across all news stories published today |
| `mean_earnings_evaluation_sentiment` | MATRIX | 1.0000 | Earnings Shock | Sentiment specifically from earnings evaluations, reports, and guidance |
| `mean_corporate_action_sentiment` | MATRIX | 1.0000 | Corporate Actions | Sentiment regarding dividends, buybacks, rights offerings, reorganizations |
| `mean_merger_acquisition_sentiment` | MATRIX | 1.0000 | M&A Catalysts | Sentiment from takeovers, mergers, divestitures, and buyout bids |
| `mean_event_novelty_score` | MATRIX | 1.0000 | Novelty Gate | Average novelty score indicating whether the news is genuinely new (0–100) |
| `mean_entity_relevance_score` | MATRIX | 1.0000 | Relevance Filter | How central the company is to the news story (prevents peripheral noise) |
| `mean_news_impact_projection` | MATRIX | 1.0000 | Impact Sizing | Model-projected price impact magnitude from news flashes |
| `rp_css_earnings` | MATRIX | 0.5000 | Topic Sentiment | Composite sentiment score focused on earnings events |
| `rp_css_dividends` | MATRIX | 0.5000 | Topic Sentiment | Composite sentiment score focused on dividend actions |
| `rp_css_credit` | MATRIX | 0.5000 | Credit Risk | Composite sentiment score for debt and creditworthiness |
| `rp_css_insider` | MATRIX | 0.5000 | Insider Activity | Composite sentiment score for insider transactions and executive moves |

##### Tier B: Intraday VECTOR Fields (Requires Vector Operators `vec_avg` / `vec_sum`)

| Field ID | Type | Coverage | Vector Aggregation | Description |
|----------|------|----------|--------------------|-------------|
| `composite_sentiment_score_2` | VECTOR | 1.0000 | `vec_avg(...)` | Granular multi-source sentiment score per story |
| `earnings_evaluation_sentiment` | VECTOR | 1.0000 | `vec_avg(...)` | Individual story sentiment on earnings release |
| `event_novelty_score_2` | VECTOR | 1.0000 | `vec_avg(...)` | Novelty score within a 24-hour rolling window |
| `news_impact_projection_score` | VECTOR | 1.0000 | `vec_avg(...)` | Predicted price impact per news flash |
| `nws18_acb` | VECTOR | 0.9030 | `vec_avg(...)` | Corporate action announcement sentiment |
| `nws18_bam` | VECTOR | 1.0000 | `vec_avg(...)` | M&A sentiment indicator ($-1, 0, +1$) |
| `nws18_bee` | VECTOR | 1.0000 | `vec_avg(...)` | Earnings evaluation score ($-1$ negative, $0$ neutral, $+1$ positive) |
| `nws18_ber` | VECTOR | 1.0000 | `vec_sum(...)` | Earnings Release indicator flag |
| `nws18_nip` | VECTOR | 0.9030 | `vec_avg(...)` | Narrative impact score $[-1, +1]$ |
| `nws18_ssc` | VECTOR | 1.0000 | `vec_avg(...)` | Continuous story sentiment $[-1.00, +1.00]$ |
| `story_event_record_count` | VECTOR | 0.9030 | `vec_sum(...)` | Total event records per story (news intensity) |

#### Vector-to-Matrix Folding Rules in FastExpr
FastExpr does not allow direct matrix arithmetic on `VECTOR` data fields (e.g. `nws18_ssc + 1` causes simulation failure). Vector fields **must be folded** using:
1. `vec_avg(x)`: Computes the arithmetic mean across all intraday news items for that stock on that day.
2. `vec_sum(x)`: Computes the arithmetic sum across all intraday news items (ideal for event counts / volume).
3. If no news occurred for an instrument on day $t$, `vec_avg(x)` evaluates to `NaN`. FastExpr code must explicitly apply `ts_backfill(vec_avg(x), d)` to bridge non-event intervals.

---

### 1.4 Dataset `option9`: Options Analytics

- **Platform ID**: `option9`
- **Name**: Options Analytics
- **Category**: `option` (Option) | **Subcategory**: `option-option-analytics` (Option Analytics)
- **Primary Instrument**: US Equities with listed option chains (>2,500 tickers)
- **Delay**: Delay 1
- **Cross-Sectional Coverage**: **97.95% - 98.38%** (Exceptional coverage across TOP1000/TOP3000)
- **Date Coverage**: 1.0 (Full history)
- **Total Fields**: 74 fields (100% MATRIX type)
- **Key Academic Anchor**: Research Paper 10 ("Disagreement in Option Market and Cross Section Stock Returns")

#### Structure of the Option9 Surface
Option9 provides standardized metrics across **12 constant time-to-expiration horizons**:
$$\mathcal{T} = \{10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080\} \text{ calendar days}$$

Across these 12 horizons, Option9 exposes 6 orthogonal factor families:

| # | Factor Family | Prefix Syntax | Horizons Available | Coverage | Economic Concept |
|---|---------------|---------------|-------------------|----------|------------------|
| 1 | Put-Call Volume Ratio | `pcr_vol_<T>` | 10 to 1080d + `pcr_vol_all` | 0.9810 | Tactical flow / informed trader speculative activity |
| 2 | Put-Call Open Interest Ratio | `pcr_oi_<T>` | 10 to 1080d + `pcr_oi_all` | 0.9838 | Structural hedging / institutional downside positioning |
| 3 | Synthetic Forward Price | `forward_price_<T>` | 10 to 1080d | 0.9796 | Implied forward price via Put-Call parity ATM options |
| 4 | Call Breakeven Price | `call_breakeven_<T>` | 10 to 1080d | 0.9824 | OI-weighted price where call buyers break even |
| 5 | Put Breakeven Price | `put_breakeven_<T>` | 10 to 1080d | 0.9795 | OI-weighted price where put buyers break even |
| 6 | Total Option Breakeven | `option_breakeven_<T>` | 10 to 1080d | 0.9835 | OI-weighted combined straddle/strangle breakeven |

#### Mathematical Derivations & Factor Formulations

1. **Breakeven Asymmetry (Implied Directional Skew)**:
   The call breakeven return measures required upside, while put breakeven return measures downside cushion:
   $$\text{upside\_req} = \frac{\text{call\_breakeven\_30}}{\text{close}} - 1$$
   $$\text{downside\_cushion} = 1 - \frac{\text{put\_breakeven\_30}}{\text{close}}$$
   $$\text{breakeven\_asymmetry} = \text{downside\_cushion} - \text{upside\_req}$$

2. **Synthetic Forward Parity Dislocation**:
   Under Put-Call Parity: $F_T = S_0 e^{(r - q)T} = C - P + K e^{-rT}$.
   Normalized forward premium:
   $$\text{fwd\_premium}_T = \frac{\text{forward\_price\_}T}{\text{close}} - 1$$
   Deviation from sector median forward premium signals borrowing demand / short-sale squeeze pressure.

3. **Put-Call Volume vs Open Interest Divergence**:
   $$\text{flow\_divergence} = \text{rank}(\text{pcr\_vol\_20}) - \text{rank}(\text{pcr\_oi\_20})$$
   A surge in volume PCR without a corresponding rise in open interest indicates day-trading or temporary hedging rather than structural institutional liquidation.

---

## 2. FastExpr Syntax, Operators, and Grammar Specification

`FastExpr` is WorldQuant BRAIN's proprietary domain-specific language for vectorized, matrix-based quantitative research.

### 2.1 Grammar Rules & Structural Mechanics
1. **Multi-Statement Scripting**: FastExpr supports multi-line expressions with temporary variable assignments separated by semicolons (`;`). The last unassigned expression is returned as the Alpha weights.
   ```fastexpr
   var1 = rank(field1) - 0.5;
   var2 = signed_power(var1, 1.5);
   group_neutralize(var2, densify(subindustry))
   ```
2. **Execution Context**: All expressions are evaluated element-wise over the 3D tensor $[T \times N]$ (Dates $\times$ Instruments).
3. **Missing Value Propagation**: Arithmetic on `NaN` yields `NaN`. If a final weight contains `NaN`, the platform assigns zero position or fails closed if `nanHandling="OFF"`. Always apply `ts_backfill(x, d)` or `if_else(is_nan(x), 0, x)`.
4. **Group Densification**: `densify(group)` maps arbitrary non-contiguous group IDs to contiguous integers $[0, K-1]$. **Authoritative platform requirement**: Always wrap group variables in `densify()` inside `group_neutralize` or `group_rank` to avoid simulation timeouts.

---

### 2.2 Authoritative FastExpr Operator Catalog (66 Operators)

WorldQuant BRAIN currently exposes exactly **66 native operators** categorized into 7 functional families:

#### Family 1: Arithmetic Operators (15 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `add` | `add(x, y, filter=false)`, `x + y` | Element-wise addition. Set `filter=true` to treat NaNs as 0 before summing. |
| `subtract` | `subtract(x, y, filter=false)`, `x - y` | Element-wise subtraction. |
| `multiply` | `multiply(x, y, ..., filter=false)`, `x * y` | Element-wise multiplication. |
| `divide` | `divide(x, y)`, `x / y` | Element-wise division. Division by zero yields NaN. |
| `signed_power`| `signed_power(x, y)` | $\text{sign}(x) \cdot \|x\|^y$. Preserves sign of $x$; continuous, odd function. |
| `power` | `power(x, y)`, `x ^ y` | Element-wise exponentiation. |
| `sqrt` | `sqrt(x)` | Square root. Negative inputs yield NaN. |
| `log` | `log(x)` | Natural logarithm. Non-positive inputs yield NaN. |
| `abs` | `abs(x)` | Absolute value $\|x\|$. |
| `sign` | `sign(x)` | $+1$ if $x > 0$, $-1$ if $x < 0$, $0$ if $x == 0$. |
| `reverse` | `reverse(x)`, `-x` | Negation: $-1 \cdot x$. |
| `inverse` | `inverse(x)` | Multiplicative reciprocal: $1 / x$. |
| `min` | `min(x, y, ...)` | Element-wise minimum across 2 or more arguments. |
| `max` | `max(x, y, ...)` | Element-wise maximum across 2 or more arguments. |
| `densify` | `densify(x)` | Maps grouping integers to compact range $[0, n-1]$ to accelerate group operators. |

#### Family 2: Cross-Sectional Operators (6 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `rank` | `rank(x)` | Cross-sectional percentile rank in $[0, 1]$. Uniform distribution. |
| `scale` | `scale(x, a=1.0)` | Scales vector so sum of absolute weights equals $a$: $x_i / \sum \|x_j\| \cdot a$. |
| `winsorize` | `winsorize(x, std=4.0)` | Clamps cross-sectional outliers at $\mu \pm \text{std} \cdot \sigma$. |
| `zscore` | `zscore(x)` | Cross-sectional standardization: $(x - \mu) / \sigma$. |
| `normalize` | `normalize(x, useStd=false, limit=0.0)` | Normalizes cross-section with optional standard deviation scaling and truncation. |
| `quantile` | `quantile(x, driver="gaussian")` | Reshapes cross-sectional rank via inverse CDF of chosen distribution (e.g. Gaussian). |

#### Family 3: Group Operators (6 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `group_neutralize`| `group_neutralize(x, group)` | Demeans $x$ within each group: $x - \text{mean}_{\text{group}}(x)$. |
| `group_rank` | `group_rank(x, group)` | Percentile rank of $x$ computed independently within each group. |
| `group_mean` | `group_mean(x, weight, group)` | Group-level weighted mean. |
| `group_scale` | `group_scale(x, group)` | Rescales values within each group to $[0, 1]$. |
| `group_zscore`| `group_zscore(x, group)` | Calculates $(x - \mu_{\text{group}}) / \sigma_{\text{group}}$. |
| `group_backfill`| `group_backfill(x, group, d, std=4.0)` | Imputes missing values using group average over historical lookback $d$. |

#### Family 4: Time Series Operators (24 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `ts_decay_linear`| `ts_decay_linear(x, d, dense=false)` | Linearly weighted moving average over $d$ days: $w_t = d - t + 1$. Smooths turnover. |
| `ts_mean` | `ts_mean(x, d)` | Simple moving average over rolling window $d$. |
| `ts_sum` | `ts_sum(x, d)` | Rolling sum over $d$ days. |
| `ts_delta` | `ts_delta(x, d)` | $x_t - x_{t-d}$. Key for momentum and acceleration. |
| `ts_delay` | `ts_delay(x, d)` | Lagged value $x_{t-d}$. |
| `ts_rank` | `ts_rank(x, d, constant=0)` | Percentile rank of current value $x_t$ within its own historical past $d$ days. |
| `ts_zscore` | `ts_zscore(x, d)` | Rolling time-series z-score $(x_t - \mu_d) / \sigma_d$. |
| `ts_std_dev` | `ts_std_dev(x, d)` | Rolling sample standard deviation over $d$ days. |
| `ts_corr` | `ts_corr(x, y, d)` | Rolling Pearson correlation between series $x$ and $y$ over $d$ days. |
| `ts_covariance`| `ts_covariance(y, x, d)` | Rolling sample covariance over $d$ days. |
| `ts_backfill` | `ts_backfill(x, lookback=d, k=1)` | Carries forward the last valid observation up to $d$ days for NaNs. |
| `ts_scale` | `ts_scale(x, d, constant=0)` | Scales rolling time series to $[0, 1]$ based on min and max in window $d$. |
| `ts_product` | `ts_product(x, d)` | Rolling geometric compounding product $\prod_{i=0}^{d-1} x_{t-i}$. |
| `ts_quantile` | `ts_quantile(x, d, driver="gaussian")` | Rolling time-series quantile mapped to Gaussian inverse CDF. |
| `ts_count_nans`| `ts_count_nans(x, d)` | Number of NaN values in window $d$. |
| `ts_arg_max` | `ts_arg_max(x, d)` | Days elapsed since the maximum value in window $d$ (0 = today). |
| `ts_arg_min` | `ts_arg_min(x, d)` | Days elapsed since the minimum value in window $d$ (0 = today). |
| `ts_av_diff` | `ts_av_diff(x, d)` | $x_t - \text{ts\_mean}(x, d)$, ignoring NaNs in mean calculation. |
| `last_diff_value`| `last_diff_value(x, d)` | Most recent value different from current $x_t$ within past $d$ days. |
| `days_from_last_change` | `days_from_last_change(x)` | Trading days elapsed since $x$ last changed value. |
| `hump` | `hump(x, hump=0.01)` | Threshold filter: suppresses changes smaller than `hump` to reduce turnover. |
| `kth_element` | `kth_element(x, d, k, ignore="NaN")` | Returns $k$-th ranked element from past $d$ days. |
| `ts_step` | `ts_step(1)` | Incremental calendar counter incrementing daily. |
| `ts_regression`| `ts_regression(y, x, d, lag=0, rettype=0)` | Rolling OLS regression metrics (slope, intercept, residuals). |

#### Family 5: Logical Operators (11 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `if_else` | `if_else(condition, val_true, val_false)` | Conditional ternary branching. |
| `is_nan` | `is_nan(x)` | Returns 1 if $x$ is NaN, 0 otherwise. |
| `and` | `and(a, b)` | Logical AND ($1$ if both non-zero, else $0$). |
| `or` | `or(a, b)` | Logical OR ($1$ if either non-zero, else $0$). |
| `not` | `not(x)` | Logical NOT ($1$ if $x == 0$, $0$ if $x \ne 0$). |
| `equal` | `a == b` | Equality comparison. |
| `not_equal` | `a != b` | Inequality comparison. |
| `greater` | `a > b` | Greater than. |
| `greater_equal` | `a >= b` | Greater than or equal to. |
| `less` | `a < b` | Less than. |
| `less_equal`| `a <= b` | Less than or equal to. |

#### Family 6: Transformational Operators (2 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `trade_when` | `trade_when(trigger, alpha, exit)` | Event-driven execution: updates alpha to new value when `trigger` is true; holds previous value when false; closes position (`NaN`) when `exit` is true. Set `exit = -1` to disable exit closing. |
| `bucket` | `bucket(rank(x), range="0, 1, 0.1", ...)` | Partitions cross-section into discrete classification buckets for group operations. |

#### Family 7: Vector Operators (2 operators)

| Operator | Exact Signature | Description & Rules |
|----------|-----------------|---------------------|
| `vec_avg` | `vec_avg(x)` | Computes the arithmetic mean across all elements in a vector field for each instrument and date, collapsing `VECTOR` $\to$ `MATRIX`. |
| `vec_sum` | `vec_sum(x)` | Computes the arithmetic sum across all elements in a vector field, collapsing `VECTOR` $\to$ `MATRIX`. |

---

## 3. BRAIN API `/data-fields` Specification & Validator Rules

### 3.1 Endpoint Contract

- **Method**: `GET`
- **Path**: `https://api.worldquantbrain.com/data-fields`
- **Authentication**: Session cookie (`Cookie: ...`) or Basic/Bearer Authorization.
- **Rate Limit Policy**: Returns `429 Too Many Requests` on burst. Handled with exponential backoff:
  $$\text{wait\_sec} = \max(\text{Retry-After}, 2^{\text{attempt}} + 1.0)$$

#### Query Parameters

| Parameter | Type | Required? | Valid Values / Example | Purpose |
|-----------|------|-----------|------------------------|---------|
| `instrumentType` | String | Strongly Recommended | `EQUITY` | Filter by asset class |
| `region` | String | Strongly Recommended | `USA`, `GLB`, `EUR`, `ASI` | Target geographic market |
| `delay` | Integer | Required | `0`, `1` | Trading simulation execution delay |
| `universe` | String | Required | `TOP1000`, `TOP3000`, `TOP200`, `TOP500`, `TOPSP500` | Target liquid asset universe |
| `dataset.id` | String | Optional | `model51`, `model16`, `news18`, `option9` | Filter fields by specific dataset ID |
| `id` | String | Optional | `beta_last_30_days_spy` | Query exact field metadata by ID |
| `search` | String | Optional | `sentiment`, `volatility`, `cashflow` | Free-text search over description/name |
| `type` | String | Optional | `MATRIX`, `VECTOR` | Filter by tensor format |
| `limit` | Integer | Optional | $1 \le \text{limit} \le 50$ (Default: 10) | Page size |
| `offset` | Integer | Optional | $\ge 0$ (Default: 0) | Pagination cursor offset |

#### Response Schema Structure

```json
{
  "count": 16,
  "results": [
    {
      "id": "beta_last_30_days_spy",
      "description": "The rolling beta value of the security relative to SPY...",
      "dataset": {
        "id": "model51",
        "name": "Systematic Risk Metrics"
      },
      "category": {
        "id": "model",
        "name": "Model"
      },
      "subcategory": {
        "id": "model-risk-models",
        "name": "Risk Models"
      },
      "region": "USA",
      "delay": 1,
      "universe": "TOP1000",
      "type": "MATRIX",
      "dateCoverage": 1.0,
      "coverage": 0.9765,
      "userCount": 73,
      "alphaCount": 145,
      "themes": [],
      "dateCreated": "2022-05-01"
    }
  ]
}
```

### 3.2 Schema Validation Requirements
Before any candidate expression is dispatched for simulation, the automated validator must verify:
1. **Field Existence & Typo Guard**: Field ID exists in the target dataset's `/data-fields` metadata catalog.
2. **Dimension Compatibility**: If `type == "VECTOR"`, verify the field is enclosed in `vec_avg(...)` or `vec_sum(...)`. Raw vectors in matrix arithmetic generate `400 Bad Request` or runtime simulation aborts.
3. **Coverage Check**: Cross-sectional coverage $\ge 0.70$ (or accompanied by `ts_backfill(..., d)`).
4. **Delay Compatibility**: Selected field must be available under the specified simulation delay (`delay=1`).
5. **Neutralization Group Integrity**: Neutralization group inside `group_neutralize(x, densify(group))` must match valid groupings (`subindustry`, `industry`, `sector`, `market`).

---

## 4. Economic Rationales, Factor Architectures & Failure Modes

### 4.1 Systematic Risk Metrics (`model51`)

#### Economic Hypothesis
Traditional CAPM and multi-factor pricing models assume idiosyncratic risk is unpriced because it can be diversified away. However, in practice, institutional mandate limits and limits to arbitrage create the **Idiosyncratic Volatility Puzzle** (Ang, Hodrick, Xing, Zhang 2006): stocks experiencing sudden idiosyncratic variance spikes or unstable benchmark correlations experience pricing inefficiencies. By isolating the idiosyncratic component ($\text{unsystematic\_risk} = 1 - R^2$) and measuring horizon beta term structures ($\beta_{30d} - \beta_{360d}$), we identify transient mispricings caused by index rebalancing and retail flow shocks.

#### Factor Architecture Idea: Systematic-Idiosyncratic Term Spread
```fastexpr
idio_short = ts_backfill(unsystematic_risk_last_30_days, 10);
idio_long = ts_backfill(unsystematic_risk_last_360_days, 15);
idio_term_spread = rank(idio_short / (idio_long + 0.001)) - 0.5;
beta_shock = rank(beta_last_30_days_spy - beta_last_360_days_spy) - 0.5;
reversal = rank(ts_delta(close, 3)) - 0.5;
raw_sig = 0.60 * signed_power(idio_term_spread, 1.4) - 0.30 * signed_power(beta_shock, 1.2) - 0.10 * reversal;
group_neutralize(signed_power(raw_sig, 3.2), densify(subindustry))
```

#### Potential Failure Modes & Safeguards
- **Macro Market Stress / Liquidity Regime Shift**: During systemic crashes (e.g. March 2020), pairwise correlations spike toward 1.0, causing $1 - R^2 \to 0$ and compressing signal dispersion.
  - *Safeguard*: Winsorization and Subindustry neutralization to isolate relative, not market-wide, risk.
- **Option Availability Bias**: `model51` is computed on optionable equities. In `TOP3000`, micro-caps without liquid options have `NaN` coverage.
  - *Safeguard*: Restrict universe to `TOP1000` or backfill with group averages.

---

### 4.2 Fundamental Scores & Derivative Acceleration (`model16`)

#### Economic Hypothesis
Static accounting metrics (P/E, Book-to-Market) are notorious for long half-lives, high crowd density, and susceptibility to "Value Traps" (cheap companies deteriorating structurally). In contrast, the **rate of change (first derivative)** and **acceleration (second derivative)** of composite fundamental scores reveal inflection points in corporate operational efficiency before they are fully digested by the sell-side consensus (Lakonishok, Shleifer, Vishny 1994).

#### Factor Architecture Idea: Fundamental Acceleration vs Price Reversal
```fastexpr
fund_accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
cashflow_eff = rank(cashflow_efficiency_rank_derivative) - 0.5;
quality_rev = rank(analyst_revision_rank_derivative) - 0.5;
core_fundamental = 0.50 * fund_accel + 0.30 * cashflow_eff + 0.20 * quality_rev;
price_mom_damp = 0.12 * (rank(ts_delta(close, 5)) - 0.5);
sig = signed_power(core_fundamental, 1.5) - price_mom_damp;
group_neutralize(signed_power(sig, 3.5), densify(subindustry))
```

#### Potential Failure Modes & Safeguards
- **Earnings Season Revision Clustering**: Analyst revisions cluster heavily around quarterly earnings calls (Jan/Apr/Jul/Oct). Signals may suffer from seasonal variance.
  - *Safeguard*: Use `ts_decay_linear(..., 10)` to smooth revisions into a steady flow.
- **Coverage Disparity in Pentagon Scores**: Raw `fscore_*` scores only cover ~30% of stocks.
  - *Safeguard*: Build production candidates exclusively on the 8 `*_derivative` fields (`cov=1.0`), which provide 100% cross-sectional coverage across all universes.

---

### 4.3 RavenPack News Data (`news18`)

#### Economic Hypothesis
Information diffusion in financial markets is non-instantaneous. While algorithmic traders react within milliseconds to earnings headlines, institutional portfolio managers take days to adjust holdings for subtle corporate events, executive commentary, and supply-chain shocks. RavenPack's structured sentiment metrics capture both event novelty and sentiment polarity. By combining `mean_earnings_evaluation_sentiment` and corporate action sentiment while filtering out unoriginal syndicated stories, we capture post-news announcement drift (Chan 2003).

#### Factor Architecture Idea: Novelty-Weighted Sentiment Drift
```fastexpr
raw_sent = ts_backfill(vec_avg(composite_sentiment_score_2), 15);
novelty = ts_backfill(vec_avg(event_novelty_score_2), 15);
sent_clean = if_else(is_nan(raw_sent), 0, rank(raw_sent) - 0.5);
novelty_wt = if_else(is_nan(novelty), 0.5, rank(novelty));
news_alpha = signed_power(sent_clean, 1.4) * (0.5 + 0.5 * novelty_wt);
vol_burst = signed_power(rank(volume / ts_mean(volume, 20)) - 0.5, 1.2);
reversal_damp = 0.08 * (rank(ts_delta(close, 3)) - 0.5);
composite = 0.70 * news_alpha + 0.20 * vol_burst - reversal_damp;
group_neutralize(signed_power(composite, 3.8), densify(subindustry))
```

#### Potential Failure Modes & Safeguards
- **Excessive Turnover & Short Half-Life**: Raw news signals decay in 1–2 days. Unsmoothed news expressions routinely hit 120%+ turnover, failing BRAIN's strict 70% threshold.
  - *Safeguard*: Apply `ts_decay_linear(..., 10)` and set simulation decay `decay: 16` to `decay: 22`.
- **Media Sentiment Bubbles & False Positives**: Clickbait headlines or syndicated news flashes inflate sentiment without economic substance.
  - *Safeguard*: Weight sentiment by `mean_entity_relevance_score` and dampen with short-term reversal.

---

### 4.4 Options Analytics (`option9`)

#### Economic Hypothesis
The options market is the preferred venue for informed institutional investors possessing asymmetric information or seeking asymmetric payoff structures (Black 1975, Easley, O'Hara, Srinivas 1998). Because buying stock requires 100% capital commitment whereas options offer embedded leverage, informed directional bets first appear in out-of-the-money call/put volumes and breakeven shifts. When put-call ratios plunge or call breakevens contract relative to put breakevens, the options market is pricing an impending upward stock re-rating before it manifests in cash equity prices.

#### Factor Architecture Idea: Breakeven Asymmetry & PCR Flow Dislocation
```fastexpr
call_be_ret = ts_backfill(call_breakeven_30 / close - 1, 10);
put_be_ret = ts_backfill(1 - put_breakeven_30 / close, 10);
be_asymmetry = rank(put_be_ret - call_be_ret) - 0.5;
pcr_flow = rank(ts_decay_linear(ts_backfill(pcr_vol_20, 8), 5)) - 0.5;
reversal = 0.08 * (rank(ts_delta(close, 3)) - 0.5);
sig = 0.55 * signed_power(be_asymmetry, 1.4) - 0.40 * signed_power(pcr_flow, 1.3) - reversal;
group_neutralize(signed_power(sig, 4.0), densify(subindustry))
```

#### Potential Failure Modes & Safeguards
- **Expiration Pinning & Quadruple Witching**: Near monthly options expiration (third Friday of the month), market makers gamma-hedge aggressively, pinning stocks near heavy open interest strikes and creating artificial breakeven distortions.
  - *Safeguard*: Combine intermediate terms (30d and 60d) rather than ultra-short (10d) maturities.
- **Dividend Ex-Date Drop**: Implied forward prices and call breakevens drop on high-dividend stocks due to anticipated dividend deductions.
  - *Safeguard*: Sector or subindustry neutralization removes industry-wide dividend yield bias.

---

## 5. Quantitative Research & Platform Submission Rules Summary

Per official BRAIN Learn and Challenge Guidelines:
1. **Submission Check Gate**:
   - Delay 1 thresholds: **Sharpe $\ge 1.25$**, **Fitness $\ge 1.00$**, **Turnover $1.0\% - 70.0\%$**, **Returns $> 8.0\%$**, Maximum stock weight $< 10\%$.
   - Delay 0 thresholds: Sharpe $\ge 2.00$, Fitness $\ge 1.30$.
2. **Sub-universe Sharpe Test**:
   $$\text{subuniverse\_sharpe} \ge 0.75 \times \sqrt{\frac{\text{subuniverse\_size}}{\text{alpha\_universe\_size}}} \times \text{alpha\_sharpe}$$
3. **Correlation Bounds**:
   - BRAIN Self-correlation: PnL correlation with existing user alphas $< 0.70$ (or Sharpe $\ge 10\%$ higher).
   - AlphaFind Local Portfolio Policy: Pairwise Pearson correlation $\rho < 0.70$, with accretive portfolio Sharpe ($\Delta\text{Sharpe} > 0, \Delta\text{Vol} \le 0$).
4. **Execution Concurrency**:
   - Maximum 3 concurrent simulation workers per account (`DEFAULT_WORKERS_PER_ACCOUNT = 3`).
