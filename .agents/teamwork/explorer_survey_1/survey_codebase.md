# AlphaFind Codebase Architectural Survey

**Date**: 2026-10-24  
**Author**: Codebase Architect Explorer (Survey 1)  
**Target Repository**: `/Users/phamtoan/Developer/alphafind`  
**Integrity Mode**: Development / Quantitative Alpha Research Framework  

---

## 1. Executive Summary

This report delivers an exhaustive architectural survey of the `alphafind` repository to establish the engineering foundation for the four quantitative alpha research requirements (R1: Factor Taxonomy, R2: Field Metadata & Schema Validator, R3: Expression Builder & Combinator, R4: Verification & Evaluator) while strictly complying with the platform rules and Git hygiene standards documented in `AGENTS.md`.

### Core Strengths of the Existing Codebase
- **Robust Rust Core (v1.9.0)**: Production-grade asynchronous architecture using `tokio`, `reqwest`, `clap`, and `serde`.
- **Strict Quantitative Verification & Fail-Closed Guards**: Peer-reviewed Pearson correlation engine (`src/correlation.rs`), portfolio merge simulations, delay-aware threshold partitions (`src/config.rs`), and fail-closed submission check evaluations (`evaluate_submission_checks`).
- **Bounded Concurrency & Platform Politeness**: Screener (`src/screener.rs`) strictly respects per-account semaphore limits (`workers_per_account = 3`), non-idempotent POST safety (`classify_transport_error`), and 64-bit FNV-1a request fingerprinting.
- **Strict Research Hygiene & Sanitization**: Gitignore partitions private candidate files, local caches, and scratch scripts. A pre-push gatekeeper script (`scripts/sanitize_check.sh`) prevents leaks of real Alpha IDs, private candidate expressions, and proprietary field patterns.

### Key Architectural Gaps for R1–R4
- **No Formal Expression AST or Syntax Validator (R3 Gap)**: FastExpr strings are currently treated as opaque strings or handled via rudimentary token-based lexical filtering (`src/crowd_allocation.rs`). No syntax validation (bracket pairing, arity checking, operator verification) exists before API dispatch.
- **Field Metadata Incompleteness (R2 Gap)**: `DataFieldEntry` in `src/models.rs` lacks critical fields (`type` like MATRIX vs VECTOR, `category`, `subcategory`, `dateCreated`). Bulk querying by dataset (`dataset=model51`, etc.) is missing from `src/client.rs`.
- **Absence of Dedicated Math Taxonomy for New Alternative Datasets (R1 Gap)**: Current `FactorPillar` covers legacy pillars, but lacks formal mathematical specifications, economic rationales, rolling horizon derivations, and failure modes for `model51` (Systematic Risk), `model16` (Fundamental Scores), `news18` (Ravenpack News), and `option9` (Options Analytics).
- **Test Isolation Sensitivity**: When `data/private_candidates.json` is populated locally, `tests/taxonomy_tests.rs` fails unless isolated via `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json`.

---

## 2. Cargo.toml & Project Layout

### 2.1 Dependencies (`Cargo.toml`)
```toml
[package]
name = "alphafind"
version = "1.9.0"
edition = "2021"
authors = ["AlphaFind Quant Team"]
description = "High-Performance Quantitative Alpha Mining Engine for WorldQuant BRAIN"

[dependencies]
tokio = { version = "1.40", features = ["full"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "cookies", "rustls-tls", "gzip"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
csv = "1.3"
clap = { version = "4.5", features = ["derive"] }
dotenvy = "0.15"
colored = "2.1"
anyhow = "1.0"
base64 = "0.22"

[dev-dependencies]
wiremock = "0.6"
```

