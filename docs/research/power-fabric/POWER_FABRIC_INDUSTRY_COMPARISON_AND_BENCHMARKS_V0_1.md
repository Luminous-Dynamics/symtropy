# Power Fabric Industry Comparison and Benchmark Protocol v0.1

**Snapshot date:** 2026-11-14  
**Status:** public-source capability scan and proposed matched benchmark; not a product certification or performance comparison  
**Programme:** [Power Fabric human interaction #5158](https://github.com/Luminous-Dynamics/symtropy/issues/5158), [module contract #5159](https://github.com/Luminous-Dynamics/symtropy/issues/5159)  
**Draft documentation PR:** [#5162](https://github.com/Luminous-Dynamics/symtropy/pull/5162)

## 1. Why this comparison exists

Luminous should not claim to revolutionize electrical systems merely by adding a dashboard, smart breaker, digital twin, AI, or modular battery. Those capabilities already exist in parts of the industry.

The defensible question is:

> Under the same installation, equipment, data quality, scenario, authority boundaries and user task, does the Power Fabric help people and qualified practitioners understand, operate, change and restore the electrical system more accurately, safely and efficiently—with complete evidence and no weaker protection?

This document is a public-source baseline. It does not claim to know competitors' proprietary internal designs or every feature of their current products.

## 2. Industry capability snapshot

### Residential smart panels — SPAN

SPAN publicly describes:
- circuit-level control and energy monitoring;
- solar and battery flow visibility;
- configurable backup priorities;
- estimated remaining backup availability;
- an on-premise web application for local monitoring/control when internet access is disrupted.

Crucially, SPAN's own app guidance says software control of a circuit is **not** a suitable OFF position for routine service or maintenance. This is an important safety distinction and a good human-interface precedent.

Public product updates relevant to the comparison:
- SPAN's 25 August 2026 app update adds breaker-level energy cost analysis for supported panel models: https://www.span.io/blog/a-new-span-home-app-experience
- SPAN Home On-Premise is described as a local Wi-Fi/web access path for monitoring and circuit/backup-priority control; the support article was updated 12 November 2026: https://support.span.io/hc/en-us/articles/38584072525207-SPAN-Home-On-Premise
- SPAN's guidance explicitly warns that software circuit control is not an appropriate OFF position for routine service: https://support.span.io/hc/en-us/articles/4411214235159-Navigating-the-SPAN-Home-App

Sources:
- SPAN Home App: https://www.span.io/app
- SPAN 2026 app update: https://www.span.io/blog/a-new-span-home-app-experience
- SPAN Home On-Premise: https://support.span.io/hc/en-us/articles/38584072525207-SPAN-Home-On-Premise
- SPAN app navigation and service caution: https://support.span.io/hc/en-us/articles/4411214235159-Navigating-the-SPAN-Home-App

### Commercial/industrial electrical digital twins — Schneider Electric / EcoStruxure

Schneider Electric publicly describes:
- electrical network model management and data integration;
- distribution analysis, planning and simulation;
- connecting monitoring systems with electrical models;
- evaluating operational changes before implementation;
- digital-twin training/operations use cases and connected-services support.

These are strong industry baselines. The Power Fabric must not claim a general digital-twin novelty. Its candidate contribution is a tighter, vendor-neutral contract between installed module identity, typed AC/DC/thermal service ports, evidence validity, physical configuration, role-specific human views and change invalidation.

Sources:
- Grid digital-twin guide: https://www.se.com/ww/en/work/campaign/grid-digital-twins-guide/
- Electrical digital twins, 24 Jun 2026: https://blog.se.com/energy-management-energy-efficiency/2026/06/24/electrical-digital-twins-turning-data-into-decisions/
- South Africa digital services: https://www.se.com/za/en/work/services/digital-services/

### Engineering digital-twin and interoperability baselines — ETAP and IEC

**ETAP Digital Twin** publicly describes a unified engineering and real-time platform used to model, design, visualize, analyze, predict, control and provide insight on electrical power-system management and performance. Power Fabric must therefore not claim that an electrical digital twin, graph model or engineering-to-operations platform is itself novel.

Sources:
- ETAP Digital Twin fact sheet: https://etap.com/docs/default-source/brochures/fact-sheets/digital-twin-fact-sheet-web.pdf

The interoperability standards landscape also already provides important pieces:
- **IEC 81346-1:2022 / ISO 81346-10:2022** provide general and power-supply-specific structuring and reference-designation rules. ISO shows an edition-2 Working Draft under development; do not represent the draft as a published edition.
- **IEC 61970-301:2020 + AMD1:2022** provides the CIM base model for power-system network objects.
- **IEC 61968-11:2013** provides CIM distribution extensions.
- **IEC 61850-6:2009 + AMD1:2018 + AMD2:2024** defines SCL for exchanging IED and substation configuration descriptions between engineering tools.
- **IEC TR 61850-1-1:2026** updates the series overview and includes namespace concepts.
- **IEC 62351** is a multi-part power-system communication security series; its parts have different versions and scope.

Sources:
- IEC 81346-1:2022: https://webstore.iec.ch/en/publication/64021
- ISO 81346-10:2022: https://www.iso.org/standard/75471.html
- ISO/WD 81346-10 (edition 2 working draft): https://www.iso.org/standard/93711.html
- IEC 61970-301: https://webstore.iec.ch/en/publication/62698
- IEC 61968-11: https://webstore.iec.ch/en/publication/6199
- IEC 61850-6 consolidated version: https://webstore.iec.ch/en/publication/103863
- IEC TR 61850-1-1:2026: https://webstore.iec.ch/en/publication/105222
- IEC 62351 series catalogue: https://webstore.iec.ch/en/publication/6912

**Power Fabric implication:** the defensible research contribution is not a new universal electrical ontology. It is an evidence-bound composition layer that connects typed module compatibility, versioned as-built graph, provenance, loss-aware adapters to existing standards, human-readable projections and change invalidation. Standards-specific semantics must remain in their own versioned mapping profiles; successful serialization is not conformance or safety.

### Existing electrical installation safety practice — South Africa

SANS 10142-1 requires correct and permanent marking/identification in its inspection/report process. ECA(SA)'s October 2026 technical explainer emphasizes that an incorrect label can be more dangerous than no label because a person may isolate the wrong circuit and believe the intended equipment is isolated. ECA's November 2026 series also highlights that electrical verification includes measured earth continuity, not merely the visible presence of an earth conductor.

This is a concrete opening for an as-built graph whose labels are derived from a verified asset/circuit relationship and invalidated when the installation changes.

**Standards edition note:** ECA(SA) published a notice for a SANS 10142-1 Edition 3.3 webinar on 2 September 2026. This is evidence that Edition 3.3 was being discussed by the local industry association; it is not a substitute for confirming the exact current SABS publication, amendments, effective status and applicable clauses before encoding normative rules. The PR should not retain an earlier Edition 3.03 assumption as a definitive current-standard claim.

Sources:
- ECA(SA), correct and permanent marking/labelling: https://ecasa.co.za/member-support/wired-to-comply-understanding-the-standard-test-report-part-4/
- ECA(SA), earth continuity verification (10 Nov 2026): https://ecasa.co.za/member-support/wired-to-comply-understanding-the-standard-test-report-part-6/
- ECA(SA), SANS 10142-1 Edition 3.3 webinar notice (2 Sep 2026): https://ecasa.co.za/member-support/missed-the-sans-10142-1-edition-3-3-webinar-catch-up-with-the-recording/

### Human factors and maintainability

The UK HSE's maintenance-error guidance notes that slips and omissions occur even among experienced, trained and motivated technicians. Its principles include designing for maintainability, reducing complexity, improving access and labelling, conducting human-error analysis on safety-critical tasks, involving maintenance personnel in design, and making schematics/job aids available.

The electrical-safety scoping review similarly argues that incidents emerge from individual, environmental, team and organizational factors; they should not be treated solely as frontline worker mistakes.

Sources:
- HSE, Maintenance error: https://www.hse.gov.uk/humanfactors/topics/error.htm
- Shaping Frontline Practices review: https://www.mdpi.com/2313-576X/7/4/76

## 3. What is already industry-grade, and what Luminous should add

| Capability | Existing market baseline | Power Fabric research target |
|---|---|---|
| Circuit visibility | Smart panels and metering apps | Source-to-service graph with exact installed identity and measurement boundary |
| Backup prioritization | SPAN and other energy-management products | Preserve rights/service floors, expose uncertainty and compare delivered service under matched scenarios |
| Local control | Some smart-panel on-premise interfaces | Local visibility and user controls with clear authority limits; protection remains independent |
| Electrical digital twin | Schneider/EcoStruxure, ETAP and other engineering platforms | Versioned as-built graph that binds topology, module configuration, evidence and human-facing views |
| Documentation | Single-lines, circuit schedules, labels and CoC records | All views generated from one content-addressed graph; changes invalidate affected outputs |
| Module integration | Vendor datasheets and engineered interface projects | Typed AC/DC/thermal/measurement/control/protection ports with fail-closed evidence-bound compatibility |
| Safety labels | Standards-based marking and installation inspection | Verified label-to-asset traceability, drift detection and plain-language source/backup explanation |
| Maintenance | Procedures, CMMS and vendor-specific diagnostics | Evidence-linked service boundaries, residual-hazard explanation, replacement/change impact and repair closure |
| AI/analytics | Predictive monitoring and decision support | Read-only diagnosis/proposals with independent physical checks and no protection authority |

This is a proposed research advantage, not a claim that no competitor has any equivalent feature. Public product documentation is incomplete as a view of proprietary capabilities.

### Solid-state protection benchmark update

Solid-state DC breakers are not maintenance-free by definition. A December 2025 study reports more than three million opening cycles on a 1.2 kV low-voltage DC solid-state breaker, while identifying five failure modes and three operational issues spanning electrical design, mechanical structure, thermal management and cybersecurity. This is useful component-level durability evidence, not proof of field reliability for every breaker or installation.

A June 2026 IEEE paper presents a prototype hybrid DC breaker/current-limiting branch. Its reported prototype clamps 4.5 kA for 100 microseconds and interrupts within 180 microseconds under its tested conditions. These results are research evidence for one prototype and test envelope, not a universal DC protection rating.

Design implication: benchmark fault interruption, selectivity, on-state losses, surge/thermal stress, false trips, physical isolation, cyber failure and maintenance under identical declared test profiles. Do not simply score opening speed.

Sources:
- Liu et al., *Failure modes and remedies of a solid-state low voltage DC circuit breaker through 3,000,000+ operation cycles* (Dec 2025): https://doi.org/10.1016/j.pedc.2025.100119
- IEEE, *A Novel Hybrid Circuit Breaker Architecture With Built-in Current Limiting Capability – I: Semiconductor Design* (June 2026): https://ieeexplore.ieee.org/document/11316411/

## 4. Differentiation hypothesis

Power Fabric should attempt to unify six capabilities that are commonly split across engineering, installation, monitoring and maintenance workflows:

1. **One exact as-built service graph:** not a dashboard-specific reconstruction or a stale PDF.
2. **Typed, evidence-bound module contracts:** capability and interoperability claims depend on the exact module, revision, mode, limits and supporting evidence.
3. **Automatic view consistency:** resident, operator, installer, maintenance and emergency views are projections of one verified graph.
4. **Physical and digital error resistance:** keyed interfaces and touch-safe construction where appropriate; software flags unknown or stale states; physical protection remains independent.
5. **Human comprehension as a benchmark:** test whether people correctly understand sources, backup boundaries, faults and allowed actions—not just whether the software can display more telemetry.
6. **Change-aware lifecycle evidence:** every replacement or alteration propagates into the compatibility graph, labels, diagrams, maintenance records and required re-verification.

This is stronger than "AI for electricity" or "a smart panel". It is a candidate **electrical design-to-operation compiler and evidence fabric**.

## 5. Matched evaluation protocol

### Comparison arms

1. Conventional installation package: single-line diagram, circuit schedule, permanent labels, applicable CoC/test documentation and normal operator instructions.
2. Commercial residential smart-panel workflow: a SPAN configuration with app/on-premise behavior, if a comparable permitted test installation or simulator is available.
3. Commercial/industrial electrical digital-twin workflow: a comparable Schneider/EcoStruxure/ETAP or other documented engineering-and-monitoring workflow, where available.
4. Power Fabric candidate: same physical/reference topology, measurements, configuration and user authority, with the graph, module contract and generated views.

Do not compare a fully instrumented candidate with a less instrumented baseline, or give one arm better topology information, a shorter task, additional assistance or a looser safety boundary.

### Participant cohorts

- residents/ordinary users for comprehension and safe user-facing tasks;
- building/facility operators for status, backup and outage decisions;
- apprentices for training/comprehension tasks;
- registered/qualified electrical practitioners for technical traceability, change review and evidence tasks;
- accessibility cohorts covering language, reading level, color vision and relevant visual/motor differences.

Use paper/screen scenarios and de-energized mock-ups. Do not expose participants to live electrical hazards or ask unqualified participants to perform electrical work.

### Core metrics

- correct circuit and served-load identification;
- fraction of all possible sources/backfeeds correctly identified;
- correct explanation of what remains powered during loss of utility;
- false-safe interpretation rate (hard gate);
- incorrect device/circuit identification;
- time to answer a task, with accuracy retained;
- confidence calibration (confidence should fall when data are stale/unknown);
- stale-label/as-built mismatch detection;
- task completion and accessibility by cohort;
- time for a qualified practitioner to trace asset identity, topology, protection references and evidence;
- number of unresolved assumptions and undocumented changes;
- amount of technical information that is inaccessible or ambiguous to the user.

### Initial aspirational targets

- **Zero false-safe statements** in the frozen hostile suite: software must not state that equipment is safe to touch based only on telemetry or a control state.
- **Zero hidden declared feeds:** every declared alternate source must appear, otherwise the output must be marked incomplete.
- **Zero silent drift:** a relevant topology/configuration change invalidates affected views until reconciled.
- **At least 95% correct** on defined non-hazardous resident comprehension tasks, reported separately by cohort and language.
- **At least 30% lower median task time** for selected routine identification/explanation tasks versus the conventional baseline, without worse critical error or accessibility outcomes.
- **Zero false accepts** in the frozen critical module-incompatibility suite.
- All numeric targets must be preregistered and reviewed; these are programme targets, not current results or standards thresholds.

## 6. Electrical safety principle

A user-facing "OFF" state must distinguish at least:
- a software request to turn a load off;
- a reported switch/relay state;
- a verified local device indication;
- a professionally established isolation boundary;
- absence-of-voltage verification under the applicable work procedure.

These states are not interchangeable. The UI should not compress them into a green "safe" badge.

A protective device trip is not a diagnosis or a repair instruction. Explain that a protection event occurred, show the evidence and affected service, and direct ordinary users to the safe role-appropriate next step. The interface must never encourage repeated resetting of an unresolved fault or access to exposed conductors.

## 7. Competitive claim ceiling

Do not claim:
- "the first digital twin for electricity";
- "the first smart panel";
- "fully foolproof electrical work";
- "no maintenance" because the design is solid-state;
- improved safety because the UI has a green status;
- better efficiency without a matched, measured delivered-service boundary;
- interoperability because two connectors physically mate;
- standards compliance from a model or software-generated checklist.

A credible claim requires a frozen comparison design, exact system boundary, comparable configuration, data rights, independent evaluation and an explicit evidence tier.

## 8. Decision rule

Power Fabric should be considered better than the relevant industry baseline only if it improves at least one user or engineering outcome—comprehension, critical error rate, system availability, repair time, lifecycle cost, delivered efficiency or evidence completeness—without degrading other hard safety, service or accessibility constraints.

If a conventional product is better for a particular layer, use it. The Power Fabric's value is the trustworthy composition and lifecycle interface, not owning every electrical component.
