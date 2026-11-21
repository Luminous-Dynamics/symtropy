# AV Active Sensing Decision-Value Benchmark v0

**Issue:** #6546  
**EVSI / observation value:** Symtropy #6222 / #6225  
**Temporal eligibility / replay:** Symtropy #6253 / #6201  
**Modality necessity:** Symtropy #6305 / PR #6307  
**Long-horizon memory:** Symtropy #6357 / PR #6359  
**AV consumer:** Symthaea #12837

## Objective

Evaluate whether a system chooses the next audiovisual observation because it is expected to improve a decision, not merely because it reduces uncertainty or maximizes raw information gain.

The initial tranche defines a typed action catalogue, causal deadlines, resource budgets, synthetic scenario workload, hidden action outcomes and a known-answer action-policy table. It contains no raw media or physical sensor execution.

## Research framing

Active Visual Perception (2026) argues that active sensing needs a common closed loop tying together missing task evidence, acquisition actions, expected observation value, action execution and state update. It warns that final task accuracy alone does not isolate the contribution of the sensing policy. Active Video Perception (AVP, 2025) is a strong external comparator using iterative plan–observe–reflect on long videos; its paper reports 5.7 percentage points higher average accuracy than the best agentic baseline across five long-video benchmarks, at 18.4% of inference time and 12.4% of input tokens in that comparison.

References:
- Active Visual Perception review: https://www.mdpi.com/1424-8220/26/20/6544
- AVP: https://arxiv.org/abs/2512.05774

## Scenario set

Eight synthetic scenarios cover high-EIG/zero-EVSI, low-EIG/high-EVSI, necessary non-speech audio, common-cause redundancy, a late observation beyond deadline, budget allocation across two requests, partial/unavailable sensor action and sufficient-evidence stopping.

- fixtures/agent/workload-v1.json exposes only the current observation, public action catalogue, deadline and resource caps.
- fixtures/evaluator-only/action-truth-v1.json contains action outcomes, decision value and optimal action/stop policy.
- fixtures/expected/known-answer-v1.json freezes the finite policy semantics.

This is a public repository; the evaluator-only truth is not secret or a blind set. Held-out evaluation needs restricted truth and process isolation.

## Core hypotheses

1. High information gain can have zero decision value; the optimal action is to stop.
2. A small observation can have low EIG but high EVSI if it changes a high-cost decision.
3. An observation delivered after deadline cannot improve the earlier decision.
4. Correlated views do not create independent corroboration.
5. A budget-aware policy should choose the best net decision value rather than simply the most uncertain query.
6. Partial/unavailable actions must not mint a delivered observation.
7. Once evidence is sufficient, further sensing should stop when expected benefit does not justify cost.

## Proposed metrics and targets

Report answer accuracy, safe-decision accuracy, cost-weighted regret, realized EVSI and net EVSI, action count, wasted/late acquisition, missed response windows, resource usage, modality necessity compliance, replay correctness and stop-decision correctness separately.

Research targets (not achieved):
- 100% finite action/outcome replay correctness.
- Zero fabricated deliveries, future evidence or post-deadline credit.
- Zero common-cause double-counting.
- No acquisition on the high-EIG/zero-EVSI fixture.
- Acquire the low-EIG/high-EVSI observation when it arrives before deadline and fits budget.
- At matched decision quality, eventually target 20% lower measured AV input/energy than always-on full-fidelity A+V.

The finite fixtures are evaluator mechanics, not model results or SOTA comparisons.

## Ownership

This benchmark reuses #6222/#6225 for EVSI, #6201/#6253 for replay and time eligibility, #6307 for modality ablations, #6359 for long-horizon memory and #12837 for the AV consumer. It does not add a scheduler, EVSI engine, clock service, sensor fusion engine or media producer.

## Deterministic evaluator and reference policy (21 November 2026)

The first evaluator implementation now lives at:

