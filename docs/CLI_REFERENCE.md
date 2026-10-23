# CLI reference

This reference describes command interfaces in `src/main.rs` as inspected on 2026-10-12. Check `alphafind --help` for the binary you built. API behavior and BRAIN rules can change independently of this CLI.

## Commands

| Command | Purpose | Important limitation |
|---|---|---|
| `auth` | Authenticate configured BRAIN clients. | Logs may contain account identifiers or email addresses. |
| `sync` | Fetch portfolio metadata into the local portfolio file. | Fetch paths have pagination/classification and freshness limitations; verify against BRAIN. |
| `score` | Fetch profile/competition information. | Platform scoring formulas are not reproduced by this command. |
| `sim` | Create and poll one simulation on the main account. | Keep live expressions private. |
| `screen` | Simulate candidates from the local or fallback catalog. | Consumes a bounded `mpsc` queue with a per-account semaphore (`--workers` bounds in-flight simulations per account, not cross-process platform quota). Use `--single-account` (`ALPHAFIND_SINGLE_ACCOUNT=1`) to comply with single-account rules. |
| `tune` | Try configured parameter variants for one expression. | A higher in-sample metric is not evidence of better OOS performance. |
| `check` | Request platform submission check results. | Fail-closed locally: requires every core check present and every returned status explicitly `PASS`; `PENDING`, `ERROR`, unknown, missing, and empty responses reject. Not version-aware and does not capture the raw response, so BRAIN remains the authority. |
| `submit` | Send a submission request to BRAIN. | Runs the fail-closed platform check gate plus a separate local portfolio-policy gate (self-correlation `< 0.70` and staleness check). `--skip-checks` needs `ALPHAFIND_ALLOW_SKIP_CHECKS=1` and prints a bypass warning. An ambiguous POST outcome is reported as unknown and exits non-zero. |
| `audit` | Compare candidate PnL with locally available portfolio PnLs. | Reports active/OOS classification and cache age; supports `--refresh`. Fails closed when overlap `< 30` days. |
| `portfolio` | Calculate a local merged-PnL diagnostic. | Reports active/OOS classification and cache age; supports `--refresh`. Not BRAIN's official `isScore` calculation. |
| `impact` | Estimate local portfolio change after adding a candidate. | Depends on included alphas, weights, date coverage (`>= 100` days), and cache freshness (`--refresh`). |
| `radar` | Display dataset census fields returned by the API. | Census fields are time-sensitive; this is not a validated alpha uniqueness forecast. |
| `matrix` | Display configured dataset interaction heuristics. | Uses hard-coded affinity assumptions, not measured platform-wide alpha co-occurrence. |
| `blueprint` | Produce rule-based portfolio gap suggestions. | Computes actual merged Sharpe/correlation when overlap is sufficient (otherwise `N/A`); fixed archetypes are hypotheses, not validated forecasts. |
| `shadow` | Inspect live BRAIN dataset/field usage, local profile allocations, and optional local PnL sensitivity. | It does not predict competitor PnL or future official Uniqueness. |
| `health` | Print local portfolio decay, half-life, and marginal Sharpe diagnostics. | Reports active/OOS classification and cache age; supports `--refresh`. |
| `allocate` | Compute turnover Shannon entropy, HRP risk weights, retirement candidates, and fail-closed candidate admission (`Go`/`No-Go`). | Fails closed (`UNKNOWN` / `NoGo`) when PnL series are missing, flatline, or have `< 30` (`< 100` for merge) overlapping days; supports `--refresh`. |

## Usage synopsis

```text
alphafind auth
alphafind sync
alphafind score
alphafind sim --expr <PRIVATE_EXPRESSION> [--universe TOP1000] [--decay 5]
              [--neutralization SUBINDUSTRY] [--truncation 0.05]
alphafind screen [--pillar <PILLAR>] [--universe TOP1000]
                 [--min-fitness 1.50] [--workers 3] [--no-tune] [--single-account]
alphafind tune --expr <PRIVATE_EXPRESSION> [--universe TOP1000]
                [--target-fitness 1.50] [--decay 10]
                [--neutralization SUBINDUSTRY]
alphafind check <ALPHA_ID>
alphafind submit <ALPHA_ID> [--name <NAME>] [--category <CATEGORY>]
                  [--color <COLOR>] [--tags <TAG,...>] [--skip-checks]
alphafind audit <ALPHA_ID> [--label <LABEL>] [--refresh]
alphafind portfolio [--refresh]
alphafind impact <ALPHA_ID> [--refresh]
alphafind radar [--region USA] [--universe <UNIVERSE>] [--top 15]
                 [--filter <KEYWORD>] [--green-only]
alphafind matrix [--universe TOP200] [--expr <PRIVATE_EXPRESSION>]
                  [--green-only] [--region USA]
alphafind blueprint [--universe <UNIVERSE>] [--export <PATH>]
alphafind shadow [--candidate <ALPHA_ID> | --expr <PRIVATE_EXPRESSION>]
                 [--region USA] [--universe TOP1000] [--delay 1] [--simulate]
alphafind health [--alpha-id <ALPHA_ID>] [--export <PATH>] [--all] [--refresh]
alphafind allocate [--candidate <ALPHA_ID>] [--turnover <FLOAT>]
                   [--trim] [--export <PATH>] [--refresh]
```

