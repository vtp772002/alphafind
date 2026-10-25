# Milestone M4 Specification & Handoff Report: In-Sample Qualification, Submission Checks & Portfolio Impact Evaluation

**Author**: `explorer_m4_2` (Teamwork Specification Miner)  
**Date**: 2026-10-24T22:28:00Z  
**Target Milestone**: M4 (Verification, Simulation Dispatcher, Check Gate & Portfolio Impact)  
**Recipient**: `parent` (Orchestrator: `8b020a34-8c2d-4255-98f9-544909ff7652`)  

---

## Executive Summary

This specification defines the complete quantitative evaluation architecture for Milestone M4. It audits the active Out-of-Sample (OS) portfolio baseline, establishes the mathematical mechanics and fail-closed guarantees for Pearson cross-correlation and portfolio merge impact, formalizes the 6-core platform submission check gate, and specifies four backtest-ready candidate hypotheses across alternative datasets (`model51`, `model16`, `news18`, and `option9`).

---

## Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | Baseline Portfolio | Active OS Portfolio Baseline | 94 active OS alphas with full metadata and FASTEXPR code definitions | `portfolio_os.json` | 94 records (universes: TOP1000: 26, TOP3000: 25, TOP500: 23, TOP200: 20) | Bail if file missing (`alphafind sync` required) | `src/commands/mod.rs:118-160` |
| 2 | PnL Cache | Local PnL History Cache | Daily historical PnL per alpha over 1,236 common trading days (2019-01-02 to 2023-12-29) | `~/.cache/alphafind/portfolio_pnl.json` | 113 alpha records; 100% coverage of active OS alphas | Stale warning if age > 24h; live fetch if missing ID | `src/config.rs:103`, `src/commands/portfolio.rs:26-69` |
| 3 | Correlation Math | Overlapping-Date Pearson Correlation | Pairwise Pearson $\rho$ evaluated strictly over shared non-NaN trading days | `x_dict`, `y_dict` (`HashMap<String, f64>`) | `(Option<f64>, usize)` | Returns `None` if $N < 30$ or variance $\le 10^{-12}$ (fail-closed) | `src/correlation.rs:11-62` |
| 4 | Correlation Audit | Candidate Correlation Audit | Exhaustive pairwise audit of candidate PnL against all portfolio alphas (skipping self) | `candidate_id`, `cand_pnl`, `os_pnls` | `CorrelationAuditReport` ($\max \rho$, $\bar{\rho}$, most correlated ID) | Fail-closed if 0 valid comparisons ($N < 30$) | `src/correlation.rs:84-140` |
| 5 | Portfolio Simulation | Equal-Weight Portfolio Aggregation | Multi-alpha portfolio PnL, annualized mean return, sample volatility, and Sharpe | `Vec<&HashMap<String, f64>>` | `PortfolioMergeMetrics` ($\text{Sharpe}$, $\text{Vol}$, $\text{PnL}$, $\bar{\rho}$) | Returns `None` if intersection trading days $< 100$ | `src/correlation.rs:154-234` |
| 6 | Merge Impact | Incremental Portfolio Merge Impact | Evaluates $\Delta\text{Sharpe} = \text{Sharpe}_{\text{merged}} - \text{Sharpe}_{\text{base}}$ and $\Delta\text{Vol} = \text{Vol}_{\text{merged}} - \text{Vol}_{\text{base}}$ | Candidate PnL + OS baseline PnLs | `PortfolioImpactResult` ($\Delta\text{Sharpe}$, $\Delta\text{Vol}$, safety buffer) | Returns `None` if baseline or merged sim has $< 100$ common days | `src/correlation.rs:237-316` |
| 7 | Admission Decision | Automated Admission Gate | Quantitative admission decision evaluating Sharpe delta, variance collapse, turnover entropy, and safety buffer | Candidate PnL, turnover, OS alphas, PnL cache | `CandidateAdmissionReport` (`StrongGo`, `Go`, `NoGo`, `TrimCandidate`) | Fail-closed (`NoGo`) on any missing pairs or $\max \rho \ge 0.70$ | `src/allocation.rs:455-600` |
| 8 | Submission Checks | 6 Core Submission Checks Evaluator | Evaluates `LOW_SHARPE`, `LOW_FITNESS`, `LOW_TURNOVER`, `HIGH_TURNOVER`, `CONCENTRATED_WEIGHT`, `SELF_CORRELATION` | `InSampleStats` (`checks: Vec<CheckItem>`) | `SubmissionCheckResult` (`passed: bool`, message, counts) | Fails closed on empty checks, missing core check, `PENDING`, `ERROR`, `UNKNOWN` | `src/client.rs:97-260` |
| 9 | Pre-flight Gate | Pre-Flight Submission Verification | Enforces platform check verification before allowing POST to `/alphas/{id}/submit` | Alpha ID, main client | Execution gate; `--skip-checks` requires `ALPHAFIND_ALLOW_SKIP_CHECKS=1` | Blocks submission if checks fail or if skip-checks unauthorized | `src/commands/submission.rs:78-98, 229-370` |
| 10 | Concurrency Limiter | Multi-Account Screener Concurrency | Semaphore-bounded worker queue enforcing strict 3 concurrent simulation jobs per account | Work queue `mpsc::channel`, Semaphore | Dispatches simulations with 500ms stagger | Rejects cross-process concurrent screeners per `AGENTS.md` | `src/screener.rs:49, 184-190`, `src/config.rs:106` |
| 11 | Alternative Hypotheses | Automated Alternative Factor Combinator | Generates FastExpr candidate alphas for `model51`, `model16`, `news18`, and `option9` | `FactorSpecification` | Validated `GeneratedExpression` | Rejects malformed ASTs, unbackfilled fields, or undensified groups | `src/expression_builder.rs:480-685` |

