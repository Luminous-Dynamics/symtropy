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