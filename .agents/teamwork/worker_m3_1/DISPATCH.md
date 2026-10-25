## 2026-10-24T20:51:42Z
You are worker_m3_1, a teamwork_preview_worker implementing Milestone M3: Automated Expression Builder & Factor Combinator (R3).

Working Directory: /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m3_1
Parent Conversation ID: 8b020a34-8c2d-4255-98f9-544909ff7652

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

MANDATORY FIRST STEP:
Read `/Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md` before starting work.
Also read:
- `/Users/phamtoan/Developer/alphafind/PROJECT.md`
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/expression_engine_architecture.md`
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/handoff.md`
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_2/factor_combinator_specifications.md`
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_2/handoff.md`
- `AGENTS.md` (Git hygiene, concurrency, and platform policies)

Write Ownership:
- Exclusive ownership: `src/expression_builder.rs`
- Module wiring: `src/lib.rs`, `src/main.rs`, and/or `src/commands/research.rs`
- Standalone tool: `scratch/expression_builder.py`
- Test suite: `tests/expression_builder_tests.rs`

Scope of Work:
1. Implement `src/expression_builder.rs`:
   - Multi-layer generation pipeline:
     * Layer 1: Core alternative signal generator across all 4 datasets:
       - `model51` (systematic risk spreads, beta dynamics, idiosyncratic curvature, benchmark decoupling ratio).
       - `model16` (multi-factor acceleration, cashflow efficiency, analyst revisions, with Tier 1 100% coverage derivatives and pentagon backfill `ts_backfill(..., 10)`).
       - `news18` (vector folding via `vec_avg(nws18_ssc)` / `vec_sum`, sentiment divergence, PEAD, backfill `ts_backfill(..., 15)`).
       - `option9` (breakeven directional asymmetry with `/ close` spot normalization, PCR slope term structure, forward price dislocation, integer horizons without trailing 'd').
     * Layer 2: Dynamic non-linear transforms (`signed_power`, `rank`, `winsorize`, `ts_decay_linear`).
     * Layer 3: Group neutralizations across `SUBINDUSTRY`, `INDUSTRY`, `SECTOR` with mandatory `densify()` wrappers (`densify(subindustry)`).
     * Layer 4: Reversal & liquidity dampeners (`- c * (rank(ts_delta(close, d)) - 0.5)` or `signed_power(rank(ts_decay_linear(returns, 5)) - 0.5, 1.5)`).
     * Cross-Sanctuary Combinator: Multi-pillar fusion uniting Economic Anchors (`model51` or `model16`), Risk Catalysts (`news18` or `option9`), and Friction Dampeners.
   - Pre-Dispatch Automated Syntax & Schema Verification:
     * Directly integrate with `crate::schema_validator::SchemaValidator::new()` and call `.validate(&expr)`.
     * Deterministic auto-repair pipeline if any hazardous pattern is generated (e.g., unfold vector -> wrap in `vec_avg`, raw group -> wrap in `densify`, low coverage -> wrap in `ts_backfill`, trailing 'd' -> strip 'd', unnormalized breakeven -> divide by `close`).
     * Guarantee that 100% of emitted `GeneratedExpression` instances have `is_valid == true` with 0 errors.
   - Core Types & API:
     * `ExpressionBuilder`, `BuilderConfig`, `GeneratedExpression`, `AlternativePillar`, `TransformConfig`, `CombinatorGrid`, `CombinatorPreset`.
     * `ExpressionBuilder::generate_batch(&self, dataset: Option<&str>, pillar: Option<AlternativePillar>, count: usize) -> Vec<GeneratedExpression>`.
     * Convert `GeneratedExpression` into `CandidateAlpha` for the screener.
2. CLI Subcommand:
   - Add `Generate` subcommand to `src/main.rs` & `src/commands/research.rs`:
     `alphafind generate [--dataset <id>] [--pillar <pillar>] [--count <N>] [--validate] [--json]`
3. Python Utility:
   - Create `scratch/expression_builder.py` providing an equivalent standalone generator for quick scripting and parameter exploration.
4. Comprehensive Integration Test Suite:
   - Create `tests/expression_builder_tests.rs` with extensive coverage:
     * Single-dataset generation tests for `model51`, `model16`, `news18`, and `option9`.
     * Cross-sanctuary combinator tests.
     * Non-linear transform and parameter grid tests.
     * Group neutralization and `densify()` wrapper tests.
     * Reversal dampener tests.
     * 100% pass rate in `SchemaValidator::validate`.
     * Auto-repair robustness tests.
5. Verification:
   - Run `cargo test --test expression_builder_tests` and ensure all tests pass.
   - Run full `cargo test` to ensure zero regressions across the codebase.
   - Run `bash scripts/sanitize_check.sh` to ensure zero Git leaks of private candidate formulas or field patterns.
   - Write handoff report to `/Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m3_1/handoff.md` and send completion message to parent (`8b020a34-8c2d-4255-98f9-544909ff7652`).
