# AeroWeft — Reciprocal Recoverability and Shared-Airspace Option Externalities

**Tracking issue:** [AEROWEFT-RECIPROCAL-OPTIONS-001 #5207](https://github.com/Luminous-Dynamics/symtropy/issues/5207)  
**Parent benchmark:** [AEROWEFT-DCB-001 #5186](https://github.com/Luminous-Dynamics/symtropy/issues/5186)  
**Related reachability contract:** [AEROWEFT-KINEMATICS-001 #5163](https://github.com/Luminous-Dynamics/symtropy/issues/5163)  
**Status:** research specification / synthetic-first  
**Claim ceiling:** finite benchmark measurement only; no operational flight, ATC/UTM, separation, dispatch, clearance or certification authority.

## 1. Research thesis

A schedule can satisfy present resource capacity and pairwise conflict checks while consuming the future options needed to respond to a disturbance. It can also preserve one aircraft's options by removing those of another aircraft.

AeroWeft therefore needs two separate questions:

1. **Individual recoverability:** does each aircraft retain a non-anticipative continuation policy over all declared compatible future branches?
2. **Reciprocal recoverability:** does the joint allocation preserve compatible continuations for all participating aircraft, and who bears any loss of options?

This is an aviation-specific measurement layer over the candidate schedules produced by #5186. It is not a new route optimizer, flight-dynamics model, general reachability engine or generic policy-tree evaluator.

## 2. Existing ownership and reuse

| Responsibility | Owner | This work consumes |
|---|---|---|
| Aircraft physics and model applicability | Symthaea #6249 | Versioned, aircraft-specific domains and declared synthetic model profiles |
| Reachable occupancy / enclosure | Symtropy #5163 | Model-bounded occupancy and its explicit limitations |
| 4D conflict and airspace benchmark | Symtropy #2279/#2297 | Candidate geometric/resource constraints |
| Sequential and batch DCB candidates | Symtropy #5186 | Frozen candidate schedules and matched policy inputs |
| Observation-constrained option sets | Symthaea #11291 | Per-agent feasible continuation semantics |
| Multi-step policy-tree recoverability | Symthaea #11349 | OR-policy / AND-disturbance / observation-refinement semantics |
| Reciprocal option externalities | Symthaea #11394 | Agent-indexed option-change and externality semantics |
| Multimodal optimization | Symtropy #1601 | Whole-journey objective and existing optimizer |
| Mission-chain continuity | Symtropy #1887 | Transfer and end-to-end recovery requirements |
| Cargo qualification | Symtropy #1636 | Cargo handling hard gates |
| Spatial/source context | Sol-Atlas #189/#825 | Read-only source-qualified synthetic or admitted context |
| Identity and provenance | Mycelix #5023 | Reservation, provider and evidence receipts |

Do not transplant road-specific action or observation semantics from Symthaea. Do not introduce a second generic scheduler, mission-chain engine, physical model, viability solver or policy-tree implementation. If a required upstream receipt or domain is unavailable, report the gap; do not synthesize it.

## 3. Semantic contract

### 3.1 Per-agent information boundary

For each aircraft i, freeze:

- aircraft/profile/configuration ID and revision;
- model/applicability identity and declared operating regime;
- current state set and its validity horizon;
- candidate allocation and resource reservations;
- action/continuation domain, including a decision-sufficiency declaration;
- delivered observation history and delivery times;
- compatible hidden-state, weather, resource and model branches;
- branch coupling to other aircraft and shared resources;
- exact scenario, policy, evaluator and toolchain identities.

An evaluator-only hidden-state label is not an observation. A policy may distinguish two futures only after the corresponding distinguishing evidence is delivered to that policy.

### 3.2 Robust lower and possible upper option sets

Let h_i denote an aircraft's actual delivered policy history and k the remaining finite decision horizon.

The **robust lower option set** O_i^- (h_i,k) contains first continuation actions for which there exists a finite continuation policy tree that:

- selects one action for every observation-equivalence class;
- accounts for every declared admissible successor compatible with that history;
- branches only on later observations actually delivered;
- respects aircraft-specific model applicability and all declared hard constraints;
- reaches the declared mission terminal or recovery condition by the horizon.

The **possible upper option set** O_i^+ (h_i,k) contains actions not refuted by the admitted compatible branches, but which may depend on unresolved hidden-state distinctions. It is not a guarantee and must never be presented as a robust continuation.

A third result is required: UnknownOrIncomplete, for missing branches, unavailable observations, unsupported model regimes, stale inputs or incomplete manifests. Unknown is not included in the robust lower set.

This is a finite evidence-relative evaluator. It is not a continuous-time aircraft viability proof.

### 3.3 Joint continuation set

For the complete participating fleet, define J as the set of joint policies for which:

- each aircraft's policy respects that aircraft's own delivered observations and action domain;
- shared 4D resources and capacities are respected in every declared joint branch;
- inter-aircraft conflict constraints are respected by the declared independent conflict oracle;
- energy, landing, charging, transfer, recovery and mission-chain conditions are satisfied;
- all interacting branches share a consistent world/resource history.

Do not construct an impossible joint scenario by independently cross-producting marginal weather or resource branches that the scenario manifest says are correlated. Conversely, do not omit a physically admissible joint branch merely because it is inconvenient.

Individual non-empty option sets do not imply a non-empty joint continuation set.

## 4. Reciprocal option externality

For a frozen scenario S, candidate schedule P, aircraft i, and affected aircraft j, define a paired counterfactual:

- P: the reference allocation;
- P[i <- a']: the same allocation with only aircraft i's declared reservation/action changed to a', unless additional coupled changes are explicitly part of the intervention.

The option externality is:

\[
E_{j\leftarrow i}(S,P,a')
=
\mu_j\!\left(O_j^-(S,P[i\leftarrow a'])\right)
-
\mu_j\!\left(O_j^-(S,P)\right)
\]

where mu_j is a declared measure over aircraft j's own frozen, decision-sufficient option domain.

The sign convention is:

- positive: the intervention increases j's robust options;
- zero: no measured change;
- negative: the intervention removes robust options from j.

The evaluator must also report set identity/inclusion changes, not just a number.

### 4.1 Comparability rules

- Compare options only within the same aircraft and same frozen option domain.
- Keep raw option IDs, set sizes and evidence completeness.
- A within-aircraft retention ratio may be reported only when baseline and candidate share the same qualified domain and the baseline measure is non-zero.
- A count of speed choices for one aircraft is not commensurate with a count of diversion sites for another.
- Do not label option count, volume or measure as a safety margin.
- Pairwise externalities are not assumed additive. Interactions among several aircraft must be exposed by matched multi-agent counterfactuals, not allocated by an arbitrary proportional rule.
- A schedule-wide objective may report max loss, min retention and distributional outcomes, but cannot conceal one aircraft losing every robust continuation.

### 4.2 Causal attribution discipline

A counterfactual that changes one aircraft's reservation and reruns a global optimizer may also change other reservations. Such a result is a **system response to intervention**, not an isolated direct externality. Record every changed allocation and label the estimand accordingly.

Only a one-axis controlled contrast may be described as a single-aircraft allocation effect. If more than one allocation changes, report the combined intervention or use a separately specified interaction design.

## 5. Non-anticipative policy-tree evaluation

At every policy node:

- **OR:** the policy selects an action from the actions supported by the delivered history;
- **AND:** every admissible successor branch consistent with the selected action must be evaluated;
- **observation refinement:** branches may split only when a new, policy-visible observation differs under the frozen observation-equivalence rule;
- **joint coupling:** simultaneous aircraft decisions and shared resource outcomes use the same declared world branch.

A candidate that chooses action left for hidden state wind-A and right for hidden state wind-B, when both states produce the same delivered observation, is a clairvoyant-policy failure.

A future policy may condition on an authenticated, fresh and delivered source update; it may not condition on an evaluator-only truth label, future event or branch ID.

## 6. Tiny synthetic option-externality trap

Use a finite, hand-solvable scenario with:

- two heterogeneous synthetic aircraft;
- one shared crossing/merge resource;
- one wider, longer bypass;
- time-dependent landing/charging capacity;
- at least two declared wind/disturbance branches;
- one delayed or missing observation;
- one emergency request or resource outage;
- explicit, fictional aircraft-specific speed/turn/energy option domains;
- a named recovery resource and terminal/recovery deadline.

Construct two candidate schedules:

1. **Nominally greedy allocation:** allocates the most convenient shared window to the aircraft with more nominal flexibility. The other aircraft retains a nominal path but has no robust continuation on at least one admissible disturbance branch.
2. **Option-preserving allocation:** changes the flexible aircraft's slot or uses the wider bypass, retaining a joint continuation for both aircraft at a measurable cost in delay, utilization or energy.

The fixture must make the option loss visible as a concrete violating successor witness. Do not label one policy superior before the exhaustive oracle evaluates all declared branches.

All geometry, capacity, vehicles, times and dynamics are synthetic. They do not describe operational airspace or real aircraft.

## 7. Required scorecard

### Per-aircraft vector

Report each independently:

- robust lower-set identity and size;
- possible upper-set identity and size;
- recoverable / reduced-options / dead-end / unresolved / incomplete disposition;
- worst-case recovery horizon;
- compatible recovery-resource identities and capacity windows;
- aircraft-profile-qualified speed, turn, climb and energy authority remaining, when supplied;
- option retention relative to that aircraft's own baseline;
- missing branches, unsupported conditions and source/model applicability limitations.

### Fleet / system vector

Report:

- existence and identity of joint continuation policies;
- reciprocal option-externality matrix;
- worst affected aircraft and distribution of option loss;
- minimum per-aircraft retention where defined;
- emergency insertion success and recovery-resource contention;
- throughput, completed/on-time requests, p50/p95/max delay and starvation;
- fairness and max-min delay;
- schedule churn, runtime, memory and budget-exhausted outcomes;
- exact rejected-action and dead-end successor witnesses.

Hard gates remain separate from all quality metrics. A schedule that fails a declared hard constraint is not rehabilitated by throughput, fairness or energy performance.

## 8. Matched baselines

Use identical scenario versions, demands, aircraft profiles, initial state sets, disturbance branches, observations, hard constraints, seed, compute budget and stopping criteria:

1. static/fixed corridor reservation;
2. FIFO / sequential vertiport DCB;
3. sequential waypoint DCB;
4. speed-constrained airborne-delay DCB;
5. joint batch allocation from #5186;
6. joint batch allocation with the existing maneuverability-reserve dimensions;
7. reciprocal option-preservation as an independent evaluation layer applied to each candidate schedule.

Where NASA's exact implementation is not available as executable source, use a transparent, labelled reference implementation of the published algorithm class. Do not call it a faithful NASA reproduction.

## 9. Falsification and metamorphic corpus

At minimum include:

1. **Greedy externality:** one aircraft's allocation removes every robust continuation for another on a declared branch.
2. **Avoidable loss:** a matched alternate allocation preserves a joint continuation.
3. **Unavoidable conflict:** no policy preserves every aircraft's options; report the trade rather than inventing a feasible joint plan.
4. **Clairvoyance negative control:** hidden branches with identical delivered observations must share the same action.
5. **Observation refinement:** genuinely different, timely delivered observations may permit different future actions.
6. **Individual-vs-joint trap:** every aircraft has an individual continuation, but shared capacity makes all combinations incompatible.
7. **Emergency insertion:** an emergency request tests whether reserve options are real and not merely nominal slack.
8. **Resource outage:** remove a landing/charging/crossing resource and preserve exact recovery witnesses.
9. **Heterogeneous action domains:** ensure raw counts are not compared across unlike aircraft.
10. **Branch omission:** omit an admissible disturbance successor; the result must become incomplete/not-assessable, not robust.
11. **Stale or unsupported profile:** reject the affected option set without contaminating other aircraft's evidence.
12. **Permutation invariance:** request order, aircraft IDs and resource labels may change without changing semantic results.
13. **Monotone constraint expansion:** adding a hard restriction cannot create a new feasible continuation.
14. **Replay and identity:** changed profile, schedule, observation, branch or oracle digest invalidates the old receipt.

The tiny oracle should enumerate all action/policy choices and all coupled adversarial successors. The implementation under test must not call the oracle to produce its own expected result.

## 10. Acceptance targets

- 100% agreement with an independently authored exhaustive oracle on the frozen finite corpus.
- Zero hard capacity/conflict/applicability violations accepted.
- Zero clairvoyant policy branches.
- 100% accounting of declared aircraft, successor branches, delivered observations, resources and recovery terminals.
- Zero unknown, stale, unsupported or incomplete branches promoted to robust continuations.
- Exact deterministic replay from frozen manifests.
- Exact semantic invariance under input-order and ID relabelling.
- Explicit distinction among robust, possible-only, unresolved, incomplete and dead-end outcomes.
- Report compute/latency and memory separately; do not relax soundness or completeness gates to improve them.

These are finite synthetic benchmark targets, not real-world aircraft performance or safety claims.

## 11. Literature and competitive positioning

Reachability, zonotopes, viability theory, multi-agent option preservation and demand/capacity balancing are established research areas. Relevant references include:

- ETH Zürich, Air Traffic Control: backward reachability and reach-avoid tubes for maneuvering freedom under wind disturbance: https://control.ee.ethz.ch/research/applications/air-traffic-control.html
- *Determination of reachable flight regions: a zonotopic approach* (2017), combining zonotopic state estimation and reachable-tube prediction; preliminary CALC landing-challenge simulations: https://www.sciencedirect.com/science/article/pii/S240589631731162X
- NASA, *Demand-Capacity Balancing Algorithms for Urban Air Mobility Operations* (2024): https://ntrs.nasa.gov/citations/20240011341
- NASA/TP-20250003243, *Flight Evaluation of In-Flight Strategic Path Planning Automation for Future High-Density Operations* (2025): https://ntrs.nasa.gov/citations/20250003243
- EUROCONTROL, *Specification for Trajectory Prediction*, Edition 2.0: https://www.eurocontrol.int/publication/eurocontrol-specification-trajectory-prediction
- EASA, *Easy Access Rules for U-space*: https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space

The testable contribution is not a new reachability or policy-tree theory. It is a reproducible aviation-specific composition of aircraft-applicable options, shared 4D resources, observation-constrained recovery and reciprocal allocation externalities.

## 12. Execution sequence and evidence status

1. Freeze the tiny topology, aircraft-specific domains, delivered observation histories, disturbance coupling and exact oracle outcomes.
2. Define adapters from the #5163 reachability evidence and #5186 candidate schedule receipts.
3. Reuse relevant policy-history, option-set and reciprocal-externality semantics from Symthaea #11291/#11349/#11394.
4. Implement an independent exhaustive finite oracle and adversarial controls.
5. Evaluate the complete matched baseline family and publish per-aircraft and fleet scorecards.
6. Expand only after oracle agreement, non-anticipativity and hard-gate controls pass.

**Current status:** this file is a research specification only. No executable reciprocal-option evaluator, policy-tree integration, performance result or aviation-model qualification is claimed.

**Core invariant:** a schedule is not robustly recoverable merely because each aircraft has a nominal path or each aircraft has some individually feasible continuation. The fleet must retain a non-anticipative, jointly compatible continuation across every declared admissible branch.
