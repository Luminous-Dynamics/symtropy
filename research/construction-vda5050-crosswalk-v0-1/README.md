# Construction VDA 5050 v3 Crosswalk — v0.1

**Status:** synthetic semantic crosswalk and pure-Rust projection guard. Not a production VDA adapter, fleet manager, safety system, construction acceptance authority, or standards-conformity assessment.

## Objective

Reuse VDA 5050 where it is genuinely useful, while preventing its transport and fleet-control meanings from being mistaken for construction work semantics.

The canonical construction work contract remains owned by parent issue #3644. This slice defines a version-pinned mapping boundary:

- VDA messages carry transport order, route, robot-action, navigation-map, robot-state and fleet-traffic information.
- A separate construction sidecar binds the order to the approved work package, design revision, site epoch, exact target frame/transform, robot configuration/capability profile, resource reservations, pre/postconditions and evidence requirements.
- The robot's own state remains an executor report. Inspection and independent acceptance are separate external records.

A successful VDA message exchange is transport interoperability, not proof of useful accepted construction output.

## Verified VDA 5050 v3 baseline

The published VDA 5050 v3.0.0 release (March 2026) adds support for freely navigating robots, planned/intermediate path sharing, and a zone concept. The order is a node-edge route graph. The map coordinate system is project-specific, right-handed, z-up, uses metres for position and radians for orientation. Map identity/version and zone/action state have their own VDA meanings. The guideline uses MQTT; QoS 0 is specified for order, instantActions, state, factsheet, zoneSet, responses and visualization, while connection uses QoS 1. The guideline leaves broker security to external configuration and says a robot keeps its order and executes up to the last released node after a broker disconnection.

The VDA repository says that its published document is authoritative if its Markdown or JSON schemas differ. The upstream issue #660 records a release-artifact integrity defect: three JSON Schema files in the 3.0.0 tag (order, factsheet, visualization) are invalid JSON due to trailing commas. The correction is present at upstream commit ad169568749f3799e3aac72ae814c4167a7677f9. This profile pins the release-tag Git blob IDs and the corrective commit's blob IDs separately. The corrected repository snapshot is useful for schema tooling, but is not silently relabelled as the 3.0.0 release artifact. Any production adapter must verify the normative published VDA document and its exact licensed/authorized source artifacts before claiming VDA conformance.

## Semantic crosswalk

| Construction meaning | VDA surface | Allowed interpretation | Required boundary |
|---|---|---|---|
| Task ID | orderId | VDA transport-order correlation only | Keep canonical task ID in sidecar |
| Task update | orderUpdateId | Transport-order update sequence only | Never use as design/task revision |
| Work package | None | Not represented | Required sidecar |
| Design revision/digest | None | Not represented | Required sidecar |
| Site epoch | None | Not represented | Required sidecar; check freshness before every new admission |
| Target frame | nodePosition coordinates | Bounded navigation-coordinate projection | Exact frame/transform evidence remains sidecar |
| Navigation map | mapId/mapVersion | Robot navigation map only | Never substitute for model revision or site epoch |
| Work zone | zoneSet zoneId | Fleet traffic/navigation zone only | Explicit project-zone crosswalk and geometry revision required |
| Robot capability | factsheet typeSpecifications | Robot-reported capability data | Current configuration and qualification remain external |
| Route | order nodes/edges | Navigation route only | Not the construction work/dependency graph |
| Robot action | node/edge actions | Robot command/action parameters | Not a postcondition, inspection, or acceptance |
| Resource access | zone requests/releases | Fleet traffic access only | Not a general capacity lease or safety permit |
| Execution progress | state nodeStates/edgeStates/actionStates | Executor-reported progress | Not physical measurement or accepted output |
| Output evidence | None | Not represented | External receipt required |
| Inspection | None | Not represented | Independent inspection receipt required |
| Independent acceptance | None | Not representable | External authorized receipt required |
| Safety authorization | None | Not representable | External approved safety architecture only |

The full mapping table is in profile/vda5050-construction-crosswalk-v0.1.json. Every row requires the construction sidecar, including rows with bounded VDA projections. There is no field-level "lossless" claim in this version.

## Sidecar binding

The reference Rust API checks that these fields are present, well-formed and identical across the expected task context and the sidecar received alongside the VDA order:

- task and work-package identifiers;
- VDA transport order ID and update ID;
- design revision digest and current site epoch;
- target frame and transform-bundle digest;
- robot configuration and capability-profile digests;
- work-zone identity and resource reservation digest;
- precondition and postcondition digests;
- evidence profile and acceptance-authority references.

Digest syntax is checked as 64 lowercase hexadecimal characters. The crate does not compute those digests, fetch the referenced bytes, authenticate the actor or sidecar, prove the authority reference, or grant dispatch permission. Structural equality is only one precondition supplied to the existing owning assurance components.

## Guarded state projection

The core can project:
- order received -> transport order observed;
- node traversed -> route progress reported;
- action finished -> executor action finished reported;
- zone access granted -> fleet traffic access reported;
- robot error -> robot fault reported;
- connection lost/state unavailable -> communication or state unavailable;
- unknown extension -> unclassified signal.

