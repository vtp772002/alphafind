# Forensic Audit Report: Milestone M2 Remediation Recheck (Iteration 2)

**Auditor**: Forensic Integrity Auditor (`auditor_m2_recheck`)  
**Role**: critic, specialist, auditor  
**Date**: 2026-10-24T20:25:00Z  
**Target**: Milestone M2 Remediation (`src/schema_validator.rs`, `src/models.rs`, `src/client.rs`, `scratch/metadata_tool.py`, test suites)  
**Profile**: General Project (with quantitative research & confidentiality standards from `AGENTS.md`)  
**Integrity Mode**: Development (per `ORIGINAL_REQUEST.md:8`)  
**Verdict**: **CLEAN**

---

## 1. Executive Summary

A forensic integrity recheck was conducted on the Milestone M2 remediation work product following fixes implemented by `worker_m2_remediation`. The audit evaluated all five previously flagged fail-open vulnerabilities (backfill poisoning bypass, lookback parameter bypass via arithmetic/variables, price normalization false positives on literal scalars, scratch tool regex breakdown on nested commas, and missing function arity/script structure checks), inspected source code for hardcoded bypasses or facade implementations, audited Git hygiene for confidential research material leaks, and independently executed the complete unit, integration, and stress test suites.

All forensic checks passed unconditionally. The implementation contains authentic AST tokenization, recursive descent parsing, constant folding arithmetic evaluation, strict vector and lookback validation, and fail-closed error diagnostics. The binary verdict is **CLEAN**.

---

## 2. Forensic Phase Results

| # | Forensic Check | Result | Evidence & Observations |
|---|----------------|--------|--------------------------|
| 1 | **Authenticity & Non-Facade Check** | **PASS** | `src/schema_validator.rs` (1,942 lines) implements genuine tokenization (`tokenize`), recursive descent parsing (`Parser::parse_statements`), AST evaluation (`eval_expr`), and constant folding (`extract_constant_number`). No dummy facades, no stubbed returns, zero `unimplemented!()` or `todo!()` blocks. |
| 2 | **Hardcoded Output & Anti-Cheating Check** | **PASS** | Source code analysis confirms zero hardcoded test outputs or string matching on test case names (e.g. no special casing of `fscore_momentum` or specific alpha formulas). AST nodes are semantically validated against the 6-type lattice (`Scalar`, `Matrix`, `Vector`, `Group`, `DensifiedGroup`, `BooleanMatrix`). |
| 3 | **Pre-Populated Artifact & Verification Fabrications** | **PASS** | No pre-populated result logs, mock attestations, or static verification outputs exist in the repository. All test results were produced through direct, unmocked execution of `cargo` and Python. |
| 4 | **Git Hygiene & Confidentiality Gate (AGENTS.md)** | **PASS** | `scripts/sanitize_check.sh` executed cleanly with exit code 0 (`Pre-Push Audit PASSED. Safe to push`). Scans verified 0 Alpha IDs in commits/diffs, all tracked files clear of proprietary field patterns, and 0 private candidate expressions exposed. Candidate alphas in `src/taxonomy.rs` strictly use abstract placeholders (`<COMPOSITE_SCORE_DERIVATIVE>`, `<CALL_BREAKEVEN_FIELD>`, etc.). |
| 5 | **Remediation Invariant Verification** | **PASS** | Verified resolution of all 5 defect categories: <br>• Backfill poisoning resolved via multi-origin `low_coverage_origins` accumulator and conjunctive `is_backfilled` logic in `AstExpr::Binary`.<br>• Lookback arithmetic bypass resolved via `extract_constant_number` supporting `+`, `-`, `*`, `/`, unary negation, and variable bindings.<br>• Dimensionless scalars (`1.0`) initialized with `is_normalized_price: true`, eliminating false positives in price ratios.<br>• Python tool parenthesis depth tracker (`extract_call_args`) accurately isolates top-level arguments.<br>• Strict arity and terminal expression requirements reject empty/assignment scripts and 0-argument calls. |
| 6 | **Independent Test Execution** | **PASS** | 100% test pass rate across all test targets:<br>• `cargo test --test metadata_schema_tests`: **25 passed; 0 failed**<br>• `cargo test --test stress_schema_validator_tests`: **12 passed; 0 failed**<br>• `cargo test --test stress_metadata_tests`: **16 passed; 0 failed**<br>• Full `cargo test`: **153 passed; 0 failed; 0 ignored** across 8 test suites. |

---

## 3. Detailed Forensic Code Inspection

