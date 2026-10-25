# Empirical Challenge Report: FastExpr Type & AST Schema Validator

**Challenger**: Challenger 1 (FastExpr Type & AST Challenger)  
**Milestone**: M2 (Field Metadata Extraction & Schema Validator)  
**Target Module**: `src/schema_validator.rs`  
**Test Suite**: `tests/stress_schema_validator_tests.rs`  
**Date**: 2026-10-24T19:57:00Z  

---

## 1. Executive Summary

- **Overall Risk Assessment**: **HIGH**
- **Explicit Verdict**: **REQUEST_CHANGES**
- **Empirical Test Coverage**: 12 adversarial test cases executed via `tests/stress_schema_validator_tests.rs` (12 passed; 0 failed).
- **Core Finding**: While the AST parser and type lattice handle basic expressions, folded vectors, and simple group neutralizations effectively, the validator contains **critical fail-open vulnerabilities** in its backfill tracking, lookback parameter evaluation, function arity enforcement, and unknown field diagnosis that allow malformed and invalid expressions to pass local validation.

---

## 2. Identified Vulnerabilities & Failure Modes

### [Critical] Vulnerability 1: Backfill Poisoning / Bypass in Binary Expressions
- **Location**: `src/schema_validator.rs:1520` & `src/schema_validator.rs:1282`
- **Challenged Assumption**: "Expressions with sparse coverage (< 50%) fields always fail unless wrapped in ancestral `ts_backfill(..., d >= 5)`."
- **Attack Scenario**:
  In binary operations (`AstExpr::Binary`), the `ValueMeta` for the binary node sets:
  ```rust
  is_backfilled: l.is_backfilled || r.is_backfilled,
  ```
  If operand `l` was backfilled (e.g., `ts_backfill(fscore_momentum, 15)`), `is_backfilled` evaluates to `true`.
  When operand `r` is an **unbackfilled** low-coverage field (e.g., `fscore_growth` with 29.8% coverage), its low-coverage origin is appended to `combined_origins`.
  However, at line 1282:
  ```rust
  if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled {
      // emit ErrMissingBackfill
  }
  ```
  Because `final_meta.is_backfilled` was poisoned to `true` by operand `l`, the error check is completely bypassed!
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
  ```
  *Result*: Evaluated as **PASSED (0 errors)** despite `fscore_growth` being completely unbackfilled!
- **Blast Radius**: High. In Milestone M3 factor combinations where multiple signals are linearly blended, any backfilled signal will shield all other unbackfilled sparse signals, causing simulation failures or subuniverse consistency test disqualifications on BRAIN.
- **Mitigation**:
  1. Do not use a blanket boolean `is_backfilled` to bypass check when `low_coverage_origins` is non-empty. In `ts_backfill(sig, d >= 5)`, low coverage origins are already cleared (`Vec::new()`). If `low_coverage_origins` is non-empty, that means unbackfilled fields are present and MUST be rejected, regardless of `is_backfilled`.
  2. For binary operations, `is_backfilled` should be `l.is_backfilled && r.is_backfilled`.

---

### [High] Vulnerability 2: Lookback Parameter Bypass via Binary Expressions and Variables
- **Location**: `src/schema_validator.rs:1618-1624` & `src/schema_validator.rs:1684-1698`
- **Challenged Assumption**: "Non-positive and look-ahead lookback lags ($d \le 0$) are strictly rejected."
- **Attack Scenario**:
  The function `extract_literal_number` only handles `AstExpr::Literal(n)` and `AstExpr::Unary { op: "-", ... }`.
  When an expression like `1 - 2` or a variable `lag = -5; ts_delay(close, lag)` is used, `extract_literal_number` returns `None`.
  Line 1685 performs:
  ```rust
  if let Some(lag) = extract_literal_number(&args[1]) {
      if lag <= 0.0 { ... }
  }
  ```
  When `extract_literal_number` returns `None`, the check is silently bypassed!
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))"
  cargo run -- fields --validate "lag = -5; group_neutralize(rank(ts_delay(close, lag)), densify(subindustry))"
  ```
  *Result*: Both evaluate to **PASSED (0 errors)** with negative lookback!
- **Blast Radius**: High. Bypasses look-ahead bias prevention; negative lags peek into future data on platform simulations.
- **Mitigation**:
  1. Evaluate constant folded binary arithmetic inside `extract_literal_number` (or fail closed when the lag parameter is not a statically resolvable positive integer literal).
  2. If a variable is used as a lag, check the constant value in `symbol_table` or disallow non-literal lags.

---

