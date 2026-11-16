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

## Adaptive space-time branch-and-bound

The adaptive query recursively bisects the common normalized time interval only while per-window swept AABBs overlap. A disjoint pair safely prunes that interval. Remaining candidate leaves are checked with the exact capsule continuous-collision solver restricted to that leaf; local time is mapped back to the original trajectory. Candidate windows are sorted and disjoint in their interiors.

Outcomes are explicit:
- `Contact`: a narrow-phase witness proves contact.
- `NoContact`: every candidate interval was checked and clear, or all intervals were safely pruned.
- `Unresolved`: at least one candidate interval could not receive a reliable narrow-phase result.

An unresolved interval can never be re-labelled clear. An established contact takes precedence because the existence of contact is already proven.

The proof is limited to rigid fixed-orientation capsules undergoing affine translation over a common time domain: the union of endpoint AABBs encloses each capsule throughout a window. If the per-window boxes are disjoint, no instantaneous contact is possible in that window.

The adaptive query is benchmarked against the full-interval analytic query and the uniform time-sliced AABB baseline. Its purpose is selective pruning before expensive downstream narrow phases, not an assumed speed advantage over this already-small analytic kernel. Benchmark node visits, pruned intervals, candidate time fraction, exact narrow-phase calls and end-to-end latency together. See [#5541](https://github.com/Luminous-Dynamics/symtropy/issues/5541).

### Adaptive query input errors

The adaptive API returns typed errors for invalid normalized time windows, maximum depth above 20, and minimum window widths outside (0, 1]. These are input/contract failures, not collision outcomes. The shared `TranslationCcdError` owner exposes them alongside numerical conditioning and overflow errors so callers can distinguish invalid requests from `Unresolved` geometry.

## Replayable adaptive CCD certificates

The certificate API records one terminal ledger leaf for every part of the normalized time interval:
- pruned intervals include both conservative swept AABBs;
- exact-clear leaves include the analytic closest-approach result;
- contact leaves include a contact witness;
- numerically ill-conditioned leaves are recorded as unresolved.

The verifier requires the leaf windows to form a complete, ordered, gap-free dyadic partition of [0,1]. It recomputes the swept AABBs, checks every prune is truly disjoint, reruns exact narrow phase on checked leaves, and recomputes the overall status and counters. A successful no-contact verification therefore requires every terminal interval to be either a verified geometric prune or an exact clear result, with no unresolved leaves.

The certificate-producing variant evaluates every surviving candidate leaf rather than stopping at the first contact, so its ledger is complete. The existing adaptive query remains available for early-contact latency. Certificates are typed replay records, not signed/cryptographically authenticated attestations or formal proofs; callers must bind the embedded geometry and translations to exact design, site, robot, tool, calibration and controller identities in their outer evidence record.

The certificate API has a 65,536 terminal-leaf cap to bound memory use. See [#5651](https://github.com/Luminous-Dynamics/symtropy/issues/5651).
