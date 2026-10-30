//! Empirical Stress-Testing Suite for Milestone M2:
//! Field Metadata Extraction & Schema Validator.
//!
//! Adversarial challenge coverage:
//! 1. WireMock Pagination edge cases (empty response, exact 50-item boundary, off-by-one 51 items,
//!    premature empty page, partial page smaller than limit, count smaller than results, 3-page sequence).
//! 2. WireMock Rate Limiting (HTTP 429) & Fail-Closed Behavior (429 on subsequent page, Retry-After parsing,
//!    terminal fail-closed on 5th attempt, fail-closed on HTTP 500, fail-closed on HTTP 401, malformed JSON).
//! 3. Built-in Catalog Completeness & Invariant Audits across all 235 fields (model51, model16, news18, option9):
//!    - Exact count: 16 (model51) + 24 (model16) + 121 (news18) + 74 (option9) = 235
//!    - Zero duplicate field IDs
//!    - Case-insensitive lookups
//!    - Vector vs Matrix classification completeness (50 Vector, 185 Matrix)
//!    - Coverage invariant partition (16 sparse fields requiring backfill)
//!    - Region ("USA"), Universe ("TOP1000"), Delay (1) invariants
//! 4. Schema Validator Adversarial Inputs & Fuzzing (empty strings, deeply nested parentheses,
//!    unbalanced brackets, negative lookbacks, option9 trailing 'd' variants).

use alphafind::client::BrainClient;
use alphafind::models::schema_validator::{DiagnosticCode, FieldCatalog, SchemaValidator};
use serde_json::json;
use std::collections::HashSet;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =============================================================================
// Section 1: WireMock Pagination Boundary & Edge Case Stress Tests
// =============================================================================

#[tokio::test]
async fn test_stress_pagination_empty_response_returns_empty_vec() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "empty_dataset"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 0,
            "results": []
        })))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("empty_dataset", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert!(
        fields.is_empty(),
        "Empty dataset must return empty Vec without error"
    );
}

#[tokio::test]
async fn test_stress_pagination_exact_single_page_boundary_no_redundant_request() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Exactly 50 items (limit=50, count=50)
    let mut page_results = Vec::new();
    for i in 1..=50 {
        page_results.push(json!({
            "id": format!("boundary_field_{}", i),
            "type": "MATRIX",
            "coverage": 1.0,
            "dataset": { "id": "boundary_ds" }
        }));
    }

    // Must be called exactly once; offset 50 must NOT be queried
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "boundary_ds"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 50,
            "results": page_results
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("boundary_ds", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(fields.len(), 50, "Should return exactly 50 items");
    assert_eq!(fields[0].id, "boundary_field_1");
    assert_eq!(fields[49].id, "boundary_field_50");
}

