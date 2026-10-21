# Resident preference probe v0.1

**Status:** synthetic, deterministic reference implementation; current branch still requires exact-head formatting, tests, Clippy and benchmark execution.

## Why add a question selector?

A Pareto frontier preserves non-dominated choices; it does not tell the system which trade-off a particular resident or household wants. Choosing a fixed weighted sum would hide the assumption that the selected weights are the resident's true preferences. Showing every alternative shifts the burden to the resident and may create unnecessary cognitive work.

The v0.1 probe accepts a finite set of explicit plausible preference-weight scenarios. It asks which declared pairwise comparison has the largest **worst-case decision-regret reduction**, while keeping question effort and duration as separate Pareto dimensions. There are no scenario probabilities, so this is not expected value of information.

## Exact integer model

Each alternative carries upstream-normalized criterion preference scores in `0..=10_000` basis points, with higher scores preferred. Each model has criterion weights in basis points summing exactly to 10,000. Utility is the integer numerator

`U_s(a) = sum_j score(a,j) * weight(s,j)`

with no floating-point arithmetic or hidden re-normalization. The maximum utility numerator is 100,000,000. Score normalization and criterion admission belong upstream; this module does not prove that those scores describe a real resident.

Let `best(s)` be the highest alternative utility in scenario `s`. For any non-empty scenario subset `S`, define minimax regret

`R(S) = min_a max_{s in S} [best(s) - U_s(a)]`.

For a pairwise question, scenarios are divided into left-preferred, right-preferred and indifferent groups according to the supplied model. The post-answer worst-case regret is the largest `R(S_k)` across the non-empty groups. Gross reduction is the pre-question regret minus this post-answer value. The partition cannot increase regret because each possible answer leaves a subset of the original scenario set and the decision can be re-optimized.

The report also computes an exact Pareto frontier over:
- maximize worst-case regret reduction;
- minimize declared query-cost/effort units;
- minimize declared query-duration units.

It does not scalarize these into one score or declare one question universally best.

## Fail-closed input rules

- Candidates must be hard-verified by the caller's independent verifier; Blocked or Indeterminate alternatives are rejected.
- Candidate revision SHA-256 syntax, receipt SHA-256 syntax and receipt-to-revision equality are checked. The receipt is not authenticated or recomputed here.
- Criterion IDs, candidate IDs, scenario IDs and question IDs must be unique and stable.
- Candidate scores and scenario weights must fit the declared basis-point range. Scenario weights must sum exactly to 10,000.
- Dimensions must match, questions must reference two distinct known alternatives, and duplicate unordered pairs are rejected.
- The finite problem has explicit bounds for objective, alternative, scenario and question counts.

The model assumes answers consistent with one of the provided utility scenarios. Human answers can be noisy, context-sensitive, inconsistent, or outside the model set; v0.1 does not learn or authenticate preferences. Production use would require explicit consent, opt-in data handling, an indifference/noise policy, preference drift handling, and an appeal/change mechanism.

## Test and benchmark protocol

Unit tests include a known-answer minimax-regret case, no-information cases, fail-closed verification and receipt controls, weight and dimension validation, input-order invariance and generated small instances. A separate direct reference recomputes utilities instead of reusing the production cache; every generated question's answer partition, baseline regret, post-answer regret and reduction is compared against that reference.

The scaling probe covers these synthetic workloads:

| Alternatives | Preference scenarios | Criteria | Pairwise questions |
|---:|---:|---:|---:|
| 8 | 8 | 4 | 28 |
| 16 | 32 | 8 | 120 |
| 32 | 64 | 8 | 496 |
| 48 | 128 | 16 | 1,128 |

Input generation is outside the timed region; the timed API includes validation, scenario utility construction, question evaluation, Pareto-frontier assembly and report generation. It reports p50/p95/max, sample counts, query count, minimax regret and frontier cardinality. These synthetic timings are not latency guarantees and do not measure cognitive load, preference accuracy or field outcomes. Peak memory should be added before operational scaling claims.

## Research and industry comparison

- **Function2Scene (2026)** constructs functional constraints around occupant tasks and uses measured/tool-grounded checks during scene refinement. Its public report's 94.3% aggregate figure is a participant preference rate over pairwise rendered comparisons, not a safety or hard-feasibility rate: https://function2scene.github.io/ and https://arxiv.org/abs/2605.30819.
- **SceneEval-500 (WACV 2026)** has 500 descriptions and evaluates explicit scene requirements and implicit plausibility such as collision and navigability. It is a scene-evaluation baseline, not a preference-question selection benchmark: https://github.com/3dlg-hcvc/SceneEval.
- **Bayesian Preference Elicitation for Decision Support in Multi-Objective Optimization** uses interactive pairwise comparisons and a Bayesian utility model to guide query selection: https://doi.org/10.1002/mcda.70019.
- A 2026 survey of preference-guided multi-objective optimization emphasizes that query/model strategy depends on objective count and whether preferences are explicit or uncertain: https://doi.org/10.1007/s10472-026-10012-6.

The novelty claim here is intentionally limited: a dependency-light, exact finite-model minimax-regret baseline with integer arithmetic and a separate query-cost/duration frontier. Preference elicitation itself is established research.

## Proposed gates (targets, not results)

1. 100% exact agreement with the direct reference on the frozen, tractable generated corpus.
2. Zero candidate admission when hard feasibility is Blocked/Indeterminate or receipt-to-revision binding is stale.
3. 100% deterministic report under permutations of alternatives, scenarios and question input order.
4. Every question must satisfy `post regret <= baseline regret`; no negative "gross reduction" is accepted.
5. Report sample counts and workload sizes; use independent reference agreement rather than a scalar benchmark score.
6. Later, run consent-based paired user studies to measure regret after a fixed question budget, number of questions, time, cognitive load, accessibility, consistency under repeated questioning and participant-reported agency. Compare against random questioning, asking every pair and an established Bayesian preference-elicitation baseline when implementations/data/licensing permit.

## Claim ceiling

The probe computes the exact consequences of an explicit finite utility-model set. It does not establish that those models are adequate, recover any person's true values, prove layout safety/accessibility, establish a human outcome, or show external SOTA superiority. These must be evaluated separately under [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766) and [INTERIOR-E2E-EVAL-001 #2516](https://github.com/Luminous-Dynamics/symtropy/issues/2516).
