# Payload-First Cargo and Human Mission Contract v0.1

**Status:** synthetic-only contract and deterministic decision-support audit. It is not a vehicle qualification, route clearance, patient-care protocol, permit, purchase recommendation or deployment authorization.

## The design shift: define the payload and mission before the carrier

Payload rating alone is an inadequate design criterion. A nominal 200 kg capacity says nothing about whether a package fits, is restrained, remains upright, survives transmitted shock/vibration, stays in a required temperature range, or keeps the combined load centre of mass inside the stability envelope. Multiple individually compatible packages can still overlap or exceed the loading bay when carried together.

People must never be modeled as ordinary cargo. Seated passengers require suitable restraints and egress; wheelchair users additionally require accessible boarding/transfer and securement; stretcher patients require appropriate support, caregiver access, restraint and a protected transfer/egress plan. Human factors must be integrated upstream, not retrofitted after the vehicle layout is fixed.

A review of ambulance human factors found important gaps in standards coverage and in knowledge about occupant and contents behavior during transport. NIST's patient-compartment design work likewise highlights occupant restraint, ergonomics, reach, equipment use and communication. These findings motivate explicit human-transport gates; they do not establish compliance for a novel off-road or robotic platform. Sources: [PubMed review](https://pubmed.ncbi.nlm.nih.gov/32678770/) and [NIST patient-compartment design workshop](https://www.nist.gov/publications/workshop-report-ambulance-patient-compartment-design).

## Mission classes covered

- **Bulk/medical supplies:** mass, dimensions, securement, load arrangement, ingress and recovery.
- **Cold-chain cargo:** a permissible temperature interval, a controlled compartment whose declared operating range is inside the payload's allowed range, securement, privacy where applicable, and temperature monitoring in a later evidence-linked version.
- **Fragile scientific instruments:** shock, vibration, tilt, environmental and securement limits.
- **Contained liquids/samples:** spill containment, temperature/tilt bounds and safe handoff.
- **Seated passengers:** occupant restraints, cabin/thermal range, vibration, egress and human supervision.
- **Wheelchair passenger:** the passenger requirements plus verified boarding/accessibility and wheelchair restraint/securement.
- **Stretcher patient:** the person-transport gates plus patient-support interface, caregiver access, egress and more restrictive ride/tilt envelopes.
- **Field crew:** declared occupancy/mass envelope, restraints, environment, accessibility as required, and egress.

These are abstract contract categories, not clinical values or permission to transport people in a particular vehicle.

## Candidate architecture, not one universal chassis

The best mission chain may combine existing modes: a shallow-draft vessel for the river leg, a protected cold-chain module at a landing site, and a smaller last-mile carrier on an approved trail. A polar resupply mission may prefer a simple sled behind an already-authorized carrier over a more autonomous crawler. A desert field team may need a crew carrier while instruments ride in a shock-isolated module. A patient transfer is a different vehicle configuration from unoccupied cargo delivery.

Intermodal design needs explicit handoff contracts. Each handoff should declare the interface, mass/geometry, securing method, transfer duration, temperature/condition control continuity, responsible operator, and recovery method. This v0.1 auditor checks one candidate carrier leg per assessment; route-chain and handoff optimization are a deliberate next version rather than falsely implied functionality.

