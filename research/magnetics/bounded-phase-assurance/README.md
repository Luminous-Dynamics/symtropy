# Magnetic bounded phase assurance (v0.1)

A zero-dependency Rust reference for distribution-free, deterministic assurance of a multipole encoder's field-to-angle function.

## Why this exists

The covariance-to-phase evaluator in the magnetic encoder function benchmark reports first-order standard uncertainty. This crate answers a different question:

**Can the encoder's hard requirements be guaranteed for every realization inside independently established absolute error bounds?**

It does not turn standard deviations into hard bounds, and it does not infer the bounds itself.

## Bounded-error model

For a measured in-plane field vector

    b = (Bx, By)

and a deterministic Euclidean error-radius bound (r), assume the true vector lies in the disk

    ||b_true - b||_2 <= r.

When (r < ||b||_2), the disk does not reach the origin. Its maximum angular deviation from the measured vector is

    delta_phi_field = asin(r / ||b||_2)

and the true field magnitude has the lower bound

    B_min = ||b||_2 - r.

If the disk touches or contains the origin, the phase is not guaranteed to be defined and the minimum guaranteed field is zero. The evaluator reports a conservative pi-radian phase-error ceiling and fails the positive minimum-field requirement.

For pole-pair count (p), mechanical-reference absolute error bound (delta_theta), and independent phase-zero absolute error bound (delta_phi_0), use the conservative worst-case sum

    delta_phi_total <= delta_phi_field + p * delta_theta + delta_phi_0.

The per-sample circular absolute phase-error upper bound is

    min(pi, abs(wrapped_nominal_residual) + delta_phi_total).

The trace-level peak, RMS and nearest-rank p95 metrics are computed from the per-sample upper bounds. The corresponding margins remain separate; there is no weighted composite score.

## Evidence requirements

- Field-vector radii must be deterministic absolute bounds, simultaneously valid for every sample in the trace.
- Mechanical-reference and phase-zero values are absolute error bounds, not standard deviations.
- Field-bound, angular-bound and phase-zero calibration records are independent of the trace under evaluation.
- The angular-error record binds to the exact trace ID, coordinate frame, operating condition and grid, as well as the phase-zero calibration ID.
- Calibration identity, coordinate frame, operating condition, grid and evidence class are preserved.
- The trace must be a full, uniformly sampled revolution. No interpolation, sorting, offset fitting or implicit registration is performed.
- Synthetic scenarios and physical builds remain different evidence classes.

Opaque record IDs provide identity binding only. This software does not authenticate records or establish traceability. The validity of any physical guarantee is limited by the validity of the supplied simultaneous bounds.

## What it does not claim

No probability, confidence coverage, yield, CVaR, lifetime reliability, statistical calibration, sensor-ASIC model, physical qualification, solver validation, magnetization authority, machine-control authority or product-release authority is provided.

A passing software test proves the arithmetic and contract only. It does not prove that the supplied bounds hold for a manufactured component.

## Run

    cargo fmt --manifest-path research/magnetics/bounded-phase-assurance/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/bounded-phase-assurance/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/bounded-phase-assurance/Cargo.toml

## Relationship to adjacent work

- Symtropy #7413: continuous encoder function and covariance-to-phase standard uncertainty.
- Symtropy #7452: first-order covariance propagation.
- Symtropy #7449: Hall-latch and digital quadrature state model.
- Symtropy #7454: reconciliation of captured A/B transitions against model predictions.
- Symtropy #7334: calibrated 3D field metrology.
- Symtropy #7474: this bounded, distribution-free acceptance layer.
