//! Test Suite for Milestone M2: Field Metadata Extraction & Schema Validator.
//!
//! Covers:
//! - DataFieldType and DataFieldEntry serialization and deserialization
//! - SchemaValidator 6-type lattice and FastExpr rule enforcement
//! - Built-in 235-field catalog invariants across model51, model16, news18, option9
//! - WireMock mock API pagination, query parameter enforcement, and HTTP 429 backoff

use alphafind::client::BrainClient;
use alphafind::models::schema_validator::{DiagnosticCode, FieldCatalog, SchemaValidator};
use alphafind::models::{DataFieldEntry, DataFieldType, DatasetCategoryInfo};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =============================================================================
// Suite A: Data Models & Serialization Tests
// =============================================================================

#[test]
fn test_data_field_type_matrix_and_vector() {
    let matrix_json = r#""MATRIX""#;
    let t_mat: DataFieldType = serde_json::from_str(matrix_json).unwrap();
    assert_eq!(t_mat, DataFieldType::Matrix);
    assert!(t_mat.is_matrix());
    assert!(!t_mat.is_vector());
    assert_eq!(t_mat.as_str(), "MATRIX");

    let vector_json = r#""VECTOR""#;
    let t_vec: DataFieldType = serde_json::from_str(vector_json).unwrap();
    assert_eq!(t_vec, DataFieldType::Vector);
    assert!(!t_vec.is_matrix());
    assert!(t_vec.is_vector());
    assert_eq!(t_vec.as_str(), "VECTOR");

    let unknown_json = r#""TENSOR_3D""#;
    let t_unk: DataFieldType = serde_json::from_str(unknown_json).unwrap();
    assert_eq!(t_unk, DataFieldType::Other);
    assert_eq!(t_unk.as_str(), "OTHER");
}

#[test]
fn test_data_field_entry_matrix_deserialization() {
    let json_str = r#"{
        "id": "beta_last_60_days_spy",
        "description": "Rolling 60-day OLS slope",
        "dataset": { "id": "model51", "name": "Systematic Risk" },
        "category": { "id": "model", "name": "Risk Models" },
        "subcategory": { "id": "model-risk-models", "name": "Risk Models" },
        "region": "USA",
        "delay": 1,
        "universe": "TOP1000",
        "type": "MATRIX",
        "coverage": 0.9752,
        "dateCoverage": 1.0,
        "userCount": 42,
        "alphaCount": 128
    }"#;

    let entry: DataFieldEntry = serde_json::from_str(json_str).unwrap();
    assert_eq!(entry.id, "beta_last_60_days_spy");
    assert_eq!(entry.dataset_id(), Some("model51"));
    assert_eq!(entry.field_type, Some(DataFieldType::Matrix));
    assert!(entry.is_matrix());
    assert!(!entry.is_vector());
    assert!(!entry.requires_vector_folding());
    assert!(!entry.requires_backfill(0.50));
    assert_eq!(entry.coverage, Some(0.9752));
    assert_eq!(entry.user_count, Some(42));
}

#[test]
fn test_data_field_entry_vector_deserialization() {
    let json_str = r#"{
        "id": "nws18_ssc",
        "description": "Continuous story sentiment score",
        "dataset": { "id": "news18", "name": "RavenPack News Data" },
        "type": "VECTOR",
        "coverage": 0.9030,
        "dateCoverage": 1.0
    }"#;

    let entry: DataFieldEntry = serde_json::from_str(json_str).unwrap();
    assert_eq!(entry.id, "nws18_ssc");
    assert_eq!(entry.dataset_id(), Some("news18"));
    assert_eq!(entry.field_type, Some(DataFieldType::Vector));
    assert!(!entry.is_matrix());
    assert!(entry.is_vector());
    assert!(entry.requires_vector_folding());
}

#[test]
fn test_data_field_entry_missing_type_safely_defaults() {
    let json_str = r#"{
        "id": "legacy_field",
        "coverage": 0.35
    }"#;

    let entry: DataFieldEntry = serde_json::from_str(json_str).unwrap();
    assert_eq!(entry.id, "legacy_field");
    assert!(entry.field_type.is_none());
    assert!(!entry.is_vector());
    assert!(!entry.is_matrix());
    assert!(entry.requires_backfill(0.50));
}