---

## Edge Cases & Observed Validator Behavior

| # | Feature | Input Condition | Observed Behavior | Governing Code / Test |
|---|---------|-----------------|-------------------|------------------------|
| 1 | Pearson Overlap | Overlapping trading days $N = 29$ | Returns `(None, 29)` — fails closed; cannot claim compliance. | `src/correlation.rs:23-25`, `tests/correlation_tests.rs:51-60` |
| 2 | Pearson Overlap | Overlapping trading days $N = 30$ | Computes exact covariance and returns `(Some(corr), 30)`. | `src/correlation.rs:23-55`, `tests/correlation_tests.rs:22-34` |
| 3 | Zero Variance | Series flatlines with constant daily PnL (e.g. all \$0.0 or \$5.0) | Denominator $(s_x^2 s_y^2)^{1/2} \le 10^{-12}$; returns `(None, N)`. | `src/correlation.rs:52-60`, `tests/correlation_tests.rs:75-93` |
| 4 | NaN / Infinity PnL | Candidate PnL contains `NaN` or `Infinity` on 10 of 35 common dates | Invalid entries filtered out; valid $N = 25 < 30 \implies (None, 25)$. | `src/correlation.rs:19-25`, `tests/correlation_tests.rs:268-301` |
| 5 | Portfolio Simulation | Portfolio common trading days across all alphas $N = 95 < 100$ | Returns `None`; simulation fails closed. | `src/correlation.rs:178-180`, `tests/correlation_tests.rs:304-319` |
| 6 | Candidate Self-Match | Auditing an alpha whose ID is already in `portfolio_pnl.json` | Candidate skips itself; only compares against other alphas. | `src/correlation.rs:97-99`, `tests/correlation_tests.rs:118-128` |
| 7 | Empty Portfolio | Auditing candidate against empty portfolio or missing cache | `valid_comparisons = 0`; fails closed (`is_within_correlation_proxy = false`). | `src/correlation.rs:125-128`, `tests/correlation_tests.rs:131-144` |
| 8 | Missing Core Check | Platform returns 5 of 6 core checks (omits `SELF_CORRELATION`) | `evaluate_submission_checks` returns `passed = false` (`FAIL-CLOSED: Missing required core checks`). | `src/client.rs:214-226`, `tests/submission_gate_tests.rs:116-130` |
| 9 | Pending Core Check | Platform returns `PENDING` for `LOW_SHARPE` | Returns `passed = false` (`CHECKS STILL PENDING: LOW_SHARPE`). | `src/client.rs:229-239`, `tests/submission_gate_tests.rs:99-114` |
| 10 | Unrecognized Check Status | Platform returns new status `REVIEW` or empty string | Evaluated as `CheckDisplay::Unknown`, fails closed (`passed = false`). | `src/commands/submission.rs:121-127`, `tests/submission_gate_tests.rs:48-59` |
| 11 | Already Submitted Alpha | Platform returns single check `ALREADY_SUBMITTED` | Evaluator returns `passed = false` with explicit explanation; never mislabeled as pass. | `src/client.rs:195-212`, `tests/submission_gate_tests.rs:150-165` |
| 12 | Skip Checks Bypass | `--skip-checks` invoked without `ALPHAFIND_ALLOW_SKIP_CHECKS=1` | Command terminates immediately with fatal error; gate remains shut. | `src/commands/submission.rs:88-98, 255-263`, `tests/submission_gate_tests.rs:80-96` |

