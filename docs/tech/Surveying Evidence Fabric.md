# Surveying Evidence Fabric

**Design proposal v0.1 — SURVEY-EVIDENCE-001 / #1607**

Status: **architecture specified; not implemented or field-qualified by this document.**

## Purpose

Build a reusable, evidence-first surveying pipeline for Symtropy's terrain and engineering digital twins. It should accept measurements and geometry from total stations, GNSS, terrestrial/mobile LiDAR, photogrammetry, SLAM, borehole instruments, and manual field observations while preserving each modality's limits and traceability.

The goal is not to replace surveyors, metrology practice, PROJ, or specialist point-cloud tools. It is to make coordinate semantics, uncertainty, provenance, independent validation, and the admission of a result into an authoritative digital twin explicit and machine-checkable.

> **Core distinction:** raw observation ≠ transformed observation ≠ registered survey product ≠ adjusted control solution ≠ qualified change estimate ≠ admitted world mutation.

Every transition must record exact inputs, the algorithm/configuration, coordinate frame, uncertainty method, evidence outputs, and an explicit qualification disposition.

## Why this matters

Symtropy's current terrain system includes a radius-based `ExcavationEvent` and chunk-local voxel loops. A separate material-aware excavation contract is being developed in #1588 / PR #1603, and exact voxel addressing is tracked in #1329. Surveying must not bypass those contracts or mutate terrain from a scan merely because registration looks plausible.

Underground surveying introduces especially difficult cases: repetitive or feature-poor surfaces, dust, water, occlusion, constrained instrument lines of sight, accumulated drift, and weak data-transfer/geodetic infrastructure. A published review of underground mine laser scanning describes both useful applications and these continuing constraints. The evidence fabric therefore treats coverage, control, registration drift, and uncertainty as first-class outputs—not just a visually convincing point cloud.

## Architecture

The pipeline is a sequence of typed, inspectable artifacts:

```text
Instrument / importer / simulator
              |
              v
  1. Immutable raw observation manifest
              |
              v
  2. Frame and calibration admission
              |
              v
  3. Coordinate transform graph
              |
              v
  4. Control-network adjustment / pose-graph solution
              |
              v
  5. Registered geometry product
              |
              v
  6. Independent accuracy qualification
              |
              v
  7. Change estimate + uncertainty + coverage
              |
              v
  8. Candidate spatial delta (not authoritative)
              |
              v
  9. Exact-generation world admission / atomic application
```

A failed or incomplete stage can preserve diagnostic evidence but must not emit a qualified success artifact. Retries create new attempt artifacts rather than overwriting prior observations or results.

### 1. Capture and raw evidence

Represent the source without pretending every instrument produces the same kind of measurement.

Modalities can include total-station angle/distance observations, GNSS fixes, terrestrial or mobile point clouds, photogrammetric reconstructions, camera/IMU/SLAM trajectories, borehole measurements, and manual points. Each importer declares its version and support envelope.

A session and every raw artifact should bind:

- stable session and observation identity, capture sequence, acquisition time, time scale, and clock-quality metadata;
- sensor/instrument profile and applicable calibration certificate or calibration artifact;
- measurement units, declared coordinate frame, acquisition configuration, firmware/software and importer versions;
- relevant environment/quality conditions such as GNSS fix mode, range, incidence, image overlap, visibility, dust/water limits or SLAM tracking state, where the source supports them;
- content digest, byte length, media/schema version and immutable locator;
- completeness, dropouts, saturation, invalid-return and exclusion counters.

Raw input bytes are immutable. Corrections and reprocessing are new derived artifacts with explicit parent references. No filename, sensor brand, numeric coordinate range, map tile, or UI setting may silently determine the datum or unit.

Use bounded parsers. A format importer must declare byte/point limits, reject malformed and non-finite inputs, and report truncation rather than silently accepting partial data.

### 2. Coordinate-frame and transform graph

A CRS identifier alone is insufficient to identify a usable frame for all surveying tasks. A `CoordinateFrameV1` should carry, as applicable:

