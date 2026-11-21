# Orbital Structural Interface Conformance — Protocol v0.1

**Status:** research protocol draft  
**Tracking issue:** [ORBITAL-STRUCTURAL-INTERFACE-CONFORMANCE-001 #6523](https://github.com/Luminous-Dynamics/symtropy/issues/6523)  
**Claim ceiling:** synthetic profile compatibility only. No physical structural capability, qualification, flight readiness, or certification is established.

## 1. Decision to be made

Given a structural-interface provider profile and a client requirement profile, determine whether the exact provider configuration can satisfy the client's complete declared requirements for a named project interface-control document (ICD), including:

- identity and geometric mating;
- assembly access and task envelope;
- structural load-transfer and stiffness requirements;
- environmental and lifecycle applicability;
- inspection, verification, removal, and replacement;
- supporting evidence scope, currentness, uncertainty, and authority.

The result is one of:

- `COMPATIBLE_SYNTHETIC_PROFILE`
- `INCOMPATIBLE`
- `INDETERMINATE`
- `MALFORMED_OR_STALE`

A synthetic compatibility result is never a physical acceptance or permission to assemble.

## 2. Why structural interfaces are a separate profile family

The current ISAM nominal compatibility work in #6329 / PR #6336 addresses prepared mechanical capture/retention and storable-fluid transfer. The robust compatibility work in #6351 / PR #6372 handles conservative capability composition under supported uncertainty semantics.

This profile family addresses **primary structural joints**. It is not interchangeable with a capture interface: a structural joint transfers declared forces and moments through a load path, has stiffness/deformation behaviour, and may require inspection and service access that are unrelated to simple mating or retention.

It composes:
- #6329 / #6351 for generic profile identity, envelope compatibility, and robust-decision semantics;
- #5818 for robotic grasp, approach, insertion, verification, release and task-feasibility references;
- #4263 for as-built structure, joint inspection, structural response and acceptance;
- #2361 / #3820 / #2303 / #4190 for bounded structural mechanics, support semantics, external solver qualification and response comparison;
- Symthaea #5686 / #5074 / #6224 / #6225 for process, material state, process–structure–property lineage and uncertainty;
- Mycelix #5149 for authorized custody, evidence and provenance references;
- Sol-Atlas #405 / #6167 for read-only facility, geography and ISAM capability context.

No owner is duplicated by this protocol.

## 3. Authority and truth boundaries

| Activity | Owner / source | This protocol may do |
|---|---|---|
| Candidate design reasoning | Symthaea #12319 | Consume exact design/configuration references |
| Structural model admission and execution | Symtropy #5376 and existing solver owners | Consume admitted model/profile results; never create mechanics |
| Robot assembly-task capability | Symtropy #5818 | Check exact declared task-envelope references |
| As-built structural acceptance | Symtropy #4263 | Consume separate geometric, joint, load and inspection dispositions |
| Material/process facts | Symthaea #5686 / #5074 / #6224 | Reference exact process/material evidence |
| Measurement and uncertainty | Existing measurement/UQ owners | Consume scoped calibration and uncertainty records |
| Authority and custody | Mycelix #5149 | Reference authority/provenance findings |
| Facility and logistics context | Sol-Atlas #405 | Consume spatial context without turning it into hardware approval |

A valid digest is not proof of a physical fact. A supplier declaration is not an independently tested capability. A process exit code is not solver convergence. A robot's success signal is not proof of complete joint engagement. A digital record is not a structural sign-off.

Human engineering authorities retain all design-release, test-release, flight, crew-safety and certification decisions.

## 4. Profile model

The v0.1 profile is a reference envelope, not a new universal ontology. Each field must carry a namespace/version and reference the canonical owner whenever one exists.

### 4.1 Shared identity

Both provider and client profiles bind:

- profile ID, schema version, producer, role and exact hardware/configuration revision;
- project ICD ID/revision and any explicit, independently reviewed compatibility mapping;
- member, joint, fastener/retention and mating-feature identity;
- nominal geometry/configuration digest;
- datum/reference frame, axis/sign conventions and SI-unit semantics;
- effectivity, service state, supersession and claim ceiling.

Different frame identities are indeterminate unless an exact transform and its evidence are supplied. No inferred transform, default axis or friendly-name match is permitted.

### 4.2 Installation and service

Reference the existing robotics/task owner for:

- approach direction/cone and allowable relative pose;
- gripper/fixture/tool compatibility and access clearance;
- insertion path, force/torque limits and locking action;
- independent engagement/retention verification;
- removal, isolation, replacement and reinstallation;
- inspection surface access, datum/fiducial visibility and post-repair checks.

This profile does not assert that a robot with a compatible grasp can safely perform the task. It checks declared task-envelope compatibility only.

### 4.3 Structural behaviour

The provider profile may reference admitted structural capability evidence for:

- exact declared load cases and combined force/moment vectors;
- support and boundary-condition identities;
- qualified load-interaction model or test envelope;
- stiffness/compliance and permitted displacement/rotation;
- slip, retention, interface gap and preload where applicable;
- temperature/environment applicability;
- lifetime/cycle envelope only where a separate evidence owner supports it.

**Do not reduce combined loading to independent per-axis maximums unless the admitted physical model explicitly justifies that representation.** Component-wise checks can accept a joint that fails under a combined axial/shear/bending/torsion case.

v0.1 does not define a universal six-degree-of-freedom strength model. Unsupported interaction laws remain indeterminate.

### 4.4 Manufactured state and evidence

Reference—not copy—the canonical records for:

- exact material specification, condition, lot and batch;
- process definition, executed process history, orientation and post-processing;
- realized structure/microstructure and conditioned property evidence;
- specimen/article correspondence and scale-transfer claim;
- measurement methods, calibration, uncertainty and common-cause dependencies;
- solver/model build, applicability, mesh/convergence where relevant;
- inspection results, raw-data digests, validity/effectivity, revocation and supersession.

Coupon evidence is not automatically component evidence. Component evidence is not automatically joint evidence. Joint evidence is not automatically assembled-structure evidence.

## 5. Compatibility decision rules

The evaluator must keep the following dispositions separate:

1. Identity and ICD.
2. Geometry and mating tolerances.
3. Installation/task envelope.
4. Structural capability for every required load combination and condition.
5. Stiffness/deformation.
6. Environment and lifecycle scope.
7. Inspection/verification feasibility.
8. Disassembly/replacement/service.
9. Evidence completeness and currentness.
10. Authority scope, where an approval assertion is relevant.

Overall compatibility is non-compensatory. A hard failure in a critical dimension cannot be offset by a favourable mass, cost, insertion-time or stiffness result.

The evaluator must reject or abstain when:
- the exact configuration/ICD is stale or contradictory;
- a provider envelope only partially covers the client's hard requirement;
- material/process state differs from the evidence subject;
- the required combined-load model is absent or out of domain;
- calibration or common-cause evidence is missing where required;
- robot installation is feasible but independent engagement verification is not;
- an inspection-critical region cannot be observed;
- claimed fatigue, fracture, buckling, creep, thermal stress, radiation or nonlinear-contact capability is outside the admitted profile.

Uncertainty types must retain their meanings. Hard bounds, standard uncertainty, expanded uncertainty, probabilistic intervals and model prediction intervals are not interchangeable. If the existing robust adapter does not support a particular representation and decision rule, return `INDETERMINATE`; do not improvise a conversion.

## 6. Frozen synthetic benchmark

Freeze at least 40 deterministic cases, with a separately implemented reference oracle and expected reason code for each case. The case set must include positive, incompatible, indeterminate, malformed/stale and metamorphic cases.

| ID | Challenge | Expected disposition |
|---|---|---|
| SI-001 | Exact provider/client identities, ICD and all declared requirements match | Compatible synthetic |
| SI-002 | Same connector name, incompatible ICD revision, no mapping | Malformed/stale |
| SI-003 | Similar CAD envelope but different mating feature identity | Incompatible |
| SI-004 | Frame, sign convention or unit mismatch | Indeterminate or malformed |
| SI-005 | Nominal fit passes but worst-case tolerance stack fails | Incompatible |
| SI-006 | Provider alignment envelope only partly covers client requirement | Incompatible |
| SI-007 | Robot can insert but cannot independently verify locking | Indeterminate |
| SI-008 | Partial engagement gives a plausible completion signal | Incompatible |
| SI-009 | Missing or wrong retention component | Incompatible |
| SI-010 | Combined axial/shear/bending/torsion exceeds qualified envelope | Incompatible |
| SI-011 | All individual load components pass; coupled interaction fails | Incompatible |
| SI-012 | Strength passes but stiffness/deflection requirement fails | Incompatible |
| SI-013 | Stiffness passes but slip/retention requirement fails | Incompatible |
| SI-014 | Material name matches; temper/orientation/cure differs | Indeterminate |
| SI-015 | Supplier assertion substituted for scoped test/certificate evidence | Indeterminate |
| SI-016 | Coupon property is linked to wrong article/process lineage | Malformed/stale |
| SI-017 | Changed process/batch inherits old capability profile | Malformed/stale |
| SI-018 | Environment lies outside admitted profile | Incompatible or indeterminate |
| SI-019 | Thermal mismatch changes alignment/preload beyond evidence | Indeterminate |
| SI-020 | Inspection-critical joint surface is occluded | Indeterminate |
| SI-021 | Calibration is stale or common-mode with reference measurement | Indeterminate |
| SI-022 | Uncertainty overlaps a hard limit; no accepted decision rule | Indeterminate |
| SI-023 | Nominal intervals overlap but do not fully contain requirement | Incompatible |
| SI-024 | Replacement geometry fits but material/process capability is unresolved | Indeterminate |
| SI-025 | Replacement is possible but safe isolation/re-verification is missing | Indeterminate |
| SI-026 | Torque number supplied without required measurement/process context | Indeterminate |
| SI-027 | Fatigue capability inferred from static coupon strength | Indeterminate |
| SI-028 | Nonlinear/contact request routed to a linear admitted profile | Malformed or out of domain |
| SI-029 | Stale design/geometry/model/source revision | Malformed/stale |
| SI-030 | Signed provenance is treated as structural acceptance | Indeterminate |
| SI-031 | Solver exits zero but convergence or request identity is absent | Indeterminate |
| SI-032 | Correlated measurements are presented as independent | Indeterminate |
| SI-033 | Adapter silently changes geometry/load interpretation | Malformed/stale |
| SI-034 | Required load combination omitted | Indeterminate |
| SI-035 | Client temperature/cycle range exceeds provider scope | Incompatible or indeterminate |
| SI-036 | Joint requires unavailable tool/fixture/access envelope | Incompatible |
| SI-037 | Repair restores dimensions but not stiffness/retention evidence | Indeterminate |
| SI-038 | Component passes but assembled load path remains unresolved | Indeterminate |
| SI-039 | Equivalent rigid transform with explicit transform evidence | Invariant disposition |
| SI-040 | Unit-conversion and input-order permutation; digest mutation negative control | Invariant / malformed as appropriate |

The final corpus partition and expected reason codes must be frozen before comparisons. Hold out provider/client profile pairs and entire connector families for generalization tests.

## 7. Baselines

All baselines receive the same requirements, provider/client declarations, evidence availability and time budget.

- **B0 Manual ICD/drawing review:** competent human reviewers; record labor-hours, elapsed time, unresolved items and rework.
- **B1 Name/standard-label matching:** deliberately weak control.
- **B2 Connector-family/profile-ID matching:** tests reliance on labels and nominal identities.
- **B3 Independent scalar min/max checks:** deliberately weak control that omits coupled load interactions.
- **B4 Nominal conformance:** existing #6329 evaluator, only on the semantics it supports.
- **B5 Robust conformance:** existing #6351 evaluator, only when the required hard-bound and lifecycle evidence exists.
- **Candidate:** structural specialization with exact owners and reason-coded dispositions.

The aim is not to outperform a FEM solver. It is to reduce false-compatible pairings, integration work and downstream rework without weakening the decision boundary.

## 8. Metrics and proposed targets

Report a vector, never one opaque score:

- false-compatible, false-incompatible and indeterminate rates, with denominators and confidence intervals;
- hard-negative rejection by mechanism;
- missing/stale/out-of-domain evidence detection;
- median and p95 decision latency;
- engineering labor-hours and elapsed integration time;
- number of bespoke adapters, tools and fixtures required;
- first-pass physical fit and independently verified engagement (future physical stage);
- critical-joint inspection coverage and repair/replacement effort (future physical stage);
- stale artifact/configuration rejection and evidence traceability.

Proposed targets (not results):
- zero false-compatible results on the frozen hard-negative synthetic set;
- 100% expected reason-code agreement on the frozen corpus;
- 100% stale identity and unsupported-domain negative controls rejected or left indeterminate;
- at least 20% lower median engineering integration hours than matched manual/bespoke ICD integration on held-out pairs, with no increase in false-compatible rate.

Publish sample sizes, uncertainty, excluded cases and all failed runs. The 20% labour reduction is a hypothesis to test, not a claim of superiority.

## 9. External comparison and standards applicability

Reference only standards within their actual scope. Project/authority applicability must be determined separately.

| Reference | Relevant use | Boundary |
|---|---|---|
| NASA-STD-7009B | M&S credibility and context of use | Does not certify a structural interface |
| NASA-HDBK-7009B (3 Feb 2026) | Implementation guidance for M&S credibility | Guidance, not a solver validation result |
| NASA-STD-5001B Change 3 | Spaceflight structural design/test and service-life factors | Not a universal connector interchangeability standard |
| NASA-STD-5002B (10 Mar 2026) | Spacecraft/payload load-analysis practices | Not an interface ICD |
| NASA-STD-6016C Change 1 | Spacecraft materials and processes | Material process compliance alone does not establish component strength |
| NASA-STD-7001C (14 Jul 2026) | Payload vibroacoustic test criteria | Not a structural-joint compatibility standard |
| NASA-STD-7002B Change 1 | Payload test programme requirements | Applicability depends on hardware/programme |
| IDSS/IDD, ISO 24330:2022, ANSI/AIAA S-157-2025 | Relevant docking, RPO/OOS or prepared-fluid scopes | Not a universal primary structural joint standard |

Primary sources:
- [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009)
- [NASA-HDBK-7009B](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009)
- [NASA-STD-5001](https://standards.nasa.gov/standard/NASA/NASA-STD-5001)
- [NASA-STD-5002B](https://standards.nasa.gov/standard/NASA/NASA-STD-5002)
- [NASA-STD-6016](https://standards.nasa.gov/standard/nasa/nasa-std-6016)
- [NASA-STD-7001C](https://standards.nasa.gov/standard/nasa/nasa-std-7001)
- [NASA-STD-7002](https://standards.nasa.gov/standard/NASA/NASA-STD-7002)
- [NASA ISAM](https://www.nasa.gov/isam/)
- [DARPA NOM4D/ASTRA announcement, 5 Nov 2026](https://www.darpa.mil/news/2026/nom4d-program-demonstrates-automated-in-space-assembly)
- [Caltech ASTRA announcement, 5 Nov 2026](https://www.caltech.edu/about/news/caltech-robotic-space-demo-successfully-builds-in-space)

NASA's ISAM capability inventory and DARPA/Caltech's bounded ASTRA demonstration are comparison context, not an end-to-end interoperability leaderboard. The public ASTRA announcement does not provide all raw geometry, preload, strain and modal data necessary to reproduce a quantitative structural acceptance comparison; missing telemetry must remain unavailable.

## 10. Advancement gates

- **P0 — protocol:** approve vocabulary, owner mapping, profile identity, dispositions and claim ceiling.
- **P1 — synthetic semantics:** freeze schema, corpus and independent evaluator; reproduce all expected dispositions.
- **P2 — reuse:** build a thin Rust adapter over #6329/#6351 and #5818 references; no duplicated solver or generic engine.
- **P3 — evidence composition:** consume exact manufacturing/material/as-built/measurement references.
- **P4 — physical ground campaign:** only after the separately reviewed equipment, fixture, measurement, environmental and safety plans are accepted; begin with benign, uncrewed connector coupons and a non-flight structure.
- **P5 — held-out interoperability:** independently test cross-provider and cross-family pairs; publish NRE, fit, inspection and rework comparisons.

Passing P1 does not promote to P2 physical evidence. Coupon evidence does not automatically transfer to full members or assembled structures. Any scale transfer must be explicit and supported.

## 11. Non-goals

v0.1 does not implement or claim:
- a new structural solver, FEM/FEA engine, or six-axis constitutive law;
- a new robot planner, controller, actuation path or task scheduler;
- flight, orbital or crewed assembly authority;
- fatigue, fracture, buckling, creep, nonlinear contact or radiation qualification;
- universal connector interchangeability;
- material or process qualification;
- airworthiness, structural certification or commercial superiority.

## 12. Review questions

1. Are the structural interface capability and client-requirement fields sufficient without duplicating the canonical units, material, process, uncertainty, robot or evidence models?
2. Does the decision preserve coupled-load interaction rather than accepting component-wise scalar checks?
3. Are installation verification, inspection access and replacement correctly treated as independent hard dimensions?
4. Are all synthetic cases independently oracle-checkable and are expected dispositions justified?
5. Does the proposed 20% integration-hours target have a fair, preregistered manual baseline and held-out profiles?
6. Are all claims kept within the stated synthetic/ground-test/flight evidence boundaries?


## 13. Industry and research comparator map

The field already has mature commercial lifecycle and discipline tools. The programme must be evaluated as an **open conformance/evidence-composition layer**, not as an attempt to replace established CAD, PLM, CAE, MES, metrology or certification systems.

| Ecosystem | Documented strength | Fair comparison boundary |
|---|---|---|
| Siemens Teamcenter / NX / Simcenter | Product lifecycle/configuration management, requirements and engineering linkage, simulation/test data management, model-based design/manufacturing and sustainment continuity | Compare exact cross-tool identity, semantics, evidence scope and change-propagation across exported interfaces; do not claim to replace NX/Simcenter/Teamcenter analysis or PLM functionality |
| Dassault 3DEXPERIENCE / CATIA / SIMULIA / DELMIA / ENOVIA | Integrated design, simulation, manufacturing operations and PLM/product-data continuity for aerospace | Compare vendor-neutral profile exchange, independent evidence checking and held-out provider/client interoperability; do not claim superiority in CATIA/Abaqus/DELMIA-native workflows |
| Ansys Twin Builder / MBSE and solver portfolio | Multiphysics engineering analysis, reduced-order/system models, co-simulation and simulation-based digital twins | Compare reproducibility, model applicability and exact artifact linkage; do not claim to beat domain solvers on their native physics |
| Hexagon metrology / PC-DMIS | Coordinate measurement, dimensional inspection and CAD-to-CMM traceability; PC-DMIS 2026.2 added specialised-geometry inspection and strengthened STEP/QIF digital-thread workflows | Compare measurement evidence semantics, calibration/currentness, datum and feature correspondence, and independent as-built acceptance; do not recreate the CMM/metrology engine |
| NIST model-based enterprise / digital-thread work | Open standards and conformance research across product definition, manufacturing, inspection and spatial data | Reuse STEP/PMI/QIF and relevant standards; test semantic loss, GUID continuity and inspection-to-requirement traceability instead of inventing another incompatible exchange format |

Primary comparator sources:
- Siemens aerospace PLM and lifecycle continuity: https://blogs.sw.siemens.com/teamcenter/aerospace-and-defense-plm/
- Siemens simulation process and data management: https://www.siemens.com/en-us/products/teamcenter/solutions/simulation-process-data-management-spdm/
- Dassault 3DEXPERIENCE product lifecycle portfolio: https://www.3ds.com/technologies
- Dassault aerospace design/analysis context: https://www.3ds.com/products/simulia/aerospace-defense/commercial-aircraft-design
- Ansys MBSE: https://www.ansys.com/applications/mbse
- Ansys Twin Builder: https://www.ansys.com/products/digital-twin/ansys-twin-builder
- Hexagon PC-DMIS 2026.2 (29 October 2026): https://hexagon.com/about/news-media/media-alerts/2026/pc-dmis
- NIST Extended Digital Thread: https://www.nist.gov/programs-projects/extended-digital-thread
- NIST Digital Thread for Manufacturing: https://www.nist.gov/programs-projects/digital-thread-manufacturing
- NIST design/manufacturing/inspection integration study: https://www.nist.gov/publications/defining-requirements-integrating-information-between-design-manufacturing-and

NIST reports that one standards-based experiment could not fully link all required information across design, manufacturing and inspection. Its current research programmes continue to target interoperability gaps, product/analysis/spatial-data integration and conformance testing. This supports the relevance of the problem; it does not prove that every commercial deployment is fragmented or defective.

### Required fair-comparison experiment

Where accessible, test the same neutral structural-interface package through:
1. native PLM/CAE/metrology workflow;
2. standards-based exchange (e.g., STEP product definition/PMI and QIF inspection information where applicable);
3. the proposed structural-interface contract and adapter.

Compare:
- identity and configuration preservation across export/import;
- geometry, datum, unit, feature and load-case semantic preservation;
- evidence links, authority scope, validity and supersession preservation;
- detection of stale geometry, process/batch, inspection and solver revisions;
- round-trip information loss and unresolved mappings;
- engineering labor-hours, elapsed time and manual mapping steps;
- false-compatible, false-incompatible and indeterminate results on the same frozen corpus.

Do not infer that a vendor tool failed because an unsupported export omitted data. Record the exact product/version, module, format, configuration, mapping, permissions and source evidence. A commercial system may keep semantics internally that a neutral export does not carry; test both the native workflow and the exchange boundary when possible.

**Competitive objective:** deliver the best open, reproducible cross-vendor structural-interface conformance suite and evidence handoff—not a universal replacement for established engineering software.


## 14. Cross-scale manufacturing and structural-evidence transfer

### Why this is a separate gate

This protocol decides whether a provider/client structural-interface profile is compatible. It must not silently infer that the evidence supporting the provider profile transfers to every target scale or manufactured instance.

Compose these existing owners rather than copying their records or semantics:

- Symthaea [MFG-FOUNDATION-001I #5763](https://github.com/Luminous-Dynamics/symthaea/issues/5763): research-to-manufacturing transfer assessment;
- Symthaea [MAT-PSPP-001 #6224](https://github.com/Luminous-Dynamics/symthaea/issues/6224) and [ENG-MAT-001 #5074](https://github.com/Luminous-Dynamics/symthaea/issues/5074): process → realized structure → conditioned property → engineering performance and exact material-state applicability;
- Symthaea [MFG-PROC-000 #5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686): process definition, capability, recipe, process plan and executed-process distinction;
- Symthaea [MAT-013 #4344](https://github.com/Luminous-Dynamics/symthaea/issues/4344): physical sample, process and characterization lineage;
- Symtropy [AERO-INSPECTION-OBSERVABILITY-001 #4165](https://github.com/Luminous-Dynamics/symtropy/issues/4165): defect- and method-specific inspectability evidence;
- Symtropy [ORBITAL-TRUSS-ACCEPTANCE-001 #4263](https://github.com/Luminous-Dynamics/symtropy/issues/4263): as-built geometry, joint inspection and assembled-structure response;
- Mycelix [construction/manufacturing provenance #5149](https://github.com/Luminous-Dynamics/mycelix/issues/5149): source references, authority, custody, revocation, dispute and evidence lineage.

### Paradigm shift: qualify the transfer, not just the source result

The required chain is:

`research result -> process window -> executed process -> realized material state -> specimen/coupon -> subcomponent -> joint -> component -> assembled load path -> service state`

This is a set of separately justified transfer edges, not a staircase in which evidence automatically becomes stronger with scale. A result may support a coupon-level property and still be indeterminate for a joint because of geometry, residual stress, fastener/bond details, defects, boundary conditions, manufacturing variability or inspection coverage.

Every transfer edge must be explicit and profile-relative. A minimal derived adapter should reference canonical source records and declare:

- source subject identity, source scale and exact configuration/process/material revisions;
- target subject identity, target scale and intended engineering claim;
- transfer method: direct test, validated model, similarity argument, conservative bound or other named method;
- changed variables between source and target, including geometry, scale, process, lot, orientation, thermal history, surface state, joining method, environment and load path;
- assumptions, applicability domain and excluded mechanisms;
- measurement and characterization methods, calibration, uncertainty and common-cause dependencies;
- supporting and contradicting evidence, including failed scale-up and negative results;
- effectivity, expiry, supersession, change triggers and review authority references.

The adapter is a derived assessment over canonical references. It does not copy manufacturing recipes, raw measurements, supplier claims, material lots, work orders or custody events into a second ledger.

### Transfer dispositions

Use distinct, non-authorizing states:

- `NO_TRANSFER_EVIDENCE`: no admissible bridge evidence exists;
- `INCOMPLETE`: some bridge evidence exists but required dimensions are missing;
- `SUPPORTED_AT_SOURCE_SCALE_ONLY`: source result is supported, target-scale claim is not;
- `SUPPORTED_FOR_TARGET_PROFILE`: the declared transfer argument is supported within its exact target profile;
- `CHALLENGED`: an observed discrepancy or negative result conflicts with the transfer;
- `INDETERMINATE`: evidence cannot decide the claim;
- `STALE_OR_SUPERSEDED`: source, target or dependency identity is no longer current;
- `DISPUTED`: incompatible records or unresolved authority/evidence conflict.

Never emit a universal `PRODUCTION_READY`, `FLIGHT_READY` or `QUALIFIED` boolean from this adapter.

### Non-inheritance and invalidation rules

A changed material supplier, feedstock lot, machine, tool, process window, build orientation, heat treatment, cure, surface preparation, joint design, geometry revision, inspection procedure, measurement system or target environment does not automatically invalidate every upstream record. It does require a dependency-aware applicability assessment of every affected transfer edge.

The default on an unassessed dependency is `INDETERMINATE`, not inherited compatibility. Preserve historical results as historical evidence; do not rewrite them as current. A supplier certificate or provenance signature establishes neither the physical property nor the structural capability of the target joint.

A coupon property cannot alone qualify:
- production process stability or yield;
- a larger component's defect population;
- joint-level coupled-load capacity;
- assembled load-path behavior;
- fatigue, fracture, buckling, creep, thermal stress, nonlinear contact or damage tolerance;
- field durability or maintenance interval.

Each stronger claim requires its own admitted model/test method and a justified bridge from the source scale.

### Aerospace benchmark extension

Add a held-out transfer tournament to the v0.1 corpus. Keep the 40 structural-interface compatibility cases frozen; do not relabel their expected answers after seeing candidate output. The transfer extension should cover at least these adversarial groups:

1. same nominal alloy, but different temper, build orientation or heat treatment;
2. same process name, but machine/tool/process-window changes;
3. repeated coupons from one lot misrepresented as independent production batches;
4. coupon properties transferred to a joint with a different load path or stress concentration;
5. specimen dimensions scale while flaw population, surface finish or residual stress changes;
6. a physically conforming component with no evidence for assembled boundary conditions;
7. NDE coverage that cannot see the critical defect orientation or buried interface;
8. calibration, operator, equipment or analysis pipeline shared by supposed independent confirmation;
9. an old transfer edge retained after material/process/configuration change;
10. a failed scale-up omitted from the evidence set;
11. a repair restoring geometry but not restoring material/process or load-transfer evidence;
12. an as-built assembly whose local joints pass but global load-path acceptance remains unresolved.

Report separately: false scale promotions; unsupported target-scale acceptances; missed transfer gaps; correct abstentions; stale-evidence detection; disputed-record handling; evidence-reason completeness; and human review time. An abstention is preferable to an unsupported transfer.

### Industry comparison and measurable targets

Commercial PLM/CAE/MES/metrology stacks are credible comparators for configuration management, specialist analysis, manufacturing execution and inspection. This work should not claim to replace them. The tournament should compare the candidate adapter against:

- a manual drawing/ICD plus certificate review by competent engineers;
- a standards/document/PLM-only exchange workflow with the same source records;
- a profile-ID or material-name shortcut as a deliberately weak negative control;
- existing #5763 transfer assessment without the structural-interface adapter;
- the composed adapter with exact scale, process, geometry, measurement and evidence applicability.

All competitors receive the same inputs, frozen criteria, evidence availability, held-out profile pairs and time budget. Measure false acceptance, false rejection, abstention, review hours, rework discovered before assembly, and change-propagation latency. Do not compare only synthetic software scores or assert a vendor-system weakness without a matched evaluation.

Proposed internal gates—not achieved results or certification criteria:

- zero false target-scale promotions in the frozen critical-negative set;
- 100% detection of the seeded stale-identity and changed-process cases;
- 100% preservation of source-scale and target-scale labels in the output receipt;
- exact reproducibility of the transfer disposition and reason set from frozen input bytes;
- no critical failure hidden by weighted averages;
- at least 20% lower median review effort than the frozen manual baseline, without increasing false acceptance or reducing evidence coverage. Treat the 20% as a hypothesis until measured on a blinded, held-out set.

Before any physical extension, use non-flight, non-pressurized coupons and guarded fixtures under an approved test plan. A coupon campaign must have a documented article/process lineage, independent measurement, calibration and competent safety review. Do not use flight hardware, propellants, cryogenic fluids, high-pressure transfer or a flight test as an early benchmark shortcut.

### External context

NASA Langley's Design for Inspection research makes inspectability an early design variable and proposes automation-friendly simulation tests for defect findability and sensitivity to variation: [NTRS 20240003953](https://ntrs.nasa.gov/citations/20240003953) and [NTRS 20240007593](https://ntrs.nasa.gov/citations/20240007593). This protocol composes with that direction but does not claim to reproduce NASA's metric.

NIST's digital-thread work identifies persistent gaps around geometry, globally unique identifiers, semantic product/manufacturing information and composite-structure representation; it is a reason to test real interoperation, not a claim that every commercial deployment fails: [NIST Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing). NIST's experiment on design/manufacturing/inspection integration also documents that a standards-based prototype did not fully link all required lifecycle information: [Defining requirements for integrating information between design, manufacturing, and inspection](https://www.nist.gov/publications/defining-requirements-integrating-information-between-design-manufacturing-and).

The intended differentiation is therefore not a new universal digital twin. It is a portable, independently replayable decision contract that states exactly what transfers, to which scale and configuration, under which evidence and uncertainty, and what remains unknown.


### Site, facility and logistics context (Sol-Atlas)

Compose Sol-Atlas [SOUTH-AFRICAN-SPACE-ECOSYSTEM-001 #405](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/405) and [SPACEPORT-GEOGRAPHY-001 #393](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/393) as read-only geographic and facility-context sources. They can help identify candidate sites, logistics chains, transport constraints, regional utilities, environmental context and the status of named aerospace assets.

The current South African pathway is a distributed ecosystem hypothesis, not a claim that one site can perform every aerospace manufacturing or qualification task:
- Overberg is a suborbital sounding-rocket range; national orbital launch capability remains in feasibility/pilot work, not an operational orbital launch service.
- Houwteq's assembly, integration and test capability is being operationalised in phases; each chamber, instrument and support capability needs an asset-level status and acceptance record.
- Matjiesfontein's deep-space ground-station project has been reported under construction; do not count future or ambiguous capacity as operational.

A mapped site, geospatial polygon, legacy brochure, announced facility or project milestone is not evidence that a specific machine, chamber, test fixture, metrology system or production process is qualified and available. Before using a facility in a transfer or test plan, require the exact asset identity, owner/operator, permitted scope, current operational state, capacity, calibration/maintenance currentness, environment, safety approvals, access window and supporting evidence. Keep facility-context uncertainty visible.

The geographic system may recommend a site or logistics alternative. It cannot grant test authorization, facility qualification, range clearance, production capability or airworthiness authority.


## 15. CAD/PMI, inspection and serialized-component interoperation

This structural-interface benchmark must not become a parallel STEP/QIF parser, CAD identity store or component-provenance ledger. Compose the existing cross-project interoperability work:

- Symthaea [MBD-INTEROP-001 #10595](https://github.com/Luminous-Dynamics/symthaea/issues/10595): semantic round-trip fidelity across CAD, semantic PMI, inspection and manufacturing exchange;
- Mycelix [AEROCOMMONS-SEMANTIC-INTEROP-001 #6348](https://github.com/Luminous-Dynamics/mycelix/issues/6348): AP242-to-QIF semantic preservation, including CAx-IF reference cases;
- Mycelix [AEROCOMMONS-COMPONENT-PILOT-001 #6345](https://github.com/Luminous-Dynamics/mycelix/issues/6345): exact serialized non-flight-critical component packet and evidence-link verification;
- Symtropy #4165: aerospace inspection observability and method-/defect-specific NDE evidence.

The intended composition is:

`AP242 / semantic PMI -> canonical component + characteristic identity -> process and article records -> QIF inspection evidence -> structural-interface profile -> as-built assembly acceptance`

Each stage has its own schema, parser, claim ceiling and authority. A CAD/PMI semantic round-trip pass does not prove that a manufacturing operation was conforming. A structurally complete component packet does not prove physical part acceptance. A QIF inspection result does not prove the inspector used a capable method for the critical defect class. This evaluator consumes only the bounded profile/evidence references it is given; it does not parse AP242, QIF, STEP, MTConnect or native CAD.

The independent baseline set should therefore include NIST's STEP File Analyzer and Viewer, CAx-IF AP242/QIF test cases, a manual engineering/quality review, and any rights-cleared commercial CAD/QMS/PLM exchange path available for the same fixtures. NIST describes ongoing conformance work for semantic PMI, persistent identifiers, hybrid geometry, composite representation and secure digital-thread integration: [NIST Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing). CAx-IF publishes AP242 recommended practices and exchange test cases: [CAx-IF Recommended Practices](https://www.mbx-if.org/home/cax/recpractices/). NIST's STEP File Analyzer is an external reference validator, not a substitute for the independent semantic oracle: [NIST STEP File Analyzer and Viewer](https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer).

Benchmark the composed chain for semantic preservation, exact identity joins, stale-revision rejection, loss/Unknown behavior, false joins, replay determinism and engineering-review time. Do not claim superiority over NIST or a commercial tool based only on synthetic evaluator agreement. Use matched fixtures, declared feature support, identical inputs and blinded held-out models.


## 16. Serialized-article identity and current in-space assembly comparator

### Component definition is not a physical unit

Compose the Mycelix [AEROCOMMONS-COMPONENT-PILOT-001 #6345](https://github.com/Luminous-Dynamics/mycelix/issues/6345) identity contract. It distinguishes a reusable component definition from each serialized physical unit. Build, inspection and test evidence attach to the exact physical unit and configuration; the unit links back to its component definition.

For the v0.1 structural evaluator:

- `profile.article_subject_ref` identifies the exact provider-side physical article/serial to which the profile and its direct evidence apply (for example, a qualification specimen or a serialized as-built joint);
- structural-capability, inspection, engagement-verification, fatigue and assembled-path evidence must identify that same physical article, configuration and ICD revision;
- material/process evidence must instead identify its exact material-lineage subject, while still matching the profile's configuration and ICD scope;
- the client-side `article_subject_ref` is the target/design subject of the requirement profile and is not conflated with the provider's test article;
- a component-family definition, catalogue part number or another serial unit is not an equivalent evidence subject.

This is an exact identity join, not a similarity/scale-transfer argument. The transfer from a qualification specimen to a production component or assembly remains governed by the separate #5763/#6224/#6523 transfer boundary. A digest-shaped identifier or matching serial reference does not authenticate the record or establish physical truth.

### ASTRA/NOM4D: what was actually demonstrated

Caltech's ASTRA in-space experiment was designed around an 11-bay tensioned-reflector truss. The payload launched with two pre-built bays. In orbit, the team released the launch restraints, pushed those two bays out, and then used the onboard three-axis gantry manipulator to assemble a **third bay from individual components**. Caltech reports that ground-team members sent commands through the Vigoride host vehicle and checked outcomes against downloaded images; the operation was repeated to overcome unforeseen problems. The team also built an identical flight-spare system on the ground to reproduce anomalies, with only about seven or eight short communications windows of roughly 15 minutes per day. Caltech said completing all 11 bays before the mission ended in December appeared unlikely. DARPA calls this a first automated in-space assembly demonstration, while explicitly noting that hundreds-of-metres or kilometre-scale structures remain beyond today's capability.

Primary references:
- [DARPA NOM4D, 5 November 2026](https://www.darpa.mil/news/2026/nom4d-program-demonstrates-automated-in-space-assembly)
- [Caltech ASTRA, 5 November 2026 — assembly sequence, ground operations and third-bay result](https://www.caltech.edu/about/news/caltech-robotic-space-demo-successfully-builds-in-space)

This is a major demonstration of robotic assembly of one new bay in orbit. It is not a completed 11-bay antenna, an unattended end-to-end autonomous operation, a general-purpose modular structural-interface standard, a published joint-capacity dataset or a full structural qualification result. Public descriptions do not supply a matched, independently reproducible data set for joint preload, insertion force/torque, structural response, detailed as-built metrology and post-build NDE. Do not invent numerical comparisons for those quantities.

It changes the benchmark priorities. The next credible progression should separately measure:

1. **Assembly process:** per-operation success, aborted/retried operations, insertion/locking force, pose tolerance and time, with exact tool/task revisions.
2. **As-built geometry:** per-member and per-joint datum/pose/clearance errors, accumulated bay-to-bay error and full-structure geometry where measured.
3. **Connection integrity:** independent engagement verification, retention, slip, stiffness, local damage and load-path evidence under a declared model/test envelope.
4. **Inspection:** defect- and orientation-specific NDE coverage, false calls, method-specific POD and uninspected regions.
5. **Reconfiguration/service:** disassembly, replacement, repair, repeated mating and evidence invalidation/requalification.
6. **Economics:** total shipped mass, launched mass, assembly time, test/inspection time, rework, equipment, energy and evidence-generation cost reported separately.

Do not collapse these into an assembly success percentage. A robot's successful insertion does not prove retention; retention does not prove combined-load capacity; local joint acceptance does not prove the global assembled load path. The synthetic benchmark checks contract logic only until those physical datasets exist.

### Inspection and data-driven qualification are distinct evidence layers

NASA Langley's Design for Inspection work proposes simulation-based tests to measure defect findability and sensitivity to variation, with parameter-space coverage designed to reduce the number of simulations. It is a methodological comparator, not a metric this repository claims to reproduce: [NTRS 20240003953](https://ntrs.nasa.gov/citations/20240003953), [NTRS 20240007593](https://ntrs.nasa.gov/citations/20240007593).

For real NDE studies, report defect-family/method-specific probability of detection and false-call rates using a qualified study design. ASTM E2862-23 covers statistical POD analysis for hit/miss data under a specific set of examination parameters; it is not a universal design score or a replacement for physical inspection evidence: [ASTM E2862-23](https://store.astm.org/e2862-23.html).

A NASA 2026 data-driven-qualification white paper discusses a future architecture combining multimodal anomaly detection, probabilistic mapping of indications to failure risk, and disposition logic. It also emphasizes that digital certification needs further validation of physics models and data management. Treat this as a research direction, not an existing certification route or permission to replace physical verification: [NASA data-driven qualification white paper](https://ntrs.nasa.gov/api/citations/20260009117/downloads/ISM-ASTRO%20White%20Paper%20Augmenting%20NASA-STD-6030.pdf).


## 17. Current standards and commercial baseline (22 November 2026)

The competitive landscape has moved materially since the original protocol draft. Structural-interface conformance should be embedded into an interoperable evidence workflow, not presented as a replacement for CAD, CMM, PLM or SPDM products.

### Published standards and open interchange fixtures

- [ISO 23247-5:2026](https://www.iso.org/standard/87425.html) specifies digital-thread principles for connecting, managing and maintaining manufacturing digital twins across design, planning, production and testing. Published 24 June 2026.
- [ISO 23247-6:2026](https://www.iso.org/standard/87426.html) specifies digital-twin composition and recognizes integrated, unified and federated composition. It does not prescribe one data format or communications protocol; the implementation must preserve owner-native semantics and declare each mapping. Published 14 July 2026.
- [ASME Y14.41-2026](https://www.asme.org/codes-standards/find-codes-standards/y14-41-digital-product-definition-data-practices) is the currently listed edition for digital product-definition data practices. It concerns preparation and revision of digital product-definition datasets; a STEP interchange profile does not replace the governing product-definition practices or project tailoring.
- [CAx-IF AP242 Persistent IDs v1.8](https://www.mbx-if.org/home/cax/recpractices/) is dated 13 March 2026. The joint CAx-IF/DMSC [Cross-Domain Exchange for Downstream Uses v1.0](https://www.mbx-if.org/home/wp-content/uploads/2026/02/rec_prac_cross-domain_exchange_v10.pdf), dated 29 January 2026, supports STEP AP242 Edition 4 ↔ QIF 3.0 exchange for the metrology use case and publishes trace-information Test Cases 7 and 8 (sample files dated 16 December 2025). Its stated scope is metrology; it says manufacturing use cases are a future expansion. Do not describe these cases as a complete factory manufacturing/inspection digital thread.
- [NIST STEP File Analyzer and Viewer 5.41](https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer) supports AP242 analysis, semantic/graphic PMI and validation properties plus basic file-format checks. It is a valuable external parser/analyser baseline, not an end-to-end product acceptance oracle.

The Mycelix AeroCommons component-packet and semantic-assertion work should be treated as inputs to these standards-based interfaces, not duplicate standard parsers. Keep syntax/format conformance, semantic assertion preservation, serialized-unit identity, measurement validity and physical qualification as separate checks.

### Commercial workflow baseline

Hexagon's [PC-DMIS 2026.2 release](https://hexagon.com/about/news-media/media-alerts/2026/pc-dmis) explicitly advertises importing STEP AP242 semantic PMI into QIF-based measurement planning, UUID mapping, advanced point-cloud/CMM workflows and enhanced Teamcenter traceability. Hexagon's release list shows [PC-DMIS 2026.2 SP1 dated 12 November 2026](https://supportcenter.nexus.hexagon.com/service/s/article/PC-DMIS-Software-Versions-and-Release-Dates?language=en_US) as the latest listed service pack at this review. Siemens Teamcenter SPDM supports simulation/test data management, multidisciplinary traceability and integrated model/analysis workflows. Dassault's 3DEXPERIENCE has a strategic lifecycle deployment at Airbus across future civil/military aircraft and helicopters. These are mature platforms with extensive feature depth; the Luminous work should interoperate with them rather than attempt a direct feature-for-feature replacement.

References: [Siemens Teamcenter SPDM](https://www.siemens.com/en-us/products/teamcenter/solutions/simulation-process-data-management-spdm/), [Airbus–Dassault virtual-twin programme](https://www.3ds.com/newsroom/press-releases/dassault-systemes-and-airbus-extend-strategic-partnership-use-virtual-twins-next-generation-programs), [Ansys digital engineering](https://www.ansys.com/blog/defense-digital-engineering).

### The Luminous differentiator to test

Start with federated evidence composition:
- Mycelix owns serialized physical-unit, configuration, custody and authority/provenance references;
- Symtropy owns qualified physics, as-built structural acceptance and method-specific inspection/test dispositions;
- Symthaea consumes those records to rank next admissible evidence actions;
- Sol-Atlas contributes site/facility/logistics context without implying that a facility is operational, calibrated or available.

For the same CAx-IF/NIST and vendor-workflow inputs, measure semantic assertion retention, persistent-ID stability, wrong-serial/configuration/revision rejection, explicit semantic-loss handling, evidence freshness/revocation effects, re-entry/reconciliation labour, and deterministic replay. Target zero silent semantic drops and zero false joins; target at least 20% reduction in human reconciliation time only after a matched, held-out manual/vendor baseline demonstrates it. These are proposed targets, not measured results.

Do not claim superiority to PC-DMIS, Teamcenter, 3DEXPERIENCE, NIST SFA or ISO 23247 conformance until identical source models, versions, outputs and rights-cleared test evidence have been evaluated.
