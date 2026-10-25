# Simulation, Evaluation, & Portfolio Verification Pipeline Survey

**Author**: Simulation & Verification Researcher (`explorer_survey_3`)  
**Date**: 2026-10-24  
**Workspace**: `/Users/phamtoan/Developer/alphafind`  
**Compliance Context**: `AGENTS.md`, `docs/BRAIN_RULES_AND_SUBMISSION.md`, `ORIGINAL_REQUEST.md`

---

## Executive Summary

The simulation, evaluation, and portfolio verification architecture in `alphafind` is designed as a dual-layer system combining **Rust high-assurance core modules** (`src/client.rs`, `src/screener.rs`, `src/correlation.rs`, `src/allocation.rs`, `src/commands/submission.rs`) and **Python high-throughput research scripts** (`scratch/engine.py`, `scratch/screen_and_evaluate.py`).

Key architectural highlights:
1. **Simulation Dispatch**: Clean abstraction over WorldQuant BRAIN's REST API (`https://api.worldquantbrain.com`) with SSRF host validation, non-idempotent request classification, FNV-1a fingerprinting, and automatic exponential backoff on `HTTP 429`.
2. **Strict Concurrency Enforcement**: Per `AGENTS.md`, concurrency is strictly capped at `max_workers = 3` per account using a bounded `tokio::sync::mpsc` queue and `tokio::sync::Semaphore` permits in Rust, and `ThreadPoolExecutor(max_workers=3)` in Python. A cross-account transfer mutex prevents race conditions during official submission gates.
3. **Fail-Closed In-Sample Check Evaluator**: Every candidate alpha is evaluated against 6 mandatory core checks (`LOW_SHARPE`, `LOW_FITNESS`, `LOW_TURNOVER`, `HIGH_TURNOVER`, `CONCENTRATED_WEIGHT`, `SELF_CORRELATION`). Empty responses, pending states, errors, or unrecognized statuses fail closed. Pre-flight bypass (`--skip-checks`) is disabled by default and requires explicit opt-in via `ALPHAFIND_ALLOW_SKIP_CHECKS=1`.
4. **Local Portfolio Baseline & Accretive Merge Audit**: 84 active Out-of-Sample (OS) alphas from `portfolio_os.json` and cached PnL in `~/.cache/alphafind/portfolio_pnl.json` serve as the portfolio baseline. Pearson correlation is computed over common trading days ($N \ge 30$). Accretive merge metrics enforce $\Delta \text{Sharpe} > 0$, $\Delta \text{Vol} \le 0$, and $\rho_{\max} < 70\%$, with safety buffer classification (`StrongGo`, `ConditionalGo`, `NoGo`).
5. **Environment & Testing Verification**: Live credentials (`MAIN_ACCOUNT_EMAIL`, `MAIN_ACCOUNT_PASSWORD`) are present in `.env`. A complete offline mock testing harness using `wiremock` (`tests/mock_api_tests.rs`) verifies all simulation, check, and submission pathways with 100% test pass rate.

---

## 1. Simulation Dispatch Architecture

### 1.1 API Endpoints & Transport Layer
The WorldQuant BRAIN API client is implemented in `src/client.rs` (`BrainClient`), with base URL `https://api.worldquantbrain.com`:

