# Material Flow Benchmark (MFB-1K)
## Reproducible evaluation for Mycelix Transit, NestPort and HomeOps

**Status:** Research benchmark specification v0.1; proposed, not yet implemented  
**Date:** 2026-11-15  
**Tracking:** Symtropy #5422  
**Related:** Symtropy #5393, #5422 and #5447; Mycelix #6258, #6259, #6264, #6267, #6269 and PRs #6265/#6268; Symthaea PR #12343 and CEB evidence-continuity protocols; Sol-Atlas PR #879

## 1. Thesis and benchmark boundary

Most robotics benchmarks isolate a robot, task or planning problem. Real residential logistics is a chain of constrained transfers across organizations, vehicles, containers, robots, buildings and people. It is also a distributed physical transaction: a physical action may occur even when its acknowledgement is lost, so software cannot safely assume that retrying an operation is equivalent to retrying a database write.

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
- Append-only, independently authored mission and custody events can preserve accountability through concurrency and network partitions without pretending that physical actuation is exactly-once or rollbackable.

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
- asset-specific payload mass and dimensional envelopes, including the loaded container rather than package-only checks;
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

Integrate the existing Commons Waste/Circular domains where the exact hApp deployment and call graph are qualified:
- `waste_registry`: material-stream identity and classification;
- `waste_collection`: collection request and collection-run records;
- `compost_control`: compost-batch and bounded sensor/process observations;
- `circular_marketplace`: secondary-material listings and demand matching.

The division of responsibility is deliberate: Waste/Circular owns material classification, processing and recovery records; Transit owns shared vehicles, route segments, booking, collection/pickup missions and transport custody. A waste-collection run must not create a second fleet-routing or vehicle registry. Compost zomes record bounded process evidence; local compost equipment controllers retain process and safety authority.

**Deployment caveat:** all four Waste/Circular zome pairs are declared in the current Commons DNA manifest, but the shared `COMMONS_LOCAL_ZOMES` allowlist does not currently list them. Mycelix #6269 tracks reconciliation of manifests, Cargo members, SDK, router and actual tests. Do not assume generic bridge dispatch is available until that work is completed. The same issue records that Tool Library and Community Calendar appear in the Cargo workspace but not in the current DNA manifest.

Report recovered mass, validated reuse/repair/recycling destination, contamination/rejection rate, secondary-material quality and yield, tote return rate, cleaning/inspection cost, losses, empty backhaul and reverse-trip energy. Any avoided-virgin-material or carbon claim must bind a versioned life-cycle method, source factors, system boundary and uncertainty; unverified self-reports must not mint credits or count as verified impact. Do not treat reuse as inherently beneficial without counting cleaning, transport, damage and end-of-life costs.

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

### T8 — Distributed physical-transaction correctness

Exercise the coordination protocol across independent household, provider, carrier, robot and receiving-device agents. The benchmark must distinguish:
- **logical intent:** a request or reservation that has not yet authorized motion;
- **physical action:** a non-transactional release, movement, grasp, transfer or placement;
- **observation:** sensor/device evidence that the physical action occurred;
- **custody acknowledgement:** independently authored evidence from the receiving party;
- **reconciliation:** an explicit resolution of contradictory, missing or late evidence.

Do not promise exactly-once physical execution. Require idempotent logical command identities, bounded/consume-once authorization where the underlying authority model can support it, independent receiver acknowledgement, and a reconciliation state for unknown outcomes. A timeout after actuation must not trigger a blind second transfer.

Inject concurrent reservations, duplicate and replayed commands, reordered events, conflicting party responses, clock skew/uncertainty, stale or revoked grants, network partitions before and after physical action, conductor restarts and one-sided acknowledgement. Evaluate both safety and convergence after the partition heals.

A system may converge to a safe **held/unresolved** state. It must never converge to false delivery, unauthorized movement or over-capacity acceptance. If global no-overbooking cannot be proven for the chosen Holochain/DHT authority design, the implementation must narrow the guarantee to a single reservation authority or fail closed; benchmark documentation must disclose the scope.

Report:
- over-capacity reservation attempts accepted;
- duplicate physical actions accepted;
- false-delivery and false-custody assertions;
- unauthorized mutations under expiry/revocation races;
- replay and duplicate-event detection;
- divergent state duration and reconciliation latency;
- unresolved custody age and successful safe-hold recovery;
- event convergence under the declared network and clock model.

These are hard correctness gates, not weighted metrics.

### T9 — Evidence continuity and selective revalidation

Measure whether the system knows which qualification claims remain usable as source code, software/firmware, configuration, environment, policy, sensors and evaluators change.

Each consequential claim should carry a typed support closure to the evidence it depends on. Candidate dependencies include:
- exact source commit/tree, build and dependency lock;
- robot/aircraft/controller firmware and configuration digests;
- sensor identity, calibration, freshness and metrology procedure;
- facility, site, route and environmental data revisions;
- package/container/asset capability versions;
- grant, signer, trust-root and authorization-policy versions;
- task, workload, oracle, metric and grader versions;
- simulation model, solver and HIL fixture versions.

For every change event, the system must classify each affected claim as **retain**, **revalidate**, **invalidate**, **supersede** or **unresolved**, with a reason and the minimum sufficient next action. Do not assume that later timestamps invalidate all evidence; do not assume that absence of a discovered dependency proves non-impact.

Separate three outcomes:
- **rerun:** the product, workload, configuration, environment or execution context changed and fresh execution evidence is needed;
- **regrade:** the product artifact is unchanged but the oracle/grader or evaluation rule changed;
- **reuse:** the change is metadata-only or is proven irrelevant within the claim's declared scope.

The system should revalidate only affected claims while keeping unaffected evidence usable. However, when dependency closure is incomplete or the change's impact is unknown, it must choose revalidate/unresolved rather than falsely retaining a consequential safety, authority or performance claim.

