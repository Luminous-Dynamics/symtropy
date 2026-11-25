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
- sampling_event_id (shared across all specimen cuts from one collection event when appropriate) and unique specimen_item_id within that event; source system, source record ID, assertion method, issuer ID and provenance reference. One parent traveler/source_record_id and one sampling_event_id may legitimately cover several specimens.

The evaluator reconstructs ancestry from the material-lot → process-batch → panel → specimen hierarchy and compares it with both the declared and verified fields. It does not derive a source identity from a label, a hierarchy edge, row order or the same record it is supposed to independently check.

One assertion per specimen is required. Sampling-event IDs may be shared by multiple specimen cuts from the same panel/traveler. Specimen-item IDs must be unique; reusing a parent traveler or sampling-event ID is permitted when it legitimately covers multiple cuts. Missing, duplicate, replayed, unresolved or contradictory assertions are not positive evidence.

### 2.2 Independent intended-configuration assertion

The intended_configuration_assertions collection independently identifies the intended product configuration for the specimen. Its declared and verified configuration IDs must agree with each other and the specimen's hierarchy reference.

This is deliberately separate from specimen.configuration_id. A specimen record cannot independently corroborate itself.

### 2.3 Acquisition-time calibration

Every observation contains:

- acquired_at_unix_ms, an unsigned integer count of milliseconds since the Unix epoch;
- time_basis, which must explicitly be UTC_UNIX_EPOCH_MS.

Every test campaign references an instrument, method/protocol revision and calibration certificate. A calibration certificate contains:

- unique certificate ID and distinct calibration_evidence_id (the exact Mycelix AeroEvidence record identity);
- instrument ID and method revision ID;
- calibration laboratory;
- Mycelix packet/artifact reference;
- imported authenticity disposition (authenticated, unauthenticated or unknown);
- valid_from_unix_ms and valid_until_unix_ms.

The interval rule is fixed by this profile:

**[valid_from_unix_ms, valid_until_unix_ms) — start inclusive, end exclusive.**

Thus acquisition exactly at valid_from_unix_ms is inside the interval; acquisition exactly at valid_until_unix_ms is outside it. The integer field is millisecond-precision UTC epoch time, matching Mycelix AeroCommons CalibrationValidityV1 (`valid_from_unix_ms`, `valid_until_unix_ms`, `used_at_unix_ms`) without lossy second-level rounding. Source-native timestamp, timezone and clock evidence must still be preserved in the referenced packet; the integer alone does not authenticate the clock. Unknown time basis, negative/missing time, or future acquisition relative to the evaluation time blocks a positive metrology-binding disposition.

The evaluator requires the certificate instrument to match the campaign instrument and the certificate method revision to match the campaign protocol revision.

### 2.4 Append-only calibration events

The calibration_events collection preserves revoked, suspended, corrected and superseded events, with:

- immutable event ID and certificate ID;
- effective_at_unix_ms;
- recorded_at_unix_ms;
- explicit retroactive boolean;
- for corrected / superseded, a corrects_event_id reference.

Events are evaluated as known at evaluated_at_unix_ms. A revocation recorded after acquisition but effective before evaluation blocks positive binding and requires an accountable disposition. A retroactive event also blocks affected historical observations. Corrections must point to a prior event for the same certificate; competing corrections to the same event are treated as contradictory history and block.

This is intentionally conservative: the evaluator does not decide whether a revoked certificate invalidates every earlier physical result. It preserves the event and abstains until the responsible metrology/engineering authority establishes the applicable disposition.

### 2.5 Artifact and authenticity boundary

Certificate bytes, packet digests and byte lengths remain owned by Mycelix. The certificate_artifact_ref is a reference, not a second artifact store. authenticity_status is an imported assertion; v0.2 does not perform PKI validation, establish issuer authority, inspect instrument firmware, verify acquisition-clock trust or prove sample identity in the physical world.

## 3. Rust evaluator and report semantics

