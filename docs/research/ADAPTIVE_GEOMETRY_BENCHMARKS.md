# Adaptive geometry and mesh benchmark protocol

Status: benchmark harness and conservative-query regression tests added. No speedup is claimed until results are measured against a baseline commit on the same hardware.

## Initial executable workload

Run:

```sh
cargo test -p symtropy-mesh --test bvh_query_correctness
cargo bench -p symtropy-mesh --bench bvh_queries
SYMTROPY_BVH_BENCH_LARGE=1 cargo bench -p symtropy-mesh --bench bvh_queries
```

The dependency-free harness currently measures a deterministic planar grid mesh at 10k and 100k triangles (1M is opt-in). It reports one build-time observation, then runs 7 repeats over the same 2,000-query corpus in different deterministic orders. Query latency percentiles are computed over means from batches of 25 queries, so they describe **batch-mean latency**, not individual-query p95/p99. Aggregate candidate counts and checksums must remain stable across repetitions; a mismatch aborts the run.

The build-time field is explicitly a single sample. Repeat the whole command in fresh processes and retain every raw output before concluding that build cost changed. This initial corpus is a reproducible smoke baseline, not representative of arbitrary geometry or a claim that the current algorithm is optimal.

## Geometry correctness gates

- BVH broadphase may produce false positives but must not omit any triangle whose exact vertex AABB overlaps a query AABB.
- Results contain no duplicate or out-of-range triangle indices.
- A fixed-seed nonuniform 512-triangle corpus (including zero-area and elongated triangles) is checked against an independent O(N) triangle-AABB overlap oracle across 256 queries with point-like through broad boxes.
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

The first harness only implements a planar grid. The remaining distributions are required before any claim about broad applicability; do not infer their performance from this grid result.

## Required metrics

Build/refit time, query p50/p95/p99, candidate triangles per query, narrowphase invocation count, memory footprint, false-negative count, false-positive candidate ratio, and end-to-end simulation-step time. The current initial harness covers only single-sample build time, query batch-mean percentiles, candidates, and a checksum. Mark metrics as unavailable rather than imply they were measured.

Report scenario-specific regressions rather than only a geometric mean. Keep correctness tests independent of the optimized implementation and preserve negative controls.

## Measurement protocol

- Pin the exact source commit, Rust version, target profile, CPU, operating system, and relevant environment variables.
- Preserve raw stdout for every run; do not report only a best run.
- Compare baseline and candidate on the same host and toolchain, alternate or randomize A/B order where feasible, and use repeated fresh processes for build-time comparisons.
- Separate tuning seeds/traces from held-out seeds/traces for adaptive policies.
- Report distributions and per-scenario regressions; a single mean can conceal tail latency or a correctness failure.
- Treat benchmark outputs as implementation measurements, not as proof of physical validity or workload-wide superiority.

## Provisional numeric project targets (not industry standards)

Freeze these targets before testing candidates; revise only with written rationale after the first baseline is captured.

- **Correctness is absolute:** zero broadphase false negatives against a brute-force oracle for the tested AABB semantics; zero duplicate or out-of-range candidates. A speedup cannot compensate for one missed candidate.
- **Coverage before optimization claims:** at least 3 geometry families (uniform grid, clustered, and adversarial long-thin or high-overlap), with at least 30 held-out deterministic seeds/query traces per family. The current harness covers only grid geometry.
- **First performance bar:** target at least 15% lower p95 *batch-mean* query latency or 25% fewer candidate triangles on at least 2 held-out families versus the same-commit median-split baseline. Treat these as screening targets, not guarantees.
- **Regression budget:** no more than 5% p95 batch-mean latency regression on any predeclared critical family without an explicitly accepted trade-off; report build/refit cost and memory separately.
- **Adaptive selection:** require at least 10% improvement over the best fixed policy on a predeclared held-out cost metric, with a 95% paired confidence interval excluding zero across independent run pairs. If not, keep the simpler fixed policy.

## Acceptance

An optimization is accepted only when it has a reproducible improvement on a declared target workload, no correctness-gate regressions, a comparison against the prior commit and at least one credible alternative, and an explicit statement of workload limitations. Performance numbers without hardware/toolchain/commit provenance are not an acceptance result.
