## 2026-10-24T20:43:56Z
You are the Expression Engine Architect for Milestone M3 (Automated Expression Builder & Factor Combinator).
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/
You must first read:
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
- /Users/phamtoan/Developer/alphafind/PROJECT.md
- /Users/phamtoan/Developer/alphafind/AGENTS.md
- /Users/phamtoan/Developer/alphafind/docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md
- /Users/phamtoan/Developer/alphafind/src/schema_validator.rs

Your task:
1. Design the Expression Builder & Combinator engine (`src/expression_builder.rs`):
   - Multi-layer generation pipeline:
     * Layer 1: Core alternative signal selection (model51, model16, news18, option9) with automatic vector folding (`vec_avg`) and missing value handling (`ts_backfill`).
     * Layer 2: Dynamic non-linear transforms (`signed_power`, `rank`, `winsorize`, `ts_decay_linear`).
     * Layer 3: Group neutralization across SUBINDUSTRY, INDUSTRY, and SECTOR with mandatory `densify()` wrapper.
     * Layer 4: Liquidity and short-term reversal dampeners (e.g. `ts_decay_linear(sig, d)` or subtracting short-term price momentum).
   - Pre-dispatch Automated Syntax & Schema Verification:
     * Integrate with `SchemaValidator::validate` before any simulation payload is generated. Ensure that 100% of generated FastExpr strings pass syntax and schema verification with 0 errors.
2. Design CLI and programmatic interfaces:
   - CLI command in `src/main.rs`: e.g. `alphafind generate --dataset <id> --pillar <pillar> --max-expressions <N> [--validate]`.
   - Standalone Python utility in `scratch/expression_builder.py`.
3. Design test suite in `tests/expression_builder_tests.rs`.

Deliverables:
- Write architecture design to /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/expression_engine_architecture.md
- Write /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m3_1/handoff.md
- Send completion message to parent.