#[test]
fn test_data_field_serialization_preserves_camel_case() {
    let entry = DataFieldEntry {
        id: "pcr_vol_30".to_string(),
        description: Some("30-day put-call volume ratio".to_string()),
        dataset: Some(DatasetCategoryInfo {
            id: Some("option9".to_string()),
            name: Some("Options Analytics".to_string()),
        }),
        category: None,
        subcategory: None,
        region: Some("USA".to_string()),
        delay: Some(1),
        universe: Some("TOP1000".to_string()),
        field_type: Some(DataFieldType::Matrix),
        coverage: Some(0.98),
        date_coverage: Some(1.0),
        user_count: Some(55),
        alpha_count: Some(210),
        themes: None,
        date_created: None,
    };

    let serialized = serde_json::to_string(&entry).unwrap();
    assert!(serialized.contains("\"dateCoverage\""));
    assert!(serialized.contains("\"userCount\""));
    assert!(serialized.contains("\"alphaCount\""));
    assert!(serialized.contains("\"type\":\"MATRIX\""));

    let deserialized: DataFieldEntry = serde_json::from_str(&serialized).unwrap();
    assert_eq!(entry, deserialized);
}

// =============================================================================
// Suite B: FastExpr Schema Validator Engine Tests
// =============================================================================

#[test]
fn test_validator_passes_standard_matrix_field() {
    let validator = SchemaValidator::new();
    let report =
        validator.validate("group_neutralize(rank(beta_last_60_days_spy), densify(subindustry))");
    assert!(
        report.is_valid,
        "Expected valid expression but got: {:?}",
        report.errors
    );
    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.extracted_fields.len(), 1);
    assert_eq!(report.extracted_fields[0].id, "beta_last_60_days_spy");
}

#[test]
fn test_validator_passes_folded_vector_field() {
    let validator = SchemaValidator::new();
    let report =
        validator.validate("group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))");
    assert!(
        report.is_valid,
        "Expected valid folded vector but got: {:?}",
        report.errors
    );
    assert_eq!(report.errors.len(), 0);
    assert!(report.extracted_fields[0].is_vector);
}

#[test]
fn test_validator_fails_unfolded_vector_field_in_rank() {
    let validator = SchemaValidator::new();
    let report = validator.validate("group_neutralize(rank(nws18_ssc), densify(subindustry))");
    assert!(!report.is_valid);
    assert!(report
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
}

#[test]
fn test_validator_fails_unfolded_vector_field_in_binary_op() {
    let validator = SchemaValidator::new();
    let report = validator
        .validate("signal = nws18_ssc + 1.0; group_neutralize(rank(signal), densify(subindustry))");
    assert!(!report.is_valid);
    assert!(report
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
}

#[test]
fn test_validator_enforces_group_neutralize_densify() {
    let validator = SchemaValidator::new();

    // Passing with densify
    let report_valid = validator.validate("group_neutralize(rank(close), densify(subindustry))");
    assert!(report_valid.is_valid);

    // Failing without densify
    let report_invalid = validator.validate("group_neutralize(rank(close), subindustry)");
    assert!(!report_invalid.is_valid);
    assert!(report_invalid
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUndensifiedGroup));
}

#[test]
fn test_validator_enforces_low_coverage_backfill() {
    let validator = SchemaValidator::new();

    // fscore_growth has coverage ~29.87% < 50%
    let report_invalid =
        validator.validate("group_neutralize(rank(fscore_growth), densify(subindustry))");
    assert!(!report_invalid.is_valid);
    assert!(report_invalid
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrMissingBackfill));

    // Passing when ancestral ts_backfill is present with d >= 5
    let report_valid = validator
        .validate("group_neutralize(rank(ts_backfill(fscore_growth, 15)), densify(subindustry))");
    assert!(report_valid.is_valid);
}

#[test]
fn test_validator_rejects_negative_lookback() {
    let validator = SchemaValidator::new();

    let report_neg =
        validator.validate("group_neutralize(rank(ts_delay(close, -5)), densify(subindustry))");
    assert!(!report_neg.is_valid);
    assert!(report_neg
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    let report_zero =
        validator.validate("group_neutralize(rank(ts_delta(close, 0)), densify(subindustry))");
    assert!(!report_zero.is_valid);
    assert!(report_zero
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));
}

