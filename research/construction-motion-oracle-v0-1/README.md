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

## Co-translating capsule CCD

The co-translation extension handles two rigid fixed-orientation capsules that each translate linearly over one shared normalized time interval. It reduces the pair to relative translation `moving_delta - obstacle_delta` and reuses the three-variable convex active-set oracle. Closest points are reconstructed in world coordinates.

It separates synchronized continuous narrow-phase distance/contact, endpoint-only synchronized baseline, and union-of-swept-AABBs broad-phase candidate (which may overlap even when the bodies never meet at the same common time).

Tests cover head-on mid-interval contact with clear endpoints, equal-velocity co-motion, unequal same-direction velocities, tangency, swept-AABB false positive, skew 3D axes, swapping symmetry, rigid transforms, scaling, invalid inputs and deterministic dense-sampling sanity checks.

This is not arbitrary continuous-to-continuous collision checking: both bodies must retain fixed orientation and share the same time parameter. Rotation, joint-space interpolation, articulated robots, deformable geometry, asynchronous clocks and physical safety are outside this API. See [#5453](https://github.com/Luminous-Dynamics/symtropy/issues/5453).

### Time-coherent broad phase

For two translating capsules, the full swept-AABB union ignores whether the bodies occupy overlapping coordinates at the same time. The new dyadic time-sliced candidate query partitions the shared normalized interval and computes a conservative swept AABB for each body within each slab. It returns the number of candidate slabs, not a collision verdict. Narrow phase remains mandatory for positive candidates.

The construction invariant is one-sided: if the exact synchronized continuous narrow phase reports contact, at least one slab must remain a candidate for any positive subdivision count. For nested 1, 2, 4, 8, ... refinements, candidate time fraction is non-increasing; raw candidate slab count may increase when parent slabs split. More subdivisions tighten broad-phase selectivity but cost more tests; benchmark both cost and candidate reduction. This is still an AABB broad phase, not a safety guarantee or exact dynamic collision result.

## Absolute-time capsule motion spans

The timed adapter accepts two capsule motion spans whose start and end timestamps may differ. It requires matching site epoch, target frame and monotonic clock-domain identity, intersects the half-open execution intervals, and rebases each capsule's linear translation onto the common interval before calling the relative-motion CCD oracle.

- Non-overlapping or boundary-touching half-open intervals return `NoTemporalOverlap`, not a "clear" collision verdict.
- A positive overlap is checked continuously, and its terminal geometric limit is included conservatively. If closest approach is exactly at that excluded end limit, `closest_at_terminal_limit` is set.
- The current version rejects non-zero declared clock uncertainty as `ClockUncertaintyUnmodeled`. It does not invent clock synchronization or widen the geometry/time model.

This remains a fixed-orientation linear-translation geometry primitive. It is not an asynchronous multi-clock solver, articulated robot collision system or safety function. See [#5478](https://github.com/Luminous-Dynamics/symtropy/issues/5478).

## Multi-body time-swept broad phase

The multi-body owner in `multibody_time_swept_broadphase.rs` consumes absolute-time `CapsuleMotionSpan` records with unique body IDs and common site epoch, frame and exact monotonic clock domain.

It partitions the global horizon into deterministic dyadic slabs. In each slab it builds each active body's outward-rounded swept AABB, chooses the axis with greatest endpoint spread (stable X/Y/Z tie order), and performs sweep-and-prune. Coarse spatial overlaps are then checked for positive time co-activity and both body bounds are rebuilt on the *same common interval* before a pair candidate is emitted. Candidate pairs and slab receipts are sorted deterministically.

The report distinguishes coarse spatial pairs, time-coherent pairs and final candidate pairs. Each pair records active duration, candidate duration, candidate-time fraction and the exact slab intervals. This is useful for diagnosing broad-phase selectivity without confusing a pair candidate with a collision result.

Bounds are widened by eight representable floating-point steps on every face to reduce inward rounding in this software broad phase. This is a numerical guard, not a formal directed-rounding proof. Input spans must be finite and use zero declared clock uncertainty; non-zero uncertainty abstains through the absolute-time validator.

The Rust benchmark measures sparse scaling at 16/64/256/1,024/4,096/10,000 bodies, plus clustered, dense and long-thin strata through 1,024 bodies. A whole-horizon all-pairs AABB reference is measured for sparse cases through 1,024 bodies. Report all density strata: dense candidate generation is inherently quadratic when nearly every pair overlaps.

The owner is broad phase only. Every emitted pair still needs the existing timed narrow-phase oracle; no emitted pair means only that this supported conservative AABB method did not identify a candidate under the supplied geometry/time model. Rotation, articulated trajectories, non-zero clock uncertainty and dynamic-human safety remain out of scope. See [#5482](https://github.com/Luminous-Dynamics/symtropy/issues/5482).
