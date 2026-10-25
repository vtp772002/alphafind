# BRAIN `/data-fields` API & Metadata Architecture Specification
**Milestone M2: Field Metadata Extraction & Schema Validator**  
**Author**: API & Metadata Architect (`explorer_m2_1`)  
**Target Repository**: `alphafind`  
**Date**: 2026-10-24  
**Status**: APPROVED ARCHITECTURAL DESIGN  

---

## 1. Executive Summary & Scope

Milestone M2 establishes the foundational data ingestion and schema validation layer for WorldQuant BRAIN Alternative Data research within `alphafind`. To eliminate HTTP 400 simulation errors and alpha rejection caused by incorrect field tensor shapes, this architecture formalizes:
1. **The exact HTTP query protocol** for BRAIN's `/data-fields` endpoint, cataloging all mandatory query parameters, the strict pagination limit upper bound, and the precise dataset filtering syntax.
2. **Empirical census and dimensional classification** across the four premier alternative datasets (`model51`, `model16`, `news18`, and `option9`), confirming the exact 235-field surface and segregating `MATRIX` (2D) vs `VECTOR` (3D/event) fields.
3. **Data models design in `src/models.rs`**, extending `DataFieldEntry` with typed enums, category hierarchies, coverage metrics, and FastExpr folding helpers.
4. **Disk caching and Git hygiene architecture**, providing deterministic local serialization to `data/fields_{dataset}.json` and `data/fields_alternative.json` in compliance with `AGENTS.md` and `.gitignore`.
5. **Client API method in `src/client.rs`**, specifying `fetch_dataset_fields` with automatic offset pagination, HTTP 429 exponential backoff, and fail-closed error handling.

---

## 2. BRAIN `/data-fields` Endpoint Protocol & Empirical Ground Truth

### 2.1 Endpoint Overview
- **URL**: `https://api.worldquantbrain.com/data-fields`
- **Method**: `GET`
- **Authentication**: HTTP Basic Auth session cookie (`Cookie: ...`) or Basic Authorization header (`Authorization: Basic ...`).

### 2.2 Mandatory vs Optional Query Parameters
Through empirical combinatorial boundary testing against the live BRAIN API, the parameter requirements to prevent `HTTP 400 Bad Request - ["Invalid query"]` were established:

| Parameter | Type | Required? | Default / Example | Notes & Empirical Verification |
|---|---|---|---|---|
| `instrumentType` | String | **MANDATORY** | `EQUITY` | Omitting returns `HTTP 400 - ["Invalid query"]`. |
| `region` | String | **MANDATORY** | `USA` | Omitting returns `HTTP 400 - ["Invalid query"]`. Must match target market. |
| `delay` | Integer | **MANDATORY** | `1` | Omitting returns `HTTP 400 - ["Invalid query"]`. Delay 1 or Delay 0. |
| `universe` | String | **MANDATORY** | `TOP1000` | Omitting returns `HTTP 400 - ["Invalid query"]`. E.g., `TOP1000`, `TOP500`, `TOP3000`. |
| `limit` | Integer | **MANDATORY** | `50` | Omitting `limit` entirely returns `HTTP 400 - ["Invalid query"]`. **Strict maximum is 50!** |
| `offset` | Integer | Optional | `0` | 0-indexed pagination window offset. Defaults to 0 if omitted. |
| `dataset.id` | String | Optional* | `model51` | ***CRITICAL***: Must use dot-notation `dataset.id=<id>`. |
| `id` | String | Optional | `beta_last_30_days_spy` | Single-field exact match lookup. |
| `category.id` | String | Optional | `model`, `news`, `option` | Higher-level category filter. |
| `subcategory.id`| String | Optional | `model-risk-models` | Subcategory taxonomy filter. |
| `type` | String | Optional | `MATRIX`, `VECTOR` | Filters by tensor dimension at query time. |