| Operation | HTTP Method | Endpoint | Request Payload / Params | Return Type / Key Headers |
|---|---|---|---|---|
| **Authentication** | `POST` | `/authentication` | Basic Auth header: `Basic base64(email:password)` | Sets session cookie (`Set-Cookie`) |
| **Simulation Dispatch** | `POST` | `/simulations` | JSON payload: `SimulationPayload` | `Location` header or JSON `{"id": "..."}` |
| **Simulation Polling** | `GET` | `/simulations/{id}` | None (Cookie / Auth header) | `SimulationResponse` (status, progress, alpha ID) |
| **Alpha Details (IS)** | `GET` | `/alphas/{id}` | None | `AlphaDetails` (IS metrics: Sharpe, Fitness, etc.) |
| **Submission Checks** | `GET` | `/alphas/{id}/check` | None | `InSampleStats` with `checks` array and `selfCorrelated` |
| **Daily PnL Records** | `GET` | `/alphas/{id}/recordsets/daily-pnl` | None | `DailyPnlResponse` (array of `[date, pnl]`) |
| **Out-of-Sample Submit** | `POST` | `/alphas/{id}/submit` | Empty JSON `{}` | `SubmitStatus` (`Accepted`, `Rejected`, `Unknown`) |
| **Field Metadata** | `GET` | `/data-fields` | Query: `instrumentType`, `region`, `universe`, `delay`, `id` | `DataFieldListResponse` |
| **Dataset Census** | `GET` | `/data-sets` | Query: `region`, `limit`, `offset` | `DatasetListResponse` |

### 1.2 Authentication & Session Management
- In Rust (`src/client.rs:276-400`):
  - Client initializes with `reqwest::ClientBuilder` configured with `cookie_store(true)`, `gzip(true)`, and a 60-second timeout.
  - `authenticate()` sends a `POST /authentication` with `Authorization: Basic <base64>`.
  - Retries up to 4 times with exponential backoff on `HTTP 429` (parsing `Retry-After` header) and transient connection failures.
  - Sets an atomic flag `authenticated = true` to avoid redundant auth handshakes.
- In Python (`scratch/engine.py:28-36`, `scratch/screen_and_evaluate.py:31-38`):
  - Base64 encodes credentials and extracts cookies from `Set-Cookie` response header.
  - Attaches `Cookie` header to all subsequent HTTP requests via `urllib.request`.

### 1.3 Simulation Payload & Alpha Settings Schema
Simulation requests use `SimulationPayload` (`src/models.rs:88-93`):
```rust
pub struct SimulationPayload<'a> {
    #[serde(rename = "type")]
    pub sim_type: &'a str,      // Always "REGULAR"
    pub settings: &'a AlphaSettings,
    pub regular: &'a str,       // FastExpr expression string
}
```

The `AlphaSettings` struct (`src/models.rs:4-31`) standardizes BRAIN simulation parameters:
```rust
pub struct AlphaSettings {
    pub instrument_type: String, // Default "EQUITY"
    pub region: String,          // Default "USA"
    pub universe: String,        // Default "TOP1000" (or "TOP200", "MINVOL1000")
    pub delay: i32,              // Default 1 (or 0)
    pub decay: i32,              // Default 5 (range: 4 - 25)
    pub neutralization: String,  // Default "SUBINDUSTRY" (or "INDUSTRY", "SECTOR")
    pub truncation: f64,         // Default 0.05 (or 0.055 - 0.065)
    pub pasteurization: String,  // Default "ON"
    pub unit_handling: String,   // Default "VERIFY"
    pub nan_handling: String,    // Default "OFF"
    pub language: String,        // Default "FASTEXPR"
    pub visualization: bool,     // Default false
}
```

### 1.4 SSRF Protection & Safe URL Handling
In `src/client.rs:500-514`, `poll_simulation` guards against Server-Side Request Forgery (SSRF) when following URL redirects:
- If a full URL is returned in the `Location` header, `reqwest::Url::parse` extracts the scheme and host.
- The scheme and host MUST strictly match the configured `base_url` (`https://api.worldquantbrain.com`).
- If an attacker-controlled or redirected external host is detected, the request immediately errors out with `"Security violation: Rejected follow-up URL outside configured BRAIN host"`.
- If an alphanumeric ID is passed instead, it is validated using `id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')`.

### 1.5 Non-Idempotent Request Handling & Retry Policy
Simulations and submissions are non-idempotent operations:
- **Transport Failure Classification** (`src/client.rs:40-66`):
  - `PostFailure::NotSent`: Connection could not be established (`reqwest::Error::is_connect()`). Safe to retry.
  - `PostFailure::Ambiguous`: Timeout, TCP reset, or truncated response after bytes were sent. The platform may have received and processed the request. Retrying is unsafe because it could duplicate simulations or trigger unwanted submission actions.
