# Adaptive geometry and mesh benchmark protocol

Status: benchmark harness and conservativeness regression tests added. No speedup is claimed until measurements are recorded against a baseline commit on the same hardware.

## Initial executable workload

Run:

```sh
cargo test -p symtropy-mesh --test bvh_query_correctness
cargo bench -p symtropy-mesh --bench bvh_queries
SYMTROPY_BVH_BENCH_LARGE=1 cargo bench -p symtropy-mesh --bench bvh_queries
```

The benchmark reports BVH build time and mean query time / candidate count for deterministic planar grid meshes. The million-triangle case is opt-in because the current recursive median-split builder may be expensive at that scale. Store stdout, compiler version, CPU, operating system, commit SHA, and whether the large case ran.

This is a reproducible baseline harness, not a claim that the current algorithm is optimal. Compare old and new code at the same CPU/toolchain and preserve raw outputs.

## Geometry correctness gates

- BVH broadphase may produce false positives but must not omit any triangle whose exact vertex AABB overlaps a query AABB.
- Results contain no duplicate or out-of-range triangle indices.
- Transformed mesh queries are conservative under translation and rotation.
- Degenerate triangles, empty meshes, large coordinate magnitudes, and near-coplanar contacts are covered before changing predicate or narrowphase logic.
- Collision results stay finite; independently check against brute-force candidate scans or a separate reference implementation where feasible.

## Candidate optimization experiments

1. **BVH construction:** current longest-axis median split versus binned surface-area heuristic (SAH), Morton/LBVH, and hybrid top-down / bottom-up construction.
2. **Dynamic geometry:** measure refit cost and query quality over repeated deformations; rebuild only when traversal metrics justify it.
3. **Query-aware optimization:** test hot-region weighting only against held-out query traces; compare against query-oblivious SAH.
4. **Meshlets:** compare actual multi-cluster partitionings by triangle count, query candidates, cache locality, and cluster rebuild cost. A single-cluster prototype is not treated as a scalable implementation.
5. **Narrowphase:** record candidate count and exact GJK/EPA calls, then optimize only after reference correctness passes.

## Workload matrix

- 10k / 100k triangles by default; 1M triangles as an explicit large run.
- Uniform planar grid, clustered geometry, long thin geometry, uneven triangle density, and high-overlap scenes.
- Static geometry and moving/deforming geometry.
- Small / medium / broad query boxes and queries concentrated in hot regions.
- Collision workloads with many shapes, including edge and vertex contacts.

## Required metrics

Build/refit time, query p50/p95/p99, candidate triangles per query, narrowphase invocation count, memory footprint, false-negative count, false-positive candidate ratio, and end-to-end simulation-step time. Report scenario-specific regressions rather than only a geometric mean.

## Acceptance

An optimization is accepted only when it has a reproducible improvement on a declared target workload, no correctness-gate regressions, a comparison against the prior commit and at least one credible alternative, and an explicit statement of workload limitations. Performance numbers without hardware/toolchain/commit provenance are not an acceptance result.
