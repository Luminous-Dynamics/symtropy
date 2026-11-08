# Scan-to-BIM decision-sufficient capture v0.1

**Status:** design/benchmark charter; no field next-best-view planner or physical survey result is implemented.  
**Issue:** #4207 — SCAN-BIM-DECISION-CAPTURE-001.  
**Composes with:** #1892, #1922, #1607, #4086, #4062, #3331, #1226.  
**Research review:** 2026-11-08.

## Thesis: capture what changes the decision

The next scan should not be selected simply because it adds points, fills the largest area, or lowers a global uncertainty score. It should be selected because its possible outcomes can change a declared, admissible building decision.

Example: a dense frontal scan may add a large number of wall points without changing whether a doorway meets its approved clear-width requirement. A reachable oblique view may resolve the opening boundary and change the acceptance decision at much lower acquisition cost.

This is **task-conditional level of accuracy (T-LOA)**: the required geometry, semantics and topology are derived from an approved downstream decision and requirement revision. It is not one universal tolerance, and it must not weaken an engineering requirement.

## Research landscape

### Active reconstruction baselines

- **Active3D (AAAI 2026)** uses multi-level uncertainty and expected hybrid information gain for next-best-view selection, with a risk-sensitive motion planner. It reports strong reconstruction results on its evaluated 3D reconstruction benchmarks. This is a direct uncertainty-led baseline, not a building decision oracle. https://doi.org/10.1609/aaai.v40i8.37585
- **Zhu, Sellán & Terenin, SIGGRAPH 2026**, *A Bayesian Approach for Task-Specific Next-Best-View Selection with Uncertain Geometry*, uses Bayesian decision theory to choose scans based on downstream task utility rather than uniformly reducing uncertainty. Their evaluated tasks include semantic classification, segmentation and PDE-guided simulation. https://arxiv.org/abs/2605.05095
- **Luo et al. (2026)** review indoor Scan-to-BIM sensing through modelling and note the lack of mature, widely accepted end-to-end benchmarks covering mapping, perception, structural reconstruction and BIM generation. https://doi.org/10.1016/j.autcon.2025.106731

These methods are useful competitors. Their probabilistic expected-information/utility approaches must not be conflated with deterministic worst-case decision impact. Run separate tracks; use probabilities only after their prior, likelihood, calibration and applicability are independently evidenced.

### Commercial baseline

Leica Cyclone 3DR 2026 already provides per-element BIM-vs-scan tolerance checking, configurable tolerance/exclusion/coverage parameters, within/out-of-tolerance/no-data status, manual review and PDF/BCF exports. Therefore simple heatmaps, tolerance coloring and per-element deviation reports are not a new contribution. https://rcdocs.leica-geosystems.com/cyclone-3dr/2026.0/Ex_ToleranceChecking

The differentiator to evaluate is whether targeted acquisition can reduce total capture/review effort while preserving decision correctness, unknowns and evidence lineage across tools and asset lifecycle.

## Architecture and ownership

Build an adapter, not another planner:

**Scan-to-BIM hypotheses and reachable views → #1892 decision-impact cases/actions → #1922 adaptive policy tree → capture and evidence receipts**

- #1892 owns finite possible states, mutually exclusive/exhaustive outcome partitions, hard-first robust feasibility, Pareto recomputation, measurement cost/time bases and decision-impact vectors.
- #1922 owns adaptive follow-up trees and stop/reopen/escalate leaves.
- #1607 owns sensor pose, frame, calibration, registration and metrology evidence.
- #4086 owns element association, semantic/topological consistency and model mutation admission.
- #4062 owns known-answer building-condition fixtures and the independent oracle.
- #3331 owns persistent component identity and supersession.
- #1226 owns blind-run event ordering and tamper-evident receipts.
- #4205 owns directed geometric accuracy primitives, not acquisition planning.

No copied VOI engine, generic NBV planner, survey schema, component registry, event log or IFC parser is in scope.

## P0 adapter contract

Each Scan-to-BIM decision case must bind:
- building/scene, exact model and survey revision;
- affected IFC/component identities;
- approved requirement revision and downstream decision;
- coordinate frame and qualified survey/registration references;
- current geometry/semantic/topology hypotheses and critical unknowns;
- candidate capture actions with reachable pose, sensor configuration, predicted visible region, blockers, modality, cost/time basis, setup/travel burden and evidence revision;
- outcome partitions showing which current hypotheses remain possible after each valid observation.

A predicted view may not claim to resolve a surface hidden by an occluder, outside the sensor envelope, or unsupported by qualified pose/frame evidence. A follow-up action may partition only states that survived the previous outcome.

