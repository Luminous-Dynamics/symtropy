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
- Separate EWMA guard with measurement uncertainty propagated through the same positive-weight recurrence as the EWMA residual. Each pointwise residual also includes reference-certificate expanded uncertainty.
- Exact artifact, characteristic, instrument, calibration revision, certificate validity, policy digest, observation digest and timestamp identity.
- Frozen initial EWMA baseline and policy; no adaptive baseline drift and no automatic recalibration.
- 25 Rust library tests and 2 CLI corpus tests.

## Known answers

The stable case uses residuals [2, −1, 3, 0] µm with 10 µm observation uncertainty plus 1 µm certificate uncertainty (maximum pointwise guard 14 µm), and remains within the 100 µm pointwise and 45 µm EWMA limits.

The persistent-drift case uses [20, 20, 30, 40, 50, 60] µm residuals. Every input point is below the 100 µm pointwise limit (full-input maximum guard 71 µm including certificate uncertainty). With lambda 0.5 and a frozen zero baseline, the EWMA plus propagated uncertainty crosses the 45 µm limit at the fifth observation (guard 52 µm) and returns HOLD before consuming the sixth. This demonstrates the added value of a history-sensitive channel over a pointwise-only detector; the values are synthetic.

## Claim ceiling

A PASS establishes only deterministic implementation of the supplied policy on the supplied synthetic observations. It does not establish actual scanner stability, calibration traceability, ISO 10360-13 conformity, weld geometry accuracy, deposited volume, NDT, WPS qualification or production acceptance. A HOLD causes review; it does not auto-correct or recalibrate the instrument.


### Uncertainty propagation hardening — 2026-11-14

The check-artifact certificate now carries its own expanded uncertainty. Each pointwise residual guard includes both observation uncertainty and certificate uncertainty. The EWMA uncertainty is propagated recursively using the same positive weights as the EWMA residual, including a frozen initial baseline uncertainty; it is not replaced by only the latest sample's uncertainty. Regression tests demonstrate that an earlier high-uncertainty observation remains represented in the EWMA decision, and that certificate uncertainty can independently trigger a pointwise HOLD.

All SHA-256 fields in the synthetic fixture are identifier-shaped placeholders. The v0 verifier validates their syntax and binds them through the report; it does not claim to hash raw artifact bytes or authenticate policy signatures.


### Uncertainty regression examples

- A 90 µm residual plus 5 µm observation uncertainty and 6 µm certificate uncertainty yields a 101 µm pointwise guard and HOLD against a 100 µm policy limit.
- The sequence [0, 40, 40, 40] µm with observation uncertainties [50, 1, 1, 1] µm and 1 µm certificate uncertainty keeps each point below the 100 µm pointwise limit. With an EWMA limit of 39 µm, propagated uncertainty from the first observation makes the final EWMA guard exceed the limit; using only the latest sample uncertainty would understate the guard.


### Fixed-point uncertainty rounding

The EWMA residual estimate uses nearest rounding; the EWMA uncertainty bound uses ceiling on every fixed-point recurrence and adds one scaled unit to cover residual-update quantization. This keeps the propagated uncertainty one-sided/conservative rather than allowing a sequence of tiny downward roundings to reduce the guard.
