use alphafind::correlation::{
    audit_candidate, calculate_portfolio_impact, pearson_correlation, simulate_portfolio,
};
use std::collections::HashMap;

/// Helper to create a PnL map from a slice of (date, value) pairs
fn make_pnl(data: &[(&str, f64)]) -> HashMap<String, f64> {
    data.iter().map(|(d, v)| (d.to_string(), *v)).collect()
}

/// Generate synthetic daily PnL with n days starting from 2020-01-01
fn generate_pnl(n: usize, seed: f64) -> HashMap<String, f64> {
    (0..n)
        .map(|i| {
            let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
            let value = ((i as f64 * seed).sin()) * 0.01;
            (date, value)
        })
        .collect()
}

#[test]
fn test_pearson_perfect_positive_correlation() {
    let x = generate_pnl(100, 1.0);
    let y = x.clone();
    let (corr_opt, days) = pearson_correlation(&x, &y);
    assert_eq!(days, 100);
    let corr = corr_opt.expect("Expected Some(1.0) for perfect positive correlation");
    assert!(
        (corr - 1.0).abs() < 1e-10,
        "Perfect self-correlation should be 1.0, got {}",
        corr
    );
}

#[test]
fn test_pearson_perfect_negative_correlation() {
    let x = generate_pnl(100, 1.0);
    let y: HashMap<String, f64> = x.iter().map(|(k, v)| (k.clone(), -v)).collect();
    let (corr_opt, days) = pearson_correlation(&x, &y);
    assert_eq!(days, 100);
    let corr = corr_opt.expect("Expected Some(-1.0) for perfect negative correlation");
    assert!(
        (corr + 1.0).abs() < 1e-10,
        "Perfect negative correlation should be -1.0, got {}",
        corr
    );
}

#[test]
fn test_pearson_insufficient_days_fails_closed() {
    let x = make_pnl(&[("2020-01-01", 1.0), ("2020-01-02", 2.0)]);
    let y = make_pnl(&[("2020-01-01", 3.0), ("2020-01-02", 4.0)]);
    let (corr, days) = pearson_correlation(&x, &y);
    assert_eq!(days, 2);
    assert!(
        corr.is_none(),
        "Should fail-closed (return None) when less than 30 common days"
    );
}

#[test]
fn test_pearson_no_overlap_fails_closed() {
    let x = make_pnl(&[("2020-01-01", 1.0)]);
    let y = make_pnl(&[("2020-02-01", 1.0)]);
    let (corr, days) = pearson_correlation(&x, &y);
    assert_eq!(days, 0);
    assert!(
        corr.is_none(),
        "Should fail-closed (return None) when no overlap"
    );
}

#[test]
fn test_pearson_constant_series_fails_closed() {
    let n = 50;
    let x: HashMap<String, f64> = (0..n)
        .map(|i| (format!("2020-01-{:02}", i + 1), 5.0))
        .collect();
    let y = generate_pnl(n, 2.0);
    // Only common dates matter
    let common: HashMap<String, f64> = x
        .keys()
        .filter(|k| y.contains_key(*k))
        .map(|k| (k.clone(), 5.0))
        .collect();
    let (corr, _days) = pearson_correlation(&common, &y);
    // Constant series has zero variance, must fail-closed as None
    assert!(
        corr.is_none(),
        "Constant series has zero variance, should return None"
    );
}

#[test]
fn test_audit_candidate_basic() {
    let cand = generate_pnl(200, 1.0);
    let mut os_pnls = HashMap::new();
    os_pnls.insert("alpha_A".to_string(), generate_pnl(200, 2.0));
    os_pnls.insert("alpha_B".to_string(), generate_pnl(200, 3.0));

    let report = audit_candidate("candidate", &cand, &os_pnls);
    assert_eq!(report.candidate_id, "candidate");
    assert_eq!(report.pairwise_results.len(), 2);
    assert_eq!(report.valid_comparisons, 2);
    assert_eq!(report.insufficient_overlap_count, 0);
    let max = report
        .max_correlation
        .expect("Should have valid max correlation");
    let avg = report
        .avg_correlation
        .expect("Should have valid avg correlation");
    assert!((-1.0..=1.0).contains(&max));
    assert!((-1.0..=1.0).contains(&avg));
}

