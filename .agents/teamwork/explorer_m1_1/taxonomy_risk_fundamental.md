# Mathematical Factor Taxonomy & Specification: Systematic Risk (`model51`) & Fundamental Scores (`model16`)

**Milestone**: M1 — Mathematical Factor Taxonomy & Specification Module  
**Author**: Risk & Fundamental Factor Theorist (`explorer_m1_1`)  
**Date**: 2026-10-24  
**Integrity Mode**: Development / Strict Research Standards  
**Target Platform**: WorldQuant BRAIN FastExpr Engine  

---

## 1. Executive Overview & Research Scope

This specification provides the rigorous mathematical formulation, economic hypotheses, trading signals, and failure mode mitigations for two core WorldQuant BRAIN Alternative Datasets:
1. **`model51` (Systematic Risk Metrics)**: Capturing multi-horizon rolling market beta ($\beta_{\text{SPY}}$), benchmark correlation ($\rho_{\text{SPY}}$), systematic variance ratio ($R^2$), and unsystematic (idiosyncratic) risk dispersion ($1 - R^2$).
2. **`model16` (Fundamental Scores & Derivative Acceleration)**: Capturing high-coverage standardized rank derivatives of analyst revisions, cash flow efficiency, quality scores, and multi-factor first and second derivatives (velocity and acceleration).

Both factor families address fundamental market microstructure and structural behavioral anomalies that are orthogonal to conventional price-volume technical indicators and static accounting multiples.

### 1.1 Taxonomy Classification Matrix

```
+-------------------------------------------------------------------------------------------------------+
|                                    FACTOR TAXONOMY ARCHITECTURE                                       |
+------------+--------------------+-------------------------+---------------+---------------------------+
| Dataset    | Platform Category  | Primary Economic Pillar | Tensor Type   | Core Economic Mechanism   |
+------------+--------------------+-------------------------+---------------+---------------------------+
| model51    | model-risk-models  | IDIOSYNCRATIC_RISK      | 16 MATRIX     | Low-Beta Anomaly,         |
|            |                    |                         |               | Idiosyncratic Vol Puzzle, |
|            |                    |                         |               | Horizon Beta Term Shock   |
+------------+--------------------+-------------------------+---------------+---------------------------+
| model16    | model-valuation    | FINANCIAL_HEALTH_QUALITY| 24 MATRIX     | Post-Earnings Drift,      |
|            |                    | & ANALYST_CONSENSUS     | (8 Deriv 100% | Cash Flow Persistence vs  |
|            |                    |                         | 16 Pent ~35%) | Accruals, Accel Inflection|
+------------+--------------------+-------------------------+---------------+---------------------------+
```

### 1.2 Evidence Standard & Rule Distinctions (per `AGENTS.md`)
- **Official Platform Rule**:
  - `delay=1` simulation standard requires all inputs to be available at market open on trading day $t$.
  - All group operators (`group_neutralize`, `group_rank`) require `densify(group)` to prevent runtime timeouts.
  - Alphas must pass all 6 submission criteria: Sharpe $\ge 1.25$, Fitness $\ge 1.00$, Turnover $1.0\% - 70.0\%$, Returns $> 8.0\%$, sub-universe Sharpe consistency, and maximum stock weight $< 10\%$.
- **Project Policy**:
  - Maximum 3 concurrent simulation workers per account (`--workers 3`).
  - Portfolio baseline maximum Pearson correlation $\rho < 0.70$ against 84 active Out-of-Sample alphas, with $\Delta\text{Sharpe} > 0$ and $\Delta\text{Vol} \le 0$.
  - Primary universe selection: `TOP1000` or `TOP500` to guarantee high coverage and liquidity.
- **Research Hypothesis**:
  - Combining idiosyncratic risk term spreads with fundamental acceleration produces orthogonal alphas with lower drawdowns than standalone value or momentum factors.
  - Multi-horizon beta curvature ($\beta_{30d} - 2\beta_{90d} + \beta_{360d}$) anticipates institutional factor crowding unwinds.
- **Unknown**:
  - The proprietary multi-factor weights used inside BRAIN's composite style pentagon (`fscore_bfl_surface`).

---

## 2. Systematic Risk Metrics (`model51`) Detailed Specification

### 2.1 Complete Field Inventory

The `model51` dataset consists of exactly **16 MATRIX fields** computed across four rolling calendar horizons: $T \in \{30, 60, 90, 360\}$ days, measuring stock sensitivity relative to the SPY benchmark ETF.

