# Factor Combinator Grid & Alternative Signal Strategy Specifications

**Document Version**: 1.0.0  
**Target Milestone**: M3 (Automated Expression Builder & Factor Combinator) / M4 (Simulations)  
**Author**: Combinator Strategy & Parameter Theorist (`explorer_m3_2`)  
**Evaluation Target**: WorldQuant BRAIN FastExpr Engine  
**Datasets Covered**: `model51` (Systematic Risk), `model16` (Fundamental Scores), `news18` (RavenPack News), `option9` (Options Analytics)  

---

## 1. Executive Summary & Specification Scope

The goal of Milestone M3 is to transition theoretical factor taxonomies into an automated, high-throughput expression generation combinator grid. By systematically combining orthogonal alternative data pillars with parameterized multi-layer transforms, group neutralizations, and friction dampeners, the combinator engine generates hundreds of distinct, mathematically sound, syntactically verified FastExpr candidate alphas ready for dispatch in Milestone M4.

This specification establishes:
1. The **exact mathematical formulas** and structural definitions for four premier alternative datasets (`model51`, `model16`, `news18`, `option9`).
2. The **combinator parameter grid**, spanning non-linear exponents, winsorization limits, decay smoothing windows, group neutralizations, and reversal dampeners.
3. Pre-dispatch **FastExpr syntax invariants** conforming to `SchemaValidator` (AST type lattice, vector folding, mandatory backfilling, and group densification).
4. Rigorous **economic rationales and transmission mechanisms** grounding every factor family to prevent data-mining biases.

---

## 2. Epistemological Classification Standard (AGENTS.md Compliance)

Per the research and evidence standard defined in `AGENTS.md`, all assertions, thresholds, and configurations in this specification are partitioned into four strict epistemological categories:

- **Official Platform Rule**:
  - All simulation jobs require explicit `delay` (1 for alternative datasets; Delay 1 fields fail in Delay 0).
  - Intraday 3D event vectors (`VECTOR` type) cannot participate in matrix arithmetic without vector folding operators (`vec_avg`, `vec_sum`, `vec_max`, etc.).
  - Group neutralization functions (`group_neutralize`, `group_rank`) require densified group indices via `densify(group)` to prevent runtime out-of-memory and HTTP 504 timeouts.
  - In-Sample submission gates require: Sharpe $\ge 1.25$, Fitness $\ge 1.00$, Turnover $1.0\% \le \tau \le 70.0\%$, Returns $> 8.0\%$, Sub-universe consistency decile checks pass, Max Stock Weight $< 10.0\%$, and all 6 core submission checks pass.
  - Simulation concurrency must adhere to account limits.

- **Project Policy**:
  - Single CLI process screening concurrency is strictly bounded to 3 workers (`DEFAULT_WORKERS_PER_ACCOUNT = 3`, `--workers 3`).
  - Cross-sectional coverage threshold for mandatory backfill: any field with coverage $< 0.50$ (or $< 50\%$) MUST be wrapped in `ts_backfill(..., d \ge 5)`.
  - Fail-closed pre-dispatch schema validation: 100% of candidate expressions must pass `SchemaValidator::validate` with 0 errors before transmission.
  - Portfolio merge acceptance gate: Pearson cross-correlation against active portfolio alphas $\max \rho < 0.70$, accretive portfolio impact ($\Delta \text{Sharpe} > 0, \Delta \text{Vol} \le 0$).

- **Research Hypothesis**:
  - Low-beta stability combined with idiosyncratic risk curvature contraction provides superior downside risk-adjusted return relative to raw momentum.
  - Fundamental acceleration convexity identifies operational inflection points earlier than quarterly earnings announcements, eliminating static value traps.
  - Divergence between factual composite news sentiment and editorial commentary exploits retail narrative overreaction.
  - Options breakeven asymmetry isolates risk-neutral skew saturation that reverts cross-sectionally.

- **Unknown**:
  - Real-time crowd competition weighting, exact un-normalized leaderboard rank weights, and proprietary cross-account submission correlation matrices on BRAIN.

---

## 3. Features Discovered & Edge Cases

