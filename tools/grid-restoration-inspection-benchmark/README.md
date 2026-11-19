# Grid restoration inspection decision benchmark v0.1

**Parent:** [#6273](https://github.com/Luminous-Dynamics/symtropy/issues/6273) — decision-valued inspection and evidence acquisition under resource constraints  
**Programme:** #5241 Power Fabric benchmark  
**Snapshot:** 2026-11-19

## What this package does

This is a candidate-visible episode contract only. It carries observed facts, unknowns, unresolved conflicts, inspection candidates, candidate recovery proposals, resource-state references and typed references to existing owners.

It deliberately does **not** implement diagnosis, Bayes/VOI, a power-flow engine, an inspection scheduler, crew routing, service prediction, work-order execution or authority. The diagnosis/observation model and decision-loss semantics remain with Symthaea #12403 (and generic epistemic action selection with #6203). Symtropy composes its output with #6213 execution history, #6237 shared supplier/resource capacity, #6235 service outcomes and #5520 selective-repair scope.

## Important semantics

- `effectiveHour` is when the observation applies; `observedHour` is when it became available to the decision maker. Neither can exceed the decision's `asOfHour`.
- `sourceRef`, `sourceFamily` and `independenceGroupRef` are separate. The example telemetry and dashboard mirror share an auxiliary-power/telecom failure-domain reference; they must not be treated as two independent observations.
- Inspection time, result latency, resource requirements, access prerequisites, authority requirement, applicability, calibration and observation-model references are explicit.
- A test candidate does not imply permission or capacity. The actual resource state is referenced through a pinned manifest; proposed inspection capacity cannot be counted as booked capacity.
- Candidate recovery actions are proposals with evidence/resource/authority prerequisites, not authorizations or performed work.
- Probability distributions and observation matrices are not copied into this schema. Their owner must provide a calibrated, versioned model or an explicitly synthetic/robust assumption. Missing calibration or independence evidence stays unknown.
- This development fixture is public and synthetic, not blind and not a physical or operator dataset. It is not a claim about any transformer, utility or outage.

## The benchmark question

The fixture intentionally includes a fast dashboard refresh that shares the same failure domain as the existing alarm channel, alongside a slower independent inspection. The former may add information but not independent evidence; the latter consumes the same specialist crew that the repair proposal needs. The benchmark will compare fixed inspection, entropy-greedy, one-step net value, resource-aware value, selective repair, robust abstention and a bounded exact oracle. The schema alone does not declare which action should win; that result must be derived by the decision-model owner and an independent evaluator.

## External comparator map

- Han et al. (2026), UAV vision + crew dispatch/reinspection/load restoration, IEEE DOI 10.1109/ICPSAsia70813.2026.11692267.
- Diadelmo et al. (August 2026), Bayesian field-updated inspection-crew routing, RESS DOI 10.1016/j.ress.2026.112527.
- Uddin et al. (August 2026), calendar-aware RCPSP corpus, DOI 10.1016/j.dib.2026.113256. The source-semantic audit remains in #1938/#1950 and PR #1811.
- The grid-specific uncertainty-aware repair/depots/mobile-generation comparator remains under parent #6242, SEGAN DOI 10.1016/j.segan.2026.102294.

## Claim boundary

Schema and synthetic fixture only. No fault posterior, inspection outcome, route feasibility, restoration schedule, service improvement, real resource booking, work authorization, switching, energization or field validation is produced.
