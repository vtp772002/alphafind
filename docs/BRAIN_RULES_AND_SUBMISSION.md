# WorldQuant BRAIN: rules, scoring, and submission

**Checked:** 2026-10-12. Platform rules and score algorithms can change; confirm them in the signed-in BRAIN interface before acting. The regular Challenge score description below was also checked directly against the BRAIN API using the main account configured in `.env`.

This page separates rules shown by WorldQuant from AlphaFind's own research preferences. It is not a promise that a submission will be accepted or score well.

## Which competition is this?

BRAIN has more than one competition surface. The regular **WorldQuant Challenge** and a separately registered event such as the **International Quant Championship (IQC)** do not share one guaranteed scoring formula.

The authenticated `OPTIONS /competitions/challenge/boards/leader` schema describes `Total Score` as an **equally weighted score across ranked values of Uniqueness, IS Score, and Days of Submission**. In compact form:

`Total Score = (rank_score(Uniqueness) + rank_score(IS Score) + rank_score(Days of Submission)) / 3`

This expresses the API's equal weighting; it is not a claim that BRAIN publishes this exact implementation formula. The schema does not specify how ranks are normalized into component scores, how ties are handled, or whether the leaderboard uses a particular percentile transform. It does say lower `Uniqueness` means higher uniqueness, so better performance on that component corresponds to lower merged-PnL correlation with other users. It describes `Days of Submission` as submissions in the last 60 days and `IS Score` as combined portfolio performance based on equal weighting of submitted alphas. The competition resource's `scoring: "PERFORMANCE"` field is a scoring category label, not the formula.

The regular Challenge page's guidelines also say that only alphas qualifying for out-of-sample (OOS) testing are scored; qualifying alphas are evaluated relative to other alphas in the system; existing scores may change with OOS performance; and WorldQuant may change the scoring algorithm. The API documents the three equally weighted ranked components, but not their precise rank-to-score transformation.

The 2026 IQC is a distinct team competition. Its official page says scoring is relative to other qualifying IQC alphas, individual team-member scores are aggregated, quality and quantity both matter, and methodology may vary by stage. These rules must not be used as a substitute for the regular Challenge rules.

## Regular Challenge rules visible in BRAIN

The current in-product *WorldQuant Challenge Guidelines* state, in summary:

- Participants must meet the stated age and eligibility requirements, use one active account, and must not share an account or account ID.
- Alphas count for scoring only when they qualify for OOS testing. Alphas created before joining the competition are not counted under the displayed guideline.
- Scoring compares qualifying submissions against others. The current leaderboard API schema says Total Score equally weights the ranked values of Uniqueness, IS Score, and Days of Submission.
- Existing scores can rise or remain unchanged as OOS results are evaluated periodically.
- WorldQuant reserves the right to change the scoring algorithm and competition rules.
- Consultant opportunities are discretionary; reaching a level or rank does not guarantee an offer or compensation.

The current Challenge page also says top rankers may receive consultant invitations. Treat that as a possibility, not a guarantee.

## Alpha submission criteria shown in Learn

The signed-in BRAIN Learn article [Clear these tests before submitting an Alpha](https://platform.worldquantbrain.com/learn/documentation/interpret-results/alpha-submission) currently lists these six criterion groups:

| Criterion | Threshold shown in Learn | Notes |
|---|---|---|
| Fitness | Greater than 1.3 for Delay 0; greater than 1 for Delay 1 | The article calls this at least “Average”. |
| Sharpe | Greater than 2 for Delay 0; greater than 1.25 for Delay 1 | Keep the delay-specific thresholds separate. |
| Turnover | Greater than 1% and less than 70% | Both boundaries are strict in the displayed table. |
| Weight | Maximum weight in any stock below 10% | It also checks that enough stocks receive weight for enough days; the count depends on the universe. |
| Sub-universe | Sharpe must clear a universe-scaled threshold | Learn gives `subuniverse_sharpe >= 0.75 * sqrt(subuniverse_size / alpha_universe_size) * alpha_sharpe`. |
| Self-correlation | PnL correlation below 0.7, or Sharpe at least 10% above each more-correlated submitted Alpha | Learn describes a four-year window for self-correlation; this is not a fixed 1,236-day requirement. |

This is the current Learn article's public-facing table, not a guarantee that every API response contains exactly six records. The API's check payload and the UI may expose additional status or configuration checks. Always inspect the actual response for the alpha and settings being submitted.

## What the thresholds do not say

The table does **not** establish a platform rule that every candidate must have a particular return, a specific preferred turnover band, a particular truncation value, an internal-portfolio correlation below 0.15, or exactly one submission per UTC day. Those can be local research policies, but label them as such and justify them with data.

Likewise, the platform's self-correlation criterion and AlphaFind's pairwise correlation against a locally assembled OOS portfolio are different checks. Neither local Pearson correlation nor a synthetic crowd estimate is the platform's `Uniqueness` score.

## Recommended submission workflow

1. Use one account belonging to the participant. Do not share credentials or operate multiple accounts for one individual; follow the current BRAIN agreement and competition rules.
2. Confirm region, instrument, delay, universe, neutralization, pasteurization, and other settings in the actual simulation. Do not infer them from a project default.
3. Simulate the candidate and read the platform's current results and all check details. Distinguish Delay 0 from Delay 1; treat `PENDING`, `ERROR`, unknown, or missing checks as unresolved rather than passing.
4. Submit only when the platform UI/API confirms eligibility for OOS. Recheck the alpha's OOS status in BRAIN after submission.
5. Track OOS behavior and internal portfolio overlap separately. Use local correlation and impact tools as diagnostics, with coverage and cache freshness visible.
6. Recheck the current Challenge scoring schema and leaderboard before optimizing. Use the three equally weighted ranked components documented by the API; treat rank normalization, tie handling, and any later rule changes as unspecified unless BRAIN documents them.

AlphaFind's check evaluator now fails closed: it requires every locally expected core check to be present and every returned status to be an explicit `PASS`, rejecting empty, `PENDING`, `ERROR`, unknown, and missing results. It is still not version-aware, its expected core list is maintained by hand, and it does not capture the raw platform response, so treat `alphafind check` and `alphafind submit` as convenience tooling rather than authoritative approval. `--skip-checks` is disabled unless `ALPHAFIND_ALLOW_SKIP_CHECKS=1` is set, and local preflight never replaces BRAIN's own UI/API status.

## Official sources

- [WorldQuant BRAIN Challenge](https://platform.worldquantbrain.com/competition/challenge) — signed-in page with current leaderboard, Guidelines, and FAQ.
- [BRAIN API leaderboard endpoint](https://api.worldquantbrain.com/competitions/challenge/boards/leader) — send authenticated `OPTIONS` to inspect the schema used to verify Total Score's equally weighted ranked components and the leaderboard field descriptions (checked 2026-10-12).
- [BRAIN Learn: Alpha submission tests](https://platform.worldquantbrain.com/learn/documentation/interpret-results/alpha-submission) — current delay-specific criteria and test descriptions.
- [WorldQuant Challenge Guidelines](https://platform.worldquantbrain.com/competition/challenge) — open **Guidelines** on the Challenge page for account, eligibility, scoring, and rule-change terms.
- [International Quant Championship 2026](https://www.worldquant.com/brain/iqc/) and [official IQC Guidelines](https://www.worldquant.com/brain/iqc-guidelines/) — separate competition and stage-specific scoring rules.

