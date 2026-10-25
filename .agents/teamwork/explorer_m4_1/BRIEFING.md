# BRIEFING — 2026-10-24T22:25:30Z

## Mission
Investigate codebase architecture and design the Simulation Pipeline & Concurrency Architecture for Milestone M4.

## 🔒 My Identity
- Archetype: explorer
- Roles: investigator, architect, reporter
- Working directory: /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m4_1
- Original parent: 8b020a34-8c2d-4255-98f9-544909ff7652
- Milestone: M4

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Enforce AGENTS.md policy: 3 concurrent workers per account, fail-closed handling, private research material protection, git hygiene
- Propose architecture and runner design for M4 without modifying production source code

## Current Parent
- Conversation ID: 8b020a34-8c2d-4255-98f9-544909ff7652
- Updated: 2026-10-24T22:21:47Z

## Investigation State
- **Explored paths**:
  - `src/config.rs` (operational constants, `DEFAULT_WORKERS_PER_ACCOUNT = 3`, platform vs project thresholds)
  - `src/screener.rs` (`MultiAccountScreener`, `AccountWorker`, `tokio::sync::Semaphore` permits, mpsc channel)
  - `src/client.rs` (`submit_simulation`, `poll_simulation`, `get_alpha_details`, `check_submission`, fail-closed retry logic, non-idempotent fingerprinting)
  - `src/commands/simulation.rs` (`cmd_screen`, `cmd_sim`, `cmd_radar`, `cmd_tune`)
  - `src/expression_builder.rs` (M3 builder, auto-repair, schema validator integration)
  - `scratch/screen_and_evaluate.py` (legacy 466-line evaluation runner)
  - `tests/mock_api_tests.rs` (wiremock-based API mocking, rate limiting, SSRF protection, non-retryable transport errors)
- **Key findings**:
  1. Concurrency: `DEFAULT_WORKERS_PER_ACCOUNT = 3` in `src/config.rs:106` enforced by `Arc<tokio::sync::Semaphore>` in `src/screener.rs:49,184`. Permit is held across simulation submit + poll + details. Need cross-process PID lock (`.alphafind_sim.lock`) to prevent multi-process quota oversubscription.
  2. BRAIN API: `/simulations` endpoint accepts `SimulationPayload` (`sim_type: "REGULAR"`, `settings`, `regular`). Polling validates domain against SSRF and parses `COMPLETE`/`WARNING`/`ERROR`. Failed non-idempotent POSTs fail closed with 64-bit FNV-1a fingerprint, only retrying `err.is_connect()` and explicit HTTP 429 backoff (`Retry-After`).
  3. Metrics & Checks: `GET /alphas/{id}` parses `InSampleStats` (Sharpe >= 1.25, Fitness >= 1.00, Turnover 1%-70%, Returns > 8%). `GET /alphas/{id}/check` enforces 6 core checks (`CORE_SUBMISSION_CHECKS`), failing closed on missing or pending checks.
  4. Execution Runner: Dual-runner design. Rust CLI integration (`alphafind screen --alternative` or evaluation harness) plus standalone runner `scratch/evaluate_alternative_alphas.py`. Offline mode enables deterministic mock evaluation for CI and non-credentialed test environments.
- **Unexplored areas**: None, all target paths fully analyzed.

## Key Decisions Made
- Architecture blueprint finalized with 5 concrete components for M4 Worker.
- Defined deterministic offline evaluation mode specifying mocked responses and synthetic PnL generation.
- Formulated advisory cross-process lockfile protocol for AGENTS.md multi-process safety.

## Artifact Index
- `.agents/teamwork/explorer_m4_1/DISPATCH.md` — Inbound instructions log
- `.agents/teamwork/explorer_m4_1/BRIEFING.md` — Situational awareness
- `.agents/teamwork/explorer_m4_1/progress.md` — Liveness heartbeat
- `.agents/teamwork/explorer_m4_1/handoff.md` — Final deliverable report
