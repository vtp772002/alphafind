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
- The check evaluator fails closed: it requires every core check in `CORE_SUBMISSION_CHECKS` to be present, and every returned check to be an explicit `PASS`. Empty responses, `PENDING`, `ERROR`, missing core checks, and unrecognized statuses all fail. It is not version-aware, it does not capture the raw response, and the list of core checks is maintained by hand, so treat `alphafind check` as a local signal only and automate `alphafind submit` only after the platform gate is verified against the current platform response.
- `--skip-checks` is opt-in: it aborts unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` (or `true`) is set, and the run prints a bypass warning. Never make it the default, and never describe a submission as verified when it was used.
- Keep external submission as a deliberate operation after reviewing the current platform result. Simulation and submission POSTs are non-idempotent: ambiguous transport failures (timeouts, resets, truncated responses) now fail closed with a `fingerprint` for reconciliation instead of being retried. Connection failures, where nothing was sent, and explicit `429` responses are still retried. Do not add retries or resubmit after an ambiguous result without checking platform state first.

## Account concurrency and screening

- **Project policy**: When searching for or screening new candidate alphas, always run 3 concurrent simulation workers (`--workers 3`, matching `DEFAULT_WORKERS_PER_ACCOUNT = 3` in `src/config.rs`) on the primary account within a single CLI process.
- The screener consumes a bounded `mpsc` work queue and holds a per-account semaphore permit for each in-flight simulation, so `workers_per_account` (`3`) is the maximum in-flight simulations per account in that process. Because the CLI has no cross-process quota coordination and the per-dispatch sleep is politeness rather than a cross-process rate limiter, do not run multiple concurrent CLI screening processes at the same time or increase concurrency above `3` until cross-process rate limiting and platform-quota handling are in place.
- `transfer_lock` is a process-local mutex; it is not a lock across machines or CLI processes.
- Do not describe worker-account promotion as compliant unless the account holders and platform rules permit the workflow.

## Private research material and Git hygiene

Alpha expressions under evaluation, Alpha IDs, pending metrics, private PnL, credentials, and candidate metadata are confidential research material.

- Keep private candidate formulas and ledgers outside tracked files. Load them from a local, permission-restricted store excluded from Git. Public examples must use abstract placeholders such as `<SIGNAL>`, `<FIELD>`, `<GROUP>`, and `<PARAMETER>`.
- Do not put live IDs, pending formulas, or exact candidate metrics in source, tests, documentation, logs, branch names, commit messages, releases, or pull-request descriptions.
- Tracked source no longer carries candidate formulas: `src/taxonomy.rs` holds abstract placeholders and loads real candidates from the ignored local store. The remaining release blocker is history, not the working tree: earlier commits in `src/taxonomy.rs` (for example `f8f7b3a`, `c9268c0`, `9d4e3f7`) contain real field identifiers. Rewrite history before publishing, or keep the repository private and rotate the exposed fields.
- Downloaded field metadata (`data/fields_*.json`) is untracked and ignored: it names the exact fields under research. Keep those files local; the CLI degrades gracefully when they are absent.
- Before staging, inspect `git status`, the staged diff, ignore rules, and the complete tracked-file scan. Stage only named files; never blindly stage the whole tree.
- Run the sanitizer as one signal only. It scans tracked files under `src`, `tests`, `docs`, `data`, and `scripts` (excluding itself), checks added diff lines, scans every commit's added lines for Alpha IDs (the outgoing-diff range alone collapses to the last commit when no upstream is configured; set `ALPHAFIND_STRICT_HISTORY=1` to make historical hits fatal instead of a warning), and looks for candidate names/expressions loaded from `data/private_candidates.json`. Its explicit dataset-field patterns come from the ignored store `data/sanitize_field_patterns.txt` so the script itself never publishes those names; when that file, the private store, or the Alpha-ID sources are missing it says so instead of passing silently. It cannot detect a formula that was rewritten by hand. Inspect its coverage before relying on it.
- Exploratory scripts are disposable: put them in ignored `scratch/`, never in the repository root, and remove them when finished. Do not retain scratch output in Git.

## Documentation standards

- Date time-sensitive platform claims and link to official sources.
- Label recommendations and thresholds as internal policy or hypotheses unless an official source verifies them.
- Document actual code behavior and current limitations. Do not claim exact forecasts, all-user crowd data, fixed sample lengths, SIMD, bounded workers, or complete pagination unless the implementation proves it.
- Keep examples generic and non-sensitive. Do not include live candidate formulas or IDs.

## High-priority engineering backlog

1. Make check parsing version-aware and capture the raw response safely; the current evaluator is fail-closed but the expected check list is hardcoded.
2. Add an explicit local portfolio-policy gate (self-correlation, concentration, staleness) as a step separate from the platform check gate.
3. Purge or retire the Git history that contains real field identifiers, then re-run the sanitizer and review public history.
4. Add reconciliation for ambiguous simulation/submission requests (query platform state, then decide) plus cross-process rate limiting and platform-quota handling.
5. Report explicit active/OOS classification and refresh controls for every Alpha/PnL read; cache age is currently reported for the local PnL cache and field metadata only.
6. Add tests for correlation coverage and account policy boundaries before claiming production readiness.
