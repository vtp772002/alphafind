# Architecture and implementation limits

AlphaFind is an experimental Rust CLI that calls WorldQuant BRAIN endpoints and performs local screening and portfolio diagnostics. This page documents the implementation in the repository; it does not imply that a feature has been validated at production scale.

## Component map

| Area | Main implementation | Current responsibility |
|---|---|---|
| CLI | `src/main.rs` | Parses commands and coordinates API/local workflows. |
| API client | `src/client.rs` | Authenticates, requests simulations, polls results, fetches Alpha and portfolio data, and sends check/submit calls. |
| Candidate catalog | `src/taxonomy.rs` | Defines the research taxonomy and returns candidates loaded from the ignored local store `data/private_candidates.json` (override with `ALPHAFIND_CANDIDATES_PATH`), falling back to placeholder templates that contain no private field names. |
| Screening | `src/screener.rs` | Assigns candidates across configured account clients and promotes promising candidates to the main client for another simulation/check. |
| Parameter sweep | `src/autotuner.rs` | Tries configured variants of settings/expression transformations. It does not guarantee improved OOS results. |
| Local portfolio analysis | `src/correlation.rs`, `src/health.rs`, `src/allocation.rs` | Computes fail-closed pairwise correlations, local merged-PnL diagnostics, multi-horizon decay/half-life health, turnover Shannon entropy, HRP weights, and candidate admission gates from fetched/cached series. These are not official platform scores. |
| Crowd usage and local PnL diagnostic | `src/crowd_allocation.rs`, `src/crowd_model.rs`, `src/commands/research.rs` (`shadow`) | Maps formula fields to aggregate BRAIN usage counts and can reweight the user's own profile PnLs. It does not model competitor PnL or forecast official Uniqueness. |
| Other heuristic reports | `src/matrix.rs`, `src/forecast.rs`, `src/crowd.rs`, `src/shadow.rs` | Produces configured interaction, blueprint, crowd-risk, and legacy synthetic-proxy helpers. These do not establish platform-wide alpha co-movement. |

## Screening and account constraints

The screener consumes a bounded `mpsc` queue: `workers_per_account` worker tasks run per configured account, and each task holds that account's semaphore permit while a simulation is in flight. `workers_per_account` is therefore the maximum number of concurrent in-flight simulations per account, not a platform capacity figure. There is a short sleep between dispatches as politeness; it is not a rate limiter, and the CLI does not coordinate quotas across processes.

The transfer lock is an in-process Tokio mutex. It serializes qualifying-candidate promotion inside one process; it is not a lock shared across CLI processes or machines.

BRAIN's Challenge guidelines say an individual should have one active account and must not share an account or ID. Do not use secondary credentials for the same person or share credentials. Any multi-person setup must be assessed against the current platform agreement and each account holder must control their own credentials. Remove secondary-account workflows until their intended compliant use is explicit.

## Local PnL and portfolio calculations

Correlation and portfolio commands operate on data available in local caches and fetched platform records. They are diagnostics, not a reproduction of BRAIN's `isScore` or Challenge `Uniqueness` calculation. Results need to report:

- which Alpha IDs were included and why they were classified as active/OOS;
- date intersection, sample count, missing data, and cache timestamps;
- the portfolio weighting method;
- whether any request was paginated or truncated.

Alpha and dataset reads paginate (`/users/self/alphas` at 100 per page, `/data-sets` at 50 per page) and fail on a page error instead of silently truncating. Pairwise correlation, HRP weight calculation, and candidate admission all fail closed: fewer than 30 overlapping dates (100 for portfolio merge), missing PnL series, or a zero-variance series returns no correlation / `UNKNOWN` / `NoGo` rather than a neutral value or pass. Every portfolio read (`load_portfolio_os`) reports the age of `portfolio_os.json` and explicit active/OOS classification counts, excluding decommissioned or in-sample records. Portfolio commands (`audit`, `portfolio`, `impact`, `health`, `allocate`) expose `--refresh` to force live PnL re-fetches, and local private files (`portfolio_os.json`, `portfolio_pnl.json`, `data_field_usage.json`, `submittable_alphas.csv`) are written with `0600` permissions on Unix.

