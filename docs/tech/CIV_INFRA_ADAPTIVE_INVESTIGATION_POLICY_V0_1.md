# CIV-INFRA Adaptive Investigation Policy v0.1

**Status:** synthetic contract specification with a bounded reference evaluator in [PR #1942](https://github.com/Luminous-Dynamics/symtropy/pull/1942); this document alone does not establish engineering validity.  
**Parent issue:** [CIV-INFRA-POLICY-001 #1941](https://github.com/Luminous-Dynamics/symtropy/issues/1941)  
**Composes:** [decision-impact planner #1799](https://github.com/Luminous-Dynamics/symtropy/pull/1799), [measurement compatibility #1810](https://github.com/Luminous-Dynamics/symtropy/pull/1810), [robust design #1769](https://github.com/Luminous-Dynamics/symtropy/issues/1769), and [bridge/tunnel evidence profile #1921](https://github.com/Luminous-Dynamics/symtropy/issues/1921).

## 1. Research thesis

The next planning layer should choose a **policy tree of investigations**, not merely rank one survey. It should use the conservative state-to-possible-outcome relation from the measurement-compatibility evaluator rather than treating all observations as mutually exclusive partitions.

The policy is an offline finite-state planning object. It has no direct authority to order surveys, access sites, change operations, issue procurement, authorize closures, or approve construction.

The latest NIST report on uncertainty quantification argues that computational-model credibility should scale with model influence and decision consequence; the current railway digital-twin literature likewise identifies data integration, interoperability and the balance between model fidelity and computational cost as persistent challenges. This spec uses those concerns to justify preserving uncertainty at every policy transition rather than hiding it in a score.

Research anchors:
- NIST AMS 100-85 (2026-10-05): https://doi.org/10.6028/NIST.AMS-100-85
- BIPM/JCGM GUM-6:2020 measurement models: https://doi.org/10.59161/JCGMGUM-6-2020
- Song & Zhang, *Digital twin technology in railway infrastructure*, 2026: https://doi.org/10.1108/RS-02-2026-0013
- OGC InfraGML Part 5 — Railways: https://docs.ogc.org/is/16-105r2/16-105r2.html

These motivate the design requirements; none validates this algorithm.

## 2. State and compatibility semantics

Each synthetic world has a decision signature across three candidate corridors: surface detour, bridge alternative and tunnel alternative. Feasible, infeasible and unknown are disjoint states; frontier membership can include only feasible candidates.

For an action, its compatibility relation is a set of (state_id, outcome_id) edges. An outcome's child state set consists of all states connected to that outcome. A state may connect to multiple outcomes. A state that overlaps two outcomes must not be coerced into a single bucket. Outcomes carry no implied probability and are not equiprobable.

An open or unqualified model-support boundary adds an explicit out-of-model-support branch. That branch preserves the full pre-observation state set and terminates in review_required. A convenient result label or complete-looking JSON shape cannot suppress this branch. The policy may report its supported-outcome metrics conditionally, but it must not call them a guaranteed information gain unless outcome support has been separately qualified.

## 3. Synthetic corridor fixture

Fixture: [adaptive-investigation-v0.1.json](../../tools/civ-infra-adaptive-investigation/fixtures/adaptive-investigation-v0.1.json).

The five synthetic states are stable/dry ground, bridge-scour risk, tunnel-groundwater risk, marginal ground, and unresolved geology. The three declared actions are a bridge screen, a tunnel-oriented borehole, and a bridge-focused follow-up. The candidate policy selects the tunnel investigation after a bridge-clear outcome and the focused bridge follow-up after a bridge-concern outcome.

The fixture deliberately maps marginal-ground and unresolved-geology to both outcomes of the first-stage bridge screen. This is the essential regression: an implementation that assumes a partition would either reject the valid compatibility graph or silently discard possible worlds. The policy then emits an explicit review terminal for model-support failure at every action whose support state remains open.

The candidate policy is a **test vector**, not a recommended field-investigation sequence. All states, edges and model references are invented; there are no calibrated observation models, survey costs, geological measurements or solver results.

## 4. Adaptive policy state machine

At each node:

1. Start with the set of states consistent with all preceding supported observations.
2. Apply the selected action's state-to-outcome edges; keep every compatible state on each branch.
3. If the action's model support is open, preserve a separate no-update failure branch and terminate it in review.
4. Continue the declared policy until its action depth/budget is exhausted or stopping criteria are met.
5. Recompute candidate decision signatures for every supported leaf. Never report zero ambiguity while signatures differ or any required state was dropped without compatible evidence.

The evaluator must reject cycles, duplicate action/outcome IDs, unknown state references, policy edges naming a result incompatible with their action, and paths beyond the declared depth.

The implementation must also bound **total expansion**, not just depth. The reference evaluator uses memoized longest-path analysis for graph validation, caps recursive operation accounting at 100,000 operations, output at 10,000 leaves, and aggregate state memberships across leaves at 100,000. It returns an error without a partial policy report when any limit is exceeded. Caps are versioned implementation constraints, not promises about maximum useful project size. Any future limit increase must include adversarial time/memory measurements and preserve fail-closed semantics.

## 5. Objective and reporting

V0 uses a transparent lexicographic objective over supported leaves only:
1. minimum worst-case distinct decision-signature classes;
2. minimum worst-case decision-discordant state pairs;
3. minimum worst-case residual state count;
4. then declared worst-case action count and stable policy ID.

Action cost and duration remain separately reported, with units and evidence basis. They should not be traded against ambiguity via undocumented weights. No expected cost, probability, or expected value of information is emitted without a later, separate calibrated probability contract.

Every report must also expose support-failure leaves separately, with the full preserved state set, signature classes, action path, and review disposition. Do not discard those leaves from the denominator.

## 6. Benchmark ladder

- **P0 — Contract:** exact input-byte hash, schema/version checks, deterministic canonical report, unknown outcomes and model-support failures remain visible.
- **P1 — Tiny exhaustive oracle:** enumerate all valid policy trees under a small state/action/depth cap; require exact agreement on leaf state sets, disposition classes, ranking and worst-case metrics.
- **P2 — Adversarial corpus:** overlapping outcomes, state compatible with every outcome, action with no decision impact, missing edge, duplicate outcome, stale digest, incompatible model version, cycle, hidden future observation, and out-of-support reading.
- **P3 — Generated corpus:** at least 100 deterministic worlds and 10,000 state/outcome edges; report exact denominators, oracle matches, work budget, runtime and memory. Include a layered branching DAG whose path count grows exponentially; require an explicit budget error, no partial report, and bounded operation/leaf counts.
- **P4 — Admitted measurements:** only after a verified artifact resolver, measurement-model applicability and discrepancy evidence have been reviewed. Synthetic performance is not a physical measurement claim.

Core acceptance targets:
- zero false singleton/zero-ambiguity classifications;
- 100% preservation of prior states on support-failure branches;
- exact small-case oracle agreement;
- zero probabilities or expected-VoI values without calibrated evidence;
- byte-stable replay for identical canonical inputs and evaluator version.

## 7. Ownership boundaries

- **Symtropy** owns this policy evaluator and finite synthetic policy benchmarks.
- **Symtropy measurement compatibility** owns transform/interval semantics and the source compatibility graph.
- **Symtropy domain solvers** retain bridge, tunnel, hydrology, rail-dynamics and construction physics separately.
- **Symthaea** may suggest candidate investigations and critique the results, but cannot fabricate measurement-model performance.
- **Sol-Atlas** projects qualified corridor/asset/investigation evidence into the spatial view; rendering is not physical qualification.
- **Mycelix** retains organizational authority, procurement/work evidence and approval records; a policy ranking is not authorization to act.

Do not create another GIS source of truth, universal civil solver, or parallel rail scheduler.

## 8. Claim ceiling

This spec and fixture are synthetic contract artifacts. They do not model bridge scour, railway loads, tunnel ground response, groundwater flow or the effectiveness of a real borehole. They establish no solver accuracy, standards conformance, safety, permitting, construction authority, inspection deferral or rail operating permission.