#[test]
fn test_audit_skips_self() {
    let cand = generate_pnl(200, 1.0);
    let mut os_pnls = HashMap::new();
    os_pnls.insert("candidate".to_string(), cand.clone()); // Same ID as candidate
    os_pnls.insert("alpha_A".to_string(), generate_pnl(200, 2.0));

    let report = audit_candidate("candidate", &cand, &os_pnls);
    // Should skip "candidate" from OS portfolio, only compare with alpha_A
    assert_eq!(report.pairwise_results.len(), 1);
    assert_eq!(report.valid_comparisons, 1);
}

#[test]
fn test_audit_empty_portfolio_fails_closed() {
    let cand = generate_pnl(200, 1.0);
    let os_pnls = HashMap::new();

    let report = audit_candidate("candidate", &cand, &os_pnls);
    assert_eq!(report.pairwise_results.len(), 0);
    assert_eq!(report.valid_comparisons, 0);
    assert!(report.max_correlation.is_none());
    assert!(report.avg_correlation.is_none());
    // Crucial fail-closed guarantees: cannot claim compliant or orthogonal without data!
    assert!(!report.is_within_correlation_proxy);
    assert!(!report.is_locally_orthogonal);
}

#[test]
fn test_audit_insufficient_overlap_fails_closed() {
    let cand = make_pnl(&[("2020-01-01", 1.0), ("2020-01-02", 2.0)]);
    let mut os_pnls = HashMap::new();
    os_pnls.insert(
        "alpha_short_history".to_string(),
        make_pnl(&[("2020-01-01", 3.0), ("2020-01-02", 4.0)]),
    );

    let report = audit_candidate("candidate", &cand, &os_pnls);
    assert_eq!(report.pairwise_results.len(), 0);
    assert_eq!(report.valid_comparisons, 0);
    assert_eq!(report.insufficient_overlap_count, 1);
    assert!(report.max_correlation.is_none());
    assert!(!report.is_within_correlation_proxy);
    assert!(!report.is_locally_orthogonal);
}

#[test]
fn test_simulate_portfolio_empty() {
    let result = simulate_portfolio(&[]);
    assert!(result.is_none());
}

#[test]
fn test_simulate_portfolio_insufficient_days() {
    let small_pnl = make_pnl(&[("2020-01-01", 1.0), ("2020-01-02", 2.0)]);
    let result = simulate_portfolio(&[&small_pnl]);
    assert!(
        result.is_none(),
        "Should return None with < 100 common days"
    );
}

#[test]
fn test_simulate_portfolio_single_alpha() {
    let pnl = generate_pnl(200, 1.0);
    let result = simulate_portfolio(&[&pnl]);
    assert!(result.is_some());
    let metrics = result.unwrap();
    assert_eq!(metrics.n_alphas, 1);
    assert!(metrics.n_trading_days >= 100);
    assert_eq!(metrics.avg_pairwise_corr, 0.0, "Single alpha has no pairs");
}

#[test]
fn test_simulate_portfolio_identical_alphas() {
    let pnl = generate_pnl(200, 1.0);
    let result = simulate_portfolio(&[&pnl, &pnl]);
    assert!(result.is_some());
    let metrics = result.unwrap();
    assert_eq!(metrics.n_alphas, 2);
    assert!(
        (metrics.avg_pairwise_corr - 1.0).abs() < 1e-10,
        "Identical alphas should have corr=1.0"
    );
}

#[test]
fn test_calculate_portfolio_impact_basic() {
    let pnl1 = generate_pnl(200, 1.0);
    let pnl2 = generate_pnl(200, 2.0);
    let candidate_pnl = generate_pnl(200, 3.0);

    let mut os_pnls = HashMap::new();
    os_pnls.insert("alpha_1".to_string(), pnl1);
    os_pnls.insert("alpha_2".to_string(), pnl2);

    let impact = calculate_portfolio_impact("cand_1", &candidate_pnl, &os_pnls);
    assert!(impact.is_some());
    let imp = impact.unwrap();

    assert_eq!(imp.candidate_id, "cand_1");
    assert_eq!(imp.baseline_alphas, 2);
    assert_eq!(imp.new_alphas, 3);
    assert!(imp.safety_buffer_pct >= 0.0);
}

