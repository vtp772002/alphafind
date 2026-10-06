use alphafind::taxonomy::{get_curated_candidates, FactorPillar};

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
}

#[test]
fn test_curated_candidates_analyst_pillar() {
    let candidates = get_curated_candidates(Some(FactorPillar::AnalystConsensus), None);
    assert!(!candidates.is_empty());
    for c in &candidates {
        assert_eq!(c.pillar, "ANALYST_CONSENSUS");
        assert_eq!(c.universe, "TOP500"); // Analyst consensus uses TOP500
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
