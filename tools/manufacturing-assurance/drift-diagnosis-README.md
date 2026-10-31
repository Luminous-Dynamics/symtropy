# Manufacturing twin drift diagnosis (v0.1)

This is a dependency-free Rust reference for transparent, synthetic known-truth diagnosis across four distinct sources of disagreement: primary-sensor bias, out-of-domain process context, model discrepancy and interface mapping/clock error.

It is deliberately **not** an online recalibration engine. It consumes declared samples and thresholds, emits a diagnostic category or abstention, and never updates a model or controller.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/drift_diagnosis.rs -o /tmp/mfg-drift-tests
/tmp/mfg-drift-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/drift_diagnosis.rs -o /tmp/mfg-drift
/tmp/mfg-drift
```

Or run `bash tools/manufacturing-assurance/run-tests.sh`.

## Diagnostic input contract

Each sample contains:
- primary sensor value;
- independently qualified reference sensor value;
- digital-twin prediction;
- measured process/context input;
- source interface value, declared scale/offset mapping and received mapped value;
- source timestamp, expected transport latency and received timestamp;
- calibration qualification, stale and replay flags.

The expected mapped value is computed from the source value and the declared affine transform; it is not copied from a second supplied answer. Clock residual is computed from source time + declared expected transport latency versus received time.

## Diagnostic rules

Default thresholds are frozen in `Thresholds` and must be chosen from the test context:
- primary/reference mean absolute disagreement indicates a possible primary sensor bias;
- reference/prediction residual in-domain with agreeing sensors indicates model discrepancy;
- process input outside the declared envelope produces OUT_OF_VALIDITY_DOMAIN, not model-discrepancy attribution;
- mapping conversion residual and clock residual have separate signatures;
- multiple simultaneous signatures produce UNRESOLVED_MULTIPLE_FAULTS;
- missing channels, invalid numeric values or too few samples cause abstention;
- stale/replayed data and unqualified calibration block normal root-cause attribution.

A fault label is a synthetic signature class, not proof of real-world root cause. A reference channel must be independently qualified for the sensor/model distinction to be meaningful.

## Frozen controls

The authored tests cover baseline/no fault, primary sensor bias, process/context shift, in-domain model discrepancy, interface mapping failure, clock skew, simultaneous faults, missing reference, unqualified calibration, stale/replayed data, insufficient window size, deterministic replay and invalid thresholds.

## Claim ceiling

The reference validates diagnostic logic against synthetic signatures only. It does not detect a real plant fault, prove physical model discrepancy, establish causal origin, perform recalibration, update VVUQ, authorize an intervention, or actuate equipment. Unresolved is a valid and required outcome.
