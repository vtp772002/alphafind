## 2026-10-24T19:31:19Z
You are the Quantitative Developer Worker for Milestone M2 (Field Metadata Extraction & Schema Validator).
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m2_1/

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

You must first read:
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
- /Users/phamtoan/Developer/alphafind/PROJECT.md
- /Users/phamtoan/Developer/alphafind/AGENTS.md
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_1/metadata_api_architecture.md
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_2/schema_validation_theory.md
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/cli_tooling_blueprint.md

Exclusive Write Ownership:
- src/models.rs
- src/client.rs
- src/schema_validator.rs
- src/commands/research.rs
- src/main.rs
- scratch/metadata_tool.py
- tests/metadata_schema_tests.rs

Your Tasks:
1. Extend Data Models (`src/models.rs`):
   - Add `DataFieldType` (`Matrix`, `Vector`) enum with serde serialization/deserialization.
   - Update `DataFieldEntry` to include `data_type` / `field_type`, `category`, `subcategory`, and helper methods `is_vector()`, `is_matrix()`, `requires_vector_folding()`, `requires_backfill()`.
2. Implement BRAIN Client Method (`src/client.rs`):
   - Implement `fetch_dataset_fields(dataset_id: &str, ...)`:
     - Enforce required query params: `instrumentType=EQUITY`, `region=USA`, `delay=1`, `universe=TOP1000`, `dataset.id={dataset_id}`, and `limit=50`.
     - Implement automatic offset-based pagination (`offset += limit` until all records retrieved).
     - Implement exponential backoff for HTTP 429 ($2^{\text{attempt}} + 1.0\text{s}$, max 3 retries).
     - Implement disk caching to `data/fields_{dataset}.json` and `data/fields_alternative.json` (untracked, gracefully handled if absent per AGENTS.md).
3. Implement Schema Validator Engine (`src/schema_validator.rs`):
   - 6-type lattice: `Scalar`, `Matrix`, `Vector`, `Group`, `DensifiedGroup`, `BooleanMatrix`.
   - Validate that `VECTOR` fields (e.g. 50 vector fields in `news18`) cannot be passed to scalar/matrix operators without `vec_avg(...)` or `vec_sum(...)`.
   - Validate low coverage fields (`coverage < 0.50`) require ancestral `ts_backfill(..., d >= 5)`.
   - Validate temporal lookbacks to prevent look-ahead bias ($d \ge 1$, Delay 1 settings).
   - Provide built-in schema catalog containing the verified 235 fields for `model51`, `model16`, `news18`, and `option9` as fallback if offline.
4. Implement CLI Command & Script:
   - Add `Fields` (or `Metadata`) subcommand in `src/main.rs` & `src/commands/research.rs`: `alphafind fields --dataset <id> [--refresh] [--validate <expr>]`.
   - Add complementary Python utility in `scratch/metadata_tool.py` for Python workflows.
5. Create Test Suite (`tests/metadata_schema_tests.rs`):
   - Test deserialization of Matrix and Vector field entries.
   - Test schema validator on valid and invalid FastExpr strings (passing expression, vector type mismatch error, missing backfill warning, look-ahead guard).
   - Test offline fallback catalog and wiremock mock API pagination if applicable.
6. Verification:
   - Run `cargo check`, `cargo test --test metadata_schema_tests`, and `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json cargo test`.
   - Run `bash scripts/sanitize_check.sh` to ensure pre-push sanitization audit passes with 0 leaks.
   - Document commands and results in your handoff report.

Deliverables:
- Write comprehensive handoff report to /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m2_1/handoff.md following the 5-component structure (Observation, Logic Chain, Caveats, Conclusion, Verification).
- Send completion message to parent.