### [High] Vulnerability 3: Missing Function Arity Validation
- **Location**: `src/schema_validator.rs:1534`, `1559`, `1668`, `1710`
- **Challenged Assumption**: "Operator signatures and parameter counts are validated to prevent platform HTTP 400 Bad Request errors."
- **Attack Scenario**:
  - `ts_*` functions (e.g. `ts_delay`) check `if !args.is_empty()` and `if args.len() >= 2`, but do NOT error if `args.len() < 2`. Calling `ts_delay(close)` or `ts_delay()` passes with 0 errors.
  - `rank()` checks `if !args.is_empty()`, but if empty, falls through to the default handler and returns `Matrix` with 0 errors.
  - `densify()` has no argument length check; `densify()` with 0 arguments passes with 0 errors.
  - `vec_avg(...)` checks `if args.is_empty()`, but does not reject excess arguments (`vec_avg(nws18_ssc, nws18_ssc)` passes with 0 errors).
- **Empirical Reproducers**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(ts_delay()), densify(subindustry))"
  cargo run -- fields --validate "group_neutralize(rank(), densify(subindustry))"
  cargo run -- fields --validate "group_neutralize(rank(close), densify())"
  ```
  *Result*: All pass with **0 errors**.
- **Blast Radius**: High. Alphas generated by M3 with malformed arities will be dispatched to BRAIN, consuming simulation quota and returning HTTP 400.
- **Mitigation**:
  Enforce strict parameter count checks for all known functions (`ts_delay`: exactly 2; `rank`: exactly 1; `densify`: exactly 1; `vec_*`: exactly 1).

---

### [Medium] Vulnerability 4: Inverted Diagnostic Codes & Unknown Fields Failing Open
- **Location**: `src/schema_validator.rs:1447-1456` vs `src/schema_validator.rs:1368`
- **Challenged Assumption**: "Unrecognized field identifiers fail closed with `ErrUnknownField`."
- **Attack Scenario**:
  The developer defined both `ErrUnknownField` and `WarnHorizonSyntax`. However:
  1. On line 1368 (trailing 'd' in option9, e.g. `pcr_vol_20d`), `ErrUnknownField` is used.
  2. On line 1450 (an actual unknown field not found in catalog), `warnings.push(ValidationWarning { code: DiagnosticCode::WarnHorizonSyntax, ... })` is used, and the return type defaults to `Matrix` with `is_backfilled: true`.
  Because it is a warning rather than an error, any misspelled or completely invented field identifier passes validation as a valid alpha!
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(completely_invented_fake_field_12345), densify(subindustry))"
  ```
  *Result*: Evaluates to **PASSED (0 errors, 1 warnings)**!
- **Blast Radius**: Medium. Typoed field names bypass validation and fail only at simulation runtime.
- **Mitigation**:
  Fix diagnostic code inversion:
  - If a field is not found in catalog and not a built-in keyword (`subindustry`, `market`, etc.), push `ValidationError` with `DiagnosticCode::ErrUnknownField`.
  - Use `WarnHorizonSyntax` for horizon formatting discrepancies.

---

### [Medium] Vulnerability 5: Empty Expressions and Assignment-Only Scripts Pass Validation
- **Location**: `src/schema_validator.rs:1240-1257` & `1260-1297`
- **Challenged Assumption**: "Expressions must contain an executable terminal tensor expression."
- **Attack Scenario**:
  - Validating `""` or `"# comment"` parses 0 statements; `errors.is_empty()` returns true.
  - Validating `x = 1; y = 2;` parses 2 assignments and 0 expressions; `errors.is_empty()` returns true.
  - Unbackfilled low-coverage fields inside assignments (`x = fscore_growth;`) are never validated because the backfill check is only executed within `AstStatement::Expression`.
- **Empirical Reproducers**:
  ```bash
  cargo run -- fields --validate ""
  cargo run -- fields --validate "# Just a comment"
  cargo run -- fields --validate "x = fscore_growth;"
  ```
  *Result*: All evaluate to **PASSED (0 errors)**.
- **Blast Radius**: Medium. Dispatches empty payloads or incomplete scripts to the platform.
- **Mitigation**:
  Require that `statements` is non-empty and contains at least one final `AstStatement::Expression`.

---

