## 2026-10-24T20:27:01Z

You are the Quantitative Developer Worker for Milestone M2 Hardening (Iteration 3).
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m2_hardening/

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

You must first read:
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
- /Users/phamtoan/Developer/alphafind/PROJECT.md
- /Users/phamtoan/Developer/alphafind/AGENTS.md
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/challenger_m2_recheck/challenge_report.md
- /Users/phamtoan/Developer/alphafind/src/schema_validator.rs

Exclusive Write Ownership:
- src/schema_validator.rs
- tests/stress_schema_validator_tests.rs
- tests/metadata_schema_tests.rs

Your Tasks:
Fix the 3 fail-open vulnerabilities identified by Challenger in `src/schema_validator.rs`:
1. `ts_*` Lookback Strict Fail-Closed Enforcement:
   - For all time-series lookback/lag operators (`ts_delay`, `ts_delta`, `ts_decay_linear`, `ts_mean`, `ts_sum`, `ts_std_dev`, `ts_zscore`, `ts_rank`, `ts_backfill`, etc.), the window/lag argument MUST be verified as a statically positive integer ($d \ge 1$).
   - If `extract_constant_number` cannot resolve the expression to a constant $d \ge 1$ (e.g. unhandled operators like `^`, division by zero, or non-constant variables), it MUST fail closed: emit `ValidationError::ErrNegativeLookback { lag: 0, field: ... }` or `ValidationError::ErrSyntaxMalformed { message: "Unresolvable non-positive lookback window" }`. Never fail open!
2. `densify(...)` Group Input Type Verification:
   - `densify(...)` must verify that its argument is of type `ExprType::Group` (or known grouping identifier like `subindustry`, `industry`, `sector`).
   - If argument is `Matrix`, `Vector`, or `Scalar` (e.g. `densify(close)` or `densify(nws18_ssc)`), emit `ValidationError::ErrGroupTypeMismatch { op: "densify".into(), found: arg_type }`.
   - Propagate any `low_coverage_origins` from the argument so that `densify(fscore_growth)` does NOT erase missing backfill warnings.
3. `vec_*` Input Type Verification:
   - `vec_avg(...)` and `vec_sum(...)` must verify that their input expression evaluates to `ExprType::Vector`.
   - If input is `Matrix` or `Scalar` (e.g. `vec_avg(close)`), emit `ValidationError::ErrVecTypeMismatch { field: ... }`.
4. Test Verification:
   - Add test cases in `tests/stress_schema_validator_tests.rs` for:
     - `ts_delay(close, 0 - (2 ^ 3))` -> fails closed
     - `densify(close)` -> fails closed
     - `vec_avg(close)` -> fails closed
     - valid `ts_delay(close, 5)` -> passes
     - valid `densify(subindustry)` -> passes
     - valid `vec_avg(nws18_ssc)` -> passes
   - Run `cargo check`, `cargo test --test stress_schema_validator_tests`, `cargo test --test metadata_schema_tests`, and full `cargo test`.
   - Run `bash scripts/sanitize_check.sh`.
   - Document commands and results in your handoff report.

Deliverables:
- Write comprehensive handoff report to /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m2_hardening/handoff.md following the 5-component structure.
- Send completion message to parent.
