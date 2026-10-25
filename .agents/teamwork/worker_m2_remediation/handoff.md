# Handoff Report: Milestone M2 Remediation (Iteration 2)

**Author**: Quantitative Developer Worker (`worker_m2_remediation`)  
**Role**: implementer, qa, specialist  
**Date**: 2026-10-24T20:18:45Z  
**Recipient**: Lead Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)  
**Status**: REMEDIATION COMPLETE & VERIFIED  

---

## 1. Observation

Adversarial testing and review by Challenger 1 (`challenger_m2_1`) and Reviewer 2 (`reviewer_m2_2`) identified 5 distinct fail-open algorithmic defects in `src/schema_validator.rs` and `scratch/metadata_tool.py`:

1. **Backfill Poisoning Bypass in Binary Operations**:
   - In `src/schema_validator.rs` (previously lines 1520 and 1282), binary expressions computed `is_backfilled` as `l.is_backfilled || r.is_backfilled`.
   - In the root statement post-check, `if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled` was evaluated. When operand `l` was backfilled (e.g., `ts_backfill(fscore_momentum, 15)`), `final_meta.is_backfilled` became `true`, completely suppressing `ErrMissingBackfill` for unbackfilled low-coverage fields in operand `r` (e.g., `fscore_growth` with 29.8% coverage).
   - Direct reproducer command:
     ```bash
     cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
     ```
     Initially produced `✅ FastExpr Schema Validation: PASSED (0 errors)`.

2. **Lookback Parameter Bypass via Arithmetic Expressions & Variables**:
   - `extract_literal_number` only matched `AstExpr::Literal(n)` and `AstExpr::Unary { op: "-", expr }`. Expressions involving binary arithmetic (e.g., `1 - 2`) or variable references returned `None`.
   - The lag check `if let Some(lag) = extract_literal_number(&args[1]) { if lag <= 0.0 { ... } }` was silently skipped when `None` was returned, allowing negative lookbacks like `ts_delay(close, 1 - 2)` to pass validation with 0 errors.

3. **Literal Scalars Inverting Price Normalization**:
   - `AstExpr::Literal` and `AstExpr::StringLit` were initialized with `is_normalized_price: false`.
   - When subtracting `1.0` from a normalized ratio (e.g., `call_breakeven_30 / close - 1.0`), the binary subtraction evaluated `true && false = false`, erroneously classifying the normalized ratio as an unnormalized price level and triggering a false-positive `WarnUnnormalizedPrice`.

4. **Python Scratch Tool Regex Breakdown on Nested Commas**:
   - In `scratch/metadata_tool.py:263`, `re.findall(r"group_neutralize\s*\([^,]+,\s*([^)]+)\)", expr)` split on the first comma inside `group_neutralize(...)`.
   - In expressions with nested function calls like `group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))`, the regex extracted `5` instead of `densify(subindustry)`, failing with:
     `ERROR: group_neutralize grouping argument '5' must be wrapped in densify(...).`

5. **Missing Function Arity Checks, Empty Scripts & Inverted Diagnostics**:
   - Functions `densify()`, `rank()`, `ts_delay()`, `vec_avg()` accepted 0 arguments without error.
   - Scripts with no statements (`""`, `"# comment"`) or only assignments (`"x = 1; y = 2;"`) evaluated as valid alphas without a terminal expression.
   - Unknown field identifiers pushed `ValidationWarning` with `WarnHorizonSyntax` instead of `ValidationError` with `ErrUnknownField`.
   - `ts_backfill` accepted unfolded `VECTOR` arguments without type error.

---

## 2. Logic Chain

The remediation addressed each defect systematically at its root cause without regressions:

1. **Fixing Backfill Poisoning & Vector Backfill**:
   - `low_coverage_origins` is the multi-origin accumulator tracking fields that require `ts_backfill(..., d >= 5)`. Wrapping a sparse field in `ts_backfill(..., d >= 5)` clears `low_coverage_origins`.
   - Therefore, at the root statement check in `src/schema_validator.rs`, `if !final_meta.low_coverage_origins.is_empty()` is evaluated directly (removing `&& !final_meta.is_backfilled`), ensuring any unbackfilled sparse field emits `ErrMissingBackfill`.
   - In `AstExpr::Binary`, `is_backfilled` is computed as:
     `l_non_sparse && r_non_sparse && (l.is_backfilled || r.is_backfilled)`
     where `l_non_sparse = l.is_backfilled || l.low_coverage_origins.is_empty()`. An expression is backfilled only if no unbackfilled sparse field exists in either subtree and at least one operand was backfilled.
   - In `ts_backfill`, if `sig_meta.expr_type == FastExprType::Vector`, `ValidationError` with `DiagnosticCode::ErrVecTypeMismatch` is emitted immediately.

2. **Fixing Lookback Parameter Constant Folding**:
   - Added `scalar_value: Option<f64>` to `ValueMeta`.
   - Implemented `extract_constant_number(expr: &AstExpr, symbol_table: &HashMap<String, ValueMeta>) -> Option<f64>` which evaluates constants across literals, unary negation/plus, binary arithmetic (`+`, `-`, `*`, `/`), and variable bindings in `symbol_table`.
   - In `ts_backfill` and `ts_*` handlers:
     - If `extract_constant_number` resolves to `Some(lag)` and `lag <= 0.0`, `ErrNegativeLookback` is emitted.
     - For `ts_backfill`, if the window parameter cannot be statically resolved to a positive integer, it fails closed with `ErrNegativeLookback`.
     - For `ts_*`, if the lag cannot be statically resolved and its evaluated type is not `Scalar`, it fails closed with `ErrNegativeLookback`.

