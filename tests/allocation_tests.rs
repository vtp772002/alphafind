use alphafind::allocation::{
    compute_hrp_weights, evaluate_candidate_admission, evaluate_candidate_swap,
    evaluate_turnover_entropy, identify_retirement_candidates, AdmissionVerdict, TurnoverBucket,
};
use alphafind::models::PortfolioAlpha;
use std::collections::HashMap;

fn create_mock_alpha(id: &str, turnover: f64) -> PortfolioAlpha {
    PortfolioAlpha {
        id: id.to_string(),
        name: Some(format!("Alpha_{}", id)),
        status: Some("ACTIVE".to_string()),
        stage: Some("OS".to_string()),
        date_submitted: Some("2026-01-01".to_string()),
        universe: Some("TOP1000".to_string()),
        decay: Some(10),
        category: Some("PRICE_VOLUME".to_string()),
        color: Some("BLUE".to_string()),
        sharpe: Some(2.0),
        fitness: Some(1.5),
        returns: Some(0.12),
        turnover: Some(turnover),
        direct_url: None,
        code: None,
        tags: Some(vec![]),
        settings: None,
    }
}

#[test]
fn test_turnover_bucket_ranges() {
    assert_eq!(TurnoverBucket::from_turnover(0.05), TurnoverBucket::UltraSlow);
    assert_eq!(TurnoverBucket::from_turnover(0.15), TurnoverBucket::Core);
    assert_eq!(TurnoverBucket::from_turnover(0.35), TurnoverBucket::Fast);
    assert_eq!(TurnoverBucket::from_turnover(0.60), TurnoverBucket::VeryFast);
}

#[test]
fn test_turnover_entropy_spectrum() {
    let alphas = vec![
        create_mock_alpha("A1", 0.05),
        create_mock_alpha("A2", 0.15),
        create_mock_alpha("A3", 0.35),
        create_mock_alpha("A4", 0.55),
    ];
    let rep = evaluate_turnover_entropy(&alphas);
    assert_eq!(rep.total_alphas, 4);
    assert_eq!(rep.ultra_slow_count, 1);
    assert_eq!(rep.core_count, 1);
    assert_eq!(rep.fast_count, 1);
    assert_eq!(rep.very_fast_count, 1);
    assert!((rep.diversity_score - 100.0).abs() < 1e-4);
}

#[test]
fn test_compute_hrp_weights_synthetic() {
    let a1 = create_mock_alpha("A1", 0.15);
    let a2 = create_mock_alpha("A2", 0.20);
    let alphas = vec![a1, a2];

    let mut pnl1 = HashMap::new();
    let mut pnl2 = HashMap::new();

    // 120 trading days
    for day in 0..120 {
        let d = format!("2026-01-{:03}", day);
        let val1 = (day as f64 * 0.1).sin() * 100.0;
        let val2 = (day as f64 * 0.1).cos() * 50.0;
        pnl1.insert(d.clone(), val1);
        pnl2.insert(d, val2);
    }

    let mut cache = HashMap::new();
    cache.insert("A1".to_string(), pnl1);
    cache.insert("A2".to_string(), pnl2);

    let weights = compute_hrp_weights(&alphas, &cache);
    assert_eq!(weights.len(), 2);
    let total_w: f64 = weights.iter().map(|w| w.hrp_weight).sum();
    assert!((total_w - 1.0).abs() < 1e-4);
}

#[test]
fn test_candidate_admission_decision_go() {
    let a1 = create_mock_alpha("A1", 0.15);
    let alphas = vec![a1];

    let mut pnl1 = HashMap::new();
    let mut cand_pnl = HashMap::new();

    for day in 0..120 {
        let d = format!("2026-01-{:03}", day);
        pnl1.insert(d.clone(), 10.0 + (day as f64 % 3.0));
        // Cand has positive, low correlation returns
        cand_pnl.insert(d, 20.0 - (day as f64 % 2.0));
    }

    let mut cache = HashMap::new();
    cache.insert("A1".to_string(), pnl1);

    let rep = evaluate_candidate_admission("CAND1", 0.35, &cand_pnl, &alphas, &cache);
    assert_eq!(rep.candidate_id, "CAND1");
    assert!(rep.safety_buffer_pct > 10.0);
    assert!(rep.quota_deficit_addressed);
    assert!(rep.verdict == AdmissionVerdict::StrongGo || rep.verdict == AdmissionVerdict::ConditionalGo);
}

#[test]
fn test_candidate_admission_decision_collision_no_go() {
    let a1 = create_mock_alpha("A1", 0.15);
    let alphas = vec![a1];

    let mut pnl1 = HashMap::new();
    let mut cand_pnl = HashMap::new();

    // Perfectly identical PnL -> correlation = 1.0 -> Collision
    for day in 0..120 {
        let d = format!("2026-01-{:03}", day);
        let val = day as f64 * 5.0;
        pnl1.insert(d.clone(), val);
        cand_pnl.insert(d, val);
    }

    let mut cache = HashMap::new();
    cache.insert("A1".to_string(), pnl1);

    let rep = evaluate_candidate_admission("CAND1", 0.35, &cand_pnl, &alphas, &cache);
    assert_eq!(rep.verdict, AdmissionVerdict::NoGo);
}

#[test]
fn test_identify_retirement_candidates() {
    let a1 = create_mock_alpha("A1", 0.15);
    let a2 = create_mock_alpha("A2", 0.15);
    let alphas = vec![a1, a2];

    let hrp_weights = vec![
        alphafind::allocation::HrpAlphaWeight {
            alpha_id: "A1".to_string(),
            name: "Alpha_A1".to_string(),
            universe: "TOP1000".to_string(),
            hrp_weight: 0.85,
            equal_weight: 0.50,
            health_ratio: 1.70,
            verdict: "PILLAR".to_string(),
        },
        alphafind::allocation::HrpAlphaWeight {
            alpha_id: "A2".to_string(),
            name: "Alpha_A2".to_string(),
            universe: "TOP1000".to_string(),
            hrp_weight: 0.15,
            equal_weight: 0.50,
            health_ratio: 0.30, // Low HR
            verdict: "DRAG".to_string(),
        },
    ];

    let retirements = identify_retirement_candidates(&alphas, &hrp_weights);
    assert_eq!(retirements.len(), 1);
    assert_eq!(retirements[0].alpha_id, "A2");
}

#[test]
fn test_evaluate_candidate_swap() {
    let a1 = create_mock_alpha("A1", 0.15);
    let a2 = create_mock_alpha("A2", 0.15);
    let alphas = vec![a1, a2];

    let mut pnl1 = HashMap::new();
    let mut pnl2 = HashMap::new();
    let mut cand_pnl = HashMap::new();

    for day in 0..120 {
        let d = format!("2026-01-{:03}", day);
        pnl1.insert(d.clone(), 50.0 + (day as f64 % 5.0)); // Strong alpha
        pnl2.insert(d.clone(), 1.0 - (day as f64 % 10.0)); // Weak/declining alpha
        cand_pnl.insert(d, 40.0 + (day as f64 % 3.0));     // Good candidate
    }

    let mut cache = HashMap::new();
    cache.insert("A1".to_string(), pnl1);
    cache.insert("A2".to_string(), pnl2);

    let swap_rep = evaluate_candidate_swap("CAND1", &cand_pnl, &alphas, &cache, "A2").unwrap();
    assert_eq!(swap_rep.replaced_alpha_id, "A2");
    assert_eq!(swap_rep.candidate_id, "CAND1");
    assert!(swap_rep.is_net_improvement);
    assert!(swap_rep.sharpe_delta > 0.0);
}
