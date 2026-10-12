# Architecture and implementation limits

AlphaFind is an experimental Rust CLI that calls WorldQuant BRAIN endpoints and performs local screening and portfolio diagnostics. This page documents the implementation in the repository; it does not imply that a feature has been validated at production scale.

## Component map

| Area | Main implementation | Current responsibility |
|---|---|---|
| CLI | `src/main.rs` | Parses commands and coordinates API/local workflows. |
| API client | `src/client.rs` | Authenticates, requests simulations, polls results, fetches Alpha and portfolio data, and sends check/submit calls. |
| Candidate catalog | `src/taxonomy.rs` | Stores candidate expressions/settings grouped by internal research labels. The expressions are private research material and should not be committed to a public repository. |
| Screening | `src/screener.rs` | Assigns candidates across configured account clients and promotes promising candidates to the main client for another simulation/check. |
| Parameter sweep | `src/autotuner.rs` | Tries configured variants of settings/expression transformations. It does not guarantee improved OOS results. |
| Local portfolio analysis | `src/correlation.rs` | Computes correlations and local merged-PnL diagnostics from fetched/cached series. These are not official platform scores. |
| Crowd usage and local PnL diagnostic | `src/crowd_allocation.rs`, `src/crowd_model.rs`, `src/main.rs` (`shadow`) | Maps formula fields to aggregate BRAIN usage counts and can reweight the user's own profile PnLs. It does not model competitor PnL or forecast official Uniqueness. |
| Other heuristic reports | `src/matrix.rs`, `src/forecast.rs`, `src/crowd.rs`, `src/shadow.rs` | Produces configured interaction, blueprint, crowd-risk, and legacy synthetic-proxy helpers. These do not establish platform-wide alpha co-movement. |

## Screening and account constraints

The screener creates one asynchronous task per candidate and assigns tasks to configured account clients in round-robin order. `workers_per_account` currently affects the displayed worker count; it does not bound task concurrency with a semaphore or fixed-size work queue. Candidate counts can therefore create more in-flight work than the printed worker total suggests.

The transfer lock is an in-process Tokio mutex. It serializes qualifying-candidate promotion inside one process; it is not a lock shared across CLI processes or machines.

BRAIN's Challenge guidelines say an individual should have one active account and must not share an account or ID. Do not use secondary credentials for the same person or share credentials. Any multi-person setup must be assessed against the current platform agreement and each account holder must control their own credentials. Remove secondary-account workflows until their intended compliant use is explicit.

## Local PnL and portfolio calculations

Correlation and portfolio commands operate on data available in local caches and fetched platform records. They are diagnostics, not a reproduction of BRAIN's `isScore` or Challenge `Uniqueness` calculation. Results need to report:

- which Alpha IDs were included and why they were classified as active/OOS;
- date intersection, sample count, missing data, and cache timestamps;
- the portfolio weighting method;
- whether any request was paginated or truncated.

As currently implemented, several fetch paths are not fully paginated and some commands reuse an existing cache instead of refreshing it. Correlation code returns a neutral-looking value when overlap is too short. Treat missing/stale/insufficient data as **unknown**, not as zero correlation or a passing audit.

## Submission path

`alphafind submit` invokes the platform submit endpoint after a local check unless `--skip-checks` is supplied. The current check parser treats only the exact `FAIL` result as a failure. It can therefore pass when results are `PENDING`, `ERROR`, unknown, or absent from a partial response. It does not enforce every local portfolio policy. Until the parser verifies that every expected current check is present with an explicit `PASS`, use the BRAIN UI/API as the authority and consider disabling the CLI submit path.

Network retries around simulation creation also need idempotency review: retrying a non-idempotent request after an ambiguous response can create duplicate simulations. The API client should validate the host of URLs returned by the platform before attaching credentials.

## Heuristic modules

- `alphafind shadow` reports field/dataset participation counts and local profile allocations. Direct field mappings and prefix-based inferences are distinguished. Optional PnL sensitivity reweights only the user's local Alpha PnLs by available field/dataset `alphaCount`; field counts older than 24 hours are excluded from weights. It is not a crowd PnL sample. Field-usage cache entries are labeled with age.
- `src/crowd.rs` retains a deprecated historical-count heuristic with static fallback values; the CLI no longer calls it because those values can be stale.
- `src/shadow.rs` retains legacy helpers that synthesize a comparison series from local Alpha PnLs and a scalar input. They are not used by the `shadow`, `impact`, or `health` CLI flows and cannot estimate competitor PnL or official Uniqueness. Do not describe their proxy correlation as a platform score.
- The `shadow` local PnL sensitivity compares two baskets derived from the same user's profile. It is a local diagnostic, not evidence about cross-user PnL correlation. Only the current official Uniqueness value can be read from the account competition response; the CLI does not forecast a future value.
- The dataset interaction matrix uses configured affinity values and a limited dataset-count sample; it is not a measured pairwise co-occurrence matrix over all submitted Alphas.
- Blueprint recommendations contain fixed archetypes/assumptions; they are not an optimizer with a statistically validated forecast.

Rename outputs and docs accordingly, and show data provenance and assumption flags in every report.

## Changes required before relying on automated submission

1. Make API checks fail closed: require a complete, versioned set of expected statuses, reject unknown/missing statuses, and capture the raw response safely.
2. Enforce the platform's current submission settings and explicit local policy in separate gates; make bypasses conspicuous and unavailable by default.
3. Add a bounded queue/rate limiter and idempotency handling to simulation dispatch.
4. Add pagination, cache timestamps/refresh controls, and explicit coverage checks to Alpha/PnL reads.
5. Restrict authenticated follow-up URLs to the configured BRAIN host.
6. Move private expressions, Alpha IDs, and candidate metrics out of tracked files into a local, permission-restricted store; scan git history before public release.
7. Add tests for API response parsing, pagination, cache staleness, low-overlap correlation, retries, and submission gates before enabling automatic submit.

See [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md) for current official criteria and [CLI Reference](CLI_REFERENCE.md) for the command-level caveats.
