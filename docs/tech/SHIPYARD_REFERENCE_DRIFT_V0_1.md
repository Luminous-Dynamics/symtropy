# Shipyard reference-artifact drift sentinel v0.1

Tracking: [SHIPYARD-REFERENCE-DRIFT-001 #5141](https://github.com/Luminous-Dynamics/symtropy/issues/5141)

## Research thesis

Periodic instrument verification is necessary but does not by itself prove the measurement process remained stable during a sequence of inter-pass scans. This sentinel measures an unchanged certified check artefact through time and creates a separate health decision before scan pairs can be compared.

The design follows two metrology principles:
- ISO 10360-13:2021 provides acceptance and reverification tests for optical 3D CMS, for surfaces whose characteristics stay within a cooperative range. It does not make every weld surface cooperative.
- NIST's measurement-process guidance uses repeated check-standard observations to monitor bias and long-term variability; its EWMA discussion shows why small persistent shifts can be detected even when no single measurement breaches an individual limit.

These are design references, not a conformance claim. No universal micrometre thresholds are invented; the instrument owner must supply qualified error limits, EWMA limit, baseline and environment range.

## V0 implementation

- Dependency-isolated Rust 2024 workspace with local Cargo.lock.
- Draft 2020-12 JSON Schema and frozen corpus: one stable known-answer sequence, one persistent-drift sequence and 18 named negative controls.
- Fixed-point EWMA using parts-per-million lambda; no floating point in the evaluator.
- Separate pointwise guard: |measurement − certified value| + expanded uncertainty.
- Separate EWMA guard with the maximum observation uncertainty added conservatively.
- Exact artifact, characteristic, instrument, calibration revision, certificate validity, policy digest, observation digest and timestamp identity.
- Frozen initial EWMA baseline and policy; no adaptive baseline drift and no automatic recalibration.
- 22 Rust library tests and 2 CLI corpus tests.

## Known answers

The stable case uses residuals [2, −1, 3, 0] µm with 10 µm expanded uncertainty and remains within the 100 µm pointwise and 45 µm EWMA limits.

The persistent-drift case uses [20, 20, 30, 40, 50, 60] µm residuals, each still inside the pointwise limit (worst pointwise guard 70 µm). With lambda 0.5 and a frozen zero baseline, the EWMA plus uncertainty crosses the 45 µm limit and returns HOLD. This demonstrates the added value of a history-sensitive channel over a pointwise-only detector; the values are synthetic.

## Claim ceiling

A PASS establishes only deterministic implementation of the supplied policy on the supplied synthetic observations. It does not establish actual scanner stability, calibration traceability, ISO 10360-13 conformity, weld geometry accuracy, deposited volume, NDT, WPS qualification or production acceptance. A HOLD causes review; it does not auto-correct or recalibrate the instrument.
