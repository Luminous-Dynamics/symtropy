# Broadphase industry comparison and research plan

Date: 2026-11-27

This document separates documented engine capabilities, Symtropy's current experiments, and prospective research targets. It is not a performance comparison. No external engine has yet been run on the same frozen temporal scenes and hardware.

## 1. Industry architecture map

| Engine | Documented architecture / strength | What Symtropy must compare fairly |
|---|---|---|
| Box2D 3.x | Binary dynamic AABB tree. Query stats expose leaf visits and node visits; public metrics include tree height, area ratio, proxy count and bytes used. | Query frontier work, tree quality, bytes, node-pool effects, filters and the cost of pair/contact updates. Box2D is a 2D engine; do not compare its full simulation cost directly to a 3D engine. |
| Jolt 5.2 | Multiple broadphase layers with separate AABB quadtrees, commonly static and dynamic. Concurrent body movement expands root-to-leaf nodes lock-free; a tighter tree is rebuilt in the background and swapped before the step ends. | Layered static/dynamic/sleeping partitioning, expansion debt, background rebuild overlap, synchronization, generation identity and query consistency while rebuilds are in flight. |
| Bullet DBVT | Separate dynamic and fixed DBVT sets, staging/maintenance and a persistent overlapping-pair cache. Contact-manifold lifetime and warm-start reuse are downstream behaviors to measure separately. | Pair creation/deletion churn, fixed↔dynamic migration, pair-cache consistency, manifold creation/removal, warm-start retention and incremental maintenance costs. |
| MuJoCo Warp 3.17 | NxN and SAP broadphases support an incremental sleeping pass. With the prior awake mask, only pairs involving newly awakened bodies are emitted. Graph conditionals can skip the NxN pass wholesale when no body woke; SAP still runs sort/scan but filters emitted pairs. SAP has tiled and segmented GPU sorting paths. | Exact wake-frontier semantics, no-wake skip cost, wake storms, GPU sort/scan costs, pair buffer handling and downstream narrowphase refresh. This is the closest direct comparator for sleep-aware pair emission. |
| NVIDIA PhysX 5.5 | eSAP, eMBP, eABP, ePABP and eGPU. Documentation characterizes SAP as strong with sleeping populations but less suited to all-moving or high insertion/removal churn; MBP addresses moving/insertion-heavy cases but requires regions; ABP automatically manages regions; PABP parallelizes ABP; GPU broadphase is a separate execution lane. | Workload-stratified comparisons, region setup/occupancy, migration and insertion spikes, CPU/GPU transfer and synchronization, and identical filter semantics. |
| Newton | NxN for small worlds, SAP for coherent larger scenes, and EXPLICIT precomputed compatible pairs. Explicit topology can be efficient for fixed shape connectivity, but its storage scales with compatible pair count and must be reconsidered when topology changes. | Static-topology workloads, pair-array memory, topology mutation, SAP coherence, candidate buffer capacity and the cost of rebuilding explicit pairs. |

### Primary references

- Box2D dynamic tree and query statistics: https://box2d.org/documentation/group__tree.html
- Jolt Physics 5.2 broadphase: https://jrouwe.github.io/JoltPhysicsDocs/5.2.0/
- Bullet DBVT broadphase and its fixed/dynamic trees + pair cache: https://github.com/bulletphysics/bullet3/blob/master/src/BulletCollision/BroadphaseCollision/btDbvtBroadphase.h
- MuJoCo Warp 3.17 API (incremental sleeping pass, NxN and SAP_TILE/SAP_SEGMENTED): https://mujoco.readthedocs.io/en/3.17.0/mjwarp/api.html
- PhysX 5.5 broadphase choices: https://nvidia-omniverse.github.io/PhysX/physx/5.5.0/docs/RigidBodyCollision.html
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

**Current limitation:** the indexed tournament backend is still measured through safe snapshot-derived dirty discovery, which remains O(N). It removes the later O(kN) scan over current proxies, but does not establish sublinear end-to-end frame work in the current campaign. A separate trusted `step_with_dirty_ids` path now permits an O(1) empty-journal no-op and O(k log N + C) indexed reconciliation under the canonical-frame and complete-journal preconditions; it revalidates only dirty current AABBs. A checked variant retains O(N) frame validation and reports it separately. Neither journal path is yet the timed tournament backend. Production use must bind the journal to exact world/proxy/filter revisions, prove canonical non-dirty state at the mutation-source boundary, and charge or separately measure journal production.

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


## 7. Updated industry state and benchmark strategy (2026-11-27)

