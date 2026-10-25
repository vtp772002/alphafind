use alphafind::taxonomy::{get_curated_candidates, FactorPillar, PlaceholderRegistry};
use std::collections::HashMap;

#[test]
fn test_factor_pillar_from_str_analyst() {
    assert_eq!(
        FactorPillar::from_str("ANALYST"),
        Some(FactorPillar::AnalystConsensus)
    );
    assert_eq!(
        FactorPillar::from_str("analyst"),
        Some(FactorPillar::AnalystConsensus)
    );
    assert_eq!(
        FactorPillar::from_str("Analyst_Consensus"),
        Some(FactorPillar::AnalystConsensus)
    );
    assert_eq!(
        FactorPillar::from_str("CAPITAL_ALLOCATION_ANALYST"),
        Some(FactorPillar::AnalystConsensus)
    );
}

#[test]
fn test_factor_pillar_from_str_options() {
    assert_eq!(
        FactorPillar::from_str("OPTIONS"),
        Some(FactorPillar::OptionsVrp)
    );
    assert_eq!(
        FactorPillar::from_str("OPT"),
        Some(FactorPillar::OptionsVrp)
    );
    assert_eq!(
        FactorPillar::from_str("VRP"),
        Some(FactorPillar::OptionsVrp)
    );
}

#[test]
fn test_factor_pillar_from_str_micro() {
    assert_eq!(
        FactorPillar::from_str("MICRO"),
        Some(FactorPillar::MicrostructureVwap)
    );
    assert_eq!(
        FactorPillar::from_str("VWAP"),
        Some(FactorPillar::MicrostructureVwap)
    );
}

#[test]
fn test_factor_pillar_from_str_quality() {
    assert_eq!(
        FactorPillar::from_str("QUAL"),
        Some(FactorPillar::FinancialHealthQuality)
    );
    assert_eq!(
        FactorPillar::from_str("FUND"),
        Some(FactorPillar::FinancialHealthQuality)
    );
    assert_eq!(
        FactorPillar::from_str("HEALTH"),
        Some(FactorPillar::FinancialHealthQuality)
    );
}

#[test]
fn test_factor_pillar_from_str_risk() {
    assert_eq!(
        FactorPillar::from_str("RISK"),
        Some(FactorPillar::IdiosyncraticRisk)
    );
    assert_eq!(
        FactorPillar::from_str("IDIO"),
        Some(FactorPillar::IdiosyncraticRisk)
    );
}

#[test]
fn test_factor_pillar_from_str_short() {
    assert_eq!(
        FactorPillar::from_str("SHORT"),
        Some(FactorPillar::ShortInterest)
    );
    assert_eq!(
        FactorPillar::from_str("SQUEEZE"),
        Some(FactorPillar::ShortInterest)
    );
}

#[test]
fn test_factor_pillar_from_str_unknown() {
    assert_eq!(FactorPillar::from_str("UNKNOWN"), None);
    assert_eq!(FactorPillar::from_str(""), None);
    assert_eq!(FactorPillar::from_str("VALUATION"), None);
}

