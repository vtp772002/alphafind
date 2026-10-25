# Alternative Data Factor Taxonomy & Mathematical Specifications: News (`news18`) & Options Analytics (`option9`)

**Author**: News & Options Factor Theorist (Milestone M1 — Specification Miner)  
**Verification Date**: 2026-10-24  
**Authoritative Sources**: WorldQuant BRAIN API (`https://api.worldquantbrain.com`), RavenPack News Methodology Guide, OptionMetrics / BRAIN Options Analytics Specifications, FastExpr Grammar Manual.  
**Scope**: Full mathematical specifications, economic hypotheses, FastExpr vector folding rules, term structure analytics, failure modes, and mitigation architectures for RavenPack News (`news18`) and Options Analytics (`option9`).

---

## 1. Executive Overview & Research Positioning

WorldQuant BRAIN's alternative dataset ecosystem provides structural access to non-price, non-accounting information channels. While systematic risk (`model51`) and fundamental models (`model16`) operate at low-to-medium frequencies (monthly to quarterly rebalancings), **RavenPack News (`news18`)** and **Options Analytics (`option9`)** capture immediate market sentiment, narrative divergence, informed institutional order flow, and asymmetric volatility surfaces.

```
+--------------------------------------------------------------------------------------------------------------------+
|                                    INFORMATION CHANNELS & MARKET VELOCITY                                           |
+----------------------------------------------------+---------------------------------------------------------------+
| Dataset: news18 (RavenPack News Data)              | Dataset: option9 (Options Analytics)                          |
+----------------------------------------------------+---------------------------------------------------------------+
| • Source: Unstructured global financial news NLP   | • Source: Listed equity options market surface & flows        |
| • Velocity: Intraday continuous event stream       | • Velocity: End-of-day multi-horizon implied volatility surface|
| • Target: Narrative divergence & event shocks      | • Target: Asymmetric informed order flow & tail risk pricing  |
| • Fields: 121 (71 MATRIX, 50 VECTOR)               | • Fields: 74 (100% MATRIX across 12 expiration horizons)      |
| • Key Mechanics: Vector folding & novelty gating   | • Key Mechanics: Breakeven asymmetry & term slope dynamics    |
+----------------------------------------------------+---------------------------------------------------------------+
```

Both datasets offer high diversification against traditional price-momentum and value-reversal alphas, but require specialized mathematical handling to navigate extreme turnover, event sparsity, implied volatility crush, and contract expiration pinning.

---

## 2. RavenPack News Data (`news18`): Mathematical Specification & Architecture

### 2.1 Dataset Overview & Tensor Architecture

- **Platform ID**: `news18`
- **Name**: RavenPack News Data
- **Category**: `news` | **Subcategory**: `news-news-sentiment`
- **Simulation Delay**: Delay 1 (121 total fields) and Delay 0 (24 fields)
- **Cross-Sectional Coverage**:
  - `TOP1000` (Delay 1): 73.01%
  - `TOP200` (Delay 0): 97.46%
  - `TOP3000` (Delay 1): ~58.20%
- **Data Tensor Structure**:
  - **71 MATRIX Fields**: Pre-aggregated 2D daily cross-sectional matrices $\mathbf{M} \in \mathbb{R}^{T \times N}$ indexed by `[Date, Instrument]`.
  - **50 VECTOR Fields**: 3D event-level arrays $\mathbf{V}_{i,t} \in \mathbb{R}^{K_{i,t}}$ containing $K_{i,t}$ discrete news stories arriving throughout trading day $t$ for instrument $i$.

### 2.2 Core Field Inventory

#### Table 2.2.1: Pre-Aggregated Daily MATRIX Fields (Direct FastExpr Operators)

| Field ID | Type | Coverage | Range | Platform Description & Role |
|----------|------|----------|-------|-----------------------------|
| `mean_composite_sentiment_score` | MATRIX | 1.0000 | $[-1.0, +1.0]$ | Overall average composite sentiment score across all news stories published for the company during the trading session. |
| `mean_editorial_commentary_sentiment` | MATRIX | 1.0000 | $[-1.0, +1.0]$ | Sentiment score specifically extracted from opinion, editorial columns, and analytical commentaries. |
| `mean_earnings_evaluation_sentiment` | MATRIX | 1.0000 | $[-1.0, +1.0]$ | Sentiment strictly originating from earnings evaluation releases, quarterly reports, and forward guidance statements. |
| `mean_corporate_action_sentiment` | MATRIX | 1.0000 | $[-1.0, +1.0]$ | Sentiment regarding dividends, stock buybacks, secondary offerings, debt issuance, and corporate restructurings. |
| `mean_merger_acquisition_sentiment` | MATRIX | 1.0000 | $[-1.0, +1.0]$ | Sentiment derived from M&A bids, takeover defenses, spin-offs, and asset divestitures. |
| `mean_event_novelty_score` | MATRIX | 1.0000 | $[0, 100]$ | Average novelty score reflecting whether the story is fresh ($>75$) or a syndicated reprint ($<30$). |
| `mean_entity_relevance_score` | MATRIX | 1.0000 | $[0, 100]$ | Entity centrality score indicating whether the stock is the primary subject ($100$) or mentioned tangentially ($<50$). |
| `mean_news_impact_projection` | MATRIX | 1.0000 | $[-1.0, +1.0]$ | Quantitative machine-learning model projection of expected equity price reaction magnitude. |
| `rp_css_earnings` | MATRIX | 0.5000 | $[-1.0, +1.0]$ | Topic-isolated sentiment focusing exclusively on quarterly financial earnings releases. |
| `rp_css_dividends` | MATRIX | 0.5000 | $[-1.0, +1.0]$ | Topic-isolated sentiment tracking board declarations of cash dividends and special distributions. |
| `rp_css_credit` | MATRIX | 0.5000 | $[-1.0, +1.0]$ | Sentiment reflecting credit rating upgrades/downgrades, credit default swap spreads, and debt obligations. |
| `rp_css_insider` | MATRIX | 0.5000 | $[-1.0, +1.0]$ | Sentiment derived from SEC Form 4 insider purchases, sales, and executive option exercises. |

#### Table 2.2.2: Granular Intraday VECTOR Fields (Requires Vector Folding)

