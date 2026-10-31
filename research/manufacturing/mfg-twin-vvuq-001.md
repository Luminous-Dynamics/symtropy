# MFG-TWIN-VVUQ-001 — Composed manufacturing twin assurance profile

**Status:** research specification / first integration slice.  
**Parent work:** [Symtropy issue #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121).  
**Branch:** `research/manufacturing-twin-vvuq-20261031`.  
**Claim ceiling:** this document defines a bounded-context research qualification profile. It is not a factory certification, product certificate, safety case, or claim of conformity to ISO/AWI 23247-8.

## 1. Thesis

Manufacturing digital twins must carry evidence for the claims they make. A simulator output is not automatically a validated prediction; a measurement is not automatically a known truth; a guard-band ACCEPT is not a deterministic statement about the true measurand; and an authorized proposal is not proof that an intervention succeeded.

The system-level goal is therefore:

> Given a declared product, process, machine, material, measurement method and context of use, independently reproduce the chain from engineering intent to physical prediction, observed evidence, conformity decision and verified intervention outcome.

The core innovation is **composition-level assurance**. Qualification of each component is necessary but not sufficient: connecting two individually credible twins can create new semantic, temporal, numerical, statistical, cyber and authority failures.

## 2. Standards and external anchors

As checked on 31 October 2026:

| Anchor | Status / role | Engineering implication |
|---|---|---|
| ISO 23247-5:2026 | Digital-thread lifecycle connections | Bind design, process, production, inspection and service evidence with lifecycle-aware relations |
| ISO 23247-6:2026 | Published 14 July 2026; integrated, unified and federated twin composition | Qualify each composition mode and its interfaces |
| ISO/AWI 23247-8 | Approved work item registered 23 October 2026; under development | Build a testable VVUQ profile now; do not claim compliance with an unpublished standard |
| NISTIR 8620 | Published 21 July 2026 | Treat interoperability, VVUQ, cybersecurity and workforce readiness as first-class concerns |
| ISO 23952:2020 / QIF 3.0 | Quality-information exchange | Preserve inspection-plan, measurement-resource, result and analysis semantics |
| MTConnect | Machine/process data model and interfaces | Preserve version, schema, document-kind, stream sequence and native semantics |
| ISO 14253-1:2017 | Current, confirmed in 2023; measurement-uncertainty decision rules | Separate guarded acceptance/rejection from claims about physical truth |
| JCGM 106:2012 | Measurement uncertainty in conformity assessment | Use a qualified post-measurement PDF for probability/risk claims; do not infer one from expanded uncertainty alone |
| ISO/IEC Guide 98-3:2008/Amd.1:2026 | Published July 2026; significant nonlinearity in measurement models | Compare first-order propagation with higher-order or distribution-propagation references |
| ISO 22514-2:2026 | Published February 2026; time-dependent process capability/performance | Separate time-dependent process variation from measurement uncertainty and model uncertainty |
| NIST AM-Bench 2025 | Controlled AM measurements and challenge problems | Use challenge-specific observables and external held-out data |
| NIST Smart Manufacturing Systems Test Bed | CAD/CAM/CAI/PDM/V&V, CNC, inspection, MTConnect | Use as an interoperability and digital-thread reference |

Standards are normative anchors, not automatically executable benchmarks. The exact editions, schema bundles, test plans, licenses and product-specific applicability must be pinned in admission receipts.

## 3. Reuse boundaries

This programme composes existing owners; it does not replace them.

| Evidence / authority | Existing owner |
|---|---|
| Product/component identity and genealogy | Symtropy #2933 |
| Machine telemetry intake | Symtropy #2956 |
| MTConnect/QIF mapping and schema admission | Symtropy #3025, #3026, #3033; draft PR #3030 |
| Thermal/solidification physics verification | Symtropy #2925, #2980, #2982, #2999 |
| NIST AM-Bench thermal observable extraction | Symtropy #3053; draft PR #3055 |
| MSA, calibration, drift and uncertainty budgets | Symtropy #2995 |
| Scalar guarded conformity | Symtropy #2996; draft PR #2997 |
| Posterior-based conformity risk | Symtropy #3007; draft PR #3008 |
| Dependence-robust joint/part conformity | Symtropy #3000, #3009; draft PRs #3047, #3050 |
| Nonlinear measurement uncertainty benchmark | Symtropy #3093; draft PR #3094 |
| Time-dependent process capability | Symtropy #2963 and children #3096, #3101; draft PRs #3098, #3103 |
| Intervention proposal/authority/postcondition | Symtropy #2976; Symthaea #9199, #9200 |
| Decision-aware inspection acquisition | Symthaea #9232, #9234 |
| Deterministic twin rehearsal / credibility boundary | Symtropy #1467, #1475 |

A linked identifier or digest does not replace the upstream verifier. The composition verifier checks that the evidence exists, is correctly scoped, is compatible and is complete; each evidence owner remains responsible for its domain semantics and truth conditions.

## 4. Assurance architecture

Use four evidence layers and one authority boundary:

1. **Physical model evidence:** geometry, material state, process inputs, numerical method, solver configuration, analytical verification and external validation.
2. **Observation/metrology evidence:** raw source, instrument, calibration, MSA, measurement model, uncertainty budget, environmental conditions, timestamp and transformation history.
3. **Conformity evidence:** requirement/specification revision, characteristic identity, decision rule, scalar disposition, posterior/risk evidence if qualified, and part/assembly aggregation.
4. **Action evidence:** proposed intervention, model context, operating envelope, authorization, execution telemetry, post-action measurement and postcondition verification.
5. **Authority boundary:** engineering reasoning may recommend; the policy/authorization function decides; equipment interlocks and machine controllers govern physical execution. The research slice is read-only.

### 4.1 Component-level credibility

Every component twin must declare:
- context of use and intended engineering claims;
- exact model/code artifact, version, build configuration and material/recipe/geometry inputs;
- validity domain, exclusions, assumptions and known failure modes;
- input/output variables, units and reference frames;
- numerical verification evidence (analytical or independent oracle, conservation, mesh/time convergence, solver stability);
- validation evidence using data not reused for model calibration;
- uncertainty decomposition and calibration evidence;
- OOD/applicability-domain behavior and explicit abstention conditions.

A component-level VVUQ result is scoped to the declared context and test corpus. It must not be generalized to a new alloy, process window, machine, geometry, sensor or product family without evidence.

### 4.2 Interface and composition credibility

The composition must explicitly model each connection:
- source component and exact output variable;
- destination component and exact input variable;
- semantic mapping and version;
- unit conversion, coordinate/frame transform and time transform;
- sampling, interpolation, aggregation and event-order policy;
- missing, stale, duplicated, reordered and contradictory input behavior;
- coupling direction, synchronization and feedback semantics;
- shared uncertainty sources and covariance/nuisance identity;
- provenance and verification receipt.

Cycles are allowed only when the coupling contract declares the state boundary, update order, timestep/event semantics and replay behavior. Do not mistake an arbitrary graph cycle for a well-defined co-simulation coupling.

### 4.3 Three ISO 23247-6 composition modes

| Mode | Required verification |
|---|---|
| Integrated | Common-schema mapping is lossless for declared semantics; canonical identity and unit rules are explicit; schema changes fail closed |
| Unified | Component schemas remain independently owned; typed adapters preserve native meaning; cross-schema equivalence is tested without erasing source detail |
| Federated | Peer identity, version negotiation, event ordering, clock skew, staleness, retries, duplicate/replay protection, partitions and conflicting peer state are tested |

The three modes are alternatives with different operational trade-offs. Do not assume the easiest mode is the only supported architecture.

## 5. Uncertainty is a typed vector, not one sigma

Keep these sources distinct:

- measurement uncertainty (including calibration, bias, repeatability, reproducibility, linearity and drift);
- process/population variability;
- uncertain physical/material parameters;
- boundary and initial-condition uncertainty;
- numerical discretization and solver error;
- model-form discrepancy;
- sensor/data synchronization and transformation uncertainty;
- shared/correlated nuisance variables between components.

Where dependence is known, preserve covariance or an explicit joint model. Where dependence is unknown, do not silently multiply marginal probabilities or use independent root-sum-square combination.

For nonlinear measurement models, record the propagation method and assumptions. Compare first-order GUM linearization with higher-order Taylor terms and a distribution-propagation reference where appropriate. Deterministic quadrature, Monte Carlo and analytic integration are distinct methods and must be labelled accurately.

For conformity:
- the guard-band route consumes an explicitly qualified coverage interval under a declared decision rule;
- a probability-of-conformity route consumes a separately qualified post-measurement distribution;
- expanded uncertainty alone does not define a PDF;
- a characteristic ACCEPT does not authorize shipment;
- part-level conformity must retain every required characteristic and its own evidence; missing characteristics remain incomplete, not silently removed from the denominator.

## 6. Read-only composition manifest

The first version should be a small, versioned manifest rather than a new general-purpose ontology.

Required top-level fields:

| Field | Meaning |
|---|---|
| `schema_version` | Manifest format identity |
| `composition_id`, `revision` | Stable composition identity |
| `composition_mode` | integrated / unified / federated |
| `context_of_use` | Product, process, machine, material, geometry and operating envelope |
| `claims` | Exact outputs/decisions the composition is intended to support |
| `components[]` | Component identity, model artifact, interface schema and VVUQ receipt references |
| `interfaces[]` | Typed mappings, transforms, clocks, ordering and dependency declarations |
| `evidence_refs[]` | Source artifact, digest, evidence class, authority and verification receipt |
| `validity` | Time/condition range, expiry and invalidation conditions |
| `authority` | Read-only / proposal-only / separately authorized execution boundary |

For component and interface records, require exact source identifiers and digests. SHA-256 proves byte identity relative to a declared digest; it does not prove source truth, correct calibration, or signer authority. Verify signatures and authorization through their existing owners where applicable.

Evidence classes must never be collapsed:
- synthetic known-answer fixture;
- simulation result;
- public measured dataset;
- independently measured pilot/factory data;
- external published benchmark result;
- independently audited verifier receipt.

A synthetic success is evidence of implementation behavior, not physical validation.

## 7. Independent integration verifier

The verifier is deliberately separate from production mapping, simulation and conformity code. It should consume frozen manifests and receipts, and check:

1. Every component, interface and evidence reference resolves to an admitted exact identity/revision.
2. All required graph nodes and edges are present; no unknown or duplicate identity is silently accepted.
3. Source and destination semantics, units, frames and time domains are compatible.
4. The declared composition mode is respected.
5. VVUQ claims apply to the exact model/configuration/context and have not expired or been invalidated.
6. Shared uncertainties/dependencies are represented consistently or cause an explicit unresolved-dependence state.
7. The end-to-end claim has all required evidence, with no substitution of a synthetic artifact for an external measured one.
8. Conformity and intervention authority remain distinct.
9. The same deterministic inputs produce identical verifier receipts where deterministic replay is promised.

The verifier must not call the production mapper or production decision function to compute its own expected answer. Use an independently authored oracle and separately generated fixtures.

## 8. Qualification and benchmark matrix

### 8.1 Physical prediction

**Reference lanes:** exact analytical Stefan cases; independently implemented reduced-order references; NIST AM-Bench measurements/challenge problems; an independently configured multiphysics solver when licenses and model equivalence permit.

**Metrics:** analytical relative/absolute error; energy/mass balance residual; observed convergence order; mesh/time-step sensitivity; phase/thermal observable error; runtime and memory.

For AMB2025-07 PCRTAM, retain the exact NIST definitions for time above melt (TAM) and solid cooling rate (SCR), including the same-spike relationship. Distribution comparison and spatially paired error are separate; do not invent one composite score.

### 8.2 Measurement and uncertainty

**Reference lanes:** calibration standards and certificates, crossed MSA studies, independent stability/linearity experiments, analytic nonlinear functions, Monte Carlo or deterministic distribution propagation.

**Metrics:** repeatability/reproducibility, bias, linearity, stability/drift, uncertainty-budget reconstruction, empirical coverage, interval width, calibration curves, residual tails and unresolved probability mass.

Report nominal coverage at multiple levels (for example 50%, 80%, 90%, 95% and 99%) with sample counts and lot/campaign-clustered confidence intervals. Do not claim a calibrated uncertainty model from synthetic fixtures alone.

### 8.3 Process capability

**Reference lanes:** ISO 22514-2:2026 semantics where applicable, independent SPC package output, analytically known time-dependent process distributions, held-out production lots.

**Metrics:** within-subgroup Cp/Cpk and overall Pp/Ppk kept distinct; time-windowed worst-case capability; mean/variance shift detection; false alarm and missed detection rates; lot-clustered interval coverage; drift detection delay.

### 8.4 Conformity and decisions

**Reference lanes:** ISO 14253-1 decision-rule cases, JCGM 106 probability/risk formulations, exact finite-support operating-characteristic oracle, independent joint-probability enumeration.

**Metrics:** false acceptance, false rejection, indeterminate rate, evidence-incomplete rate, consumer/producer risk bounds, operating-characteristic curves and expected decision loss.

### 8.5 Composition and operational value

**Reference lanes:** NIST SMS Test Bed, official MTConnect/QIF fixtures, synthetic adverse timing and identity fixtures, independently observed pilot data.

**Metrics:** semantic preservation, identity-link precision/recall on known joins, invalid-link rejection, clock/synchronization error, stale-data detection, deterministic replay, composition discrepancy and evidence completeness.

Operational measures (first-pass yield, rework, scrap, cycle time, energy per conforming unit, inspection cost, delivery P50/P90) must be normalized for product mix and accompanied by uncertainty. Do not infer field savings from synthetic runs.

### 8.6 Intervention assurance

**Reference lanes:** state-machine model checking / exhaustive transition oracle; expired, replayed, unsigned, wrong-scope, stale, out-of-envelope and conflicting-action fixtures.

**Metrics:** zero unauthorized/expired/replayed/out-of-envelope executions in the frozen negative-control corpus; zero closure without a required postcondition receipt; postcondition verification latency; recovery completion; audit completeness.

## 9. Comparison with industrial baselines

The benchmark must compare like with like.

- NIST AM-Bench is the external scientific-validation anchor for additive-manufacturing physics and measured thermal/process observables.
- NIST SMS Test Bed is an integration/digital-thread anchor for CAD/CAM/inspection/manufacturing data exchange.
- MTConnect and QIF are interoperability standards, not competing simulators.
- Industrial metrology/SPC suites (for example, Hexagon Q-DAS and CMM/inspection ecosystems) are appropriate baselines for accepted quality-data capture and process-capability workflows.
- Commercial multiphysics tools (for example, Ansys or COMSOL where equivalent models and disclosed settings are available) are appropriate numerical/physics comparators, not a universal gold standard.

Symtropy should not claim to replace full-featured PLM/MES, CMM software, or every commercial multiphysics suite. The narrow differentiator to prove is the reproducible, independently verifiable connection between model claims, measured evidence, uncertainty-aware disposition and bounded intervention across heterogeneous systems.

A superiority claim requires matched:
- geometry, material state, process inputs and boundary conditions;
- data split and independent physical measurements;
- model fidelity and solver tolerances;
- measurement method and uncertainty;
- compute hardware, wall time and tuning budget;
- decision policy and product-risk assumptions.

Report accuracy and evidence quality together. A slightly less accurate model with well-calibrated uncertainty may support safer decisions than a point predictor with hidden error; that is an empirical hypothesis to test, not an automatic advantage.

## 10. Advancement gates

Targets, not achieved results:

- 100% rejection/abstention on frozen wrong-identity, wrong-unit, stale-schema, missing-calibration, authority, OOD, replay and incomplete-evidence negative controls.
- Zero invalid/fuzzy part-lot-operation joins accepted in the reference corpus.
- Every accepted claim binds exact model, configuration, source data, requirements, context of use and applicable VVUQ evidence.
- No component qualification is promoted to composition qualification without interface and end-to-end tests.
- Coverage is evaluated on independent lots/campaigns with clustered confidence intervals at multiple nominal levels.
- On real held-out data, match or improve the best disclosed baseline on predictive error while improving calibration or decision loss, or disclose the trade-off. A single case cannot support a global SOTA claim.
- Zero successful unauthorized action attempts and zero intervention closures without required postcondition evidence in the frozen failure/attack corpus.

## 11. First implementation sequence

### Slice A — manifest and verifier (read-only)
- Freeze a minimal versioned composition-manifest schema.
- Build one scenario joining an admitted MTConnect machine/process cycle to a QIF inspection result, exact part/lot/operation, Symtropy model run, metrology/uncertainty receipt, conformity outcome and non-executable intervention proposal.
- Write an independent verifier with positive, negative and metamorphic fixtures.
- Keep source artifacts and fixture evidence classes explicit.

### Slice B — composition modes
- Add equivalent integrated, unified and federated fixtures.
- Introduce adversarial schema, clock, event-order, outage, replay and correlation cases.
- Verify that composition-level residuals/uncertainties are not silently discarded.

### Slice C — external measured validation
- Admit NIST AM-Bench and NIST SMS fixtures with exact source version, license, byte digest and independent validation receipt.
- Compare physical-model predictions with held-out observations.
- Add independent-lot coverage and decision-risk evaluation.

### Slice D — bounded intervention rehearsal
- Consume existing Symtropya proposal and Symtropy authority/postcondition contracts.
- Simulate authorization/execution/recovery in a deterministic offline fixture.
- Keep live machine writes and PLC control out of scope until separate safety, security, authority and deployment qualification.

## 12. Explicit non-goals

- No new universal manufacturing simulator or material/process registry.
- No new sensor, calibration, MSA, uncertainty or conformity authority.
- No PDF inferred from expanded uncertainty.
- No synthetic-to-real evidence promotion.
- No acceptance/quality result treated as shipment authorization.
- No direct actuation, PLC writes or autonomous process changes.
- No claim that hash integrity alone proves the underlying physical claim.
- No ISO certification or claim of conformity to ISO/AWI 23247-8.

## References

- ISO 23247-6:2026: https://www.iso.org/standard/87426.html
- ISO/AWI 23247-8: https://committee.iso.org/standard/94858.html
- NISTIR 8620: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- NIST Smart Manufacturing Systems Test Bed: https://www.nist.gov/laboratories/tools-instruments/smart-manufacturing-systems-sms-test-bed
- NIST AM-Bench: https://www.nist.gov/ambench
- NIST AM-Bench 2025 data guidance: https://www.nist.gov/ambench/direct-am-bench-data-links-and-referencing-guidance
- ISO 14253-1:2017: https://www.iso.org/standard/70137.html
- JCGM Guides in Metrology: https://www.bipm.org/en/web/guest/publications/guides
- ISO/IEC Guide 98-3 Amendment 1:2026: https://www.iso.org/standard/92716.html
- ISO 22514-2:2026: https://www.iso.org/standard/88883.html
- QIF / ISO 23952:2020: https://qifstandards.org/
- MTConnect: https://www.mtconnect.org/
- Hexagon Manufacturing Intelligence: https://hexagon.com/company/divisions/manufacturing-intelligence
