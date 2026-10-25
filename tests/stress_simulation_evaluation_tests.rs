//! Empirical Stress Test Suite for Milestone M4:
//! Quantitative evaluation math, correlation boundaries, fail-closed check evaluator,
//! and portfolio merge delta impact.

use alphafind::client::{evaluate_submission_checks, CORE_SUBMISSION_CHECKS};
use alphafind::config::quant::{BRAIN_MAX_CORRELATION, MIN_CORRELATION_DAYS, MIN_PORTFOLIO_DAYS};
use alphafind::correlation::{
    audit_candidate, calculate_portfolio_impact, pearson_correlation, simulate_portfolio,
};
use alphafind::models::{CheckItem, InSampleStats};
use std::collections::HashMap;

fn make_check(name: &str, result: &str) -> CheckItem {
    CheckItem {
        name: name.to_string(),
        result: result.to_string(),
        ..CheckItem::default()
    }
}

// ==============================================================================
// 1. Pearson Correlation & Overlap Boundary Stress Tests
// ==============================================================================

#[test]
fn test_stress_pearson_correlation_overlap_spectrum_0_to_35() {
    for n in 0..=35 {
        let mut x = HashMap::new();
        let mut y = HashMap::new();
        for i in 1..=n {
            let d = format!("2023-D{:03}", i);
            x.insert(d.clone(), i as f64);
            y.insert(d, (i * 3) as f64);
        }

        let (corr, common_days) = pearson_correlation(&x, &y);
        assert_eq!(common_days, n, "Common days count must equal exact overlap");

        if n < MIN_CORRELATION_DAYS {
            assert!(
                corr.is_none(),
                "For N = {} (< 30), correlation must fail-closed and return None",
                n
            );
        } else {
            assert!(
                corr.is_some(),
                "For N = {} (>= 30), correlation must compute",
                n
            );
            let val = corr.unwrap();
            assert!(
                (val - 1.0).abs() < 1e-9,
                "For linear series, correlation must be 1.0"
            );
        }
    }
}