> ⚠️ **CRITICAL FINDING: Dataset Query Parameter Syntax**
> - Using `dataset=<dataset_id>` is **silently ignored** by BRAIN's query parser, returning the full global catalog of 4,367 fields rather than filtering.
> - Using `search=<dataset_id>` triggers an unstructured full-text search across titles and descriptions, returning noisy partial matches (e.g., 3,799 fields).
> - **The only valid syntax for dataset filtering is `dataset.id=<dataset_id>`.**

### 2.3 Strict Pagination Upper Bound (`limit <= 50`)
Empirical testing revealed that BRAIN enforces a strict server-side ceiling on the `limit` parameter:
- `limit=50`: **HTTP 200 SUCCESS** (returns up to 50 items).
- `limit=51`: **HTTP 400 Bad Request - `["Invalid query: pagination limit too high."]`**
- `limit=100`: **HTTP 400 Bad Request - `["Invalid query: pagination limit too high."]`**

**Architectural Rule**: The client pagination engine MUST hardcode `const DATA_FIELDS_PAGE_LIMIT: usize = 50;` and never request a page size exceeding 50.

### 2.4 Pagination Protocol
The response payload schema from `/data-fields` is:
```json
{
  "count": 121,
  "results": [ ... ]
}
```
- `count` (`usize`): Total cross-sectional field count satisfying the query filter.
- `results` (`Vec<DataFieldEntry>`): Slice of field records for the current page window.

**Loop Termination Invariants**:
1. Increment `offset += results.len()`.
2. Terminate when `offset >= count`.
3. Terminate when `results.is_empty()`.
4. Fail-closed if `results.is_empty()` while `offset < count` after max retries (detects truncated or corrupted stream).

### 2.5 Rate-Limiting & HTTP 429 Backoff Dynamics
BRAIN enforces API rate limits on `/data-fields` to prevent catalog scraping denial-of-service.

**Behavior**:
- Response: `HTTP 429 Too Many Requests`.
- Header: `Retry-After` (optional, floating-point or integer seconds).

**Backoff Algorithm**:
1. Check for `Retry-After` header. If present and parseable as `f64`, sleep `f64::max(retry_after, 1.0)` seconds.
2. If `Retry-After` is missing, execute exponential backoff with deterministic jitter:
   $$\Delta t = 2^{\min(\text{attempt}, 5)} + 1.0 \text{ seconds}$$
   - Attempt 0: 2.0s
   - Attempt 1: 3.0s
   - Attempt 2: 5.0s
   - Attempt 3: 9.0s
   - Attempt 4: 17.0s
3. **Fail-Closed Threshold**: Cap retries at 5 attempts (`0..5`). If the 5th attempt fails, abort execution and return `anyhow::bail!("BRAIN API rate limit (HTTP 429) exceeded while fetching dataset fields for {dataset_id}")`. Never swallow 429 errors into empty result vectors.

---

## 3. Alternative Dataset Census & Tensor Dimensionality

Direct querying against the authenticated BRAIN API confirmed the following exact metrics for the four target datasets under `region=USA`, `delay=1`, `universe=TOP1000`:

| Dataset ID | Dataset Name | Total Count | MATRIX Count | VECTOR Count | Category | Subcategory |
|---|---|:---:|:---:|:---:|---|---|
| **`model51`** | Systematic Risk Metrics | 16 | 16 (100%) | 0 (0%) | `model` | `model-risk-models` |
| **`model16`** | Fundamental Scores | 24 | 24 (100%) | 0 (0%) | `model` | `model-valuation-models` |
| **`news18`** | Ravenpack News Data | 121 | 71 (58.7%) | 50 (41.3%) | `news` | `news-news-sentiment` |
| **`option9`** | Options Analytics | 74 | 74 (100%) | 0 (0%) | `option` | `option-option-analytics` |
| **TOTAL** | **Alternative Surface** | **235** | **185 (78.7%)** | **50 (21.3%)** | — | — |

