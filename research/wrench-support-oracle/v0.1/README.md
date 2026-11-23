# Wrench Support Oracle v0.1

A dependency-free Rust research oracle for comparing a reduced contact-point set against the full candidate patch's feasible resultant-wrench support.

This is intentionally a small, auditable model. It does not implement a general 3D rigid-body contact solver and does not establish that Symtropy, Bullet, Jolt, Rapier or any other engine is physically superior.

## Model and exactness boundary

The model has:
- 3D contact points relative to an explicitly declared wrench origin, each with a finite non-negative narrow-phase penetration depth;
- one shared contact normal, fixed to +Z, with all points required to be coplanar in a plane parallel to XY;
- unit total normal load, distributed arbitrarily across nonnegative contact loads;
- a Coulomb friction disk at each point, with common coefficient mu;
- no torsional spin moment at an individual point;
- force components normalized by total normal load and moment components by that load times a characteristic length.

For a wrench direction d = (d_F, d_M), contact position r and characteristic length L, define:

    q_i = d_F + (d_M cross r_i) / L

Under the shared +Z normal and friction-disk assumptions, that point's unit-load directional support is:

    s_i(d) = q_i,z + mu * sqrt(q_i,x^2 + q_i,y^2)

Because total normal load is fixed and may be allocated over the available contacts, patch support is:

    h_W(d) = max_i s_i(d)

This is exact for the model above. It is not exact for varying contact normals, bounded positive per-point load, torsional friction, anisotropic friction, compliance, cohesive contact, rolling resistance, arbitrary contact clusters, or the full complementarity/contact dynamics problem.

Directions are normalized after the force/moment scale convention is declared. For a frozen direction set D, reported normalized deficit is:

    deficit(d) = max(0, h_full(d) - h_reduced(d))
                 / max(abs(h_full(d)), support_floor)

Headline metrics are maximum and mean deficit across the declared directions. They are finite-direction measurements, not an all-directions bound.

## Exact subset selection

For small candidate sets, the oracle enumerates every k-point subset and minimizes:
1. maximum normalized directional support deficit;
2. mean normalized directional support deficit;
3. lexicographic retained stable IDs as deterministic tie-break.

A configurable maximum-subset limit fails closed before enumeration. This gives an exact optimizer for the finite direction set and restricted physical model.

A deterministic greedy baseline is also included. It adds one point at a time, minimizing the same current max-deficit/mean-deficit/lexicographic tuple. It is intentionally not claimed to be globally optimal or to have a general approximation ratio; the exact selector measures its empirical optimality gap on small patches. A five-point synthetic counterexample demonstrates a real local-choice gap: the exact three-point subset preserves all nine frozen axis directions (zero sampled deficit), while the greedy three-point set loses more than 40% in its worst normalized direction.

The evaluate_retained_set function provides the common evaluation path for externally supplied retained IDs, so future Bullet/Jolt/Rapier/Symtropy adapters can be compared on the same candidate set and directions.

## General polyhedral contact-wrench-cone model

The general module supports:
- arbitrary 3D contact positions and varying contact normals;
- per-contact material tangent hints, projected into each contact plane to orient anisotropic friction;
- full-versus-reduced finite-direction support evaluation and exact/greedy contact-subset selection;
- per-contact anisotropic friction semi-axes;
- an inscribed, equally spaced polyhedral friction cone with a declared ray count;
- lower and upper per-contact normal-load fractions summing to a feasible fixed unit total load.

For one contact, each friction ray is a force generator with unit normal component. Mapping it to the object frame produces a primitive wrench:

    w_ij = [ f_ij, (r_i cross f_ij) / L ]

For a normalized query direction d, each contact's best ray gives its support per unit normal load. The remaining allocation is a bounded linear program:

    maximize sum_i lambda_i * s_i(d)
    subject to sum_i lambda_i = 1
               min_i <= lambda_i <= max_i

The exact optimum is obtained by assigning all lower bounds first, then allocating the remaining load to contacts in descending support order until their upper bounds are reached. This is exact for the finite polyhedral friction generators and stated load bounds.

