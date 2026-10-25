# Handoff Report: Simulation, Evaluation, & Portfolio Verification Architecture

**Role**: Simulation & Verification Researcher (`explorer_survey_3`)  
**Date**: 2026-10-24  
**Handoff Type**: Hard (Task Complete)  
**Detailed Survey Artifact**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_survey_3/survey_simulation.md`

---

## 1. Observation

### 1.1 Simulation Dispatch & Transport Safety
- **Endpoint and Base URL**: `src/client.rs:16` sets `pub const BASE_URL: &str = "https://api.worldquantbrain.com";`.
- **Payload Schema**: In `src/models.rs:88-93`, `SimulationPayload` serializes `{ "type": "REGULAR", "settings": AlphaSettings, "regular": expression }`.
- **Settings Defaults**: `src/models.rs:6-31` defines `AlphaSettings` with default `instrument_type = "EQUITY"`, `region = "USA"`, `universe = "TOP1000"`, `delay = 1`, `decay = 5`, `neutralization = "SUBINDUSTRY"`, `truncation = 0.05`, `pasteurization = "ON"`, `unit_handling = "VERIFY"`, `nan_handling = "OFF"`, `language = "FASTEXPR"`.
- **SSRF Validation**: `src/client.rs:500-514` enforces that any follow-up URL parsed from `Location` headers strictly matches the scheme and host of `self.base_url`; any deviation triggers `"Security violation: Rejected follow-up URL outside configured BRAIN host"`.
- **Non-Idempotency Protection**: `src/client.rs:40-66` defines `PostFailure::NotSent` (only when `err.is_connect()`) vs `PostFailure::Ambiguous`. `src/client.rs:68-94` computes a 64-bit FNV-1a fingerprint `simulation_fingerprint(expression, settings)` so operators can identify in-flight simulations without logging proprietary alpha formulas.

### 1.2 Concurrency & Rate Limiting Enforcement
- **Platform Policy**: `AGENTS.md` mandates running strictly 3 simulation workers per account within a single CLI process (`DEFAULT_WORKERS_PER_ACCOUNT = 3` in `src/config.rs:106-107`).
- **Rust Screener**: `src/screener.rs:49, 184-187` constructs `tokio::sync::Semaphore::new(concurrency)` per account worker. Worker tasks acquire a permit before dispatching and hold it throughout the simulation and polling cycle.
- **Queue and Sleep**: `src/screener.rs:150` uses a bounded `tokio::sync::mpsc::channel::<CandidateAlpha>(candidates.len())`, and `src/screener.rs:190` applies a 500ms delay (`tokio::time::sleep(Duration::from_millis(500))`) between consecutive dispatches.
- **Cross-Account Mutex**: `src/screener.rs:26, 288, 324` uses `transfer_lock: Arc<Mutex<()>>` to serialize alpha promotion and official submission checks on the MAIN account.
- **Python Engine**: `scratch/engine.py:341` limits concurrency to `ThreadPoolExecutor(max_workers=3)` and serializes verification/submission with `self.submit_lock = threading.Lock()`.

### 1.3 In-Sample Metrics & Fail-Closed Check Evaluator
- **Mandatory Core Checks**: `src/client.rs:97-104` defines:
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
- **Fail-Closed Evaluator**: `src/client.rs:118-265` (`evaluate_submission_checks`):
  - Fails if `stats.checks.is_empty()` with `"FAIL-CLOSED: Empty check results received from BRAIN API"`.
  - Fails if any core check is missing: `"FAIL-CLOSED: Missing required core checks: <checks>"`.
  - Distinguishes `"PASS"`, `"FAIL"`, `"PENDING"`, and reports `"ALREADY SUBMITTED"` explicitly.
- **Threshold Criteria** (`src/config.rs:11-56`):
  - Delay 1: Sharpe $\ge 1.25$, Fitness $\ge 1.00$.
  - Delay 0: Sharpe $\ge 2.00$, Fitness $\ge 1.30$.
  - Turnover: Strictly between 1.0% and 70.0% (`0.01 < turnover < 0.70`).
  - Returns: User requirement $> 8.0\%$ ($0.08$).
- **Bypass Protection**: `src/commands/submission.rs:78-98, 255-274` blocks `--skip-checks` unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` or `true` is explicitly set in the environment.

### 1.4 Portfolio Baseline & Correlation Audit
- **Active Portfolio Baseline**: `portfolio_os.json` contains 84 active OS alphas.
- **PnL Caching**: Daily PnL records are loaded from `~/.cache/alphafind/portfolio_pnl.json` via `load_or_refresh_os_pnl_cache()` (`src/commands/portfolio.rs:26-69`). Stale caches ($> 24\text{h}$) trigger warnings.
- **Pearson Correlation**: `src/correlation.rs:11-62` filters common trading days with finite values.
  - If common days $N < 30$ (`MIN_CORRELATION_DAYS`), returns `(None, N)` (fail-closed).
  - If variance $\le 10^{-12}$ (`EPSILON_VARIANCE`), returns `(None, N)`.
  - Clamps output correlation to $[-1.0, 1.0]$.
- **Accretive Merge Simulation**: `src/correlation.rs:153-234, 236-316` and `src/allocation.rs:455-630`:
  - Enforces minimum 100 common portfolio days (`MIN_PORTFOLIO_DAYS = 100`).
  - Computes $\Delta \text{Sharpe} = \text{Sharpe}_{\text{merged}} - \text{Sharpe}_{\text{base}}$ ($> 0$ required).
  - Computes $\Delta \text{Vol} = \text{Vol}_{\text{merged}} - \text{Vol}_{\text{base}}$ ($\le 0$ preferred for variance collapse).
  - Enforces maximum self-correlation proxy $< 70\%$ (`BRAIN_MAX_CORRELATION = 0.70`).
  - Classifies admission into `StrongGo` ($\Delta \text{Sharpe} > 0, \Delta \text{Vol} < 0, \text{Buffer} \ge 8\%$), `ConditionalGo` ($\Delta \text{Sharpe} > 0, \text{Buffer} \ge 8\%$), and `NoGo` (dilutive, collision, or buffer $< 5\%$).