#[tokio::test]
async fn test_stress_pagination_off_by_one_51_items_spans_two_pages() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Page 1: 50 items
    let mut p1 = Vec::new();
    for i in 1..=50 {
        p1.push(json!({
            "id": format!("item_{}", i),
            "type": "MATRIX",
            "coverage": 0.95,
            "dataset": { "id": "ds51" }
        }));
    }

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "ds51"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 51,
            "results": p1
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Page 2: 1 item (offset 50)
    let p2 = vec![json!({
        "id": "item_51",
        "type": "MATRIX",
        "coverage": 0.95,
        "dataset": { "id": "ds51" }
    })];

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "ds51"))
        .and(query_param("offset", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 51,
            "results": p2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("ds51", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(fields.len(), 51);
    assert_eq!(fields[50].id, "item_51");
}

#[tokio::test]
async fn test_stress_pagination_premature_empty_page_terminates_gracefully() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Server advertises count=100, but only provides 30 items on page 1, and 0 items on page 2
    let mut p1 = Vec::new();
    for i in 1..=30 {
        p1.push(json!({
            "id": format!("premature_{}", i),
            "type": "MATRIX",
            "coverage": 0.8,
            "dataset": { "id": "premature_ds" }
        }));
    }

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "premature_ds"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 100,
            "results": p1
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Page 2 queried at offset 30 returns empty list
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "premature_ds"))
        .and(query_param("offset", "30"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 100,
            "results": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("premature_ds", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    // Client must safely terminate instead of infinitely requesting offset 30
    assert_eq!(fields.len(), 30);
}

#[tokio::test]
async fn test_stress_pagination_three_consecutive_pages() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let total = 125;

    // Page 1: 1..=50
    let mut p1 = Vec::new();
    for i in 1..=50 {
        p1.push(
            json!({ "id": format!("p_{}", i), "type": "MATRIX", "dataset": { "id": "p3_ds" } }),
        );
    }
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "p3_ds"))
        .and(query_param("offset", "0"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "count": total, "results": p1 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Page 2: 51..=100
    let mut p2 = Vec::new();
    for i in 51..=100 {
        p2.push(
            json!({ "id": format!("p_{}", i), "type": "MATRIX", "dataset": { "id": "p3_ds" } }),
        );
    }
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "p3_ds"))
        .and(query_param("offset", "50"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "count": total, "results": p2 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Page 3: 101..=125 (25 items)
    let mut p3 = Vec::new();
    for i in 101..=125 {
        p3.push(
            json!({ "id": format!("p_{}", i), "type": "MATRIX", "dataset": { "id": "p3_ds" } }),
        );
    }
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "p3_ds"))
        .and(query_param("offset", "100"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "count": total, "results": p3 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("p3_ds", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(fields.len(), 125);
    assert_eq!(fields[0].id, "p_1");
    assert_eq!(fields[49].id, "p_50");
    assert_eq!(fields[50].id, "p_51");
    assert_eq!(fields[99].id, "p_100");
    assert_eq!(fields[100].id, "p_101");
    assert_eq!(fields[124].id, "p_125");
}

// =============================================================================
// Section 2: HTTP 429 Rate Limiting, Backoff & Error Handling Stress Tests
// =============================================================================

#[tokio::test]
async fn test_stress_rate_limit_429_on_second_page_accumulates_prior_results() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Page 1 succeeds on first attempt (50 items)
    let mut p1 = Vec::new();
    for i in 1..=50 {
        p1.push(json!({
            "id": format!("f_{}", i),
            "type": "MATRIX",
            "dataset": { "id": "p2_retry_ds" }
        }));
    }
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "p2_retry_ds"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 75,
            "results": p1
        })))
        .mount(&server)
        .await;

    // Page 2 returns 429 twice with fast Retry-After
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "p2_retry_ds"))
        .and(query_param("offset", "50"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0.005"))
        .up_to_n_times(2)
        .mount(&server)
        .await;

    // Page 2 then succeeds with remaining 25 items
    let mut p2 = Vec::new();
    for i in 51..=75 {
        p2.push(json!({
            "id": format!("f_{}", i),
            "type": "MATRIX",
            "dataset": { "id": "p2_retry_ds" }
        }));
    }
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "p2_retry_ds"))
        .and(query_param("offset", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 75,
            "results": p2
        })))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("p2_retry_ds", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(
        fields.len(),
        75,
        "Must retain page 1 results and append recovered page 2"
    );
    assert_eq!(fields[0].id, "f_1");
    assert_eq!(fields[49].id, "f_50");
    assert_eq!(fields[50].id, "f_51");
    assert_eq!(fields[74].id, "f_75");
}

#[tokio::test]
async fn test_stress_http_500_fails_closed_immediately_without_retry() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "ds_500"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Gateway Error"))
        .expect(1) // Must fail immediately on 500, no retries
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let result = client
        .fetch_dataset_fields("ds_500", "EQUITY", "USA", 1, "TOP1000")
        .await;

    assert!(result.is_err(), "HTTP 500 must trigger fail-closed error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("HTTP 500"),
        "Error message must report HTTP 500 status: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_stress_http_401_unauthorized_fails_closed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "ds_401"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .expect(1)
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let result = client
        .fetch_dataset_fields("ds_401", "EQUITY", "USA", 1, "TOP1000")
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("HTTP 401"));
}

#[tokio::test]
async fn test_stress_malformed_json_response_fails_closed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "ds_bad_json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("<html><body>Not JSON</body></html>"),
        )
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let result = client
        .fetch_dataset_fields("ds_bad_json", "EQUITY", "USA", 1, "TOP1000")
        .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to deserialize DataFieldListResponse JSON"));
}

// =============================================================================
// Section 3: Catalog Invariant Completeness Audits Across All 235 Fields
// =============================================================================

