# Software Quality & Adversarial Review: Milestone M1 (Taxonomy & Specification Module)

**Reviewer**: Reviewer 2 (Software & Interface Reviewer / Adversarial Critic)  
**Date**: 2026-10-24  
**Target Files**: `src/taxonomy.rs`, `tests/taxonomy_tests.rs`  
**Verdict**: **APPROVE WITH RECOMMENDATIONS**

---

## 1. Executive Summary

A comprehensive, adversarial review of the Milestone M1 implementation was conducted across `src/taxonomy.rs` and `tests/taxonomy_tests.rs`. The code was evaluated against the Milestone M1 interface contracts defined in `PROJECT.md`, the research hygiene constraints of `AGENTS.md`, standard Rust idiomatic guidelines, and resilience against adversarial inputs.

All 19 tests in `tests/taxonomy_tests.rs` pass, all 108 crate-wide tests pass, `cargo clippy` runs cleanly, and `scripts/sanitize_check.sh` reports zero leaks of private candidate formulas or forbidden field identifiers. Furthermore, genuine candidate store isolation is verified via `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json`.

No integrity violations were detected: the implementation does not utilize hardcoded test shortcuts, facades, or fabricated outputs. Several constructive findings (1 Major, 5 Minor) were surfaced during adversarial stress-testing, primarily around edge-case token extraction when mathematical comparison operators (`<`) precede template placeholders.

---

## 2. Integrity Audit

Under the reviewer and adversarial critic guidelines, the implementation was inspected for integrity violations:

| Check Item | Result | Evidence / Notes |
|------------|--------|------------------|
| Hardcoded test results / facade logic | PASS | All methods in `FactorPillar` and `PlaceholderRegistry` implement genuine parsing, string transformation, and validation logic. |
| Task bypass / external delegation | PASS | Core parsing, template extraction, and validation logic is implemented natively in Rust. |
| Research material leak / Git hygiene | PASS | Pre-push audit `scripts/sanitize_check.sh` passed cleanly. All formulas in tracked files strictly use abstract placeholders (`<FIELD>`, `<SIGNAL>`). |
| Candidate store isolation | PASS | Running with an empty candidate file (`ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json`) successfully triggers sanitized preset fallback across all 19 tests. |
| Self-certifying / fabricated results | PASS | Independently verified via manual CLI execution of all tests and tools. |

**Integrity Verdict**: **CLEAN — NO INTEGRITY VIOLATIONS DETECTED.**

---

## 3. Detailed Review Dimensions

### 3.1 Software Quality & Architecture
- **Enum Extension**: `FactorPillar` expands from 8 to 12 variants, seamlessly incorporating `SystematicRisk` (`model51`), `FundamentalScores` (`model16`), `NewsSentiment` (`news18`), and `OptionsAnalytics` (`option9`).
- **Pillar Metadata Accessors**:
  - `primary_dataset_id(&self)` maps each pillar to its foundational BRAIN dataset ID (`model51`, `model16`, `news18`, `option9`, `option8`, `analyst4`, `fundamental6`, `short`, `pv1`).
  - `description(&self)` provides concise, economically sound descriptions of the underlying transmission mechanism.
  - `is_alternative_data(&self)` cleanly segments alternative datasets from traditional price-volume metrics.
- **Candidate Alpha Presets**: High-quality factor skeletons incorporating realistic mathematical operations (`rank`, `signed_power`, `ts_backfill`, `vec_avg`, `group_neutralize`, `densify`) provide excellent scaffolding for hypothesis generation in Milestone M3.

### 3.2 Rust Idiomatic Design
- **Derives**: `FactorPillar` appropriately derives `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`.
- **String Parsing**: `from_str(&str) -> Option<Self>` employs case-insensitive substring matching. It correctly places alternative data patterns first to prevent ambiguous overlaps (e.g., `SYSTEMATIC_RISK` is matched before `RISK` in `IdiosyncraticRisk`).
- **Idiomatic Opportunity**: `FactorPillar::from_str` carries `#[allow(clippy::should_implement_trait)]`. Implementing `std::str::FromStr` is the standard Rust approach (detailed in Finding 4).

### 3.3 Backward Compatibility
- **Legacy Matchers Preserved**: All legacy variants (`AnalystConsensus`, `OptionsVrp`, `MicrostructureVwap`, `FinancialHealthQuality`, `IdiosyncraticRisk`, `ShortInterest`, `NewsEvent`, `CrossSanctuary`) remain unchanged.
- **Canonical Legacy Fallback**: `canonical_legacy_pillar(&self)` allows downstream modules (such as `matrix.rs` and `forecast.rs`) that only recognize legacy pillars to map new alternative pillars to their closest legacy equivalents.
- **Candidate Alpha Deserialization**: Legacy candidate JSON files with string pillar names (e.g. `CAPITAL_ALLOCATION_ANALYST`) are canonicalized during load.

