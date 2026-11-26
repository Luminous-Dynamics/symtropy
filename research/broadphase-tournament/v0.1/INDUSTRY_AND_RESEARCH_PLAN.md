# Broadphase industry comparison and research plan

Date: 2026-11-26

This document separates documented engine capabilities, Symtropy's current experiments, and prospective research targets. It is not a performance comparison. No external engine has yet been run on the same frozen temporal scenes and hardware.

## 1. Industry architecture map

| Engine | Documented architecture / strength | What Symtropy must compare fairly |
|---|---|---|
| Box2D 3.2 | Binary dynamic AABB tree; query stats expose leaf visits and node visits; public metrics include tree height, area ratio, proxy count and byte count. | Query frontier work, tree quality, bytes, node-pool effects, filters and the cost of pair/contact updates. Box2D is a 2D engine; do not compare its full simulation cost directly to a 3D engine. |
| Jolt 5.1 | Multiple broadphase layers with separate AABB quadtrees, commonly static and dynamic. Concurrent body movement expands root-to-leaf nodes lock-free; a tighter tree is rebuilt in the background and swapped during the step. Body IDs use sequence numbers to avoid immediate stale-ID reuse. | Layered static/dynamic/sleeping partitioning, expansion debt, background rebuild overlap, synchronization, generation identity and query consistency while rebuilds are in flight. |
| Bullet DBVT | Separate dynamic and fixed DBVT sets, staging/maintenance and a persistent overlapping-pair cache. Cached pairs can preserve contact data such as previous constraint forces for warmstarting. | Pair creation/deletion churn, fixed↔dynamic migration, pair-cache consistency, contact warmstart retention and incremental maintenance costs. |
| MuJoCo Warp | NxN and SAP broadphases have an incremental sleeping pass. With the prior awake mask, only pairs involving newly awakened bodies are emitted; where graph conditionals are available the pass can be skipped entirely if no body woke. SAP has tiled and segmented GPU sorting paths. | Exact wake-frontier semantics, no-wake skip cost, wake storms, GPU sort/scan costs, pair buffer handling and downstream narrowphase refresh. This is the closest direct comparator for sleep-aware pair emission. |
| NVIDIA PhysX | eSAP, eMBP, eABP, ePABP and eGPU. Documentation characterizes SAP as strong with sleeping populations but less suited to all-moving or high insertion/removal churn; MBP addresses moving/insertion-heavy cases but requires regions; ABP automatically manages regions; PABP parallelizes ABP; GPU broadphase has a different execution model. | Workload-stratified comparisons, region setup/occupancy, migration and insertion spikes, CPU/GPU transfer and synchronization, and identical filter semantics. |
| Newton | NxN for small worlds, SAP for coherent larger scenes, and EXPLICIT precomputed compatible pairs. Explicit topology can be efficient for fixed shape connectivity, but its storage scales with compatible pair count and must be reconsidered when topology changes. | Static-topology workloads, pair-array memory, topology mutation, SAP coherence, candidate buffer capacity and the cost of rebuilding explicit pairs. |

### Primary references

- Box2D dynamic tree: https://box2d.org/documentation/group__tree.html
- Jolt Physics 5.1 broadphase: https://jrouwe.github.io/JoltPhysicsDocs/5.1.0/
- Bullet DBVT broadphase: https://github.com/bulletphysics/bullet3/blob/master/src/BulletCollision/BroadphaseCollision/btDbvtBroadphase.h
- MuJoCo Warp API (collision, NxN and SAP): https://mujoco.readthedocs.io/en/latest/mjwarp/api.html
- PhysX broadphase API: https://github.com/NVIDIA-Omniverse/PhysX/blob/main/physx/include/PxBroadPhase.h
- Newton collision pipeline: https://newton-physics.github.io/newton/latest/concepts/collisions.html

## 2. What is and is not novel

Persistent pair caching is not new. Bullet has an overlapping-pair cache that supports reuse of contact information; MuJoCo Warp documents a wake-only incremental pass; PhysX and Jolt already select or partition broadphase strategies according to workload.

The research opportunity is therefore not "cache pairs instead of rebuilding them." It is to make pair/contact maintenance an explicit, revision-bound state protocol with independently replayable evidence:

1. **Pair membership:** `added` and `removed` establish exact active-relation changes.
2. **Contact invalidation:** `revalidated` identifies retained pairs whose endpoint, shape or policy state requires narrowphase/contact refresh.
3. **Identity and policy provenance:** each pair refers to generation-stable collider identities and the exact world, transform, shape and filter revisions that qualified it.
4. **Reproducible transition evidence:** every frame can be replayed against an independent complete oracle; incomplete journals, generation mismatches and unaccounted policy changes fail closed.
5. **Consumer-aware cost:** pair events are not the end of the cost model. Charge contact refresh, manifold churn, warm-start loss, CCD and solver-row consequences.

This is an intended contribution. The current AABB tournament only implements a partial subset; it is not yet a production physics-world integration.

## 3. Architectural hypotheses to test

### 3.1 Pair state is not contact state

A pair can remain in the broadphase relation while its contact geometry changes. Conversely, a pair can remain geometrically overlapping while its filter policy changes. Maintain separate versions for:
- pair membership revision;
- endpoint transform revision;
- shape geometry / scale revision;
- collision filter and layer policy revision;
- narrowphase/manifold revision.

A contact record is reusable only when all relevant revisions still match. The broadphase should emit membership deltas and contact-refresh obligations separately.

### 3.2 Indexed dirty-frontier reconciliation

Use the persistent spatial index for current neighbours and cached adjacency for old neighbours:
- old adjacency reveals pair deaths without querying the old world;
- tree queries reveal new and retained neighbours without scanning every current proxy;
- current tight AABBs refine conservative fat-tree hits;
- canonical pair deduplication handles two simultaneously changed endpoints.