### 2.2 Directory Structure
```
alphafind/
├── Cargo.lock
├── Cargo.toml
├── AGENTS.md                  # Strict operational, platform, & git hygiene policies
├── ORIGINAL_REQUEST.md        # User mission & R1-R4 requirements
├── docs/                      # Architecture, methodology, platform rules, CLI ref
│   ├── ARCHITECTURE.md
│   ├── BRAIN_KNOWLEDGE_GRAPH.md
│   ├── BRAIN_RULES_AND_SUBMISSION.md
│   ├── CLI_REFERENCE.md
│   └── METHODOLOGY.md
├── src/
│   ├── lib.rs                 # Library root & module declarations
│   ├── main.rs                # 17 CLI commands defined via Clap
│   ├── allocation.rs          # Turnover Shannon entropy & HRP weights
│   ├── autotuner.rs           # Hyperparameter search (decay, exponent, neutralization)
│   ├── client.rs              # Fail-closed HTTP client with SSRF guard
│   ├── commands/              # CLI subcommands (auth, simulation, submission, portfolio, research)
│   ├── config.rs              # Tiered threshold partitions (platform, policy, hypothesis)
│   ├── correlation.rs         # Fail-closed Pearson correlation & merge impact
│   ├── crowd.rs               # Dataset census & radar heuristics
│   ├── crowd_allocation.rs    # FastExpr field extraction, dataset mapping & caching
│   ├── crowd_model.rs         # Profile PnL sensitivity diagnostics
│   ├── forecast.rs            # Portfolio deficit audit & inverse blueprint archetypes
│   ├── health.rs              # Multi-horizon decay & degradation diagnostics
│   ├── matrix.rs              # Interaction matrix & hyper-synergy scoring
│   ├── models.rs              # Serde data models & submission check evaluator
│   ├── screener.rs            # Bounded MPSC worker queue & account semaphore
│   ├── shadow.rs              # Legacy synthetic-proxy correlation helpers
│   └── taxonomy.rs            # Factor pillars & local candidate loader
├── tests/                     # 13 integration test suites with WireMock
├── data/                      # Local data stores (mostly gitignored)
│   ├── census_snapshots.json  # Cached census for datasets (model51, model16, news18, option9)
│   ├── fields_*.json          # Downloaded field metadata per dataset
│   ├── private_candidates.json# Untracked real research candidates
│   └── sanitize_field_patterns.txt
├── scripts/
│   └── sanitize_check.sh      # Pre-push sanitization gatekeeper
└── scratch/                   # Untracked Python research harness & prototypes
    ├── engine.py              # Python simulation & verification client
    ├── screen_and_evaluate.py # Multi-candidate screening & portfolio impact
    └── generate_hypotheses.py # Candidate generation scripts
```

---

## 3. Architecture & Module Breakdown

### 3.1 CLI Structure (`src/main.rs`)
The CLI provides 17 subcommands structured logically:
1. `auth`: Authenticates configured BRAIN sessions concurrently.
2. `sync`: Pulls active Out-of-Sample alphas to `portfolio_os.json`.
3. `score`: Checks live user competition rank, score, and leaderboard status.
4. `check`: Verifies mandatory platform In-Sample submission checks for an Alpha ID.
5. `audit`: Computes Pearson correlation over overlapping trading dates vs active OS portfolio.
6. `portfolio`: Simulates merged multi-alpha Out-of-Sample portfolio Sharpe and diversification.
7. `submit`: Submits alpha to OS after pre-flight check gate and local portfolio gate.
8. `screen`: Screens candidates using a bounded work queue (`max_workers=3`).
9. `sim`: Dispatches a single FastExpr simulation to BRAIN.
10. `radar`: Displays dataset usage census and crowd saturation.
11. `tune`: Performs grid sweep across decay, exponent, and neutralization groups.
12. `impact`: Simulates portfolio Sharpe/volatility/correlation delta if candidate alpha is merged.
13. `matrix`: Dataset synergy and interaction scoring.
14. `blueprint`: Identifies factor pillar deficits and generates candidate skeletons.
15. `shadow`: Inspects formula field usage and profile PnL sensitivity.
16. `health`: Analyzes alpha decay ratios and half-life degradation.
17. `allocate`: Calculates turnover Shannon entropy and HRP portfolio risk weights.

### 3.2 Data Models & Serialization (`src/models.rs`)
- `AlphaSettings`: Standard settings payload with defaults (`instrumentType: "EQUITY"`, `region: "USA"`, `universe: "TOP1000"`, `delay: 1`, `decay: 5`, `neutralization: "SUBINDUSTRY"`, `truncation: 0.05`, `pasteurization: "ON"`, `unitHandling: "VERIFY"`, `nanHandling: "OFF"`, `language: "FASTEXPR"`).
- `InSampleStats` & `CheckItem`: In-Sample performance metrics (`sharpe`, `fitness`, `returns`, `turnover`, `drawdown`, `margin`) and checks vector.
- `SubmissionCheckResult`: Evaluates submission checks in a strictly fail-closed manner.
- `DataFieldEntry`: Represents BRAIN field metadata.
  - *Observation*: Currently parses `id`, `description`, `dataset`, `region`, `delay`, `universe`, `coverage`, `dateCoverage`, `userCount`, `alphaCount`.
  - *Missing*: Does NOT parse `type` (MATRIX vs VECTOR), `category`, `subcategory`, `dateCreated`, or `themes`.