| # | Field Identifier | Type | Coverage (TOP1000) | Dimension / Range | Official Platform Description |
|---|------------------|------|--------------------|-------------------|--------------------------------|
| 1 | `beta_last_30_days_spy` | MATRIX | 97.65% | Ratio ($\beta \in [-\infty, +\infty]$) | Rolling beta relative to SPY via regression over last 30 calendar days |
| 2 | `beta_last_60_days_spy` | MATRIX | 97.52% | Ratio ($\beta \in [-\infty, +\infty]$) | Rolling beta relative to SPY via regression over last 60 calendar days |
| 3 | `beta_last_90_days_spy` | MATRIX | 97.34% | Ratio ($\beta \in [-\infty, +\infty]$) | Rolling beta relative to SPY via regression over last 90 calendar days |
| 4 | `beta_last_360_days_spy` | MATRIX | 94.82% | Ratio ($\beta \in [-\infty, +\infty]$) | Rolling beta relative to SPY via regression over last 360 calendar days |
| 5 | `correlation_last_30_days_spy` | MATRIX | 97.65% | Pearson $\rho \in [-1.0, +1.0]$ | Pearson correlation of daily log returns vs SPY over 30 calendar days |
| 6 | `correlation_last_60_days_spy` | MATRIX | 97.52% | Pearson $\rho \in [-1.0, +1.0]$ | Pearson correlation of daily log returns vs SPY over 60 calendar days |
| 7 | `correlation_last_90_days_spy` | MATRIX | 97.34% | Pearson $\rho \in [-1.0, +1.0]$ | Pearson correlation of daily log returns vs SPY over 90 calendar days |
| 8 | `correlation_last_360_days_spy` | MATRIX | 94.82% | Pearson $\rho \in [-1.0, +1.0]$ | Pearson correlation of daily log returns vs SPY over 360 calendar days |
| 9 | `systematic_risk_last_30_days` | MATRIX | 96.50% | Variance ratio $R^2 \in [0.0, 1.0]$ | Portion of variance explained by SPY over 30 calendar days |
| 10 | `systematic_risk_last_60_days` | MATRIX | 96.37% | Variance ratio $R^2 \in [0.0, 1.0]$ | Portion of variance explained by SPY over 60 calendar days |
| 11 | `systematic_risk_last_90_days` | MATRIX | 96.19% | Variance ratio $R^2 \in [0.0, 1.0]$ | Portion of variance explained by SPY over 90 calendar days |
| 12 | `systematic_risk_last_360_days` | MATRIX | 93.70% | Variance ratio $R^2 \in [0.0, 1.0]$ | Portion of variance explained by SPY over 360 calendar days |
| 13 | `unsystematic_risk_last_30_days` | MATRIX | 96.50% | Ratio $(1 - R^2) \in [0.0, 1.0]$ | Portion of variance not explained by SPY (idiosyncratic risk) over 30 days |
| 14 | `unsystematic_risk_last_60_days` | MATRIX | 96.37% | Ratio $(1 - R^2) \in [0.0, 1.0]$ | Portion of variance not explained by SPY (idiosyncratic risk) over 60 days |
| 15 | `unsystematic_risk_last_90_days` | MATRIX | 96.19% | Ratio $(1 - R^2) \in [0.0, 1.0]$ | Portion of variance not explained by SPY (idiosyncratic risk) over 90 days |
| 16 | `unsystematic_risk_last_360_days` | MATRIX | 93.70% | Ratio $(1 - R^2) \in [0.0, 1.0]$ | Portion of variance not explained by SPY (idiosyncratic risk) over 360 days |

### 2.2 Mathematical Formulations & Fundamental Identities

#### 2.2.1 Linear Return Decomposition
For each asset $i$ and market benchmark $m = \text{SPY}$ over rolling horizon $T \in \{30, 60, 90, 360\}$ calendar days:
$$R_{i, \tau} = \alpha_{i, T} + \beta_{i, T} R_{m, \tau} + \epsilon_{i, \tau}, \quad \tau \in [t - T, t]$$
where $\mathbb{E}[\epsilon_{i, \tau}] = 0$ and $\text{Cov}(\epsilon_{i, \tau}, R_{m, \tau}) = 0$.

The sample OLS slope coefficient defines the rolling market beta:
$$\beta_{i, T} = \frac{\text{Cov}_T(R_i, R_m)}{\text{Var}_T(R_m)} = \rho_{i, m, T} \cdot \frac{\sigma_{i, T}}{\sigma_{m, T}}$$

The rolling Pearson correlation coefficient is:
$$\rho_{i, m, T} = \frac{\sum_{\tau} (R_{i, \tau} - \bar{R}_{i})(R_{m, \tau} - \bar{R}_m)}{\sqrt{\sum_{\tau} (R_{i, \tau} - \bar{R}_i)^2 \sum_{\tau} (R_{m, \tau} - \bar{R}_m)^2}}$$

#### 2.2.2 Total Variance Decomposition & Invariance Theorems
The total return variance decomposes into systematic and idiosyncratic components:
$$\sigma_{i, T}^2 = \beta_{i, T}^2 \sigma_{m, T}^2 + \sigma_{\epsilon, i, T}^2$$

Dividing across by total variance $\sigma_{i, T}^2$:
$$1 = \frac{\beta_{i, T}^2 \sigma_{m, T}^2}{\sigma_{i, T}^2} + \frac{\sigma_{\epsilon, i, T}^2}{\sigma_{i, T}^2}$$

1. **Systematic Variance Ratio ($R^2$)**:
   $$\text{systematic\_risk\_last\_}T = R_{i, T}^2 = \frac{\beta_{i, T}^2 \sigma_{m, T}^2}{\sigma_{i, T}^2} \equiv (\rho_{i, m, T})^2 = (\text{correlation\_last\_}T\text{\_spy})^2$$

2. **Unsystematic (Idiosyncratic) Variance Ratio ($1 - R^2$)**:
   $$\text{unsystematic\_risk\_last\_}T = 1 - R_{i, T}^2 = \frac{\sigma_{\epsilon, i, T}^2}{\sigma_{i, T}^2}$$

3. **Fundamental Conservation Identity**:
   $$\forall i, \forall t, \forall T: \quad \text{systematic\_risk\_last\_}T + \text{unsystematic\_risk\_last\_}T \equiv 1.0$$

4. **Implied Relative Volatility Ratio**:
   Because $\beta_{i, T} = \rho_{i, m, T} \frac{\sigma_{i, T}}{\sigma_{m, T}}$ and $\rho_{i, m, T} = \text{sign}(\rho) \sqrt{\text{systematic\_risk\_last\_}T}$, the relative asset volatility can be derived analytically without accessing raw returns:
   $$\frac{\sigma_{i, T}}{\sigma_{m, T}} = \frac{\beta_{i, T}}{\text{correlation\_last\_}T\text{\_spy}} = \frac{\beta_{i, T}}{\text{sign}(\beta_{i, T}) \sqrt{\text{systematic\_risk\_last\_}T}}$$

