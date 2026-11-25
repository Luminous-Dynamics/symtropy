# Static Broadphase AABB BVH Prototype v0.1

This is an opt-in comparison backend for Symtropy's existing cached static/kinematic broadphase population.

## Existing production behavior

- `StaticBroadphase` stores cached Static and Kinematic AABB entries in a flat vector.
- Under the dynamic-body threshold, every Dynamic query linearly scans the cached population.
- At or above the threshold, the current implementation falls back to a full LBVH rebuild.
- The normal `PhysicsWorld` path still calls `StaticBroadphase::rebuild` and `find_pairs_incremental`; neither selects the experimental BVH.

## Prototype

- `rebuild_with_bvh` builds a deterministic median-split AABB BVH over the cached entries.
- `StaticQueryBackend::LinearScan` and `CachedBvh` are explicitly selectable for matched runs.
- `find_pairs_incremental_with_backend_and_stats` returns the exact pair list plus query, node, leaf, entry-test and AABB-hit counters.
- Both backends apply the same final AABB and mutual collision-mask policy and return canonical sorted/deduplicated pairs.
- A normal `rebuild` invalidates the experimental index; selecting CachedBvh without an explicit BVH build returns an error.
- Kinematic-to-static and kinematic-to-kinematic pairs are included consistently with the canonical `find_pairs` semantics; static-static remains excluded.

## Correctness gates

The tests compare:
1. cached BVH query results versus linear scan for the same AABB;
2. full incremental pair sets versus canonical `find_pairs`;
3. static/kinematic combinations and kinematic pair uniqueness;
4. index-not-built and rebuild-invalidation behavior.

The BVH is not a producer of proof-carrying prune certificates; it is a complete candidate generator whose result set must equal the reference path.

## Performance protocol

Measure separately:
- index build/rebuild time and amortization;
- query p50/p95/p99, nodes visited, leaves visited, entries tested and overlap hits;
- emitted pairs, candidate precision and exact pair recall;
- full broadphase and full physics-step p50/p95/p99;
- static/dynamic/kinematic population, moved fraction, churn, anisotropy and overlap density.

Do not change the production default from LinearScan until the BVH reaches 100% pair recall and shows a repeatable p95 full-step improvement on held-out static-heavy scenes without unacceptable regressions on small or dynamic-heavy worlds. The fixed 50-body threshold remains a hypothesis to measure, not a proven optimum.

## Run

```sh
cargo fmt --manifest-path crates/core/symtropy-physics/Cargo.toml -- --check
cargo test -p symtropy-physics --locked broadphase::tests
cargo clippy -p symtropy-physics --locked --lib -- -D warnings
```

This branch is stacked on PR #7472 so it includes the incremental kinematic-pair correctness repair. Retarget to main after that dependency merges.
