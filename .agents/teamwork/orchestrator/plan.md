# Plan: Quantitative Alpha Research Framework for WorldQuant BRAIN

## Objective
Research and build an in-depth quantitative alpha research framework for WorldQuant BRAIN focused on proprietary Alternative Data datasets (`model51`, `model16`, `news18`, `option9`), satisfying requirements R1-R4 and acceptance criteria.

## Phase 0: Survey & Scope Mapping
- Dispatch 3 Explorers / Spec Miners:
  - Explorer 1 (Codebase Architecture & Existing AlphaFind Tools): Survey existing Rust code (`src/`, `tests/`), configs, models, CLI commands, and execution hooks.
  - Spec Miner 2 (BRAIN Alternative Data Specs & Documentation): Survey `docs/`, `AGENTS.md`, BRAIN rules, dataset specs (`model51`, `model16`, `news18`, `option9`), FastExpr operators, and `/data-fields` formats.
  - Explorer 3 (Portfolio Baseline, Simulation, & Verification Pipeline): Survey existing portfolio baseline evaluation, simulation runner, credential handling, correlation calculation, and constraints.
- Synthesize findings into `PROJECT.md` (Feature Inventory, Architecture, Milestones, Interface Contracts, Code Layout).

## Phase 1: Milestone M1 — Mathematical Factor Taxonomy & Specification Module (R1)
- Comprehensive Markdown specifications detailing economic hypotheses, formula derivations, data inputs, and failure modes for:
  - Systematic Risk (`model51`)
  - Fundamental Scores & Derivative Accel (`model16`)
  - Ravenpack News (`news18`)
  - Options Analytics (`option9`)
- Verification via review, challenger, and audit.

## Phase 2: Milestone M2 — Field Metadata Extraction & Schema Validator (R2)
- Query, inspect, parse, and cache metadata from BRAIN's `/data-fields` endpoint.
- Matrix/vector dimension identification, FastExpr compatibility, historical coverage, missing value handling without look-ahead.
- Automated tests and verification.

## Phase 3: Milestone M3 — Automated Expression Builder & Factor Combinator (R3)
- Modular expression generation engine combining core signals with non-linear transforms, group neutralizations (`SUBINDUSTRY`, `INDUSTRY`, `SECTOR`), and reversal/liquidity dampeners.
- Syntax verification against FastExpr grammar before simulation dispatch.
- Automated tests and verification.

## Phase 4: Milestone M4 — Verification & Portfolio Impact Evaluator (R4)
- Verification script / CLI integration adhering strictly to `max_workers=3` per account.
- Performance metric parsing: Sharpe >= 1.25, Fitness >= 1.00, Turnover 1%-70%, Returns > 8%.
- Local Pearson correlation calculation against active portfolio baseline (max_rho < 70%, delta_sharpe > 0, delta_vol <= 0).
- Run live/simulated backtests for >= 3 distinct hypotheses and qualify at least 1 candidate alpha.

## Phase 5: Final Validation & Sentinel Completion Report
- E2E test suite validation.
- Forensic integrity audit.
- Final synthesis and reporting to Sentinel.