### 3.1 Features Discovered
| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | Systematic Risk (`model51`) | Multi-Horizon Beta Term Spread | Measures dynamic slope change $\beta_{30d} - \beta_{360d}$ to detect sudden corporate distress | `beta_last_30_days_spy`, `beta_last_360_days_spy` (Matrix) | Matrix $[-2.0, +2.0]$ | Fail-closed if lag $< 1$ | `src/schema_validator.rs`, `FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` |
| 2 | Systematic Risk (`model51`) | Idiosyncratic Risk Curvature | Second finite difference of unsystematic variance across horizons $\epsilon_{30} - 2\epsilon_{60} + \epsilon_{90}$ | `unsystematic_risk_last_{30,60,90}_days` | Matrix $[-1.0, +1.0]$ | Missing backfill if unhandled | `src/schema_validator.rs` line 313 |
| 3 | Systematic Risk (`model51`) | Benchmark Decoupling Ratio | Ratio of unsystematic variance to systematic variance $(1-R^2)/(R^2 + 0.01)$ | `unsystematic_risk_last_60_days`, `systematic_risk_last_60_days` | Matrix $[0, \infty)$ | Division by zero if un-offset | Formula Identity $R^2 + (1-R^2) \equiv 1$ |
| 4 | Fundamental Scores (`model16`) | Multi-Factor Acceleration Convexity | Second time derivative of composite fundamental health score (100% cross-sectional coverage) | `multi_factor_acceleration_score_derivative` | Matrix $[0.0, 1.0]$ | 0 errors; 100% complete | `src/schema_validator.rs` line 357 |
| 5 | Fundamental Scores (`model16`) | Cashflow Efficiency Velocity | First derivative of cashflow generation combined with static multi-factor derivative | `cashflow_efficiency_rank_derivative`, `multi_factor_static_score_derivative` | Matrix $[0.0, 1.0]$ | 0 errors; 100% complete | `src/schema_validator.rs` line 353 |
| 6 | Fundamental Scores (`model16`) | Analyst Revision Drift | Consensus sell-side revision velocity interacted with earnings certainty | `analyst_revision_rank_derivative`, `earnings_certainty_rank_derivative` | Matrix $[0.0, 1.0]$ | 0 errors; 100% complete | `src/schema_validator.rs` line 352 |
| 7 | News Sentiment (`news18`) | Sentiment Divergence | Difference in rank between hard news composite and editorial commentary | `mean_composite_sentiment_score`, `mean_editorial_commentary_sentiment` | Matrix $[-1.0, +1.0]$ | NaN propagation if unbackfilled | `src/schema_validator.rs` line 482 |
| 8 | News Sentiment (`news18`) | Corporate Action Shocks | Costly managerial signaling from dividends, buybacks, and restructurings | `mean_corporate_action_sentiment` (Matrix) or vector folded | Matrix $[-1.0, +1.0]$ | NaN if zero news on day | `src/schema_validator.rs` line 484 |
| 9 | News Sentiment (`news18`) | Earnings Evaluation PEAD | Post-earnings announcement drift from RavenPack transcript & release NLP | `mean_earnings_evaluation_sentiment` (Matrix) or vector folded | Matrix $[-1.0, +1.0]$ | NaN if unbackfilled | `src/schema_validator.rs` line 483 |
| 10 | News Sentiment (`news18`) | Intraday Event Stream Folding | Vector aggregation collapsing 3D event streams into daily 2D matrices | `nws18_ssc`, `composite_sentiment_score_2`, `nws18_bam` (Vector) | Matrix $[-1.0, +1.0]$ | `ErrVecTypeMismatch` if unfolded | `src/schema_validator.rs` line 559 |
| 11 | Options Analytics (`option9`) | Breakeven Directional Asymmetry | Difference between downside put cushion and upside call hurdle normalized by close | `call_breakeven_<H>`, `put_breakeven_<H>`, `close` | Matrix $[-1.0, +1.0]$ | `WarnUnnormalizedPrice` if un-normalized | `src/schema_validator.rs` line 636 |
| 12 | Options Analytics (`option9`) | Forward Price Parity Dislocation | Synthetic forward discount $(F/S - 1)$ revealing unexpressed short interest | `forward_price_<H>`, `close` | Matrix $[-0.5, +0.5]$ | `WarnUnnormalizedPrice` if raw | `src/schema_validator.rs` line 641 |
| 13 | Options Analytics (`option9`) | Volume & Open Interest Term Slopes | Term structure slope of put-call ratios separating tactical panic from structural hedging | `pcr_vol_20 - pcr_vol_90`, `pcr_oi_30 - pcr_oi_180` | Matrix $(-\infty, +\infty)$ | `ErrUnknownField` if 'd' suffix added | `src/schema_validator.rs` line 639 |

### 3.2 Edge Cases & Observed Validator Behavior
| # | Feature | Input | Observed Behavior |
|---|---------|-------|-------------------|
| 1 | Vector folding | `group_neutralize(rank(nws18_ssc + 1.0), densify(subindustry))` | **FAILED**: `ErrVecTypeMismatch` — direct arithmetic between VECTOR and scalar is rejected. |
| 2 | Vector folding | `group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))` | **PASSED**: `vec_avg` folds 3D vector to 2D matrix, resolving type lattice. |
| 3 | Backfill threshold | `group_neutralize(rank(ts_backfill(fscore_growth, 4)), densify(subindustry))` | **FAILED**: `ErrMissingBackfill` — `fscore_growth` coverage is 29.8% (<50%), requires window $\ge 5$. |
| 4 | Lookback causality | `group_neutralize(rank(ts_delay(close, 0)), densify(subindustry))` | **FAILED**: `ErrNegativeLookback` — non-positive lookback window ($d \le 0$) violates causality. |
| 5 | Group densification | `group_neutralize(rank(close), subindustry)` | **FAILED**: `ErrUndensifiedGroup` — raw grouping keyword causes memory exhaustion; requires `densify(subindustry)`. |
| 6 | Option horizon syntax | `group_neutralize(rank(call_breakeven_30d / close - 1), densify(subindustry))` | **FAILED**: `ErrUnknownField` — option9 field names must not have trailing 'd'; must use `call_breakeven_30`. |
| 7 | Price normalization | `group_neutralize(rank(call_breakeven_30), densify(subindustry))` | **WARNING**: `WarnUnnormalizedPrice` — nominal price level ranked across assets of differing share prices. |
| 8 | Multi-statement script | `sig = vec_avg(nws18_ssc);` | **FAILED**: `ErrSyntaxMalformed` — script must terminate with an expression, not an assignment. |

---

## 4. Multi-Layer Combinator Pipeline & Parameter Grid Architecture

The automated combinator engine generates alpha candidates through a 4-layer functional composition pipeline:

$$\text{Alpha} = \mathcal{N} \circ \mathcal{D} \circ \mathcal{T} \circ \mathcal{S} (\text{Fields})$$

```
+---------------------------------------------------------------------------------------------------+
|                                LAYER 1: CORE ALTERNATIVE SIGNAL                                   |
|   - Field selection (model51, model16, news18, option9)                                          |
|   - Vector folding: vec_avg(v) or vec_sum(v)                                                      |
|   - Missing data handling: ts_backfill(s, w) with w in {10, 15, 20}                               |
+---------------------------------------------------------------------------------------------------+
                                                  │
                                                  ▼
+---------------------------------------------------------------------------------------------------+
|                                LAYER 2: NON-LINEAR TRANSFORM GRID                                 |
|   - Centered cross-sectional rank: rank(s) - 0.5                                                  |
|   - Outlier winsorization: winsorize(s, std) with std in {3.0, 4.0}                               |
|   - Non-linear tail shaping: signed_power(s, p) with p in {0.5, 1.5, 2.0}                         |
|   - Time-series decay smoothing: ts_decay_linear(s, d) with d in {5, 10, 15, 20}                  |
+---------------------------------------------------------------------------------------------------+
                                                  │
                                                  ▼
+---------------------------------------------------------------------------------------------------+
|                                LAYER 3: GROUP NEUTRALIZATION GRID                                 |
|   - Group granularity: densify(group)                                                             |
|   - Grid: densify(subindustry) | densify(industry) | densify(sector)                              |
+---------------------------------------------------------------------------------------------------+
                                                  │
                                                  ▼
+---------------------------------------------------------------------------------------------------+
|                                LAYER 4: REVERSAL & TURNOVER DAMPENERS                             |
|   - Multi-horizon return momentum subtraction: - w_rev * rank(ts_decay_linear(returns, 5))         |
|   - Price-delta reversal: - w_rev * signed_power(rank(ts_delta(close, 3)) - 0.5, 1.5)             |
|   - Volume liquidity dampener: * (1.0 - w_vol * rank(volume / ts_mean(volume, 20)))               |
+---------------------------------------------------------------------------------------------------+
```

