# Adversarial Challenge Report: Milestone M2 Remediation Recheck (Iteration 2)

**Author**: Empirical Challenger (`challenger_m2_recheck`)  
**Roles**: critic, specialist  
**Date**: 2026-10-24T20:25:00Z  
**Target Module**: `src/schema_validator.rs`  
**Overall Risk Assessment**: **HIGH**  
**Verdict**: **REQUEST_CHANGES**

---

## 1. Challenge Summary

Milestone M2 remediation (Iteration 2) successfully resolved the 5 primary defects identified in Iteration 1:
- Backfill poisoning in standard binary operations (`+`, `-`, `*`, `/`) is prevented.
- Simple arithmetic lookback expressions (`1 - 2`, `lag = -5`) are properly folded and caught.
- Unitless scalar literals no longer break price normalization (`call_breakeven_30 / close - 1.0`).
- Function arity is enforced for 0-argument calls (`densify()`, `rank()`, `ts_delay()`).
- Empty strings and assignment-only scripts fail closed.

All 12 empirical stress tests in `tests/stress_schema_validator_tests.rs` pass with `12 passed; 0 failed`.

However, deep adversarial probing beyond the 12 existing test cases revealed **two new significant fail-open vulnerabilities** in `src/schema_validator.rs`:
1. **[CRITICAL] Look-Ahead Lag Fail-Open Evasion in `ts_*` Operators**: `ts_*` time-series operators (e.g., `ts_delay`, `ts_delta`, `ts_mean`, `ts_decay_linear`) fail open when the lag parameter cannot be statically evaluated by `extract_constant_number` (such as power operators `^`, division by zero, or unhandled binary ops), provided the evaluated AST node has type `Scalar`. Look-ahead expressions like `ts_delay(close, 0 - (2 ^ 3))` (evaluating to -8) and `ts_delay(close, 0 * (2 ^ 1))` (evaluating to 0) pass validation with 0 errors!
2. **[HIGH] `densify()` Type Erasure & Sparse Field Backfill Bypass**: `densify(...)` evaluates its inner expression but ignores the resulting type, unconditionally returning `FastExprType::DensifiedGroup` and resetting `low_coverage_origins: Vec::new()`. Consequently, continuous matrices (`densify(close)`), unfolded 3D event vectors (`densify(nws18_ssc)`), and unbackfilled low-coverage fields (`densify(fscore_growth)`) all pass validation without error, bypassing vector folding rules and backfill requirements.
3. **[MEDIUM] `vec_*` Folding Functions Accept Non-Vector Arguments**: `vec_avg(close)` passes validation without type error, failing to assert that the inner argument is of type `Vector`.

---

## 2. Challenges

### [Critical] Challenge 1: Non-Causal Look-Ahead Lag Evasion via Unhandled Scalar Operations in `ts_*`

- **Assumption challenged**: The worker assumed in `worker_m2_remediation/handoff.md:63` that:
  > "For `ts_*`, if the lag cannot be statically resolved and its evaluated type is not `Scalar`, it fails closed with `ErrNegativeLookback`."
  and claimed in lines 99-100:
  > "Non-causal negative or zero lookback lags wrapped in arithmetic expressions or variables are strictly caught."
- **Attack scenario**:
  `extract_constant_number` in `src/schema_validator.rs:1147-1174` only supports `+`, `-`, `*`, `/`. For any binary expression involving other operators like `^` (power), division by zero, or unhandled functions, `extract_constant_number` returns `None`.
  When `extract_constant_number` returns `None`, line 1838 is reached:
  ```rust
  } else {
      let lag_meta = self.eval_expr(&args[1], symbol_table, errors, warnings, extracted_fields);
      if lag_meta.expr_type != FastExprType::Scalar {
          errors.push(ValidationError {
              code: DiagnosticCode::ErrNegativeLookback,
              field_id: None,
              message: format!("Operator '{}' lookback lag must be a scalar constant", func),
              suggestion: Some("Provide a constant positive integer for the lag parameter".to_string()),
          });
      }
  }
  ```
  Because numeric expressions evaluate to `FastExprType::Scalar`, `lag_meta.expr_type != FastExprType::Scalar` evaluates to `false`!
  No error is pushed, and the negative or zero lag passes validation!
  
  **Empirical Reproducer 1 (Negative Lookback / Future Look-Ahead Bias)**:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 - (2 ^ 3))), densify(subindustry))"
  ```
  *Actual Result*:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```
  *Severity*: Critical. `0 - (2 ^ 3) = -8`, allowing future price peeking that corrupts backtests and causes platform 400 Bad Request simulation rejection.

  **Empirical Reproducer 2 (Zero Lookback / Non-Causal Contemporaneity)**:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 * (2 ^ 1))), densify(subindustry))"
  ```
  *Actual Result*:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```
  *Severity*: Critical. In FastExpr, `ts_delay` requires lag $d \ge 1$.

