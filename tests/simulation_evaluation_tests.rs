//! Integration tests for Milestone M4:
//! - Concurrency lock acquisition, contention, timeout, and release behavior (`src/concurrency_lock.rs`).
//! - In-Sample threshold qualification gates (Sharpe >= 1.25, Fitness >= 1.00, Turnover 1%-70%, Returns > 8.0%).
//! - Fail-closed platform submission check evaluator (`src/client.rs`).
//! - Pearson correlation and portfolio merge impact math (`src/correlation.rs`).

use alphafind::client::{evaluate_submission_checks, CORE_SUBMISSION_CHECKS};
use alphafind::concurrency_lock::ProcessLock;
use alphafind::config::quant::{
    min_fitness_for_delay, min_sharpe_for_delay, turnover_within_platform_bounds,
    BRAIN_MAX_CORRELATION,
};
use alphafind::correlation::{
    audit_candidate, calculate_portfolio_impact, pearson_correlation, simulate_portfolio,
};
use alphafind::models::{CheckItem, InSampleStats};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ==============================================================================
// 1. Concurrency Lock Tests
// ==============================================================================

#[test]
fn test_concurrency_lock_acquisition_and_release() {
    let temp_dir = std::env::temp_dir().join(format!("af_lock_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let lock_path = temp_dir.join("test_sim.lock");

    // Acquire lock
    let lock = ProcessLock::try_acquire_at(&lock_path)
        .expect("Should acquire advisory process lock at clean path");

    assert!(lock.is_held(), "Lock must report is_held = true");
    assert_eq!(lock.lock_path(), lock_path.as_path());
    assert!(lock_path.exists(), "Lock file must exist on disk");

    // Verify metadata inside lockfile
    let content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(content.contains("pid="), "Lock file must record pid");
    assert!(
        content.contains("timestamp="),
        "Lock file must record timestamp"
    );

    // Release lock explicitly
    lock.release().expect("Explicit release should succeed");

    // Clean up
    let _ = std::fs::remove_file(&lock_path);
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_concurrency_lock_contention_mutual_exclusion() {
    let temp_dir = std::env::temp_dir().join(format!("af_contention_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let lock_path = temp_dir.join("contention.lock");

    // Acquire first lock
    let lock1 =
        ProcessLock::try_acquire_at(&lock_path).expect("First lock acquisition must succeed");
    assert!(lock1.is_held());

    // Attempt to acquire second lock on the same path -> must fail closed
    let lock2_res = ProcessLock::try_acquire_at(&lock_path);
    assert!(
        lock2_res.is_err(),
        "Second lock acquisition on the same file must fail due to flock contention"
    );

    let err_msg = lock2_res.err().unwrap().to_string();
    assert!(
        err_msg.contains("held by another process"),
        "Error message must clearly state lock contention per AGENTS.md: got '{}'",
        err_msg
    );

    // Drop first lock (RAII unlock)
    drop(lock1);

    // Now third acquisition must succeed
    let lock3 = ProcessLock::try_acquire_at(&lock_path)
        .expect("Acquisition after first lock is dropped must succeed");
    assert!(lock3.is_held());
    drop(lock3);

    let _ = std::fs::remove_file(&lock_path);
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_concurrency_lock_timeout_success() {
    let temp_dir = std::env::temp_dir().join(format!("af_timeout_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let lock_path = temp_dir.join("timeout_success.lock");

    let lock1 = ProcessLock::try_acquire_at(&lock_path).unwrap();

    let path_clone = lock_path.clone();
    let thread_done = Arc::new(AtomicBool::new(false));
    let thread_done_c = thread_done.clone();

    // Spawn thread that holds lock1 for 60ms then releases
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(60));
        drop(lock1);
        thread_done_c.store(true, Ordering::SeqCst);
    });

    // Main thread polls with timeout of 500ms
    let lock2 = ProcessLock::acquire_with_timeout(
        &path_clone,
        Duration::from_millis(500),
        Duration::from_millis(15),
    )
    .expect("Should acquire lock once worker thread releases it");

    assert!(lock2.is_held());
    assert!(thread_done.load(Ordering::SeqCst));
    drop(lock2);

    let _ = std::fs::remove_file(&lock_path);
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_concurrency_lock_timeout_expiry() {
    let temp_dir = std::env::temp_dir().join(format!("af_timeout_exp_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let lock_path = temp_dir.join("timeout_expire.lock");

    let _lock1 = ProcessLock::try_acquire_at(&lock_path).unwrap();

    // Attempt acquisition with short timeout (40ms) while lock1 is kept alive
    let start = std::time::Instant::now();
    let lock2_res = ProcessLock::acquire_with_timeout(
        &lock_path,
        Duration::from_millis(40),
        Duration::from_millis(10),
    );
    let elapsed = start.elapsed();

    assert!(lock2_res.is_err(), "Must time out and return error");
    assert!(
        elapsed >= Duration::from_millis(35),
        "Elapsed time must reflect timeout duration"
    );

    let err_msg = lock2_res.err().unwrap().to_string();
    assert!(
        err_msg.contains("Timed out"),
        "Error message must indicate timeout: {}",
        err_msg
    );

    let _ = std::fs::remove_file(&lock_path);
    let _ = std::fs::remove_dir(&temp_dir);
}

// ==============================================================================
// 2. In-Sample Threshold Evaluation Tests
// ==============================================================================

fn evaluate_candidate_is_gate(
    sharpe: Option<f64>,
    fitness: Option<f64>,
    turnover: Option<f64>,
    returns: Option<f64>,
    delay: i32,
) -> bool {
    let req_sharpe = min_sharpe_for_delay(delay);
    let req_fitness = min_fitness_for_delay(delay);

    match (sharpe, fitness, turnover, returns) {
        (Some(s), Some(f), Some(t), Some(r)) => {
            s >= req_sharpe && f >= req_fitness && turnover_within_platform_bounds(t) && r > 0.08
        }
        _ => false, // Fail closed on missing metrics
    }
}

#[test]
fn test_in_sample_threshold_qualification_gate_boundaries() {
    // Platform Delay 1 thresholds: Sharpe >= 1.25, Fitness >= 1.00
    assert_eq!(min_sharpe_for_delay(1), 1.25);
    assert_eq!(min_fitness_for_delay(1), 1.00);

    // Exact passing boundary: Sharpe=1.25, Fitness=1.00, Turnover=0.015 (1.5%), Returns=0.081 (>8%)
    assert!(
        evaluate_candidate_is_gate(Some(1.25), Some(1.00), Some(0.015), Some(0.081), 1),
        "Exact boundary values must qualify"
    );

    // Typical qualifying candidate: Sharpe=1.55, Fitness=1.40, Turnover=0.18, Returns=0.22
    assert!(
        evaluate_candidate_is_gate(Some(1.55), Some(1.40), Some(0.18), Some(0.22), 1),
        "Strong alternative candidate must qualify"
    );

    // Sharpe below threshold
    assert!(
        !evaluate_candidate_is_gate(Some(1.249), Some(1.00), Some(0.18), Some(0.15), 1),
        "Sharpe < 1.25 must fail"
    );

    // Fitness below threshold
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(0.999), Some(0.18), Some(0.15), 1),
        "Fitness < 1.00 must fail"
    );

    // Turnover below threshold (<= 1.0% fails strictly > 1% rule)
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.010), Some(0.15), 1),
        "Turnover == 1.0% must fail strictly > 1% platform rule"
    );
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.009), Some(0.15), 1),
        "Turnover < 1.0% must fail"
    );

    // Turnover above threshold (>= 70.0% fails strictly < 70% rule)
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.700), Some(0.15), 1),
        "Turnover == 70.0% must fail strictly < 70% platform rule"
    );
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.701), Some(0.15), 1),
        "Turnover > 70.0% must fail"
    );

    // Returns equal to or below 8.0%
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.20), Some(0.080), 1),
        "Returns == 8.0% must fail (strictly > 8.0%)"
    );
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.20), Some(0.05), 1),
        "Returns < 8.0% must fail"
    );

    // Missing metric fails closed
    assert!(
        !evaluate_candidate_is_gate(None, Some(1.20), Some(0.20), Some(0.15), 1),
        "None Sharpe must fail closed"
    );
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), None, Some(0.20), Some(0.15), 1),
        "None Fitness must fail closed"
    );
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), None, Some(0.15), 1),
        "None Turnover must fail closed"
    );
    assert!(
        !evaluate_candidate_is_gate(Some(1.50), Some(1.20), Some(0.20), None, 1),
        "None Returns must fail closed"
    );
}

