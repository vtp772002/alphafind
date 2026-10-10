use alphafind::correlation::pearson_correlation;
use alphafind::models::PortfolioAlpha;
use alphafind::shadow::{
    audit_portfolio_crowd_exposure, calibrate_shadow_portfolio, project_uniqueness_impact,
    UniquenessPhaseVerdict,
};
use std::collections::HashMap;

fn create_mock_data() -> (Vec<PortfolioAlpha>, HashMap<String, HashMap<String, f64>>) {
    let mut alphas = Vec::new();
    let mut pnls = HashMap::new();

    let n_days = 120;
    let n_alphas = 10;

    for i in 0..n_alphas {
        let aid = format!("ALPHA_{:02}", i);
        let cat = match i % 4 {
            0 => "FUNDAMENTAL",
            1 => "ANALYST",
            2 => "OPTIONS",
            _ => "MOMENTUM",
        };

        alphas.push(PortfolioAlpha {
            id: aid.clone(),
            name: Some(format!("Mock Alpha {}", i)),
            status: Some("ACTIVE".to_string()),
            stage: Some("OS".to_string()),
            date_submitted: None,
            universe: Some("TOP1000".to_string()),
            decay: Some(5),
            category: Some(cat.to_string()),
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

        let mut pnl_map = HashMap::new();
        for d in 0..n_days {
            let date_str = format!("2026-01-{:03}", d);
            // Simulated return: common factor plus idiosyncratic shock
            let val = (d as f64 * 0.1).sin() * 50.0 + ((i * 17 + d) as f64).cos() * 20.0;
            pnl_map.insert(date_str, val);
        }
        pnls.insert(aid, pnl_map);
    }

    (alphas, pnls)
}

#[test]
fn test_calibrate_shadow_portfolio_exact_correlation() {
    let (alphas, pnls) = create_mock_data();
    let shadow =
        calibrate_shadow_portfolio(&alphas, &pnls, Some(0.54)).expect("calibration failed");

    assert_eq!(shadow.anchor_uniqueness, 0.54);
    assert_eq!(shadow.n_trading_days, 120);

    // Compute user portfolio merged PnL
    let mut user_merged = HashMap::new();
    let dates: Vec<String> = shadow.shadow_pnl.keys().cloned().collect();
    for d in &dates {
        let sum: f64 = pnls.values().map(|p| p[d]).sum();
        user_merged.insert(d.clone(), sum / (pnls.len() as f64));
    }

    let (corr, days) = pearson_correlation(&user_merged, &shadow.shadow_pnl);
    assert_eq!(days, 120);
    assert!(
        (corr - 0.54).abs() < 1e-4,
        "Expected calibrated correlation ~0.54, got {:.6}",
        corr
    );
}

#[test]
fn test_project_uniqueness_impact_negative_hedge() {
    let (alphas, pnls) = create_mock_data();
    let shadow =
        calibrate_shadow_portfolio(&alphas, &pnls, Some(0.54)).expect("calibration failed");

    // Create an adversarial candidate: inverse of shadow PnL
    let mut cand_pnl = HashMap::new();
    for (d, val) in &shadow.shadow_pnl {
        cand_pnl.insert(d.clone(), -val * 2.0);
    }

    let report = project_uniqueness_impact("ADVERSARIAL_01", &cand_pnl, &alphas, &pnls, &shadow)
        .expect("impact projection failed");

    eprintln!("REPORT: {:?}", report);
    assert!(
        report.delta_uniqueness < 0.0,
        "Delta uniqueness must be negative"
    );
    assert!(
        report.verdict == UniquenessPhaseVerdict::AdversarialNegativeHedge
            || report.verdict == UniquenessPhaseVerdict::OrthogonalSanctuary,
        "Verdict must be hedge or sanctuary"
    );
}

#[test]
fn test_project_uniqueness_impact_crowd_clone() {
    let (alphas, pnls) = create_mock_data();
    let shadow =
        calibrate_shadow_portfolio(&alphas, &pnls, Some(0.54)).expect("calibration failed");

    // Create a clone candidate: identical to shadow PnL
    let mut clone_pnl = HashMap::new();
    for (d, val) in &shadow.shadow_pnl {
        clone_pnl.insert(d.clone(), val * 3.0);
    }

    let report = project_uniqueness_impact("CLONE_01", &clone_pnl, &alphas, &pnls, &shadow)
        .expect("impact projection failed");

    assert!(
        report.delta_uniqueness > 0.0,
        "Delta uniqueness must be positive"
    );
    assert_eq!(report.verdict, UniquenessPhaseVerdict::DangerousCrowdClone);
}

#[test]
fn test_audit_portfolio_crowd_exposure() {
    let (alphas, pnls) = create_mock_data();
    let shadow =
        calibrate_shadow_portfolio(&alphas, &pnls, Some(0.54)).expect("calibration failed");

    let audit = audit_portfolio_crowd_exposure(&alphas, &pnls, &shadow);
    assert_eq!(audit.active_alphas_count, 10);
    assert_eq!(audit.baseline_crowd_corr, 0.54);
    assert!(!audit.highest_crowd_alphas.is_empty());
    assert!(!audit.lowest_crowd_alphas.is_empty());
    assert!(audit.highest_crowd_alphas[0].2 >= audit.lowest_crowd_alphas[0].2);
}
