# Directional Motion Envelopes v0.1

Reference implementation for outward-rounded, per-axis swept/fat proxy envelopes under fixed-orientation translation.

## Contract

Given a tight AABB B and a supplied conservative signed displacement interval [d_min, d_max] per axis, this crate encloses the union of the initial AABB and all translated AABBs in that interval, then expands by an explicit nonnegative guard. The min side is rounded down and the max side rounded up at every arithmetic boundary.

The receipt binds stable collider identity (including body generation), world revision, frame, horizon, integrator digest, motion-bound producer digest, arithmetic profile, exact tight bounds, displacement interval, guard and output envelope. Replay recomputes the result and rejects any changed input.

## Symtropy semi-implicit Euler adapter

The crate also provides a one-step adapter that accepts the post-integrator linear velocity and timestep used by the current CCD path. The integrator first updates velocity, applies exponential damping and a speed clamp, then advances position using the resulting velocity. If post-angular-velocity squared exceeds the integrator's 1e-24 rotation threshold, the adapter returns UnsupportedRotation and no translation certificate.

With deterministic-net Q16.16 enabled, the world quantizes the post-integrator velocity and position before its CCD pass. The adapter includes an outward-rounded guard of half a position quantum plus dt times half a velocity quantum, accounting for the difference between the pre-quantized velocity used by integration and the post-quantized velocity used by CCD.

This adapter is bound to a supplied post-integrator state; it does not independently prove that the integrator inputs, force callback or velocity were produced correctly. Its trust boundary is the exact code path identified by the integrator digest plus the caller's state snapshot.

Rotational and unknown/unbounded motion produce typed unsupported statuses and no certificate. Non-finite arithmetic fails closed.

## Important trust boundary

The certificate proves that the output encloses the inputs' declared translation interval. It does not prove that the upstream displacement interval contains the body's real trajectory. That motion-bound producer must be qualified separately against an independent trajectory/swept-pair oracle. Rotation and unsupported shapes must use another qualified path.

## Run

```sh
cargo fmt --manifest-path research/directional-motion-envelopes/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/directional-motion-envelopes/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/directional-motion-envelopes/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Initial tests

- Mixed-sign 2D displacement intervals and directional rather than symmetric padding.
- Endpoint-box containment, stationary bounds and explicit guard expansion.
- 4D reference use.
- Semi-implicit Euler post-velocity sweep, endpoint enclosure, Q16.16 deterministic-net guard, rotation fallback, and receipt replay over velocity/timestep/quantization inputs.
- Unsupported rotation/unbounded motion returns no certificate.
- Replay rejection for changed identity, revision, horizon, arithmetic profile or displacement.
- Invalid identities, intervals, guards, bounds and arithmetic overflow.

This is an envelope derivation reference, not a full CCD algorithm, not a general trajectory proof and not a performance result.
