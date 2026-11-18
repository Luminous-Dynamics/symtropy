# MFB Drone–Carrier Co-Availability Contract v0.1

**Status:** research design contract; synthetic-only until source and physical qualification are admitted  
**Parent:** [MFB-DRONE-CARRIER-COOPT-001 #6083](https://github.com/Luminous-Dynamics/symtropy/issues/6083)  
**Programme:** [Material Flow Benchmark #5422](https://github.com/Luminous-Dynamics/symtropy/issues/5422)

## 1. Purpose and boundary

This document defines the interface between a proposed electric-carrier itinerary and one or more candidate delivery-UAV sorties. It is a feasibility/admission layer, not another route planner, aircraft model, battery model, vehicle dynamics model, flight controller, airspace service or command authority.

The evaluator consumes source-bound capability envelopes and candidate plans. It returns `FEASIBLE`, `INFEASIBLE` or `INDETERMINATE`, with explicit reasons and evidence references. It never dispatches, reserves or actuates.

A route optimizer may propose a sortie. The evaluator independently checks whether that proposal can be supported by the admitted input evidence. A structurally valid plan is not an operational authorization.

## 2. Ownership and adapters

| Concern | Owner to consume | Boundary in this contract |
|---|---|---|
| UAV mission power / energy / mass closure | Symthaea AIRCRAFT-ENERGY-001 #7646 and related mission/propulsor benchmarks | Read-only phase-resolved capability envelope |
| Battery and energy-system evidence | Symtropy #3885, #3903, #4080 | Read-only pack/system limits and evidence identity |
| Road vehicle / EV mission | Symtropy #2112; Symthaea #6247 | Read-only route, duty and time-indexed capability |
| Multimodal routing / resource allocation | Symtropy #1601 and #1639 | Candidate itinerary and resource intervals only |
| Material-flow and custody | Symtropy #5422; Mycelix #6259 | Package, endpoint, handoff and operational-authority references |
| Spatial site context | Sol-Atlas PR #879 | Evidence-qualified context only, not airspace or legal permission |

An adapter must preserve original source identity, unknown fields, units, validity interval, geography, evidence class and supersession status. It must not convert missing data into zero or silently translate a reference-design assumption into an as-built fact.

## 3. Canonical evaluation objects

### 3.1 Evaluation manifest

Every run binds:
- schema and evaluator version;
- exact source commit / build identity;
- canonical scenario and configuration identities;
- seed and deterministic-replay policy;
- hashes of every input envelope, route, weather record, permission record and fixture;
- claim ceiling and evidence tier.

### 3.2 Carrier itinerary

A carrier itinerary contains ordered stops and time windows. Each stop has a canonical physical location reference, arrival interval, service interval, departure interval, road-resource claims, cargo state and vehicle energy boundary. Times must include a time basis and uncertainty semantics; wall-clock strings without a timezone or time authority are invalid.

A rendezvous references a canonical physical site and a compatible access window. Two geometrically coincident but separately identified locations are not automatically the same resource.

### 3.3 UAV capability envelope

The UAV envelope is supplied by its responsible engineering owner and includes:
- aircraft configuration and revision;
- pack/system configuration, evidence and validity domain;
- payload mass, dimensional and centre-of-gravity limits;
- phase-specific duration, continuous/peak power and energy bounds;
- usable battery energy and power versus SOC, SOH and temperature;
- reserve and contingency policy;
- valid payload, wind, temperature, altitude, takeoff/landing and duty domains;
- required turnaround, cooling, charging or exchange interval;
- uncertainty, calibration and evidence class.

The contract may not infer aerodynamic performance from nominal range, motor peak power, cell-level specific energy or a marketing specification.

### 3.4 Sortie proposal

A sortie binds:
- unique proposal and aircraft identity;
- launch carrier, launch site and launch interval;
- package and destination identities;
- ordered mission phases and phase intervals;
- recovery carrier/site and recovery interval;
- candidate alternative disposition (e.g. return to launch site or named safe diversion) only if that alternative is separately supported;
- resource claims for pad, dock, crew, charger and package handling.

A sortie identity cannot be reused for a different payload, configuration, route or evidence set.

### 3.5 Environment and operating-domain envelope

Environmental records include source, sensor/model, timestamp, spatial coverage, forecast horizon, uncertainty, wind vector and gust, temperature, visibility and precipitation as available.

Airspace/site restrictions and permissions are separate typed records. A map overlay or geospatial suitability score is not permission to fly. Missing, stale, conflicting, revoked or out-of-scope authorization yields `INDETERMINATE` or `INFEASIBLE` according to the reason; it never defaults to allowed.

## 4. Units and energy boundaries

Use explicit SI base units in machine contracts:
- mass: milligrams or grams (choose one per schema version and document range);
- distance: millimetres;
- time: integer milliseconds or nanoseconds;
- power: milliwatts or watts;
- energy: milliwatt-hours or watt-hours;
- temperature: milli-degrees Celsius or kelvin;
- angles: microradians or radians.

The first executable schema must pick a single integer representation for each quantity and reject overflow, NaN, infinity, negative physical quantities where prohibited, and ambiguous units.

Maintain separate ledgers for:
1. carrier battery-terminal traction / auxiliary energy;
2. UAV battery-terminal mission energy;
3. carrier-to-UAV charging input, conversion loss and battery-terminal accepted energy;
4. site input energy and upstream generation/storage boundaries if evaluated;
5. work/tool/handling energy where included.

Do not count the same onboard truck-battery energy once as truck consumption and again as free energy delivered to the aircraft. If charging from the truck, show the carrier battery withdrawal, converter losses and UAV battery acceptance as linked ledger entries. The same rule applies to depot or stationary buffer energy. Report energy per successful delivery using a clearly declared whole-system boundary.

## 5. Feasibility semantics

A candidate is `FEASIBLE` only when every required hard constraint is supported by fresh, in-domain evidence and every phase fits its time, energy, power, payload and resource envelopes.

A candidate is `INFEASIBLE` when a known constraint is violated, such as:
- package mass, dimensions or centre of gravity outside the admitted envelope;
- peak or continuous power exceeded in any required phase;
- usable battery energy after SOH/temperature/rate derating insufficient for mission plus declared reserve;
- charge / cooling / exchange turnaround cannot fit before the next sortie;
- launch or recovery window missed;
- physical dock, charger, crew or aircraft resource conflict;
- environment outside the declared operating envelope;
- an explicit route/airspace/site restriction excludes the operation.

A candidate is `INDETERMINATE` when required evidence is unknown, stale, conflicting, revoked, unsupported, outside calibration domain or missing. It must not be ranked as feasible because the model has an optimistic point estimate.

Do not collapse the three outcomes into a numeric confidence score. Reasons and violated/unknown constraints must remain individually inspectable.

## 6. Time and synchronization

Use interval semantics rather than single-point ETA. A sortie's launch must fit the carrier's stationary or permitted launch interval. Recovery must fit the identified recovery site's interval and the carrier's arrival/departure window. Model loading/unloading, safe access, pad occupancy, charging, cool-down and dispatch preparation as explicit durations/resources.

The v0.1 slice covers:
- truck-only service;
- parked-carrier launch and same-site recovery;
- parked-carrier launch and different authorized static rendezvous;
- fixed-hub aircraft service as a separate baseline.

Moving-carrier rendezvous, in-motion deployment and in-motion recovery are excluded from v0.1. They need a later dynamics, control and safety qualification scope.

## 7. Uncertainty and reserve

Never treat the expected value as a safe upper bound. The first implementation should support bounded/worst-case scenario envelopes and explicit reserve policy. A future probabilistic layer may report distributions and chance constraints, but must preserve the hard deterministic constraints and label calibration evidence.

At minimum, evaluate uncertainty in:
- wind vector and gust;
- carrier travel and stop duration;
- mission phase energy and power;
- battery available energy / power under SOC, SOH and temperature;
- charge acceptance and turnaround;
- package mass and handling time where the source does not provide exact measured values.

Missing uncertainty is not zero uncertainty.

## 8. Benchmark lanes and reporting

Compare under identical demand, service windows, package eligibility, road graph, weather, permitted operating domain, depot, staffing, charging resources, energy boundary and failure injections:

1. truck-only;
2. truck plus parked-carrier UAV;
3. UAV from fixed hub / depot;
4. truck plus ground robot / AMR;
5. mixed truck, UAV and ground-robot operation.

Reproduce deterministic greedy and rule-based baselines before heuristic or learning methods. On small standard TSP-D / FSTSP / VRP-D instances, use exact or exhaustive solutions where feasible and report proof status / optimality gap. On larger instances, compare ALNS / decomposition and the existing multimodal allocator under identical compute budgets. Preserve source instance identity and license. Report variant-specific results: same-node return and distinct-node rendezvous are not interchangeable.

Metrics must remain disaggregated:
- on-time authorized end-to-end completed deliveries;
- late, unserved and excluded demand;
- false-feasible and false-infeasible outcomes;
- system energy per successful delivery by boundary;
- lifecycle cost per successful delivery;
- synchronization idle/wait time and missed rendezvous;
- utilization and charging / handling queues;
- battery throughput, degradation assumptions and maintenance downtime;
- fault recovery, safe disposition and human intervention;
- runtime, memory and optimality gap.

## 9. Negative-control minimum set

The evaluator test suite must include at least:
1. overweight package;
2. package outside dimensional envelope;
3. centre-of-gravity boundary violation;
4. peak power exceeded;
5. continuous power exceeded;
6. energy sufficient at nominal conditions but insufficient after reserve / SOH / temperature derating;
7. charge turnaround outside the dispatch window;
8. stale or contradictory weather;
9. wind / temperature outside aircraft envelope;
10. stale, revoked, unknown or wrong-scope permission;
11. carrier arrival after launch window;
12. missed recovery rendezvous;
13. charger / pad / crew / aircraft interval overlap;
14. stale or changed aircraft capability digest;
15. duplicate sortie identity with changed payload or itinerary;
16. post-launch lost acknowledgement / network partition: no blind duplicate command and reconciliation remains required.

## 10. First executable acceptance slice

- Closed, versioned JSON Schema with deny-unknown-fields behavior at the Rust boundary.
- Rust domain types with checked integer arithmetic and explicit units.
- One synthetic carrier route, one synthetic UAV configuration, one parcel, one destination, one same-node sortie, one distinct-node rendezvous, and a truck-only comparator.
- Independent hand-computable timing, mass, energy, reserve and resource-capacity oracles.
- Property tests for input order, overflow, interval overlap, duplicate identities and missing evidence.
- Every negative control reaches its expected `INFEASIBLE` / `INDETERMINATE` outcome.
- Canonical byte-identical report for 100/100 input-order permutations.
- SHA-256 manifest bound to exact source, inputs, config, schema and seed.
- All values labeled synthetic; no aircraft performance, actual savings, HIL or field performance claim.

## 11. Qualification ladder

- L0 contract and arithmetic tests;
- L1 deterministic simulation and replay;
- L2 software/HIL interface tests with production-intent sensors/controllers and no physical payload risk;
- L3 controlled rig tests with guarded fixtures and manual recovery;
- L4 restricted field pilot under independently reviewed operating procedures and required regulatory permissions.

The benchmark must never upgrade an L0/L1 result to HIL or field evidence by implication.

## 12. External baseline notes

The 2026 review by Liu et al. identifies synchronization, energy uncertainty, rendezvous design, airspace constraints and scalability as key open challenges. It reports branch-price-and-cut results up to 50 vertices for a specific VRP-D method and describes ALNS as prevalent for larger instances. That is context for benchmark design, not a universal state-of-the-art guarantee: https://link.springer.com/article/10.1186/s12544-026-00808-2.

Commercial reference architectures also differ materially:
- Amazon Prime Air describes hub-based service with pre-planned flight paths, defined operating envelopes, monitoring, detect-and-avoid and contingency landing: https://www.primeair.amazon/operations.
- Wing/Walmart describe a large fixed-site delivery network: https://wing.com/news/wing-walmart-expand-drone-delivery-coast-to-coast.
- Zipline describes docking/charging and hub/dropbox operations: https://www.zipline.com/about/zipline-fact-sheet.

These are distinct comparators. Their stated performance/scale figures should not be treated as directly comparable with a truck-carried drone system without normalizing service area, parcel eligibility, weather, operating hours, aircraft payload, fleet density and infrastructure.

For South Africa, SACAA UAS information and the Civil Aviation Regulations must be treated as jurisdictional gating inputs. The 2023 amendment to Part 101 regulation 101.05.4 addresses release/delivery of objects and requires the relevant certificate holder and approved operations-manual provision. Exact applicability and the autonomous-system approval route require regulator confirmation: https://www.caa.co.za/industry-information/uas/ and https://www.gov.za/sites/default/files/gcis_document/202303/48228rg11556gon3169.pdf.

## 13. Non-claims

This document is an engineering interface proposal. It does not establish accurate aerodynamic or battery models, flight safety, airworthiness, legal permission, a working dispatch system, real-world energy savings or commercial superiority.
