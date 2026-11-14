# Power Fabric Human Interaction Program v0.1

**Status:** research specification; not a wiring design, certified product, installation method, or safety approval  
**Date:** 2026-11-14  
**Owner issue:** [POWER-HUMAN-INTERACTION-001 #5158](https://github.com/Luminous-Dynamics/symtropy/issues/5158)  
**Related module contract:** [POWER-FABRIC-MODULE-CONTRACT-001 #5159](https://github.com/Luminous-Dynamics/symtropy/issues/5159)

## 1. Thesis

Electrical infrastructure should be understandable by construction, not by asking people to decode inconsistent labels, undocumented modifications, vendor-specific screens, and dense single-line diagrams.

The design target is **misuse-resistant, observable, explainable, repairable and fail-safe** infrastructure. "Foolproof" is not an absolute guarantee. A screen, QR code, software indication or interlock is never proof that a conductor is de-energized or safe to touch.

The programme separates three things:

1. **Comprehension:** help a person understand what supplies a service, what is backed up, what has failed, and what actions are appropriate to their role.
2. **Mistake prevention:** remove foreseeable wrong choices through typed interfaces, physical keying, touch-safe construction, interlocks and explicit constraints.
3. **Professional assurance:** preserve competent design, installation, inspection, testing, statutory certification and utility approval. Software supports evidence; it does not replace authority.

## 2. Paradigm shifts

### 2.1 Electrical design as a compiler pipeline

Compile declared service intent, module identities, topology, applicable constraints and evidence into a common design artifact:

`intent + topology + modules + constraints + evidence
  -> validated electrical service graph
  -> technical and human-facing views
  -> commissioning evidence package`

A compiler can reject missing ratings, incompatible interfaces, unknown alternate feeds, unsupported operating modes and stale configuration. It must not invent cable ratings, protective-device settings or code compliance.

### 2.2 One versioned as-built graph, multiple role-specific views

The resident, operator, qualified electrical practitioner, maintainer and emergency-response views should all be generated from the same versioned graph and exact configuration digest. A label or diagram that no longer matches the installation is a defect, not a documentation inconvenience.

### 2.3 Energy-path legibility

Explain the *complete possible energy path*, not merely the name of a breaker. Include utility supply, PV, generator, battery, inverter, alternate feeds, AC/DC conversion, isolation boundaries and residual stored-energy hazards. "Main switch off" must never be interpreted as "everything is safe to touch".

### 2.4 Physical constraints before warnings

Text and color are weak barriers when used alone. Prefer mechanically keyed incompatible interfaces, finger-safe dead fronts, touch-safe covers, clear physical identity, engineered access interlocks, and de-energized replacement boundaries where supported by a risk assessment. Use color as one cue among text, shape, location and physical keying.

### 2.5 Local safety without cloud or AI

Protection and safe equipment response must remain local and independent of Symthaea, Mycelix, internet access and supervisory compute. Loss of communications may reduce optimization or visibility; it must not disable protection or create an unsafe permissive state.

## 3. Electrical Service Graph

A typed, versioned graph should describe:

- **Sources:** utility, PV, generator, fuel cell, storage and other declared inputs.
- **Conversion:** MPPT, DC/DC, rectifier, inverter, transformer, bidirectional converter and solid-state transformer.
- **Distribution:** AC/DC buses, boards, feeders, branch circuits, connectors and protective devices.
- **Services:** buildings, rooms, outlets, appliances, EVSE, compute, motors, process loads and critical services.
- **Safety:** isolation points, protective zones, grounding/earthing, alternate feeds and residual-energy hazards.
- **Thermal:** heat sources/sinks, cooling loops, passive heat transfer, thermal stores and auxiliary demand.
- **Observations:** meters, sensors, measurands, units, calibration, sample time, freshness, uncertainty, quality and provenance.
- **Change history:** asset replacement, approved alteration, inspection, test, professional sign-off and handover evidence.

Every graph edge has a typed relation: power, protection, measurement, control, thermal, physical or authority. AC and DC are distinct types. Units and sign conventions are explicit. Any unknown, stale, contradictory or unverified condition remains visible as `UNKNOWN`; it is never silently coerced to safe.

## 4. Power Fabric Design Compiler

### Inputs

- User service requirements: what must be powered, at what service level and under which declared scenarios.
- Exact equipment identity and rating evidence.
- A topology assembled from typed module contracts.
- Installation context and jurisdiction.
- Applicable standards/utility requirements with edition and applicability evidence.
- Assumptions, unresolved questions and the authority responsible for resolving them.

### Outputs

- Canonical machine-readable graph and content digest.
- Human-readable single-line and source-flow diagrams.
- Circuit, equipment and protective-device directory.
- Alternative-source and backup-power map.
- Module compatibility and unknown/blocked-claim report.
- Declared operating modes and backup behaviour.
- Professional commissioning/inspection checklist, with evidence fields.
- Plain-language resident/operator guide.
- Change-impact report identifying every stale derived view after a modification.

The first version is a deterministic documentation and constraint-checking tool. Any calculation that could affect installation, conductor selection, protection or code compliance requires an explicitly applicable and independently validated rule set, exact input evidence and professional review. No generic DIY mains-wiring instructions are in scope.

## 5. Human-facing views

| View | Primary audience | Main questions answered |
|---|---|---|
| Resident | Occupants and ordinary users | What is powered? What is backed up? What does this alarm mean? What user control is safe? When do I call a professional? |
| Operator | Site/building operator | What is the source mix, reserve, current service state, constraints and recovery status? |
| Technical | Registered/qualified practitioner | What is the verified topology, all feeds, ratings, protections, isolation boundaries, test evidence and revision history? |
| Maintenance | Authorized maintainer | Which asset is affected, what is its service boundary, what evidence is required, what changed, and which role is permitted to act? |
| Emergency | Authorized responder | What energy sources and storage remain, where are isolation points, and what residual hazards are declared? |

Design requirements:

- Plain language before abbreviations; explain units and status terms.
- Use text, icons, shape, position and color redundancy. Never depend on color alone.
- High contrast, readable typography and a low-cognitive-load overview, with technical depth available on demand.
- Local-language support, including South African language expansion, without changing the underlying technical meaning.
- Every warning names the observed condition, likely consequence, data freshness/confidence and safe next action.
- Do not instruct an ordinary user to open a panel, expose conductors, test live circuits or alter fixed wiring.
- A software state may say "telemetry reports switch open"; it may not say "safe to touch".

## 6. Physical misuse resistance

Evaluate, do not assume, the suitability of:

- Physically keyed and non-interchangeable connectors.
- Touch-safe, finger-safe enclosures and barriers.
- Engineered interlocks and captive access features.
- Unambiguous, permanent device identity and local disconnect identification.
- De-energized replaceable modules with declared stored-energy and isolation constraints.
- Mechanically robust labels linked to an as-built identity.
- Local fail-safe protection during sensor, network, clock or supervisory-compute failure.

No generic hot-swapping of hazardous circuits. A module is not serviceable merely because it has a plug. Isolation and absence-of-voltage verification remain governed by applicable professional procedures. Interlocks supplement those procedures; they do not replace them.

## 7. Proof-carrying as-built state

Bind the following into a reproducible package:

- Design and graph revision.
- Exact installed module/configuration identities.
- Source and alternate-feed map.
- Label payloads and their generated digest.
- Inspection and test results.
- Instrument identity and calibration evidence.
- Applicable standards/utility edition and scope.
- Registered professional's sign-off and Certificate of Compliance references where applicable.
- Unresolved defects, exclusions and limitations.

A component change invalidates affected documentation and compatibility claims until the as-built graph is reconciled and the appropriate checks are repeated. Software must not issue or imply a CoC or professional certification.

## 8. Benchmarks

Freeze a conventional baseline using normal paper single-lines, circuit schedules, labels and vendor interfaces. Compare the Power Fabric view and physical mock-ups under the same tasks and information.

| Benchmark | Measurement |
|---|---|
| Circuit comprehension | Correct identification of served location/load and protective device |
| Source comprehension | Correct identification of every possible source and alternate feed |
| Backup comprehension | Correct understanding of what remains powered during utility loss |
| Status interpretation | Distinguish normal status, alarm, trip, unknown and stale telemetry |
| Isolation-boundary understanding | Correctly identify what an isolation indication does and does not prove |
| Label integrity | Mismatch rate between physical circuit, label and verified graph |
| Change impact | Fraction of affected views correctly invalidated after controlled changes |
| Compatibility | Wrong-mating / voltage / polarity incompatibility prevention on de-energized fixtures |
| Accessibility | Task success and comprehension across language, visual and experience groups |
| Compiler correctness | Missing/invalid/unknown conditions rejected with stable reason codes |
| Evidence integrity | Claims traceable to exact assets, revisions, tests and calibration records |

### Hard qualification gates

- Zero critical unsafe recommendations in the frozen adversarial scenario suite.
- Zero software-only assertions that equipment is safe to touch.
- Every declared source/alternate feed is shown, or the artifact is explicitly incomplete.
- Zero silent documentation drift after a relevant change.
- Zero conversion of unknown or stale topology into a "safe" state.
- Demonstrate a statistically credible reduction in critical misidentification versus baseline without lowering task completion or excluding accessibility groups.

Set numerical usability and time thresholds after a baseline study and preregistration, not after observing candidate results. Test comprehension with paper/screen scenarios and de-energized mock-ups only.

### Hostile cases

- Incorrect breaker label.
- Hidden PV/battery/generator backfeed.
- Grid failure while local source remains available.
- Stale telemetry or lost communications.
- Changed circuit not reflected in diagrams.
- Replaced inverter with a different operating envelope.
- Contradictory measurements.
- QR code pointing to a superseded configuration.
- Similar-looking connectors with incompatible electrical properties.
- User misreads a status indicator or treats a trip as a repair instruction.
- Color-only status indication under color-vision variation.
- Emergency view omits stored-energy or alternate-source information.

## 9. South African and international standards context

Maintain a jurisdiction- and edition-aware applicability register. Do not bake unverified clauses into code.

- **South Africa:** SANS 10142-1 and the Electrical Installation Regulations, including the competent registered-person, inspection, testing and CoC boundaries. Confirm the exact current SABS edition and amendments before normative rule mapping; public industry notices in 2026 use differing edition notation.
- **IEC 60364-8-82:2022 + AMD1:2026:** prosumer low-voltage installations with local generation/storage and grid-connected, islanded and stand-alone operation.
- Apply product, assembly, protection, wiring, utility-interconnection and workplace-safety requirements only within their actual scope.
- Treat human-factors guidance as design input, not as an alternative to electrical engineering approval.

Research references:

1. Shaping Frontline Practices: A Scoping Review of Human Factors Implicated in Electrical Safety Incidents (2021): https://www.mdpi.com/2313-576X/7/4/76
2. ECA(SA), correct and permanent circuit/device marking and labelling in SANS 10142-1 (7 Oct 2026): https://ecasa.co.za/member-support/wired-to-comply-understanding-the-standard-test-report-part-4/
3. ECA(SA), current-edition availability notice (26 Jun 2026): https://ecasa.co.za/technical/notification-of-the-availability-of-sans-10142-12026-ed-3-03/
4. IEC, IEC 60364-8-82:2022 + AMD1:2026 consolidated publication (16 Apr 2026): https://webstore.iec.ch/en/publication/113148
5. South African Department of Employment and Labour, CoC and registered-contractor reminder (22 Oct 2025): https://www.gov.za/news/media-statements/employment-and-labour-electrical-installation-laws-and-certificate-compliance
6. NFPA 70E public input record on additional electrical safety-by-design methods: https://docinfofiles.nfpa.org/files/AboutTheCodes/70E/70E_A2023_EEW_AAA_FD_PIResponses.pdf

## 10. Integration and ownership

- Symtropy owns deterministic graph validation, energy-flow replay, typed constraints and scenario execution.
- Symthaea consumes verified data for read-only understanding, explanations, anomaly diagnosis, forecasting and human-reviewed proposals.
- Mycelix owns role/authority, provenance and auditable approvals where governance is needed.
- Electrical protection remains local and independent.
- Qualified electrical practitioners and relevant authorities own physical design approval, installation, testing, certification and energization.

## 11. First implementation increments

1. Typed Electrical Service Graph schema and small hostile fixture set.
2. Read-only graph validator with stable error/unknown reason codes.
3. Two synthetic reference systems: AC-only building and grid + PV + battery hybrid.
4. Generate single-line, source map, circuit directory and resident guide from one frozen graph digest.
5. Mutation tests for hidden source, wrong label, stale revision, missing calibration and incompatible module.
6. De-energized human-comprehension mock-up and preregistered comparison against conventional documentation.
7. Independent review by South African registered electrical practitioners and human-factors specialists before applying rules to actual installations.

**Definition of done:** a non-expert can understand system sources, backup boundaries, common status meanings and the point at which to stop and call a professional without touching hazardous equipment. A registered practitioner can trace the same versioned graph through actual equipment, all possible feeds, isolation boundaries, tests and evidence. Both views remain bound to the exact as-built revision.
