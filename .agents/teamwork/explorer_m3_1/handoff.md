# Handoff Report: Expression Engine Architect (Milestone M3)

## 1. Observation
1. **Factor Taxonomy & Economic Hypotheses (`docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`)**:
   - `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md:27-42` defines the four alternative factor pillars: `SystematicRisk` (`model51`, 16 matrix fields), `FundamentalScores` (`model16`, 24 matrix fields), `NewsSentiment` (`news18`, 71 matrix + 50 vector fields), and `OptionsAnalytics` (`option9`, 74 matrix fields).
   - `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md:304-307` specifies the vector folding rule: "Direct arithmetic between a `VECTOR` and a `MATRIX` or `SCALAR` causes a fatal syntax failure: $\mathbf{V}_{i, t} + 1.0 \implies \text{\textbf{HTTP 400 Bad Request (Type Mismatch)}}$. All `VECTOR` fields must be folded into 2D matrices using `vec_avg(...)` or `vec_sum(...)` before any downstream operators."
   - `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md:400` documents the option9 horizon naming invariant: "Field identifiers in `option9` use exact integer horizons without the 'd' character (e.g., `pcr_vol_20`, `pcr_oi_30`, `forward_price_60`, `call_breakeven_30`). Using `pcr_vol_20d` results in an unknown field error."
   - `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md:557-560` documents the group densification invariant: "Passing raw groups into `group_neutralize` or `group_rank` causes runtime memory exhaustion and 504 Gateway Timeouts. Mandatory Rule: Always wrap grouping variables: `densify(subindustry)`."
   - `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md:491-520` specifies the canonical multi-layer fusion framework: "Anchor (Structural Core: `model51` / `model16`) + Catalyst (Timing Trigger: `news18` / `option9`) + Friction Dampener (`- 0.05 * (rank(ts_delta(close, 3)) - 0.5)`) + Group Neutralization (`group_neutralize(..., densify(group))`."

2. **Schema Validator Engine (`src/schema_validator.rs`)**:
   - `src/schema_validator.rs:24-42` establishes the 6-type semantic lattice: `Scalar`, `Matrix`, `Vector`, `Group`, `DensifiedGroup`, `BooleanMatrix`, `Unknown`, `TypeError`.
   - `src/schema_validator.rs:60-101` defines diagnostic codes including `ErrVecTypeMismatch`, `ErrMissingBackfill`, `ErrNegativeLookback`, `ErrUndensifiedGroup`, `ErrGroupTypeMismatch`, `ErrUnknownField`, `ErrSyntaxMalformed`, `ErrLookaheadDelay`, `WarnSubuniverseRisk`, `WarnUnnormalizedPrice`, `WarnHorizonSyntax`.
   - `src/schema_validator.rs:264-710` implements the built-in offline catalog of 235 verified fields across `model51` (16 matrix), `model16` (8 Tier 1 derivatives with 100% coverage + 16 pentagons with ~30-38% coverage), `news18` (71 matrix + 50 vector fields), and `option9` (74 matrix fields across 12 horizons).
   - `src/schema_validator.rs:1806-1816` strictly verifies that `group_neutralize`'s second argument must evaluate to `FastExprType::DensifiedGroup` or a call to `densify(...)`. If not, it fails validation with `DiagnosticCode::ErrUndensifiedGroup`.
   - `src/schema_validator.rs:1856-1878` and `1934-1955` enforce strictly positive causal lookbacks ($d \ge 1$) for time-series operators (`ts_backfill`, `ts_delay`, `ts_decay_linear`, `ts_delta`).
   - `src/schema_validator.rs:1991-1998` detects unnormalized breakeven and forward price levels passed to `rank()` without spot `close` normalization (`/ close`).
   - `src/schema_validator.rs:1225-1383` implements `SchemaValidator::validate(&self, expression: &str) -> ValidationReport`, returning `is_valid: bool`, errors, warnings, and extracted field metadata.

3. **CLI & Screener Interfaces (`src/main.rs`, `src/screener.rs`, `src/commands/research.rs`)**:
   - `src/main.rs:12-329` specifies CLI subcommands using `clap`. Subcommands include `Check`, `Screen`, `Sim`, `Tune`, `Fields`, `Blueprint`, etc.
   - `src/screener.rs:150-207` implements the bounded multi-worker screener consuming `CandidateAlpha` structs through a bounded `tokio::sync::mpsc` channel, holding a per-account semaphore permit (`DEFAULT_WORKERS_PER_ACCOUNT = 3`).
   - `src/commands/research.rs:1095-1109` shows how `SchemaValidator` is invoked from CLI commands:
     ```rust
     let validator = crate::models::schema_validator::SchemaValidator::new().with_delay(delay);
     let report = validator.validate(expr);
     report.print_diagnostics();
     ```

4. **Git Hygiene & Sanitizer Rules (`scripts/sanitize_check.sh`, `data/sanitize_field_patterns.txt`)**:
   - `scripts/sanitize_check.sh:176-196` scans tracked files against `data/sanitize_field_patterns.txt`.
   - `data/sanitize_field_patterns.txt:4` contains the exact pattern `unsystematic_risk_last`.
   - `src/schema_validator.rs:313` dynamically formats this field identifier (`format!("unsystematic_{}_last_{}_days", "risk", h)`) to prevent false-positive sanitizer trips.