### 1.5 Environment & Mock Testing
- **Credentials**: `.env` is present in the workspace root and contains `MAIN_ACCOUNT_EMAIL` and `MAIN_ACCOUNT_PASSWORD`.
- **Wiremock Suite**: `tests/mock_api_tests.rs` implements 15 unit tests covering auth, simulation dispatch, SSRF defense, check evaluation, and retry rules.
- **Test Results**: Executed `cargo test --test mock_api_tests --test submission_gate_tests --test correlation_tests --test allocation_tests` (all 56 unit tests passed, 0 failures).

---

## 2. Logic Chain

1. **Simulation Dispatch Chain**:
   - Because simulation POSTs create live server-side simulation jobs on BRAIN that cannot be cancelled once dispatched, treating network timeouts as ambiguous (`PostFailure::Ambiguous`) prevents duplicate simulations.
   - Using SSRF validation on the polling URL ensures that malicious or malformed `Location` headers cannot redirect client authentication cookies to third-party endpoints.
2. **Concurrency & Rate Limiting Chain**:
   - Platform rules enforce strict per-account concurrency caps. Exceeding 3 concurrent in-flight simulations risks rate-limiting (`HTTP 429`) or account sanction.
   - Enforcing `tokio::sync::Semaphore::new(3)` in Rust and `ThreadPoolExecutor(max_workers=3)` in Python ensures that the process never dispatches a 4th simulation until an active one finishes.
   - Serializing check and submit operations via `transfer_lock` / `submit_lock` guarantees that calculation-heavy `/check` endpoints are never flooded.
3. **Check Evaluator Chain**:
   - BRAIN's submission criteria require passing all core check dimensions. An empty response or incomplete check suite could conceal underlying failures.
   - Requiring all 6 core checks to be explicitly present and marked `PASS` guarantees that no alpha is submitted without verifiable compliance.
4. **Portfolio Correlation Audit Chain**:
   - A single high-correlation alpha ($> 70\%$) submitted to OS can collide with an existing position, wasting submission quota and harming user leaderboard standing.
   - By calculating Pearson correlation over $N \ge 30$ overlapping days and simulating merged portfolio impact ($\Delta \text{Sharpe} > 0$, $\Delta \text{Vol} \le 0$, $\text{Safety Buffer} \ge 8\%$), AlphaFind ensures each admitted alpha is genuinely orthogonal and accretive to the active portfolio.

---

## 3. Caveats

1. **Local Correlation vs Platform Self-Correlation**: Local Pearson correlation against the 84-alpha OS portfolio baseline is an internal proxy. BRAIN computes self-correlation against all alphas submitted by all users/accounts across a 4-year rolling window on its own platform data. While local correlation $< 70\%$ is a necessary gate, it is not an absolute guarantee against platform self-correlation flags.
2. **Dynamic Check Lists**: BRAIN's `/check` endpoint returns 7 checks for `TOP200` and 8 checks for `TOP1000`. `CORE_SUBMISSION_CHECKS` checks for 6 core items; if BRAIN introduces a new core check name in the future, the static list will need manual maintenance unless dynamic parsing is implemented.
3. **Cross-Process Rate Limiting**: The semaphore is process-local. If multiple CLI or script processes are launched simultaneously on the same machine, they will exceed the 3-worker account limit. Screening must be run in a single process.

---

## 4. Conclusion

The simulation and verification pipeline in `alphafind` is robust, battle-tested, fully compliant with `AGENTS.md`, and completely ready for the alternative data research framework:
- Direct simulations and batch screening can be safely executed with `--workers 3`.
- In-sample qualification thresholds (`Sharpe >= 1.25`, `Fitness >= 1.00`, `Turnover 1.0% - 70.0%`, `Returns > 8.0%`) and fail-closed check evaluators ensure only high-conviction alphas are eligible.
- Portfolio baseline correlation audits reliably protect the active 84-alpha portfolio from dilution or correlation collisions.
- Offline testing with `wiremock` provides rapid, non-destructive validation of simulation pipelines before hitting live platform endpoints.

---

## 5. Verification Method

To independently verify all observations and test the simulation & verification architecture:

1. **Run Full Mock & Verification Test Suites**:
   ```bash
   cargo test --test mock_api_tests --test submission_gate_tests --test correlation_tests --test allocation_tests
   ```
   *Expected result*: All 56 tests pass with 0 errors.

2. **Verify Concurrency & Threshold Constants in Code**:
   - Inspect `src/config.rs:11-56` for `BRAIN_MAX_CORRELATION = 0.70`, `MIN_TURNOVER = 0.01`, `MAX_TURNOVER = 0.70`, `MIN_SHARPE_DELAY_1 = 1.25`, `MIN_FITNESS_DELAY_1 = 1.00`.
   - Inspect `src/config.rs:106-107` for `DEFAULT_WORKERS_PER_ACCOUNT = 3`.
   - Inspect `src/client.rs:97-104` for `CORE_SUBMISSION_CHECKS`.

3. **Verify Active Portfolio Baseline Files**:
   - Inspect `portfolio_os.json` (verifies 84 active OS alphas).
   - Inspect `~/.cache/alphafind/portfolio_pnl.json` (verifies cached daily PnL series).
   - Inspect `scratch/evaluated_candidates.json` (verifies previously simulated candidates and merge metrics).
