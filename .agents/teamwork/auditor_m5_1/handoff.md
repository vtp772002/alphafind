# Forensic Integrity Audit & Adversarial Review Report

**Work Product**: Quantitative Alpha Research Framework (Milestones M1–M5)  
**Profile**: General Project (`Integrity mode: development` per `ORIGINAL_REQUEST.md`)  
**Auditor**: `auditor_m5_1`  
**Date**: 2026-10-25  
**Verdict**: **`CLEAN`**

---

## 1. Observation

Direct empirical observations collected via independent tool execution across the repository:

### 1.1 Source Code & Implementation Authenticity
1. **No Dummy/Facade Implementations**:
   - `src/concurrency_lock.rs:86-104`: Real POSIX advisory file locking via `libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB)` with RAII drop releasing the lock (`libc::flock(fd, libc::LOCK_UN)`).
   - `src/schema_validator.rs:1-2170`: Full lexer and recursive descent AST parser evaluating a 6-type lattice (`Scalar`, `Matrix`, `Vector`, `Group`, `DensifiedGroup`, `BooleanMatrix`), enforcing vector folding (`vec_avg`, `vec_sum`), mandatory backfill for sparse fields, densified grouping, and non-negative lookbacks ($d \ge 1$).
   - `src/expression_builder.rs:1-1629`: Complete factor combinator pipeline implementing multi-layer signal synthesis, non-linear transforms (`signed_power`, `rank`, `winsorize`, `ts_decay_linear`), group neutralization with mandatory `densify()`, liquidity dampeners, cross-sanctuary combinators, and deterministic auto-repair.
   - `src/models.rs:414-490`: Fully typed schema models distinguishing `DataFieldType::Matrix` vs `DataFieldType::Vector`, with methods `is_vector()`, `requires_vector_folding()`, and `requires_backfill(threshold)`.
   - `src/client.rs:430-545`: Paginated `/data-fields` client with query parameters (`instrumentType`, `region`, `delay`, `universe`, `dataset.id`, `limit`, `offset`), HTTP 429 backoff parsing `Retry-After`, and fail-closed termination on HTTP 401/500 errors.
   - Zero occurrences of `todo!()`, `unimplemented!()`, or mock/dummy return stubs in production paths (`src/`).

2. **Test Suite Integrity**:
   - Zero occurrences of tautological assertions (`assert!(true)`, `assert_eq!(1, 1)`).
   - All tests in `tests/` execute genuine assertions verifying AST parsing errors, lock contention across 32 threads, numerical overflow protection in Pearson correlation, and fail-closed submission checks.

### 1.2 Independent Test & Tool Execution
1. **Workspace Test Suite (`cargo test -- --test-threads=1`)**:
   - Command: `cargo test -- --test-threads=1`
   - Output:
     ```
     unittests: 3 passed; 0 failed
     tests/allocation_tests.rs: 10 passed; 0 failed
     tests/autotuner_tests.rs: 5 passed; 0 failed
     tests/benchmark_tests.rs: 3 passed; 0 failed
     tests/correlation_tests.rs: 18 passed; 0 failed
     tests/crowd_tests.rs: 7 passed; 0 failed
     tests/expression_builder_tests.rs: 20 passed; 0 failed
     tests/health_tests.rs: 5 passed; 0 failed
     tests/matrix_tests.rs: 6 passed; 0 failed
     tests/metadata_schema_tests.rs: 25 passed; 0 failed
     tests/mock_api_tests.rs: 15 passed; 0 failed
     tests/models_tests.rs: 17 passed; 0 failed
     tests/reporting_tests.rs: 5 passed; 0 failed
     tests/shadow_tests.rs: 4 passed; 0 failed
     tests/simulation_evaluation_tests.rs: 17 passed; 0 failed
     tests/stress_concurrency_tests.rs: 10 passed; 0 failed
     tests/stress_expression_builder_tests.rs: 10 passed; 0 failed
     tests/stress_metadata_tests.rs: 16 passed; 0 failed
     tests/stress_schema_validator_tests.rs: 15 passed; 0 failed
     tests/stress_simulation_evaluation_tests.rs: 15 passed; 0 failed
     tests/stress_taxonomy_tests.rs: 17 passed; 0 failed
     tests/submission_gate_tests.rs: 13 passed; 0 failed
     tests/taxonomy_tests.rs: 19 passed; 0 failed
     Total: 275 passed; 0 failed; 0 ignored; finished with exit code 0.
     ```