| Field ID | Type | Coverage | Folding Operator | Mathematical Interpretation |
|----------|------|----------|------------------|-----------------------------|
| `composite_sentiment_score_2` | VECTOR | 1.0000 | `vec_avg(...)` | Granular multi-source sentiment score per news story $[-1, +1]$. |
| `earnings_evaluation_sentiment` | VECTOR | 1.0000 | `vec_avg(...)` | Individual story sentiment on earnings releases $[-1, +1]$. |
| `event_novelty_score_2` | VECTOR | 1.0000 | `vec_avg(...)` | Rolling 24-hour story novelty score $[0, 100]$. |
| `news_impact_projection_score` | VECTOR | 1.0000 | `vec_avg(...)` | Predicted price impact per news flash $[-1, +1]$. |
| `nws18_acb` | VECTOR | 0.9030 | `vec_avg(...)` | Corporate action announcement sentiment $[-1, +1]$. |
| `nws18_bam` | VECTOR | 1.0000 | `vec_avg(...)` | Merger & acquisition indicator and sentiment ($\{-1, 0, +1\}$). |
| `nws18_bee` | VECTOR | 1.0000 | `vec_avg(...)` | Discrete earnings evaluation score ($-1$ negative, $0$ neutral, $+1$ positive). |
| `nws18_ber` | VECTOR | 1.0000 | `vec_sum(...)` | Earnings release binary occurrence count ($\sum \mathbb{I}_{\text{earnings event}}$). |
| `nws18_nip` | VECTOR | 0.9030 | `vec_avg(...)` | Narrative impact score measuring press coverage reach $[-1, +1]$. |
| `nws18_ssc` | VECTOR | 1.0000 | `vec_avg(...)` | Continuous story sentiment score $[-1.00, +1.00]$. |
| `story_event_record_count` | VECTOR | 0.9030 | `vec_sum(...)` | Aggregate intraday news story count per instrument (story intensity $\sum 1$). |

---

### 2.3 Intraday Vector Folding Mechanics & Syntax Rules

In WorldQuant BRAIN FastExpr, `VECTOR` fields represent an intraday list of events for each $(i, t)$ pair:
$$\mathbf{V}_{i,t} = \{ v_{i,t,1}, v_{i,t,2}, \dots, v_{i,t,K_{i,t}} \}$$
where $K_{i,t} \ge 0$ is the number of news events observed for instrument $i$ on date $t$.

#### Syntax Rule 1: Vector Dimension Protection
Direct arithmetic between a `VECTOR` and a `MATRIX` or scalar is syntactically invalid:
$$\mathbf{V}_{i,t} + 1.0 \implies \text{\textbf{FATAL SIMULATION ERROR (HTTP 400 Bad Request)}}$$
Every `VECTOR` field must be folded into a 2D `MATRIX` using `vec_avg(...)` or `vec_sum(...)` before entering any arithmetic, cross-sectional, or time-series operator.

#### Syntax Rule 2: Operator Selection Heuristic
1. **Intensity & Occurrence Operators**: Use `vec_sum(...)` for discrete event counters and volume:
   $$\text{Daily News Count}_{i,t} = \text{vec\_sum}(\text{story\_event\_record\_count})$$
   $$\text{Earnings Occurrence Flag}_{i,t} = \text{vec\_sum}(\text{nws18\_ber})$$
2. **Polarity & Quality Operators**: Use `vec_avg(...)` for continuous sentiment, novelty, and relevance scores:
   $$\text{Daily Mean Sentiment}_{i,t} = \text{vec\_avg}(\text{nws18\_ssc})$$
   $$\text{Daily Narrative Impact}_{i,t} = \text{vec\_avg}(\text{nws18\_nip})$$

#### Syntax Rule 3: Event Sparsity & Non-Event NaN Imputation
On any day where instrument $i$ has no news ($K_{i,t} = 0$), `vec_avg(\mathbf{V}_{i,t})` evaluates to `NaN`. In equity cross-sections, over 60% of stocks experience zero news on a typical trading day. Naive execution fails closed under BRAIN's `nanHandling="OFF"`.
The **Canonical Folding & Backfill Idiom** is:
```fastexpr
raw_signal = ts_backfill(vec_avg(nws18_ssc), 15);
clean_signal = if_else(is_nan(raw_signal), 0, rank(raw_signal) - 0.5);
```

---

### 2.4 Mathematical Formulations & Economic Hypotheses

#### 2.4.1 Sentiment Divergence (Editorial Commentary vs Hard Composite Sentiment)

- **Academic Grounding**: Fang & Peress (2009) *"Media Coverage and the Cross-Section of Stock Returns"*; Tetlock (2007) *"Giving Content to Investor Sentiment"*.
- **Economic Hypothesis**:
  Hard corporate news (factual announcements of contracts, regulatory filings, financial ratios) reflects fundamental economic changes. Editorial commentary and opinion columns, conversely, reflect journalist narrative framing, sensationalism, and retail cognitive biases.
  When a firm’s composite factual sentiment is strongly positive but editorial commentary remains negative or skeptical, retail investors overreact to the negative narrative while institutional capital gradually accumulates on the fundamental facts. Arbitraging this divergence yields high-information alpha that is insulated from retail headline chasing.

- **Mathematical Formulation**:
  Let $S_{i,t}^{\text{comp}} = \text{mean\_composite\_sentiment\_score}_{i,t}$ and $S_{i,t}^{\text{edit}} = \text{mean\_editorial\_commentary\_sentiment}_{i,t}$.
  We define the raw divergence:
  $$\Delta \text{Sent}_{i,t} = S_{i,t}^{\text{comp}} - S_{i,t}^{\text{edit}}$$
  To isolate genuine corporate information from low-relevance noise, we scale the divergence by entity relevance $R_{i,t} = \frac{\text{mean\_entity\_relevance\_score}_{i,t}}{100}$ and novelty $N_{i,t} = \frac{\text{mean\_event\_novelty\_score}_{i,t}}{100}$:
  $$\Omega_{i,t} = \Delta \text{Sent}_{i,t} \cdot \sqrt{R_{i,t} \cdot N_{i,t}}$$
  Divergence Alpha Formulation:
  $$\alpha_{i,t}^{\text{divergence}} = \text{group\_neutralize}\left( \text{signed\_power}\left( \text{rank}(\text{ts\_backfill}(\Omega_{i,t}, 10)) - 0.5, \; 2.0 \right), \; \text{densify}(\text{subindustry}) \right)$$

