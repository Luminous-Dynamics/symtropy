# Magnetic encoder function benchmark (v0.1)

A small, zero-dependency Rust reference for evaluating the field-to-angle function of a multipole magnetic encoder track.

The key shift is from scoring a magnet by a nominal field component to scoring the signal that a sensor would use. Given a declared pole-pair count p, mechanical reference angle theta, and independently calibrated phase-zero offset phi0:

- expected electrical phase = p * theta + phi0
- measured electrical phase = atan2(By, Bx)

The evaluator reports circular phase error, field-amplitude margin/ripple and out-of-plane leakage separately. Hard limits are evaluated independently; there is no weighted aggregate score.

## Inputs

Each trace contains a full mechanical revolution sampled uniformly at one fixed sensor plane. It is bound to:
- an opaque trace ID;
- a coordinate-frame ID;
- an operating-condition ID binding the sensor plane, air gap and declared environmental conditions;
- an angular-grid ID;
- an independent phase-zero calibration-record ID;
- an evidence class: synthetic scenario or physical build;
- samples containing mechanical reference angle and Bx, By, Bz in SI units.

Traces in a set must share the exact evidence class, frame ID, grid ID, sample count and sample-angle coordinates. The evaluator does not interpolate or silently reorder samples. The phase-zero offset must come from a separate calibration record; the trace under evaluation is never used to fit its own zero. Electrical phase progression is signed (+1 or -1) so both magnetic rotation senses are represented. The condition ID prevents traces at different air gaps, temperatures, eccentricity or sensor registrations from being silently pooled.

## Reported metrics

- peak, RMS and nearest-rank p95 absolute electrical phase error;
- minimum, mean and maximum in-plane field amplitude;
- amplitude ripple: (maximum - minimum) / mean;
- RMS out-of-plane / in-plane field ratio;
- number of samples below the required in-plane field threshold;
- pass/fail for each independently declared hard limit.

The set evaluator reports worst-case metrics and requires every supplied trace to pass. This is a finite-set robustness result only. It does not infer probabilities, CVaR, production yield or lifetime reliability.

## Scope boundaries

This v0.1 evaluator does not simulate a magnet, infer magnetization, solve demagnetization or nonlinear B-H response, model Hall-sensor/ASIC calibration or bandwidth, or qualify a physical encoder. It validates the arithmetic and identity/acceptance contract for supplied traces. Synthetic unit tests are not physical data.

No machine-control, magnetization, component-release or certification authority is provided.

## Run

    cargo fmt --manifest-path research/magnetics/encoder-function-benchmark/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/encoder-function-benchmark/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/encoder-function-benchmark/Cargo.toml


## Covariance-to-phase uncertainty

The crate also provides `propagate_phase_uncertainty`, which consumes an identity-matched per-sample covariance trace for (Bx, By), plus independent angular-reference uncertainty records.

For each sample, the atan2 Jacobian is:

    g = (-By / (Bx^2 + By^2), Bx / (Bx^2 + By^2))

The field-measurement contribution is `g * Sigma_Bxy * g^T`. Independent reference contributions are added separately:

    sigma_phi_total^2 =
        sigma_phi_measurement^2
        + (pole_pairs * sigma_mechanical_reference)^2
        + sigma_phase_zero_calibration^2

The phase-zero calibration ID must match the ID in the phase requirements. Covariance samples must match trace ID, coordinate frame, operating condition, grid ID and bit-identical sample angles. 2x2 covariance inputs are checked for finite values and positive semidefiniteness.

The output reports measurement-only and total phase standard uncertainty separately, including mean, RMS, p95 and maximum. It also reports residual / standard-uncertainty ratios as descriptive diagnostics; they are not p-values or coverage estimates.

The model does not assume independent errors across samples. Shared calibration, registration and thermal errors may be correlated across the whole trace; per-sample marginal covariance does not encode that joint structure. No Gaussian coverage, confidence interval or production-yield claim is made. A field below the declared minimum is rejected for phase-uncertainty propagation.