// ==============================================================================
// 3. Fail-Closed Platform Submission Check Gate Tests
// ==============================================================================

fn make_check(name: &str, result: &str) -> CheckItem {
    CheckItem {
        name: name.to_string(),
        result: result.to_string(),
        ..CheckItem::default()
    }
}

#[test]
fn test_submission_check_gate_all_six_core_pass() {
    let checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();

    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };

    let res = evaluate_submission_checks(&stats);
    assert!(res.passed, "All 6 core checks PASS must qualify");
    assert_eq!(res.passed_count, 6);
    assert_eq!(res.total_count, 6);
    assert!(res.failed_checks.is_empty());
    assert!(res.pending_checks.is_empty());
    assert!(res.missing_core_checks.is_empty());
    assert!(res.message.contains("6/6 PASS"));
}

#[test]
fn test_submission_check_gate_missing_core_fails_closed() {
    // 5 checks present, missing "SELF_CORRELATION"
    let checks = vec![
        make_check("LOW_SHARPE", "PASS"),
        make_check("LOW_FITNESS", "PASS"),
        make_check("LOW_TURNOVER", "PASS"),
        make_check("HIGH_TURNOVER", "PASS"),
        make_check("CONCENTRATED_WEIGHT", "PASS"),
    ];

    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };

    let res = evaluate_submission_checks(&stats);
    assert!(!res.passed, "Missing SELF_CORRELATION must fail closed");
    assert!(res
        .missing_core_checks
        .contains(&"SELF_CORRELATION".to_string()));
    assert!(res.message.contains("Missing required core checks"));
}