2. **Linter & Formatting**:
   - `cargo clippy --tests -- -D warnings`: Exit code 0, 0 warnings.
   - `cargo fmt --check`: Exit code 0, clean formatting.

3. **CLI End-to-End Subcommands**:
   - `cargo run -- fields --summary`: Certified 235 total alternative fields (185 Matrix, 50 Vector) across `model51` (16), `model16` (24), `news18` (121), and `option9` (74). Exit code 0.
   - `cargo run -- generate --dataset all -n 5 --validate`: Generated 5 distinct alternative data formulas across all pillars, all 5 passing schema validation. Exit code 0.
   - `cargo run -- screen --help`: Verified `--workers` defaults to 3, with `--alternative` and `--dataset` flags present. Exit code 0.

4. **Python Companion Tools**:
   - `python3 scratch/metadata_tool.py --dataset all --summary`: Verified 235 fields matching Rust catalog. Exit code 0.
   - `python3 scratch/expression_builder.py --dataset all --validate`: Generated 10 validated expressions. Exit code 0.
   - `python3 scratch/evaluate_alternative_alphas.py --offline`: Successfully executed offline simulation of 4 alternative hypotheses, qualifying all 4 candidates:
     - `US_M51_Beta_Term_Spread` (model51): Sharpe 1.34, Fitness 1.28, Turnover 14.5%, $\Delta\text{Sharpe} +0.0335$, $\Delta\text{Vol} -\$3,511.41$, Max $\rho = 5.19\%$.
     - `US_M16_Fundamental_Acceleration_Convexity` (model16): Sharpe 1.42, Fitness 1.36, Turnover 16.8%, $\Delta\text{Sharpe} +0.0430$, $\Delta\text{Vol} -\$3,209.27$, Max $\rho = 6.47\%$.
     - `US_N18_Vector_Folded_Story_Sentiment` (news18): Sharpe 1.31, Fitness 1.15, Turnover 23.5%, $\Delta\text{Sharpe} +0.0609$, $\Delta\text{Vol} -\$3,162.03$, Max $\rho = 7.72\%$.
     - `US_O9_Breakeven_Directional_Asymmetry_30d` (option9): Sharpe 1.48, Fitness 1.45, Turnover 21.2%, $\Delta\text{Sharpe} +0.0343$, $\Delta\text{Vol} -\$2,806.40$, Max $\rho = 7.87\%$.
     - Exit code 0.

### 1.3 Git Hygiene & Confidentiality (`AGENTS.md`)
1. **Pre-Push Sanitization Gatekeeper (`scripts/sanitize_check.sh`)**:
   - Executed `bash scripts/sanitize_check.sh`:
     - Commit-history scan: no Alpha IDs in added lines.
     - Dataset-field scan: all tracked files clear of configured field patterns.
     - Candidate artifact scan: 12 private literals absent from tracked files.
     - Exit code 0, PASSED.
   - Executed with strict mode: `ALPHAFIND_STRICT_HISTORY=1 bash scripts/sanitize_check.sh`:
     - Exit code 0, PASSED.
2. **Git Tracking Scan (`git ls-files`)**:
   - Verified that `.env`, `submittable_alphas.csv`, `portfolio_os.json`, `data/private_candidates.json`, `data/fields_*.json`, and `scratch/*` are completely excluded and untracked.
3. **Workspace Layout Compliance**:
   - Audited `.agents/teamwork/`: Contains exclusively Markdown metadata/documentation files (`.md`) and `.init`. No source files, compiled binaries, test suites, or proprietary data are present.

### 1.4 Policy & Concurrency Enforcement
1. **Worker Concurrency Limit**:
   - `src/config.rs:106`: `DEFAULT_WORKERS_PER_ACCOUNT = 3`.
   - `src/main.rs:102`: `workers` default value set to 3.
   - `scratch/evaluate_alternative_alphas.py:756`: `ThreadPoolExecutor(max_workers=3)`.
   - `src/concurrency_lock.rs`: Enforces advisory cross-process lock on `~/.cache/alphafind/sim_runner.lock` via POSIX `flock(2)`, preventing multiple CLI screening processes from running concurrently.

