# Milestone M2 Tooling & CLI Integration Blueprint: Field Metadata Extraction & Schema Validator

**Document Version**: 1.0.0  
**Author**: Tooling & CLI Integration Engineer (`explorer_m2_3`)  
**Target Milestone**: M2 (Field Metadata Extraction & Schema Validator)  
**Parent Conversation ID**: `8b020a34-8c2d-4255-98f9-544909ff7652`  
**Working Directory**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m2_3/`  
**Date**: 2026-10-24  

---

## 1. Executive Summary & Architectural Scope

Milestone M2 operationalizes the alternative data factor taxonomy established in M1 by providing an automated pipeline to extract, cache, inspect, and validate field metadata from WorldQuant BRAIN's `/data-fields` endpoint.

This document delivers the technical implementation blueprint for the Worker. It establishes:
1. **The CLI Interface (`alphafind fields` with alias `alphafind metadata`)**: A production-grade CLI command offering dataset filtering, pagination synchronization, live refresh, high-level summary tables, and inline FastExpr expression validation.
2. **The Rust Programmatic Interface (`src/schema_validator.rs`)**: An in-memory schema validation engine with FastExpr token/AST parsing, a 6-type lattice (`Scalar`, `Matrix`, `Vector`, `Group`, `DensifiedGroup`, `BooleanMatrix`), vector-folding verification, and coverage backfill enforcement.
3. **The Python Script Complementary Interface (`scratch/metadata_tool.py`)**: A self-contained, dependency-light Python utility adhering to `AGENTS.md` git hygiene rules for exploratory extraction, inspection, and verification.
4. **Data Model & API Client Extensions**: Upgrades to `src/models.rs` (`DataFieldType`, `DataFieldEntry`) and `src/client.rs` (`fetch_dataset_fields` with pagination, 429 exponential backoff, and disk persistence to `data/fields_{dataset}.json`).
5. **The Comprehensive Test Suite (`tests/metadata_schema_tests.rs`)**: Unit tests for schema deserialization, WireMock integration tests for paginated `/data-fields` extraction and rate-limit recovery, and comprehensive expression validation test matrices.
6. **Worker Step-by-Step Implementation Guide**: An exact file-by-file coding roadmap for the Worker.

---

## 2. CLI Interface Specification: `alphafind fields`

### 2.1 Subcommand Definition in `src/main.rs`

In `src/main.rs`, the `Commands` enum is extended with the `Fields` subcommand:

```rust
    /// Query, inspect, and validate field metadata from WorldQuant BRAIN
    #[command(alias = "metadata")]
    Fields {
        /// Target Dataset ID (e.g., model51, model16, news18, option9, or all)
        #[arg(short, long)]
        dataset: Option<String>,

        /// Force live re-fetch from BRAIN /data-fields and update disk cache
        #[arg(long)]
        refresh: bool,

        /// Validate a FastExpr expression against field schemas (type check & coverage)
        #[arg(short, long)]
        validate: Option<String>,

        /// Target Universe for metadata query and coverage check (default: TOP1000)
        #[arg(short, long, default_value = "TOP1000")]
        universe: String,

        /// Signal Delay (default: 1)
        #[arg(long, default_value_t = 1)]
        delay: i32,

        /// Target Region (default: USA)
        #[arg(short, long, default_value = "USA")]
        region: String,

        /// Show high-level summary of datasets and field type breakdown
        #[arg(short, long)]
        summary: bool,

        /// Flag low coverage fields (<50% cross-sectional, <90% date coverage)
        #[arg(long)]
        check_coverage: bool,

        /// Export metadata to JSON file
        #[arg(long)]
        export: Option<String>,
    },
```

In `src/main.rs`'s `match cli.command`:
```rust
        Commands::Fields {
            dataset,
            refresh,
            validate,
            universe,
            delay,
            region,
            summary,
            check_coverage,
            export,
        } => {
            alphafind::commands::metadata::cmd_fields(
                dataset,
                refresh,
                validate,
                universe,
                delay,
                region,
                summary,
                check_coverage,
                export,
            )
            .await
        }
