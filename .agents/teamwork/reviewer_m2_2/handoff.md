# Handoff Report: Reviewer 2 (Schema Validator & CLI Reviewer) for Milestone M2

**Author**: Reviewer 2 (`reviewer_m2_2`)  
**Parent Agent**: Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)  
**Date**: 2026-10-24T20:00:00Z  
**Type**: Hard Handoff (Review Complete)  
**Verdict**: **REQUEST_CHANGES**  

---

## 1. Observation

Direct observations from source inspection, compiler execution, test runners, and fuzzing:

1. **Test Suite Execution**:
   - `cargo check`:
     ```
     Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
     ```
   - `cargo test --test metadata_schema_tests`:
     ```
     test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     ```
   - `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json cargo test`:
     ```
     test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     ```
   - `bash scripts/sanitize_check.sh`:
     ```
     🔍 [SANITIZATION GATEKEEPER] Running Pre-Push Audit...
        ✓ Commit-history scan: no Alpha IDs in added lines.
        ✓ Dataset-field scan: all tracked files are clear of configured field patterns.
        ✓ Candidate artifact scan: 12 private literals absent from tracked files.
     ✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.
     ```

2. **Source Code Observations in `src/schema_validator.rs`**:
   - Line 1520: In binary expressions, `is_backfilled` is evaluated as:
     ```rust
     is_backfilled: l.is_backfilled || r.is_backfilled,
     ```
   - Lines 1282–1294: The statement-level check checks:
     ```rust
     if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled {
         for fid in &final_meta.low_coverage_origins {
             errors.push(ValidationError { code: DiagnosticCode::ErrMissingBackfill, ... });
         }
     }
     ```
   - Lines 1618–1624: `extract_literal_number` only matches literals and unary negation:
     ```rust
     fn extract_literal_number(expr: &AstExpr) -> Option<f64> {
         match expr {
             AstExpr::Literal(n) => Some(*n),
             AstExpr::Unary { op, expr } if op == "-" => extract_literal_number(expr).map(|n| -n),
             _ => None,
         }
     }
     ```
   - Line 1329: `AstExpr::Literal(_)` sets `is_normalized_price: false`.
   - Line 1450: Uncataloged fields push `DiagnosticCode::WarnHorizonSyntax` instead of `WarnUnknownField`.
   - Line 798 vs Lines 963–1117: `?` and `:` are tokenized in `tokenize()`, but `Parser` has no rule parsing ternary operators.

3. **CLI & Empirical Fuzzing Invocations**:
   - Command: `cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_bfl_growth, 15) + fscore_momentum), densify(subindustry))"`
     - *Result*: Exited 0 with **PASSED (0 errors)** despite `fscore_momentum` (29.7% coverage) being completely unbackfilled.
   - Command: `cargo run -- fields --validate "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))"`
     - *Result*: Exited 0 with **PASSED (0 errors)** despite lookback window being $-1$ ($1 - 2$).
   - Command: `cargo run -- fields --validate "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))"`
     - *Result*: Exited 0 but emitted `WARNING: [WARN_UNNORMALIZED_PRICE] Raw price or breakeven level passed to rank() without close normalization` despite `call_breakeven_30 / close` being normalized.
   - Command: `cargo run -- fields --validate "group_neutralize(rank(ts_delay()), densify(subindustry))"`
     - *Result*: Exited 0 with **PASSED (0 errors)** despite 0 arguments to `ts_delay`.
   - Command: `python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"`
     - *Result*: Exited 1 with `ERROR: group_neutralize grouping argument '5' must be wrapped in densify(...)`.

4. **Catalog & Invariants Verification**:
   - `cargo run -- fields --summary` shows exactly 235 fields across 4 pillars: `model51` (16), `model16` (24), `news18` (121), `option9` (74).
   - `cargo run -- fields --check-coverage` flags all 16 sparse fields (8 raw + 8 backfilled pentagons).

---

## 2. Logic Chain

1. **Integrity Verification**:
   - Based on Observation 1 and code review, all unit tests execute genuine AST evaluations and WireMock HTTP requests. No hardcoded results, facades, or secret data leaks were detected.
