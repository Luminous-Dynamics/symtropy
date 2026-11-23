# Nonanticipative Service Oracle and Value-of-Information Benchmark v0.1

**Programme:** [SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 #6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Capacity source:** [#6909 — sustainable mobility service capacity](https://github.com/Luminous-Dynamics/symtropy/issues/6909)  
**Linked capacity/request consumer:** [PR #7074](https://github.com/Luminous-Dynamics/symtropy/pull/7074)  
**Clairvoyant reference:** [PR #7129](https://github.com/Luminous-Dynamics/symtropy/pull/7129)  
**Parent research issue:** [#7133 — nonanticipative service oracle](https://github.com/Luminous-Dynamics/symtropy/issues/7133)  
**Branch:** \`research/nonanticipative-service-oracle-v0-1\`  
**Research cut-off:** 23 November 2026  
**Status:** source-level Rust oracle, strict scenario schema, synthetic fixture, CLI and workflow integration committed; exact-head Rust verification remains pending.

## 1. The benchmark error this removes

The existing exact bitmask dynamic programme in #7129 solves a frozen full-horizon request/opportunity assignment. It is valuable because it produces an exact upper bound for a small instance. But it sees all future requests before the first assignment. It is therefore **clairvoyant**: useful as a perfect-information upper bound, not a fair online policy benchmark.

Comparing online FCFS, EDF, MPC, ADP or reinforcement-learning policies directly against that score conflates two different things:

1. **Policy quality:** how well the decision rule uses information available at decision time.
2. **Information advantage:** how much more a decision-maker could achieve if it knew future requests, weather, permission and outages before they were revealed.

This v0.1 adds an exact finite scenario-tree oracle. The tree has shared information nodes. The action is optimized once per node and is inherited by every descendant scenario. Two futures that have identical revealed histories cannot take different actions merely because their hidden futures differ.

The correct comparison is therefore:

- deployed online policy vs exact **nonanticipative** optimum = policy regret;
- exact nonanticipative optimum vs exact **clairvoyant** optimum = value of perfect information (VPI).

The latter is not evidence that a deployable system can achieve the clairvoyant result.

## 2. Paradigm shift: schedule over information, not over a guessed future

A decision at time \(t\) may use only:
- requests whose release/arrival evidence is available by \(t\);
- capacity receipts already produced by the upstream event simulation;
- current weather and permission observations;
- known maintenance/no-go intervals and reservations;
- current state and evidence validity.

It may not use a future request, future weather realization or future charger outage just because those values appear somewhere in the scenario file.

Each tree node carries:
- parent node and decision time;
- conditional branch probability in integer parts-per-million;
- newly revealed request receipts;
- newly revealed aerial opportunity weather/permission state;
- newly revealed fallback availability;
- source evidence reference, observation time and validity interval.

A node is one information set. Its canonical digest excludes the node identifier and hashes the decision time, sorted new observations and the parent information-set digest. Identical sibling histories are rejected unless represented as one shared node. This prevents duplicated nodes from silently choosing different actions.

## 3. Existing capacity receipts remain the source of aerial opportunities

This child does not author another aircraft/battery model.

It re-runs the upstream #6909 capacity event evaluator and the #7074 linked request evaluator. Every aerial opportunity must resolve to the exact event-derived capacity receipt and payload qualification. Aircraft identity, mission start, duration, turnaround and source configuration are not manually re-entered into the tree.

The evaluator compares:
- the exact policy under the scenario information structure;
- the exact clairvoyant optimum for each fully revealed leaf path.

Unknown, denied or no-go weather/permission blocks aerial use. Fallback availability is an explicit observation. A missing or replayed observation is a hard input rejection, not nominal availability.

## 4. Executable v0.1 contract

Files:
- \`tools/ev-passenger-cost-frontier/src/nonanticipative_service_oracle.rs\`
- \`tools/ev-passenger-cost-frontier/src/bin/nonanticipative_service_oracle_screen.rs\`
- \`tools/ev-passenger-cost-frontier/schemas/nonanticipative_service_oracle_v0_1.schema.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/nonanticipative_service_oracle_v0_1.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/nonanticipative_linked_service_v0_1.json\`

Run:

\`\`\`bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin nonanticipative_service_oracle_screen -- \\
  tools/ev-passenger-cost-frontier/fixtures/nonanticipative_service_oracle_v0_1.json \\
  tools/ev-passenger-cost-frontier/fixtures/nonanticipative_linked_service_v0_1.json \\
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json
\`\`\`

The initial bounded envelope is deliberately small:
- at most 4 requests;
- at most 4 combined aerial/fallback opportunities on a path;
- at most 31 tree nodes and 16 leaves;
- at most 4 decision levels;
- at most 1,000,000 transition evaluations.

Expected values use integer probability numerators and a common \(10^{24}\) denominator. The evaluator reports an explicit transition bound rather than silently returning a truncated solution as optimal.

## 5. Frozen two-branch known answer

The fixture has one common root decision and two possible continuations.

At time 0, the system sees:
- one early request, “request-visible-early”;
- one event-derived aerial opportunity from the fixed Dock-shaped capacity simulation;
- a current synthetic weather=Go and permission=Approved observation.

At time 300 seconds, two branches diverge:

| Branch | Conditional probability | New information |
|---|---:|---|
| high-demand | 0.4 | one urgent aerial-only request is revealed; one fallback locker opportunity is available |
| low-demand | 0.6 | no further request is revealed; the same fallback locker opportunity is available |

The hidden request is not visible at the root, even though the frozen fixture stores its request definition. Its evidence is revealed only at the high-demand child. The common root action must be identical across both branches.

### Expected policy and upper bound

The root policy assigns the early visible request to the t=0 aircraft opportunity. At t=300, the high-demand branch has no remaining aerial opportunity for the hidden aerial-only request; the low-demand branch has no further demand.

| Measure | Nonanticipative policy | Clairvoyant leaf optimum |
|---|---:|---:|
| High-demand on-time services | 1 | 2 |
| Low-demand on-time services | 1 | 1 |
| Expected on-time services | 1.0 | 1.4 |
| VPI (expected additional on-time services) | — | +0.4 |

This fixture isolates the value of information. A clairvoyant allocator can reserve the time-zero aerial opportunity for the request that will only be revealed at t=300 in the high-demand branch, and fulfil the early request by fallback at t=300. A nonanticipative allocator cannot condition its time-zero assignment on that future reveal.

The figures above are the independently reasoned synthetic known answer; they remain expected results until the exact-head Rust workflow has executed the evaluator.

## 6. How to compare against contemporary research fairly

### Dynamic truck–drone ADP under weather and stochastic requests

Zhao, Zhang and Coelho (October 2026) formulate a route-based MDP for dynamic truck–drone service with stochastic request arrivals and changing wind. Their approximate dynamic programming approach decides request acceptance, assignment and route actions as new requests arrive. The authors report instances approaching 300 requests and up to 18.58% more accepted orders than wind-unaware benchmark approaches in their own experiments; their article also reports sub-0.2-second online decision time.

This is the most directly relevant online-information comparator. Reproduce their native truck–drone instance family and decision definitions first. Then compare ADP with the exact nonanticipative oracle on small trees and with the same event-derived capacity/resource gates on a depot workload. Do not compare their 300-request routing benchmark numerically with this 2-request synthetic depot fixture.

Source: [Zhao, Zhang & Coelho, Transportation Research Part E 214 (2026), 105074](https://doi.org/10.1016/j.tre.2026.105074).

### Distributionally robust learned battery scheduling

Du and Zhu (16 November 2026) formulate electric heavy-truck swapping-station scheduling as a KL-ambiguity robust MDP and use offline VAE transition estimation with DR-SAC. They report 78.9% peak shaving for their default calibration, and 3.7% relative daily-profit variation under their combined demand/price perturbation. Their paper's high-demand case still has 5.3 constraint-violating periods/day for DR-SAC, versus 2.9 for SAC and 21.7 for DDPG. The authors identify safety-layer work as still needed under high demand.

The lesson for the mobility programme is not “replace DR-SAC with an exact tree DP.” It is to test learned/MPC/ADP proposers behind the same independently qualified hard feasibility checks, then compare policy regret against the nonanticipative oracle on bounded traces. Reward, peak shaving and mean cost must never conceal reserve/permission violations.

Source: [Du & Zhu, Applied Sciences 16(22) (2026), 11346](https://doi.org/10.3390/app162211346).

### Dock-level commercial benchmark

DJI's Dock 4 specifications state:
- battery swap time below 25 seconds, measured from full cover closure to swap completion;
- landing-to-next-takeoff operational turnaround below 45 seconds;
- maximum three stored batteries;
- two parallel charging channels;
- 27 minutes per battery for 15% to 95% charge at 25°C.

DJI labels the swap and turnaround measures controlled-test reference values. The 27-minute charge interval is a different physical measure from the mechanical swap. None of these alone establishes a sustained network mission rate, service-level rate, lifetime cost, weather availability or grid-export eligibility.

Use these values only as a sourced component reference profile. Keep assumed mission duration, pack compatibility, payload, charger input, SOC curve, field outage and request demand explicitly labelled as model inputs unless separately measured.

Sources: [DJI Dock 4 specification](https://enterprise.dji.com/dock-4/specs) and [DJI Dock support/specification page](https://www.dji.com/support/product/dock-4).

### Physical fleet/route evidence

Cicek et al. (2026) study integrated EV-truck and drone delivery using empirical delivery-route data and flight-energy regression. This is a useful external realism comparator for route/energy modelling, but its objectives, fleet and assumptions need native reproduction before transfer to our depot-level service metrics.

Source: [Cicek et al., Drones 10(7) (2026), 476](https://doi.org/10.3390/drones10070476).

Xu et al. (September 2026) model mobile battery-swapping vans, racks, battery inventories and charging under uncertain demand. This is a relevant future comparator for extending the scenario tree to mobile replenishment and multi-site service, but it is a different operating topology from a fixed dock.

Source: [Xu et al., Transportation Research Part D 158 (2026), 105458](https://doi.org/10.1016/j.trd.2026.105458).

## 7. Benchmark tournament

Use the same frozen capacity receipts, request definitions, reveal times, weather/permission evidence, fallback fleet, energy reserves and runtime budget across methods.

Compare:
1. online FCFS;
2. online EDF;
3. rolling-horizon MPC that only sees information available at each decision time;
4. native Zhao-style ADP/value-function approximation on its own truck–drone formulation, then a matched depot adaptation;
5. robust/chance-constrained policies with explicit scenario/distribution assumptions;
6. DR-SAC or other learned policy proposals with an independent deterministic reserve/permission/commitment shield;
7. exact nonanticipative scenario-tree optimum on small trees;
8. clairvoyant exact DP, reported only as a perfect-information upper bound.

Publish separately:
- expected on-time jobs and completed-by-horizon jobs;
- worst-case on-time jobs and worst-case shortfall;
- late and unserved jobs; in-progress-at-horizon;
- p50/p95/p99 request latency and lateness;
- capacity receipt utilization, aircraft wait and fallback share;
- weather/permission/maintenance gate rejections;
- VPI: clairvoyant minus nonanticipative objective;
- policy regret against nonanticipative optimum;
- optimality gap, transitions, runtime and memory;
- hard false-acceptance count for future/stale evidence, permission/weather, receipt replay and reserve violation.

Do not report an “acceptance uplift” without the identical request stream, capacity receipt set and reveal schedule. Do not count a scheduled sortie as a completed delivery.

## 8. Target ladder (research targets, not achievements)

| Benchmark | Target |
|---|---|
| Exact bounded nonanticipative oracle | 100% agreement with an independent enumerator |
| Information-set nonanticipativity | Zero different actions for identical revealed histories |
| Future-information leakage | Zero future request/weather/permission observations accepted |
| Receipt integrity | Zero duplicate/replayed or mismatched opportunity/evidence receipts |
| Hard service gates | Zero aerial admission under unknown/denied permission, no-go weather or protected intervals |
| Capacity safety | Zero aircraft/opportunity double use and no reserve encroachment |
| Larger policy comparison | Mean regret <=1%, p95 <=5% against a tractable nonanticipative reference |
| Decision performance | Target p99 <10 ms for the bounded tree evaluator on declared hardware |
| Tail service | Report worst-case and CVaR/quantile shortfall separately from expected utility |
| Value of information | Publish VPI and the information source whose reveal has the highest marginal value |

VPI itself is a decision-design signal: it can identify whether the most valuable investment is better weather sensing, faster permission confirmation, demand forecasting, reserve visibility, or charger/aircraft state telemetry. It is not a guaranteed commercial revenue value.

## 9. Next research increments

**v0.2 — stronger tree semantics.** Add explicit node action windows, reveal-time chronology, hidden-vs-known future reservation separation, observation freshness cut-offs and an independent exhaustive tree enumerator.

**v0.3 — nonanticipative MPC baseline.** Replay the same scenario tree using receding-horizon decisions. At every node, MPC must receive only that node's observations; report regret against the exact nonanticipative optimum, not the clairvoyant DP.

**v0.4 — forecast value experiment.** Vary which signals are revealed early (demand, wind, site import headroom, charger outage, permissions). Compute marginal VPI per sensor/forecast and compare value with acquisition/communications cost.

**v0.5 — correlated multi-site service.** Add shared fallback vans, mobile swap units, grid/ESS outages, and common-cause weather/communications failures. Continue to consume exact upstream capacity/commitment receipts rather than duplicate those models.

**v0.6 — field calibration.** Replace synthetic request/flight-success assumptions only with timestamped, independently sourced traces. Publish forecast calibration, drift, coverage, missingness and model error.

## 10. Claim boundary

This v0.1 oracle is a bounded exact policy benchmark on declarative synthetic observations and an event-derived synthetic capacity model. It is not a production dispatcher or a deployable policy; it does not route aircraft, estimate calibrated weather distributions, authorize flights, establish physical delivery, qualify batteries or grant market/grid authority. The current implementation and fixtures require exact-head Rust CI verification before any executable pass is claimed.