#[test]
fn test_pearson_boundary_29_vs_30_days() {
    let pnl_29 = generate_pnl(29, 1.0);
    let pnl_30 = generate_pnl(30, 1.0);

    // 29 days must fail-closed per AGENTS.md (< 30 days)
    let (corr_29, days_29) = pearson_correlation(&pnl_29, &pnl_29);
    assert_eq!(days_29, 29);
    assert!(
        corr_29.is_none(),
        "29 days overlap must fail-closed (return None)"
    );

    // 30 days must pass and yield valid correlation
    let (corr_30, days_30) = pearson_correlation(&pnl_30, &pnl_30);
    assert_eq!(days_30, 30);
    assert!(
        corr_30.is_some(),
        "30 days overlap should yield valid correlation"
    );
    assert!((corr_30.unwrap() - 1.0).abs() < 1e-10);
}

#[test]
fn test_pearson_zero_variance_fails_closed() {
    // Normal series with variance
    let pnl_normal = generate_pnl(50, 1.0);

    // Constant series (all 0.0 or constant value, var = 0)
    let pnl_constant: HashMap<String, f64> = (0..50)
        .map(|i| {
            let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
            (date, 5.0)
        })
        .collect();

    let (corr, days) = pearson_correlation(&pnl_normal, &pnl_constant);
    assert_eq!(days, 50);
    assert!(
        corr.is_none(),
        "Zero variance series must fail-closed (return None)"
    );
}

#[test]
fn test_pearson_nan_and_infinity_filtering() {
    let mut pnl_a = generate_pnl(35, 1.0);
    let pnl_b = pnl_a.clone();

    // Inject NaN and Infinity into 10 dates of pnl_a
    for i in 0..10 {
        let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
        if i % 2 == 0 {
            pnl_a.insert(date, f64::NAN);
        } else {
            pnl_a.insert(date, f64::INFINITY);
        }
    }

    // After filtering 10 invalid entries, 25 valid common dates remain (< 30) -> must return None
    let (corr, valid_days) = pearson_correlation(&pnl_a, &pnl_b);
    assert_eq!(valid_days, 25);
    assert!(
        corr.is_none(),
        "When valid non-NaN days < 30, must fail-closed"
    );

    // Now test with 50 days total, 10 NaN -> 40 valid days remain (>= 30) -> valid correlation
    let mut pnl_c = generate_pnl(50, 1.0);
    let pnl_d = pnl_c.clone();
    for i in 0..10 {
        let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
        pnl_c.insert(date, f64::NAN);
    }
    let (corr_valid, days_valid) = pearson_correlation(&pnl_c, &pnl_d);
    assert_eq!(days_valid, 40);
    assert!(corr_valid.is_some(), "40 valid non-NaN days should succeed");
    assert!((corr_valid.unwrap() - 1.0).abs() < 1e-10);
}

#[test]
fn test_simulate_portfolio_nan_handling() {
    let mut pnl1 = generate_pnl(105, 1.0);
    let pnl2 = generate_pnl(105, 2.0);

    // Corrupt 10 days with NaN: 105 - 10 = 95 valid days (< 100 threshold)
    for i in 0..10 {
        let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
        pnl1.insert(date, f64::NAN);
    }

    let result = simulate_portfolio(&[&pnl1, &pnl2]);
    assert!(
        result.is_none(),
        "Should return None when finite common days < 100"
    );

    // Now test with 120 days total: 120 - 10 = 110 valid days (>= 100)
    let mut pnl3 = generate_pnl(120, 1.0);
    let pnl4 = generate_pnl(120, 2.0);
    for i in 0..10 {
        let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
        pnl3.insert(date, f64::NAN);
    }

    let result2 = simulate_portfolio(&[&pnl3, &pnl4]);
    assert!(
        result2.is_some(),
        "Should succeed when finite common days >= 100"
    );
    let metrics = result2.unwrap();
    assert_eq!(metrics.n_trading_days, 110);
    assert!(metrics.merged_sharpe.is_finite());
}
