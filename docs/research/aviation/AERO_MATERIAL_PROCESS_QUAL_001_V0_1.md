# AERO Material / Process Qualification v0.1

**Issue:** [AERO-MATERIAL-PROCESS-QUAL-001 #7299](https://github.com/Luminous-Dynamics/symtropy/issues/7299)  
**Status:** synthetic-first research contract; no physical material qualification  
**Owner:** Symtropy benchmark and physical-model interface  
**Composition:** Symthaea engineering reasoning and experiment selection; Mycelix manufactured-unit and process-evidence lineage; Sol-Atlas source rights and operating context  
**Claim ceiling:** schema and fixture integrity only; no material allowable, process capability, structural qualification, part acceptance, airworthiness or certification claim.

## 1. Research question

Can we preserve the applicability and uncertainty of evidence as it moves through:

`material lot → process batch → panel → specimen/coupon → element/joint → subcomponent`

The key distinction is that a traceable coupon result is not automatically a design allowable, and a coupon from one material/process/configuration does not silently qualify another. Manufacturing variation and sampling dependence are part of the engineering state, not post-processing nuisances.

## 2. Paradigm shifts

1. **Process-conditioned material evidence.** A measured property is conditional on supplier and lot, storage/handling, process route/revision, machine/tooling, site, geometry/layup, conditioning, test method, calibration and exact configuration.
2. **Hierarchical experimental identity.** Specimens from one panel, panels from one batch and batches from one campaign are correlated groups unless a justified model says otherwise. Row count is not independent sample count.
3. **Explicit scale transfer.** Coupon → joint/element → subcomponent is a sequence of separately scoped claims. Every transfer names its parent evidence, model, domain, assumptions, uncertainty and remaining physical validation.
4. **Design for inspectability and repair.** Structural mass and stiffness are optimized alongside defect detectability, access, NDE coverage, repairability, process window and lifecycle burden.
5. **Decision-value-based testing.** Select the next test or inspection by the effect it may have on the engineering decision, but do not relax a hard acceptance criterion to save resources.
6. **Honest abstention.** Out-of-domain, incomplete, contradictory, poorly calibrated or statistically unsupported evidence produces UNKNOWN / NOT_COVERED / INDETERMINATE, never a convenient default pass.

## 3. Existing owners to compose

- Symtropy [AERO-AIRFRAME-001 #3921](https://github.com/Luminous-Dynamics/symtropy/issues/3921): AER-01 aircraft architecture and design portfolio.
- Symtropy [AERO-WHOLE-AIRCRAFT-CLOSURE-001 #3945](https://github.com/Luminous-Dynamics/symtropy/issues/3945): mass, CG, inertia, aero, propulsion, thermal and mission closure.
- Symtropy [AERO-INSPECTION-OBSERVABILITY-001 #4165](https://github.com/Luminous-Dynamics/symtropy/issues/4165): design-for-inspection, NDE and SHM.
- Symtropy [AERO-PORTFOLIO-MDO-001 #5853](https://github.com/Luminous-Dynamics/symtropy/pull/5853): matched whole-aircraft architecture MDO.
- Symthaea [TRANSPORT-FIXED-WING-001 #6249](https://github.com/Luminous-Dynamics/symthaea/issues/6249): fixed-wing aerodynamics, flight dynamics, loads and applicability.
- Symthaea [AIRROAD-VERIF-001 #11445](https://github.com/Luminous-Dynamics/symthaea/issues/11445): cross-fidelity validation and promotion gates.
- Symthaea [structural evidence-action selector #13112](https://github.com/Luminous-Dynamics/symthaea/pull/13112): decision-valued analysis / inspection proposals.
- Mycelix [MFG-EVID-001 #3084](https://github.com/Luminous-Dynamics/mycelix/issues/3084): physical-product, manufactured-unit and qualification lineage.
- Mycelix [AEROCOMMONS-COMPONENT-PILOT-001 #6345](https://github.com/Luminous-Dynamics/mycelix/issues/6345) and [implementation PR #6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347): non-flight-critical build/inspect/test packet structure.

This work does not create a second MDO optimizer, FEM/CFD solver, quality system, provenance ledger, materials database or certification engine.

## 4. v0.1 case contract

The first case is intentionally synthetic and dimensionless. Its response values are **not MPa, modulus, strength, allowable, or any real material property**.

The JSON Schema and fixture in this directory define:
- exact material-lot, process-batch, panel, specimen, test-campaign and product-configuration identities;
- observed, censored, excluded and missing measurements without deleting unsuccessful or incomplete records;
- calibration state and process-domain disposition;
- train / holdout / challenge specimen assignments;
- grouping dimensions that the evaluator must check for disjointness;
- a known synthetic data-generating model that is never treated as a physical model.

A structurally valid JSON document is not sufficient to establish hierarchy closure, independent sampling or model applicability. The v0.1 Rust semantic evaluator now checks unique identities, hierarchy and observation foreign keys, specimen assignment completeness, observation-state requirements, and group-level split disjointness derived from the lineage graph. It reports out-of-domain process states and non-active calibration labels as blockers. The machine-readable report keeps structural completeness, process-domain status, model-applicability status, evidence-sufficiency status, decision eligibility and physical-qualification status separate; model applicability and evidence sufficiency are explicitly NOT_EVALUATED_V0_1. It does **not** run a standards-compliant Draft 2020-12 JSON Schema validator or verify calibration validity intervals against measurement timestamps.

Three important checks are not representable in the current v0.1 schema and are deliberately reported as capability gaps: (1) an independently asserted specimen source-material identity that can be reconciled against panel/batch/lot ancestry; (2) an independently sourced intended product-definition/configuration assertion that can be compared with the specimen's recorded configuration; and (3) calibration validity start/end times tied to each measurement timestamp. Artifact digest and byte-length binding remains with the Mycelix packet layer. These must be closed before wrong-lot, wrong-intended-configuration or time-stale-calibration detection can be claimed.

## 5. Minimum evaluator obligations

### Identity and lineage

- Reject duplicate or blank identities and dangling references.
- Recompute each specimen's lot, batch, panel, campaign and configuration ancestry.
- Do not infer identity from row order or display labels.
- Keep component definition, physical unit, evidence artifact, process execution and observation identities distinct.
- Preserve artifact digest and byte-length verification as a separate Mycelix responsibility.

### Evidence state

- Keep OBSERVED, CENSORED, EXCLUDED and MISSING records visible with reason/limit fields.
- An expired or unknown calibration cannot silently qualify an observation.
- A test result and a calibration attestation do not establish the physical truth of the measurement by themselves.
- Missing evidence is UNKNOWN, not zero.
- Process out-of-domain status is not rescued by a statistically attractive response value.

### Statistical unit and leakage

- Derive group memberships from the lineage graph, not caller-supplied split labels alone.
- Check disjointness at every declared grouping level (material lot, process batch, panel, campaign, configuration).
- Reject specimen-level random splits when any declared higher-level group crosses partitions.
- Preserve a census of offered, eligible, observed, censored, excluded, missing and challenge records.
- Future physical studies must preregister the experimental unit, cluster hierarchy, exclusions, confirmatory split and statistical method before observing outcomes.

### Claim promotion

Keep these states independent:
1. STRUCTURALLY_COMPLETE — required records and references exist.
2. MODEL_APPLICABLE — declared physical/model domain covers the case.
3. EVIDENCE_SUFFICIENT — quantity, calibration, sampling and independent evidence obligations are met for the specific claim.
4. DECISION_ELIGIBLE — hard requirements, uncertainty and relying-party policy allow the proposed decision.
5. PHYSICALLY_QUALIFIED / APPROVED — remains outside this software contract and belongs to competent engineering organizations, laboratories and aviation authorities.

No later state follows automatically from an earlier state.

## 6. Benchmark design and baselines

Use a common frozen case, corpus, data budget and scoring budget for all methods.

| Baseline | Question |
|---|---|
| Catalogue / handbook nominal value | What does a conventional value-only approach conclude, and what applicability is hidden? |
| Sequential building-block workflow | What does the conventional coupon → element → subcomponent chain conclude? |
| Process-conditioned deterministic envelope | Does an explicit bounded process/evidence envelope prevent unsupported transfer? |
| Hierarchical / mixed-effects model | Does a justified cluster model improve estimation and calibration on held-out groups? |
| Distribution-free bounded method | What can be concluded without strong distributional assumptions? |
| Adaptive evidence selection | Does decision-value-based testing reduce total test effort at matched decision quality? |

Do not assume Bayesian, machine-learning or adaptive methods win. Compare against simple baselines and preserve a no-model / abstain option.

## 7. Prospective acceptance targets

These are research objectives for the finite benchmark, not regulatory criteria or achieved results.

- 100% agreement with an independently authored oracle on bounded synthetic cases.
- Zero false passes on frozen dangling/unknown-configuration-reference, dangling-ancestry, out-of-domain, censoring/missingness and leakage controls in the v0.1 semantic corpus; wrong-lot, wrong-intended-configuration and time-stale-calibration controls remain v0.2 requirements until independent source-identity and calibration-time fields exist.
- 100% deterministic output under input-record permutation.
- Zero overlap across declared training, holdout and challenge groups at each grouping level.
- 100% explicit retention and accounting of censored, excluded, missing and failed records.
- 100% traceability from each metric to the specimen, panel, process batch, material lot, campaign, configuration and source artifact.
- 100% explicit UNKNOWN / NOT_COVERED / INDETERMINATE when evidence is unsupported.
- Report false-accept and false-reject counts separately. Report denominators, exclusions, subgroup performance, interval coverage/width, abstention, compute and evidence cost.
- Only after rights-cleared independent physical holdouts exist, consider a preregistered nominal 95% interval-coverage target by batch/process/environment stratum. Do not claim calibration from synthetic data or underpowered samples.

No composite score may trade a hard engineering requirement against mass, cost, speed or yield.

## 8. Industry and standards comparison

The benchmark should interoperate with established practice, not claim to replace it:

- **FAA AC 20-107B** is active guidance and an acceptable, non-exclusive means of showing compliance for composite aircraft structures under applicable 14 CFR parts; it is guidance, not itself a qualification or certification. https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentID/99693
- **CMH-17** building-block methodology is a key reference for progressing through material characterization, coupons, elements, joints and subcomponents. Use the exact applicable volume, revision and statistical method; do not infer allowables from the synthetic corpus.
- **ISO 10303-242 (STEP AP242)** carries product definition and configuration information.
- **ISO 23952 (QIF)** carries quality / metrology information.
- **ISO 23247-5:2026**, published 24 June 2026, specifies a manufacturing digital thread across design, planning, production and testing: https://www.iso.org/standard/87425.html
- **NIST IR 8536**, published September 2026, provides a manufacturing supply-chain traceability meta-framework intended to connect diverse systems while preserving their existing standards: https://csrc.nist.gov/pubs/ir/8536/final

For broader portfolio context:
- NASA HEMM lists 16 kW/kg and 99% as development targets for a 1.4 MW aircraft motor, not an installed propulsion-package achievement: https://www.nasa.gov/eap-technology/electric-machines/hemm/
- An August 2026 eVTOL mission-level electrothermal study reports thermal-management energy above 20% in its own studied cases; this is a study-specific result, not a universal design constant: https://doi.org/10.1016/j.applthermaleng.2026.132253
- EASA MOC-5 SC-VTOL Issue 2 is dated 5 November 2026 and applies to VTOL-capable aircraft certification means of compliance, not generic fixed-wing composite qualification: https://www.easa.europa.eu/en/downloads/144423/en


### Current revision and applicability notes (reviewed 24 November 2026)

- Wichita State / NIAR's CMH-17 publication update reports Volume 3 Revision H released in October 2025. SAE lists Volume 5 Revision B published 27 August 2026. These are different material families; do not treat one volume/revision as a universal composite profile. The CMH-17 roadmaps explicitly cover new-material qualification, lamina-allowable development, equivalence for revised material/process and second sources, bolted/bonded joints, and repairs. Our benchmark should map to the applicable roadmap but must not reproduce or substitute for the handbook's material statistical methods or proprietary data. See https://www.wichita.edu/industry_and_defense/NIAR/cmh-17/cmh-17.php, https://www.cmh17.org/HOME/FAQ and https://saemobilus.sae.org/books/composite-materials-handbook-volume-5-revision-b-r-460.
- ISO reports ISO 23952:2020 (QIF) was reviewed and confirmed current on 14 June 2026. It is an information model and exchange format; a valid QIF document does not itself establish measurement validity. https://www.iso.org/standard/77461.html
- ISO 23247-5:2026 was published on 24 June 2026. It defines a manufacturing digital thread across design, planning, production and testing. Our Mycelix handoff should interoperate with this thread while leaving engineering evidence appraisal to the appropriate owner. https://www.iso.org/standard/87425.html
- NIST IR 8536 was finalized on 9 September 2026. Its contribution is interoperable manufacturing traceability and selective disclosure across existing systems; it is not a composite qualification method or physical acceptance authority. https://csrc.nist.gov/pubs/ir/8536/final
- FAA AC 20-107B remains listed as active. It is an acceptable, non-exclusive means of showing compliance for composite aircraft structures under the applicable 14 CFR parts, and provides associated design/manufacturing/maintenance guidance. It is guidance, not mandatory regulation and not a standalone approval. https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentID/99693
- EASA MOC-5 SC-VTOL Issue 2 is dated 5 November 2026. It is relevant to VTOL-capable aircraft certification means of compliance; do not apply it indiscriminately to a conventional fixed-wing/eSTOL configuration. https://www.easa.europa.eu/en/downloads/144423/en
- NASA's 1.4 MW HEMM page publishes 16 kW/kg and 99% efficiency as development targets. Our comparison must keep motor-only metrics separate from installed motor/inverter/gearbox/cooling/harness/containment package metrics. https://www.nasa.gov/eap-technology/electric-machines/hemm/
- Nakayama et al. (August 2026) report thermal-management energy above 20% in their particular mission-level eVTOL study cases. Treat this as a reason to model TMS energy and seasonal conditions explicitly, not as a universal aircraft design constant. https://doi.org/10.1016/j.applthermaleng.2026.132253

### Benchmark comparators and competitive positioning

Use established tools and guidance as matched baselines, not straw men:

- **Aircraft MDO:** NASA's open-source [Aviary](https://github.com/OpenMDAO/Aviary) is a direct aircraft analysis/design/optimization comparator, including analytic-gradient methods. Compare on the same mission, configuration, discipline models, constraints, derivative quality, convergence tolerances and evaluation budget. A different fidelity or mission invalidates a headline speed/weight comparison.
- **High-fidelity aeroelastic optimization:** use the [NASA/AIAA aeroelastic optimization benchmark](https://ntrs.nasa.gov/citations/20240016218) where its configuration and scope match. Its motivation is directly relevant: inconsistent baselines, design variables, objectives and constraints make published optimizer comparisons difficult.
- **Model-assisted material qualification:** align to [NASA/NIST CM4QC Phase II](https://www.nist.gov/programs-projects/cm4qc/cm4qc-phase-ii) and [NIST AM-Bench](https://www.nist.gov/programs-projects/metrology-am-model-validation) style datasets and challenge cases as rights-cleared reference data become available. Do not claim access to unpublished data or imply that the Luminous synthetic fixture is comparable to physical round-robin evidence.
- **Composite structures:** retain applicable [FAA AC 20-107B](https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.information/documentNumber/20-107B) and CMH-17 building-block practice as the engineering baseline. A digital thread or AI score does not replace substantiation by test and analysis.

The competitive hypothesis is therefore not that a new model beats every specialist CFD, FEM, materials or MDO tool. It is that an evidence layer can connect their outputs to exact configurations, manufacturing lineage, metrology, uncertainty, applicability and change history—and then select the next evidence action without overstating what has been established.

### Benchmark scorecard and prospective targets

Report a vector of metrics, never a single weighted score:

| Dimension | Benchmark | Prospective research target |
|---|---|---|
| Safety / decision error | False acceptance and false rejection by claim family; adversarial negative controls | Zero false acceptance on the bounded, preregistered challenge corpus |
| Applicability | Whole-lot, whole-batch, whole-site, whole-machine, whole-recipe and configuration holdouts | No promotion outside the validated domain; explicit abstention |
| Uncertainty | Prediction-interval coverage and width on independent physical holdouts | For nominal 95% intervals, assess observed coverage against a preregistered tolerance such as 93–97% only with adequate independent sample size |
| Traceability | Claim-to-unit / process / configuration / calibration / source-artifact path completeness | 100% traceability for every reported claim |
| Reproducibility | Input permutation, independent evaluator, frozen toolchain and corpus digest | 100% deterministic outputs for deterministic inputs |
| Test efficiency | Test count, independent lots/batches, lab cost, engineering hours and calendar time at matched acceptance risk | Aim for at least 20% lower total verification effort only after a physical, independently held-out comparison |
| Model transfer | Coupon → element/joint → subcomponent error and uncertainty by scale | No scale promotion without its own validation and explicit transfer uncertainty |

These are research targets, not regulatory criteria or achieved results. In particular, fewer tests are not an improvement if false-accept risk, uncertainty, independent-group coverage or hard-requirement coverage deteriorates.

## 9. Next physical evidence ladder

1. Synthetic contract and evaluator.
2. Non-flight-critical coupon / calibration fixture with independent metrology.
3. Coupon campaign across explicitly independent lots and process runs.
4. Representative joint / element tests.
5. Subcomponent tests with realistic load paths and boundary conditions.
6. Aircraft structural substantiation and certification evidence under accountable engineering and authority processes.

The physical ladder cannot be shortcut by a successful software test, simulation, a ledger attestation or an AI recommendation.

## 10. Reproduction status

v0.1 at this branch contains the research protocol, strict JSON Schema, known-truth synthetic case, and a Rust semantic evaluator crate at `crates/domains/symtropy-aero-material-qual`. The evaluator includes unit tests for baseline closure, duplicate identity, dangling ancestry, split leakage, missing assignments, non-active calibration blockers, record-order determinism and explicit capability gaps. These tests are authored but have **not** been compiled or run in this authoring pass; exact-head CI status is reported separately. The evaluator does not perform full Draft 2020-12 schema validation. Its capability-gap report explicitly flags missing independent material-lot and intended-configuration assertions, plus time-bounded calibration binding. No physical result, material allowable, manufacturing capability, part acceptance, airworthiness or certification claim is included in v0.1.
