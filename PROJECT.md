# Project: Quantitative Alpha Research Framework (WorldQuant BRAIN Alternative Data)

## Architecture
The quantitative alpha research framework expands `alphafind` to systematically discover, specify, validate, generate, and evaluate quantitative alphas based on WorldQuant BRAIN alternative datasets:
- **`model51` (Systematic Risk Metrics)**: 16 matrix fields measuring multi-horizon market beta, benchmark correlation, systematic variance ($R^2$), and unsystematic idiosyncratic risk ($1-R^2$).
- **`model16` (Fundamental Scores & Derivative Accel)**: 24 matrix fields, featuring 8 rank derivatives with 100% cross-sectional coverage (multi-factor acceleration, cashflow efficiency, analyst revision, earnings certainty) and 16 style pentagon fields.
- **`news18` (RavenPack News Data)**: 121 fields under Delay 1 (71 daily matrix sentiment means + 50 intraday vector event streams requiring `vec_avg`/`vec_sum` folding and `ts_backfill`).
- **`option9` (Options Analytics)**: 74 matrix fields across 12 constant expiration horizons (10d to 1080d) capturing put-call ratios (volume and open interest), synthetic forward prices, and breakeven asymmetry.

### High-Level Data & Execution Flow
```
[BRAIN API /data-fields] ──> [Field Metadata Extractor & Schema Validator (M2)]
                                     │ (validated field schemas & types)
                                     ▼
[Factor Taxonomy (M1)] ────> [Automated Expression Builder & Combinator (M3)]
                                     │ (syntactically valid FastExpr formulas)
                                     ▼
[FastExpr Syntax Validator] ─> [Simulation Dispatcher (max_workers=3) (M4)]
                                     │ (simulation execution & stats)
                                     ▼
[Fail-Closed Check Gate] ───> [Portfolio Baseline Impact Evaluator (M4)]
                                     │ (Pearson rho < 70%, delta_sharpe > 0)
                                     ▼
                           [Qualified OS Candidate Alpha]
```

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Mathematical Factor Taxonomy (`model51`) | Mathematical specifications, economic hypotheses, and failure modes for systematic/unsystematic risk metrics | M1 | Survey (R1) |
| 2 | Mathematical Factor Taxonomy (`model16`) | Mathematical specifications, economic hypotheses, and failure modes for fundamental rank derivatives and pentagon scores | M1 | Survey (R1) |
| 3 | Mathematical Factor Taxonomy (`news18`) | Mathematical specifications, economic hypotheses, vector folding rules, and failure modes for RavenPack news sentiment | M1 | Survey (R1) |
| 4 | Mathematical Factor Taxonomy (`option9`) | Mathematical specifications, economic hypotheses, term skew, and failure modes for options analytics surface | M1 | Survey (R1) |
| 5 | Field Metadata Extractor | Paginated query and extraction tool for BRAIN `/data-fields` with required query parameters (`instrumentType=EQUITY`, `universe=...`) | M2 | Survey (R2) |
| 6 | Field Schema & Type Validator | Parser distinguishing `MATRIX` vs `VECTOR` types, historical coverage, and FastExpr compatibility checks | M2 | Survey (R2) |
| 7 | FastExpr Syntax Validator | Pre-dispatch bracket, syntax, and operator validator preventing HTTP 400 simulation errors | M3 | Survey (R3) |
| 8 | Automated Factor Combinator | Layered generator combining core alternative signals with non-linear transforms (`signed_power`, `rank`, `winsorize`, `ts_decay_linear`) | M3 | Survey (R3) |
| 9 | Group Neutralization & Densification | Group neutralization across `SUBINDUSTRY`, `INDUSTRY`, `SECTOR` with mandatory `densify()` wrapper | M3 | Survey (R3) |
| 10 | Reversal & Liquidity Dampeners | Turnover stabilizers and short-term reversal dampeners suppressing high-frequency noise | M3 | Survey (R3) |
| 11 | Simulation Dispatcher (`max_workers=3`) | Concurrency-bounded simulation runner enforcing strict 3-worker limit per account per `AGENTS.md` | M4 | Survey (R4) |
| 12 | In-Sample Metric Evaluator | Gate checking Sharpe >= 1.25, Fitness >= 1.00, Turnover 1.0%-70.0%, Returns > 8.0%, and fail-closed submission checks | M4 | Survey (R4) |
| 13 | Portfolio Correlation & Impact Auditor | Pearson correlation ($\max \rho < 70.0\%$) and accretive portfolio merge impact ($\Delta \text{Sharpe} > 0, \Delta \text{Vol} \le 0$) against 84 OS alphas | M4 | Survey (R4) |
| 14 | E2E Alternative Alpha Screening & Simulation | Execution of simulations for >= 3 distinct alternative hypotheses, qualifying >= 1 candidate alpha | M4 | Survey (R4) |
| 15 | E2E Integration & Verification Test Suite | Full programmatic test suite and pre-push sanitizer verification | M5 | Survey (Acceptance) |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Mathematical Factor Taxonomy & Specification Module | Design and document complete factor taxonomy (`docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`) covering `model51`, `model16`, `news18`, `option9` | none | DONE |
| M2 | Field Metadata Extraction & Schema Validator | Implement `/data-fields` extraction, schema parsing (`MATRIX` vs `VECTOR`), coverage metrics, and validation tool | none | DONE |
| M3 | Automated Expression Builder & FastExpr Syntax Validator | Implement factor combinator engine with non-linear transforms, group neutralizations, dampeners, and syntax validator | M1, M2 | DONE |
| M4 | Verification & Portfolio Impact Evaluator & Simulations | Implement evaluation pipeline (`max_workers=3`), simulate >= 3 alternative hypotheses, qualify candidate alpha, evaluate portfolio merge | M3 | DONE |
| M5 | Final E2E Test Suite, Challenger & Forensic Audit Verification | Execute complete test suite, run adversarial challenger, perform forensic integrity audit, verify Git hygiene | M4 | DONE |