```

### 2.2 Command Handler: `src/commands/metadata.rs`

Create `src/commands/metadata.rs` and register it in `src/commands/mod.rs`:
```rust
pub mod metadata;
```

#### Key Capabilities of `cmd_fields`
1. **Validation Mode (`--validate "<expression>"`)**:
   - Parses the expression using `src/schema_validator::SchemaValidator`.
   - Resolves all extracted fields against local cached field metadata (`data/fields_*.json` and `data/fields_alternative.json`).
   - Verifies:
     - All fields exist in the catalog or built-in vocabulary.
     - `VECTOR` fields are strictly folded using `vec_avg(...)` or `vec_sum(...)` before entering matrix arithmetic.
     - Low-coverage fields ($C < 0.50$) have ancestral `ts_backfill(..., d)`.
     - Group neutralizations wrap the grouping variable with `densify(...)`.
     - No look-ahead negative time-lags ($d \le 0$).
   - Returns exit status `Ok(())` on valid expressions, or prints colored diagnostic reports with line/column markers and returns `Err` on type mismatches.

2. **Summary Mode (`--summary`)**:
   - Loads all cached alternative datasets (`model51`, `model16`, `news18`, `option9`).
   - Renders a formatted terminal table:
     ```
     ====================================================================================================
     DATASET ID  NAME                     CATEGORY   MATRIX  VECTOR  TOTAL  AVG COV  STATUS   CACHE AGE
     ====================================================================================================
     model51     Systematic Risk Metrics  Risk       16      0       16     96.3%    🟢 VALID 1.2h ago
     model16     Fundamental Scores       Valuation  24      0       24     61.2%    🟢 VALID 1.2h ago
     news18      RavenPack News Data      News       71      50      121    82.5%    🟢 VALID 1.2h ago
     option9     Options Analytics        Option     74      0       74     98.1%    🟢 VALID 1.2h ago
     ====================================================================================================
     Total Alternative Data Fields: 235 (185 Matrix, 50 Vector) across 4 orthogonal pillars
     ```

3. **Inspection Mode (`--dataset <id>`)**:
   - Filters fields by dataset ID.
   - Prints detailed table with ID, Type (`MATRIX` in Green, `VECTOR` in Magenta), Coverage, Date Coverage, User Count, and Description.
   - Highlights any `VECTOR` fields with clear warnings that folding is required.

4. **Live Refresh Mode (`--refresh`)**:
   - Authenticates `BrainClient` via `get_main_client()`.
   - Invokes `client.fetch_dataset_fields(dataset_id, &universe, delay, &region).await`.
   - Employs pagination (`limit=50`), handles rate-limiting (`429 Retry-After`), and persists results to `data/fields_{dataset_id}.json`.
   - Also merges all alternative fields into `data/fields_alternative.json`.

5. **Coverage Audit Mode (`--check-coverage`)**:
   - Scans target dataset(s) for fields with cross-sectional coverage $< 0.50$ or date coverage $< 0.90$.
   - Explicitly flags `model16` raw/backfilled pentagon fields (`fscore_*` / `fscore_bfl_*`) requiring `ts_backfill`.

---

## 3. Programmatic Interfaces Architecture

### 3.1 Rust Module: `src/schema_validator.rs`

The schema validator engine lives in `src/schema_validator.rs` and is exported in `src/lib.rs` as:
```rust
pub mod schema_validator;
```

#### 3.1.1 Type Lattice
```rust
/// The semantic type lattice for WorldQuant BRAIN FastExpr expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastExprType {
    /// Scalar constant or parameter (e.g., 0.5, 10, "USA")
    Scalar,
    /// Standard 2D cross-sectional daily matrix [Date, Instrument]
    Matrix,
    /// 1D variable-length event-stream array per cell (e.g., intraday news events)
    Vector,
    /// Categorical grouping identifier (e.g., subindustry, industry, sector)
    Group,
    /// Densified grouping matrix guaranteed free of empty bins
    DensifiedGroup,
    /// Boolean conditional mask matrix resulting from comparisons
    BooleanMatrix,
    /// Incompatible or invalid type error state
    TypeError,
}
```

#### 3.1.2 Diagnostics & Error Codes
Following the error taxonomy defined by `explorer_m2_2`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCode {
    /// Direct arithmetic or matrix operator applied to a VECTOR field without folding
    ErrVecTypeMismatch,
    /// Field cross-sectional coverage < 0.50 without required ts_backfill(...)
    ErrMissingBackfill,
    /// Operator with non-causal lag parameter (d <= 0)
    ErrNegativeLookback,
    /// group_neutralize called without densify(...) wrapping grouping variable
    ErrUndensifiedGroup,
    /// Unknown or unmapped field identifier
    ErrUnknownField,
    /// Unclosed parentheses or syntax parse error
    ErrSyntaxMalformed,
    /// Field coverage warning for subuniverse consistency
    WarnSubuniverseRisk,
    /// Option breakeven dollar amount used without close normalization
    WarnUnnormalizedPrice,
    /// Horizon naming syntax discrepancy (e.g., pcr_vol_20d instead of pcr_vol_20)
    WarnHorizonSyntax,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: DiagnosticCode,
    pub field_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: DiagnosticCode,
    pub field_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub expression: String,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub extracted_fields: Vec<ExtractedFieldMeta>,
}
```

