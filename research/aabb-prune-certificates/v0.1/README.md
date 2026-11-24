# AABB Prune Certificates v0.1

A dependency-free Rust reference for independently replayable, proof-carrying **AABB disjointness** decisions.

## Scope

The verifier proves that two supplied finite, ordered, represented AABBs are strictly disjoint on at least one coordinate. A certificate binds canonical collider identities (including body-handle generation and compound subshape path), world revision, simulation step, exact represented interval endpoints, integrator/provider/transform/arithmetic profile labels, exact AABB endpoint bit patterns, a separating coordinate and orientation, and an outward-rounded positive gap interval.

The verifier replays the coordinate inequality and gap interval against the current envelopes. It does not trust a producer's prune boolean. Changed identities, contexts or endpoint bits invalidate replay.

The typed decision is one of:

- `CertifiedDisjoint(certificate)`: the supplied represented AABBs are strictly separated with a positive conservative gap lower bound;
- `OverlapPossible`: overlap, touching, or a gap too small to retain a positive outward-rounded lower bound;
- `Unsupported`: no qualified bound is available for the requested shape or motion model;
- `Invalid`: malformed input or a proof-context mismatch.

Every non-certificate outcome must continue to another refinement stage or exhaustive fallback. A learned score cannot emit a prune.

## Claim boundary

A valid certificate proves **only disjointness of the supplied represented AABBs**. It does not prove that either AABB contains a shape, swept shape, or trajectory over the represented time interval. Bound conservatism remains a separate obligation on the provider. Provider and profile identifiers are caller-supplied labels, not authenticated identities.

This crate does not claim continuous collision detection, swept-bound correctness, zero tunnelling, independent provider validity, zero missed physical collisions, production integration, performance improvement, cryptographic authenticity, or SOTA superiority.

The outward-rounded gap interval encloses the coordinate difference from finite represented endpoints. If subtraction overflows, the lower bound is conservatively set to `f64::MAX` and the upper endpoint to infinity. If the lower endpoint is not strictly positive, no prune certificate is emitted.

## Adversarial tests

The frozen unit tests cover canonical pair order, x/y/z witnesses, exact snapshot replay, changed endpoints, signed-zero bit changes, context mismatches, self-pairs, overlap, touching, unsupported motion, invalid inputs, subnormal gaps and overflow.

## Run

```sh
cargo fmt --manifest-path research/aabb-prune-certificates/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/aabb-prune-certificates/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/aabb-prune-certificates/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Qualification ladder

This is the independent checker slice for [#7281](https://github.com/Luminous-Dynamics/symtropy). The next steps are to connect only a frozen translation-only temporal-envelope producer; create an independent swept-pair oracle with interior extrema, endpoint-disjoint/interior-intersecting trajectories, rotation and high-aspect-ratio controls; bind proof dispositions to the pair lifecycle ledger (#7255); measure verifier cost, proof coverage, unsupported/fallback rates, pair recall and end-to-end p50/p95/p99; and add further conservative levels only when their producers independently pass enclosure tests.

Promotion targets in #7281 are targets, not results: 100% independent pair recall; 100% certificate coverage for pruned pairs; 100% safe fallback for injected invalid/overflow/unsupported inputs; zero accepted certificates with non-positive or indeterminate gap; verifier cost below 5% of total culling work on the frozen report workload; and at least 20% less narrowphase work on two held-out workload families without more than 5% p95 end-to-end regression.

## Industry comparison

[Box2D's dynamic tree](https://box2d.org/documentation/group__tree.html) exposes overlap-query and traversal statistics, allowing candidate selectivity and query work to be measured rather than inferred from tree shape. [MuJoCo's speculative-contact path](https://mujoco.readthedocs.io/en/stable/computation/speculative.html) combines swept broadphase bounds, a bounding-sphere filter and conservative advancement; its documentation also explicitly notes that the speculative sweep uses pre-step velocities and ignores acceleration. These are implementation/performance comparators, not independent proof comparators.

Symtropy's potential differentiator is not merely another tree or a lower overlap score. It is an auditable culling contract: every removed eligible pair has a replayable geometric witness, every unsupported case falls back, and proof cost is measured alongside simulation cost. This crate implements only the supplied-AABB checker component of that architecture.