This section updates the architecture map with newer engine documentation and a stronger external benchmark methodology. It remains an architecture comparison, not a matched performance result.

### 7.1 Industry developments to include in the next baseline

| Engine / suite | Newer documented capability or benchmark practice | Consequence for Symtropy |
|---|---|---|
| MuJoCo 3.17 / MuJoCo Warp 3.17 | The incremental sleeping pass emits contacts involving newly awakened bodies; persistent batch simulation state supports sleep; GPU SAP has tiled and segmented sort lanes. | Measure idle-no-wake skip, wake-frontier size, island propagation, and CPU/GPU paths separately. Wake-only pair emission is baseline functionality, not the novelty claim. |
| Jolt 5.4 | Lockless quadtree update path expands ancestor bounds during concurrent mutations and performs tighter rebuild in separate prepare/finalize phases. | Add concurrent update/query, expansion debt, background rebuild overlap, synchronization and worst-frame consistency tests. |
| PhysX 5.5 | SAP, MBP, ABP, PABP and GPU broadphase have workload-specific trade-offs; GPU may be sensitive to dense projected intervals and transfer/synchronization costs. | Treat broadphase as a strategy portfolio. Benchmark region topology, migration, density, high-churn and GPU transfer costs, not just CPU query time. |
| Rapier / Nexus / Physmark | Dimforge's Q3 2026 engineering report describes persistent graph coloring, intra-island parallelism and parallelized broadphase work in Rapier. It reports a separate GPU physics lane for Nexus. Physmark publishes reproducible engine trials across several physics backends, including pinned source revisions, shared scenes, warm-up/synchronized runs, replay checks, and retained failure evidence. | Add parallel CPU and GPU lanes. Preserve the difference between vendor-reported results and independently repeated matched results. Use Physmark's evidence discipline as a model; do not compare its full physics-step throughput directly to our current AABB microbenchmark. |

