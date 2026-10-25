# Handoff Report: Milestone M2 Hardening (Iteration 3)

**Author**: Quantitative Developer Worker (`worker_m2_hardening`)  
**Roles**: implementer, qa, specialist  
**Date**: 2026-10-24T20:38:30Z  
**Task**: Fix 3 fail-open vulnerabilities in `src/schema_validator.rs` identified in Challenger report (`challenger_m2_recheck/challenge_report.md`).  
**Handoff Type**: Hard (Task Complete)

---

## 1. Observation

Adversarial probing by Challenger (`challenger_m2_recheck`) identified three fail-open vulnerabilities in `src/schema_validator.rs`:

1. **`ts_*` Look-Ahead Lag Fail-Open Evasion**:
   - In `src/schema_validator.rs:1826-1848`, when `extract_constant_number(&args[1], symbol_table)` failed to evaluate the lag parameter statically (e.g., due to unhandled binary operator `^`, division by zero, or variable lookbacks), the code fell back to:
     ```rust
     let lag_meta = self.eval_expr(&args[1], symbol_table, errors, warnings, extracted_fields);
     if lag_meta.expr_type != FastExprType::Scalar {
         errors.push(...);
     }
     ```
   - Because numeric expressions evaluate to `FastExprType::Scalar`, `lag_meta.expr_type != FastExprType::Scalar` was `false`.
   - Consequently, look-ahead and non-causal expressions like `ts_delay(close, 0 - (2 ^ 3))` (evaluating to -8) and `ts_delay(close, 0 * (2 ^ 1))` (evaluating to 0) passed validation with `0 errors`.

2. **`densify()` Type Erasure & Sparse Field Backfill Bypass**:
   - In `src/schema_validator.rs:1667-1689`, `densify(...)` unconditionally returned:
     ```rust
     return ValueMeta {
         expr_type: FastExprType::DensifiedGroup,
         is_backfilled: true,
         backfill_window: None,
         low_coverage_origins: Vec::new(),
         is_normalized_price: true,
         scalar_value: None,
     };
     ```
   - It failed to verify whether `args[0]` evaluated to `FastExprType::Group`. Continuous matrices (`densify(close)`), 3D event vectors (`densify(nws18_ssc)`), and unbackfilled low-coverage fields (`densify(fscore_growth)`) all passed validation. Furthermore, resetting `low_coverage_origins: Vec::new()` erased missing backfill requirements for low-coverage fields passed inside `densify`.
   - In `group_neutralize` (line 1735), `low_coverage_origins` only took `sig_meta.low_coverage_origins`, discarding any origins present in `group_meta`.

3. **`vec_*` Folding Functions Missing Input Type Verification**:
   - In `src/schema_validator.rs:1636-1664`, vector folding operators (`vec_avg`, `vec_sum`, `vec_max`, `vec_min`, `vec_count`, `vec_std`) evaluated `args[0]` but did not assert `arg_meta.expr_type == FastExprType::Vector`.
   - As a result, matrix and scalar fields like `vec_avg(close)` passed validation with `0 errors`.

---

## 2. Logic Chain

To resolve these defects while preserving all existing valid formulas and enforcing platform invariants:

1. **Diagnostic Taxonomy Expansion**:
   - Added `DiagnosticCode::ErrGroupTypeMismatch` (`"ERR_GROUP_TYPE_MISMATCH"`) to `src/schema_validator.rs`.
   - This provides distinct, actionable diagnostics when non-group expressions (`Matrix`, `Vector`, `Scalar`) are supplied to `densify(...)` or group arguments.

2. **Power Operator Support in Constant Folding**:
   - In `extract_constant_number` (`src/schema_validator.rs:1161-1175`), added support for the binary exponentiation operator `"^"` using `l.powf(r)` (checking `res.is_finite()`).
   - In `eval_expr` for `AstExpr::Binary` (line 1625), added `"^"` to compute scalar values when both operands have known scalar values.

3. **Strict Fail-Closed Lookback Enforcement across `ts_*`**:
   - In `ts_*` operator handling (`src/schema_validator.rs:1826-1875`), the lag argument (located at index `args.len() - 1`) is evaluated using `extract_constant_number`.
   - If resolved, the lag must satisfy `lag_val >= 1 && lag > 0.0 && lag.fract() == 0.0`. If non-positive or non-integer, `DiagnosticCode::ErrNegativeLookback` is emitted.
   - If `extract_constant_number` returns `None` (unresolvable non-constant, division by zero, or variable non-scalar), `eval_expr(&args[lag_idx], ...)` is invoked to extract any inner fields, and `DiagnosticCode::ErrNegativeLookback` is strictly emitted with message `"Operator '{}' lookback lag must be a statically resolvable positive integer"`.
   - In `ts_backfill` (line 1785), similar fail-closed handling ensures `w > 0.0 && w.fract() == 0.0` and evaluates `args[1]` on unresolvable expressions.