The cone is an inscribed polygonal approximation of the Coulomb friction ellipse/disk. More rays reduce angular conservatism but increase support-evaluation work. The module reports the chosen ray count; comparisons must hold it fixed or explicitly account for cone-approximation error. No spin friction or rolling-resistance torque is currently modeled.

## Research baseline and next extensions

Compare on frozen identical candidate sets:
- full-candidate support ceiling;
- current Symtropy deepest/farthest/area/centroid best-N heuristic, reproduced as a deterministic baseline using the same candidate depths and positions;
- Bullet-style deepest-point plus maximum-area policy;
- Jolt's bounded stable-manifold policy;
- Rapier-style contact clustering with solver groups kept distinct from geometric manifolds;
- exact small-set wrench-support subset oracle;
- deterministic greedy selector, directly scored against the exact result;
- future branch-and-bound and direction-adaptive selectors.

Add NVIDIA Newton's documented contact reduction as another baseline: it groups contacts by normal direction, scores spatial distribution and penetration depth, and uses aspect-ratio-based normalized voxel grids for mesh/SDF coverage. The key commonality across engines is that reduction is not a point-count-only problem; geometry coverage, penetration/depth, normal clusters, persistence and solver correspondence matter.

The crate now includes a source-aligned reproduction of Symtropy's current best-N stages (deepest, farthest from deepest, farthest from the first-two line, then farthest from the three-point centroid). On the same five-point counterexample used for the greedy test, the reproduced best-N policy selects p1/p2/p4 and has more than 40% worst sampled normalized support deficit, while the exact three-point oracle selects p2/p3/p4 with zero deficit across the nine frozen axes. This is a synthetic model result, not a measured production-engine outcome; the tie-break policy is made stable by point IDs.

The industry descriptions are not matched performance results. Rapier documents that contact clustering can merge same-normal manifolds and solve at most four contacts per plane; when clustering applies, geometric manifold impulses are not the solver impulses. Jolt documents a four-point stable manifold and exposes reduction visualization. Bullet describes retaining the deepest point and maximizing area when reducing its persistent cache to four. Newton documents normal bins, spatial distribution and penetration depth. These are architectural references; compare actual outputs with identical fixtures before claiming a win.

Contact-wrench cones and grasp-wrench spaces are established robotics concepts: the feasible object-wrench set can be built from contact friction-cone generators mapped through contact positions. The opportunity here is not to claim that concept as new; it is to apply a versioned, error-measured wrench-support objective to physics-engine manifold reduction, then connect its geometric decisions to actual solver rows, numerical outcomes and downstream physical tasks. A 2026 tutorial emphasizes the distinction between equivalent contact formulations and convex relaxations, which can introduce artifacts such as lift-off and creep. Each model/solver pairing must therefore be qualified independently.

Sources:
- Rapier integration parameters: https://www.rapier.rs/docs/user_guides/c/integration_parameters/
- Rapier contact graph: https://www.rapier.rs/docs/user_guides/c/advanced_collision_detection/
- Jolt ContactConstraintManager: https://jrouwe.github.io/JoltPhysics/class_contact_constraint_manager.html
- Bullet PersistentManifold: https://www.apt-browse.com/browse/ubuntu/trusty/universe/amd64/libbullet-dev/2.81-rev2613%2Bdfsg2-1/file/usr/include/bullet/BulletCollision/NarrowPhaseCollision/btPersistentManifold.h
- NVIDIA Newton contact reduction: https://newton-physics.github.io/newton/latest/concepts/collisions.html
- Contact-patch wrench spaces: https://drdv.github.io/pdf/icra12_1.pdf
- Patch-contact friction wrench modelling: https://doi.org/10.1177/02783649231198560
- Complementarity-based contact models tutorial (2026): https://arxiv.org/abs/2611.26600

## Required next experiments and promotion targets

