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

`fixtures/ray-query-known-answers-v0-1.json` contains four deterministic triangles and eleven rays:

- coincident nearest blockers, including an exact nearest-hit tie set;
- a farther blocker behind the nearest surfaces;
- far clipping and near clipping that selects a farther hit;
- exact self-triangle exclusion by stable identity;
- shared-edge and shared-vertex hits;
- parallel and coplanar misses under an explicit `coplanar_policy = miss`;
- near/far clipping-boundary inclusion under the declared metric distance epsilon;
- an off-axis miss.

All coordinates are metres in a right-handed XYZ frame. Directions are unit length. The fixture freezes the metric intersection epsilon, nearest-hit tie epsilon, dimensionless parallel-cosine epsilon, barycentric epsilon and coplanar policy. The standalone f64 brute-force oracle verifies every expected nearest identity set before an engine adapter is compared.

A second fixture, `ray-query-known-answers-scale-1e3-v0-1.json`, applies a uniform 1,000× scale to coordinates, ray origins, near/far distances and metric tolerances while keeping unit directions and dimensionless parallel/barycentric tolerances unchanged. The same strict f64 oracle is run against both fixtures; nearest identities and hit/miss classifications must remain invariant while metric hit distances scale by 1,000×.

A third fixture, `ray-query-stress-overlap-v0-1.json`, adds 40 triangles and three queries:
- 24 exactly coincident nearest blockers, requiring all 24 identities in the nearest tie set;
- 16 long, thin triangles with strongly overlapping AABBs;
- a near-clipped long-thin query that skips the first surface and returns the next surface at 4.25 m.

The brute-force oracle runs all three fixtures (base, 1,000× scale and overlap stress) in the replay-contract workflow. This corpus now probes difficult synthetic spatial distributions, but it is not a substitute for licensed building meshes or physical scan data. It does not yet test arbitrary large/small coordinate ranges, degenerate-input rejection fixtures, randomized differential rays or real-building geometry.

P0 now checks the fixture with both a strict JSON Schema (2020-12) and an executable, independent brute-force Möller–Trumbore f64 oracle. The oracle's parallel and barycentric thresholds are fields in the fixture contract rather than hidden hard-coded tolerances. The parallel threshold is relative to the triangle-normal magnitude and ray-direction norm, making it dimensionless and scale-consistent. A coplanar ray is explicitly a miss. The replay-contract workflow runs the oracle on both the base and 1,000× scale fixtures and asserts seven hit / four miss outcomes and eleven checked nearest-identity sets in each.

## Engines to compare

### In-repository references

- **Brute-force triangle kernel:** retained as the small semantic oracle. Its simplicity is valuable for differential correctness tests.
- **Median-split AABB BVH:** current deterministic visibility evaluator in #4292. It uses a longest-centroid-axis median partition and max eight triangles per leaf.
- **Binned SAH AABB BVH:** now implemented as a test-only in-repository comparison prototype. It bins centroids on all three axes, evaluates surface-area-weighted split costs, breaks ties deterministically and falls back to the stable median split when a valid partition cannot be formed. The frozen 32-ray/266-triangle test requires identical nearest-hit identities from median and SAH against brute force; SAH leaves remain at or below eight triangles. A separate 17-triangle identical-centroid test forces the fallback path and checks that all equal-distance nearest hits are retained in stable identity order. It then reverses the triangle input order and requires identical tree node/leaf counts, nearest-hit identity order, primitive-test count and node-visit count.
- **Spatial-split / two-level BVHs:** optional external/next experiments after correctness, licensing and rebuild/refit semantics are established.

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

The median BVH is the production evaluator path. The SAH prototype is compiled only for tests; it is not selected by requests or shipped as a production query option. The two builders are compared on deterministic hit identities and structural/work counters, not wall-clock latency. SAH tests include sparse multi-ray differential work, a coincident-centroid median-fallback path and input-permutation invariance. The corpus now adds a 24-way tie and 16 long-thin overlapping-AABB triangles. A separately qualified runtime/benchmark API is still needed before p50/p95/p99 latency, memory or Embree comparisons can be made.

## Independent Intel Embree adapter (source added; build not yet qualified)

A standalone benchmark-only C++ adapter now exists at `embree/`:
- Exact upstream pin: Intel Embree 4.4.1, tag `v4.4.1`, commit `f590db83ef6559387df7f6d8725c34fb7acf851d`, Apache-2.0.
- Uses Embree 4's argument intersection filter, rejecting every candidate hit while collecting `ray->tfar` and primitive identity. This follows the upstream `tutorials/next_hit/next_hit_device.cpp` pattern and preserves nearest-hit tie sets rather than relying on one arbitrary first hit.
- Builds the exact fixture triangles as float32 vertex/index buffers; output explicitly reports `precision=f32`.
- Exposes low/medium/high build quality and device thread configuration; each receipt records that ray calls are serial single-ray invocations.
- Applies the fixture's metric intersection epsilon symmetrically to Embree `tnear`/`tfar` so its segment semantics match the f64 oracle's inclusive clipping contract; it does not widen the expected-distance or tie tolerances.
- Uses CMake with exact Embree 4.4.1 and nlohmann-json 3.11.3 packages plus OpenSSL 3 Crypto.
- Registers CTest smoke runs for base, 1,000× scale and overlap-stress fixtures.

The adapter source has been reviewed against the pinned Embree headers and upstream all-hits tutorial, but it is **not yet compiled or executed**. The authoring environment lacks Embree and nlohmann-json development headers/libraries. CTest results, runtime correctness, build/query latency and memory are not yet available. Do not treat the existence of `main.cpp` as an engine comparison result. Detailed build/run instructions and remaining qualification limits are in [`embree/README.md`](embree/README.md).

Embree's public API does not expose its internal primitive-test or BVH-node-visit counters through this adapter; those receipt fields remain null. Memory fields remain null until a separate pinned profiler is used. Float32 boundary mismatches remain visible and are not hidden by changing the fixture's f64 oracle thresholds.

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

## Common benchmark receipt

Use the closed v0.1 receipt schema at `schema/engine-benchmark-receipt-v0-1.schema.json`. It binds:
- engine source/version, adapter revision, licence, precision mode and build configuration;
- exact fixture, geometry and ray-set digests;
- OS/kernel/CPU/thread count/compiler/flags/build profile;
- build and refit time, peak and steady memory;
- p50/p95/p99 query latency, throughput, primitive tests and node visits where exposed;
- brute-force known-answer agreement, exact nearest-identity matches and categorized mismatches;
- repeated-run and input-permutation determinism;
- an explicit qualification state and claim scope.

The checked-in `receipts/benchmark-run-template-v0-1.json` is intentionally `template_only`: its timing, memory and correctness-measurement fields are null. It is not a benchmark result. Never replace missing engine telemetry with an estimate or a proxy field from another library.

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
