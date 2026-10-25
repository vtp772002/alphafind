# Tier 5 Adversarial Validation & Challenge Report

**Agent**: `challenger_m5_1`  
**Role**: EMPIRICAL CHALLENGER (critic, specialist)  
**Milestone**: M5 (Final E2E Test Suite, Adversarial Validation & Verification Runner)  
**Date**: 2026-10-24  
**Verdict**: 🔴 **REQUEST_CHANGES**  

---

## 1. Observation

Directly observed facts and command outputs executed during validation:

### 1.1 Test Suite & Sanitization Verification (Passed)
- `cargo test -- --test-threads=1`: 253 tests run, 253 passed, 0 failed, 0 ignored (Exit code 0).
- `cargo clippy --tests -- -D warnings`: Clean exit with 0 warnings (Exit code 0).
- `cargo fmt --check`: Clean formatting across workspace (Exit code 0).
- `bash scripts/sanitize_check.sh`:
  ```
  🔍 [SANITIZATION GATEKEEPER] Running Pre-Push Audit...
     ✓ Commit-history scan: no Alpha IDs in added lines.
     ✓ Dataset-field scan: all tracked files are clear of configured field patterns.
     ✓ Candidate artifact scan: 12 private literals absent from tracked files.
  ✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.
  ```
- Python companion tools:
  - `python3 scratch/metadata_tool.py --dataset all --summary`: PASSED (235 fields categorized).
  - `python3 scratch/expression_builder.py --dataset all --validate`: PASSED (10 formulas generated and validated).
  - `python3 scratch/evaluate_alternative_alphas.py --offline`: PASSED (4 candidate alphas qualified, 6/6 PASS checks, low correlation $\max \rho \le 7.87\%$).

### 1.2 Observed Bug 1: Unhandled Recursive Parser Stack Overflow (SIGABRT)
- **Command**:
  ```bash
  python3 -c "import subprocess; expr = '(' * 1500 + 'close' + ')' * 1500; res = subprocess.run(['target/debug/alphafind', 'fields', '--validate', expr], capture_output=True, text=True); print('Exit code:', res.returncode); print('Stderr:', res.stderr)"
  ```
- **Output**:
  ```
  Exit code: -6
  Stderr: 
  thread 'main' (3342384) has overflowed its stack
  fatal runtime error: stack overflow, aborting
  ```
- **Source location**: `src/schema_validator.rs:985-1142` (`Parser::parse_expr`, `parse_primary`).

### 1.3 Observed Bug 2: Tokenizer Dropped Characters and Missing Semicolon Multi-Statement Bypass
- **Command 1**: `cargo run -- fields --validate 'close $ 5'`
- **Output 1**:
  ```
  🔍 Validating FastExpr Formula Schema (delay=1):
  --------------------------------------------------
  Expression: close $ 5

  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)

  📊 Extracted Fields (1) :
    - close [MATRIX] (coverage: 100.0%)
  ```
- **Command 2**: `cargo run -- fields --validate 'close open volume'`
- **Output 2**:
  ```
  🔍 Validating FastExpr Formula Schema (delay=1):
  --------------------------------------------------
  Expression: close open volume

  ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)

  📊 Extracted Fields (3) :
    - open [MATRIX] (coverage: 100.0%)
    - volume [MATRIX] (coverage: 100.0%)
    - close [MATRIX] (coverage: 100.0%)
  ```
- **Source location**:
  - `src/schema_validator.rs:897-900` (`tokenize` silently skips unhandled characters).
  - `src/schema_validator.rs:956-983` (`Parser::parse_statements` treats consecutive expressions without semicolons as separate statements).

### 1.4 Observed Bug 3: Terminal Expression Allows Non-Matrix Types (Scalars, Strings, Groups, Raw 3D Vectors)
- **Commands & Outputs**:
  - `cargo run -- fields --validate 42` -> `PASSED (0 errors, 0 warnings)`.
  - `cargo run -- fields --validate '"hello"'` -> `PASSED (0 errors, 0 warnings)`.
  - `cargo run -- fields --validate subindustry` -> `PASSED (0 errors, 0 warnings)`.
  - `cargo run -- fields --validate 'densify(subindustry)'` -> `PASSED (0 errors, 0 warnings)`.
  - `cargo run -- fields --validate nws18_ssc` ->
    ```
    ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
    📊 Extracted Fields (1) :
      - nws18_ssc [VECTOR (folded)] (coverage: 100.0%)
    ```