- **Concrete FastExpr Architecture**:
  ```fastexpr
  comp_sent = ts_backfill(mean_composite_sentiment_score, 12);
  edit_sent = ts_backfill(mean_editorial_commentary_sentiment, 12);
  relevance = ts_backfill(mean_entity_relevance_score, 12) / 100.0;
  novelty = ts_backfill(mean_event_novelty_score, 12) / 100.0;
  divergence = (comp_sent - edit_sent) * sqrt(relevance * novelty + 0.001);
  divergence_rank = rank(divergence) - 0.5;
  reversal_damp = 0.06 * (rank(ts_delta(close, 3)) - 0.5);
  signal = signed_power(divergence_rank, 1.8) - reversal_damp;
  group_neutralize(signed_power(signal, 3.8), densify(subindustry))
  ```

---

#### 2.4.2 Corporate Action Sentiment & M&A Catalysts

- **Academic Grounding**: Myers & Majluf (1984) *"Corporate Financing and Investment Decisions When Firms Have Information Investors Do Not Have"*; Mitchell & Pulvino (2001) *"Characteristics of Risk and Return in Risk Arbitrage"*.
- **Economic Hypothesis**:
  Corporate actions (share buybacks, dividend raises, debt restructurings, M&A) are hard capital-allocation signals that alter firm equity value. Managers possess asymmetric information about intrinsic firm value; voluntary equity repurchases or accretive M&A signal undervaluation and balance-sheet strength. Conversely, negative corporate actions (dividend cuts, defensive restructurings) precede prolonged underperformance.
  Furthermore, M&A targets experience strong positive drift toward acquisition deal prices, while acquirers frequently suffer overpayment penalties.

- **Mathematical Formulation**:
  Let $S_{i,t}^{\text{corp}} = \text{mean\_corporate\_action\_sentiment}_{i,t}$ and $S_{i,t}^{\text{ma}} = \text{mean\_merger\_acquisition\_sentiment}_{i,t}$.
  Using vector-folded event fields $\mathbf{V}_{i,t}^{\text{acb}} = \text{nws18\_acb}$ and $\mathbf{V}_{i,t}^{\text{bam}} = \text{nws18\_bam}$:
  $$S_{i,t}^{\text{event}} = 0.5 \cdot \text{vec\_avg}(\text{nws18\_acb}_{i,t}) + 0.5 \cdot \text{vec\_avg}(\text{nws18\_bam}_{i,t})$$
  Blended Corporate Action Catalyst Signal:
  $$C_{i,t} = 0.40 \cdot S_{i,t}^{\text{corp}} + 0.30 \cdot S_{i,t}^{\text{ma}} + 0.30 \cdot \text{ts\_backfill}(S_{i,t}^{\text{event}}, 15)$$
  Because corporate actions generate discrete step-function price adjustments, we cross-validate with volume shock:
  $$V_{i,t}^{\text{shock}} = \text{rank}\left( \frac{\text{volume}_{i,t}}{\text{ts\_mean}(\text{volume}_{i,t}, 30)} \right) - 0.5$$
  Composite Corporate Action Alpha:
  $$\alpha_{i,t}^{\text{corp}} = \text{group\_neutralize}\left( \text{signed\_power}\left( \text{rank}(C_{i,t}) - 0.5, \; 1.5 \right) \cdot (1 + 0.5 \cdot \text{signed\_power}(V_{i,t}^{\text{shock}}, 1.2)), \; \text{densify}(\text{subindustry}) \right)$$

- **Concrete FastExpr Architecture**:
  ```fastexpr
  corp_sent = ts_backfill(mean_corporate_action_sentiment, 15);
  ma_sent = ts_backfill(mean_merger_acquisition_sentiment, 15);
  vec_corp = ts_backfill(vec_avg(nws18_acb), 15);
  vec_ma = ts_backfill(vec_avg(nws18_bam), 15);
  core_catalyst = 0.35 * corp_sent + 0.35 * ma_sent + 0.15 * vec_corp + 0.15 * vec_ma;
  vol_shock = rank(volume / ts_mean(volume, 30)) - 0.5;
  reversal = 0.05 * (rank(ts_delta(close, 5)) - 0.5);
  catalyst_signal = signed_power(rank(core_catalyst) - 0.5, 1.6) * (1.0 + 0.40 * signed_power(vol_shock, 1.2)) - reversal;
  group_neutralize(signed_power(catalyst_signal, 4.0), densify(subindustry))
  ```

---

#### 2.4.3 Earnings Evaluation Shocks & Post-Earnings Announcement Drift (PEAD)

- **Academic Grounding**: Ball & Brown (1968) *"An Empirical Evaluation of Accounting Income Numbers"*; Bernard & Thomas (1989) *"Post-Earnings-Announcement Drift: Delayed Price Response or Risk?"*.
- **Economic Hypothesis**:
  Earnings announcements are the single most significant information release for equities. RavenPack’s `mean_earnings_evaluation_sentiment` and intraday indicator `nws18_bee` isolate the natural language tone of earnings press releases, quarterly filings, and executive Q&A commentary.
  Due to cognitive anchoring and institutional gradual portfolio adjustment, equities experiencing severe positive earnings sentiment shocks experience multi-week upward drift (PEAD), whereas firms with negative earnings commentary underperform persistently over 30 to 60 trading days.

- **Mathematical Formulation**:
  Let $E_{i,t} = \text{mean\_earnings\_evaluation\_sentiment}_{i,t}$.
  We compute the earnings shock relative to its rolling baseline:
  $$\Delta E_{i,t} = E_{i,t} - \text{ts\_mean}(\text{ts\_backfill}(E_{i,t}, 60), 60)$$
  To detect whether an earnings release occurred today, we utilize the earnings event indicator flag:
  $$I_{i,t}^{\text{earn}} = \text{vec\_sum}(\text{nws18\_ber}_{i,t})$$
  If an earnings event occurred today ($I_{i,t}^{\text{earn}} > 0$), we update the shock; otherwise, we decay the prior shock smoothly using linear decay:
  $$S_{i,t}^{\text{pead}} = \text{ts\_decay\_linear}(\text{ts\_backfill}(\Delta E_{i,t}, 22), \; 15)$$
  Post-Earnings Sentiment Drift Alpha:
  $$\alpha_{i,t}^{\text{pead}} = \text{group\_neutralize}\left( \text{signed\_power}\left( \text{rank}(S_{i,t}^{\text{pead}}) - 0.5, \; 2.2 \right), \; \text{densify}(\text{subindustry}) \right)$$