#[test]
fn test_factor_pillar_from_str_alternative_data() {
    // SystematicRisk (model51)
    assert_eq!(
        FactorPillar::from_str("SYSTEMATIC"),
        Some(FactorPillar::SystematicRisk)
    );
    assert_eq!(
        FactorPillar::from_str("systematic_risk"),
        Some(FactorPillar::SystematicRisk)
    );
    assert_eq!(
        FactorPillar::from_str("sys_risk"),
        Some(FactorPillar::SystematicRisk)
    );
    assert_eq!(
        FactorPillar::from_str("model51"),
        Some(FactorPillar::SystematicRisk)
    );

    // FundamentalScores (model16)
    assert_eq!(
        FactorPillar::from_str("FUNDAMENTAL_SCORES"),
        Some(FactorPillar::FundamentalScores)
    );
    assert_eq!(
        FactorPillar::from_str("derivative_accel"),
        Some(FactorPillar::FundamentalScores)
    );
    assert_eq!(
        FactorPillar::from_str("model16"),
        Some(FactorPillar::FundamentalScores)
    );
    assert_eq!(
        FactorPillar::from_str("fscore"),
        Some(FactorPillar::FundamentalScores)
    );

    // NewsSentiment (news18)
    assert_eq!(
        FactorPillar::from_str("NEWS_SENTIMENT"),
        Some(FactorPillar::NewsSentiment)
    );
    assert_eq!(
        FactorPillar::from_str("news_sentiment"),
        Some(FactorPillar::NewsSentiment)
    );
    assert_eq!(
        FactorPillar::from_str("ravenpack"),
        Some(FactorPillar::NewsSentiment)
    );
    assert_eq!(
        FactorPillar::from_str("news18"),
        Some(FactorPillar::NewsSentiment)
    );

    // OptionsAnalytics (option9)
    assert_eq!(
        FactorPillar::from_str("OPTIONS_ANALYTICS"),
        Some(FactorPillar::OptionsAnalytics)
    );
    assert_eq!(
        FactorPillar::from_str("options_analytic"),
        Some(FactorPillar::OptionsAnalytics)
    );
    assert_eq!(
        FactorPillar::from_str("option9"),
        Some(FactorPillar::OptionsAnalytics)
    );
    assert_eq!(
        FactorPillar::from_str("breakeven"),
        Some(FactorPillar::OptionsAnalytics)
    );
}

#[test]
fn test_factor_pillar_as_str() {
    assert_eq!(FactorPillar::AnalystConsensus.as_str(), "ANALYST_CONSENSUS");
    assert_eq!(FactorPillar::OptionsVrp.as_str(), "OPTIONS_VRP");
    assert_eq!(
        FactorPillar::MicrostructureVwap.as_str(),
        "MICROSTRUCTURE_VWAP"
    );
    assert_eq!(
        FactorPillar::FinancialHealthQuality.as_str(),
        "FINANCIAL_HEALTH_QUALITY"
    );
    assert_eq!(
        FactorPillar::IdiosyncraticRisk.as_str(),
        "IDIOSYNCRATIC_RISK"
    );
    assert_eq!(FactorPillar::ShortInterest.as_str(), "SHORT_INTEREST");
    assert_eq!(FactorPillar::NewsEvent.as_str(), "NEWS_EVENT");
    assert_eq!(FactorPillar::CrossSanctuary.as_str(), "CROSS_SANCTUARY");
    // Alternative Data Pillars
    assert_eq!(FactorPillar::SystematicRisk.as_str(), "SYSTEMATIC_RISK");
    assert_eq!(
        FactorPillar::FundamentalScores.as_str(),
        "FUNDAMENTAL_SCORES"
    );
    assert_eq!(FactorPillar::NewsSentiment.as_str(), "NEWS_SENTIMENT");
    assert_eq!(FactorPillar::OptionsAnalytics.as_str(), "OPTIONS_ANALYTICS");
}

#[test]
fn test_factor_pillar_all_and_primary_datasets() {
    let all = FactorPillar::all();
    assert_eq!(all.len(), 12, "Should contain exactly 12 pillars");

    assert_eq!(FactorPillar::SystematicRisk.primary_dataset_id(), "model51");
    assert_eq!(
        FactorPillar::FundamentalScores.primary_dataset_id(),
        "model16"
    );
    assert_eq!(FactorPillar::NewsSentiment.primary_dataset_id(), "news18");
    assert_eq!(
        FactorPillar::OptionsAnalytics.primary_dataset_id(),
        "option9"
    );

    assert_eq!(
        FactorPillar::SystematicRisk.canonical_legacy_pillar(),
        FactorPillar::IdiosyncraticRisk
    );
    assert_eq!(
        FactorPillar::FundamentalScores.canonical_legacy_pillar(),
        FactorPillar::FinancialHealthQuality
    );
    assert_eq!(
        FactorPillar::NewsSentiment.canonical_legacy_pillar(),
        FactorPillar::NewsEvent
    );
    assert_eq!(
        FactorPillar::OptionsAnalytics.canonical_legacy_pillar(),
        FactorPillar::OptionsVrp
    );

    assert!(FactorPillar::SystematicRisk.is_alternative_data());
    assert!(FactorPillar::FundamentalScores.is_alternative_data());
    assert!(FactorPillar::NewsSentiment.is_alternative_data());
    assert!(FactorPillar::OptionsAnalytics.is_alternative_data());
    assert!(!FactorPillar::MicrostructureVwap.is_alternative_data());

    for p in all {
        assert!(!p.description().is_empty());
    }
}