`<PRIVATE_EXPRESSION>` is a placeholder, not literal Fast Expression syntax. Do not replace it with a pending candidate in tracked documentation or a public issue.

## Submission commands

### `check`

```bash
alphafind check <ALPHA_ID>
```

This calls the platform check endpoint and polls until no check is left `PENDING`. The local verdict is fail-closed:

- an empty response fails;
- every name in `CORE_SUBMISSION_CHECKS` must be present;
- every returned check must be an explicit `PASS`;
- `PENDING`, `ERROR`, `FAIL`, and unrecognized statuses all reject.

The report table shows each returned status by name, so a pending or errored check is never displayed as a failure or a pass. Two caveats: the expected core list is hardcoded rather than version-aware, and the raw platform response is not captured. Treat the signed-in BRAIN interface as the authority.

### `submit`

```bash
alphafind submit <ALPHA_ID> [--name <NAME>] [--category <CATEGORY>] \
  [--color <COLOR>] [--tags <TAG,...>] [--skip-checks]
```

This command sends an external submission request. It runs the same fail-closed check first and refuses to continue when the gate rejects.

`--skip-checks` is disabled by default and aborts unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` (or `true`) is set; when it is set, the run prints a conspicuous bypass warning and the success message states that pre-flight checks were bypassed.

The submission POST is non-idempotent. A transport failure is reported as an unknown outcome with instructions to verify the alpha stage in BRAIN, and the command exits non-zero instead of pretending the submission failed. Reconcile in the platform before retrying.

## Local portfolio diagnostics

`audit`, `portfolio`, `impact`, and `health` operate on fetched or cached PnL data. Review which alphas were included, active/OOS classification, cache age, date intersection, and number of observations. Insufficient overlap or missing data must be reported as unknown; it must not be interpreted as zero correlation.

Local portfolio correlation is distinct from BRAIN self-correlation and from competition `Uniqueness`. For current official score/submission rules, see [BRAIN Rules and Submission](BRAIN_RULES_AND_SUBMISSION.md).

## Research helpers

`shadow` reads aggregate `userCount`/`alphaCount` data from `/data-sets` and field usage from `/data-fields`; its local profile allocation is parsed from `portfolio_os.json`. Exact field-to-dataset mappings come from live/cache metadata or downloaded local metadata; consistent local prefixes may be used as an explicitly labeled inference. Field metadata may come from a cache labeled with its age. Higher usage means more observed platform participation, not that those users' PnLs are correlated. The platform does not expose competitors' formulas or merged PnL through these endpoints. Therefore this command cannot calculate or forecast future official `Uniqueness`.

When local PnL data is available, `shadow` can build a usage-weighted basket from the user's own allocated Alpha PnLs. For each local Alpha, it averages fresh mapped-field `alphaCount` values within each dataset, then averages across the Alpha's datasets; it falls back to dataset `alphaCount` where field counts are unavailable. Fresh field counts come from live responses or cache entries no older than 24 hours. It then compares that basket with an equal-weight merge of the same local profile. This is a local sensitivity diagnostic derived from the user's own series, not a crowd PnL estimate. `--simulate` with `--expr` runs one private simulation to obtain that expression's PnL for the local diagnostic. The command may display the current official Uniqueness returned by the account API, but it does not project a future score.

For a candidate formula, the command also reports mean field `alphaCount` and `userCount` as a share of the corresponding dataset counts when both are available. This estimates how commonly those fields are used within their datasets; it does not estimate PnL correlation.

`radar`, `matrix`, `blueprint`, and `shadow` offer research context but do not establish that a dataset is uncrowded, an alpha is unique, or an expression will pass. `matrix` and `blueprint` contain heuristic assumptions. Treat each output as a hypothesis to verify with current platform simulations and data.

## Account and data handling

- Use only an account controlled by the participant. Do not share credentials or account IDs.
- Keep expressions, Alpha IDs, private PnL, credentials, and pending performance out of tracked files and logs shared publicly.
- Keep local private files permission-restricted and excluded from version control.