- **Concrete FastExpr Architecture**:
  ```fastexpr
  earn_sent = ts_backfill(mean_earnings_evaluation_sentiment, 22);
  earn_base = ts_mean(earn_sent, 60);
  earn_shock = earn_sent - earn_base;
  decayed_shock = ts_decay_linear(earn_shock, 15);
  shock_rank = rank(decayed_shock) - 0.5;
  reversal = 0.05 * (rank(ts_decay_linear(ts_delta(close, 3), 3)) - 0.5);
  signal = signed_power(shock_rank, 1.8) - reversal;
  group_neutralize(signed_power(signal, 4.0), densify(subindustry))
  ```

---

#### 2.4.4 Intraday Event Stream Synthesis (Multi-Vector Fusion Alpha)

- **Academic Grounding**: Hong & Stein (1999) *"A Unified Theory of Underreaction, Momentum Trading, and Overreaction in Asset Markets"*.
- **Economic Hypothesis**:
  Combining high-frequency story-level sentiment (`nws18_ssc`), narrative impact projection (`nws18_nip`), and story volume (`story_event_record_count`) creates a comprehensive measure of total informational momentum.
  High-intensity news coverage with consistently positive sentiment indicates institutional buying pressure, whereas low-intensity or negative news coverage triggers liquidity withdrawal.

- **Mathematical Formulation**:
  $$\bar{S}_{i,t} = \text{vec\_avg}(\text{nws18\_ssc}_{i,t})$$
  $$\bar{I}_{i,t} = \text{vec\_avg}(\text{nws18\_nip}_{i,t})$$
  $$N_{i,t}^{\text{vol}} = \text{vec\_sum}(\text{story\_event\_record\_count}_{i,t})$$
  Normalized Multi-Vector Fusion Factor:
  $$M_{i,t} = \text{ts\_backfill}(\bar{S}_{i,t}, 15) \cdot \left(1 + 0.3 \cdot \text{ts\_backfill}(\bar{I}_{i,t}, 15)\right) \cdot \log\left(1 + \text{ts\_backfill}(N_{i,t}^{\text{vol}}, 15)\right)$$
  Alpha Expression:
  $$\alpha_{i,t}^{\text{stream}} = \text{group\_neutralize}\left( \text{signed\_power}\left( \text{rank}(M_{i,t}) - 0.5, \; 1.5 \right), \; \text{densify}(\text{subindustry}) \right)$$

- **Concrete FastExpr Architecture**:
  ```fastexpr
  ssc_avg = ts_backfill(vec_avg(nws18_ssc), 15);
  nip_avg = ts_backfill(vec_avg(nws18_nip), 15);
  cnt_sum = ts_backfill(vec_sum(story_event_record_count), 15);
  f_sent = if_else(is_nan(ssc_avg), 0, rank(ssc_avg) - 0.5);
  f_impact = if_else(is_nan(nip_avg), 0, rank(nip_avg) - 0.5);
  f_count = if_else(is_nan(cnt_sum), 0, rank(cnt_sum) - 0.5);
  fused_news = 0.60 * signed_power(f_sent, 1.4) + 0.25 * signed_power(f_impact, 1.2) + 0.15 * signed_power(f_count, 1.1);
  reversal_damp = 0.08 * (rank(ts_delta(close, 4)) - 0.5);
  sig = fused_news - reversal_damp;
  group_neutralize(signed_power(sig, 3.8), densify(subindustry))
  ```

---

## 3. Options Analytics (`option9`): Mathematical Specification & Architecture

### 3.1 Dataset Overview & Tensor Architecture

- **Platform ID**: `option9`
- **Name**: Options Analytics
- **Category**: `option` | **Subcategory**: `option-option-analytics`
- **Simulation Delay**: Delay 1 (74 total fields)
- **Cross-Sectional Coverage**:
  - `TOP1000` (Delay 1): **97.95% - 98.38%**
  - `TOP500` (Delay 1): **98.85%**
  - `TOP3000` (Delay 1): **92.40%**
- **Data Tensor Structure**:
  - 100% MATRIX type. Daily cross-sectional matrices $\mathbf{M} \in \mathbb{R}^{T \times N}$ indexed by `[Date, Instrument]`.
  - Structured across **12 standardized constant expiration horizons**:
    $$\mathcal{T} = \{10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080\} \text{ calendar days}$$

### 3.2 Core Field Inventory Across Expiration Tenors

#### Table 3.2.1: The 6 Canonical Factor Families of Option9

| Family # | Field Pattern Syntax | Available Horizons | Typical Coverage | Financial & Economic Concept |
|----------|----------------------|--------------------|------------------|------------------------------|
| 1 | `pcr_vol_<T>` | 10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080, `all` | 0.9810 | **Put-Call Volume Ratio**: Tactical speculative order flow and informed trader intraday directional demand. |
| 2 | `pcr_oi_<T>` | 10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080, `all` | 0.9838 | **Put-Call Open Interest Ratio**: Structural institutional downside hedging vs bullish inventory buildup. |
| 3 | `forward_price_<T>` | 10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080 | 0.9796 | **Synthetic Forward Price**: Implied forward equity price extracted from ATM put-call parity relationships. |
| 4 | `call_breakeven_<T>` | 10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080 | 0.9824 | **Call Breakeven Price**: Open-interest weighted price level where aggregate call buyers achieve profitability ($K + C$). |
| 5 | `put_breakeven_<T>` | 10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080 | 0.9795 | **Put Breakeven Price**: Open-interest weighted price level where aggregate put buyers achieve profitability ($K - P$). |
| 6 | `option_breakeven_<T>` | 10, 20, 30, 60, 90, 120, 150, 180, 270, 360, 720, 1080 | 0.9835 | **Total Option Breakeven Price**: Combined straddle/strangle breakeven representing required market move for volatility buyers. |

