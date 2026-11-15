# AeroWeft DCB-001 — Maneuverability-Aware Batch Space-Time Allocation

**Tracking issue:** [AEROWEFT-DCB-001 #5186](https://github.com/Luminous-Dynamics/symtropy/issues/5186)  
**Parent:** [AIRSPACE-OPS-001 #2279](https://github.com/Luminous-Dynamics/symtropy/issues/2279)  
**Status:** research benchmark contract / synthetic-first  
**Claim ceiling:** no live ATC/UTM, flight authorization, clearance, dispatch, separation assurance, flight-control or certification authority.

## 1. Research motivation

AeroWeft should benchmark not only whether two supplied 4D paths conflict, but also how the system allocates shared space-time resources when multiple flight requests compete for vertiports, crossings, merges, corridor entries/exits, charging and recovery options.

NASA has already studied relevant baselines:
- DCB-Waypoint addresses en-route waypoint capacity as well as vertiports but processes one flight and one waypoint at a time.
- DCB-Airborne-Delays adds feasible speed changes to pre-departure delay, using aircraft speed ranges.
- NASA's 2025 flight evaluation of strategic path-planning automation reported that operations within flow corridors were less successful in some maneuvers because tighter volumes left fewer degrees of freedom for conflict-resolution manoeuvring.

These findings motivate a benchmark question, not an assumed conclusion: can batch optimization and maneuverability-aware resource allocation improve system-wide delay and resilience without consuming too much of the aircraft's future ability to adapt?

References:
- NASA DCB algorithms, NTRS 20240011341: https://ntrs.nasa.gov/citations/20240011341
- NASA/TP-20250003243, published 1 July 2025: https://ntrs.nasa.gov/citations/20250003243

## 2. What is being optimized?

The basic object is a flight request competing for a chain of time-dependent resources. Each resource may be a vertiport pad, approach/departure slot, route segment, crossing, merge, charging position or recovery location.

A valid schedule must satisfy each resource's capacity/time contract and each aircraft profile's model applicability and energy/performance constraints.

**Maneuverability reserve** is an experimental planning-quality measure of the future options left available to a flight. It is not an approved safety margin. Candidate dimensions include:
- remaining qualified speed, turn and climb authority;
- reachable alternative resource-time windows;
- lateral/vertical/temporal slack inside the declared model;
- alternate and recovery options;
- energy and reserve margin after a declared candidate maneuver;
- sensitivity to wind, delay and resource loss.

Do not collapse this into a single "safety score". Report dimensions independently, and treat any domain-specific hard limit as a non-compensatory constraint.

## 3. Policy families

Use transparent reference implementations for:

1. FIFO / first-come-first-served vertiport scheduling.
2. Sequential DCB at vertiports.
3. Sequential DCB at vertiports plus en-route waypoints.
4. Sequential DCB with aircraft-profile-constrained airborne speed delay.
5. Shortest feasible path plus first-fit resource slotting.
6. Joint batch time-expanded scheduling with hard resource constraints.
7. Scenario-robust batch scheduling with uncertainty, recovery and maneuverability reserve.

Where NASA's exact implementation is not publicly available as executable source, describe the baseline as a transparent reference implementation of the published algorithm class—not a faithful reproduction of NASA's system.

Use equal scenario inputs, demand, fleet, hard constraints, compute budget and stopping criteria. Record every baseline's implementation identity.

## 4. Synthetic benchmark topology

Create a hand-solvable topology with:
- two origin hubs and one destination hub;
- two intersecting/merging route resources;
- one optional bypass with a wider but longer time-space volume;
- one shared charging/turnaround resource;
- two synthetic aircraft profiles with different performance domains;
- multiple requests, time windows and deadlines;
- one wind disturbance, one resource outage and one inserted priority request.

Every feature, aircraft parameter, capacity, time and disturbance is fictional. No geometry corresponds to an operational airspace.

## 5. Non-compensatory feasibility gates

Before policy scoring:
- model profile and revision must be supported for the declared regime;
- all time, frame, vertical-datum and source semantics must be compatible;
- required resource capacities and validity windows must be present and current in the synthetic scenario;
- no resource capacity may be exceeded;
- candidate paths must pass the bounded conflict-screen oracle;
- required energy/recovery conditions must be represented;
- missing or contradictory information returns unresolved/not-assessable;
- no synthetic reservation becomes a flight authorization.

A high throughput score cannot compensate for one invalid or unresolved hard constraint.

## 6. Metrics

Report separately:
- completed requests, throughput and cancellations;
- total, median, p95 and maximum delay;
- delay variance, worst-operator delay, starvation and max-min fairness;
- vertiport, route, waypoint, charging and recovery utilization;
- overbooked resources and unresolved conflicts;
- time to first feasible solution and optimality gap where valid;
- runtime, peak memory, explored states and budget-exhausted outcomes;
- speed/turn/climb authority consumed and remaining;
- maneuverability-reserve distribution and recovery-option count;
- energy use and reserve after candidate changes;
- schedule churn and replanning stability.

Plot Pareto frontiers of throughput, delay, fairness, energy, flexibility and recovery. Avoid a single weighted score that hides a severe loss of flexibility or unfair delay assignment.

## 7. Falsifiable targets

- 100% agreement with an independent exhaustive oracle on the tiny frozen topology.
- Zero capacity or known-conflict violations accepted.
- 100% fail-closed outcomes for unsupported model profiles and missing/expired required constraints.
- Deterministic results under request order and ID relabeling.
- Every claimed optimal result has an exhaustive proof or valid lower-bound/gap certificate.
- Quantify, rather than assume, whether batch policies improve delay/throughput over sequential DCB.
- Quantify whether flexible volumes preserve more usable contingency options than rigid corridors at matched constraints.
- Report the cost of retained maneuverability: utilization, delay and energy may worsen when more flexibility is reserved.

All numeric targets apply to synthetic, bounded confirmatory cases only.

## 8. Cross-project responsibilities

- **Symthaea #6249:** fixed-wing physical models and applicability envelopes. Symtropy does not duplicate aircraft dynamics.
- **Symthaea #7729:** candidate-specific mass-coupled mission inputs.
- **Symtropy #2279/#2297/#5163:** 4D scenarios, reachable-occupancy integration, conflict screening and numerical soundness.
- **Symtropy #1601:** existing multimodal optimization infrastructure; no second generic optimizer.
- **Symtropy #1887:** existing mission-chain and transfer continuity.
- **Sol-Atlas #189:** source-qualified, read-only spatial/resource context.
- **Mycelix #5023:** identity, provider provenance, reservation evidence and audit; not a real-time separation-control dependency.

## 9. Execution sequence

1. Freeze the tiny topology, demand and resource calendar.
2. Implement the transparent sequential baselines and independent exhaustive oracle.
3. Add the batch reference policy using existing optimizer infrastructure.
4. Add the profile-aware maneuverability-reserve accounting.
5. Inject disturbances and priority requests.
6. Compare complete scorecards and Pareto frontiers.
7. Expand only when oracle, applicability and hard-gate contracts pass.

**The benchmark's success is not maximum sky utilization. It is better service with demonstrable feasibility, retained options and fair resource allocation.**
