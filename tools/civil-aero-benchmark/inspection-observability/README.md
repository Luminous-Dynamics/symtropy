# AER-01 inspection-observability oracle v0.1

**Tracking:** AERO-INSPECTION-OBSERVABILITY-001 / #4165  
**Related:** #3921 aircraft architecture; #4093 structural state bridge; #4139/#4142 NASTRAN source admission; #4236 instrument observation operators; #2890 experiment value-of-information.  
**Status:** synthetic Tier-0 exact oracle only.

## Purpose

This crate implements a small, deterministic sensor-subset benchmark for a finite-dimensional linearized local observation model. It verifies the arithmetic and claim boundaries required before an inspection/observability optimizer consumes real structural models.

It does not parse NASTRAN, solve a beam, predict strain, identify physical damage, estimate fatigue life, qualify an NDE procedure, or establish aircraft SHM capability.

## Model

For parameter vector theta and a selected set of sensor channels:

    y = H theta + epsilon
    epsilon covariance = R = diag(sensor variances)

Each parameter declares a positive characteristic scale. The evaluator uses the normalized sensitivity matrix:

    H_normalized = H diag(parameter_scales)
    F = H_normalized^T R^-1 H_normalized

The evaluator reports, without regularization:
- observation-matrix rank and nullity for each selected subset;
- Fisher information matrix;
- ascending eigenvalues and minimum eigenvalue;
- log determinant only for full-rank positive-definite subsets;
- exhaustive D-optimal (maximum log-determinant) and E-optimal (maximum minimum eigenvalue) selections;
- exact input SHA-256, numerical tolerances and claim ceiling.

Sensor IDs are sorted before enumeration, and ties are resolved lexicographically. The small-instance limit is 16 candidate sensors and 8 parameters; every subset of the requested cardinality is enumerated exactly.

## Frozen synthetic known answer

Four synthetic sensors observe two normalized parameters. Select two sensors.

The locked answer is:
- 6 subsets enumerated;
- all 6 are full rank for this fixture;
- D-optimal subset: S_A + S_C;
- E-optimal subset: S_A + S_C;
- Fisher information for S_A + S_C: [[2,1],[1,1]];
- minimum eigenvalue: (3 - sqrt(5))/2, approximately 0.38196601125;
- log determinant: approximately 0 (floating-point result is within the locked tolerance).

This fixture is constructed solely to test known-answer arithmetic and deterministic selection. Its sensor sensitivities, variances and parameter scales are not physical CRM or AER-01 values.

## Reproducible toolchain inputs

Direct Rust dependencies are pinned in Cargo.toml and the complete resolved dependency graph is committed in Cargo.lock. CI uses Cargo's --locked mode for tests and execution, so dependency resolution may not silently mutate the lockfile. The lockfile SHA-256 is included in the retained evidence bundle. The bundle also records the checked-in Rust toolchain file, rustc/cargo/Python versions, runner OS/architecture/image identifiers and kernel identity, with its own SHA-256 receipt.

## Deliberate v0.1 boundaries

- Only a synthetic local linear observation model is admitted.
- Measurement noise is independent across sensors and represented by one positive scalar variance per sensor. Correlated covariance, time dynamics, frequency response and nonlinear observability are not supported in v0.1.
- Parameter scaling is explicit because singular values and Fisher metrics depend on coordinate scaling.
- Rank and eigenvalue tolerances are fixed and reported. They are numerical policies, not physical detectability thresholds.
- D-optimal selection is unavailable when no full-rank subset exists.
- E-optimal results preserve rank and nullity; a best E-optimal subset can still be unobservable.
- No diagonal loading, ridge term or other regularization is applied to make a singular information matrix look observable.
- No probability-of-detection (POD) or false-alarm claim can be inferred from this synthetic model.

## Local execution

From the repository root:

    cargo fmt --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml
    cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml > /tmp/observability-result.json
    python3 tools/civil-aero-benchmark/inspection-observability/verify_independent.py --rust-output /tmp/observability-result.json

The Python verifier uses only the standard library and independently recomputes all six subsets, ranks, Fisher matrices, eigenvalues and selection results. It does not import Rust output-generation code.

## Next qualification steps

1. Add full covariance only with exact channel identity/order, calibration, units and source-bound covariance provenance.
2. Compose #4236 so each sensor's spatial/temporal transfer, mask, calibration and timebase is applied before comparison.
3. Add declared sensor dropout, drift and common-cause power/data failures, retaining observability loss explicitly.
4. Only after #4139 admits exact NASA BDF bytes and terms, derive candidate structural DOFs from its topology. Do not infer physical stiffness, masses, loads, damage signatures or sensor noise from unknown BDF fields.
5. Compare uniform/engineering placement, effective independence, QR-pivot, D-optimal, E-optimal, modal/strain-energy and decision-value-guided policies under identical total cost.
6. Use physical NDE/POD evidence for defect-detection claims; synthetic observability is not physical inspection qualification.

## Claim ceiling

The only positive claim is synthetic_local_observability_only. The output is not structural adequacy, damage detection, damage tolerance, fatigue, flutter, airworthiness, SHM safety or maintenance-release evidence.