#### 3.1.3 `FieldCatalog` Resolver
The `FieldCatalog` stores known field metadata. To guarantee resilience in air-gapped environments, CI pipelines, and unit tests, `FieldCatalog` incorporates:
1. **Hardcoded Platform Invariant Schemas**:
   - `model51`: 16 matrix fields, horizons 30, 60, 90, 360.
   - `model16`: 24 matrix fields (8 rank derivatives with 1.0 coverage, 16 pentagons with ~0.35 coverage).
   - `news18`: 71 matrix fields, 50 vector fields (`nws18_ssc`, `nws18_bee`, `composite_sentiment_score_2`, etc.).
   - `option9`: 74 matrix fields across 12 horizons (10 to 1080).
2. **Dynamic Disk Cache Ingestion**:
   - Scans `data/fields_*.json` and `data/fields_alternative.json`.
   - Overrides or updates live coverage, user counts, and alpha counts when present.

```rust
pub struct FieldCatalog {
    fields: HashMap<String, DataFieldEntry>,
}

impl FieldCatalog {
    pub fn new() -> Self {
        let mut catalog = Self {
            fields: HashMap::new(),
        };
        catalog.load_hardcoded_invariants();
        catalog.load_disk_cache(std::path::Path::new("data"));
        catalog
    }
    
    pub fn get_field(&self, id: &str) -> Option<&DataFieldEntry> {
        self.fields.get(&id.to_lowercase())
    }
    // ...
}
```

#### 3.1.4 FastExpr AST Parser & Validation Rules
The `SchemaValidator` parses FastExpr expressions into a syntax tree and executes 5 validation rules:

1. **Rule 1: Vector Folding Enforcement**:
   - Traverse expression. If token `id` matches a field with `field_type == DataFieldType::Vector`:
   - Inspect immediate parent function:
     - If parent is one of `vec_avg`, `vec_sum`, `vec_max`, `vec_min`, `vec_count`, `vec_std`: VALID (evaluates to `FastExprType::Matrix`).
     - If parent is ANY other function (e.g. `rank`, `ts_delta`, `signed_power`) or an arithmetic operator (`+`, `-`, `*`, `/`):
       Emit `ErrVecTypeMismatch` fatal error!
       Suggestion: `"Wrap with vec_avg(<id>) or vec_sum(<id>)"`.

2. **Rule 2: Low-Coverage Temporal Backfill Enforcement**:
   - If token `id` has cross-sectional coverage $C < 0.50$ (such as `model16` raw pentagons `fscore_growth` at $29.87\%$ or topic sentiment fields `rp_css_*` at $50\%$):
   - Traverse ancestor call stack:
     - If enclosed within `ts_backfill(..., d)` with $d \ge 5$: VALID.
     - Otherwise: Emit `ErrMissingBackfill` (or `WarnSubuniverseRisk` in relaxed mode).
       Suggestion: `"Wrap with ts_backfill(<id>, 10) to prevent sub-universe test failure"`.

3. **Rule 3: Group Neutralization Densification**:
   - If expression invokes `group_neutralize(signal, group)`:
   - Verify the second argument is explicitly wrapped in `densify(...)`.
   - If not, emit `ErrUndensifiedGroup`.
     Suggestion: `"Wrap grouping parameter in densify(group), e.g., densify(subindustry)"`.