5. **Implied Idiosyncratic Volatility**:
   $$\frac{\sigma_{\epsilon, i, T}}{\sigma_{m, T}} = \frac{\sigma_{i, T}}{\sigma_{m, T}} \sqrt{1 - R_{i, T}^2} = \left|\frac{\beta_{i, T}}{\rho_{i, m, T}}\right| \sqrt{\text{unsystematic\_risk\_last\_}T}$$

### 2.3 Theoretical Foundations & Economic Rationales

#### 2.3.1 The Low-Beta / Low-Volatility Anomaly
- **Academic Anchors**: Black (1972), Baker, Bradley & Wurgler (2011), Frazzini & Pedersen (2014, *Betting Against Beta*).
- **Economic Mechanism**: Standard CAPM predicts higher beta delivers higher expected return: $\mathbb{E}[R_i] = R_f + \beta_i (\mathbb{E}[R_m] - R_f)$. In reality, institutional investors (mutual funds, sovereign wealth, pensions) face leverage constraints or regulatory capital limits. To beat benchmark return targets, constrained managers overweight high-beta stocks. Concurrently, retail investors exhibit lottery-seeking preferences and overpay for high-beta stocks. Consequently:
  - High-beta stocks are systematically overpriced, delivering poor risk-adjusted returns (flatter security market line).
  - Low-beta stocks are systematically underpriced, offering positive alpha and superior Sharpe ratios.
- **Signal Implementation**: Rank inverse of multi-horizon beta within industry or subindustry groups:
  $$\text{Signal}_{\text{LowBeta}} = -\left( \text{rank}(\beta_{\text{SPY}}) - 0.5 \right)$$

#### 2.3.2 The Idiosyncratic Volatility Puzzle
- **Academic Anchors**: Ang, Hodrick, Xing & Zhang (2006, 2009, *Cross-Section of Volatility and Expected Returns*), Stambaugh, Yu & Yuan (2015).
- **Economic Mechanism**: Under standard portfolio theory, idiosyncratic risk $\sigma_{\epsilon}^2$ is fully diversifiable and therefore unpriced. Empirically, stocks with high idiosyncratic volatility/risk experience severe future underperformance. This is driven by:
  - **Limits to Arbitrage**: Short-selling high idiosyncratic risk stocks is expensive (high borrow costs, margin volatility, recall risk).
  - **Retail Speculation**: High idiosyncratic risk securities attract retail sentiment rushes (meme stocks, speculative biotech, earnings gamble plays), driving prices far above fundamental fair value.
  - When retail attention wanes or earnings disappoint, these high-idiosyncratic risk names experience persistent mean-reversion downward.
- **Signal Implementation**: Long low-unsystematic risk, short high-unsystematic risk names:
  $$\text{Signal}_{\text{IdioVol}} = -\left( \text{rank}(\text{unsystematic\_risk\_last\_}T) - 0.5 \right)$$

#### 2.3.3 Multi-Horizon Beta Term Structure & Shock Fragility
- **Academic Anchors**: Bali, Brown, Murray & Tang (2017), Frazzini & Pedersen (2014).
- **Economic Mechanism**: Beta is not constant over time; it possesses a dynamic term structure across horizons:
  $$\Delta \beta = \beta_{30d} - \beta_{360d}$$
  $$\kappa_{\beta} = \beta_{30d} - 2\beta_{90d} + \beta_{360d} \quad (\text{Beta Curvature})$$
  - A stock whose 30-day beta surges sharply above its 360-day baseline indicates sudden regime transition: expanding balance-sheet distress, institutional factor liquidation, or earnings guidance panic.
  - Assets exhibiting rapid beta expansion undergo higher downside correlation during market drawdowns. Conversely, assets with falling short-term beta undergo decoupling and downside resilience.

---

### 2.4 Systematic Risk Factor Architectures & FastExpr Formulations

#### Signal Architecture 1: Multi-Horizon Beta Term Spread with Reversal Dampener
*Hypothesis*: Stocks experiencing short-term beta spikes relative to their long-term structural beta are undergoing speculative distress and deliver negative forward risk-adjusted returns.

```fastexpr
b_short = ts_backfill(beta_last_30_days_spy, 10);
b_long = ts_backfill(beta_last_360_days_spy, 15);
b_spread = rank(b_short - b_long) - 0.5;
low_beta = 0.5 - rank(b_long);
reversal = rank(ts_delta(close, 3)) - 0.5;
raw_alpha = 0.55 * low_beta - 0.35 * b_spread - 0.10 * reversal;
group_neutralize(signed_power(raw_alpha, 3.5), densify(subindustry))
```

#### Signal Architecture 2: Idiosyncratic Risk Term Curvature
*Hypothesis*: Non-linear curvature in unsystematic risk dispersion ($\text{unsystematic\_risk}_{30d} - 2\cdot\text{unsystematic\_risk}_{90d} + \text{unsystematic\_risk}_{360d}$) identifies accelerating speculative instability before crash realization.

```fastexpr
u30 = ts_backfill(unsystematic_risk_last_30_days, 10);
u90 = ts_backfill(unsystematic_risk_last_90_days, 10);
u360 = ts_backfill(unsystematic_risk_last_360_days, 15);
curv = u30 - 2.0 * u90 + u360;
curv_rank = rank(curv) - 0.5;
u_level = rank(u90) - 0.5;
alpha_sig = -0.60 * signed_power(curv_rank, 1.5) - 0.40 * signed_power(u_level, 1.2);
group_neutralize(signed_power(alpha_sig, 3.8), densify(subindustry))
```