### 3.1 Backfill Poisoning Remediation (`src/schema_validator.rs:1602-1608, 1341-1353`)
```rust
let l_non_sparse = l.is_backfilled || l.low_coverage_origins.is_empty();
let r_non_sparse = r.is_backfilled || r.low_coverage_origins.is_empty();
let is_bfl = l_non_sparse && r_non_sparse && (l.is_backfilled || r.is_backfilled);

let mut combined_origins = l.low_coverage_origins;
combined_origins.extend(r.low_coverage_origins);
```
At the root statement evaluation (`lines 1341-1353`), the validator checks:
```rust
if !final_meta.low_coverage_origins.is_empty() {
    for fid in &final_meta.low_coverage_origins {
        errors.push(ValidationError {
            code: DiagnosticCode::ErrMissingBackfill,
            field_id: Some(fid.clone()),
            message: format!(
                "Field '{}' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill(..., d >= 5)",
                fid
            ),
            suggestion: Some(format!("Wrap '{}' with ts_backfill({}, 15)", fid, fid)),
        });
    }
}
```
**Finding**: Any low-coverage field that lacks a wrapping `ts_backfill(..., d >= 5)` retains its identifier in `low_coverage_origins`, propagating through all binary and unary operations up to the statement root. The previous fail-open vulnerability is completely eliminated.

### 3.2 Constant Folding Lookback Evaluation (`src/schema_validator.rs:1146-1174, 1825-1848`)
```rust
fn extract_constant_number(expr: &AstExpr, symbol_table: &HashMap<String, ValueMeta>) -> Option<f64> {
    match expr {
        AstExpr::Literal(n) => Some(*n),
        AstExpr::Unary { op, expr } => {
            let inner = extract_constant_number(expr, symbol_table)?;
            match op.as_str() {
                "-" => Some(-inner),
                "+" => Some(inner),
                _ => None,
            }
        }
        AstExpr::Binary { op, left, right } => {
            let l = extract_constant_number(left, symbol_table)?;
            let r = extract_constant_number(right, symbol_table)?;
            match op.as_str() {
                "+" => Some(l + r),
                "-" => Some(l - r),
                "*" => Some(l * r),
                "/" => if r.abs() > 1e-9 { Some(l / r) } else { None },
                _ => None,
            }
        }
        AstExpr::Variable(name) => {
            symbol_table.get(name).and_then(|m| m.scalar_value)
        }
        _ => None,
    }
}
```
**Finding**: Expressions such as `ts_delay(close, 1 - 2)` or variable bindings `lookback = -5; ts_delay(close, lookback)` are statically folded and evaluated against $d \le 0$, strictly emitting `ErrNegativeLookback`. If the lag cannot be statically resolved and its evaluated type is not scalar, it fails closed.

### 3.3 Price Normalization Dimensionless Neutrality (`src/schema_validator.rs:1140, 1591-1600`)
```rust
let is_norm = if op == "/" {
    if let AstExpr::Variable(var) = right.as_ref() {
        var.eq_ignore_ascii_case("close")
    } else {
        l.is_normalized_price && r.is_normalized_price
    }
} else {
    l.is_normalized_price && r.is_normalized_price
};
```
**Finding**: Literal constants are initialized with `is_normalized_price: true`. In `call_breakeven_30 / close - 1.0`, `(true && true) = true`, preventing false-positive `WarnUnnormalizedPrice` warnings while continuing to flag raw price levels such as `rank(call_breakeven_30)`.

---

## 4. Empirical Test Evidence

### 4.1 CLI Integration Reproductions
1. **Backfill Poisoning Check**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
   ```
   **Output**:
   ```
   ❌ FastExpr Schema Validation: FAILED (1 errors, 2 warnings)
     ERROR: [ERR_MISSING_BACKFILL] Field 'fscore_growth' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill(..., d >= 5)
   ```
   *(Exit code 1, verified fail-closed)*

2. **Price Normalization Check**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))"
   ```
   **Output**:
   ```
   ✅ FastExpr Schema Validation: PASSED (0 errors, 0 warnings)
   ```
   *(Exit code 0, verified clean)*

3. **Python Scratch Tool Check**:
   ```bash
   python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"
   ```
   **Output**:
   ```
   ✅ Schema Validation PASSED (0 errors)
   ```

### 4.2 Git Sanitization Gatekeeper
```bash
GIT_CONFIG_GLOBAL=/dev/null bash scripts/sanitize_check.sh
```
**Output**:
```
🔍 [SANITIZATION GATEKEEPER] Running Pre-Push Audit...
   ✓ Commit-history scan: no Alpha IDs in added lines.
   ✓ Dataset-field scan: all tracked files are clear of configured field patterns.
   ✓ Candidate artifact scan: 12 private literals absent from tracked files.
✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.
```

### 4.3 Automated Crate Test Suites
- `metadata_schema_tests.rs`: 25 passed; 0 failed
- `stress_schema_validator_tests.rs`: 12 passed; 0 failed
- `stress_metadata_tests.rs`: 16 passed; 0 failed
- Overall `cargo test`: 153 passed; 0 failed across all crate units and integration targets.

---

## 5. Audit Verdict

**FINAL VERDICT: CLEAN**

Milestone M2 Remediation satisfies all functional requirements, security and platform integrity rules, and Git hygiene constraints. The work product is approved for baseline integration into Milestone M3.