- **Blast radius**: Non-causal formulas with negative or zero lags pass local pre-simulation checks, leading to failed simulations on BRAIN, wasted quota, and risk of backtest look-ahead contamination.
- **Mitigation**:
  Align `ts_*` lag validation with `ts_backfill` (line 1780):
  If the lag parameter cannot be statically resolved by `extract_constant_number` to a constant positive integer $\ge 1$, it must fail closed with `ErrNegativeLookback`:
  ```rust
  if let Some(lag) = extract_constant_number(&args[1], symbol_table) {
      if lag <= 0.0 {
          errors.push(ValidationError {
              code: DiagnosticCode::ErrNegativeLookback,
              field_id: None,
              message: format!("Operator '{}' received non-positive lookback lag ({})", func, lag),
              suggestion: Some("Lag parameter must be >= 1 to prevent look-ahead bias".to_string()),
          });
      }
  } else {
      errors.push(ValidationError {
          code: DiagnosticCode::ErrNegativeLookback,
          field_id: None,
          message: format!("Operator '{}' lookback lag must be a statically resolvable positive integer", func),
          suggestion: Some("Provide a constant positive integer for the lag parameter (d >= 1)".to_string()),
      });
  }
  ```
  Additionally, add `^` support to `extract_constant_number` using `f64::powf`.

---

### [High] Challenge 2: `densify()` Type Erasure & Sparse Field Backfill Bypass

- **Assumption challenged**: The validator assumes `densify(...)` operates only on categorical groups and does not alter the validation requirements of other fields.
- **Attack scenario**:
  In `src/schema_validator.rs:1667-1689`:
  ```rust
  if func_lower == "densify" {
      if args.len() != 1 { ... }
      let _ = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
      return ValueMeta {
          expr_type: FastExprType::DensifiedGroup,
          is_backfilled: true,
          backfill_window: None,
          low_coverage_origins: Vec::new(),
          is_normalized_price: true,
          scalar_value: None,
      };
  }
  ```
  Notice that `densify` unconditionally returns `FastExprType::DensifiedGroup` and wipes out all `low_coverage_origins`. It never checks whether the argument passed to `densify` is actually a `Group`.
  
  **Empirical Reproducer 1 (Sparse Field Backfill Bypass via `densify`)**:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(fscore_growth))"
  ```
  *Actual Result*:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 1 warnings)
  ```
  `fscore_growth` (coverage 29.9%) has no ancestral `ts_backfill`, but `densify()` erased its origin from `low_coverage_origins`, bypassing `ERR_MISSING_BACKFILL`!

  **Empirical Reproducer 2 (Continuous Price Matrix Treated as Group)**:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(close))"
  ```
  *Actual Result*:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```
  Continuous matrix `close` is accepted as a grouping field!

  **Empirical Reproducer 3 (3D Unfolded Event Vector Passed to `densify`)**:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(nws18_ssc))"
  ```
  *Actual Result*:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  Extracted Fields: nws18_ssc [VECTOR (folded)]
  ```
  An unfolded 3D event vector stream is accepted into `densify` and mislabeled as folded!

- **Blast radius**: Formulas passing continuous fields or vectors into `densify` cause simulation errors on BRAIN. Low-coverage fields passed into grouping positions escape the mandatory backfill rule.
- **Mitigation**:
  In `densify`, inspect `arg_meta = self.eval_expr(&args[0], ...)`:
  - If `arg_meta.expr_type != FastExprType::Group`, emit `ErrUndensifiedGroup` or `ErrSyntaxMalformed` ("densify argument must be a categorical group, found ...").
  - Preserve `low_coverage_origins` from `arg_meta` or emit `ErrMissingBackfill` if `arg_meta` contains low-coverage fields.

---

### [Medium] Challenge 3: `vec_*` Folding Operators Lack Input Type Verification

- **Assumption challenged**: Folding operators (`vec_avg`, `vec_sum`, `vec_max`, `vec_min`, `vec_count`, `vec_std`) are expected to validate that their input is a `VECTOR`.
- **Attack scenario**:
  In `src/schema_validator.rs:1636-1664`:
  ```rust
  let arg_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
  return ValueMeta {
      expr_type: FastExprType::Matrix,
      ...
  };
  ```
  The validator does not check `arg_meta.expr_type == FastExprType::Vector`.
  
  **Empirical Reproducer**:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(vec_avg(close)), densify(subindustry))"
  ```
  *Actual Result*:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```
