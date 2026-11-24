# AERO Evidence Binding and Qualification Architecture v0.2

**Issue:** [AERO-EVIDENCE-BINDING-001 #7342](https://github.com/Luminous-Dynamics/symtropy/issues/7342)  
**Base:** AERO material/process evaluator v0.1, PR #7319  
**Status:** synthetic-only v0.2 contract and Rust semantic evaluator; no physical qualification  
**Claim ceiling:** independent source-assertion consistency and acquisition-time metrology binding only  
**Owners:** Symtropy (aviation semantics and evaluation); Mycelix (manufactured-unit lineage and artifact integrity); Symthaea BASALT (generic freshness/invalidation); Symthaea ASSURE (qualification lifecycle); competent engineering and laboratory authorities (physical truth and acceptance)

## 1. Research objective

v0.1 checks that declared records form a coherent hierarchy. That does not establish that the hierarchy is true. A consistent edit to both a specimen's ancestry and the records used to reconstruct it can conceal a wrong source assertion. Likewise, a campaign-level calibration status of active does not establish that a particular observation was acquired while its instrument and method were within a valid calibration interval.

v0.2 adds two independent evidence bindings:

1. A specimen traveler/sampling assertion independently declares and verifies the specimen's material lot, process batch and panel ancestry.
2. An intended product-configuration assertion independently binds each specimen to its intended product-definition revision.
3. Each observation carries an acquisition timestamp and time basis. Its campaign binds instrument and method revision to a first-class calibration certificate with an explicit UTC validity interval.
4. Corrections, revocations, suspensions and supersession are represented as append-only events, not in-place certificate edits.

The evaluator checks consistency and abstains when required identity, time or authority information is missing or contradictory. A parseable schema, signature, ledger record, certificate reference or evaluator pass does not prove a physical measurement is true.

## 2. v0.2 record contract

### 2.1 Independent specimen source assertion

The specimen_source_assertions collection carries:

- specimen_id;
- declared_material_lot_id, declared_process_batch_id, declared_panel_id;
- verified_material_lot_id, verified_process_batch_id, verified_panel_id;
- identity_status: verified, unresolved or conflict;
- source system, source record ID, assertion method, issuer ID and provenance reference.

The evaluator reconstructs ancestry from the material-lot → process-batch → panel → specimen hierarchy and compares it with both the declared and verified fields. It does not derive a source identity from a label, a hierarchy edge, row order or the same record it is supposed to independently check.

One assertion per specimen is required. Missing, duplicate, replayed, unresolved or contradictory assertions are not positive evidence.

### 2.2 Independent intended-configuration assertion

The intended_configuration_assertions collection independently identifies the intended product configuration for the specimen. Its declared and verified configuration IDs must agree with each other and the specimen's hierarchy reference.

This is deliberately separate from specimen.configuration_id. A specimen record cannot independently corroborate itself.

### 2.3 Acquisition-time calibration

Every observation contains:

- acquired_at_utc, in canonical YYYY-MM-DDTHH:MM:SSZ form;
- time_basis, which must explicitly be UTC.

Every test campaign references an instrument, method/protocol revision and calibration certificate. A calibration certificate contains:

- unique certificate ID;
- instrument ID and method revision ID;
- calibration laboratory;
- Mycelix packet/artifact reference;
- imported authenticity disposition (authenticated, unauthenticated or unknown);
- valid_from_utc and valid_until_utc.

The interval rule is fixed by this profile:

**[valid_from_utc, valid_until_utc) — start inclusive, end exclusive.**

Thus acquisition exactly at valid_from_utc is inside the interval; acquisition exactly at valid_until_utc is outside it. Timestamps are normalized to UTC before comparison. Unknown timezone, offset not normalized, malformed date, impossible calendar date, missing timestamp or future acquisition relative to evaluation time blocks a positive metrology-binding disposition.

The evaluator requires the certificate instrument to match the campaign instrument and the certificate method revision to match the campaign protocol revision.

### 2.4 Append-only calibration events

The calibration_events collection preserves revoked, suspended, corrected and superseded events, with:

- immutable event ID and certificate ID;
- effective_at_utc;
- recorded_at_utc;
- explicit retroactive boolean;
- for corrected / superseded, a corrects_event_id reference.

Events are evaluated as known at evaluated_at_utc. A revocation recorded after acquisition but effective before evaluation blocks positive binding and requires an accountable disposition. A retroactive event also blocks affected historical observations. Corrections must point to a prior event for the same certificate; competing corrections to the same event are treated as contradictory history and block.

This is intentionally conservative: the evaluator does not decide whether a revoked certificate invalidates every earlier physical result. It preserves the event and abstains until the responsible metrology/engineering authority establishes the applicable disposition.

### 2.5 Artifact and authenticity boundary

Certificate bytes, packet digests and byte lengths remain owned by Mycelix. The certificate_artifact_ref is a reference, not a second artifact store. authenticity_status is an imported assertion; v0.2 does not perform PKI validation, establish issuer authority, inspect instrument firmware, verify acquisition-clock trust or prove sample identity in the physical world.

## 3. Rust evaluator and report semantics

The v0.2 evaluator lives in crates/domains/symtropy-aero-material-qual/src/evidence_binding_v0_2.rs and is exposed as evidence_binding_v0_2::evaluate_binding_case.

It reports independent specimen identity, intended-configuration identity, metrology binding, exact violations/blockers and per-observation dispositions. The only non-blocked per-observation status is:

**METROLOGY_BINDING_CONSISTENT_ONLY**

This is a narrow binding result. It is not measurement truth, uncertainty sufficiency, material-model applicability, an allowable, process capability, part acceptance, airworthiness or certification. The case remains synthetic-only and the decision gate is always ineligible.

### Known v0.2 adversarial checks

- wrong declared lot while hierarchy remains unchanged;
- hierarchy ancestry changed to another valid lot while the source assertion remains unchanged;
- inconsistent intended configuration;
- acquisition before valid_from, after valid_until, exactly at either boundary;
- instrument mismatch and method-revision mismatch;
- missing/unauthenticated certificate;
- unknown time basis and malformed/ambiguous timestamp;
- certificate revoked after acquisition but before evaluation;
- duplicate certificate IDs and replayed event IDs;
- missing correction target and competing corrections to one event;
- deterministic output under record-order permutations.

The tests are synthetic software checks. Their expected dispositions are not yet an independently reviewed physical or metrology oracle. Independent review and a rights-cleared external corpus remain required before any external performance claim.

## 4. Paradigm shifts beyond a better optimizer

### 4.1 Engineering claims as typed objects

Treat every engineering claim as parameterized by an explicit context:

(physical unit, material state, process domain, configuration, environment, model/toolchain, observation time, population, uncertainty, evidence roots).

A claim may only be consumed by a downstream design decision when the required context is compatible. Missing dimensions yield UNKNOWN or NOT_COVERED, not implicit wildcard matches. This is an engineering evidence type system, not a second generic dependency graph.

### 4.2 Separate content identity, physical identity and authority

A digest establishes byte identity, not physical identity. A physical serial/lot label establishes a declared identity, not necessarily its truth. A signature establishes that a key signed bytes, not that the signer was competent or the measurement was correct. Keep these claims separate and compose them through Mycelix, engineering authorities and independently observed source records.

### 4.3 Causal change impact, not simple file invalidation

A change should propagate through typed causal dependencies: source lot, process recipe, machine state, specimen, instrument, calibration, method, model, mesh, load case and product configuration. Unaffected evidence may be retained only where observer coverage and dependency closure are complete. Missing edges must produce unresolved impact, not a false claim of no impact.

BASALT continues to own generic freshness and transitive invalidation; ASSURE owns qualification lifecycle; CEB owns typed claim-support closure. The aviation adapter only maps domain resources, claims and verification actions to those existing owners.

### 4.4 Optimize for evidence value and risk, not nominal performance alone

Jointly consider mass, stiffness, fatigue/damage tolerance, process variability, inspectability, defect detectability, access, repairability, uncertainty, test burden, energy, thermal performance and lifecycle burden. Keep hard constraints hard. A weighted score must never exchange an airworthiness-critical failure for lower mass or cost.

### 4.5 Decision-directed testing

Choose the next coupon, joint, NDE inspection or rig test by its expected value to a specified engineering decision. Use expected value of sample information / decision-loss reduction, with a cost and schedule model. The objective is not merely maximum variance reduction. Preserve minimum evidence requirements and acceptance criteria; an optimizer may propose an experiment, not waive a required test.

### 4.6 Scale transfer is a new claim

Coupon → element/joint → subcomponent → aircraft is not a simple extrapolation. Each transition needs a scoped transfer model, validation domain, uncertainty and physical evidence. A material allowable or structural claim cannot be inferred from a model score or a coupon-only benchmark.

### 4.7 Design for inspectability and repairability from the start

Optimize the physical design and its verification path together: defect types, probability of detection, access, sensor placement, inspection interval, repair limits, replaceability and maintenance downtime. Inspection capability and structural integrity are coupled design variables, not late-stage paperwork.

### 4.8 Evidence debt as an operational metric

Every design claim accumulates evidence debt when an input is unobserved, stale, out of domain, ambiguously identified or unsupported by independent validation. Track the debt by claim and configuration. Do not summarize it as a scalar that can be traded away; expose its causes and the actions needed to discharge it.

## 5. Benchmark programme and matched baselines

Use frozen input datasets, versioned interfaces, published scoring code and independent challenge cases. Report a metric vector; do not collapse safety, uncertainty, performance and cost into one score.

| Track | External/reference baselines | Required Luminous comparison |
|---|---|---|
| Aircraft sizing / mission MDO | NASA Aviary / OpenMDAO | Same geometry, mission, subsystems, constraints, derivatives, optimizer budget and convergence tolerances |
| High-fidelity aeroelastic optimization | NASA/AIAA High-Fidelity Aeroelastic Optimization Benchmark; MPhys/FUN3D reference cases | Same case, variables, meshes, objective, constraints, fidelity and compute budget |
| Materials/process qualification | Applicable CMH-17 volume/revision and conventional building-block practice | Evidence completeness, false acceptance, group holdout performance, interval coverage, transfer error and test burden |
| Model-assisted materials | NIST CM4QC Phase II / AM-Bench public challenges when scope and rights permit | Blind prediction, calibration, uncertainty, domain detection and physical residuals |
| Metrology and quality exchange | ISO 23952 QIF | Lossless round-trip, units, uncertainty, calibration/procedure identity and provenance retention |
| Product definition and configuration | ISO 10303-242:2025 AP242 | Configuration identity, geometry/PMI linkage, revision and change-impact completeness |
| Manufacturing digital thread | ISO 23247-5:2026; NIST IR 8536 | Cross-system traceability, selective disclosure, source identity, replay resistance and lineage query correctness |
| Composite substantiation | FAA AC 20-107B and applicable CMH-17 methods | Compatibility with the required building-block substantiation; no claim of replacing it |

NASA Aviary is a real conceptual aircraft sizing, mission analysis and gradient-based optimization baseline; it is not a high-fidelity structural solver. The NASA/AIAA aeroelastic benchmark exists specifically because design variables, baselines, objectives and constraints must be consistent for meaningful method comparisons. NIST CM4QC Phase II is developing aviation additive-manufacturing pathfinder work with AM-Bench; its programme must not be misrepresented as an existing public composite-laminate allowable database.

## 6. Prospective benchmark targets

These are research targets, not current results, certification criteria or regulatory limits.

| Dimension | Target / gate |
|---|---|
| Bounded semantic corpus | 100% exact agreement with independently authored expected dispositions before external claims |
| False acceptance | Zero false passes on the bounded adversarial controls, including wrong-lot and out-of-interval calibration |
| Identity closure | 100% of specimens linked to independent source and intended-configuration assertions, or explicit abstention |
| Metrology binding | 100% of admitted observations carry unambiguous UTC acquisition time and matching in-interval instrument/method certificate |
| Change-event integrity | No replayed IDs or contradictory correction histories accepted |
| Determinism | Identical reports for semantically equivalent record-order permutations |
| Applicability | No promotion outside the validated material/process/environment/configuration domain |
| Uncertainty | For nominal 95% intervals, assess 93–97% empirical coverage only on adequately sized, independent physical holdouts; report interval width and subgroup coverage as well |
| Group leakage | Zero overlap across lot, batch, panel, campaign, machine/site and configuration holdouts where those are relevant dependence groups |
| Change impact | Zero missed dependencies on a bounded independent oracle; retention only with complete qualified observer coverage |
| Verification effort | At least 20% lower total effort is a stretch target, only at matched false-accept risk and hard-requirement coverage on physical independent holdouts |

Do not report only pass rate. Include denominators, sample counts, cluster counts, exclusions, abstention, false-positive/false-negative dispositions, confidence intervals, uncertainty width, compute, lab cost and engineering hours.

## 7. Change-impact benchmark arms

Use the same frozen aerospace graph, same generic invalidation engine and same change set for all arms:

1. Blanket product requalification.
2. Manual engineering impact matrix.
3. Resource reachability without typed claim-support closure.
4. Generic BASALT + ASSURE without aviation mapping.
5. Generic BASALT + ASSURE + CEB with the aviation resource/claim/action profile.

Score missed impacted claims, false invalidations, stale evidence accepted, unresolved observer coverage, claim-support precision/recall, successor/fork correctness, evidence reuse, verification actions, cost, calendar time and explanation completeness. Include interacting changes across common lot, batch, machine, campaign and calibration roots; delayed, out-of-order, missing and contradictory events; and digest A→B→A histories.

## 8. External standards and current reference points

- [NASA Aviary](https://github.com/OpenMDAO/Aviary) provides open-source conceptual aircraft sizing/mission analysis and gradient-based optimization. Compare at matched fidelity and budget.
- [NASA/AIAA aeroelastic optimization benchmark paper](https://ntrs.nasa.gov/citations/20240014293) presents a shared high-fidelity optimization case using MPhys and FUN3D and reports the effect of increasing aerodynamic fidelity on optimization wall time.
- [NIST CM4QC Phase II](https://www.nist.gov/programs-projects/cm4qc/cm4qc-phase-ii) is developing realistic aviation AM pathfinder components with extensive in-situ/ex-situ measurement and model-validation challenges.
- [NIST AM-Bench](https://www.nist.gov/ambench) publishes controlled measurement datasets and blind challenge problems; 2025 benchmark results and publications are still distinct from the planned 2028 round.
- [ISO 23247-5:2026](https://www.iso.org/standard/87425.html) specifies a manufacturing digital thread across design, planning, production and testing.
- [NIST IR 8536](https://csrc.nist.gov/pubs/ir/8536/final) is the finalized manufacturing traceability meta-framework (September 2026), emphasizing interoperability and selective disclosure.
- [ISO 23952:2020 QIF](https://www.iso.org/standard/77461.html) was confirmed current in June 2026; it is a quality-information exchange model, not measurement truth.
- [ISO 10303-242:2025 AP242](https://www.iso.org/standard/84300.html) is the current published edition for managed model-based 3D engineering product data.
- [FAA AC 20-107B](https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.information/documentNumber/20-107B) remains active and is an acceptable, non-exclusive means of showing compliance for applicable composite aircraft structures; it is guidance, not itself certification.
- [CMH-17](https://www.cmh17.org/HOME/FAQ) revisions differ by material family and volume. Select the applicable volume/revision rather than claiming a universal handbook baseline.

## 9. Execution sequence

1. Keep v0.1 intact as the initial hierarchy/partition evaluator.
2. Add v0.2 independent specimen and configuration assertions, UTC acquisition-time checks, first-class certificate records and append-only event checks (this branch).
3. Validate the schema and semantic evaluator against the targeted workflow; report exact-head status.
4. Have an independent reviewer author or audit the expected-disposition corpus before claiming oracle accuracy.
5. Inspect the exact current BASALT/ASSURE APIs and qualification states, then implement only the aviation mapper and adapter tests. Do not create another graph, event ledger, invalidation engine or lifecycle.
6. Build a rights-cleared, non-flight-critical physical coupon/metrology campaign with independent lot/batch holdouts and traceable instruments. Register sample units, uncertainty, exclusions, calibration requirements and analysis before testing.
7. Progress through representative joints/elements and subcomponents only after cross-fidelity and uncertainty gates are met.
8. Connect the evidence-qualified material/structural claims to whole-aircraft MDO and installed propulsion/electrothermal/mission closure.

## 10. Limitations and non-claims

- The v0.2 fixture is synthetic and dimensionless.
- The evaluator checks data consistency, not physical truth or cryptographic certificate authenticity.
- It does not produce material design allowables, process capability, NDE probability of detection, fatigue/damage-tolerance substantiation, structural margins, aircraft performance approval or certification.
- A successful software workflow is not physical qualification.
- A signature, digest, source label or ledger attestation is not sufficient evidence by itself.
- No claim is made that this implementation outperforms NASA tools, CMH-17 practice or an industry qualification programme. The competitive proposition must be demonstrated using matched baselines and independently reviewed physical evidence.
