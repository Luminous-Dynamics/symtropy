# Construction Motion Oracle & Differential Benchmark v0.1

**Status:** synthetic analytic reference for two bounded geometry families. Not a general collision checker, planner or safety qualification.

Parent implementation: [motion envelope #5345 / PR #5347](https://github.com/Luminous-Dynamics/symtropy/pull/5347). Programme issue: [#5363](https://github.com/Luminous-Dynamics/symtropy/issues/5363).

## Independent oracle

This crate uses separate mathematical formulas for:
- closest approach between a straight 3D segment and a stationary sphere;
- closest approach between an upper semicircle and a stationary sphere;
- exact coordinate extrema for those path families.

It consumes the envelope producer's output only to test containment and broad-phase candidates. It does not reuse the producer's AABB expansion helpers.

The semicircle case is deliberately adversarial: its endpoint chord is the diameter, which misses an obstacle at the arc apex. Endpoints-only and chord-only baselines therefore return false while the analytical continuous oracle returns true.

## Capsule narrow phase

The follow-on capsule module represents a stationary capsule as a finite 3D centreline segment plus radius and computes the exact finite-segment closest approach to a moving sphere's straight centre path. It handles parallel, collinear, skew and degenerate segments. The swept-sphere/capsule AABB is broad phase only; the segment-distance result is the analytic narrow phase.

This is not a rotating capsule, an articulated link sweep or a general mesh CCD implementation.

The translating-capsule child adds bounded continuous distance for a rigid capsule translated linearly without rotation against a stationary capsule. It minimizes the squared distance over translation time and both finite centerline parameters in the unit cube by enumerating the 27 active/free states of the convex quadratic. It returns the closest time and both closest-axis parameters. Exactly singular free faces are covered by their lower-dimensional boundaries. A nonzero free-face pivot below the relative conditioning threshold (1e-12 of the largest matrix coefficient) returns `NumericallyUnresolved`; callers must not convert that result into a clear verdict. Endpoint-only and swept-AABB baselines remain separate methods. This is not rotating-capsule CCD or articulated-link kinematics; see [#5395](https://github.com/Luminous-Dynamics/symtropy/issues/5395).

## Current synthetic corpus

The machine-readable fixture contains 12 explicit geometry cases with path endpoints/arc parameters, body and obstacle radii, all seven per-axis margin channels, expected analytic/endpoints/chord/physical/stopping/coordination outcomes, primitive-manifest completeness, and six controlled mutations. All current cases are development cases; there is no claim of a statistically sufficient held-out corpus yet. See schema/oracle-corpus-v0.1.schema.json. Adapter results use schema/adapter-result-v0.1.schema.json.

The Rust unit suite includes:
- straight-segment analytic containment;
- explicit detection of an understated chord-deviation claim;
- arc collision missed by endpoints and chord baselines;
- thin barrier crossed between safe endpoints;
- conservative AABB false-positive candidate;
- separation of physical collision from stopping/policy overlap;
- uncertainty monotonicity;
- uniform-scale collision invariance;
- capsule transverse hit, parallel near miss, exact tangency, skew/parallel/collinear/degenerate segments, endpoint clamping, broad-phase false positive, and capsule rigid-transform/scale invariance;
- translating-capsule contact between clear endpoints, separated parallel axes, exact tangency, skew 3D axes, zero translation, degenerate axes, endpoint-clamped optimum, AABB false positive, rigid-transform/scale invariance, invalid input and deterministic generated one-sided sampling controls.

All values are synthetic and expressed in millimetres. The upper semicircle is planar, has a fixed z coordinate and covers exactly 180 degrees. The linear reference is piecewise straight. The sphere oracle does not prove general articulated-robot containment or arbitrary mesh swept volumes. The capsule extension covers moving sphere versus stationary capsule and fixed-orientation linearly translating capsule versus stationary capsule. It does not cover rotation, articulated motion or general mesh CCD.

## Industry differential adapters

The frozen inputs/results can be adapted to:
- MoveIt continuous robot-world checking with the exact detector backend and collision matrix recorded;
- Tesseract CONTINUOUS and LVS_CONTINUOUS evaluators;
- additional independently versioned checkers.

The MoveIt continuous robot-world API and self-collision checks are distinct scopes; backend support must also be verified. The currently inspected MoveIt FCL continuous overload is a stub, while the Bullet tutorial documents CCD. Tesseract evaluator interpolation/cast rules must be recorded. No adapter result is accepted as analytical truth, and a missing backend is NOT_RUN, never PASS.

See docs/differential-adapter-contract.md.

## Commands

- cargo fmt --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml -- --check
- cargo test --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --locked
- cargo clippy --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --locked --all-targets -- -D warnings
- cargo run --release --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --bin differential-bench

## Claim ceiling

Passing tests establish only the implemented mathematics for the declared line and planar semicircle families and the adapter's tested structural behavior. They do not establish arbitrary continuous collision detection, physical robot model accuracy, localization/tracking/stopping truth, safe separation, construction productivity, standards conformity or vendor interoperability.

## Two independently moving capsules

The two-moving-capsule child reduces independent constant translations to relative translation: `d_relative = d_A - d_B`. It runs the existing fixed-orientation translating-capsule oracle against the other capsule's initial pose, then applies `d_B * t` to both closest points to return the shared world-frame contact geometry.

This supports two rigid capsules with fixed orientations and constant linear translations over the same normalized interval. It does not cover rotation, articulated links, acceleration, deformation, uncertain tracking, or physical human-protection qualification.

The child provides:
- exact relative-motion narrow phase and closest world points at the common time;
- a separate endpoint-only baseline;
- conservative per-object swept-AABB candidate generation;
- typed propagation of non-finite, overflow and near-singular `NumericallyUnresolved` outcomes;
- metamorphic and adversarial tests for crossing between clear endpoints, equal velocity, closing/receding, tangency, time reversal, common rigid transforms, scaling, near-parallel abstention and broad-phase containment.

The Linux formula benchmark prints separate latency distributions for relative narrow phase, endpoints-only and per-object swept-AABB. These are formula microbenchmarks, not moving-robot field performance or a MoveIt/Tesseract comparison.
