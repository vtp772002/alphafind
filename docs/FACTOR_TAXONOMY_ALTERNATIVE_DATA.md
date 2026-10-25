# WorldQuant BRAIN Alternative Data Factor Taxonomy & Mathematical Specifications

**Document Version**: 1.0.0  
**Target Platform**: WorldQuant BRAIN FastExpr Engine  
**Datasets Covered**: `model51` (Systematic Risk), `model16` (Fundamental Scores & Derivative Accel), `news18` (RavenPack News), `option9` (Options Analytics)  
**Research Standard**: Strict Academic Rigor & AGENTS.md Git Hygiene Compliance  

---

## 1. Executive Summary & Taxonomy Architecture

### 1.1 Scope & Purpose
Quantitative alpha research on WorldQuant BRAIN historically concentrates on price-volume technical indicators and static accounting multiples. Because thousands of global quants evaluate identical data feeds, alphas constructed exclusively from standard pricing inputs experience severe alpha decay, high correlation against the crowd, and low Uniqueness scores on the Challenge leaderboard.

This specification details an advanced mathematical framework for four premier WorldQuant BRAIN **Alternative Data** datasets:
1. **`model51` (Systematic Risk Metrics)**: Quantifies cross-sectional variance decomposition, multi-horizon rolling market beta, benchmark correlation, and idiosyncratic risk dispersion.
2. **`model16` (Fundamental Scores & Derivative Acceleration)**: Captures high-coverage rank derivatives, fundamental acceleration (second time derivatives), cashflow efficiency, and style pentagon geometry.
3. **`news18` (RavenPack News Data)**: Unstructured real-time NLP streams capturing sentiment divergence, M&A and corporate action catalysts, earnings evaluation shocks, and vector event streams.
4. **`option9` (Options Analytics Surface)**: End-of-day options surface metrics across 12 constant expiration horizons, measuring breakeven directional asymmetry, synthetic forward price parity dislocation, and put-call term skews.

### 1.2 Factor Pillar Taxonomy Architecture
AlphaFind organizes alternative signals into an orthogonal multi-pillar matrix. Each pillar represents a distinct economic transmission mechanism, operating across varying information velocities:

```
+-------------------------------------------------------------------------------------------------------------+
|                                    ALTERNATIVE DATA FACTOR TAXONOMY                                         |
+----------------------+--------------------+---------------+--------------------+----------------------------+
| Pillar               | Dataset ID         | Tensor Type   | Horizon / Half-Life| Economic Mechanism         |
+----------------------+--------------------+---------------+--------------------+----------------------------+
| SystematicRisk       | model51            | 16 MATRIX     | 30d – 360d         | Low-Beta Anomaly,          |
|                      |                    |               | (Slow / Monthly)   | Idiosyncratic Vol Puzzle   |
+----------------------+--------------------+---------------+--------------------+----------------------------+
| FundamentalScores    | model16            | 24 MATRIX     | 10d – 60d          | Fundamental Acceleration,  |
|                      |                    | (8 Deriv 100%)| (Medium / Bi-wkly) | Value Trap Mitigation      |
+----------------------+--------------------+---------------+--------------------+----------------------------+
| NewsSentiment        | news18             | 71 MATRIX     | 1d – 15d           | Narrative Divergence,      |
|                      |                    | 50 VECTOR     | (Fast / Daily)     | Post-News Drift (PEAD)     |
+----------------------+--------------------+---------------+--------------------+----------------------------+
| OptionsAnalytics     | option9            | 74 MATRIX     | 10d – 1080d        | Asymmetric Informed Flow,  |
|                      |                    |               | (Fast / Daily)     | Forward Parity Squeeze     |
+----------------------+--------------------+---------------+--------------------+----------------------------+
```

### 1.3 Platform Compliance Baseline & Submission Safeguards
All factor formulations in this taxonomy are engineered to satisfy WorldQuant BRAIN submission criteria under standard execution settings:
- **Simulation Delay**: Strictly `delay = 1` (inputs available before market open on day $t$; execution at day $t$ close/VWAP).
- **Universe Selection**: Primary focus on `TOP1000` and `TOP500` for high liquidity and complete cross-sectional coverage; `TOP3000` evaluated with explicit backfilling safeguards.
- **In-Sample Thresholds**:
  - Information Ratio / Sharpe Ratio: $\text{Sharpe} \ge 1.25$
  - Fitness Criterion: $\text{Fitness} \ge 1.00$
  - Annualized Returns: $\text{Returns} > 8.0\%$
  - Annualized Turnover: $1.0\% \le \text{Turnover} \le 70.0\%$
  - Sub-universe Consistency: Positive Sharpe across top capitalization deciles
  - Maximum Stock Weight: $< 10.0\%$
- **Local Portfolio Policy**:
  - Maximum Pearson correlation against active portfolio alphas: $\max \rho < 0.70$.
  - Accretive portfolio impact: $\Delta \text{Sharpe} > 0$ and $\Delta \text{Vol} \le 0$.

---

## 2. Dataset Family 1: Systematic Risk Metrics (`model51`)

### 2.1 Overview & Schema Specification
The `model51` dataset (`category: model-risk-models`) contains **16 MATRIX fields** measuring the empirical relationship between individual equity returns and the market benchmark (SPY ETF) across four rolling horizons: $T \in \{30, 60, 90, 360\}$ calendar days.

The fields map into four canonical statistical dimensions:
1. `beta_last_<T>_days_spy`: OLS slope coefficient $\beta_i$ relative to SPY.
2. `correlation_last_<T>_days_spy`: Pearson correlation coefficient $\rho_{i, m}$ relative to SPY.
3. `systematic_risk_last_<T>_days`: Proportion of return variance explained by SPY ($R^2$).
4. Unsystematic / Idiosyncratic Risk fields: Proportion of return variance independent of SPY ($1 - R^2$).