- **Fingerprinting** (`src/client.rs:68-94`):
  - A 64-bit FNV-1a hash is calculated over the expression and alpha settings (`simulation_fingerprint(expression, settings)`).
  - Ambiguous errors log the fingerprint instead of the private research formula: `AMBIGUOUS_SIMULATION_NOTE` informs the researcher to reconcile in the BRAIN interface before resubmitting.

---

## 2. Concurrency & Rate Limiting

### 2.1 Concurrency Limits & Strict Policy Enforcement
`AGENTS.md` mandates:
> **"When searching for or screening new candidate alphas, always run 3 concurrent simulation workers (`--workers 3`, matching `DEFAULT_WORKERS_PER_ACCOUNT = 3` in `src/config.rs`) on the primary account within a single CLI process."**
> **"Because the CLI has no cross-process quota coordination... do not run multiple concurrent CLI screening processes at the same time or increase concurrency above 3."**

### 2.2 Rust Multi-Account Screener Implementation
Implemented in `src/screener.rs:13-89, 123-385`:
1. **Account Worker Representation**:
   ```rust
   pub struct AccountWorker {
       pub client: BrainClient,
       pub role: String,
       pub is_main: bool,
       pub semaphore: Arc<tokio::sync::Semaphore>, // Size: concurrency (default 3)
   }
   ```
2. **Bounded Work Queue**:
   - `tokio::sync::mpsc::channel::<CandidateAlpha>(candidates.len())` creates a bounded queue.
   - Candidates are sent to the channel, and the sender is closed so workers terminate naturally upon queue exhaustion.
3. **Semaphore Permit Acquisition**:
   - Each worker loop acquires a permit from `semaphore.acquire().await` before dispatching.
   - The permit is held for the entire simulation lifecycle (POST + polling loop), ensuring that active in-flight simulations on an account never exceed `workers_per_account = 3`.
4. **Polite Dispatch Sleep**:
   - `tokio::time::sleep(Duration::from_millis(500)).await` introduces a polite 500ms delay between consecutive simulation POSTs to prevent burst traffic.
5. **Cross-Account Transfer Mutex**:
   - Secondary worker accounts scout candidates. When an alpha qualifies, it is re-simulated on the `MAIN` account.
   - `transfer_lock: Arc<Mutex<()>>` serializes all promotions and official `/check` requests on the MAIN account, preventing concurrency spikes on the primary account.

### 2.3 Python Autonomous Engine Implementation
In `scratch/engine.py:335-370` and `scratch/screen_and_evaluate.py:330-380`:
- Uses `concurrent.futures.ThreadPoolExecutor(max_workers=3)`.
- Implements `self.submit_lock = threading.Lock()` to serialize candidate verification and submission gates so that pre-flight checks and official submissions occur sequentially, one candidate at a time.

---

## 3. In-Sample Metrics & Check Evaluator

### 3.1 Metrics Parsing
When an alpha simulation completes, `client.get_alpha_details(alpha_id)` fetches GET `/alphas/{id}`. The `is` field maps to `InSampleStats` (`src/models.rs:125-140`):
- `sharpe`: Annualized Sharpe ratio (`f64`).
- `fitness`: BRAIN fitness score (`f64`).
- `returns`: Annualized return (`f64`, e.g., 0.18 = 18.0%).
- `turnover`: Daily portfolio turnover (`f64`, e.g., 0.25 = 25.0%).
- `margin`: Average margin in decimal form (multiplied by 10,000 for basis points).
- `drawdown`: Maximum in-sample drawdown (`f64`).
- `checks`: List of `CheckItem { name, result, value, limit }`.
- `self_correlated`: Container with `max` correlation and partner alpha records.

### 3.2 In-Sample Thresholds
Thresholds are categorized in `src/config.rs`:

| Metric | Delay 0 Threshold | Delay 1 Threshold | Category in Code | Reference |
|---|---|---|---|---|
| **Sharpe Ratio** | $\ge 2.00$ | $\ge 1.25$ | `platform_rules::min_sharpe_for_delay` | BRAIN Learn rules |
| **Fitness Score** | $\ge 1.30$ | $\ge 1.00$ | `platform_rules::min_fitness_for_delay` | BRAIN Learn rules |
| **Turnover Band** | $1.0\% < \text{Turnover} < 70.0\%$ | $1.0\% < \text{Turnover} < 70.0\%$ | `platform_rules::turnover_within_platform_bounds` | BRAIN Learn rules |
| **Annual Return** | $> 8.0\%$ | $> 8.0\%$ | User requirement (`ORIGINAL_REQUEST.md`) | Local research filter |
| **Self-Correlation** | $< 0.70$ | $< 0.70$ | `platform_rules::BRAIN_MAX_CORRELATION` | BRAIN Learn rules |
| **Orthogonality** | $\le 0.15$ | $\le 0.15$ | `research_hypotheses::ORTHOGONAL_CORRELATION_THRESHOLD` | Local hypothesis preference |

### 3.3 Fail-Closed Submission Check Evaluator
Implemented in `src/client.rs:96-265` (`evaluate_submission_checks`):

1. **Mandatory Core Checks**:
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
2. **Fail-Closed Rules**:
   - **Empty Response**: If `stats.checks.is_empty()`, fails immediately: `"FAIL-CLOSED: Empty check results received from BRAIN API"`.
   - **Missing Core Checks**: If any of the 6 core checks are missing from the returned checks list, fails immediately: `"FAIL-CLOSED: Missing required core checks: <missing_list>"`.
   - **Pending Calculation**: If any check has status `PENDING`, reports pending state and prompts retry/polling.
   - **Failure Status**: If any check is not `PASS` (e.g. `FAIL`, `ERROR`, or unknown string), fails immediately.
   - **Already Submitted State**: If the platform returns `ALREADY_SUBMITTED`, reports that checks cannot be re-evaluated.
3. **Pre-flight Check Bypass Security**:
   - In `src/commands/submission.rs:78-98, 255-274`, `--skip-checks` is rejected by default.
   - It only proceeds if `ALPHAFIND_ALLOW_SKIP_CHECKS=1` or `true` is explicitly set in the process environment.
   - When bypassed, a prominent warning is printed and documented.

---

## 4. Portfolio Baseline & Correlation Audit

### 4.1 Active Portfolio Loading & Baseline Management
The active Out-of-Sample portfolio baseline is managed via two components:
1. `portfolio_os.json`: Contains structured records of all active OS alphas (currently 84 active alphas). Loaded via `load_portfolio_os()` (`src/commands/mod.rs`).
2. PnL Cache: Daily PnL time series are stored in `~/.cache/alphafind/portfolio_pnl.json` (or `.alphafind/portfolio_pnl.json`).
   - `load_or_refresh_os_pnl_cache()` checks cache age against `PNL_CACHE_MAX_AGE_HOURS = 24`.
   - If missing or stale (or if `--refresh` is passed), missing PnLs are queried from GET `/alphas/{id}/recordsets/daily-pnl`.

### 4.2 Pearson Correlation Calculation
Implemented in `src/correlation.rs:11-62` (`pearson_correlation`):
- Overlap Extraction:
  - Takes two dictionaries `HashMap<String, f64>` ($x$ and $y$).
  - Finds intersection of dates where both values are finite: `common_dates = x_keys ∩ y_keys`.
  - Let $N = |\text{common\_dates}|$.
- Fail-Closed Overlap Threshold:
  - If $N < \text{MIN\_CORRELATION\_DAYS}$ (`30` trading days), returns `(None, N)`. Insufficient overlap is never treated as zero correlation.