ISO 668 defines dimensions and ratings for Series 1 freight containers for intercontinental transport. It is a useful example of why interface standards matter, but it is not a universal dimensional standard for small robotic platforms or passenger modules. Select standards by system and mission scope: [ISO 668:2020](https://www.iso.org/standard/76912.html).

## What is validated by the contract

### 1. Payload envelope

Each payload has a bounded mass range, dimensions, a center-of-mass fraction inside its own package envelope, optional allowed temperature range, maximum shock/tilt/vibration tolerance, containment and securement requirements, and explicit occupant flags. Human profiles must satisfy occupant-class consistency: a wheelchair profile requires accessible boarding and wheelchair securement; a stretcher patient requires patient support and caregiver access; all occupants require a restraint and emergency-egress declaration.

### 2. Platform envelope

Each platform declares supported environments and payload classes, maximum mass, cargo-bay dimensions, ambient operating range, optional controlled-compartment temperature range, transmitted shock/vibration limits, route-tilt capability, combined centre-of-mass bounds, securement/containment, occupant/accessibility/patient features, privacy, recovery and emergency stop.

### 2b. Route mode is a mission constraint

A mission declares which route modes are admissible for the tested leg. A platform that carries the payload but cannot traverse the declared mode/route is excluded. This v0.1 benchmark evaluates one carrier leg at a time; it must not disguise an intermodal trip as a single-platform mission.

### 3. Loading layout

Every payload must be placed exactly once using non-negative millimetre coordinates. In v0.1, package boxes are axis-aligned and may not overlap; rotations, deformable containers and sophisticated 3-D packing are not inferred. Each box must fit the bay and the upper bounds of the combined mass must remain within platform capacity.

For the combined centre of mass, the audit checks every min/max mass-endpoint combination across the declared payloads (up to twelve in v0.1). The centre of mass is computed from package positions and package-relative centre-of-mass fractions and must remain inside the platform's declared normalized bounds. The synthetic baseline currently yields 9 candidates eligible for human review, 8 blocked by failed hard gates, 2 blocked by platform/mode mismatch, and 2 needing gate evidence. Those counts are fixture checks, not evidence that nine real platforms are suitable. This is a bounded screening calculation, not a substitute for physical stability, acceleration, dynamic load transfer or occupant crash testing.

### 4. Feasibility gates

Assessment disposition precedence is fail-closed:

- `blocked_hard_gate`: a mandatory route, environmental, securement, patient/access or human-safety gate failed.
- `blocked_platform_mismatch`: at least one payload/platform/load-layout condition fails, even if headline capacity is nominally sufficient.
- `needs_gate_evidence`: no known hard failure, but a required gate remains unknown.
- `not_assessable_incomplete_metrics`: no known hard failure/unknown gate, but one or more required metric intervals are unknown.
- `eligible_for_human_review`: the contract is internally complete, its declared gates pass, the payload/layout fits the synthetic platform envelope, and metrics are complete.

The final state is **not** authorization to move, carry cargo or patients, or enter any route. No output authorizes autonomous hazard handling. Afghanistan examples always require explicit explosive-hazard route evidence; unknown route status remains a hard stop.

### 5. Pareto comparison

Eligible candidate carriers for the same mission are compared using five separate interval metrics: mission duration, energy per mission, handoff time, a synthetic payload-exposure proxy and recovery time. The report exposes only a conservative robust Pareto set. It creates no weighted utility score, no hidden global winner and no unsupported probability. Metrics and all numeric fixture values are synthetic.

## Benchmark targets

- **Contract completeness:** every mission gate, metric and payload placement occurs exactly once; missing/duplicate/undeclared IDs reject input.
- **Human transport:** zero passenger/patient assessments admitted without declared restraints/egress; wheelchair users need accessibility and wheelchair securement; stretcher patients need support and caregiver access.
- **Fit and loading:** zero packages outside the bay, pairwise box overlaps, over-capacity loads, incompatible temperature intervals, shock/vibration over-limit candidates or centre-of-mass envelope violations admitted as eligible.
- **Safety gates:** zero failed/unknown mandatory gates treated as passed; unknown route/explosive-hazard evidence never grants authorization.
- **Mass-bound robustness:** combined centre-of-mass bounds are checked across all endpoint combinations for the versioned payload-count limit.
- **Oracle correctness:** all 243 combinations of five mandatory gate states match the independent reference disposition rule.
- **Determinism:** byte-identical reports after input-order permutation.
- **No false certainty:** zero weighted universal score, deployment authorization, patient-care authorization or autonomous hazardous-object authorization emitted.
- **Field benchmark (future):** record actual load/unload and transfer times, cargo damage and temperature excursion, occupant vibration/comfort, restraint/egress performance, recovery success, repair time and energy by payload class and environment. Report samples and confidence bounds per mission type; do not pool unlike modes and climates into a single advertised success rate.

## Research anchors

- [Human factors and ergonomics in ambulance design](https://pubmed.ncbi.nlm.nih.gov/32678770/).
- [NIST: Workshop Report on Ambulance Patient Compartment Design](https://www.nist.gov/publications/workshop-report-ambulance-patient-compartment-design).
- [ISO 668:2020 freight container dimensions/ratings](https://www.iso.org/standard/76912.html).
- [NASA/JPL ERNEST desert field test (June 2026)](https://science.nasa.gov/photojournal/desert-field-test-with-nasa-advanced-rover-prototype/) as an example of environmental-analogue testing, not a qualification claim for our platform.

## Known v0.1 limits

- One carrier leg per assessment; a versioned multimodal route graph and transfer-continuity accounting are not implemented yet.
- Box packing is axis-aligned with no rotations or flexible-load geometry.
- The centre-of-mass calculation is a static bounded screening model; dynamic stability, wheel/track loads, braking/turning acceleration and occupant restraint loads require appropriate engineering analysis and physical tests.
- Synthetic gate status and capability values are test data. Real deployment requires source-linked, site/season-specific evidence and applicable local review, permits, engineering validation and trained operators.

## Next phase

Implement a multimodal transfer graph with load continuity and timed handoffs; add orientation-aware packing; model liquid slosh and isolator mounts for sensitive cargo; test ride-vibration and occupant restraints on instrumented rigs; and use supervised, permitted analogue tests to replace synthetic bounds with measured, configuration-linked evidence.
