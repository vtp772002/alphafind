# Handoff Report: Tooling & CLI Integration Blueprint (Milestone M2)

**Agent ID**: `explorer_m2_3`  
**Role**: Tooling & CLI Integration Engineer  
**Milestone**: M2 (Field Metadata Extraction & Schema Validator)  
**Deliverable**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/cli_tooling_blueprint.md`  
**Parent Conversation ID**: `8b020a34-8c2d-4255-98f9-544909ff7652`  
**Date**: 2026-10-24T19:30:00Z  

---

## 1. Observation

1. **CLI Structure and Subcommand Registration**:
   - In `src/main.rs:21-289`, the `Commands` enum registers all subcommands using `clap(derive)`. Every subcommand is a short noun or verb (`Auth`, `Sync`, `Screen`, `Sim`, `Radar`, `Tune`, `Impact`, `Matrix`, `Blueprint`, `Shadow`, `Health`, `Allocate`).
   - In `src/main.rs:295-382`, `main()` dispatches each enum variant to a dedicated command handler in `alphafind::commands::*`.
   - In `src/commands/mod.rs:1-15`, subcommands are partitioned into domain-specific modules (`auth`, `portfolio`, `research`, `simulation`, `submission`). Adding a new domain module (such as `src/commands/metadata.rs`) is completely consistent with this architecture.

2. **Data Model Deficiencies**:
   - In `src/models.rs:416-436`, `DataFieldEntry` currently defines:
     ```rust
     pub struct DataFieldEntry {
         pub id: String,
         pub description: Option<String>,
         pub dataset: Option<DatasetCategoryInfo>,
         pub region: Option<String>,
         pub delay: Option<i32>,
         pub universe: Option<String>,
         pub coverage: Option<f64>,
         pub date_coverage: Option<f64>,
         pub user_count: Option<i64>,
         pub alpha_count: Option<i64>,
     }
     ```
     It completely lacks the `type` field (`MATRIX` vs `VECTOR`), as well as `category` and `subcategory`.
   - In existing cached files (e.g., `data/fields_capex.json:20`), the BRAIN API returns `"type": "MATRIX"`, `"dateCoverage": 1.0`, `"coverage": 0.5`.

3. **Existing API Client Query & Pagination Patterns**:
   - In `src/client.rs:1052-1111`, `fetch_data_field` queries single fields using `("id", field_id)` with `limit=5` and `offset=0`.
   - In `src/client.rs:1114-1173`, `fetch_all_datasets` demonstrates the established pagination pattern: a `loop` requesting `limit=50` and `offset`, inspecting `count` vs `results.len()`, with up to 5 retries handling HTTP 429 using `Retry-After`.

4. **Testing Infrastructure**:
   - In `Cargo.toml:21-23`, `dev-dependencies` includes `wiremock = "0.6"`.
   - In `tests/mock_api_tests.rs:10-73`, `wiremock::MockServer` is already used to test authentication, simulation submission, SSRF protection, and rate-limit retries.

5. **Formal Validation Theory (`explorer_m2_2/schema_validation_theory.md`)**:
   - Mined dataset parameters:
     - `model51`: 16 matrix fields, 4 horizons (`beta_last_<T>_days_spy`, etc.).
     - `model16`: 24 matrix fields (8 rank derivatives with 100% coverage; 16 pentagon scores with ~35% coverage requiring `ts_backfill`).
     - `news18`: 121 Delay 1 fields (71 matrix + 50 vector event streams requiring `vec_avg`/`vec_sum`).
     - `option9`: 74 matrix fields across 12 horizons without `'d'` (e.g. `pcr_vol_20`).
   - Defined error codes: `ErrVecTypeMismatch`, `ErrMissingBackfill`, `ErrNegativeLookback`, `ErrUndensifiedGroup`, `ErrUnknownField`, `ErrSyntaxMalformed`.

---

## 2. Logic Chain

1. **Step 1 (CLI Ergonomics)**: Because developers and research scripts interact with field metadata both for bulk inspection and inline expression verification, adding `alphafind fields` with an alias `alphafind metadata` to `src/main.rs` provides a unified, intuitive command conforming to the existing CLI grammar (Observation 1).
2. **Step 2 (Type-Safe Deserialization)**: Adding `DataFieldType` enum (`Matrix`, `Vector`) and mapping `"type"` via `#[serde(rename = "type")]` to `DataFieldEntry` resolves the omission observed in Observation 2 while preserving backward compatibility with legacy snapshots via `#[serde(default)]`.
3. **Step 3 (Client Pagination)**: Adapting the pagination and 429 exponential backoff loop from `fetch_all_datasets` (Observation 3) to `/data-fields` with `dataset.id={dataset_id}` and `instrumentType=EQUITY` ensures robust, paginated metadata synchronization without triggering HTTP 400 or quota depletion.
4. **Step 4 (Programmatic AST Validation)**: By implementing `src/schema_validator.rs` with hardcoded fallback schemas for the 4 alternative datasets and dynamic disk ingestion, the validator operates reliably both in offline unit tests and in live production. Traversing expression tokens detects un-folded `VECTOR` fields and low-coverage fields without `ts_backfill`, eliminating simulation 400 errors before API dispatch (Observation 5).
5. **Step 5 (Unit & Mock Test Suite)**: Utilizing `wiremock` (Observation 4) allows testing single-page and multi-page pagination, rate-limit backoff, and 400 Bad Request fail-closed behaviors deterministically without consuming BRAIN execution quotas.

---

## 3. Caveats

1. **Platform Field Evolution**: Field IDs, counts, and coverage metrics reflect the current platform mine. If BRAIN introduces new alternative datasets or changes existing field types, local cache updates (`alphafind fields --refresh`) or catalog updates will be needed.
2. **Offline Mode**: In environments without network credentials, `alphafind fields --refresh` will fail closed; however, `alphafind fields --validate` and the test suite remain 100% functional due to hardcoded fallback invariant schemas in `FieldCatalog`.
3. **Scratch Script Git Hygiene**: Per `AGENTS.md`, the Python script is placed in `scratch/metadata_tool.py` (which is git-ignored) and uses abstract placeholders only.

---

## 4. Conclusion

The tooling and CLI architecture for Milestone M2 is fully designed, documented, and ready for immediate implementation by the Worker.
- The blueprint `cli_tooling_blueprint.md` provides complete code structures, exact function signatures, and a 7-phase step-by-step implementation plan.
- The design provides seamless dual-interface access: CLI command `alphafind fields` and programmatic Rust module `src/schema_validator.rs`, supplemented by `scratch/metadata_tool.py`.
- The test suite design in `tests/metadata_schema_tests.rs` provides 100% verification coverage across unit deserialization, mock WireMock pagination, and all semantic validation error codes.

---

## 5. Verification Method

1. **Inspect Blueprint Deliverables**:
   ```bash
   test -f /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/cli_tooling_blueprint.md
   wc -l /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/cli_tooling_blueprint.md
   ```
2. **Verify Interface Contracts**:
   - Check that `cli_tooling_blueprint.md` covers CLI options (`--dataset`, `--refresh`, `--validate`, `--summary`, `--check-coverage`, `--export`).
   - Check that `src/schema_validator.rs` type lattice and error taxonomy align with `explorer_m2_2/schema_validation_theory.md`.
3. **Verify Git Hygiene**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
4. **Post-Implementation Worker Verification Commands**:
   - `cargo test --test metadata_schema_tests`
   - `cargo test --test mock_api_tests`
   - `cargo run -- fields --summary`
   - `cargo run -- fields --validate "group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))"`
   - `python3 scratch/metadata_tool.py --summary`
