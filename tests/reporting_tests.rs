//! Reporting-format tests for console helpers.

use alphafind::commands::{format_epoch_utc, format_snapshot_timestamp};

#[test]
fn test_format_epoch_utc_handles_epoch_and_leap_day() {
    assert_eq!(format_epoch_utc(0), "1970-01-01 00:00");
    assert_eq!(format_epoch_utc(86_400), "1970-01-02 00:00");
    assert_eq!(
        format_epoch_utc(951_782_400),
        "2000-02-29 00:00",
        "leap-day handling must be exact"
    );
}

#[test]
fn test_format_epoch_utc_matches_known_snapshot() {
    assert_eq!(format_epoch_utc(1_792_216_032), "2026-10-17 05:47");
}

#[test]
fn test_format_snapshot_timestamp_renders_epoch_with_age() {
    let raw = "Unix Epoch 1792216032";
    let rendered = format_snapshot_timestamp(raw, 1_792_216_032 + 3_600);

    assert!(
        rendered.contains("2026-10-17 05:47 UTC"),
        "epoch must become a readable date, got: {rendered}"
    );
    assert!(
        rendered.contains("1.0h old"),
        "age must be reported, got: {rendered}"
    );
    assert!(
        !rendered.contains("1792216032"),
        "raw epoch must not leak into the display, got: {rendered}"
    );
}

#[test]
fn test_format_snapshot_timestamp_age_buckets() {
    let secs = 1_792_216_032;
    let raw = format!("Unix Epoch {secs}");

    assert!(format_snapshot_timestamp(&raw, secs + 120).contains("2m old"));
    assert!(format_snapshot_timestamp(&raw, secs + 90_000).contains("1.0d old"));
    assert!(
        format_snapshot_timestamp(&raw, secs).contains("0m old"),
        "a just-written snapshot should report zero minutes"
    );
}

#[test]
fn test_format_snapshot_timestamp_passes_through_unknown_formats() {
    assert_eq!(format_snapshot_timestamp("", 0), "");
    assert_eq!(
        format_snapshot_timestamp("2026-10-17T05:47:12Z", 0),
        "2026-10-17T05:47:12Z"
    );
    assert_eq!(
        format_snapshot_timestamp("Unix Epoch not-a-number", 0),
        "Unix Epoch not-a-number"
    );
}
