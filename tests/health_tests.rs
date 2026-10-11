use alphafind::health::{evaluate_portfolio_health, AlphaDecayStatus, AlphaRoleVerdict};
use alphafind::models::PortfolioAlpha;
use std::collections::HashMap;

fn create_mock_alphas_and_pnls(
    n_alphas: usize,
    n_days: usize,
) -> (Vec<PortfolioAlpha>, HashMap<String, HashMap<String, f64>>) {
    let mut alphas = Vec::new();
    let mut pnls = HashMap::new();

    for i in 0..n_alphas {
        let aid = format!("MOCK_ALPHA_{:02}", i);
        let cat = match i % 3 {
            0 => "PRICE_VOLUME",
            1 => "ANALYST",
            _ => "OPTIONS",
        };

        alphas.push(PortfolioAlpha {
            id: aid.clone(),
            name: Some(format!("Mock Strategy {}", i)),
            status: Some("ACTIVE".to_string()),
            stage: Some("OS".to_string()),
            date_submitted: None,
            universe: Some("TOP1000".to_string()),
            decay: Some(5),
            category: Some(cat.to_string()),
            color: None,
            sharpe: Some(1.6),
            fitness: Some(1.8),
            returns: Some(0.18),
            turnover: Some(0.15),
            direct_url: None,
            code: None,
            tags: None,
            settings: None,
        });

        let mut pnl_map = HashMap::new();
        for d in 0..n_days {
            let date_str = format!("2026-01-{:03}", d);
            // Steady positive trend + noise
            let base = 50.0 + (i as f64 * 5.0);
            let noise = ((d as f64 * 0.2 + i as f64).sin()) * 20.0;
            pnl_map.insert(date_str, base + noise);
        }
        pnls.insert(aid, pnl_map);
    }

    (alphas, pnls)
}

#[test]
fn test_decay_status_and_role_badges() {
    assert!(!AlphaDecayStatus::Accelerating.badge().is_empty());
    assert!(!AlphaDecayStatus::Robust.badge().is_empty());
    assert!(!AlphaDecayStatus::ModerateDecay.badge().is_empty());
    assert!(!AlphaDecayStatus::SevereDecay.badge().is_empty());

    assert!(!AlphaRoleVerdict::PillarDriver.badge().is_empty());
    assert!(!AlphaRoleVerdict::PositiveContributor.badge().is_empty());
    assert!(!AlphaRoleVerdict::NeutralDiversifier.badge().is_empty());
    assert!(!AlphaRoleVerdict::DilutiveDrag.badge().is_empty());
}

#[test]
fn test_evaluate_portfolio_health_empty_handling() {
    let (alphas, pnls) = create_mock_alphas_and_pnls(0, 0);
    let result = evaluate_portfolio_health(&alphas, &pnls, None);
    assert!(result.is_none());
}

#[test]
fn test_evaluate_portfolio_health_insufficient_days() {
    let (alphas, pnls) = create_mock_alphas_and_pnls(3, 30); // less than 60 days
    let result = evaluate_portfolio_health(&alphas, &pnls, None);
    assert!(result.is_none());
}

#[test]
fn test_evaluate_portfolio_health_full_synthetic() {
    let (alphas, pnls) = create_mock_alphas_and_pnls(6, 120);
    let report_opt = evaluate_portfolio_health(&alphas, &pnls, None);

    assert!(report_opt.is_some());
    let report = report_opt.unwrap();

    assert_eq!(report.total_alphas, 6);
    assert_eq!(report.common_trading_days, 120);
    assert!(report.portfolio_sharpe > 0.0);
    assert!(report.health_score >= 0.0 && report.health_score <= 100.0);

    let total_statuses = report.accelerating_count
        + report.robust_count
        + report.moderate_decay_count
        + report.severe_decay_count;
    assert_eq!(total_statuses, 6);
    assert_eq!(report.all_alpha_diagnostics.len(), 6);
    assert!(!report.top_pillar_drivers.is_empty());
}