---

## 2. Logic Chain
1. From **Observation 1 & 2**, alternative data expressions require strict multi-layer transformations:
   - Vector fields (`news18`) require `vec_avg(...)` or `vec_sum(...)` folding before arithmetic or time-series operations (Observation 1, 2).
   - Low-coverage fields (<50% coverage, such as `model16` pentagon scores and news sentiment) require ancestral `ts_backfill(..., d >= 5)` to satisfy `SchemaValidator` and prevent sub-universe test failures (Observation 1, 2).
   - Group neutralization targets (`subindustry`, `industry`, `sector`) require explicit wrapping in `densify(...)` to avoid memory exhaustion and `ErrUndensifiedGroup` errors (Observation 1, 2).
   - Options analytics price levels (`call_breakeven`, `put_breakeven`, `forward_price`) require normalization by spot `close` to avoid unnormalized price warnings (Observation 1, 2).
   - Options horizons must be formatted as raw integers without trailing 'd' (Observation 1, 2).
2. From **Observation 3**, the existing screener (`src/screener.rs`) consumes `CandidateAlpha` structs with fields `name`, `expression`, `universe`, `decay`, `neutralization`, `truncation`, and `pillar`. Dispatched expressions that fail platform syntax validation cause immediate HTTP 400 Bad Request simulation errors, halting screening runs.
3. Therefore, the Expression Builder (`src/expression_builder.rs`) must:
   - Implement a modular 4-layer generation pipeline:
     * Layer 1: Core alternative signal selection (`model51`, `model16`, `news18`, `option9`) with automatic vector folding and missing value handling.
     * Layer 2: Dynamic non-linear transforms (`signed_power`, `rank`, `winsorize`, `ts_decay_linear`).
     * Layer 3: Group neutralization across `SUBINDUSTRY`, `INDUSTRY`, and `SECTOR` with mandatory `densify()` wrapper.
     * Layer 4: Liquidity and short-term reversal dampeners (`- c * (rank(ts_delta(close, d)) - 0.5)`).
   - Feature an automated pre-dispatch syntax and schema verification loop coupled with `SchemaValidator::validate`.
   - Implement automated diagnostic repairs for detected hazards (wrapping unfolded vectors in `vec_avg`, wrapping groups in `densify`, adding `ts_backfill(..., 15)`, stripping trailing 'd', normalizing price levels by `/ close`).
   - Guarantee that 100% of emitted `GeneratedExpression` instances have `is_valid == true` with 0 errors.
4. From **Observation 3 & 4**, to support CLI invocation, research workflows, and automated testing without leaking private candidate formulas:
   - A `Generate` subcommand must be integrated into `src/main.rs` (`alphafind generate --dataset <id> --pillar <pillar> -n <max> [--validate]`).
   - A standalone Python utility must be placed in `scratch/expression_builder.py` with an embedded AST validator.
   - A dedicated unit and integration test suite must be placed in `tests/expression_builder_tests.rs`.
   - All code generation must respect `data/sanitize_field_patterns.txt` and `AGENTS.md` git hygiene policies.

---

## 3. Caveats
1. **Network Disconnected Simulation**: This milestone is purely an architectural design and code structure milestone. Platform API simulations (Milestone M4) require active network access and valid BRAIN session credentials.
2. **Field Catalog Completeness**: While the built-in catalog in `src/schema_validator.rs` contains 235 verified fields across the 4 alternative datasets, newly added BRAIN fields require running `alphafind fields --refresh` to populate local `data/fields_*.json` caches.
3. **Execution Decay vs Formula Decay**: The formula-level dampener `ts_decay_linear(sig, d)` operates on the alpha tensor cross-sectionally, whereas simulation execution decay (`decay: 16` to `22`) controls portfolio weight turnover in BRAIN's backtest engine. Both should be tuned harmoniously.

---

## 4. Conclusion
1. The architectural design for Milestone M3 is complete and documented in `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/expression_engine_architecture.md`.
2. The design fully specifies:
   - Modular 4-layer generation pipeline across `model51`, `model16`, `news18`, `option9`, and Cross-Sanctuary multi-pillar combinators.
   - Pre-dispatch automated syntax & schema verification via `SchemaValidator::validate`.
   - Deterministic auto-repair pipeline ensuring 100% valid FastExpr output with 0 errors.
   - CLI command design in `src/main.rs` and handler in `src/commands/research.rs`.
   - Standalone Python utility design in `scratch/expression_builder.py`.
   - Test suite specification for `tests/expression_builder_tests.rs`.
3. The design is actionable, robust against all identified platform failure modes, and ready for code implementation by the builder agents.

---

## 5. Verification Method
To independently verify this architectural specification:
1. **Inspect Architecture Document**:
   - Verify `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/expression_engine_architecture.md` covers all 4 layers, vector folding rules, missing value backfilling, densified neutralization, reversal dampening, pre-dispatch verification, CLI design, Python utility, and test suite.
2. **Inspect Existing Validation Invariants**:
   - Run `cargo test --test stress_schema_validator_tests` to verify the underlying schema validator rules that the expression builder couples with.
3. **Inspect Sanitizer & Git Hygiene**:
   - Run `bash scripts/sanitize_check.sh` to confirm no private literals or tracked field pattern leaks exist in the working directory.
