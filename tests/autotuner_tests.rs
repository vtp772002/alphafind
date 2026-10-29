use alphafind::autotuner::{
    evaluate_robustness, replace_last_signed_power_exponent, switch_neutralization_in_expr,
};
use alphafind::models::{RobustnessVerdict, SensitivityPoint};

#[test]
fn test_replace_last_signed_power_exponent() {
    let expr =
        "signal = rank(close); group_neutralize(signed_power(signal, 1.5), densify(subindustry))";
    let updated = replace_last_signed_power_exponent(expr, 4.4);
    assert!(updated.is_some());
    let res = updated.unwrap();
    assert!(res.contains("signed_power(signal, 4.4)"));
}

#[test]
fn test_replace_last_signed_power_exponent_nested() {
    let expr = "sig1 = signed_power(rank(close) - 0.5, 1.2); group_neutralize(signed_power(sig1, 2.0), densify(market))";
    let updated = replace_last_signed_power_exponent(expr, 3.8);
    assert!(updated.is_some());
    let res = updated.unwrap();
    assert!(res.contains("signed_power(sig1, 3.8)"));
    assert!(res.contains("signed_power(rank(close) - 0.5, 1.2)"));
}

#[test]
fn test_switch_neutralization_subindustry() {
    let expr = "group_neutralize(signal, densify(market))";
    let switched = switch_neutralization_in_expr(expr, "SUBINDUSTRY");
    assert_eq!(switched, "group_neutralize(signal, densify(subindustry))");
}

#[test]
fn test_switch_neutralization_market() {
    let expr = "group_neutralize(signal, densify(subindustry))";
    let switched = switch_neutralization_in_expr(expr, "MARKET");
    assert_eq!(switched, "group_neutralize(signal, densify(market))");
}

#[test]
fn test_switch_neutralization_sector() {
    let expr = "group_neutralize(signal, densify(subindustry))";
    let switched = switch_neutralization_in_expr(expr, "SECTOR");
    assert_eq!(switched, "group_neutralize(signal, densify(sector))");
}

#[test]
fn test_evaluate_robustness_robust_plateau() {
    // Smooth Sharpe distribution across neighboring decay parameters
    let points = vec![
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 6.0,
            alpha_id: Some("A1".to_string()),
            sharpe: 1.28,
            fitness: 1.10,
            returns: 0.10,
            turnover: 0.22,
            margin: 0.0010,
        },
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 8.0,
            alpha_id: Some("A2".to_string()),
            sharpe: 1.35,
            fitness: 1.20,
            returns: 0.11,
            turnover: 0.18,
            margin: 0.0012,
        },
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 10.0,
            alpha_id: Some("A3".to_string()),
            sharpe: 1.38,
            fitness: 1.25,
            returns: 0.12,
            turnover: 0.16,
            margin: 0.0014,
        },
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 12.0,
            alpha_id: Some("A4".to_string()),
            sharpe: 1.36,
            fitness: 1.22,
            returns: 0.11,
            turnover: 0.14,
            margin: 0.0013,
        },
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 14.0,
            alpha_id: Some("A5".to_string()),
            sharpe: 1.30,
            fitness: 1.15,
            returns: 0.10,
            turnover: 0.13,
            margin: 0.0011,
        },
    ];

    let report = evaluate_robustness(points, "decay", 1.25, 25.0);
    assert_eq!(report.verdict, RobustnessVerdict::RobustPlateau);
    assert_eq!(report.optimal_value, 10.0);
    assert_eq!(report.optimal_sharpe, 1.38);
    // (1.38 - 1.35) / 1.38 = 2.17%, (1.38 - 1.36) / 1.38 = 1.45%
    assert!(report.max_neighbor_drop_pct < 5.0);
}

#[test]
fn test_evaluate_robustness_overfit_needle_peak() {
    // Sharp needle spike at exponent P = 4.4, collapsing at neighbors P = 4.0 and P = 4.8
    let points = vec![
        SensitivityPoint {
            param_name: "exponent".to_string(),
            param_value: 3.6,
            alpha_id: Some("A1".to_string()),
            sharpe: 0.65,
            fitness: 0.40,
            returns: 0.05,
            turnover: 0.20,
            margin: 0.0005,
        },
        SensitivityPoint {
            param_name: "exponent".to_string(),
            param_value: 4.0,
            alpha_id: Some("A2".to_string()),
            sharpe: 0.78,
            fitness: 0.50,
            returns: 0.06,
            turnover: 0.19,
            margin: 0.0006,
        },
        SensitivityPoint {
            param_name: "exponent".to_string(),
            param_value: 4.4,
            alpha_id: Some("A3".to_string()),
            sharpe: 1.45,
            fitness: 1.30,
            returns: 0.14,
            turnover: 0.15,
            margin: 0.0015,
        },
        SensitivityPoint {
            param_name: "exponent".to_string(),
            param_value: 4.8,
            alpha_id: Some("A4".to_string()),
            sharpe: 0.70,
            fitness: 0.45,
            returns: 0.05,
            turnover: 0.14,
            margin: 0.0005,
        },
        SensitivityPoint {
            param_name: "exponent".to_string(),
            param_value: 5.2,
            alpha_id: Some("A5".to_string()),
            sharpe: 0.60,
            fitness: 0.35,
            returns: 0.04,
            turnover: 0.13,
            margin: 0.0004,
        },
    ];

    let report = evaluate_robustness(points, "exponent", 1.25, 25.0);
    assert_eq!(report.verdict, RobustnessVerdict::OverfitNeedlePeak);
    assert_eq!(report.optimal_value, 4.4);
    assert_eq!(report.optimal_sharpe, 1.45);
    // (1.45 - 0.70) / 1.45 = 51.72% drop
    assert!(report.max_neighbor_drop_pct > 40.0);
    assert!(report.summary.contains("OVERFIT"));
}

#[test]
fn test_evaluate_robustness_insufficient_points() {
    let points = vec![
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 6.0,
            alpha_id: Some("A1".to_string()),
            sharpe: 1.30,
            fitness: 1.10,
            returns: 0.10,
            turnover: 0.20,
            margin: 0.0010,
        },
        SensitivityPoint {
            param_name: "decay".to_string(),
            param_value: 8.0,
            alpha_id: Some("A2".to_string()),
            sharpe: 1.35,
            fitness: 1.15,
            returns: 0.11,
            turnover: 0.18,
            margin: 0.0011,
        },
    ];

    let report = evaluate_robustness(points, "decay", 1.25, 25.0);
    assert_eq!(report.verdict, RobustnessVerdict::Inconclusive);
}