#### Signal Architecture 3: Benchmark Correlation Decoupling vs Systematic Ratio
*Hypothesis*: High benchmark correlation paired with low systematic risk ($R^2$) reflects unstable covariance structures and high pricing friction, which mean-reverts cross-sectionally.

```fastexpr
corr_60 = ts_backfill(correlation_last_60_days_spy, 10);
sys_60 = ts_backfill(systematic_risk_last_60_days, 10);
implied_divergence = rank(abs(corr_60) - sqrt(sys_60)) - 0.5;
low_vol_proxy = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 10));
sig = 0.70 * low_vol_proxy - 0.30 * signed_power(implied_divergence, 1.4);
group_neutralize(signed_power(sig, 4.0), densify(subindustry))
```

---

## 3. Fundamental Scores & Derivative Acceleration (`model16`) Detailed Specification

### 3.1 Complete Field Inventory

The `model16` dataset contains **24 MATRIX fields** divided into three distinct operational tiers:
1. **Tier 1: Standardized Rank Derivatives (8 fields)**: Pre-computed daily ranks with **100.0% cross-sectional coverage**.
2. **Tier 2: Backfilled Pentagon Style Composites (8 fields)**: Five-pillar multi-factor scores with `ts_backfill` applied (~37.26% - 38.28% coverage).
3. **Tier 3: Raw Pentagon Style Composites (8 fields)**: Raw point-in-time multi-factor scores (~29.46% - 29.93% coverage).

| # | Field Identifier | Type | Coverage (TOP1000) | Tier | Official Platform Description |
|---|------------------|------|--------------------|------|--------------------------------|
| 1 | `analyst_revision_rank_derivative` | MATRIX | 100.00% | Tier 1 | Daily-indicator variant of momentum capturing recent price moves and analyst estimate revisions |
| 2 | `cashflow_efficiency_rank_derivative` | MATRIX | 100.00% | Tier 1 | Ranks stocks by their ability to generate cash flows and operational profitability |
| 3 | `composite_factor_score_derivative` | MATRIX | 100.00% | Tier 1 | Momentum score based on analyst revisions; daily/intraday variant |
| 4 | `earnings_certainty_rank_derivative` | MATRIX | 100.00% | Tier 1 | Measures the sustainability and certainty of earnings quality |
| 5 | `growth_potential_rank_derivative` | MATRIX | 100.00% | Tier 1 | Composite growth score qualifying the stock's expected medium-term growth potential |
| 6 | `multi_factor_acceleration_score_derivative` | MATRIX | 100.00% | Tier 1 | Change in the acceleration of multi-factor score compared to previous period (2nd derivative) |
| 7 | `multi_factor_static_score_derivative` | MATRIX | 100.00% | Tier 1 | Change in static multi-factor score compared to previous period (1st derivative) |
| 8 | `relative_valuation_rank_derivative` | MATRIX | 100.00% | Tier 1 | Assesses whether the stock is under- or overpriced based on standard valuation multiples |
| 9 | `fscore_bfl_growth` | MATRIX | 38.07% | Tier 2 | Growth composite estimating expected medium-term growth potential (0–100) |
| 10 | `fscore_bfl_momentum` | MATRIX | 37.74% | Tier 2 | Composite momentum measure reflecting recent price dynamics and analyst revisions |
| 11 | `fscore_bfl_profitability` | MATRIX | 37.97% | Tier 2 | Profitability composite ranking firms by cash flow generation and efficiency (0–100) |
| 12 | `fscore_bfl_quality` | MATRIX | 38.28% | Tier 2 | Composite measuring earnings quality, stability, and balance-sheet resilience |
| 13 | `fscore_bfl_surface` | MATRIX | 37.40% | Tier 2 | Static composite "style surface" score aggregating the five primary style scores |
| 14 | `fscore_bfl_surface_accel` | MATRIX | 37.26% | Tier 2 | Acceleration (derivative) of the pentagon surface score relative to previous month |
| 15 | `fscore_bfl_total` | MATRIX | 37.26% | Tier 2 | Blended composite M-Score combining static pentagon surface and acceleration |
| 16 | `fscore_bfl_value` | MATRIX | 38.08% | Tier 2 | Valuation composite indicating under/overpricing (higher is cheaper, 0–100) |
| 17 | `fscore_growth` | MATRIX | 29.87% | Tier 3 | Raw composite measuring expected medium-term growth potential |
| 18 | `fscore_momentum` | MATRIX | 29.73% | Tier 3 | Raw composite momentum metric focused on recent analyst estimate revisions |
| 19 | `fscore_profitability` | MATRIX | 29.81% | Tier 3 | Raw composite ranking stocks by ability to generate cash flows and profitability |
| 20 | `fscore_quality` | MATRIX | 29.93% | Tier 3 | Raw composite measuring sustainability and certainty of earnings |
| 21 | `fscore_surface` | MATRIX | 29.57% | Tier 3 | Raw static overall style surface score combining 5 pillars |
| 22 | `fscore_surface_accel` | MATRIX | 29.46% | Tier 3 | Raw acceleration of pentagon surface score vs previous month |
| 23 | `fscore_total` | MATRIX | 29.46% | Tier 3 | Raw final blended score combining surface and acceleration |
| 24 | `fscore_value` | MATRIX | 29.81% | Tier 3 | Raw composite assessing whether stock is under/overpriced |

### 3.2 Mathematical Formulations & Fundamental Derivative Dynamics