#[test]
fn test_stress_pearson_correlation_nan_inf_filtering() {
    // 50 total dates, but with various NaN and Infinity injections
    let mut x = HashMap::new();
    let mut y = HashMap::new();

    // Days 1..25: clean finite values
    for i in 1..=25 {
        let d = format!("d_{:02}", i);
        x.insert(d.clone(), i as f64);
        y.insert(d, (i * 2) as f64);
    }

    // Days 26..29: clean finite values (now 29 finite common days)
    for i in 26..=29 {
        let d = format!("d_{:02}", i);
        x.insert(d.clone(), i as f64);
        y.insert(d, (i * 2) as f64);
    }

    // Days 30..35: NaN injected in x
    for i in 30..=35 {
        let d = format!("d_{:02}", i);
        x.insert(d.clone(), f64::NAN);
        y.insert(d, (i * 2) as f64);
    }

    // Days 36..40: +inf injected in y
    for i in 36..=40 {
        let d = format!("d_{:02}", i);
        x.insert(d.clone(), i as f64);
        y.insert(d, f64::INFINITY);
    }

    // Days 41..45: -inf injected in x
    for i in 41..=45 {
        let d = format!("d_{:02}", i);
        x.insert(d.clone(), f64::NEG_INFINITY);
        y.insert(d, (i * 2) as f64);
    }

    // At this point, finite common days = 29 (< 30)
    let (corr_29, n_29) = pearson_correlation(&x, &y);
    assert_eq!(n_29, 29, "Only finite common days must be counted");
    assert!(
        corr_29.is_none(),
        "29 finite common days must fail-closed (return None)"
    );

    // Add 30th finite common day
    x.insert("d_46".to_string(), 46.0);
    y.insert("d_46".to_string(), 92.0);

    let (corr_30, n_30) = pearson_correlation(&x, &y);
    assert_eq!(n_30, 30);
    assert!(
        corr_30.is_some(),
        "30 finite common days must compute valid Pearson correlation"
    );
    assert!((corr_30.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn test_stress_pearson_correlation_zero_variance_and_epsilon_thresholds() {
    let mut x = HashMap::new();
    let mut y = HashMap::new();

    // 40 days
    for i in 1..=40 {
        let d = format!("d_{:02}", i);
        x.insert(d.clone(), 100.0); // Constant 100.0 (var = 0.0)
        y.insert(d, (i as f64) * 2.0);
    }

    let (corr_zero, _) = pearson_correlation(&x, &y);
    assert!(
        corr_zero.is_none(),
        "Zero variance on x must return None (fail closed)"
    );

    // Swap: y constant, x variable
    let (corr_zero_y, _) = pearson_correlation(&y, &x);
    assert!(
        corr_zero_y.is_none(),
        "Zero variance on y must return None (fail closed)"
    );

    // Both constant
    let mut y_const = HashMap::new();
    for i in 1..=40 {
        y_const.insert(format!("d_{:02}", i), 50.0);
    }
    let (corr_both_zero, _) = pearson_correlation(&x, &y_const);
    assert!(
        corr_both_zero.is_none(),
        "Zero variance on both must return None"
    );

    // Micro-variance strictly below EPSILON_VARIANCE (1e-12)
    let mut x_micro = HashMap::new();
    for i in 1..=40 {
        let d = format!("d_{:02}", i);
        // Perturbation of 1e-9 -> variance across 40 items is around (1e-9)^2 = 1e-18 < 1e-12
        x_micro.insert(d, 100.0 + (i as f64) * 1e-9);
    }
    let (corr_micro, _) = pearson_correlation(&x_micro, &y);
    assert!(
        corr_micro.is_none(),
        "Sub-epsilon variance must fail-closed as zero variance"
    );
}

#[test]
fn test_stress_pearson_correlation_extreme_overflow_scale() {
    let mut x = HashMap::new();
    let mut y = HashMap::new();

    // Large numbers that trigger overflow in sum of squares (dx * dx)
    for i in 1..=40 {
        let d = format!("d_{:02}", i);
        // 1e160 squared is 1e320 which overflows IEEE 754 f64 to INFINITY
        x.insert(d.clone(), (i as f64) * 1e160);
        y.insert(d, (i as f64) * 2.0);
    }

    let (corr, n) = pearson_correlation(&x, &y);
    assert_eq!(n, 40);
    // Observe exact behavior when intermediate variance overflows to infinity
    println!("Overflow test result: corr = {:?}", corr);
    // If corr is Some(0.0), this demonstrates that cov / inf evaluates to 0.0
    // Documenting this finding!
}

#[test]
fn test_stress_self_correlation_skipping_in_audit_and_impact() {
    let mut os_pnls = HashMap::new();
    // simulate_portfolio requires n_days >= MIN_PORTFOLIO_DAYS (100)
    let n_days = 120;

    // Build 3 OS alphas
    for a_idx in 1..=3 {
        let mut p = HashMap::new();
        for d in 1..=n_days {
            p.insert(format!("2023-D{:03}", d), (d as f64) * (a_idx as f64));
        }
        os_pnls.insert(format!("ALPHA_{:02}", a_idx), p);
    }

    // Now insert the candidate itself into os_pnls with an identical clone PnL
    let mut cand_pnl = HashMap::new();
    for d in 1..=n_days {
        cand_pnl.insert(format!("2023-D{:03}", d), (d as f64) * 10.0);
    }
    os_pnls.insert("CAND_SELF".to_string(), cand_pnl.clone());

    // Run audit_candidate for "CAND_SELF"
    let audit = audit_candidate("CAND_SELF", &cand_pnl, &os_pnls);
    assert_eq!(
        audit.valid_comparisons, 3,
        "Candidate must be compared against 3 OS alphas, skipping itself"
    );
    for (partner_id, _, _) in &audit.pairwise_results {
        assert_ne!(
            partner_id, "CAND_SELF",
            "audit_candidate must never compare candidate against itself"
        );
    }

    // Run calculate_portfolio_impact for "CAND_SELF"
    let impact = calculate_portfolio_impact("CAND_SELF", &cand_pnl, &os_pnls)
        .expect("Portfolio impact calculation should succeed with 120 days");

    assert_eq!(
        impact.baseline_alphas, 3,
        "Baseline must exclude CAND_SELF (3 alphas)"
    );
    assert_eq!(
        impact.new_alphas, 4,
        "New portfolio must contain baseline + CAND_SELF (4 alphas)"
    );
    assert_ne!(
        impact.most_correlated_id, "CAND_SELF",
        "Most correlated alpha ID must not be candidate itself"
    );
}

// ==============================================================================
// 2. Submission Checks Fail-Closed Gate Stress Tests
// ==============================================================================

#[test]
fn test_stress_submission_checks_all_single_missing_core_permutations() {
    for (missing_idx, &missing_name) in CORE_SUBMISSION_CHECKS.iter().enumerate() {
        let checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
            .iter()
            .enumerate()
            .filter(|&(idx, _)| idx != missing_idx)
            .map(|(_, name)| make_check(name, "PASS"))
            .collect();

        assert_eq!(checks.len(), 5, "Must test exactly 5 of 6 core checks");

        let stats = InSampleStats {
            checks,
            ..InSampleStats::default()
        };

        let res = evaluate_submission_checks(&stats);
        assert!(
            !res.passed,
            "Gate must fail-closed when '{}' is missing",
            missing_name
        );
        assert!(
            res.missing_core_checks.contains(&missing_name.to_string()),
            "missing_core_checks must list '{}'",
            missing_name
        );
        assert!(
            res.message.contains("Missing required core checks"),
            "Message must report missing core checks: got '{}'",
            res.message
        );
    }
}

#[test]
fn test_stress_submission_checks_unrecognized_and_malformed_statuses() {
    let adversarial_statuses = [
        "UNKNOWN",
        "ERROR",
        "WARNING",
        "SKIPPED",
        "IN_PROGRESS",
        "QUEUED",
        "",
        "   ",
        "12345",
        "NULL",
        "UNDEFINED",
    ];

    for bad_status in adversarial_statuses {
        let mut checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
            .iter()
            .map(|name| make_check(name, "PASS"))
            .collect();

        // Mutate one check to the adversarial status
        checks[0].result = bad_status.to_string();

        let stats = InSampleStats {
            checks,
            ..InSampleStats::default()
        };

        let res = evaluate_submission_checks(&stats);
        assert!(
            !res.passed,
            "Gate must fail-closed for unrecognized status '{}'",
            bad_status
        );
        assert!(
            res.failed_checks.iter().any(|f| f.contains(bad_status)),
            "failed_checks must capture unrecognized status '{}': got {:?}",
            bad_status,
            res.failed_checks
        );
    }
}

#[test]
fn test_stress_submission_checks_pending_variations() {
    // 1. Single core check PENDING
    let mut checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();
    checks[2].result = "PENDING".to_string();

    let stats1 = InSampleStats {
        checks: checks.clone(),
        ..InSampleStats::default()
    };
    let res1 = evaluate_submission_checks(&stats1);
    assert!(!res1.passed);
    assert_eq!(res1.pending_checks, vec!["LOW_TURNOVER".to_string()]);
    assert!(res1.message.contains("CHECKS STILL PENDING"));

    // 2. Multiple core checks PENDING
    checks[3].result = "PENDING".to_string();
    let stats2 = InSampleStats {
        checks: checks.clone(),
        ..InSampleStats::default()
    };
    let res2 = evaluate_submission_checks(&stats2);
    assert!(!res2.passed);
    assert_eq!(res2.pending_checks.len(), 2);

    // 3. Extra non-core check PENDING
    let mut checks3: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();
    checks3.push(make_check("SUB_UNIVERSE", "PENDING"));
    let stats3 = InSampleStats {
        checks: checks3,
        ..InSampleStats::default()
    };
    let res3 = evaluate_submission_checks(&stats3);
    assert!(!res3.passed, "Non-core check PENDING must also fail closed");
    assert!(res3.pending_checks.contains(&"SUB_UNIVERSE".to_string()));
}

#[test]
fn test_stress_submission_checks_non_core_failure() {
    let mut checks: Vec<CheckItem> = CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| make_check(name, "PASS"))
        .collect();
    // All 6 core checks pass, but an additional platform check fails
    checks.push(make_check("SUB_UNIVERSE", "FAIL"));

    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };
    let res = evaluate_submission_checks(&stats);
    assert!(
        !res.passed,
        "Any non-PASS check in the suite must fail the gate"
    );
    assert!(res.failed_checks.contains(&"SUB_UNIVERSE".to_string()));
}

#[test]
fn test_stress_submission_checks_already_submitted_explains_and_fails_closed() {
    let checks = vec![make_check("ALREADY_SUBMITTED", "PASS")];
    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };
    let res = evaluate_submission_checks(&stats);
    assert!(
        !res.passed,
        "ALREADY_SUBMITTED response must fail closed (not submittable again)"
    );
    assert!(res.message.contains("ALREADY SUBMITTED"));
}

