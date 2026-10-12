# AlphaFind project instructions

## Scope and evidence standard

These instructions govern work in this repository. They describe project policy, not WorldQuant BRAIN policy. Verify platform rules in the signed-in BRAIN interface before relying on them; current links and a dated summary are in [`docs/BRAIN_RULES_AND_SUBMISSION.md`](docs/BRAIN_RULES_AND_SUBMISSION.md).

Separate findings into:

- **Official platform rule** — supported by current BRAIN Learn, Challenge Guidelines, or an official API response.
- **Project policy** — a chosen local safety or research threshold.
- **Research hypothesis** — a proposed strategy or parameter choice that still needs evidence.
- **Unknown** — unavailable, stale, incomplete, or account-specific information.

Do not describe a heuristic or internal target as an official BRAIN threshold. The current Challenge leaderboard API schema describes `Total Score` as equally weighted across ranked `Uniqueness`, `IS Score`, and `Days of Submission`. Use that documented relationship, but do not invent the rank normalization, tie handling, or an `isScore` calculation beyond what the API documents.

## Accounts and platform rules

- Use one BRAIN account controlled by the individual participant. Do not share account credentials or account IDs, or operate multiple accounts for one participant.
- Do not configure secondary-account workflows for one person. Any multi-person use must comply with current platform terms, with credentials controlled by their individual account holders.
- The regular Challenge scores only qualifying OOS alphas and evaluates them relative to other alphas. The current leaderboard schema says Total Score equally weights the ranks of Uniqueness, IS Score, and Days of Submission; lower Uniqueness is better. The schema does not specify rank normalization or tie handling. The platform can change scoring rules. Do not assume that Challenge, consultant status, and IQC share a score formula.
- The current BRAIN Learn submission article lists Fitness, Sharpe, Turnover, Weight, Sub-universe, and Self-correlation criterion groups. Thresholds depend on delay and universe. See the linked, dated reference in `docs/`.
- Do not state that exactly one alpha per UTC day is a platform rule. A submission cadence may be an internal policy only if its purpose and evidence are documented.

## Quantitative research standards

- Internal correlation against the local portfolio is different from BRAIN self-correlation and competition `Uniqueness`.
- Report date coverage, observation count, missing data, cache age, included alphas, and weighting assumptions for local portfolio analysis.
- Treat insufficient overlap, stale data, absent results, and API errors as unknown/fail-closed; never translate them to zero correlation or a pass.
- Do not promise performance from a universe, data set, neutralization, decay, transform, or truncation choice. Record such choices as hypotheses and verify them with platform simulations and OOS behavior.
- Avoid look-ahead, repeated selection on one in-sample period, and claims of causality from backtests.

## Submission safety

- The official BRAIN UI/API is the authority for current checks and OOS eligibility. AlphaFind output is not approval.
- The current client check parser treats only the exact `FAIL` result as a failure; `PENDING`, `ERROR`, unknown results, and missing checks can therefore be misreported as a pass. Do not rely on `alphafind check` or automate `alphafind submit` until it verifies complete expected coverage and explicit passing statuses.
- `--skip-checks` bypasses local preflight. Never make it the default or describe a submission as verified when it was used.
- Keep external submission as a deliberate operation after reviewing the current platform result. Do not dispatch duplicate simulations after ambiguous non-idempotent POST failures without reconciling server state.

## Account concurrency and screening

- Do not claim a fixed nine-worker capacity. The current screener starts an async task per candidate; `workers_per_account` does not bound in-flight tasks.
- Do not increase screening volume or concurrency until there is a bounded work queue, rate limiting, and handling for platform quotas and ambiguous requests.
- `transfer_lock` is a process-local mutex; it is not a lock across machines or CLI processes.
- Do not describe worker-account promotion as compliant unless the account holders and platform rules permit the workflow.

## Private research material and Git hygiene

Alpha expressions under evaluation, Alpha IDs, pending metrics, private PnL, credentials, and candidate metadata are confidential research material.

- Keep private candidate formulas and ledgers outside tracked files. Load them from a local, permission-restricted store excluded from Git. Public examples must use abstract placeholders such as `<SIGNAL>`, `<FIELD>`, `<GROUP>`, and `<PARAMETER>`.
- Do not put live IDs, pending formulas, or exact candidate metrics in source, tests, documentation, logs, branch names, commit messages, releases, or pull-request descriptions.
- The current repository contains candidate expressions in tracked source. Treat this as a release-blocking privacy issue. Moving or deleting them in a new commit does not remove prior Git history; review public history and rotate exposed candidates if they were published.
- Before staging, inspect `git status`, the staged diff, ignore rules, and the complete tracked-file scan. Stage only named files; never blindly stage the whole tree.
- Run the sanitizer only as one signal. Its current checks do not establish that every formula or private artifact is absent. Inspect its coverage before relying on it.
- Exploratory scripts are disposable: put them in ignored `scratch/`, never in the repository root, and remove them when finished. Do not retain scratch output in Git.

## Documentation standards

- Date time-sensitive platform claims and link to official sources.
- Label recommendations and thresholds as internal policy or hypotheses unless an official source verifies them.
- Document actual code behavior and current limitations. Do not claim exact forecasts, all-user crowd data, fixed sample lengths, SIMD, bounded workers, or complete pagination unless the implementation proves it.
- Keep examples generic and non-sensitive. Do not include live candidate formulas or IDs.

## High-priority engineering backlog

1. Make check parsing complete, explicit, version-aware, and fail-closed; test empty, partial, unknown, and failed responses.
2. Disable or guard submission until the official check gate is reliable.
3. Move candidate expressions and private performance records out of tracked source and inspect Git history.
4. Add bounded simulation concurrency, rate limiting, and idempotency/reconciliation for simulation requests.
5. Add pagination and explicit active/OOS classification to Alpha and PnL reads; add cache age and refresh controls.
6. Reject authenticated follow-up URLs outside the configured BRAIN host.
7. Add tests for API parsing, account policy boundaries, correlation coverage, cache behavior, and submission safety before claiming production readiness.