- **Source location**:
  - `src/schema_validator.rs:1352-1398` (`AstStatement::Expression` does not verify `final_meta.expr_type == FastExprType::Matrix`).
  - `src/schema_validator.rs:197-198` (`print_diagnostics` unconditionally displays `VECTOR (folded)` for all vector fields regardless of whether they were folded).

### 1.5 Observed Bug 4: Generation Count Edge Case (`count = 0` yields 1 expression)
- **Command**: `cargo run -- generate -n 0`
- **Output**:
  ```
  ⚡ Automated Alternative Expression Builder & Factor Combinator
  ===============================================================
  Dataset Filter: all
  Pillar Filter:  all
  Universe:       TOP1000 (delay=1)
  Neutralization: SUBINDUSTRY
  Count:          1

  [01] US_M51_Beta_Term_Spread [model51] ...
  ```
- **Source location**: `src/expression_builder.rs:477` (`(count / len).max(1)`) and line 488 (`results.push` occurs before `results.len() >= count` check).

### 1.6 Observed Bug 5: Silent Fallback on Invalid Dataset and Pillar Arguments in `generate`
- **Commands & Outputs**:
  - `cargo run -- generate --dataset bogus -n 5 --validate`: Silently falls back to `model51` without error or warning.
  - `cargo run -- generate --pillar bogus -n 5 --validate`: Silently falls back to generating all pillars without error or warning.
- **Source location**: `src/expression_builder.rs:453-458` and `src/commands/research.rs:1400`.

### 1.7 Observed Discrepancy 6: Inaccurate Class Attribution in `TEST_READY.md`
- **Text in `TEST_READY.md:146`**:
  `Automatic formula repair capabilities (SchemaValidator::auto_repair_expression)`
- **Actual Code**:
  Method is `ExpressionBuilder::auto_repair(&self, expr: &str, report: &ValidationReport) -> String` located in `src/expression_builder.rs:717`. `SchemaValidator` has no repair method.

---

## 2. Logic Chain

1. **Premise 1 (R3 / Acceptance Criteria)**:
   Per `ORIGINAL_REQUEST.md §R3` and `PROJECT.md`, the FastExpr syntax validator must prevent HTTP 400 Bad Request simulation errors by ensuring that all passed expressions are syntactically and semantically valid WorldQuant BRAIN alpha expressions.
2. **Inference from Observation 1.3 (Bug 2)**:
   In WorldQuant BRAIN FastExpr, `close $ 5` and `close open volume` are syntactically invalid. Semicolons are required between statements, and unexpected tokens (`$`) are illegal. Because `tokenize` silently drops unexpected characters and `parse_statements` treats unseparated tokens as independent statements, invalid expressions pass validation locally. If dispatched to BRAIN, they cause HTTP 400 Bad Request simulation errors.
3. **Inference from Observation 1.4 (Bug 3)**:
   In WorldQuant BRAIN, an alpha expression must produce a 2D cross-sectional numerical weight matrix tensor. Constants (`42`), string literals (`"hello"`), categorical groups (`subindustry`, `densify(subindustry)`), and bare raw 3D vectors (`nws18_ssc`) cannot produce valid alpha returns. `SchemaValidator` fails to assert that the terminal expression evaluates to `FastExprType::Matrix`, allowing non-alpha formulas to pass. Furthermore, labeling unfolded `nws18_ssc` as `[VECTOR (folded)]` is misleading.
4. **Inference from Observation 1.2 (Bug 1)**:
   The recursive-descent parser lacks a maximum recursion depth limit. Expressions with nesting depth >= 1500 exhaust the thread call stack, causing an unhandled fatal abort (SIGABRT). Robust software must fail closed with structured validation errors rather than terminating the process with an OS signal.
5. **Inference from Observation 1.5 & 1.6 (Bugs 4 & 5)**:
   `generate -n 0` violates the principle of least astonishment by returning 1 expression when 0 were requested. Passing nonexistent datasets/pillars should fail fast with a descriptive error rather than silently substituting default datasets.
