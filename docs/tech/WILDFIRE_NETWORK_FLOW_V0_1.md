# Wildfire Directed Network Flow Prototype v0.1

**Issue:** [WILDFIRE-NETWORK-001](https://github.com/Luminous-Dynamics/symtropy/issues/2269)  
**Fixture:** [WILDFIRE_NETWORK_FLOW_V0_1.json](../../benchmarks/fixtures/WILDFIRE_NETWORK_FLOW_V0_1.json)  
**Status:** deterministic synthetic fixed-route network model; not a full CTM and not an operational evacuation planner.

## Why model the network, not isolated routes?

SUMO's Dynamic User Assignment documentation warns that sending all vehicles down the empty-network fastest paths can jam shared bottlenecks. SUMO already supports incremental assignment, periodic rerouting and iterative approximate dynamic user equilibrium. Symtropy should compare against those established mechanisms rather than use a straw-man baseline: https://sumo.dlr.de/docs/Demand/Dynamic_User_Assignment.html.

Moreno et al. (2026) model evacuation route, shelter and flow decisions using a Cell Transmission Model with time-varying road properties, multiple periods, capacities and exact Benders decomposition. Their results on their own study instances are not claimed by this prototype; the study data are listed as available on request: https://doi.org/10.1016/j.tre.2026.104965.

## Model boundary

This v0.1 model simulates directed road-link queues with fixed OD routes, discrete travel delays, per-step sending/entry limits, finite storage, time-dependent *entry* closures, merge contention, destination ingress limits, and total/accessibility shelter capacity. Vehicles transfer at most one edge per simulation step because requests are built from the pre-step state. New entrants do not consume a downstream link's freed storage until the following step; this is conservative and explicit.

When several streams request the same receiving link, a deterministic round-robin scheduler allocates its entry slots while respecting each upstream link's sending capacity. A closed link blocks new admissions at or after its configured step; vehicles already admitted to that link may leave it. Queues that cannot progress remain counted at the source or on the network as unserved/stranded at the horizon.

This is **not a full Cell Transmission Model**: links are finite FIFO queues with travel delay, not multiple density cells governed by a fundamental diagram; v0.1 has fixed routes and no rerouting, generalized signalized junctions, contraflow, pedestrian movement, fire spread, smoke, behavior model or calibration against observed traffic. It is a network-conservation step beyond isolated candidate corridors, not parity with SUMO/Visum/Vissim.

## Frozen scenarios

1. Two origin streams merge into one finite receiving-capacity bottleneck.
2. A sink ingress bottleneck fills finite road storage and propagates backpressure upstream.
3. A bridge stops accepting new vehicles during a departure wave; already admitted vehicles remain accounted for.
4. A one-vehicle-per-step shared link tests deterministic round-robin merge fairness.
5. A shelter's total capacity is a hard ceiling.
6. A shelter's accessible capacity is independently hard; blocked vehicles are not miscounted as served.

## Invariants

- Every vehicle is in exactly one state at each step: source queue, one directed link, or an arrived sink.
- Source queue + all link occupancy + cumulative arrivals equals demand each step and at the horizon.
- Link entry and exit counts do not exceed the configured per-step capacity.
- Link occupancy never exceeds storage.
- No new vehicle enters a closed edge at or after its closure step.
- Sink cumulative arrivals, accessible arrivals and per-step ingress remain within their distinct constraints.
- Input order does not affect served counts or clearance under the declared deterministic scheduler.
- Receipt pins exact source SHA and the raw fixture SHA-256. A separate Rust verifier recomputes stepwise mass balance, per-link entry/exit capacities, post-closure entries, sink constraints, percentile calculations, served/unserved totals and aggregate scorecard before artifact upload.

## Evaluation metrics and next steps

Report total and per-demand served/unserved vehicles; clearance step; p50/p90/p95 arrivals when individual arrivals exist; queue vehicle-steps; max source/network queue; per-link occupancy, entry/exit traces and utilization; closure-blocked admissions; and total versus accessible shelter arrivals. Keep feasibility and accessibility violations non-compensatory.

The next steps are (1) add movement-level FIFO rules at diverges and more realistic node models, (2) generate fixed-seed network topology families and property tests, (3) write an exporter/adapter for a matched SUMO network and time-stamped demand, and (4) compare using identical graph, demand, closures, link capacities, storage, shelters, time step and compute budgets. Only after that compare with a published CTM/mixed-integer baseline or held-out drills. Synthetic tests do not establish real-world safety or SOTA superiority.