## Interface Contracts

### M1 ↔ M3: Taxonomy & Specifications to Expression Builder
- Factor specification output defines formal parameter signatures:
  - `core_signal`: string representation of field or folded vector (e.g., `vec_avg(nws18_ssc)`, `beta_last_60d_spy`, `multi_factor_acceleration_score_derivative`, `pcr_vol_20d - pcr_vol_90d`).
  - `transforms`: sequence of applied functions (`rank`, `signed_power(..., 2.0)`, `winsorize(..., std=3)`).
  - `group`: grouping field (`subindustry`, `industry`, `sector`) wrapped in `densify()`.
  - `dampener`: `ts_decay_linear(..., d)` with decay window $d \in [5, 20]$.

### M2 ↔ M3: Field Metadata & Schema Validator to Expression Builder
- Metadata representation:
  ```json
  {
    "id": "string",
    "dataset": "string",
    "type": "MATRIX" | "VECTOR",
    "coverage": float,
    "dateCoverage": float,
    "userCount": int,
    "alphaCount": int
  }
  ```
- Schema validator rule:
  - If `type == "VECTOR"`, expression MUST wrap in `vec_avg(...)` or `vec_sum(...)` before applying scalar/matrix operators.
  - If `coverage < 0.50`, expression MUST apply `ts_backfill(..., d)`.

### M3 ↔ M4: Expression Builder to Evaluator & Simulator
- Input to simulation dispatcher:
  ```json
  {
    "name": "string",
    "hypothesis": "string",
    "dataset": "model51" | "model16" | "news18" | "option9",
    "regular": "FastExpr string (syntactically validated)",
    "settings": {
      "instrumentType": "EQUITY",
      "region": "USA",
      "universe": "TOP1000",
      "delay": 1,
      "decay": 5,
      "neutralization": "SUBINDUSTRY",
      "truncation": 0.05,
      "pasteurization": "ON",
      "unitHandling": "VERIFY",
      "nanHandling": "OFF",
      "language": "FASTEXPR"
    }
  }
  ```

### M4 ↔ M5: Evaluator Output to Final Verification & Portfolio Baseline
- Simulation result metrics:
  - `is_sharpe`: float ($\ge 1.25$)
  - `is_fitness`: float ($\ge 1.00$)
  - `is_turnover`: float ($0.01 \le \text{turnover} \le 0.70$)
  - `is_returns`: float ($> 0.08$)
  - `checks`: list of check statuses (all 6 core checks MUST be `PASS`)
  - `max_rho`: float ($< 0.70$ against 84 active OS alphas)
  - `delta_sharpe`: float ($> 0$)
  - `delta_vol`: float ($\le 0$)

## Code Layout
- `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`: Mathematical factor taxonomy, economic hypotheses, and failure modes (M1).
- `src/models.rs`: Core data structures updated with `type: MATRIX | VECTOR` and schema validation (M2).
- `src/client.rs` / `src/commands/`: Metadata extraction and validation commands (M2).
- `src/expression_builder.rs` / `scratch/expression_builder.py`: Modular expression generation and FastExpr syntax validator (M3).
- `src/screener.rs` / `scratch/evaluate_alternative_alphas.py`: Verification and portfolio impact evaluator enforcing `max_workers=3` (M4).
- `tests/`: Automated unit and integration test suites for metadata parsing, syntax validation, and correlation evaluation.
- `portfolio_os.json` & `~/.cache/alphafind/portfolio_pnl.json`: Portfolio baseline data.
- `scripts/sanitize_check.sh`: Pre-push Git hygiene and research material leak gatekeeper.