4. **`densify(...)` Group Input Type Assertion & Origin Propagation**:
   - In `densify` handling (`src/schema_validator.rs:1685-1745`):
     - Evaluates `arg_meta = self.eval_expr(&args[0], ...)`.
     - Validates that `arg_meta.expr_type == FastExprType::Group` (or named variable matching `"subindustry" | "industry" | "sector" | "market"`).
     - If `arg_meta.expr_type == FastExprType::Vector`, emits `DiagnosticCode::ErrVecTypeMismatch`.
     - If not a group, emits `DiagnosticCode::ErrGroupTypeMismatch` and returns `FastExprType::TypeError` with preserved `low_coverage_origins`.
     - When valid, returns `FastExprType::DensifiedGroup` preserving `arg_meta.low_coverage_origins`.
   - In `group_neutralize` (line 1775), unions `low_coverage_origins` from both `sig_meta` and `group_meta`, ensuring unbackfilled low-coverage fields anywhere in the formula trigger `DiagnosticCode::ErrMissingBackfill`.

5. **`vec_*` Input Type Assertion**:
   - In `vec_*` folding operators (`src/schema_validator.rs:1655-1683`):
     - Checks `if arg_meta.expr_type != FastExprType::Vector`.
     - Emits `DiagnosticCode::ErrVecTypeMismatch` with `field_id: Some(var_name)` if input is `Matrix` or `Scalar`.
     - Returns `FastExprType::TypeError` when input type mismatches, preventing cascading type confusion.

---

## 3. Caveats

- **Symbolic Arithmetic Scope**: `extract_constant_number` folds constant numeric literals and binary operations (`+`, `-`, `*`, `/`, `^`, unary `-`/`+`), as well as assigned constants stored in `symbol_table`. Dynamically calculated series lookbacks (e.g., lookback dependent on rolling volatility) remain strictly prohibited under BRAIN FastExpr syntax and correctly fail closed.
- **Grouping Keywords**: Currently recognizes the standard WorldQuant BRAIN categorical grouping fields: `subindustry`, `industry`, `sector`, and `market`.
- **No platform credentials or live network calls** were required or made for this local syntax/schema hardening task.

---

## 4. Conclusion

All 3 fail-open vulnerabilities identified by Challenger are completely remediated with genuine fail-closed logic:
- `ts_delay(close, 0 - (2 ^ 3))` correctly fails closed with `ERR_NEGATIVE_LOOKBACK` (-8 lag detected).
- `densify(close)` correctly fails closed with `ERR_GROUP_TYPE_MISMATCH` (MATRIX detected instead of GROUP).
- `densify(fscore_growth)` correctly fails closed with both `ERR_GROUP_TYPE_MISMATCH` and `ERR_MISSING_BACKFILL`.
- `vec_avg(close)` correctly fails closed with `ERR_VEC_TYPE_MISMATCH` (MATRIX detected instead of VECTOR).
- Valid expressions (`ts_delay(close, 5)`, `densify(subindustry)`, `vec_avg(nws18_ssc)`) pass validation cleanly with 0 errors.
- All 15 adversarial tests in `tests/stress_schema_validator_tests.rs` pass (`15 passed; 0 failed`).
- All 25 tests in `tests/metadata_schema_tests.rs` pass (`25 passed; 0 failed`).
- Entire workspace test suite passes (107 tests across 9 targets, 0 failed).
- Git hygiene audit (`scripts/sanitize_check.sh`) passed clean with 0 leaks.

---

## 5. Verification Method

### 1. Build and Stress Test Execution
```bash
cargo check
cargo test --test stress_schema_validator_tests
cargo test --test metadata_schema_tests
cargo test
```

### 2. Pre-Push Sanitization Gatekeeper Audit
```bash
bash scripts/sanitize_check.sh
```

### 3. CLI Empirical Reproducers
```bash
# 1. Negative lookback lag with power operator fails closed:
cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 - (2 ^ 3))), densify(subindustry))"
# Output: FAILED (1 errors) [ERR_NEGATIVE_LOOKBACK] Operator 'ts_delay' received non-positive lookback lag (-8)

# 2. Continuous matrix in densify fails closed:
cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(close))"
# Output: FAILED (1 errors) [ERR_GROUP_TYPE_MISMATCH] 'densify' argument must be a categorical group, found MATRIX

# 3. Unbackfilled sparse field in densify fails closed with both errors:
cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(fscore_growth))"
# Output: FAILED (2 errors) [ERR_GROUP_TYPE_MISMATCH] and [ERR_MISSING_BACKFILL]

# 4. Matrix in vec_avg fails closed:
cargo run --quiet -- fields --validate "group_neutralize(rank(vec_avg(close)), densify(subindustry))"
# Output: FAILED (1 errors) [ERR_VEC_TYPE_MISMATCH] Vector folding operator 'vec_avg' requires argument of type VECTOR, found MATRIX

# 5. Valid formula passes with 0 errors:
cargo run --quiet -- fields --validate "group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))"
# Output: PASSED (0 errors, 0 warnings)
```
