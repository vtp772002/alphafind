use alphafind::correlation::{pearson_correlation, simulate_portfolio};
use alphafind::matrix::analyze_formula_hyper_synergy;
use std::collections::HashMap;
use std::time::Instant;

/// Helper to generate synthetic daily PnL
fn generate_pnl(n: usize, seed: f64) -> HashMap<String, f64> {
    (0..n)
        .map(|i| {
            let date = format!("2020-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
            let value = ((i as f64 * seed).sin()) * 0.01;
            (date, value)
        })
        .collect()
}

#[test]
fn bench_correlation_pairwise_scaling() {
    let n_alphas = 30;
    let n_days = 300;
    let alphas: Vec<HashMap<String, f64>> = (0..n_alphas)
        .map(|i| generate_pnl(n_days, i as f64 + 1.0))
        .collect();

    let start = Instant::now();
    let mut pairs_computed = 0;
    let mut sum_corr = 0.0;

    for i in 0..n_alphas {
        for j in (i + 1)..n_alphas {
            let (corr, days) = pearson_correlation(&alphas[i], &alphas[j]);
            assert_eq!(days, n_days);
            if let Some(c) = corr {
                sum_corr += c;
            }
            pairs_computed += 1;
        }
    }

    let elapsed = start.elapsed();
    assert_eq!(pairs_computed, n_alphas * (n_alphas - 1) / 2);
    assert!(sum_corr.is_finite());
    println!(
        "Benchmark: computed {} correlation pairs across {} alphas ({} days) in {:?}",
        pairs_computed, n_alphas, n_days, elapsed
    );
    let max_allowed_ms = if cfg!(debug_assertions) { 5000 } else { 300 };
    assert!(elapsed.as_millis() < max_allowed_ms, "Correlation benchmark exceeded {}ms: {:?}", max_allowed_ms, elapsed);
}

#[test]
fn bench_simulate_portfolio_scalability() {
    let n_alphas = 30;
    let n_days = 300;
    let alphas: Vec<HashMap<String, f64>> = (0..n_alphas)
        .map(|i| generate_pnl(n_days, (i as f64 + 1.0) * 0.1))
        .collect();
    let alpha_refs: Vec<&HashMap<String, f64>> = alphas.iter().collect();

    let start = Instant::now();
    let result = simulate_portfolio(&alpha_refs);
    let elapsed = start.elapsed();

    assert!(result.is_some());
    let metrics = result.unwrap();
    assert_eq!(metrics.n_alphas, 30);
    assert_eq!(metrics.n_trading_days, n_days);
    println!(
        "Benchmark: simulate_portfolio with 30 alphas ({} days) completed in {:?}",
        n_days, elapsed
    );
    let max_allowed_ms = if cfg!(debug_assertions) { 10000 } else { 500 };
    assert!(elapsed.as_millis() < max_allowed_ms, "Portfolio simulation exceeded {}ms: {:?}", max_allowed_ms, elapsed);
}

#[test]
fn bench_hyper_synergy_throughput() {
    let dummy_expr = "risk = ts_backfill(unsystematic_risk_last_60_days, 10); \
                      borrow = ts_backfill(shares_sold_short_count_2, 40); \
                      vwap_dev = rank(vwap / close - 1); \
                      signal = 0.5 * risk + 0.3 * borrow + 0.2 * vwap_dev; \
                      trade_when(abs(signal) > 0.01, signal, -1)";

    let start = Instant::now();
    let iterations = 200;
    for _ in 0..iterations {
        let analysis = analyze_formula_hyper_synergy(dummy_expr, "TOP200", &[], None);
        assert_eq!(analysis.participating_pillars.len(), 3);
    }
    let elapsed = start.elapsed();
    println!(
        "Benchmark: {} hyper synergy formula analyses completed in {:?}",
        iterations, elapsed
    );
    let max_allowed_ms = if cfg!(debug_assertions) { 2000 } else { 100 };
    assert!(elapsed.as_millis() < max_allowed_ms, "Hyper synergy benchmark exceeded {}ms: {:?}", max_allowed_ms, elapsed);
}