---

## 1. Observation

Direct code and data analysis reveals the following findings:

### 1.1 Active OS Portfolio Baseline (`portfolio_os.json`)
- **Location**: `/Users/phamtoan/Developer/alphafind/portfolio_os.json`
- **Total Alphas**: Exactly **94** records (line 1 to line 3759).
- **Status & Stage**: All 94 records have `"status": "ACTIVE"` and `"stage": "OS"`.
- **Classification Compliance**: In `src/commands/mod.rs:81-114`, `classify_portfolio_alpha()` verifies that records with `status == "ACTIVE"` and `stage == "OS"` map to `AlphaClassification::ActiveOs`. All 94 alphas are retained in `load_portfolio_os()` (lines 138-143).
- **Date Submitted Distribution**:
  - Submissions range from `2026-09-14T03:23:54-04:00` to `2026-10-24T03:11:17-04:00`.
  - Exactly **84 alphas** were submitted prior to `2026-10-24`.
  - Exactly **10 alphas** were submitted on `2026-10-24`.
  - This explains the citation of "84 baseline alphas" in `PROJECT.md` line 42 and the dispatch assignment: 84 was the historical baseline prior to the latest batch of 10 alphas.
- **Universe Allocation**:
  - `TOP1000`: 26 alphas (27.7%)
  - `TOP3000`: 25 alphas (26.6%)
  - `TOP500`: 23 alphas (24.5%)
  - `TOP200`: 20 alphas (21.3%)
- **Settings Consistency**: All 94 alphas share standard settings: `delay: 1`, `language: "FASTEXPR"`, `instrumentType: "EQUITY"`, `region: "USA"`, `pasteurization: "ON"`, `unitHandling: "VERIFY"`, `nanHandling: "OFF"`, `truncation: 0.05` to `0.06`.

### 1.2 Local PnL Cache (`~/.cache/alphafind/portfolio_pnl.json`)
- **Location**: `/Users/phamtoan/.cache/alphafind/portfolio_pnl.json`
- **Total Cache Entries**: 113 unique Alpha IDs.
- **Coverage**: All 94 active OS alphas in `portfolio_os.json` exist in the cache (100% active baseline coverage).
- **Observation Count**: Every single active alpha has exactly **1,236 common daily PnL records**.
- **Date Coverage**: Exact span from **`2019-01-02` to `2023-12-29`** (5 full calendar years).
- **Cache Staleness & Freshness Policy**:
  - `src/config.rs:103`: `pub const PNL_CACHE_MAX_AGE_HOURS: u64 = 24;`
  - `src/commands/mod.rs:218-232`: `cache_age_label()` classifies cache age into `Fresh (<24h)`, `Stale (>24h)`, or `Missing`.
  - `src/commands/portfolio.rs:26-69`: `load_or_refresh_os_pnl_cache()` checks cache age and loads existing cache; if `--refresh` is supplied or an alpha ID is missing, it calls `client.fetch_daily_pnl(&id)` and writes back to disk with `0600` permissions via `write_private_file()`.

