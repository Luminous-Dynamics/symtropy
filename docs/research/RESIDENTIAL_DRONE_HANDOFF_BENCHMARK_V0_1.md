# NestPort × Symtropy — Residential Drone Handoff and Recoverability Benchmark v0.1

**Status:** Research benchmark contract; synthetic-first. Not a flight release, aircraft design approval, safety case, or certification basis.  
**Date:** 2026-11-15  
**Programme:** NestPort / SkyGarden / AeroWeft  
**Owners:** Symtropy (physical scenario/replay), Symthaea (model proposals and uncertainty), AeroWeft (aircraft/mission envelope), Sol-Atlas (site context), Mycelix (resident rights and custody receipts).

## 1. Purpose and non-goals

Existing residential drone delivery demonstrates that aircraft can move useful payloads over short routes and perform controlled handoffs. The engineering gap addressed here is the coupled **aircraft approach × payload transfer × residential geometry × receiving mechanism × storage × recoverability** problem.

This benchmark asks:

- Can the delivery method reach a declared handoff volume without falsely classifying an unsafe site as acceptable?
- Can the transfer mechanism place a parcel inside the receiving system's declared acceptance envelope?
- Can the system detect when the resident, animal, vehicle, weather, obstacle, station, or package state invalidates delivery?
- Can the aircraft and logistics process abort, divert, retry, or return without losing custody or creating a second ambiguous mutation?
- Can the receiving station secure and store the parcel under its declared environmental conditions?

This is not a benchmark for beating Wing, Zipline, or Amazon on aircraft range, speed, payload, or fleet scale. Those systems have proprietary aircraft, operational data, and approval envelopes that cannot be recreated from public marketing specifications. Their public operating profiles are reference architectures, not direct numerical baselines.

No controller or planner developed from this benchmark may command an aircraft or enable a live flight. Physical operation remains under the qualified aircraft operator and applicable aviation authority.

## 2. Existing research to reuse

Do not duplicate existing system-identification and aircraft-reachability work.

The public Symtropy research branches already contain:

- Nano-quadrotor dataset ingress and a multi-horizon metric receipt;
- failure-aware rollout and intervention-conditioned recoverability evaluators;
- AeroWeft kinematic reachability, independent analytic-extrema fixtures, and an aviation benchmark suite;
- mission-level electric-aviation co-design research.

Relevant research artifacts (separate research branches; not a claim of default-branch integration or physical-flight validation):

