# Magnetic field covariance reference

A zero-dependency Rust research crate for:
- validating symmetric 3x3 positive-semidefinite covariance matrices;
- representing position covariance in m² and direct field-sensor covariance in T²;
- propagating position covariance through an explicit local magnetic-field Jacobian;
- adding the directly measured field-sensor covariance.

## Run

    cargo fmt --manifest-path research/magnetics/field-covariance/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/field-covariance/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/field-covariance/Cargo.toml

The nested Cargo workspace is isolated from the production Symtropy workspace
and lockfile.

## Important semantics

The output is the first-order approximation

    Sigma_B = G * Sigma_x * G^T + Sigma_sensor

where G is a local field Jacobian. A valid Jacobian must be supplied; a missing
Jacobian is not equivalent to an all-zero matrix. The output is local and does
not cover nonlinear or large-displacement effects.

Common-mode calibration, registration and temperature errors shared across
multiple samples remain map-level common-mode terms. They must not be entered
as independent per-sample sensor noise. This crate does not implement a
map-level common-mode covariance model.

A zero covariance is an explicitly known mathematical zero, not a representation
for missing or unknown evidence. This crate does not estimate covariances,
calibrate probes, or establish predictive coverage.

Synthetic tests verify matrix arithmetic only. They are not physical field
measurements, calibration certificates or component qualification.
