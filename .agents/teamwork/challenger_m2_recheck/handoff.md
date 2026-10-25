# Handoff Report: Milestone M2 Remediation Recheck (Iteration 2)

**Author**: Empirical Challenger (`challenger_m2_recheck`)  
**Roles**: critic, specialist  
**Date**: 2026-10-24T20:26:00Z  
**Recipient**: Lead Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)  
**Verdict**: **REQUEST_CHANGES**

---

## 1. Observation

Adversarial testing and empirical probing of `src/schema_validator.rs` produced the following verbatim observations:

### 1.1 Existing Test Suite Status
- Command:
  ```bash
  cargo test --test stress_schema_validator_tests
  ```
  Result:
  ```text
  running 12 tests
  test test_adversarial_option9_syntax_and_trailing_d ... ok
  test test_adversarial_multi_statement_scripts ... ok
  test test_adversarial_densify_enforcement ... ok
  test test_adversarial_negative_lookbacks_and_zero_lags ... ok
  test test_challenger_reproduce_backfill_poisoning_bypass ... ok
  test test_adversarial_trade_when_with_vectors_and_backfill ... ok
  test test_adversarial_vector_folding_rules ... ok
  test test_challenger_reproduce_binary_expr_negative_lookback_bypass ... ok
  test test_challenger_reproduce_empty_and_assignment_only_scripts_pass ... ok
  test test_challenger_reproduce_missing_arity_checks ... ok
  test test_challenger_reproduce_unknown_field_emitted_as_warning_not_error ... ok
  test test_challenger_reproduce_literal_scalar_breaks_price_normalization ... ok

  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
  ```
- Command:
  ```bash
  cargo test
  ```
  Result:
  `143 passed; 0 failed; 0 ignored` across all unit and integration test suites.
- Command:
  ```bash
  bash scripts/sanitize_check.sh
  ```
  Result:
  `✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.`

### 1.2 Verbatim Evasion Failures in Remediated Engine

#### Observation A: Look-Ahead Lag Fail-Open Evasion in `ts_*` Operators
- In `src/schema_validator.rs:1826-1848`:
  ```rust
  if let Some(lag) = extract_constant_number(&args[1], symbol_table) {
      if lag <= 0.0 {
          errors.push(ValidationError {
              code: DiagnosticCode::ErrNegativeLookback,
              field_id: None,
              message: format!(
                  "Operator '{}' received non-positive lookback lag ({})",
                  func, lag
              ),
              suggestion: Some("Lag parameter must be >= 1 to prevent look-ahead bias".to_string()),
          });
      }
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
- In `src/schema_validator.rs:1158-1167`:
  `extract_constant_number` only matches `+`, `-`, `*`, `/`. For other operators such as `^` (power), it returns `None`.
- Command (Negative lookahead lag evaluating to -8):
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 - (2 ^ 3))), densify(subindustry))"
  ```
  Verbatim output:
  ```text
  🔍 Validating FastExpr Formula Schema (delay=1):
  --------------------------------------------------
  Expression: group_neutralize(rank(ts_delay(close, 0 - (2 ^ 3))), densify(subindustry))

  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)

  📊 Extracted Fields (1) :
    - close [MATRIX] (coverage: 100.0%)
  ```
- Command (Contemporaneous zero lag evaluating to 0):
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 * (2 ^ 1))), densify(subindustry))"
  ```
  Verbatim output:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```

#### Observation B: `densify()` Type Erasure & Low Coverage Origin Reset
- In `src/schema_validator.rs:1667-1689`:
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
- Command (Low coverage field `fscore_growth` with 29.9% coverage without `ts_backfill`):
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(fscore_growth))"
  ```
  Verbatim output:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 1 warnings)
  WARNING: [WARN_SUBUNIVERSE_RISK] Field 'fscore_growth' has low coverage (29.9%).
  ```
  `ERR_MISSING_BACKFILL` was completely suppressed because `densify` wiped `low_coverage_origins`.
- Command (Continuous price matrix passed to `densify`):
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(close))"
  ```
  Verbatim output:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```
- Command (3D event vector passed to `densify`):
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(nws18_ssc))"
  ```
  Verbatim output:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  Extracted Fields: nws18_ssc [VECTOR (folded)]
  ```

#### Observation C: `vec_*` Folding Functions Missing Input Type Check
- Command:
  ```bash
  cargo run --quiet -- fields --validate "group_neutralize(rank(vec_avg(close)), densify(subindustry))"
  ```
  Verbatim output:
  ```text
  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
  ```

---

## 2. Logic Chain