### 2.2 Mathematical Formulations & Fundamental Identities

#### 2.2.1 Linear Return Decomposition
For each equity $i$ and benchmark index $m$ (SPY) over rolling estimation window $\tau \in [t - T, t]$:
$$R_{i, \tau} = \alpha_{i, T} + \beta_{i, T} R_{m, \tau} + \epsilon_{i, \tau}$$
where $\mathbb{E}[\epsilon_{i, \tau}] = 0$ and $\text{Cov}(\epsilon_{i, \tau}, R_{m, \tau}) = 0$.

The rolling sample OLS beta is defined as:
$$\beta_{i, T} = \frac{\text{Cov}_T(R_i, R_m)}{\text{Var}_T(R_m)} = \rho_{i, m, T} \cdot \frac{\sigma_{i, T}}{\sigma_{m, T}}$$

The rolling Pearson correlation coefficient is:
$$\rho_{i, m, T} = \frac{\sum_{\tau} (R_{i, \tau} - \bar{R}_i)(R_{m, \tau} - \bar{R}_m)}{\sqrt{\sum_\tau (R_{i, \tau} - \bar{R}_i)^2 \sum_\tau (R_{m, \tau} - \bar{R}_m)^2}}$$

#### 2.2.2 Total Variance Decomposition & Invariance Theorems
Total asset return variance $\sigma_{i, T}^2$ decomposes orthogonally into systematic variance and idiosyncratic (unsystematic) variance:
$$\sigma_{i, T}^2 = \beta_{i, T}^2 \sigma_{m, T}^2 + \sigma_{\epsilon, i, T}^2$$

Dividing by total variance $\sigma_{i, T}^2$:
$$1 = \frac{\beta_{i, T}^2 \sigma_{m, T}^2}{\sigma_{i, T}^2} + \frac{\sigma_{\epsilon, i, T}^2}{\sigma_{i, T}^2} = R_{i, T}^2 + (1 - R_{i, T}^2)$$

1. **Systematic Risk Ratio ($R^2$)**:
   $$\text{SystematicRisk}_T = R_{i, T}^2 = \frac{\beta_{i, T}^2 \sigma_{m, T}^2}{\sigma_{i, T}^2} \equiv (\rho_{i, m, T})^2$$

2. **Idiosyncratic (Unsystematic) Risk Ratio ($1 - R^2$)**:
   $$\text{IdiosyncraticRisk}_T = 1 - R_{i, T}^2 = \frac{\sigma_{\epsilon, i, T}^2}{\sigma_{i, T}^2}$$

3. **Fundamental Conservation Identity**:
   $$\forall i, \forall t, \forall T: \quad \text{SystematicRisk}_T + \text{IdiosyncraticRisk}_T \equiv 1.0$$

4. **Implied Relative Volatility**:
   Because $\beta_{i, T} = \rho_{i, m, T} \frac{\sigma_{i, T}}{\sigma_{m, T}}$, the ratio of asset volatility to market volatility can be extracted without accessing raw daily returns:
   $$\frac{\sigma_{i, T}}{\sigma_{m, T}} = \frac{\beta_{i, T}}{\rho_{i, m, T}} = \frac{\beta_{i, T}}{\text{sign}(\beta_{i, T}) \sqrt{\text{SystematicRisk}_T}}$$

### 2.3 Theoretical Foundations & Economic Rationales

#### 2.3.1 The Low-Beta / Low-Volatility Anomaly
- **Academic Foundations**: Black (1972), Baker, Bradley & Wurgler (2011), Frazzini & Pedersen (2014, *Betting Against Beta*).
- **Economic Mechanism**: Institutional asset managers are benchmarked to equity indices and face severe leverage constraints or regulatory capital restrictions. To beat market returns, constrained managers overweight high-beta securities. Simultaneously, retail investors exhibit lottery-ticket preferences and bid up speculative high-beta stocks.
- **Market Consequence**: High-beta assets become persistently overpriced, delivering inferior risk-adjusted returns (a flattened Security Market Line). Low-beta assets become neglected and underpriced, delivering significant positive alpha and superior Sharpe ratios when hedged or neutralized.

#### 2.3.2 The Idiosyncratic Volatility Puzzle
- **Academic Foundations**: Ang, Hodrick, Xing & Zhang (2006, 2009), Stambaugh, Yu & Yuan (2015).
- **Economic Mechanism**: In frictionless classical CAPM, idiosyncratic volatility is unpriced because it can be eliminated through costless diversification. In real equity markets, limits to arbitrage (borrow costs, short-sale recall risk, margin capital volatility) prevent rational arbitrageurs from shorting overpriced high-idiosyncratic-risk stocks. Retail sentiment rushes inflate these names, leading to subsequent negative performance drift.
- **Factor Implication**: Equities ranking in the lowest quintile of idiosyncratic risk systematically outperform high-idiosyncratic-risk speculative names on a risk-adjusted basis.

#### 2.3.3 Multi-Horizon Beta Term Structure & Shock Fragility
Beta is dynamic and displays term structure properties:
$$\Delta \beta_i = \beta_{i, 30d} - \beta_{i, 360d}$$
$$\kappa_{\beta, i} = \beta_{i, 30d} - 2\beta_{i, 90d} + \beta_{i, 360d} \quad (\text{Beta Curvature})$$
A sharp expansion in short-term beta relative to long-term structural beta indicates sudden corporate distress, sector rebalancing flows, or institutional liquidation contagion.

### 2.4 Canonical Factor Architectures (Abstract FastExpr)

#### Architecture 2.4.1: Systematic-Idiosyncratic Term Spread Alpha
*Hypothesis*: Stocks experiencing short-term beta expansion paired with elevated idiosyncratic volatility represent distressed speculative positions. Long low-beta, low-idiosyncratic names with reversal protection.