### 1.3 Pearson Correlation & Portfolio Merge Impact Mathematics
- **Pearson Implementation (`src/correlation.rs:11-62`)**:
  ```rust
  let common: Vec<&&String> = x_keys.intersection(&y_keys)
      .filter(|&&d| x_dict[d].is_finite() && y_dict[d].is_finite())
      .collect();
  let n = common.len();
  if n < MIN_CORRELATION_DAYS { return (None, n); } // MIN_CORRELATION_DAYS = 30
  ```
  Zero-variance filter:
  ```rust
  let denom = (var_x * var_y).sqrt();
  if var_x > EPSILON_VARIANCE && var_y > EPSILON_VARIANCE && denom > EPSILON_VARIANCE {
      (Some((cov / denom).clamp(-1.0, 1.0)), n)
  } else {
      (None, n)
  }
  ```
- **Portfolio Merge Simulation (`src/correlation.rs:154-234`)**:
  - Finds common intersection dates across all $M$ alphas.
  - Requires $N_{\text{days}} \ge 100$ (`MIN_PORTFOLIO_DAYS = 100`).
  - Computes daily portfolio return: $R_{\text{port}, t} = \frac{1}{M} \sum_{m=1}^M R_{m, t}$.
  - Computes sample variance: $s^2 = \frac{1}{N - 1} \sum_{t=1}^N (R_{\text{port}, t} - \bar{\mu})^2$.
  - Annualizes: $\text{PnL}_{\text{ann}} = \bar{\mu} \times 252.0$, $\text{Vol}_{\text{ann}} = s \times \sqrt{252.0}$.
  - Merged Sharpe: $\text{Sharpe}_{\text{merged}} = \text{PnL}_{\text{ann}} / \text{Vol}_{\text{ann}}$ if $\text{Vol}_{\text{ann}} > 10^{-9}$.
- **Portfolio Merge Impact (`src/correlation.rs:237-316`)**:
  - Baseline portfolio simulated without candidate.
  - Combined portfolio simulated with candidate.
  - $\Delta\text{Sharpe} = \text{Sharpe}_{\text{merged}} - \text{Sharpe}_{\text{base}}$.
  - $\Delta\text{Vol} = \text{Vol}_{\text{merged}} - \text{Vol}_{\text{base}}$.
  - $\Delta\text{PnL} = \text{PnL}_{\text{merged}} - \text{PnL}_{\text{base}}$.
  - Max Pairwise Correlation: $\max_{j} \rho(\text{cand}, \alpha_j)$.
  - Safety Buffer: $\max(0.0, 0.70 - \max \rho) \times 100\%$.
  - Accretive Impact Criteria: $\max \rho < 0.70$, $\Delta\text{Sharpe} > 0$, $\Delta\text{Vol} \le 0$.

### 1.4 Submission Check Evaluator (`src/client.rs:97-260`, `src/commands/submission.rs:100-226`)
- **6 Core Checks**:
  ```rust
  pub const CORE_SUBMISSION_CHECKS: &[&str] = &[
      "LOW_SHARPE",
      "LOW_FITNESS",
      "LOW_TURNOVER",
      "HIGH_TURNOVER",
      "CONCENTRATED_WEIGHT",
      "SELF_CORRELATION",
  ];
  ```
