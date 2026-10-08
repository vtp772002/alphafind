use alphafind::matrix::{
    analyze_formula_hyper_synergy, build_dynamic_matrix, classify_pillar_role,
    detect_all_participating_pillars, detect_fusion_pattern, DatasetRole, FusionPattern,
};
use alphafind::models::DatasetEntry;
use alphafind::taxonomy::FactorPillar;

#[test]
fn test_detect_three_datasets() {
    let expr = "raw_60 = ts_backfill(unsystematic_risk_last_60_days, 10); \
                short = ts_backfill(vec_avg(shares_sold_short_count_2), 40); \
                vwap_dev = vwap / close - 1; \
                signal = raw_60 + short + vwap_dev;";
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
    let expr = "s_idio = unsystematic_risk_last_60_days; \
                s_short = shares_sold_short_count_2; \
                delta_damp = 0.04 * ts_delta(close, 4); \
                trade_when(s_idio > 0, s_idio * (1 + s_short) - delta_damp, -1)";

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
                     delta_damp = 0.04 * ts_delta(close, 4); \
                     trade_when(anchor > 0, anchor - delta_damp, -1)";
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
    let apex_expr = "raw_60 = ts_backfill(unsystematic_risk_last_60_days, 10); \
                     short_cov = ts_backfill(vec_avg(shares_sold_short_count_2), 40); \
                     vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.3); \
                     delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); \
                     signal = 0.42 * raw_60 + 0.20 * short_cov + 0.08 * vwap_dev - delta_damp; \
                     trade_when(abs(signal) > 0.015, signal, -1)";

    let analysis = analyze_formula_hyper_synergy(apex_expr, "TOP200", &[], None);

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
