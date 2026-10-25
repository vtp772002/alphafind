# Original User Request

## 2026-10-24T18:30:37Z

Research and build an in-depth quantitative alpha research framework for WorldQuant BRAIN focused on proprietary Alternative Data datasets (`model51` Systematic Risk, `model16` Fundamental Scores, `news18` Ravenpack News, and `option9` Options Analytics). The framework includes mathematical factor specifications, automated field metadata discovery, and high-throughput hypothesis generation pipelines that produce orthogonal, backtest-ready alpha expressions.

Working directory: `/Users/phamtoan/Developer/alphafind`
Integrity mode: development

## Requirements

### R1. Mathematical Factor Taxonomy & Specification Module
Design and document mathematical specifications and economic rationales for factor architectures derived from BRAIN's alternative datasets:
- Systematic Risk Metrics (`model51`): Rolling multi-horizon beta, benchmark correlation, and residual risk dispersion.
- Fundamental Scores & Derivative Accel (`model16`): Analyst revisions, cashflow efficiency, quality derivatives, and factor momentum.
- Ravenpack News (`news18`): Sentiment divergence, corporate actions, and earnings evaluation shocks.
- Options Analytics (`option9`): Breakeven asymmetry, put-call forward price parity, and term skew.

### R2. Field Metadata Extraction & Schema Validator
Implement an automated tool to query, inspect, and cache metadata from BRAIN's `/data-fields` endpoint for the target datasets. The tool must identify matrix/vector dimensions, verify FastExpr compatibility, record historical coverage, and handle missing values without look-ahead bias.

### R3. Automated Expression Builder & Factor Combinator
Develop a modular expression generation engine in Python/Rust that combines alternative data core signals with multi-layer orthogonal filters:
- Dynamic non-linear transforms (`signed_power`, `rank`, `winsorize`, `ts_decay_linear`).
- Group neutralization across `SUBINDUSTRY`, `INDUSTRY`, and `SECTOR`.
- Liquidity and short-term reversal dampeners to stabilize turnover and suppress high-frequency noise.

### R4. Verification & Portfolio Impact Evaluator
Provide an integrated evaluation script that:
- Runs simulations on BRAIN adhering strictly to concurrency limits (`max_workers=3` per account per `AGENTS.md`).
- Parses In-Sample performance metrics: Sharpe >= 1.25, Fitness >= 1.00, Turnover 1.0% - 70.0%, Returns > 8.0%.
- Conducts local Pearson correlation audits against the active portfolio baseline, enforcing max_rho < 70.0% and accretive portfolio merge impact (delta_sharpe > 0, delta_vol <= 0).

## Acceptance Criteria

### Architecture & Code Quality
- [ ] A dedicated module or script suite exists under `src/` or `scratch/` with clean CLI or programmatic interfaces for metadata extraction and hypothesis generation.
- [ ] Mathematical documentation detailing economic hypothesis, formula derivation, and failure modes for each alternative dataset family is provided in Markdown.
- [ ] Automated syntax verification passes for all generated FastExpr strings before API dispatch (preventing 400 Bad Request simulation errors).

### Verification & Performance Targets
- [ ] Programmatic test suite verifies field schema parsing, formula rendering, and correlation calculations with test coverage.
- [ ] Pipeline generates and successfully executes simulations on BRAIN for at least 3 distinct alternative data hypotheses.
- [ ] At least one candidate alpha from the new alternative architectures achieves full In-Sample qualification (Sharpe >= 1.25, Fitness >= 1.00, Turnover within limits) and proves low cross-correlation (max_rho < 70%) against existing portfolio alphas.