```fastexpr
idio_spread = ts_backfill(<IDIO_RISK_SHORT>, 10) / (ts_backfill(<IDIO_RISK_LONG>, 15) + 0.001);
beta_spread = ts_backfill(<BETA_SHORT>, 10) - ts_backfill(<BETA_LONG>, 15);
raw_sig = 0.65 * signed_power(0.5 - rank(idio_spread), 1.4) - 0.35 * signed_power(rank(beta_spread) - 0.5, 1.2);
rev_damp = 0.08 * (rank(ts_delta(close, 3)) - 0.5);
clean_alpha = raw_sig - rev_damp;
group_neutralize(signed_power(clean_alpha, 3.5), densify(subindustry))
```

#### Architecture 2.4.2: Low-Beta Residual Momentum Alpha
*Hypothesis*: Low-beta equities with stable market correlation offer resilient compound returns when purged of high-frequency price mean reversion.

```fastexpr
low_beta = signed_power(0.5 - rank(ts_backfill(<BETA_FIELD>, 15)), 1.5);
corr_penalty = signed_power(rank(ts_backfill(<CORR_FIELD>, 15)) - 0.5, 1.2);
rev = 0.06 * signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5);
sig = 0.60 * low_beta - 0.35 * corr_penalty - rev;
group_neutralize(signed_power(sig, 4.0), densify(subindustry))
```

#### Architecture 2.4.3: Idiosyncratic Risk Curvature Alpha
*Hypothesis*: Non-linear second differences in idiosyncratic risk across horizons detect accelerating instability prior to equity drawdowns.

```fastexpr
curv = ts_backfill(<IDIO_RISK_30D>, 10) - 2.0 * ts_backfill(<IDIO_RISK_60D>, 10) + ts_backfill(<IDIO_RISK_90D>, 10);
curv_rank = rank(curv) - 0.5;
sig = -1.0 * signed_power(curv_rank, 2.5);
group_neutralize(signed_power(sig, 3.8), densify(subindustry))
```

### 2.5 Market Regime Failure Modes & Quantitative Mitigations
1. **Macro Correlation Spikes ($R^2 \to 1.0$)**:
   - *Failure*: During systemic liquidity shocks (e.g., March 2020), cross-sectional correlation spikes toward unity, collapsing the variance of idiosyncratic risk.
   - *Mitigation*: Neutralize within `densify(subindustry)` to preserve intra-industry rank dispersion, and apply non-linear power transforms `signed_power(..., 3.5)` to separate signal from noise.
2. **Low-Beta Factor Crowding & Rotation Drawdowns**:
   - *Failure*: Low-volatility crowding inflates valuations. When macro rates surge or sentiment pivots to risk-on, low-beta defensives suffer sharp drawdowns.
   - *Mitigation*: Interlock beta signals with fundamental acceleration (`model16`) and penalize sudden short-term beta expansion ($\beta_{30d} - \beta_{360d}$).
3. **Coverage Disparities in Broad Universes**:
   - *Failure*: In `TOP3000`, newly listed stocks lack 360-day history, generating NaNs.
   - *Mitigation*: Restrict primary universes to `TOP1000` / `TOP500`, apply `ts_backfill(..., 15)`, and impute remaining NaNs with neutral zero weight: `if_else(is_nan(sig), 0, sig)`.

---

## 3. Dataset Family 2: Fundamental Scores & Derivative Acceleration (`model16`)

### 3.1 Overview & Dual-Tiered Schema Specification
The `model16` dataset (`category: model-valuation`) contains **24 MATRIX fields** bifurcated into two distinct operational tiers:
1. **Tier 1: Standardized Rank Derivatives (8 fields)**: Cross-sectional percentile ranks ($[0.0, 1.0]$) with **100.0% coverage** across liquid universes.
2. **Tier 2/3: Pentagon Style Scores (16 fields)**: Multi-factor composite scores (`fscore_*`, `fscore_bfl_*`) with lower coverage (~30% to 38%).

#### Core Tier 1 Rank Derivative Fields (100% Cross-Sectional Coverage):
- `multi_factor_acceleration_score_derivative`: Second time derivative of the multi-factor corporate score (convexity).
- `multi_factor_static_score_derivative`: First time derivative of the multi-factor corporate score (velocity).
- `analyst_revision_rank_derivative`: High-frequency indicator capturing consensus analyst revisions and estimate momentum.
- `cashflow_efficiency_rank_derivative`: Operating cash flow generation normalized by capital base and operating assets.
- `earnings_certainty_rank_derivative`: Variance and predictability of corporate earnings reports.
- `relative_valuation_rank_derivative`: Composite percentile rank of valuation multiples (P/E, EV/EBITDA, P/B).
- `growth_potential_rank_derivative`: Composite expected medium-term sales and earnings growth.
- `composite_factor_score_derivative`: Blended multi-factor revision velocity.

### 3.2 Mathematical Formulations & Fundamental Derivative Dynamics

#### 3.2.1 Multi-Factor State Space and Taylor Expansion
Let $S_i(t) \in [0, 1]$ represent the aggregate fundamental health score of firm $i$ at time $t$, derived from underlying fundamental accounting vectors:
$$S_i(t) = \mathbf{w}^T \mathbf{F}_i(t)$$
where $\mathbf{F}_i(t) = [V_i(t), G_i(t), P_i(t), M_i(t), Q_i(t)]^T$ represents normalized valuation, growth, profitability, momentum, and quality vectors.

Expanding the fundamental state trajectory via second-order Taylor expansion over horizon $\Delta t$:
$$S_i(t + \Delta t) \approx S_i(t) + \frac{\partial S_i(t)}{\partial t} \Delta t + \frac{1}{2} \frac{\partial^2 S_i(t)}{\partial t^2} (\Delta t)^2 + \mathcal{O}((\Delta t)^3)$$

