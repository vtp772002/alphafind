# Research methodology

This document describes AlphaFind's local research approach. It does not define WorldQuant BRAIN's official scoring formula. Current platform submission criteria and Challenge rules are summarized in [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md).

## 1. Portfolio diversification: useful model, limited claim

For an equal-weight portfolio of `N` signals with equal individual volatility and the same pairwise correlation `rho`, a simplified variance model gives:

```text
portfolio variance / single-signal variance = 1/N + (N - 1) * rho/N
```

This illustrates why adding highly correlated signals may contribute little diversification. The assumptions are restrictive: actual signals have different returns, volatilities, correlations, missing dates, and portfolio weights. This equation is not BRAIN's `isScore` formula and does not predict a leaderboard score.

AlphaFind's local Pearson audit measures correlation on overlapping dates from the PnL series available to the tool. A local target such as `abs(rho) <= 0.15` is an internal portfolio research preference, not the platform's published self-correlation threshold and not the platform's cross-user `Uniqueness` metric.

## 2. Factor ideas are hypotheses

The code groups candidate ideas into internal labels such as analyst, options, microstructure, quality, risk, short interest, and news. These are research buckets, not an official BRAIN taxonomy or proof that a dataset is predictive or uncrowded.

For each candidate, record and compare:

- economic rationale and information timing;
- data availability, delay, and missingness;
- universe and neutralization settings;
- turnover, drawdown, Sharpe, Fitness, and OOS behavior;
- overlap with the existing portfolio and robustness across sub-universes.

Treat recommendations about a specific universe, dataset, decay, exponent, or neutralization as hypotheses to test. A transformation can improve one metric while harming another; no parameter range guarantees a passing result.

## 3. Avoid selection bias

Repeatedly tuning a formula on the same in-sample period can overfit even when each individual simulation looks plausible. Keep a record of tried variants, avoid selecting solely on the best observed score, and use platform OOS results as a separate validation signal. Preserve a holdout or a predeclared evaluation rule for local experiments where possible.

Do not describe an association, backtest, or simulated PnL as causal evidence. Avoid conclusions that depend on one short interval or one universe.

## 4. Platform submission checks

The current BRAIN Learn page lists Fitness, Sharpe, Turnover, Weight, Sub-universe, and Self-correlation criteria. The exact Delay 0/Delay 1 thresholds and the sub-universe rule are in [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md).

The platform's visible Learn article currently presents six criterion groups. AlphaFind's API check code contains an eight-check presentation, but its parser currently treats responses with no explicit `FAIL` as passing. Therefore:

- a missing, partial, or changed API response must not be treated as approval;
- a local pass is not proof that the BRAIN UI will allow submission;
- the API response must be checked for complete coverage and freshness before this gate is trusted.

## 5. Internal review gates

These gates are useful only when backed by current data and adequate coverage:

1. Re-run the exact candidate and settings on the intended account.
2. Confirm each official BRAIN criterion and OOS eligibility on the platform.
3. Compute local correlation only on a stated date intersection and report sample size, missingness, and cache age.
4. Compare portfolio impact using a clearly stated weighting rule; label it a local estimate.
5. Review candidate provenance and ensure no pending formula, ID, or private metrics enter tracked public files.

A failed or unavailable data fetch should fail closed. Zero overlap, stale PnL, or an empty active portfolio is not evidence of zero correlation or good diversification.

## 6. Crowd usage estimate

`alphafind shadow` maps formula identifiers to datasets and reads aggregate `userCount`/`alphaCount` from BRAIN `/data-sets` and `/data-fields`. It also counts dataset allocations found in the local `portfolio_os.json`. Field mappings use live/cache API metadata or local downloaded metadata; when neither maps a field directly, a unique dataset association shared by that field prefix in local metadata is labeled as an inference. These counts measure participation and observed concept usage only. They do not reveal which competitors hold a field, how they weight it, their PnL, or the correlation of their merged portfolios.

When PnLs are available, `shadow` can create a usage-weighted basket from the user's own allocated Alpha PnLs. It averages fresh mapped-field `alphaCount` values per dataset and then across each Alpha's mapped datasets; where no fresh field counts exist it falls back to dataset `alphaCount`. Fresh field counts come from a live response or cache no older than 24 hours. It compares that basket with an equal-weight merge from the same local profile. This is a local sensitivity diagnostic, not a crowd PnL sample; it does not establish a quantitative relationship with cross-user correlation. The command can display the current official Uniqueness from the account competition response, but it must not add the local proxy-correlation change to that score or present it as a future-score forecast. From aggregate usage counts alone, future Uniqueness is not identifiable.

For candidate fields, the CLI can report the mean ratio `field alphaCount / dataset alphaCount` and `field userCount / dataset userCount`, using matched region, universe, and delay settings. These are popularity measures. A small ratio may prioritize a candidate for further research, but it does not imply low PnL correlation or high Uniqueness.

## Further reading

- [BRAIN rules and submission](BRAIN_RULES_AND_SUBMISSION.md)
- [BRAIN knowledge graph](BRAIN_KNOWLEDGE_GRAPH.md)
- [Architecture and implementation limits](ARCHITECTURE.md)