#[test]
fn test_validator_flags_option9_trailing_d_syntax() {
    let validator = SchemaValidator::new();

    let report_bad =
        validator.validate("group_neutralize(rank(pcr_vol_20d), densify(subindustry))");
    assert!(!report_bad.is_valid);
    assert!(report_bad
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUnknownField));
    let err = report_bad
        .errors
        .iter()
        .find(|e| e.code == DiagnosticCode::ErrUnknownField)
        .unwrap();
    assert!(err.suggestion.as_ref().unwrap().contains("pcr_vol_20"));

    let report_good =
        validator.validate("group_neutralize(rank(pcr_vol_20), densify(subindustry))");
    assert!(report_good.is_valid);
}

#[test]
fn test_validator_rejects_unbalanced_parentheses() {
    let validator = SchemaValidator::new();
    let report =
        validator.validate("group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry)");
    assert!(!report.is_valid);
    assert!(report
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));
}

#[test]
fn test_validator_enforces_delay_compatibility() {
    // Delay 1 field tested under delay=0 validator
    let validator_d0 = SchemaValidator::new().with_delay(0);
    let report = validator_d0
        .validate("group_neutralize(rank(beta_last_60_days_spy), densify(subindustry))");
    assert!(!report.is_valid);
    assert!(report
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrLookaheadDelay));

    // Under delay=1, it passes
    let validator_d1 = SchemaValidator::new().with_delay(1);
    let report_d1 = validator_d1
        .validate("group_neutralize(rank(beta_last_60_days_spy), densify(subindustry))");
    assert!(report_d1.is_valid);
}

#[test]
fn test_validator_multi_statement_complex_expression() {
    let validator = SchemaValidator::new();
    let expr = r#"
        anchor_beta = 0.5 - rank(ts_backfill(beta_last_60_days_spy, 15));
        news_sent = ts_backfill(vec_avg(composite_sentiment_score_2), 15);
        call_be = ts_backfill(call_breakeven_30 / close - 1.0, 10);
        put_be = ts_backfill(1.0 - put_breakeven_30 / close, 10);
        opt_asym = rank(put_be - call_be) - 0.5;
        composite = 0.50 * anchor_beta + 0.25 * news_sent + 0.25 * opt_asym;
        group_neutralize(signed_power(composite, 2.0), densify(subindustry))
    "#;

    let report = validator.validate(expr);
    assert!(
        report.is_valid,
        "Expected complex expression to pass: {:?}",
        report.errors
    );
    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.extracted_fields.len(), 5);
}

// =============================================================================
// Suite C: Built-in Catalog Invariants Tests
// =============================================================================

#[test]
fn test_catalog_invariants_across_all_four_alternative_datasets() {
    let catalog = FieldCatalog::new();

    let m51 = catalog.get_dataset_fields("model51");
    assert_eq!(m51.len(), 16, "model51 must contain exactly 16 fields");
    assert!(m51.iter().all(|f| f.is_matrix()));

    let m16 = catalog.get_dataset_fields("model16");
    assert_eq!(m16.len(), 24, "model16 must contain exactly 24 fields");
    let tier1_count = m16.iter().filter(|f| f.coverage == Some(1.0)).count();
    assert_eq!(
        tier1_count, 8,
        "model16 must have 8 Tier 1 rank derivatives with 1.0 coverage"
    );

    let n18 = catalog.get_dataset_fields("news18");
    assert_eq!(n18.len(), 121, "news18 must contain exactly 121 fields");
    let n18_matrix = n18.iter().filter(|f| f.is_matrix()).count();
    let n18_vector = n18.iter().filter(|f| f.is_vector()).count();
    assert_eq!(n18_matrix, 71, "news18 must contain 71 Matrix fields");
    assert_eq!(n18_vector, 50, "news18 must contain 50 Vector fields");

    let opt9 = catalog.get_dataset_fields("option9");
    assert_eq!(opt9.len(), 74, "option9 must contain exactly 74 fields");
    assert!(opt9.iter().all(|f| f.is_matrix()));

    let total_alt = m51.len() + m16.len() + n18.len() + opt9.len();
    assert_eq!(
        total_alt, 235,
        "Total alternative surface must equal exactly 235 fields"
    );
}