This track adapts the public Symthaea CEB evidence-continuity and claim-support-closure protocols. It is a benchmark integration target, not a claim that a complete CEB engine is already connected to MFB.

Inject source/API changes, toolchain/dependency changes, firmware/configuration changes, sensor recalibration, new route/site/climate data, grant expiry/revocation, trust-root rotation, safety-policy changes, benchmark task changes and grader corrections.

Report:
- false-retain rate (stale evidence used for a consequential decision);
- false-invalidation rate (valid evidence discarded without reason);
- correct rerun/regrade/reuse classification;
- claim-support closure completeness and open-world remainder;
- affected-claim precision/recall;
- unnecessary full-suite reruns avoided;
- time/cost to restore current decision-use validity;
- age of unresolved evidence and blocked actions.

Hard gate: zero false-retain decisions for safety, authority, custody or other explicitly consequential claims in the frozen challenge suite.

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
| Circular-material coordination | Mycelix Commons Waste/Circular | Material stream and collection intent, compost/process records and secondary-material matching; Transit still owns vehicle assignment and physical collection missions |
| Resource availability | Mycelix Resource-Mesh | Candidate shared-resource matching after schema, deployment and authorization review |
| Shared equipment lending | Mycelix Tool Library | Candidate equipment lending/return interface; currently in Cargo workspace but absent from Commons DNA manifest, pending #6269 |
| Custody/event vocabulary | SupplyChain adapter + EPCIS mapping | Authorized, minimal chain-of-custody event projection after source security qualification |
| Operational signals | Mycelix Pulse | Bounded status summaries only; not safety control or raw home telemetry |
| Evidence continuity | Symthaea CEB protocols / repository-twin and decision-use validity work | Claim-support closure, dependency-aware retain/revalidate/invalidate/supersede/unresolved, and minimum-sufficient requalification; integration remains a research target until executable adapter evidence exists |

No distributed ledger or remote planner may be required for collision avoidance, emergency stop, guarding, flight stabilization, thermal control or other local protective functions.

## 6. Baselines

Each track must use matched baselines. At minimum:

1. **B0 — Validated deterministic policy:** simple transparent rules, nearest-compatible destination, fixed route and explicit exception handling.
2. **B1 — Classical optimization:** a best-known feasible routing/assignment method (for example MILP, CP-SAT or a documented heuristic) with identical inputs and constraints. Exact optimality is only claimed for instances where it is proved.
3. **B2 — Strong learned/planning baseline:** the strongest reproducible public or independently rerun policy suitable for the same task definition.
4. **B3 — Ablation:** remove one proposed capability at a time (container standardization, evidence gate, local autonomy, Symthaea planner, shared fleet allocation, etc.) to establish what causes an improvement.
5. **B4 — Frozen generalist policy:** use the strongest public/reproducible VLA or world-action model under the same observations, action space, tool budget, evaluation time and compute budget.
6. **B5 — Procedure-composition baseline:** a HiPhys-RSI-style agent that creates/reuses executable pipelines and routes between procedures and VLA skills, but without MFB's typed evidence admission, independent authority boundary or anti-self-approval checks.
7. **B6 — Evidence-gated procedure system:** B5 plus version-bound procedure graphs, per-step evidence dependencies, independent evaluator, fail-closed unknown handling, explicit custody/reconciliation and selective procedure revalidation.

The comparison must include at least these ablations: no procedure reuse; reuse without independent evidence evaluation; typed graph without the authority/executor separation; single-sensor receipt accepted without corroboration; no procedure invalidation after firmware/calibration/site change. This isolates whether MFB's assurance architecture reduces unsafe success and false delivery rather than merely increasing task completion.

For truck/air comparisons, B0 must include truck-only delivery. Use the same demand, delivery windows, route data, payload eligibility, service time, energy accounting, reserve policy and operational assumptions. Include aircraft energy, launch/recovery, transfer dwell, queueing, repositioning, return-to-carrier, charging losses and expected failed/aborted missions. Compare system totals, not just the truck's energy.

For mobile-manipulation comparisons, use RoboCasa365 and BEHAVIOR protocols where task overlap exists. Report official benchmark scores separately; do not relabel a kitchen/manipulation score as parcel-delivery success.

## 7. External benchmark and interoperability alignment

| Reference | Use | Important limitation |
|---|---|---|
| RoboCasa365 | Standardized multi-task kitchen manipulation and compositional generalization | Not a logistics, custody, aviation or full-home delivery benchmark |
| BEHAVIOR-1K / 2026 BEHAVIOR Challenge | Long-horizon household mobile manipulation and activity completion | Simulation outcomes do not establish real-world reliability; use its official observation restrictions and hidden-instance protocol |
| HomeRobot OVMM | Real-world open-vocabulary mobile manipulation reference | Its task definition and original real-world baseline must be matched before comparing |
| VDA 5050 v3.0 | Task/status and mixed-fleet mobile-robot interface, including zone/path-sharing concepts | Interoperability interface, not safety certification or aircraft protocol |
| GS1 EPCIS 2.0.1 | Business event visibility and cross-organization custody semantics | Does not itself provide household privacy or authorization |
| ISO 3691-4:2023 | Published industrial-truck / AMR safety requirements edition | Edition 3, ISO/DIS 3691-4, is under development at enquiry voting; it is not yet the published replacement |
| ISO 13482:2014 / Edition 2 | Service-robot human-contact hazards | Edition 2 reached publication stage 60.00 on 2026-11-11; until its publication is complete, verify which edition is contractually applicable |

External protocol versions and baselines must be frozen in each benchmark release. Do not claim conformance from simply using a message schema.

### 7.1 Snapshot of published benchmark status (2026-11-16)

