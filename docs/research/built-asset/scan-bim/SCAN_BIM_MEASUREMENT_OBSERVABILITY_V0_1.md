# Scan-to-BIM Measurement Observability Contract v0.1

**Status:** Research proposal; no sensor calibration or physical measurement qualification is claimed.  
**Issue:** [#4589 — SCAN-BIM-MEASUREMENT-OBSERVABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/4589).  
**Programme:** Evidence-Native Built Environment Intelligence.

## 1. Purpose

Separate three claims that are often compressed into a single visibility or confidence flag:

1. **Geometric visibility:** the reference surface is reachable by a line of sight under the declared pose, field-of-view and occluder assumptions.
2. **Measurement detectability:** the exact sensor/configuration, operating conditions and target interaction support a valid measurement with characterized detection and error behaviour.
3. **Decision sufficiency:** the resulting measurement evidence is suitable for one named engineering task and tolerance.

The sequence is not implication-preserving by default:

`visible \not\Rightarrow detected \not\Rightarrow accurate \not\Rightarrow decision\text{-}sufficient`

A missing return may be caused by occlusion, a sensor operating limit, a surface interaction, motion, processing, or missing evidence. It must not automatically be interpreted as geometric absence. Likewise, a visible target is not automatically a valid measurement.

## 2. Ownership map

| Concern | Existing owner | This proposal consumes |
|---|---|---|
| Sensor session, calibration, coordinate frames, registration and measurement uncertainty | #1607 | Exact survey/session/calibration and uncertainty references |
| Geometric visibility and occlusion | #4292 | Evaluator visibility receipt and reference-geometry digest |
| Surface sampling and represented-area accounting | #4225 | Surface/sample identity and area evidence |
| Candidate/evaluator isolation and reproducible replay | #4116 | Split, lineage, artifact and replay receipts |
| Reconstruction, element correspondence, topology, change and admission | #4086 | Candidate element/task profile and proposed decision |
| Value of information and action choice | #1892 | Measurement costs, feasible actions and decision-impact vector |
| Component continuity | #3331 | Stable component identity and lifecycle references |
| IFC exchange and independent openBIM validation | #4271 | Interoperability result; not evidence of metrology by itself |

This work does not add another ray engine, survey registry, calibration authority, identity service, change-event ledger, measurement planner, IFC parser, IDS engine or model mutation mechanism.

## 3. Evidence model

### 3.1 Three orthogonal axes

A scene element or sampled region has independent records for:

- **Visibility:** `OBSERVABLE`, `SELF_OCCLUDED`, `EXTERNALLY_OCCLUDED`, `OUTSIDE_SENSOR_COVERAGE`, `UNKNOWN`, `CONTRADICTORY`.
- **Observation:** `CAPTURED`, `NOT_CAPTURED`, `INVALID_RETURN`, `MIXED_RETURN`, `CONTRADICTORY`, `UNKNOWN`.
- **Qualification:** `PASS`, `FAIL`, `UNRESOLVED`, `NOT_RUN`, with explicit reason codes and claim scope.

These axes must not be substituted for one another. For example, `OBSERVABLE + NOT_CAPTURED` is different from `OBSERVABLE + INVALID_RETURN`; both differ from `UNKNOWN + UNKNOWN`.

The observation record retains the actual raw or losslessly referenced measurement, source identity, capture sequence/time, sensor configuration, pose revision, frame, units, and upstream artifact digests. A digest establishes byte identity, not measurement truth.

### 3.2 Sensor-specific response profile

A sensor profile is bound to an exact hardware identity or explicitly declared hardware class, firmware/software version, configuration, calibration evidence and tested operating domain.

For TLS v0, the operating domain should record available and applicable variables such as:

- range and incidence angle;
- return intensity/amplitude or other instrument-specific response descriptors;
- target material / reflectance class where independently characterized;
- beam footprint and target size/edge proximity;
- surface curvature and mixed-pixel/edge conditions;
- sensor pose, motion and registration evidence;
- environmental conditions actually measured;
- scan resolution, pulse/return mode and filtering configuration.

Do not invent absent variables, infer a material from an RGB label without a validated method, or assume all scanners share one response model. If a predictor or sensor does not expose a relevant variable, record it as unavailable and constrain the applicability claim.

### 3.3 Measurement uncertainty

A measurement-quality report must make its uncertainty representation explicit: standard uncertainty/covariance, expanded uncertainty with stated coverage factor, empirical interval, conservative bound, or another versioned method.

Retain distinct contributors where the evidence supports them:

- instrument/range and angular measurement;
- calibration and reference-target uncertainty;
- sensor pose and coordinate transformation;
- registration and control network;
- target response, incidence and footprint/mixed-edge effects;
- motion and environmental effects;
- numerical, sampling and processing effects.

Do not assume contributors are independent. Where spatial, temporal or cross-view correlations are material, preserve the covariance/correlation model or a conservative bound. If an input contributor is unknown, the output may be unresolved; it must not be silently set to zero.

## 4. Typed record sketch

The following is a conceptual interface, not an implemented Rust API or finalized JSON Schema.

```text
MeasurementEvidence {
  subject: {
    building_id,
    element_id,
    surface_id,
    geometry_revision,
    task_profile_id
  },
  source: {
    survey_session_id,
    sensor_id,
    sensor_profile_digest,
    firmware_and_configuration_digest,
    capture_id,
    capture_time,
    pose_revision,
    coordinate_frame_id
  },
  upstream: {
    visibility_receipt_digest,
    sampling_receipt_digest,
    survey_evidence_digest,
    raw_observation_artifact_digest
  },
  conditions: {
    range_m?,
    incidence_angle_rad?,
    footprint_m?,
    response_descriptors?,
    environment_descriptors?,
    applicability_domain_id
  },
  observation: {
    status,
    measurement_artifact_digest?,
    measurement_units?,
    measurement_values_or_reference?,
    invalid_or_missing_reason?
  },
  uncertainty: {
    method_id,
    representation,
    coverage_probability?,
    coverage_factor?,
    correlation_model_digest?,
    independent_validation_set_id?
  },
  qualification: {
    visibility_status,
    detection_status,
    metrology_status,
    task_sufficiency_status,
    reason_codes,
    exclusions,
    claim_scope
  },
  computation: {
    implementation_id,
    implementation_revision,
    configuration_digest,
    deterministic_receipt_digest
  }
}
```

The eventual schema must be closed/versioned, reject unknown critical fields, bound cardinality and payload size, and canonically bind all decision-relevant inputs. A value copied from a producer must be checked against the exact producer receipt and artifact digest rather than trusted because it appears in a caller-authored JSON object.

## 5. Qualification ladder

| Gate | Question | What a pass establishes |
|---|---|---|
| G0 — integrity | Are exact inputs, profiles, revisions and artifacts identified? | Evidence is bound to the declared inputs |
| G1 — visibility | Is the geometric visibility result independently derived and complete for its stated scope? | The stated geometric visibility classification |
| G2 — detectability | Is the sensor's valid-return behaviour characterized for this operating domain? | Empirical detectability only within the tested scope |
| G3 — metrology | Is the error/uncertainty model validated against independent references? | Measurement uncertainty under stated assumptions |
| G4 — task sufficiency | Does the validated uncertainty meet the named task tolerance/detection requirement? | Suitability for that task and scope |
| G5 — admission | Does the separate building-state admission policy authorize the proposed update? | Only the policy-scoped candidate/admission outcome |

No later gate is inferred from an earlier gate. A hard failure or unresolved required gate blocks qualification. Qualification does not provide regulatory, structural or physical-work authority.

## 6. P0 deterministic contract oracle

Before connecting real hardware, implement finite known-answer fixtures for:

1. Visible, in-domain surface with a valid measurement.
2. Visible but outside the qualified measurement range.
3. Grazing incidence with a declared degraded or unresolved response.
4. Two target response classes with intentionally different reference error/detection profiles.
5. Mixed-material boundary and beam-footprint edge effects.
6. Geometric occlusion versus visible target with a missing/invalid return.
7. Stale calibration, configuration, pose or transform revision.
8. Pose/registration uncertainty that makes a task tolerance fail.
9. Correlated views and duplicate capture lineage.
10. Unknown material or environment outside the qualified domain.
11. One observation adequate for coarse layout but inadequate for a tighter installation tolerance.
12. Change whose uncertainty crosses the declared level-of-detection threshold.
13. Missing, contradictory or stale producer receipts.
14. Malformed covariance, non-finite values, invalid units, overflow, oversized inputs and resource-budget exhaustion.
15. Input permutation and deterministic receipt replay.

The oracle must test reason codes and full dispositions, not only a single Boolean. Synthetic success is contract evidence only.

## 7. P1 physical qualification design

Begin with a bounded terrestrial laser scanner profile. This is not a claim about all LiDAR systems, photogrammetry, structured-light cameras or mobile mapping.

Use a calibrated reference artefact or independently surveyed target assembly. Plan a controlled set of captures over range, incidence angle, target response/material, surface size/curvature, edge proximity, sensor settings and repeat sessions. Record environmental variables where they are relevant and measurable.

Separate:
- detection/valid-return frequency;
- false, multipath, mixed-edge and invalid-return behaviour;
- conditional measurement bias and dispersion when a return is valid;
- completeness/point density and their spatial distribution;
- propagated uncertainty for an explicitly named geometric or change metric.

Keep reference metrology uncertainty in the evaluation. Separate calibration/tuning captures from group-held-out test captures by target and survey session. Correlated points and views from one acquisition are not independent calibration samples. A profile that has not been tested on a material, range or condition is out-of-domain, not implicitly qualified.

Use established instrument field-testing and metrology methods as references. Do not claim that this experiment replaces ISO 17123 procedures, an accredited calibration laboratory or competent survey practice.

## 8. P2 task-conditioned qualification

Use one measurement set to evaluate multiple declared task profiles. Examples:

- coarse room and wall layout;
- opening boundary and dimension extraction;
- MEP clearance;
- installation tolerance;
- defect or physical-change detection;
- area/volume estimation for repair or retrofit.

Each task declares its own required tolerance, minimum detectable change, spatial scope, uncertainty method and decision policy. A measurement may pass for one task and fail or remain unresolved for another.

A downstream result must expose residual unknown regions and the evidence required to resolve them. It must not turn missing observations into completed geometry.

## 9. Benchmark protocol

Report these tracks independently:

| Track | Measures |
|---|---|
| Visibility | Oracle agreement; false-observable rate; unknown/contradictory area |
| Detection | Valid-return frequency; false/multipath/invalid return rates by stratum |
| Metrology | Bias, repeatability, error distributions and independent-checkpoint residuals |
| Uncertainty | Empirical interval coverage, interval width and group-wise calibration |
| Robustness | Out-of-domain detection, abstention and false qualification |
| Decision | Task-specific pass/fail/unresolved; false acceptance and false rejection |
| Acquisition | Decision-risk reduction, capture time, processing time and human review burden |

### P0 hard gates

- 100% exact expected disposition/reason agreement on the frozen finite oracle.
- Zero false `PASS` outcomes on seeded missing-calibration, stale-profile, unsupported-domain, hidden-target and tolerance-violation controls.
- Zero conversion of visibility-only evidence into a qualified measurement.
- Zero double-counting of duplicated capture lineage as independent validation evidence.

### P1 empirical pre-registration

First reproduce a sensor-specific baseline. Then pre-register calibration/test group splits, the nominal interval coverage (for example, 90% and 95% where appropriate), acceptable interval width per task, detection-rate strata, false-qualified rate and sample-size rationale.

Report interval coverage and width together, confidence intervals, target/session counts, all exclusions and failures. Do not use one aggregate expected-calibration-error statistic as proof of metrological calibration. No numeric improvement claim is allowed until an equivalent baseline has been reproduced with matched hardware, settings, targets, environment, reference uncertainty and compute.

## 10. Next-best-view research

GO-PRE (ICML 2026) motivates optimizing information gain in the prediction space. The Luminous experiment should extend the comparison to **decision consequences**:

- Compare geometric/uncertainty NBV, GO-PRE and a task-conditioned decision-impact policy.
- Use the same initial state, reachable sensor poses, capture budget and downstream task.
- Measure remaining task uncertainty, probability/robustness of wrong decisions where empirically justified, number of unresolved regions, acquisition cost and human review.
- Where calibrated likelihoods are not available, use finite-state worst-case decision impact as in #1892; do not invent priors or expected value.

The best next capture is the one that most efficiently resolves an important decision blocker—not necessarily the one that adds the most points or reduces a generic reconstruction loss.

## 11. External baselines and standards

- **NISTIR 6922 (2002):** scanner calibration experiments identify incidence and reflectivity as error contributors. [NIST publication](https://www.nist.gov/publications/calibration-experiments-laser-scanner).
- **Winiwarter et al. (2020):** empirical TLS ranging uncertainty model for change detection; surface response and geometry affect level-of-detection. [ISPRS Annals paper](https://doi.org/10.5194/isprs-annals-V-2-2020-789-2020).
- **Malihi (2026):** uncertainty-aware digital twins link calibration and benchmark residuals to lifecycle/LoD and decision tolerance. [Accepted manuscript](https://www.repository.cam.ac.uk/items/544ba1bf-d49e-437a-98bf-e10610424346).
- **Wang et al. (16 October 2026):** representation-centred Scan-to-BIM review highlights loss of measurement support, identity, relationships, uncertainty and edit history across transformations. [Buildings 16(20), 4095](https://doi.org/10.3390/buildings16204095).
- **Song et al. (ICML 2026):** GO-PRE optimizes next-best-view selection through predictive rendering entropy. [PMLR 306](https://proceedings.mlr.press/v306/song26k.html).
- **buildingSMART IFC Validation Service:** provides independent syntax, schema and normative-rule checks; these checks do not establish sensor accuracy or measurement sufficiency. [Validation service](https://dev.validate.buildingsmart.org/).

## 12. Claim ceiling and advancement

P0 establishes deterministic behaviour of the typed contract/oracle on finite synthetic inputs only.

P1 qualifies only the exact instrument/profile, tested target classes and conditions, calibration procedure, reference uncertainty and declared operating envelope.

P2 qualifies only the exact named engineering task and tolerance. It does not establish universal sensor accuracy, legal survey status, structural adequacy, code compliance, safety or authority to perform physical work.

The programme advances only when the next gate has exact-head implementation evidence, independent tests and reproducible receipts. Queued, skipped, cancelled or stale CI is not a pass. Physical validation remains pending until independently conducted and documented.

---

**Design principle:** The system must be able to say not only “the surface was visible,” but “this instrument returned this observation under these conditions; this is the independently supported uncertainty; this is what the measurement can and cannot justify.”

## 13. P0 implementation snapshot — 10 November 2026

A standalone Rust oracle has now been added on the #4589 draft branch:

- Package: `tools/scan-bim-measurement-observability/`.
- CLI: accepts a bounded JSON corpus (8 MiB / 10,000-case ceilings) and emits a content-digested qualification receipt.
- Input types are closed with `serde(deny_unknown_fields)`; unknown enum values fail parsing.
- Four gates are reported independently: geometric visibility, observation, metrology and task sufficiency.
- Overall PASS requires all four gates to pass. FAIL and UNRESOLVED remain distinct; downstream gates are NOT_RUN when prerequisites are unresolved or failed.
- Measurement sufficiency is task-conditioned. The scalar P0 measurand retains stochastic standard uncertainty and a separate deterministic systematic-error bound; its conservative total bound is k·u + b. The terms are not RSS-combined. An unknown systematic bound remains UNRESOLVED and never defaults to zero.
- Sensor applicability is derived from a declared profile envelope (minimum/maximum range, maximum incidence angle, qualified surface-response classes and environment classes) and explicit observed conditions. A caller-provided in-domain boolean is not accepted.
- Duplicate or correlated reference lineages, stale/missing calibration, out-of-domain range, grazing incidence, unsupported surface response, unknown domain/conditions, conflicting visibility/observation and invalid uncertainty are explicit negative controls.
- The 28-case frozen fixture is at `tools/scan-bim-measurement-observability/fixtures/measurement-observability.json`.
- Eleven Rust unit tests cover corpus agreement, visibility/observation separation, duplicate lineage, task-conditioned acceptance, case-order determinism and out-of-domain rejection.
- Exact-path workflow: `.github/workflows/scan-bim-measurement-observability.yml`, with format, test, strict Clippy and frozen-corpus receipt checks.

**Validation status:** source, fixture, workflow and PR branch were inspected on GitHub. At the latest status check, the workflow attempts for the then-current branch head were pending/queued; no successful exact-head result had been observed. The source has not been compiled locally in this environment. Treat code qualification as pending until the final branch head's workflow completes successfully.

The corpus uses synthetic values and digest-shaped identifiers. It does not authenticate upstream evidence, establish real survey independence, calibrate hardware or demonstrate physical measurement accuracy. The P0 scalar metre-valued measurand is not a full 3D point covariance, pose covariance or shared-observation correlation model. Those remain owned by #1607 and required for P1. The declared coverage factor, coverage probability and systematic interval are synthetic inputs, not calibrated coverage guarantees. No live sensor import or hardware claim is required for P0.
