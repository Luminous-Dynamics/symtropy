# Incremental AABB Tree Reference v0.1

A dependency-free, deterministic dynamic AABB tree baseline for broadphase tournaments.

## Contract

- Stable caller-supplied `ProxyId(u64)`; duplicate IDs are rejected.
- Finite, ordered AABBs only. Invalid bounds and invalid fat margins are explicit errors.
- Inclusive overlap treats touching AABBs as candidates, preserving conservative contact semantics.
- Configurable fattening margin. A proxy update skips reinsertion only when the new tight AABB remains inside the old fat AABB; otherwise the leaf is detached and reinserted.
- Insert, remove, update, query, query-pairs and deterministic rebuild operations.
- Query output is sorted by proxy ID; pair output is canonical and sorted.
- Structural counters expose internal node visits, leaf tests, AABB tests and emitted candidate pairs.
- Tree statistics expose proxy count, node count, height, free-node count, insert/remove/update counts, fat-contained updates, proxy reinsertions and full rebuilds. These are operation counts, not timing results.

## Deliberate scope boundary

This is a benchmark reference, not a production DBVT clone. It uses deterministic perimeter-cost sibling selection but does not implement rotations, concurrent mutation, SIMD traversal, collision layers or a sophisticated rebuild policy. Height and traversal counters are exposed so those limitations are measured rather than hidden. Compare its output against brute force before using its timing data.

## Run

```sh
cargo fmt --manifest-path research/incremental-aabb-tree-reference/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/incremental-aabb-tree-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/incremental-aabb-tree-reference/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Tournament matrix

- 2D, 3D and 4D.
- 50 through 50,000 proxies.
- 0–100% moving fractions and 0–10% insert/remove churn per step.
- Uniform, clustered, anisotropic, high-aspect-ratio, high-overlap and burst-teleport workloads.
- Build, update/reinsert, remove, query, rebuild and amortized pair-query cost.
- 100% brute-force pair recall as a hard gate; candidate inflation and p50/p95/p99 latency as separate metrics.
