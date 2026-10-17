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
