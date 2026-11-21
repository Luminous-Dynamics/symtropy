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