1. **Fundamental Velocity (First Derivative)**:
   $$v_i(t) \equiv \frac{\partial S_i(t)}{\partial t} \approx \frac{S_i(t) - S_i(t - \Delta t)}{\Delta t}$$
   Captured directly by `multi_factor_static_score_derivative`.
2. **Fundamental Acceleration / Convexity (Second Derivative)**:
   $$a_i(t) \equiv \frac{\partial^2 S_i(t)}{\partial t^2} \approx \frac{v_i(t) - v_i(t - \Delta t)}{\Delta t} = \frac{S_i(t) - 2 S_i(t - \Delta t) + S_i(t - 2\Delta t)}{(\Delta t)^2}$$
   Captured directly by `multi_factor_acceleration_score_derivative`.
   - $a_i(t) > 0, v_i(t) > 0$: Accelerating expansion (maximum momentum compounding).
   - $a_i(t) > 0, v_i(t) \le 0$: Trough inflection (operational turnaround; high upside convexity).
   - $a_i(t) < 0, v_i(t) > 0$: Peak deceleration (precursor to earnings disappointment).

#### 3.2.2 Style Pentagon Geometric Area
The style pentagon models firms across five normalized axes spaced at equal angular intervals ($\theta = \frac{2\pi}{5} = 72^\circ$):
$$\mathbf{r}_i = [r_{V}, r_{G}, r_{P}, r_{M}, r_{Q}], \quad r_k \in [0, 1]$$
The total enclosed polygonal surface area $\mathcal{A}_i(t)$ represents the static style surface (`fscore_bfl_surface`):
$$\mathcal{A}_i(t) = \frac{1}{2} \sin(72^\circ) \left( r_V r_G + r_G r_P + r_P r_M + r_M r_Q + r_Q r_V \right)$$
where $\sin(72^\circ) \approx 0.95106$.

### 3.3 Theoretical Foundations & Economic Rationales

#### 3.3.1 Post-Earnings Announcement Drift (PEAD) & Analyst Underreaction
- **Academic Foundations**: Ball & Brown (1968), Bernard & Thomas (1989), Chan, Jegadeesh & Lakonishok (1996).
- **Economic Mechanism**: Sell-side equity analysts suffer from cognitive conservatism, career risk aversion, and institutional incentives. When corporate earnings or forward guidance beat expectations, analysts adjust forecast models incrementally over weeks rather than in a single step.
- **Factor Implication**: `analyst_revision_rank_derivative` captures this revision velocity, providing a steady, persistent drift signal that bypasses transient price momentum reversals.

#### 3.3.2 Acceleration vs Static Value Traps
- **Academic Foundations**: Lakonishok, Shleifer & Vishny (1994), Piotroski (2000), Asness, Frazzini & Pedersen (2019, *Quality Minus Junk*).
- **Economic Mechanism**: Static valuation factors (low P/E or high B/M) suffer from "value traps": distressed firms that appear statistically cheap but face deteriorating balance sheets, collapsing margins, and technological obsolescence.
- **Factor Implication**: Intersecting `relative_valuation_rank_derivative` with `multi_factor_acceleration_score_derivative` isolates cheap firms experiencing positive fundamental inflections, eliminating declining value traps.

#### 3.3.3 Cash Flow Persistence vs Accrual Quality
- **Academic Foundations**: Sloan (1996), Richardson et al. (2005).
- **Economic Mechanism**: Accounting earnings decompose into operating cash flows and accruals:
  $$\text{NetIncome} = \text{OperatingCashFlow} + \text{Accruals}$$
  Earnings generated via aggressive accounting accruals (unbilled receivables, deferred expenses) have low persistence and mean-revert downward. Cash flows (`cashflow_efficiency_rank_derivative`) represent realized liquidity with high persistence and dividend safety.

### 3.4 Canonical Factor Architectures (Abstract FastExpr)

#### Architecture 3.4.1: Convex Fundamental Acceleration Alpha
*Hypothesis*: Firms in the top quintile of fundamental acceleration with high cashflow efficiency outperform, provided high-frequency price reversals are dampened.

```fastexpr
accel = rank(<FUND_ACCEL_FIELD>) - 0.5;
cashflow = rank(<CASHFLOW_EFF_FIELD>) - 0.5;
rev = rank(<ANALYST_REV_FIELD>) - 0.5;
core_fund = 0.50 * accel + 0.30 * cashflow + 0.20 * rev;
price_mom = 0.10 * (rank(ts_delta(close, 5)) - 0.5);
sig = signed_power(core_fund, 1.5) - price_mom;
group_neutralize(signed_power(sig, 3.6), densify(subindustry))
```

#### Architecture 3.4.2: Multi-Factor Quality-Value Inflection Alpha
*Hypothesis*: Cheap firms displaying accelerating multi-factor consensus revisions avoid value traps and experience multiple expansion.

```fastexpr
composite = rank(<COMPOSITE_SCORE_DERIVATIVE>) - 0.5;
val = rank(<RELATIVE_VAL_FIELD>) - 0.5;
blended = 0.65 * composite + 0.35 * val;
rev_damp = 0.05 * (rank(ts_delta(close, 3)) - 0.5);
sig = blended - rev_damp;
group_neutralize(signed_power(sig, 4.0), densify(subindustry))
```

#### Architecture 3.4.3: Cashflow Quality & Earnings Certainty Drift
*Hypothesis*: High cashflow generation combined with predictable earnings minimizes credit distress and produces persistent positive drift.

```fastexpr
cf = rank(<CASHFLOW_EFF_FIELD>) - 0.5;
cert = rank(<EARNINGS_CERTAINTY_FIELD>) - 0.5;
sig = 0.55 * cf + 0.45 * cert;
group_neutralize(signed_power(sig, 3.8), densify(subindustry))
```