2. **Backfill Poisoning Vulnerability**:
   - Based on Observation 2 (lines 1520 & 1282) and Observation 3, combining a backfilled field with an unbackfilled sparse field sets `parent.is_backfilled = true`.
   - When the statement terminates, `!final_meta.is_backfilled` evaluates to `false`, silently suppressing `ErrMissingBackfill`.
   - Therefore, the requirement that low-coverage fields MUST be backfilled is bypassed whenever blended with another backfilled field.
3. **Causal Lookback Bypass**:
   - Based on Observation 2 (lines 1618–1624) and Observation 3, passing an arithmetic binary expression (e.g. `1 - 2`) causes `extract_literal_number` to return `None`.
   - The lag check `if lag <= 0.0` is never invoked, allowing non-causal lookbacks to pass validation.
4. **Diagnostic False Positives**:
   - Based on Observation 2 (line 1329) and Observation 3, setting `is_normalized_price = false` on `AstExpr::Literal` causes unitless constants (`1.0`) to taint normalized price ratios (`call_breakeven_30 / close - 1.0`), emitting incorrect warnings.
5. **Tooling & Parser Incompleteness**:
   - Based on Observation 3, `scratch/metadata_tool.py` fails on nested commas, and `SchemaValidator` accepts 0-argument calls (`ts_delay()`, `densify()`) and fails to parse claimed ternary expressions.
6. **Verdict Deduction**:
   - Because these vulnerabilities compromise fail-closed validation of factor expressions that will be generated in Milestone M3, changes must be requested to correct the validator before factor combination begins.

---

## 3. Caveats

1. **Independent Verification**: All vulnerabilities were reproduced both via direct CLI execution and automated stress tests in `tests/stress_schema_validator_tests.rs`.
2. **Impact on M3**: The vulnerabilities do not affect happy-path single-factor alphas, but pose material risk for multi-factor combinatorial hypotheses generated in M3.
3. **Remediation Size**: The required fixes in `src/schema_validator.rs` are small and well-scoped (<15 lines of code).

---

## 4. Conclusion

**Verdict**: **REQUEST_CHANGES**

Milestone M2 is fundamentally well-engineered with complete models, robust pagination, and 235 certified fields. However, approval is conditioned on remediating the following items in `src/schema_validator.rs`:
1. Remove `&& !final_meta.is_backfilled` condition at line 1282; trigger `ErrMissingBackfill` whenever `!final_meta.low_coverage_origins.is_empty()`.
2. Update binary backfill propagation at line 1520 to `is_backfilled: l.is_backfilled && r.is_backfilled`.
3. Set `is_normalized_price: true` for `AstExpr::Literal` and `AstExpr::StringLit` at lines 1329 and 1336.
4. Extend `extract_literal_number` to evaluate constant binary arithmetic or reject non-constant lag expressions.
5. Add arity enforcement for `ts_delay` (2), `densify` (1), and `rank` (1).

---

## 5. Verification Method

To independently reproduce the findings and verify future fixes:

1. **Reproduce Backfill Poisoning Bypass**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_bfl_growth, 15) + fscore_momentum), densify(subindustry))"
   ```
   *Expected After Fix*: Fails with `[ERR_MISSING_BACKFILL] Field 'fscore_momentum' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill`.

2. **Reproduce Non-Causal Lookback Bypass**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))"
   ```
   *Expected After Fix*: Fails with `[ERR_NEGATIVE_LOOKBACK] Operator 'ts_delay' received non-positive lookback lag`.

3. **Reproduce Price Normalization Warning False-Positive**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))"
   ```
   *Expected After Fix*: Passes with 0 warnings for `WARN_UNNORMALIZED_PRICE`.

4. **Verify Test Suites & Sanitizer**:
   ```bash
   cargo test --test metadata_schema_tests
   cargo test --test stress_schema_validator_tests
   bash scripts/sanitize_check.sh
   ```
   *Expected*: All tests pass, and Git pre-push sanitization passes with 0 leaks.