3. **Fixing Price Normalization False Positives**:
   - Literal numeric constants (`AstExpr::Literal(n)`), string literals (`AstExpr::StringLit`), and `ValueMeta::default()` initialize `is_normalized_price: true` and `scalar_value: Some(*n)`.
   - Unitless scalar numbers are dimensionally neutral. In `call_breakeven_30 / close - 1.0`, `l.is_normalized_price && r.is_normalized_price` evaluates to `true && true = true`, eliminating false `WarnUnnormalizedPrice` warnings while continuing to catch raw price levels like `rank(call_breakeven_30)`.

4. **Fixing Python Scratch Tool Argument Extraction**:
   - Added `extract_call_args(text: str, func_name: str) -> list[list[str]]` in `scratch/metadata_tool.py`.
   - The parser tracks parenthesis nesting depth (`depth += 1` on `(`, `depth -= 1` on `)`), only splitting arguments on commas at `depth == 1`.
   - Top-level arguments for `group_neutralize(...)` are correctly extracted even when inner functions contain commas (e.g. `ts_decay_linear(rank(close), 5)`).

5. **Fixing Script Structure, Function Arity & Unknown Field Diagnostics**:
   - In `src/schema_validator.rs`:
     - Empty statement lists emit `ErrSyntaxMalformed` ("Expression is empty; expected a valid FastExpr formula").
     - Scripts lacking a terminal expression emit `ErrSyntaxMalformed` ("Script must end with a terminal alpha expression, not an assignment").
     - Strict arity checks:
       - `vec_*`: exactly 1 argument (`ErrSyntaxMalformed` if `args.len() != 1`).
       - `densify`: exactly 1 argument (`ErrSyntaxMalformed` if `args.len() != 1`). Evaluates `args[0]` to inspect inner tokens.
       - `group_neutralize`: exactly 2 arguments (`ErrSyntaxMalformed` if `args.len() != 2`).
       - `ts_backfill`: exactly 2 arguments (`ErrSyntaxMalformed` if `args.len() != 2`).
       - `ts_*`: at least 2 arguments (`ErrSyntaxMalformed` if `args.len() < 2`).
       - `rank`: exactly 1 argument (`ErrSyntaxMalformed` if `args.len() != 1`).
       - Default function call handler: requires at least 1 argument (`ErrSyntaxMalformed` if `args.is_empty()`).
     - Uncataloged identifiers not matching `true`, `false`, `nan`, or group keywords emit `ValidationError` with `DiagnosticCode::ErrUnknownField` and return `FastExprType::TypeError`.

---

## 3. Caveats

- **No Caveats**: All 5 identified issues have been resolved with genuine AST parsing and type evaluation logic.
- Constant folding supports standard arithmetic operations (`+`, `-`, `*`, `/`) and variable symbol table lookups; non-constant, data-dependent lookback windows (e.g. `ts_delay(close, close)`) fail closed as required by platform causality rules.

---

## 4. Conclusion

The Schema Validator Engine (`src/schema_validator.rs`) and Python validation script (`scratch/metadata_tool.py`) are now strictly fail-closed, robust against nested syntax structures, and fully compliant with Milestone M2 specifications:
- Unbackfilled low-coverage fields cannot bypass validation through binary combinations.
- Non-causal negative or zero lookback lags wrapped in arithmetic expressions or variables are strictly caught.
- Unitless scalar literals no longer trigger price normalization warnings.
- Function arity is strictly enforced across all built-in operators.
- Empty and assignment-only scripts are rejected with malformed syntax errors.
- Uncataloged field identifiers fail closed as hard errors.
- Pre-push Git hygiene audit passes cleanly with 0 confidential data leaks.

---

## 5. Verification Method

To independently verify all remediation changes, execute the following commands in `/Users/phamtoan/Developer/alphafind`:

1. **Adversarial Stress Test Suite**:
   ```bash
   cargo test --test stress_schema_validator_tests
   ```
   *Expected Output*: `12 passed; 0 failed` (all 12 adversarial test cases assert error detection and fail-closed validation).

2. **Integration Test Suite**:
   ```bash
   cargo test --test metadata_schema_tests
   ```
   *Expected Output*: `25 passed; 0 failed` (including all 4 new Suite E remediation invariant tests).

3. **Complete Crate Test Suite**:
   ```bash
   cargo test
   ```
   *Expected Output*: `143+ passed; 0 failed; 0 ignored` across all crates and integration test suites.

4. **Python Scratch Tool Validation**:
   ```bash
   python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"
   ```
   *Expected Output*: `✅ Schema Validation PASSED (0 errors)`

5. **CLI Integration Test (Backfill Poisoning Prevention)**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
   ```
   *Expected Output*: `❌ FastExpr Schema Validation: FAILED (1 errors)` with `[ERR_MISSING_BACKFILL] Field 'fscore_growth' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill(..., d >= 5)`.

6. **CLI Integration Test (Price Normalization Preservation)**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))"
   ```
   *Expected Output*: `✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)`

7. **Pre-Push Sanitization Audit**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
   *Expected Output*: `✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.`
