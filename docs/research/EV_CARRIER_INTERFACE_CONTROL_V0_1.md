# EV Carrier Interface Control and Qualification Contract v0.1

**Programme:** EV-CARRIER-REF-01  
**Issue:** [EV-CARRIER-ICD-001 #6308](https://github.com/Luminous-Dynamics/symtropy/issues/6308)  
**Parent:** [EV-CARRIER-REF-001 #6109](https://github.com/Luminous-Dynamics/symtropy/issues/6109)  
**Status:** interface-control template; no physical module interface is yet qualified  
**Claim ceiling:** configuration and evidence-admission contract only

## 1. Purpose and architectural decision

This document establishes a versioned contract for the physical and digital interfaces between the electric logistics carrier and interchangeable cargo, UAV, trailer and AMR modules.

The carrier programme compares:
- integrated enclosed drone bay;
- detachable UAV/service pod;
- mobile drone-station trailer;
- AMR/cargo-handoff module;
- truck-only configuration as the unchanged control.

The paradigm shift is **product-family modularity with independently qualified boundaries**, not a single universal connector. A shared interface is adopted only when a real product/mission family benefits from it and the shared mechanical, electrical, thermal, environmental, cybersecurity and service envelopes can be qualified together.

A common schema or matching connector name is not proof of physical interchangeability.

## 2. Ownership and boundaries

| Domain | Existing owner | ICD responsibility |
|---|---|---|
| Carrier product/reference configuration | Symtropy #6109 | Exact vehicle/module configuration identity and interface revision |
| Vehicle architecture, mission and verification | Symtropy #2112 / #2114 / #2164 | Source vehicle envelope and requirement/test links |
| Coupled logistics physical scene | Symtropy #5393 | Coupled bodies, cargo, human/robot occupancy, transfer boundaries |
| UAV/carrier co-availability | Symtropy #6083 | Flight/rendezvous and pad/crew/aircraft time windows |
| Carrier mass/energy/power screen | Symtropy PR #6175 | Incremental mass, battery, bus and charging boundary |
| Module process/manufacturing overlay | Symtropy PR #6193 | Manufacturing route, EOL process and service station evidence |
| Factory archetypes | Symtropy #2355 | Plant-level assembly capacity; not connector or structural approval |
| Robot fleet communication | VDA 5050 adapter owner within MFB | Fleet-control/AMR message interface only |
| Asset, serial and custody lineage | Mycelix #3709 / #6062 / #6076 | Identity, ownership, lifecycle and passport projection |
| Place/site and rights context | Sol-Atlas | Sourced location/site constraints, not aviation or electrical approval |

The ICD does not create another vehicle simulator, battery model, aircraft model, route solver, geospatial engine, factory scheduler, asset registry or battery-passport compiler.

## 3. Interface decomposition

Each carrier/module pair must bind the following interface families independently. A single overall compatibility result is the conjunction of the applicable family results.

### 3.1 Mechanical structure and retention
The controlled profile must identify:
- coordinate system, origin, datum targets, mounting plane, hole/fastener pattern and tolerance-stack revision;
- structural load path into the vehicle body/chassis/subframe;
- material, fastener, locking, corrosion and galvanic-isolation specification;
- module mass, centre of gravity, inertia and permitted payload/load distribution;
- limit and ultimate loads for vertical, lateral, longitudinal, torsional, braking, cornering, impact, vibration, fatigue, lifting and recovery cases;
- restraint/latch geometry, positive retention indication, inspection method and wear limits;
- axle-load, gross-mass, centre-of-gravity, braking/handling and stability impacts for the as-installed configuration.

No numeric load or structural rating is introduced by this template. Those values require an accountable design authority, analysis method, test article/configuration and acceptance report.

### 3.2 High-voltage DC
The controlled profile must specify:
- voltage class and operating range, polarity, current/power continuous and peak envelopes, peak duration and transient conditions;
- connector, keying, shielding, cable and strain-relief profile;
- protective devices, short-circuit coordination, isolation monitoring, protective bonding and discharge behavior;
- contactor/pre-charge sequencing, HV interlock (HVIL), service disconnect, touch-safe state and verification;
- emergency isolation, fault containment, safe-state timing and re-energization conditions;
- source-to-load measurement boundary and energy accounting.

A module may not energize merely because a connector physically mates. Live mating is not assumed. Vehicle and module owners must define the approved de-energized mating/de-mating sequence and verify it.

### 3.3 Low-voltage power
The selected rail(s) remain unresolved until the platform owner specifies them. Each profile must bind voltage range, continuous/peak current, inrush, reverse-polarity protection, brownout/restart behavior, return/ground scheme, connector keying, fusing, EMC and emergency-power behavior.

### 3.4 Thermal, fluid and fire boundary
Specify heat rejection, temperature range, coolant/fluid identity, flow/pressure limits, coupler, leak detection, isolation, ventilation, thermal barriers, vent direction, propagation prevention, detection/suppression strategy and service safe-state. Aircraft battery and charger spaces require their own hazard analysis; vehicle battery qualification does not automatically qualify the aircraft battery or pod.

### 3.5 Data, control and cybersecurity
Bind:
- physical link layer, connector, topology, protocol and protocol revision;
- message/schema compatibility and version negotiation;
- identity of vehicle, module, aircraft, charger and controller;
- command authority, message freshness, clock assumptions, authentication and authorization;
- diagnostics, fault reporting, event sequencing, secure boot/update, rollback, revocation and decommissioning;
- safe response to link loss, replay, contradictory controller state and stale telemetry.

VDA 5050 is relevant to the central fleet-control ↔ mobile-robot communication boundary; it is not a mechanical, HV, flight-control or safety qualification.

### 3.6 Cargo, UAV and AMR
For cargo: envelope, restraint, latch status, payload, centre of gravity, access/egress and verified package identity.

For UAVs: storage/retention, aircraft and battery removal, charger isolation, maintenance clearance, stationary launch/recovery fixture, pad alignment, propeller clearance, weather protection, exclusion zone, safe maintenance and emergency egress. v0.1 is parked/stationary launch and recovery only.

For AMRs: docking/transfer geometry, physical load transfer, localization reference, stop/interlock semantics, transfer-complete evidence and interoperability profile.

Compatibility of a bay, pad or dock does not establish flight permission, airworthiness, route clearance or task authority.

### 3.7 Environment and service
Bind mounting-location temperature, vibration/shock, dust/water ingress, corrosion, chemical exposure, cleaning, connector mate/de-mate cycles, inspection, safe isolation, lifting/tooling, access and replacement procedure. Environmental class must be specific to actual mounting location; a vehicle exterior, sealed cargo bay and protected electronics enclosure do not share one assumed IP or vibration rating.

## 4. Interface-control record

The machine-readable contract in `schemas/ev_carrier_interface_control_v0_1.schema.json` binds:
- exact vehicle and module configuration IDs/revisions;
- unique interface-pair and port IDs;
- family, side, revision, mating-profile ID and controlled profile reference;
- evidence class, source reference, configuration scope, report digest and validity window;
- per-pair compatibility assessment and reason codes.

Each controlled profile reference must resolve to an approved drawing, specification, ICD section, calculation, test procedure or test report under the responsible discipline owner.

### Result semantics

- `compatible`: all required profile identifiers and revisions match; the exact vehicle/module configuration is covered; every release-critical family has current, independently reviewed qualification evidence; no unresolved change or mismatch exists.
- `incompatible`: a known, explicit contradiction exists (for example, different keyed mating profile, incompatible declared voltage/polarity, insufficient qualified load envelope or protocol mismatch).
- `indeterminate`: evidence is missing, synthetic, stale, conflicted, revoked, unreviewed, out of scope or not tied to the exact configuration revision.

Unknown required values must never be converted to zero, default ratings, generic connector equivalence or a compatibility pass. A software result does not authenticate its source documents; it records and checks their declared identities and states.

## 5. Qualification and change-control matrix

| Family | Analysis / inspection | Qualification test examples | Required independent evidence |
|---|---|---|---|
| Mechanical mount | datum/tolerance analysis, load path, axle/CG | static pull/shear/torsion, vibration, fatigue, impact and retention | drawing/revision, FEA assumptions, measured load test, metrology and as-built inspection |
| HV DC | protection coordination, insulation and fault analysis | isolation, dielectric withstand, HVIL, contactor/pre-charge, discharge, short/fault and thermal rise | exact voltage/current envelope, wiring diagram, test report, calibration and safety review |
| LV power | load budget, transient and grounding analysis | brownout, inrush, reverse polarity, short circuit and reset recovery | exact rail/current envelope and test report |
| Thermal/fire | heat balance, propagation and vent analysis | thermal cycling, flow/leak/pressure, fault isolation and applicable fire/propagation testing | exact battery/module type, hazard analysis and independent test report |
| Data/control | protocol/schema compatibility and threat analysis | disconnect/reconnect, replay, stale, revoked identity, downgrade and rollback | protocol/profile version, threat analysis, security test and configuration digest |
| Cargo/UAV/AMR | access, restraint, handoff and failure analysis | latch/retention cycles, docking repeatability, failed transfer, safe isolation | physical test article identity, load/geometry reports and safe-state witness |
| Environment | mounting-location profile | temperature, shock/vibration, ingress, corrosion/chemical/cleaning | test profile, equipment calibration, exact enclosure and placement revision |
| Service | maintenance task/hazard analysis | repeated mate/de-mate, inspection, replacement and requalification | approved method, tools, time study, worker safety review and replacement record |

Each result must identify test article serial/configuration, procedure revision, instrumentation/calibration, environment, sample count, measured values, uncertainty, acceptance criteria, failures/rework, witness/approver and immutable report digest.

A planned test, synthetic fixture or simulation is not completed physical qualification. A changed load path, HV pinout, connector keying, module mass/CG, firmware, thermal boundary or safety sequence triggers impact analysis and dependent requalification.

## 6. Standards applicability register

These are engineering review anchors, not an assertion of conformity or universal applicability:

- **IEC 61851-1:2017**: EV supply equipment, connection and electrical-safety requirements for conductive charging: https://webstore.iec.ch/en/publication/33644
- **ISO 15118-20:2022 + Amendment 1:2026**: EV ↔ EVSE communication and power-transfer control; it does not define every internal vehicle/module bus: https://www.iso.org/standard/77845.html
- **ISO 6469-3:2021**: voltage-class-B electric road-vehicle electrical safety; **ISO/DIS 6469-3** is in development and may replace it: https://www.iso.org/standard/81746.html and https://www.iso.org/standard/90267.html
- **ISO 16750-1:2023**: vehicle electrical/electronic equipment environmental conditions; a revision is in working draft: https://www.iso.org/standard/77578.html and https://www.iso.org/standard/91675.html
- **ISO 20653:2023**: road-vehicle enclosure IP classification and test methods: https://www.iso.org/standard/76116.html
- **ISO/SAE 21434:2021**: road-vehicle cybersecurity engineering across concept, development, production, operation, maintenance and decommissioning; an Edition 2 work item is under development: https://www.iso.org/standard/70918.html and https://www.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/09/54/95437.html
- **ISO 12100:2010**: machinery risk-assessment and risk-reduction method; confirmed current by ISO: https://www.iso.org/standard/51528.html
- **VDA 5050 v3.0.0 (March 2026)**: central fleet-control ↔ mobile-robot message interface; it does not certify mechanical/electrical fit or robot safety: https://www.vda.de/en/topics/automotive-industry/vda-5050 and https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN

The final applicability register must name vehicle category, module class, operating context, jurisdiction, responsible legal manufacturer/economic operator and technical service. Current draft revisions must be tracked; do not claim a draft is a published standard.

## 7. Initial acceptance targets

- 100% of physical ports have unique IDs, side, family, revision, mating-profile ID and controlled profile reference.
- 100% of release-critical mechanical/HV interfaces have source-bound envelopes and named qualification plans.
- Zero unknown, stale, conflicted, revoked, synthetic-only or out-of-configuration interfaces classified as `compatible`.
- Zero known mismatches classified as `compatible`.
- Every mating pair has an explicit safe mate/de-mate/isolation sequence.
- Every configuration change has a dependency impact analysis across BOM, fit, installation instructions, software, tests and maintenance.
- At least one negative control per family (wrong datum/keying, voltage/polarity, load, fluid, protocol, ingress or service tool profile).
- No production-release claim until applicable physical tests, regulatory review and independent discipline sign-off are recorded.

## 8. Next implementation slice

The first software contract must:
1. validate exact vehicle/module and interface revisions, unique port/pair IDs and strict unknown-field rejection;
2. bind controlled profile references and evidence scope;
3. classify only explicit known mismatches as incompatible;
4. keep absent or non-qualified release-critical evidence indeterminate;
5. be deterministic under input-order permutations;
6. emit no power, actuator, unlock, launch, reservation or dispatch commands.

The executable screen should initially be an evidence-admission/compatibility contract. Family-specific numerical envelope comparisons should be added only when the responsible engineering owners supply signed-off profile semantics and test evidence. Do not manufacture placeholder load, voltage, current, temperature, IP or tolerance values to make a synthetic fixture pass.

## 9. Non-claims

This template does not establish a connector selection, structural rating, HV safety, fire safety, EMC compliance, functional safety, cybersecurity approval, roadworthiness, airworthiness, homologation, production release or commercial readiness. All such claims require exact configuration data, qualified engineering analysis, physical tests and appropriate regulatory/technical approval.
