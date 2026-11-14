# Locomotor Gait Hysteresis Reference V0

Standalone, dependency-free Rust reference model for [LOCO-HYSTERESIS-001 / #3964](https://github.com/Luminous-Dynamics/symtropy/issues/3964).

## Purpose and claim boundary

This crate distinguishes a memoryless single speed threshold from a simple two-threshold hysteretic gait-regime selector.

It makes three properties executable:
- the up-transition and down-transition can occur at different measured speeds;
- the same instantaneous speed can retain different regimes depending on the initial/history state;
- a single threshold can chatter when a noisy or held signal sits exactly at the threshold, while a hysteresis band can retain its current state.

This is a deterministic synthetic reference. It does not identify biological hysteresis, infer a neural CPG, model receptor adaptation, simulate physical contact, or account for sensor/actuator delay. A measured up/down loop may also be caused by ramp-rate effects, observation latency, actuator lag, mechanics, or a biased gait classifier. This crate now includes an exact first-order lag reference that demonstrates an apparent loop with a memoryless threshold when event timing is scored in actual-speed coordinates.

## Models

The single-threshold baseline changes:
- Wave -> Tripod when achieved speed is greater than or equal to the threshold;
- Tripod -> Wave when achieved speed is less than or equal to the same threshold.

The hysteretic selector uses two declared thresholds:
- Wave -> Tripod at the upper threshold;
- Tripod -> Wave at the lower threshold;
- retain the current regime inside the band.

The initial regime is an explicit input. It is not inferred from instantaneous speed.

## Delay-only negative control

The crate includes a first-order lag model for a piecewise-linear achieved-speed input:

    tau * dy/dt = v(t) - y(t)

It integrates this first-order system exactly over each linear input segment. A single threshold is then applied to the lagged signal, while the loop width is measured using the actual speed at the detected event samples. This can produce a positive apparent up/down width without any state-dependent threshold.

A second known-answer test varies the ramp duration while holding the lag time constant fixed. Faster ramps produce a wider apparent loop. This is a synthetic confound demonstration, not a model of a particular sensor, actuator or animal.

## Measurement protocol

A sweep is a sequence of strictly increasing timestamps and finite, nonnegative **achieved/measured** speeds (not just commanded speed).

The first complete Wave -> Tripod -> Wave cycle reports:
- measured speed at the first sample triggering the up transition;
- measured speed at the first sample triggering the down transition;
- their difference as sampled hysteresis width;
- transition times.

Transition speeds are sample-grid dependent. This V0 does not interpolate threshold crossings. Comparisons across sample rates require an explicit common event-time/interpolation definition before they are considered comparable.

## Deterministic tests

Fourteen known-answer and negative-control tests cover:
- recovery of declared up/down thresholds and known width;
- zero width for a single-threshold baseline on a matched triangular sweep;
- same speed retaining different regimes inside the hysteresis band;
- single-threshold chattering at the switching point;
- hysteresis-band retention and reduced chattering;
- invalid or reversed thresholds;
- non-monotonic timestamps and non-finite speeds;
- negative speed rejection;
- incomplete sweeps abstaining rather than inventing a width.
- exact first-order lag response for a piecewise-linear speed ramp;
- apparent hysteresis from lag alone under a memoryless threshold;
- ramp-rate dependence of the latency-induced apparent loop;
- invalid lag constant and missing/invalid threshold-crossing rejection.

## Required next increments

1. Add rate-dependent transition dynamics beyond the current first-order lag negative control.
2. Add actuator-lag and mechanical-settling negative controls to distinguish apparent from genuine hysteresis.
3. Connect speed-dependent coupling continuation to the six-leg phase network in #3926.
4. Add basin/multistability and phase-pulse perturbation experiments near the transition.
5. Add repeated sweeps and individual/session-clustered uncertainty.
6. Integrate measured contact, support, load and sensory observations through #3345/#3693.
7. Evaluate held-out full transition trajectories with frozen forecasts through #3829.
8. Compare against CPG, coupling-modulated CPG, proprioceptive DRL-CPG and reference-free MPC under matched body, physics, sensing and compute through #3357/#3475.

## Run

    cargo fmt --manifest-path research/loco-gait-hysteresis-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-gait-hysteresis-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-gait-hysteresis-v0/Cargo.toml --locked --all-targets -- -D warnings

## Literature anchors

- Maoudj et al. (2026). *SPINDER: an open-source 18-DoF hexapod robot with hierarchical central pattern generator control and analytic inverse kinematics*. Scientific Reports 16:26147. https://doi.org/10.1038/s41598-026-56856-0
- Jiang et al. (2026). *Bio-Inspired CPG Modulation via Proprioceptive Deep Reinforcement Learning for Adaptive Hexapod Locomotion Across Terrain Transitions*. Biomimetics 11(8):570. https://doi.org/10.3390/biomimetics11080570
