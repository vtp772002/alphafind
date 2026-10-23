//! Submission-gate, check-report, and cache-freshness reporting tests.
//!
//! These cover the local policy surface around an external submission: how checks
//! are displayed, when `--skip-checks` may run, and how cache age is classified.

use alphafind::client::{evaluate_submission_checks, CORE_SUBMISSION_CHECKS};
use alphafind::commands::cache_age_label;
use alphafind::commands::submission::{
    format_check_table_row, skip_checks_permitted, CheckDisplay,
};
use alphafind::models::{CheckItem, InSampleStats};
use std::time::Duration;

const HOUR: u64 = 3600;

fn check(name: &str, result: &str) -> CheckItem {
    CheckItem {
        name: name.to_string(),
        result: result.to_string(),
        ..CheckItem::default()
    }
}

fn stats_with(checks: Vec<CheckItem>) -> InSampleStats {
    InSampleStats {
        checks,
        ..InSampleStats::default()
    }
}

fn all_core_passing() -> Vec<CheckItem> {
    CORE_SUBMISSION_CHECKS
        .iter()
        .map(|name| check(name, "PASS"))
        .collect()
}

#[test]
fn test_check_display_classifies_every_known_status() {
    assert_eq!(CheckDisplay::from_result("PASS"), CheckDisplay::Pass);
    assert_eq!(CheckDisplay::from_result(" pass "), CheckDisplay::Pass);
    assert_eq!(CheckDisplay::from_result("PENDING"), CheckDisplay::Pending);
    assert_eq!(CheckDisplay::from_result("FAIL"), CheckDisplay::Fail);
    assert_eq!(CheckDisplay::from_result("ERROR"), CheckDisplay::Error);
}

#[test]
fn test_check_display_is_fail_closed_for_unrecognized_status() {
    let display = CheckDisplay::from_result("SOMETHING_NEW_FROM_PLATFORM");
    assert_eq!(display, CheckDisplay::Unknown);
    assert!(
        !display.is_pass(),
        "an unrecognized status must never render as a pass"
    );
    assert!(
        !CheckDisplay::from_result("").is_pass(),
        "an empty status must never render as a pass"
    );
}

#[test]
fn test_check_table_row_matches_status_and_never_mislabels_pending() {
    let row = format_check_table_row("LOW_SHARPE", "PENDING");
    assert!(row.contains("LOW_SHARPE"), "row must name the check: {row}");
    assert!(
        row.contains("PENDING"),
        "a pending check must not be rendered as PASS/FAIL: {row}"
    );

    let unknown = format_check_table_row("NEW_CHECK", "WEIRD");
    assert!(
        unknown.contains("UNKNOWN"),
        "unknown statuses must be explicit: {unknown}"
    );

    let pass = format_check_table_row("LOW_FITNESS", "PASS");
    assert!(pass.contains("PASS"));
}

#[test]
fn test_skip_checks_is_denied_by_default() {
    assert!(!skip_checks_permitted(None));
    assert!(!skip_checks_permitted(Some("")));
    assert!(!skip_checks_permitted(Some("0")));
    assert!(!skip_checks_permitted(Some("false")));
    assert!(!skip_checks_permitted(Some("yes")));
    assert!(!skip_checks_permitted(Some("allow")));
}

#[test]
fn test_skip_checks_requires_explicit_opt_in() {
    assert!(skip_checks_permitted(Some("1")));
    assert!(skip_checks_permitted(Some(" 1 ")));
    assert!(skip_checks_permitted(Some("true")));
    assert!(skip_checks_permitted(Some("TRUE")));
}

#[test]
fn test_submission_gate_rejects_pending_and_unknown_statuses() {
    let mut checks = all_core_passing();
    checks.push(check("NEW_PLATFORM_CHECK", "SOMETHING_NEW"));
    let eval = evaluate_submission_checks(&stats_with(checks));
    assert!(
        !eval.passed,
        "an unrecognized status must fail the gate: {}",
        eval.message
    );

    let mut pending = all_core_passing();
    pending.push(check("IS_LADDER_SHARPE", "PENDING"));
    let eval = evaluate_submission_checks(&stats_with(pending));
    assert!(!eval.passed, "pending checks must fail the gate");
    assert!(!eval.pending_checks.is_empty());
}

#[test]
fn test_submission_gate_requires_reported_core_checks() {
    let mut checks = all_core_passing();
    checks.retain(|c| c.name != "SELF_CORRELATION");
    let eval = evaluate_submission_checks(&stats_with(checks));
    assert!(
        !eval.passed,
        "a missing core check must fail the gate: {}",
        eval.message
    );
    assert!(eval
        .missing_core_checks
        .iter()
        .any(|m| m == "SELF_CORRELATION"));
}

#[test]
fn test_submission_gate_passes_only_when_all_reported_checks_pass() {
    let eval = evaluate_submission_checks(&stats_with(all_core_passing()));
    assert!(
        eval.passed,
        "all core checks explicitly passing must pass: {}",
        eval.message
    );
}

#[test]
fn test_submission_gate_fails_closed_without_any_checks() {
    let eval = evaluate_submission_checks(&stats_with(Vec::new()));
    assert!(!eval.passed, "an empty check response must fail closed");
    assert_eq!(eval.total_count, 0);
}

#[test]
fn test_submission_gate_explains_already_submitted_state() {
    let eval = evaluate_submission_checks(&stats_with(vec![check("ALREADY_SUBMITTED", "FAIL")]));

    assert!(
        !eval.passed,
        "a submitted alpha must not be reported as a verified pass"
    );
    assert!(
        eval.message.contains("ALREADY SUBMITTED"),
        "message must name the platform state, got: {}",
        eval.message
    );
    assert!(
        !eval.message.contains("Missing required core checks"),
        "the reason must not be misreported as a coverage gap, got: {}",
        eval.message
    );
    assert_eq!(eval.total_count, 1);
    assert_eq!(eval.passed_count, 0);
}