4. **Rule 4: Causal Directionality Guard**:
   - For all time-series operators `ts_delay(x, d)`, `ts_delta(x, d)`, `ts_mean(x, d)`, `ts_decay_linear(x, d)`:
   - Check lookback integer $d$:
     - If $d \le 0$: Emit `ErrNegativeLookback` ("Illegal non-causal negative lookback creates look-ahead bias").

5. **Rule 5: Syntax & Bracket Balance**:
   - Enforce balanced `()`, absence of unresolved `<...>` placeholders, and valid identifier characters.

---

### 3.2 Python Script Alternative: `scratch/metadata_tool.py`

To satisfy Requirement R2 and user acceptance criteria while adhering to `AGENTS.md` (which mandates exploratory scripts reside in git-ignored `scratch/`), provide `scratch/metadata_tool.py`.

#### Script Architecture
- **Location**: `scratch/metadata_tool.py`
- **Dependencies**: Python standard library (`sys`, `os`, `json`, `re`, `argparse`, `urllib.request`)
- **CLI Commands**:
  - `python3 scratch/metadata_tool.py --summary`
  - `python3 scratch/metadata_tool.py --dataset news18 --inspect`
  - `python3 scratch/metadata_tool.py --validate "rank(nws18_ssc)"`
  - `python3 scratch/metadata_tool.py --fetch news18 --delay 1 --universe TOP1000`

#### Core Python Validation Logic
```python
import json, re, sys, os

KNOWN_VECTOR_FIELDS = {
    "composite_sentiment_score_2", "earnings_evaluation_sentiment",
    "event_novelty_score_2", "news_impact_projection_score",
    "nws18_acb", "nws18_bam", "nws18_bee", "nws18_ber", "nws18_nip", "nws18_ssc",
    "story_event_record_count"
}

def validate_expression(expr: str):
    errors = []
    warnings = []
    
    # 1. Check balanced parentheses
    if expr.count("(") != expr.count(")"):
        errors.append("Unbalanced parentheses")
        
    # 2. Check densify in group_neutralize
    if "group_neutralize" in expr and "densify(" not in expr:
        errors.append("group_neutralize requires densify(...) wrapper around grouping variable")
        
    # 3. Check vector folding
    for vec_field in KNOWN_VECTOR_FIELDS:
        if re.search(rf"\b{vec_field}\b", expr):
            folded = (f"vec_avg({vec_field})" in expr or 
                      f"vec_sum({vec_field})" in expr or
                      f"vec_max({vec_field})" in expr or
                      f"vec_min({vec_field})" in expr)
            if not folded:
                errors.append(f"Field '{vec_field}' is of type VECTOR but not folded with vec_avg/vec_sum.")
                
    return {"valid": len(errors) == 0, "errors": errors, "warnings": warnings}
```

---

## 4. Data Models & API Client Extensions

### 4.1 Data Structures in `src/models.rs`

Extend `src/models.rs` lines 413-445:

```rust
/// Field data type in WorldQuant BRAIN FastExpr engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DataFieldType {
    Matrix,
    Vector,
}

impl DataFieldType {
    pub fn is_matrix(&self) -> bool {
        matches!(self, Self::Matrix)
    }
    pub fn is_vector(&self) -> bool {
        matches!(self, Self::Vector)
    }
}

/// Field-level usage statistics returned by GET /data-fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFieldEntry {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dataset: Option<DatasetCategoryInfo>,
    #[serde(default)]
    pub category: Option<DatasetCategoryInfo>,
    #[serde(default)]
    pub subcategory: Option<DatasetCategoryInfo>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub delay: Option<i32>,
    #[serde(default)]
    pub universe: Option<String>,
    #[serde(default, rename = "type")]
    pub field_type: Option<DataFieldType>,
    #[serde(default)]
    pub coverage: Option<f64>,
    #[serde(default)]
    pub date_coverage: Option<f64>,
    #[serde(default)]
    pub user_count: Option<i64>,
    #[serde(default)]
    pub alpha_count: Option<i64>,
}
```

