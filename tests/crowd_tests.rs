use alphafind::crowd::{
    detect_dataset_and_pillar, evaluate_crowd_risk, NeutralizationShield, UniquenessRisk,
};

#[test]
fn test_detect_dataset_and_pillar_options() {
    let expr =
        "ts_backfill(implied_volatility_call_90, 5) / ts_backfill(implied_volatility_call_30, 5)";
    let (ds, pillar) = detect_dataset_and_pillar(expr);
    assert_eq!(ds, "option8");
    assert_eq!(pillar, "OPTIONS_VRP");
}

#[test]
fn test_detect_dataset_and_pillar_idio_risk() {
    let expr = "raw_60 = ts_backfill(unsystematic_risk_last_60_days, 10); raw_60";
    let (ds, pillar) = detect_dataset_and_pillar(expr);
    assert_eq!(ds, "model51");
    assert_eq!(pillar, "IDIOSYNCRATIC_RISK");
}

#[test]
fn test_detect_dataset_and_pillar_short() {
    let expr = "short_val = ts_backfill(vec_avg(shares_sold_short_count_2), 40); short_val";
    let (ds, pillar) = detect_dataset_and_pillar(expr);
    assert_eq!(ds, "short");
    assert_eq!(pillar, "SHORT_INTEREST");
}

#[test]
fn test_detect_dataset_and_pillar_news() {
    let expr = "news_sl = ts_backfill(vec_avg(nws12_afterhsz_sl), 20); news_sl";
    let (ds, pillar) = detect_dataset_and_pillar(expr);
    assert_eq!(ds, "news12");
    assert_eq!(pillar, "NEWS_EVENT");
}

#[test]
fn test_detect_dataset_and_pillar_fundamental() {
    let expr = "group_neutralize(ts_rank(assets / cap, 250), densify(sector))";
    let (ds, pillar) = detect_dataset_and_pillar(expr);
    assert_eq!(ds, "fundamental6");
    assert_eq!(pillar, "FINANCIAL_HEALTH_QUALITY");
}

#[test]
fn test_evaluate_crowd_risk_sanctuary_subneut() {
    let expr = "raw_60 = ts_backfill(unsystematic_risk_last_60_days, 10); raw_60";
    let report = evaluate_crowd_risk(expr, "TOP200", "SUBINDUSTRY", None);
    assert_eq!(report.detected_dataset, "model51");
    assert_eq!(report.detected_pillar, "IDIOSYNCRATIC_RISK");
    assert_eq!(report.neutralization_shield, NeutralizationShield::Strong);
    assert!(
        report.overall_uniqueness_risk == UniquenessRisk::Low
            || report.overall_uniqueness_risk == UniquenessRisk::Moderate
    );
}

#[test]
fn test_evaluate_crowd_risk_exposed_neut() {
    let expr = "fnd6_ebit = operating_income / assets; fnd6_ebit";
    let report = evaluate_crowd_risk(expr, "TOP3000", "MARKET", None);
    assert_eq!(report.detected_dataset, "fundamental6");
    assert_eq!(report.neutralization_shield, NeutralizationShield::Exposed);
    assert!(
        report.overall_uniqueness_risk == UniquenessRisk::High
            || report.overall_uniqueness_risk == UniquenessRisk::Critical
    );
    assert!(!report.warnings.is_empty());
}
