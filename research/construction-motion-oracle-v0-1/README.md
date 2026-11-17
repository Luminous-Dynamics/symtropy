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


## Multi-body relative-time and clock-domain CCD reference

The relative-time multi-body owner composes the absolute-time capsule oracle, the bounded clock-uncertainty pairwise oracle and the multi-body body identity model. It does not invent a shared clock by renaming timestamps.

Each body supplies a source-clock motion span plus a qualified mapping record containing: source and destination clock-domain IDs, site epoch, mapping ID and evidence reference, an offset interval anchored at a reference source time, a hard drift bound in ppm, and a half-open validity interval. The adapter checks the complete motion span against that validity window, checks source/destination identity and the already-resolved world frame, then maps timestamps using the midpoint offset. It outward-rounds the half-width, adds maximum drift measured at the farthest interval endpoint, and includes source-span timestamp uncertainty in a hard symmetric bound.

Every unordered pair receives a deterministic result. Non-zero timing uncertainty can produce PotentialContactUnresolved, NoPossibleContact or NoPossibleTemporalOverlap; it cannot produce an exact collision or exact-clear disposition. Missing mapping evidence, expired mappings, missing drift bounds, wrong source clocks, wrong site epochs or frame mismatch leave affected pairs Unresolved. All pairs are retained, including unresolved pairs.

This initial owner is an O(n²) exhaustive reference implementation, capped at 1,024 bodies and 524,288 pair checks. The benchmark records pair count, disposition counts and p50/p95/p99 latency at 8/16/32/64 bodies. It deliberately does not reuse the exact-clock SAP broad phase, because that owner rejects non-zero clock uncertainty. A later conservative multi-clock broad phase must prove its candidate set is a superset of these reference results before it can replace the exhaustive evaluator.

