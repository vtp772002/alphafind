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

The platform's visible Learn article currently presents six criterion groups. AlphaFind's local check evaluator (`evaluate_submission_checks` in `src/client.rs`) is fail-closed: it requires every expected core check in `CORE_SUBMISSION_CHECKS` to be present and every returned check to be an explicit `PASS`, rejecting empty responses, `PENDING`, `ERROR`, `FAIL`, and unrecognized statuses. However, the expected check list is hardcoded rather than version-aware, and the raw response is not persisted. Therefore:

- a missing, partial, or changed API response must not be treated as approval;
- a local pass is not proof that the BRAIN UI will allow submission;
- the API response must be checked in the signed-in BRAIN UI/API before relying on automated submission.

## 5. Five-gate review and automated submission policy

These 5 gates are executed sequentially to filter candidate alphas and prevent overfitting. When a candidate passes all 5 gates, automated submission (`alphafind submit`) is authorized:

1. **Gate 1 - Official Platform Pre-flight Checks**: Full `PASS` on all core checks from BRAIN API (`LOW_SHARPE >= 1.25`, `LOW_FITNESS >= 1.00`, turnover within bounds, `LOW_SUB_UNIVERSE_SHARPE`, `SELF_CORRELATION < 0.70`).
2. **Gate 2 - Held-out Test Period Gate**: Preserves Sharpe on the newest 2-year held-out block of IS daily PnL (Train Sharpe $\ge 0.50$, Test Sharpe $> 0$, Retention $\ge 50\%$).
3. **Gate 3 - Local Portfolio Policy Gate**: Pairwise Pearson correlation against active OS alphas across $\ge 100$ overlapping days must satisfy $\max(\rho) < 0.70$.
4. **Gate 4 - Portfolio Merge Impact Gate**: Non-negative marginal delta Sharpe ($\Delta \text{Sharpe} \ge 0.0$), with portfolio volatility preserved or collapsed.
5. **Gate 5 - Overfitting Robustness & Research Hygiene**:
   - Signal rank test (preserving positive Sharpe without non-linear tails).
   - Binary quantization test (preserving sign direction).
   - Super-universe invariance (e.g. profitable on TOP1000).
   - Research hygiene (zero leaked formulas or private IDs in tracked files).

A failed or unavailable data fetch fails closed. Zero overlap, stale PnL, or an empty active portfolio is not evidence of zero correlation or good diversification.

## 6. Crowd usage estimate

`alphafind shadow` maps formula identifiers to datasets and reads aggregate `userCount`/`alphaCount` from BRAIN `/data-sets` and `/data-fields`. It also counts dataset allocations found in the local `portfolio_os.json`. Field mappings use live/cache API metadata or local downloaded metadata; when neither maps a field directly, a unique dataset association shared by that field prefix in local metadata is labeled as an inference. These counts measure participation and observed concept usage only. They do not reveal which competitors hold a field, how they weight it, their PnL, or the correlation of their merged portfolios.

When PnLs are available, `shadow` can create a usage-weighted basket from the user's own allocated Alpha PnLs. It averages fresh mapped-field `alphaCount` values per dataset and then across each Alpha's mapped datasets; where no fresh field counts exist it falls back to dataset `alphaCount`. Fresh field counts come from a live response or cache no older than 24 hours. It compares that basket with an equal-weight merge from the same local profile. This is a local sensitivity diagnostic, not a crowd PnL sample; it does not establish a quantitative relationship with cross-user correlation. The command can display the current official Uniqueness from the account competition response, but it must not add the local proxy-correlation change to that score or present it as a future-score forecast. From aggregate usage counts alone, future Uniqueness is not identifiable.

For candidate fields, the CLI can report the mean ratio `field alphaCount / dataset alphaCount` and `field userCount / dataset userCount`, using matched region, universe, and delay settings. These are popularity measures. A small ratio may prioritize a candidate for further research, but it does not imply low PnL correlation or high Uniqueness.

## Further reading

- [BRAIN rules and submission](BRAIN_RULES_AND_SUBMISSION.md)
- [BRAIN knowledge graph](BRAIN_KNOWLEDGE_GRAPH.md)
- [Architecture and implementation limits](ARCHITECTURE.md)
