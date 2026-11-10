# SCAN-BIM-RAYQUERY-001 — independent ray-query comparison

**Status:** P0 benchmark contract and first known-answer fixture started. No comparative performance result is claimed.  
**Issue:** [#4456](https://github.com/Luminous-Dynamics/symtropy/issues/4456)  
**Parent semantics:** [#4292 evaluator visibility oracle](https://github.com/Luminous-Dynamics/symtropy/issues/4292)  
**Stack:** `research/scan-bim-visibility-v0-1`

## Objective

Compare the deterministic median-split BVH now used by the visibility evaluator with mature independent ray-query implementations under exactly the same canonical triangles, rays, clipping distances, tolerance rules and expected hit/tie sets.

The benchmark must distinguish three independent questions:

1. **Semantic correctness:** did the engine return the correct nearest hit set and distance under the frozen query contract?
2. **Resource cost:** how many triangle tests, node visits, build operations and bytes were required?
3. **Operational latency:** what were build/refit times and p50/p95/p99 query latency on pinned hardware/toolchains?

A faster result that misses a nearest hit is not a performance improvement.

## First pinned fixture

`fixtures/ray-query-known-answers-v0-1.json` contains a small, deterministic set of triangles and four rays:

- two coincident nearest blockers to exercise exact tie-set reporting;
- a farther triangle behind those blockers;
- a ray whose far limit stops before the geometry;
- a ray whose near limit skips the first surface and reaches the farther surface;
- an off-axis ray that misses the complete scene.

All coordinates are metres in a right-handed XYZ frame. Directions are unit length. The fixture freezes the hit distance tolerance and expected nearest identity sets.

The fixture is only an initial lexical/numeric known-answer set. It does not yet test edge/vertex hits, coplanar/grazing rays, very large/small coordinates, degenerates, or a dense real-building mesh.

## Engines to compare

### In-repository references

- **Brute-force triangle kernel:** retained as the small semantic oracle. Its simplicity is valuable for differential correctness tests.
- **Median-split AABB BVH:** current deterministic production candidate in #4292. It uses a longest-centroid-axis median partition and max eight triangles per leaf.
- **Binned SAH BVH:** proposed alternative. Compare build cost and traversal cost; do not assume SAH wins every query distribution.
- **Spatial-split / two-level BVHs:** optional experiments only after correctness, licensing and rebuild/refit semantics are established.

### Independent implementations

- **Intel Embree 4.4.1** is the first high-performance CPU comparator. It is Apache-2.0. Embree exposes scene build-quality settings: low build quality favors faster/dynamic updates; high build quality can enable spatial splits for some geometry types, trading slower commits for better traversal. Pin the exact tag/build/compiler and settings. Sources: https://www.embree.org/ and https://github.com/RenderKit/embree/blob/master/doc/src/api/rtcSetSceneBuildQuality.md
- **Open3D RaycastingScene** is a secondary interoperability comparator. It exposes first-hit, all-intersections and occlusion queries. Its documented ray tensor uses float32, so any difference from the evaluator's f64 reference must be measured and classified; do not silently loosen tolerances. Sources: https://open3d.org/html/python_api/open3d.t.geometry.RaycastingScene.html and https://www.open3d.org/docs/latest/tutorial/geometry/ray_casting.html

Do not add C++/Open3D or Python dependencies to the production evaluator merely to make a benchmark. Keep the adapter process isolated, pin licences and versions, and serialize the same canonical input fixture to every engine.

## Frozen query semantics

The common adapter must declare:
- ray origin and unit direction in metres;
- `tnear` and `tfar` clipping;
- exact target-triangle self-hit exclusion by stable (surface_id, triangle_id);
- primitive intersection tolerance and nearest-hit distance tie epsilon;
- whether all equal-nearest hits or one stable representative are returned;
- triangle winding policy and two-sided/one-sided intersection policy;
- invalid/degenerate/non-finite triangle handling;
- deterministic normalization/order of hit identities.

Visibility policy (observable, unknown, self-occluded, etc.) belongs to #4292 and must not be reimplemented differently in every ray engine.

## Workload matrix

| Geometry distribution | Primary stress |
|---|---|
| Spatially separated triangles | Broad-phase pruning |
| Dense overlapping surfaces | Worst-case leaf tests and tie handling |
| Clustered rooms and corridors | Highly anisotropic centroids |
| Long thin triangles | AABB overlap and poor split quality |
| Repeated component instances | Two-level BVH/instancing |
| High-tessellation wall/MEP meshes | Memory and build cost |
| Static reference mesh | High-quality build / repeated queries |
| Incremental revision | Refit vs rebuild cost |

Scale: 10^3, 10^4, 10^5 and 10^6 triangles where memory allows. Report each workload separately.

## Measurements

Every run must bind exact engine/build identifiers and input digests and report:

- known-answer agreement and discrepancy categories;
- BVH build and refit time;
- p50/p95/p99 single-ray and batched-query latency;
- primitive intersection tests and node visits where exposed;
- memory / bytes per triangle / peak build memory;
- single-thread and parallel scaling;
- cold-build and warm-query phases;
- deterministic receipt agreement across repeated runs and input permutations.

Do not infer Embree primitive test counts if its public API does not expose them. Mark a metric unavailable instead of substituting a guessed proxy.

## Proposed gates (research targets, not achieved results)

- 100% agreement with the brute-force oracle on the frozen exact-semantics corpus.
- Zero missed nearest hits and no false unoccluded outcomes on adversarial negatives.
- 100% input-order invariance for semantically identical stable identities.
- p50/p95/p99 and memory published for each engine and workload.
- Show the build-time/query-time Pareto frontier, not one headline speedup.
- Preserve a small deterministic brute-force audit mode even if an external engine is selected for throughput.
- No engine may be selected for production solely because it is fastest on the sparse synthetic fixture.

## Paradigm opportunity: workload-conditioned query planning

The final answer may be a portfolio, not one accelerator:

- immutable evaluator reference mesh → high-quality BVH built once, queried many times;
- frequently revised mesh → cheaper rebuild/refit or two-level instance tree;
- coherent dense image rays → batched/coherent traversal;
- sparse task-selected NBV rays → incoherent traversal optimized for query count and tail latency;
- deterministic brute-force / pinned f64 mode → audit and known-answer reference;
- faster external accelerator → only after it returns semantically equivalent results and its receipt identifies exact engine/build settings.

Any policy switch must be explicit in the receipt and must not change nearest-hit or tie semantics.

## Claim ceiling

This work measures declared ray-query implementations under pinned synthetic/licensed workloads. It does not establish physical survey accuracy, completeness of real occluder inventories, calibrated sensor detection probability, Scan-to-BIM quality, model editability, building-code compliance, structural adequacy or universal vendor superiority.
