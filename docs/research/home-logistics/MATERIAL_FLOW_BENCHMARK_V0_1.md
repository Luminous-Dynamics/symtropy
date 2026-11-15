# Material Flow Benchmark (MFB-1K)
## Reproducible evaluation for Mycelix Transit, NestPort and HomeOps

**Status:** Research benchmark specification v0.1; proposed, not yet implemented  
**Date:** 2026-11-15  
**Tracking:** Symtropy #5422  
**Related:** Symtropy #5393; Mycelix #6258, #6259, #6264 and PR #6265; Symthaea PR #12343; Sol-Atlas PR #879

## 1. Thesis and benchmark boundary

Most robotics benchmarks isolate a robot, task or planning problem. Real residential logistics is a chain of constrained transfers across organizations, vehicles, containers, robots, buildings and people.

The Material Flow Benchmark (MFB) evaluates whether a system can move a known object from an authorized source to an authorized destination while preserving identity, handling conditions, custody, privacy, safety and recoverability.

A delivery is not successful merely because a simulator places an object at the goal coordinate. It is successful only when:
1. the move was authorized for this principal, destination, service and time window;
2. the correct package and container were used;
3. every transfer respected asset and physical constraints;
4. the correct storage class and destination were reached;
5. the required custody and environmental evidence is complete;
6. there was no hard-safety or privacy violation; and
7. exceptions were resolved without an unaccounted physical mutation.

MFB is an evaluation protocol, not a new robot stack, control system, standards certification or substitute for an aviation/operator safety case.

**Working name and scale:** MFB-1K proposes 100 canonical task templates and at least 1,000 frozen, seeded episodes in its initial public release. These counts are proposed acceptance criteria; no benchmark or results are currently claimed to exist.

## 2. Research hypotheses

- Container-first interoperability reduces adapter complexity and makes capability mismatches detectable before motion.
- Proof-carrying missions reduce false delivery, custody ambiguity and unauthorized handling.
- Federated autonomy can preserve local navigation and safety while sharing mission intent, zones and status with fleet coordination.
- Planning against a coupled physical model reduces energy and makespan only when handoff, launch/recovery, reserve, charging, queueing and failure costs are included.
- Local household inventory and explicit, revocable consent can coexist with useful shared delivery services.
- Reverse logistics and reusable containers can lower system-wide resource use when returns, cleaning, loss, repair and empty backhaul are measured.

## 3. Unit of evaluation

An episode is a complete scenario with:
- scenario ID, version, track and task template;
- exact code, model, configuration and asset digests;
- random seed and deterministic replay settings;
- initial world state and time basis;
- actor, device, vehicle and site capability snapshots;
- package, tote and destination manifests;
- authorization and custody policy requirements;
- environmental and disturbance parameters;
- success predicates, forbidden states and required evidence;
- a result bundle containing event trace, outcome, metrics and exception classification.

Every physical quantity must carry a unit and coordinate frame. Every clock must declare its time basis and uncertainty. Every observation must identify its source, calibration/status, timestamp and freshness envelope. Missing or contradictory critical inputs are invalid, not zero-valued defaults.

A scenario result must be reproducible from frozen inputs, subject to a declared tolerance where hardware or floating-point nondeterminism prevents bitwise identity. The tolerance and comparison method must be fixed before evaluation.

## 4. Scenario tracks

### T1 — Container and interface interoperability

Evaluate tote/package compatibility across carriers, cargo bays, dock fixtures, manipulators, AMRs, conveyors, lifts and storage.

Required properties:
- stable opaque identifier and machine-readable version;
- external dimensions and tolerances;
- mass and centre-of-gravity bounds;
- orientation and acceleration/tilt limits;
- permitted grasp, restraint and support interfaces;
- handling, fragility, hygiene/contamination and temperature classes;
- seal state and evidence requirements;
- return, cleaning and inspection state.

The benchmark must include near-boundary dimensions, skewed totes, sensor uncertainty, incompatible interface versions and out-of-range mass/CoG. The first implementation may use a conservative axis-aligned fit check; it must label that limitation and must not claim general 3D packing.