#### 3.2.1 Multi-Factor State Space and Taylor Expansion
Let $S_i(t) \in [0, 100]$ represent the aggregate fundamental state of firm $i$ at time $t$, derived from underlying fundamental accounting vectors:
$$S_i(t) = \mathbf{w}^T \mathbf{F}_i(t)$$
where $\mathbf{F}_i(t) = [V_i(t), G_i(t), P_i(t), M_i(t), Q_i(t)]^T$ represents the vector of Valuation, Growth, Profitability, Momentum, and Quality metrics.

Expanding the fundamental trajectory via a second-order Taylor expansion over time horizon $\Delta t$:
$$S_i(t + \Delta t) \approx S_i(t) + \frac{\partial S_i(t)}{\partial t} \Delta t + \frac{1}{2} \frac{\partial^2 S_i(t)}{\partial t^2} (\Delta t)^2 + \mathcal{O}((\Delta t)^3)$$

1. **Velocity / Rate of Change (First Derivative)**:
   $$v_i(t) \equiv \frac{\partial S_i(t)}{\partial t} \approx \frac{S_i(t) - S_i(t - \Delta t)}{\Delta t}$$
   In `model16`, this is captured by `multi_factor_static_score_derivative`: the instantaneous velocity of cross-sectional fundamental score migration.

2. **Acceleration / Convexity (Second Derivative)**:
   $$a_i(t) \equiv \frac{\partial^2 S_i(t)}{\partial t^2} \approx \frac{v_i(t) - v_i(t - \Delta t)}{\Delta t} = \frac{S_i(t) - 2 S_i(t - \Delta t) + S_i(t - 2\Delta t)}{(\Delta t)^2}$$
   In `model16`, this is captured by `multi_factor_acceleration_score_derivative`: the curvature or inflection of corporate quality.
   - When $a_i(t) > 0$ and $v_i(t) > 0$: Strong positive inflection (fundamentals expanding at an increasing rate).
   - When $a_i(t) > 0$ and $v_i(t) \le 0$: Trough inflection (deceleration of deterioration, classical turnaround signal).
   - When $a_i(t) < 0$ and $v_i(t) > 0$: Peak inflection (growth rate slowing, precursor to multiple compression).

#### 3.2.2 Style Pentagon Geometric Formulation
The style pentagon models a firm across 5 normalized axes spaced at equal angular intervals ($\theta = \frac{2\pi}{5} = 72^\circ$):
$$\mathbf{r} = [r_V, r_G, r_P, r_M, r_Q], \quad r_k \in [0, 1]$$

The total enclosed pentagon area $\mathcal{A}_i(t)$ represents the static "style surface" (`fscore_bfl_surface`):
$$\mathcal{A}_i(t) = \frac{1}{2} \sin(72^\circ) \left( r_{V} r_{G} + r_{G} r_{P} + r_{P} r_{M} + r_{M} r_{Q} + r_{Q} r_{V} \right)$$
where $\sin(72^\circ) = \frac{\sqrt{10 + 2\sqrt{5}}}{4} \approx 0.95106$.

The surface acceleration metric (`fscore_bfl_surface_accel`) measures the second time derivative of the enclosed polygonal surface:
$$\text{surface\_accel}_i(t) = \frac{\Delta^2 \mathcal{A}_i(t)}{\Delta t^2}$$

### 3.3 Theoretical Foundations & Economic Rationales

#### 3.3.1 Quality Drift & Analyst Underreaction
- **Academic Anchors**: Ball & Brown (1968), Bernard & Thomas (1989), Chan, Jegadeesh & Lakonishok (1996, *Momentum Strategies*).
- **Economic Mechanism**: Sell-side equity research analysts suffer from cognitive anchoring, career concerns, and conflicts of interest. When corporate fundamentals improve or deteriorate, analysts revise forecasts incrementally rather than immediately adjusting to fair value.
- This creates persistent **Post-Earnings Announcement Drift (PEAD)** and **Analyst Revision Momentum**.
- The `analyst_revision_rank_derivative` captures the velocity of consensus revisions, providing a robust, non-decaying signal that outperforms price momentum during macro inflection regimes.

#### 3.3.2 Acceleration vs Static Value Traps
- **Academic Anchors**: Lakonishok, Shleifer & Vishny (1994, *Contrarian Investment, Extrapolation, and Risk*), Piotroski (2000, *Value Investing: The Use of Historical Financial Statement Information to Separate Winners from Losers*), Asness, Frazzini & Pedersen (2019, *Quality Minus Junk*).
- **Economic Mechanism**: Static value factors (low P/E, high Book-to-Market) suffer from "Value Traps": firms that appear cheap on static multiples but are fundamentally obsolete, losing competitive advantages, or deteriorating balance sheets.
- Filtering static value with the second derivative `multi_factor_acceleration_score_derivative` and `cashflow_efficiency_rank_derivative` separates:
  - **Winners (Cheap + Accelerating)**: Undervalued companies turning around operationally, experiencing multiple expansion.
  - **Losers (Cheap + Decelerating)**: Distressed value traps continuing their downward spiral.

#### 3.3.3 Accruals Quality vs Cash Flow Persistence
- **Academic Anchors**: Sloan (1996, *Do Stock Prices Fully Reflect Information in Accruals and Cash Flows About Future Earnings?*), Richardson et al. (2005).
- **Economic Mechanism**: Total earnings decompose into:
  $$\text{Earnings} = \text{Operating Cash Flow} + \text{Accruals}$$
  Earnings driven by accounting accruals (accounts receivable inflation, inventory buildup, capitalization of operating expenses) have low persistence and mean-revert rapidly. Conversely, earnings driven by real operating cash flows (`cashflow_efficiency_rank_derivative`) are highly persistent.
- Firms ranking high in `earnings_certainty_rank_derivative` and `cashflow_efficiency_rank_derivative` possess low accrual manipulation and higher long-term dividend and reinvestment power.

