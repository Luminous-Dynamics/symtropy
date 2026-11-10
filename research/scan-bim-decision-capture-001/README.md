# SCAN-BIM-DECISION-CAPTURE-001 — task-conditional measurement adapter

**Status:** P0 adapter implemented; synthetic semantics only.  
**Issue:** [#4207](https://github.com/Luminous-Dynamics/symtropy/issues/4207)  
**Adapter implementation:** `tools/spatial-resilience-qualifier/src/scan_bim.rs`  
**Stack:** [spatial resilience decision qualifier PR #1849](https://github.com/Luminous-Dynamics/symtropy/pull/1849)

## Objective

Choose the next measurement according to whether its possible outcomes can change a named building decision—not simply because it adds points, fills a region or lowers an uncalibrated uncertainty score.

The adapter consumes a task profile, finite current building hypotheses, candidate feasibility/objectives and candidate scan actions. It qualifies action references/target visibility and delegates the hard-first robust-feasibility/Pareto calculation to the existing spatial-resilience decision-impact engine in PR #1849. It does not implement a new VOI engine, motion planner, survey solver or reconstruction algorithm.

## P0 executable contract

- Building group, scene, model revision, coordinate frame and approved requirement revision are explicit.
- The task identifies a property, comparison, threshold in metres, target element identities, minimum theoretically observable target-area fraction and unresolved critical unknowns.
- Possible physical states include a property value and an explicit requirement-satisfied flag. The flag is recomputed against the threshold; contradiction fails closed.
- Candidate models declare hard feasibility and a fixed objective vector for every possible state.
- Every scan action binds sensor identity, source model revision, frame, pose revision, pose evidence digest, sensor-profile digest, visibility-profile digest and evaluator visibility receipt digest.
- Each target entity has an evaluator-derived visibility status and observable/theoretical area. A blocked/unknown target or insufficient visible fraction makes the action evidence-invalid.
- Cost/duration units and their basis IDs are mandatory. They are synthetic in the P0 fixture, not real labour/economic values.
- Outcome partitions must be mutually exclusive and exhaustive over the current possible states. The existing decision-impact engine validates the partition and rejects stale revisions or unresolved-unknown scope errors.
- No priors, likelihoods, probability or expected-VOI fields are accepted.

## First frozen experiment — doorway clearance

Fixture: `fixtures/door-clearance-v0_1.json`  
Schema: `schema/scan-bim-decision-case-v0_1.schema.json`

The task is whether a doorway meets a declared 0.90 m minimum clear-width requirement. Two possible states are represented: 0.94 m (compliant) and 0.82 m (non-compliant).

Four actions are evaluated:
1. **Targeted oblique clearance scan:** sufficient predicted theoretical visibility, valid pose/revision, separates the two physical hypotheses, synthetic cost 2/time 1.
2. **Full-room TLS sweep:** also separates the same hypotheses, but has synthetic cost 25/time 8.
3. **Dense frontal wall scan:** geometrically observable at the target, but both hypotheses remain compatible and it resolves no critical unknown. It is not decision-relevant.
4. **Blocked backside scan:** self-occluded; it is rejected and cannot resolve the doorway unknown.

The expected recommendation is targeted oblique scan before the full-room sweep. The dense scan remains visible in the report but is not recommended. The blocked scan is rejected before the decision engine. The CLI reports four counts separately: evidence-qualified actions, decision-engine-accepted actions, decision-relevant actions, and decision-rejected actions. In this fixture those counts are 3, 3, 2 and 1 respectively. The dense frontal scan is valid evidence but is not decision-relevant.

This is a deliberately small known-answer experiment. It does not imply that the scan actually measures the doorway, that the pose or visibility receipt is authentic, or that the synthetic cost/time values reflect field operations.

## Implementation and tests

- `decision_impact::evaluate_decision_case_value` exposes a one-case entry point to the existing #1892-style finite-state decision logic. It does not copy the evaluator.
- `scan_bim::evaluate_scan_bim_decision_bytes` validates the v0.1 JSON Schema, typed receipt and task/visibility bindings.
- `scan_bim_decision_capture` is a CLI that emits a machine-readable report.
- 11 adapter tests cover targeted-vs-fixed ordering, no-decision dense capture, self-occlusion rejection, stale model revision, insufficient visibility area, missing target coverage, incomplete outcome partitions, unsupported probability fields, contradictory requirement labels, preservation of unrelated critical unknowns and input-order invariance.
- The workflow runs this CLI against the embedded fixture and checks the expected recommendation and dispositions.

Run:

```sh
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml --locked \
  --bin scan_bim_decision_capture -- --check --json
```

## Research baselines

- **Active3D (AAAI 2026):** uncertainty-led next-best-view selection with expected hybrid information gain. Compare on visibility/geometry quality and action count; do not claim that generic uncertainty-aware NBV is novel. https://doi.org/10.1609/aaai.v40i8.37585
- **Zhu, Sellán & Terenin (SIGGRAPH 2026):** Bayesian task-specific NBV under uncertain geometry. It optimizes task utility, but probabilistic expected value requires calibrated priors/likelihoods. https://arxiv.org/abs/2605.05095
- **GO-PRE (ICML 2026):** predictive-rendering entropy for next-best-view selection. A separate comparator when adapter/benchmark data are available. https://proceedings.mlr.press/v306/song26k.html
- **Leica Cyclone 3DR:** established element-level tolerance, no-data and coverage reporting. Heatmaps and per-element deviation reports are not our novelty. https://rcdocs.leica-geosystems.com/cyclone-3dr/2026.0/Ex_ToleranceChecking

The proposed Luminous difference is **decision-sufficient capture with explicit evidence gates**: only recommend a reachable action with valid revision/frame/pose/visibility evidence and a partition that can change a hard constraint, Pareto set or critical-unknown set. Compare at matched terminal obligations and report views, travel, capture time, processing time, expert review, unresolved blockers and accepted decisions.

## Ownership

- #1892 / #1849: generic finite-state outcomes, decision-impact vectors and robust/Pareto evaluator.
- #1922 / #1849: adaptive follow-up policy trees and stopping rules.
- #1607: survey frame, pose, calibration, registration and uncertainty evidence.
- #4292 / #4449: theoretical visibility receipt.
- #4086: element association, topology and model-mutation admission.
- #3331: persistent physical-component identity.
- #4116: replay, evaluator separation and evidence custody.

This adapter must not create a competing VOI engine, sensor calibration model, survey schema, identity registry, IFC parser or evidence ledger.

## Claim ceiling

P0 qualifies a typed synthetic decision adapter only. Digest strings and target-visibility summaries are declared inputs; this crate does not authenticate their bytes or verify physical calibration. No real survey accuracy, calibrated observation probability, field cost reduction, construction/code compliance, structural adequacy, safety or commercial superiority is claimed.