### T2 — Residential receiving and put-away

Evaluate:
- courier or carrier acceptance at an external service vestibule;
- identity, mass/dimension and seal checks;
- quarantine of unknown, damaged or restricted packages;
- tote association and capacity checks;
- transfer into ambient, chilled, frozen, pantry, workshop, returns or recycling destinations;
- resident-accessible retrieval and manual bypass.

The initial operating boundary is sealed-package handling. Automatic opening, food preparation and individual-item unpacking are out of scope.

### T3 — Multimodal logistics and fleet allocation

Compare:
- truck/EV-only delivery;
- truck + parked-carrier aircraft;
- truck + ground robot;
- community hub consolidation;
- multi-leg missions using combinations of road, air, fixed transfer and ground robot.

A vehicle being fitted with a drone dock does not imply that the aircraft is payload-eligible or safe to dispatch. Each asset and mission must satisfy the capability envelope and operating domain.

### T4 — Household authority and privacy

Evaluate purpose-bound Hearth grants and local HomeOps enforcement:
- correct household, principal, delegate, service provider and endpoint;
- action scope, one-shot or bounded use and expiry;
- revocation and policy-version mismatch;
- delegation cannot increase scope or expiry;
- duplicate and replay attempts;
- provider attempts to access inventory, presence, care schedules or room maps;
- household versus cooperative, tenant, guest and caregiver authority.

The provider should receive only the minimal delivery intent and endpoint authorization needed for its service. Household-private inventory and presence are not benchmark telemetry exports.

### T5 — Safety, fault handling and recovery

Evaluate humans, children, guests, mobility aids, animals, doors, egress routes, occupied zones, interlocks, emergency stop and manual recovery.

Inject:
- sensor staleness or disagreement;
- blocked transfer gate or corridor;
- unexpected occupancy;
- power interruption;
- network partition;
- damaged seal or unknown package;
- thermal excursion;
- dock/latch mismatch;
- mechanical jam or lift overload;
- missing or conflicting custody acknowledgement.

Safety is a hard gate, not a weighted objective that can be offset by faster delivery.

### T6 — Circular and resilient operations

Evaluate returnable containers, packaging return, empty backhaul, cleaning, inspection, repair, reuse cycles, charging availability, alternative hubs, asset unavailability and emergency supply continuity.

Report loss, damage, cleaning and reverse-trip cost rather than treating reuse as inherently beneficial.

### T7 — Evidence integrity and provenance

Evaluate:
- two-sided custody handoff and receiving confirmation;
- event ordering, replay, duplicates and clock uncertainty;
- device identity and configuration digest;
- sensor freshness and calibration state;
- sourced/versioned energy and emissions factors;
- immutable or tamper-evident result manifests;
- resolution of unknown outcomes without duplicate physical mutation.

GS1 EPCIS 2.0.1 is a candidate external event vocabulary for interoperable business visibility. The MFB event profile must map only the necessary WHAT/WHEN/WHERE/WHY/HOW facts and must not copy private household context into shared events.

## 5. Reference architecture under test

| Plane | Responsible system | Benchmark contract |
|---|---|---|
| Household consent | Mycelix Hearth | Explicit, scoped, expiring/revocable household capability grant |
| Shared service coordination | Mycelix Commons / Transit | Assets, offers, bookings, service providers, hubs and multimodal mission records |
| Local residence | NestPort / HomeOps | Detailed inventory, local policy, receiving, transfer, storage and manual recovery |
| Robot mission record | Mycelix Civic Robotics Dispatch | Device/asset identity, capability snapshot, mission authorization envelope, summary telemetry and recall |
| Planning and cognition | Symthaea | Proposals, task decomposition, perception, uncertainty and alternative plans |
| Physical twin | Symtropy | Scenario state, contact/material interactions where modeled, deterministic replay and fault injection |
| Geographic evidence | Sol-Atlas | Site/access/terrain/climate/hazard and infrastructure context with provenance and uncertainty |
| Aircraft capability | AeroWeft | Frozen aircraft, payload, propulsion, dock, charging and flight-energy envelopes |
| Custody/event vocabulary | SupplyChain adapter + EPCIS mapping | Authorized, minimal chain-of-custody event projection after source security qualification |
| Operational signals | Mycelix Pulse | Bounded status summaries only; not safety control or raw home telemetry |

