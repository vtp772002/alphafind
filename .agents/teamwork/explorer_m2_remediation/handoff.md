# Handoff Report: Schema Validator Remediation Strategy (Milestone M2, Iteration 2)

**Author**: Schema Validator Remediation Strategist (`explorer_m2_remediation`)  
**Target Recipient**: Worker (`worker_m2_1`), Orchestrator, Reviewer 2, Challenger 1  
**Working Directory**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_remediation/`  
**Date**: 2026-10-24T20:25:00Z  

---

## 1. Observation

Direct code analysis and test execution revealed the following concrete issues across `src/schema_validator.rs`, `scratch/metadata_tool.py`, and `tests/stress_schema_validator_tests.rs`:

1. **Backfill Poisoning Bypass (`src/schema_validator.rs:1520` and `1282`)**:
   - In `eval_expr` for `AstExpr::Binary`:
     ```rust
     1520: is_backfilled: l.is_backfilled || r.is_backfilled,
     ```
   - In `validate`:
     ```rust
     1282: if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled {
     ```
   - Verbatim test in `tests/stress_schema_validator_tests.rs:167`:
     ```rust
     let expr = "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))";
     let rep = validator.validate(expr);
     assert!(rep.is_valid); // Passes with 0 errors despite fscore_growth coverage = 29.8%
     ```

2. **Lookback Parameter Arithmetic Bypass (`src/schema_validator.rs:1618` and `1685`)**:
   - In `extract_literal_number`:
     ```rust
     1618: fn extract_literal_number(expr: &AstExpr) -> Option<f64> {
     1619:     match expr {
     1620:         AstExpr::Literal(n) => Some(*n),
     1621:         AstExpr::Unary { op, expr } if op == "-" => extract_literal_number(expr).map(|n| -n),
     1622:         _ => None,
     1623:     }
     1624: }
     ```
   - For `ts_delay(close, 1 - 2)` or `lag = -5; ts_delay(close, lag)`, `extract_literal_number` returns `None`. At line 1685, `if let Some(lag) = extract_literal_number(&args[1])` fails to match, silently bypassing the `lag <= 0.0` check.

3. **Price Normalization Inversion on Literals (`src/schema_validator.rs:1329`)**:
   - `AstExpr::Literal` initializes `is_normalized_price: false`.
   - In `call_breakeven_30 / close - 1.0`, line 1512 calculates `l.is_normalized_price && r.is_normalized_price`, which evaluates to `true && false = false`.
   - Line 1721 warns: `WARN_UNNORMALIZED_PRICE: Raw price or breakeven level passed to rank() without close normalization`, contradicting the system's own suggestion.

4. **Python Scratch Tool Regex Breakdown (`scratch/metadata_tool.py:263`)**:
   - `re.findall(r"group_neutralize\s*\([^,]+,\s*([^)]+)\)", expr)` splits on the first comma inside `ts_decay_linear(rank(close), 5)`, treating `5` as the grouping argument.

5. **Missing Function Arity Checks and Assignment-Only Empty Script Handling (`src/schema_validator.rs:1240–1297`)**:
   - `validate("")`, `validate("# comment")`, and `validate("x = 1; y = 2;")` produce 0 errors, returning `is_valid: true`.
   - `ts_delay()`, `rank()`, and `densify()` accept 0 arguments and return matrix types with 0 errors.
   - Line 1450 pushes `ValidationWarning { code: DiagnosticCode::WarnHorizonSyntax, ... }` for unknown fields, returning `FastExprType::Matrix`, failing open on misspelled/fake fields.

---

## 2. Logic Chain

1. **Backfill Tracking Logic**:
   - Observation 1 proves that `is_backfilled` boolean flag poisoning bypasses `ErrMissingBackfill`.
   - `ts_backfill(sig, d >= 5)` clears `low_coverage_origins = Vec::new()`.
   - Therefore, any field present in `low_coverage_origins` at the terminal expression was never backfilled.
   - Condition line 1282 strictly on `!final_meta.low_coverage_origins.is_empty()` and set `is_backfilled: l.is_backfilled && r.is_backfilled` in binary nodes. This guarantees that unbackfilled sparse fields will always trigger `ErrMissingBackfill`.

2. **Lookback Resolution Logic**:
   - Observation 2 proves that binary arithmetic expressions and variables evaluate to `None` in `extract_literal_number`.
   - FastExpr lookbacks must be positive integers ($d \ge 1$).
   - By adding `scalar_value: Option<f64>` to `ValueMeta` and extending `extract_literal_number` (or `extract_constant_number`) to evaluate recursive binary operations (`+`, `-`, `*`, `/`) and symbol table variables, `1 - 2` evaluates to `-1.0`, and `lag = -5` evaluates to `-5.0`.
   - Both strictly trigger `ErrNegativeLookback`. Any non-scalar lag expression triggers `ErrNegativeLookback`/`ErrSyntaxMalformed`.

3. **Price Normalization Dimensionality Logic**:
   - Observation 3 proves that numeric literals (`1.0`, `0.5`, `2.0`) currently carry `is_normalized_price: false`.
   - Scalars and offsets are dimensionless constants, not asset-specific price levels.
   - Setting `is_normalized_price: true` for `AstExpr::Literal` and `AstExpr::StringLit` ensures that normalized price ratios like `(call_breakeven_30 / close) - 1.0` evaluate to `true && true = true`, eliminating false positive warnings while raw prices like `call_breakeven_30 - 1.0` evaluate to `false && true = false`, preserving true warning detection.

4. **Regex vs. AST Argument Splitting Logic**:
   - Observation 4 proves that regexes without bracket counters cannot distinguish between commas in nested argument lists and top-level commas.
   - Implementing parenthesis-aware bracket matching `extract_call_args` isolates the true top-level arguments, resolving the issue.

5. **Arity and Script Completeness Logic**:
   - Observation 5 proves that empty scripts and 0-argument function calls fail open.
   - FastExpr alpha scripts require non-empty statements ending in an executable terminal expression.
   - Function signatures on BRAIN have fixed arities (`densify`: 1, `rank`: 1, `vec_*`: 1, `group_neutralize`: 2, `ts_backfill`: 2, `ts_*`: $\ge 2$). Enforcing these counts stops invalid expressions from passing local validation.

---

## 3. Caveats

- **Ternary Operator Syntax**: The current FastExpr parser tokenizes `?` and `:` but does not implement ternary grammar productions. The official BRAIN platform syntax for conditional expressions is `if_else(cond, true_val, false_val)`. We recommend documenting `if_else(...)` as the supported conditional syntax and validating its arity as 3 arguments.
- **Dynamic Lags**: BRAIN FastExpr does not support non-constant time-series lags (e.g. `ts_delay(close, volume)`). Our design enforces that lookback arguments evaluate to statically resolvable scalar constants.

---

## 4. Conclusion

The schema validator failure modes identified by Reviewer 2 and Challenger 1 are completely resolvable through targeted algorithmic fixes in `src/schema_validator.rs`, `scratch/metadata_tool.py`, and `tests/stress_schema_validator_tests.rs`.
The complete, line-by-line patch instructions and test specifications have been codified in:
`/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_remediation/remediation_plan.md`.

Once the Worker applies these specifications:
- Backfill bypass is eliminated.
- Lookback arithmetic bypass is closed.
- Price normalization false positives are eliminated.
- Python tool nested commas parse correctly.
- Arity and script emptiness are strictly enforced.
- Unknown fields fail closed with `ErrUnknownField`.

---

## 5. Verification Method

The Worker can independently verify the implementation with the following commands:

1. **Execute Stress Schema Validator Tests**:
   ```bash
   cargo test --test stress_schema_validator_tests
   ```
   *Expected*: All 12 adversarial test cases pass (asserting fail-closed rejections).

2. **Execute Metadata Schema Unit Tests**:
   ```bash
   cargo test --test metadata_schema_tests
   ```
   *Expected*: All 21 unit and mock API tests pass with 0 regressions.

3. **Execute Entire Crate Test Suite**:
   ```bash
   cargo test
   ```
   *Expected*: 139+ tests pass across models, reporting, shadow, stress, submission, and taxonomy suites.

4. **Verify Python Metadata Tool**:
   ```bash
   python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"
   ```
   *Expected*: `Validation: PASSED`.

5. **Verify CLI Validation**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
   ```
   *Expected*: `VALIDATION FAILED` with `[ERR_MISSING_BACKFILL]`.

6. **Pre-Push Sanitizer Audit**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
   *Expected*: Pre-Push Audit PASSED with 0 confidential data leaks.