The public projection enum deliberately has no variants for work completed, independently accepted, safety authorized, motion authorized or safe. There is therefore no code path in this crate that converts an action-finished or route-traversed report into accepted construction output.

## Relationship to adjacent standards and owners

The standards landscape is layered rather than competitive. No one item below supplies the entire construction-specific contract.

- **VDA 5050 v3.0.0** is a vendor-neutral robot-to-fleet-control interface. It provides transport orders, route node/edge graphs, robot action state, map identity/version and fleet traffic zones. It is the primary first transport target here; it does not define construction work-package acceptance, site safety authorization or independent physical output evidence.
- **AMRA-271:2025 — Mobile Robots: Common Communication Protocols** is a direct comparator worth including in the next cross-protocol plugfest. Its published scope covers transactions between mobile robots and a mobile-robot management system, robots and peripherals, and management systems and peripherals; it describes the message structure at the presentation layer. It covers wheels, tracks and legs, but not internal robot-module communications. It is broader than a single VDA order mapping and should be treated as a separate protocol candidate, not a field synonym for VDA 5050. https://www.amr-alliance.org/amra-271/
- **ISO 21423:2026** standardizes communication/interoperability for industrial mobile robots, fleet managers and other machines/applications in its defined shared industrial environment. Its published scope excludes navigation and safety requirements; its definition of the industrial environment includes restricted public access / production-related contexts. A construction project with public interfaces or outdoor/site-specific hazards must not assume applicability or safety coverage. This profile does not claim ISO conformity.
- **AMRA-201:2026 — Mobile Robots: General Requirements and Test Methods** applies to mobile robots operating on solid travel surfaces and includes wheeled, legged and tracked robots and multiple terrains (except underwater or in the air). It is a useful comparative baseline for robot-side product/performance tests, not a construction work-package or acceptance contract. https://www.amr-alliance.org/amra-201/
- **AMRA-220:2022 — Mobile Robots: Safety Requirements, Verification and Validation** is explicitly scoped to wheeled mobile robots in indoor industrial and/or service environments and excludes outdoor applications. Its safety principles may be considered outdoors, but it must not be represented as an outdoor construction-site standard. https://www.amr-alliance.org/amra-220/
- **MassRobotics AMR Interoperability** is a useful mobile-robot data-sharing comparator for location, destination, speed, direction, identity, timestamp, dimensions and operational state. Its own description says it is not a fleet-management, navigation or safety system. https://www.massrobotics.org/what-is-the-massrobotics-amr-interoperability-standard/
- **OPC UA for Robotics (OPC 40010-1 v1.02)** provides a robotics information model spanning industrial robots, mobile robots, controllers and peripherals, with asset-management, condition-monitoring, configuration and runtime data. It is complementary vertical/asset information, not a construction production acceptance standard. https://reference.opcfoundation.org/specs/OPC-40010-1
- **IFC 4.3 / ISO 16739-1:2024** is the open built-asset information exchange model. **IDS 1.0** can express machine-checkable alphanumeric exchange requirements for IFC objects, classifications, properties and values/units; IDS does not validate geometry or physical correctness. This crosswalk does not implement an IFC parser, BIM clash engine or IDS validator.
- #3561/#3564 own pre-dispatch design/export/frame/inventory admission; #1607 owns survey/frame/transform evidence; #3542/#3558/#3566 own site epochs, observation coverage and end-to-end assurance; #2005 owns IFC/IDS-to-fabrication semantic-loss benchmarking.
- #3644 owns the protocol-neutral construction task contract and task/resource lifecycle. This child consumes it and tests the VDA projection boundary; it does not duplicate the task state machine or resource calendar.

## External implementation candidates (not conformance evidence)

A quick ecosystem check identifies useful software counterparts for a future independent plugfest:

- **NVIDIA Isaac Mission Dispatch / Mission Client**: its repository changelog says VDA 5050 3.0 support was added in August 2026. The system includes a ROS 2 mission client and a lightweight robot-client simulator. The project explicitly says Mission Dispatch supplies communication and mission-state plumbing but does not itself perform task allocation or path-conflict resolution. Candidate pinned source commit: `nvidia-isaac/isaac_mission_dispatch@56455eb757173cb6c0babe2e52cfb2a8e3a8bdee`; ROS 2 client repository snapshot: `NVIDIA-ISAAC-ROS/isaac_ros_cloud_control@90b21661a770ff4b6395b798c42320d0ef791c4f`. This is a useful external VDA 3.0 software endpoint, not a construction robot or a field validation.
- **vda5050-lab**: a separate Rust project with a VDA 5050 3.0.0 trace diagnostic tool and isolated MQTT demo using synthetic robot clients and fleet control. Pinned observed repository commit: `ekusiadadus/vda5050-lab@4a1a812f9418084f658b65856cf3e1a5bbc2cf51`. It could serve as an independently authored passive observer or software test peer; its own README says no customer trace, hardware execution or production interoperability has been validated. Do not treat its synthetic demo as a real robot.
- **coatyio/vda-5050-lib.js**: its README currently documents support for VDA 5050 versions 1.1, 2.0 and 2.1, not 3.0. It is potentially useful for legacy-version migration/negative testing, but it must not be mislabelled a VDA 3.0 adapter until a separately pinned 3.0 implementation is demonstrated.