No distributed ledger or remote planner may be required for collision avoidance, emergency stop, guarding, flight stabilization, thermal control or other local protective functions.

## 6. Baselines

Each track must use matched baselines. At minimum:

1. **B0 — Validated deterministic policy:** simple transparent rules, nearest-compatible destination, fixed route and explicit exception handling.
2. **B1 — Classical optimization:** a best-known feasible routing/assignment method (for example MILP, CP-SAT or a documented heuristic) with identical inputs and constraints. Exact optimality is only claimed for instances where it is proved.
3. **B2 — Strong learned/planning baseline:** the strongest reproducible public or independently rerun policy suitable for the same task definition.
4. **B3 — Ablation:** remove one proposed capability at a time (container standardization, evidence gate, local autonomy, Symthaea planner, shared fleet allocation, etc.) to establish what causes an improvement.

For truck/air comparisons, B0 must include truck-only delivery. Use the same demand, delivery windows, route data, payload eligibility, service time, energy accounting, reserve policy and operational assumptions. Include aircraft energy, launch/recovery, transfer dwell, queueing, repositioning, return-to-carrier, charging losses and expected failed/aborted missions. Compare system totals, not just the truck's energy.

For mobile-manipulation comparisons, use RoboCasa365 and BEHAVIOR protocols where task overlap exists. Report official benchmark scores separately; do not relabel a kitchen/manipulation score as parcel-delivery success.

## 7. External benchmark and interoperability alignment

| Reference | Use | Important limitation |
|---|---|---|
| RoboCasa365 | Standardized multi-task kitchen manipulation and compositional generalization | Not a logistics, custody, aviation or full-home delivery benchmark |
| BEHAVIOR-1K / 2026 BEHAVIOR Challenge | Long-horizon household mobile manipulation and activity completion | Simulation outcomes do not establish real-world reliability |
| HomeRobot OVMM | Real-world open-vocabulary mobile manipulation reference | Its task definition and original real-world baseline must be matched before comparing |
| VDA 5050 v3.0 | Task/status and mixed-fleet mobile-robot interface, including zone/path-sharing concepts | Interoperability interface, not safety certification or aircraft protocol |
| GS1 EPCIS 2.0.1 | Business event visibility and cross-organization custody semantics | Does not itself provide household privacy or authorization |
| ISO 3691-4 | Relevant safety requirements for driverless industrial trucks and systems | Applicability and current revision status must be checked for the actual platform |
| ISO 13482 | Relevant service-robot human-contact hazards | Edition 2 is in publication as of this specification; check its published status and precise scope before relying on it |

External protocol versions and baselines must be frozen in each benchmark release. Do not claim conformance from simply using a message schema.

## 8. Metrics and score reporting

Publish each metric by track, scenario family, environment split, severity and evidence level. Do not collapse safety and service into a single compensatory score.

### 8.1 Safety and authority gates

- accepted critical safety violations: **0** in the qualified test suite;
- unauthorized physical mutations: **0**;
- false-delivery assertions: **0**;
- forbidden privacy disclosures: **0** in the test suite;
- safety-stop, interlock and manual-recovery behaviour: demonstrated independently of remote services.

Zero observed events is not proof of zero risk. Report exposure counts, hazard analysis, fault-injection coverage, confidence bounds and the independent review scope.

### 8.2 Service and integrity metrics

- valid authorized end-to-end completion;
- correct package/container/destination;
- correct storage-class routing;
- required custody-field completeness and bilateral acknowledgement;
- unresolved-custody and false-delivery rate;
- cold-chain excursion probability, magnitude and duration;
- package damage and seal exceptions;
- held-out task and layout generalization;
- resident retrieval/accessibility success.

