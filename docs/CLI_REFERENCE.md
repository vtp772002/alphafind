# CLI reference

This reference describes command interfaces in `src/main.rs` as inspected on 2026-10-12. Check `alphafind --help` for the binary you built. API behavior and BRAIN rules can change independently of this CLI.

## Commands

| Command | Purpose | Important limitation |
|---|---|---|
| `auth` | Authenticate configured BRAIN clients. | Logs may contain account identifiers or email addresses. |
| `sync` | Fetch portfolio metadata into the local portfolio file. | Fetch paths have pagination/classification and freshness limitations; verify against BRAIN. |
| `score` | Fetch profile/competition information. | Platform scoring formulas are not reproduced by this command. |
| `sim` | Create and poll one simulation on the main account. | Keep live expressions private. |
| `screen` | Simulate candidates from the in-code catalog. | Tasks are spawned per candidate; `--workers` does not currently bound concurrency. Multi-account use must comply with BRAIN's account rules. |
| `tune` | Try configured parameter variants for one expression. | A higher in-sample metric is not evidence of better OOS performance. |
| `check` | Request platform submission check results. | The parser treats only explicit `FAIL` as failure; `PENDING`, `ERROR`, unknown, or missing checks can be misreported as PASS. Do not rely on it as an approval gate. |
| `submit` | Send a submission request to BRAIN. | Has `--skip-checks`; use BRAIN's own status as authority. |
| `audit` | Compare candidate PnL with locally available portfolio PnLs. | Depends on date overlap and possibly stale cached series. |
| `portfolio` | Calculate a local merged-PnL diagnostic. | Not BRAIN's official `isScore` calculation. |
| `impact` | Estimate local portfolio change after adding a candidate. | Depends on included alphas, weights, date coverage, and cache freshness. |
| `radar` | Display dataset census fields returned by the API. | Census fields are time-sensitive; this is not a validated alpha uniqueness forecast. |
| `matrix` | Display configured dataset interaction heuristics. | Uses hard-coded affinity assumptions, not measured platform-wide alpha co-occurrence. |
| `blueprint` | Produce rule-based portfolio gap suggestions. | Fixed archetypes are hypotheses, not statistically validated forecasts. |
| `shadow` | Inspect live BRAIN dataset/field usage, local profile allocations, and optional local PnL sensitivity. | It does not predict competitor PnL or future official Uniqueness. |
| `health` | Print local portfolio diagnostics. | Results inherit portfolio classification, data coverage, and cache limitations. |

## Usage synopsis

```text
alphafind auth
alphafind sync
alphafind score
alphafind sim --expr <PRIVATE_EXPRESSION> [--universe TOP1000] [--decay 5]
              [--neutralization SUBINDUSTRY] [--truncation 0.05]
alphafind screen [--pillar <PILLAR>] [--universe TOP1000]
                 [--min-fitness 1.50] [--workers 3] [--no-tune]
alphafind tune --expr <PRIVATE_EXPRESSION> [--universe TOP1000]
                [--target-fitness 1.50] [--decay 10]
                [--neutralization SUBINDUSTRY]
alphafind check <ALPHA_ID>
alphafind submit <ALPHA_ID> [--name <NAME>] [--category <CATEGORY>]
                  [--color <COLOR>] [--tags <TAG,...>] [--skip-checks]
alphafind audit <ALPHA_ID> [--label <LABEL>]
alphafind portfolio
alphafind impact <ALPHA_ID>
alphafind radar [--region USA] [--universe <UNIVERSE>] [--top 15]
                 [--filter <KEYWORD>] [--green-only]
alphafind matrix [--universe TOP200] [--expr <PRIVATE_EXPRESSION>]
                  [--green-only] [--region USA]
alphafind blueprint [--universe <UNIVERSE>] [--export <PATH>]
alphafind shadow [--candidate <ALPHA_ID> | --expr <PRIVATE_EXPRESSION>]
                 [--region USA] [--universe TOP1000] [--delay 1] [--simulate]
alphafind health [--alpha-id <ALPHA_ID>] [--export <PATH>] [--all]
```

`<PRIVATE_EXPRESSION>` is a placeholder, not literal Fast Expression syntax. Do not replace it with a pending candidate in tracked documentation or a public issue.

## Submission commands

### `check`

```bash
alphafind check <ALPHA_ID>
```

This calls the platform check endpoint. A local `PASS` should only be trusted after the client verifies that the response contains every expected check, every status is explicitly passing, and the payload belongs to the requested alpha and current settings. The current parser does not enforce those conditions reliably. Review the response in BRAIN.

### `submit`

```bash
alphafind submit <ALPHA_ID> [--name <NAME>] [--category <CATEGORY>] \
  [--color <COLOR>] [--tags <TAG,...>] [--skip-checks]
```

This command can send a submission request. `--skip-checks` bypasses local preflight. Since the local preflight is incomplete, use the signed-in platform UI/API as the source of truth, and do not automate this command until the gate is fixed and covered by tests.

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