// =============================================================================
// Suite D: WireMock Mock API Client Pagination & Rate Limit Backoff Tests
// =============================================================================

#[tokio::test]
async fn test_mock_fetch_dataset_fields_single_page() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("instrumentType", "EQUITY"))
        .and(query_param("region", "USA"))
        .and(query_param("delay", "1"))
        .and(query_param("universe", "TOP1000"))
        .and(query_param("dataset.id", "model51"))
        .and(query_param("limit", "50"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 2,
            "results": [
                {
                    "id": "beta_last_30_days_spy",
                    "type": "MATRIX",
                    "coverage": 0.9765,
                    "dateCoverage": 1.0,
                    "dataset": { "id": "model51" }
                },
                {
                    "id": "beta_last_60_days_spy",
                    "type": "MATRIX",
                    "coverage": 0.9752,
                    "dateCoverage": 1.0,
                    "dataset": { "id": "model51" }
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("model51", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].id, "beta_last_30_days_spy");
    assert_eq!(fields[1].id, "beta_last_60_days_spy");
    assert!(fields[0].is_matrix());
}

#[tokio::test]
async fn test_mock_fetch_dataset_fields_multi_page_pagination() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Page 1: 50 items (offset 0)
    let mut page1_results = Vec::new();
    for i in 1..=50 {
        page1_results.push(json!({
            "id": format!("field_{}", i),
            "type": "MATRIX",
            "coverage": 1.0,
            "dataset": { "id": "option9" }
        }));
    }

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "option9"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 74,
            "results": page1_results
        })))
        .mount(&server)
        .await;

    // Page 2: 24 items (offset 50)
    let mut page2_results = Vec::new();
    for i in 51..=74 {
        page2_results.push(json!({
            "id": format!("field_{}", i),
            "type": "MATRIX",
            "coverage": 1.0,
            "dataset": { "id": "option9" }
        }));
    }

    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "option9"))
        .and(query_param("offset", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 74,
            "results": page2_results
        })))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("option9", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(
        fields.len(),
        74,
        "Client should paginate and collect all 74 fields"
    );
    assert_eq!(fields[0].id, "field_1");
    assert_eq!(fields[73].id, "field_74");
}

#[tokio::test]
async fn test_mock_fetch_dataset_fields_rate_limit_backoff() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // First attempt returns 429 with short Retry-After
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "news18"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0.01"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second attempt returns 200 with result
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "news18"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "count": 1,
            "results": [
                {
                    "id": "nws18_ssc",
                    "type": "VECTOR",
                    "coverage": 0.9030,
                    "dataset": { "id": "news18" }
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let fields = client
        .fetch_dataset_fields("news18", "EQUITY", "USA", 1, "TOP1000")
        .await
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].id, "nws18_ssc");
    assert!(fields[0].is_vector());
}