- horizontal CRS and datum identifier, authority/name and definition version;
- vertical datum and height type (for example ellipsoidal versus orthometric height), geoid/grid model identity, and referenced grid-file digests;
- coordinate epoch / time-dependent datum context when required;
- axis order, axis direction, units, handedness and dimensionality;
- parent/local frame relationship, frame incarnation and owner scope;
- an explicit statement of which fields are unknown or not applicable.

No unknown is silently coerced to WGS84, metres, a local origin, or a presumed vertical datum.

A `TransformEdgeV1` binds exact source and target frame IDs, the operation definition and pipeline, parameter/grid artifact hashes, area and epoch of validity, direction, software/version, and transform uncertainty. Transform identity includes the exact operation and referenced resources—not merely the two endpoint EPSG codes. Transform composition creates a new derivation record. Reverse transformation must use an explicitly supported inverse, not assume every operation is exactly invertible.

Reuse established implementations when their scope, licensing, and build constraints fit. Candidate components include Rust Geodesy and GeoRust's geometry/PROJ interoperability. Keep CRS math behind an adapter; do not develop a proprietary CRS engine as part of this project.

### 3. Uncertainty model

Avoid a single “accuracy score” that conflates internal precision, absolute accuracy, completeness, and uncertainty. A qualification report should retain them separately.

For point-like observations, a first implementation may use a 3D covariance matrix in declared units and frame plus a clearly named confidence/coverage interpretation. For survey-network or transformation parameters, retain the parameter covariance or a declared alternative uncertainty representation. For point clouds, support spatially varying uncertainty and correlation metadata where available; a cloud-level scalar may be shown as a summary but must not replace richer inputs.

Validation must check finite values, covariance symmetry and positive-semidefiniteness under a documented numeric tolerance, units, matrix order and frame. Do not silently repair malformed covariance. Unknown uncertainty is represented as unknown/unqualified, never zero.

Propagation must account for transformation sensitivity/Jacobians and correlations. Do not combine all terms by root-sum-square unless the independence assumptions are documented and justified. Where correlations or systematic errors are unknown, expose the limitation and prevent a stronger qualification than the evidence supports.

### 4. Control network adjustment

Keep raw angle/distance/GNSS/control observations immutable and separate from the adjusted solution.

A `ControlNetworkSolutionV1` should report:

- network datum strategy: fixed, weighted constraints, or free-network adjustment;
- solver and version, model/weighting choices, robust-estimation policy and convergence disposition;
- observation count, degrees of freedom/redundancy, control/checkpoint identities and excluded observations with reasons;
- adjusted point estimates, covariance/uncertainty method, residual vector and normalized residual diagnostics;
- traverse/loop closures and other relevant geometric consistency checks;
- warnings, failed constraints, weak geometry, sensitivity/influence diagnostics and scope limits.

Do not discard an outlier just to obtain a passing closure. An outlier flag is a reviewable result. Any exclusion changes the derived solution's identity and must preserve both the pre-exclusion and post-exclusion evidence. Under-constrained networks do not produce an absolute-accuracy claim.

### 5. Registration, fusion and coverage

Registration may use targets, control points, feature-based alignment, point-to-plane methods, bundle adjustment, SLAM/pose graphs or a declared specialist solver. Every result binds the exact input subset, software/configuration and transform estimates.

The report must distinguish:

- internal consistency / relative precision;
- control-constrained absolute accuracy;
- independent checkpoint results;
- spatial coverage, density, occlusion and unobserved regions;
- drift indicators, loop closures, control geometry and registration degeneracy.

A low registration residual is not proof of absolute accuracy. Points used to fit/align/calibrate a product cannot also be reported as independent validation checkpoints. If a dataset contains no suitable independent control/checkpoints, report that limitation and do not promote internal fit residuals into an absolute-accuracy claim.

Different modalities remain distinguishable. “Fusion” records the evidence relationships; it must not erase whether a point came from a total station, GNSS, photogrammetry, LiDAR, or SLAM estimate. If one pipeline uses a prior model to register a new scan, the prior must be recorded so the scan is not later misrepresented as fully independent confirmation of that same model.

### 6. Change detection and volume estimates

A `ChangeEstimateV1` compares explicitly identified epochs/products after frame and registration qualification. It reports spatial regions, signed/unsigned displacement where appropriate, coverage, exclusions, detection limit, uncertainty method and area/volume uncertainty.

