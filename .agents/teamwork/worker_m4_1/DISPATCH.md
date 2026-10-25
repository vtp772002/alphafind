# Dispatch: Worker M4 (Verification, Simulation Dispatcher, Check Gate & Portfolio Impact)

**Assigned Agent**: `worker_m4_1` (teamwork_preview_worker)
**Target Milestone**: M4 (Verification & Portfolio Impact Evaluator & Simulations)
**Working Directory**: `/Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m4_1`
**Parent Conversation ID**: `8b020a34-8c2d-4255-98f9-544909ff7652`
**Reference Documents**:
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md`
- `/Users/phamtoan/Developer/alphafind/PROJECT.md`
- `AGENTS.md` (concurrency policy: 3 workers per account, submission safety, fail-closed, git hygiene)
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m4_1/handoff.md`
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m4_2/handoff.md`
- `/Users/phamtoan/Developer/alphafind/portfolio_os.json`
- `/Users/phamtoan/Developer/alphafind/src/expression_builder.rs`
- `/Users/phamtoan/Developer/alphafind/src/correlation.rs`
- `/Users/phamtoan/Developer/alphafind/src/screener.rs`
- `/Users/phamtoan/Developer/alphafind/src/client.rs`

## 2026-10-24T22:29:42Z
You are worker_m4_1, a teamwork_preview_worker implementing Milestone M4: Verification, Simulation Dispatcher, Check Gate & Portfolio Impact (R4).

Working Directory: /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m4_1
Parent Conversation ID: 8b020a34-8c2d-4255-98f9-544909ff7652

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

MANDATORY FIRST STEP:
Read `/Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md` before starting work.
Also read:
- `/Users/phamtoan/Developer/alphafind/PROJECT.md`
- `AGENTS.md` (concurrency policy: 3 workers per account, submission safety, fail-closed, git hygiene)
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m4_1/handoff.md`
- `/Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m4_2/handoff.md`
- `/Users/phamtoan/Developer/alphafind/portfolio_os.json`
- `/Users/phamtoan/Developer/alphafind/src/expression_builder.rs`
- `/Users/phamtoan/Developer/alphafind/src/correlation.rs`
- `/Users/phamtoan/Developer/alphafind/src/screener.rs`
- `/Users/phamtoan/Developer/alphafind/src/client.rs`

Scope of Work:
1. Concurrency Control & Process Lock:
   - Implement `src/concurrency_lock.rs` providing cross-process advisory lock (`~/.cache/alphafind/sim_runner.lock`) to strictly enforce that no more than 3 simulation workers per account operate concurrently across processes, complying with `AGENTS.md`.
   - Wire into `src/lib.rs` and `src/screener.rs`.
2. Implement Integrated Simulation & Evaluator Runner:
   - Create `scratch/evaluate_alternative_alphas.py`:
     * Enforces `max_workers=3` with polite staggering (sleep 500ms / 2.5s initial thread offset) and acquires `~/.cache/alphafind/sim_runner.lock`.
     * Evaluates the 4 candidate hypotheses across `model51`, `model16`, `news18`, and `option9` specified in `explorer_m4_2/handoff.md`:
       1. `US_M51_Beta_Term_Spread`
       2. `US_M16_Fundamental_Acceleration_Convexity`
       3. `US_N18_Vector_Folded_Story_Sentiment`
       4. `US_O9_Breakeven_Directional_Asymmetry_30d`
     * Supports both Live Execution mode (via BRAIN API session credentials) and Deterministic Offline Mock mode (`--offline`) for hermetic evaluation in test/CI environments.
     * Evaluates In-Sample stats: Sharpe >= 1.25, Fitness >= 1.00, Turnover 1.0% - 70.0%, Returns > 8.0%.
     * Evaluates all 6 core submission checks (`LOW_SHARPE`, `LOW_FITNESS`, `LOW_TURNOVER`, `HIGH_TURNOVER`, `CONCENTRATED_WEIGHT`, `SELF_CORRELATION`), failing closed on any non-PASS or missing checks.
     * Evaluates pairwise Pearson correlation against the 94 active OS alphas in `portfolio_os.json` using `~/.cache/alphafind/portfolio_pnl.json` (requiring $N \ge 30$, enforcing $\max \rho < 0.70$).
     * Evaluates portfolio merge impact: common trading days $N \ge 100$, confirming accretive impact ($\Delta\text{Sharpe} > 0$, $\Delta\text{Vol} \le 0$).
     * Generates structured output in `scratch/evaluated_candidates.json` and updates `submittable_alphas.csv` (permissions 0600).
3. Rust CLI Alternative Alpha Screening Integration:
   - Wire `ExpressionBuilder` into `alphafind screen` (in `src/commands/simulation.rs` and `src/main.rs`) via an `--alternative` flag so alternative candidate batches can be screened natively.
4. Comprehensive Integration Test Suite:
   - Implement `tests/simulation_evaluation_tests.rs`:
     * Test concurrency lock acquisition and contention behavior.
     * Test In-Sample threshold evaluations.
     * Test fail-closed submission check evaluator.
     * Test Pearson correlation calculation and portfolio merge delta math against baseline data.
5. Verification:
   - Run `cargo test --test simulation_evaluation_tests`.
   - Run full `cargo test` to verify zero regressions across all 136+ tests.
   - Run `python3 scratch/evaluate_alternative_alphas.py --offline` to verify complete execution and outputs.
   - Run `bash scripts/sanitize_check.sh` to confirm zero leaks of private candidate formulas or Alpha IDs.
6. Deliverables:
   - Write comprehensive handoff report in `/Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m4_1/handoff.md`.
   - Send completion message to parent orchestrator (`8b020a34-8c2d-4255-98f9-544909ff7652`).