### 3.5 Reporting Calendar Failure Modes & Quantitative Mitigations
1. **Earnings Season Step Discontinuities**:
   - *Failure*: Fundamental scores update primarily around quarterly reporting dates (Jan/Apr/Jul/Oct). Un-smoothed signals produce spikes in daily turnover ($> 70\%$).
   - *Mitigation*: Smooth derivative ranks with `ts_decay_linear(..., 15)` and configure simulation execution decay to `decay: 18` or `decay: 22`.
2. **Sub-Universe Test Failures from Incomplete Pentagon Coverage**:
   - *Failure*: Tier 2/3 style scores (`fscore_*`) only cover 30-38% of equities, generating widespread NaNs and failing BRAIN's sub-universe consistency check.
   - *Mitigation*: Construct production factors exclusively using the 8 **Tier 1 rank derivative fields** (`*_derivative`), ensuring 100.0% cross-sectional coverage across all market cap tiers.
3. **Point-in-Time Dissemination Compliance**:
   - *Failure*: Incorporating earnings data before official SEC filing timestamps creates fatal look-ahead bias.
   - *Mitigation*: Execute strictly at `delay: 1`. Under Delay 1, BRAIN guarantees all inputs were publicly disseminated prior to market open on execution day $t$.

---

## 4. Dataset Family 3: RavenPack News Sentiment (`news18`)

### 4.1 Overview & Dual-Tiered Tensor Architecture
The `news18` dataset (`category: news`) provides real-time NLP sentiment analysis across global news publications:
- **71 Daily MATRIX Fields**: Pre-aggregated 2D daily cross-sectional matrices $\mathbf{M} \in \mathbb{R}^{T \times N}$ containing daily sentiment averages, entity relevance, and novelty scores.
- **50 Intraday VECTOR Fields**: 3D event-level arrays $\mathbf{V}_{i, t} \in \mathbb{R}^{K_{i, t}}$ containing $K_{i, t}$ individual news stories published for equity $i$ on trading day $t$.

#### Core News Fields Overview:
- `mean_composite_sentiment_score` (MATRIX): Daily average composite sentiment across all publications ($[-1.0, +1.0]$).
- `mean_editorial_commentary_sentiment` (MATRIX): Sentiment isolated from opinion pieces and editorial commentary ($[-1.0, +1.0]$).
- `mean_earnings_evaluation_sentiment` (MATRIX): Sentiment derived strictly from quarterly earnings reports and guidance ($[-1.0, +1.0]$).
- `mean_corporate_action_sentiment` (MATRIX): Sentiment on dividends, repurchases, and restructurings ($[-1.0, +1.0]$).
- `mean_merger_acquisition_sentiment` (MATRIX): Sentiment tracking M&A bids, takeovers, and divestitures ($[-1.0, +1.0]$).
- `mean_event_novelty_score` (MATRIX): Measures news freshness vs syndicated reprints ($[0, 100]$).
- `mean_entity_relevance_score` (MATRIX): Measures subject centrality of the equity in the news story ($[0, 100]$).
- Intraday story sentiment vector streams (VECTOR): Require vector folding operators.

### 4.2 Intraday Vector Folding Mechanics & Syntax Rules
In FastExpr, vector fields represent variable-length lists of intraday records:
$$\mathbf{V}_{i, t} = \{ v_{i, t, 1}, v_{i, t, 2}, \dots, v_{i, t, K_{i, t}} \}$$

#### Rule 1: Vector Dimension Protection
Direct arithmetic between a `VECTOR` and a `MATRIX` or `SCALAR` causes a fatal syntax failure:
$$\mathbf{V}_{i, t} + 1.0 \implies \text{\textbf{HTTP 400 Bad Request (Type Mismatch)}}$$
All `VECTOR` fields must be folded into 2D matrices using `vec_avg(...)` or `vec_sum(...)` before any downstream operators.

#### Rule 2: Folding Operator Selection
- **Polarity & Quality Metrics**: Use `vec_avg(...)` for continuous sentiment, novelty, and relevance.
- **Event Intensity & Counts**: Use `vec_sum(...)` for discrete event counters and story volumes.

#### Rule 3: Event Sparsity & Mandatory Backfilling
On days with zero news stories for equity $i$, `vec_avg(\mathbf{V}_{i, t})` evaluates strictly to `NaN`. Because over 60% of equities have no news on a typical trading day, unhandled NaNs collapse portfolio positions.
The canonical WorldQuant BRAIN news idiom is:
```fastexpr
raw_signal = ts_backfill(vec_avg(<NEWS_SENTIMENT_VECTOR>), 15);
clean_signal = if_else(is_nan(raw_signal), 0, rank(raw_signal) - 0.5);
```

### 4.3 Theoretical Foundations & Economic Hypotheses

#### 4.3.1 Sentiment Divergence (Editorial Commentary vs Hard News)
- **Academic Foundations**: Tetlock (2007), Fang & Peress (2009).
- **Economic Mechanism**: Factual corporate announcements (contract wins, SEC filings, operational updates) convey fundamental intrinsic value shifts. In contrast, editorial commentary and opinion pieces reflect media framing, sensation seeking, and retail narrative biases.
- **Factor Implication**: When factual composite sentiment is strongly positive but editorial commentary is skeptical or negative, retail investors hesitate while informed institutional capital accumulates. Arbitraging this divergence captures high-information alpha.

#### 4.3.2 Corporate Action Sentiment & M&A Catalysts
- **Academic Foundations**: Myers & Majluf (1984), Mitchell & Pulvino (2001).
- **Economic Mechanism**: Corporate actions (share buybacks, dividend increases, debt restructuring) represent costly capital allocation signals from management who possess private information. Positive corporate actions signal balance-sheet health and undervaluation, initiating multi-week upward drift.

#### 4.3.3 Earnings Evaluation Shocks & Post-Earnings Announcement Drift
- **Academic Foundations**: Ball & Brown (1968), Bernard & Thomas (1989).
- **Economic Mechanism**: Quarterly earnings reports cause immediate repricing. Decomposing earnings releases with RavenPack NLP captures tone beyond headline EPS figures (e.g., forward guidance caveats, margin commentary). Positive earnings shocks relative to historical baselines generate persistent PEAD drift over 15 to 45 trading days.

