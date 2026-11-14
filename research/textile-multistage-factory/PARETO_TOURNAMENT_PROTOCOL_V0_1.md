# Textile production/recovery Pareto tournament protocol v0.1

Status: executable evaluator + benchmark protocol. No industrial line has been measured and no candidate policy has won this tournament.

Parent issues:
- [TEXTILE-MULTISTAGE-FACTORY-001 #4875](https://github.com/Luminous-Dynamics/symtropy/issues/4875)
- [TEXTILE-ROUTE-SCHEDULE-BINDING-001 #5001](https://github.com/Luminous-Dynamics/symtropy/issues/5001)
- [TEXTILE-SCHEDULE-RESILIENCE-001 #5070](https://github.com/Luminous-Dynamics/symtropy/issues/5070)
- [TEXTILE-SCHEDULE-PARETO-001 #5116](https://github.com/Luminous-Dynamics/symtropy/issues/5116)
- implementation: [PR #5116](https://github.com/Luminous-Dynamics/symtropy/pull/5116)

## 1. Benchmark purpose

Compare schedule and recovery policies by **receiver-accepted material value and resource burden**, not by makespan alone.

The independent validator checks whether a candidate satisfies declared machine, operator, setup, precedence, route and utility constraints. The resilience module checks a caller-supplied replan after a declared disruption. The Pareto module compares candidates only after those independent gates.

The evaluator does not generate schedules, optimize them or select a single winner. It exposes feasible trade-offs without converting incomparable outcomes into an arbitrary weighted score.

## 2. Industrial arms

### Arm A — existing Fibersort + Trimclean process

Valvan describes Fibersort as NIR fibre/blend identification plus RGB colour sorting. Its Trimclean process then cuts sorted textile into 20–100 mm clippings, applies optical camera and metal detection and pneumatically separates trim-bearing clippings.

Sources:
- https://www.valvan.com/en/solutions/textile-sorting-recycling
- https://www.valvan.com/uploads/general/VALVAN_trimclean_leaflet-A4_08_lowres.pdf?v=1685433923

This is a clipping-level trim separation route, not whole-garment seam-ripping automation.

### Arm B — pre-shredder disassembly candidate

The proposed differentiator is to identify construction, joining method, material and recoverable components before size reduction; then select a route (reuse, repair, component recovery, panel separation, mechanical recycling, chemical recycling, downcycling or reject) and remove attachments only when the join and action are admitted.

The potential advantage is preserving larger textile panels, longer fibres and reusable components. It is a hypothesis to measure, not an achieved result.

### Arm C — incumbent plus selective pre-shredder route

A fair hybrid may retain Fibersort/Trimclean for unsuitable or unknown post-consumer feedstock and route only qualified garments to panel/component recovery. Compare against Arm A on the same incoming lot and receiver criteria.

## 3. Benchmark identity and matched-lot controls

Every candidate in one tournament shares:
- benchmark suite/version;
- frozen incoming lot and input mass;
- garment/material strata and condition annotations;
- receiving specification/version and sample acceptance method;
- scenario identity and event observation tick;
- utility tick unit and demand/capacity measurement definitions;
- objective schema and hard-gate policy version.

Candidate outputs may have distinct output-lot IDs and process-trial IDs, but each must bind its own route receipt, process evidence and receiver acceptance. No output receipt may be borrowed between candidates.

Randomize or split matched sublots where a single lot cannot enter multiple process arms. Do not assign easier garments to one arm. Keep all specimens, frames, replicates and output lots grouped by physical identity for train/calibration/test partitioning.

## 4. Hard gates — not objectives

A candidate is excluded from Pareto ranking when:
- the independent machine/operator/setup/precedence schedule report fails;
- any scheduled job lacks one eligible route receipt or its receipt/evidence-manifest digest binding;
- time-indexed utility validation fails or has unknown/missing capacity;
- a required disruption tournament lacks a valid baseline/replan and utility replay for the exact scenario;
- receiver status is rejected;
- receiver acceptance does not bind the exact output lot, process trial and receiving specification;
- receiver issuer/verifier are not independent of one another and of the process issuer;
- any objective is absent, has the wrong unit, wrong provenance class or invalid evidence digest;
- objective evidence is detached from the candidate's input/output lot, producing process trial or scenario.

Missing evidence is reported as insufficient; hard contradictions or failed constraints are ineligible. Neither may be compensated by an attractive objective value.

The evaluator consumes the independent upstream reports. Its structural checks do not authenticate receipt bytes or prove the source facts. The upstream verifier remains responsible for authenticating those artifacts before they are admitted.

## 5. Objective vector

### Baseline tournament

Every candidate must provide these ten measures with fixed units and provenance:

| Direction | Measure | Unit | Required basis |
|---|---|---|---|
| Maximize | Receiver-accepted output mass | mg | Independent receiver acceptance |
| Maximize | Conservative fibre-length-qualified output / input mass | ppm | Physical fibre metrology |
| Minimize | Residual trim mass | mg | Independent physical inspection |
| Minimize | Damage area | mm² | Independent physical inspection |
| Minimize | Electricity per accepted kg | Wh/kg accepted | Calibrated meter or calibrated process trial |
| Minimize | Steam per accepted kg | g/kg accepted | Calibrated meter or calibrated process trial |
| Minimize | Water per accepted kg | mL/kg accepted | Calibrated meter or calibrated process trial |
| Minimize | Wastewater per accepted kg | mL/kg accepted | Calibrated meter or calibrated process trial |
| Minimize | Operator/setup labour per accepted kg | min/kg accepted | Worker time record |
| Minimize | Makespan | common integer tick | Independent schedule replay |

Do not treat engineering estimates as measured utility or worker-time evidence. If a candidate is only estimated/simulated, it may be explored outside the measured leaderboard but is not eligible in this tournament.

### Disruption tournament

Adds:
- total tardiness (tick);
- plan churn (changed/added/dropped/reassigned operation count);
- post-event completion horizon (tick).

Every candidate must have a feasible independent baseline and replan report for the same exact event/scenario; these metrics must equal the replay report. Static candidates cannot populate these values with zeros.

## 6. Pareto analysis

For eligible candidates only:
- compare all required objective dimensions with fixed minimize/maximize directions;
- candidate A dominates B only if A is no worse on every metric and strictly better on at least one;
- mutually incomparable candidates remain on the same front;
- equal vectors tie; candidate ID is display sorting only;
- report the full vector, evidence IDs/digests, front index, dominates and dominated-by relations.

Do not report a weighted total score. Do not report hypervolume unless objective normalization and reference point are preregistered and fixed before the run. Hypervolume is supplementary and never replaces the full Pareto front.

## 7. Required external comparators

Compare, where available and under matched compute/time/acceptance constraints:
- current installed Fibersort + Trimclean process;
- current human/manual routing and trim-removal process;
- earliest-deadline/dispatch heuristic;
- setup-aware least-completion-time/critical-ratio dispatch;
- MILP / CP-SAT baseline;
- LCS / large-neighbourhood search or local improvement;
- genetic algorithm / multiobjective evolutionary algorithm;
- learning-assisted policy (only after training data and test separation are pinned);
- hybrid policy with route-eligibility and evidence gates.

The independent feasibility verifier and bounded tiny-instance exact oracle remain separate from every policy. The current exact oracle is bounded to tiny integer-time cases and zero-duration setup transitions; it is not the production optimizer.

## 8. Disruption corpus

Every disruption tournament freezes:
- event type and exact observation tick;
- machine/operator/material/utility affected;
- event interval and evidence provenance;
- already-started work and non-interruptibility boundary;
- exact replan problem/utility inputs;
- objective and hard-gate versions.

Required scenarios: machine outage, operator absence, late material, longer operation duration, due-date tightening, known utility derating, utility capacity unknown, and paired compound events. Report baseline feasibility, replan feasibility, post-event completion horizon, tardiness and churn separately.

## 9. Research targets — not achieved results

Initial stretch objectives after the first measured baseline:
- zero hard machine/operator/route/utility/receiver violations among ranked candidates;
- zero false eligible routes for unknown, conditional or rejected source material;
- 100% accepted-output mass bound to an independent receiver's exact output lot and trial;
- 100% fibre metrics method/scope/calibration/lot-bound, with zero cross-method equivalence without paired bridge;
- at least 5% improvement in accepted output per input tonne or resource intensity without worse receiver grade or damage;
- at least 10% lower operator/intervention burden per accepted kg without transferring risk to unmeasured work;
- at least 20% reduction in post-disruption completion horizon or churn without losing receiver acceptance;
- exact oracle agreement on every supported tiny instance, and zero hard feasibility violations on larger benchmark cases.

These are proposed objectives. They are not industry standards, measured results or performance guarantees.

## 10. Claim ceiling

A Pareto front describes only eligible candidates under the declared objective schema, source receipts and receiver specification. It does not automatically identify the best policy, prove commercial value, certify worker safety or establish environmental benefits.

A valid industry-superiority claim requires matched lots, an independently measured incumbent, exact configurations, calibrated utility observations, independent damage/metrology and receiving-process acceptance. Supplier-stated throughput alone is not a measured accepted-output rate.