The change engine must consider spatial correlation, registration drift, surface roughness, density differences and occlusion; a pointwise nearest-neighbour distance alone is not a general volumetric change estimator. Choose and document a solver appropriate to the data and use case. A method designed for exposed surfaces should not be presumed valid for underground voids, loose spoil, moving machinery or highly occluded geometry.

Each result distinguishes:
- **Detected change:** evidence exceeds a declared detection criterion in the qualified domain.
- **No detected change:** data were adequate and the estimated change is below that criterion; this is not proof of exactly zero physical change.
- **Indeterminate:** missing coverage, weak registration/control, excessive uncertainty or out-of-domain data prevent a defensible conclusion.
- **Rejected:** inputs or invariants are invalid.

For excavation, removed solid, loose spoil and displaced material are separate physical quantities. Do not infer all three from a surface difference without a domain-appropriate model.

### 7. Candidate delta and world admission

Survey processors produce a **candidate**, not an authority to mutate terrain. The candidate binds:

- the source survey-product and qualification artifact digests;
- base world generation and base geometry identity;
- source and target coordinate frames and the exact admitted transform;
- change-model version, region/geometry, material or feature scope and uncertainty;
- qualification disposition, warnings and the policy that permitted the candidate.

A separate terrain/world adapter rechecks the exact current generation, base geometry, coordinate mapping and policy immediately before mutation. It must reject stale base state, mismatched frame/material identity, out-of-limit uncertainty, incomplete qualification and replay of an already-consumed operation. Apply changes atomically or preserve the prior authoritative state if the transaction fails. This composes with #1329's exact terrain lattice address and #1588 / PR #1603's material-aware spatial-delta contract; it does not replace them.

## Suggested Rust contract boundaries

The first crate should be headless and should not depend on Bevy rendering, live sensor SDKs, or a world-mutation runtime. Names are provisional pending the existing crate/identity conventions.

- `SurveySessionV1`
- `SurveyArtifactRefV1` (stable locator + byte length + content digest + schema/media type)
- `CoordinateFrameV1` and `TransformEdgeV1`
- `SurveyInstrumentProfileV1` / `CalibrationEvidenceRefV1`
- `SurveyObservationV1` with modality-specific payload references
- `UncertaintyV1` / `Covariance3V1` and explicit units/frame
- `ControlNetworkInputV1` and `ControlNetworkSolutionV1`
- `RegisteredSurveyProductV1`
- `SurveyQualificationV1`
- `ChangeEstimateV1`
- `CandidateSpatialDeltaV1`
- closed, versioned result dispositions separating completed, completed-with-warnings, rejected, out-of-domain, non-converged and incomplete evidence

Use canonical, domain-separated identity bytes with fixed field/order semantics. Do not hash Rust `Debug` text, filesystem paths, display labels, insertion order, timestamps without explicit semantics or a serializer's unspecified map iteration order. Enforce maximum payloads, observation counts, identifier lengths, transform-graph depth and numeric bounds before expensive work.

No live world mutation API belongs in the first crate.

## Qualification plan

### Deterministic synthetic/reference tests

1. Golden coordinate-transform vectors for every initially supported operation, including declared axis order and units.
2. Transform round-trip and composition tests with tolerances derived from operation scope, not arbitrary exact equality.
3. Analytic covariance propagation through a simple affine transform and a nontrivial Jacobian; correlated versus independent input cases.
4. Reject absent/ambiguous horizontal or vertical frame, unknown units, invalid epoch, unsupported transform, missing grid digest, non-finite values and oversized payloads.
5. Reject malformed/non-symmetric/non-PSD covariance; preserve explicit unknown uncertainty.
6. Solve a small weighted network against independently calculated coordinates, covariance and residuals.
7. Reject under-constrained networks and failed closure; preserve flagged outliers and evidence before/after documented exclusions.
8. Ensure fitting/control points are never counted as independent checkpoints.
9. Replay the same immutable inputs and exact solver configuration to the same canonical identity and declared numerical tolerance.
10. Two survey epochs with known displacement, known no-change noise, correlated alignment drift and a displacement below the detection limit.
11. Mark insufficient density/occlusion, weak registration and missing independent checkpoints as indeterminate rather than passing.
12. Reject candidate application on a changed world generation, changed base geometry or mismatched coordinate frame.
13. Inject failure at each pipeline stage and confirm no qualified output or partial authoritative world mutation escapes.
14. Resource exhaustion and malformed-file corpus: truncation, repeated IDs, duplicate timestamps/sequences, invalid array lengths, NaN/infinity and arithmetic overflow.