### 3.1 FastExpr Compatibility & Folding Rules
FastExpr operates primarily on 2D matrices (Cross-Section $\times$ Time). Passing a 3D `VECTOR` field into matrix operators triggers platform simulation errors.

1. **`MATRIX` Fields (185 fields)**:
   - Represents daily continuous or discrete point-in-time cross-sectional measurements.
   - Compatible directly with unary, binary, cross-sectional, and group neutralization operators:
     ```
     rank(beta_last_60_days_spy)
     group_neutralize(analyst_revision_rank_derivative, densify(subindustry))
     ```

2. **`VECTOR` Fields (50 fields in `news18`)**:
   - Represents intraday event sequences (e.g., individual news stories published on day $t$).
   - **Mandatory Reduction Rule**: MUST be collapsed to scalar/matrix values along the event dimension prior to cross-sectional operations using vector aggregators:
     - `vec_avg(vector_field)`: Mean sentiment across all stories for stock $i$ on day $t$.
     - `vec_sum(vector_field)`: Aggregate news volume or cumulative score.
     - `vec_max(vector_field)` / `vec_min(vector_field)`: Extreme event sentiment shock.
     - `vec_count(vector_field)`: Intraday story frequency.

3. **Missing Value & Look-Ahead Safeguards**:
   - News event streams exhibit discontinuous cross-sectional coverage on days without news for stock $i$.
   - **Mandatory Backfill Rule**: If coverage $< 0.50$ (or on event vectors), wrap in `ts_backfill(vec_avg(...), d)` where $d \in [5, 20]$ to prevent forward look-ahead and avoid `NaN` propagation.

---

## 4. Data Models Architecture (`src/models.rs`)

### 4.1 Type Definitions
To provide robust type safety, `DataFieldType` is modeled as a strongly typed enum with fail-safe fallback:

```rust
use serde::{Deserialize, Serialize};

/// Tensor dimensionality of a BRAIN data field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum DataFieldType {
    /// 2D Matrix tensor: Time x Assets. Directly usable in FastExpr matrix operators.
    #[default]
    Matrix,
    /// 3D Vector tensor: Time x Assets x Events. Requires vector folding (`vec_avg`, etc.).
    Vector,
    /// Fallback for potential new platform types (e.g., TENSOR, SCALAR).
    #[serde(other)]
    Other,
}

impl DataFieldType {
    pub fn is_matrix(&self) -> bool {
        matches!(self, DataFieldType::Matrix)
    }

    pub fn is_vector(&self) -> bool {
        matches!(self, DataFieldType::Vector)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DataFieldType::Matrix => "MATRIX",
            DataFieldType::Vector => "VECTOR",
            DataFieldType::Other => "OTHER",
        }
    }
}
```

### 4.2 Extended `DataFieldEntry`
`DataFieldEntry` is extended to capture all attributes returned by BRAIN `/data-fields` and disk caches, with camelCase serde mapping and default fallbacks:

```rust
/// Category / subcategory info in DatasetEntry and DataFieldEntry
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DatasetCategoryInfo {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Field-level usage statistics and metadata returned by GET /data-fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[serde(default)]
    pub themes: Option<Vec<String>>,
    #[serde(default)]
    pub date_created: Option<String>,
}

impl DataFieldEntry {
    /// Returns the dataset ID string if present.
    pub fn dataset_id(&self) -> Option<&str> {
        self.dataset.as_ref().and_then(|d| d.id.as_deref())
    }

    /// Returns true if the field is a VECTOR requiring folding.
    pub fn is_vector(&self) -> bool {
        matches!(self.field_type, Some(DataFieldType::Vector))
    }

    /// Returns true if the field is a standard MATRIX.
    pub fn is_matrix(&self) -> bool {
        matches!(self.field_type, Some(DataFieldType::Matrix))
    }

    /// Indicates whether the expression builder must fold this field into a matrix.
    pub fn requires_vector_folding(&self) -> bool {
        self.is_vector()
    }

    /// Indicates whether missing value backfill is mandatory given a coverage threshold.
    pub fn requires_backfill(&self, threshold: f64) -> bool {
        self.coverage.map(|c| c < threshold).unwrap_or(false)
    }
}
```