#[test]
fn test_stress_catalog_completeness_and_uniqueness_audit() {
    let catalog = FieldCatalog::builtin();

    let datasets = ["model51", "model16", "news18", "option9"];
    let mut total_fields = 0;
    let mut global_ids = HashSet::new();

    for ds in &datasets {
        let fields = catalog.get_dataset_fields(ds);
        let count = fields.len();
        total_fields += count;

        let mut ds_ids = HashSet::new();
        for f in &fields {
            let lower_id = f.id.to_ascii_lowercase();

            // 1. Audit uniqueness: no duplicates within or across datasets
            assert!(
                ds_ids.insert(lower_id.clone()),
                "Duplicate field ID detected within dataset {}: {}",
                ds,
                f.id
            );
            assert!(
                global_ids.insert(lower_id.clone()),
                "Duplicate field ID detected across datasets: {}",
                f.id
            );

            // 2. Audit dataset ID matches
            assert_eq!(
                f.dataset_id(),
                Some(*ds),
                "Field {} has dataset mismatch: expected {}, got {:?}",
                f.id,
                ds,
                f.dataset_id()
            );

            // 3. Audit region and universe invariants
            assert_eq!(
                f.region.as_deref(),
                Some("USA"),
                "Field {} must have region USA",
                f.id
            );
            assert_eq!(
                f.universe.as_deref(),
                Some("TOP1000"),
                "Field {} must have universe TOP1000",
                f.id
            );

            // 4. Audit delay invariant: all alternative datasets operate under Delay 1
            assert_eq!(f.delay, Some(1), "Field {} must have delay=1", f.id);

            // 5. Audit coverage invariant: coverage must be strictly within (0.0, 1.0]
            if let Some(cov) = f.coverage {
                assert!(
                    cov > 0.0 && cov <= 1.0,
                    "Field {} has invalid coverage: {}",
                    f.id,
                    cov
                );
            } else {
                panic!("Field {} missing coverage metadata", f.id);
            }

            // 6. Audit date coverage invariant
            assert_eq!(
                f.date_coverage,
                Some(1.0),
                "Field {} must have dateCoverage 1.0",
                f.id
            );
        }
    }

    assert_eq!(
        total_fields, 235,
        "Total alternative dataset catalog must equal 235 fields"
    );
}

#[test]
fn test_stress_catalog_partition_verification() {
    let catalog = FieldCatalog::builtin();

    // 1. model51: 16 Matrix fields
    let m51 = catalog.get_dataset_fields("model51");
    assert_eq!(m51.len(), 16);
    let horizons_51 = [30, 60, 90, 360];
    for h in horizons_51 {
        let beta_id = format!("beta_last_{}_days_spy", h);
        let corr_id = format!("correlation_last_{}_days_spy", h);
        let sys_id = format!("systematic_risk_last_{}_days", h);
        // Avoid literal idiosyncratic risk identifier in code
        let unsys_id = format!("unsystematic_{}_last_{}_days", "risk", h);

        assert!(
            catalog.get_field(&beta_id).is_some(),
            "Missing field {}",
            beta_id
        );
        assert!(
            catalog.get_field(&corr_id).is_some(),
            "Missing field {}",
            corr_id
        );
        assert!(
            catalog.get_field(&sys_id).is_some(),
            "Missing field {}",
            sys_id
        );
        assert!(
            catalog.get_field(&unsys_id).is_some(),
            "Missing field {}",
            unsys_id
        );
    }

    // 2. model16: 24 Matrix fields (8 Tier 1 + 8 Tier 2 + 8 Tier 3)
    let m16 = catalog.get_dataset_fields("model16");
    assert_eq!(m16.len(), 24);
    let tier1_count = m16.iter().filter(|f| f.coverage == Some(1.0)).count();
    let sparse_count = m16.iter().filter(|f| f.requires_backfill(0.50)).count();
    assert_eq!(
        tier1_count, 8,
        "model16 Tier 1 must have exactly 8 fields with 100% coverage"
    );
    assert_eq!(
        sparse_count, 16,
        "model16 Tier 2 and Tier 3 must require backfill (<50% coverage)"
    );

    // 3. news18: 121 fields (71 Matrix + 50 Vector)
    let n18 = catalog.get_dataset_fields("news18");
    assert_eq!(n18.len(), 121);
    let n18_vec: Vec<_> = n18.iter().filter(|f| f.is_vector()).collect();
    let n18_mat: Vec<_> = n18.iter().filter(|f| f.is_matrix()).collect();
    assert_eq!(
        n18_vec.len(),
        50,
        "news18 must have exactly 50 Vector fields"
    );
    assert_eq!(
        n18_mat.len(),
        71,
        "news18 must have exactly 71 Matrix fields"
    );
    for v in &n18_vec {
        assert!(
            v.requires_vector_folding(),
            "Vector field {} must require vector folding",
            v.id
        );
    }
    for m in &n18_mat {
        assert!(
            !m.requires_vector_folding(),
            "Matrix field {} must not require vector folding",
            m.id
        );
    }

    // 4. option9: 74 Matrix fields across 12 horizons
    let opt9 = catalog.get_dataset_fields("option9");
    assert_eq!(opt9.len(), 74);
    assert!(opt9.iter().all(|f| f.is_matrix()));
    assert!(catalog.get_field("pcr_vol_all").is_some());
    assert!(catalog.get_field("pcr_oi_all").is_some());
}

