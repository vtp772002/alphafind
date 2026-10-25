# Quality & Adversarial Review Report: Milestone M2 (Field Metadata Extraction & Schema Validator)

**Reviewer**: Reviewer 2 (`reviewer_m2_2`) — Schema Validator & CLI Reviewer  
**Roles**: Reviewer, Adversarial Critic  
**Date**: 2026-10-24T19:59:00Z  
**Verdict**: **REQUEST_CHANGES**  
**Overall Risk Assessment**: **HIGH**  

---

## 1. Review Summary

The quantitative developer worker (`worker_m2_1`) delivered a substantial implementation for Milestone M2:
- An AST-aware `SchemaValidator` with a 6-type lattice (`Scalar`, `Matrix`, `Vector`, `Group`, `DensifiedGroup`, `BooleanMatrix`).
- A built-in offline fallback catalog for all 235 certified alternative data fields across `model51` (16), `model16` (24), `news18` (121), and `option9` (74).
- WireMock-backed pagination (50 items/page), rate-limiting exponential backoff, and strict parameter enforcement (`instrumentType=EQUITY`, `delay=1`, `universe=TOP1000`) in `src/client.rs`.
- CLI integration via `alphafind fields` (`--summary`, `--dataset`, `--validate`, `--check-coverage`, `--export`).
- Pre-push Git hygiene audit passing with 0 leaks (`bash scripts/sanitize_check.sh`).

However, adversarial stress testing and code analysis revealed **critical fail-open logic flaws and regression bugs** in the AST evaluation engine:
1. **Backfill Poisoning Bypass**: In binary expressions, a backfilled operand poisons the parent node's `is_backfilled` boolean flag to `true`, causing unbackfilled sparse fields (e.g. `fscore_growth` with 29.8% coverage) to bypass the `ErrMissingBackfill` check completely.
2. **Lookback Arithmetic Bypass**: Non-positive lookback evaluation only parses literal integers and unary negation; arithmetic expressions like `1 - 2` return `None` and silently bypass the causal lookback guard.
3. **Price Normalization Inversion**: `AstExpr::Literal` initializes `is_normalized_price: false`, causing signals normalized according to BRAIN guidelines (e.g., `call_breakeven_30 / close - 1.0` or `signal * 2.0`) to trigger false-positive `WarnUnnormalizedPrice` warnings.
4. **Python Tool Regex Breakdown**: `scratch/metadata_tool.py` splits on the first comma inside `group_neutralize(...)`, mistaking inner function parameters as the grouping argument.

Because these defects directly compromise two core requirements of M2 ("low-coverage backfill requirement" and "causal lookback guard") and risk emitting invalid formulas during Milestone M3 factor combination, the verdict is **REQUEST_CHANGES**.

---

## 2. Integrity Audit

In accordance with reviewer instructions, the work was audited for fraudulent practices or integrity violations:

| Check | Result | Evidence |
|---|---|---|
| Hardcoded test results in source | **PASS** | Tests execute against live AST recursive parsing and WireMock HTTP servers. |
| Dummy or facade implementations | **PASS** | `SchemaValidator`, lexer, parser, symbol table, and client logic are fully implemented. |
| Task bypass / external delegation | **PASS** | All core validation and catalog structures built natively in Rust. |
| Fabricated verification logs | **PASS** | Verified independently via CLI commands, test runners, and WireMock executions. |
| Self-certifying without verification | **PASS** | Independent reproduction confirmed all claims. |

**Integrity Finding**: No integrity violations detected. The codebase is genuine and earnest, but contains algorithmic edge-case bugs that require remediation.

---

## 3. Findings & Adversarial Challenges

