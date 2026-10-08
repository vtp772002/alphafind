use alphafind::matrix::{
    analyze_formula_hyper_synergy, build_dynamic_matrix, classify_pillar_role,
    detect_all_participating_pillars, detect_fusion_pattern, DatasetRole, FusionPattern,
};
use alphafind::models::DatasetEntry;
use alphafind::taxonomy::FactorPillar;

#[test]
fn test_detect_three_datasets() {
    let expr = "f_risk = ts_backfill(unsystematic_risk_last_60_days, 10); \
                f_short = ts_backfill(shares_sold_short_count_2, 40); \
                f_vwap = vwap / close - 1; \
                signal = f_risk + f_short + f_vwap;";
    let pillars = detect_all_participating_pillars(expr);
    assert_eq!(pillars.len(), 3);

    let pillar_names: Vec<&str> = pillars.iter().map(|(_, p)| p.as_str()).collect();
    assert!(pillar_names.contains(&"IDIOSYNCRATIC_RISK"));
    assert!(pillar_names.contains(&"SHORT_INTEREST"));
    assert!(pillar_names.contains(&"MICROSTRUCTURE_VWAP"));
}

#[test]
fn test_detect_four_datasets() {
    let expr = "raw_60 = unsystematic_risk_last_60_days; \
                short = shares_sold_short_count_2; \
                news = mean_composite_sentiment_score; \
                reversal = ts_delta(close, 4); \
                signal = raw_60 + short + news + reversal;";
    let pillars = detect_all_participating_pillars(expr);
    assert_eq!(pillars.len(), 4);
}

#[test]
fn test_classify_economic_roles() {
    let expr = "anchor = unsystematic_risk_last_60_days; \
                catalyst = shares_sold_short_count_2; \
                friction = ts_delta(close, 4); \
                trade_when(anchor > 0, anchor * (1 + catalyst) - friction, -1)";

    let role_idio = classify_pillar_role(FactorPillar::IdiosyncraticRisk, expr, 3);
    let role_short = classify_pillar_role(FactorPillar::ShortInterest, expr, 3);
    let role_micro = classify_pillar_role(FactorPillar::MicrostructureVwap, expr, 3);

    assert_eq!(role_idio, DatasetRole::EconomicAnchor);
    assert_eq!(role_short, DatasetRole::RiskCatalyst);
    assert_eq!(role_micro, DatasetRole::FrictionHedge);
}

#[test]
fn test_detect_fusion_patterns() {
    let naive_expr = "rank(operating_income / assets) + rank(volume / adv20)";
    assert_eq!(
        detect_fusion_pattern(naive_expr, 2),
        FusionPattern::NaiveLinearAddition
    );

    let hier_expr = "anchor = unsystematic_risk_last_60_days; \
                     cat = shares_sold_short_count_2; \
                     hedge = ts_delta(close, 4); \
                     trade_when(anchor > 0, anchor - hedge, -1)";
    assert_eq!(
        detect_fusion_pattern(hier_expr, 3),
        FusionPattern::HierarchicalConditioning
    );

    let single_expr = "rank(operating_income / assets)";
    assert_eq!(
        detect_fusion_pattern(single_expr, 1),
        FusionPattern::SingleDataset
    );
}

#[test]
fn test_build_dynamic_matrix_structure() {
    let datasets: Vec<DatasetEntry> = Vec::new();
    let report = build_dynamic_matrix(&datasets, None, "TOP200");

    assert_eq!(report.universe, "TOP200");
    assert_eq!(report.pillars.len(), 7);
    // 7 choose 2 = 21 pairwise interactions
    assert_eq!(report.interactions.len(), 21);

    for inter in &report.interactions {
        assert!(inter.hybrid_uniqueness_index >= 0.0 && inter.hybrid_uniqueness_index <= 1.0);
        assert!(!inter.risk_tier.is_empty());
    }
}

#[test]
fn test_analyze_formula_hyper_synergy_apex() {
    let dummy_expr = "risk = ts_backfill(unsystematic_risk_last_60_days, 10); \
                      borrow = ts_backfill(shares_sold_short_count_2, 40); \
                      vwap_dev = rank(vwap / close - 1); \
                      friction = ts_delta(close, 4); \
                      signal = 0.5 * risk + 0.3 * borrow + 0.2 * vwap_dev - friction; \
                      trade_when(abs(signal) > 0.01, signal, -1)";

    let analysis = analyze_formula_hyper_synergy(dummy_expr, "TOP200", &[], None);

    assert_eq!(analysis.detected_k, 3);
    assert_eq!(
        analysis.fusion_pattern,
        FusionPattern::HierarchicalConditioning
    );
    assert!(analysis.composite_uniqueness_index >= 0.80);
    assert!(
        analysis.uniqueness_verdict.contains("ORTHOGONAL")
            || analysis.uniqueness_verdict.contains("INSTITUTIONAL")
    );
}