- `PortfolioImpactResult`: Baseline vs merged metrics: `delta_sharpe`, `delta_pnl`, `delta_vol`, `delta_avg_corr`, `max_pairwise_corr`.

### 3.3 Thresholds & Policies (`src/config.rs`)
Enforces strict 4-way separation of concerns per `AGENTS.md`:
1. `platform_rules`: Verified BRAIN Learn rules (`BRAIN_MAX_CORRELATION = 0.70`, `MIN_TURNOVER = 0.01`, `MAX_TURNOVER = 0.70`, `MIN_SHARPE_DELAY_1 = 1.25`, `MIN_FITNESS_DELAY_1 = 1.00`).
2. `project_policies`: Local safety gates (`MIN_CORRELATION_DAYS = 30`, `MIN_PORTFOLIO_DAYS = 100`).
3. `research_hypotheses`: Internal targets (`ORTHOGONAL_CORRELATION_THRESHOLD = 0.15`, `INTERNAL_TARGET_SHARPE_HEURISTIC = 1.58`).
4. `operational`: Concurrency & cache limits (`DEFAULT_WORKERS_PER_ACCOUNT = 3`, `PNL_CACHE_MAX_AGE_HOURS = 24`, `SIMULATION_POLL_INTERVAL_MS = 3000`).

### 3.4 API Client & Safety Guards (`src/client.rs`)
- **SSRF Guard**: Resolves and validates URLs against `api.worldquantbrain.com` before sending session cookies/credentials.
- **Idempotency & Retries**: Only `PostFailure::NotSent` (connection errors) and HTTP `429` (rate limit with exponential backoff) are retried. Ambiguous transport errors (timeouts, socket resets) fail closed with 64-bit FNV-1a fingerprinting to prevent duplicate simulations.
- **Submission Gate Evaluator**: Enforces all `CORE_SUBMISSION_CHECKS`:
  - `LOW_SHARPE`
  - `LOW_FITNESS`
  - `LOW_TURNOVER`
  - `HIGH_TURNOVER`
  - `CONCENTRATED_WEIGHT`
  - `SELF_CORRELATION`
  Fails closed if empty, pending, error, or unrecognized status occurs.

### 3.5 Correlation & Portfolio Impact Engine (`src/correlation.rs`)
- `pearson_correlation(x, y)`: Calculates sample Pearson correlation across intersecting dates. Requires $\ge 30$ common dates and non-zero variance. Clamps result in $[-1.0, 1.0]$.
- `simulate_portfolio(pnls)`: Equal-weighted portfolio simulation requiring $\ge 100$ common dates across all included alphas. Computes annualized PnL ($252 \times \mu$), annualized volatility ($\sqrt{252} \times \sigma$), and merged Sharpe.
- `calculate_portfolio_impact(candidate_id, cand_pnl, os_pnls)`: Measures delta impact of adding a candidate alpha to the baseline portfolio. Computes `delta_sharpe`, `delta_vol`, `delta_pnl`, `delta_avg_corr`, `max_pairwise_corr`.

---

## 4. Expression Handling & Syntax Analysis

### 4.1 Current Expression Lifecycle
Currently, FastExpr alpha expressions are handled as raw strings throughout the entire pipeline:
1. Candidate definition: Hardcoded string templates in `src/taxonomy.rs` or `src/forecast.rs` with placeholders (`<FIELD>`, `<SIGNAL>`).
2. Field extraction: `src/crowd_allocation.rs` (`extract_field_candidates`) uses a character-by-character scanner to find identifier tokens:
   - Skips quoted strings.
   - Skips pure numeric literals and reserved words (`true`, `false`, `nan`, `inf`, `market`, `sector`, `industry`, `subindustry`, `cap`).
   - Skips tokens immediately followed by `(` (function calls) or `=` (variable assignments).
3. Dispatch: The raw string is transmitted to `/simulations` without prior syntax or semantic validation.