The desired indexed frontier complexity is O(k log N + C) under favourable spatial distributions, where k is the dirty-proxy count and C is the number of spatially returned candidates. Dense scenes can still make C = O(kN), and index maintenance remains part of the cost.

**Current limitation:** the indexed tournament prototype uses safe snapshot-derived dirty discovery, which remains O(N). It removes the later O(kN) scan over current proxies, but does not establish sublinear end-to-end frame work. The authoritative-journal method exists and is tested; a physics-world adapter must bind it to exact world revisions and charge or separately measure its producer.

### 3.3 Bounded negative-space certificates

A future optimization may cache why two tree regions cannot overlap. Such a certificate must bind to exact node identities/generations, bound revisions, filter revision and time horizon. Any refit, expansion, rotation, rebuild, collider-generation change or policy change must invalidate dependent certificates. Start with exact tree queries; do not rely on cached exclusions until the full invalidation proof and negative controls exist.

### 3.4 Motion-budgeted temporal envelopes

Investigate time-bounded conservative bounds that incorporate translation, angular motion, acceleration uncertainty and a declared horizon. An exclusion or pair-retention certificate expires when its motion budget is exhausted. Benchmark the trade-off between fewer re-queries and more conservative candidates, especially for rotation-heavy, high-speed and teleporting objects.

### 3.5 Motion-class portfolio

Compare separate populations for static, kinematic, sleeping, waking, active dynamic, high-speed/CCD and debris/churn. Candidate strategies include:
- static tree with rarely updated bounds;
- dynamic tree / SAP for active bodies;
- wake-frontier query for newly awake bodies;
- brute force for small populations;
- regional or grid partitioning for very large moving populations;
- GPU SAP/LBVH where transfer and launch overhead amortize.

Do not select by proxy count alone. Density, size disparity, motion coherence, dirty fraction, teleport rate, pair-cache density, hardware and downstream narrowphase cost all matter.

## 4. Benchmark matrix

### Scale and dirty fraction

Use proxy counts 128, 1k, 10k and 100k where memory permits. Sweep dirty counts 0, 1, 2, 0.1%, 1%, 10%, 50% and 100%. Include long steady sleep intervals followed by one-body wakes, synchronized wake storms and wake propagation across interacting islands.

### Scene families

- sparse coherent motion;
- dense clustered and worst-case all-overlap;
- static-heavy / mixed static-kinematic-dynamic;
- sleeping-heavy / one-body wake / wake storm;
- high insertion-removal churn and generation reuse;
- teleports and region migration;
- high aspect ratio and severe collider-size disparity;
- jitter across AABB boundaries;
- rotation-dominated conservative bounds;
- high-speed swept bounds and CCD;
- collision filter/layer policy changes;
- nested bounds and equal Morton codes.

Keep D=2/3/4 as data-structure diagnostics. Claims about a production rigid-body engine must use that engine's supported dimensions and its real shape-to-AABB, filters and lifecycle.

### Required denominators and metrics

Correctness gates:
- exact oracle active-pair set;
- exact additions and removals versus consecutive oracle sets;
- exact revalidation set: retained pairs touching a dirty endpoint or another declared invalidator;
- zero duplicates, false negatives, stale generations or orphan adjacency;
- complete oracle coverage and exact world/filter revisions;
- contact/manifold and solver-row denominators preserved across methods.

Work and cost:
- dirty identities and dirty fraction;
- journal producer time, journal validation time and fallback frequency;
- full-frame scan visits;
- tree node visits, leaf tests, returned candidates and exact AABB tests;
- raw and unique affected-pair counts;
- additions, removals, revalidations and pair-cache size;
- index inserts/removes/refits/reinsertions/rotations/rebuilds;
- contact refreshes, new manifolds, removed manifolds, warm-start reuse;
- CCD candidate/work counts and solver rows;
- allocation count, retained bytes, peak memory and memory bandwidth where measurable;
- p50/p95/p99 frame cost, worst-frame spikes, throughput and multi-thread scaling;
- energy per 1k simulated steps on a pinned hardware lane.

Always report component costs and the end-to-end tick. Do not compare pair-event generation time with full candidate-list generation time as if they were equivalent outputs.

## 5. Prospective performance goals

These are targets, not achieved results:

- Zero pair or contact false negatives under the exact declared collision semantics.
- At least 20% lower p95 broadphase-plus-contact-maintenance cost on two held-out sleeping/mixed workload families.
- No more than 5% p95 regression on every other declared family.
- Selector median regret <=5%, p95 <=10%, and selector plus switching overhead <=2%, measured on held-out temporal sequences without future-frame leakage.
- Report small-scene and dense worst-case regressions honestly; no universal backend claim.
- For indexed frontier work, show lower candidate enumeration and total work at low dirty fractions after including journal production, index mutation, contact refresh and cache memory.

## 6. Matched external comparison protocol

1. Pin exact engine commits, compiler, flags, backend versions and device driver/runtime.
2. Export identical temporal shape/collider identities, transforms, conservative AABBs, filters and lifecycle transitions.
3. Match static/kinematic/dynamic and pair-filter semantics; publish a mapping table for features that cannot be matched.
4. Use identical temporal sequences rather than independently generated worlds.
5. Measure each engine's native broadphase and its actual narrowphase/contact update path.
6. Report CPU and GPU lanes separately; do not compare a CPU AABB microbenchmark to a GPU pipeline.
7. Preserve warmup, repetitions, timer resolution, synchronization and memory accounting.
8. Publish raw per-frame receipts and scripts. Keep the selector shadow-only until exact correctness and held-out regret gates pass.

The current internal CPU tournament is only the first AABB-level lane. It is not an external SOTA result.