- **Blast radius**: Passing matrix fields to vector-only operators causes HTTP 400 Bad Request simulation errors on BRAIN.
- **Mitigation**:
  Check `if arg_meta.expr_type != FastExprType::Vector { errors.push(ErrVecTypeMismatch); }`.

---

## 3. Stress Test Results

| Test ID | Scenario | Expected Behavior | Actual Behavior | Result |
|---|---|---|---|---|
| `stress_1` | 12 tests in `tests/stress_schema_validator_tests.rs` | All 12 pass | 12 passed; 0 failed | **PASS** |
| `stress_2` | Deep left-leaning binary tree with sparse field | Fail with `ERR_MISSING_BACKFILL` | Failed with `ERR_MISSING_BACKFILL` | **PASS** |
| `stress_3` | Deep right-leaning binary tree with sparse field | Fail with `ERR_MISSING_BACKFILL` | Failed with `ERR_MISSING_BACKFILL` | **PASS** |
| `stress_4` | Balanced binary tree with mixed backfilled/unbackfilled | Fail with `ERR_MISSING_BACKFILL` | Failed with `ERR_MISSING_BACKFILL` | **PASS** |
| `stress_5` | 3 sparse fields, 2 backfilled and 1 unbackfilled | Fail with `ERR_MISSING_BACKFILL` | Failed with `ERR_MISSING_BACKFILL` | **PASS** |
| `stress_6` | 3 sparse fields, all 3 backfilled ($d \ge 5$) | Pass with 0 errors | Passed (0 errors) | **PASS** |
| `stress_7` | Direct VECTOR argument to `ts_backfill` | Fail with `ERR_VEC_TYPE_MISMATCH` | Failed with `ERR_VEC_TYPE_MISMATCH` | **PASS** |
| `stress_8` | Folded vector `vec_avg(nws18_ssc)` to `ts_backfill` | Pass with 0 errors | Passed (0 errors) | **PASS** |
| `stress_9` | `ts_backfill(fscore_growth, 5 - 10)` | Fail with `ERR_NEGATIVE_LOOKBACK` | Failed with `ERR_NEGATIVE_LOOKBACK` | **PASS** |
| `stress_10` | `ts_backfill(fscore_growth, 2 * 2)` (window 4 < 5) | Fail with `ERR_MISSING_BACKFILL` | Failed with `ERR_MISSING_BACKFILL` | **PASS** |
| `stress_11` | `ts_delay(close, 2 * (3 - 5))` (evaluates to -4) | Fail with `ERR_NEGATIVE_LOOKBACK` | Failed with `ERR_NEGATIVE_LOOKBACK` | **PASS** |
| `stress_12` | `ts_delay(close, 0 - (2 ^ 3))` (evaluates to -8) | Fail with `ERR_NEGATIVE_LOOKBACK` | **PASSED (0 errors)** | **FAIL** |
| `stress_13` | `ts_delay(close, 0 * (2 ^ 1))` (evaluates to 0) | Fail with `ERR_NEGATIVE_LOOKBACK` | **PASSED (0 errors)** | **FAIL** |
| `stress_14` | `group_neutralize(rank(close), densify(close))` | Fail: `close` is not a group | **PASSED (0 errors)** | **FAIL** |
| `stress_15` | `group_neutralize(rank(close), densify(fscore_growth))` | Fail with `ERR_MISSING_BACKFILL` | **PASSED (0 errors)** | **FAIL** |
| `stress_16` | `group_neutralize(rank(close), densify(nws18_ssc))` | Fail with `ERR_VEC_TYPE_MISMATCH` | **PASSED (0 errors)** | **FAIL** |
| `stress_17` | `group_neutralize(rank(vec_avg(close)), densify(subindustry))` | Fail: `close` is not a vector | **PASSED (0 errors)** | **FAIL** |
| `stress_18` | Pre-push Git hygiene audit (`scripts/sanitize_check.sh`) | Pass clean with 0 leaks | Passed with 0 leaks | **PASS** |

---

## 4. Unchallenged Areas

- **Platform Network Simulation Execution**: Live network requests to BRAIN API were out of scope for Milestone M2 schema validator recheck.
- **Full Dataset Field Downloader Coverage**: Data field caching pagination logic was verified via unit tests; live API rate limiting was not challenged live.

---

## 5. Explicit Recommendation

**Verdict**: **REQUEST_CHANGES**

The implementer must address Challenges 1 and 2:
1. Make `ts_*` operator lookback lag validation fail closed if the lag cannot be statically resolved to a positive constant integer $d \ge 1$ (mirroring `ts_backfill`).
2. Require `densify(group)` argument to evaluate to `FastExprType::Group`, and prevent it from erasing `low_coverage_origins`.