#[test]
fn test_curated_candidates_analyst_pillar() {
    let candidates = get_curated_candidates(Some(FactorPillar::AnalystConsensus), None);
    assert!(!candidates.is_empty());
    for c in &candidates {
        assert_eq!(c.pillar, "ANALYST_CONSENSUS");
        // Analyst consensus in private store or presets targets liquid universes (TOP500 or TOP200)
        assert!(
            c.universe == "TOP500" || c.universe == "TOP200",
            "Analyst consensus universe should be TOP500 or TOP200, got: {}",
            c.universe
        );
    }
}

#[test]
fn test_curated_candidates_options_pillar() {
    let candidates = get_curated_candidates(Some(FactorPillar::OptionsVrp), None);
    assert!(!candidates.is_empty());
    for c in &candidates {
        assert_eq!(c.pillar, "OPTIONS_VRP");
    }
}

#[test]
fn test_curated_candidates_alternative_pillars() {
    let alt_pillars = [
        FactorPillar::SystematicRisk,
        FactorPillar::FundamentalScores,
        FactorPillar::NewsSentiment,
        FactorPillar::OptionsAnalytics,
    ];

    for &pillar in &alt_pillars {
        let candidates = get_curated_candidates(Some(pillar), None);
        assert!(
            !candidates.is_empty(),
            "Curated candidates for {:?} must not be empty",
            pillar
        );
        for c in &candidates {
            assert_eq!(c.pillar, pillar.as_str());
            assert!(c.decay > 0);
            assert!(c.truncation > 0.0);
            assert!(!c.expression.is_empty());
            assert!(
                PlaceholderRegistry::has_unresolved_placeholders(&c.expression),
                "Preset candidate expression should use abstract placeholders: {}",
                c.expression
            );
        }
    }
}

#[test]
fn test_curated_candidates_default_batch() {
    let candidates = get_curated_candidates(None, Some("TOP1000"));
    assert!(
        candidates.len() >= 3,
        "Default batch should have multiple candidates across pillars"
    );
    // Should have candidates from different pillars
    let pillars: std::collections::HashSet<&str> =
        candidates.iter().map(|c| c.pillar.as_str()).collect();
    assert!(
        pillars.len() >= 2,
        "Default batch should span multiple pillars"
    );
}

#[test]
fn test_candidate_to_settings() {
    let candidates = get_curated_candidates(Some(FactorPillar::AnalystConsensus), None);
    let c = &candidates[0];
    let settings = c.to_settings();
    assert_eq!(settings.universe, c.universe);
    assert_eq!(settings.decay, c.decay);
    assert_eq!(settings.neutralization, c.neutralization);
    assert_eq!(settings.truncation, c.truncation);
    // Default fields should be set
    assert_eq!(settings.instrument_type, "EQUITY");
    assert_eq!(settings.region, "USA");
    assert_eq!(settings.delay, 1);
    assert_eq!(settings.pasteurization, "ON");
}

#[test]
fn test_factor_pillar_from_str_cross() {
    assert_eq!(
        FactorPillar::from_str("CROSS"),
        Some(FactorPillar::CrossSanctuary)
    );
    assert_eq!(
        FactorPillar::from_str("SANCTUARY"),
        Some(FactorPillar::CrossSanctuary)
    );
    assert_eq!(
        FactorPillar::from_str("HYBRID"),
        Some(FactorPillar::CrossSanctuary)
    );
    assert_eq!(FactorPillar::CrossSanctuary.as_str(), "CROSS_SANCTUARY");
}