- **Fail-Closed Gate**:
  - Empty checks response: returns `passed = false` (`"FAIL-CLOSED: Empty check results received from BRAIN API"`).
  - Missing any of the 6 core checks: returns `passed = false` (`"FAIL-CLOSED: Missing required core checks: ..."`).
  - Any check with status `PENDING`: returns `passed = false` (`"CHECKS STILL PENDING: ..."`).
  - Any check with status `FAIL`: returns `passed = false` (`"FAILED CHECKS: ..."`).
  - Unrecognized status: classified as `CheckDisplay::Unknown`, returns `passed = false`.
  - Pass condition: strictly requires 100% of reported checks to be explicit `PASS` and all 6 core checks to be present.
- **Pre-flight Enforcement**:
  - `cmd_submit` enforces check verification by default: queries `check_submission(&alpha_id, 100)`.
  - `--skip-checks` requires environment variable `ALPHAFIND_ALLOW_SKIP_CHECKS=1` or `true`. Otherwise aborts with an error.

### 1.5 Concurrency Bounding & Screening Engine (`src/screener.rs`)
- In `src/screener.rs:49`, each `AccountWorker` instantiates a semaphore with permits equal to `concurrency = workers_per_account.max(1)`.
- Default: `DEFAULT_WORKERS_PER_ACCOUNT = 3` in `src/config.rs:106`.
- In `src/screener.rs:184-190`, every worker task must acquire a semaphore permit before submitting a simulation, enforcing that at most 3 simulations are in flight concurrently per account.
- Rate-limiting: `tokio::time::sleep(Duration::from_millis(500))` polite spacing between dispatches.

---

## 2. Logic Chain

1. **Baseline Portfolio Size & History**:
   - Observation 1.1 shows 94 active OS alphas in `portfolio_os.json`.
   - 84 were submitted prior to 2026-10-24, and 10 on 2026-10-24.
   - Observation 1.2 shows that all 94 alphas exist in `~/.cache/alphafind/portfolio_pnl.json` with 1,236 common daily PnL records spanning 2019 to 2023.
   - Therefore, the baseline portfolio correlation audit can evaluate candidates across the full active OS portfolio (94 alphas) with 1,236 trading days of common history, far exceeding the $N \ge 30$ minimum overlap threshold.

2. **Mathematical Robustness of Overlap & Merge Impact**:
   - Observation 1.3 shows that `pearson_correlation()` requires $N \ge 30$ and strictly filters out non-finite (`NaN`/`Infinity`) PnL values.
   - Any pair with $N < 30$ or variance $\le 10^{-12}$ returns `None`.
   - In `audit_candidate()`, `is_within_correlation_proxy` is `false` if `valid_comparisons == 0`.
   - In `simulate_portfolio()`, the common intersection across all alphas must have $N_{\text{days}} \ge 100$.
   - In `calculate_portfolio_impact()`, if any pairwise correlation fails or if the baseline or merged simulation cannot be computed over 100 days, the calculation returns `None` (fail-closed).
   - Therefore, no candidate can pass the correlation or merge impact gate on insufficient or degraded data.

3. **Accretive Merge Criteria**:
   - A candidate is accretive to the portfolio if and only if:
     1. $\max \rho < 0.70$ (satisfies the platform proxy threshold).
     2. $\Delta\text{Sharpe} = \text{Sharpe}_{\text{merged}} - \text{Sharpe}_{\text{base}} > 0$ (improves risk-adjusted return).
     3. $\Delta\text{Vol} = \text{Vol}_{\text{merged}} - \text{Vol}_{\text{base}} \le 0$ (provides variance reduction / diversification benefit).
   - In `src/allocation.rs:580-600`, candidates satisfying these conditions alongside turnover quota balance receive `StrongGo` or `Go` admission verdicts.

4. **Submission Safety and Gate Invariants**:
   - Observation 1.4 confirms that `evaluate_submission_checks()` requires all 6 core checks (`LOW_SHARPE`, `LOW_FITNESS`, `LOW_TURNOVER`, `HIGH_TURNOVER`, `CONCENTRATED_WEIGHT`, `SELF_CORRELATION`) to be present and explicitly `PASS`.
   - Empty responses, pending checks, errors, and unrecognized statuses fail closed.
   - Pre-flight verification in `cmd_submit` prevents unverified candidates from being submitted.
   - Therefore, the submission evaluator ensures that only fully qualified alphas reach the platform submission API.