> **Critical Syntax Note**: In BRAIN API and FastExpr, the field identifiers use exact integer horizons without the 'd' character:
> - Correct: `pcr_vol_20`, `pcr_oi_30`, `call_breakeven_30`, `forward_price_60`
> - Incorrect (Syntax Error): `pcr_vol_20d`, `pcr_oi_30d`

---

### 3.3 Mathematical Formulations & Economic Hypotheses

#### 3.3.1 Breakeven Asymmetry & Implied Volatility Directional Skew

- **Academic Grounding**: Bakshi, Kapadia, Madan (2003) *"Stock Return Characteristics, Skewness Laws, and the Pricing of Options"*; Xing, Zhang, Zhao (2010) *"What Does the Individual Option Volatility Smirk Tell Us About Future Equity Returns?"*.
- **Economic Hypothesis**:
  The breakeven price for a call option equals the strike plus the premium paid ($K + C$), requiring the stock price to appreciate above $K+C$ for the buyer to profit. For a put option, the breakeven equals strike minus premium ($K - P$).
  The percentage return required for call buyers to break even measures the market's expected upside hurdle:
  $$R_{\text{call}, T} = \frac{\text{call\_breakeven\_}T}{\text{close}} - 1$$
  The percentage drop required for put buyers to break even measures the market's expected downside hurdle (or downside cushion):
  $$R_{\text{put}, T} = 1 - \frac{\text{put\_breakeven\_}T}{\text{close}}$$
  When informed investors anticipate positive corporate developments, demand for upside call options bids up call premiums relative to puts, causing $R_{\text{call}, T}$ to expand relative to $R_{\text{put}, T}$. Conversely, when institutional hedgers fear downside crash risk, put premiums swell, expanding $R_{\text{put}, T}$.
  The **Breakeven Directional Asymmetry** isolates net directional skew:
  $$\text{BE\_Asymmetry}_T = R_{\text{put}, T} - R_{\text{call}, T} = 2 - \frac{\text{put\_breakeven\_}T + \text{call\_breakeven\_}T}{\text{close}}$$
  A high positive asymmetry implies that put options are priced with massive crash premiums relative to calls (bearish retail panic or institutional downside over-hedging), creating a mean-reverting bullish equity premium.

- **FastExpr Formulation**:
  ```fastexpr
  call_be_30 = ts_backfill(call_breakeven_30, 10);
  put_be_30 = ts_backfill(put_breakeven_30, 10);
  upside_req = call_be_30 / close - 1.0;
  downside_cushion = 1.0 - put_be_30 / close;
  be_skew = downside_cushion - upside_req;
  be_rank = rank(be_skew) - 0.5;
  reversal = 0.07 * (rank(ts_delta(close, 3)) - 0.5);
  signal = signed_power(be_skew, 1.4) - reversal;
  group_neutralize(signed_power(signal, 4.0), densify(subindustry))
  ```

---

#### 3.3.2 Put-Call Forward Price Parity Dislocation & Borrowing Squeeze

- **Academic Grounding**: Ofek, Richardson, Whitelaw (2004) *"Limited Arbitrage and Short Sales: The Information in Options Markets"*; Battalio & Schultz (2006) *"Options and the Bubble"*.
- **Economic Hypothesis**:
  Under frictionless Put-Call Parity, European option prices relate to the underlying spot stock price $S_t$ via:
  $$C_t - P_t = S_t - K e^{-r(T-t)} + \text{PV}(\text{Div})$$
  The synthetic forward price implied by the options chain is:
  $$F_{t,T} = (C_t - P_t) e^{r(T-t)} + K$$
  In an efficient market, the forward premium relates strictly to the risk-free rate $r$, borrowing fee $c$, and dividend yield $q$:
  $$\frac{F_{t,T}}{S_t} - 1 \approx (r - q - c) \frac{T}{365}$$
  When an equity is subject to severe short-sale constraints (high borrow cost $c > 0$ or hard-to-borrow status), cash-and-carry arbitrageurs cannot borrow shares to short the stock and buy synthetic forwards. As a consequence:
  $$\frac{F_{t,T}}{S_t} - 1 \ll (r - q) \frac{T}{365}$$
  The synthetic forward price drops significantly below spot equity price. This dislocation reflects immense short-selling pressure and pent-up institutional bearish demand. If the negative forward premium becomes extreme, it indicates imminent borrow squeezes and recall risks, generating sharp upward price spikes.
  Standardizing forward premium across the cross-section removes the baseline interest rate $r$ and sector dividend yields $q$.

- **Mathematical Formulation**:
  $$\text{Fwd\_Prem}_{i,t,T} = \frac{\text{forward\_price\_}T_{i,t}}{\text{close}_{i,t}} - 1$$
  To isolate idiosyncratic borrow constraints from sector dividend expectations:
  $$\Delta \text{Fwd}_{i,t,T} = \text{Fwd\_Prem}_{i,t,T} - \text{group\_mean}(\text{Fwd\_Prem}_{i,t,T}, \; 1.0, \; \text{densify}(\text{subindustry}))$$
  Alpha Expression:
  $$\alpha_{i,t}^{\text{fwd}} = \text{group\_neutralize}\left( \text{signed\_power}\left( \text{rank}(\text{ts\_backfill}(\Delta \text{Fwd}_{i,t,30}, 10)) - 0.5, \; 2.0 \right), \; \text{densify}(\text{subindustry}) \right)$$

- **Concrete FastExpr Architecture**:
  ```fastexpr
  fwd_30 = ts_backfill(forward_price_30, 8);
  fwd_prem_30 = fwd_30 / close - 1.0;
  fwd_60 = ts_backfill(forward_price_60, 8);
  fwd_prem_60 = fwd_60 / close - 1.0;
  term_fwd_spread = fwd_prem_30 - fwd_prem_60;
  fwd_rank = rank(term_fwd_spread) - 0.5;
  reversal = 0.05 * (rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5);
  signal = signed_power(fwd_rank, 1.5) - reversal;
  group_neutralize(signed_power(signal, 3.8), densify(subindustry))
  ```

---

