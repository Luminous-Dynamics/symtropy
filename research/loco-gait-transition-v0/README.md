# LOCO Gait Transition V0

Standalone, dependency-free Rust reference implementation for [LOCO-HYSTERESIS-001 / #3964](https://github.com/Luminous-Dynamics/symtropy/issues/3964).

## Scientific question

Does an observed difference between upward and downward gait-transition speeds reflect a quasi-static hysteresis loop, or can it be explained by finite loop latency acting during a speed ramp?

A delay-only loop can create an apparent hysteresis width. Under matched, approximately linear up/down speed ramps with the same effective delay:

    observed_width(rate) = static_hysteresis_width + 2 * effective_delay * rate

V0 estimates the intercept and slope of this relation from matched up/down sweep pairs. The estimated delay is an **effective loop delay**: it combines any sensor, actuator, estimator, controller, transport or mechanical lag that enters the measured transition. It is not attributed to a biological receptor or neural pathway.

## Data contract

Each trace contains:
- stable trace and matched-pair IDs;
- explicit increasing/decreasing direction;
- strictly increasing timestamps;
- strictly monotonic **measured** speed samples (not merely commanded speed);
- a measured binary gait-regime label at every sample.

Each sweep must contain exactly one observed regime transition. Each pair must contain exactly one increasing and one decreasing sweep under matched conditions. The measured speed/time relation is checked for approximate linearity, and the two ramp rates must agree within a declared tolerance.

Transition events are returned as measured-speed brackets. The implementation reports their midpoints and retains bracket uncertainty; it does not pretend the exact transition occurred at an arbitrary point between two samples.

## Outputs

- Up-transition and down-transition measured-speed brackets and midpoint estimates.
- Apparent hysteresis width and propagated bracket interval for each matched pair.
- Ramp rate independently estimated from measured speed and timestamps.
- Linear width-versus-ramp-rate regression.
- Quasi-static hysteresis-width intercept.
- Effective loop-delay estimate (half the fitted width/rate slope), standard error and fit quality.
- Explicit refusal when there are too few distinct ramp rates, mismatched pair conditions, non-monotonic speed/time, multiple or absent transitions, or a poor linear-ramp fit.

## Known-answer and negative controls

The tests cover:
- genuine static hysteresis with zero effective delay;
- delay-only apparent hysteresis with zero static-width intercept;
- combined hysteresis plus delay;
- retained transition-bracket uncertainty;
- input-order invariance;
- duplicate trace and pair rejection;
- missing transition and multiple-transition rejection;
- monotonic timestamp and measured-speed checks;
- ramp-rate mismatch rejection;
- insufficient distinct ramp-rate rejection.

## Statistical and scientific limits

This is a deterministic measurement/analysis oracle, not a fitted gait-regime classifier or a biological result.

The linear decomposition assumes the effective lag is approximately constant across the tested rates and that up/down ramps are matched. A rate-dependent biological adaptation process, nonlinear speed tracking, contact settling, classifier latency or asymmetric dynamics can violate that assumption. The regression intercept must therefore be treated as a *quasi-static width under this model*, not proof of biological hysteresis.

The package does not infer gait labels from kinematics, estimate a transition-state dynamical model, separate sensor from actuator latency, model repeated-animal covariance, or establish a universal gait transition threshold. Those require independent data, explicit timing/latency measurements and held-out sweep prediction.

## Validation

    cargo fmt --manifest-path research/loco-gait-transition-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-gait-transition-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-gait-transition-v0/Cargo.toml --locked --all-targets -- -D warnings