5. **Candidate Hypothesis Design for M4**:
   - From `src/expression_builder.rs` and `factor_combinator_specifications.md`, alternative datasets (`model51`, `model16`, `news18`, `option9`) provide orthogonal information pillars completely absent from conventional price-volume momentum.
   - All generated expressions have been validated with 0 errors via `SchemaValidator::validate`.
   - Standardizing settings to `universe: "TOP1000"`, `delay: 1`, `decay: 16-20`, `neutralization: "SUBINDUSTRY"`, `truncation: 0.05` ensures liquidity, platform compliance, and optimal information extraction.

---

## 3. Caveats

1. **Local Correlation vs Platform Self-Correlation**:
   - `alphafind` calculates Pearson $\rho$ using daily dollar PnL from the local cache.
   - WorldQuant BRAIN calculates its own proprietary self-correlation metric during the submission check using tick-level or platform-standard returns over rolling windows.
   - While $\max \rho < 0.70$ locally serves as an effective pre-screening proxy, the official platform check is the sole authority for OOS eligibility.
2. **Account Concurrency Limits**:
   - `MultiAccountScreener` bounds in-flight simulations to 3 per account within a single process.
   - Because process-local semaphores do not coordinate across multiple terminal processes or external scripts, only one CLI simulation process must be run at any given time.
3. **Data Coverage & Vector Folding**:
   - `news18` intraday vector fields (`VECTOR` type, such as `nws18_ssc` and `nws18_bam`) require explicit vector folding (`vec_avg` or `vec_sum`) and `ts_backfill(..., 15)`. Directly combining a vector with matrix arithmetic triggers HTTP 400 simulation failure.
4. **Options Analytics Field Naming**:
   - `option9` field names omit trailing `'d'` (e.g. `call_breakeven_30`, NOT `call_breakeven_30d`), and nominal breakeven price levels must be normalized by `close`.

---

## 4. Concrete Formulas, Candidate Selection & M4 Verification Thresholds

### 4.1 Candidate Hypotheses Selection

The following 4 candidate hypotheses across `model51`, `model16`, `news18`, and `option9` are specified for simulation in Milestone M4:

#### Candidate 1: `US_M51_Beta_Term_Spread` (`model51` Systematic Risk)
- **FastExpr Formula**:
  ```fastexpr
  group_neutralize(signed_power(((rank(0.65 * signed_power(0.5 - rank(ts_backfill(beta_last_60_days_spy, 10)), 1.5) - 0.35 * signed_power(rank(ts_backfill(beta_last_30_days_spy, 10) - ts_backfill(beta_last_360_days_spy, 15)) - 0.5, 1.5)) - 0.5) - 0.05 * (rank(ts_delta(close, 3)) - 0.5)), 4.0), densify(subindustry))
  ```
- **Parameter Configuration**:
  - `universe`: `TOP1000`
  - `delay`: 1
  - `decay`: 18
  - `neutralization`: `SUBINDUSTRY`
  - `truncation`: 0.05
  - `pasteurization`: `ON`, `unitHandling`: `VERIFY`, `nanHandling`: `OFF`, `language`: `FASTEXPR`
- **Economic Justification & Transmission**:
  - Combines the structural Low-Beta Anomaly (Frazzini & Pedersen 2014 BAB; low-beta equities outperform lottery-like high-beta equities) with a Beta Term Slope ($\beta_{30d} - \beta_{360d}$). A sudden surge in short-term beta signals institutional distress and liquidation selling. Subtracting short-term 3-day price reversal stabilizes turnover.

---