#[test]
fn test_cache_age_label_flags_stale_and_unknown_ages() {
    let (label, stale) = cache_age_label(Some(Duration::from_secs(2 * HOUR)));
    assert!(!stale, "a 2h cache is fresh");
    assert!(
        label.contains('h'),
        "label should be human readable: {label}"
    );

    let (_, stale) = cache_age_label(Some(Duration::from_secs(23 * HOUR)));
    assert!(!stale, "23h is still within the 24h policy");

    let (label, stale) = cache_age_label(Some(Duration::from_secs(25 * HOUR)));
    assert!(stale, "25h exceeds the 24h freshness policy");
    assert!(label.contains('h'));

    let (label, stale) = cache_age_label(None);
    assert!(stale, "an unreadable age must not be treated as fresh");
    assert!(label.contains("unknown"), "got label: {label}");

    let (label, stale) = cache_age_label(Some(Duration::from_secs(30)));
    assert!(!stale);
    assert!(label.contains('s'), "short ages report seconds: {label}");
}

#[test]
fn test_mark_alpha_submitted_in_csv_updates_lowercase_serde_bool() {
    use alphafind::commands::submission::mark_alpha_submitted_in_csv;
    use alphafind::models::SubmittableAlphaRecord;

    let tmp_path = std::env::temp_dir().join(format!(
        "alphafind_test_submittable_{}.csv",
        std::process::id()
    ));

    let records = vec![
        SubmittableAlphaRecord {
            alpha_id: "ALPHA_ONE".to_string(),
            name: "First".to_string(),
            universe: "TOP1000".to_string(),
            sharpe: 1.85,
            fitness: 1.55,
            turnover: 18.2,
            annual_return: 14.5,
            submitted: false,
            expression: "rank(<FIELD>)".to_string(),
            settings: "{}".to_string(),
        },
        SubmittableAlphaRecord {
            alpha_id: "ALPHA_TWO".to_string(),
            name: "Second".to_string(),
            universe: "TOP3000".to_string(),
            sharpe: 2.10,
            fitness: 1.70,
            turnover: 22.0,
            annual_return: 16.0,
            submitted: false,
            expression: "rank(<OTHER_FIELD>)".to_string(),
            settings: "{}".to_string(),
        },
    ];

    {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(true)
            .from_path(&tmp_path)
            .unwrap();
        for r in &records {
            wtr.serialize(r).unwrap();
        }
        wtr.flush().unwrap();
    }

    let updated = mark_alpha_submitted_in_csv(&tmp_path, "ALPHA_ONE").unwrap();
    assert!(updated, "ALPHA_ONE should be marked as submitted");

    let mut rdr = csv::Reader::from_path(&tmp_path).unwrap();
    let loaded: Vec<SubmittableAlphaRecord> = rdr
        .deserialize::<SubmittableAlphaRecord>()
        .map(Result::unwrap)
        .collect();
    assert_eq!(loaded.len(), 2);
    assert!(loaded[0].submitted, "ALPHA_ONE must have submitted == true");
    assert!(
        !loaded[1].submitted,
        "ALPHA_TWO must remain submitted == false"
    );

    let _ = std::fs::remove_file(&tmp_path);
}

#[test]
fn test_delay_thresholds_and_portfolio_alpha_classification() {
    use alphafind::commands::{classify_portfolio_alpha, AlphaClassification};
    use alphafind::config::quant::{
        min_fitness_for_delay, min_sharpe_for_delay, turnover_within_platform_bounds,
    };
    use alphafind::models::PortfolioAlpha;

    // Delay 0 vs Delay 1 official thresholds
    assert!((min_sharpe_for_delay(0) - 2.00).abs() < 1e-9);
    assert!((min_sharpe_for_delay(1) - 1.25).abs() < 1e-9);
    assert!((min_fitness_for_delay(0) - 1.30).abs() < 1e-9);
    assert!((min_fitness_for_delay(1) - 1.00).abs() < 1e-9);

    // Exclusive turnover bounds (1%, 70%)
    assert!(!turnover_within_platform_bounds(0.01));
    assert!(turnover_within_platform_bounds(0.0101));
    assert!(turnover_within_platform_bounds(0.6999));
    assert!(!turnover_within_platform_bounds(0.70));

    let base_alpha = PortfolioAlpha {
        id: "A1".to_string(),
        name: None,
        status: Some("ACTIVE".to_string()),
        stage: Some("OS".to_string()),
        date_submitted: None,
        universe: None,
        decay: None,
        category: None,
        color: None,
        sharpe: None,
        fitness: None,
        returns: None,
        turnover: None,
        direct_url: None,
        code: None,
        tags: None,
        settings: None,
    };
    assert_eq!(
        classify_portfolio_alpha(&base_alpha),
        AlphaClassification::ActiveOs
    );

    let mut decommissioned = base_alpha.clone();
    decommissioned.status = Some("DECOMMISSIONED".to_string());
    assert_eq!(
        classify_portfolio_alpha(&decommissioned),
        AlphaClassification::InactiveOrNonOs
    );

    let mut in_sample = base_alpha.clone();
    in_sample.stage = Some("IS".to_string());
    assert_eq!(
        classify_portfolio_alpha(&in_sample),
        AlphaClassification::InactiveOrNonOs
    );
}
