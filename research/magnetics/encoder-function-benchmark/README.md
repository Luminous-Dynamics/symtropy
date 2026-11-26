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

The same-condition trace-set evaluator requires exact operating-condition identity. The additional operating-condition tournament allows distinct condition IDs (e.g. air gap, eccentricity, tilt or temperature) while fixing evidence class, frame, grid and exact sample angles. It preserves each scenario's condition ID, reports worst-case phase/field metrics and requires every supplied scenario to pass. It is a finite-set robustness result only; it does not infer probabilities, CVaR, production yield or lifetime reliability.

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


## Test inventory

The current source contains 29 authored unit tests: 14 core field-to-angle controls and 15 covariance-to-phase / uncertainty negative controls. They cover known answers, rotational covariance, field-amplitude scaling, pole-pair scaling, rank-one PSD, indefinite and non-finite matrices, identity mismatches, weak/zero field and zero-uncertainty residuals. Test authorship is not a test pass; consult the current exact-head PR checks before using the crate as a verified reference.


## Response-aware magnetic design tournament

The crate also exposes `design_tournament::rank_magnetic_designs`. It accepts a finite list of complete candidate design field-map archives. Each candidate binds:
- exact geometry, material and field-archive SHA-256 digests;
- one or more exact solver-run IDs;
- a family of field traces covering its preregistered operating conditions.

All candidates must share the same ordered operating-condition IDs, evidence class, coordinate frame, angular-grid identity and bit-identical mechanical-angle coordinates. Solver-run IDs cannot be reused to identify two different candidate designs. Conditions within each candidate may differ (for example, nominal air gap, gap extremes, eccentricity, tilt or temperature), but one failing condition rejects the candidate. The same hard encoder requirements and independently sourced phase-zero calibration are used for every design. No candidate is allowed to fit its own phase zero.

Among fully accepted candidates, the tournament preserves a Pareto frontier over minimum in-plane field (maximize), worst peak and RMS phase error (minimize), worst amplitude ripple (minimize), and worst out-of-plane/in-plane ratio (minimize). It does not create a weighted scalar score or declare a single winner when designs trade off objectives.

The tournament only ranks supplied solver outputs. It does not generate a magnetization pattern, solve a field, authenticate the source digests, prove global optimality, infer manufacturing yield or qualify a physical device.

The current design-tournament module contains ten authored unit tests. Authored tests are not a test pass; consult the exact-head CI checks.


## Distribution-free bounded design tournament

The crate also exposes `bounded_design_tournament::rank_bounded_magnetic_designs`. It composes with the nominal response-aware design tournament and binds each uncertainty record to the exact field trace, frame, operating condition, grid, field-archive digest, solver-run ID and independent phase-zero calibration.

Each sample carries:
- a simultaneous Euclidean 3D field-vector error radius, tesla;
- a simultaneous absolute mechanical-reference-angle error bound, radians.

The same field-error radius conservatively bounds its in-plane projection and out-of-plane component. For nominal in-plane magnitude `m` and radius `r < m`, the maximum field-direction error is `asin(r/m)`. The total sample phase-error bound adds `pole_pairs * mechanical_angle_error_bound`, the independent sensor/field-frame rotation bound, and the independent absolute phase-zero error bound. The per-scenario report keeps field-direction, mechanical-reference, sensor-frame and phase-zero contributions separate. If `r >= m`, zero in-plane field is admissible, so phase is unbounded and the scenario fails closed.

The module reports nominal results and robust upper/lower bounds separately. It computes conservative peak/RMS/nearest-rank-p95 phase errors, minimum field lower bound, amplitude-ripple upper bound and RMS cross-axis-ratio upper bound. The latter two use adverse numerator/denominator bounds. Accepted robust designs are Pareto-ranked on these separate worst-case metrics; no weighted score or probability is inferred.

The error radii must be simultaneous deterministic bounds. Standard deviations, typical datasheet curves and finite-scenario variation are not silently converted into hard bounds. The source IDs are identity links, not cryptographic authentication.

The bounded module currently contains eleven authored unit tests, including an exact `asin(r/m)` field-direction control, sensor-frame rotation propagation and cross-candidate uncertainty-record uniqueness. Authored tests are not a test pass; the stacked PR's exact-head checks must establish formatting, Clippy and test results.
