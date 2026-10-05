# AGENTS.md — WorldQuant BRAIN Alpha Operational Charter & Guardrails

> [!IMPORTANT]
> **MANDATORY DIRECTIVE FOR ALL AI AGENTS:**
> This repository is governed by quantitative trading rules and API constraints of WorldQuant BRAIN.
> Every AI agent operating on this codebase must internalize and strictly adhere to every directive in this document.

---

## 0. Core Strategic Charter & Objective

> **"Not blindly maximizing quantity, but maximizing the quantity of high-quality, truly diversified Alphas within the available timeframe and compute budget."**

* **Primary Strategic Objective:** Build a top-tier institutional Out-of-Sample portfolio (Target Score: $\ge 0.88$, `isScore`: $\ge 20,000$, `uniquenessScore`: $\le 0.00$).
* **Quantitative Quality Standards:** Target Sharpe $\ge 1.35$, Fitness $\ge 1.50$ (Grade: GOOD) up to $\ge 2.50$ (SPECTACULAR), Turnover $\in [10\%, 35\%]$, and pairwise correlation $\overline{\rho} \le 0.15$.
* **Submission Cadence:** **Exactly 1 Alpha per UTC calendar day**. Never submit multiple alphas in one day (wastes distinct submission days).

---

## 0.1. Mandatory Operational Rules: Compute & Ephemeral Scripting

### Rule 1: Full 9-Worker Asynchronous Concurrency Across Configured Sessions
1. **Always deploy all 9 parallel workers across configured sessions** (3 on Primary, 3 on Worker Pool 1, 3 on Worker Pool 2) for all hypothesis screening, parameter grid sweeps, decay sweeps, and autotuning runs.
2. **Never restrict exploration to single-threaded execution.** Single-threaded execution severely slows iteration velocity.
3. **Role Partitioning**:
   - `PRIMARY SESSION` (Main): 3 concurrent workers. Exploration, correlation auditing, and official 8/8 PASS verification (`/check`).
   - `WORKER SESSION 1` (Pool 1): 3 concurrent workers. High-throughput hypothesis exploration and parameter sweeps.
   - `WORKER SESSION 2` (Pool 2): 3 concurrent workers. High-throughput hypothesis exploration and parameter sweeps.
4. **Automated Transfer Lock**: When any worker discovers an 8/8 PASS Alpha or high-conviction candidate ($\text{Fitness} \ge 1.40$), automatically serialize verification on the Primary Session using `transfer_lock`.

### Rule 2: Ephemeral Scripting & Zero Garbage Charter
1. **ALL ALPHA DISCOVERY SCRIPTS ARE STRICTLY DISPOSABLE (EPHEMERAL):**
   - Every single script created for alpha hunting, hypothesis screening, parameter sweeps, or testing is **temporary by definition**.
   - **Never create one-off test scripts in the root directory.**
   - Exploratory workflows must follow:
     - **Method A (Preferred)**: Run directly through core engines via CLI (e.g., `./target/release/alphafind screen --pillar ANALYST --universe TOP500`).
     - **Method B (Ad-hoc Hypothesis)**: Create the experimental script strictly inside the git-ignored `scratch/` directory (e.g., `scratch/hunt_day21.py`).
