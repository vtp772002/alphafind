use alphafind::client::{
    retry_is_safe, simulation_fingerprint, BrainClient, PostFailure, SubmitStatus,
    AMBIGUOUS_SIMULATION_NOTE,
};
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
    assert!(
        result.is_ok(),
        "Authentication should succeed with 200 response"
    );
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
                .append_header(
                    "Location",
                    format!("{}/simulations/sim-12345", server.uri()),
                )
                .set_body_json(json!({"id": "sim-12345"})),
        )
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let settings = AlphaSettings::default();
    let sim_id = client
        .submit_simulation("rank(close)", &settings)
        .await
        .unwrap();
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
    let (status, msg) = client.submit_alpha("alpha-good").await.unwrap();

    assert_eq!(status, SubmitStatus::Accepted);
    assert!(msg.contains("HTTP 201"));
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
    let (status, msg) = client.submit_alpha("alpha-bad").await.unwrap();

    assert_eq!(status, SubmitStatus::Rejected);
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
    let all_alphas = client
        .fetch_user_alphas_paginated("stage=OS")
        .await
        .unwrap();

    assert_eq!(
        all_alphas.len(),
        125,
        "Pagination loop should fetch all 125 alphas across both pages"
    );
}

/// Minimal HTTP server used to reproduce a connection that dies mid-request.
///
/// It answers `POST /authentication` with 200 and then, for the configured path,
/// closes the socket without sending any response. That is exactly the ambiguous
/// case a broker or proxy can produce, and it must never trigger a duplicate POST.
struct InterruptingServer {
    base_url: String,
    port: u16,
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl InterruptingServer {
    fn start(close_path: &str) -> Self {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        use std::sync::Arc;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let port = listener.local_addr().expect("local addr").port();
        let hits = Arc::new(AtomicUsize::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));
        let close_path = close_path.to_string();

        let hits_thread = Arc::clone(&hits);
        let shutdown_thread = Arc::clone(&shutdown);

        std::thread::spawn(move || {
            use std::sync::atomic::Ordering;
            for stream in listener.incoming() {
                if shutdown_thread.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { continue };

                let Ok(read_half) = stream.try_clone() else {
                    continue;
                };
                let mut reader = BufReader::new(read_half);
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
                    continue;
                }

                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
                if content_length > 0 {
                    let mut body = vec![0u8; content_length];
                    let _ = reader.read_exact(&mut body);
                }

                if request_line.contains(&close_path) {
                    hits_thread.fetch_add(1, Ordering::SeqCst);
                    // Drop the connection with no response: the client cannot know
                    // whether the platform processed the request.
                    continue;
                }

                let response = if request_line.contains("/authentication") {
                    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            port,
            hits,
            shutdown,
        }
    }

    fn hits(&self) -> usize {
        self.hits.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Unblocks the accept loop so the worker thread can exit.
    fn stop(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(("127.0.0.1", self.port));
    }
}

#[tokio::test]
async fn test_ambiguous_simulation_failure_is_not_retried() {
    let server = InterruptingServer::start("/simulations");
    let client =
        BrainClient::with_base_url("user@example.com", "secret", &server.base_url).unwrap();
    let settings = AlphaSettings::default();

    let err = client
        .submit_simulation("<PRIVATE_EXPRESSION>", &settings)
        .await
        .expect_err("an interrupted POST must fail closed");

    let msg = err.to_string();
    assert!(
        msg.contains(AMBIGUOUS_SIMULATION_NOTE) || msg.contains("AMBIGUOUS SIMULATION STATE"),
        "error must explain the ambiguous state, got: {msg}"
    );
    assert!(
        msg.contains("fingerprint"),
        "error must carry a reconciliation fingerprint, got: {msg}"
    );
    assert_eq!(
        server.hits(),
        1,
        "an ambiguous simulation POST must not be retried"
    );

    server.stop();
}

#[tokio::test]
async fn test_ambiguous_submission_outcome_is_reported_unknown() {
    let server = InterruptingServer::start("/alphas/alpha-unknown/submit");
    let client =
        BrainClient::with_base_url("user@example.com", "secret", &server.base_url).unwrap();

    let (status, msg) = client
        .submit_alpha("alpha-unknown")
        .await
        .expect("transport ambiguity is reported, not propagated as a hard error");

    assert_eq!(status, SubmitStatus::Unknown);
    assert!(
        msg.contains("AMBIGUOUS SUBMISSION STATE"),
        "message must warn about unconfirmed submission state, got: {msg}"
    );
    assert_eq!(
        server.hits(),
        1,
        "an ambiguous submission POST must be sent at most once"
    );

    server.stop();
}

#[tokio::test]
async fn test_mock_submit_simulation_retries_after_rate_limit() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/authentication"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // First attempt: explicit rate limit, which the platform refused.
    Mock::given(method("POST"))
        .and(path("/simulations"))
        .respond_with(ResponseTemplate::new(429).append_header("Retry-After", "0"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second attempt: accepted.
    Mock::given(method("POST"))
        .and(path("/simulations"))
        .respond_with(ResponseTemplate::new(201).append_header(
            "Location",
            format!("{}/simulations/sim-after-429", server.uri()),
        ))
        .mount(&server)
        .await;

    let client = BrainClient::with_base_url("user@example.com", "secret", server.uri()).unwrap();
    let settings = AlphaSettings::default();
    let sim_id = client
        .submit_simulation("rank(close)", &settings)
        .await
        .expect("a 429 is safe to retry");

    assert!(sim_id.contains("sim-after-429"));
}

#[test]
fn test_retry_policy_only_allows_provably_unsent_requests() {
    assert!(
        retry_is_safe(PostFailure::NotSent),
        "connection failures never reached the platform and are safe to retry"
    );
    assert!(
        !retry_is_safe(PostFailure::Ambiguous),
        "ambiguous failures must not be retried"
    );
}

#[test]
fn test_simulation_fingerprint_is_stable_and_settings_sensitive() {
    let settings = AlphaSettings::default();
    let first = simulation_fingerprint("<PRIVATE_EXPRESSION>", &settings);
    let second = simulation_fingerprint("<PRIVATE_EXPRESSION>", &settings);
    assert_eq!(first, second, "fingerprint must be deterministic");
    assert_eq!(first.len(), 16, "fingerprint must be a 16-char hex digest");
    assert!(
        first.chars().all(|c| c.is_ascii_hexdigit()),
        "fingerprint must be hexadecimal"
    );

    let mut changed = settings.clone();
    changed.decay += 1;
    assert_ne!(
        first,
        simulation_fingerprint("<PRIVATE_EXPRESSION>", &changed),
        "settings changes must change the fingerprint"
    );
    assert_ne!(
        first,
        simulation_fingerprint("<OTHER_EXPRESSION>", &settings),
        "expression changes must change the fingerprint"
    );
}
