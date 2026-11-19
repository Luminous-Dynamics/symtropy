# Context-Aware Spatial Design and Habitability Benchmark v0.1

**Status:** Research/contract specification for [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766). This document does not claim an existing end-to-end qualified interior-design system.

## Summary

Move from **render-first room generation** to **people-, task-, context- and evidence-first spatial design**. The output is not merely a picture or a universal comfort score; it is a set of candidate layouts with explicit requirements, assumptions, trade-offs, unknowns, performance evidence, and human/professional review gates.

This work extends rather than duplicates:
- [Accessible, resident-led home design #1687](https://github.com/Luminous-Dynamics/symtropy/issues/1687).
- [Faceted spatial topology #455](https://github.com/Luminous-Dynamics/symtropy/issues/455).
- [Exact realized-geometry provider qualification #683](https://github.com/Luminous-Dynamics/symtropy/issues/683).
- [Place/home emergence #449](https://github.com/Luminous-Dynamics/symtropy/issues/449).
- [Portable blueprints #493](https://github.com/Luminous-Dynamics/symtropy/issues/493).
- [Housing evidence framework PR #1596](https://github.com/Luminous-Dynamics/symtropy/pull/1596).
- [Planet-neutral world start #1387](https://github.com/Luminous-Dynamics/symtropy/issues/1387).

## Four-system composition (proposed, not yet integrated)

### Symtropy — spatial structure and deterministic simulation substrate

Own typed geometry/spatial snapshots, facet-specific topology, deterministic scenario execution and resource-accounting interfaces. A room can have distinct body-access, air/pressure, acoustic, visibility, thermal and service-routing relations. A renderer mesh, topology graph, or generic physics simulation is not by itself a building-code or engineering verdict.

### Symthaea — requirement synthesis and alternative analysis

Use resident-stated tasks/preferences and declared constraints to propose variants and expose uncertainty/conflicts. Compare alternatives on the same frozen scenario inputs. The system must be able to abstain when geometry, climate, usage, code or specialist-model evidence is missing. It must not infer a resident's preferences from demographic stereotypes or silently weaken an essential requirement.

### Mycelix — consent, governance and decision provenance

Bind preference/context changes to authorized sources, consent, scope, revision and appeal/correction lineage. Keep resident preference, community decision, legal requirement, professional finding, ownership/title and operational authority distinct. A governance record cannot prove that a layout is physically safe.

### Sol-Atlas — site/context adapter (only through qualified existing interfaces)

Where qualified sources exist, supply versioned location, orientation, solar/environment profile, terrain and time/coordinate-frame references. For off-world contexts, include explicit environment assumptions such as lighting cycles, dust, radiation, thermal extremes and external maintenance conditions. Each field must carry exact source/version, units, epoch, uncertainty, coordinate frame and validity. Do not infer source qualification from a globe, map or visualization.

## Proposed contract family

The concrete Rust types should be reviewed against current owners before implementation; do not create parallel authoritative schemas.

- **DesignContextV1:** site/world ref, use scope, climate/environment ref, time horizon, occupant/task scenario set, applicable jurisdiction/profile, provenance and unknowns.
- **ResidentTaskV1:** resident-chosen task, affected spaces/routes, operating context, success criterion and optional preference; store functional needs rather than unnecessary diagnoses.
- **DesignRequirementV1:** requirement ID, task/space scope, source/authority, priority/mandatory status, evidence state, units if applicable, reviewer role and expiry/review trigger.
- **DesignVariantV1:** immutable variant identity, exact geometry/blueprint refs, parameter bindings, solver/model profile refs, known limitations and provenance.
- **TradeoffVectorV1:** independent outcomes for access, task completion, daylight/glare, thermal comfort, acoustic separation, air quality/ventilation, privacy, maintenance, energy/water/materials, cost and adaptability.
- **DesignEvidenceRefV1:** evidence source identity/version/digest, measured/simulated/authored status, valid-time interval, uncertainty, transformation lineage and qualification scope.
- **DecisionReceiptV1:** compared variants, participating/authorizing roles, selected trade-offs, unresolved dissent, requirements not met and why, review gates and explicit non-certification statement.

Unverifiable requirements remain unknown. Missing data cannot silently become a successful default. Profile selection or image generation can never mark a design verified.

## Paradigm shifts

1. **From style prompts to task-first design.** Start from what residents need and want to do, then determine spatial, circulation, equipment, sensory and maintenance constraints.
2. **From static rooms to adaptive places.** Model use over day/night, seasons, changing household needs, repairs, retrofit constraints and emergency states.
3. **From one room graph to typed spatial facets.** A doorway may have different consequences for body passage, airflow, sound, line of sight, thermal transfer and pressure isolation.
4. **From one score to explicit Pareto trade-offs.** A quiet sleeping area, social kitchen, accessible route and easy-to-maintain service space may compete. Show the trade-off; do not average away blocked tasks or minority needs.
5. **From generated claim to evidence-bearing artifact.** Every statement is labelled as resident preference, proposal, synthetic simulation, measurement, needs-site-measurement, needs-professional-review or verified-for-declared-scope. Promotion requires evidence from the relevant authority.
6. **From smart-home dependence to graceful fallback.** Connected features should document manual controls, power/network failure, maintainability, privacy and who can override or repair them.
7. **From Earth-only interior patterns to domain-specific habitability.** Reuse human-task and governance contracts, but keep off-world pressure, life-support, fire, radiation, thermal and lighting constraints behind dedicated qualified models and responsible safety authority.

## Synthetic benchmark pack

1. Step-free route through a small apartment with declared mobility/reach constraints.
2. Multigenerational shared home with different schedules, privacy and acoustic needs.
3. Low-vision navigation with glare, contrast, lighting and tactile-cue alternatives.
4. Deaf/hard-of-hearing communication and alert modalities with qualified fire/life-safety boundaries.
5. Sensory regulation versus daylight, ventilation and shared-space trade-offs.
6. Rental retrofit that excludes irreversible structural changes.
7. Hot-climate interior orientation and shading with a named building-energy model.
8. Cold-climate daylight, heat-loss and condensation assessment with a named specialist model.
9. Resident-selected cultural kitchen/hospitality pattern without identity inference.
10. Power/network outage and manual fallback for essential home functions.
11. Maintenance-layout comparison that tests access to replaceable equipment and isolation points.
12. Habitat module with separate pressure, lighting, noise/vibration, stowage, privacy and emergency constraints; unqualified until human-rated engineering requirements and physical models are reviewed.

All initial scenarios are synthetic. They test contract behavior, not cultural legitimacy, field usability or physical performance.

## Benchmark targets

Targets below are proposed gates, not achieved measurements.

### Integrity and traceability
- **100%** of recommendations bind a resident task/requirement, affected space, evidence state and review role.
- **100%** of missing/unqualified inputs remain explicitly unknown or blocked.
- **0** promotions to verified status based only on profile, preference, generated geometry or rendering.
- **100%** deterministic replay for identical frozen inputs, algorithm/profile versions and deterministic solver versions.
- **100%** of variants bind exact geometry/profile/model digests; stale or mismatched refs fail closed.
- Incremental topology recomputation must match full rebuild for every invariant the profile claims to preserve.

### Human-centred performance
- Measure each task's completion, time, needed assistance and failure modes by distinct synthetic/co-design cohort; report each cohort and worst-case, not just average.
- Measure accessibility over complete journeys, not isolated rooms or doors.
- Track unsatisfied mandatory tasks and unresolved high-severity requirement conflicts; specified promotion gates block until they are resolved or explicitly dispositioned by the proper authority.
- Report glare/daylight, acoustics, thermal response, ventilation, wayfinding, privacy and maintenance access separately with units and model provenance.
- **100%** of selected resident priorities appear in the export or have an explicit conflict/omission record and next action.
- **100%** of connected-system proposals record manual fallback and power/network-failure handling where applicable.

### Resources and adaptability
- Report energy, water, materials, cost and embodied impacts only when supplied by a named model with appropriate input data and validation scope.
- Track lifecycle cost, maintenance access, repair time/cost, component replacement, reversible options, retrofit feasibility and stranded cost across declared future scenarios.
- Do not report an ecological/health/thermal score unless an appropriate validated domain model provides it.

### Human and professional evaluation
- Accessible-interface audit (including WCAG 2.2 AA where applicable).
- Paid co-design with people having distinct access needs; do not collapse multiple disabilities into one generic cohort.
- Real projects require qualified local review of site dimensions, structure, fire/egress, alarms, ventilation, accessibility and applicable law. The application is requirements and decision support, not certification.

## Execution sequence

1. Map the current accessible-home, spatial-topology, exact geometry, housing evidence and blueprint contracts.
2. Freeze a small DesignContext/Requirement/Variant/Evidence/Tradeoff/Decision receipt schema through review.
3. Build one Rust-first synthetic room + route + resident task fixture.
4. Add independent requirement coverage, geometry currentness, resource ledger and replay checks.
5. Add transparent alternative comparison and conflict reporting.
6. Add qualified solar/climate/site adapters and one specialist solver at a time.
7. Add read-only Symthaea and Mycelix adapters only after exact interface and authority review; connect Sol-Atlas through the same evidence boundary.
8. Evaluate with resident/co-design participants and qualified professionals before real-project use.
9. Only then add space-habitat profiles with their own independently reviewed human-factors and engineering envelopes.

## Research references

- NIST, Building Digitization and Semantic Interoperability (updated 19 February 2026): https://www.nist.gov/programs-projects/building-digitization-and-semantic-interoperability
- *Performance-Driven Generative Design in Buildings: A Systematic Review* (published 17 December 2025; 153 core studies): https://doi.org/10.3390/buildings15244556
- NASA-STD-3001 Volume 2, Version F, dated 14 July 2026: https://standards.nasa.gov/node/237
- NASA, Habitability Design: https://www.nasa.gov/directorates/esdmd/hhp/habitability-design/
- Resident-led accessible home planning and local standards are tracked in #1687 and should remain the governing workstream for resident access requirements.

## Claim ceiling

This spec defines an architecture and benchmark programme. It does not claim that the four systems are already integrated, that generated layouts are buildable/accessibly compliant, or that any physical building/habitat safety has been established.
