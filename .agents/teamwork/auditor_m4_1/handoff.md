# Forensic Audit Report: Milestone M4 — Verification, Simulation Dispatcher, Check Gate & Portfolio Impact

- **Auditor**: `auditor_m4_1` (teamwork_preview_auditor)
- **Roles**: critic, specialist, auditor
- **Working Directory**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/auditor_m4_1`
- **Target Recipient**: Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)
- **Date**: 2026-10-24T22:48:00Z
- **Work Product**: Milestone M4 (`src/concurrency_lock.rs`, `scratch/evaluate_alternative_alphas.py`, `tests/simulation_evaluation_tests.rs`, `submittable_alphas.csv`, git hygiene)
- **Profile**: General Project (Integrity Forensics)
- **Verdict**: **CLEAN**

---

## 1. Observation

### 1.1 Concurrency Advisory File Lock (`src/concurrency_lock.rs`)
- File path: `/Users/phamtoan/Developer/alphafind/src/concurrency_lock.rs`
- In `try_acquire_at` (lines 77–99):
  ```rust
  let fd = file.as_raw_fd();
  let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
  if ret != 0 {
      let err = std::io::Error::last_os_error();
      let raw_os_err = err.raw_os_error().unwrap_or(0);
      if raw_os_err == libc::EWOULDBLOCK || raw_os_err == libc::EAGAIN {
          return Err(anyhow!(
              "Simulation runner lock is held by another process at '{}'. AGENTS.md policy limits concurrent simulation workers per account to 3 across all processes.",
              path_ref.display()
          ));
  ```
- File permissions set to `0o600` via `fs::set_permissions(path_ref, fs::Permissions::from_mode(0o600))` (line 79).
- Metadata recording: Writes current PID (`std::process::id()`) and Unix epoch timestamp to the lock file (lines 102–107).
- RAII unlock in `unlock_internal` and `Drop` (lines 157–182):
  ```rust
  let ret = unsafe { libc::flock(fd, libc::LOCK_UN) };
  ```
- Integration: `src/screener.rs:146-151` acquires `ProcessLock` before screening batches, ensuring cross-process mutual exclusion.
- Finding: **Genuine kernel-level advisory file locking (`flock`)**, NOT a mock facade.

### 1.2 Mathematical Rigor of Alternative Alpha Evaluator (`scratch/evaluate_alternative_alphas.py`)
- File path: `/Users/phamtoan/Developer/alphafind/scratch/evaluate_alternative_alphas.py`
- Mutual Exclusion: `CrossProcessLock` uses `fcntl.flock(self.file_handle, fcntl.LOCK_EX | fcntl.LOCK_NB)` at `~/.cache/alphafind/sim_runner.lock` (lines 134–181).
- Pearson Correlation (lines 522–545):
  - Genuinely calculates intersection of common valid dates $N \ge 30$.
  - Genuinely computes sample mean, covariance, sample variances, and standard deviation:
    ```python
    cov = sum((x[i] - mx) * (y[i] - my) for i in range(n))
    var_x = sum((x[i] - mx) ** 2 for i in range(n))
    var_y = sum((y[i] - my) ** 2 for i in range(n))
    denom = math.sqrt(var_x * var_y)
    if var_x > 1e-12 and var_y > 1e-12 and denom > 1e-12:
        corr = cov / denom
    ```
  - Fails closed on division-by-zero or zero variance.
- Multi-Alpha Portfolio Simulation & Merge Impact (lines 546–638):
  - Genuinely extracts intersection dates ($N \ge 100$).
  - Genuinely computes equal-weighted portfolio daily returns, mean, Bessel-corrected sample variance $(N-1)$, annualized return ($252 \times \mu$), annualized volatility ($\sqrt{252} \times \sigma$), and Sharpe ratio.
  - Genuinely computes $\Delta\text{Sharpe} = \text{Sharpe}_{\text{new}} - \text{Sharpe}_{\text{baseline}}$ and $\Delta\text{Vol} = \text{Vol}_{\text{new}} - \text{Vol}_{\text{baseline}}$.
  - Audits pairwise Pearson $\rho$ against all active OS baseline alphas, identifying $\max \rho$, $\text{avg } \rho$, and safety buffer.
- Offline Mode Alignment (lines 442–461):
  - Genuinely extracts the exact 1,236 common dates from `~/.cache/alphafind/portfolio_pnl.json`.
  - Generates synthetic Gaussian daily returns aligned to those dates and evaluates them against the 94 real active OS alphas in `portfolio_pnl.json`.
  - Finding: **Genuine mathematical calculations and dynamic date alignment**, NOT hardcoded outputs.

### 1.3 Test Suite Integrity (`tests/simulation_evaluation_tests.rs`)
- File path: `/Users/phamtoan/Developer/alphafind/tests/simulation_evaluation_tests.rs`
- 17 distinct integration tests covering:
  - `test_concurrency_lock_acquisition_and_release`: Validates flock file creation, PID/timestamp metadata, explicit release.
  - `test_concurrency_lock_contention_mutual_exclusion`: Validates that second flock attempt on held file fails closed with `EWOULDBLOCK`/`EAGAIN`, and succeeds once dropped.
  - `test_concurrency_lock_timeout_success`: Spawns a background worker thread holding the lock for 60ms; polling main thread successfully acquires upon release.
  - `test_concurrency_lock_timeout_expiry`: Verifies polling times out when lock remains held.
  - `test_in_sample_threshold_qualification_gate_boundaries`: Rigorously tests exact boundary conditions: Sharpe (1.25 vs 1.249), Fitness (1.00 vs 0.999), Turnover (strictly $>0.010$ and $<0.700$), Returns (strictly $>0.080$), and `None` fail-closed states.
  - `test_submission_check_gate_all_six_core_pass`: Verifies `6/6 PASS` qualification.
  - `test_submission_check_gate_missing_core_fails_closed`: Verifies missing `SELF_CORRELATION` fails closed.
  - `test_submission_check_gate_pending_fails_closed`: Verifies `PENDING` check fails closed.
  - `test_submission_check_gate_failed_check_fails_closed`: Verifies `FAIL` check fails closed.
  - `test_submission_check_gate_self_correlation_fail_formatting`: Verifies failed check reporting.
  - `test_submission_check_gate_empty_response_fails_closed`: Verifies empty check list fails closed.
  - `test_pearson_correlation_minimum_overlap_boundary`: Verifies $N=29$ fails closed (`None`) and $N=30$ succeeds.
  - `test_pearson_correlation_zero_variance_fails_closed`: Verifies constant series division-by-zero fails closed (`None`).
  - `test_simulate_portfolio_insufficient_trading_days`: Verifies $N=99$ fails closed and $N=100$ succeeds.
  - `test_portfolio_merge_delta_math_accretive_candidate`: Verifies positive Sharpe delta and low correlation on multi-alpha baseline.
  - `test_portfolio_merge_delta_math_high_correlation_candidate`: Verifies candidate cloned from baseline yields $\max \rho \ge 0.70$ and 0% safety buffer.
  - `test_audit_candidate_skips_self_and_fails_closed_when_empty`: Verifies candidate does not correlate against itself.
- Execution command: `cargo test --test simulation_evaluation_tests`
  - Result: `17 passed; 0 failed; 0 ignored; finished in 0.07s`.
- Codebase full test command: `cargo test`
  - Result: `153 passed; 0 failed; 0 ignored; finished in 0.08s`.
- Finding: **Non-tautological, rigorous assertions with authentic edge-case coverage**.

### 1.4 Git Hygiene, Candidate Confidentiality & Sanitization (`scripts/sanitize_check.sh`)
- Executed: `bash scripts/sanitize_check.sh`
- Result (exit code 0):
  ```
  🔍 [SANITIZATION GATEKEEPER] Running Pre-Push Audit...
     ✓ Commit-history scan: no Alpha IDs in added lines.
     ✓ Dataset-field scan: all tracked files are clear of configured field patterns.
     ✓ Candidate artifact scan: 12 private literals absent from tracked files.
  ✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.
  ```
- Checked `submittable_alphas.csv`:
  - Command: `git check-ignore -v submittable_alphas.csv`
    - Result: `.gitignore:45:submittable_alphas.csv submittable_alphas.csv` (properly gitignored).
  - Command: `ls -la submittable_alphas.csv`
    - Result: `-rw-------@ 1 phamtoan staff 133045 Oct 25 05:38 submittable_alphas.csv` (permissions `0600`).
- Checked `scratch/`:
  - Command: `git check-ignore -v scratch/evaluate_alternative_alphas.py`
    - Result: `.gitignore:58:scratch/ scratch/evaluate_alternative_alphas.py` (properly gitignored).
- Finding: **Zero confidential leaks in tracked files; clean Git hygiene**.

---

## 2. Logic Chain

1. **Advisory Concurrency Lock**:
   - `concurrency_lock.rs` directly invokes `libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB)` on Unix/macOS, returning `EWOULDBLOCK`/`EAGAIN` upon contention.
   - Independent adversarial Python stress-test confirmed mutual exclusion between distinct processes.
   - Therefore, the concurrency lock is authentic and compliant with `AGENTS.md`.

2. **Mathematical Correlation & Portfolio Impact**:
   - `scratch/evaluate_alternative_alphas.py` contains full mathematical derivations of Pearson correlation, sample variance with $(N-1)$ Bessel correction, equal-weighted multi-alpha portfolio aggregation, and portfolio Sharpe/volatility deltas.
   - Offline execution verified that PnL vectors are aligned with 1,236 real dates from `~/.cache/alphafind/portfolio_pnl.json` and computed against 94 active OS alphas.
   - Therefore, simulation metrics and portfolio merge evaluations are genuine and not fabricated.

3. **Assertion Rigor and Test Coverage**:
   - `tests/simulation_evaluation_tests.rs` tests actual code in `src/concurrency_lock.rs`, `src/client.rs`, and `src/correlation.rs`.
   - Assertions test strict inequality boundaries ($N=29$ vs $N=30$, $N=99$ vs $N=100$, Sharpe $1.25$ vs $1.249$, Turnover $1.0\%$ vs $1.001\%$), fail-closed missing check gates, and division-by-zero resilience.
   - All 17 tests pass with zero failures and zero warnings.

4. **Git Hygiene & Private Candidate Protection**:
   - `submittable_alphas.csv` and `scratch/` are excluded by `.gitignore`.
   - Permissions on `submittable_alphas.csv` are verified at `0600`.
   - `scripts/sanitize_check.sh` completed with 0 errors, confirming that no private Alpha IDs, real candidate formulas, or tracked field patterns exist in tracked git files.

---

## 3. Caveats

- `flock` provides OS-level advisory locking across all processes on the local machine. It does not synchronize across distributed physical machines, which is explicitly noted in `AGENTS.md` and `concurrency_lock.rs` documentation.
- Offline mode uses deterministic Gaussian synthetic PnLs aligned to real historical dates to allow hermetic local verification without incurring live BRAIN API rate limits or requiring active credentials.

---

## 4. Conclusion

Milestone M4 passes all forensic integrity checks without any violations, facades, or shortcuts:
- Authentic POSIX `flock` advisory locking implemented and verified.
- Genuine Pearson correlation and portfolio merge calculations implemented and verified.
- 17/17 M4 integration tests and 153/153 full codebase tests pass.
- Git hygiene check passes with 0 leaks; `submittable_alphas.csv` is gitignored with `0600` permissions.

**Verdict**: **CLEAN**

---

## 5. Verification Method

1. **Verify M4 Integration Tests**:
   ```bash
   cargo test --test simulation_evaluation_tests
   ```
2. **Verify Full Project Test Suite**:
   ```bash
   cargo test
   ```
3. **Execute Offline Alternative Alpha Evaluation**:
   ```bash
   python3 scratch/evaluate_alternative_alphas.py --offline
   ```
4. **Verify Git Hygiene Pre-Push Gatekeeper**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
5. **Verify File Permissions and Git Exclusion**:
   ```bash
   git check-ignore -v submittable_alphas.csv scratch/evaluate_alternative_alphas.py
   ls -la submittable_alphas.csv
   ```