#[test]
fn test_stress_submission_checks_case_sensitivity_fails_closed() {
    // Platform API sends uppercase names. If lowercase names are passed,
    // they should not match CORE_SUBMISSION_CHECKS and must fail closed.
    let checks = vec![
        make_check("low_sharpe", "PASS"),
        make_check("low_fitness", "PASS"),
        make_check("low_turnover", "PASS"),
        make_check("high_turnover", "PASS"),
        make_check("concentrated_weight", "PASS"),
        make_check("self_correlation", "PASS"),
    ];
    let stats = InSampleStats {
        checks,
        ..InSampleStats::default()
    };
    let res = evaluate_submission_checks(&stats);
    assert!(
        !res.passed,
        "Mismatched casing on check names must fail closed as missing core checks"
    );
    assert_eq!(res.missing_core_checks.len(), 6);
}

// ==============================================================================
// 3. Portfolio Merge Delta Math Stress Tests
// ==============================================================================

#[test]
fn test_stress_portfolio_simulation_overlap_spectrum_90_to_105() {
    for n_days in 90..=105 {
        let mut p1 = HashMap::new();
        let mut p2 = HashMap::new();
        for i in 1..=n_days {
            let d = format!("d_{:03}", i);
            p1.insert(d.clone(), 100.0 + (i as f64));
            p2.insert(d, 200.0 + (i as f64) * 0.5);
        }

        let res = simulate_portfolio(&[&p1, &p2]);
        if n_days < MIN_PORTFOLIO_DAYS {
            assert!(
                res.is_none(),
                "For {} common days (< 100), portfolio simulation must return None",
                n_days
            );
        } else {
            assert!(
                res.is_some(),
                "For {} common days (>= 100), portfolio simulation must succeed",
                n_days
            );
            let m = res.unwrap();
            assert_eq!(m.n_trading_days, n_days);
            assert_eq!(m.n_alphas, 2);
        }
    }
}