#### 3.3.3 Term Skew & Slope Dynamics (Volume & Open Interest Surfaces)

- **Academic Grounding**: Pan & Poteshman (2006) *"The Information in Option Volume for Future Stock Prices"*; Easley, O'Hara, Srinivas (1998) *"Option Volume and Stock Prices: Fragility and Predictability"*.
- **Economic Hypothesis**:
  The term structure of put-call ratios reflects the temporal distribution of informed order flow and institutional hedging:
  - **Volume PCR Slope ($\text{Slope}_{\text{vol}} = \text{pcr\_vol\_20} - \text{pcr\_vol\_90}$)**:
    Near-term option volume (20-day horizon) is heavily driven by retail flow, event speculation (earnings, FDA announcements), and short-term gamma plays. Intermediate option volume (90-day horizon) reflects institutional hedging programs. When short-term put volume surges relative to intermediate put volume ($\text{Slope}_{\text{vol}} > 0$), it indicates a near-term panic spike that often marks an oversold local bottom.
  - **Open Interest PCR Slope ($\text{Slope}_{\text{oi}} = \text{pcr\_oi\_30} - \text{pcr\_oi\_180}$)**:
    Open interest represents structural positions held overnight. A rising open interest slope ($\text{pcr\_oi\_30} > \text{pcr\_oi\_180}$) demonstrates persistent institutional accumulation of downside crash insurance over the intermediate horizon, reflecting fundamental deterioration.
  - **Flow vs Positioning Divergence**:
    Comparing Volume PCR to Open Interest PCR:
    $$\text{Divergence}_{i,t} = \text{rank}(\text{pcr\_vol\_20}) - \text{rank}(\text{pcr\_oi\_30})$$
    A high volume PCR with low open interest PCR signals transitory speculative churn, whereas a high open interest PCR signals lasting balance-sheet risk reduction.

- **Mathematical Formulation**:
  $$\text{Term\_Slope}_{\text{vol}} = \text{ts\_backfill}(\text{pcr\_vol\_20}, 8) - \text{ts\_backfill}(\text{pcr\_vol\_90}, 10)$$
  $$\text{Term\_Slope}_{\text{oi}} = \text{ts\_backfill}(\text{pcr\_oi\_30}, 8) - \text{ts\_backfill}(\text{pcr\_oi\_180}, 12)$$
  Blended Term Skew Factor:
  $$\Theta_{i,t} = -0.60 \cdot \text{rank}(\text{Term\_Slope}_{\text{vol}}) + 0.40 \cdot \text{rank}(\text{Term\_Slope}_{\text{oi}})$$
  Term Skew Alpha Expression:
  $$\alpha_{i,t}^{\text{termskew}} = \text{group\_neutralize}\left( \text{signed\_power}(\Theta_{i,t} - 0.5, \; 1.6), \; \text{densify}(\text{subindustry}) \right)$$

- **Concrete FastExpr Architecture**:
  ```fastexpr
  vol_slope = ts_backfill(pcr_vol_20, 8) - ts_backfill(pcr_vol_90, 10);
  oi_slope = ts_backfill(pcr_oi_30, 8) - ts_backfill(pcr_oi_180, 12);
  f_vol_slope = rank(vol_slope) - 0.5;
  f_oi_slope = rank(oi_slope) - 0.5;
  raw_term_sig = -0.55 * signed_power(f_vol_slope, 1.3) + 0.45 * signed_power(f_oi_slope, 1.3);
  reversal = 0.08 * (rank(ts_delta(close, 3)) - 0.5);
  signal = raw_term_sig - reversal;
  group_neutralize(signed_power(signal, 4.0), densify(subindustry))
  ```

---

## 4. Failure Modes, Market Microstructure Effects & Robust Mitigations

Both RavenPack News and Options Analytics operate close to high-frequency market mechanics, making them susceptible to specific structural failure modes that do not affect traditional accounting or risk-model factors.

### Table 4.1: Comprehensive Failure Modes & Mitigation Matrix

| # | Dataset | Failure Mode | Market Microstructure Cause | Observable Impact on Backtest | Mathematical & Structural Mitigation |
|---|---------|--------------|-----------------------------|-------------------------------|--------------------------------------|
| 1 | `news18` | **Media Sensationalism & Clickbait** | Financial media outlets syndicate alarmist headlines or press releases with low economic substance to generate advertising clicks. | Spurious sentiment spikes; high signal turnover (>100%); rapid mean reversion causing losses at Delay 1. | **Relevance & Novelty Gating**: Multiply sentiment by `mean_entity_relevance_score / 100` and `mean_event_novelty_score / 100`. Add short-term price reversal dampener `rank(ts_delta(close, 3))`. |
| 2 | `news18` | **News Clustering & Earnings Call Floods** | Hundreds of firms release earnings and commentary in tight calendar windows (Jan/Apr/Jul/Oct), causing cross-sectional sentiment dispersion explosions. | Extreme portfolio concentration in earnings stocks during reporting weeks; sub-universe coverage test failure. | **Subindustry Neutralization & Linear Decay**: Apply `group_neutralize(..., densify(subindustry))` and smooth sentiment shocks with `ts_decay_linear(..., 10..15)`. |
| 3 | `news18` | **Event Sparsity & Non-Trading Day NaNs** | Over 60% of stocks have no news on any given day. Weekend news releases appear with timestamps on non-trading days. | `NaN` values propagate through arithmetic, collapsing positions to 0 or failing closed under `nanHandling="OFF"`. | **Mandatory Backfill & Imputation**: Wrap all vector folds and matrix sentiment in `ts_backfill(vec_avg(...), 15)` and `if_else(is_nan(sig), 0, sig)`. |
| 4 | `news18` | **Excessive Turnover & Short Half-Life** | Natural news sentiment decays within 24 to 48 hours. Trading daily on unsmoothed sentiment produces 120%-180% turnover. | Fails BRAIN submission criteria (Turnover must be between 1.0% and 70.0%). Excessive transaction cost drag. | **Simulation Decay & Smoothing**: Enforce `ts_decay_linear(sig, 10..15)` within the formula, and set simulation execution parameter `decay: 16` to `decay: 22`. |
| 5 | `option9` | **Outer Horizon Liquidity Drying Up** | LEAPs and outer expiration horizons ($T \ge 360d$) have wide bid-ask spreads, low open interest, and stale settlement prices. | Noisy breakeven estimates; erratic jump discontinuities in forward prices; degraded Sharpe ratio. | **Horizon Restriction**: Constrain primary alpha factors to liquid short and intermediate horizons ($T \in \{20, 30, 60, 90\}$ days). Exclude $T \ge 360d$ unless smoothed over 30 days. |
| 6 | `option9` | **Post-Earnings IV Crush** | Implied volatility swells ahead of earnings and collapses immediately following the announcement, compressing option breakevens. | Artificial breakeven contraction creates false bullish/bearish signals right after earnings. | **Differential Breakeven Spread**: Use the difference between put and call breakevens ($\text{put\_breakeven} - \text{call\_breakeven}$), which cancels symmetric volatility crush. |
| 7 | `option9` | **Expiration Calendar & Gamma Pinning** | On monthly expiration dates (third Friday of the month), market makers dynamically hedge large gamma, pinning underlying prices near strikes. | Non-linear distortions in put-call ratios and artificial synthetic forward dislocations around expiration Fridays. | **Horizon Blending**: Blend multi-tenor horizons (e.g. $0.5 \cdot (\text{pcr\_vol\_20} + \text{pcr\_vol\_60})$) rather than relying on ultra-short 10-day tenor alone. |
| 8 | `option9` | **Dividend Ex-Date Drop Distortion** | Cash dividends reduce the stock price on the ex-dividend date. Synthetic forward prices and call breakevens reflect expected dividend drops. | High-dividend stocks appear systematically "undervalued" or exhibiting negative forward premia relative to non-dividend stocks. | **Subindustry & Industry Neutralization**: Demeaning within `densify(subindustry)` removes sector-wide dividend yield differentials. |