After an outcome, reuse #1892 to recompute robust-feasible/Pareto sets. Reuse #1922 for the conditional follow-up and stop policy. A leaf is **stop_decision_stable** only when every required decision is invariant across remaining states and no critical blocker remains. Otherwise it must reopen the design or escalate the blocker.

The output is a decision-impact vector: changed critical unknowns, hard-feasibility changes, Pareto-set changes, worst-case remaining state count, residual blockers, action cost and duration. Do not collapse this into one score.

## Frozen hostile synthetic suite

1. Door-clearance discriminator: dense frontal wall scan does not resolve the opening; an oblique view can distinguish within/outside requirement hypotheses.
2. High-density no-decision scan: point count and surface coverage increase, but robust/Pareto decision sets do not change.
3. Occluded service access: an apparent viewpoint is blocked and cannot resolve the hidden state.
4. Wall-vs-recess ambiguity: a second non-coplanar view is required to distinguish topology.
5. Opening-vs-furniture confounder: distinguish permanent geometry from external occlusion or retain unknown.
6. Duplicate IFC Names/repeated geometry: no identity promotion without #3331 evidence.
7. Stale survey transform: reject capture evidence based on an obsolete frame/registration revision.
8. Independent critical unknown: resolving one condition cannot silently clear another unrelated blocker.
9. Invalid partitions: missing, duplicate, overlapping, undeclared or parent-eliminated state membership must fail closed.
10. Unsupported probabilities: expected-VoI fields without an independent calibration contract must fail.
11. Input ordering metamorphism: canonical output invariant to candidate/state/outcome array permutation.
12. Adaptive branch: a cheap first measurement requires an expensive follow-up only on one outcome; compare with a universal fixed scan at identical terminal obligations.

## Benchmark matrix

| Baseline | What it optimizes | Required comparison |
|---|---|---|
| Fixed grid / uniform sweep | Predictable coverage | Views, travel, time, residual unknowns |
| Coverage-greedy | Largest visible unobserved area | Marginal coverage vs decision impact |
| Uncertainty-greedy / Active3D-style | Local/global reconstruction uncertainty | Calibration-qualified uncertainty reduction, views and geometric quality |
| Bayesian task-specific NBV | Expected downstream task utility | Task performance and views, only with calibrated probabilistic inputs |
| Expert surveyor/BIM operator | Professional judgement | Decisions, review effort, cost and time |
| Scan-to-BIM adapter | Robust decision impact without unsupported probabilities | Feasibility/Pareto/blocker changes, worst-case cost and accepted decisions |

Measure per action and per building:
- hard-feasibility and Pareto-set changes;
- critical-unknown resolution and worst-case remaining states;
- geometric, semantic and topological quality on their separate metrics;
- false confident hidden-surface assertions and false model mutations;
- number of views, travel distance, capture/processing/review time and total cost;
- residual uncertainty and unresolved blockers.

A method wins only if it meets the same terminal decision obligations at lower cost, or improves accepted decisions at matched cost. More points alone are not a win.

## Proposed acceptance ladder (targets, not results)

- Exact agreement with the independent #1892 decision oracle on every frozen finite Scan-to-BIM world.
- 100% rejection of stale frame/revision, invalid visibility, malformed partitions, unsupported probabilities and references to eliminated states.
- Zero robust-feasible admissions while a required critical geometry/topology/identity unknown remains unresolved.
- Deterministic/canonical results under input-order permutations.
- At least one synthetic case where targeted acquisition meets the same accepted decision as a fixed plan with lower worst-case cost.
- Scale to at least 100 finite decision worlds and 10,000 candidate/state evaluations before making speed/generalization claims.
- Use building-level holdouts in external validation; TUB1/TUB2 remain one physical-building group.

Initial cost/time and state values are synthetic mechanics values, not field economics, safety tolerances or regulatory thresholds.

## Execution sequence

1. Map Scan-to-BIM hypotheses/actions into #1892 records; preserve the existing oracle.
2. Freeze the twelve hostile cases and differential-test against the independent exhaustive evaluator.
3. Compose #1922 adaptive policy trees with #1607 pose/visibility/registration evidence.
4. Feed decision-admitted measurements into #4086 reconstruction/mutation admission.
5. Bind capture/model revision and decision receipts to #1226 and #3331.
6. Compare fixed, coverage-greedy, uncertainty-led, task-specific and expert baselines under matched buildings and requirements after rights and independent reference access are confirmed.

## Claim ceiling

The first implementation qualifies adapter semantics and deterministic finite-state decisions on synthetic fixtures only. It does not establish real-world survey accuracy, safe BIM, calibrated probabilities, SOTA, or commercial advantage. Those require independent metrology, real held-out buildings, calibrated observation models, measured labor/cost and professional review.