### 4.1 Parameter Combinator Grid Specification

| Parameter Dimension | Symbol | Grid Values | Economic Purpose |
|---------------------|--------|-------------|------------------|
| **Power Exponent** | $p$ | `{0.5, 1.5, 2.0}` | $p=0.5$: Sub-linear tail compression (robust to outliers).<br>$p=1.5$: Moderate tail amplification.<br>$p=2.0$: Quadratic tail weighting (high-conviction extreme bets). |
| **Winsorize Limit** | $\kappa$ | `{3.0, 4.0}` | Clamps distribution outliers at $\pm \kappa \cdot \sigma$ to prevent single-stock portfolio dominance. |
| **Smoothing Window**| $d_{\text{decay}}$ | `{5, 10, 15, 20}` | Suppresses high-frequency turnover, matching holding period to information half-life. |
| **Neutralization Group** | $\mathcal{G}$ | `densify(subindustry)`, `densify(industry)`, `densify(sector)` | Eliminates sector/industry allocation bets to isolate pure idiosyncratic stock-selection alpha. |
| **Reversal Dampener Window** | $d_{\text{rev}}$ | `{3, 5}` | Offsets 1-day to 5-day liquidity mean-reversion at Delay 1. |
| **Reversal Weight** | $w_{\text{rev}}$ | `{0.04, 0.06, 0.08}` | Calibrates penalty subtracted from core alternative signal. |
| **Volume Dampener Weight** | $w_{\text{vol}}$ | `{0.20, 0.30}` | Downweights illiquid volume surge churn. |
| **Sizing Exponent** | $p_{\text{size}}$ | `{3.5, 3.8, 4.0, 4.2}` | Applied to net signal prior to group neutralization to concentrate portfolio weight on highest-conviction extremes. |

---

## 5. Dataset Family 1: Systematic Risk Metrics (`model51`)

### 5.1 Factor 1: Beta Momentum vs Beta Reversal
- **Economic Hypothesis**:
  - *Low-Beta Anomaly & Dynamic Re-rating*: In equilibrium, high-beta equities become overpriced because benchmark-constrained asset managers and retail lottery-seekers bid up high-beta securities (Frazzini & Pedersen 2014 BAB). Over medium-to-long horizons, low-beta equities outperform.
  - *Beta Momentum vs Reversal Transmission*: A sudden spike in short-term beta ($\beta_{30d} - \beta_{360d} > 0$) indicates surging systematic vulnerability, distress, or forced institutional liquidation. Conversely, when short-term beta expands beyond 2 standard deviations, it exhibits statistical mean-reversion (Beta Reversal).
- **Exact Mathematical Formulations**:
  1. *Beta Term Spread (Distress / Expansion Indicator)*:
     $$\Delta \beta_i = \text{ts\_backfill}(\beta_{30d, i}, 10) - \text{ts\_backfill}(\beta_{360d, i}, 15)$$
  2. *Low-Beta Core Stability*:
     $$\text{LB}_i = 0.5 - \text{rank}(\text{ts\_backfill}(\beta_{60d, i}, 10))$$
  3. *Beta Momentum vs Reversal Composite*:
     $$\text{Sig}_{\beta, i} = 0.65 \cdot \text{signed\_power}(\text{LB}_i, p_1) - 0.35 \cdot \text{signed\_power}(\text{rank}(\Delta \beta_i) - 0.5, p_2) - w_{\text{rev}} \cdot \text{RevDamp}$$

