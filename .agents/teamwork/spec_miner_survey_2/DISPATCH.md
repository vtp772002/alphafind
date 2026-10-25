## 2026-10-24T18:33:40Z
[Message] timestamp=2026-10-24T18:33:40Z sender=8b020a34-8c2d-4255-98f9-544909ff7652 priority=MESSAGE_PRIORITY_HIGH content=You are the BRAIN Specification Miner for the quantitative alpha research framework.
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/spec_miner_survey_2/
You must first read the original user request at: /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
Also read /Users/phamtoan/Developer/alphafind/AGENTS.md and /Users/phamtoan/Developer/alphafind/docs/ (e.g., BRAIN_RULES_AND_SUBMISSION.md, FASTEXPR.md or any other docs).

Your task is to thoroughly mine specifications and domain requirements for WorldQuant BRAIN Alternative Datasets and FastExpr:
1. Alternative datasets: model51 (Systematic Risk), model16 (Fundamental Scores & Derivative Accel), news18 (Ravenpack News), option9 (Options Analytics).
   - What fields exist or are typical in BRAIN for these datasets? Check data/ (e.g. data/fields_*.json if present, docs/, or any reference lists).
   - What are their data structures (matrix vs vector), dimensions, historical coverage, and missing data behavior?
2. FastExpr syntax, operators, and rules:
   - What mathematical functions, time-series transforms (ts_decay_linear, ts_rank, etc.), non-linear transforms (signed_power, rank, winsorize), and group neutralizations (SUBINDUSTRY, INDUSTRY, SECTOR) are supported and valid in FastExpr?
   - How are vector vs matrix fields operated on (vector operators, folding, indexing)?
3. BRAIN API `/data-fields` endpoint specification:
   - Query parameters, schema response structure, caching format, validation requirements.
4. Economic rationales and failure modes:
   - Detail economic hypotheses, rationale, and potential failure modes (regime shifts, liquidity traps, crowding, decay, look-ahead) for each of the 4 dataset families.

Deliverables:
- Write your detailed findings to /Users/phamtoan/Developer/alphafind/.agents/teamwork/spec_miner_survey_2/survey_specs.md
- Write /Users/phamtoan/Developer/alphafind/.agents/teamwork/spec_miner_survey_2/handoff.md with verified specifications, schemas, and factor ideas.
- Send a message to your parent (orchestrator) notifying completion.
