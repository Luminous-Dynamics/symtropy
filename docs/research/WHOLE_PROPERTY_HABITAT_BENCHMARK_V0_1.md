# Whole-Property Habitat Systems Benchmark v0.1

**Status:** Research contract / benchmark proposal. Documentation only.  
**Primary execution owner:** Symtropy, for deterministic scenario execution and spatial/resource comparisons.  
**Cross-project composition:** Symthaea, Mycelix, Sol-Atlas.  
**This document is not:** a CAD engine, licensed design service, building-code checker, structural/fire/MEP solver, safe-room design, construction release, or evidence that any physical building is safe.

## 1. Objective

Test whether the four-project stack can help professionals design, compare, realize, furnish, landscape, commission, and maintain a high-performance residence as one coupled system—not merely generate attractive room images.

The first market hypothesis is a premium, privacy-sensitive residence with sophisticated automation, landscape integration, extensive amenities, and an optional refuge. The engineering abstractions must still work for ordinary homes and accessible retrofit projects; “ultra-luxury” is a client segment, not a separate physics domain.

**Core paradigm shift:** optimize for residents' validated capabilities and long-lived service outcomes across the whole property, while keeping every claim scoped to evidence and professional authority.

The unit of design is:

`site + ecology + building + interiors + furnishings + utilities + automation + operations + maintenance`

These are coupled but must not be collapsed into one opaque score. A beautiful render, a valid model, an accepted construction package, a commissioned building, and a well-performing occupied building are distinct outcomes.

## 2. Scope relative to existing work

This is a **whole-property benchmark and integration profile**, not a replacement for existing owners:

