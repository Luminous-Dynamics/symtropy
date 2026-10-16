# Deterministic spatial index for cached static broadphase

Status: implementation and differential regression committed; **runtime correctness and performance are not yet verified on this exact branch**.

## Problem statement

The incremental broadphase caches Static and Kinematic body descriptors to avoid rebuilding a hierarchy each frame. A cache is only useful if its query operation is cheaper than the work it replaces. A linear scan over every cached descriptor still costs O(active × cached) AABB tests per frame. At 10,000 static bodies and 32 active bodies, this is 320,000 active-vs-cache tests before considering any Kinematic queries.

## Change

Replace the cached-entry linear scan with a deterministic median-split AABB hierarchy rebuilt when the existing static-tree dirty flag requires it.

- Each cache entry has one leaf with its orientation-aware exact AABB.
- Internal bounds are unions of child bounds.
- Split axis is the largest current node extent; split point is the median centroid.
- Centroid ties are broken by body handle, making construction deterministic for the same body set.
- Queries prune disjoint node bounds, then check leaf AABBs and collision masks.
- Kinematic-vs-cached pair generation uses the same spatial query and keeps handle-order duplicate suppression.
- The ordinary `find_pairs_incremental` result is still sorted and deduplicated before return.

The tree uses the existing `arrayvec` dependency for a small inline traversal stack. The builder is balanced by median split; practical depth is logarithmic. Cache rebuild cost and steady-state query cost must both be measured; this change does not presume that the tree wins every scene.

## Correctness evidence added

A fixed-seed test builds a 2,048-entry mixed Static/Kinematic cache and compares 256 queries against an independent linear scan. The query corpus includes queries centered on stored entries, randomly placed queries, guaranteed far-empty-space controls, varied radii, and asymmetric collision masks. The test also checks the full-binary-tree node-count and one-leaf-per-entry invariants.

The existing mixed-body differential test (from the prerequisite kinematic-pair repair) continues to compare full incremental pair sets against brute force across 40 deterministic scene/seed combinations. A threshold-boundary regression also compares against brute force at 48, 49, 50, and 51 combined Dynamic + Kinematic bodies, exercising both sides of the current small-N/LBVH policy switch.

These are committed tests, not reported passes: neither targeted Cargo tests nor the benchmark has been executed in this session.

## Evaluation

The static-heavy crossover harness is being added separately in [PR #1698](https://github.com/Luminous-Dynamics/symtropy/pull/1698). It should be run after the kinematic pair repair and this spatial index are integrated, so the exact implementation under measurement is clear:

```sh
cargo test -p symtropy-physics broadphase
cargo bench -p symtropy-physics --bench physics_bench -- broadphase_static_heavy
```

Retain exact commit SHA, Rust version, CPU/OS, benchmark command, raw Criterion output, static/dynamic/kinematic counts, cache rebuild frequency, and scene seed.

## Proposed gates

These are project targets, not industry standards:

1. **Correctness:** zero pair omissions relative to the independent brute-force AABB oracle; zero invalid/duplicate pairs.
2. **Spatial query:** zero disagreement between tree query and independent linear query over the fixed-seed large-cache corpus.
3. **Amortized benefit:** target at least 15% lower total cost on at least two predeclared static-heavy workloads after charging cache rebuild cost at the declared dirty rate.
4. **Regression budget:** no more than 5% regression on a predeclared critical workload without an explicitly accepted trade-off.
5. **Coverage:** held-out clustered, uniform, dense-overlap, kinematic-motion, insertion/removal, and sleep/wake traces before changing the default selection policy.

The paradigm shift is to optimize total workload cost under an exact semantic contract—not to assume that reusing state or using a hierarchy is inherently faster.