### 4.4 Canonical Factor Architectures (Abstract FastExpr)

#### Architecture 4.4.1: Novelty-Weighted News Sentiment Drift Alpha
*Hypothesis*: Fresh news events with high entity relevance produce stronger forward momentum than syndicated repeats.

```fastexpr
raw_sent = ts_backfill(vec_avg(<NEWS_SENTIMENT_VECTOR>), 15);
novelty = ts_backfill(vec_avg(<NOVELTY_SCORE_VECTOR>), 15);
sent_clean = if_else(is_nan(raw_sent), 0, rank(raw_sent) - 0.5);
nov_wt = if_else(is_nan(novelty), 0.5, rank(novelty));
sig = signed_power(sent_clean, 1.4) * (0.5 + 0.5 * nov_wt) - 0.06 * (rank(ts_delta(close, 3)) - 0.5);
group_neutralize(signed_power(sig, 4.0), densify(subindustry))
```

#### Architecture 4.4.2: Earnings Evaluation Sentiment Shock Alpha
*Hypothesis*: Sentiment shocks in earnings releases predict multi-week post-earnings drift when decayed linearly.

```fastexpr
earnings_sent = ts_backfill(vec_avg(<EARNINGS_EVAL_VECTOR>), 20);
rel = ts_backfill(vec_avg(<RELEVANCE_SCORE_VECTOR>), 20);
f_sent = if_else(is_nan(earnings_sent), 0, rank(earnings_sent) - 0.5);
f_rel = if_else(is_nan(rel), 0.5, rank(rel));
sig = signed_power(f_sent, 1.5) * (0.4 + 0.6 * f_rel);
group_neutralize(signed_power(sig, 4.2), densify(subindustry))
```

#### Architecture 4.4.3: Corporate Action & Volume Surge Catalyst Alpha
*Hypothesis*: Positive corporate actions accompanied by volume expansion signal decisive institutional accumulation.

```fastexpr
corp = ts_backfill(vec_avg(<CORP_ACTION_VECTOR>), 25);
f_corp = if_else(is_nan(corp), 0, rank(corp) - 0.5);
vol_ratio = rank(volume / ts_mean(volume, 30)) - 0.5;
sig = 0.65 * signed_power(f_corp, 1.5) + 0.35 * signed_power(vol_ratio, 1.2);
group_neutralize(signed_power(sig, 3.8), densify(subindustry))
```

### 4.5 Market Microstructure Failure Modes & Quantitative Mitigations
1. **Media Sensationalism & Churn Turnover**:
   - *Failure*: Unfiltered news sentiment decays within 24-48 hours. Trading on raw sentiment produces 120-180% turnover, violating BRAIN's 70% turnover ceiling.
   - *Mitigation*: Apply `ts_decay_linear(..., 12)` within the formula and execute simulation runs with decay settings `decay: 18` to `decay: 24`.
2. **Event Sparsity & Weekend Timestamp Clumping**:
   - *Failure*: Over 60% of trading days feature no news for typical stocks. Weekend stories cluster on Monday open.
   - *Mitigation*: Wrap all news inputs in `ts_backfill(..., 15)` and apply explicit NaN guards `if_else(is_nan(x), 0, x)`.
3. **Reversal at Delay 1**:
   - *Failure*: News headline spikes frequently overreact intraday on day $0$ and mean-revert on day $1$.
   - *Mitigation*: Add an explicit short-term price reversal dampener: `- 0.06 * (rank(ts_delta(close, 3)) - 0.5)`.

---

## 5. Dataset Family 4: Options Analytics Surface (`option9`)

### 5.1 Overview & Multi-Horizon Surface Schema Specification
The `option9` dataset (`category: option`) provides **74 MATRIX fields** derived from listed US equity options chains, organized across **12 standardized constant expiration horizons**:
$$\mathcal{T} = \{10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080\} \text{ calendar days}$$

The dataset encompasses six primary factor families:
1. `pcr_vol_<T>`: Put-Call trading volume ratio across horizon $T$ days (tactical flow).
2. `pcr_oi_<T>`: Put-Call open interest ratio across horizon $T$ days (structural positioning).
3. `forward_price_<T>`: Synthetic forward equity price implied by ATM put-call parity.
4. `call_breakeven_<T>`: Open-interest-weighted upside breakeven price ($K + C$).
5. `put_breakeven_<T>`: Open-interest-weighted downside breakeven price ($K - P$).
6. `option_breakeven_<T>`: Combined straddle/strangle breakeven price move.

> **CRITICAL SYNTAX INVARIANT**: Field identifiers in `option9` use exact integer horizons without the 'd' character (e.g., `pcr_vol_20`, `pcr_oi_30`, `forward_price_60`, `call_breakeven_30`). Using `pcr_vol_20d` results in an unknown field error.

### 5.2 Mathematical Formulations & Volatility Surface Geometry

#### 5.2.1 Breakeven Directional Asymmetry
For an option contract with strike $K$, call premium $C$, and put premium $P$, the required percentage moves for option buyers to break even at horizon $T$ are:
- **Upside Call Hurdle**:
  $$R_{\text{call}, T} = \frac{\text{call\_breakeven\_}T}{\text{close}} - 1$$
- **Downside Put Cushion**:
  $$R_{\text{put}, T} = 1 - \frac{\text{put\_breakeven\_}T}{\text{close}}$$

The **Breakeven Directional Asymmetry** isolates net directional skew:
$$\Delta \text{BE}_T = R_{\text{put}, T} - R_{\text{call}, T} = 2 - \frac{\text{put\_breakeven\_}T + \text{call\_breakeven\_}T}{\text{close}}$$
When informed investors demand upside call exposure, call premiums expand relative to puts, depressing $\Delta \text{BE}_T$. Conversely, extreme positive $\Delta \text{BE}_T$ indicates elevated put crash premiums (bearish crowding or excessive hedging), which historically precedes bullish equity mean-reversion.

