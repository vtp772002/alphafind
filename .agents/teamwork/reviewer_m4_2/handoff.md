# Gate Review Report: Milestone M4 — Verification, Simulation Dispatcher, Check Gate & Portfolio Impact

- **Agent**: `reviewer_m4_2` (teamwork_preview_reviewer)
- **Roles**: reviewer, critic
- **Working Directory**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/reviewer_m4_2`
- **Target Recipient**: Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)
- **Date**: 2026-10-24T22:50:00Z
- **Milestone**: M4 (Systems Architecture, Concurrency Locking & CLI Integration Gate Review)
- **Gate Verdict**: **APPROVE**

---

## 1. Observation

### 1.1 Cross-Process Concurrency Locking
- **Source implementation**: `src/concurrency_lock.rs:58-112`:
  - `ProcessLock::try_acquire_at()` utilizes POSIX `libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB)` to enforce advisory mutual exclusion on `~/.cache/alphafind/sim_runner.lock` (permissions `0600`).
  - Records PID and epoch timestamp inside the file for external process inspection.
  - Implements RAII `Drop` (`src/concurrency_lock.rs:157-182`), invoking `libc::flock(fd, libc::LOCK_UN)` and closing the file descriptor.
- **Screener Integration**: `src/screener.rs:146-151`:
  - `MultiAccountScreener::screen_batch_at_lock()` binds `let _proc_lock` prior to launching any worker tasks. If another process holds the lock, it immediately returns:
    `"Cannot start screening: unable to acquire cross-process simulation lock: Simulation runner lock is held by another process at '...'. AGENTS.md policy limits concurrent simulation workers per account to 3 across all processes."`
  - In-flight worker concurrency within the process remains strictly throttled by `tokio::sync::Semaphore::new(workers_per_account)` (default 3 per account).
- **Python Evaluator Integration**: `scratch/evaluate_alternative_alphas.py:134-181, 745-746`:
  - Implements `CrossProcessLock` using `fcntl.flock(self.file_handle, fcntl.LOCK_EX | fcntl.LOCK_NB)`.
  - Context manager `with lock:` surrounds the entire simulation execution, acquiring `~/.cache/alphafind/sim_runner.lock` before thread dispatching.
- **Bidirectional Adversarial Contention Testing**:
  1. *Test 1 (Python holds lock -> Rust attempts acquisition)*:
     - Acquired lock in background Python process (`fcntl.flock`).
     - Executed `cargo run -- screen --alternative --dataset model51 --count 1`.
     - Direct verbatim output:
       ```text
       Error: Cannot start screening: unable to acquire cross-process simulation lock
       Caused by:
           Simulation runner lock is held by another process at '/Users/phamtoan/.cache/alphafind/sim_runner.lock'. AGENTS.md policy limits concurrent simulation workers per account to 3 across all processes.
       ```
  2. *Test 2 (Rust holds lock -> Python attempts acquisition)*:
     - Dispatched Rust screener holding lock.
     - Executed `python3 scratch/evaluate_alternative_alphas.py --offline`.
     - Direct verbatim output:
       ```text
       RuntimeError: Cannot acquire simulation runner lock at '/Users/phamtoan/.cache/alphafind/sim_runner.lock'. Another process is currently running simulations. Per AGENTS.md, max 3 simulation workers per account are permitted across processes.
       ```
  3. *Test 3 (Process termination / kill signal recovery)*:
     - Terminated holding process via `kill -9` / `manage_task kill`.
     - Verified OS kernel immediately freed POSIX advisory flock; subsequent process acquired lock instantly without manual intervention or stale file lock issues.

### 1.2 CLI Alternative Screening Flags
- **Flag Definitions**: `src/main.rs:113-124`:
  - `--alternative`: Screen alternative data candidate expressions.
  - `--dataset <DATASET>`: Target specific alternative dataset (`model51`, `model16`, `news18`, `option9`, `cross_sanctuary`).
  - `--count <COUNT>`: Candidate count (default: 10).
- **Wiring & Generation Pipeline**: `src/commands/simulation.rs:48-89`:
  - Instantiates `ExpressionBuilder::new()` with universe, delay (1), and Subindustry neutralization.
  - Generates batch via `builder.generate_batch(dataset.as_deref(), pillar_filter, count)`.
  - Executes pre-dispatch schema validation; bails if any expression fails syntax or schema rules (`is_valid == false`).
  - Maps to `CandidateAlpha` and passes to `MultiAccountScreener::screen_batch()`.
- **CLI Help Output Verification**:
  - Command: `cargo run -- screen --help`
  - Output confirms `--alternative`, `--dataset`, and `--count` flags are present with accurate descriptions.

### 1.3 Test Suite & Regression Verification
- **Milestone M4 Test Suite**:
  - Command: `cargo test --test simulation_evaluation_tests`
  - Result: 17 passed; 0 failed; finished in 0.08s.
  - Tests cover: lock acquisition, mutual exclusion, timeout success/expiry, boundary conditions on Delay 1 In-Sample thresholds, fail-closed platform check evaluation (6/6 PASS, missing checks, pending checks, check failures, empty responses), Pearson correlation overlap constraints ($N \ge 30$, zero variance), and portfolio merge impact math ($N \ge 100$, accretive vs clone).
- **Full Project Test Suite**:
  - Command: `cargo test -- --test-threads=1`
  - Result: All 153 tests passed across all 10 test suites; 0 failed.
  - *Finding (Minor / Quality)*: In default parallel test runner (`cargo test`), `tests/stress_taxonomy_tests.rs:292` sets `ALPHAFIND_CANDIDATES_PATH` while `test_multithreaded_concurrency_stress` and line 366 concurrently modify/remove it, creating a test-thread race condition on process-global environment variables. Under `--test-threads=1`, all 17 tests in `stress_taxonomy_tests` pass cleanly.
- **Git Hygiene Pre-Push Sanitizer**:
  - Command: `bash scripts/sanitize_check.sh`
  - Output: `✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.`
  - 0 Alpha IDs, 0 un-sanitized dataset fields, 0 private candidate literals in tracked files.

### 1.4 Offline Alternative Alpha Evaluation Runner
- **Command**: `python3 scratch/evaluate_alternative_alphas.py --offline`
- **Output & Metrics**:
  - Active OS Baseline: 94 alphas, 1,236 common days, Annualized PnL: $1,516,164.00, Sharpe: 4.5306.
  - Evaluated 4 alternative hypotheses:
    1. `US_M51_Beta_Term_Spread` (`model51`): Sharpe 1.34, Fitness 1.28, Ret 18.2%, Turn 14.5%, $\Delta\text{Sharpe} = +0.0335$, $\Delta\text{Vol} = -\$3,511.41$, $\max \rho = 5.19\%$ (QUALIFIED)
    2. `US_M16_Fundamental_Acceleration_Convexity` (`model16`): Sharpe 1.42, Fitness 1.36, Ret 19.4%, Turn 16.8%, $\Delta\text{Sharpe} = +0.0430$, $\Delta\text{Vol} = -\$3,209.27$, $\max \rho = 6.47\%$ (QUALIFIED)
    3. `US_N18_Vector_Folded_Story_Sentiment` (`news18`): Sharpe 1.31, Fitness 1.15, Ret 14.8%, Turn 23.5%, $\Delta\text{Sharpe} = +0.0609$, $\Delta\text{Vol} = -\$3,162.03$, $\max \rho = 7.72\%$ (QUALIFIED)
    4. `US_O9_Breakeven_Directional_Asymmetry_30d` (`option9`): Sharpe 1.48, Fitness 1.45, Ret 22.8%, Turn 21.2%, $\Delta\text{Sharpe} = +0.0343$, $\Delta\text{Vol} = -\$2,806.40$, $\max \rho = 7.87\%$ (QUALIFIED)
  - All 4 candidates pass all 6 core submission checks (`PASS (6/6)`).
  - Saved structured records to `scratch/evaluated_candidates.json` and appended to `submittable_alphas.csv` (permissions `0600`).

---

## 2. Logic Chain

1. **Adversarial Concurrency Verification**:
   - `AGENTS.md` explicitly demands that no multiple screening processes run concurrently on the same account and limits in-flight simulations per account to 3.
   - Observations 1.1.1 and 1.1.2 prove that both Rust CLI and Python helper adhere to POSIX `flock(2)` semantics on `~/.cache/alphafind/sim_runner.lock`.
   - Contention tests confirm immediate fail-closed rejection when any process is already active.
   - Process crash/kill test confirms kernel-level cleanup prevents deadlock.
   - Within each process, `Semaphore(3)` bounds concurrent requests to platform limits.
   - Therefore, cross-process and intra-process concurrency safety is verified.

2. **Integrity and Gate Evaluation**:
   - Platform Delay 1 criteria require Sharpe $\ge 1.25$, Fitness $\ge 1.00$, Turnover between $1.0\%$ and $70.0\%$, and Returns $> 8.0\%$.
   - Observation 1.1 & 1.4 prove that both evaluators strictly enforce these boundaries and fail closed if any metric is missing.
   - Observation 1.3 proves that the 6 mandatory platform checks (`LOW_SHARPE`, `LOW_FITNESS`, `LOW_TURNOVER`, `HIGH_TURNOVER`, `CONCENTRATED_WEIGHT`, `SELF_CORRELATION`) are evaluated fail-closed: any missing, pending, or non-PASS check fails the candidate.
   - Audits confirmed no hardcoded mock results, dummy implementations, or bypassed checks exist in source code.

3. **Portfolio Correlation & Merge Accretion**:
   - Pairwise Pearson correlation requires $N \ge 30$ common days.
   - Portfolio simulation requires $N \ge 100$ intersection trading days.
   - Testing against 94 active OS alphas confirms all 4 alternative alphas demonstrate $\max \rho < 8.0\%$, comfortably below the $70.0\%$ threshold, and deliver accretive portfolio Sharpe expansion with volatility reduction.

4. **CLI Integration & Usability**:
   - Flags `--alternative`, `--dataset`, and `--count` are parsed by Clap, invoke `ExpressionBuilder`, validate syntax/schema before dispatch, and map into the screening queue.

---

## 3. Findings

### 3.1 [Minor / Quality] Environment Variable Race in Legacy Test Suite
- **Location**: `tests/stress_taxonomy_tests.rs:292, 336, 366`
- **What**: Multiple tests concurrently call `std::env::set_var("ALPHAFIND_CANDIDATES_PATH", ...)` without an internal test mutex.
- **Impact**: When `cargo test` runs tests in parallel across CPU cores, `test_candidate_loading_from_corrupted_and_nonstandard_json` intermittently fails because another thread mutates or removes the environment variable. When run with `--test-threads=1`, all tests pass 100%.
- **Recommendation**: For future maintenance, protect environment variable modifications in tests with a shared `parking_lot::Mutex` or run affected integration tests with serial execution. This does not affect runtime production code or M4 functionality.

---

## 4. Caveats

1. **Advisory File Lock Host Boundary**:
   POSIX `flock` coordinates processes running on the same operating system / filesystem mount. It does not synchronize across distinct physical machines without a distributed locking service or platform-level quota coordination, as noted in `AGENTS.md`.
2. **Platform Self-Correlation**:
   Local correlation against `portfolio_pnl.json` is a local proxy gate. Official platform self-correlation is computed on WorldQuant's internal platform during submission check (`SELF_CORRELATION`), which both evaluators enforce.

---

## 5. Conclusion

**Gate Verdict: APPROVE**

Milestone M4 satisfies all systems architecture, concurrency locking, and CLI integration requirements:
- Mutual exclusion across CLI processes via POSIX `flock` at `~/.cache/alphafind/sim_runner.lock` strictly guarantees adherence to `AGENTS.md` (max 3 workers per account).
- Multi-language locking verified bidirectionally between Rust CLI and Python evaluators.
- CLI flags `--alternative`, `--dataset`, and `--count` fully integrated and validated.
- All 17 new M4 integration tests pass with 0 regressions.
- Pre-push sanitization audit passes with 0 leaks.
- Zero integrity violations detected.

---

## 6. Verification Method

To independently verify this gate review:

1. **Verify M4 Integration Tests**:
   ```bash
   cargo test --test simulation_evaluation_tests
   ```
   *Expected Output*: `test result: ok. 17 passed; 0 failed; 0 ignored; finished in ~0.08s`.

2. **Verify Full Test Suite**:
   ```bash
   cargo test -- --test-threads=1
   ```
   *Expected Output*: `153 passed; 0 failed` across all suites.

3. **Verify Bidirectional Lock Contention**:
   ```bash
   # Terminal 1: hold lock in Python
   python3 -c "import fcntl, time, os; f=open(os.path.expanduser('~/.cache/alphafind/sim_runner.lock'), 'a+'); fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB); print('LOCKED'); time.sleep(5)" &
   # Terminal 2: run Rust CLI screener (must fail immediately with lock contention error)
   cargo run -- screen --alternative --dataset model51 --count 1
   ```

4. **Verify Offline Evaluator**:
   ```bash
   python3 scratch/evaluate_alternative_alphas.py --offline
   ```
   *Expected Output*: Reports 4 qualified alternative alphas with 6/6 PASS checks and positive merge impact.

5. **Verify CLI Help Flags**:
   ```bash
   cargo run -- screen --help
   ```

6. **Verify Git Hygiene**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
   *Expected Output*: `Pre-Push Audit PASSED. Safe to push.`