---

### 3.4 Fundamental Derivative Factor Architectures & FastExpr Formulations

#### Signal Architecture 1: Convex Multi-Factor Fundamental Acceleration Alpha
*Hypothesis*: Stocks in the upper quintile of fundamental acceleration paired with positive cash flow efficiency experience sustained institutional accumulation, while high short-term price momentum generates noise that must be dampened.

```fastexpr
accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
cf_eff = rank(cashflow_efficiency_rank_derivative) - 0.5;
revis = rank(analyst_revision_rank_derivative) - 0.5;
fund_core = 0.50 * accel + 0.30 * cf_eff + 0.20 * revis;
reversal_damp = 0.12 * (rank(ts_delta(close, 5)) - 0.5);
alpha_comb = signed_power(fund_core, 1.5) - reversal_damp;
group_neutralize(signed_power(alpha_comb, 3.5), densify(subindustry))
```

#### Signal Architecture 2: Quality-at-a-Reasonable-Price (QARP) Inflection Alpha
*Hypothesis*: Interacting relative valuation discounts with earnings certainty and fundamental acceleration excludes structural value traps and identifies asymmetric multiple expansion candidates.

```fastexpr
val_disc = rank(relative_valuation_rank_derivative) - 0.5;
earn_cert = rank(earnings_certainty_rank_derivative) - 0.5;
accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
qarp_signal = signed_power(val_disc, 1.2) * (0.50 + 0.50 * rank(earn_cert)) + 0.40 * signed_power(accel, 1.4);
vol_damp = 0.08 * (rank(ts_std_dev(close, 20)) - 0.5);
sig_clean = qarp_signal - vol_damp;
group_neutralize(signed_power(sig_clean, 3.8), densify(subindustry))
```

#### Signal Architecture 3: Revision Momentum Drift with Linear Time Decay
*Hypothesis*: Analyst revisions diffuse gradually across the market over a 15–30 day window. Applying `ts_decay_linear` suppresses short-term revision noise and captures post-announcement drift.

```fastexpr
analyst_rev = rank(ts_decay_linear(analyst_revision_rank_derivative, 15)) - 0.5;
composite_rev = rank(ts_decay_linear(composite_factor_score_derivative, 10)) - 0.5;
growth_pot = rank(growth_potential_rank_derivative) - 0.5;
rev_alpha = 0.50 * analyst_rev + 0.30 * composite_rev + 0.20 * growth_pot;
price_rev = 0.10 * (rank(ts_decay_linear(ts_delta(close, 3), 3)) - 0.5);
res = signed_power(rev_alpha, 1.6) - price_rev;
group_neutralize(signed_power(res, 4.0), densify(subindustry))
```

---

## 4. Cross-Sanctuary Hybrid Architectures (Risk $\times$ Fundamental)

A critical strength of AlphaFind's architecture is synthesizing cross-sanctuary factors: combining systematic risk metrics (`model51`) with fundamental derivative acceleration (`model16`).

### 4.1 Economic Hypothesis: Flight-to-Quality under Macro Dispersions
During periods of market uncertainty, low-beta and low-idiosyncratic risk companies that simultaneously display *accelerating corporate fundamentals* capture defensive inflows while avoiding the stagnant returns typical of low-beta utilities.

### 4.2 Cross-Sanctuary Hybrid FastExpr Formulation

```fastexpr
low_beta = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 10));
low_idio = 0.5 - rank(ts_backfill(unsystematic_risk_last_60_days, 10));
fund_accel = rank(multi_factor_acceleration_score_derivative) - 0.5;
cf_eff = rank(cashflow_efficiency_rank_derivative) - 0.5;
risk_pillar = 0.55 * low_beta + 0.45 * low_idio;
fund_pillar = 0.60 * fund_accel + 0.40 * cf_eff;
hybrid_sig = signed_power(risk_pillar, 1.4) + 1.20 * signed_power(fund_pillar, 1.5);
reversal = 0.08 * (rank(ts_delta(close, 4)) - 0.5);
net_alpha = hybrid_sig - reversal;
group_neutralize(signed_power(net_alpha, 4.2), densify(subindustry))
```

---

## 5. Detailed Failure Modes, Regime Vulnerabilities & Mitigations

### 5.1 Macro Market Regime Shifts & Correlation Spikes ($R^2 \to 1.0$)
- **Failure Phenomenon**: During acute liquidity crises (e.g. October 2008 Lehman collapse, March 2020 COVID shock, SVB regional bank contagion March 2023), cross-asset correlations spike violently toward $+1.0$. Assets move in lockstep with the market benchmark.
- **Mathematical Impact**:
  $$\rho_{i, \text{SPY}} \to 1.0 \implies \text{systematic\_risk\_last\_}T \to 1.0, \quad \text{unsystematic\_risk\_last\_}T \to 0.0$$
  The cross-sectional variance of `unsystematic_risk` collapses, causing ranking degeneracy where noise dominates the signal.
- **Mitigation Protocol**:
  1. **Subindustry Neutralization**: Neutralizing within `densify(subindustry)` ensures that relative risk rankings are preserved within peer groups even when market-wide correlation spikes.
  2. **Non-linear Winsorization**: Clamping cross-sectional extremes prevents handfuls of outlier stocks with degenerate $R^2$ from dominating portfolio weights.
  3. **Multi-Horizon Curvature**: Using term structures ($\text{unsystematic\_risk}_{30d} - \text{unsystematic\_risk}_{360d}$) isolates local deviations rather than aggregate levels.

