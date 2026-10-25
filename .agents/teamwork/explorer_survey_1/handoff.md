# Handoff Report — Codebase Architectural Survey (Explorer Survey 1)

**Task**: In-depth survey of the alphafind codebase for requirements R1–R4 and AGENTS.md policy adherence  
**Author**: Codebase Architect Explorer (Survey 1)  
**Date**: 2026-10-24  
**Working Directory**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_survey_1`  

---

## 1. Observation

1. **Cargo.toml and Crate Dependencies**:
   - `Cargo.toml:1-24` defines `alphafind` v1.9.0 as a single Rust binary crate with `tokio = { version = "1.40", features = ["full"] }`, `reqwest = { version = "0.12", ... }`, `serde`, `serde_json`, `csv`, `clap = { version = "4.5", features = ["derive"] }`, `dotenvy`, `colored`, `anyhow`, `base64`, and dev-dependency `wiremock = "0.6"`.
2. **CLI Commands and Routing**:
   - `src/main.rs:21-289` defines 17 CLI commands via `clap`: `Auth`, `Sync`, `Score`, `Check`, `Audit`, `Portfolio`, `Submit`, `Screen`, `Sim`, `Radar`, `Tune`, `Impact`, `Matrix`, `Blueprint`, `Shadow`, `Health`, and `Allocate`.
   - Command implementations are partitioned in `src/commands/`: `auth.rs`, `portfolio.rs`, `research.rs`, `simulation.rs`, `submission.rs`.
3. **Data Field Models and Censuses**:
   - `src/models.rs:414-436` defines `DataFieldEntry` containing `id`, `description`, `dataset`, `region`, `delay`, `universe`, `coverage`, `date_coverage`, `user_count`, and `alpha_count`.
   - `data/fields_option.json:20` and `data/census_snapshots.json:800-1965` show BRAIN's `/data-fields` and `/data-sets` responses include `"type": "MATRIX"` or `"VECTOR"`, `category`, and `subcategory`. These fields are currently unmodeled in `DataFieldEntry`.
   - `data/census_snapshots.json` contains full metadata entries for all 4 target datasets: `model51` (line 931, "Systematic Risk Metrics", 16 fields), `model16` (line 799, "Fundamental Scores", 24 fields), `news18` (line 1327, "Ravenpack News Data", 121 fields), and `option9` (line 1855, "Options Analytics").
4. **Expression Handling & Parsing**:
   - FastExpr formulas are treated as raw strings across `src/taxonomy.rs`, `src/models.rs`, and `src/screener.rs`.
   - `src/crowd_allocation.rs:125-181` (`extract_field_candidates`) implements a manual character scanner skipping strings, numbers, reserved words, and function names/assignments. No syntax tree (AST), bracket validator, or vector-operator validator exists.
5. **Correlation and Portfolio Impact**:
   - `src/correlation.rs:11-62` (`pearson_correlation`) enforces fail-closed semantics: requires $\ge 30$ common dates and non-zero variance; otherwise returns `(None, n)`.
   - `src/correlation.rs:154-234` (`simulate_portfolio`) requires $\ge 100$ common dates for multi-alpha merge.
   - `src/correlation.rs:237-317` (`calculate_portfolio_impact`) computes `delta_sharpe`, `delta_vol`, `delta_pnl`, `delta_avg_corr`, and `max_pairwise_corr`.
6. **Concurrency and Simulation Dispatch**:
   - `src/config.rs:106` sets `DEFAULT_WORKERS_PER_ACCOUNT = 3`.
   - `src/screener.rs:49,73` bounds account simulation concurrency using `tokio::sync::Semaphore::new(concurrency)`.
   - `scratch/engine.py:341` uses `ThreadPoolExecutor(max_workers=3)`.
7. **Test Suites & Local State Interference**:
   - 13 test files in `tests/`.
   - `cargo test` execution requires `BypassSandbox: true` due to file locking on `target/debug/.cargo-build-lock`.
   - In default environment with `data/private_candidates.json` present, `tests/taxonomy_tests.rs` failed 2 tests:
     - `test_curated_candidates_analyst_pillar`: panic `left == right` (`"CAPITAL_ALLOCATION_ANALYST"` vs `"ANALYST_CONSENSUS"`).
     - `test_curated_candidates_default_batch`: panic `Default batch should have multiple candidates across pillars`.
   - When run with `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json cargo test`, all 14 tests in `tests/taxonomy_tests.rs` and all 104+ total tests pass with 0 failures.
8. **Git Hygiene and Research Material Protection**:
   - `scripts/sanitize_check.sh:1-219` runs pre-push checks against tracked files, diff additions, and commit history for Alpha IDs, candidate literals, and dataset field patterns.
   - Running `bash scripts/sanitize_check.sh` outputs:
     `✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.`

---

## 2. Logic Chain

1. **R1 (Factor Taxonomy)**:
   - *Observation*: `FactorPillar` (`src/taxonomy.rs`) and `classify_alpha_pillar` (`src/forecast.rs`) have generic enum variants, but no mathematical specification or economic rationale exists for the 4 target alternative datasets (`model51`, `model16`, `news18`, `option9`).
   - *Inference*: A dedicated mathematical specification document (`docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`) and taxonomy mapping must be created to formalize the economic rationale, mathematical derivations, and failure modes for these four dataset families.
2. **R2 (Field Metadata & Schema Validator)**:
   - *Observation*: `data/census_snapshots.json` holds high-level census data for `model51`, `model16`, `news18`, and `option9`, but `DataFieldEntry` in `src/models.rs` omits `type` (MATRIX/VECTOR), and `src/client.rs` lacks a paginated dataset-wide query for `/data-fields?dataset=...`.
   - *Inference*: Extending `DataFieldEntry` to parse `type`, implementing a metadata extractor script/command to cache `fields_{dataset}.json`, and adding a schema validator will prevent invalid FastExpr generation (such as missing `vec_avg` on vector fields).
3. **R3 (Expression Builder & Factor Combinator)**:
   - *Observation*: Formula strings are currently static templates with placeholders. No syntax validator exists before simulation POSTs.
   - *Inference*: A combinator module that programmatically layers Core Signals + Non-Linear Transforms + Group Neutralization + Reversal Dampeners, coupled with an automated syntax validator (checking parentheses balance and valid operator signatures), is required to satisfy R3 and eliminate HTTP 400 Bad Request simulation errors.
4. **R4 (Verification & Portfolio Impact Evaluator)**:
   - *Observation*: Concurrency bounding (`max_workers=3`) is implemented in `src/screener.rs` and `scratch/engine.py`; Pearson correlation and portfolio merge impact calculations exist in `src/correlation.rs` and `scratch/screen_and_evaluate.py`.
   - *Inference*: We can consolidate these components into an integrated evaluation tool that enforces `max_workers=3`, parses IS criteria (Sharpe $\ge 1.25$, Fitness $\ge 1.00$, Turnover $1.0\% - 70.0\%$, Returns $> 8.0\%$), and verifies local Pearson correlation ($\max \rho < 70.0\%$, $\Delta\text{Sharpe} > 0$, $\Delta\text{Vol} \le 0$).
5. **Git Hygiene Compliance**:
   - *Observation*: `scripts/sanitize_check.sh` enforces zero leaks of Alpha IDs, candidate formulas, and proprietary field patterns into tracked files.
   - *Inference*: All implementation artifacts must adhere strictly to this policy by keeping real candidate formulas in gitignored files (`data/private_candidates.json` or `scratch/`) and using placeholders in tracked source code.

---

## 3. Caveats

1. **Local Test Environment Sensitivity**: The unit test failure in `tests/taxonomy_tests.rs` when `data/private_candidates.json` is present demonstrates that local uncommitted candidate files can influence test expectations. Unit tests must be executed with `ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json` to test baseline catalog defaults.
2. **Live BRAIN API Endpoint Availability**: The official `/data-fields` endpoint returns live metadata and rate limits (HTTP 429); extraction scripts must use exponential backoff and adhere to politeness rules.
3. **Read-Only Scope**: This task was strictly read-only investigation. No source files were modified.

---

## 4. Conclusion

The `alphafind` codebase provides a high-quality, architecturally disciplined base with robust concurrency controls, fail-closed quantitative metrics, and rigorous Git hygiene. 

To fulfill requirements R1–R4:
- **R1**: Create comprehensive mathematical factor taxonomy documentation (`docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md`) covering `model51`, `model16`, `news18`, and `option9`.
- **R2**: Enhance `DataFieldEntry` to support matrix/vector types and build a dedicated field metadata extraction & schema validation tool.
- **R3**: Implement an expression builder and FastExpr syntax validator ensuring balanced brackets, valid function signatures, and vector-operator compliance.
- **R4**: Deliver an integrated evaluation pipeline adhering to `max_workers=3`, IS metric gates, Pearson correlation audits, and accretive portfolio merge criteria.

---

## 5. Verification Method

To independently verify the observations and findings in this report:

1. **Verify Entire Test Suite Passes**:
   ```bash
   ALPHAFIND_CANDIDATES_PATH=/tmp/empty.json cargo test
   ```
   *Expected result*: All 104+ unit and integration tests across 13 test suites pass with 0 failures.

2. **Verify Git Hygiene Pre-Push Sanitizer**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
   *Expected result*: `✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.`

3. **Verify Target Alternative Datasets in Cached Metadata**:
   ```bash
   python3 -c '
   import json
   with open("data/census_snapshots.json") as f:
       data = json.load(f)
   target_ids = {"model51", "model16", "news18", "option9"}
   found = {d["id"]: d["name"] for d in data.get("datasets", []) if d["id"] in target_ids}
   print(found)
   assert len(found) == 4, f"Missing target datasets: {target_ids - set(found.keys())}"
   '
   ```
   *Expected result*: Prints all 4 target datasets with their official BRAIN names.