#### Candidate 2: `US_M16_Fundamental_Acceleration_Convexity` (`model16` Fundamental Scores)
- **FastExpr Formula**:
  ```fastexpr
  group_neutralize(signed_power(((rank(0.70 * signed_power(rank(multi_factor_acceleration_score_derivative) - 0.5, 1.5) + 0.30 * signed_power(rank(multi_factor_static_score_derivative) - 0.5, 1.5)) - 0.5) - 0.06 * signed_power(rank(ts_delta(close, 3)) - 0.5, 1.5)), 4.0), densify(subindustry))
  ```
- **Parameter Configuration**:
  - `universe`: `TOP1000`
  - `delay`: 1
  - `decay`: 18
  - `neutralization`: `SUBINDUSTRY`
  - `truncation`: 0.05
- **Economic Justification & Transmission**:
  - Static fundamental valuation often traps capital in structurally impaired firms (value traps). Fundamental acceleration convexity (the second time derivative of multi-factor fundamental health) identifies operational inflections where earnings growth is accelerating ahead of sell-side consensus revisions. These fields possess 100% cross-sectional coverage across all US equities.

---

#### Candidate 3: `US_N18_Vector_Folded_Story_Sentiment` (`news18` RavenPack News)
- **FastExpr Formula**:
  ```fastexpr
  group_neutralize(signed_power(((rank(signed_power(ts_decay_linear(if_else(is_nan(ts_backfill(vec_avg(nws18_ssc), 15)), 0, rank(ts_backfill(vec_avg(nws18_ssc), 15)) - 0.5), 10), 1.5)) - 0.5) - 0.05 * signed_power(rank(ts_delta(close, 3)) - 0.5, 1.5)), 4.0), densify(subindustry))
  ```
- **Parameter Configuration**:
  - `universe`: `TOP1000`
  - `delay`: 1
  - `decay`: 16
  - `neutralization`: `SUBINDUSTRY`
  - `truncation`: 0.05
- **Economic Justification & Transmission**:
  - RavenPack real-time news stories arrive as intraday 3D event vectors (`VECTOR` type). Folding with `vec_avg(nws18_ssc)` collapses story sentiment into a daily cross-sectional matrix. Because corporate news is episodic, applying `ts_backfill(..., 15)` preserves signal continuity. Decaying with `ts_decay_linear(..., 10)` captures post-news announcement drift while suppressing high-frequency turnover churn.

---

#### Candidate 4: `US_O9_Breakeven_Directional_Asymmetry_30d` (`option9` Options Analytics)
- **FastExpr Formula**:
  ```fastexpr
  group_neutralize(signed_power((signed_power(rank(ts_backfill(1.0 - put_breakeven_30 / close, 10) - ts_backfill(call_breakeven_30 / close - 1.0, 10)) - 0.5, 1.5) - 0.06 * signed_power(rank(ts_delta(close, 3)) - 0.5, 1.5)), 4.0), densify(subindustry))
  ```
- **Parameter Configuration**:
  - `universe`: `TOP1000`
  - `delay`: 1
  - `decay`: 16
  - `neutralization`: `SUBINDUSTRY`
  - `truncation`: 0.05
- **Economic Justification & Transmission**:
  - The breakeven price difference $(1 - \frac{\text{put\_breakeven}}{\text{close}}) - (\frac{\text{call\_breakeven}}{\text{close}} - 1)$ isolates directional options skew. When downside put cushions widen excessively relative to upside call hurdles, options market participants are paying an extreme panic premium for crash insurance. As transient panic dissipates, equity prices mean-revert upward.

---

### 4.2 Milestone M4 Evaluation & Qualification Checklist

For M4 Worker execution, each simulated candidate must be evaluated against the following 8 qualification gates:

| # | Gate | Threshold / Condition | Source / Evaluator | Policy Classification |
|---|------|-----------------------|--------------------|-----------------------|
| 1 | In-Sample Information Ratio | $\text{Sharpe} \ge 1.25$ (Delay 1) | BRAIN IS Simulation Stats | Official Platform Rule |
| 2 | In-Sample Fitness | $\text{Fitness} \ge 1.00$ (Delay 1) | BRAIN IS Simulation Stats | Official Platform Rule |
| 3 | Annualized Turnover | $1.0\% \le \tau \le 70.0\%$ | BRAIN IS Simulation Stats | Official Platform Rule |
| 4 | Annualized Return | $\text{Returns} > 8.0\%$ | BRAIN IS Simulation Stats | Official Platform Rule |
| 5 | Platform Submission Checks | 6 / 6 Core Checks `PASS` | `evaluate_submission_checks` (`src/client.rs`) | Official Platform Rule |
| 6 | Portfolio Cross-Correlation | $\max \rho < 70.0\%$ (vs active OS baseline) | `audit_candidate` (`src/correlation.rs`) | Project Policy |
| 7 | Incremental Sharpe Delta | $\Delta\text{Sharpe} = \text{Sharpe}_{\text{merged}} - \text{Sharpe}_{\text{base}} > 0$ | `calculate_portfolio_impact` | Project Policy |
| 8 | Incremental Volatility Delta | $\Delta\text{Vol} = \text{Vol}_{\text{merged}} - \text{Vol}_{\text{base}} \le 0$ | `calculate_portfolio_impact` | Project Policy |

---

## 5. Verification Method

To independently verify the components of this specification:

1. **Verify Expression Generation & Schema Validation**:
   ```bash
   cargo run -- generate -d model51 -n 1 -u TOP1000 --delay 1 --neutralization SUBINDUSTRY --validate
   cargo run -- generate -d model16 -n 1 -u TOP1000 --delay 1 --neutralization SUBINDUSTRY --validate
   cargo run -- generate -d news18 -n 1 -u TOP1000 --delay 1 --neutralization SUBINDUSTRY --validate
   cargo run -- generate -d option9 -n 1 -u TOP1000 --delay 1 --neutralization SUBINDUSTRY --validate
   ```
   *Expected Result*: All generated expressions output `0 errors` and pass `SchemaValidator` with green confirmation.

2. **Run Programmatic Unit & Property Test Suites**:
   ```bash
   cargo test --test expression_builder_tests
   cargo test --test correlation_tests
   cargo test --test submission_gate_tests
   cargo test --test stress_schema_validator_tests
   ```
   *Expected Result*: All 66 tests pass with 0 failures (`test result: ok`).

3. **Verify Baseline Portfolio & PnL Cache Integrity**:
   ```bash
   python3 -c "
   import json, os
   with open('portfolio_os.json') as f:
       os_data = json.load(f)
   with open(os.path.expanduser('~/.cache/alphafind/portfolio_pnl.json')) as f:
       pnl_data = json.load(f)
   assert len(os_data) == 94, f'Expected 94 OS alphas, got {len(os_data)}'
   os_ids = set(x['id'] for x in os_data)
   assert os_ids.issubset(set(pnl_data.keys())), 'Missing PnL cache for some OS alphas'
   print('Baseline Portfolio and PnL Cache Integrity: VERIFIED')
   "
   ```

4. **Verify Pre-Push Git Hygiene Sanitizer**:
   ```bash
   ./scripts/sanitize_check.sh
   ```
   *Expected Result*: Sanitizer exits with code 0 (`ALL SANITIZATION CHECKS PASSED`).

---

## Invalidation Conditions

This specification is invalidated if:
1. WorldQuant BRAIN modifies the Delay 1 In-Sample qualification thresholds (e.g. Sharpe $< 1.25$ or Fitness $< 1.00$).
2. The platform submission API alters the set of core checks or removes `LOW_SHARPE`, `LOW_FITNESS`, `LOW_TURNOVER`, `HIGH_TURNOVER`, `CONCENTRATED_WEIGHT`, or `SELF_CORRELATION`.
3. The active OS portfolio baseline changes (e.g. decommissioned alphas) such that common trading days drop below 100 days.
4. FastExpr syntax rules change regarding `vec_avg()` folding or `densify()` group wrappers.