2. **Mandatory Post-Discovery Cleanup Protocol:**
   - As soon as a qualified 8/8 PASS Alpha is discovered and verified on Main Account:
     - Record its expression, universe, decay, truncation, neutralization, ID, and metrics into [`submittable_alphas.csv`](file:///Users/phamtoan/Developer/alphafind/submittable_alphas.csv) and [`portfolio_os.json`](file:///Users/phamtoan/Developer/alphafind/portfolio_os.json).
     - **The temporary script in `scratch/` MUST BE DELETED IMMEDIATELY before concluding the session.**
   - No throwaway `hunt_*.py`, `tune_*.py`, `test_*.py`, or temporary `.json` files are ever permitted to persist in Git.

---

## 1. Quantitative Doctrine & Portfolio Math

### 1. The Orthogonal Variance Law vs Portfolio Dilution
* WorldQuant BRAIN calculates `isScore` using an equal-weighted multi-alpha portfolio:
  $$\text{Sharpe}_{\text{merged}} = \frac{\overline{R}}{\overline{\sigma} \sqrt{\frac{1}{N} + \frac{N-1}{N} \overline{\rho}}}$$
* If new alphas share high pairwise correlation ($\overline{\rho} \approx 0.55$), the denominator converges to $\sqrt{0.55} = 0.74$, capping merged Sharpe at $\approx 2.25$ and `isScore` at $\approx 5,600$ regardless of alpha count.
* **Top 1 Standard ($\overline{\rho} \le 0.10$)**: True orthogonal alphas collapse portfolio covariance, surging merged Sharpe past $\ge 6.00$ and `isScore` past **$20,000+$**!
* **Mandatory Threshold**: Pairwise correlation with existing OS portfolio must be **$\le 0.15$** (verified via `alphafind audit <id>`).
* **Absolute Moratorium**: Ban on Fundamental DuPont / Cash Flow duplicates on `TOP1000`/`TOP200`.

### 2. Single-Dataset Factor Purity vs. Hierarchical Conditioning
* **Single-Dataset Purity (Pathway A)**: 100% of predictive power from one cohesive dataset (e.g., pure options IV surface, pure analyst revisions). Preserves `uniquenessScore` with official `DATA_USAGE: SINGLE_DATA_SET` classification.
* **Hierarchical Conditioning (Pathway B)**: Never use naive linear addition ($A + B$). Use substantive conditioning where Dataset A is the economic anchor and Dataset B acts as a risk/execution catalyst (e.g. Options Volatility Catalyst Alpha).

### 3. Economic Grounding & Neutralization Rules
* **Analyst Consensus**: MUST use `SECTOR` or `MARKET` neutralization on `TOP500`. Never use `SUBINDUSTRY` (collapses Sharpe from 1.42 to 0.74 because analyst upgrades occur in sector-wide waves).
* **Options VRP & Volatility Surface**: Best suited for `TOP3000` or `TOP1000` with `SUBINDUSTRY` or `MARKET` neutralization.
* **Microstructure VWAP**: Best on `TOP1000`/`TOP500` capturing order flow dislocations.

### 4. Non-Linear Convexity & The Fitness Denominator Trap
* BRAIN clamps the Fitness denominator at $\max(\text{Turnover}, 0.125)$. Suppressing turnover below $12.5\%$ provides zero marginal benefit in the denominator.
* To achieve **Fitness $\ge 1.50$ (GOOD)** or $\ge 2.50$ (SPECTACULAR), maximize annual returns via non-linear convex transformations `signed_power(signal, p)` with $p \in [3.8, 4.4]$ while keeping `truncation: 0.065 - 0.07` to avoid `CONCENTRATED_WEIGHT` violations.

---

## 2. The 8 Mandatory In-Sample Submission Checks

An Alpha must achieve **8/8 PASS** on `GET /alphas/{id}/check` to be admitted into Out-of-Sample (OS) forward tracking:

| # | Check Name | Official PASS Threshold | Quantitative Meaning & System Standard |
|:---:|:---|:---|:---|
| 1 | **LOW_SHARPE** | **$\text{Sharpe} \ge 1.25$** | System target: $\ge 1.35$. Remediate with `MARKET` neutralization. |
| 2 | **LOW_FITNESS** | **$\text{Fitness} \ge 1.00$** | System target: **$\ge 1.50$ (Grade: GOOD)**. $\text{Fit} = \text{Sharpe} \times \sqrt{\frac{\text{Returns}}{\max(\text{Turnover}, 0.125)}}$. |
| 3 | **LOW_TURNOVER** | **$\text{Turnover} \ge 1.0\%$** | Prevents passive/static portfolio allocations. |
| 4 | **HIGH_TURNOVER** | **$\text{Turnover} \le 70.0\%$** | Avoids transaction friction and execution slippage. |
| 5 | **CONCENTRATED_WEIGHT** | **PASS** | Weight distribution check; requires `truncation` $\le 0.08$ (optimal: $0.065 - 0.07$). |
| 6 | **LOW_SUB_UNIVERSE_SHARPE** | **PASS** | Profitability must hold across capitalization tiers within the universe. |
| 7 | **MATCHES_COMPETITION** | **PASS** | `Region: USA`, `EQUITY`, `Delay: 1`, `Pasteurization: ON`. |
| 8 | **SELF_CORRELATION** | **$\text{Corr} \le 0.70$** OR $\text{Sharpe}_{\text{new}} \ge 1.10 \times \text{Sharpe}_{\text{old}}$ | Must not correlate $> 0.70$ with any active OS alpha in portfolio. |

### Performance Grade Standards:
* 🌟 **SPECTACULAR**: $\text{Fitness} > 2.50$ (Delay 1)
* ⭐ **EXCELLENT**: $\text{Fitness} > 2.00$ (Delay 1)
* 🟢 **GOOD (Standard Target)**: $\text{Fitness} \ge 1.50$ (Delay 1) — High-Conviction submission standard.
* 🟡 **AVERAGE**: $\text{Fitness} \ge 1.00$ — Minimum platform compliance; do NOT submit without tuning.
* 🔴 **FAIL**: $\text{Fitness} < 1.00$ — Rejected by platform.

---

## 3. Pre-Reporting & Submission Protocol

> [!IMPORTANT]
> **MANDATORY PROTOCOL BEFORE PRESENTING ANY ALPHA TO USER:**
> 1. Verify official **8/8 PASS** via Main Account: `GET https://api.worldquantbrain.com/alphas/{alpha_id}/check`.
> 2. Ensure **Grade $\ge 1.50$ (GOOD)** (Sharpe $\ge 1.35$, Return $\ge 12.0\%$, Turnover $\in [10\%, 35\%]$).
> 3. Audit exact 1,236-day Pearson cross-correlation against existing portfolio ($\rho \le 0.15$).
> 4. Record entry in [`submittable_alphas.csv`](file:///Users/phamtoan/Developer/alphafind/submittable_alphas.csv).
> 5. Present user with: Alpha ID, In-Sample Metrics (Sharpe, Fitness, Return, Turnover), Category & Tags, and the **1-Click Web Submission URL**:
>    `https://platform.worldquantbrain.com/alpha/{alpha_id}`

---

## 4. Specialized Documentation & Repository Pointers

To maintain strict context window hygiene and avoid duplicate bloat, refer to specialized project files:

| Dimension / Topic | Canonical File / Reference | Purpose & Contents |
|:---|:---|:---|
| **Active OS Portfolio** | [`portfolio_os.json`](file:///Users/phamtoan/Developer/alphafind/portfolio_os.json) | Database of active Out-of-Sample Alphas with exact expressions, IDs, settings, and metrics. |
| **Verified Submittable Candidate Ledger** | [`submittable_alphas.csv`](file:///Users/phamtoan/Developer/alphafind/submittable_alphas.csv) | Master audit log of verified 8/8 PASS Alphas ready for scheduled submission. |
| **Strategic Portfolio Roadmap** | [`STRATEGIC_ROADMAP.md`](file:///Users/phamtoan/Developer/alphafind/STRATEGIC_ROADMAP.md) | In-depth breakdown of the 3-pillar scoring system, empirical milestones, and daily schedule. |
| **FASTEXPR Operators & Diagnostics** | [`docs/BRAIN_KNOWLEDGE_GRAPH.md`](file:///Users/phamtoan/Developer/alphafind/docs/BRAIN_KNOWLEDGE_GRAPH.md) | Comprehensive index of all 66 FASTEXPR operators, syntax constraints, and heuristic unsticking recipes. |
| **Factor Pillars & Field Schemas** | [`src/taxonomy.rs`](file:///Users/phamtoan/Developer/alphafind/src/taxonomy.rs) & `data/` | 6 Economic factor pillars (Valuation, Quality, Financial Health, Analyst Consensus, Options/Volatility, Microstructure). |
| **Portfolio Analytics & Audit Engine** | [`src/correlation.rs`](file:///Users/phamtoan/Developer/alphafind/src/correlation.rs) | Microsecond Pearson audits, disk-cached PnLs, and merged Sharpe simulator (`alphafind audit`, `alphafind portfolio`). |
| **Core Distributed Simulation Engine** | [`src/screener.rs`](file:///Users/phamtoan/Developer/alphafind/src/screener.rs) | 9-worker distributed screening across 3 accounts with auto-transfer lock (`alphafind screen`). |
| **Official Submission & Check CLI** | [`src/main.rs`](file:///Users/phamtoan/Developer/alphafind/src/main.rs) | Unified native binary CLI for 8/8 PASS verification and submission dispatch (`alphafind submit`, `alphafind check`). |
| **Ephemeral Sandbox** | `scratch/` (Git-ignored) | Disposable workspace for session research scripts, purged post-discovery. |
