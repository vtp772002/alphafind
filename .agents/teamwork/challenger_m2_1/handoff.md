# Handoff Report: Challenger 1 — Milestone M2 (Schema Validator Engine)

**Author**: Empirical Challenger 1 (`challenger_m2_1`)  
**Parent Agent**: Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)  
**Date**: 2026-10-24T19:57:30Z  
**Type**: Hard Handoff (Review Complete)  
**Verdict**: **REQUEST_CHANGES**  

---

## 1. Observation

Direct empirical observations and verbatim tool outputs:

1. **Backfill Poisoning / Bypass in Binary Operations**:
   - In `src/schema_validator.rs:1520`:
     ```rust
     ValueMeta {
         expr_type: FastExprType::Matrix,
         is_backfilled: l.is_backfilled || r.is_backfilled,
         ...
     }
     ```
   - In `src/schema_validator.rs:1282`:
     ```rust
     if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled {
     ```
   - Execution command and verbatim output:
     ```bash
     cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
     ```
     Output:
     ```
     ✅ FastExpr Schema Validation: PASSED (0 errors, 2 warnings)
     ```
     `fscore_growth` has 29.87% cross-sectional coverage and was unbackfilled, yet passed with 0 errors.

2. **Negative Lookback Evaluation Bypass**:
   - In `src/schema_validator.rs:1618-1624`:
     `extract_literal_number` evaluates only single literals and unary `-`, returning `None` for binary expressions and variables.
   - Execution command and verbatim output:
     ```bash
     cargo run -- fields --validate "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))"
     cargo run -- fields --validate "lag = -5; group_neutralize(rank(ts_delay(close, lag)), densify(subindustry))"
     ```
     Output: Both evaluated as **PASSED (0 errors)** with non-causal lookbacks.

3. **Missing Function Arity Checks**:
   - `src/schema_validator.rs:1668` (`ts_*`), line 1710 (`rank`), line 1559 (`densify`):
     ```bash
     cargo run -- fields --validate "group_neutralize(rank(ts_delay()), densify(subindustry))"
     cargo run -- fields --validate "group_neutralize(rank(), densify(subindustry))"
     cargo run -- fields --validate "group_neutralize(rank(close), densify())"
     ```
     Output: All evaluated as **PASSED (0 errors)**.

4. **Unknown Fields Emitted as Warning instead of Error**:
   - In `src/schema_validator.rs:1447-1456`:
     ```rust
     warnings.push(ValidationWarning {
         code: DiagnosticCode::WarnHorizonSyntax,
         field_id: Some(name.clone()),
         message: format!("Field '{}' not found in alternative data field catalog", name),
         suggestion: Some("Verify field name against BRAIN /data-fields".to_string()),
     });
     ```
   - In `src/schema_validator.rs:1368`: Option9 trailing 'd' uses `ErrUnknownField`.
   - Execution command and verbatim output:
     ```bash
     cargo run -- fields --validate "group_neutralize(rank(completely_invented_fake_field_12345), densify(subindustry))"
     ```
     Output: `PASSED (0 errors, 1 warnings)`.

5. **Empty Expressions & Assignment-Only Scripts Pass**:
   - Executing `cargo run -- fields --validate ""` and `cargo run -- fields --validate "x = 1; y = 2;"` outputs `PASSED (0 errors)`.
   - Unbackfilled fields inside assignments (`x = fscore_growth;`) are not validated because line 1282 only inspects `AstStatement::Expression`.

6. **Price Normalization False Positive on Scalar 1.0**:
   - In `src/schema_validator.rs:1324-1330`: `AstExpr::Literal` initializes with `is_normalized_price: false`.
   - Executing `group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))` triggers `WarnUnnormalizedPrice` despite following the suggestion.