### 4.3 Disk Cache Envelope (Optional Structured Wrapper)
For catalog exports that include provenance metadata:
```rust
/// Envelope container for cached dataset fields on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetFieldCacheEnvelope {
    pub dataset_id: String,
    pub fetched_at_unix_secs: u64,
    pub instrument_type: String,
    pub region: String,
    pub universe: String,
    pub delay: i32,
    pub total_count: usize,
    pub fields: Vec<DataFieldEntry>,
}
```

---

## 5. Disk Caching & Storage Architecture

### 5.1 Storage Layout & File Naming Conventions
- Per-dataset cache files: `data/fields_{dataset_id}.json`
  - Examples: `data/fields_model51.json`, `data/fields_model16.json`, `data/fields_news18.json`, `data/fields_option9.json`.
- Consolidated alternative dataset cache: `data/fields_alternative.json`.

### 5.2 Serialization Format
Cache files are stored as pretty-printed JSON arrays (`Vec<DataFieldEntry>`). This guarantees:
1. **100% Backwards Compatibility**: Existing code in `src/crowd_allocation.rs` (`load_local_field_datasets`) iterates over `data/fields_*.json` and deserializes arrays of field objects.
2. **Schema Uniformity**: Exactly matches the structure already established in `data/fields_option.json` and `data/fields_nws12.json`.

### 5.3 Git Hygiene & Research Confidentiality Compliance
Per `AGENTS.md` and `.gitignore`:
- Rule in `.gitignore` (line 71): `data/fields_*.json` is untracked and excluded from version control.
- `data/fields_alternative.json` matches the glob `data/fields_*.json` and will never be accidentally committed.
- No live candidate formulas or proprietary Alpha IDs are contained in field metadata files.

---

## 6. Client API Method Specification (`src/client.rs`)

### 6.1 Function Signature
```rust
impl BrainClient {
    /// Maximum page limit permitted by BRAIN /data-fields endpoint.
    pub const DATA_FIELDS_PAGE_LIMIT: usize = 50;

    /// Fetches all data fields for a specified dataset with automatic pagination,
    /// exponential backoff on HTTP 429 rate limits, and fail-closed error handling.
    pub async fn fetch_dataset_fields(
        &self,
        dataset_id: &str,
        instrument_type: &str,
        region: &str,
        delay: i32,
        universe: &str,
    ) -> Result<Vec<DataFieldEntry>>;

    /// Convenience wrapper using standard quantitative production settings
    /// (`instrument_type="EQUITY"`, `region="USA"`, `delay=1`, `universe="TOP1000"`).
    pub async fn fetch_dataset_fields_default(
        &self,
        dataset_id: &str,
    ) -> Result<Vec<DataFieldEntry>> {
        self.fetch_dataset_fields(dataset_id, "EQUITY", "USA", 1, "TOP1000").await
    }
}
```

### 6.2 Detailed Implementation Logic