### 4.2 Deficiencies & Risks
- **No Balanced Delimiter Checking**: Mismatched parentheses `(`, `)` or square brackets `[` lead to immediate HTTP 400 Bad Request simulation errors on BRAIN.
- **No Vector vs Matrix Operator Validation**:
  - Vector fields in BRAIN (e.g., certain alternative data fields) require vector reduction operators like `vec_avg()`, `vec_sum()`, or `vec_max()` before matrix operations (`ts_mean`, `rank`, `group_neutralize`).
  - Passing a vector field directly into `rank(vector_field)` causes BRAIN compile failure.
- **No Missing Value Safety Validator**: Alternative data fields frequently exhibit NaN values. FastExpr expressions that do not backfill (`ts_backfill(..., horizon)`) or sanitize (`if_else(is_nan(x), 0, x)`) cause alpha degradation or execution errors.
- **No AST / Combinator Pipeline**: Combining signals (e.g. Core Signal + Non-linear Transform + Group Neutralization + Reversal Dampener) is currently manual copy-paste rather than modular composable building blocks.

---

## 5. Field Metadata, Caching & Target Datasets

### 5.1 Investigation of the 4 Target Datasets in `data/`
Inspection of `data/census_snapshots.json` confirms official BRAIN metadata for the four target Alternative Data families:

| Dataset ID | Official Name | Category | Subcategory | Field Count | Typical Coverage |
|---|---|---|---|---|---|
| `model51` | Systematic Risk Metrics | Model | Risk Models | 16 fields | ~96.3% |
| `model16` | Fundamental Scores | Model | Valuation Models | 24 fields | ~55.8% |
| `news18` | Ravenpack News Data | News | News Sentiment | 121 fields | ~73.0% |
| `option9` | Options Analytics | Option | Option Analytics | 30+ fields | ~98.1% |

Existing cached field metadata files in `data/`:
- `data/fields_option.json` and `data/fields_interest.json` contain fields belonging to `option9` (e.g., `pcr_vol_20`, `pcr_vol_180`, `pcr_oi_360`, `implied_volatility_call_90`, etc.).
- Fields for `model51`, `model16`, and `news18` have census records in `data/census_snapshots.json`, but dedicated `data/fields_model51.json`, `data/fields_model16.json`, and `data/fields_news18.json` have not yet been queried or cached locally.

### 5.2 Field Metadata Model Gap
The `DataFieldEntry` in `src/models.rs`:
```rust
pub struct DataFieldEntry {
    pub id: String,
    pub description: Option<String>,
    pub dataset: Option<DatasetCategoryInfo>,
    pub region: Option<String>,
    pub delay: Option<i32>,
    pub universe: Option<String>,
    pub coverage: Option<f64>,
    pub date_coverage: Option<f64>,
    pub user_count: Option<i64>,
    pub alpha_count: Option<i64>,
}
```
In BRAIN `/data-fields`, each field JSON has:
```json
{
  "id": "pcr_vol_20",
  "description": "...",
  "type": "MATRIX",          // <--- MISSING from Rust model
  "category": { ... },       // <--- MISSING from Rust model
  "subcategory": { ... },    // <--- MISSING from Rust model
  "dateCoverage": 1.0,
  "coverage": 0.981,
  "dateCreated": "2022-05-01"// <--- MISSING from Rust model
}
```
Recognizing `"type": "MATRIX"` vs `"VECTOR"` is critical for FastExpr syntax validation (R2 & R3).

---

## 6. Test Suites & Execution Analysis

### 6.1 Test Suites Overview
The project has 13 test files in `tests/`:
1. `allocation_tests.rs`: Tests Shannon turnover entropy, HRP weights, and admission logic.
2. `autotuner_tests.rs`: Tests hyperparameter decay sweeps.
3. `benchmark_tests.rs`: Benchmark execution throughput for correlation and matrix ops.
4. `correlation_tests.rs`: Comprehensive Pearson tests (perfect positive, negative, zero variance, NaN filtering, boundary 29 vs 30 days, merge simulation).
5. `crowd_tests.rs`: Tests pillar detection and crowd-risk heuristics.
6. `health_tests.rs`: Tests decay half-life and degradation regimes.
7. `matrix_tests.rs`: Tests multi-dataset synergy patterns.
8. `mock_api_tests.rs`: WireMock HTTP tests for authentication, SSRF protection, retry policy, and pagination.
9. `models_tests.rs`: Tests serialization/deserialization of AlphaSettings, InSampleStats, and fail-closed check evaluation.
10. `reporting_tests.rs`: Tests epoch timestamp rendering and cache age labeling.
11. `shadow_tests.rs`: Tests shadow portfolio calibration.
12. `submission_gate_tests.rs`: Tests fail-closed submission gate and `--skip-checks` opt-in.
13. `taxonomy_tests.rs`: Tests factor pillar parsing and candidate retrieval.