- [AeroWeft benchmark suite](https://github.com/Luminous-Dynamics/symtropy/blob/feat/aeroweFt-kinematic-reachability-001/docs/research/aviation/AEROWEFT_BENCHMARK_SUITE_V1.md)
- [AeroWeft kinematic reachability increment](https://github.com/Luminous-Dynamics/symtropy/blob/feat/aeroweFt-kinematic-reachability-001/tools/aeroweft-reachability/RESEARCH_INCREMENT_2026-11-14.md)
- [Nano-quadrotor system-identification adapter](https://github.com/Luminous-Dynamics/symtropy/blob/research/nanodrone-metrics-parity-v0-1/docs/research/SYMTROPY_NANODRONE_SYSTEM_IDENTIFICATION_ADAPTER_V0_1.md)
- [Nano-drone failure-aware rollout](https://github.com/Luminous-Dynamics/symtropy/blob/research/nanodrone-recoverability-frontier-v0-1/docs/research/SYMTROPY_NANODRONE_FAILURE_AWARE_ROLLOUT_V0_1.md)
- [Nano-drone recoverability frontier](https://github.com/Luminous-Dynamics/symtropy/blob/research/nanodrone-recoverability-frontier-v0-1/docs/research/SYMTROPY_NANODRONE_RECOVERABILITY_FRONTIER_V0_1.md)
- [Electric aviation Gen4 RFC](https://github.com/Luminous-Dynamics/symtropy/blob/research/electric-aviation-gen4-rfc-2026-11/docs/research/ELECTRIC_AVIATION_GEN4_ENGINEERING_RFC_2026-11-12.md)

The IDSIA Nano-Quadrotor data is based on Crazyflie 2.1 Brushless flight data with four motor-speed inputs. It is useful for small-quadrotor identification and model comparison, but is not representative of a larger parcel-delivery aircraft, tethered payload, residential transfer mechanism, or aircraft with a different propulsion/control topology. Keep those regimes separate and label any transfer of model class as a hypothesis requiring new evidence.

Symtropy's documented physics core is rigid-body-oriented. Do not describe it as a validated flexible-cable solver, CFD package, structural FEA, acoustic model, or building-physics engine. Those phenomena require specialized models, analytic fixtures, measured data, and explicit applicability limits.

## 3. System under test

Treat the delivery system as six coupled but separately qualified elements:

1. **Aircraft:** vehicle geometry, mass properties, propulsion topology, control model, sensor package, energy state, payload capacity, and declared operating envelope.
2. **Mission/approach:** route, time window, airspace constraints, weather, obstacles, uncertainty, and reserved recovery options.
3. **Payload:** parcel mass, dimensions, centre of mass, packaging stiffness, attachment point, allowable acceleration, and handling class.
4. **Transfer mechanism:** touchdown interface, winch/tether, controlled lowering, chute, or other declared handoff method.
5. **Residential site and station:** building/terrain/vegetation geometry, approach/transfer volume, occupancy state, compartment dimensions, door/latch logic, storage classes and local power.
6. **Custody/evidence:** delivery intent, carrier authority, station acceptance, transfer observations, receipt, storage condition and recovery status.

Each artifact must carry a version, units, coordinate frame/datum, time basis, provenance, validity interval, uncertainty and evidence class.

## 4. Handoff modes to compare

Evaluate at least four architectures as distinct configurations, not interchangeable modes of one generic drone.

| Mode | Description | Main questions |
|---|---|---|
| H1 — Tether/winch | Aircraft hovers outside the home's immediate receiving area; a payload descends to a transfer target | Tether swing, snagging, payload contact, wind sensitivity, load release and safe retraction |
| H2 — Controlled descent/drop | Aircraft descends to a defined low-altitude transfer point and releases a parcel | Ground hazard detection, descent corridor, rotor/downwash exposure, package rebound and site clearance |
| H3 — Dedicated station docking | Aircraft lands at an approved dock and transfers a parcel into an interlocked receiver | Landing accuracy, propeller/rotor exclusion, latch state, safe loading, charging and maintenance |
| H4 — Shared hub plus ground robot | Aircraft delivers to a community hub; a ground robot or courier performs the final handoff | Queueing, transfer between modes, accessibility, hub capacity and total journey energy |

H4 may outperform a home touchdown design on safety, throughput, community noise, and economics even when its aircraft-to-home time is longer. Do not assume the most technologically elaborate option is the best.

## 5. Scenario model and solver boundary

### 5.1 Scene contract

A reproducible scenario includes:

- site geometry and coordinate frame;
- aircraft class, aircraft configuration and model revision;
- route/approach volume and validity interval;
- wind field or declared wind disturbance envelope;
- terrain, building, roof, tree, wire and adjacent-property obstacles;
- dynamic occupancy states: person, animal, vehicle, loose object, open door, or blocked receiver;
- parcel mass, dimensions, centre of gravity and attachment geometry;
- transfer mechanism configuration and station capability;
- environmental/storage acceptance class;
- sensor errors, timestamp uncertainty, missingness and communication outage;
- mission energy, reserve, abort branches and alternative destination.

Use synthetic-first fixtures and preserve exact input bytes. Later field observations require permission, sensor calibration, privacy protection, and rights-aware provenance.

### 5.2 Physics decomposition

- Rigid parcel, door, latch, pad, and aircraft-body interactions may use an appropriate rigid-body model.
- A simplified tether pendulum can be used for early analytic test fixtures, with explicit assumptions and independent equations.
- Flexible tether deformation, line snagging, contact/friction, and dynamic reel behaviour require a suitable cable/multibody solver; do not silently approximate them with one rigid body.
- Rotor downwash, ground effect, crosswind and building-induced flow require CFD or a validated reduced-order aerodynamic model.
- Structural integrity of a pad, roof, cantilever, or receiving structure requires a qualified structural solver and physical/material inputs.
- Thermal storage and battery fire behaviour require separate validated thermal/electrical models.
- Sound exposure requires an acoustic model and field measurements.

The benchmark's job is to make these models exchange typed, versioned evidence and to test their coupling—not to claim one engine replaces all specialist solvers.

### 5.3 Uncertainty and model applicability

Keep two outputs separate:

1. **Deterministic bounded envelope:** an outer set for the declared bounded model and disturbance domain. A missing state, under-approximated set, stale datum, or out-of-domain input is a hard failure/abstention.
2. **Probabilistic prediction:** calibrated distribution or interval for observed conditions. Report subgroup coverage, calibration, sharpness, tail misses and drift. A 95% prediction interval is not a worst-case guarantee.

The model must expose unsupported regimes rather than extrapolating confidently from a nano-quadrotor dataset to a larger delivery vehicle.

## 6. Benchmark tracks and hard gates

Do not use one weighted score. A fast planner cannot compensate for a false-safe outcome, unsafe transfer, or corrupted custody record.

### Track A — Approach and site-envelope soundness

Metrics:
- outer-envelope under-approximation count;
- containment against independent analytic/interval fixtures;
- false-safe count and false-positive burden;
- occupancy/obstacle detection coverage in the declared test domain;
- envelope width/inflation and model-applicability coverage;
- stale map, datum, time, wind, or site-state rejection.

Proposed synthetic gate: zero under-approximation and zero false-safe results against the frozen independent oracle corpus. This qualifies only the stated synthetic mathematical domain, not a real aircraft.

### Track B — Payload and transfer dynamics

Metrics:
- parcel position/orientation error at the receiving boundary;
- peak tether tension, tether-angle and swing envelopes;
- package acceleration/impact and contact events;
- line/parcel snag or entanglement events;
- release confirmation and release-point uncertainty;
- transfer success conditional on predeclared parcel, site and weather classes.

Report touchdown, tether, controlled-drop and ground-robot modes separately.

### Track C — Receiving-station integrity

Metrics:
- package presence/mass-delta agreement across independent sensors;
- door/latch interlock correctness;
- false-accept and false-reject rates;
- jam, obstruction and power-loss safe-state correctness;
- time to secure a parcel after transfer;
- storage compartment and package compatibility.

Proposed gate: 100% fail-closed behavior for the frozen invalid/ambiguous sensor-state corpus. No claim about hardware reliability follows from software-only tests.

### Track D — Recoverability and safe abort

Measure recoverability as state × intervention × remaining time/energy budget, not as a scalar property of the current state.

Compare:
- continue only when the declared envelope remains valid;
- hold/loiter where the aircraft and operation support it;
- retract/reattach payload;
- abort transfer and return;
- divert to an alternate approved receiver;
- hand off to hub/ground delivery.

Metrics:
- lower confidence bound on recoverability at fixed checkpoints;
- missed recovery windows;
- energy and reserve at recovery;
- safe-abort and safe-diversion rate;
- recovery action latency and result;
- events where a model says recoverable but the independent branch outcome is not.

Reuse the existing Symtropy intervention-conditioned recoverability and failure-aware rollout evaluators; extend the scenario/target contract, do not create a duplicate evaluator without a measured gap.

### Track E — Operational and environmental quality

Metrics:
- accepted deliveries per station-hour;
- successful delivery, safe-abort, diversion and retry rates;
- p50/p95/p99 handoff duration;
- energy per delivered kg-km including return, station and charging loss;
- lifecycle emissions against a matched ground alternative;
- LAeq and sound exposure level at the property and nearest affected boundary;
- station occupancy and queue spillback;
- parcel damage, packaging waste and return completion.

### Track F — Robustness, privacy and cyber-physical faults

Scenarios:
- GNSS degradation, clock drift, stale geospatial data, sensor dropout, spoofed station occupancy;
- lost command/telemetry link, low battery, adverse wind, rain, dust and visibility limits;
- occupied/blocked transfer zone, unexpected person/pet/vehicle entry;
- tether snag, package swing, failed release, partial transfer;
- jammed compartment, open latch, interrupted power, false package-present reading;
- replayed authorization, operator-key compromise, fake aircraft identity, duplicate receipt, and delayed/out-of-order events.

Every scenario has a declared expected safe response and an independent oracle. A failed or unknown condition must not be relabelled as successful delivery.

## 7. Baselines and how to compare fairly

Industry reference architectures:

- **Wing:** lightweight dual-propulsion aircraft with vertical lift and separate cruise propulsion; public page lists up to 5 lb, about 60 mph and up to 12 miles. Public company page reports more than one million residential deliveries.
- **Zipline Platform 2:** home-delivery platform with a tethered delivery payload; public fact sheet lists up to 8 lb, 10-mile service radius, up to 24 miles one-way dock-to-dock, and delivery from up to 300 ft.
- **Amazon MK30:** public page lists up to 7.5 miles from its delivery centre and 60+ mph; its operations page describes delivery-site hazard detection and return when a hazard is detected.

These are manufacturer/operational claims from different dates and are not a matched benchmark. Public information is insufficient to reproduce fleet-level reliability, actual energy per delivered parcel, or complete noise distributions.

For a fair evaluation, compare four baselines in the same scenario and payload envelope:
1. conventional ground courier;
2. aerial delivery with a fixed predeclared route and current qualified delivery mechanism;
3. capacity-aware planner with conservative site acceptance;
4. proposed AeroWeft/Symtropy evidence-gated approach with explicit safe-abstention and recovery.

Use identical mission definitions, parcel mass/dimensions, weather, time window, reserve policy, and site. Report each result by scenario family and aircraft class. Do not claim superiority from mismatched marketing specifications.

## 8. Proposed research targets

Targets below are hypotheses to preregister before confirmatory tests, not achieved results:

- Zero false-safe outputs on frozen synthetic analytic/oracle cases.
- 100% rejection of unsupported model classes, invalid units/frames, stale site evidence, and exhausted compute budgets.
- 100% correct fail-closed outcomes across the frozen ambiguous-transfer and invalid-authorization corpus.
- Full custody event continuity across success, rejection, abort, diversion, quarantine and manual recovery fixtures.
- Calibrated probabilistic coverage reported by aircraft, wind, site, parcel and prediction horizon; no single pooled calibration score can hide subgroup failure.
- A Pareto comparison that shows service time, payload, energy, noise, infrastructure cost, safe-abort quality, and delivery success together.

No quantitative field-safety target is meaningful until the aircraft, station, operational envelope, safety case, and measurement system are fixed. Preregister numerical goals only after an initial reproducible baseline.

## 9. Data and split discipline

The existing Nano-Quadrotor benchmark is a model-identification dataset, not a home-delivery dataset. Retain its published split conventions and independent test flights if it is used for model research; do not repurpose overlapping windows as independent observations or mix it into the residential transfer score.

Create a new rights-cleared residential delivery corpus with:
- exact aircraft/station/site configuration IDs;
- parcel envelope and instrumented mass/centre-of-gravity state;
- transfer type, tether state where relevant, and time-synchronized observations;
- weather and local flow observations with calibration;
- dynamic occupancy/obstacle labels;
- receiver state and storage environment;
- all aborts, retries, non-deliveries, and missing measurements;
- raw-data digests, calibration certificates, dataset rights and split manifests.

Split by complete site, aircraft configuration, and mission family when evaluating generalization. Keep at least one site/configuration class untouched for confirmation. Do not tune on the final holdout.

## 10. Stack integration

| Layer | Work |
|---|---|
| AeroWeft | Provide versioned aircraft, mission, payload and reachable-envelope contracts with applicability limits |
| Sol-Atlas | Provide site geometry, rights, constraints, terrain and dated source provenance |
| Symtropy | Compose physical scenarios, event identity, record/replay, collision and transfer models; integrate specialist solver outputs |
| Symthaea | Propose handoff method, identify model disagreement, plan under constraints and explain abstention/recovery alternatives |
| Mycelix / NestPort | Represent resident consent, station capabilities, parcel manifests, signed custody receipts, maintenance and dispute evidence |

The station's local safety controller and the operator's flight-control/DAA systems remain independent of the cognitive planner and Mycelix governance plane.

## 11. Initial implementation sequence

1. **Static contracts:** define station, parcel, mission, handoff, observation, storage, recovery, and evidence records.
2. **Analytic fixtures:** point-mass parcel, simple pendulum tether, obstacle-box approach volume, and deterministic handoff state machine.
3. **Independent oracles:** exact envelope containment and state-transition validator separate from the simulator.
4. **Scenario harness:** site × aircraft × parcel × transfer × weather × failure matrix, deterministic seeds, immutable receipts.
5. **Model comparison:** fixed route, conservative rules, simple optimizer, and evidence-gated planner on identical held-out cases.
6. **Hardware-in-loop:** only once a specific station and aircraft test article exist; include sensor faults, power loss, clock drift and communication interruption.
7. **Supervised field test:** only with a qualified operator, written regulator determination, approved site, risk assessment and independent safety review.

## 12. Acceptance and claim ceiling

A benchmark pass applies only to its frozen scenario and declared model applicability. It does not prove:
- that an aircraft is airworthy;
- that an approach or delivery is legally permitted;
- that a property is a certified landing/receiving site;
- that a tether, rotor, station, or battery system is safe in all conditions;
- that an AI planner can operate an aircraft;
- that a simulated result transfers to physical operation.

The research contribution is a reproducible, independently checked bridge from aircraft/mission evidence to residential handoff and custody evidence, with safe rejection and recovery treated as first-class outcomes.