#[test]
fn test_curated_candidates_cross_sanctuary() {
    let candidates = get_curated_candidates(Some(FactorPillar::CrossSanctuary), Some("TOP1000"));
    assert!(
        !candidates.is_empty(),
        "Should generate cross-sanctuary candidates"
    );
    for c in &candidates {
        assert_eq!(c.pillar, "CROSS_SANCTUARY");
        assert_eq!(c.universe, "TOP1000");
        assert_eq!(c.neutralization, "SUBINDUSTRY");
    }
}

#[test]
fn test_placeholder_registry_extraction_and_rendering() {
    let expr =
        "sig = rank(<BETA_FIELD>) - rank(<CORR_FIELD>); group_neutralize(sig, densify(<GROUP>))";
    let extracted = PlaceholderRegistry::extract_placeholders(expr);
    assert_eq!(extracted.len(), 3);
    assert!(extracted.contains(&"<BETA_FIELD>".to_string()));
    assert!(extracted.contains(&"<CORR_FIELD>".to_string()));
    assert!(extracted.contains(&"<GROUP>".to_string()));
    assert!(PlaceholderRegistry::has_unresolved_placeholders(expr));

    // Full binding rendering
    let mut bindings = HashMap::new();
    bindings.insert("<BETA_FIELD>", "beta_last_60_days_spy");
    bindings.insert("<CORR_FIELD>", "correlation_last_60_days_spy");
    bindings.insert("<GROUP>", "subindustry");

    let rendered = PlaceholderRegistry::render_template(expr, &bindings);
    assert!(rendered.is_ok());
    let concrete_expr = rendered.unwrap();
    assert_eq!(
        concrete_expr,
        "sig = rank(beta_last_60_days_spy) - rank(correlation_last_60_days_spy); group_neutralize(sig, densify(subindustry))"
    );
    assert!(!PlaceholderRegistry::has_unresolved_placeholders(
        &concrete_expr
    ));

    // Partial binding fails with remaining placeholders
    let mut partial_bindings = HashMap::new();
    partial_bindings.insert("<BETA_FIELD>", "beta_last_60_days_spy");
    let partial_res = PlaceholderRegistry::render_template(expr, &partial_bindings);
    assert!(partial_res.is_err());
    let missing = partial_res.unwrap_err();
    assert_eq!(missing.len(), 2);
    assert!(missing.contains(&"<CORR_FIELD>".to_string()));
    assert!(missing.contains(&"<GROUP>".to_string()));
}

#[test]
fn test_placeholder_registry_syntax_validation() {
    // Valid expression
    let valid = "sig = rank(close); group_neutralize(sig, densify(subindustry))";
    assert!(PlaceholderRegistry::validate_fastexpr_syntax(valid).is_ok());

    // Unresolved placeholder error
    let with_placeholder = "sig = rank(<FIELD>); group_neutralize(sig, densify(subindustry))";
    let err1 = PlaceholderRegistry::validate_fastexpr_syntax(with_placeholder);
    assert!(err1.is_err());
    let errors = err1.unwrap_err();
    assert!(errors[0].contains("unresolved abstract placeholders"));

    // Unmatched parentheses error
    let bad_parens = "sig = rank(close; group_neutralize(sig, densify(subindustry))";
    let err2 = PlaceholderRegistry::validate_fastexpr_syntax(bad_parens);
    assert!(err2.is_err());
    let errors = err2.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("parenthes")));

    // Missing densify wrapper on group_neutralize error
    let no_densify = "sig = rank(close); group_neutralize(sig, subindustry)";
    let err3 = PlaceholderRegistry::validate_fastexpr_syntax(no_densify);
    assert!(err3.is_err());
    let errors = err3.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("densify")));
}