#[test]
fn test_submission_check_gate_pending_fails_closed() {
    let mut checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();

    // Mark LOW_SHARPE as PENDING
    checks[0].result = "PENDING".to_string();

    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };

    let res = evaluate_submission_checks(&stats);
    assert!(!res.passed, "PENDING check must fail closed");
    assert_eq!(res.pending_checks, vec!["LOW_SHARPE".to_string()]);
    assert!(res.message.contains("CHECKS STILL PENDING"));
}

#[test]
fn test_submission_check_gate_failed_check_fails_closed() {
    let mut checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();

    // Mark LOW_FITNESS as FAIL
    checks[1].result = "FAIL".to_string();

    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };

    let res = evaluate_submission_checks(&stats);
    assert!(!res.passed, "FAIL check must reject candidate");
    assert_eq!(res.failed_checks, vec!["LOW_FITNESS".to_string()]);
    assert!(res.message.contains("FAILED CHECKS"));
}

#[test]
fn test_submission_check_gate_self_correlation_fail_formatting() {
    let mut checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();

    // Mark SELF_CORRELATION as FAIL
    checks[5].result = "FAIL".to_string();

    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };

    let res = evaluate_submission_checks(&stats);
    assert!(!res.passed);
    assert!(res.failed_checks[0].contains("SELF_CORRELATION"));
}

#[test]
fn test_submission_check_gate_empty_response_fails_closed() {
    let stats = InSampleStats {
        checks: Vec::new(),
        ..InSampleStats::default()
    };

    let res = evaluate_submission_checks(&stats);
    assert!(!res.passed, "Empty check response must fail closed");
    assert!(res.message.contains("FAIL-CLOSED: Empty check results"));
}

// ==============================================================================
// 4. Pearson Correlation & Portfolio Merge Delta Math Tests
// ==============================================================================

#[test]
fn test_pearson_correlation_minimum_overlap_boundary() {
    // 29 common days (< MIN_CORRELATION_DAYS = 30)
    let mut x_29 = HashMap::new();
    let mut y_29 = HashMap::new();
    for i in 1..=29 {
        let d = format!("2023-01-{:02}", i);
        x_29.insert(d.clone(), i as f64);
        y_29.insert(d, (i * 2) as f64);
    }

    let (corr_29, n_29) = pearson_correlation(&x_29, &y_29);
    assert_eq!(n_29, 29);
    assert!(
        corr_29.is_none(),
        "N = 29 common days must fail closed and return None"
    );

    // 30 common days (>= MIN_CORRELATION_DAYS = 30)
    let mut x_30 = x_29.clone();
    let mut y_30 = y_29.clone();
    x_30.insert("2023-01-30".to_string(), 30.0);
    y_30.insert("2023-01-30".to_string(), 60.0);

    let (corr_30, n_30) = pearson_correlation(&x_30, &y_30);
    assert_eq!(n_30, 30);
    assert!(
        corr_30.is_some(),
        "N = 30 common days must compute exact Pearson correlation"
    );
    let val = corr_30.unwrap();
    assert!(
        (val - 1.0).abs() < 1e-9,
        "Perfect linear correlation must equal 1.0"
    );
}

#[test]
fn test_pearson_correlation_zero_variance_fails_closed() {
    let mut x = HashMap::new();
    let mut y = HashMap::new();
    for i in 1..=40 {
        let d = format!("2023-01-{:02}", i);
        x.insert(d.clone(), 5.0); // Constant / zero variance series
        y.insert(d, (i as f64) * 1.5);
    }

    let (corr, n) = pearson_correlation(&x, &y);
    assert_eq!(n, 40);
    assert!(
        corr.is_none(),
        "Constant series must return None (fail closed against division by zero)"
    );
}