### [Critical] Finding 1: Backfill Poisoning / Bypass in Binary Expressions
- **What**: When combining a backfilled field with an unbackfilled sparse field in a binary operator, the unbackfilled field bypasses validation without error.
- **Where**: `src/schema_validator.rs:1520` and `src/schema_validator.rs:1282-1294`
- **Why**:
  In `eval_expr` for `AstExpr::Binary`:
  ```rust
  is_backfilled: l.is_backfilled || r.is_backfilled,
  ```
  And in `validate` statement post-check:
  ```rust
  if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled {
      for fid in &final_meta.low_coverage_origins {
          errors.push(ValidationError { code: DiagnosticCode::ErrMissingBackfill, ... });
      }
  }
  ```
  If operand `l` was backfilled (e.g., `ts_backfill(fscore_bfl_growth, 15)`), `l.is_backfilled` is `true`. The binary operation sets the resulting node's `is_backfilled = true`. When operand `r` is unbackfilled (e.g., `fscore_momentum` with 29.7% coverage), its identifier is added to `combined_origins`. However, at the statement check, `!final_meta.is_backfilled` evaluates to `false` because operand `l` poisoned the flag. Consequently, `ErrMissingBackfill` is never emitted!
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_bfl_growth, 15) + fscore_momentum), densify(subindustry))"
  ```
  *Output*: Evaluated as **PASSED (0 errors)** despite `fscore_momentum` being completely unbackfilled.
- **Blast Radius**: High. In Milestone M3 combinator generation, multi-factor blends will silently pass through invalid unbackfilled sparse signals, causing runtime simulation failures or subuniverse consistency test disqualifications on the BRAIN platform.
- **Suggested Fix**:
  1. In `src/schema_validator.rs:1282`, do not condition on `!final_meta.is_backfilled`. `ts_backfill` already clears `low_coverage_origins = Vec::new()`. If `low_coverage_origins` is non-empty at the root, unbackfilled sparse fields exist and MUST trigger `ErrMissingBackfill`:
     ```rust
     if !final_meta.low_coverage_origins.is_empty() { ... }
     ```
  2. In `src/schema_validator.rs:1520`, set `is_backfilled: l.is_backfilled && r.is_backfilled`.

---

### [Major] Finding 2: Lookback Parameter Bypass via Arithmetic Expressions
- **What**: Arithmetic expressions in lookback lag arguments (e.g. `1 - 2`) bypass non-causal lookback detection.
- **Where**: `src/schema_validator.rs:1618-1624` & `src/schema_validator.rs:1684-1698`
- **Why**:
  `extract_literal_number` only matches `AstExpr::Literal(n)` and `AstExpr::Unary { op: "-", expr }`.
  When a binary expression such as `1 - 2` or `5 - 10` is passed, `extract_literal_number` returns `None`.
  Line 1685 performs:
  ```rust
  if let Some(lag) = extract_literal_number(&args[1]) {
      if lag <= 0.0 {
          errors.push(ValidationError { code: DiagnosticCode::ErrNegativeLookback, ... });
      }
  }
  ```
  Because `extract_literal_number` returns `None`, the check is silently bypassed, and non-causal negative lookbacks evaluate to valid.
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))"
  ```
  *Output*: Evaluated as **PASSED (0 errors)** with negative lookback of -1!
- **Blast Radius**: High. Violates core requirement of look-ahead bias prevention.
- **Suggested Fix**:
  Extend `extract_literal_number` to recursively evaluate constant binary arithmetic, or fail closed (`ErrNegativeLookback` / `ErrSyntaxMalformed`) when the lookback lag is not a statically resolvable positive constant.

---