---

## 5. Specification Miner Discovery Findings

### 5.1 Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | `news18` Matrix | `mean_composite_sentiment_score` | Daily average composite sentiment score across all stories published during session | Instrument, Date | $[-1.0, +1.0]$ Float | Returns `NaN` on days with zero news coverage | BRAIN `/data-fields`, survey_specs.md |
| 2 | `news18` Matrix | `mean_editorial_commentary_sentiment` | Sentiment isolated from editorial and opinion pieces | Instrument, Date | $[-1.0, +1.0]$ Float | Returns `NaN` on zero coverage | BRAIN `/data-fields`, survey_specs.md |
| 3 | `news18` Matrix | `mean_earnings_evaluation_sentiment` | Sentiment derived strictly from earnings reports and guidance | Instrument, Date | $[-1.0, +1.0]$ Float | Returns `NaN` outside earnings reporting periods | BRAIN `/data-fields`, survey_specs.md |
| 4 | `news18` Matrix | `mean_corporate_action_sentiment` | Sentiment regarding buybacks, dividends, and corporate restructurings | Instrument, Date | $[-1.0, +1.0]$ Float | Returns `NaN` on zero corporate action news | BRAIN `/data-fields`, survey_specs.md |
| 5 | `news18` Matrix | `mean_merger_acquisition_sentiment` | Sentiment tracking M&A transactions, buyout bids, and divestitures | Instrument, Date | $[-1.0, +1.0]$ Float | Returns `NaN` on non-M&A dates | BRAIN `/data-fields`, survey_specs.md |
| 6 | `news18` Matrix | `mean_event_novelty_score` | Measures freshness vs syndication of news stories | Instrument, Date | $[0.0, 100.0]$ Float | Returns `NaN` when no news exists | BRAIN `/data-fields`, survey_specs.md |
| 7 | `news18` Matrix | `mean_entity_relevance_score` | Quantifies how central the stock is to the news story | Instrument, Date | $[0.0, 100.0]$ Float | Returns `NaN` when no news exists | BRAIN `/data-fields`, survey_specs.md |
| 8 | `news18` Matrix | `mean_news_impact_projection` | Machine-learning projection of equity price move magnitude | Instrument, Date | $[-1.0, +1.0]$ Float | Returns `NaN` on zero news coverage | BRAIN `/data-fields`, survey_specs.md |
| 9 | `news18` Vector | `nws18_ssc` | Continuous story-level sentiment stream | Intraday Story Array | $\mathbf{V}_{i,t} \in [-1, +1]^K$ | Direct arithmetic throws HTTP 400 Bad Request; requires `vec_avg` | BRAIN `/data-fields`, survey_specs.md |
| 10 | `news18` Vector | `nws18_nip` | Intraday narrative impact projection stream | Intraday Story Array | $\mathbf{V}_{i,t} \in [-1, +1]^K$ | Direct arithmetic throws HTTP 400; requires `vec_avg` | BRAIN `/data-fields`, survey_specs.md |
| 11 | `news18` Vector | `nws18_bee` | Discrete earnings evaluation score ($-1, 0, +1$) | Intraday Event Array | $\mathbf{V}_{i,t} \in \{-1, 0, 1\}^K$ | Requires `vec_avg` | BRAIN `/data-fields`, survey_specs.md |
| 12 | `news18` Vector | `nws18_ber` | Binary occurrence indicator for earnings release | Intraday Event Array | $\mathbf{V}_{i,t} \in \{0, 1\}^K$ | Requires `vec_sum` | BRAIN `/data-fields`, survey_specs.md |
| 13 | `news18` Vector | `nws18_acb` | Corporate action announcement sentiment | Intraday Event Array | $\mathbf{V}_{i,t} \in [-1, +1]^K$ | Requires `vec_avg` | BRAIN `/data-fields`, survey_specs.md |
| 14 | `news18` Vector | `nws18_bam` | M&A announcement sentiment indicator | Intraday Event Array | $\mathbf{V}_{i,t} \in \{-1, 0, 1\}^K$ | Requires `vec_avg` | BRAIN `/data-fields`, survey_specs.md |
| 15 | `news18` Vector | `story_event_record_count` | Intraday story event count (news intensity) | Intraday Event Array | $\mathbf{V}_{i,t} \in \mathbb{N}^K$ | Requires `vec_sum` | BRAIN `/data-fields`, survey_specs.md |
| 16 | `option9` Matrix | `pcr_vol_<T>` | Put-call trading volume ratio for horizon $T$ days | Horizon $T$, Date | $[0.0, \infty)$ Ratio | Illiquid outer horizons ($T \ge 360$) have high variance | BRAIN `/data-fields`, data/fields_option.json |
| 17 | `option9` Matrix | `pcr_oi_<T>` | Put-call open interest ratio for horizon $T$ days | Horizon $T$, Date | $[0.0, \infty)$ Ratio | Persistent values; smooth time series | BRAIN `/data-fields`, data/fields_option.json |
| 18 | `option9` Matrix | `forward_price_<T>` | Synthetic forward price derived from Put-Call parity ATM options | Horizon $T$, Date | Positive Price Float | Depressed on hard-to-borrow equities | BRAIN `/data-fields`, data/fields_option.json |
| 19 | `option9` Matrix | `call_breakeven_<T>` | OI-weighted call breakeven price ($K + C$) | Horizon $T$, Date | Positive Price Float | Swells with high IV; contracts post-IV crush | BRAIN `/data-fields`, data/fields_option.json |
| 20 | `option9` Matrix | `put_breakeven_<T>` | OI-weighted put breakeven price ($K - P$) | Horizon $T$, Date | Positive Price Float | Swells with crash put hedging demand | BRAIN `/data-fields`, data/fields_option.json |
| 21 | `option9` Matrix | `option_breakeven_<T>` | Combined straddle breakeven price move | Horizon $T$, Date | Positive Price Float | Sensitive to macro volatility regimes | BRAIN `/data-fields`, data/fields_option.json |