6. **Conclusion**:
   While the baseline functionality, unit tests (253 tests), POSIX advisory concurrency locking, and sanitization audit are sound, the schema validator contains critical fail-open loopholes and parser fragility that must be addressed before final approval.

---

## 3. Adversarial Challenge Analysis

### Overall Risk Assessment: HIGH

### Challenges

#### [High] Challenge 1: Silent Token Drop and Semicolon-Less Multi-Statement Bypass
- **Assumption challenged**: All invalid FastExpr syntax is rejected with structured errors before platform dispatch.
- **Attack scenario**: Submitting corrupted expressions like `close $ 5` or `close open volume`.
- **Blast radius**: Local validator reports `PASSED (0 errors)`, but dispatch to BRAIN returns HTTP 400 Bad Request.
- **Mitigation**:
  1. Update `tokenize` in `src/schema_validator.rs`: emit an error on unrecognized characters instead of silently skipping them.
  2. In `Parser::parse_statements`: require explicit semicolons `;` between consecutive statements. If a statement concludes without a semicolon and more tokens remain, return an AST parser error: `Consecutive statements must be separated by semicolons`.

#### [High] Challenge 2: Non-Matrix Terminal Expressions Pass Validation
- **Assumption challenged**: Expressions passing validation are executable quantitative alpha expressions.
- **Attack scenario**: Submitting `42`, `"hello"`, `subindustry`, or bare `nws18_ssc`.
- **Blast radius**: Bare vectors, scalars, and groups pass local validation, causing runtime type errors during simulation.
- **Mitigation**:
  In `SchemaValidator::validate` (`src/schema_validator.rs:1375-1398`), verify that the terminal expression evaluated type is `FastExprType::Matrix` or `FastExprType::BooleanMatrix`. If it is `Scalar`, `String`, `Group`, `DensifiedGroup`, or `Vector`, push `DiagnosticCode::ErrSyntaxMalformed` or `DiagnosticCode::ErrVecTypeMismatch`. Also fix `print_diagnostics` to only label vectors as `[VECTOR (folded)]` if they are actually folded.

#### [Medium] Challenge 3: Unhandled Recursive Stack Overflow in Parser
- **Assumption challenged**: Parser handles arbitrary expression nesting gracefully without crashing.
- **Attack scenario**: Expressions with >= 1500 parentheses: `((...close...))`.
- **Blast radius**: Thread stack overflow terminates the CLI process with SIGABRT (exit code -6).
- **Mitigation**:
  Add `depth: usize` field to `Parser` with a maximum limit (e.g. `128`). Increment on entering parenthesized expressions or function calls, and error out if `depth > 128`.

#### [Low] Challenge 4: Zero-Count Generation and Unhandled Dataset Arguments
- **Assumption challenged**: CLI arguments obey numerical bounds and fail fast on unknown inputs.
- **Attack scenario**: `alphafind generate -n 0` or `alphafind generate --dataset bogus`.
- **Blast radius**: Output contains unwanted expressions or silently wrong datasets.
- **Mitigation**:
  In `generate_batch`: `if count == 0 { return Vec::new(); }`. In `cmd_generate`: reject unknown dataset and pillar names with `anyhow::bail!`.

---

## 4. Stress Test Results Summary

