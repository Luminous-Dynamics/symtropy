# AeroWeft Medical Air-Ground Mission-Chain Benchmark

**Tracking issue:** [AEROWEFT-MEDCHAIN-001 #5181](https://github.com/Luminous-Dynamics/symtropy/issues/5181)  
**Status:** synthetic benchmark contract / design v0.1  
**Owners:** Symtropy benchmark composition; existing transport, mission-chain, cargo, aviation and spatial owners retain their model authority.  
**Claim ceiling:** no medical advice, live dispatch, operational flight recommendation, airspace authorization or safety certification.

## 1. Design rule: compose existing transport infrastructure

This benchmark is an aviation-specific application of the existing transport stack:

- #1601 owns whole-life multimodal transport optimization and its graph/objective contracts.
- #1887 owns ordered mission chains, transfer/handoff continuity and payload-state continuity.
- #1636 owns physical cargo qualification gates.
- #1667 owns transport source admission and region/evidence qualification.
- #2279 / #2297 own aviation 4D planning, conflict screening and numerical/broadphase obligations.
- #5163 owns the AeroWeft model-bounded reachable-occupancy research layer.
- Sol-Atlas #189 owns read-only spatial context; Mycelix #5023 owns identity/provenance/federated coordination.

Do not introduce a new generic graph engine, min-cost-flow solver, mission-chain schema, cargo schema or provider coordination protocol here. This benchmark owns the **synthetic scenario, aviation-specific adapters, matched comparison protocol and report semantics**.

## 2. First bounded scenario

Use a fictional, local Cartesian region. The following labels and values are invented solely for a hand-solvable benchmark. They do not correspond to real locations, aircraft, roads, flight times, legal operating areas, medical service levels or operational capacity.

### Nodes

| ID | Synthetic role | Ground / transfer interface |
|---|---|---|
| DEPOT | Medical supply depot | package release and custody handoff |
| HOSP | District hospital | receiving and inventory handoff |
| CLINIC-N | Northern clinic | receiving and custody handoff |
| CLINIC-S | Southern clinic | receiving and custody handoff |
| LAB | Pathology laboratory | sample intake and custody handoff |

Every node is a synthetic service interface, not a claimed airfield, runway, helipad, vertiport or approved landing site. A later spatial fixture may add candidate landing/recovery sites, but geometry alone must not imply operational availability or legal permission.

### Ground service segments

The initial ground-only comparator uses declared direct service-time fixtures:

| Segment | Synthetic travel minutes |
|---|---:|
| DEPOT → HOSP | 45 |
| DEPOT → CLINIC-N | 85 |
| DEPOT → CLINIC-S | 110 |
| DEPOT → LAB | 82 |
| HOSP → CLINIC-N | 35 |
| HOSP → CLINIC-S | 55 |
| HOSP → LAB | 50 |
| CLINIC-N → LAB | 28 |
| CLINIC-S → LAB | 24 |

The initial corpus uses these as fixed synthetic edge attributes, not estimates of any real road or courier service. Directionality, reverse travel, terminal dwell, congestion and capacity must be represented explicitly in a later schema version rather than inferred.

### Candidate aerial service segments

A separate synthetic service fixture may declare candidate air links:

| Segment | Synthetic airborne minutes |
|---|---:|
| DEPOT → HOSP | 18 |
| DEPOT → CLINIC-N | 28 |
| DEPOT → CLINIC-S | 36 |
| HOSP → CLINIC-N | 16 |
| HOSP → CLINIC-S | 23 |
| HOSP → LAB | 20 |
| CLINIC-N → LAB | 14 |
| CLINIC-S → LAB | 13 |

These are abstract candidate service durations only. They do not specify a real path, airspeed, distance, climb, turn, aircraft performance or legal permission. They must not be converted into aircraft kinematics or treated as a flight trajectory. Later executable comparisons must source candidate airborne service options from explicitly qualified aircraft and airspace profiles.

### Handling and transfer

Give each node an explicit synthetic handoff duration and each payload a handling-contract identifier. For example, a sealed medical parcel may require identity/custody continuity; a sample parcel may have a stricter synthetic time/temperature continuity contract. These are benchmark abstractions, not clinical instructions or validated medical packaging requirements.

Every chain records:
- ordered leg and handoff IDs;
- origin/destination agreement at each transition;
- release/receipt responsibility;
- payload identity, mass and declared packaging/handling profile;
- dwell and transfer time;
- deadline and service window;
- unavailable/unknown/failed condition states;
- recovery/retrieval path and final receipt.

A chain that passes every leg separately but loses custody or misses a handoff is not successful.

## 3. Synthetic demand set

Start with three shipments:

| Demand | Origin → destination | Payload | Synthetic deadline |
|---|---|---:|---:|
| MED-01 | DEPOT → CLINIC-N | 1.0 kg | 60 min |
| LAB-01 | CLINIC-S → LAB | 0.5 kg | 45 min |
| SUP-01 | DEPOT → HOSP | 3.0 kg | 50 min |

The masses and deadlines are arbitrary fixtures. No clinical priority or real medical service level is implied. Priority must be an explicit policy input and must not automatically grant a physically impossible or legally unauthorized airspace action.

## 4. Evaluation policies

Run the same demand, payload, handoff and network versions through:
1. road-only;
2. fixed candidate air-service links plus ground transfers;
3. FIFO air-resource reservation;
4. shortest feasible door-to-door chain;
5. the existing #1601 capacity-aware or robust multimodal optimizer;
6. an AeroWeft-informed candidate where aviation-specific constraints are consumed as inputs by the same optimizer.

For the first tiny corpus, enumerate all allowed candidate chains and resource allocations and compute an independent exhaustive reference. Do not call a candidate chain feasible merely because its sum of nominal travel minutes meets a deadline.

## 5. Hard gates

- All required legs and handoffs exist and join in order.
- Payload custody and handling conditions remain continuous.
- Every declared resource capacity and time window is respected.
- Airborne options carry an explicitly supported model/profile and applicable scenario constraints before they are eligible for the aviation-aware comparison.
- Unknown/stale/missing restrictions, source coverage or required performance evidence remain unresolved; they are never interpreted as permission.
- A synthetic reservation is never flight authorization.
- Physical cargo, aircraft, infrastructure and operational approval are outside this benchmark's claims.

## 6. Scorecard

Report each independently:
- deliveries completed and on time;
- median, p95 and worst-case door-to-door service time;
- total transfer/dwell time and missed-handoff count;
- cost per completed shipment and whole-life infrastructure cost;
- energy per delivered kilogram and lifecycle emissions where the accounting boundary supports them;
- aircraft/road/terminal resource utilization and contention;
- cancellation, recovery and disruption outcomes;
- accessibility / service coverage by community class;
- noise and ground-impact proxies with explicit missing-model markers;
- unsupported, unresolved and abstained case counts;
- evidence completeness and deterministic replay.

Hard gate failures cannot be averaged into a favorable score. Show the Pareto frontier for non-hard objectives and report all policy weights and sensitivities.

## 7. Independent oracle and adversarial corpus

The tiny initial case should be small enough to exhaustively enumerate every declared chain and permitted resource allocation. The oracle should be separately implemented from the policy under test.

Required controls:
- missing handoff, wrong origin/destination and duplicate leg;
- payload identity swap and custody gap;
- transfer longer than the remaining deadline;
- capacity exactly at and just beyond its boundary;
- demand-order and node-label permutation invariance;
- a closed landing/transfer resource;
- stale or contradictory airspace constraint;
- unknown aircraft profile;
- weather/wind bound outside model applicability;
- aircraft or charging resource loss;
- impossible recovery after a disrupted leg;
- a deliberately cheaper route that fails one hard gate;
- ground-only route where all air options are unresolved.

Acceptance targets: 100% agreement with the exhaustive oracle on the frozen tiny corpus, zero hard-gate bypasses, deterministic reports and explicit unresolved classifications.

## 8. Data and publication stages

V0.1 uses synthetic-only data and may be published openly.

Any later real South African or international scenario requires separate admission of:
- transport-network source and license;
- airfield/landing site and aeronautical information source;
- aircraft/model performance source and applicability;
- weather/terrain/obstacle source and validity;
- road/ground logistics source and observation period;
- medical handling requirements from qualified service owners;
- privacy and commercial-use permissions.

A public map or public API does not automatically grant commercial, operational or redistribution rights. Real data are not needed to validate the tiny mathematical oracle.

## 9. Go / no-go sequence

1. Freeze the tiny scenario, all fictional values and expected oracle outcomes.
2. Map the candidate legs and transfers onto existing #1601 and #1887 contracts.
3. Verify cargo/handling gates via #1636 interfaces.
4. Admit aviation options only when the corresponding 4D/model applicability evidence is present.
5. Run identical demand and constraints across all baseline policies.
6. Publish service quality, whole-life cost, energy, access, resilience and unresolved cases separately.
7. Expand to a regional scenario only after the tiny oracle and end-to-end replay pass.

The first successful result is a reproducible demonstration of when aerial service does and does not add value—not a claim that every medical shipment should fly.


## Capacity-allocation follow-on

The medical-chain scenario may later feed the air-resource scheduling benchmark
[AEROWEFT-DCB-001 #5186](https://github.com/Luminous-Dynamics/symtropy/issues/5186).
That benchmark compares sequential and batch resource allocation, while
preserving mission-chain continuity (#1887), existing multimodal optimization
(#1601), aircraft model applicability and recovery constraints. It must not
introduce a second generic route or mission-chain solver.