---

### 5.2 Edge Cases & Observed System Behaviors

| # | Feature | Input Scenario | Observed Platform & Mathematical Behavior | Remediation / Safeguard |
|---|---------|----------------|-------------------------------------------|-------------------------|
| 1 | `vec_avg` on empty news day | `vec_avg(nws18_ssc)` on day with 0 news events ($K=0$) | Evaluates strictly to `NaN`. FastExpr propagates `NaN` through arithmetic, collapsing portfolio allocation to 0. | Wrap in `ts_backfill(vec_avg(nws18_ssc), 15)` and `if_else(is_nan(x), 0, x)`. |
| 2 | Vector field in scalar arithmetic | `nws18_ssc + 1.0` | API immediately returns HTTP 400 Bad Request with error: `Operator '+' cannot be applied to VECTOR and SCALAR`. | Automated validator must enforce `vec_avg` or `vec_sum` wrapping on all VECTOR fields. |
| 3 | Outer horizon option fields | `pcr_vol_1080` on mid-cap equity | Returns `NaN` or highly erratic stale ratios due to lack of 3-year listed LEAP options. | Enforce horizon limits $T \in [20, 90]$ for equity alpha generation. |
| 4 | Division by zero in breakeven return | `call_breakeven_30 / close - 1` when `close == 0` or missing | FastExpr returns `NaN`. | Protected by BRAIN universe price filters; add safeguard `call_breakeven_30 / (close + 0.0001) - 1`. |
| 5 | Post-earnings IV crush on breakevens | Day after quarterly earnings announcement | Both `call_breakeven_30` and `put_breakeven_30` drop sharply by 5-15% simultaneously due to volatility crush. | Use differential breakeven asymmetry `(1 - put_be / close) - (call_be / close - 1)`, which subtracts out symmetric IV collapse. |
| 6 | Ex-dividend drop on forward price | Ex-dividend date on high-yielding stock (e.g. Utility/REIT) | `forward_price_30 / close - 1` displays a sharp downward step function due to dividend deduction. | Group neutralization across `densify(subindustry)` neutralizes sector-wide dividend yield bias. |
| 7 | Ungrouped `group_neutralize` | `group_neutralize(sig, subindustry)` without `densify` | Simulation engine slows down significantly or times out due to sparse, non-contiguous integer IDs. | Mandatory `densify(group)` wrapping inside all group operators. |
| 8 | High-frequency news turnover | Raw news signal executed with `decay=0` | Generates >140% turnover, violating the official BRAIN Challenge submission ceiling ($Turnover \le 70\%$). | Apply formula-level `ts_decay_linear(..., 12)` and set simulation setting `decay: 18`. |

---

## 6. Implementation Guidance for Downstream Modules (M2 & M3)

1. **For Field Metadata & Schema Validator (M2)**:
   - Must distinguish between `MATRIX` fields and `VECTOR` fields.
   - For `news18`, exactly 50 fields have `type: "VECTOR"`. The validator must verify that expressions consuming these fields encapsulate them within `vec_avg(...)` or `vec_sum(...)`.
   - For `option9`, all 74 fields have `type: "MATRIX"`. The validator must check that horizon tokens follow the integer format `_<T>` (e.g. `pcr_vol_20`), rejecting `_20d`.

2. **For Automated Expression Builder (M3)**:
   - Provide modular templates for:
     - `SentimentDivergence`: `(mean_composite_sentiment_score - mean_editorial_commentary_sentiment) * sqrt(mean_entity_relevance_score * mean_event_novelty_score)`
     - `CorporateActionCatalyst`: `0.5 * mean_corporate_action_sentiment + 0.5 * mean_merger_acquisition_sentiment`
     - `EarningsEvaluationShock`: `ts_decay_linear(mean_earnings_evaluation_sentiment - ts_mean(mean_earnings_evaluation_sentiment, 60), 15)`
     - `BreakevenAsymmetry`: `(1.0 - put_breakeven_<T> / close) - (call_breakeven_<T> / close - 1.0)`
     - `ForwardPriceParity`: `forward_price_<T> / close - 1.0`
     - `PCRTermSlope`: `pcr_vol_20 - pcr_vol_90` and `pcr_oi_30 - pcr_oi_180`
   - Every candidate expression must incorporate:
     - Short-term reversal dampener: `- 0.05 * (rank(ts_delta(close, 3)) - 0.5)`
     - Non-linear transform: `signed_power(..., 1.4..2.0)`
     - Group neutralization: `group_neutralize(signed_power(signal, 3.8), densify(subindustry))`
     - Simulation decay: `decay: 16` to `decay: 22` to comfortably satisfy the $Turnover \le 70\%$ constraint.

---
*End of Specification Document (`taxonomy_news_options.md`).*
