# Orbital time-varying acquisition benchmark v0.1

A Rust-first synthetic evaluator for projecting **time-varying point-support motion envelopes** into exposure, rolling-row and complete-frame image displacement bounds.

## Kinematic envelope

At the first-row exposure midpoint (t=0), the supplied whole-support bounds are:
- point speed magnitude V0;
- point acceleration magnitude A0;
- point jerk magnitude J.

With jerk magnitude bounded across the interval, the speed envelope is:

`V(t) <= V0 + A0*|t| + 0.5*J*t^2`.

The bounds must apply to every point in the declared support and conservatively include all relative motion effects relevant to the upstream motion model. The evaluator does not derive or qualify those upstream bounds.

## Three distinct integration windows

Let E be row exposure duration and T the first-to-last row exposure-midpoint span.

Row midpoint displacement:
`Drow = V0*T + A0*T^2/2 + J*T^3/6`.

Worst row exposure, centred at the latest row midpoint:
`Dexp = V0*E + A0*I1(T,E) + J*I2(T,E)/2`.

- If T >= E/2: I1 = T*E.
- Otherwise: I1 = T^2 + E^2/4.
- I2 = E*T^2 + E^3/12.

Complete frame displacement must be integrated **directly** over [-E/2, T+E/2]:
`Dframe = V0*(T+E) + A0*I1frame + J*I2frame/2`.

- I1frame = (E^2 + (2*T+E)^2)/8.
- I2frame = ((2*T+E)^3 + E^3)/24.

Do not replace the direct complete-window integral with Drow + Dexp. The TV-016 frozen counterexample shows that this shortcut can under-bound when T < E/2. All terms use checked integer arithmetic and ceiling division. Pixel bounds apply the same minimum-depth perspective derivative used by #7399.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-time-varying-acquisition-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-time-varying-acquisition-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-time-varying-acquisition-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-time-varying-acquisition-bench/Cargo.toml --bin orbital-time-varying-acquisition-bench -- docs/research/orbital-truss/fixtures/time-varying-acquisition-v0_1.json <40-hex-source-commit>
python3 tools/orbital-time-varying-acquisition-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/time-varying-acquisition-v0_1.json <40-hex-source-commit>
```

The independent Python oracle uses exact standard-library `fractions.Fraction` arithmetic and a separate derivation. It is test-only.

## Claim boundary

This evaluates supplied synthetic point-speed, acceleration and jerk bounds. It does not validate a motion estimator, acceleration/jerk envelope, robot model, calibration, clock mapping, blur, feature observability, measurement uncertainty, POD/NDE or flight readiness. Physical promotion requires time-synchronized motion capture/encoder/IMU evidence, a measured whole-support motion envelope, independent image-space reference trajectories and repeated sessions.