- Variance & Covariance:
  $$\mu_x = \frac{1}{N}\sum x_i, \quad \mu_y = \frac{1}{N}\sum y_i$$
  $$\text{cov}(x,y) = \sum (x_i - \mu_x)(y_i - \mu_y)$$
  $$\text{var}_x = \sum (x_i - \mu_x)^2, \quad \text{var}_y = \sum (y_i - \mu_y)^2$$
- Zero-Variance Guard:
  - Checks $\text{var}_x > \epsilon_{\text{var}}$ ($10^{-12}$) and $\text{var}_y > \epsilon_{\text{var}}$.
  - If either series is constant (e.g. flatline PnL), returns `(None, N)`.
- Output:
  $$\rho = \frac{\text{cov}(x,y)}{\sqrt{\text{var}_x \cdot \text{var}_y}}, \quad \rho \in [-1.0, 1.0]$$

### 4.3 Accretive Merge Simulation & Evaluation
Implemented in `src/correlation.rs:153-316` and `src/allocation.rs:455-630`:

1. **Multi-Alpha Equal-Weighted Portfolio Simulation** (`simulate_portfolio`):
   - Finds common dates across all included alphas ($N \ge \text{MIN\_PORTFOLIO\_DAYS} = 100$).
   - Computes daily portfolio return: $R_{\text{port}, t} = \frac{1}{M}\sum_{m=1}^M R_{m, t}$.
   - Annualizes metrics (using $T = 252$ trading days):
     $$\text{Annualized PnL} = \mu_{\text{daily}} \times 252$$
     $$\text{Annualized Volatility} = \sigma_{\text{daily}} \times \sqrt{252}$$
     $$\text{Merged Sharpe} = \frac{\text{Annualized PnL}}{\text{Annualized Volatility}}$$
   - Average Pairwise Correlation: $\bar{\rho} = \frac{2}{M(M-1)}\sum_{i < j} \rho_{ij}$.

2. **Portfolio Merge Delta Metrics** (`calculate_portfolio_impact`):
   - Baseline: Simulates existing $M$ portfolio alphas $\to (\text{Sharpe}_{\text{base}}, \text{Vol}_{\text{base}}, \text{PnL}_{\text{base}}, \bar{\rho}_{\text{base}})$.
   - Combined: Simulates $M + 1$ alphas $\to (\text{Sharpe}_{\text{new}}, \text{Vol}_{\text{new}}, \text{PnL}_{\text{new}}, \bar{\rho}_{\text{new}})$.
   - Evaluates:
     $$\Delta \text{Sharpe} = \text{Sharpe}_{\text{new}} - \text{Sharpe}_{\text{base}} \quad (> 0 \text{ required})$$
     $$\Delta \text{Vol} = \text{Vol}_{\text{new}} - \text{Vol}_{\text{base}} \quad (\le 0 \text{ indicates variance collapse})$$
     $$\Delta \text{PnL} = \text{PnL}_{\text{new}} - \text{PnL}_{\text{base}}$$
     $$\Delta \bar{\rho} = \bar{\rho}_{\text{new}} - \bar{\rho}_{\text{base}}$$
     $$\text{Safety Buffer} = (0.70 - \rho_{\max}) \times 100.0\%$$

3. **Admission Decision Engine** (`evaluate_candidate_admission`):
   - `StrongGo`: $\Delta \text{Sharpe} > 0$, $\Delta \text{Vol} < 0$, addresses turnover bucket deficit, and $\text{Safety Buffer} \ge 8.0\%$.
   - `ConditionalGo`: $\Delta \text{Sharpe} > 0$ and $\text{Safety Buffer} \ge 8.0\%$.
   - `NoGo`:
     - $\rho_{\max} \ge 0.70$ (Self-correlation collision).
     - $\text{Safety Buffer} < 5.0\%$ (Excessive risk of OOS collision).
     - $\Delta \text{Sharpe} \le 0$ (Dilutive to overall portfolio performance).
     - Incomplete/insufficient overlap ($N < 30$ pairs) $\to$ Fails closed.

---

## 5. Environment Verification & Mock Testing

