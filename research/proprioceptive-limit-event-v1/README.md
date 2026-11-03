# Proprioceptive Limit Event V1

A dependency-free, fixed-point event detector for joint-limit observations in the Symtropy embodied sensorimotor research programme.

## Scope

This package implements one narrow mechanical/observation contract:

- detect an upper- or lower-limit threshold crossing from ordered, timestamped joint-angle samples;
- use fixed-point microradians rather than floating-point comparisons at the event boundary;
- emit at most one event per approach until a declared hysteresis release boundary is crossed;
- reject wrong-joint, non-monotonic, stale-gap, out-of-range and malformed body-snapshot inputs;
- bind each event to the two body-state digest references that bracket the crossing;
- never infer a crossing across an unobserved interval.

A detected event localizes the crossing only to the interval between its two samples. It does not invent a sub-tick event time.

## Biological boundary

This is not a CxHP8 model and does not reproduce the Pratt et al. paper. It does not encode a biological threshold, receptor adaptation, calcium kinetics, optogenetic response or motor-circuit mapping. Thresholds and joint ranges are caller-supplied calibration outputs. A source-grounded receptor model must be validated separately before its output can use a biological evidence tier.

The package emits an event only. It does not command a muscle, generate a reflex gain, mutate a body or bypass authoritative physics.

Related scientific work: Symtropy [ANIMA-NEURO-003 / #3456](https://github.com/Luminous-Dynamics/symtropy/issues/3456).

## Synthetic tests

The 16 tests cover upper/lower crossings, one-shot/latch semantics, hysteresis re-arming, initial conditions already beyond a limit, sample-gap rebaselining, out-of-order samples, wrong joint IDs, body-state digest syntax, declared angle bounds, zero ticks and invalid hysteresis configuration.

All tests are synthetic and establish only the event contract.

## Run

    cargo fmt --manifest-path research/proprioceptive-limit-event-v1/Cargo.toml -- --check
    cargo test --manifest-path research/proprioceptive-limit-event-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/proprioceptive-limit-event-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace under research/. It does not change the main Symtropy workspace or dependencies.
