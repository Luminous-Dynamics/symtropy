# VIS-ACT-001 — Visual Active-Action Decision Frontier

**Status:** proposed protocol; no policy result or SOTA claim is qualified by this document.  
**Issue:** [Symtropy #2239](https://github.com/Luminous-Dynamics/symtropy/issues/2239)  
**Scope:** deterministic simulator evaluation of choosing between more compute, a new observation/view, and abstention.

## 1. Research question and paradigm shift

A visual agent can often improve its input evidence in more than one way. It may spend more compute on the current frame, wait for a new temporal sample, move to a different viewpoint, adjust a legal sensor parameter, use a second sensor, or abstain. These actions have different consequences and costs.

The benchmark asks:

> Given a task, current permitted belief, feasible next actions, and remaining resource budgets, which action most improves the *downstream decision* after execution?

This is a composition of existing work—not a new global value-of-information system. Reuse:

- [Symthaea active perception #607](https://github.com/Luminous-Dynamics/symthaea/issues/607) for hypothetical proposal versus executed observation, predicted versus realized gain, and spatial objective semantics.
- [Vision Mind #3934](https://github.com/Luminous-Dynamics/symthaea/issues/3934) for typed provenance, structured visual evidence, world-state belief, active vision, and closed-loop qualification.
- [VIS-REP-001 #8152](https://github.com/Luminous-Dynamics/symthaea/issues/8152) and [protocol PR #8153](https://github.com/Luminous-Dynamics/symthaea/pull/8153) for representation ablations and cost planes.
- [VIS-DVPC-001 #8156](https://github.com/Luminous-Dynamics/symthaea/issues/8156) / [offline reference PR #8163](https://github.com/Luminous-Dynamics/symthaea/pull/8163) for downstream decision-value arithmetic.
- [ANIMAL-VOI-002 #2171](https://github.com/Luminous-Dynamics/symtropy/issues/2171) for active sensing under information and energy budgets.
- [Blind visual evaluator PR #1214](https://github.com/Luminous-Dynamics/symtropy/pull/1214), anti-shortcut [#1281](https://github.com/Luminous-Dynamics/symtropy/issues/1281), causal-stream blocker [#1282](https://github.com/Luminous-Dynamics/symtropy/issues/1282), and evaluator-side decision-value work [#2185](https://github.com/Luminous-Dynamics/symtropy/issues/2185) for hidden-truth isolation and causal delivery.

Do not duplicate the existing evidence, receipt, experiment-plan, authority, or VOI contracts. Unqualified predecessors remain blockers to dependent claims.

## 2. Explicit action vocabulary

A policy may choose only among evaluator-declared feasible actions:

1. `KeepCheap`: retain current cheap output and make the best allowed decision from it.
2. `EscalateCurrent`: spend more compute on the current observation.
3. `AcquireNext`: request the next causal time sample.
4. `ChangeView`: propose a legal camera pose, crop, zoom, exposure, or second-sensor acquisition according to the scenario's declared action set.
5. `WaitForEvent`: wait for a declared sensor/event condition with an explicit deadline cost.
6. `AbstainOrHandoff`: decline to decide, ask for human review, or take a safe fallback as defined by the task.

A proposal is not execution. A view action creates no new visual evidence until the environment executes it and emits an observation through the allowed sensor channel. A view that is infeasible, prohibited, redundant, or never executed earns no evidence-gain credit.

This protocol does not grant real-world actuation authority. The first implementation is a deterministic simulator fixture.

## 3. Cost accounting and comparison fairness

Do not force unlike resources into one arbitrary scalar. Keep independently measured dimensions and compare the feasible Pareto frontier:

- end-to-end latency, deadline misses and time-to-correct-decision;
- inference work or FLOPs (explicitly marked as estimates/proxies when not measured);
- peak memory;
- actual energy only from a documented, calibrated measurement setup;
- views/sensor samples and data/communication volume;
- duration and physical actuation effort for executed viewpoint changes where measured;
- task loss, failure severity and recovery cost.

For each resource dimension, state the unit, measurement method, missingness semantics, budget and measurement uncertainty. Missing is `UNKNOWN/NOT_MEASURED`, never zero. Do not claim “energy efficient” from sparse events, FLOPs, GPU time or frame rate alone.

Compare policies with matched start states, intervention assignments and seeds. Use matched feasible budgets or plot a frontier; do not compare a policy allowed free extra observations to one charged for all compute and sensing.

## 4. Baseline policies

Evaluate at least:

- passive one-frame / cheap current-result baseline;
- always expensive current-frame perception;
- fixed periodic view sweep or static coverage;
- random action among feasible actions, with the same budget;
- confidence-threshold trigger;
- surprise-only trigger;
- predicted information gain / expected value of sample information (EVSI) policy;
- a pre-action downstream-decision-value predictor;
- a hybrid which can choose between current-frame compute, new view/sample and abstention;
- evaluator-only clairvoyant upper bound.

The oracle is diagnostic only and must never enter the eligible leaderboard. Train or tune predictors only on disjoint development scenes/seeds. Confidence, entropy, object count, visual similarity, and information gain are diagnostic features—not substitutes for realized task value.

## 5. Synthetic intervention matrix

Start with deterministic, small, independently authored expected outcomes:

| Scenario family | Distinguishes |
| --- | --- |
| Near-identical target identities | Whether a changed view beats more compute on the same aliased pixels |
| Hidden persist/remove event and reappearance | Whether the model revises belief from causal new evidence rather than future leakage |
| Crossing tracks under partial occlusion | Whether another view improves identity association |
| Parallax/depth ambiguity | Whether viewpoint change resolves geometry |
| Blur / exposure / high dynamic range | Whether another exposure or time sample beats compute escalation |
| Delayed or misleading distractor | Whether the policy resists premature commitment and updates after disconfirmation |
| Repeated, correlated views | Whether redundant observations are incorrectly counted as independent information |
| Unavailable or unsafe candidate view | Whether feasibility and authority constraints dominate information score |
| No informative feasible action | Whether abstention can be the optimal permitted action |
| Escalation harms or wastes resources | Whether a larger model is avoided when it worsens the downstream choice |
| Resource-overload scenario | Whether the policy respects trial and run budgets and degrades deterministically |
| Held-out scene/appearance factors | Whether learned action choices generalize beyond development seeds |

Use the 48-case pilot only to validate fixture plumbing, diagnostics and negative controls; it is not a statistically powered superiority study. Before confirmatory runs, preregister the independent episode unit, sample size/power method, primary loss, practical effect threshold, non-inferiority margin, confidence interval method, held-out splits, budgets, and stop rule.

## 6. Causality and anti-shortcut invariants

The orchestrator owns the hidden trial and advances observations one step at a time:

`policy history -> committed proposal -> feasibility check -> action execution -> fresh sensor observation -> belief update`

The policy must not receive:
- future frames or the backing vector/slice of future sensor frames;
- canonical entity identity, intervention assignment, scenario seed, hidden branch, or oracle association;
- the outcome of an unselected alternate action;
- any evaluator-only counterfactual before the action choice and relevant output are committed.

A producer-owned iterator is not sufficient if the producer can advance it repeatedly before submitting an output. Use orchestrator-controlled causal release, or an equivalently enforced capability boundary. Reuse the causal-stream work in #1282; do not mutate frozen #1214 while that gate is unresolved.

Required negative controls deliberately attempt to leak future frames, hidden identity, alternate-view outcomes, renderer state and unexecuted observations. Every such path must be rejected or demonstrably unable to influence the policy. The renderer is a generator of world consequences, not an untracked evidence side channel.

## 7. Metrics and analysis

Report separate planes; never hide a critical failure inside a composite scalar:

**Task value**
- task success and declared downstream loss;
- failure severity, recovery time and time-to-correct-decision;
- paired policy-minus-baseline effects and uncertainty over independent episodes;
- regret relative to a clearly labeled evaluator-only oracle ceiling.

**Belief/evidence quality**
- prediction calibration where a reference distribution exists;
- false identity persistence, missed identity revisions and association errors;
- predicted versus realized gain from *fresh* evidence;
- abstention frequency and utility of abstention;
- false updates from absent, incompatible, redundant or causally unavailable evidence.

**Action and resource quality**
- infeasible/unexecuted proposals, wasted views, deadline misses and overloads;
- per-resource use, measurement completeness and budget violations;
- risk/quality/cost frontiers with per-stratum outcomes.

Use HOTA/IDF1 only when the submitted outputs and scorer support their actual definitions. Use segmentation metrics only if masks are really emitted and compared. Centroids, points and synthetic belief labels do not magically qualify as those metrics.

## 8. Proposed qualification gates (targets, not results)

1. Independently authored synthetic expected outcomes match 100%.
2. Hidden-state/future-frame negative controls detect all deliberately injected leakage paths.
3. No credit for information from an unexecuted, infeasible or causally unavailable observation.
4. No controller path to hidden world truth or counterfactual action outcomes before commitment.
5. At least one preregistered held-out stratum shows a practically meaningful downstream-loss improvement over the strongest eligible non-oracle baseline, with its interval clearing that threshold.
6. Designated safety-critical strata meet a predeclared non-inferiority bound; if no informative feasible action exists, abstention/fallback is evaluated explicitly.
7. The proposed policy is Pareto-better on at least one decision-quality/resource trade-off without suppressing failures or material regressions in an averaged score.
8. Exact-head reproducible receipts include code, scenario/config identity, data/seed family, policy and model versions, resource measurements, failures and all incomplete lanes.

These are proposed gates—not achieved performance. A failed, queued, skipped or missing run is not a pass. This document alone establishes neither active-vision benefit nor superiority to any external model.

## 9. Research context and external comparators

- Li et al., **Active Visual Perception: Opportunities and Challenges**, *Sensors* (15 Oct 2026): https://www.mdpi.com/1424-8220/26/20/6544. Motivates separating sensing evidence, downstream task improvement and sensing cost.
- Pham Cong & Luong, **The Decision Value of Perception Compute** (28 Sep 2026): https://arxiv.org/abs/2609.35910. Motivates measuring downstream decision impact rather than assuming more perception compute is useful. Its findings are specific to its tested tasks and are not evidence of Symthaea performance.
- **ActiveArena: Benchmarking and Understanding Active Perception in Robotic Manipulation** (2026): https://arxiv.org/abs/2609.24124 and https://leeibo.github.io/ActiveArena/. Provides relevant multi-round evidence-acquisition and distribution-shift context.
- **SAM 3.1** (official release notes, 27 Mar 2026): https://github.com/facebookresearch/sam3/blob/main/RELEASE_SAM3p1.md. A useful external task-level comparator for eligible text-prompted video tracking/segmentation tasks; published results are mixed across benchmarks. It is not a drop-in comparison for action-selection regret or embodied decision value.

### External reference points—not directly comparable leaderboard numbers

| Reference | Published signal | Proper use here | Limitation |
| --- | --- | --- | --- |
| [ActiveArena](https://arxiv.org/abs/2609.24124), 2026 | Its project page reports 35 tasks with ID/OOD splits; the strongest listed ActiveArena-OFT row reports 62.97% ID / 47.60% OOD macro task success, while MemER reports 35.31% / 23.43%. | A useful active-viewpoint, memory, multi-round evidence-acquisition and OOD reference. Compare task families, ID/OOD gap, task macro-success and action/sensing budget when compatible. | External methods retain their method-specific inputs and this is not a matched-architecture comparison. Do not claim a direct win from a different scene, action space or resource budget. |
| [SAM 3.1 official release notes](https://github.com/facebookresearch/sam3/blob/main/RELEASE_SAM3p1.md), 27 Mar 2026 | The release reports a +2.0 J&F change on MOSEv2 val (60.3 to 62.3), while its video prompt-segmentation results are mixed across metrics/datasets. | Strong external task-level comparator where the task is genuinely video object segmentation/tracking and Symtropy emits matching masks/tracks. | Not a comparator for action-selection regret or hidden-state belief revision without a compatible task adapter. Vendor-published results are not local measurements. |
| [The Decision Value of Perception Compute](https://arxiv.org/abs/2609.35910), 28 Sep 2026 | In its geometry/planning experiments, the authors report 34–54% of escalations that changed downstream loss made it worse; at a 20% latency budget, only lightweight routers beat random after charging allocator compute. | Strong motivation for an explicit decision-value baseline and allocator-cost accounting. | These results are specific to the authors' tested geometry/planning tasks, models and budget definition; they are not Symthaea results or a transferable target score. |

These figures are research reference points to ground benchmark design, not acceptance thresholds for unlike tasks. Before any apples-to-apples claim, pin dataset/version and license, task definition, exact model/checkpoint, inference configuration, preprocessing, seed protocol, hardware, measurement method and failure accounting.

Select external comparators by task compatibility and license. Freeze checkpoint, inference settings, hardware, preprocessing and measurement procedure; distinguish vendor-reported throughput from independently measured local throughput. Do not claim “better than SOTA” based on unrelated tasks or hardware.

## 10. Implementation order

1. Freeze the protocol and validate action/cost fields without a learned policy.
2. Build tiny deterministic fixtures, with hidden-truth and leakage negative controls.
3. Implement the evaluator-side post-commit paired counterfactual scorer, reusing existing evaluator contracts.
4. Run baseline policies and verify budgets/receipt completeness.
5. Establish confirmatory power and run a held-out seed/scene tranche.
6. Only after synthetic qualification, test a calibrated real sensor or robotics task with declared safety/actuation, timing, energy, licensing and data-rights evidence.

No stage assumes that currently open PRs, queued workflows, simulated fixtures or offline arithmetic already establish model competence.
