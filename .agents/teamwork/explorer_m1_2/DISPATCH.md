## 2026-10-24T18:47:25Z

You are the News & Options Factor Theorist for Milestone M1 (Mathematical Factor Taxonomy & Specification Module).
Your working directory is: /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m1_2/
You must first read the original user request at: /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md
Also read:
- /Users/phamtoan/Developer/alphafind/PROJECT.md
- /Users/phamtoan/Developer/alphafind/AGENTS.md
- /Users/phamtoan/Developer/alphafind/.agents/teamwork/spec_miner_survey_2/survey_specs.md

Your task is to mathematically specify and detail:
1. RavenPack News (news18):
   - Sentiment divergence (mean_composite_sentiment_score vs mean_editorial_commentary_sentiment).
   - Corporate actions (mean_corporate_action_sentiment, mean_merger_acquisition_sentiment).
   - Earnings evaluation shocks (mean_earnings_evaluation_sentiment).
   - Intraday event streams (50 vector fields like nws18_ssc, nws18_nip requiring vec_avg/vec_sum folding and ts_backfill).
   - Mathematical equations, economic rationale (information diffusion, underreaction to earnings sentiment, media bias arbitrage).
2. Options Analytics (option9):
   - Breakeven asymmetry (call_breakeven_<T> vs put_breakeven_<T>, option_breakeven asymmetry).
   - Put-call forward price parity (forward_price_<T> / close - 1).
   - Term skew and slope (PCR volume and open interest term slope: pcr_vol_20d - pcr_vol_90d, pcr_oi_30d - pcr_oi_180d).
   - Mathematical formulations, economic hypotheses (informed options order flow, hedging pressure, tail risk pricing).
3. Detailed failure modes and mitigations for both families:
   - Media sensationalism, news clustering, and low-relevance noise.
   - Option liquidity drying up in illiquid names / outer horizons.
   - Post-earnings implied volatility crush and expiration calendar effects.
   - Look-ahead bias and non-trading day NaN handling.

Deliverables:
- Write detailed specifications to /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m1_2/taxonomy_news_options.md
- Write /Users/phamtoan/Developer/alphafind/.agents/teamwork/explorer_m1_2/handoff.md
- Send completion message to parent.