### 4.2 Client API Method in `src/client.rs`

Implement `fetch_dataset_fields` in `src/client.rs`:

```rust
    /// Fetches all data fields for a dataset with automatic pagination (50 fields/page)
    /// and exponential backoff on HTTP 429.
    pub async fn fetch_dataset_fields(
        &self,
        dataset_id: &str,
        universe: &str,
        delay: i32,
        region: &str,
    ) -> Result<Vec<DataFieldEntry>> {
        self.authenticate().await?;

        let mut all_fields = Vec::new();
        let mut offset = 0;
        let limit = 50;
        let delay_str = delay.to_string();

        loop {
            let url = format!("{}/data-fields", self.base_url);
            let mut fetched_page = false;

            for attempt in 0..5 {
                let headers = self.auth_headers()?;
                let resp = self
                    .client
                    .get(&url)
                    .query(&[
                        ("instrumentType", "EQUITY"),
                        ("region", region),
                        ("universe", universe),
                        ("delay", delay_str.as_str()),
                        ("dataset.id", dataset_id),
                        ("limit", &limit.to_string()),
                        ("offset", &offset.to_string()),
                    ])
                    .headers(headers)
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.status().as_u16() == 429 => {
                        let wait = r
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(2.0f64.powi(attempt) + 1.0);
                        sleep(Duration::from_secs_f64(wait)).await;
                        continue;
                    }
                    Ok(r) => {
                        let r = r.error_for_status()?;
                        let list: DataFieldListResponse = r.json().await?;
                        let total = list.count;
                        let n_fetched = list.results.len();

                        all_fields.extend(list.results);
                        offset += limit;

                        if offset >= total || n_fetched == 0 {
                            return Ok(all_fields);
                        }
                        fetched_page = true;
                        break;
                    }
                    Err(_) if attempt < 4 => {
                        sleep(Duration::from_secs(2)).await;
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            if !fetched_page {
                anyhow::bail!(
                    "Failed to fetch fields for dataset '{}' at offset {} after 5 retries",
                    dataset_id,
                    offset
                );
            }
        }
    }
```

---

## 5. Comprehensive Test Suite Design: `tests/`

Create `tests/metadata_schema_tests.rs` containing three test suites:

### 5.1 Suite A: Schema Parsing & Serialization Unit Tests
1. `test_data_field_entry_matrix_deserialization`:
   Verifies `"type": "MATRIX"` parses into `Some(DataFieldType::Matrix)`.
2. `test_data_field_entry_vector_deserialization`:
   Verifies `"type": "VECTOR"` parses into `Some(DataFieldType::Vector)`.
3. `test_data_field_entry_missing_type_safely_defaults`:
   Verifies JSON payloads without `"type"` parse cleanly with `field_type: None`.
4. `test_data_field_serialization_preserves_camel_case`:
   Verifies serialization emits `"dateCoverage"`, `"userCount"`, `"alphaCount"`, and `"type"`.

### 5.2 Suite B: Mock API Paginated Extraction (WireMock)
1. `test_mock_fetch_dataset_fields_single_page`:
   Mocks single-page dataset (e.g. `model51`, 16 fields, count=16) and verifies exact query parameters (`instrumentType=EQUITY`, `dataset.id=model51`).
2. `test_mock_fetch_dataset_fields_multi_page_pagination`:
   Mocks multi-page dataset (e.g. `option9`, total 74 fields across 2 pages: 50 on page 1, 24 on page 2). Confirms client fetches both pages and yields 74 records.
3. `test_mock_fetch_dataset_fields_rate_limit_backoff`:
   Mocks 429 response with `Retry-After: 0`, verifies client waits and retries successfully.
4. `test_mock_fetch_dataset_fields_400_bad_request_fails_closed`:
   Mocks 400 Bad Request (simulating invalid universe or missing param), confirms client returns an error.

### 5.3 Suite C: FastExpr Schema & Type Validation Tests
1. `test_validator_passes_matrix_field`:
   `rank(beta_last_30_days_spy) - 0.5` -> Valid.
2. `test_validator_passes_folded_vector_field`:
   `rank(vec_avg(nws18_ssc))` -> Valid.