1. **Defect 1 (`ts_*` Lookback Fail-Open)**:
   - Observation A proves that when `extract_constant_number` cannot resolve a lag expression (e.g. `0 - (2 ^ 3)`), it returns `None`.
   - The fallback logic in lines 1838-1848 only tests `lag_meta.expr_type != FastExprType::Scalar`.
   - Since `0 - (2 ^ 3)` has type `Scalar`, the check evaluates to `false`. No error is recorded.
   - Consequently, non-causal lookbacks ($d \le 0$) can be injected via unhandled scalar arithmetic or operators, completely evading causal validation and violating AGENTS.md quantitative standards.

2. **Defect 2 (`densify()` Erasure & Fail-Open)**:
   - Observation B proves that `densify` executes `let _ = self.eval_expr(&args[0], ...)` and unconditionally returns `FastExprType::DensifiedGroup` with `low_coverage_origins: Vec::new()`.
   - Any operand passed into `densify(...)` is accepted, whether it is a continuous matrix (`close`), an unfolded 3D event vector (`nws18_ssc`), or an unbackfilled sparse field (`fscore_growth`).
   - Wiping `low_coverage_origins` creates a new backfill poisoning loophole: placing a sparse field inside `densify()` conceals it from root backfill verification.

3. **Comparison with Invariant Contracts**:
   - `PROJECT.md:10` requires: "Strictly positive causal lookbacks ($d \ge 1$) on time-series operators."
   - `PROJECT.md:78` requires: "If type == 'VECTOR', expression MUST wrap in vec_avg(...) or vec_sum(...) before applying scalar/matrix operators."
   - `PROJECT.md:79` requires: "If coverage < 0.50, expression MUST apply ts_backfill(..., d >= 5)."
   - Both Defect 1 and Defect 2 create concrete, reproducible bypasses to these mandatory project and platform invariants.

---

## 3. Caveats

- **No Caveats**: All vulnerabilities were empirically verified via direct execution of `cargo run -- fields --validate` and reproduced with exact error/pass states.
- The 12 regression tests in `tests/stress_schema_validator_tests.rs` pass, proving that the worker fixed the specific test cases from Iteration 1, but the underlying validator logic remains structurally fail-open for unresolvable scalar lookbacks and `densify` inputs.

---

## 4. Conclusion

**Verdict: REQUEST_CHANGES**

The Milestone M2 Schema Validator implementation cannot be approved in its current state. While the initial 5 bugs from Iteration 1 were fixed, the validator fails open on two critical security/quant invariants:
1. Negative lookbacks can bypass validation via power operators or unresolvable scalar expressions.
2. `densify` accepts non-group fields and erases low-coverage origins, bypassing backfill enforcement.

The worker must make the following specific fixes:
1. In `src/schema_validator.rs:1838-1848` (`ts_*` operators): fail closed if the lag parameter cannot be statically resolved to a positive integer ($d \ge 1$), exactly like `ts_backfill` does on line 1780.
2. In `src/schema_validator.rs:1147-1174` (`extract_constant_number`): add support for `^` (power) via `f64::powf` so valid constant exponents can be evaluated.
3. In `src/schema_validator.rs:1667-1689` (`densify`): enforce that `args[0]` evaluates to `FastExprType::Group`, reject `Vector` and `Matrix`, and do not erase unbackfilled `low_coverage_origins`.
4. In `src/schema_validator.rs:1636-1664` (`vec_*`): enforce that `args[0]` evaluates to `FastExprType::Vector`.

---

## 5. Verification Method

To verify these findings and confirm fixes:

1. **Verify Lookahead Bypass (Should Fail, Currently Passes)**:
   ```bash
   cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 - (2 ^ 3))), densify(subindustry))"
   ```
   *Expected behavior after fix*: Fails with `ERR_NEGATIVE_LOOKBACK`.

2. **Verify Contemporaneous Lag Bypass (Should Fail, Currently Passes)**:
   ```bash
   cargo run --quiet -- fields --validate "group_neutralize(rank(ts_delay(close, 0 * (2 ^ 1))), densify(subindustry))"
   ```
   *Expected behavior after fix*: Fails with `ERR_NEGATIVE_LOOKBACK`.

3. **Verify `densify` Sparse Field Bypass (Should Fail, Currently Passes)**:
   ```bash
   cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(fscore_growth))"
   ```
   *Expected behavior after fix*: Fails with `ERR_UNDENSIFIED_GROUP` or `ERR_MISSING_BACKFILL`.

4. **Verify `densify` Continuous Matrix Bypass (Should Fail, Currently Passes)**:
   ```bash
   cargo run --quiet -- fields --validate "group_neutralize(rank(close), densify(close))"
   ```
   *Expected behavior after fix*: Fails with `ERR_UNDENSIFIED_GROUP`.

5. **Full Regression Suite**:
   ```bash
   cargo test --test stress_schema_validator_tests
   cargo test --test metadata_schema_tests
   cargo test
   bash scripts/sanitize_check.sh
   ```
