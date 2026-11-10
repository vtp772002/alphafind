//! Local Test Period (held-out block) gate.
//!
//! **Project policy / research hypothesis, not a BRAIN rule.** BRAIN advises reserving the
//! newest ~2 years of the IS window as a Test Period. The thresholds below are internal
//! heuristics. A short block has a noisy Sharpe (standard error is roughly `1/sqrt(years)`),
//! so this gate is a local signal only and never replaces the platform check gate.
//!
//! The gate is fail-closed: missing, non-finite, or insufficient data is `Unknown`, never `Pass`.

/// Project-policy thresholds for the held-out comparison.
#[derive(Debug, Clone, Copy)]
pub struct TestPeriodPolicy {
    /// Minimum train (development) Sharpe for the comparison to be meaningful.
    pub min_train_sharpe: f64,
    /// Test Sharpe must be at least this fraction of train Sharpe.
    pub min_retention: f64,
    /// Minimum length of the held-out block, in years.
    pub min_test_years: f64,
}

impl Default for TestPeriodPolicy {
    fn default() -> Self {
        Self {
            min_train_sharpe: 0.5,
            min_retention: 0.5,
            min_test_years: 1.5,
        }
    }
}

/// Inputs, all optional so that absence is representable and fails closed.
#[derive(Debug, Clone, Copy, Default)]
pub struct TestPeriodInput {
    pub train_sharpe: Option<f64>,
    pub test_sharpe: Option<f64>,
    pub test_years: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestPeriodVerdict {
    Pass { retention: f64 },
    Fail { reason: String },
    Unknown { reason: String },
}

impl TestPeriodVerdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }
}

fn finite(v: Option<f64>) -> Option<f64> {
    v.filter(|x| x.is_finite())
}

/// Evaluate the held-out block against the policy. Only `Pass` permits proceeding.
pub fn evaluate_test_period(input: &TestPeriodInput, policy: &TestPeriodPolicy) -> TestPeriodVerdict {
    let (Some(train), Some(test), Some(years)) = (
        finite(input.train_sharpe),
        finite(input.test_sharpe),
        finite(input.test_years),
    ) else {
        return TestPeriodVerdict::Unknown {
            reason: "train Sharpe, test Sharpe, or test length missing/non-finite".into(),
        };
    };
    if years < policy.min_test_years {
        return TestPeriodVerdict::Unknown {
            reason: format!(
                "test block {years:.2}y shorter than policy minimum {:.2}y",
                policy.min_test_years
            ),
        };
    }
    if train < policy.min_train_sharpe {
        return TestPeriodVerdict::Unknown {
            reason: format!(
                "train Sharpe {train:.2} below {:.2}; comparison not meaningful",
                policy.min_train_sharpe
            ),
        };
    }
    if test <= 0.0 {
        return TestPeriodVerdict::Fail {
            reason: format!("test Sharpe {test:.2} is not positive"),
        };
    }
    let retention = test / train;
    if retention < policy.min_retention {
        return TestPeriodVerdict::Fail {
            reason: format!(
                "retention {retention:.2} below policy minimum {:.2}",
                policy.min_retention
            ),
        };
    }
    TestPeriodVerdict::Pass { retention }
}

/// Trading days per year used to annualize the daily Sharpe (matches `config`).
const ANNUALIZATION_DAYS: f64 = 252.0;
/// Minimum observations per block for a Sharpe estimate to be reported.
pub const MIN_BLOCK_OBSERVATIONS: usize = 60;
/// Default held-out block length in calendar years (BRAIN's suggested 2 years).
pub const DEFAULT_TEST_YEARS: u32 = 2;

/// Days since 1970-01-01 for a proleptic Gregorian date (Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parses a strict `YYYY-MM-DD` string into (year, month, day, day number).
fn parse_date(s: &str) -> Option<(i64, i64, i64, i64)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d, days_from_civil(y, m, d)))
}

/// Annualized Sharpe of a daily PnL slice; `None` if too short or flat.
fn annualized_sharpe(daily: &[f64]) -> Option<f64> {
    if daily.len() < MIN_BLOCK_OBSERVATIONS {
        return None;
    }
    let n = daily.len() as f64;
    let mean = daily.iter().sum::<f64>() / n;
    let var = daily.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = var.sqrt();
    if !sd.is_finite() || sd < 1e-12 {
        return None;
    }
    Some(mean / sd * ANNUALIZATION_DAYS.sqrt())
}

/// Splits a daily PnL series into train/test blocks and computes both Sharpes.
///
/// The test block is the newest `test_years` calendar years *of the series itself*
/// (relative to its last date, not today). Values must be daily PnL, not cumulative.
/// Rows with unparsable dates or non-finite values are dropped; if nothing usable
/// remains the returned input is empty, which evaluates to `Unknown`.
pub fn split_daily_pnl(
    pnl: &std::collections::HashMap<String, f64>,
    test_years: u32,
) -> TestPeriodInput {
    let mut rows: Vec<(i64, i64, i64, i64, f64)> = pnl
        .iter()
        .filter(|(_, v)| v.is_finite())
        .filter_map(|(k, v)| parse_date(k).map(|(y, m, d, n)| (n, y, m, d, *v)))
        .collect();
    rows.sort_by_key(|r| r.0);
    let Some(&(_, ly, lm, ld, _)) = rows.last() else {
        return TestPeriodInput::default();
    };
    // Cutoff: same month/day `test_years` earlier (Feb 29 clamps to Feb 28).
    let (cy, cd) = (ly - test_years as i64, if lm == 2 && ld == 29 { 28 } else { ld });
    let cutoff = days_from_civil(cy, lm, cd);
    let train: Vec<f64> = rows.iter().filter(|r| r.0 <= cutoff).map(|r| r.4).collect();
    let test: Vec<f64> = rows.iter().filter(|r| r.0 > cutoff).map(|r| r.4).collect();
    let test_days = rows.last().map(|r| r.0).unwrap_or(cutoff) - cutoff;
    TestPeriodInput {
        train_sharpe: annualized_sharpe(&train),
        test_sharpe: annualized_sharpe(&test),
        test_years: Some(test_days as f64 / 365.25),
    }
}