```rust
pub async fn fetch_dataset_fields(
    &self,
    dataset_id: &str,
    instrument_type: &str,
    region: &str,
    delay: i32,
    universe: &str,
) -> Result<Vec<DataFieldEntry>> {
    self.authenticate().await?;

    let url = format!("{}/data-fields", self.base_url);
    let delay_str = delay.to_string();
    let limit_str = Self::DATA_FIELDS_PAGE_LIMIT.to_string();

    let mut all_fields: Vec<DataFieldEntry> = Vec::new();
    let mut offset: usize = 0;

    loop {
        let offset_str = offset.to_string();
        let query_params = [
            ("instrumentType", instrument_type),
            ("region", region),
            ("delay", delay_str.as_str()),
            ("universe", universe),
            ("dataset.id", dataset_id),
            ("limit", limit_str.as_str()),
            ("offset", offset_str.as_str()),
        ];

        let mut page_succeeded = false;

        for attempt in 0..5 {
            let headers = self.auth_headers()?;
            let resp = self
                .client
                .get(&url)
                .query(&query_params)
                .headers(headers)
                .send()
                .await;

            match resp {
                Ok(response) => {
                    let status = response.status();

                    if status.as_u16() == 429 {
                        if attempt == 4 {
                            anyhow::bail!(
                                "BRAIN API rate limit (HTTP 429) exceeded fetching fields for dataset '{}' at offset {}",
                                dataset_id,
                                offset
                            );
                        }
                        let wait = response
                            .headers()
                            .get(RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(2.0f64.powi(attempt) + 1.0);
                        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                        continue;
                    }

                    if !status.is_success() {
                        let err_text = response.text().await.unwrap_or_default();
                        anyhow::bail!(
                            "Failed to fetch data-fields for dataset '{}' (HTTP {}): {}",
                            dataset_id,
                            status,
                            err_text
                        );
                    }

                    let page_data: DataFieldListResponse = response
                        .json()
                        .await
                        .context("Failed to deserialize DataFieldListResponse JSON")?;

                    let total_count = page_data.count;
                    let fetched_count = page_data.results.len();

                    all_fields.extend(page_data.results);
                    offset += fetched_count;

                    // Loop termination condition
                    if offset >= total_count || fetched_count == 0 {
                        return Ok(all_fields);
                    }

                    page_succeeded = true;
                    break;
                }
                Err(e) => {
                    if attempt == 4 {
                        return Err(e).context(format!(
                            "Network connection failed fetching dataset '{}' fields at offset {}",
                            dataset_id, offset
                        ));
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        if !page_succeeded {
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

## 7. Verification & Testing Strategy

### 7.1 Unit Tests (`tests/models_tests.rs`)
1. **Serialization & Deserialization of `DataFieldType`**:
   - `MATRIX` and `VECTOR` parse to their respective enum variants.
   - Unrecognized strings gracefully map to `DataFieldType::Other`.
2. **`DataFieldEntry` Deserialization from JSON**:
   - Parse sample objects from all four datasets.
   - Verify `type: "VECTOR"` triggers `entry.is_vector() == true` and `entry.requires_vector_folding() == true`.
   - Verify `coverage < 0.50` triggers `entry.requires_backfill(0.50) == true`.
3. **Array Caching Round-Trip**:
   - Serialize `Vec<DataFieldEntry>` to JSON and deserialize back; assert field-by-field equality.

### 7.2 Mock API Integration Tests (`tests/mock_api_tests.rs`)
1. **Query Parameter Validation**:
   - Verify that Wiremock receives query parameters `instrumentType=EQUITY`, `region=USA`, `delay=1`, `universe=TOP1000`, `dataset.id=model51`, `limit=50`, `offset=0`.
2. **Multi-Page Pagination**:
   - Mock a dataset of 121 fields (like `news18`) across 3 pages (50, 50, 21 items).
   - Assert `fetch_dataset_fields` automatically aggregates all 121 records.
3. **HTTP 429 Exponential Backoff**:
   - Mock HTTP 429 with `Retry-After: 0.1` on attempt 0, followed by HTTP 200 on attempt 1.
   - Assert clean recovery and successful data return.
4. **Fail-Closed on Terminal 429**:
   - Mock persistent HTTP 429 across 5 attempts; verify client returns explicit error instead of silent empty results.

---

## 8. Summary of Downstream Contracts (M2 ↔ M3 / M4)

1. **For M3 Expression Builder**:
   - Query field metadata from cache (`data/fields_{dataset}.json`).
   - If `field.is_vector()` is true: wrap in `vec_avg(field)` or `vec_sum(field)`.
   - If `field.requires_backfill(0.50)` is true: wrap in `ts_backfill(field, 5)`.
2. **For M4 Simulation & Evaluator**:
   - Use `field.user_count` and `field.alpha_count` for crowding diagnostics and factor novelty scoring.
