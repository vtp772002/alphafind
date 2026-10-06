# CLI Command Reference

AlphaFind provides a unified native binary covering the quantitative trading lifecycle: authentication, backtest simulation, distributed screening, correlation auditing, and official Out-of-Sample submission.

```text
Usage: alphafind <COMMAND>

Commands:
  auth       Authenticate all configured BRAIN accounts concurrently
  sync       Sync active Out-of-Sample (OS) portfolio into local cache
  portfolio  Simulate merged multi-alpha Out-of-Sample portfolio Sharpe and risk
  sim        Simulate an individual FastExpr formula backtest directly
  screen     Launch 9-worker distributed candidate screening across 3 accounts
  check      Verify the 8 mandatory In-Sample platform submission checks
  audit      Run exact 1,236-day Pearson correlation audit against active portfolio
  submit     Submit an Alpha to Out-of-Sample (OS) after 8/8 PASS verification
  help       Print this message or the help of the given subcommand(s)
```

---

## 1. `alphafind auth`

Verifies credentials in `.env` and tests concurrent session cookie generation across all configured accounts and worker sessions.

### Usage
```bash
alphafind auth
```

### Description
Tests connectivity, authenticates each account in parallel via `reqwest` with cookie jars, and displays account user IDs and worker concurrency readiness.

---

## 2. `alphafind sync`

Synchronizes your active Out-of-Sample (OS) portfolio directly from WorldQuant BRAIN and caches the metadata and PnL time series locally.

### Usage
```bash
alphafind sync
```

### Description
* Connects to your Main Account.
* Paginates through all active Out-of-Sample alphas.
* Saves expressions, IDs, settings, and metrics to `portfolio_os.json`.
* Displays a universe distribution breakdown and live leaderboard status.

---

## 3. `alphafind score`

Fetches and displays live user status, rank, and official WorldQuant BRAIN leaderboard metrics.

### Usage
```bash
alphafind score
```

### Metrics Displayed
* **User Profile**: User ID, Full Name, Email, and Consultant Tier (e.g., `GOLD`).
* **Active Competition**: Competition title (e.g., `Challenge - Vietnam`).
* **Leaderboard Rank**: Live position on the national leaderboard.
* **Total Score & Components**: Normalized Score, In-Sample Score (`isScore`), Uniqueness Score (`uniquenessScore`), and `daysOfSubmission` (rolling 60-day window).
* **Affiliation**: Associated university or institution.
* **Active Portfolio Snapshot**: Number of tracked Out-of-Sample Alphas.

---

## 3. `alphafind portfolio`

Simulates equal-weighted multi-alpha portfolio performance over 1,236 common backtest trading days.

### Usage
```bash
alphafind portfolio
```

### Metrics Computed
* **Common Backtest Trading Days**: Intersection of all historical trading dates (typically 1,236 days).
* **Annualized Merged PnL**: Combined multi-alpha dollar PnL.
* **Annualized Merged Volatility**: Portfolio standard deviation factoring in cross-covariance.
* **Merged Portfolio Sharpe Ratio**: Multi-alpha Sharpe ratio reflecting true diversification.
* **Average Pairwise Correlation ($\overline{\rho}$)**: Cross-correlation across all active alphas.

---

## 4. `alphafind sim`

Dispatches an individual FASTEXPR formula backtest directly to the BRAIN simulation API and polls for results.

### Usage
```bash
alphafind sim [OPTIONS] --expr <EXPRESSION>
```

### Options
| Flag | Short | Default | Description |
|:---|:---:|:---:|:---|
| `--expr <STR>` | `-e` | *Required* | FASTEXPR Alpha expression to simulate |
| `--universe <U>` | `-u` | `TOP1000` | Equity universe (`TOP3000`, `TOP1000`, `TOP500`, `TOP200`) |
| `--decay <N>` | `-d` | `5` | Linear decay factor in trading days |
| `--neutralization <N>` | `-n` | `SUBINDUSTRY` | Neutralization regime (`MARKET`, `SECTOR`, `INDUSTRY`, `SUBINDUSTRY`) |
| `--truncation <F>` | `-t` | `0.05` | Maximum single-stock position weight truncation |

### Example
```bash
alphafind sim \
  --expr "term_slope = ts_backfill(implied_volatility_call_90 / implied_volatility_call_30, 5); group_neutralize(signed_power(rank(ts_decay_linear(term_slope, 14)) - 0.5, 4.4), densify(market))" \
  --universe TOP1000 \
  --decay 32 \
  --neutralization MARKET \
  --truncation 0.065
```

---

## 5. `alphafind screen`

Launches the 9-worker distributed screening engine across 3 parallel accounts with automated parameter tuning and atomic transfer lock.

### Usage
```bash
alphafind screen [OPTIONS]
```