\`tools/av-modality-necessity-evaluator/active_sensing_evaluate.py\`

Run it from the repository root:

    python3 tools/av-modality-necessity-evaluator/active_sensing_evaluate.py \
      --workload research/av-active-sensing-decision-value-v0/fixtures/agent/workload-v1.json \
      --truth research/av-active-sensing-decision-value-v0/fixtures/evaluator-only/action-truth-v1.json \
      --submission research/av-active-sensing-decision-value-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-active-sensing-reference-scorecard.json

The synthetic reference policy requests only three actions:
- the low-EIG/high-EVSI hazard cue;
- the non-speech machine audio needed for the machine state;
- the cell B alarm cue under a one-action budget.

It stops on high-EIG/zero-EVSI, common-cause redundancy, late-only evidence, partial/unavailable sensors and sufficient existing evidence.

The frozen reference scorecard expects 8/8 correct answers, 8/8 correct acquisition/stop decisions, zero decision regret, three acquisitions, zero wasted/late acquisitions, zero budget/evidence violations, mean realized EVSI 1.875, and 16,384 catalogue audio bytes at 1.3 catalogue energy-cost units. The latter are **scenario catalogue costs**, not measured energy, and are not a real system resource result.

The evaluator checks the candidate's actual requested action sequence against the evaluator-optimal action sequence, validates action request IDs and max-use caps, verifies outcome arrival against action latency, enforces budgets, admits only requested observations, rejects fabricated and post-deadline citations, requires action-derived evidence citations where truth specifies them, and rejects common-cause corroboration claims. A late observation preserves its identity but cannot support a timely decision. A partial/unavailable outcome cannot mint an observation reference.

Candidate EIG/EVSI/cost estimates are checked for finite values and valid ranges but are **not** trusted as realized EVSI. Realized EVSI comes from evaluator truth and only counts distinct within-limit, delivered, on-time and independent evidence. A repeated over-limit request cannot multiply the realized value of one observation.

The reference policy is hand-authored synthetic mechanics, not an ML model run or external benchmark result. The evaluator-only truth and action outcomes live in this public repository and are not a blind test set. A real blind test needs evaluator-controlled truth/media, process isolation and restricted credentials.

## Comparators and research ladder

Run matched baselines under identical observation availability, candidate action catalogue, initial state, deadlines, model/runtime revisions, random seeds, context and resource limits:

1. passive always-on A+V;
2. fixed-cadence sampling;
3. random acquisition under the same budget;
4. maximum expected information gain (EIG);
5. myopic expected value of sample information (EVSI);
6. deadline-aware net EVSI / stop policy;
7. budget-aware multi-query value allocation;
8. AVP-style iterative plan–observe–reflect;
9. Symthaea native and hybrid evidence-qualified policies.

The core scientific comparison is whether EVSI-aware policies improve *downstream decision quality and deadline correctness* at lower acquisition cost than EIG, fixed sampling and AVP-style baselines. Keep these dimensions separate:
- decision accuracy and cost-weighted regret;
- actual action sequence / stop correctness;
- realized EVSI and candidate estimate calibration;
- wasted/late acquisitions and missed response windows;
- evidence citations and temporal admissibility;
- audio bytes, frames, tokens, wall time and energy when measured.

A future larger benchmark should add multi-step episodes where the system observes, re-estimates, chooses a second action, handles partial outcomes, changes its belief and stops. The current eight episodes are finite protocol scaffolding only. Reproduce AVP's official five-benchmark setup and the official DIVE response-window protocol before comparing published results; do not equate this synthetic scorecard with either benchmark.

## Paradigm shifts to investigate

- **Decision-value control, not uncertainty chasing:** request only evidence that can alter the action or reduce decision loss enough to justify cost.
- **Active sensing with a stop action:** stop when marginal net EVSI is non-positive or remaining latency exceeds the response window.
- **Causal sensor routing:** treat observations as interventions in an acquisition policy, with source dependencies and delivery/eligibility distinct from requests.
- **Anytime evidence acquisition:** use a cheap native path for immediate safe decisions while bounded specialist requests complete in the background; never let late work retroactively justify an earlier action.
- **Evidence-qualified control:** every decision-changing acquired cue must be cited and admissible; no answer may borrow hidden or undelivered outcomes.
- **Pareto-efficient intelligence:** compare decision quality, deadline hit rate, energy, input bandwidth, compute and privacy/interruption cost instead of a single scalar score.

Research targets (not achieved):
- 100% finite action/outcome replay and zero fabricated delivery or future-evidence use.
- Zero post-deadline evidence credit and common-cause overcounting.
- No action for high-EIG/zero-EVSI and a timely action for low-EIG/high-EVSI.
- At matched decision quality, eventually target at least 20% reduction in measured AV input/energy versus always-on full-fidelity A+V.
- On a powered, rights-qualified held-out cohort, statistically supported regret or response-window gains over fixed, random, EIG and AVP baselines.


### What the first executable evaluator qualifies

The current evaluator is tools/av-modality-necessity-evaluator/active_sensing_evaluate.py; run instructions and claim boundaries are in EVALUATOR.md. It checks exact action sequences, unique request identity, per-action max uses, scenario action arrival time, per-episode budgets, actual admitted observation references, future/post-deadline citation, action-required source citations, common-cause double counting, regret and realized EVSI.

Candidate-supplied EIG/EVSI estimates are not treated as ground truth. The evaluator's realized EVSI comes from the separate hidden scenario outcome, and only a unique, within-limit, on-time, delivered independent result earns value. A second over-limit request cannot double-count the same observation's EVSI.

The finite synthetic reference requests three actions and stops in the other five episodes. Its expected known answer is 8/8 task answers, 8/8 optimal action/stop sequences, zero regret, three acquisitions, no wasted/late acquisitions, no budget/evidence violations, and mean realized EVSI 1.875. Scenario catalogue costs sum to 16,384 audio bytes and 1.3 energy-cost units; these are synthetic configured costs, not measured hardware telemetry.

The evaluator and fixture now have adversarial tests for high-EIG/zero-EVSI waste, missing low-EIG/high-EVSI evidence, late delivery, fabricated observations, required action-evidence omission, common-cause dependence, resource overrun, partial sensor outcomes, duplicate request and EVSI over-credit. The exact-head Actions run is the qualification gate; no model result is claimed.


## External active-perception comparison (21 November 2026)

Two visual active-perception systems should be primary baselines, in addition to the general long-video suites:

- **AVP (CVPR Findings 2026):** an iterative plan–observe–reflect loop. Its paper reports the best overall result across five long-video benchmarks and +5.7 percentage points average accuracy over the best agentic baseline, at 18.4% of inference time and 12.4% of input tokens in its reported comparison. Reproduce its official benchmark setup and compare under its original metric and cost definitions.
- **VAP (May 2026):** training-free active keyframe acquisition using a text-conditioned video-generation prior. The paper reports zero-shot SOTA on EgoSchema, NExT-QA, ActivityNet-QA, IntentQA and CLEVRER, with up to 5.6× frame efficiency over the cited baselines. It is a strong visual-only keyframe-selection comparator, not a multimodal physical sensor-control result.
- **Active Visual Perception review (15 October 2026):** recommends separating passive/fixed and random acquisition baselines, greedy versus horizon-aware policies, before/after observation quality, downstream task change, view/action count, time, energy, compute/communication, robustness to noise/delay/loss/occlusion and stopping quality.

Sources:
- AVP CVPR paper: https://openaccess.thecvf.com/content/CVPR2026F/html/Wang_Active_Video_Perception_Iterative_Evidence_Seeking_for_Agentic_Long_Video_CVPRF_2026_paper.html
- AVP project: https://activevideoperception.github.io/
- VAP paper: https://arxiv.org/abs/2605.01662
- Active Visual Perception review: https://www.mdpi.com/1424-8220/26/20/6544

### Current implementation boundary

The v0 executable evaluator is a **single decision-round action-selection harness**. It scores one ordered list of action requests against synthetic action outcomes; it does not yet give the candidate an observation, wait for a replan, and then issue a second request in an interactive session. That multi-round event-sourced executor is tracked separately in [#6555](https://github.com/Luminous-Dynamics/symtropy/issues/6555). Do not describe PR #6549 by itself as a complete interactive AVP-like runtime.

The first matched external evaluation should use AVP's five official long-video benchmarks and VAP's official keyframe-selection protocol, with identical backbone, candidate observation budget, prompt/context, decoding, hardware and repeated seeds where possible. Then add our orthogonal tests for action latency, physical modality availability, EVSI versus EIG, required evidence citations, common-cause dependence, deadline validity and audio/video acquisition. This makes the claim falsifiable: better decision quality at matched acquisition cost, or non-inferior quality with lower bytes/frames/tokens/energy—not a high score on eight synthetic cases.
