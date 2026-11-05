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

- **ISO 21423:2026** is a communication/interoperability standard for industrial mobile robots, fleet managers and other machines/applications in its defined shared industrial environment. Its published scope says navigation and safety requirements are excluded, and the scope's restricted-public-access/industrial-production context must be assessed before applying it to a construction site. This profile does not claim ISO conformity.
- **MassRobotics AMR Interoperability** is a useful mobile-robot data-sharing comparator for location, direction, speed, identity and operational state. It is not a fleet-management, navigation or safety system.
- **IFC 4.3 / ISO 16739-1:2024** is the open built-asset information exchange model. **IDS 1.0** can express machine-checkable exchange requirements. This crosswalk does not implement an IFC parser, a BIM clash engine, or IDS validation.
- #3561/#3564 own pre-dispatch design/export/frame/inventory admission; #1607 owns survey/frame/transform evidence; #3542/#3558/#3566 own site epochs, observation coverage and end-to-end assurance; #2005 owns IFC/IDS-to-fabrication semantic-loss benchmarking.
- #3644 owns the protocol-neutral construction task contract and task/resource lifecycle. This child consumes it and tests the VDA projection boundary; it does not duplicate the task state machine or resource calendar.

## Frozen cases and tests

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