---

## 2. Logic Chain

1. **Premise 1 (Ground-Truth Integrity Mode)**:
   Per `ORIGINAL_REQUEST.md` line 8, the integrity mode is explicitly specified as `development`. Under Development mode, standard and third-party libraries (`reqwest`, `serde`, `libc`, `tokio`, etc.) are permitted for auxiliary and networking infrastructure, but hardcoded test outputs, dummy implementations, and fabricated verification artifacts are strictly prohibited.
2. **Premise 2 (Genuine Implementation)**:
   Direct AST inspections and code audits of `schema_validator.rs`, `expression_builder.rs`, `concurrency_lock.rs`, and `evaluate_alternative_alphas.py` confirm full, genuine algorithmic logic. No dummy methods, `todo!()`, or facade shortcuts were found in production paths (Observation 1.1).
3. **Premise 3 (Test Authenticity & Execution)**:
   Independent execution of `cargo test -- --test-threads=1` produced 275 passed tests and 0 failures. Test inspection revealed zero tautologies (`assert!(true)`). Clippy and format checks passed cleanly (Observation 1.2).
4. **Premise 4 (Confidentiality & Git Hygiene)**:
   The sanitization script `sanitize_check.sh` was independently executed under both standard and strict modes (`ALPHAFIND_STRICT_HISTORY=1`), passing with 0 errors. No private candidate formulas, real Alpha IDs, or credentials exist in tracked files or commit additions (Observation 1.3).
5. **Premise 5 (Policy Compliance)**:
   The max 3 workers per account policy specified in `AGENTS.md` is strictly enforced via `DEFAULT_WORKERS_PER_ACCOUNT = 3`, thread pool configuration, and POSIX `flock` cross-process advisory locking (Observation 1.4).
6. **Premise 6 (Layout Compliance)**:
   Auditing `.agents/teamwork/` verified that only Markdown metadata files exist within agent directories, satisfying repository workspace rules (Observation 1.3).
7. **Deductive Conclusion**:
   Because every check required by the Integrity Forensics protocol passes empirically without exception, the work product satisfies all requirements and contains zero integrity violations.

---

## 3. Caveats

1. **Live BRAIN Network Simulation**:
   Direct live API simulations against the WorldQuant BRAIN backend require active account credentials (`.env`). In this audit environment, tests and evaluations operated in deterministic offline mock mode as designed, which is standard for CI and reproducible verification.
2. **Advisory Flock Across Network Mounts**:
   POSIX `flock` provides robust cross-process mutual exclusion on local filesystems (macOS / Linux APFS / ext4), but may require NFS-specific lock daemons if user caches are mounted over networked filesystems. For local multi-process execution, it is completely secure.

---

## 4. Conclusion & Verdict

The quantitative alpha research framework in `alphafind` is a genuine, high-quality, fully implemented quantitative alpha research engine adhering strictly to all project constraints and platform policies.

**Verdict**: **`CLEAN`**

---

## 5. Verification Method

To independently reproduce this forensic audit:

```bash
# 1. Verify workspace test suite (275 tests)
cargo test -- --test-threads=1

# 2. Verify static analysis and formatting
cargo clippy --tests -- -D warnings
cargo fmt --check

# 3. Verify CLI subcommands
cargo run -- fields --summary
cargo run -- generate --dataset all -n 5 --validate
cargo run -- screen --help

# 4. Verify Python companion tools
python3 scratch/metadata_tool.py --dataset all --summary
python3 scratch/expression_builder.py --dataset all --validate
python3 scratch/evaluate_alternative_alphas.py --offline

# 5. Verify Git hygiene and secret sanitization
ALPHAFIND_STRICT_HISTORY=1 bash scripts/sanitize_check.sh
```

Invalidation Conditions:
- Any test failure in `cargo test`.
- Any exit code != 0 from `scripts/sanitize_check.sh`.
- Discovery of any un-densified group neutralization or unfolded vector in generated FastExpr expressions.
- Discovery of any real Alpha IDs or private candidate formulas in tracked Git files.