- **RoboCasa365 (live leaderboard checked 2026-11-16):** the current multi-task board covers 50 target tasks across Atomic-Seen (18), Composite-Seen (16) and Composite-Unseen (16), with 24 models listed. HexaShell reports **85.6 overall**, 91.1 Atomic-Seen, 85.0 Composite-Seen and 80.0 Composite-Unseen. Crucial split nuance: policies are evaluated in the pretraining kitchens; Composite-Unseen holds out task compositions, not entirely unseen homes/layouts. RoboCasa365 itself spans 365 tasks and 2,500 kitchen environments, but this leaderboard's first public multi-task comparison is the 50-task subset. Use the exact 50-task evaluator to compare a manipulation policy; do not present 85.6 as a household-delivery or custody score. See the [live leaderboard](https://robocasa.ai/leaderboard) and [evaluation protocol](https://robocasa.ai/leaderboard.html).
- **BEHAVIOR 2026:** 100 full-length household tasks across seven scenes. The evaluation track restricts policy inputs to onboard RGB, depth and proprioception; simulator-only privileged state such as ground-truth object pose/segmentation is forbidden at evaluation. Its primary Q averages the fraction of goal predicates satisfied at episode end; it is **not** full-task success. HiPhys-RSI / The North Star reports **Q = 0.683 and 48.2% full-task success** on hidden test instances; second place reports Q = 0.615. Its approach uses a frozen VLM coding agent to diagnose rollout failures, compose and refine executable pipelines, retain reusable procedures and route tasks between those procedures and VLA policies. The agent is not trained during the improvement loop: scaling is stored in executable structure and tools. This is strong evidence for testing reusable procedural knowledge, but does not prove safe handling, custody, authorization or household privacy. See the [official evaluation rules](https://behavior.stanford.edu/challenge/evaluation.html), [2026 challenge description](https://behavior.stanford.edu/challenge/index.html), [champion's results/method](https://behavior1k-champion.com/) and [technical report](https://behavior1k-champion.com/assets/HiPhys-RSI_tech_report.pdf).
- **SafeVLA-Bench (updated 2026-09-26):** 24 policies, five suites, 22,500 episodes and eight safety specifications. On RoboCasa-365 Atomic-Seen, the safety-ranked leader reports **53.1% safety**, 58.4% native success, **24.0% success-but-unsafe (SBU)** and 0.222 worst-violation severity. The success-ranked leader reports 81.9% success but 50.1% safety and 37.7% SBU. Safety is defined as the share of episodes satisfying every applicable spec; SBU counts episodes that finish the task while violating at least one. These are the failures an SR-only scoreboard hides. MFB adopts safety share, SBU and severity as separate dimensions; these scores are context, not numerically comparable to MFB's own task set. See the [live SafeVLA leaderboard](https://safevla.org/leaderboard.html).
- **VDA 5050 3.0.0:** the current version adds support for freely navigating mobile robots, zones, path sharing, refined operating modes and action states. MFB should consume the published standard through a version-pinned adapter, not invent a competing fleet protocol. The standard is an interoperability layer, not the safety case. See [VDA's version 3.0 release](https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN) and [release repository](https://github.com/VDA5050/VDA5050/releases).
- **ISO 21423:2026 (published 2026-10-14):** newly published industrial mobile-robot communications/interoperability standard covering communication among industrial mobile robots, fleet managers and shared industrial-environment systems such as lifts and doors. It explicitly excludes consumer applications and public roads, and is **not safety-related**; navigation is out of scope. MFB's residential HomeOps track is therefore not an ISO 21423 conformity claim. Use it as an industrial adapter/crosswalk reference where the operating context is actually in scope, and keep safety evidence separate. See the [official ISO scope and publication record](https://www.iso.org/standard/86749.html).
- **ISO 3691-4:** ISO 3691-4:2023 Edition 2 remains the published safety-requirements/verification edition for driverless industrial trucks; Edition 3 is still a draft under development. Keep the current published edition and draft status distinct in any site-specific applicability record. See [ISO 3691-4:2023](https://www.iso.org/standard/83545.html) and [ISO/DIS 3691-4](https://www.iso.org/standard/88615.html).
- **GS1 EPCIS:** version 2.0.1, published 2025-07-01, remains the version recorded in the official GS1 archive snapshot used by this document.
- **ISO 13482:** Edition 2 has reached publication-stage processing in November 2026 and is replacing the 2014 edition. Check the live catalogue and acquire the legitimate standard text before freezing any clause-level applicability matrix.

**Interoperability and claims crosswalk:** MFB composes with Symtropy [#3703/#3704](https://github.com/Luminous-Dynamics/symtropy/issues/3703) for VDA 5050 semantic projection and [#5016](https://github.com/Luminous-Dynamics/symtropy/issues/5016) for standards-to-evidence applicability. Do not duplicate either adapter or infer safety, navigation, physical acceptance or legal authorization from fleet-message conformance.

### 7.2 South African UAS operating gate

For a South African deployment, the aircraft track must carry a regulatory applicability decision as an explicit evidence artifact.

SACAA's published UAS guidance states that private operations are restricted to R-VLOS and Class 1A/1B aircraft and says private operators must not release, dispense, drop, deliver or deploy objects. Its approval matrix lists UASOC, RPAS Letter of Approval, RPL and registration for corporate/non-profit operations, and an Air Service Licence additionally for commercial operations. SACAA also lists autonomous unmanned aircraft as excluded from Part 101's stated scope. That exclusion is **not** evidence that autonomous delivery is unregulated or approved; the responsible authority must confirm the applicable legal pathway before any autonomous or delivery operation.

MFB must therefore distinguish remote-piloted Part 101 use, private use, commercial operation and autonomous aircraft as separate regulatory cases. Do not infer flight permission from property ownership, a Sol-Atlas site score, a Mycelix mission grant or a Symtropy simulation.

## 8. Metrics and score reporting

Publish each metric by track, scenario family, environment split, severity and evidence level. Do not collapse safety and service into a single compensatory score.

### 8.1 Safety and authority gates

Report the SafeVLA-style **safety-compliance share**, **success-but-unsafe (SBU)** and **violation severity** alongside ordinary task success. An episode that completes the task while violating a safety specification is a failure, not a partial success.

- accepted critical safety violations: **0** in the qualified test suite;
- unauthorized physical mutations: **0**;
- false-delivery assertions: **0**;
- forbidden privacy disclosures: **0** in the test suite;
- false-retain of stale safety/authority/custody evidence: **0** in the frozen change-challenge suite;
- safety-stop, interlock and manual-recovery behaviour: demonstrated independently of remote services.

Zero observed events is not proof of zero risk. Report exposure counts, hazard analysis, fault-injection coverage, confidence bounds and the independent review scope.

### 8.2 Service and integrity metrics

- valid authorized end-to-end completion;
- correct package/container/destination;
- correct storage-class routing;
- required custody-field completeness and bilateral acknowledgement;
- unresolved-custody and false-delivery rate;
- accepted over-capacity reservations, duplicate physical actions, unauthorized retries and unresolved reconciliation age;
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
- degraded-mode performance under service, power and network loss;
- event convergence and time to safe reconciliation after distributed partitions;
- false-retain and false-invalidation rates for evidence after change events;
- rerun/regrade/reuse classification accuracy, closure completeness and requalification cost.

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
| Accepted over-capacity reservations / false deliveries / unauthorized retries | 0 | Hard correctness gates under the declared concurrency and fault model |
| Truck + air system makespan | ≥15% better initially | Matched truck-only comparison |
| Truck + air total operational energy | ≥10% better initially | Include aircraft and all supporting energy |

**Stretch research goals:** ≥25% makespan improvement and ≥18% total operational-energy reduction against truck-only in an eligible demand/route regime. These are deliberately ambitious, not promised. The 2026 published truck–drone study reported 24.5% makespan and 16.2% total energy improvement in one selected scenario (three truck routes, a shared ten-drone fleet and 456 deliveries in the evaluated dataset subset). Its own methods section says the reported experiments did not enforce mission-level battery capacity directly: feasibility relied mainly on a maximum service radius and preprocessed feasible assignments. It also aggregates drone workload rather than modeling explicit sortie sequencing, mission-level synchronization, waiting, stochastic weather, battery degradation and real-time dispatch. MFB should reproduce that published formulation first, then compare a higher-fidelity model with explicit sorties, launch/recovery, charging, reserves, failed/aborted missions, weather uncertainty and queueing. Only then can a ≥25% / ≥18% result credibly claim superiority.

A score is not valid if it omits failed tasks, excludes interventions, ignores energy consumed by failed missions or changes the operational domain between systems.

### 9.1 External leaderboard research targets — matched protocols only

These are ambitious **research targets**, not achieved MFB results. They apply only when Symthaea/MFB is evaluated under the exact source benchmark's task set, observation restrictions, action space, episode budget, split and scoring implementation.

| Benchmark | Current reference (checked 2026-11-16) | MFB/Symthaea stretch target | Required fairness condition |
|---|---:|---:|---|
| RoboCasa365 50-task multi-task board | HexaShell 85.6 overall; 80.0 Composite-Unseen | ≥87.0 overall and ≥82.0 Composite-Unseen | Same 50 tasks, 18/16/16 splits, pretraining kitchens, episode count and official scoring |
| BEHAVIOR 2026 | HiPhys-RSI Q=0.683; 48.2% full-task success | Q≥0.70 and full-task success ≥55% | Official 100-task track, onboard RGB+depth+proprio only, hidden instances, identical rollout budgets; report Q and full-task success separately |
| SafeVLA-Bench / RoboCasa Atomic-Seen | 53.1% safety; 58.4% SR; 24.0% SBU; VSI 0.222 for safety-ranked leader | Safety≥60%, SR≥60%, SBU≤15%, VSI≤0.15 | Same per-step trajectories, STL specifications, benchmark release and re-scoring script; report all four metrics |

RoboCasa and BEHAVIOR scores are not directly comparable to MFB end-to-end delivery metrics. MFB's unique contribution is to add authorization, provenance/custody, physical fault response, privacy and recovery without weakening manipulation performance. The proof of improvement must therefore have two parts: (1) matched manipulation benchmark performance and (2) MFB-specific end-to-end logistics assurance. A gain on one may not compensate for regression on the other.

For MFB's own 1K proposal, split by property/layout, task composition, package family, receiver/robot platform, sensor/firmware/calibration and disturbance family. Random-seed-only holdouts are insufficient to claim generalization to unseen homes.

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

## 12.1 Executable MFB toolkit — contract, physics and protocol campaign

The benchmark branch now includes a standalone Rust tool at `tools/material-flow-benchmark/`:

- Strict Serde scenario types with unknown-field rejection.
- Schema-version, identifier and digest-shape checks.
- Package/tote axis-aligned fit and gross-mass checks.
- Handling/seal class to storage-class checks.
- Household principal, service-provider audience and receiver-device role checks.
- Purpose/time/endpoint-bound grant checks, including explicit receive and tote-transfer actions.
- Transfer-chain continuity and asset/mode compatibility checks.
- Required safety and unauthorized-mutation gates.
- Fault-injection reference checks.
- A CLI (`mfb-validate`) and synthetic stationary-carrier JSON fixture.
- A dedicated GitHub Actions workflow for formatting, Clippy, tests and fixture validation.

The contract layer intentionally does not authenticate signatures or query Hearth/Commons. Separate L1 simulation, analytical convergence and protocol fault execution modules now exercise the reduced-order receiving scene and the pure handoff state machine. None of those software checks proves real-world safety, physical sensor integrity, HIL or field performance.

### 12.2 Machine-checkable ten-fault campaign

The standalone tool now also defines a `FaultCampaignManifest` and `mfb-validate-campaign` CLI. A campaign binds to the base scenario ID, schema version and source commit; it requires ten independent episodes, one for each v1 fault family, unique episode/fault identities, valid segment references and fault-specific expected terminal oracles.

| Fault episode | Expected terminal state |
|---|---|
| Wrong tote | Rejected |
| Over-mass | Rejected |
| Expired household grant | Rejected |
| Stale temperature observation | Safe hold |
| Blocked transfer gate | Safe hold |
| Damaged seal | Safe hold |
| Duplicate handoff | Reconciliation required |
| Missing receiver acknowledgement | Reconciliation required |
| Power interruption | Reconciliation required |
| Network loss | Reconciliation required |

The fixture `tools/material-flow-benchmark/fixtures/ten-fault-campaign.json` and its CLI verify only that the declared test campaign is complete and internally consistent. It does not execute a fault injection, simulate a physical response or prove safe recovery. Those results require the Symtropy scenario runner and later evidence levels.

**Verification status (2026-11-16):** newest dedicated workflow [`43823753266`](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43823753266) passed on exact branch head `386fc63ea84561f072ec58804154cb02f0d4b8d9`. It passed strict formatting, Clippy `-D warnings`, **76 Rust tests**, nominal scenario validation, campaign-manifest validation, all ten executable protocol faults, 15-step procedure graph validation, 14/14 procedure evidence-receipt admission, four-timestep analytical free-fall convergence, L1 simulation/replay and evidence upload. The dependency graph is committed in `Cargo.lock`; checks use `--locked`. Artifact `mfb-l1-evidence-386fc63ea84561f072ec58804154cb02f0d4b8d9` (ID `14205746262`) contains six files: protocol campaign, procedure readiness, procedure evidence admission, free-fall convergence, L1 result and environment metadata. SHA-256 ZIP digest: `634d4ad1001fe0a5590cfdb9a77aa97c1df1b1f7281a8be35260da4b61b3593d`. The admission report still explicitly leaves cryptographic verification, sensor truth, motion authority, physical command and task success false. No physical sensor, power, network, actuator, HIL, guarded-rig or field fault injection is claimed.

### 12.3 Distributed physical-transaction reference state machine

The Rust module `tools/material-flow-benchmark/src/transaction.rs` is a deterministic, hardware-independent reference protocol for one physical handoff. It models:
- purpose/time/audience/endpoint/scope/use-count authorization checks;
- one logical release-command ID, with duplicate suppression and rejection of a different command after dispatch;
- independently attributed source release, receiver receipt and receiver custody acknowledgement;
- event-ID replay/conflict checks and monotonic event sequence;
- **pre-dispatch** sensor disagreement as safe hold, but **post-dispatch** timeout, partition or sensor disagreement as reconciliation-required because the physical location may already have changed;
- explicit independent-custodian reconciliation outcomes: completed, no action taken, returned or quarantined.

A reconciliation resolution is accepted only from the configured reconciler actor, which must differ from the household principal, source actor and receiver actor. It must reference two distinct, well-formed evidence digests. The old transaction then becomes terminal: even a `NoActionTaken` result does not re-arm the old command. A new attempt requires a new transaction ID and a fresh authorization grant.

**Claim boundary:** the pure state machine verifies only declared identifiers, states and digest shapes. It does not authenticate Holochain signatures, establish that evidence sources are truly independent, prove the physical object location, or operate hardware. Two distinct digests are a structural requirement, not proof of evidence independence.

The new tests cover independent reconciliation, self-reconciliation rejection, duplicate evidence digests, pre-/post-dispatch sensor disagreement and event-sequence overflow. Exact-head CI qualification is tracked separately from physical fault-injection; the tests do not mean that a real transfer, simulator episode or HIL test has occurred.

### 12.4 First L1 physics episode — stationary receiver

The benchmark has a headless simulation entry point in `tools/material-flow-benchmark/src/simulation.rs` and the `mfb-simulate` CLI. It composes the scenario contract with Symtropy's existing `symtropy-physics` engine; it does not introduce a second rigid-body engine.

The current source uses:
- an open-top tote built from static 3D box colliders using the scenario's internal/external dimensions;
- a rigid-box package proxy using declared mass and dimensions;
- a fixed seeded offset above the tote under gravity;
- **480 Hz / 1,920 fixed steps over four seconds** with the `deterministic-net` Q16.16 feature;
- per-tick bitwise world snapshots hashed into a trace digest and repeated same-host replay;
- rotated world-space package AABB containment, settling, max-contact-penetration, rotation-orthogonality/determinant and non-finite-body checks;
- handoff state-machine completion only after placement, numerical-health and replay gates pass.

**Measured refinement history:**
- 120 Hz / 480 steps: max contact penetration **10.256 mm** (first baseline, run [43768458267](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43768458267), exact source `c2fa555623bdb8a242d238c3c2a0d6cf6885a0e1)).
- 240 Hz / 960 steps: max contact penetration **5.023 mm**, about 51% lower; safe containment, zero final linear/angular speed, same-host replay and custody completion remained true (run [43769246594](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43769246594), exact source `b9a8fc5416e721f81fe9a0c41d9cb710e78d799d)).
- **480 Hz / 1,920 steps:** max contact penetration **2.307 mm**, below the 5.000 mm hard gate. The result retained safe containment, zero final linear/angular speed, healthy numerical invariants, same-host replay equality and completed custody (workflow [43769459756](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43769459756), exact source `2643fc9df9c22dfa760179c1deab0cf2e7f16a0e`).

The 120 Hz and 240 Hz results remain preserved as baselines; only the 480 Hz run is counted as meeting the 5 mm target.

**Evidence level:** L1 simulation of one stationary receiver episode. The model is a rigid-box proxy, not deformable packaging, grasp planning, a robot, full carrier, truck/aircraft, route or fleet model. It contains no sensor uncertainty/metrology, perception, human/animal exposure, door/latch or actuator constraints, guarding or emergency-stop hardware. Same-host replay is demonstrated; cross-platform bitwise determinism is not claimed. The physics engine currently approximates asymmetric-body inertia, so the centred low-angular-motion drop does not qualify tumbling packages.

**Exact-head verification:** the current 480 Hz scene and Tier-B free-fall evaluator were re-run inside dedicated workflow [43819492450](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43819492450) on `a270a138095f187b6ad034fbb530cc27f746ec4f`, now alongside the executable ten-fault protocol campaign. Formatting, Clippy `-D warnings`, **61 tests**, all ten protocol episodes, both manifest validators, convergence, L1 replay and artifact upload passed. Evidence artifact `mfb-l1-evidence-a270a138095f187b6ad034fbb530cc27f746ec4f` (ID `14204140876`). L1 measured 2.307 mm peak penetration, zero final linear/angular speed, same-host replay equality, safe containment and completed custody.

### 12.5 Tier-B analytical free-fall convergence

The new `mfb-validate-convergence` CLI runs the same one-second, pre-contact free-fall initial condition at (dt = 1/30, 1/60, 1/120, 1/240) s and compares against the closed-form uniform-gravity position/velocity solution.

On exact source `b9a8fc5416e721f81fe9a0c41d9cb710e78d799d` (re-run unchanged in workflow [43819492450](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43819492450)), it measured:
- absolute position error: **163.436 mm, 81.725 mm, 41.228 mm, 20.812 mm**;
- absolute velocity error: **0.125 mm/s, 0.125 mm/s, 0.791 mm/s, 0.791 mm/s**;
- relative energy drift: **−0.164%, −0.0818%, −0.0404%, −0.0200%**;
- mean estimated position-convergence order: **0.9911**, with monotonically decreasing position error and all numerical-health checks true.

This supports a narrow Tier-B statement for **translational free fall before contact** under the tested Q16.16 build/host. It does not validate contact dynamics, packaging, asymmetric inertia, articulated systems, robot handling or the whole receiver. Independent-engine comparison and broader contact validation remain open in [#5469](https://github.com/Luminous-Dynamics/symtropy/issues/5469).

### 12.6 Executable ten-fault protocol campaign

The benchmark now distinguishes campaign **plan validation** from campaign **execution**:

- `mfb-validate-campaign` checks the fixed ten-fault manifest and expected terminal-state oracle.
- `mfb-run-campaign` creates a fresh handoff transaction for each episode, injects typed fault events into the Rust reference state machine and emits a machine-readable trace report.

The ten executed protocol cases are:
1. Wrong tote → preflight rejection; no release command.
2. Over-mass payload → preflight rejection; no release command.
3. Stale temperature observation → safe hold; no release command.
4. Interlocked/blocked transfer gate → safe hold; no release command.
5. Damaged or unknown seal → safe hold; no release command.
6. Duplicate handoff → same logical command replay is suppressed; a second distinct physical-release observation moves the transaction to reconciliation; retry is blocked.
7. Missing receiver acknowledgement after release → reconciliation required; retry is blocked.
8. Expired grant → rejected before dispatch.
9. Power interruption after release → reconciliation required; retry is blocked.
10. Network partition after release → reconciliation required; retry is blocked.

The runner checks that every observed terminal state matches the campaign oracle, no pre-dispatch case issues a command, every post-dispatch ambiguous case blocks retry, and no episode is falsely completed.

**Evidence level:** deterministic protocol-event injection against a pure in-memory state machine. These are real executed software transitions and negative controls, but not physical fault injection. It does not actually interrupt power, partition a network, corrupt a temperature sensor, jam a gate or move an object. It does not exercise a deployed Holochain conductor or hardware. Those require separate simulator, HIL and guarded-rig campaigns.

The CI workflow archives `protocol-fault-campaign.json` with the L1 physics result and analytical convergence output. Exact-head workflow [43819492450](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43819492450) passed on `a270a138095f187b6ad034fbb530cc27f746ec4f`: 10/10 protocol episodes were accepted; pre-dispatch faults never dispatched; duplicate logical command replay was suppressed; duplicate physical release evidence required reconciliation; post-dispatch timeout/power/network ambiguity blocked retry; and no fault was falsely completed. The full report is in artifact `14204140876`.

### 12.7 Evidence-typed reusable procedure graph prototype

MFB now includes `tools/material-flow-benchmark/src/procedure.rs`, a first typed procedure-DAG validator, plus `mfb-validate-procedure` and the synthetic `nominal-receiving-procedure.json` fixture.

The 15-step graph binds to the exact scenario ID, source commit and configuration digest. It declares typed evidence consumption/production for:
1. manifest validation;
2. household authorization receipt verification;
3. receiver capability verification;
4. package identity;
5. container fit;
6. payload mass;
7. seal integrity;
8. temperature envelope;
9. transfer-gate interlock;
10. release-intent preparation;
11. source-release observation;
12. receiver-arrival observation;
13. placement verification;
14. counterparty custody acknowledgement;
15. unknown-outcome reconciliation.

The validator checks unique operation/step identity, closed dependency references, acyclic ordering, exact typed evidence contracts, evidence producer ancestry and required preflight dependencies before release intent. Reconciliation remains an explicit branch after intent rather than an optimistic retry.

**Anti-self-approval boundary:** the graph's only success state is `structurally_valid_for_external_review`. The report explicitly fixes `motion_authority_granted=false`, `physical_command_emitted=false` and `task_success_attested=false`. A separately governed authority, executor, observer and evaluator are still required. The prototype validates the declared graph structure, not evidence authenticity, signatures, freshness, physical execution or success.

This is the minimal executable foundation for the procedure-learning hypothesis in [#5501](https://github.com/Luminous-Dynamics/symtropy/issues/5501).

**Exact-head verification (2026-11-16):** workflow [43821500342](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43821500342) passed on source `970ae35eb28d6ffaae0c4fbb141b5156b4a2312f`: strict formatting, Clippy, **67 Rust tests**, campaign execution, procedure graph CLI, convergence, L1 simulation/replay and evidence upload. The report confirmed a 15-step valid graph with no motion authority, no physical command and no success attestation. Artifact `mfb-l1-evidence-970ae35eb28d6ffaae0c4fbb141b5156b4a2312f` (ID `14204208667`) includes the raw procedure readiness report.



### 12.8 Procedure evidence-receipt admission and trust-domain separation

The procedure graph has a receipt-envelope admission layer in `src/procedure.rs`, a 14-receipt synthetic bundle at `fixtures/nominal-receiving-evidence.json`, and an `mfb-admit-procedure-evidence` CLI.

The admission layer checks exact procedure/scenario/source/configuration binding, subject, allowed actor role, required receipt coverage, SHA-256 digest syntax, declared time windows and per-kind maximum observation age. It now requires strict inequality at both uncertainty boundaries: an observation at the earliest plausible decision time is rejected, as is an expiry at the latest plausible decision time.

It also maps receipts into six future verifier domains:
1. household grant authority;
2. service-provider attestation;
3. receiver capability attestation;
4. physical-observation attestation;
5. counterparty custody attestation;
6. transaction-outcome attestation.

This is a routing/architecture classification only. It does not mean these six verifiers already exist or that their evidence is trusted.

The output explicitly distinguishes:
- structural envelope admission;
- declared freshness policy result;
- content digest recomputation;
- cryptographic authentication;
- issuer authorization;
- trusted chronology;
- sensor/physical truth;
- motion authority, command execution and task success.

The current implementation can report structural admission and a pass of the declared-time freshness policy. It explicitly reports `content_digest_verified=false`, `cryptographic_authentication_verified=false`, `issuer_authorization_verified=false`, `trusted_time_verified=false`, `sensor_truth_established=false`, `motion_authority_granted=false`, `physical_command_emitted=false` and `task_success_attested=false`. Digest syntax alone is not a content commitment; declared timestamps are not trusted chronology.

**Freshness policy v1** uses maximum ages: manifest 300,000 ms; authorization 60,000 ms; receiver capability and container fit 86,400,000 ms; package identity and payload mass 2,000 ms; seal 5,000 ms; temperature 2,000 ms; transfer-gate and receiver-presence 500 ms; release-execution 10,000 ms; placement 1,000 ms; counterparty custody 30,000 ms; ambiguous outcome 5,000 ms. These limits apply to declared observation times at the latest plausible scenario decision time. They are operational screening bounds, not a timestamp-authority theorem.

**Use the existing security substrate:** Symtropy PHYS-EVID-03A/#1143 defines canonical persistent evidence; B0/#1144 defines the dedicated hybrid Ed25519 + ML-DSA-65 profile and exact transcript; B1/#1145 governs credential authorization. Xenia `xenia-peer#341` defines the separate typed evidence-attestor boundary. MFB should consume the verified and issuer-authorized types from that programme. It must not implement a second signature format, a local Ed25519-only path, accept caller-selected transcript bytes, or treat cryptographic validity as proof of issuer authorization or sensor truth.

The non-physical receipt domains (household grant, provider records, receiver capability, custody and transaction outcome) must use their respective qualified native authorities. Integration and the evidence-kind-to-verifier matrix are tracked in [#5530](https://github.com/Luminous-Dynamics/symtropy/issues/5530).

**Exact-head history:** the earlier structural-admission implementation passed [workflow 43823753266](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43823753266), source `386fc63ea84561f072ec58804154cb02f0d4b8d9`, with 76 tests and 14/14 synthetic envelopes admitted. The required-verifier-domain map, new anti-claim fields and strict timestamp-boundary rejection were added afterwards and need their own exact-head qualification. The earlier run does not qualify them.


### 12.9 First L1 scene-level physical fault proxy

The L1 physics module now includes `simulate_closed_gate_obstruction` and the `mfb-simulate-gate-fault` CLI. It inserts a static collision proxy across the tote opening, replays the same deterministic 480 Hz scene, and checks that:
- the package physically remains outside the tote and the placement predicate fails;
- the scene remains numerically healthy and the contact-penetration gate passes;
- the same-host replay is bitwise equal;
- the handoff records an observation disagreement after logical release and enters `ReconciliationRequired`;
- a retry against the old logical command is blocked;
- no false delivery completion is emitted.

The report is accepted as a *fault-oracle pass* only when the above conditions hold, including a non-zero package↔gate collision-event count and measured impact impulse. The nested nominal-delivery `accepted` flag must be false. This keeps “the system responded correctly to a fault” distinct from “the delivery succeeded.”

The physics engine now records one finalized `CollisionEvent` per resolved contact manifold after the solver loop. The event vector is captured into `WorldSnapshot`, hashed into the replay trace and exposed in the fault report by exact body handles. A regression test in `symtropy-physics/src/world.rs` checks that a falling sphere produces one finalized event and that the replay snapshot retains the same body pair. This repairs an event-channel gap: the `collision_events` vector was previously cleared at each step but never populated.

**Exact-head qualification:** workflow [43836138408](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43836138408) passed on source `c1a388ab592d2bbbd07ced65662aea827b77e26e`: strict formatting, Clippy `-D warnings`, **79 tests**, the procedure/evidence/ten-fault checks, convergence, nominal 480 Hz L1 replay, and the gate-fault CLI. The gate-fault report measured 1,841 package↔gate collision events, maximum impulse 2.1043744963 N·s, peak penetration 2.6422119 mm, replay equality and settled final state. The package remained outside the tote; transaction state was `reconciliation_required`; retry was blocked; nested delivery acceptance was false; fault oracle was true.

Evidence artifact `mfb-l1-evidence-c1a388ab592d2bbbd07ced65662aea827b77e26e` (ID `14211340768`), ZIP digest `f32e707b516cdc8071d8c60aaa2e0c019bbc99e5b6985b6749d2e3b31cac38db`. The gate-fault result JSON digest is `f97581eefb8e52b2d2680158e36761a4673cd96dca6f0db77055be4cbb398cdb`.

**Claim ceiling:** the gate is static collision geometry in the L1 scene. It is not a moving door, physical interlock, actuator/power fault, sensor timing, perception/firmware test or HIL. The report is evidence about the reduced-order rigid-body model and software response only. Further fault timing, dynamic gate, power-loss and sensor-channel work remains in #5500.

### 12.10 No-drop 30-seed L1 gate-obstruction robustness sweep

The scene-level gate fault now has a deterministic multi-seed sweep API, `simulate_closed_gate_obstruction_sweep`, and CLI `mfb-simulate-gate-fault-sweep`.

The runner defaults to 30 consecutive seeds beginning at the frozen scenario seed (bounded to 1..=256). For each seed it runs the full 480 Hz scene twice, checks same-host replay, collects package↔gate collision-event count and impulse, tests the placement/reconciliation/retry oracles, and emits a per-seed trace digest. A failed simulation or failed oracle remains a row in the output with an explicit failure reason; no case is silently removed from the denominator.

The summary reports requested vs executed runs, passed/failed counts, all-oracles-passed, total/min/max gate contacts, max impact impulse, worst contact penetration, reconciliation count, retry-blocked count and false-completion-blocked count.

Unit tests check a three-seed sweep is exactly repeatable, all three cases remain present and each requires reconciliation/retry blocking, and run counts of zero or above the configured maximum reject.

**Statistical boundary:** this is a deterministic initial-offset sweep for one synthetic scenario and one static gate proxy. It does not vary home layouts, robot platforms, sensors, site conditions, package families, firmware or physics engines. Do not use 30 seeds as a claim of 30 independent field trials or as a reliability confidence interval. The full F2 design in #5500 must stratify independent task families/sites and preserve failed runs.

The workflow now requests 30 seeds and archives `gate-obstruction-fault-sweep.json` with its own SHA-256. Exact-head qualification is pending for this new runner.

### 12.11 Receiver-observation consistency prototype

The MFB toolkit now includes `src/observation.rs`, the `mfb-adjudicate-presence` CLI and `fixtures/false-positive-receiver-observation.json`.

The adjudicator takes only declared observations and a versioned policy. It does not take or expose simulator ground truth. It requires two fresh observations from distinct declared common-cause groups before reporting a positive or negative presence result. It treats common-cause group identity (controller, power, firmware, calibration or clock) as a first-class field; two sensors sharing one group are one source, and absent group membership cannot be assumed independent.

The v1 report distinguishes `ConfirmedPresent`, `ConfirmedAbsent`, `Unknown`, `Contradictory` and `Stale`. It rejects event replay, conflicting source identity, subject/scenario mismatch and readings that touch the decision-time boundary. Twelve unit tests cover a lone positive, positive quorum, correlated sensors, false-positive/negative disagreement, stale evidence, duplicate event identity, source identity conflict, clock-boundary staleness, wrong subject/scenario and unknown common-cause groups.

The fixture models a false-positive receiver-presence reading against an independent negative gate-contact reading. CI expects `Contradictory`, `positive_presence_confirmed=false` and `custody_completion_permitted=false`.

**Claim boundary:** this checks consistency of declared observation envelopes only. The report hard-codes `sensor_truth_established=false`, `cryptographic_authentication_verified=false`, `issuer_authorization_verified=false`, `trusted_time_verified=false`, `motion_authority_granted=false`, `physical_command_emitted=false` and `custody_completion_permitted=false`. The declared common-cause group is not authenticated; no sensor truth, trusted chronology or physical result is established. Ground truth remains evaluator-only in future controller-vs-oracle benchmark tracks. This is the first executable slice for [#5601](https://github.com/Luminous-Dynamics/symtropy/issues/5601); it is not yet integrated into the physical handoff transaction path.

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

- [Symthaea CEB Evidence Continuity Protocol](https://github.com/Luminous-Dynamics/symthaea/blob/main/docs/research/CEB_EVIDENCE_CONTINUITY_PROTOCOL_V0_2026-11-05.md) and [CEB Claim-Support Closure / Selective Invalidation Protocol](https://github.com/Luminous-Dynamics/symthaea/blob/main/docs/research/CEB_REVIEW_CLAIM_SUPPORT_CLOSURE_AND_SELECTIVE_INVALIDATION_PROTOCOL_V0_2026-11-08.md).
- [SafeVLA-Bench overview and live leaderboard](https://safevla.org/) and [safety-ranked results](https://safevla.org/leaderboard).
- [2026 BEHAVIOR Challenge](https://behavior.stanford.edu/challenge/index.html) and [public ninth-place solution report](https://arxiv.org/abs/2611.14432).
- [RoboCasa365 leaderboard](https://robocasa.ai/leaderboard) and [evaluation methodology](https://robocasa.ai/leaderboard.html).
- [BEHAVIOR-1K](https://behavior.stanford.edu/), [2026 challenge](https://behavior.stanford.edu/challenge/index.html), [evaluation rules](https://behavior.stanford.edu/challenge/evaluation.html) and [baseline methods](https://behavior.stanford.edu/challenge/baselines).
- [HomeRobot OVMM](https://ovmm.github.io/).
- [VDA 5050 v3.0 release](https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN).
- [GS1 EPCIS 2.0.1 archive](https://ref.gs1.org/standards/epcis/archive).
- [ISO 3691-4:2023](https://www.iso.org/standard/83545.html), [ISO/DIS 3691-4 Edition 3](https://www.iso.org/standard/88615.html) and [ISO 13482 Edition 2](https://www.iso.org/standard/83498.html).
- [SACAA UAS guidance](https://www.caa.co.za/industry-information/uas/).
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
