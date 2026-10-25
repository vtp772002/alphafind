# Challenge Report: Milestone M2 — Metadata Extraction & WireMock Stress Challenger

**Challenger**: Empirical Challenger 2 (`challenger_m2_2`)  
**Target**: Milestone M2 (Field Metadata Extraction & Schema Validator)  
**Parent Agent**: Orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`)  
**Date**: 2026-10-24T19:54:45Z  

---

## Challenge Summary

**Overall risk assessment**: **LOW**

The metadata extraction client (`BrainClient::fetch_dataset_fields`), WireMock mock API pagination harness, rate-limit (HTTP 429) backoff handler, and built-in 235-field catalog invariants have undergone adversarial stress-testing. 

Across 16 newly constructed stress tests in `tests/stress_metadata_tests.rs` (bringing the total project test suite to 139 passing tests), the implementation demonstrated robust resilience against:
1. Pagination boundary anomalies (empty datasets, single-page exact limits, off-by-one 51-item pages, premature empty pages preventing infinite loops, and multi-page sequence tracking).
2. WireMock HTTP 429 rate limits, `Retry-After` header extraction, mid-pagination retries without state corruption, and terminal fail-closed behavior on 5 consecutive failures.
3. Fail-closed error handling on HTTP 500, HTTP 401, and malformed non-JSON payloads.
4. Catalog invariant completeness across all 235 fields in `model51` (16), `model16` (24), `news18` (121), and `option9` (74) with zero duplicate identifiers and case-insensitive lookups.
5. Strict Git hygiene and sanitization audit compliance (`scripts/sanitize_check.sh`).

---

## Challenges

### [Low] Challenge 1: Empty & Whitespace Formula Pass-Through in SchemaValidator
- **Assumption challenged**: The Schema Validator rejects syntactically vacuous expressions before simulation dispatch.
- **Attack scenario**: Submitting an empty string `""` or whitespace string `"   \n\t  "` to `SchemaValidator::validate(...)`.
- **Blast radius**: Low for M2, because `SchemaValidator` is primarily a semantic type and dimensionality checker. If passed downstream directly to the simulation API, BRAIN would reject with HTTP 400 (`"Regular expression cannot be empty"`).
- **Empirical Observation**: `tokenize("")` yields 0 tokens and 0 statements; because the error list is empty, `validate` evaluates `is_valid: errors.is_empty()` which evaluates to `true` with `extracted_fields: []`.
- **Mitigation**: Add an explicit check in `SchemaValidator::validate` or in the upcoming Milestone M3 FastExpr Syntax Validator: `if expression.trim().is_empty() { errors.push(ValidationError { code: DiagnosticCode::ErrSyntaxMalformed, ... }); }`.

### [Low] Challenge 2: Static Labeling of Extracted Vector Fields as "(folded)" in CLI Display
- **Assumption challenged**: Terminal diagnostic reports accurately reflect the folding status of extracted vector fields.
- **Attack scenario**: Submitting an invalid unfolded vector expression (e.g., `group_neutralize(rank(nws18_ssc), densify(subindustry))`) to `alphafind fields --validate`.
- **Blast radius**: Cosmetic only. The validator correctly emits `DiagnosticCode::ErrVecTypeMismatch` and exits with error code 1.
- **Empirical Observation**: In `src/schema_validator.rs:188`, `type_str` is rendered as `"VECTOR (folded)"` whenever `f.is_vector` is true, even when the field in the expression was not folded.
- **Mitigation**: Render `"VECTOR"` or check `f.is_vector && !has_unfolded_error` when printing extracted field badges.

---

## Stress Test Results

| Test Scenario | Expected Behavior | Actual Behavior | Result |
|---|---|---|---|
| **Empty Dataset Pagination** (`count=0`, `results=[]`) | Returns `Ok(vec![])` without extra requests or panics | Returned empty Vec immediately at offset 0 | **PASS** |
| **Exact 50-Item Page Boundary** (`count=50`, `results.len()=50`) | Fetches 50 items at offset 0; sends exactly 1 request; no redundant call to offset 50 | 1 request made, returned 50 items | **PASS** |
| **Off-by-One 51-Item Pagination** (`count=51`) | Page 1 fetches 50 items (offset 0); Page 2 fetches 1 item (offset 50) | 2 requests made, returned 51 items | **PASS** |
| **Premature Empty Page Termination** (`count=100`, page 2 `results=[]`) | Detects `n_fetched == 0` and terminates without infinite loop | Exited loop cleanly, returned 30 items | **PASS** |
| **Multi-Page Sequence** (`count=125`) | Correct sequential pagination: offset 0 (50), offset 50 (50), offset 100 (25) | 3 requests made, all 125 items returned in order | **PASS** |
| **Mid-Pagination 429 Recovery** (Page 2 hits 429 twice, then 200) | Retries page 2 using backoff; preserves page 1 results; accumulates all 75 items | Successfully recovered; 75 items returned | **PASS** |
| **Terminal 429 Fail-Closed** (5 consecutive 429s) | Fails closed on attempt 5; returns bail error with dataset ID and offset | Returned Err with exact rate limit error | **PASS** |
| **HTTP 500 Immediate Fail-Closed** | Bails immediately on 500 without retrying non-idempotent GET | Bailed on first attempt with "HTTP 500" | **PASS** |
| **HTTP 401 Immediate Fail-Closed** | Bails immediately on 401 unauthorized | Bailed on first attempt with "HTTP 401" | **PASS** |
| **Malformed Non-JSON Response** | Bails with deserialization context error | Returned Err with deserialization context | **PASS** |
| **Catalog Completeness Audit** (235 fields) | Exactly 16 (model51) + 24 (model16) + 121 (news18) + 74 (option9) = 235 | Verified 235 total fields registered | **PASS** |
| **Catalog Uniqueness Audit** | Zero duplicate field IDs across entire catalog | 235 unique keys; zero collisions | **PASS** |
| **Vector/Matrix Partition Invariant** | Exactly 185 Matrix fields and 50 Vector fields; all vectors require folding | 185 Matrix, 50 Vector verified | **PASS** |
| **Coverage Invariant Audit** | Exactly 16 fields with coverage < 50% in model16 requiring mandatory backfill | Exactly 16 flagged; 8 Tier 1 at 100% | **PASS** |
| **Case-Insensitive Catalog Lookups** | Lookups match irrespective of uppercase, lowercase, or mixed case | Uppercase and mixed case lookups succeed | **PASS** |
| **Option9 Trailing 'd' Diagnostics** | Expressions with `_30d`, `_60d`, `_90d` fail and suggest canonical field | Emitted `ErrUnknownField` with clean suggestion | **PASS** |
| **Causal Lookback Invariants** | Operators with lag $d \le 0$ fail with `ErrNegativeLookback` | All non-positive lookback tests failed closed | **PASS** |

---

## Unchallenged Areas

- **Live BRAIN API Endpoint Authentication & Session Refresh**: Live network calls require valid credentials in `.env` and live platform connectivity. Tested against WireMock in-memory mock server providing identical HTTP transport, status codes, and JSON payload contracts.
- **M3 FastExpr Combinator Engine**: Expression generation algorithms and combinator search heuristics belong to Milestone M3 scope.

---

## Verdict

**APPROVE**

Milestone M2 meets all empirical reliability, pagination robustness, rate-limit recovery, catalog completeness, and fail-closed security criteria.
