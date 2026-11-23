# Temporal Wrench Transfer v0.1

A dependency-free Rust research oracle for redistributing a previous frame's normal impulse over a new planar contact manifold while preserving the resultant normal-force wrench as closely as possible.

## Model and exactness boundary

- All old and new contacts share a fixed +Z normal.
- Previous per-contact normal impulses are finite and non-negative.
- Total normal impulse is conserved exactly.
- New contact impulses are non-negative with no per-point upper bounds.
- The controllable moment components are Mx and My, represented by the planar center of pressure (CoP).
- Tangential friction, torsional moments, changing normals, compliance and full solver dynamics are out of scope for v0.1.

The old resultant wrench is summarized by total normal impulse N and target CoP:

    c_target = sum_i(lambda_i * r_i) / N

The new manifold can realize any CoP in the convex hull of its point positions. The transfer problem is therefore the Euclidean projection of c_target onto that convex hull, followed by a non-negative convex allocation of N across the selected points.

In two dimensions, Caratheodory's theorem guarantees that any point in the convex hull can be represented by at most three vertices. This implementation enumerates points, segments and triangles:
- a vertex is a one-point allocation;
- a segment uses a clamped scalar projection;
- a triangle uses barycentric coordinates if the target lies inside it.

If the old CoP lies inside the new hull, the prior resultant normal force and moment are preserved exactly. If it lies outside, the closest achievable CoP minimizes the Euclidean CoP error, which is equivalent to minimizing the moment-error norm for fixed N and shared normal. All transferred impulses remain non-negative.

Tie-breaks are deterministic: minimum residual, then fewest active points, then stable contact IDs, then lexical weight order. Input order does not affect the result.

## Why this matters for manifold reduction

A spatially good manifold can still be temporally poor if its selected points shift the center of pressure and the warm-started impulses are redistributed without preserving the old moment. Conversely, reusing stale points can be wrong after contact topology or normal changes. A full production algorithm must record whether old state was reused, transported, clamped or invalidated.

The oracle now includes the current scalar-cache baseline: scale the previous total normal impulse by 0.8, then distribute it equally across all new manifold points. The result reports both normal-force attenuation and the induced moment/center-of-pressure error. It can be compared against exact non-negative CoP-preserving redistribution on identical old/new contact sets.

The wider programme is designed to compare:
- current total-normal-impulse cache plus equal distribution;
- nearest-point/contact recycling;
- feature-ID and normal-frame matching;
- wrench-preserving redistribution to the new manifold;
- cold-start reference.

It does not assert that wrench transfer is always the correct physical choice. Stale-history invalidation, impact/reversal and frictional dissipation remain independent constraints.

## Tests

The unit suite contains 11 tests and covers:
- exact preservation when old CoP lies inside the new contact hull;
- closest-hull projection when old CoP lies outside;
- a triangle interior barycentric allocation;
- one-point collapse;
- contact/source ordering invariance;
- exact total-normal-impulse conservation;
- current equal-distribution warm-start residual in force and moment;
- invalid warm-start scaling;
- invalid, duplicate and non-finite input rejection;
- zero-impulse undefined-CoP rejection.

Run:

```sh
cargo fmt --manifest-path research/temporal-wrench-transfer/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/temporal-wrench-transfer/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/temporal-wrench-transfer/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Claim ceiling

Exactness is limited to the planar shared-normal normal-impulse model. This does not prove frictional warm-start stability, energy non-injection, better solver iteration count or physical task transfer.