- **Parameter Grid**:
  - Exponents $p_1 \in \{1.5, 2.0\}$, $p_2 \in \{0.5, 1.5\}$
  - Neutralization $\in \{\text{densify(subindustry)}, \text{densify(industry)}, \text{densify(sector)}\}$
  - Reversal Dampener: $- 0.05 \cdot \text{signed\_power}(\text{rank}(\text{ts\_decay\_linear}(\text{returns}, 5)) - 0.5, 1.5)$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 51.1.A: Systematic Low-Beta Term Spread with Subindustry Neutralization
  beta_spread = ts_backfill(beta_last_30_days_spy, 10) - ts_backfill(beta_last_360_days_spy, 15);
  low_beta = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 10));
  raw_sig = 0.65 * signed_power(low_beta, 1.5) - 0.35 * signed_power(rank(beta_spread) - 0.5, 1.5);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  clean_alpha = raw_sig - rev_damp;
  group_neutralize(signed_power(clean_alpha, 4.0), densify(subindustry))
  ```

  ```fastexpr
  # Candidate 51.1.B: Systematic Beta Reversal with Industry Neutralization
  beta_30 = ts_backfill(beta_last_30_days_spy, 10);
  beta_rev = 0.5 - rank(beta_30 - ts_delay(beta_30, 20));
  base_beta = 0.5 - rank(ts_backfill(beta_last_90_days_spy, 10));
  sig = 0.60 * signed_power(base_beta, 2.0) + 0.40 * signed_power(beta_rev, 1.5) - 0.04 * (rank(ts_delta(close, 3)) - 0.5);
  group_neutralize(signed_power(sig, 3.8), densify(industry))
  ```

---

### 5.2 Factor 2: Idiosyncratic Risk Curvature
- **Economic Hypothesis**:
  - *Idiosyncratic Volatility Puzzle*: Stocks with high idiosyncratic volatility suffer from limits to arbitrage and retail gambling preferences, delivering anomalously low risk-adjusted returns (Ang et al. 2006, 2009; Stambaugh et al. 2015).
  - *Non-Linear Curvature Dynamic*: While static idiosyncratic risk is informative, the **curvature** (second difference across horizons $\tau \in \{30, 60, 90\}$ days) measures accelerating divergence in firm-specific uncertainty:
    $$\kappa_{\epsilon, i} = \text{IdioRisk}_{30d, i} - 2 \cdot \text{IdioRisk}_{60d, i} + \text{IdioRisk}_{90d, i}$$
    A convex curve ($\kappa_{\epsilon} > 0$) signals an abrupt acceleration of unpriced idiosyncratic instability (e.g., pending litigation, margin contraction, or governance crises). Shorting high-curvature assets while going long decelerating idiosyncratic names provides powerful risk-adjusted alpha.
- **Exact Mathematical Formulations**:
  $$\text{Idio}_{30} = \text{ts\_backfill}(\text{unsystematic\_risk\_last\_30\_days}, 10)$$
  $$\text{Idio}_{60} = \text{ts\_backfill}(\text{unsystematic\_risk\_last\_60\_days}, 10)$$
  $$\text{Idio}_{90} = \text{ts\_backfill}(\text{unsystematic\_risk\_last\_90\_days}, 10)$$
  $$\kappa_{\epsilon} = \text{Idio}_{30} - 2.0 \cdot \text{Idio}_{60} + \text{Idio}_{90}$$
  $$\text{Sig}_{\kappa} = -1.0 \cdot \text{signed\_power}(\text{rank}(\kappa_{\epsilon}) - 0.5, p) - w_{\text{rev}} \cdot (\text{rank}(\text{ts\_delta}(\text{close}, 3)) - 0.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 51.2.A: Idiosyncratic Risk Curvature with Sector Neutralization
  curv = ts_backfill(unsystematic_risk_last_30_days, 10) - 2.0 * ts_backfill(unsystematic_risk_last_60_days, 10) + ts_backfill(unsystematic_risk_last_90_days, 10);
  sig = -1.0 * signed_power(rank(curv) - 0.5, 2.0) - 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  group_neutralize(signed_power(sig, 4.0), densify(sector))
  ```

  ```fastexpr
  # Candidate 51.2.B: Dual-Horizon Idiosyncratic Deceleration with Subindustry Neutralization
  idio_short = ts_backfill(unsystematic_risk_last_30_days, 10);
  idio_med = ts_backfill(unsystematic_risk_last_90_days, 15);
  decel = rank(idio_med) - rank(idio_short);
  level = 0.5 - rank(idio_short);
  sig = 0.55 * signed_power(level, 1.5) + 0.45 * signed_power(decel, 1.5) - 0.04 * (rank(ts_delta(close, 3)) - 0.5);
  group_neutralize(signed_power(sig, 3.8), densify(subindustry))
  ```

---

### 5.3 Factor 3: Benchmark Decoupling Ratio
- **Economic Hypothesis**:
  - *Systematic vs Idiosyncratic Decoupling*: By definition of variance decomposition, total variance is $\sigma_i^2 = R_i^2 \sigma_i^2 + (1-R_i^2) \sigma_i^2$. When systematic correlation collapses ($\rho_{i, m} \to 0$, $R^2 \to 0$) while idiosyncratic risk expands ($1-R^2 \to 1.0$), the equity has decoupled from the market benchmark.
  - *Distress vs Quality Decoupling*: If decoupling occurs alongside elevated beta or negative returns, it represents idiosyncratic distress. However, if decoupling occurs with low beta and positive operational acceleration, it identifies a high-quality "all-weather" firm with unique competitive moats independent of macroeconomic cycles.
- **Exact Mathematical Formulations**:
  $$\text{DecouplingRatio}_i = \frac{\text{ts\_backfill}(\text{unsystematic\_risk\_last\_60\_days}, 10)}{\text{ts\_backfill}(\text{systematic\_risk\_last\_60\_days}, 10) + 0.02}$$
  $$\text{CorrPenalty}_i = \text{signed\_power}(\text{rank}(\text{ts\_backfill}(\text{correlation\_last\_60\_days\_spy}, 10)) - 0.5, 1.2)$$
  $$\text{Sig}_{\text{decouple}, i} = 0.60 \cdot \text{signed\_power}(0.5 - \text{rank}(\text{DecouplingRatio}_i), 1.5) - 0.40 \cdot \text{CorrPenalty}_i$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 51.3.A: Benchmark Decoupling Ratio with Volatility Normalization
  idio_60 = ts_backfill(unsystematic_risk_last_60_days, 10);
  sys_60 = ts_backfill(systematic_risk_last_60_days, 10);
  decouple_ratio = idio_60 / (sys_60 + 0.02);
  low_beta = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 10));
  sig = 0.60 * signed_power(0.5 - rank(decouple_ratio), 1.5) + 0.40 * signed_power(low_beta, 1.5);
  clean_sig = sig - 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  group_neutralize(signed_power(clean_sig, 4.0), densify(subindustry))
  ```

---

## 6. Dataset Family 2: Fundamental Scores & Derivative Acceleration (`model16`)

### 6.1 Factor 1: Multi-Factor Acceleration Convexity
- **Economic Hypothesis**:
  - *Convexity over Static Valuation*: Static valuation metrics (low P/E, high Book/Market) suffer from "value traps" (financially impaired, structurally declining firms). In contrast, fundamental convexity $a_i(t) = \frac{\partial^2 S_i}{\partial t^2}$ (the second derivative of fundamental score) detects positive operational inflections before consensus revisions catch up (Lakonishok et al. 1994, Piotroski 2000, Asness et al. 2019 QMJ).
  - *Coverage & Robustness*: `multi_factor_acceleration_score_derivative` is a **Tier 1 field with 100.0% cross-sectional coverage**, making it completely immune to sub-universe test failures.
- **Exact Mathematical Formulations**:
  $$\text{Accel}_i = \text{rank}(\text{multi\_factor\_acceleration\_score\_derivative}) - 0.5$$
  $$\text{Velocity}_i = \text{rank}(\text{multi\_factor\_static\_score\_derivative}) - 0.5$$
  $$\text{Core}_{\text{conv}, i} = 0.70 \cdot \text{signed\_power}(\text{Accel}_i, p) + 0.30 \cdot \text{signed\_power}(\text{Velocity}_i, p)$$
  $$\text{NetAlpha}_i = \text{Core}_{\text{conv}, i} - w_{\text{rev}} \cdot \text{signed\_power}(\text{rank}(\text{ts\_decay\_linear}(\text{returns}, 5)) - 0.5, 1.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 16.1.A: Fundamental Acceleration Convexity Alpha (Tier 1 100% Coverage)
  accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
  velocity = rank(multi_factor_static_score_derivative) - 0.5;
  core_fund = 0.70 * signed_power(accel, 1.5) + 0.30 * signed_power(velocity, 1.5);
  rev_damp = 0.06 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  sig = core_fund - rev_damp;
  group_neutralize(signed_power(sig, 4.0), densify(subindustry))
  ```

  ```fastexpr
  # Candidate 16.1.B: Value Trap Neutralized Acceleration Alpha
  accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
  rel_val = rank(relative_valuation_rank_derivative) - 0.5;
  trap_free_val = signed_power(rel_val, 1.5) * (0.6 + 0.4 * signed_power(accel, 1.2));
  rev_damp = 0.05 * (rank(ts_delta(close, 4)) - 0.5);
  sig = trap_free_val - rev_damp;
  group_neutralize(signed_power(sig, 3.8), densify(sector))
  ```