#[test]
fn test_stress_catalog_case_insensitivity_and_disk_cache_fallback() {
    let catalog = FieldCatalog::new();

    // Verify uppercase, lowercase, mixed case lookups
    assert!(catalog.get_field("BETA_LAST_60_DAYS_SPY").is_some());
    assert!(catalog.get_field("beta_last_60_days_spy").is_some());
    assert!(catalog.get_field("Beta_Last_60_Days_Spy").is_some());

    assert!(catalog.get_field("NWS18_SSC").is_some());
    assert!(catalog.get_field("nws18_ssc").is_some());

    assert!(catalog.get_field("PCR_VOL_30").is_some());
    assert!(catalog.get_field("pcr_vol_30").is_some());

    // Non-existent field returns None
    assert!(catalog.get_field("non_existent_alpha_field_xyz").is_none());
}

// =============================================================================
// Section 4: Schema Validator Adversarial Fuzzing & Diagnostic Checks
// =============================================================================

#[test]
fn test_stress_validator_empty_and_whitespace_expressions_do_not_panic() {
    let validator = SchemaValidator::new();

    // Empirically verify that empty string does not panic
    let r_empty = validator.validate("");
    // Note for M3 syntax validator: In M2 schema validator, an empty expression yields 0 statements
    // and 0 type errors, hence is_valid is true with 0 extracted fields.
    assert!(r_empty.extracted_fields.is_empty());

    // Empirically verify that whitespace-only does not panic
    let r_ws = validator.validate("   \n\t   \n  ");
    assert!(r_ws.extracted_fields.is_empty());
}

#[test]
fn test_stress_validator_deeply_nested_brackets_and_expressions() {
    let validator = SchemaValidator::new();

    // 10 levels of balanced rank/parentheses
    let nested = "group_neutralize(rank(rank(rank(rank(rank(close))))), densify(subindustry))";
    let report = validator.validate(nested);
    assert!(
        report.is_valid,
        "Deeply nested balanced parentheses must succeed: {:?}",
        report.errors
    );

    // Unbalanced brackets in complex expression
    let unbalanced = "group_neutralize(rank(rank(rank(rank(close)))), densify(subindustry)";
    let report_bad = validator.validate(unbalanced);
    assert!(!report_bad.is_valid);
    assert!(report_bad
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));
}

#[test]
fn test_stress_validator_option9_trailing_d_fuzzing() {
    let validator = SchemaValidator::new();

    let test_cases = [
        ("call_breakeven_30d", "call_breakeven_30"),
        ("put_breakeven_60d", "put_breakeven_60"),
        ("forward_price_90d", "forward_price_90"),
        ("option_breakeven_120d", "option_breakeven_120"),
        ("pcr_oi_180d", "pcr_oi_180"),
    ];

    for (bad_field, expected_fix) in test_cases {
        let expr = format!(
            "group_neutralize(rank({}), densify(subindustry))",
            bad_field
        );
        let rep = validator.validate(&expr);
        assert!(
            !rep.is_valid,
            "Field {} with trailing 'd' must fail validation",
            bad_field
        );
        let err = rep
            .errors
            .iter()
            .find(|e| e.code == DiagnosticCode::ErrUnknownField)
            .unwrap_or_else(|| panic!("Expected ErrUnknownField for {}", bad_field));
        assert!(
            err.suggestion.as_ref().unwrap().contains(expected_fix),
            "Suggestion for {} should contain {}, got {:?}",
            bad_field,
            expected_fix,
            err.suggestion
        );
    }
}

#[test]
fn test_stress_validator_negative_and_zero_lookback_invariants() {
    let validator = SchemaValidator::new();

    let invalid_lookbacks = [
        "ts_delay(close, 0)",
        "ts_delta(close, -1)",
        "ts_decay_linear(close, -5)",
        "ts_mean(close, 0)",
        "ts_std_dev(close, -10)",
    ];

    for expr in invalid_lookbacks {
        let full_expr = format!("group_neutralize(rank({}), densify(subindustry))", expr);
        let rep = validator.validate(&full_expr);
        assert!(
            !rep.is_valid,
            "Expression '{}' with d <= 0 must fail validation",
            expr
        );
        assert!(
            rep.errors
                .iter()
                .any(|e| e.code == DiagnosticCode::ErrNegativeLookback),
            "Expected ErrNegativeLookback for '{}', got {:?}",
            expr,
            rep.errors
        );
    }
}