### 6.2 Key Test Findings
1. **Cargo Test Execution on macOS**:
   - `cargo test` without sandbox bypass fails with `Operation not permitted (os error 1)` on `target/debug/.cargo-build-lock`.
   - Running with `BypassSandbox: true` executes smoothly.
2. **Local Candidate Path Interference in `taxonomy_tests.rs`**:
   - Running `cargo test` in an environment where `data/private_candidates.json` is present causes 2 test failures in `tests/taxonomy_tests.rs`:
     - `test_curated_candidates_analyst_pillar`: Expects `c.pillar == "ANALYST_CONSENSUS"`, but `data/private_candidates.json` had `"CAPITAL_ALLOCATION_ANALYST"`.
     - `test_curated_candidates_default_batch`: Expects $\ge 2$ pillars in default batch, but `data/private_candidates.json` only had `OPTIONS_VRP` candidates for `TOP1000`.
   - **Verification**: Setting `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json cargo test` successfully passes all 104+ unit and integration tests!
   - **Architectural Solution**: Tests in `taxonomy_tests.rs` should either isolate themselves or the loader should prioritize test-friendly fallbacks during unit test runs.

---

## 7. Gap Analysis & Leverage Mapping for Requirements R1–R4

| Requirement | What Already Exists (Can Be Leveraged) | What Is Missing (Must Be Created) | Implementation Location |
|---|---|---|---|
| **R1. Mathematical Factor Taxonomy & Specification Module** | `FactorPillar` enum in `src/taxonomy.rs`, classification helper in `src/forecast.rs`, census metadata in `data/census_snapshots.json`. | 1. Formal mathematical specifications and economic rationales for `model51` (Systematic Risk: rolling beta, benchmark correlation, residual risk dispersion), `model16` (Fundamental Scores: analyst revisions, cashflow efficiency, quality derivatives, factor momentum), `news18` (Ravenpack News: sentiment divergence, corporate actions, earnings evaluation shocks), `option9` (Options Analytics: breakeven asymmetry, put-call forward price parity, term skew).<br>2. Comprehensive Markdown documentation covering formula derivations and failure modes. | `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`<br>and enhanced `src/taxonomy.rs` (or Python module). |
| **R2. Field Metadata Extraction & Schema Validator** | `fetch_data_field` in `src/client.rs`, `load_local_field_datasets` in `src/crowd_allocation.rs`, census snapshots in `data/`. | 1. Extend `DataFieldEntry` model in `src/models.rs` to include `type` (MATRIX/VECTOR), `category`, `subcategory`, `dateCreated`.<br>2. Automated CLI or Python script to query BRAIN `/data-fields?dataset={dataset_id}` for `model51`, `model16`, `news18`, `option9`, caching metadata into `data/fields_{dataset}.json`.<br>3. FastExpr Schema Validator checking matrix vs vector compatibility, coverage threshold validation ($\ge 70\%$), and safe missing-value treatment (`ts_backfill`, `is_nan`). | CLI command or script under `scratch/` or `src/commands/` + validator module. |
| **R3. Automated Expression Builder & Factor Combinator** | `FitnessAutoTuner` in `src/autotuner.rs`, token extraction in `src/crowd_allocation.rs`, candidate data models in `src/models.rs`. | 1. Modular Expression Generation Engine combining core alternative signals with orthogonal filters:<br>  - Non-linear transforms: `signed_power(x, p)`, `rank(x) - 0.5`, `winsorize(x)`, `ts_decay_linear(x, d)`.<br>  - Group neutralization: `group_neutralize(sig, densify(subindustry/industry/sector))`.<br>  - Liquidity/reversal dampeners: `- 0.05 * signed_power(rank(ts_delta(close, k)) - 0.5, 1.5)`.<br>2. Automated FastExpr Syntax Validator verifying balanced delimiters `()`, recognized operators, proper vector wrapper `vec_avg`, and parameter validity before simulation dispatch. | Python/Rust builder module in `src/` or `scratch/` with unit tests. |
| **R4. Verification & Portfolio Impact Evaluator** | Concurrency control (`workers=3`) in `src/screener.rs` & `scratch/engine.py`, Pearson audit in `src/correlation.rs`, portfolio delta simulation in `calculate_portfolio_impact`. | Integrated evaluation pipeline that:<br>1. Enforces strict `max_workers=3` simulation limit.<br>2. Evaluates In-Sample criteria: Sharpe $\ge 1.25$, Fitness $\ge 1.00$, Turnover $1.0\% - 70.0\%$, Returns $> 8.0\%$.<br>3. Audits local Pearson correlation against active OS baseline ($\max \rho < 70.0\%$).<br>4. Evaluates accretive portfolio merge impact ($\Delta \text{Sharpe} > 0$, $\Delta \text{Vol} \le 0$).<br>5. Outputs verified candidate records into JSON/CSV. | Integrated script in `scratch/` or CLI command. |