#[test]
fn test_stress_portfolio_simulation_nan_inf_filtering() {
    let mut p1 = HashMap::new();
    let mut p2 = HashMap::new();

    // 110 days total
    for i in 1..=110 {
        let d = format!("d_{:03}", i);
        p1.insert(d.clone(), 50.0 + (i as f64));
        p2.insert(d, 60.0 + (i as f64));
    }

    // Invalidate 15 days with NaN in p1 -> 95 valid common days remaining (< 100)
    for i in 1..=15 {
        let d = format!("d_{:03}", i);
        p1.insert(d, f64::NAN);
    }

    let res = simulate_portfolio(&[&p1, &p2]);
    assert!(
        res.is_none(),
        "Portfolio simulation with 95 valid finite days must fail closed"
    );

    // Restore 6 days -> 101 valid common days
    for i in 10..=15 {
        let d = format!("d_{:03}", i);
        p1.insert(d, 50.0 + (i as f64));
    }

    let res_valid = simulate_portfolio(&[&p1, &p2]);
    assert!(
        res_valid.is_some(),
        "Portfolio simulation with 101 valid finite days must succeed"
    );
    assert_eq!(res_valid.unwrap().n_trading_days, 101);
}

#[test]
fn test_stress_portfolio_impact_accretive_vs_destructive() {
    let n_days = 150;
    let mut os_pnls: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for a_idx in 0..5 {
        let mut pnl = HashMap::new();
        for d in 0..n_days {
            let date = format!("2023-D{:03}", d);
            let val = 80.0 + 15.0 * ((d as f64 * 0.08 + a_idx as f64).sin());
            pnl.insert(date, val);
        }
        os_pnls.insert(format!("OS_{}", a_idx), pnl);
    }

    // 1. Accretive candidate: high mean, low correlation
    let mut accretive_pnl = HashMap::new();
    for d in 0..n_days {
        let date = format!("2023-D{:03}", d);
        let val = 160.0 + 10.0 * ((d as f64 * 0.4 + 2.0).cos());
        accretive_pnl.insert(date, val);
    }

    let acc_impact = calculate_portfolio_impact("ACC_01", &accretive_pnl, &os_pnls)
        .expect("Accretive calculation should succeed");

    assert!(
        acc_impact.delta_sharpe > 0.0,
        "Accretive must increase Sharpe"
    );
    assert!(
        acc_impact.delta_vol <= 0.0,
        "Accretive must decrease or maintain Vol"
    );
    assert!(acc_impact.max_pairwise_corr < BRAIN_MAX_CORRELATION);

    // 2. Destructive candidate: massive negative drift and high volatility
    let mut destructive_pnl = HashMap::new();
    for d in 0..n_days {
        let date = format!("2023-D{:03}", d);
        let val = -200.0 + 100.0 * ((d as f64 * 0.5).sin());
        destructive_pnl.insert(date, val);
    }

    let dest_impact = calculate_portfolio_impact("DEST_01", &destructive_pnl, &os_pnls)
        .expect("Destructive calculation should succeed");

    assert!(
        dest_impact.delta_sharpe < 0.0,
        "Destructive candidate must degrade Sharpe: got {}",
        dest_impact.delta_sharpe
    );
    assert!(
        dest_impact.delta_vol > 0.0,
        "Destructive candidate must increase volatility: got {}",
        dest_impact.delta_vol
    );
}

#[test]
fn test_stress_portfolio_impact_disjoint_dates_fails_closed() {
    let mut os_pnls = HashMap::new();
    let mut base_pnl = HashMap::new();
    for i in 1..=120 {
        base_pnl.insert(format!("2023-D{:03}", i), 50.0);
    }
    os_pnls.insert("OS_01".to_string(), base_pnl);

    // Candidate has completely disjoint dates in 2024
    let mut cand_pnl = HashMap::new();
    for i in 1..=120 {
        cand_pnl.insert(format!("2024-D{:03}", i), 60.0);
    }

    let impact = calculate_portfolio_impact("CAND_DISJOINT", &cand_pnl, &os_pnls);
    assert!(
        impact.is_none(),
        "Disjoint date range between candidate and baseline must return None (fail closed)"
    );
}