#[test]
fn test_simulate_portfolio_insufficient_trading_days() {
    let mut p1 = HashMap::new();
    let mut p2 = HashMap::new();
    // 99 days (< MIN_PORTFOLIO_DAYS = 100)
    for i in 1..=99 {
        let d = format!("d_{:03}", i);
        p1.insert(d.clone(), 10.0 + (i as f64));
        p2.insert(d, 20.0 + (i as f64));
    }

    let res = simulate_portfolio(&[&p1, &p2]);
    assert!(
        res.is_none(),
        "Portfolio simulation with 99 days must fail closed"
    );

    // Add 100th day
    p1.insert("d_100".to_string(), 110.0);
    p2.insert("d_100".to_string(), 120.0);

    let res_100 = simulate_portfolio(&[&p1, &p2]);
    assert!(
        res_100.is_some(),
        "Portfolio simulation with >= 100 days must succeed"
    );
    let m = res_100.unwrap();
    assert_eq!(m.n_alphas, 2);
    assert_eq!(m.n_trading_days, 100);
    assert!(m.annualized_pnl > 0.0);
    assert!(m.annualized_vol > 0.0);
    assert!(m.merged_sharpe > 0.0);
}

#[test]
fn test_portfolio_merge_delta_math_accretive_candidate() {
    // Construct synthetic baseline of 5 alphas across 150 trading days
    let n_days = 150;
    let mut os_pnls: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for a_idx in 0..5 {
        let mut pnl = HashMap::new();
        for d in 0..n_days {
            let date = format!("2023-D{:03}", d);
            // Non-zero baseline daily PnL with sinusoidal drift
            let val = 100.0 + 20.0 * ((d as f64 * 0.1 + a_idx as f64).sin());
            pnl.insert(date, val);
        }
        os_pnls.insert(format!("OS_ALPHA_{}", a_idx), pnl);
    }

    // Candidate Alpha with strong positive drift and distinct orthogonal frequency
    let mut cand_pnl = HashMap::new();
    for d in 0..n_days {
        let date = format!("2023-D{:03}", d);
        let val = 150.0 + 15.0 * ((d as f64 * 0.45 + 1.2).cos());
        cand_pnl.insert(date, val);
    }

    let impact = calculate_portfolio_impact("CAND_ACC_01", &cand_pnl, &os_pnls)
        .expect("Impact calculation must succeed for valid data");

    // Accretive qualification checks:
    assert!(
        impact.delta_sharpe > 0.0,
        "Accretive candidate must increase portfolio Sharpe: delta_sharpe = {}",
        impact.delta_sharpe
    );
    assert!(
        impact.max_pairwise_corr < BRAIN_MAX_CORRELATION,
        "Candidate correlation must be below 0.70 proxy threshold: max_corr = {}",
        impact.max_pairwise_corr
    );
    assert!(
        impact.safety_buffer_pct > 0.0,
        "Safety buffer must be positive"
    );
    assert_eq!(impact.baseline_alphas, 5);
    assert_eq!(impact.new_alphas, 6);
}

#[test]
fn test_portfolio_merge_delta_math_high_correlation_candidate() {
    let n_days = 150;
    let mut os_pnls: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for a_idx in 0..4 {
        let mut pnl = HashMap::new();
        for d in 0..n_days {
            let date = format!("2023-D{:03}", d);
            pnl.insert(date, 50.0 + 10.0 * ((d as f64 * 0.05 + a_idx as f64).sin()));
        }
        os_pnls.insert(format!("OS_{}", a_idx), pnl);
    }

    // Candidate is a clone of OS_0 with slight multiplier (correlation ~ 1.0)
    let clone_source = &os_pnls["OS_0"];
    let mut cand_clone = HashMap::new();
    for (d, &val) in clone_source {
        cand_clone.insert(d.clone(), val * 1.02);
    }

    let impact = calculate_portfolio_impact("CAND_CLONE", &cand_clone, &os_pnls)
        .expect("Calculation succeeds, but values should reflect high correlation");

    assert!(
        impact.max_pairwise_corr >= 0.70,
        "Clone candidate must exceed 0.70 max correlation: got {}",
        impact.max_pairwise_corr
    );
    assert_eq!(
        impact.safety_buffer_pct, 0.0,
        "Safety buffer must be 0% when max correlation >= 0.70"
    );
}

#[test]
fn test_audit_candidate_skips_self_and_fails_closed_when_empty() {
    let mut os_pnls = HashMap::new();
    let mut pnl_a = HashMap::new();
    for i in 1..=40 {
        pnl_a.insert(format!("d_{}", i), i as f64);
    }
    os_pnls.insert("ALPHA_SELF".to_string(), pnl_a.clone());

    // Auditing against portfolio where only ALPHA_SELF exists -> valid_comparisons = 0
    let audit = audit_candidate("ALPHA_SELF", &pnl_a, &os_pnls);
    assert_eq!(audit.valid_comparisons, 0, "Candidate must skip itself");
    assert!(
        !audit.is_within_correlation_proxy,
        "With 0 comparisons, is_within_correlation_proxy must be false (fail-closed)"
    );
}