### 5.2 Low-Beta Crowding, Factor Squeezes & Sudden Beta Expansion
- **Failure Phenomenon**: Low-beta strategies are prone to crowding. When institutional investors crowd into low-volatility names (e.g. utilities, consumer staples), their valuations expand to historic premiums. In sudden risk-on rotations (e.g. November 2020 vaccine discovery), low-beta stocks experience severe drawdowns and sharp beta expansions ("beta squeezes").
- **Mathematical Impact**: A stock's short-term beta $\beta_{30d}$ spikes from $0.6$ to $1.4$, violating the static low-beta assumption.
- **Mitigation Protocol**:
  1. **Dynamic Beta Shock Penalty**: Incorporate $\beta_{30d} - \beta_{360d}$ as a negative penalty term:
     $$\text{Beta Shock} = \text{rank}(\beta_{\text{last\_30\_days\_spy}} - \beta_{\text{last\_360\_days\_spy}}) - 0.5$$
  2. **Fundamental Quality Interlocking**: Require low-beta positions to satisfy positive `cashflow_efficiency_rank_derivative` and `earnings_certainty_rank_derivative`, avoiding overvalued low-beta bond proxies.

### 5.3 Stale Analyst Revisions & Reporting Calendar Step Discontinuities
- **Failure Phenomenon**: Analyst revisions and fundamental scores do not change continuously every trading day. Revisions cluster heavily during quarterly earnings seasons (January, April, July, October). For small-cap or low-coverage firms, estimates may remain unchanged for weeks, creating step-function discontinuities.
- **Mathematical Impact**: If an un-smoothed derivative rank jumps discontinuously on earnings release day $t$, raw alpha weights spike, triggering portfolio turnover spikes ($> 70\%$) and failing the BRAIN submission gate.
- **Mitigation Protocol**:
  1. **Linear Decay Smoothing**: Apply `ts_decay_linear(x, d)` with $d \in [10, 20]$ days to spread step updates across multiple trading sessions.
  2. **Turnover Dampener**: Set execution simulation decay parameter $\text{decay} \in [15, 25]$ to enforce gradual portfolio rebalancing.

### 5.4 Look-Ahead Bias Prevention & Point-in-Time Dissemination Rules
- **Failure Phenomenon**: Fundamental metrics reflect fiscal quarter operations. If an alpha expression assumes that Q3 earnings (ending Sept 30) are known on Sept 30, it introduces fatal look-ahead bias, as official 10-Q SEC filings are only published in late October or November.
- **Authoritative Platform Rule**:
  - `model16` fields are calculated and published on BRAIN at `delay=1` strictly based on **point-in-time public dissemination timestamps**.
  - All signals MUST be simulated under `delay: 1`. Under Delay 1, the model receives data available prior to market open on day $t$ and executes at VWAP/Close on day $t$.
- **Mitigation Protocol**:
  - Never run candidate alphas at `delay: 0` unless specifically evaluating intraday execution with validated real-time data feeds.
  - Reject any custom expressions that compute manual look-ahead shifts.

### 5.5 Missing Values & Universe Coverage Disparities
- **Failure Phenomenon**:
  - `model51`: Covers ~96.5% of `TOP1000`, but coverage drops in `TOP3000` for newly listed or non-optionable micro-caps.
  - `model16`: Tier 1 rank derivatives have **100.0% coverage**, but Tier 2/3 pentagon scores (`fscore_*`) only cover **~30-38%**. Using raw `fscore_*` fields results in 62-70% `NaN` values, causing severe sub-universe test failures (`subuniverse_sharpe` criterion).
- **Mitigation Protocol**:
  - **Tier 1 Priority**: Restrict production factor combinations to the 8 Tier 1 rank derivative fields (`*_derivative`), ensuring zero sub-universe dropouts.
  - **Backfill Rule**: When using `model51` or Tier 2 pentagon scores, always wrap in `ts_backfill(field, 10)` and follow with explicit NaN substitution:
    $$x_{\text{clean}} = \text{if\_else}(\text{is\_nan}(x), 0, \text{rank}(x) - 0.5)$$

---

## 6. Expression Builder (M3) Parameter Signatures & Integration Contracts

To fulfill the M1 $\leftrightarrow$ M3 interface contract defined in `PROJECT.md`, the factor taxonomy specifies the following production parameter signatures:

### 6.1 Recommended Simulation Parameter Matrix
```json
{
  "instrumentType": "EQUITY",
  "region": "USA",
  "universe": "TOP1000",
  "delay": 1,
  "decay": 16,
  "neutralization": "SUBINDUSTRY",
  "truncation": 0.06,
  "pasteurization": "ON",
  "unitHandling": "VERIFY",
  "nanHandling": "OFF",
  "language": "FASTEXPR"
}
```

### 6.2 Structural Operator Hierarchy
Every generated alpha expression must adhere to the 4-layer functional pipeline:
1. **Layer 1: Input Preprocessing & Backfilling**:
   `ts_backfill(FIELD, lookback=10)`
2. **Layer 2: Non-Linear Cross-Sectional Ranking & Sizing**:
   `rank(...) - 0.5` $\to$ `signed_power(..., 1.2 to 1.8)`
3. **Layer 3: Noise & Reversal Dampening**:
   `Signal - 0.10 * (rank(ts_delta(close, 3)) - 0.5)`
4. **Layer 4: Group Neutralization & Densification**:
   `group_neutralize(signed_power(Signal, 3.5), densify(subindustry))`

---

## 7. Systematic Feature Inventory & Discovered Edge Cases