The practical implication is that we do not need to invent every protocol endpoint before testing. We can first build a read-only, evidence-preserving bridge to an external VDA 3.0 implementation and replay the same broker trace through a second independently maintained observer. That will prove cross-implementation semantic handling at the software boundary, not vendor/hardware interoperability. A construction-vendor robot and a separately maintained vendor adapter remain necessary for the higher plugfest gates.

## Industry baseline and research gap

VDA 5050 is not merely a paper standard. VDMA's 2025 Mesh-Up reported a fleet manager controlling ten mobile robots of different manufacturers over the open VDA 5050 interface in a production/logistics showcase. This is a strong precedent for mixed-fleet transport interoperability, but it is not evidence of construction work-package semantics, outdoor-site safety, independent physical acceptance or construction productivity. The showcase also predates VDA 5050 v3.0. https://www.vdma.eu/documents/34570/3317035/PI_Mesh-Up_2025_EN.pdf/5525183d-9a5f-294d-aaa9-9f3bdec3f309?t=1740489629327

Construction research shows a different bottleneck. A 2026 systematic review covers 375 construction-robotics studies and highlights barriers to real-world deployment. A separate review of 75 field deployments (2016–2025) codes recurring failure factors including environment (45 cases), system integration (44), hardware limitations (39), scalability/cost (15), and human-robot interaction (14). Those are coded case counts, not universal failure probabilities. Together they support evaluating whole-site integration and environmental robustness—not only robot cycle speed. https://doi.org/10.1016/j.rineng.2026.110406 and https://www.sciencedirect.com/science/article/pii/S2666165925002418

The comparison therefore has three distinct levels:

1. **Transport interoperability:** can robots and fleet managers exchange and act on messages? VDA 5050 has a real mixed-fleet logistics precedent; AMRA-271:2025 is a broader communication candidate including peripheral interactions.
2. **Construction semantic interoperability:** do independent implementations preserve work-package, revision, spatial, epoch, capability, resource, handoff and acceptance semantics? That remains the new programme's target.
3. **Outcome interoperability:** can independently inspected, uncertainty-qualified construction output be attributed to a specific task and accepted under a shared profile? Neither transport standard alone supplies that.

The proposed contribution is not to claim we have surpassed commercial robot performance. It is to create a repeatable route to compare heterogeneous systems on the same task, site context, acceptance profile and evidence basis.

## What the standards layering suggests

Treat protocol adapters as replaceable edge connectors, not as the system of record. Keep a canonical construction contract above one or more robot-management transports. Use IFC/IDS for design-information exchange, VDA 5050 and/or AMRA-271 for robot-management communication, OPC UA for device/condition information where appropriate, AMRA-201 and task-specific metrology for robot/product performance, and separately qualified project safety/acceptance processes for permission and output. The exact applicability of each standard must be determined for the robot type, site, environment, jurisdiction and task.

## Frozen cases and tests

The crosswalk profile and its Draft 2020-12 schema are in `profile/` and `schema/`. The synthetic fixture corpus has its own closed Draft 2020-12 schema (`schema/vda5050-crosswalk-cases-v0.1.schema.json`). The focused CI checks JSON parsing, validates both schemas, validates both data files against their schemas, and compares the Rust canonical mapping set to the profile mapping set.

The synthetic corpus in fixtures/vda5050-crosswalk-cases-v0.1.json covers:
1. transport order ID != construction task ID;
2. map version != design revision/site epoch;
3. zone grant != safety permit;
4. action finished != output acceptance;
5. node traversed != work-package complete;
6. disconnect -> state unknown;
7. missing sidecar -> dispatch precondition rejected;
8. stale site epoch -> context binding rejected;
9. mismatched order update -> sidecar binding rejected;
10. unknown extension -> typed unclassified outcome;
11. exact sidecar -> structurally bound only, not authorized;
12. factsheet -> reported capability, not configuration attestation.

Run the Rust tests:

    cargo test --manifest-path research/construction-vda5050-crosswalk-v0-1/Cargo.toml --locked
    cargo clippy --manifest-path research/construction-vda5050-crosswalk-v0-1/Cargo.toml --locked --all-targets -- -D warnings

## Claim ceiling

The profile, corpus and tests are synthetic reference artifacts. They do not prove a real vendor adapter, MQTT reliability, cyber security, authenticated identity, robot capability, physical work quality, metrology, inspection, acceptance, construction-site safety, field reliability, cost reduction, or productivity superiority.

The next external proof step is an independently maintained VDA 5050 v3 adapter, paired with an independently authored construction-side adapter and tested against the same normalized contract. A vendor's own adapter and its own validator do not constitute independent interoperability evidence.