### [Major] Finding 3: Literal Scalars Invert Price Normalization Tracking
- **What**: Applying standard offsets or scaling factors (such as `- 1.0` or `* 2.0`) marks the signal as an unnormalized price level, emitting a false-positive warning.
- **Where**: `src/schema_validator.rs:1329`, `1336`, and `1504-1513`
- **Why**:
  In `eval_expr`:
  ```rust
  AstExpr::Literal(_) => ValueMeta {
      expr_type: FastExprType::Scalar,
      ...
      is_normalized_price: false, // <-- Marked as unnormalized price!
  }
  ```
  In binary operations:
  ```rust
  let is_norm = if op == "/" { ... } else { l.is_normalized_price && r.is_normalized_price };
  ```
  When an author follows the platform recommendation to normalize option prices (`call_breakeven_30 / close - 1.0`), `call_breakeven_30 / close` is normalized (`true`), but the literal `1.0` is `false`. The binary difference combines `true && false = false`, inverting the normalization status and causing `rank()` to warn:
  `WARNING: [WARN_UNNORMALIZED_PRICE] Raw price or breakeven level passed to rank() without close normalization`!
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))"
  ```
  *Output*: Emits `WARN_UNNORMALIZED_PRICE` despite following the suggestion verbatim.
- **Blast Radius**: Medium. Emits confusing and incorrect diagnostics on valid, properly normalized expressions.
- **Suggested Fix**:
  In `AstExpr::Literal` and `AstExpr::StringLit`, initialize `is_normalized_price: true`, as scalar numbers are unitless and dimensionally neutral.

---

### [Minor] Finding 4: Missing Function Arity Validation
- **What**: Several functions accept zero arguments without failing closed.
- **Where**: `src/schema_validator.rs:1559-1567`, `1668-1707`, `1710-1737`
- **Why**:
  `densify()`, `rank()`, and `ts_delay()` do not validate parameter count.
- **Empirical Reproducer**:
  ```bash
  cargo run -- fields --validate "group_neutralize(rank(ts_delay()), densify(subindustry))"
  cargo run -- fields --validate "group_neutralize(rank(), densify())"
  ```
  *Output*: Both evaluate as **PASSED (0 errors)**.
- **Blast Radius**: Medium. Emits formulas that will cause HTTP 400 Bad Request simulation errors on BRAIN.
- **Suggested Fix**:
  Enforce strict parameter counts: `densify` exactly 1; `rank` exactly 1; `ts_*` at least 2.

---

### [Minor] Finding 5: Python Tool Regex Broken on Nested Commas
- **What**: `scratch/metadata_tool.py` validation fails on nested function arguments within `group_neutralize(...)`.
- **Where**: `scratch/metadata_tool.py:263-268`
- **Why**:
  Regex `r"group_neutralize\s*\([^,]+,\s*([^)]+)\)"` splits at the first comma. In `group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))`, the first comma is inside `ts_decay_linear(...)`, causing `5` to be treated as the grouping argument.
- **Empirical Reproducer**:
  ```bash
  python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"
  ```
  *Output*: `ERROR: group_neutralize grouping argument '5' must be wrapped in densify(...)`.
- **Blast Radius**: Low (script is in git-ignored `scratch/`).
- **Suggested Fix**:
  Use parenthesis-counting bracket matching or document that `alphafind fields --validate` is the authoritative validator.

---

### [Minor] Finding 6: Ternary Operator Syntax Not Supported in AST Parser
- **What**: The worker handoff claimed the AST parser supports "ternary logic", but `?` and `:` are only tokenized and not parsed.
- **Where**: `src/schema_validator.rs:798` vs `src/schema_validator.rs:963-1117`
- **Why**:
  `Parser` has no production rule for ternary expressions. `close > open ? 1.0 : -1.0` fails with `AST Parser error: Unexpected token: Some(Op("?"))`.
- **Blast Radius**: Low. FastExpr expressions should use `if_else(...)` instead.
- **Suggested Fix**:
  Add ternary parsing to `Parser` or clarify that `if_else(cond, true_val, false_val)` is the supported syntax.

---

### [Minor] Finding 7: Diagnostic Code Misattribution for Unknown Fields
- **What**: Uncataloged fields emit `DiagnosticCode::WarnHorizonSyntax` instead of a dedicated code.
- **Where**: `src/schema_validator.rs:1450`
- **Suggested Fix**:
  Add `WarnUnknownField` or `ErrUnknownField` rather than reusing `WarnHorizonSyntax`.

---

## 4. Verified Claims

| Feature / Claim | Verification Command | Result |
|---|---|---|
| 6-Type Semantic Lattice | `cargo test --test metadata_schema_tests` | **PASS** (6 types correctly model tensors) |
| Vector Folding Enforcement | `cargo run -- fields --validate "group_neutralize(rank(nws18_ssc), densify(subindustry))"` | **PASS** (Fails closed with `ERR_VEC_TYPE_MISMATCH`) |
| Folded Vector Acceptance | `cargo run -- fields --validate "group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))"` | **PASS** (Passed with 0 errors) |
| Group Densify Enforcement | `cargo run -- fields --validate "group_neutralize(rank(close), subindustry)"` | **PASS** (Fails closed with `ERR_UNDENSIFIED_GROUP`) |
| Option 9 Trailing 'd' Syntax | `cargo run -- fields --validate "group_neutralize(rank(pcr_vol_20d), densify(subindustry))"` | **PASS** (Fails closed with `ERR_UNKNOWN_FIELD`) |
| Dissemination Delay Invariant | `cargo test --test metadata_schema_tests test_validator_enforces_delay_compatibility` | **PASS** (Delay 1 field rejected in Delay 0) |
| 235-Field Catalog Invariants | `cargo run -- fields --summary` | **PASS** (Exact counts: 16 m51, 24 m16, 121 n18, 74 opt9) |
| Low-Coverage Field Audit | `cargo run -- fields --check-coverage` | **PASS** (Flags exactly 16 sparse fields) |
| WireMock Pagination & 429 Retry | `cargo test --test metadata_schema_tests test_mock_*` | **PASS** (Multi-page pagination & exponential backoff) |
| Git Hygiene Gatekeeper | `bash scripts/sanitize_check.sh` | **PASS** (Pre-Push Audit PASSED with 0 leaks) |

---

## 5. Coverage Gaps & Unverified Items

- **Live Platform Token Refresh**: Testing live `--refresh` with real credentials was not performed to prevent external API side-effects; verified thoroughly via WireMock in-memory HTTP test harnesses.
- **Ternary Operator AST Parsing**: Unverified because parser lacks ternary productions.

---

## 6. Actionable Recommendations for Worker

To unblock approval and ensure Milestone M3 can build on a completely fail-closed schema validator, apply the following 4 targeted modifications in `src/schema_validator.rs`:

1. **Fix Backfill Root Condition** (line 1282):
   ```rust
   // Replace:
   if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled
   // With:
   if !final_meta.low_coverage_origins.is_empty()
   ```
2. **Fix Binary Backfill Flag** (line 1520):
   ```rust
   is_backfilled: l.is_backfilled && r.is_backfilled,
   ```
3. **Fix Literal Scalar Price Normalization** (line 1329):
   ```rust
   AstExpr::Literal(_) => ValueMeta {
       ...
       is_normalized_price: true,
   },
   ```
4. **Fix Constant Arithmetic in Lookback Evaluation** (line 1618):
   Extend `extract_literal_number` to recursively evaluate `AstExpr::Binary` arithmetic (e.g. `1 - 2 => -1.0`).