---

## 8. Private Research Material & Git Hygiene Rules (`AGENTS.md`)

Compliance with `AGENTS.md` is mandatory. The survey reviewed the repository's hygiene enforcement:
1. **Never commit private candidate formulas or Alpha IDs**:
   - Tracked files in `src/` must contain only abstract placeholders (`<SIGNAL>`, `<FIELD>`, `<GROUP>`, `<PARAMETER>`).
   - Concrete candidate formulas must be loaded dynamically from untracked stores: `data/private_candidates.json` or `ALPHAFIND_CANDIDATES_PATH`.
2. **Gitignored Paths**:
   - `.env`
   - `submittable_alphas.csv`
   - `portfolio_os.json`
   - `data/results.json`
   - `data/census_snapshots.json`
   - `data/private_candidates.json`
   - `data/fields_*.json`
   - `scratch/`
3. **Pre-Push Sanitization Gatekeeper (`scripts/sanitize_check.sh`)**:
   - Verified active and passing.
   - Scans tracked files under `src/`, `tests/`, `docs/`, `data/`, and `scripts/`.
   - Scans added diff lines and all historical commits for 8-character Alpha IDs.
   - Verifies that private candidate literals from `data/private_candidates.json` and regex patterns in `data/sanitize_field_patterns.txt` are completely absent from tracked files.

---

## 9. Architectural Recommendations for Implementation

1. **Dual-Track Implementation Strategy (Rust CLI + Python Research Harness)**:
   - For rapid iteration on alternative data hypotheses, simulation, and portfolio evaluation (R1, R3, R4), Python modules in `scratch/` (such as extending `engine.py` / `screen_and_evaluate.py`) provide high flexibility and zero compilation friction while remaining fully gitignored.
   - For core verification, schema validation, and CLI commands (R2, R3 parser), Rust modules in `src/` provide type safety and high-speed execution.
2. **Enhance `DataFieldEntry` in Rust & Python**:
   - Add `data_type: String` ("MATRIX" vs "VECTOR"), `category: Option<DatasetCategoryInfo>`, and `subcategory: Option<DatasetCategoryInfo>`.
   - Update `src/client.rs` to allow paginated queries for `/data-fields?dataset={dataset_id}&limit=50&offset={offset}`.
3. **Implement a Lightweight FastExpr Syntax Validator**:
   - Token-based or recursive-descent validator checking:
     - Parentheses & bracket matching.
     - Known function whitelist (`ts_decay_linear`, `signed_power`, `rank`, `winsorize`, `ts_backfill`, `group_neutralize`, `vec_avg`, etc.).
     - Vector containment: Ensure vector fields are wrapped in vector reduction operators (`vec_avg`, etc.) before being passed into scalar/matrix functions.
4. **Isolate Unit Tests from Local Research State**:
   - In `tests/taxonomy_tests.rs`, configure test setup to override `ALPHAFIND_CANDIDATES_PATH` with a dedicated mock candidate file or empty path so local edits to `data/private_candidates.json` do not break `cargo test`.
5. **Strict Adherence to 3-Worker Limit**:
   - Ensure all simulation executors enforce `workers = 3` per account.