### [Medium] Vulnerability 6: `ts_backfill` Directly Accepted on Unfolded Vector Fields
- **Location**: `src/schema_validator.rs:1627-1665`
- **Challenged Assumption**: "Vector fields must be folded before time-series operators are applied."
- **Attack Scenario**:
  Unlike lines 1674-1681 where `ts_*` functions verify `sig_meta.expr_type != FastExprType::Vector`, `ts_backfill` (handled separately) lacks this check. It accepts an unfolded vector, returning `FastExprType::Vector`.
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "v = ts_backfill(nws18_ssc, 15); folded = vec_avg(v); group_neutralize(rank(folded), densify(subindustry))"
  ```
  *Result*: Evaluates to **PASSED (0 errors)**.
- **Blast Radius**: Medium. Violates the fundamental dimensional rule that vector event streams must be folded to 2D daily cross-sections before applying cross-temporal lookbacks.
- **Mitigation**:
  In `ts_backfill`, add:
  ```rust
  if sig_meta.expr_type == FastExprType::Vector {
      errors.push(ValidationError {
          code: DiagnosticCode::ErrVecTypeMismatch,
          message: "ts_backfill cannot operate directly on VECTOR field; fold first".into(),
          ...
      });
  }
  ```

---

### [Minor] Vulnerability 7: Numerical Scalar `1.0` Breaks Price Normalization
- **Location**: `src/schema_validator.rs:1324-1330` & `1505-1513`
- **Challenged Assumption**: "Standard normalized formulas like `call_breakeven_30 / close - 1.0` pass without false warnings."
- **Attack Scenario**:
  `AstExpr::Literal(_)` initializes `ValueMeta` with `is_normalized_price: false`.
  In `call_breakeven_30 / close - 1.0`, `(call_breakeven_30 / close)` has `is_normalized_price = true`, but `1.0` has `is_normalized_price = false`.
  The binary subtraction checks `l.is_normalized_price && r.is_normalized_price`, which evaluates to `false`.
  Consequently, `rank(call_breakeven_30 / close - 1.0)` triggers a false positive `WarnUnnormalizedPrice`, despite the fact that the validator's own suggestion literally recommends `call_breakeven_30 / close - 1`!
- **Empirical Reproducer**:
  `test_challenger_reproduce_literal_scalar_breaks_price_normalization` in `tests/stress_schema_validator_tests.rs`.
- **Mitigation**:
  For scalar literals (`AstExpr::Literal`), set `is_normalized_price: true` (scalars do not carry asset-level price unit dimensions).

---

### [Minor] Vulnerability 8: UI Diagnostic Mislabel of Vector Fields
- **Location**: `src/schema_validator.rs:188`
- **Observation**:
  `print_diagnostics` formats extracted fields as:
  ```rust
  let type_str = if f.is_vector { "VECTOR (folded)".magenta().bold() } else { "MATRIX".green() };
  ```
  Even when an expression FAILED specifically because a vector was unfolded, the diagnostic summary reports:
  `- nws18_ssc [VECTOR (folded)]`!
- **Mitigation**:
  Report `VECTOR (unfolded)` if the field was involved in a type mismatch error, or track folding state on `ExtractedFieldMeta`.

---

## 3. Stress Test Results Summary

Test suite `tests/stress_schema_validator_tests.rs`:
```
running 12 tests
test test_adversarial_densify_enforcement ... ok
test test_adversarial_negative_lookbacks_and_zero_lags ... ok
test test_adversarial_option9_syntax_and_trailing_d ... ok
test test_adversarial_trade_when_with_vectors_and_backfill ... ok
test test_adversarial_vector_folding_rules ... ok
test test_adversarial_multi_statement_scripts ... ok
test test_challenger_reproduce_backfill_poisoning_bypass ... ok
test test_challenger_reproduce_binary_expr_negative_lookback_bypass ... ok
test test_challenger_reproduce_empty_and_assignment_only_scripts_pass ... ok
test test_challenger_reproduce_literal_scalar_breaks_price_normalization ... ok
test test_challenger_reproduce_missing_arity_checks ... ok
test test_challenger_reproduce_unknown_field_emitted_as_warning_not_error ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

All 6 vulnerabilities were cleanly reproduced with empirical tests.

---

## 4. Verdict and Recommended Action

### Explicit Verdict: **REQUEST_CHANGES**

### Action Items for Worker (`worker_m2_1`):
1. **Fix Backfill Origin Checking (`src/schema_validator.rs:1282, 1520`)**:
   - Check `!final_meta.low_coverage_origins.is_empty()` unconditionally (if origins remain, backfill was missing).
   - Change binary backfill propagation to `l.is_backfilled && r.is_backfilled`.
2. **Close Lookback Bypasses (`src/schema_validator.rs:1618, 1684`)**:
   - Recursively evaluate constant binary expressions (`1 - 2`) in `extract_literal_number` and look up integer variables in `symbol_table`.
3. **Add Arity Enforcement**:
   - Validate exact parameter counts for `ts_*`, `rank`, `densify`, and `vec_*`.
4. **Fix Unknown Field Diagnostic Inversion (`src/schema_validator.rs:1447, 1368`)**:
   - Unknown fields MUST produce `ValidationError { code: DiagnosticCode::ErrUnknownField }` and fail validation.
   - Option9 trailing 'd' should use `DiagnosticCode::WarnHorizonSyntax`.
5. **Enforce Script Structure**:
   - Require non-empty statements with at least one terminal `AstStatement::Expression`.
6. **Disallow Unfolded Vectors in `ts_backfill`**:
   - Check `sig_meta.expr_type != FastExprType::Vector` before backfilling.
7. **Fix Scalar Normalization**:
   - Set `is_normalized_price: true` for `AstExpr::Literal`.