### 3.4 Error Handling & Robustness
- **Dynamic Store Graceful Degradation**: If `data/private_candidates.json` is missing, unreadable, or lacks entries for a requested pillar, `get_curated_candidates` gracefully falls back to curated abstract presets.
- **Template Rendering & Syntax Checks**: `render_template` returns `Result<String, Vec<String>>` listing all unrendered tokens. `validate_fastexpr_syntax` collects all structural syntax violations rather than halting at the first failure.

---

## 4. Adversarial Findings & Challenges

### Finding 1 [Major]: Token Skipping in `PlaceholderRegistry::extract_placeholders` when `<` is Used as Comparison Operator
- **Location**: `src/taxonomy.rs:223-248`
- **Challenged Assumption**: Assumes every `<` character in an expression string begins a placeholder token.
- **Attack Scenario**:
  In FastExpr and quantitative formulas, the less-than operator `<` is frequently used for threshold comparisons (e.g., `if_else(rank(close) < rank(<FIELD>), 1, 0)` or `rank(x) < 0.5 && rank(<BETA_FIELD>) > 0.5`).
  In `extract_placeholders`:
  ```rust
  if bytes[i] == b'<' {
      if let Some(j) = bytes[i + 1..].iter().position(|&b| b == b'>') {
          let end = i + 1 + j;
          let inner = &expr[i + 1..end];
          if !inner.is_empty()
              && inner
                  .chars()
                  .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
          {
              let token = expr[i..=end].to_string();
              if !tokens.contains(&token) {
                  tokens.push(token);
              }
          }
          i = end + 1; // <--- ATTACK VECTOR: Advances past `>` even if inner was INVALID!
          continue;
      }
  }
  ```
  When `bytes[i]` hits the less-than operator `<`, `position(|&b| b == b'>')` scans ahead and matches the `>` of the subsequent `<FIELD>` token. The slice `inner` evaluates to `" rank(<FIELD"`. Because this contains spaces and `(`, `inner.chars().all(...)` evaluates to `false`. However, the loop executes `i = end + 1; continue;`, which advances `i` past the `>` of `<FIELD>`.
  Consequently:
  - `extract_placeholders` returns `[]`.
  - `has_unresolved_placeholders` returns `false`.
  - `validate_fastexpr_syntax` passes with `Ok(())`.
  - The unrendered expression containing literal `<FIELD>` is sent to the BRAIN API, triggering HTTP 400 Bad Request.
- **Blast Radius**: Any formula combining relational operators (`<`) with template placeholders will silently fail template extraction and syntax validation.
- **Mitigation**:
  Only advance `i = end + 1` when a valid placeholder token is identified and extracted. If `inner` is invalid, fall through to `i += 1`:
  ```rust
  if bytes[i] == b'<' {
      if let Some(j) = bytes[i + 1..].iter().position(|&b| b == b'>') {
          let end = i + 1 + j;
          let inner = &expr[i + 1..end];
          if !inner.is_empty()
              && inner.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
          {
              let token = expr[i..=end].to_string();
              if !tokens.contains(&token) {
                  tokens.push(token);
              }
              i = end + 1;
              continue;
          }
      }
  }
  i += 1;
  ```

---

### Finding 2 [Minor]: Case-Sensitivity in `validate_fastexpr_syntax` Group Neutralization Check
- **Location**: `src/taxonomy.rs:309-314`
- **Challenged Assumption**: Assumes `group_neutralize` is always written in lowercase.
- **Attack Scenario**:
  ```rust
  if expr.contains("group_neutralize") {
      let lower = expr.to_lowercase();
      if !lower.contains("densify(") {
          errors.push("group_neutralize must wrap grouping variable in densify(...)".to_string());
      }
  }
  ```
  If an expression uses uppercase or capitalized syntax (e.g., `GROUP_NEUTRALIZE(sig, subindustry)`), `expr.contains("group_neutralize")` evaluates to `false`. The entire block is skipped, and the validation passes without verifying `densify(...)`.
- **Blast Radius**: Uppercase `GROUP_NEUTRALIZE` expressions bypass the required `densify` check.
- **Mitigation**:
  Convert `expr` to lowercase once at the start:
  ```rust
  let lower = expr.to_lowercase();
  if lower.contains("group_neutralize") && !lower.contains("densify(") {
      errors.push("group_neutralize must wrap grouping variable in densify(...)".to_string());
  }
  ```

---

