## 2026-10-24T19:22:58Z

You are the Tooling & CLI Integration Engineer for Milestone M2 (Field Metadata Extraction & Schema Validator).
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/
You must first read:
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
- /Users/phamtoan/Developer/alphafind/PROJECT.md
- /Users/phamtoan/Developer/alphafind/AGENTS.md
- /Users/phamtoan/Developer/alphafind/src/main.rs
- /Users/phamtoan/Developer/alphafind/src/commands/

Your task:
1. Design the tool and CLI interface for metadata extraction and schema validation:
   - CLI command design: e.g. `alphafind fields` or `alphafind metadata` with flags `--dataset <id>`, `--refresh`, `--validate <expression>`.
   - Programmatic Python/Rust script alternative or complementary interface under `src/` or `scratch/` (e.g. `src/schema_validator.rs` or `scratch/metadata_tool.py`).
2. Design the test suite for M2 in `tests/`:
   - Unit tests for schema parsing (`tests/metadata_schema_tests.rs`).
   - Mock API test for paginated `/data-fields` extraction using `wiremock`.
   - Validation test cases: passing expressions, vector type mismatch errors, low coverage warnings.
3. Define the implementation blueprint for the Worker.

Deliverables:
- Write implementation blueprint to /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/cli_tooling_blueprint.md
- Write /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/handoff.md
- Send completion message to parent.