The drift model is a hard relative-rate bound over the complete source interval, not a statistical standard deviation, synchronization protocol or clock-drift estimator. The adapter assumes the input geometry has already been transformed into one common frame; it does not perform frame registration. This is software-level collision reasoning only—not real-time control, physical stopping validation, human-protection logic, motion authorization or site safety qualification. Tracks [#5823](https://github.com/Luminous-Dynamics/symtropy/issues/5823) and [#5649](https://github.com/Luminous-Dynamics/symtropy/issues/5649).








### Cached body-slab bound refinement

The cached-refinement implementation builds each active body's swept AABB once per slab and shares that bound across every coarse candidate pair containing the body. It retains only one slab's body bounds at a time, so live cache memory is O(body_count), while the existing MAX_BODY_SLAB_CELLS cap still bounds total build work. During refinement, a pair first uses the two cached whole-active-slab bounds as a conservative rejection filter. If those bounds overlap, exact common-interval bounds are rebuilt only when either cached active interval is wider than the pair's shared active interval. When both cached intervals exactly match the common interval, their bounds are reused. If the coarse candidate set is empty, no slab cache is allocated or built.

The refinement receipt and benchmark report:
- total body/slab AABBs built across all slabs;
- pair/slab cached-bound tests;
- fast rejects using disjoint whole-active-slab bounds;
- common-interval pair rebuilds.

This is an attempt to reduce repeated rebasing and capsule construction on clustered/long-thin cases, where the first coarse-to-fine prototype paid per candidate pair × slab. It must preserve full direct-SAP candidate records exactly. A deterministic randomized differential test and focused tests for identical/different active intervals and empty coarse sets enforce the candidate-record and cache-accounting contracts.
### Direct algorithm comparison

The SAP-only benchmark runs the original full multi-slab algorithm and the hierarchical coarse-to-fine algorithm on the same normalized site-clock trajectories. It alternates timing order, performs warm-up calls, and reports direct/hierarchical p50/p95/p99 plus speedup, direct candidate count, coarse candidate count, refined candidate count and refinement pair-slab work. Exact complete candidate-record equality is asserted before timing is reported. This isolates the cost saved by avoiding a full body/index sweep per slab; it does not include clock-map construction or outer-envelope inflation. The integrated multi-clock-vs-exhaustive benchmark remains a separate end-to-end software path.
### Hierarchical coarse-to-fine refinement

The current multi-clock owner no longer rebuilds a full SAP for every requested time slab. It first performs one full-horizon coarse SAP pass, then refines only the emitted pair candidates by rebuilding both bodies' conservative bounds on their common active subintervals.

The work is therefore proportional to the one coarse body/index pass plus coarse candidates × refinement slabs, rather than rebuilding the body SAP independently for each slab. coarse_broadphase_candidate_pairs, broadphase_candidate_pairs and refinement_tested_pair_slabs distinguish the amount of initial work, retained output and refinement effort.

A parity test compares the complete refined candidate records against the direct full-SAP implementation for 1/2/4/8/16 subdivisions. A deterministic randomized test runs 12 seeds × 40 bodies across mixed start times, durations, 3D translations, radii and clustered/sparse positions, with complete candidate-record equality and coarse-candidate-superset checks at every subdivision. The same generated multi-clock reference candidate-superset tests remain in place. This is the intended optimization: the coarse result is a conservative superset; refinement only removes pairs/time segments whose common-interval outer AABBs are disjoint. It must never create a pair absent from the coarse superset or drop a pair present in the direct full-SAP reference.

The broad phase still relies on the existing outward-expanded floating-point AABBs, not formal directed rounding. Coarse-to-fine refinement does not raise the assurance level of the underlying bound arithmetic.
## Clock-aware conservative multi-body broad phase

The relative-time reference in #5849 checks every unordered pair. This extension converts each successfully mapped body into a conservative zero-clock-uncertainty outer span and feeds those spans into the existing dyadic sweep-and-prune broad phase.

For an original span of duration T and hard symmetric timing bound epsilon, the outer span:
- expands the half-open interval to [start-epsilon, end+epsilon);
- extrapolates the starting capsule to the widened start;
- scales the translation by (T+2 epsilon)/T so the nominal affine path is unchanged over the widened time axis;
- expands radius by |translation| epsilon/T;
- sets timing uncertainty to zero only after the complete outer envelope has been constructed.

This is a conservative numerical enclosure of the same bounded-offset model consumed by the pairwise oracle. Candidate pairs are passed to the pairwise clock-uncertainty CCD owner. A body whose mapping is invalid or whose outer envelope cannot be represented is excluded from pruning; every pair touching it is counted unresolved.

The report keeps detailed pair receipts only for emitted candidates. It reports the total possible pair count, valid outer-envelope body count, candidate count, pairwise checks, aggregate outer-space-time-pruned pair count and unresolved pair count. This avoids materializing O(n²) receipts for pruned pairs. The candidate/pairwise path remains evidence-carrying, while the omitted pairs are represented by aggregate counts and the deterministic broad-phase report.

The existing SAP uses an eight-ULP numeric expansion; this is not a formally directed-rounded proof. Therefore outer_space_time_pruned_pairs is a benchmark/pruning disposition, not a production motion-authorization or formal clearance certificate. Before use in any safety-relevant path, the outer-bound arithmetic must be independently validated with interval/directed-rounding methods and a separately governed reference corpus.

The paired benchmark compares the exhaustive multi-clock reference against the clock-aware broad phase on identical sparse scenes at 8/16/32/64/128/256/512 bodies and identical clustered, dense, long-thin, time-staggered and high-drift scenes at 32/64/128/256 bodies. For each workload it sweeps 1/2/4/8/16 temporal slabs, measures both paths with one warm-up and alternating execution order, and reports reference and broad-phase p50/p95/p99 separately plus p50 speedup. The CSV also reports total possible pairs, outer-envelope bodies, candidates, pairwise checks, pruned pairs, unresolved pairs and non-contact candidate count. At 8–64 bodies it checks that every exhaustive exact/potential/boundary contact pair appears in the candidate set at each slab count. A generated 64-body unit test performs the same candidate-superset check against the exhaustive oracle. These synthetic strata still do not represent field robot trajectories or independently calibrated site clocks. Vendor adapters, blind external geometry families and physical metrology remain separate qualification steps. Tracks #5823 and #5649.
