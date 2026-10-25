# Dispatch for M2 Explorer 2

## 2026-10-24T19:22:58Z
You are the Field Schema & Type Validator Theorist for Milestone M2 (Field Metadata Extraction & Schema Validator).
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_2/
You must first read:
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
- /Users/phamtoan/Developer/alphafind/PROJECT.md
- /Users/phamtoan/Developer/alphafind/AGENTS.md
- /Users/phamtoan/Developer/alphafind/docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/spec_miner_survey_2/survey_specs.md

Your task:
1. Define the formal field schema and validation rules for each alternative dataset:
   - `model51` (16 matrix fields, 4 horizons): all MATRIX, cross-sectional coverage >= 0.93.
   - `model16` (24 matrix fields): 8 rank derivatives (100% coverage) vs 16 pentagon fields (~30% coverage requiring ts_backfill).
   - `news18` (121 fields in Delay 1): 71 MATRIX fields (daily means) vs 50 VECTOR fields (intraday events).
   - `option9` (74 matrix fields): horizons 10d to 1080d, exact field naming (e.g. `pcr_vol_20`).
2. Design the Schema Validator engine:
   - Type compatibility validation: If a field is `VECTOR`, ensure it cannot be passed directly into scalar/matrix arithmetic operators without `vec_avg(...)` or `vec_sum(...)`.
   - Coverage threshold validation: Warn or require `ts_backfill` if `coverage < 0.50` or `dateCoverage < 0.90`.
   - Look-ahead bias prevention: Verify that Delay 1 fields are strictly used for Delay 1 simulation settings, and that no future-referencing aggregations are permitted.

Deliverables:
- Write detailed validation theory report to /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_2/schema_validation_theory.md
- Write /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_2/handoff.md
- Send completion message to parent.
