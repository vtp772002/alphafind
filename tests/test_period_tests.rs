//! Fail-closed behavior of the local Test Period gate (project policy, not a BRAIN rule).

use alphafind::test_period::{
    evaluate_test_period, TestPeriodInput, TestPeriodPolicy, TestPeriodVerdict,
};

fn input(train: Option<f64>, test: Option<f64>, years: Option<f64>) -> TestPeriodInput {
    TestPeriodInput {
        train_sharpe: train,
        test_sharpe: test,
        test_years: years,
    }
}

fn eval(i: TestPeriodInput) -> TestPeriodVerdict {
    evaluate_test_period(&i, &TestPeriodPolicy::default())
}

#[test]
fn passes_when_test_retains_train_sharpe() {
    assert!(eval(input(Some(1.8), Some(1.4), Some(2.0))).is_pass());
}

#[test]
fn fails_on_collapse_or_negative_test_sharpe() {
    assert!(matches!(eval(input(Some(2.5), Some(0.2), Some(2.0))), TestPeriodVerdict::Fail { .. }));
    assert!(matches!(eval(input(Some(2.5), Some(-0.3), Some(2.0))), TestPeriodVerdict::Fail { .. }));
}

#[test]
fn missing_or_non_finite_is_unknown_not_pass() {
    for i in [
        input(None, Some(1.0), Some(2.0)),
        input(Some(1.5), None, Some(2.0)),
        input(Some(1.5), Some(1.0), None),
        input(Some(f64::NAN), Some(1.0), Some(2.0)),
        input(Some(1.5), Some(f64::INFINITY), Some(2.0)),
    ] {
        assert!(matches!(eval(i), TestPeriodVerdict::Unknown { .. }));
    }
}

#[test]
fn short_block_or_weak_train_is_unknown() {
    assert!(matches!(eval(input(Some(1.5), Some(1.5), Some(0.5))), TestPeriodVerdict::Unknown { .. }));
    assert!(matches!(eval(input(Some(0.2), Some(0.2), Some(2.0))), TestPeriodVerdict::Unknown { .. }));
}

#[test]
fn retention_boundary_is_inclusive() {
    assert!(eval(input(Some(2.0), Some(1.0), Some(2.0))).is_pass());
}

use alphafind::test_period::split_daily_pnl;
use std::collections::HashMap;

/// Deterministic weekday series: alternating-ish noise plus a drift per block.
fn series(start_year: i64, end_year: i64, drift: impl Fn(i64) -> f64) -> HashMap<String, f64> {
    let mut m = HashMap::new();
    for y in start_year..=end_year {
        for mo in 1..=12 {
            for d in 1..=28 {
                let noise = (((y * 31 + mo * 7 + d * 13) % 11) as f64 - 5.0) / 5.0;
                m.insert(format!("{y:04}-{mo:02}-{d:02}"), drift(y) + noise);
            }
        }
    }
    m
}

#[test]
fn split_uses_newest_two_years_as_test() {
    let pnl = series(2018, 2023, |y| if y >= 2022 { 0.2 } else { 0.2 });
    let i = split_daily_pnl(&pnl, 2);
    let years = i.test_years.unwrap();
    assert!((1.8..=2.1).contains(&years), "years={years}");
    assert!(i.train_sharpe.is_some() && i.test_sharpe.is_some());
}

#[test]
fn decayed_test_block_fails_gate() {
    let pnl = series(2018, 2023, |y| if y >= 2022 { -0.3 } else { 0.4 });
    let v = alphafind::test_period::evaluate_test_period(
        &split_daily_pnl(&pnl, 2),
        &TestPeriodPolicy::default(),
    );
    assert!(matches!(v, TestPeriodVerdict::Fail { .. }), "{v:?}");
}

#[test]
fn stable_series_passes_gate() {
    let pnl = series(2018, 2023, |_| 0.3);
    let v = alphafind::test_period::evaluate_test_period(
        &split_daily_pnl(&pnl, 2),
        &TestPeriodPolicy::default(),
    );
    assert!(v.is_pass(), "{v:?}");
}

#[test]
fn empty_garbage_or_short_history_is_unknown() {
    let mut bad = HashMap::new();
    bad.insert("not-a-date".to_string(), 1.0);
    bad.insert("2023-01-02".to_string(), f64::NAN);
    for p in [HashMap::new(), bad, series(2022, 2023, |_| 0.3)] {
        let v = alphafind::test_period::evaluate_test_period(
            &split_daily_pnl(&p, 2),
            &TestPeriodPolicy::default(),
        );
        assert!(matches!(v, TestPeriodVerdict::Unknown { .. }), "{v:?}");
    }
}

#[test]
fn custom_policy_thresholds_honored() {
    let custom = TestPeriodPolicy {
        min_train_sharpe: 1.0,
        min_retention: 0.75,
        min_test_years: 1.0,
    };
    // Retention 0.70 fails when threshold is 0.75
    let inp = input(Some(2.0), Some(1.4), Some(1.5));
    let verdict = evaluate_test_period(&inp, &custom);
    assert!(matches!(verdict, TestPeriodVerdict::Fail { .. }));

    // Retention 0.80 passes
    let inp_pass = input(Some(2.0), Some(1.6), Some(1.5));
    let verdict_pass = evaluate_test_period(&inp_pass, &custom);
    assert!(verdict_pass.is_pass());

    // Train sharpe 0.8 fails when minimum is 1.0
    let inp_low_train = input(Some(0.8), Some(0.8), Some(1.5));
    let verdict_low = evaluate_test_period(&inp_low_train, &custom);
    assert!(matches!(verdict_low, TestPeriodVerdict::Unknown { .. }));
}