### 8.3 Efficiency and resilience metrics

- makespan, on-time rate, throughput and queue/dwell time;
- ETA calibration and empirical coverage;
- energy per successful delivery and Wh per kg-km;
- total energy including truck, aircraft, robot, lift/conveyor, charging and failed attempts;
- battery ageing assumptions and charging losses;
- empty-return distance and reverse-logistics cost;
- tote reuse cycles, loss, damage, cleaning and repair;
- human interventions per 100 deliveries, intervention time and recovery time;
- degraded-mode performance under service, power and network loss.

## 9. Initial targets — research goals, not achieved results

| Metric | Initial target | Qualification note |
|---|---:|---|
| Routine receiving-to-storage completion | ≥90% | Bounded task family and declared ODD |
| Held-out residential task completion | ≥85% | Hold out layouts/task compositions, not only random seeds |
| Correct storage-class routing | ≥99.5% | HIL with calibrated sensor/input assumptions |
| Required custody evidence | 100% for accepted deliveries | Missing evidence means held/unresolved, not delivered |
| Accepted critical safety violations | 0 | Hard gate; do not average away failures |
| Human intervention after stabilization | <5 per 100 deliveries | Report by intervention type and task difficulty |
| Improvement over strongest matched baseline | ≥5 percentage points | Pre-register test, report confidence intervals and per-family deltas |
| Truck + air system makespan | ≥15% better initially | Matched truck-only comparison |
| Truck + air total operational energy | ≥10% better initially | Include aircraft and all supporting energy |

**Stretch research goals:** ≥25% makespan improvement and ≥18% total operational-energy reduction against truck-only in an eligible demand/route regime. These are deliberately ambitious, not promised. The 2026 published truck–drone study reported 24.5% makespan and 16.2% total energy improvement in its selected scenario, but its model abstracts away detailed launch/recovery sequencing, queueing and some operational effects; reproduce and extend its scenario before claiming superiority.

A score is not valid if it omits failed tasks, excludes interventions, ignores energy consumed by failed missions or changes the operational domain between systems.

## 10. Evaluation protocol and statistical discipline

- Freeze scenarios, seeds, baselines, metrics, exclusions and analysis before running the comparison.
- Split by task template, environment/layout, asset configuration and disturbance family. A new random seed is not a meaningful held-out home by itself.
- Run at least 10 seeds per canonical task in the initial 1,000-episode proposal; scale up for high-variance tasks and safety exposure.
- Report Wilson or exact binomial intervals for binary success, bootstrap intervals at the scenario-family level for aggregated metrics, and paired comparisons when identical scenarios can be run by both systems.
- Disclose missing episodes, simulator crashes, timeouts, human interventions and invalid inputs. Do not silently discard them.
- Publish failures and negative controls alongside successful traces.
- Keep a blind evaluation split and a public baseline runner.
- Bind each result to source commit, model checkpoint, dependency lock, hardware/config, solver version, input hashes and random seeds.
- A zero-failure run is evidence limited by sample count and fault coverage; it is not a probability-zero statement.

## 11. Qualification ladder

| Level | Evidence | Permitted claim |
|---|---|---|
| L0 — Contract | Unit, state-machine, property and negative-control tests | Contract invariants tested |
| L1 — Simulation | Deterministic twin, fault injection, seeded and held-out scenarios | Simulated scenario performance |
| L2 — HIL | Production-intent control interfaces, sensors, firmware and interlocks | Bench/HIL results for the named setup |
| L3 — Guarded rig | Measured package/tote transfer, dock, lift, conveyor and AMR | Controlled physical test results |
| L4 — Restricted field pilot | Defined ODD, operator procedures, maintenance, incident reporting and independent review | Pilot results within the stated ODD |

No level automatically implies the next. A digital twin is not a certified aircraft model, building structural model, refrigeration validation, safety case or regulatory approval.

## 12. First executable slice

Build one reference scenario before expanding the fleet:

- one stationary, immobilized carrier;
- one package with a frozen manifest;
- one reusable tote;
- one receiver;
- one transfer gate;
- one storage compartment;
- one local HomeOps controller contract;
- one authorized household grant;
- one releasing custodian and one receiving custodian.

Run a deterministic baseline and the proposed planner over a nominal episode and the following ten explicit fault cases:

1. wrong tote;
2. over-mass or out-of-envelope centre of gravity;
3. stale temperature observation;
4. blocked/interlocked transfer gate;
5. damaged or unknown seal;
6. duplicate handoff event;
7. missing receiver acknowledgement;
8. expired or wrong-endpoint household grant;
9. power interruption during transfer;
10. network loss after physical action but before receipt.

Acceptance requires a correct nominal storage result and safe, explainable hold/recovery for every fault. A timeout after an action must not trigger a blind duplicate physical transfer.

## 13. Proposed machine-readable scenario shape

The canonical representation should be versioned JSON or CBOR with a generated Rust type and strict validation. The following is illustrative and does not yet exist as a schema:

\`\`\`yaml
schema_version: 1
scenario_id: mfb.homeops.stationary-carrier.nominal.v1
seed: 42017
evidence_level: L1
operational_domain:
  site_profile_digest: "sha256:..."
  weather_profile_digest: "sha256:..."
assets:
  - asset_id: carrier-01
    capability_digest: "sha256:..."
  - asset_id: receiver-01
    capability_digest: "sha256:..."
cargo:
  package_manifest_digest: "sha256:..."
  container_manifest_digest: "sha256:..."
authorization:
  grant_digest: "sha256:..."
  required_actions: [receive_external_package, place_ambient]
flow:
  source: carrier-01
  destination: receiver-01/ambient-01
  required_events: [release, receive, placement, custody_ack]
fault_injections: []
success_predicates:
  - package_identity_matches
  - authorized_destination
  - required_custody_complete
  - no_critical_safety_violation
\`\`\`

The actual schema must use typed, validated identifiers, explicit units, a real digest encoding, exact enumerations and unambiguous event time. The illustrative ellipses are placeholders, not valid production digests.

## 14. Immediate execution order

1. Freeze the first schema, event vocabulary and reference-scene manifest.
2. Implement the one-package/one-tote/one-receiver scenario in Symtropy.
3. Make all ten fault cases executable with deterministic replay.
4. Publish the deterministic baseline and result manifest.
5. Add the Symthaea HomeOps planner as a separately versioned proposal source.
6. Add VDA 5050 v3 adapters for an AMR/fleet-control track.
7. Map custody events to a minimal EPCIS profile; keep household-private facts local.
8. Add HIL/guarded-rig connectors only after the simulation contract is stable.

## 15. References

- [RoboCasa365 leaderboard](https://robocasa.ai/leaderboard) and [evaluation methodology](https://robocasa.ai/leaderboard.html).
- [BEHAVIOR-1K](https://behavior.stanford.edu/) and [2026 BEHAVIOR Challenge](https://behavior.stanford.edu/challenge/index.html).
- [HomeRobot OVMM](https://ovmm.github.io/).
- [VDA 5050 v3.0 release](https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN).
- [GS1 EPCIS standard](https://ref.gs1.org/standards/epcis/).
- [ISO 3691-4](https://www.iso.org/standard/83545.html) and [ISO 13482](https://www.iso.org/standard/83498.html).
- [2026 truck–drone collaborative delivery study](https://www.mdpi.com/2504-446X/10/7/476).

## 16. Completion criteria

MFB v0.1 is ready for implementation only when:
- the scenario schema and 100 task templates are versioned;
- 1,000 seeded episodes and split manifests are reproducible;
- a deterministic baseline is executable;
- nominal plus ten failure cases replay deterministically;
- service, authority, safety, evidence and efficiency metrics are individually reported;
- the result bundle includes source/model/config/input digests and uncertainty;
- benchmark claims stay within their evidence level and operational domain.