#[tokio::test]
async fn test_mock_fetch_dataset_fields_fail_closed_on_persistent_429() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Persistent 429 across all retries
    Mock::given(method("GET"))
        .and(path("/data-fields"))
        .and(query_param("dataset.id", "model51"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0.01"))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "pass", server.uri()).unwrap();
    let result = client
        .fetch_dataset_fields("model51", "EQUITY", "USA", 1, "TOP1000")
        .await;

    assert!(
        result.is_err(),
        "Expected fail-closed error on persistent HTTP 429"
    );
    assert!(result.unwrap_err().to_string().contains("rate limit"));
}

// =============================================================================
// Suite E: Remediated Validator Invariants (M2 Remediation)
// =============================================================================

#[test]
fn test_remediated_backfill_poisoning_in_binary_operations() {
    let validator = SchemaValidator::new();

    // Combining backfilled fscore_momentum with unbackfilled fscore_growth must fail
    let expr_poisoned = "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))";
    let rep_poisoned = validator.validate(expr_poisoned);
    assert!(!rep_poisoned.is_valid);
    assert!(rep_poisoned
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrMissingBackfill));

    // Both backfilled passes
    let expr_clean = "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + ts_backfill(fscore_growth, 15)), densify(subindustry))";
    let rep_clean = validator.validate(expr_clean);
    assert!(
        rep_clean.is_valid,
        "Expected both backfilled to pass: {:?}",
        rep_clean.errors
    );

    // Vector directly in ts_backfill must fail with ErrVecTypeMismatch
    let expr_vec_backfill =
        "group_neutralize(rank(ts_backfill(nws18_ssc, 15)), densify(subindustry))";
    let rep_vec = validator.validate(expr_vec_backfill);
    assert!(!rep_vec.is_valid);
    assert!(rep_vec
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
}

#[test]
fn test_remediated_lookback_arithmetic_constant_folding() {
    let validator = SchemaValidator::new();

    // Lookback with binary subtraction resulting in negative lag
    let expr_neg = "group_neutralize(rank(ts_delay(close, 2 - 5)), densify(subindustry))";
    let rep_neg = validator.validate(expr_neg);
    assert!(!rep_neg.is_valid);
    assert!(rep_neg
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // Lookback with binary addition resulting in positive lag passes
    let expr_pos = "group_neutralize(rank(ts_delay(close, 2 + 3)), densify(subindustry))";
    let rep_pos = validator.validate(expr_pos);
    assert!(
        rep_pos.is_valid,
        "Expected positive folded lag to pass: {:?}",
        rep_pos.errors
    );

    // Variable lookup for lag
    let expr_var_neg =
        "lookback = 1 - 4; group_neutralize(rank(ts_delay(close, lookback)), densify(subindustry))";
    let rep_var_neg = validator.validate(expr_var_neg);
    assert!(!rep_var_neg.is_valid);
    assert!(rep_var_neg
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));
}

#[test]
fn test_remediated_price_normalization_scalar_literal() {
    let validator = SchemaValidator::new();

    // Normalized ratio minus literal scalar 1.0 must not emit WarnUnnormalizedPrice
    let expr_norm = "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))";
    let rep_norm = validator.validate(expr_norm);
    assert!(
        rep_norm.is_valid,
        "Expected valid formula: {:?}",
        rep_norm.errors
    );
    assert!(
        !rep_norm
            .warnings
            .iter()
            .any(|w| w.code == DiagnosticCode::WarnUnnormalizedPrice),
        "Literal 1.0 must not cause price normalization false positives"
    );

    // Raw unnormalized price level must trigger WarnUnnormalizedPrice
    let expr_raw = "group_neutralize(rank(call_breakeven_30), densify(subindustry))";
    let rep_raw = validator.validate(expr_raw);
    assert!(rep_raw
        .warnings
        .iter()
        .any(|w| w.code == DiagnosticCode::WarnUnnormalizedPrice));
}

#[test]
fn test_remediated_arity_and_empty_scripts() {
    let validator = SchemaValidator::new();

    // 0-argument calls must fail closed
    let rep_ts = validator.validate("group_neutralize(rank(ts_delay()), densify(subindustry))");
    assert!(!rep_ts.is_valid);
    assert!(rep_ts
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    let rep_rank = validator.validate("group_neutralize(rank(), densify(subindustry))");
    assert!(!rep_rank.is_valid);
    assert!(rep_rank
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    let rep_dens = validator.validate("group_neutralize(rank(close), densify())");
    assert!(!rep_dens.is_valid);
    assert!(rep_dens
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    let rep_vec = validator.validate("group_neutralize(rank(vec_avg()), densify(subindustry))");
    assert!(!rep_vec.is_valid);
    assert!(rep_vec
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    // Empty and assignment-only scripts must fail closed
    let rep_empty = validator.validate("");
    assert!(!rep_empty.is_valid);
    assert!(rep_empty
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    let rep_assign = validator.validate("a = 1; b = 2;");
    assert!(!rep_assign.is_valid);
    assert!(rep_assign
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    // Unknown field must fail closed with ErrUnknownField
    let rep_unk = validator
        .validate("group_neutralize(rank(nonexistent_alpha_field_xyz), densify(subindustry))");
    assert!(!rep_unk.is_valid);
    assert!(rep_unk
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUnknownField));
}