1. Sweep candidate count 4/8/16/32/48/64 and retained count 1/2/3/4/6/8. Exact k-subset search is the oracle for workloads under a declared combination cap; use it to measure the greedy/approximate optimality gap.
2. Sweep friction 0/0.1/0.3/0.5/1/2, anisotropy ratios 1:1 through 10:1, contact-normal separation, patch aspect ratio, reference-origin shift and characteristic-length scaling.
3. Sweep polyhedral friction-cone ray counts 4/8/16/32/64. The exact angular worst-case relative deficit for an inscribed regular m-gon is 1 - cos(pi/m): about 1.92% at 16 rays, 0.48% at 32 and 0.12% at 64. Keep cone approximation error separate from manifold-selection error.
4. Add adversarial near-collinear, nearly coplanar, sliver, edge, vertex and highly anisotropic patches; include stable body-swap and point-order permutations.
5. Compare full candidate, current Symtropy best-N/area, Bullet deepest+area, Jolt bounded manifold, Rapier normal clustering, Newton normal bins + voxel/depth coverage, greedy wrench support, and exact oracle on identical raw candidate geometry and contact-law data.
6. Add direction-set refinement and certified angular bounds, not merely denser finite sampling.
7. Compare reduced-set solver outputs on static tipping, box stacks, frictional transport, torsional stability and contact-rich robot manipulation.
8. Validate downstream decisions on held-out physical tasks; retain full-patch results as the reference and record decision flips, regret, uncertainty and failure modes.

Targets, not achieved results:
- 100% accounting from generated candidate through temporary buffer, reduction, final manifold and solver groups.
- Candidate-buffer loss, deliberate reduction and final-manifold loss are always distinguished.
- Maximum normalized deficit <= 1% on the frozen direction set for the declared admissible profile; all-directions claims require a separate certificate.
- For the exact-oracle tractable suite, target median greedy gap <= 0.2%, p95 <= 1%, and worst <= 5%; any miss triggers beam search/branch-and-bound or explicit abstention.
- At equal physical quality and coverage, target >= 2x throughput or >= 30% memory reduction versus the best matched baseline. Measure end-to-end time, not selector-only time.
- Held-out physical task decision agreement >= 95%, lower 95% confidence bound >= 90%, and no new critical instability/hazard class.
- Publish p50/p95/p99 latency, memory, candidate counts, solver rows/iterations, cone approximation error, support deficit, complementarity/friction residuals, tipping/stack outcomes and run failures.

A candidate selector is not promoted merely because it reduces point count or runs faster. Promotion requires a Pareto improvement across support fidelity, physical stability, evidence completeness, memory and end-to-end latency.

## Tests

The unit suite contains 30 tests across both models and covers:
- exact zero deficit for the full set;
- a known two-point subset preserving both positive and negative moment extrema;
- greedy agreement with exact selection on a symmetric known-answer patch;
- exact-vs-greedy objective ordering and greedy evaluation guard;
- a five-point local-choice counterexample exposing a greedy optimality gap;
- the current Symtropy deepest/farthest/area best-N baseline scored against the exact wrench optimum;
- rejection of non-coplanar points in the shared-normal model;
- varying contact normals and bounded load allocation;
- per-contact anisotropic friction and material tangent-hint orientation;
- rejection of tangent hints parallel to their contact normal;
- full-patch vs reduced-patch support comparison under the general model;
- exact general-model subset selection and greedy work/deficit accounting;
- mandatory minimum-load contact retention;
- minimum normal-load constraints;
- friction-polyhedron ray-count convergence;
- infeasible aggregate load bounds;
- contact input-order invariance;
- loss from a one-point subset;
- input/direction permutation invariance;
- geometric scale invariance;
- friction support for a shared normal;
- direction magnitude invariance;
- combinatorial work guard;
- invalid/duplicate point identities;
- unknown selected IDs;
- invalid model parameters and directions.

Run:

```sh
cargo fmt --manifest-path research/wrench-support-oracle/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/wrench-support-oracle/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/wrench-support-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Claim ceiling

This crate computes an exact support function and exact k-subset selection only for its explicitly restricted mathematical model and finite declared direction set. It does not prove all-directions equivalence, the correctness of any engine's contact law, general solver accuracy, physical-task transfer or throughput superiority.