## Submission path

`alphafind submit` invokes the platform submit endpoint after a local check unless `--skip-checks` is supplied. The evaluator fails closed: it requires every name in `CORE_SUBMISSION_CHECKS` to be present and every returned check to be an explicit `PASS`; empty responses, `PENDING`, `ERROR`, missing core checks, and unrecognized statuses all fail. In addition, `submit` runs a separate local portfolio-policy gate (checking local PnL cache staleness and verifying max local pairwise correlation `< 0.70` against active OS alphas). `--skip-checks` aborts unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` is set, and the run prints a bypass warning.

The platform check gate is still not version-aware and does not capture the raw platform response. Treat the BRAIN UI/API as the authority and keep submission manual.

Non-idempotent POST handling: simulation creation retries only when the request provably never created state — connection failures and explicit `429` responses. Timeouts, resets, and truncated responses fail closed with a reconciliation `fingerprint` instead of retrying. Submission follows the same rule and reports an unknown outcome rather than a failure. Automatic reconciliation against platform state is not implemented yet. The API client also rejects follow-up URLs outside the configured BRAIN host before attaching credentials.

## Heuristic modules

- `alphafind shadow` reports field/dataset participation counts and local profile allocations. Direct field mappings and prefix-based inferences are distinguished. Optional PnL sensitivity reweights only the user's local Alpha PnLs by available field/dataset `alphaCount`; field counts older than 24 hours are excluded from weights. It is not a crowd PnL sample. Field-usage cache entries are labeled with age.
- `src/crowd.rs` retains a deprecated historical-count heuristic with static fallback values; the CLI no longer calls it because those values can be stale.
- `src/shadow.rs` retains legacy helpers that synthesize a comparison series from local Alpha PnLs and a scalar input. They are not used by the `shadow`, `impact`, or `health` CLI flows and cannot estimate competitor PnL or official Uniqueness. Do not describe their proxy correlation as a platform score.
- The `shadow` local PnL sensitivity compares two baskets derived from the same user's profile. It is a local diagnostic, not evidence about cross-user PnL correlation. Only the current official Uniqueness value can be read from the account competition response; the CLI does not forecast a future value.
- The dataset interaction matrix uses configured affinity values and a limited dataset-count sample; it is not a measured pairwise co-occurrence matrix over all submitted Alphas.
- Blueprint recommendations contain fixed archetypes/assumptions; they are not an optimizer with a statistically validated forecast.

Rename outputs and docs accordingly, and show data provenance and assumption flags in every report.

## Changes required before relying on automated submission

Done:

1. Fail-closed check evaluation: missing core checks, `PENDING`, `ERROR`, empty, and unknown statuses all reject, covered by tests.
2. Conspicuous bypass handling: `--skip-checks` requires an explicit environment opt-in and prints a bypass warning.
3. Bounded simulation dispatch: `mpsc` work queue plus per-account semaphore, and no automatic retry of ambiguous non-idempotent POSTs.
4. Pagination for Alpha and dataset reads, cache-age reporting for the local PnL cache and field-usage metadata, and fail-closed low-overlap correlation.
5. Host allowlist for authenticated follow-up URLs.
6. Private candidates, Alpha IDs, and downloaded field metadata out of tracked files, with a sanitizer that also scans for the private store's own literals.

Still open:

7. Version-aware check expectations plus safe raw-response capture.
8. A separate local portfolio-policy gate (self-correlation, concentration, staleness) in front of submission.
9. Automatic reconciliation for ambiguous simulation/submission outcomes, and cross-process rate limiting with platform-quota handling.
10. Explicit active/OOS classification and refresh controls on every Alpha/PnL read.
11. Correlation-coverage and account-policy boundary tests.
12. Git history still contains real field identifiers from earlier candidate catalogs; purge or retire it before public distribution.

See [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md) for current official criteria and [CLI Reference](CLI_REFERENCE.md) for the command-level caveats.
