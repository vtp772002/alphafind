# E2E Test Infra: Quantitative Alpha Research Framework

## Test Philosophy
- Opaque-box, requirement-driven testing. No dependency on implementation design.
- Methodology: Category-Partition + Boundary Value Analysis + Pairwise Combinatorial Testing + Workload Testing.

## Feature Inventory & Test Coverage Goals
| # | Feature | Source (Requirement) | Tier 1 (Coverage) | Tier 2 (Boundary) | Tier 3 (Cross-Feature) | Tier 4 (Real-World) |
|---|---------|----------------------|:-----------------:|:-----------------:|:----------------------:|:-------------------:|
| 1 | Mathematical Factor Taxonomy (`model51`, `model16`, `news18`, `option9`) | ORIGINAL_REQUEST §R1 | 5 tests | 5 tests | Pairwise | Real-World Workload |
| 2 | Field Metadata Extraction & Schema Validator (`/data-fields`) | ORIGINAL_REQUEST §R2 | 5 tests | 5 tests | Pairwise | Real-World Workload |
| 3 | Automated Expression Builder & FastExpr Syntax Validator | ORIGINAL_REQUEST §R3 | 5 tests | 5 tests | Pairwise | Real-World Workload |
| 4 | Verification & Portfolio Impact Evaluator (`max_workers=3`, IS checks, Pearson rho) | ORIGINAL_REQUEST §R4 | 5 tests | 5 tests | Pairwise | Real-World Workload |

## Test Architecture
- Test runner: `cargo test` and `python3 -m unittest` / pytest test runners.
- Test case format:
  - Unit tests for field metadata models and schema validator in Rust/Python.
  - Unit tests for syntax validator and expression builder.
  - Unit tests for correlation audit, merge metrics, and portfolio impact.
  - Integration simulation run tests enforcing concurrency limits and IS gates.
  - Sanitization script audit (`bash scripts/sanitize_check.sh`).
- Directory layout:
  - `tests/`: Rust test suites.
  - `scratch/`: Experimental & verification runner scripts.

## Coverage Thresholds
- Tier 1: Feature Coverage (>=5 test cases per feature area)
- Tier 2: Boundary & Corner Cases (>=5 test cases per feature area)
- Tier 3: Cross-Feature Interactions (pairwise combinations)
- Tier 4: Real-World Alternative Data Simulation Scenarios
- Pass criteria: 100% tests pass with exit code 0.