---

### 6.2 Factor 2: Cashflow Efficiency Velocity
- **Economic Hypothesis**:
  - *Accrual Anomaly & Cash Flow Persistence*: Accounting net income consists of cash flows and accounting accruals (Sloan 1996). Accruals frequently reverse, while operating cash flows represent genuine economic liquidity that persists.
  - *Cashflow Velocity Transmission*: Interacting `cashflow_efficiency_rank_derivative` with `multi_factor_static_score_derivative` isolates firms generating high operational cash return on capital whose overall fundamental trajectory is actively accelerating.
- **Exact Mathematical Formulations**:
  $$\text{CF}_i = \text{rank}(\text{cashflow\_efficiency\_rank\_derivative}) - 0.5$$
  $$\text{Growth}_i = \text{rank}(\text{growth\_potential\_rank\_derivative}) - 0.5$$
  $$\text{QualityDrift}_i = 0.65 \cdot \text{signed\_power}(\text{CF}_i, 1.5) + 0.35 \cdot \text{signed\_power}(\text{Growth}_i, 1.2)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 16.2.A: High-Efficiency Cashflow Velocity with Turnover Smoothing
  cf_eff = rank(cashflow_efficiency_rank_derivative) - 0.5;
  fund_vel = rank(multi_factor_static_score_derivative) - 0.5;
  growth = rank(growth_potential_rank_derivative) - 0.5;
  quality_sig = 0.50 * signed_power(cf_eff, 1.5) + 0.30 * signed_power(fund_vel, 1.5) + 0.20 * growth;
  smoothed_sig = ts_decay_linear(quality_sig, 10);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  alpha = smoothed_sig - rev_damp;
  group_neutralize(signed_power(alpha, 4.0), densify(subindustry))
  ```

---

### 6.3 Factor 3: Analyst Revision Drift
- **Economic Hypothesis**:
  - *Cognitive Conservatism & Incremental Revisions*: Sell-side analysts exhibit herding behavior and adjust forecast models slowly over time rather than in one discrete jump (Chan et al. 1996, Ball & Brown 1968).
  - *Predictability Interaction*: When positive revisions occur in firms with high earnings predictability (`earnings_certainty_rank_derivative`), forecast error variance is minimal and PEAD drift compounds smoothly.
- **Exact Mathematical Formulations**:
  $$\text{Rev}_i = \text{rank}(\text{analyst\_revision\_rank\_derivative}) - 0.5$$
  $$\text{Certainty}_i = \text{rank}(\text{earnings\_certainty\_rank\_derivative})$$
  $$\text{DriftSig}_i = \text{signed\_power}(\text{Rev}_i, 1.5) \cdot (0.4 + 0.6 \cdot \text{Certainty}_i) - 0.05 \cdot \text{signed\_power}(\text{rank}(\text{ts\_decay\_linear}(\text{returns}, 5)) - 0.5, 1.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 16.3.A: Analyst Revision Momentum with Earnings Certainty Multiplier
  rev = rank(analyst_revision_rank_derivative) - 0.5;
  cert = rank(earnings_certainty_rank_derivative);
  composite = rank(composite_factor_score_derivative) - 0.5;
  blended = 0.60 * signed_power(rev, 1.5) * (0.4 + 0.6 * cert) + 0.40 * signed_power(composite, 1.2);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  group_neutralize(signed_power(blended - rev_damp, 4.2), densify(subindustry))
  ```

---

## 7. Dataset Family 3: RavenPack News Sentiment (`news18`)

### 7.1 Factor 1: Sentiment Divergence (Editorial Commentary vs Hard News)
- **Economic Hypothesis**:
  - *Noise Trader Framing vs Intrinsic News Value*: Factual news releases (product launches, regulatory filings, financial results) convey objective changes in discounted cash flows. In contrast, editorial commentary and opinion pieces reflect media narrative framing, sensationalism, and retail biases (Tetlock 2007, Fang & Peress 2009).
  - *Arbitrage Mechanism*: When objective corporate composite sentiment is strongly positive but editorial commentary is skeptical/negative, retail investors hesitate while informed institutional capital quietly accumulates. Long factual positive / short editorial skepticism yields high informational alpha.
