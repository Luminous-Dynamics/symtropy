# Nonanticipative Mobility Service Oracle and Scenario Benchmark v0.1

**Programme:** [SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 #6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Scenario-tree issue:** [NONANTICIPATIVE-SERVICE-ORACLE-001 #7133](https://github.com/Luminous-Dynamics/symtropy/issues/7133)  
**Predecessor:** [PR #7129 — clairvoyant exact assignment upper bound](https://github.com/Luminous-Dynamics/symtropy/pull/7129)  
**Branch:** \`research/mobility-nonanticipative-service-oracle-v0-1\`  
**Status:** source-level implementation, strict linked schema, synthetic scenario tree, CLI and 13 focused Rust test definitions committed; exact-head CI pending.

## 1. The benchmark gap: an offline optimum is not an online policy

The exact full-horizon DP introduced in #7129 is useful, but it sees the complete request stream. It can know at time \(t=0\) that a request will arrive at time \(t=350\), and can reserve an opportunity for that future request. A real dispatcher cannot.

Calling that DP result an online optimum would overstate the evidence. It is a **clairvoyant perfect-information upper bound**.

This v0.1 layer introduces an exact bounded scenario-tree reference:
- scenarios share a common information set while their revealed request/weather/permission/opportunity histories are identical;
- the policy chooses one action for the shared information set, not one action per scenario;
- scenarios branch only when their declared observations differ;
- a source-derived service receipt that becomes unavailable under a later weather/permission revelation cannot still be counted as completed service in that branch;
- the full-information DP is retained as a separate upper bound;
- the difference is reported as the value of perfect information, not online-policy performance or algorithmic regret.

## 2. No second aircraft schedule

The oracle deliberately does **not** accept caller-authored aircraft IDs, mission start times, mission durations, turnaround or payload limits.

Its top-level input embeds:
1. the linked request/service package from #6921;
2. the SustainableServiceCapacityInput from #6909;
3. observation nodes and scenario paths.

Before solving, it reruns the #6909 capacity event simulator and the #6921 linked-service verifier. Aerial opportunities are then derived only from exact, payload-qualified capacity receipt IDs and the corresponding source record's aircraft ID, start time, mission duration and turnaround. The report includes:
- capacity source architecture ID;
- canonical capacity report SHA-256;
- exact service-opportunity digest;
- architecture configuration digest;
- total source opportunity count;
- qualified opportunity count.

A node may reveal only IDs that exist in this verified derived set. Adding an arbitrary \`opportunities\` array to the input is rejected by the deserializer. Fallback opportunities remain sourced from the linked #6921 input, not re-created in the oracle.

## 3. Information-set contract

Each observation node includes:
- decision time;
- cumulative revealed request IDs;
- currently feasible source receipt IDs;
- weather and permission state;
- exact flight-window provenance ID;
- weather and permission evidence digests;
- parent and information-set digests.

Observation content is canonicalized and SHA-256 hashed. Two nodes with the same information-set digest must have identical parent, decision time, revealed request set, feasible opportunity set, weather and permission state. Parent histories must be monotonic: requests already revealed cannot disappear, and child decision times must advance.

The state model is scenario-based and explicitly synthetic. The evidence digest binds an observation outcome to the linked time-bounded flight-evidence envelope; it is not a signature or independent measurement. Scenarios that remove a receipt from their final feasible set cannot count an earlier reservation as a successful service.

Unknown weather or permission blocks aerial assignment. A no-go scenario may leave fallback options available in future versions; the current fixture isolates the aerial service opportunity so its refusal is visible.

## 4. Frozen repeated-information challenge

The fixture uses the \`fixed_dock4_reference_profile\` as an upstream event-source profile. It qualifies one exact receipt: \`aircraft-00-mission-000002\`. The mission's aircraft, start time, duration and turnaround are re-derived from #6909; they are not repeated in this oracle fixture.

Three scenarios share the same root observation at 349 seconds:
- no later request (300,000 probability units);
- later urgent request arrives at 350 seconds and weather remains go (350,000);
- later urgent request arrives at 350 seconds but weather is revealed no-go (350,000).

At the root, only \`request-early\` is visible. Its acceptance cutoff is 349 seconds, so it cannot be silently kept eligible at the next decision. In the go branch, the later request becomes eligible and can use the future receipt. In the no-go branch, the receipt is removed from the physically feasible set and cannot be scored as a successful delivery even if a prior policy action had reserved it.

The independent small-tree enumeration used to design the fixture gives:
- nonanticipative policy: defer at the shared root; serve the later request only in the revealed go branch;
- expected on-time services: 350,000 weighted units (0.35 expected service);
- expected completed-by-horizon services: 350,000 weighted units;
- clairvoyant perfect-information upper bound: 350,000 on-time and 650,000 completed-by-horizon weighted units;
- value of perfect information: 300,000 additional completed services weighted by scenario probability.

These are frozen synthetic reference values, not executed Rust test results or real operational throughput.

## 5. Research and industry comparisons

### 5.1 Dynamic truck–drone ADP under stochastic demand and weather

Zhao, Zhang and Coelho (October 2026) formulate a route-based MDP for a truck equipped with one drone. Static requests are known in advance, dynamic requests arrive stochastically, and time-varying wind affects ground speed and energy. They develop approximate dynamic programming with value-function approximation and an online route enhancement step. Their reported experiments include instances approaching 300 requests and 18.58% more accepted requests than wind-unaware benchmark approaches.

This is a highly relevant **scalable online-policy comparator**, not an exact oracle. The testbed is one truck/one drone, so its reported acceptance-rate gain is not directly transferable to a docked fleet, multi-aircraft depot or our synthetic fixture. Reproduce the paper's own workload/denominators first; then compare ADP against the same source-derived capacity, requests, weather branches and deadline policy used by FCFS/EDF/MPC. The v0.1 nonanticipative oracle supplies a small-instance ground truth for those same information sets.  
Source: [Zhao et al., Transportation Research Part E 214 (2026), 105074](https://doi.org/10.1016/j.tre.2026.105074).

### 5.2 Commercial autonomous dock

DJI's official Dock 4 specification separates several performance dimensions:
- battery swap: under 25 seconds, measured from cover fully closed to swap completion in a controlled test;
- landing-to-next-takeoff turnaround: under 45 seconds, also a controlled-test/reference value;
- three batteries maximum across its four compartments;
- two parallel charging channels;
- 27 minutes to charge a battery from 15% to 95% at 25 °C;
- one accommodated aircraft and 2,000 W maximum AC dock input.

These are manufacturer specifications, not measured network-wide mission service or grid-flexibility claims. The 25-second battery mechanism and 27-minute replenishment are different processes. The 2 kW figure is the dock's maximum AC input and must not be divided into assumed channel power without further specification. The linked #6909 Dock-shaped profile is a comparison fixture with an explicitly synthetic charge-power assumption, not a reconstruction of DJI's proprietary charging curve.  
Source: [DJI Dock 4 official specifications](https://enterprise.dji.com/dock-4/specs).

### 5.3 What the Luminous comparison is actually measuring

| Dimension | Existing SOTA/product comparator | New benchmark contribution |
| --- | --- | --- |
| Dynamic request arrivals | Zhao et al. route-MDP/ADP | Exact scenario-tree policy that cannot see unrevealed requests |
| Weather uncertainty | Wind-aware ADP and flight-window constraints | Branch-specific go/no-go revelation with evidence cutoff |
| Assignment optimality | Offline full-horizon DP | Separate nonanticipative optimum and clairvoyant upper bound |
| Capacity realism | Dock specifications and capacity simulator | Opportunities re-derived from source event receipts |
| Decision quality | Acceptance rate / cost / route objective | On-time, completed, late, unserved, latency, VPI and no-go loss on identical paths |
| Safety | Flight/weather constraints | Zero assignment to no-go/unknown observations and source-bound opportunity availability |

The differentiator is not yet a faster online dispatcher. It is the ability to prove whether an online policy's decisions were optimal under exactly the information that existed at the time, and to quantify how much apparent performance comes from future-information leakage.

## 6. Paradigm shifts

1. **Treat information as a first-class resource constraint.** An algorithm cannot use a request, weather observation, permission decision or charger state before its declared reveal time.
2. **Optimize over information sets, not independent scenarios.** Shared histories share one action. This is the nonanticipativity constraint used in multistage stochastic programming, made explicit and replayable.
3. **Make perfect information a diagnostic, not a leaderboard.** The clairvoyant score is an upper bound. The gap is a measure of information value, not an attainable operating result.
4. **Let physical feasibility invalidate promises.** A future weather no-go removes the exact opportunity from that branch; previous reservations do not become successful delivery.
5. **Keep schedule truth upstream.** #6909 derives aircraft/time/turnaround receipts; #6921 binds payload, requests, fallback and flight evidence; the new oracle reasons over those receipts without creating a parallel schedule.
6. **Separate planning, commitment and execution.** The nonanticipative policy still has no dispatch authority. Any actual action must pass the #6827 commitment lease and #6832 journal semantics, plus the physical/flight safety authorities.

## 7. Benchmark protocol and target ladder

### Frozen small-instance assurance

- 100% equality of actions for nodes sharing an information-set digest.
- Zero use of a request before its reveal/acceptance cutoff.
- Zero aerial acceptance under weather no-go/unknown or permission denied/unknown.
- Zero acceptance of an opportunity absent from the #6909 source report or failing linked payload qualification.
- 100% agreement with a separately implemented exhaustive policy enumerator on the bounded fixture family.
- 100% replay determinism under request/opportunity/node/scenario input permutations.
- Every bounds-exceeded or transition-limit case returns an explicit non-optimal status without partial optimum/regret fields.

### Online-policy tournament

Run FCFS, EDF, greedy earliest-feasible, rolling-horizon MPC, approximate dynamic programming/ADP and (where training data permits) RL policies over identical request streams and scenario trees. Report:
- expected on-time services and completed services;
- late, in-progress, unserved and weather/permission-aborted requests;
- median/p95/p99 request-to-decision latency;
- opportunity utilization and idle/stranded capacity;
- service latency and deadline lateness distributions;
- acceptance rate with the same denominator;
- value-of-perfect-information gap, separately from policy regret.

Targets, not achievements:
- mean regret <=1% and p95 <=5% against the nonanticipative exact optimum on held-out cases where exact evaluation is tractable;
- zero hard weather/permission/evidence false acceptance;
- exact shared-information-set action equality = 100%;
- target p99 decision time <=100 ms for bounded online policy candidates on a declared reference runner (not the exact oracle's exhaustive search);
- scale the nonanticipative reference progressively to 8/16/32 requests with explicit node/transition limits and report abstention rates;
- no SOTA percentage claim without reproducing the named baseline and its native data split/denominator.

### System-level service benchmark

Retain separate metrics for mechanical swap, landing-to-takeoff turnaround, charging restoration, fleet availability, successful on-time mission-hours and verified grid service. The primary system objective should be:

\[
\frac{\text{Successful on-time mission-hours}}
{\text{Aircraft + battery + charging + carrier + site + operating lifecycle cost}}
\]

Grid revenue is reported separately and only for actually measured, authorized, delivered services.

## 8. Phasing

- **v0.1:** bounded exact nonanticipative scenario-tree oracle, linked capacity receipts, request/weather/permission revelation and clairvoyant value-of-information comparison.
- **v0.2:** independent exhaustive enumeration across randomly generated small trees, additional negative controls, scenario-conditioned fallback recourse and explicit opportunity reservation/release events.
- **v0.3:** rolling-horizon MPC/ADP adapters with fixed observation contracts; held-out stochastic request and weather traces; calibrated wind/energy model inputs.
- **v0.4:** larger scenario-tree decomposition, policy compression and runtime/regret tournament; exact oracle remains the small-instance reference.
- **v0.5:** hardware/site-in-the-loop evidence, measured weather/permission and physical mission outcomes, with no synthetic result promoted to field claim.

## 9. Claim boundary

This v0.1 oracle is a bounded synthetic reference over linked event-derived capacity receipts, supplied request inputs, scenario probabilities and declared observation histories. It is not a deployable dispatcher, calibrated demand forecaster, wind model, route planner, flight safety verifier, physical service certificate or market authority. Exact output demonstrates optimality only inside the declared bounded model and information structure.