### Options
| Flag | Short | Default | Description |
|:---|:---:|:---:|:---|
| `--pillar <PILLAR>` | `-p` | *Optional* | Factor pillar: `OPTIONS`, `ANALYST`, `MICRO`, `QUAL`, `RISK`, `SHORT` |
| `--universe <U>` | `-u` | `TOP1000` | Equity universe (`TOP3000`, `TOP1000`, `TOP500`, `TOP200`) |
| `--min-fitness <F>` | `-m` | `1.50` | Minimum Fitness threshold to trigger transfer lock & verification |
| `--workers <N>` | `-w` | `3` | Workers per account (total = workers × accounts) |
| `--no-tune` | | `false` | Disable automatic decay parameter sweeps |

### Example
```bash
# Screen Options Volatility on TOP1000:
alphafind screen --pillar OPTIONS --universe TOP1000 --min-fitness 1.50

# Screen Analyst Consensus on TOP500 with SECTOR neutralization:
alphafind screen --pillar ANALYST --universe TOP500 --min-fitness 1.40
```

---

## 6. `alphafind check`

Verifies candidate alpha against all **8 mandatory In-Sample platform submission checks** using official platform endpoints (`GET /alphas/{id}/check`).

### Usage
```bash
alphafind check <ALPHA_ID>
```

### Platform Checks Verified
1. `LOW_SHARPE` ($\ge 1.25$)
2. `LOW_FITNESS` ($\ge 1.00$)
3. `LOW_TURNOVER` ($\ge 1.0\%$)
4. `HIGH_TURNOVER` ($\le 70.0\%$)
5. `CONCENTRATED_WEIGHT` ($\le 0.08$ truncation)
6. `LOW_SUB_UNIVERSE_SHARPE` (passed across sub-tiers)
7. `SELF_CORRELATION` ($\le 0.70$ or Sharpe improvement)
8. `MATCHES_COMPETITION` (USA, Equity, Delay 1)

---

## 7. `alphafind audit`

Computes microsecond Pearson correlation between candidate alpha and all active portfolio alphas over 1,236 common backtest days.

### Usage
```bash
alphafind audit <ALPHA_ID>
```

### Output
* Total matching trading days.
* Maximum pairwise correlation against any single active alpha.
* Average portfolio pairwise correlation ($\overline{\rho}$).
* Compliance flag against platform limit ($\le 0.70$) and institutional target ($\le 0.15$).

---

## 8. `alphafind submit`

Submits a verified 8/8 PASS Alpha to Out-of-Sample (OS) forward tracking with standardized taxonomy naming and tags.

### Usage
```bash
alphafind submit [OPTIONS] <ALPHA_ID>
```

### Options
| Flag | Short | Description |
|:---|:---:|:---|
| `<ALPHA_ID>` | | Alpha ID to submit |
| `--name <NAME>` | | Institutional Alpha name |
| `--category <CAT>` | | Alpha category (e.g., `FUNDAMENTAL`, `PRICE_VOLUME`) |
| `--color <COLOR>` | | Color classification (`BLUE`, `GREEN`, `PURPLE`, `ORANGE`, `RED`) |
| `--tags <TAGS>` | | Comma-separated category tags |
| `--skip-checks` | | Skip pre-flight 8/8 check verification (not recommended) |

### Example
```bash
alphafind submit <ALPHA_ID> \
  --name "US_D1_TOP1000_OPT_TermSlope_VRP_Dec32" \
  --color PURPLE \
  --tags "OPTIONS,VOLATILITY,TOP1000,VRP"
```

---

## 9. `alphafind impact`

Simulates the exact equal-weighted portfolio merge impact of adding a candidate alpha to the active 42-alpha Out-of-Sample portfolio over 1,236 common backtest days.

### Usage
```bash
alphafind impact <ALPHA_ID>
```

### Output
* Delta Merged Portfolio Sharpe ($\Delta \text{Sharpe}$).
* Delta Annualized PnL ($\Delta \text{PnL}$).
* Delta Annualized Volatility ($\Delta \text{Vol}$) and variance collapse indicator.
* Delta Average Pairwise Correlation ($\Delta \overline{\rho}$).
* Maximum Pairwise Correlation and direct OS average correlation.
* **Safety Buffer to 70% Limit**: Distance $(70.0\% - \max \rho)$ preventing future self-correlation violations.
* Automated Recommendation verdict: `🌟 [RECOMMENDED]`, `🟡 [CAUTION]`, or `❌ [UNSUBMITTABLE]`.

---

## 10. `alphafind radar`

Live Dynamic Crowd Census Radar querying all 150 datasets on WorldQuant BRAIN Production API to track factor crowding and quant migration velocity.

### Usage
```bash
alphafind radar [OPTIONS]
```

### Options
| Flag | Short | Description | Default |
|:---|:---:|:---|:---:|
| `--region <REGION>` | `-r` | Target market region | `USA` |
| `--top <NUM>` | `-t` | Number of ranked datasets to display | `15` |
| `--filter <KEYWORD>` | `-f` | Filter by dataset name or subcategory | None |
| `--green-only` | | Show only Green Sanctuaries (`users < 300`) | `false` |

### Examples
```bash
# General live crowd radar across all USA datasets
alphafind radar

# Monitor News and Sentiment packages
alphafind radar --filter news

# View only uncrowded Green Sanctuaries
alphafind radar --green-only
```