#### 5.2.2 Synthetic Forward Price Parity Dislocation & Short-Sale Squeezes
Under European Put-Call Parity:
$$C_t - P_t = S_t - K e^{-r(T-t)} - \text{PV}(\text{Div})$$
The synthetic forward price is:
$$F_{t, T} = (C_t - P_t) e^{r(T-t)} + K$$
In frictionless markets, the forward premium reflects the risk-free rate $r$, borrowing fee $c$, and dividend yield $q$:
$$\frac{F_{t, T}}{S_t} - 1 \approx (r - q - c) \frac{T}{365}$$
When an equity faces extreme short-sale constraints ($c \gg 0$) or hard-to-borrow status, cash-and-carry arbitrage fails. Synthetic forward prices trade at deep discounts to spot equity prices ($F_{t, T} \ll S_t$). An extreme negative forward premium signals intense short-selling pressure and imminent short-squeeze risk.

#### 5.2.3 Term Skew & Slope Dynamics
The term structure of put-call ratios reflects informed order flow dynamics:
- **Volume PCR Slope**:
  $$\text{Slope}_{\text{vol}} = \text{pcr\_vol\_20} - \text{pcr\_vol\_90}$$
  Measures tactical retail panic (near-term 20d put volume) relative to institutional hedging (90d put volume).
- **Open Interest PCR Slope**:
  $$\text{Slope}_{\text{oi}} = \text{pcr\_oi\_30} - \text{pcr\_oi\_180}$$
  Measures structural institutional accumulation of downside crash protection.
- **Flow vs Positioning Divergence**:
  $$\text{Divergence} = \text{rank}(\text{pcr\_vol\_20}) - \text{rank}(\text{pcr\_oi\_30})$$

### 5.3 Theoretical Foundations & Economic Rationales
- **Informed Options Trading & Leverage Preference**: Black (1975), Easley, O'Hara & Srinivas (1998), Pan & Poteshman (2006). Informed traders with directional insights prefer options over equity shares due to embedded leverage and downside risk truncation. Order flow imbalances in options surfaces anticipate future cash equity returns.
- **Limits to Arbitrage & Short Squeezes**: Ofek, Richardson & Whitelaw (2004), Battalio & Schultz (2006). Violations of put-call parity persist when shorting the underlying stock is constrained by borrow fees or recall risk. Synthetic forward discounts quantify unexpressed short interest demand.

### 5.4 Canonical Factor Architectures (Abstract FastExpr)

#### Architecture 5.4.1: Breakeven Asymmetry Alpha
*Hypothesis*: Extreme downside put crash pricing indicates panic over-hedging that mean-reverts cross-sectionally.

```fastexpr
call_be = ts_backfill(<CALL_BREAKEVEN_FIELD> / close - 1, 10);
put_be = ts_backfill(1 - <PUT_BREAKEVEN_FIELD> / close, 10);
be_asym = rank(put_be - call_be) - 0.5;
rev = 0.06 * (rank(ts_delta(close, 3)) - 0.5);
sig = signed_power(be_asym, 1.4) - rev;
group_neutralize(signed_power(sig, 4.0), densify(subindustry))
```

#### Architecture 5.4.2: Put-Call Flow vs Positioning Dislocation Alpha
*Hypothesis*: Spikes in short-term put volume against low open interest indicate transitory speculative churning rather than structural balance-sheet deterioration.

```fastexpr
pcr_vol = rank(ts_decay_linear(ts_backfill(<PCR_VOL_FIELD>, 8), 5));
pcr_oi = rank(ts_decay_linear(ts_backfill(<PCR_OI_FIELD>, 12), 8));
flow_div = pcr_oi - pcr_vol;
group_neutralize(signed_power(flow_div, 3.6), densify(subindustry))
```

#### Architecture 5.4.3: Forward Price Parity & Term Skew Alpha
*Hypothesis*: Synthetic forward discounts combined with flattening term skew identify short-squeeze candidates.

```fastexpr
fwd_premium = ts_backfill(<FORWARD_PRICE_FIELD> / close - 1, 10);
term_skew = ts_backfill(<PCR_TERM_SHORT> - <PCR_TERM_LONG>, 10);
sig = 0.60 * signed_power(rank(fwd_premium) - 0.5, 1.4) - 0.40 * signed_power(rank(term_skew) - 0.5, 1.3);
group_neutralize(signed_power(sig, 4.2), densify(sector))
```

### 5.5 Expiration Failure Modes & Quantitative Mitigations
1. **Outer Horizon Illiquidity ($T \ge 360d$)**:
   - *Failure*: Long-dated LEAP options suffer from wide spreads and stale settlement pricing, causing jump discontinuities.
   - *Mitigation*: Confine core alpha generation to liquid intermediate horizons ($T \in \{20, 30, 60, 90\}$ days).
2. **Post-Earnings Implied Volatility Crush**:
   - *Failure*: Volatility collapses after quarterly earnings, causing simultaneous contractions in call and put breakevens.
   - *Mitigation*: Use differential breakeven spreads ($R_{\text{put}} - R_{\text{call}}$), where symmetric volatility crush cancels out mathematically.
3. **Monthly Expiration Gamma Pinning**:
   - *Failure*: Market maker hedging around monthly expiration (third Friday) distorts spot-option parity.
   - *Mitigation*: Blend adjacent horizons (e.g., $0.5 \cdot (\text{pcr\_vol\_20} + \text{pcr\_vol\_60})$) rather than relying on ultra-short tenors.