The v0.2 evaluator aligns calibration-window fields with Mycelix's millisecond-precision half-open interval contract. It does not claim the Mycelix packet PR is merged or production-qualified.

The v0.2 evaluator lives in crates/domains/symtropy-aero-material-qual/src/evidence_binding_v0_2.rs and is exposed as evidence_binding_v0_2::evaluate_binding_case. The existing aero-material-qual CLI dispatches to this evaluator when the input profile_id is aero-evidence-binding-v0.2; the targeted workflow smoke-tests both v0.1 and v0.2 reports.

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

The 31-case frozen mutation corpus is executed by the Rust tests with exact violation and blocker sets. The expanded chronology cases cover as-of evaluation-time handling, retroactive revocation, cross-certificate correction rejection, duplicate event identity, strict correction chronology, invalid intervals and unknown references/events. The evaluator also tests calibration-event permutation invariance with a non-empty competing-correction history. This is synthetic software testing only; expected dispositions remain NOT_INDEPENDENTLY_REVIEWED and are not a physical or metrology oracle. Independent review and a rights-cleared external corpus remain required before any external performance claim.

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

### 4.8 Inspectability and sensing integrity are first-class design variables

NASA Langley's Design-for-Inspection (DFI) work proposes an automation-friendly inspectability metric built from simulation tests for defect findability and sensitivity to variability, designed to cover parameter space with fewer simulations. Use it as a methodology comparator and co-design target, not as a claim that this project reproduces NASA's implementation: [NASA DFI framework, NTRS 20240003953](https://ntrs.nasa.gov/citations/20240003953) and [Inspectability Metric, NTRS 20240007593](https://ntrs.nasa.gov/citations/20240007593).

The inspection design variables should be coupled to structure and manufacturing: access, geometry, joint design, defect families, flaw orientation/size/depth, NDE method, coverage, probability-of-detection evidence, threshold of detection, calibration, repair limits, and inspection/repair time. Report inspectability and NDE coverage as separate vectors alongside structural mass, stiffness, fatigue/damage tolerance, manufacturability and lifecycle cost. Do not replace a required inspection acceptance criterion with an aggregate inspectability score.

A further comparator is the June 2026 sensing digital twin paper for aerospace SHM. It separates structural damage from sensor faults, reconstructs sensor-fault-corrupted signals and estimates channel reliability. Its demonstration uses a simulated carbon/epoxy composite wing panel with injected bias, drift, gain, saturation, dropout and stuck-sensor faults; the authors describe model-fidelity limitations. It is an important sensing-layer research baseline, not a physical aircraft validation or transferable SHM guarantee: [Di Fiore et al., 2026](https://link.springer.com/article/10.1007/s00158-026-04324-8).

Luminous should model four competing explanations for a changed signal: (1) structural change/damage; (2) sensing-chain fault; (3) environmental/operational-state change; (4) unknown/out-of-domain. Preserve the original raw observation, calibration, acquisition time and provenance immutably. Any corrected/reconstructed signal is a new derived artifact linked to its raw parent, algorithm/model version, uncertainty and sensor-reliability assessment. A correction may not overwrite the raw sample, downgrade a damage indication, or qualify a claim by itself. Where structural and sensor hypotheses cannot be separated, abstain and request independent NDE/inspection.

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

### Current external benchmark and standards notes (reviewed 25 November 2026)

- **NASA Aviary / OpenMDAO:** Aviary is an actively developed aircraft analysis, design and optimization tool for conceptual aircraft design. Compare aircraft sizing, mission analysis, analytic gradients, optimization convergence and evaluation cost only under matched aircraft/mission/subsystem definitions; it is not a substitute for the high-fidelity aeroelastic benchmark. Official documentation: https://openmdao.github.io/Aviary/dev/introduction.html
- **High-fidelity aeroelastic optimization:** NASA Langley's first AIAA High-Fidelity Aeroelastic Optimization Benchmark case uses MPhys/OpenMDAO and FUN3D. The benchmark exists because differing baselines, variables, objectives and constraints make literature comparisons difficult. That paper reports a fidelity progression that reduced wall time in its studied case; treat this as case-specific evidence, not a universal fidelity-cost law. https://ntrs.nasa.gov/api/citations/20240014293/downloads/ae_benchmark_opt_scitech_2025.pdf
- **NIST CM4QC Phase II / AM-Bench:** the pathfinder is planning realistic aviation additive-manufactured components with extensive in-situ/ex-situ measurement and model-validation challenges. NIST says AM-Bench 2025 work has been completed but final data publications remain in progress; its AM-Bench 2028 plan shifts toward higher-TRL industrial metal components. Treat these as a developing external challenge opportunity, not as an already available public composite-allowables dataset. https://www.nist.gov/programs-projects/cm4qc/cm4qc-phase-ii and https://www.nist.gov/ambench
- **QIF:** ISO 23952:2020 was reviewed and confirmed current on 14 June 2026. It defines a quality-information model and XML workflows; format conformance is not measurement truth. https://www.iso.org/standard/77461.html
- **Product definition:** ISO 10303-242:2025 Edition 4 was published in August 2025 and is now in the review/revision track. Pin the exact edition and application profile in every comparison. https://www.iso.org/standard/84300.html
- **Manufacturing digital thread:** ISO 23247-5:2026 was published in June 2026 and specifies digital-thread principles across design, planning, production and testing. https://www.iso.org/standard/87425.html
- **Supply-chain traceability:** NIST IR 8536 was finalized on 9 September 2026. It provides an interoperable, industry-neutral meta-framework for linking and querying traceability data while supporting selective disclosure; NIST's September release described its reference implementation as still under final review. Align to its concepts without treating the reference implementation as a qualified aviation product. https://csrc.nist.gov/pubs/ir/8536/final
- **Composite substantiation:** FAA AC 20-107B remains listed active and is an acceptable, non-exclusive means of showing compliance for applicable composite aircraft structures. It is guidance, not mandatory regulation or a standalone approval. https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.information/documentNumber/20-107B
- **CMH-17 material-family discipline:** use the applicable handbook volume/revision. Volume 3 Revision H (October 2025) covers polymer-matrix composite usage/design/analysis; Volume 5 Revision B (August 2026) covers ceramic-matrix composites. The proposed Volume 7 for non-metallic additive manufacturing (including FFF and LPBF focus) is not yet a published handbook baseline. Do not transfer allowables or statistical methodology across volumes/material families. https://www.wichita.edu/industry_and_defense/NIAR/cmh-17/cmh-17.php and https://sae-eu.org/composite-materials-handbook-volume-5-gets-major-update-expanding-critical-guidance-and-data-for-ceramic-matrix-composites/

### NDE, damage-tolerance and South African authority context

- **ASTM E2862-23** is a hit/miss probability-of-detection analysis practice for a specific set of examination parameters. It is a useful statistical comparison for detection studies, but it is not a universal inspectability score and does not replace a physically representative NDE qualification: https://store.astm.org/e2862-23.html
- **MIL-HDBK-1823A** is active NDE system reliability assessment guidance in DLA ASSIST. It states that it is guidance, not itself a requirement: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=206122
- **FAA AC 25.571-1D** is an active Part 25 transport-category fatigue/damage-tolerance reference. It is relevant only where the aircraft/product certification basis makes Part 25 applicable; it must not be generalized to every aircraft class: https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentid/865446
- **South African certification pathway:** SACAA's Engineering Unit publishes current technical guidance covering type certification, type acceptance, experimental certificates of airworthiness, design-organisation approval and related engineering processes. The exact path must be determined with SACAA and qualified specialists for the intended aircraft category, design/production role and operation. A foreign advisory circular, benchmark pass or internal evidence receipt does not establish SACAA acceptance or an airworthiness certificate: https://www.caa.co.za/industry-information/airworthiness/ (verify the current controlling TGM revision at project entry).

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