3. `test_validator_fails_unfolded_vector_field`:
   `rank(nws18_ssc) + 1` -> Invalid (`ErrVecTypeMismatch`).
4. `test_validator_fails_vector_in_binary_op`:
   `nws18_ssc - nws18_bee` -> Invalid (`ErrVecTypeMismatch`).
5. `test_validator_warns_low_coverage_raw_pentagon`:
   `rank(fscore_growth)` -> Warning (`WarnSubuniverseRisk`).
6. `test_validator_passes_low_coverage_with_ts_backfill`:
   `rank(ts_backfill(fscore_growth, 20))` -> Valid.
7. `test_validator_enforces_group_neutralize_densify`:
   - `group_neutralize(rank(close), densify(subindustry))` -> Valid.
   - `group_neutralize(rank(close), subindustry)` -> Invalid (`ErrUndensifiedGroup`).
8. `test_validator_rejects_negative_lookback`:
   `ts_delay(close, -5)` -> Invalid (`ErrNegativeLookback`).
9. `test_validator_rejects_unbalanced_parentheses`:
   `rank(vec_avg(nws18_ssc)` -> Invalid (`ErrSyntaxMalformed`).

---

## 6. Worker Implementation Blueprint & Step-by-Step Plan

The Worker should execute changes in 7 sequential phases:

```
[Phase 1: models.rs] ──> [Phase 2: client.rs] ──> [Phase 3: schema_validator.rs]
                                                              │
[Phase 6: tests/]    <── [Phase 5: main.rs]   <── [Phase 4: commands/metadata.rs]
        │
        ▼
[Phase 7: scratch/metadata_tool.py & Verification]
```

### Phase 1: Update `src/models.rs`
1. Add `DataFieldType` enum (`Matrix`, `Vector`).
2. Add `field_type`, `category`, and `subcategory` to `DataFieldEntry`.
3. Verify existing tests pass with `cargo test --test models_tests`.

### Phase 2: Extend `src/client.rs`
1. Add `fetch_dataset_fields` method with query parameters, pagination, and backoff.
2. Ensure SSRF validation guards and error formatting comply with project standards.

### Phase 3: Create `src/schema_validator.rs`
1. Implement `FastExprType`, `DiagnosticCode`, `ValidationError`, `ValidationWarning`, `ValidationReport`.
2. Implement `FieldCatalog` with hardcoded fallback invariants for all 4 alternative datasets and dynamic `data/` loading.
3. Implement `SchemaValidator` with AST token traversal and validation rules.
4. Export in `src/lib.rs`.

### Phase 4: Create `src/commands/metadata.rs`
1. Implement `cmd_fields(...)` supporting `--summary`, `--dataset`, `--refresh`, `--validate`, `--check-coverage`, `--export`.
2. Register `pub mod metadata;` in `src/commands/mod.rs`.

### Phase 5: Update `src/main.rs`
1. Add `Commands::Fields` with alias `"metadata"`.
2. Route `Commands::Fields` to `cmd_fields` in `match cli.command`.

### Phase 6: Build `tests/metadata_schema_tests.rs`
1. Implement Suite A (deserialization unit tests).
2. Implement Suite B (WireMock paginated extraction tests).
3. Implement Suite C (FastExpr validation tests).
4. Run `cargo test --test metadata_schema_tests`.

### Phase 7: Create `scratch/metadata_tool.py` and Verify
1. Create `scratch/metadata_tool.py` with standalone Python validation and extraction logic.
2. Run `bash scripts/sanitize_check.sh` to ensure no research materials or live IDs are committed.
3. Run full regression test suite: `cargo test`.

---

## 7. Artifact Index & References

- `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`: Alternative data factor taxonomy and mathematical specifications (M1).
- `.agents/teamwork/spec_miner_survey_2/survey_specs.md`: Authoritative BRAIN platform survey and operator specs.
- `.agents/teamwork/explorer_m2_2/schema_validation_theory.md`: Formal schema validation theory and error codes.
- `src/models.rs`: Core platform data structures.
- `src/client.rs`: BRAIN API client.
- `src/taxonomy.rs`: Factor pillar definitions and placeholder registry.
- `tests/stress_taxonomy_tests.rs`: Adversarial stress tests.