### 5.1 Environment Configuration
- `.env` file exists at `/Users/phamtoan/Developer/alphafind/.env`.
- Configured keys:
  - `MAIN_ACCOUNT_EMAIL`
  - `MAIN_ACCOUNT_PASSWORD`
- These credentials allow authenticating directly with WorldQuant BRAIN for live simulations and check evaluations.

### 5.2 Offline Mock Testing Suite
The repository includes a comprehensive mock testing framework built with `wiremock`:
- Location: `tests/mock_api_tests.rs` (15 unit tests).
- Mock Server: Spin up an HTTP mock server on `127.0.0.1` via `wiremock::MockServer::start().await`.
- Verified Scenarios:
  - `test_mock_authenticate_success`: Verifies HTTP 200 auth response.
  - `test_mock_authenticate_unauthorized`: Verifies HTTP 401 handling.
  - `test_mock_submit_simulation_success`: Verifies POST `/simulations` and `Location` header handling.
  - `test_mock_submit_simulation_retries_after_rate_limit`: Verifies exponential backoff on HTTP 429.
  - `test_mock_poll_simulation_ssrf_protection`: Verifies rejection of URLs pointing outside the BRAIN base host.
  - `test_mock_check_submission_all_pass`: Verifies passing of all 8 checks.
  - `test_mock_check_submission_empty_fails_closed`: Verifies fail-closed behavior on empty checks.
  - `test_mock_check_submission_pending_retry_then_pass`: Verifies polling when checks are PENDING.
  - `test_ambiguous_simulation_failure_is_not_retried`: Verifies that ambiguous timeouts are not retried.
  - `test_retry_policy_only_allows_provably_unsent_requests`: Verifies transport error classification.
  - `test_simulation_fingerprint_is_stable_and_settings_sensitive`: Verifies FNV-1a fingerprinting.

All 15 mock API tests, 13 submission gate tests, 18 correlation tests, and 10 allocation tests execute and pass cleanly (56 total tests passed).

### 5.3 Offline Historical Data Artifacts
- `portfolio_os.json`: 84 active Out-of-Sample alphas with settings and IS stats.
- `~/.cache/alphafind/portfolio_pnl.json`: Daily PnL histories across common dates.
- `scratch/evaluated_candidates.json`: Contains 10 previously evaluated candidate alphas with full simulation results, check statuses, and portfolio merge impacts.
- `scratch/portfolio_comparison_report.json`: Detailed portfolio impact metrics comparing candidates against the 84-alpha baseline.

---

## 6. Structural Synthesis & Recommendations

### 6.1 Strengths
1. **End-to-End Fail-Closed Architecture**: From transport classification (`PostFailure`) to check evaluation (`evaluate_submission_checks`) and Pearson correlation (`pearson_correlation`), missing data, zero variance, and ambiguous responses always fail closed.
2. **Double-Gated Submission**: Submissions must pass both the platform pre-flight check gate and a separate local portfolio-policy gate (`audit_candidate`) ensuring no local self-correlation $> 70\%$ and accretive portfolio merge.
3. **Concurrency Compliance**: Both the Rust screener and Python engine strictly enforce `max_workers = 3` per account, avoiding platform quota violations.

### 6.2 Recommended Next Steps for Implementation Pipeline
1. **Dynamic Check Count Parsing**: Although `evaluate_submission_checks` verifies all 6 core checks, platform responses may return 7 (for `TOP200`) or 8 (for `TOP1000`) checks. Ensuring version-aware parsing accommodates minor schema updates without manual list maintenance.
2. **Accretive Merge Automation**: Integrate `calculate_portfolio_impact` directly into the hypothesis screening loop so non-accretive candidates ($\Delta \text{Sharpe} \le 0$) are pruned before reaching the official submission stage.
3. **Automated PnL Reconciliation**: Implement an automated reconciliation step that queries GET `/users/self/alphas` using the FNV-1a fingerprint whenever an ambiguous network error occurs during simulation dispatch.