4. **Dividend Ex-Date Drop Distortions**:
   - *Failure*: Cash dividends reduce spot price on ex-dates, depressing synthetic forward prices for high-yield equities.
   - *Mitigation*: Neutralize across `densify(subindustry)` to remove sector-wide dividend yield biases.

---

## 6. Cross-Dataset Fusion & Sanctuary Hybrid Architectures

### 6.1 Multi-Layer Factor Assembly (Anchor + Catalyst + Friction Dampener)
Single-dataset alphas frequently fail under changing market regimes. AlphaFind implements a **three-tier factor assembly pipeline**:
1. **Economic Anchor (Structural Core)**: Low-beta stability (`model51`) or fundamental acceleration (`model16`) providing long-term carrying value.
2. **Risk Catalyst (Timing Trigger)**: RavenPack corporate action news (`news18`) or options flow asymmetry (`option9`) providing high-frequency trade timing.
3. **Friction Dampener (Turnover Stabilizer)**: Short-term price reversal penalty (`ts_delta(close, d)`) or liquidity dampener reducing high-frequency churn.

```
       +-------------------------------------------------------+
       |               ECONOMIC ANCHOR (model51 / model16)     |
       |               (Structural Low-Beta / Quality Accel)   |
       +---------------------------+---------------------------+
                                   │
                                   ▼
       +-------------------------------------------------------+
       |               RISK CATALYST (news18 / option9)        |
       |               (M&A Sentiment / Breakeven Skew)        |
       +---------------------------+---------------------------+
                                   │
                                   ▼
       +-------------------------------------------------------+
       |               FRICTION DAMPENER & SIZING              |
       |               (- 0.05 * rank(ts_delta(close, 3)))     |
       +---------------------------+---------------------------+
                                   │
                                   ▼
       +-------------------------------------------------------+
       |               GROUP NEUTRALIZATION & DENSIFICATION    |
       |               group_neutralize(..., densify(group))   |
       +-------------------------------------------------------+
```

### 6.2 Structural Orthogonality Matrix
Pairwise correlation across the four alternative datasets is structurally low:

| Dataset Pair | Typical $\rho$ | Economic Basis for Orthogonality |
|--------------|----------------|----------------------------------|
| `model51` vs `model16` | $0.05 - 0.18$ | Market covariance vs internal accounting efficiency |
| `model51` vs `news18` | $-0.02 - 0.08$ | Trailing statistical risk vs forward textual sentiment |
| `model16` vs `option9` | $0.08 - 0.22$ | Historical quarterly fundamentals vs forward market-implied volatility |
| `news18` vs `option9` | $0.15 - 0.28$ | Unstructured public news vs institutional derivatives positioning |

### 6.3 Canonical Cross-Sanctuary Hybrid Skeleton (Abstract FastExpr)

```fastexpr
anchor_risk = 0.5 - rank(ts_backfill(<BETA_FIELD>, 15));
anchor_fund = rank(<FUND_ACCEL_FIELD>) - 0.5;
catalyst_news = if_else(is_nan(ts_backfill(vec_avg(<NEWS_SENTIMENT_VECTOR>), 15)), 0, rank(ts_backfill(vec_avg(<NEWS_SENTIMENT_VECTOR>), 15)) - 0.5);
catalyst_opt = rank(ts_backfill(1 - <PUT_BREAKEVEN_FIELD> / close, 10) - ts_backfill(<CALL_BREAKEVEN_FIELD> / close - 1, 10)) - 0.5;
core_alpha = 0.40 * anchor_risk + 0.30 * anchor_fund + 0.15 * catalyst_news + 0.15 * catalyst_opt;
reversal_damp = 0.05 * (rank(ts_delta(close, 3)) - 0.5);
net_alpha = signed_power(core_alpha, 1.5) - reversal_damp;
group_neutralize(signed_power(net_alpha, 4.0), densify(subindustry))
```

---

## 7. FastExpr Implementation Guide & Syntax Rules

### 7.1 Multi-Statement Syntax and Scoping
- Semicolons `;` delineate sequential statements.
- The final expression without a trailing semicolon defines the alpha return tensor.
- Variable names must not collide with FastExpr built-in operator keywords.

### 7.2 Vector Folding Rules
- All `VECTOR` fields must be wrapped inside `vec_avg(...)` or `vec_sum(...)` before applying scalar arithmetic or time-series operators.

### 7.3 Group Densification Requirement
- BRAIN grouping variables (e.g., `subindustry`, `industry`, `sector`) contain sparse, non-contiguous integer IDs.
- Passing raw groups into `group_neutralize` or `group_rank` causes runtime memory exhaustion and 504 Gateway Timeouts.
- **Mandatory Rule**: Always wrap grouping variables: `densify(subindustry)`.

### 7.4 Non-Linear Shaping Pipeline
1. Rank centering: `rank(x) - 0.5` converts variables to zero-mean cross-sectional uniform distribution $[-0.5, +0.5]$.
2. Non-linear amplification: `signed_power(x, p)` with $p \in [1.2, 2.0]$ expands tails while preserving sign.
3. Sizing compression: `signed_power(alpha, 3.5 to 4.2)` sharpens high-conviction positions and suppresses noise before neutralization.

---

## 8. Pre-Simulation Verification & Quality Gate

Prior to dispatching simulation jobs to WorldQuant BRAIN, every candidate expression must pass the local **AlphaFind Pre-Simulation Gatekeeper**:
1. **FastExpr AST Syntax Check**: Balanced parentheses, operator argument validation, and no unresolved placeholders.
2. **Coverage Audit**: Any field with coverage $< 95\%$ in the target universe must include `ts_backfill` and explicit NaN fallback.
3. **Turnover Protection**: Expressions must incorporate decay smoothing (`ts_decay_linear`) or execution decay ($\text{decay} \ge 16$).
4. **Git Hygiene Audit**: Codebase must execute and pass `bash scripts/sanitize_check.sh` cleanly with zero leaks of confidential research material.
