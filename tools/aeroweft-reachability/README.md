# AeroWeft Reachability Core — Research Slice 0.1

This isolated, dependency-free Rust crate propagates a position interval through a sequence of **caller-supplied bounded velocity envelopes**. It emits an axis-aligned outer occupancy box for every half-open time slice and carries a profile identity, revision, frame and vertical-datum identifiers.

## What is implemented

- Finite closed intervals with outward one-representable-step expansion after binary64 addition and multiplication.
- Separate additive velocity contributions for nominal, state, model, control and environmental terms.
- Contiguous UTC Unix-millisecond slices with an exact-integer time-domain guard.
- Conservative occupancy over the full time slice, not just at the endpoint.
- A separately propagated endpoint enclosure for composition with the next time slice.
- Fail-closed input and arithmetic errors.
- Eight synthetic unit-test definitions covering containment, monotonicity, time translation, replay determinism and invalid-input controls.

## What is not implemented

This crate **does not derive velocity envelopes from aircraft physics**. It does not model bank angle, coordinated turns, aerodynamic forces, aircraft performance, wind-field evolution, geodesic transformations, dynamic control-history consistency, probability calibration, sensor fusion or operational airspace. A caller must prove that every supplied velocity interval contains all instantaneous velocities over its complete time slice.

The axis-aligned boxes discard correlation and can inflate rapidly. The profile digest is syntax-checked only; no publisher authentication or profile truth is established. One-ULP outward expansion is a floating-point enclosure primitive for this arithmetic only; it is not a complete proof of model soundness.

**This is a mathematical integration foundation, not a flight model, route validator, conflict-clearance service or safety system.** It must not be connected to live ATC/UTM, dispatch or aircraft control.

## Run locally

```sh
cargo fmt --manifest-path tools/aeroweft-reachability/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/aeroweft-reachability/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/aeroweft-reachability/Cargo.toml
```

The authoring environment did not have `cargo` or `rustc`, so these tests are authored, not executed here. A dedicated workflow is included to run the exact-head checks.
