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

## Submission safety and 5-Gate automated submission policy

- The official BRAIN UI/API is the authority for current checks and OOS eligibility. AlphaFind output is not approval.
- The check evaluator fails closed: it requires every core check in `CORE_SUBMISSION_CHECKS` to be present, and every returned check to be an explicit `PASS`. Empty responses, `PENDING`, `ERROR`, missing core checks, and unrecognized statuses all fail. It is not version-aware, it does not capture the raw response, and the list of core checks is maintained by hand, so treat `alphafind check` as a local signal only and automate `alphafind submit` only after the platform gate is verified against the current platform response.
- `--skip-checks` is opt-in: it aborts unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` (or `true`) is set, and the run prints a bypass warning. Never make it the default, and never describe a submission as verified when it was used.
- Simulation and submission POSTs are non-idempotent: ambiguous transport failures (timeouts, resets, truncated responses) fail closed with a `fingerprint` for reconciliation instead of being retried. Connection failures, where nothing was sent, and explicit `429` responses are retried. Do not add retries or resubmit after an ambiguous result without checking platform state first.

### 5-Gate Alpha Research & Automated Submission Pipeline

To eliminate brute-force parameter mining and control overfitting, candidate alpha research must follow an idea-first quantitative workflow evaluated sequentially against 5 rigorous gates. **Automated submission (`alphafind submit <ID>`) is authorized if and only if a candidate passes all 5 gates**:

1. **Gate 1: Platform Pre-flight Checks (Official BRAIN Gate)**
   - Must return 100% explicit `PASS` on all core checks queried from the official platform check endpoint: `LOW_SHARPE` (>= 1.25), `LOW_FITNESS` (>= 1.00), `TURNOVER` within bounds (0.01 to 0.70), `LOW_SUB_UNIVERSE_SHARPE`, `CONCENTRATED_WEIGHT`, `MATCHES_COMPETITION`, and platform `SELF_CORRELATION` (< 0.70). Any pending, failed, or missing check halts the pipeline.
2. **Gate 2: Held-out Test Period Gate (Local Out-of-Sample Guardrail)**
   - Evaluated via `alphafind test-period <ID>`.
   - The newest 2 years of the daily PnL series is partitioned as a held-out test block.
   - Requirements: Train Sharpe >= 0.50, Test Sharpe > 0, and Retention (Test Sharpe / Train Sharpe) >= 50% on at least 1.5 years of test observations.
3. **Gate 3: Local Portfolio-Policy Gate (Internal Self-Correlation & Staleness)**
   - Evaluated via `alphafind audit <ID>`.
   - Daily PnL pairwise Pearson correlation against every active OS Alpha in `portfolio_os.json` across common overlapping dates (>= 100 days).
   - Maximum pairwise correlation must strictly satisfy `max_rho < 0.70` (fail-closed local check).
4. **Gate 4: Portfolio Merge Impact Gate (Marginal Value-Add)**
   - Evaluated via `alphafind impact <ID>`.
   - Adding the candidate into the equal-weighted portfolio must produce a non-negative marginal impact (`delta_sharpe >= 0.0`), preserve or collapse portfolio volatility (`delta_vol <= 0.0` or compensated by return), and maintain an adequate safety buffer to the correlation trần.
5. **Gate 5: Overfitting Robustness & Research Hygiene Gate**
   - **Hypothesis-first restriction**: Candidates must be formulated with explicit financial rationales; blind grid parameter tuning is forbidden (`--no-tune`).
   - **Robustness checks**:
     - *Rank Test*: Converting the final signal into a pure rank cross-section retains a positive Sharpe.
     - *Binary Test*: Quantizing the signal to {-1, +1} preserves positive return direction.
     - *Universe Invariance*: Preserves stability and profitability when evaluated on a larger super-universe (e.g., TOP1000).
   - **Research Hygiene**: Zero formula or identifier leakage into Git; private expressions stored in ignored local stores (`data/private_candidates_*.json`).

When all 5 gates pass, automated submission via `alphafind submit <ID>` proceeds without requiring manual interactive confirmation. The worked example below applies this loop end to end.

### Worked example: one end-to-end research round

An illustrative reconstruction of a single successful round, included so a future
contributor can repeat the shape of the work without re-deriving it. It is deliberately
abstract: every expression, identifier, and metric appears as a placeholder. The real
values stay in the ignored local stores described under *Private research material and
Git hygiene*. Nothing here is a claim that the same universe, dataset, or weight will
work again.

**0. Establish the baseline before thinking about a new alpha.**
Run `alphafind sync`, then recompute dataset usage across the whole active portfolio
(field → dataset map joined against every active OS expression). The output is a
saturation census: which datasets carry many alphas, which carry one or two. Treat a
dataset that already underpins a large share of the book as a correlation hazard, not an
opportunity, because a new expression built from it lands close to existing PnL and is
likely to fail Gate 1 `SELF_CORRELATION` — this is observed behaviour, not a hypothesis:
a strong high-Sharpe candidate from a saturated cluster was rejected at
`SELF_CORRELATION` ≈ 1.00 against an existing alpha.

**1. Pick a genuinely under-used data channel, and give it an economic reason.**
Select from the low-usage side of the census, and state the economic mechanism before
writing any expression (`<MECHANISM>`). Examples of channel *categories* that were
locally under-used at the time: share-supply and buyback activity, footnote-level
accounting accruals, event-sentiment/news-reaction, and composite style/quality scores.
Also mine the local simulation ledger (`~/.cache/alphafind/sim_history.jsonl`) before
simulating: it records every prior attempt with expression, settings, and metrics, so it
both prevents re-treading old ground and shows which primitives have already been proven
weak on their own.

**2. Draft three ideas, not thirty.**
Write one private round file per batch, e.g. `data/private_candidates_<run>_round<N>.json`,
holding: round number, universe, neutralization, decay, truncation, a hypothesis theme,
per-idea rationale, and each idea's name and expression. Three ideas per batch matches
the `--workers 3` project policy and keeps each round attributable to one hypothesis. A
typical batch spans one primary channel plus one orthogonal confirmation channel, for
example `<NEWS_REACTION_FIELD>` (behavioural underreaction) combined with
`<FOOTNOTE_ACCRUAL_FIELD>` (accounting quality) and `<NARRATIVE_FIELD>` (attention).

**3. Validate the schema before spending simulations.**
```
alphafind fields --dataset <DATASET> --universe <UNIVERSE> --validate "<EXPRESSION>"
```
This catches unknown fields, wrong operator arity, and low-coverage warnings for a batch
of one. Two concrete constraints were learned from platform API errors rather than from
documentation (observed 2026-11; re-verify before relying on them):
`winsorize` takes exactly one argument, so `winsorize(x, <N>)` is rejected; and
`vec_avg` applies only to 3D event-vector fields, so it must not be wrapped around a
plain matrix field. Prefer fields with high cross-coverage when the
`LOW_SUB_UNIVERSE_SHARPE` check matters, since sparse fields shrink the scored breadth on
smaller universes.

**4. Simulate the batch.**
```
alphafind sim --batch scratch/batch_<N>.txt --universe <UNIVERSE> --decay <DECAY> \
  --neutralization <GROUP_NEUT> --truncation <TRUNC> --workers 3