7. **Test Executions**:
   - `cargo test --test stress_schema_validator_tests`:
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

     test result: ok. 12 passed; 0 failed; finished in 0.02s
     ```
   - `bash scripts/sanitize_check.sh`:
     ```
     ✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.
     ```

---

## 2. Logic Chain

1. **Premise**: Under the project specification (R2 in `ORIGINAL_REQUEST.md` and `PROJECT.md`), the schema validator is a critical gatekeeper preventing invalid FastExpr expressions from reaching the BRAIN simulation API (`max_workers=3`), specifically enforcing vector folding, mandatory backfill for sparse fields (< 50% coverage), positive causal lookbacks, and syntactically valid operator calls.
2. **Inference from Observation 1**: Because `src/schema_validator.rs:1520` uses logical OR (`l.is_backfilled || r.is_backfilled`), combining a backfilled field with an unbackfilled sparse field poisons the entire node's `is_backfilled` flag to `true`. This causes line 1282 to skip `ErrMissingBackfill`, allowing unbackfilled sparse fields to pass into downstream generation and simulation pipelines.
3. **Inference from Observation 2**: Because `extract_literal_number` only handles literals and unary negation, any binary subtraction (e.g. `1 - 2`) or variable binding (`lag = -5`) returns `None`, causing line 1685 to silently fail open on look-ahead / negative lookbacks.
4. **Inference from Observation 3**: Functions like `ts_delay()`, `rank()`, and `densify()` lack arity validation, permitting 0-argument invocations that compile locally but fail immediately on the BRAIN platform with HTTP 400 Bad Request.
5. **Inference from Observation 4**: Inverting `ErrUnknownField` and `WarnHorizonSyntax` causes completely invalid or misspelled field identifiers to be treated as warnings rather than fatal errors.
6. **Inference from Observations 5-6**: Empty formulas and assignment-only scripts pass without generating a signal, while legitimate price-normalizing formulas (`price / close - 1.0`) trigger spurious warnings because scalars default to `is_normalized_price: false`.
7. **Conclusion**: The schema validator currently fails to provide fail-closed guarantees across these 6 identified vectors. Therefore, changes must be requested before Milestone M3 proceeds.

---

## 3. Caveats

- **WireMock and Catalog Coverage**: The offline fallback catalog correctly contains all 235 certified alternative fields across `model51`, `model16`, `news18`, and `option9`.
- **Parsing of Valid Complex Scripts**: The parser successfully processes valid multi-statement ASTs, bracket nestings, ternary and conditional logic (`trade_when`), and comments when well-formed.
- **Review Scope**: Challenger adhered strictly to the review-only constraint: no implementation code under `src/` was modified; all empirical challenges and reproducers were implemented in `tests/stress_schema_validator_tests.rs`.

---

## 4. Conclusion

**Verdict: REQUEST_CHANGES**

`src/schema_validator.rs` requires corrective remediation by `worker_m2_1` on:
1. **Backfill tracking**: Check `!low_coverage_origins.is_empty()` unconditionally at final expression, and use `l.is_backfilled && r.is_backfilled` for binary operations.
2. **Lookback evaluation**: Evaluate constant binary expressions in `extract_literal_number` and look up integer variables in `symbol_table`.
3. **Arity validation**: Enforce parameter counts for `ts_*`, `rank`, `densify`, and `vec_*`.
4. **Diagnostic code fix**: Unknown fields must emit `ErrUnknownField` (an error); option9 trailing 'd' should emit `WarnHorizonSyntax`.
5. **Script structure**: Reject empty expressions and scripts with no terminal `AstStatement::Expression`.
6. **Vector folding in backfill**: Disallow un-folded vectors in `ts_backfill`.
7. **Scalar normalization**: Set `is_normalized_price: true` for `AstExpr::Literal`.

---

## 5. Verification Method

To independently verify these findings:

1. **Run Challenger Stress Suite**:
   ```bash
   cargo test --test stress_schema_validator_tests
   ```
   *Expected*: All 12 tests pass, reproducing the vulnerability assertions.

2. **Run Direct CLI Verifications**:
   ```bash
   # Vulnerability 1 (Backfill Poisoning):
   cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"

   # Vulnerability 2 (Lookback Bypass):
   cargo run -- fields --validate "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))"

   # Vulnerability 3 (Missing Arity):
   cargo run -- fields --validate "group_neutralize(rank(ts_delay()), densify(subindustry))"

   # Vulnerability 4 (Unknown Field Fail-Open):
   cargo run -- fields --validate "group_neutralize(rank(completely_invented_fake_field_12345), densify(subindustry))"
   ```
   *Expected*: All incorrectly return `PASSED (0 errors)` in the current codebase.

3. **Verify Git Hygiene**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
   *Expected*: Audit passes cleanly with 0 leaks.
