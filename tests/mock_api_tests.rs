use alphafind::client::BrainClient;
use alphafind::models::AlphaSettings;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_mock_authenticate_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": "mock-token"})))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let result = client.authenticate().await;
    assert!(result.is_ok(), "Authentication should succeed with 200 response");
}

#[tokio::test]
async fn test_mock_authenticate_unauthorized() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Invalid credentials"))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "wrong", server.uri()).unwrap();
    let result = client.authenticate().await;
    assert!(result.is_err(), "Authentication should fail on 401");
}

#[tokio::test]
async fn test_mock_submit_simulation_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/simulations"))
        .respond_with(
            ResponseTemplate::new(201)
                .append_header("Location", format!("{}/simulations/sim-12345", server.uri()))
                .set_body_json(json!({"id": "sim-12345"})),
        )
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let settings = AlphaSettings::default();
    let sim_id = client.submit_simulation("rank(close)", &settings).await.unwrap();
    assert!(sim_id.contains("sim-12345"));
}

#[tokio::test]
async fn test_mock_poll_simulation_ssrf_protection() {
    let server = MockServer::start().await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();

    // Malicious URL targeting an outside domain
    let evil_url = "https://evil-attacker.com/simulations/steal-creds";
    let poll_result = client.poll_simulation(evil_url, 10, 1).await;

    assert!(poll_result.is_err(), "Should reject URL outside base host");
    let err_msg = poll_result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Security violation"),
        "Error message should mention Security violation, got: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_mock_check_submission_all_pass() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let pass_checks = json!({
        "is": {
            "sharpe": 1.75,
            "fitness": 1.62,
            "turnover": 0.28,
            "returns": 0.22,
            "checks": [
                {"name": "LOW_SHARPE", "result": "PASS"},
                {"name": "LOW_FITNESS", "result": "PASS"},
                {"name": "LOW_TURNOVER", "result": "PASS"},
                {"name": "HIGH_TURNOVER", "result": "PASS"},
                {"name": "CONCENTRATED_WEIGHT", "result": "PASS"},
                {"name": "SELF_CORRELATION", "result": "PASS"},
                {"name": "LOW_SUB_UNIVERSE_SHARPE", "result": "PASS"},
                {"name": "MATCHES_COMPETITION", "result": "PASS"}
            ]
        }
    });

    Mock::given(method("GET"))
        .and(path("/alphas/alpha-all-pass/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pass_checks))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let (passed, msg, stats) = client.check_submission("alpha-all-pass", 3).await.unwrap();

    assert!(passed, "All PASS checks should pass gate");
    assert!(msg.contains("ALL SUBMISSION CHECKS PASSED"));
    assert_eq!(stats.checks.len(), 8);
}

#[tokio::test]
async fn test_mock_check_submission_empty_fails_closed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let empty_checks = json!({
        "is": {
            "checks": []
        }
    });

    Mock::given(method("GET"))
        .and(path("/alphas/alpha-empty/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_checks))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let (passed, msg, _) = client.check_submission("alpha-empty", 3).await.unwrap();

    assert!(!passed, "Empty checks must fail-closed");
    assert!(msg.contains("FAIL-CLOSED"));
}

#[tokio::test]
async fn test_mock_check_submission_pending_retry_then_pass() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let pending_response = json!({
        "is": {
            "checks": [
                {"name": "LOW_SHARPE", "result": "PASS"},
                {"name": "LOW_FITNESS", "result": "PASS"},
                {"name": "LOW_TURNOVER", "result": "PASS"},
                {"name": "HIGH_TURNOVER", "result": "PASS"},
                {"name": "CONCENTRATED_WEIGHT", "result": "PASS"},
                {"name": "SELF_CORRELATION", "result": "PENDING"}
            ]
        }
    });

    let pass_response = json!({
        "is": {
            "checks": [
                {"name": "LOW_SHARPE", "result": "PASS"},
                {"name": "LOW_FITNESS", "result": "PASS"},
                {"name": "LOW_TURNOVER", "result": "PASS"},
                {"name": "HIGH_TURNOVER", "result": "PASS"},
                {"name": "CONCENTRATED_WEIGHT", "result": "PASS"},
                {"name": "SELF_CORRELATION", "result": "PASS"}
            ]
        }
    });

    // First attempt returns PENDING with short Retry-After: 0.1s
    Mock::given(method("GET"))
        .and(path("/alphas/alpha-retry/check"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("Retry-After", "0.1")
                .set_body_json(pending_response),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Subsequent attempt returns PASS
    Mock::given(method("GET"))
        .and(path("/alphas/alpha-retry/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pass_response))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let (passed, msg, _) = client.check_submission("alpha-retry", 5).await.unwrap();

    assert!(passed, "Should poll through PENDING and succeed once PASS");
    assert!(msg.contains("ALL SUBMISSION CHECKS PASSED"));
}

#[tokio::test]
async fn test_mock_submit_alpha_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/alphas/alpha-good/submit"))
        .respond_with(ResponseTemplate::new(201).set_body_string("{\"status\": \"SUBMITTED\"}"))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let (success, msg) = client.submit_alpha("alpha-good").await.unwrap();

    assert!(success, "submit_alpha should succeed on 201 Created");
    assert!(msg.contains("201 Created"));
}

#[tokio::test]
async fn test_mock_submit_alpha_failure_handled_gracefully() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/alphas/alpha-bad/submit"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string("{\"error\": \"Alpha failed Out-of-Sample turnover criterion\"}"),
        )
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let (success, msg) = client.submit_alpha("alpha-bad").await.unwrap();

    assert!(!success, "submit_alpha should report false on 400 error");
    assert!(msg.contains("HTTP 400"));
}

#[tokio::test]
async fn test_mock_fetch_user_alphas_paginated() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Page 1: 100 items (limit=100&offset=0)
    let page1_items: Vec<_> = (0..100)
        .map(|i| json!({"id": format!("alpha-p1-{:03}", i), "stage": "OS"}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/users/self/alphas"))
        .and(query_param("stage", "OS"))
        .and(query_param("limit", "100"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results": page1_items})))
        .mount(&server)
        .await;

    // Page 2: 25 items (limit=100&offset=100)
    let page2_items: Vec<_> = (0..25)
        .map(|i| json!({"id": format!("alpha-p2-{:03}", i), "stage": "OS"}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/users/self/alphas"))
        .and(query_param("stage", "OS"))
        .and(query_param("limit", "100"))
        .and(query_param("offset", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results": page2_items})))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let all_alphas = client.fetch_user_alphas_paginated("stage=OS").await.unwrap();

    assert_eq!(
        all_alphas.len(),
        125,
        "Pagination loop should fetch all 125 alphas across both pages"
    );
}
