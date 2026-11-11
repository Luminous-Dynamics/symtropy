# Inter-pass scan metrology qualification v0.1

Tracking: [SHIPYARD-SCAN-METROLOGY-001 #4736](https://github.com/Luminous-Dynamics/symtropy/issues/4736)

## Why this is a separate layer

The scan gate precedes the inter-pass planner. The planner's exact scan SHA and registration-receipt SHA are necessary but not sufficient: identity proves which bytes and transform were used, not that the sensor was within calibration, the registration was metrologically plausible, enough of the surface was observed, or the uncertainty fits the decision tolerance.

## Research/standards basis

- ISO 10360-13:2021 addresses acceptance and reverification tests for optical 3D coordinate measuring systems; published and under systematic review as of 15 July 2026. This contract does not implement or claim conformance to it.
- ISO 14253-1:2017 establishes uncertainty-aware decision rules for conformity/nonconformity against tolerances and measurement-equipment MPE; confirmed 2023 according to ISO.
- NISTIR 8499 (2024) discusses 3D scanner object orientation error and uncertainty in the ASTM E2919-22 static-pose context.
- NIST's 2024 structured-light metrology work examines sensitivity of VDI/VDE 2634-2 and ISO 10360-13 tests to model calibration parameters.
- ISO/TR 23247-101:2026 is published; ISO/AWI 23247-8 VVUQ remains under development. This crate is an independent synthetic protocol, not a claim of compliance with these documents.

## Decision chain

1. Instrument performance: traceable reference artifact/certificate, calibration receipt, valid calibration interval, configuration match, environment envelope and reference-length residual.
2. Registration: scan-source SHA binding, transform-receipt SHA, target frame, rigid-transform determinant/orthogonality and independent reference-target residual. ICP convergence is recorded but never accepted as sole proof.
3. Observation quality: cleaning receipt, occlusion-assessment receipt, profile completeness, dropped profiles, point accounting, invalid/clipped fraction, minimum valid points and occlusion threshold.
4. Decision uncertainty: only explicitly independent components may be combined by root-sum-square. Unknown/correlated terms abstain in v0.1. Expanded uncertainty uses an integer fixed-point coverage factor and must fit both the declared uncertainty ceiling and decision tolerance.
5. Receipt: QUALIFIED output binds exact scan/calibration/registration digests, WID/joint/layer/predecessor, policy revision, coverage, occlusion and expanded uncertainty. Otherwise ABSTAIN includes every detected reason.

All numbers in v0.1 are symbolic integer units. They are not actual sensor micrometres, instrument MPEs or acceptance-test values.

## CLI/library use

`evaluate_scan(&ScanEvidence)` returns either `Qualified(QualificationReceipt)` or `Abstain(Vec<AbstainReason>)`. The library intentionally accepts only synthetic contract evidence. A future measured adapter must provide hashes of authorized raw bytes, traceable calibration artefacts and reproducible transform receipts, then be qualified under the relevant metrology procedure.

## Frozen controls

The metadata corpus contains 30 controls: one positive contract baseline and 29 negative mutations. Rust unit tests cover the main software guards, including exact digest binding, calibration validity, reference check, configuration/environment, ICP vs independent targets, reflection transforms, profile completeness, point accounting, occlusion/cleaning, stale/future scans, policy freeze, correlation handling, uncertainty limits and overflow.

## Benchmarks and goals

- Zero false acceptance across all frozen invalid-measurement controls.
- Exact disposition agreement with finite known-answer cases.
- 95% prediction/uncertainty coverage only after a sufficiently powered physical reference campaign, with interval width and per-condition coverage reported.
- Repeatability and reproducibility across sensor remount, operator, shift, environment and robot configuration.
- False-accept/false-reject versus abstention curves at stated decision tolerances.
- Independent reference artefacts for scanner and registration performance; never score against the ICP objective alone.

## Claim ceiling

PASS proves only the synthetic contract logic. It does not establish optical scanner accuracy, ISO 10360-13/VDI/VDE conformity, welding geometry accuracy, physical distortion, weld acceptance, NDT acceptance, WPS qualification or robot safety.