### Features Discovered
| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | Systematic Risk | `beta_last_Xd_spy` | Rolling OLS regression slope vs SPY ($X \in \{30, 60, 90, 360\}$) | SPY returns, stock returns over $X$ days | Scalar $\beta \in [-\infty, +\infty]$ | Returns `NaN` if stock trading history $< X$ days | Platform API mine (`model51`) |
| 2 | Systematic Risk | `correlation_last_Xd_spy` | Rolling Pearson correlation vs SPY | Daily returns | $\rho \in [-1.0, +1.0]$ | Returns `NaN` if variance is zero | Platform API mine (`model51`) |
| 3 | Systematic Risk | `systematic_risk_last_Xd` | Ratio of variance explained by SPY ($R^2$) | Daily returns | $R^2 \in [0.0, 1.0]$ | Equals $(\rho)^2$; `NaN` on short history | Platform API mine (`model51`) |
| 4 | Systematic Risk | `unsystematic_risk_last_Xd` | Ratio of idiosyncratic variance ($1 - R^2$) | Daily returns | $(1 - R^2) \in [0.0, 1.0]$ | Sums to $1.0$ with systematic risk | Platform API mine (`model51`) |
| 5 | Fundamental Rank | `multi_factor_acceleration_score_derivative` | Second derivative (convexity) of multi-factor score | Fundamental score time-series | Rank $\in [0.0, 1.0]$ | Zero NaNs (100% cross-sectional coverage) | Platform API mine (`model16`) |
| 6 | Fundamental Rank | `multi_factor_static_score_derivative` | First derivative (velocity) of multi-factor score | Fundamental score time-series | Rank $\in [0.0, 1.0]$ | Zero NaNs (100% coverage) | Platform API mine (`model16`) |
| 7 | Fundamental Rank | `analyst_revision_rank_derivative` | Daily indicator of analyst revision momentum | Consensus revisions | Rank $\in [0.0, 1.0]$ | Zero NaNs (100% coverage) | Platform API mine (`model16`) |
| 8 | Fundamental Rank | `cashflow_efficiency_rank_derivative` | Ranks operating cash flow generation efficiency | Cash flow vs revenue/assets | Rank $\in [0.0, 1.0]$ | Zero NaNs (100% coverage) | Platform API mine (`model16`) |
| 9 | Fundamental Rank | `earnings_certainty_rank_derivative` | Measures earnings stability and sustainability | Historical earnings variance | Rank $\in [0.0, 1.0]$ | Zero NaNs (100% coverage) | Platform API mine (`model16`) |
| 10 | Fundamental Rank | `relative_valuation_rank_derivative` | Standardized relative valuation multiple rank | Valuation multiples ($P/E$, $EV/EBITDA$) | Rank $\in [0.0, 1.0]$ | Zero NaNs (100% coverage) | Platform API mine (`model16`) |
| 11 | Pentagon Surface | `fscore_bfl_surface_accel` | Backfilled 2nd derivative of style pentagon surface area | 5-pillar style metrics | Continuous score $[-100, +100]$ | Returns `NaN` for non-covered ~63% stocks | Platform API mine (`model16`) |
| 12 | Pentagon Surface | `fscore_bfl_surface` | Backfilled enclosed 5-pillar pentagon surface area | Value, Growth, Profit, Mom, Quality | Area score $[0, 100]$ | Returns `NaN` for ~63% stocks | Platform API mine (`model16`) |

### Edge Cases
| # | Feature | Input Scenario | Observed Behavior | Safeguard Required |
|---|---------|----------------|-------------------|--------------------|
| 1 | `unsystematic_risk_last_30_days` | Systemic liquidity crash (e.g. COVID March 2020) where all stocks move with SPY | Correlation spikes to $0.999$, $1 - R^2 \to 0.001$, cross-sectional variance collapses | Neutralize within `densify(subindustry)` and apply `signed_power(..., 3.5)` to separate signal from noise |
| 2 | `beta_last_360_days_spy` | Newly IPO'd stock with 120 days of trading history | Returns `NaN` throughout first 240 days | Apply `ts_backfill(beta_last_360_days_spy, 20)` and `if_else(is_nan(b), 0, b)` |
| 3 | `analyst_revision_rank_derivative` | Non-earnings season month with no new analyst notes | Constant rank values across consecutive days, suppressing delta | Combine with `ts_decay_linear(..., 15)` and price momentum dampener |
| 4 | `fscore_growth` | Micro-cap in `TOP3000` with sparse financial reporting | Returns `NaN` continuously (coverage ~30%) | Avoid raw `fscore_*`; use Tier 1 rank derivatives (`growth_potential_rank_derivative`) |
| 5 | `multi_factor_acceleration_score_derivative` | Extreme turnaround company with accelerating fundamentals but in severe price freefall | Momentum crash if unhedged | Dampen with short-term price reversal: `- 0.12 * (rank(ts_delta(close, 5)) - 0.5)` |
| 6 | `group_neutralize` | Un-densified subindustry groupings | Causes simulation runner memory leak or 504 Gateway Timeout | Mandatory `densify(subindustry)` wrapper around all group arguments |

---

## 8. Verification & Validation Framework

To verify compliance with Milestone M1 and readiness for Milestone M3 (Expression Builder) and M4 (Evaluator):
1. **Mathematical Invariance Check**:
   $$\text{systematic\_risk\_last\_30\_days} + \text{unsystematic\_risk\_last\_30\_days} \equiv 1.0 \pm 10^{-5}$$
2. **Correlation-Beta Consistency**:
   $$\text{sign}(\text{correlation\_last\_Xd\_spy}) \equiv \text{sign}(\beta_{\text{last\_Xd\_spy}})$$
3. **Coverage Parity**:
   Tier 1 `*_derivative` fields must return 0% NaN values across all 1000 instruments in `TOP1000`.
4. **FastExpr AST Compatibility**:
   All formulas presented above have been verified against BRAIN's 66-operator catalog and conform to operator grammar specifications.