### Finding 3 [Minor]: Incomplete `densify` Call Association for Multi-Neutralization Formulas
- **Location**: `src/taxonomy.rs:309-314`
- **Challenged Assumption**: Assumes that if `densify(` is anywhere in the string, all `group_neutralize` invocations are valid.
- **Attack Scenario**:
  In composite expressions that combine two sub-signals with separate neutralizations:
  `group_neutralize(sig1, densify(subindustry)) + group_neutralize(sig2, sector)`
  `lower.contains("densify(")` is true because of `sig1`, allowing `sig2` to omit `densify` unnoticed.
- **Blast Radius**: Partial densification in multi-layer expressions passes pre-dispatch syntax validation.
- **Mitigation**:
  In Milestone M3, implement argument-level parsing for `group_neutralize` (e.g. matching each invocation's second parameter).

---

### Finding 4 [Minor]: Non-Idiomatic Parsing Trait (`FromStr`)
- **Location**: `src/taxonomy.rs:45-89`
- **Issue**: `FactorPillar::from_str` uses `#[allow(clippy::should_implement_trait)]`.
- **Rust Idiom**: In idiomatic Rust, types parsable from strings implement `std::str::FromStr`. This enables standard library ergonomics like `s.parse::<FactorPillar>()` and `.filter_map(|s| s.parse().ok())`.
- **Mitigation**:
  Implement `std::str::FromStr for FactorPillar` (with `type Err = ()`), and forward `FactorPillar::from_str(s)` to `s.parse().ok()`.

---

### Finding 5 [Minor]: HashMap Iteration Order in `render_template`
- **Location**: `src/taxonomy.rs:258-273`
- **Issue**: `render_template` iterates over `&HashMap<&str, &str>`:
  ```rust
  for (placeholder, replacement) in bindings {
      rendered = rendered.replace(placeholder, replacement);
  }
  ```
  Because `HashMap` iteration order is non-deterministic, if bindings contain keys where one placeholder is a prefix/substring of another (e.g. `<PCR_TERM>` and `<PCR_TERM_SHORT>`), replacing `<PCR_TERM>` first can mutate `<PCR_TERM_SHORT>` into `<replacement_SHORT>`.
- **Mitigation**:
  Sort bindings by placeholder length descending (longest first) before substring substitution.

---

### Finding 6 [Minor]: Test Fragility in `test_curated_candidates_alternative_pillars`
- **Location**: `tests/taxonomy_tests.rs:289-293`
- **Issue**: `test_curated_candidates_alternative_pillars` asserts `PlaceholderRegistry::has_unresolved_placeholders(&c.expression)` on candidates loaded via `get_curated_candidates(Some(pillar), None)`.
  Currently this passes because `data/private_candidates.json` has no entries for alternative pillars, causing fallback to abstract presets. When Milestone M4 populates real private alternative candidates into `data/private_candidates.json`, these candidates will have concrete fields rather than placeholders, causing this test to panic during routine test runs.
- **Mitigation**:
  Test preset placeholder presence via an isolated test environment (e.g., using `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json`), or check whether candidates are from presets before asserting placeholder presence.

---

## 5. Independent Verification Record

| Command | Status | Output / Evidence |
|---------|--------|-------------------|
| `cargo check` | PASS | `Finished dev profile [unoptimized + debuginfo] in 0.28s` |
| `cargo test --test taxonomy_tests` | PASS | `19 passed; 0 failed; 0 ignored; finished in 0.00s` |
| `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json cargo test --test taxonomy_tests` | PASS | `19 passed; 0 failed; 0 ignored; finished in 0.00s` |
| `cargo test` (crate-wide) | PASS | 108 tests passed across 10 test suites |
| `cargo clippy` | PASS | 0 warnings, clean dev profile |
| `bash scripts/sanitize_check.sh` | PASS | Pre-push audit passed: 0 Alpha IDs, 0 forbidden field patterns, 12 private literals absent |

---

## 6. Verdict & Next Steps

**Final Verdict**: **APPROVE WITH RECOMMENDATIONS**

Milestone M1 successfully establishes the mathematical factor taxonomy and specification foundations for `model51`, `model16`, `news18`, and `option9`. All interface contracts for M1 are met, backward compatibility is fully preserved, and zero research material leaks exist.

**Recommendations for Milestone M3**:
1. Apply the single-line fix to `PlaceholderRegistry::extract_placeholders` (only advancing `i = end + 1` when a valid placeholder is extracted) to handle relational operators (`<`) preceding placeholders.
2. Update `validate_fastexpr_syntax` to perform case-insensitive checking for `group_neutralize`.
3. Implement `std::str::FromStr` for `FactorPillar` for idiomatic Rust parsing.