- **Exact Mathematical Formulations**:
  $$\text{HardNews}_i = \text{ts\_backfill}(\text{mean\_composite\_sentiment\_score}, 15)$$
  $$\text{Editorial}_i = \text{ts\_backfill}(\text{mean\_editorial\_commentary\_sentiment}, 15)$$
  $$\text{Divergence}_i = \text{rank}(\text{HardNews}_i) - \text{rank}(\text{Editorial}_i)$$
  $$\text{Sig}_{\text{div}, i} = \text{signed\_power}(\text{Divergence}_i, 1.5) - w_{\text{rev}} \cdot (\text{rank}(\text{ts\_delta}(\text{close}, 3)) - 0.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 18.1.A: News Sentiment Divergence Alpha with Industry Neutralization
  hard_news = ts_backfill(mean_composite_sentiment_score, 15);
  editorial = ts_backfill(mean_editorial_commentary_sentiment, 15);
  f_hard = if_else(is_nan(hard_news), 0, rank(hard_news) - 0.5);
  f_edit = if_else(is_nan(editorial), 0, rank(editorial) - 0.5);
  div = f_hard - f_edit;
  rev = 0.06 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  sig = signed_power(div, 1.5) - rev;
  group_neutralize(signed_power(sig, 4.0), densify(industry))
  ```

---

### 7.2 Factor 2: Corporate Action Shocks & Volume Catalysts
- **Economic Hypothesis**:
  - *Managerial Information Transmission*: Corporate actions (dividend raises, stock repurchases, balance sheet recapitalizations) represent credible, costly signals from corporate insiders possessing non-public operational visibility (Myers & Majluf 1984, Mitchell & Pulvino 2001).
  - *Volume Confirmation*: Corporate action announcements backed by unusual trading volume confirm institutional conviction and block accumulation, initiating multi-week upward price drift.
- **Exact Mathematical Formulations**:
  $$\text{CorpSent}_i = \text{ts\_backfill}(\text{mean\_corporate\_action\_sentiment}, 20)$$
  $$\text{VolShock}_i = \text{rank}(\text{volume} / \text{ts\_mean}(\text{volume}, 20)) - 0.5$$
  $$\text{Sig}_{\text{corp}, i} = \text{signed\_power}(\text{if\_else}(\text{is\_nan}(\text{CorpSent}_i), 0, \text{rank}(\text{CorpSent}_i) - 0.5), 1.5) \cdot (0.7 + 0.3 \cdot \text{signed\_power}(\text{VolShock}_i, 1.2))$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 18.2.A: Corporate Action Catalyst with Volume Shock Multiplier
  corp = ts_backfill(mean_corporate_action_sentiment, 20);
  f_corp = if_else(is_nan(corp), 0, rank(corp) - 0.5);
  vol_burst = rank(volume / ts_mean(volume, 20)) - 0.5;
  sig = signed_power(f_corp, 1.5) * (1.0 + 0.40 * signed_power(vol_burst, 1.2));
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  alpha = sig - rev_damp;
  group_neutralize(signed_power(alpha, 4.0), densify(subindustry))
  ```

---

### 7.3 Factor 3: Earnings Evaluation PEAD
- **Economic Hypothesis**:
  - *NLP Beyond Headline EPS*: Traditional PEAD relies on simple EPS surprises (SUE). RavenPack's earnings evaluation NLP parses textual sentiment across conference call transcripts and earnings releases, measuring management tone, forward guidance conservatism, and margin discussions (Ball & Brown 1968, Bernard & Thomas 1989).
  - *Decayed Horizon Smoothing*: Decaying the earnings shock via `ts_decay_linear(..., 15)` maintains an ongoing exposure for 15-30 trading days, capturing post-earnings drift without exceeding turnover limits.
- **Exact Mathematical Formulations**:
  $$\text{PEAD\_Raw}_i = \text{ts\_backfill}(\text{mean\_earnings\_evaluation\_sentiment}, 20)$$
  $$\text{PEAD\_Decayed}_i = \text{ts\_decay\_linear}(\text{if\_else}(\text{is\_nan}(\text{PEAD\_Raw}_i), 0, \text{rank}(\text{PEAD\_Raw}_i) - 0.5), 15)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 18.3.A: Decayed Earnings Sentiment PEAD Drift Alpha
  raw_pead = ts_backfill(mean_earnings_evaluation_sentiment, 20);
  clean_pead = if_else(is_nan(raw_pead), 0, rank(raw_pead) - 0.5);
  decayed_pead = ts_decay_linear(clean_pead, 15);
  reversal = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  sig = signed_power(decayed_pead, 1.5) - reversal;
  group_neutralize(signed_power(sig, 4.2), densify(subindustry))
  ```

---

### 7.4 Factor 4: Event Stream Folding (`vec_avg(nws18_ssc)` with `ts_backfill(..., 15)`)
- **Economic Hypothesis**:
  - *Ragged Event Tensor Reduction*: On WorldQuant BRAIN, real-time news stories arrive as intraday 3D event vectors $\mathbf{V}_{i, t} \in \mathbb{R}^{K_{i, t}}$. Direct arithmetic triggers an immediate type failure (`ErrVecTypeMismatch`).
  - *Vector Folding Rule*: The vector must be collapsed into a cross-sectional matrix using `vec_avg(nws18_ssc)` (average story sentiment). Because $>60\%$ of equities have no news stories on a given trading day, applying `ts_backfill(..., 15)` is mandatory to prevent signal collapse and avoid portfolio weight instability.
- **Exact Mathematical Formulations**:
  $$\text{Folded}_i = \text{ts\_backfill}(\text{vec\_avg}(\text{nws18\_ssc}), 15)$$
  $$\text{CleanSig}_i = \text{if\_else}(\text{is\_nan}(\text{Folded}_i), 0, \text{rank}(\text{Folded}_i) - 0.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 18.4.A: Validated Vector Folded Story Sentiment Alpha
  folded_ssc = ts_backfill(vec_avg(nws18_ssc), 15);
  clean_ssc = if_else(is_nan(folded_ssc), 0, rank(folded_ssc) - 0.5);
  smoothed_ssc = ts_decay_linear(clean_ssc, 10);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  sig = signed_power(smoothed_ssc, 1.5) - rev_damp;
  group_neutralize(signed_power(sig, 4.0), densify(subindustry))
  ```

---

## 8. Dataset Family 4: Options Analytics Surface (`option9`)

### 8.1 Factor 1: Breakeven Asymmetry
- **Economic Hypothesis**:
  - *Downside vs Upside Tail Pricing*: Options breakeven levels reflect the market price of directional movement required for option buyers to break even.
    $$\text{Call Hurdle: } R_{\text{call}, T} = \frac{\text{call\_breakeven\_}T}{\text{close}} - 1$$
    $$\text{Put Cushion: } R_{\text{put}, T} = 1 - \frac{\text{put\_breakeven\_}T}{\text{close}}$$
  - *Risk-Neutral Skew Saturation*: The **Breakeven Asymmetry** $\Delta \text{BE} = R_{\text{put}} - R_{\text{call}} = 2 - \frac{\text{call\_breakeven\_}T + \text{put\_breakeven\_}T}{\text{close}}$ measures risk-neutral crash skew. Extreme positive $\Delta \text{BE}$ indicates over-priced downside puts (panic hedging saturation), which systematically reverts as market fears dissipate, generating bullish forward equity returns.
- **Exact Mathematical Formulations**:
  $$\text{CallHurdle}_i = \text{ts\_backfill}(\text{call\_breakeven\_30} / \text{close} - 1, 10)$$
  $$\text{PutCushion}_i = \text{ts\_backfill}(1 - \text{put\_breakeven\_30} / \text{close}, 10)$$
  $$\text{Asym}_i = \text{PutCushion}_i - \text{CallHurdle}_i$$
  $$\text{Sig}_{\text{be}, i} = \text{signed\_power}(\text{rank}(\text{Asym}_i) - 0.5, 1.5) - 0.06 \cdot \text{signed\_power}(\text{rank}(\text{ts\_decay\_linear}(\text{returns}, 5)) - 0.5, 1.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 9.1.A: 30-Day Options Breakeven Directional Asymmetry
  call_hurdle = ts_backfill(call_breakeven_30 / close - 1, 10);
  put_cushion = ts_backfill(1 - put_breakeven_30 / close, 10);
  be_asym = rank(put_cushion - call_hurdle) - 0.5;
  rev_damp = 0.06 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  sig = signed_power(be_asym, 1.5) - rev_damp;
  group_neutralize(signed_power(sig, 4.0), densify(subindustry))
  ```

  ```fastexpr
  # Candidate 9.1.B: Multi-Tenor Breakeven Spread (30d vs 90d)
  be_30 = ts_backfill(2.0 - (call_breakeven_30 + put_breakeven_30) / close, 10);
  be_90 = ts_backfill(2.0 - (call_breakeven_90 + put_breakeven_90) / close, 10);
  be_term_spread = rank(be_30) - rank(be_90);
  sig = signed_power(be_term_spread, 1.5) - 0.05 * (rank(ts_delta(close, 3)) - 0.5);
  group_neutralize(signed_power(sig, 3.8), densify(sector))
  ```

---

### 8.2 Factor 2: Put-Call Forward Price Parity Dislocation
- **Economic Hypothesis**:
  - *Limits to Arbitrage & Short Squeeze Transmission*: Under European put-call parity, synthetic forward prices reflect spot equity prices, interest rates, and borrow fees:
    $$\frac{F_{t, T}}{S_t} - 1 \approx (r - q - c) \frac{T}{365}$$
    When an equity is heavily shorted and borrow fees $c$ are exorbitant, synthetic forward prices trade at deep discounts ($F_{t, T} \ll S_t$). An extreme negative forward premium signals intense unexpressed short selling and severe vulnerability to sudden short-squeeze rallies (Ofek et al. 2004, Battalio & Schultz 2006).
- **Exact Mathematical Formulations**:
  $$\text{FwdPremium}_i = \text{ts\_backfill}(\text{forward\_price\_60} / \text{close} - 1, 10)$$
  $$\text{PCR\_Vol}_i = \text{ts\_backfill}(\text{pcr\_vol\_60}, 10)$$
  $$\text{Sig}_{\text{fwd}, i} = 0.60 \cdot \text{signed\_power}(\text{rank}(\text{FwdPremium}_i) - 0.5, 1.5) - 0.40 \cdot \text{signed\_power}(\text{rank}(\text{PCR\_Vol}_i) - 0.5, 1.2)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 9.2.A: Forward Price Parity Dislocation & Squeeze Detection Alpha
  fwd_disc = ts_backfill(forward_price_60 / close - 1, 10);
  pcr_vol = ts_backfill(pcr_vol_60, 10);
  sig_fwd = signed_power(rank(fwd_disc) - 0.5, 1.5);
  sig_pcr = signed_power(rank(pcr_vol) - 0.5, 1.2);
  blended = 0.65 * sig_fwd - 0.35 * sig_pcr;
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  group_neutralize(signed_power(blended - rev_damp, 4.0), densify(subindustry))
  ```

---

### 8.3 Factor 3: Term Skew & Slope Dynamics (`pcr_vol_20 - pcr_vol_90`, `pcr_oi_30 - pcr_oi_180`)
- **Economic Hypothesis**:
  - *Flow vs Positioning Decoupling*:
    - **Volume Slope**: $\Delta \text{PCR}_{\text{vol}} = \text{pcr\_vol\_20} - \text{pcr\_vol\_90}$. Spikes in near-term 20-day put volume reflect transient retail panic and tactical hedging.
    - **Open Interest Slope**: $\Delta \text{PCR}_{\text{oi}} = \text{pcr\_oi\_30} - \text{pcr\_oi\_180}$. Measures durable, structural institutional downside positioning.
    - **Flow vs Positioning Divergence**: When tactical put volume spikes ($\text{pcr\_vol\_20} \uparrow$) without a commensurate rise in structural open interest ($\text{pcr\_oi\_30}$ flat), the order flow represents retail noise that mean-reverts bullishly.
- **Exact Mathematical Formulations**:
  $$\text{Slope}_{\text{vol}} = \text{ts\_decay\_linear}(\text{ts\_backfill}(\text{pcr\_vol\_20} - \text{pcr\_vol\_90}, 8), 5)$$
  $$\text{Slope}_{\text{oi}} = \text{ts\_decay\_linear}(\text{ts\_backfill}(\text{pcr\_oi\_30} - \text{pcr\_oi\_180}, 10), 5)$$
  $$\text{Sig}_{\text{slope}} = \text{signed\_power}(\text{rank}(\text{Slope}_{\text{oi}}) - \text{rank}(\text{Slope}_{\text{vol}}), 1.5) - 0.05 \cdot \text{signed\_power}(\text{rank}(\text{ts\_decay\_linear}(\text{returns}, 5)) - 0.5, 1.5)$$

- **Backtest-Ready FastExpr Formulations**:
  ```fastexpr
  # Candidate 9.3.A: Options Flow vs Positioning Slope Divergence Alpha
  vol_slope = ts_decay_linear(ts_backfill(pcr_vol_20 - pcr_vol_90, 8), 5);
  oi_slope = ts_decay_linear(ts_backfill(pcr_oi_30 - pcr_oi_180, 10), 5);
  slope_div = rank(oi_slope) - rank(vol_slope);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  sig = signed_power(slope_div, 1.5) - rev_damp;
  group_neutralize(signed_power(sig, 4.0), densify(subindustry))
  ```

---

## 9. Cross-Dataset Multi-Pillar Sanctuary Combinations

To produce alphas that achieve low cross-correlation ($\max \rho < 0.70$) against the existing 84 Out-of-Sample (OS) portfolio alphas, the combinator engine fuses orthogonal datasets into unified **Anchor + Catalyst + Dampener** architectures:

$$\text{Alpha}_{\text{Hybrid}} = w_1 \cdot \text{Anchor} + w_2 \cdot \text{Catalyst} - w_{\text{rev}} \cdot \text{ReversalDampener}$$

### 9.1 Hybrid Architecture 1: Systematic Low-Beta Anchor + RavenPack News Catalyst
- **Economic Hypothesis**:
  - Structural low-beta equities (`model51`) provide long-term downside defense and low drawdowns. RavenPack factual news sentiment (`news18`) provides high-velocity timing, ensuring the portfolio is not trapped in stagnant defensive positions during market rallies.
- **Backtest-Ready FastExpr**:
  ```fastexpr
  # Hybrid 1: Systematic Risk Anchor + News Catalyst
  anchor_beta = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 10));
  news_sent = ts_backfill(vec_avg(nws18_ssc), 15);
  catalyst_news = if_else(is_nan(news_sent), 0, rank(news_sent) - 0.5);
  core = 0.60 * signed_power(anchor_beta, 1.5) + 0.40 * signed_power(catalyst_news, 1.5);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  group_neutralize(signed_power(core - rev_damp, 4.0), densify(subindustry))
  ```

### 9.2 Hybrid Architecture 2: Fundamental Acceleration Anchor + Options Breakeven Catalyst
- **Economic Hypothesis**:
  - Firms in the highest decile of fundamental acceleration (`model16`) possess growing intrinsic value. Pairing this anchor with options breakeven asymmetry (`option9`) catches the exact inflection point where options market over-hedging collapses into a sharp re-pricing rally.
- **Backtest-Ready FastExpr**:
  ```fastexpr
  # Hybrid 2: Fundamental Convexity Anchor + Options Breakeven Catalyst
  anchor_accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
  call_h = ts_backfill(call_breakeven_30 / close - 1, 10);
  put_c = ts_backfill(1 - put_breakeven_30 / close, 10);
  catalyst_opt = rank(put_c - call_h) - 0.5;
  core = 0.55 * signed_power(anchor_accel, 1.5) + 0.45 * signed_power(catalyst_opt, 1.5);
  rev_damp = 0.05 * signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5);
  group_neutralize(signed_power(core - rev_damp, 4.0), densify(subindustry))
  ```

---

## 10. Simulation Setting Specifications for Milestone M4

Every candidate alpha generated by this combinator grid must be paired with standard WorldQuant BRAIN simulation settings:

```json
{
  "instrumentType": "EQUITY",
  "region": "USA",
  "universe": "TOP1000",
  "delay": 1,
  "decay": 16,
  "neutralization": "SUBINDUSTRY",
  "truncation": 0.05,
  "pasteurization": "ON",
  "unitHandling": "VERIFY",
  "nanHandling": "OFF",
  "language": "FASTEXPR"
}
```

### Parameter Tuning Grid for Settings:
- **Universe**: `TOP1000` (primary for high liquidity); `TOP500` (ultra-large-cap robustness); `TOP3000` (broad universe with backfill).
- **Execution Decay**: $d \in \{12, 16, 18, 22\}$. Fundamental and low-beta factors run best at $d=18\text{--}22$; News and options factors run best at $d=12\text{--}16$.
- **Truncation**: `0.05` to `0.07` (prevents individual stock weights from exceeding platform limit $<10\%$).
- **Neutralization**: `SUBINDUSTRY` (primary default); `INDUSTRY` (coarser grouping); `SECTOR` (macro-neutral).

---

## 11. Milestone M4 Backtest Gate & Portfolio Impact Checklist

Before any candidate alpha is promoted to the active portfolio, it must satisfy all 8 qualification criteria:

| # | Evaluation Gate | Threshold Requirement | Evidence Source | Policy Type |
|---|-----------------|-----------------------|-----------------|-------------|
| 1 | In-Sample Information Ratio | $\text{Sharpe} \ge 1.25$ | BRAIN Simulation Stats | Official Platform Rule |
| 2 | In-Sample Fitness | $\text{Fitness} \ge 1.00$ | BRAIN Simulation Stats | Official Platform Rule |
| 3 | Annualized Turnover | $1.0\% \le \tau \le 70.0\%$ | BRAIN Simulation Stats | Official Platform Rule |
| 4 | Annualized Return | $\text{Returns} > 8.0\%$ | BRAIN Simulation Stats | Official Platform Rule |
| 5 | Core Platform Checks | 6 / 6 checks `PASS` (Self-corr, Sub-universe, Weight, etc.) | BRAIN Check Evaluator | Official Platform Rule |
| 6 | Baseline Portfolio Correlation | $\max \rho < 0.70$ (against 84 OS alphas) | `alphafind pnl --compare` | Project Policy |
| 7 | Accretive Sharpe Impact | $\Delta \text{Sharpe} = \text{Sharpe}_{\text{merged}} - \text{Sharpe}_{\text{base}} > 0$ | Portfolio Evaluator | Project Policy |
| 8 | Portfolio Volatility Impact | $\Delta \text{Vol} = \text{Vol}_{\text{merged}} - \text{Vol}_{\text{base}} \le 0$ | Portfolio Evaluator | Project Policy |

---

## 12. Verification and Invalidation Method

To verify the syntactic validity and error-free AST evaluation of all candidate expressions:
1. Run cargo unit and adversarial tests:
   ```bash
   cargo test --test stress_schema_validator_tests
   ```
2. Validate any generated FastExpr string via `SchemaValidator::validate`:
   - Enforce 0 errors (`rep.is_valid == true`).
   - Confirm extracted fields correctly identify `MATRIX` vs `VECTOR` types.
   - Confirm group variable is densified.
   - Confirm lookbacks are positive integers.