#[test]
fn test_decay_detection_contrasting_regimes() {
    let mut alphas = Vec::new();
    let mut pnls = HashMap::new();
    let n_days = 120;

    // Alpha 0: Deteriorating (first half profitable, second half loss-making)
    let aid0 = "MOCK_DECAYING".to_string();
    alphas.push(PortfolioAlpha {
        id: aid0.clone(),
        name: Some("Decaying Strategy".to_string()),
        status: Some("ACTIVE".to_string()),
        stage: Some("OS".to_string()),
        date_submitted: None,
        universe: Some("TOP500".to_string()),
        decay: Some(5),
        category: Some("FUNDAMENTAL".to_string()),
        color: None,
        sharpe: Some(1.5),
        fitness: Some(1.5),
        returns: Some(0.15),
        turnover: Some(0.20),
        direct_url: None,
        code: None,
        tags: None,
        settings: None,
    });
    let mut pnl0 = HashMap::new();
    for d in 0..n_days {
        let date_str = format!("2026-01-{:03}", d);
        let noise = (d as f64 * 0.2).sin() * 5.0;
        if d < 60 {
            pnl0.insert(date_str, 100.0 + noise);
        } else {
            pnl0.insert(date_str, -80.0 + noise); // sharp trailing collapse
        }
    }
    pnls.insert(aid0.clone(), pnl0);

    // Alpha 1: Accelerating (first half mediocre, second half booming)
    let aid1 = "MOCK_ACCELERATING".to_string();
    alphas.push(PortfolioAlpha {
        id: aid1.clone(),
        name: Some("Accelerating Strategy".to_string()),
        status: Some("ACTIVE".to_string()),
        stage: Some("OS".to_string()),
        date_submitted: None,
        universe: Some("TOP500".to_string()),
        decay: Some(5),
        category: Some("PRICE_VOLUME".to_string()),
        color: None,
        sharpe: Some(1.5),
        fitness: Some(1.5),
        returns: Some(0.15),
        turnover: Some(0.20),
        direct_url: None,
        code: None,
        tags: None,
        settings: None,
    });
    let mut pnl1 = HashMap::new();
    for d in 0..n_days {
        let date_str = format!("2026-01-{:03}", d);
        let noise = (d as f64 * 0.2).sin() * 5.0;
        if d < 60 {
            pnl1.insert(date_str, 20.0 + noise);
        } else {
            pnl1.insert(date_str, 150.0 + noise); // sharp trailing surge
        }
    }
    pnls.insert(aid1.clone(), pnl1);

    // Alpha 2: Stable anchor to ensure common dates & portfolio baseline
    let aid2 = "MOCK_STABLE".to_string();
    alphas.push(PortfolioAlpha {
        id: aid2.clone(),
        name: Some("Stable Strategy".to_string()),
        status: Some("ACTIVE".to_string()),
        stage: Some("OS".to_string()),
        date_submitted: None,
        universe: Some("TOP500".to_string()),
        decay: Some(5),
        category: Some("ANALYST".to_string()),
        color: None,
        sharpe: Some(1.5),
        fitness: Some(1.5),
        returns: Some(0.15),
        turnover: Some(0.20),
        direct_url: None,
        code: None,
        tags: None,
        settings: None,
    });
    let mut pnl2 = HashMap::new();
    for d in 0..n_days {
        let date_str = format!("2026-01-{:03}", d);
        pnl2.insert(date_str, 50.0 + ((d as f64 * 0.1).sin()) * 10.0);
    }
    pnls.insert(aid2.clone(), pnl2);

    let report = evaluate_portfolio_health(&alphas, &pnls, None).unwrap();

    let diag_decaying = report
        .all_alpha_diagnostics
        .iter()
        .find(|a| a.alpha_id == aid0)
        .unwrap();
    assert_eq!(diag_decaying.decay_status, AlphaDecayStatus::SevereDecay);

    let diag_accelerating = report
        .all_alpha_diagnostics
        .iter()
        .find(|a| a.alpha_id == aid1)
        .unwrap();
    assert!(
        diag_accelerating.decay_status == AlphaDecayStatus::Accelerating
            || diag_accelerating.decay_status == AlphaDecayStatus::Robust
    );
}