Synthetic tests establish software properties only. They do not verify an actual instrument's field accuracy or certify a legal/mining survey.

### Field qualification

Before field accuracy claims, select a specific site and use case, a validated instrument configuration and calibration history, a qualified reference/control network, and *independent* checkpoints at a documented spatial distribution. Include checkpoint survey uncertainty in the final product accuracy. Report failures and exclusions as well as successful cases. For mining, include environmental cases such as dust, water, feature-poor surfaces and constrained sight lines, and define safe operator procedures separately.

Do not promise one global centimetre threshold. Acceptance thresholds depend on the decision: cadastral boundary, mining clearance, volume accounting, tunnel convergence, robot localization and broad regional mapping have different error consequences. Select thresholds before collecting the acceptance dataset and use a risk-based credibility plan.

## Standards and implementation choices

These are design inputs, not a declaration of compliance:

- **South Africa CD:NGI:** [Standard for the National Control Survey System, QLAS SD.1](https://ngi.dlrrd.gov.za/index.php/technical-information/standards-menu?download=65%3Astandard-for-the-national-control-survey-system). Use the applicable national control network and datum guidance when working in South Africa.
- **ISO 17123-1:2014:** [Field procedures for testing geodetic and surveying instruments — theory](https://www.iso.org/standard/64156.html). The ISO page states this edition was confirmed current in 2024.
- **USGS / ASPRS:** [Lidar Base Specification accuracy validation](https://www.usgs.gov/ngp-standards-and-specifications/lidar-base-specification-data-processing-and-handling-requirements) illustrates independent-checkpoint discipline and inclusion of survey/checkpoint uncertainty.
- **NIST AMS 100-85 (2026):** [Harmonizing uncertainty quantification and metrology](https://doi.org/10.6028/NIST.AMS.100-85), a risk-based model-credibility approach for simulation and digital twins.
- **Underground laser scanning review:** [Singh, Banerjee & Raval (2023)](https://doi.org/10.1016/j.ijmst.2022.09.022).
- **Rust Geodesy:** [geodesy](https://github.com/busstoptaktik/geodesy).
- **GeoRust:** [geometry primitives and PROJ interoperability](https://github.com/georust/geo).

Before adding a dependency, review license, build requirements, supported datum operations and grids, determinism, security posture, maintenance, and whether it actually covers the intended operation. Do not duplicate mature geodesy when an adapter is enough. Point-cloud file parsing and processing should likewise be delegated to a qualified backend rather than improvised in the contract crate.

## Rollout

1. **Contract** — freeze frame, artifact, uncertainty, observation and disposition types; create hostile-input and canonical identity tests.
2. **Reference mathematics** — add explicit transform adapters and tiny adjustment/uncertainty reference cases with golden outputs.
3. **Geometry qualification** — register selected point-cloud/mesh products and compute uncertainty-aware change estimates on pinned, licensed datasets.
4. **Modality adapters** — add one source format at a time with format-specific test corpora and explicit support matrices.
5. **Terrain adapter** — bind qualified results to exact terrain/world generation and coordinate/lattice identity, with atomic apply and replay protection.
6. **Field validation** — only after the prior software qualification succeeds, compare against independent survey-control evidence for the named application and environment.

## Non-goals

The first tranche does not implement instrument drivers, a full CRS engine, generic “AI surveying”, unqualified SLAM localization, automatic terrain writes, proprietary cloud services, live mine operations, cadastral certification, or any claim of field-verified accuracy.

## Evidence status

This document defines architecture, invariants and a test plan. It contains no implemented algorithm, executed test result, field measurement, instrument calibration result, or production-readiness claim.
