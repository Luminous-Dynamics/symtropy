# Inter-pass scan metrology qualification v0.1

Tracking: [SHIPYARD-SCAN-METROLOGY-001 #4736](https://github.com/Luminous-Dynamics/symtropy/issues/4736)

## Why this is a separate layer

The scan gate precedes the inter-pass planner. The planner's exact scan SHA and registration-receipt SHA are necessary but not sufficient: identity proves which bytes and transform were used, not that the sensor was within calibration, the registration was metrologically plausible, enough of the surface was observed, or the uncertainty fits the decision tolerance.

## Research and standards basis

- ISO 10360-13:2021 addresses acceptance and reverification tests for optical 3D coordinate measuring systems. ISO lists it as published and under systematic review as of 15 July 2026. This contract does not implement or claim conformance to it: https://www.iso.org/standard/74957.html.
- ISO 14253-1:2017 establishes uncertainty-aware decision rules for conformity/nonconformity against tolerances and measurement-equipment MPEs. ISO states it was confirmed in 2023: https://www.iso.org/standard/70137.html.
- NISTIR 8499 (2024) discusses 3D scanner object orientation error and uncertainty in the ASTM E2919-22 static-pose context: https://doi.org/10.6028/NIST.IR.8499.
- NIST's 2024 structured-light metrology work examines sensitivity of VDI/VDE 2634-2 and ISO 10360-13 tests to model calibration parameters: https://www.nist.gov/publications/vdivde-2634-2-and-iso-10360-13-performance-evaluation-tests-and-systematic-errors.
- ISO/TR 23247-101:2026 is published; ISO/AWI 23247-8 VVUQ remains under development. This crate is an independent synthetic protocol, not a claim of compliance with those documents.

## Paradigm shift: qualify the measurement before optimizing the geometry

A scan is not a truth object. It is a measurement with its own identity, calibration state, measurement model, uncertainty, coordinate transform, coverage and failure modes. An optimizer should not compensate for measurement uncertainty by trading it against time/material objectives.

Four independent gates:

1. **Instrument/system:** traceable reference artifact/certificate, calibration receipt, valid calibration interval, configuration match, environment envelope and reference-length residual.
2. **Registration:** scan-source SHA binding, transform-receipt SHA, target frame, rigid-transform determinant/orthogonality and independent reference-target residual. ICP convergence is recorded but never accepted as sole proof.
3. **Observation quality:** cleaning receipt, occlusion-assessment receipt, profile completeness, dropped profiles, point accounting, invalid/clipped fraction, minimum valid points and occlusion threshold.
4. **Decision uncertainty:** only explicitly independent components may be combined by root-sum-square. Unknown/correlated terms abstain in v0.1. Expanded uncertainty uses an integer fixed-point coverage factor and must fit both the declared uncertainty ceiling and decision tolerance.

Do not conflate instrument MPE, reference residual, registration residual, repeatability, point spacing or surface deviation. Each measures a different part of the measurement chain.

## Ownership and interfaces

- [SHIPYARD-INTERPASS-ADAPTIVE-001 #4649](https://github.com/Luminous-Dynamics/symtropy/issues/4649) consumes qualified geometry observations and abstains if this measurement gate does not pass. This issue does not own pass selection or the Pareto planner.
- [SHIPYARD-TANDEM-GMAW-DATA-ADMISSION-001 #4643](https://github.com/Luminous-Dynamics/symtropy/issues/4643) owns BAM raw-data rights/admission and source-specific scan file mapping.
- [SHIPYARD-WELD-PHYSICS-VVUQ-001 #4614](https://github.com/Luminous-Dynamics/symtropy/issues/4614) owns physical model validation and uncertainty; it consumes scan measurement uncertainty rather than inventing it.
- [COMPONENT-DIGITAL-THREAD-001 #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331) owns as-built geometry and component lineage.
- [SHIPYARD-DISTORTION-CO-DESIGN-001 #4633](https://github.com/Luminous-Dynamics/symtropy/issues/4633) owns sequence/fixture/release planning.

## V0 implementation

A dependency-free Rust 2024 nested workspace with a synthetic scan/measurement contract and 30 frozen metadata cases. No raw BAM scan is downloaded or parsed. All numerical values are integer synthetic units. The synthetic coordinate unit is millimetres to match the BAM scan-frame convention; it is not an actual sensor measurement or instrument MPE.

Each scan admission binds:
- workpiece/joint/layer and predecessor-pass identity;
- source artifact SHA-256 and acquisition/configuration revision;
- sensor identity, calibration/reverification receipt and reference-artifact identity;
- registration transform SHA-256 and independent reference-target residual set;
- point/profile counts, expected counts, coverage, occlusion, invalid/clipped/dropped samples;
- coordinate frame, units and environmental/configuration envelope;
- uncertainty components, coverage factor, correlation policy and decision tolerance;
- explicit disposition and claim ceiling.

The library returns either `Qualified(QualificationReceipt)` or `Abstain(Vec<AbstainReason>)`. A future measured adapter must provide hashes of authorized raw bytes, traceable calibration artifacts and reproducible transform receipts before measured data are admitted.

## Uncertainty and arithmetic policy

Independent standard-uncertainty components are combined by integer-ceiling root-sum-square only when independence is explicitly declared. Known-correlated and unknown-correlation components abstain; v0.1 does not silently invent covariance or assume independence. Expanded uncertainty is rounded conservatively upward using a fixed-point coverage factor.

Profile-coverage, dropped-profile and invalid-point permille rates use checked multiplication and abstain on overflow. They never saturate an overflowing count into a deceptively acceptable rate. Rigid-transform determinant checks use overflow-safe absolute difference.

## Frozen controls and tests

The machine-readable corpus has **30 controls: one positive synthetic baseline and 29 negative controls**. It covers source digests, calibration validity, reference checks, configuration/environment, ICP vs independent targets, reflection transforms, profile completeness, point accounting, occlusion/cleaning, stale/future scans, policy freeze, correlation handling, uncertainty limits and evidence-class/claim promotion.

The Rust crate currently contains **35 unit/mutation tests**. These include the baseline qualification receipt, digest binding, calibration/reference/environment failures, independent-target checks, transform reflection, profile/point-count guards, cleaning/occlusion receipts, time/policy leakage, correlation/uncertainty rules, arithmetic overflow and the signed-integer determinant edge case.

## Benchmark ladder and targets

Research metrics:
- reference-length residual distribution and maximum absolute residual;
- reference-target translation/rotation residual and uncertainty;
- calibration drift/reverification stability;
- registration residual against independent reference targets, not the optimizer's own ICP cost;
- point/profile completeness, coverage, occlusion, invalid/dropped/clipped rate;
- uncertainty calibration/coverage and interval width by surface/material/angle/reflectivity/temperature/cleaning condition;
- selective risk: error among accepted observations vs abstention rate;
- false-accept/false-reject versus abstention curves at stated decision tolerances;
- repeatability/reproducibility across sensor remount, operator, shift, temperature and robot configuration;
- time to acquire and qualify a valid scan.

Research goals: zero false acceptance on frozen negative controls; 100% exact disposition agreement with the finite known-answer oracle; nominal 95% uncertainty coverage on a sufficiently powered physical holdout; and minimum abstention only after the declared measurement limits are met. No arbitrary universal point-cloud RMSE target is imposed without a reference artifact, tolerance and measurement-task context.

## Claim ceiling

A v0 PASS proves only the synthetic contract logic. It does not establish optical scanner accuracy, ISO 10360-13 or VDI/VDE conformity, welding geometry accuracy, physical distortion, weld acceptance, NDT acceptance, WPS qualification or robot safety. Physical qualification requires traceable artifacts, competent calibration, suitable surface-condition tests, repeatability/reproducibility, uncertainty evaluation and independent metrological evidence.
