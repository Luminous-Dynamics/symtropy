# Wrench Support Oracle v0.1

A dependency-free Rust research oracle for comparing a reduced contact-point set against the full candidate patch's feasible resultant-wrench support.

This is intentionally a small, auditable model. It does not implement a general 3D rigid-body contact solver and does not establish that Symtropy, Bullet, Jolt, Rapier or any other engine is physically superior.

## Model and exactness boundary

The model has:
- 3D contact points relative to an explicitly declared wrench origin;
- one shared contact normal, fixed to +Z;
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

A configurable maximum-subset limit fails closed before enumeration. This gives an exact optimizer for the finite direction set and restricted physical model, suitable as a known-answer comparator for greedy/local-search reducers. It is not a scalable production reducer.

The evaluate_retained_set function provides the common evaluation path for externally supplied retained IDs, so future Bullet/Jolt/Rapier/Symtropy adapters can be compared on the same candidate set and directions.

## Research baseline and next extensions

Compare on frozen identical candidate sets:
- full-candidate support ceiling;
- current Symtropy best-N/area heuristic;
- Bullet-style deepest-point plus maximum-area policy;
- Jolt's bounded stable-manifold policy;
- Rapier-style contact clustering with solver groups kept distinct from geometric manifolds;
- exact small-set wrench-support subset oracle;
- future deterministic greedy and branch-and-bound selectors.

The industry descriptions are not matched performance results. Rapier documents that contact clustering can merge same-normal manifolds and solve at most four contacts per plane; when clustering applies, geometric manifold impulses are not the solver impulses. Jolt documents a four-point stable manifold and exposes reduction visualization. Bullet describes retaining the deepest point and maximizing area when reducing its persistent cache to four. These are architectural references; compare actual outputs with identical fixtures before claiming a win.

Sources:
- Rapier integration parameters: https://www.rapier.rs/docs/user_guides/c/integration_parameters/
- Rapier contact graph: https://www.rapier.rs/docs/user_guides/c/advanced_collision_detection/
- Jolt ContactConstraintManager: https://jrouwe.github.io/JoltPhysics/class_contact_constraint_manager.html
- Bullet PersistentManifold: https://www.apt-browse.com/browse/ubuntu/trusty/universe/amd64/libbullet-dev/2.81-rev2613%2Bdfsg2-1/file/usr/include/bullet/BulletCollision/NarrowPhaseCollision/btPersistentManifold.h

## Required next experiments

1. Add candidate-buffer and final-manifold capacity receipts independently.
2. Sweep friction coefficient, characteristic-length ratio, patch aspect ratio and point count.
3. Add adversarial near-collinear, near-coplanar and sliver patches.
4. Add direction-set refinement and certified angular bounds, not merely denser sampling.
5. Extend the oracle to varying normals and per-contact friction cones.
6. Add torsional/spin friction, rolling resistance and bounded normal-load constraints as separate model versions.
7. Compare reduced-set solver outputs on static tipping, box stacks, frictional transport and torsional stability.
8. Validate downstream robot decisions on held-out physical tasks before promoting any selector.

## Tests

The unit suite covers:
- exact zero deficit for the full set;
- a known two-point subset preserving both positive and negative moment extrema;
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