References:
- [MuJoCo 3.17 API](https://mujoco.readthedocs.io/en/3.17.0/mjwarp/api.html)
- [Jolt 5.4 Quadtree broadphase](https://jrouwe.github.io/JoltPhysicsDocs/5.4.0/)
- [PhysX 5.5 collision and broadphase choices](https://nvidia-omniverse.github.io/PhysX/physx/5.5.0/docs/RigidBodyCollision.html)
- [Dimforge Q3 2026 technical report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/)
- [Physmark benchmark suite](https://github.com/dimforge/physmark)
- [Rapier / Godot benchmark methodology and results](https://godot.rapier.rs/docs/documentation/performance/)

The Dimforge performance figures are vendor engineering results, not independent cross-engine adjudication. The Rapier/Godot tables are application-integrated physics-step results. Neither is directly comparable to Symtropy's stage-level AABB time. Preserve source type and output contract next to every imported result.

### 7.2 Three separate leaderboards—not one blended score

**Lane A: algorithm-stage microbenchmarks**

- Same frozen AABB streams and proxy IDs.
- Compare brute force, SAP, dynamic tree, LBVH, scan pair delta and indexed pair delta.
- Report candidate/event output contract, per-frame p50/p95/p99, full work counters, memory and exact oracle errors.
- This lane answers: which algorithm does less work for the same stage-level contract?

**Lane B: shape and contact-maintenance stage**

- Same transformed shapes, filters, lifecycle and exact contact reference.
- Use independent 2D and 3D OBB SAT from PR #7622; admit 4D only after issue #7684 passes its 56-direction generalized zonotope SAT acceptance gates.
- Enforce the conservative broadphase safety property:
  `ExactShapeContacts(frame) ⊆ BroadphaseCandidates(frame)`.
- Report candidate recall (must be 100%), candidate amplification, exact-shape FP/FN separately, contact-stamp reuse, refreshes, warm-start resets, eligibility rechecks, and solver reactivations.
- Keep this lane linked to [#7691 — shape coverage + contact freshness](https://github.com/Luminous-Dynamics/symtropy/issues/7691).

**Lane C: full-engine temporal scenes**

- Export the same scene trajectories to Symtropy and each supported external engine.
- Compare full simulation ticks: body integration, shape-to-AABB, broadphase, filtering, narrowphase, manifolds, CCD, sleeping/islands and solver.
- Pin engine commit, compiler/toolchain, feature flags, device/driver, deterministic mode, timestep, solver iterations, warm-up, sync, scene seed and hardware.
- Publish unsupported features, failed/timeout trials and engine-version incompatibilities. Never remove a failed row from aggregation; mark it failed/unsupported and explain why.
- CPU and GPU trials are separate lanes; include launch/synchronization/transfer costs in GPU results.
- Do not pool the full-engine leaderboard with Lane A or B.

### 7.3 Reproducible external benchmark protocol

Adopt the evidence discipline used by public multi-engine benchmarking systems:
1. Pin each upstream engine to an exact commit or immutable release tag.
2. Generate scenes once and export one canonical, versioned scene artifact per seed.
3. Verify identical object counts, shape geometry, transforms, masses/inertias, filters, timestep and lifecycle chronology. Publish a semantic mapping where engines differ.
4. Warm each engine with the same declared pre-roll; synchronize devices before and after measured windows.
5. Preserve every raw trial, including crashes, timeouts, numerical divergence, memory exhaustion and unsupported configuration.
6. Attach run receipts: source/toolchain/hardware manifest, input hashes, seed, frame count, p50/p95/p99, peak RSS/device memory, allocations if available, and per-frame correctness counters.
7. Replay each scene/receipt independently. A result without reproducible replay is exploratory, not a promotion result.
8. Separate vendor-reported reference figures from Symtropy's own reproduced figures.

### 7.4 Stronger correctness and performance gates

Retain the existing zero pair-FN / exact AABB oracle requirements, and add:
- 100% exact-shape-contact recall at the broadphase candidate boundary for the frozen OBB corpus;
- zero false positives/negatives in the independent SAT subset used for exact contact classification;
- zero stale manifold or warm-start reuse; zero unaccounted revalidation reason;
- independent receipt replay agrees on every added/removed/revalidated pair and every post-transition contact stamp;
- zero missed generation changes, world revision gaps or collision-policy updates;
- candidate amplification reported by family and by density; do not hide high-candidate dense projections in an aggregate;
- all p50/p95/p99 measurements include journal production, index maintenance, replay verifier, contact work and end-to-end tick;
- the current prospective promotion threshold remains at least 20% lower p95 on two held-out sleeping/mixed families with no more than 5% p95 regression on any other declared workload, plus zero correctness failures.

### 7.5 Research differentiators that remain open

The strongest contribution hypothesis is now an evidence-qualified state-maintenance protocol, not a new generic data structure:

1. **Revision-bound contact retention:** prove exactly which manifold and warm-start state remains reusable.
2. **Commit-after-action receipts:** only advance a contact stamp after the required narrowphase refresh, warm-start reset, filter recheck and solver activation actually completed.
3. **Geometric recall composition:** prove exact shape contacts are contained in the conservative broadphase candidate set, while separately reporting false-positive amplification.
4. **Motion-class portfolio with bounded regret:** use static/sleeping/active/high-speed/churn classes and hardware/workload observations; keep the selector shadow-only until held-out regret and switching costs qualify.
5. **Temporal negative-space certificates:** cache non-overlap only with exact node/bound/policy generations and explicit expiry/invalidation proof.
6. **Motion-budgeted bounds:** retain temporal envelopes only while conservative translational, angular and acceleration budgets remain valid; route teleports and uncertain motion to a safe full query.

All six are falsifiable research hypotheses. Do not claim any is novel until the relevant prior art is checked and the matched experiments are published.

### 7.6 Benchmark-denominator hardening

The Dimforge Q3 2026 technical report says Physmark detects engine timeouts and ignores the corresponding datapoint. This is understandable for an exploratory display of engines that completed a scenario, but it creates a survivor-only comparison risk if adopted as the primary campaign denominator. Symtropy's formal campaign should improve on this:

- Freeze the complete requested scenario/horizon set before running any backend.
- Preserve every timeout, OOM, crash, invalid result, unsupported feature and not-run case.
- Show completed throughput and coverage together; do not calculate the principal comparison from only completed rows.
- Count zero qualified simulated ticks for a failed or incomplete horizon while retaining its raw elapsed time and failure category.
- Publish both raw completed ticks/s and qualified ticks/s, plus the exact missing/failed denominator.
- Keep every CPU/GPU/device comparison lane separate unless both hardware and semantic comparison groups are explicitly matched.

This is a methodological differentiator, not an algorithmic speed result. It prevents a system from appearing faster by failing to finish difficult cases. See the [Dimforge Q3 2026 report](https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/), and compose with [#3822](https://github.com/Luminous-Dynamics/symtropy/issues/3822), [#3866](https://github.com/Luminous-Dynamics/symtropy/issues/3866) and [#4559](https://github.com/Luminous-Dynamics/symtropy/issues/4559).


### 7.7 Contact recycling is now established prior art

The October 2026 Box2D contact-recycling publication changes our novelty boundary. Box2D 3.2 enables contact recycling by default: it caches a relative transform, bounds relative translation/rotation, reuses contact points when movement is small, and updates separation to retain stability. The 3.2 release notes describe default thresholds around 5 cm / 11 degrees and per-world/per-body controls. The Box2D 3.2 release reports an approximately 3x total improvement over 3.1, with contact recycling one contributor; treat that as a vendor-reported breakdown, not a matched external result.

Symtropy must not claim basic relative-motion manifold reuse as novel. Compare Box2D 3.2 one-shot manifold recycling, PhysX persistent contact manifold/contact-cache alternatives, and exact pinned Bepu/Avian/Rapier implementations where appropriate.

The research target is an **evidence-qualified lease**: feature-margin lower bounds, normal-variation bounds, gap intervals, exact shape/proxy generations, material/callback response and solver freshness as separate state, current-step callback delivery preserved, and independent replay. The new `contact_reuse_lease` module is a reference admission predicate over externally supplied witnesses, not production integration.

Box2D also warns that cached relative transforms must not be rebased every recycled step because this accumulates drift. Symtropy's benchmark must include long-duration drift tests where the cached reference transform changes only on a fresh narrowphase/manifold build.

References:
- [Box2D contact recycling design (2026-10-12)](https://box2d.org/posts/2026/10/contact-recycling/)
- [Box2D 3.2 release notes](https://box2d.org/documentation/md_release__notes__v320.html)
- [Box2D simulation/contact documentation](https://box2d.org/documentation/md_simulation.html)


### 7.8 Multi-rate geometry and response semantics

Box2D 3.2's contact recycling is a particularly important comparator. Its v3.2 release notes say recycled touching contacts reuse their manifold and skip material updates and the pre-solve callback until relative movement reaches about 5 cm or rotation about 11 degrees; the behavior is on by default and can be disabled globally or per body. Box2D's documentation warns that recycled contacts skip friction/restitution updates and pre-solve callbacks. Its 3.2 announcement attributes roughly 40% of the 3.1-to-3.2 performance gain to contact recycling, with additional gains from solver and SIMD/broadphase changes. Those are vendor-reported figures, not an independently matched Symtropy comparison.

This creates an opportunity specific to Symtropy's callback architecture:
- Reuse geometric anchors/manifold data only under the conservative lease.
- Continue current-step friction_multiplier and modulate_impulse evaluation when the solver path requires it.
- Continue per-impact on_collision / apply_trauma semantics and record_dissipation; a geometric lease cannot suppress event delivery.
- Separate the global callback/response law digest from the per-endpoint dynamic response-state revision.
- Invalidate warm-start cache state when response inputs change, while retaining geometry when its independent lease still holds.
- Bind event callbacks to their own event identity/revision and commit receipt; geometry reuse is not an exactly-once event-delivery guarantee.

The new GlobalContactRevisions.contact_response_policy_sha256 and EndpointRevisions.response_state implement only the revision-decision scaffolding. The contact_reuse_lease still consumes external geometric witnesses, and no production callback/manifold adapter is implemented.

Benchmark this as three different semantic modes, not one pooled score:
1. always-fresh geometry + always-current callbacks (Symtropy correctness baseline);
2. geometric lease + always-current callbacks and event delivery (target architecture);
3. Box2D-style lease with callback/material update deferral (performance comparator, explicitly marked semantic trade-off).

Report callback counts and timing, material/response update counts, event counts and end-to-end tick latency. The target architecture must preserve the same callback/event outputs as mode 1 while reducing geometry work. Mode 3 can only be compared on scenes where the semantic difference is acceptable and must never be labelled equivalent.

References:
- [Box2D 3.2 release notes: callback/material behavior](https://box2d.org/documentation/md_release__notes__v320.html)
- [Box2D simulation guide: recycling distance and skipped updates](https://box2d.org/documentation/md_simulation.html)
- [Box2D 3.2 performance breakdown](https://box2d.org/posts/2026/10/box2d-3.2/)

### 7.9 Current CPU/GPU engine state and matched target ladder

**PhysX 5.5** is a workload portfolio, not a single baseline: SAP performs well with many sleeping objects but degrades when all move or insertion/removal is heavy; MBP is an alternative for all-moving/insertion-heavy scenes; ABP often gives a strong average performance/memory trade-off; PABP adds threading and pays more memory, especially benefitting larger scenes; GPU broadphase is designed for parallel moving/insertion-heavy cases. These are vendor-documented trade-offs and should define separate scene families rather than one global result. [PhysX 5.5 collision docs](https://nvidia-omniverse.github.io/PhysX/physx/5.5.0/docs/RigidBodyCollision.html)

**Rapier / Nexus (Dimforge Q3 2026)** are particularly important performance competitors. Rapier added intra-island parallelism, persistent graph coloring and a parallelized broadphase; Dimforge reports up to 2× single-thread and 4× multi-thread speedup versus its previous version on an M4 Max with eight worker threads. Nexus reports a 505K-body rigid-body pyramid at about 14 ms/frame on an RTX 5080, without sleeping-island optimization. These are vendor engineering measurements on their own scenes and devices, not directly comparable to Symtropy's AABB microbenchmark. [Dimforge Q3 2026 report](https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/)

**MuJoCo Warp 3.17** offers an incremental sleeping pass that emits only pairs involving newly awakened bodies, as well as SAP_TILE and SAP_SEGMENTED sorting. Benchmark no-wake frames, wake-frontier size, island propagation and sort/scan overhead. [MuJoCo Warp API](https://mujoco.readthedocs.io/en/3.17.0/mjwarp/api.html)

**Jolt 5.4** uses a lockless quadtree update path that widens parent bounds during object changes and later tightens/rebuilds the tree in prepare/finalize phases. Benchmark concurrent updates, query consistency, bound-expansion debt and rebuild cost. [Jolt QuadTree 5.4](https://jrouwe.github.io/JoltPhysicsDocs/5.4.0/class_quad_tree.html)

**Box2D 3.2** is now a direct manifold-recycling comparator. Its contact recycle default skips material updates and pre-solve callbacks until about 5 cm relative motion or 11 degrees rotation. The 3.2 announcement reports roughly 3× overall performance vs 3.1 and attributes about 40% of the gain to contact recycling; preserve this as vendor-reported and mark the callback/material semantic trade-off. [Box2D 3.2 release notes](https://box2d.org/documentation/md_release__notes__v320.html), [performance write-up](https://box2d.org/posts/2026/10/box2d-3.2/)

### 7.10 Denominator and completeness policy

Dimforge's Q3 2026 Physmark report says timeouts are detected and the corresponding datapoints are ignored. Symtropy's formal benchmark should not copy that survivor-only denominator: freeze all requested cases before execution, retain timeout/OOM/crash/unsupported/not-run outcomes, count zero qualified horizon ticks for incomplete runs, and publish completed throughput together with requested-case coverage. [Physmark timeout behavior](https://www.dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/)

The current target is not to claim an aggregate win over PhysX/Rapier/Nexus immediately. First win the matched algorithm-stage and contact-stage lanes with zero false negatives, then compare full-engine temporal scenes on exactly matched hardware, scene artifacts, solver settings and semantic callbacks. CPU/GPU and candidate-only/contact-complete results stay in separate ledgers.
### 7.11 Lease admission overhead is its own measurement

A new `contact-lease-benchmark` binary produces nine scenarios across 2D/3D/4D and reports per-check time and outcome accounting. The CI campaign runs 100,000 checks per row and retains 27 rows. This isolates the cost of processing a geometric lease witness and deciding reuse versus refresh.

Do not interpret the microbenchmark as proof of contact recycling speedup: it does not build the feature margin, normal-variation interval, updated separation, nor run actual GJK/EPA/SAT. The next matched stage comparison must pair it with fresh narrowphase cost and include witness derivation and update-separation cost. The final comparison must include solver and callback/event semantics.


### 7.12 Transform-derived rotation displacement bounds

The contact lease no longer accepts a caller-reported maximum rotation angle. Its input is the entrywise outward upper bound on the absolute difference between the current and last-fresh relative rotation matrices. It derives a Frobenius-norm upper bound and multiplies by the maximum contact-anchor lever arm.

For any anchor x, `||ΔR x||₂ ≤ ||ΔR||₂ ||x||₂ ≤ ||ΔR||F ||x||₂`. The Frobenius bound is more conservative than the spectral norm but avoids angle extraction and covers any number of independent rotation planes, including 4D. Each supplied matrix entry must already bound the absolute difference outward; the code then performs outward-rounded squares, sums, square root and radius multiplication.

This is a better witness interface than a scalar angle, but the production adapter still must derive the entrywise matrix-difference bounds from authoritative transforms and account for transform quantization/error. The current lease microbenchmark uses synthetic matrix witnesses; it does not yet measure transform extraction from live physics state.