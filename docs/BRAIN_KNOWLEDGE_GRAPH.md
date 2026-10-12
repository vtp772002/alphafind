# BRAIN operators and research notes

This is a compact orientation to operator families used by AlphaFind. It is not a complete operator specification and does not replace the signed-in [BRAIN Learn section](https://platform.worldquantbrain.com/learn), which is the authority for current syntax, data availability, and settings.

The project labels research ideas as analyst, options, microstructure, quality, risk, short-interest, news, and cross-dataset. These are internal labels; they are not an official platform taxonomy and do not imply that a data package is predictive, uncrowded, or safe to submit.

## Operator families

| Family | Examples of generic signatures | Typical role |
|---|---|---|
| Time series | `ts_mean(x, d)`, `ts_delta(x, d)`, `ts_rank(x, d)`, `ts_corr(x, y, d)` | Transform a field through time for each instrument. |
| Cross section | `rank(x)`, `scale(x, a)` | Compare or rescale values across the selected universe. |
| Group operations | `group_rank(x, group)`, `group_neutralize(x, group)` | Rank within groups or remove group-level effects. |
| Nonlinear transforms | `signed_power(x, p)`, `winsorize(x, limit)` | Change signal shape or limit outliers. |
| Conditional operations | `trade_when(condition, x, fallback)` | Apply platform-defined conditional position behavior. |

Arguments and operator semantics can vary by operator. Confirm the current signature, input type, unit, delay, and missing-data behavior in BRAIN Learn before using an operator.

## Research protocol

1. State the economic hypothesis before selecting an operator.
2. Confirm that each field exists for the selected region, universe, delay, and date range.
3. Check publication timing and missingness to avoid look-ahead or stale-value assumptions.
4. Run the platform simulation using the intended settings. Compare alternatives using the same evaluation window.
5. Review all current submission tests and then monitor OOS behavior.
6. Record unsuccessful variants in a private experiment log to account for selection bias.

Do not infer a performance gain from an operator name or from a short in-sample result. Any proposed universe, neutralization, decay, exponent, or truncation is a hypothesis requiring simulation and robustness checks.

## What AlphaFind's research helpers mean

- `radar` reports dataset census information exposed by the API at query time. Counts can change and do not establish future uniqueness.
- `matrix` uses configured affinity assumptions; it is not an observed platform-wide Alpha co-occurrence matrix.
- `blueprint` emits fixed-rule suggestions, not a validated optimizer forecast.
- `shadow` reports aggregate dataset/field usage and local profile allocations. Its optional PnL sensitivity reweights only the user's own Alpha PnLs; this is not competitor PnL and does not forecast the platform's `Uniqueness` score.

## Live dataset-census snapshot

**Checked:** 2026-10-12 through `GET /data-sets?region=USA` with the main account. The response contained 150 configuration rows but only 14 distinct dataset IDs; the same ID appears across universe/delay configurations. Treat the row as a dataset configuration, not as one unique dataset, and compare crowd counts only at the same region, universe, and delay.

Selected Delay 1 rows from the current response:

| Dataset | Universe | Users | Alphas | Coverage |
|---|---:|---:|---:|---:|
| `model16` — Fundamental Scores | TOP1000 | 253 | 495 | 0.5582 |
| `socialmedia8` — Social Media Data for Equity | TOP1000 | 378 | 1,083 | 0.9999 |
| `news18` — Ravenpack News Data | TOP1000 | 1,412 | 2,520 | 0.7301 |
| `pv1` — Price Volume Data for Equity | TOP1000 | 22,769 | 90,031 | — |
| `pv1` — Price Volume Data for Equity | TOP3000 | 92,205 | 2,165,639 | — |

These counts measure dataset usage, not the platform's `Uniqueness` score or predictive value. The lower-count `model16` row also has lower cross-sectional coverage; `socialmedia8` has much higher coverage but fewer fields. Use the census to prioritize hypotheses, then check field availability, missingness, economic rationale, simulation results, and OOS performance. Do not treat a low user count as proof that a candidate is unique or likely to pass.

The API returned 66 operators in the same session. That is a live catalog count, not a guarantee that AlphaFind's operator notes are complete; check BRAIN Learn and the current operator definitions for syntax and input constraints.

Treat these outputs as prompts for research, not as platform facts or automated submission approval. See [Architecture](ARCHITECTURE.md) for implementation limits and [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md) for current official thresholds.

API sources: [dataset inventory](https://api.worldquantbrain.com/data-sets?region=USA) and [operator catalog](https://api.worldquantbrain.com/operators). Counts above are a dated snapshot and may change.