| Scenario | Expected Behavior | Actual Behavior | Result |
|---|---|---|:---:|
| `cargo test -- --test-threads=1` | All 253 tests pass | 253 passed, 0 failed | 🟢 PASS |
| `cargo clippy --tests -- -D warnings` | 0 warnings | 0 warnings | 🟢 PASS |
| `cargo fmt --check` | Clean formatting | Clean formatting | 🟢 PASS |
| `bash scripts/sanitize_check.sh` | 0 leaks detected | 0 leaks across 3 phases | 🟢 PASS |
| `ProcessLock` multi-process race (20 procs) | Mutual exclusion, 0 race conditions | 5 acquired, 15 contended, 0 deadlock | 🟢 PASS |
| `ProcessLock` SIGKILL recovery | Kernel auto-releases flock on dead process | Lock acquired immediately without file removal | 🟢 PASS |
| `alphafind fields --validate 'ts_delay(close, open)'` | Reject non-static lookback lag | Caught `ERR_NEGATIVE_LOOKBACK` | 🟢 PASS |
| `alphafind fields --validate 'group_neutralize(rank(close), densify(close))'` | Reject non-group densify argument | Caught `ERR_GROUP_TYPE_MISMATCH` | 🟢 PASS |
| `alphafind fields --validate 'mean_composite_sentiment_score' --delay 0` | Reject Delay 1 field under Delay 0 | Caught `ERR_LOOKAHEAD_DELAY` | 🟢 PASS |
| `alphafind fields --validate 'close $ 5'` | Reject invalid character `$` | PASSED (0 errors) | 🔴 **FAIL** |
| `alphafind fields --validate 'close open volume'` | Reject missing semicolons | PASSED (0 errors) | 🔴 **FAIL** |
| `alphafind fields --validate 'nws18_ssc'` | Reject unfolded raw 3D vector | PASSED (0 errors, labeled folded) | 🔴 **FAIL** |
| `alphafind fields --validate '42'` | Reject scalar constant as alpha | PASSED (0 errors) | 🔴 **FAIL** |
| `alphafind fields --validate 'densify(subindustry)'` | Reject group matrix as alpha | PASSED (0 errors) | 🔴 **FAIL** |
| `alphafind fields --validate '((...1500 parens...))'` | Fail closed with syntax error | Stack overflow abort (SIGABRT) | 🔴 **FAIL** |
| `alphafind generate -n 0` | Return 0 candidates | Returned 1 candidate | 🔴 **FAIL** |
| `alphafind generate --dataset bogus` | Error on unknown dataset | Silently generated `model51` | 🔴 **FAIL** |

---

## 5. Caveats

- **No live BRAIN API connection**: Platform API endpoints (`/data-fields`, `/simulations`) were tested in offline mock mode (`--offline`) since valid credentials for live platform dispatch were not configured in this run. Platform-side HTTP responses were evaluated against documented BRAIN API behavior.
- **Advisory flock scope**: As documented in `AGENTS.md`, POSIX advisory locking is host-local and does not coordinate across distinct physical machines.

---

## 6. Conclusion & Recommendation

**Verdict**: 🔴 **REQUEST_CHANGES**

### Required Remediations Before Approval:
1. **Fix Tokenizer & Parser Syntax Enforcement**:
   - Do not drop unrecognized characters in `src/schema_validator.rs:897-900`.
   - Require explicit semicolons between statements in multi-statement scripts (`src/schema_validator.rs:956-983`).
2. **Enforce Matrix Type on Terminal Alpha Expressions**:
   - In `src/schema_validator.rs:1375-1398`, reject expressions whose terminal evaluation is `Scalar`, `StringLit`, `Group`, `DensifiedGroup`, or `Vector`.
   - Fix `print_diagnostics` (`src/schema_validator.rs:197-198`) so it only labels vectors as `[VECTOR (folded)]` if folding was verified.
3. **Add Recursion Depth Limit in Parser**:
   - Bound recursive calls in `src/schema_validator.rs` to max depth (e.g. 128) to prevent stack overflow aborts.
4. **Fix CLI Parameter Edge Cases**:
   - Return 0 candidates when `count == 0` in `src/expression_builder.rs:474-495`.
   - Reject unknown datasets and pillars with explicit errors in `src/commands/research.rs:1400`.
5. **Correct Documentation**:
   - Update `TEST_READY.md:146` to accurately reference `ExpressionBuilder::auto_repair`.

---

## 7. Verification Method

To independently verify these findings:
```bash
# 1. Reproduce parser stack overflow (SIGABRT)
python3 -c "import subprocess; expr = '(' * 1500 + 'close' + ')' * 1500; res = subprocess.run(['target/debug/alphafind', 'fields', '--validate', expr], capture_output=True, text=True); print('Exit code:', res.returncode)"
# Expected: -6 (SIGABRT)

# 2. Reproduce dropped token & unseparated expression bypass
target/debug/alphafind fields --validate "close $ 5"
target/debug/alphafind fields --validate "close open volume"
# Expected: both incorrectly report PASSED (0 errors)

# 3. Reproduce non-matrix terminal expression acceptance
target/debug/alphafind fields --validate "42"
target/debug/alphafind fields --validate "nws18_ssc"
# Expected: both incorrectly report PASSED (0 errors)

# 4. Reproduce zero count bug
target/debug/alphafind generate -n 0
# Expected: generates 1 expression instead of 0
```
