## 2026-10-24T18:33:40Z
You are the Simulation & Verification Researcher for the quantitative alpha research framework.
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_survey_3/
You must first read the original user request at: /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
Also read /Users/phamtoan/Developer/alphafind/AGENTS.md for strict concurrency and submission policies.

Your task is to investigate the simulation, evaluation, and portfolio verification pipeline:
1. Simulation dispatch: How does alphafind simulate alphas on BRAIN? (Inspect src/client/, src/brain/, src/screener/, simulation endpoints, payload structures, authentication/credentials).
2. Concurrency & Rate Limiting: How are worker accounts, concurrency limits (strictly max_workers=3 per account per AGENTS.md), and semaphore permits implemented?
3. In-Sample Metrics & Check Evaluator:
   - How are Sharpe, Fitness, Turnover, and Returns parsed and validated?
   - Thresholds: Sharpe >= 1.25, Fitness >= 1.00, Turnover 1.0% - 70.0%, Returns > 8.0%.
   - Check evaluator logic (fail-closed, core checks).
4. Portfolio Baseline & Correlation Audit:
   - How are active portfolio alphas loaded and managed?
   - How is Pearson correlation (max_rho < 70%) calculated between candidate alpha and portfolio baseline?
   - How are accretive merge metrics (delta_sharpe > 0, delta_vol <= 0) evaluated?
5. Environment verification: Check whether BRAIN credentials / environment variables are configured or if mock/recorded responses exist for simulation testing.

Deliverables:
- Write your comprehensive findings to /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_survey_3/survey_simulation.md
- Write /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_survey_3/handoff.md summarizing the simulation and verification architecture.
- Send a message to your parent (orchestrator) notifying completion.
