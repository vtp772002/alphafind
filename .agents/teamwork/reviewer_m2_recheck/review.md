# Comprehensive Quality & Adversarial Review Report: Milestone M2 Remediation (Iteration 2)

**Author**: Independent Reviewer & Adversarial Critic (`reviewer_m2_recheck`)  
**Target Milestone**: M2 Remediation Recheck (Iteration 2)  
**Target Files**: `src/schema_validator.rs`, `scratch/metadata_tool.py`, `tests/metadata_schema_tests.rs`, `tests/stress_schema_validator_tests.rs`  
**Date**: 2026-10-24T20:24:00Z  
**Verdict**: **APPROVE**

---

## 1. Executive Summary

Milestone M2 Remediation (Iteration 2) has successfully resolved all 5 fail-open algorithmic defects identified during previous adversarial testing. The implementations in `src/schema_validator.rs` and `scratch/metadata_tool.py` were independently audited and stress-tested with adversarial edge cases. No integrity violations, shortcuts, facades, or test hardcoding were detected. All verification suites, including compilation, full integration tests, adversarial stress tests, and pre-push sanitization checks, pass cleanly.

---

## 2. In-Depth Re-Evaluation of Remediated Defects

### 2.1 Backfill Poisoning in Binary Operations
- **Defect Description**: Previously, binary expressions combined `is_backfilled` via logical OR (`l.is_backfilled || r.is_backfilled`), allowing an unbackfilled sparse field (e.g., `fscore_growth` with 29.8% coverage) to be shielded by a backfilled sibling (e.g., `ts_backfill(fscore_momentum, 15)`).
- **Remediation Inspection**:
  - In `src/schema_validator.rs:1602-1608`, binary expressions compute:
    ```rust
    let l_non_sparse = l.is_backfilled || l.low_coverage_origins.is_empty();
    let r_non_sparse = r.is_backfilled || r.low_coverage_origins.is_empty();
    let is_bfl = l_non_sparse && r_non_sparse && (l.is_backfilled || r.is_backfilled);

    let mut combined_origins = l.low_coverage_origins;
    combined_origins.extend(r.low_coverage_origins);
    ```
  - In statement root evaluation (`src/schema_validator.rs:1341-1353`), `if !final_meta.low_coverage_origins.is_empty()` directly inspects the multi-origin tracker, emitting `ErrMissingBackfill` for every unbackfilled sparse field regardless of `is_backfilled`.
  - In `ts_backfill` (`src/schema_validator.rs:1757-1764`), `sig_meta.expr_type == FastExprType::Vector` is rejected with `ErrVecTypeMismatch`.
- **Stress-Test Verification**:
  - Formula: `group_neutralize(rank((ts_backfill(fscore_momentum, 15) * 2.0) + (fscore_growth * 0.5)), densify(subindustry))`
  - Result: FAILED with `[ERR_MISSING_BACKFILL] Field 'fscore_growth' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill(..., d >= 5)`. Poisoning bypass is completely eliminated.

### 2.2 Lookback Parameter Constant Folding
- **Defect Description**: Previously, non-literal arithmetic (e.g., `1 - 2`) or variable references returned `None` from `extract_literal_number`, silently skipping `lag <= 0.0` checks and permitting negative lookbacks.
- **Remediation Inspection**:
  - Implemented `extract_constant_number(&AstExpr, &HashMap<String, ValueMeta>) -> Option<f64>` (`src/schema_validator.rs:1147-1174`). Recursively evaluates literals, unary (`-`, `+`), binary arithmetic (`+`, `-`, `*`, `/` with zero division protection `rv.abs() > 1e-9`), and variable scalar bindings.
  - Added `scalar_value: Option<f64>` to `ValueMeta`.
  - In `ts_*` handlers (`src/schema_validator.rs:1826-1848`), resolved lags $\le 0.0$ trigger `ErrNegativeLookback`. If unresolved, non-scalar expressions fail closed.
  - In `ts_backfill` (`src/schema_validator.rs:1766-1787`), window parameters that do not statically resolve to positive integers strictly fail closed with `ErrNegativeLookback`.
- **Stress-Test Verification**:
  - Formula 1: `ts_delay(close, 10 - (2 * 5))` (evaluates to 0) -> FAILED with `[ERR_NEGATIVE_LOOKBACK] Operator 'ts_delay' received non-positive lookback lag (0)`.
  - Formula 2: `ts_delay(close, -(3 - 1))` (evaluates to -2) -> FAILED with `[ERR_NEGATIVE_LOOKBACK] Operator 'ts_delay' received non-positive lookback lag (-2)`.
  - Formula 3: `lag = -5; ts_delay(close, lag)` -> FAILED with `[ERR_NEGATIVE_LOOKBACK]`.

### 2.3 Price Normalization and Numeric Literals
- **Defect Description**: `AstExpr::Literal` and `AstExpr::StringLit` initialized `is_normalized_price: false`. Subtracting `1.0` from a normalized ratio (e.g. `call_breakeven_30 / close - 1.0`) broke normalization via `true && false = false`, emitting false-positive `WarnUnnormalizedPrice`.
- **Remediation Inspection**:
  - In `src/schema_validator.rs:1388, 1396, 1140`, literal constants and defaults initialize `is_normalized_price: true` and `scalar_value: Some(*n)`.
  - Raw price fields (`call_breakeven_*`, `forward_price_*`, `option_breakeven_*`) initialize `is_normalized_price: false`.
  - Dividing by `close` sets `is_normalized_price: true`.
  - Dimensionless scalar operations preserve `is_normalized_price: true && true = true`.