```
Read the summary table as a screen, not a verdict. A round where every idea scores far
below the Sharpe and Fitness floors is a failed *hypothesis*, so record the failure and
change the hypothesis; do not start tuning the failing formulas.

**5. Fix breadth before fixing parameters.**
The single highest-leverage lesson from this round: a composite whose sparse components
produce `NaN` for most names silently drops those names, and the loss shows up as a
`LOW_SUB_UNIVERSE_SHARPE` failure even when full-universe Sharpe looks healthy. Wrapping
each sparse component as `if_else(is_nan(<COMPONENT>), 0, <TRANSFORM>)` keeps the stock in
the cross-section with a neutral contribution. In this round that data-handling change
alone — with identical economic weights and identical constants — moved the candidate from
failing Gate 1 to clearing it comfortably. This is a robustness fix, not fitting: nothing
was selected by scanning a parameter grid.

**6. Evaluate the gates in escalating cost order.**
- Gate 1 first, on the single best candidate only: `alphafind check <ID>`. It is the
  cheapest disqualifier, since weak `LOW_SHARPE`/`LOW_FITNESS`, a sub-universe failure,
  or `SELF_CORRELATION` ≈ 1.00 ends the candidate immediately. Re-run until
  `SELF_CORRELATION` and the sub-universe check are resolved rather than `PENDING`; a
  pending check is not a pass.
- Gate 2: `alphafind test-period <ID>`.
- Gate 3: `alphafind audit <ID>` (needs a fresh `portfolio_os.json`; check the reported
  snapshot and cache ages, and treat missing overlap as unknown rather than as zero
  correlation).
- Gate 4: `alphafind impact <ID>`. Small deltas are normal for a single addition to a
  large book; what matters is the sign, the volatility direction, and the correlation
  buffer.
- Gate 5: `alphafind sweep` on the one constant that carries a real economic
  interpretation, plus the rank / binary / larger-universe probes.

**7. Submit only after all five pass, then re-sync.**
```
alphafind submit <ID> --name "<NAME>" --category <CATEGORY> --color <COLOR> --tags <TAGS>
alphafind sync && alphafind portfolio
```
Report the post-submission portfolio state (active count, universe mix, merged Sharpe,
average pairwise correlation) rather than only the single-alpha metrics.

**8. Close the round with hygiene.**
Append the outcome and the gate evidence to the private ledger, delete the throwaway
`scratch/batch_*.txt` files, and confirm with `git status` that no private store, field
metadata file, or scratch artifact became tracked. A rejected idea is worth recording
too: the ledger's value comes from the failures it prevents.

**Honest limitations of this recipe.**

- A flat sensitivity sweep carries almost no information. If five neighbouring values of
  a constant return bit-identical metrics, the constant is not driving the result — which
  is good for overfitting risk but bad as evidence. Confirm the component actually
  contributes with an *ablation* (drop the block, re-simulate) and report the delta.
- A larger-universe invariance run can exceed the CLI's client-side poll limit on a heavy
  expression. That is a transport timeout, not a platform rejection; the simulation may
  still finish server-side, so re-check before resubmitting, and never treat a timed-out
  probe as a pass.
- Gates 1–4 are local signals built on platform and cached local data. Only the platform
  UI/API decides OOS eligibility, and an out-of-sample result requires weeks of live
  tracking. Do not describe a submitted alpha as validated.
- The dataset census, dataset "under-use", and any weight or horizon choice above are
  research hypotheses at the time of writing. Recompute the census each run; stale
  saturation numbers will point you at the wrong channel.

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