- Symtropy [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766) and [benchmark PR #1767](https://github.com/Luminous-Dynamics/symtropy/pull/1767) own context-aware spatial-design and habitability evaluation. This document extends the evaluation outward to site, landscape, building services, realization, and operation; it must reuse their resident/task and geometry contracts.
- Symtropy's [accessible-home task benchmark PR #1696](https://github.com/Luminous-Dynamics/symtropy/pull/1696), [accessible-home portfolio #1740](https://github.com/Luminous-Dynamics/symtropy/issues/1740), faceted topology #455, and exact realized-geometry qualification #683 remain the owners for their respective capabilities.
- Symthaea [BUILT-ENV-001 #6032](https://github.com/Luminous-Dynamics/symthaea/issues/6032) owns the evidence-bound building/factory/site-infrastructure realization profile; [SWA-001 #6624](https://github.com/Luminous-Dynamics/symthaea/issues/6624) composes building digital twins and spatial intelligence; [ROOM-QUAL-002 #7801](https://github.com/Luminous-Dynamics/symthaea/issues/7801) covers change robustness, uncertainty, and spatial co-design.
- Sol-Atlas [site suitability PR #129](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/129), [source provenance PR #136](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/136), [GIS constraint semantics PR #140](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/140), and [built-environment evidence graph PR #138](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/138) own spatial evidence, conservative site gating, and revision-bound approval lineage.
- Mycelix [construction provenance issue #5149](https://github.com/Luminous-Dynamics/mycelix/issues/5149) owns the coordination/provenance proposal for cross-organization design changes, materials, delivery, inspection, and approvals.

The first implementation must **compose these contracts** rather than introduce a second construction ontology, GIS, solver registry, approval ledger, or universal home optimizer.

## 3. Proposed responsibility split

| Project / party | Owns | Explicitly does not prove |
|---|---|---|
| **Sol-Atlas** | Versioned site/context evidence; location, terrain, mapped constraints, coverage/freshness, spatial alternatives, and traceable source references | Ground truth, legal title, geotechnical suitability, permit approval, or the safety of a building |
| **Symtropy** | Deterministic geometry/topology scenarios, resource accounting, candidate comparison, replay, and bounded synthetic or qualified model execution | Real-world validity from determinism; building-code compliance from geometry; a physical result from a synthetic scenario |
| **Symthaea** | Requirements elicitation support, design alternatives, uncertainty/conflict explanation, change-impact reasoning, and recommendations for next useful evidence | Professional authority, unmeasured facts, guaranteed safety, or autonomous approval |
| **Mycelix** | Consent/rights, role and approval provenance, exact artifact/revision references, supplier/material/inspection lineage and disputes | Scientific truth, structural integrity, product qualification, or validity from a hash alone |
| **Qualified design team** | Architecture and discipline-specific engineering; applicable-code interpretation; review, sign/seal where required, permits, construction administration, commissioning acceptance | Nothing is delegated away by software output |
| **Builder / commissioning / operator** | Installed-as-built evidence, inspection, functional testing, handover, maintenance and operational measurement | That a design-only model matches the as-built property without reconciliation |

Specialist models (e.g. energy, daylight, structural, geotechnical, hydrology, fire, ventilation, acoustics, security, ecology) require their own named engines, versioned inputs, validity scope, applicable standards, and independent verification. An adapter existing is not proof that the engine is qualified for the site or design.

## 4. Lifecycle evidence thread

Every stage should produce a revision-bound artifact and explicit disposition:

1. **Brief and resident tasks:** intended occupants, activities, comfort/accessibility priorities, privacy boundaries, service expectations, budget/risk tolerance, maintenance capabilities, and known preferences. Do not infer sensitive traits from surveillance or demographic proxies.
2. **Site and ecology:** parcel/source identity, rights to use source data, jurisdiction, hazards, terrain, water, soil/geotechnical evidence, climate, solar/orientation, vegetation, drainage, access, fire services, utility connections, ecological constraints, and data coverage. Critical unknowns remain unresolved.
3. **Concept alternatives:** multiple candidate site/building orientations, massing, rooms, circulation, furnishings, landscape, daylight/shade, access, plant rooms, service routes, and future adaptation paths.
4. **Engineering analysis:** named and scope-qualified specialist analyses for the questions they can actually answer. Preserve model assumptions, input revisions, uncertainty, convergence, output ranges, and independent reference evidence.
5. **Trade-offs and approval:** hard constraints are gates; trade-offs are shown as separate outcomes and Pareto alternatives. Record who is authorized to decide, what evidence they reviewed, dissent/conditions, and the exact artifact digest.
6. **Open information handoff:** IFC/openBIM deliverables and machine-readable information requirements where supported; quantities, schedules, specifications, and issue/change references are reconciled rather than silently approximated.
7. **Procurement and construction:** bind approved revisions to product/material batches, permitted substitutions, delivery records, inspection findings, and installation evidence. A substitute changes the design/approval lineage and triggers applicable revalidation.
8. **Commissioning and handover:** compare intended, installed, measured, and accepted values separately; record unresolved deficiencies, operational limits, training, service access, manuals, spares, and emergency procedures.
9. **Operations and adaptation:** update only from consented, correctly scoped observations. Compare measured energy/water/comfort/service outcomes against a baseline; investigate drift; preserve manual fallback and resident control.
10. **Repair, retrofit, and end of life:** track replaceability, repair routes, maintenance and outage impacts, reversible changes, reuse/recovery options, and how changes affect previously accepted analyses.

A revision that changes geometry, an engineering assumption, a product, a source condition, or an applicable rule must invalidate every dependent conclusion that the dependency map says is affected. Old approvals remain historical evidence, not current authorization.

## 5. Product profiles: never use one “bunker score”

Use separately declared profiles with distinct hazard scopes:

- **HOME-BASE:** ordinary residential functions, comfort, accessibility, resource performance, privacy, maintainability, and lifecycle adaptability.
- **RESILIENCE-REFUGE:** a named set of scenarios such as heatwave, smoke/poor air, grid outage, water interruption, flood, or communications loss. Report each scenario and each essential service separately.
- **STORM-SHELTER:** only for the hazard class and jurisdiction supported by applicable shelter requirements. In the United States, FEMA P-361 and ANSI/ICC 500-2023 are relevant references for tornado/hurricane safe rooms and storm shelters; they are not generic certification for other threats.
- **SPECIALIST-HARDENED-REFUGE:** blast, radiation, CBRN, ballistic/security, or other specialized claims require a separate client threat model, qualified specialist design and testing, legal review, and evidence beyond this benchmark. They must not inherit a storm-shelter pass or be generated from generic prompts.

“Bunker” is an informal client label, not an engineering acceptance class. The system must ask which hazard and duration are in scope, whom the space must support, what services must remain available, and which qualified authority will approve the requirements. Unknown hazard, occupancy, structural, fire/egress, ventilation, sanitation, or utility assumptions block any safety claim.

For all profiles, explicitly model access/egress, fire and life safety, ventilation/air quality, moisture, water/sanitation, thermal conditions, power, communication, service access, accessible task completion, maintenance, and degraded-mode procedures where applicable. Do not treat more automation, greater isolation, or a visually impressive reinforced room as proof of safety.

## 6. Synthetic scenario corpus v0.1

The first corpus should be deterministic and wholly synthetic. Every case defines inputs, hard constraints, uncertainty, model scope, expected gate state, and an independent oracle or explicitly authored expected result. Include both positive examples and adversarial negative controls.

| ID | Scenario | Required behavior / negative control |
|---|---|---|
| WP-001 | Parcel with partial mapped coverage and unknown subsurface conditions | Do not turn unmapped/unknown into “safe”; block dependent foundation/permit conclusions |
| WP-002 | Flood/drainage constraints conflict with a preferred floor level or landscape concept | Preserve confirmed hard constraints and unresolved evidence separately; no soft score may override the hard gate |
| WP-003 | Hot-climate house with shading, daylight, and outdoor-space trade-offs | Compare named-model outcomes and uncertainty; no climate claim from render geometry alone |
| WP-004 | Drought-limited landscape with irrigation, storage, and household demand | Reconcile explicit water sources, volumes, time horizons, losses, and uncertainty; no double-counted supply |
| WP-005 | Heatwave plus grid outage | Report time-bounded essential service floors and uncertainty; do not label nominal battery capacity as delivered service |
| WP-006 | Water interruption and sanitation dependency failure | Detect serial bottlenecks and missing dependencies; unknown treatment/source quality must not become potable supply |
| WP-007 | Network/cloud outage in a highly automated residence | Essential features have explicit local/manual fallback, observable state, and authorized override; remote telemetry is not required for safe basic use |
| WP-008 | Furnishings and storage change a formerly feasible circulation/egress layout | Recompute affected topology and tasks; stale design approval cannot carry through a changed layout |
| WP-009 | Landscape option improves shade but conflicts with water, roots, fire, ecology, or maintenance constraints | Use separately sourced specialist/ecological results; expose trade-offs instead of optimizing a single “green” score |
| WP-010 | Maintenance task requires replacing a pump, filter, battery, valve, or control module | Test access, isolation, replacement envelope, tools/spares assumptions, and downtime; modularity is only credited if the operation is feasible |
| WP-011 | Supplier proposes a different material/product after approval | Bind the substitution to the exact revision and trigger relevant reanalysis, review, and procurement checks |
| WP-012 | As-built geometry/equipment differs from the approved design | Mark dependent receipts stale; commissioning cannot close an unexamined material deviation |
| WP-013 | Resident priorities conflict across separate household members | Keep preferences and authorized decision rights distinct; don't silently average away a person's essential requirement |
| WP-014 | Candidate storm refuge with a missing required egress, ventilation, or fire/shelter qualification input | Fail closed; generic “bunker” language does not grant a shelter pass |
| WP-015 | Specialist hardened-refuge request is outside the model's validated hazard scope | Return out-of-scope / needs-qualified-review; do not extrapolate from storm-shelter or general resilience results |
| WP-016 | Project cost, schedule, embodied impacts, or supplier availability have sparse/uncertain inputs | Preserve ranges, scenarios, and unknowns; don't impute unknown cost/carbon/time as zero |
| WP-017 | A small exhaustive design space with known constraints and outcomes | Compare the production candidate selector against a simple independent exhaustive oracle |
| WP-018 | Frozen design passes initially, then one high-impact source/model/geometry revision occurs | Recompute every declared dependent result and preserve unrelated results only where dependency analysis proves safe reuse |
| WP-019 | Compromised third-party cloud credentials attempt unauthorized subsystem changes before a network disconnect | Preserve independent authorization boundaries, local/manual fallback for essential functions, and explicit non-claim of penetration-test coverage |

Later versions can add climate zones, building typologies, landscaping/ecoregions, furnishing products, construction methods, and real measured cases only after rights, privacy, source validity, and reviewer authority have been established.

## 7. Benchmark protocol and proposed acceptance targets

These are **proposed gates, not current results**. Establish a baseline and freeze the corpus before comparing algorithms. The reference evaluator must be independent of the production code under test; no fixture should derive its expected answer by calling the same helper that generates the answer.

### Integrity and claim discipline

- **100%** of results bind the exact scenario manifest, candidate/geometry revision, source-evidence revisions, model/solver versions, units, assumptions, and applicable review scope.
- **0** stale, mismatched, revoked, or unresolved critical evidence items are promoted to a positive approval state.
- **100%** of seeded hard-constraint failures are rejected or explicitly marked unresolved by the expected status; **0 false-safe outputs** are allowed in the frozen synthetic negative controls.
- **100%** of missing critical inputs produce a visible blocked/unknown/out-of-scope result rather than a default value.
- **100%** of output status distinctions survive export/import and replay for fields the export contract declares lossless.

### Correctness and robustness

- **100%** deterministic semantic replay for identical frozen inputs and pinned deterministic execution profiles; if byte-identical outputs are part of the contract, assert bytes as well.
- For small cases, exact Pareto-frontier and constraint outcomes must match a deliberately simple exhaustive reference implementation.
- Input permutation must not change results where the contract declares order irrelevant.
- Incremental recalculation must match full recomputation for all tested dependency invariants; no stale dependent receipt may survive a relevant design change.
- Report performance over a preregistered scenario sweep and held-out scenarios. A robust-unique-winner claim is permitted only under the declared robust policy; otherwise return a Pareto set, ties, or indeterminate result.

### Physical/resource modelling

- Energy, water, thermal, structural, acoustic, daylight, hydrology, ecological, and life-safety metrics remain separate and carry units, model provenance, assumptions, uncertainty, and validation scope.
- For each conservation model, define the unit-specific residual/tolerance before testing; fail if mass, energy, or water accounting exceeds that declared tolerance. No universal tolerance is invented for unlike solvers.
- A simulated service-capacity or autonomy figure must identify demand profile, time horizon, dependencies, degradation/efficiency assumptions, uncertainty bounds, and which inputs are rated, measured, or hypothetical.
- Simulation benchmarks establish model behavior on those scenarios only. Physical accuracy needs measured/validated reference cases and independent review.

### Human, operational, and lifecycle performance

- Task success, assistance required, time, access route continuity, privacy, comfort and maintenance burden are reported per task/cohort/scenario and as distributions—not only one whole-home average.
- **100%** of mandatory resident priorities appear in the delivered brief or have a visible, authorized conflict/omission disposition.
- **100%** of connected essential services document manual/local fallback and power/network failure behavior where applicable.
- **100%** of approved physical substitutions and material deviations are linked to their affected revalidation/review outcome.
- Compare simulated predictions to measured commissioning/operational outcomes only with predeclared calibration windows, metrics and acceptance tolerances. Do not tune on the held-out evaluation set.
- Run performance benchmarks at 100, 1,000 and 10,000 candidate/scenario records when the executable exists; report per-run p50/p95/p99, memory, machine/toolchain, corpus digest, and verifier result. Runtime targets are set only after an honest baseline.

### No universal utility score

Hard safety/legal/accessibility requirements remain feasibility gates. Cost, aesthetics, energy, water, carbon, comfort, convenience, privacy, repairability, ecological outcomes, and future adaptability should remain separately visible. User-authorized weights may organize optional preferences, but they cannot buy their way out of a hard constraint, and a score cannot establish cultural fit or beauty.

## 8. Interoperability anchors

Use open standards as **exchange contracts**, not as proof of correctness:

- [IFC 4.3 / ISO 16739-1:2024](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/content/scope.htm) for open information exchange across building lifecycle disciplines.
- [buildingSMART IDS 1.0](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/) for machine-readable information requirements and checking delivered IFC model properties. IDS does not replace geometric, physical, legal, or safety analysis.
- [ASHRAE Standard 223](https://data.ashrae.org/standard223/standard223.html) as a semantic model for building automation/system data, analytics, commissioning and digital-twin integration.
- [NIST Building Digitization and Semantic Interoperability](https://www.nist.gov/programs-projects/building-digitization-and-semantic-interoperability) for lifecycle digital-twin and interoperable building-data motivation.
- [NIST, Security and Trust Considerations for Digital Twin Technology / IR 8356](https://www.nist.gov/news-events/news/2025/02/security-and-trust-considerations-digital-twin-technology-nist-releases-ir) for the cyber-physical trust boundary; the benchmark's synthetic cyber case is not a penetration test or assurance certification.
- [U.S. DOE Building Energy Modeling](https://www.energy.gov/cmei/buildings/about-building-energy-modeling) for the role of validated energy modeling in design/operations.
- [FEMA P-361](https://www.fema.gov/sites/default/files/documents/fema_p-361_safe-rooms-for-tornadoes-and-hurricanes_122024.pdf) and [ANSI/ICC 500-2023](https://codes.iccsafe.org/content/ICC5002023P1/american-national-standard) only for their applicable storm-safe-room/shelter scopes.

Verify the adopted editions and jurisdictional requirements at project start; the references above are research anchors, not a statement that any target property complies with them.

## 9. Execution plan

1. Review this contract against the existing owners named above; resolve fields to existing types before freezing any schema.
2. Use the frozen WP-001–WP-019 manifest and the standalone `tools/habitat-manifest-verifier` for strict structure and exact-byte integrity only. The independent semantic evaluator/golden oracle remains future work; it must verify expected dispositions and hard-gate behavior without sharing the candidate evaluator's decision code. Start with evidence-state correctness, change invalidation, and tiny exhaustive comparison before high-fidelity simulation.
3. Implement the first Sol-Atlas read-only adapter for site, coverage, freshness and hard-constraint receipts only after PRs #129/#136/#140 provide a reviewed interface.
4. Compose Symtropy's existing decision-impact/robustness work (#1769, #1789) to prioritize surveys and model runs by their ability to change feasibility or the Pareto set. Do not invent probabilities or expected value-of-information without calibrated observation models. Then compose Symthaea's design-alternative and uncertainty/reporting interfaces; do not give the model direct building-control or approval authority.
5. Add one specialist simulation at a time (initial candidate: whole-building energy/comfort, then water balance), each with an authoritative reference set and explicit validity envelope.
6. Link Mycelix-approved material/procurement/inspection artifacts to exact design revisions and revalidation requirements through the existing construction provenance proposal.
7. Add IFC/IDS exchange checks and building automation semantics only after the exported model and runtime data can be independently validated.
8. Progress from synthetic fixtures to measured lab and occupied-building evidence; preserve held-out buildings/climates and report performance drift.
9. Require qualified licensed/local professionals for design, permitting, construction, commissioning and any claimed refuge/shelter capability.

## Terrain-native subprofile

The separate [Terrain-Native Habitats benchmark v0.1](TERRAIN_NATIVE_HABITATS_BENCHMARK_V0_1.md) extends this whole-property contract to ravines, canyon floors and walls, mountain slopes/ridges, impact craters, volcanic craters/calderas, karst depressions, and partly embedded or underground variants. Its machine-readable 14-case corpus is `docs/research/fixtures/TERRAIN_NATIVE_HABITAT_SCENARIOS_V0_1.json`.

This profile treats landform names as investigation prompts, not suitability evidence. In particular, a cliff does not prove rockfall protection, crater geometry does not establish volcanic origin, canyon beauty does not establish flood safety, and an underground layout does not establish safe egress or survivability. It composes existing Sol-Atlas coverage/constraint receipts with qualified geotechnical, hydrologic, volcanic, ecological, and life-safety evidence where applicable. The benchmark is synthetic only and does not qualify a real site or solver.

## 10. Claim ceiling

A successful synthetic benchmark would show that the implementation handles its declared fixtures and invariants. It would **not** show that Symthaea can independently architect a real house, that Symtropy's simulator predicts a physical property accurately, that Sol-Atlas has found a legally buildable/safe parcel, that Mycelix provenance makes a certificate true, or that any design is permit-ready.

The long-term product goal is a professional-grade **evidence-to-as-built-to-operations workflow**: generate useful alternatives, find the decisive unknowns early, reduce redesign and coordination waste, make trade-offs inspectable, preserve resident control, and qualify each real-world claim with the evidence and authority it actually requires.