- **Stress-Test Verification**:
  - Formula 1: `group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))` -> PASSED (0 errors, 0 warnings).
  - Formula 2: `group_neutralize(rank((call_breakeven_30 / close - 1.0) * 100.0), densify(subindustry))` -> PASSED (0 errors, 0 warnings).
  - Formula 3: `group_neutralize(rank(call_breakeven_30 * 100.0), densify(subindustry))` -> PASSED with expected `WARNING: [WARN_UNNORMALIZED_PRICE] Raw price or breakeven level passed to rank() without close normalization`.

### 2.4 Scratch Tool Nested Function Argument Splitting
- **Defect Description**: In `scratch/metadata_tool.py`, regex `re.findall(r"group_neutralize\s*\([^,]+,\s*([^)]+)\)", expr)` split on commas inside inner functions (e.g., `ts_decay_linear(rank(close), 5)`), erroneously extracting `5` instead of `densify(subindustry)`.
- **Remediation Inspection**:
  - Implemented `extract_call_args(text: str, func_name: str) -> list[list[str]]` (`scratch/metadata_tool.py:249-279`).
  - Tracks parenthesis nesting depth (`depth += 1` on `(`, `depth -= 1` on `)`), only splitting arguments when `c == ','` and `depth == 1`.
  - Properly extracts all arguments for nested expressions.
- **Stress-Test Verification**:
  - Formula 1: `python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"` -> PASSED (0 errors).
  - Formula 2: `python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), subindustry)"` -> FAILED (1 error: `ERROR: group_neutralize grouping argument 'subindustry' must be wrapped in densify(...)`).

### 2.5 Function Arity, Script Structure, and Unknown Field Diagnostics
- **Defect Description**: Functions accepted 0 arguments, empty or assignment-only scripts passed without terminal expressions, and uncataloged fields emitted `WarnHorizonSyntax` instead of failing validation.
- **Remediation Inspection**:
  - `src/schema_validator.rs:1292-1316`: Empty statements emit `ErrSyntaxMalformed`. Scripts without terminal expressions emit `ErrSyntaxMalformed`.
  - Function arities strictly validated:
    - `vec_*`: exactly 1 argument.
    - `densify`: exactly 1 argument.
    - `group_neutralize`: exactly 2 arguments.
    - `ts_backfill`: exactly 2 arguments.
    - `ts_*`: at least 2 arguments.
    - `rank`: exactly 1 argument.
    - Default function handler: at least 1 argument.
  - Uncataloged field identifiers (`src/schema_validator.rs:1532-1542`) push `ValidationError` with `DiagnosticCode::ErrUnknownField` and set `FastExprType::TypeError`.
- **Stress-Test Verification**:
  - `densify()` -> FAILED with `[ERR_SYNTAX_MALFORMED] 'densify' requires exactly 1 argument (group), found 0`.
  - `group_neutralize(rank(close))` -> FAILED with `[ERR_SYNTAX_MALFORMED] group_neutralize requires exactly 2 arguments: (signal, densify(group)), found 1`.
  - `rank()` -> FAILED with `[ERR_SYNTAX_MALFORMED] 'rank' requires exactly 1 argument (signal), found 0`.
  - `alpha = rank(close);` -> FAILED with `[ERR_SYNTAX_MALFORMED] Script must end with a terminal alpha expression, not an assignment`.
  - `""` -> FAILED with `[ERR_SYNTAX_MALFORMED] Expression is empty; expected a valid FastExpr formula`.
  - `group_neutralize(rank(invented_random_field), densify(subindustry))` -> FAILED with `[ERR_UNKNOWN_FIELD] Field 'invented_random_field' not found in alternative data field catalog`.

---

## 3. Adversarial Stress-Testing & Integrity Audit

### 3.1 Integrity Audit
- **Hardcoded test results**: None. No string-matching on specific test expressions or hardcoded mock returns in `schema_validator.rs`.
- **Facade implementations**: None. The AST tokenizer, parser, constant folder, and type lattice evaluator perform genuine recursive analysis on arbitrary inputs.
- **Task shortcuts**: None. Both Rust production validator and Python exploratory tool have complete, independent implementations.
- **Self-certifying work**: All claims were independently tested and reproduced via CLI commands, test suites, and counter-example expressions.

### 3.2 Verification Matrix
| Test Command | Expected Result | Actual Result | Status |
|---|---|---|---|
| `cargo check` | Compile cleanly (code 0) | Finished dev profile in 0.23s | PASS |
| `cargo test --test metadata_schema_tests` | 25 passed; 0 failed | 25 passed; 0 failed (0.09s) | PASS |
| `cargo test --test stress_schema_validator_tests` | 12 passed; 0 failed | 12 passed; 0 failed (0.02s) | PASS |
| `cargo test` (Entire workspace) | 143+ passed; 0 failed | 143+ passed; 0 failed (0.24s) | PASS |
| `bash scripts/sanitize_check.sh` | Pre-push audit PASSED | Pre-Push Audit PASSED | PASS |
| `scratch/metadata_tool.py --validate` | Proper argument parsing | Passed nested, caught invalid | PASS |

---

## 4. Findings Summary

- **Critical**: 0
- **Major**: 0
- **Minor**: 0
- **Integrity Violations**: 0

---

## 5. Verdict

**APPROVE**

Milestone M2 Remediation satisfies all quantitative standards, platform rules, and adversarial resilience criteria. The codebase is ready to proceed to Milestone M3 (Automated Expression Builder & Factor Combinator).
