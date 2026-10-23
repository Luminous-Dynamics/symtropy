# SOLAR-SCALE-COMPARISON-01 — Matched renderer benchmark protocol

**Status:** Proposed, unmeasured. This is a benchmark contract, not a performance result or a claim that Symtropy currently outperforms an established renderer.

## Why this is the next step

The current solar-scale work has a precision ladder, screen-space apparent-size calculations, a deterministic hierarchy selector and a runnable illustrative viewer. The missing evidence is a reproducible comparison that couples **visual correctness, task-critical content retention, streaming behavior, frame-time tails and resource use**.

Industry reference: Cesium's 3D Tiles API documents `maximumScreenSpaceError = 16` as the default refinement threshold. Cesium's own LOD guidance describes 8–16 as a common higher-fidelity starting range and 24–64 as a performance-oriented range. Those values are useful policy sweep points, not proof that two engines render equal-quality scenes at the same numeric setting. See the [Cesium3DTileset API](https://cesium.com/learn/cesiumjs/ref-doc/Cesium3DTileset.html?classFilter=cesi) and [Cesium LOD guidance](https://cesium.com/learn/cesiumjs-learn/3d-guassian-splat-tilesets-lods/).

## Two tracks; do not conflate them

### 1. Policy conformance — available before GPU comparisons

Run the Rust coordinate, apparent-size and hierarchy policies against the frozen SOLAR-SCALE-01 cases. Sweep screen-space-error policies at 8, 16, 32 and 64 pixels. Validate each selected representation against an independent projection/geometry oracle.

Zero tolerance applies to invariant failures: semantic identity loss, protected-detail drops without an explicit unavailable result, uncertified replacement holes, coordinate-budget violations, and nondeterministic selection under input permutations.

This gives us a useful early falsification layer without needing GPU runners or an installed graphics stack.

### 2. Matched renderer — only after a common dataset adapter exists

A real cross-engine comparison requires identical geometry/asset bytes, camera samples, physical viewport dimensions, and asset-residency events. Match hardware, OS, drivers, backend and build modes where feasible. Record inevitable feature differences rather than pretending the engines are identical.

If those controls cannot be matched, publish separate results instead of a head-to-head superiority claim. The current illustrative viewer and its static circular orbit guides are not a substitute for a shared, authoritative astronomical dataset.

## Workload gates

| Case | Failure to catch | Required result |
|---|---|---|
| Local detail | Selected feature disappears during coarsening | Preserve identity and visibility or return an explicit protected-detail-unavailable result |
| Surface streaming | Missing child tiles appear as empty space | Hold a ready parent until replacement coverage is resident and certified; otherwise fail visibly |
| Orbit transition | Origin jump or proxy scale discontinuity | Canonical positions stay unchanged and discontinuity stays within the frozen budget |
| Earth–Moon | Large-world subtraction loses detail | Independent position/projection error remains within tolerance |
| Inner/outer system | Absolute f32 placement or uncontrolled exaggeration | No absolute-f32 astronomical placement; separate physical radius from presentation radius |
| Return round trip | Original selection cannot be restored | Same semantic entity and canonical position return |
| Adversarial residency | Loading, invalid coverage claims or shuffled ECS order change meaning | Fail-closed coverage and deterministic output |

The machine-readable cases and six hard acceptance gates are in [`SOLAR-SCALE-COMPARISON-01.json`](../benchmarks/render/SOLAR-SCALE-COMPARISON-01.json).

## Measurement method

For rendered runs, warm up for 120 frames, measure at least 1,000 frames per repeat, and collect at least five repeats. Report each repeat and repeat-level p50/p95/p99; do not pool samples in a way that hides run-to-run variation. Record CPU frame interval and GPU timestamp independently where available. Missing instrumentation must be represented as unavailable, never replaced with estimates.

Every receipt binds the exact source commit, workload digest, lockfile, Rust toolchain, build flags, shader/asset digests, renderer/backend, OS, driver, GPU/CPU, resolution and camera path. Synthetic performance and real-world performance must be reported separately.

## Proposed acceptance targets — not yet qualified

- 60 FPS interactive target: frame-interval p95 no higher than 16.7 ms and p99 no higher than 25 ms.
- 30 FPS fallback target: p95 no higher than 33.3 ms and p99 no higher than 50 ms.
- All six correctness/retention gates pass with zero violations.
- Performance comparisons include resident/peak memory, missing assets, representation count, temporal chatter and camera discontinuities, not frame rate alone.

These targets are hypotheses to test on named hardware, not claims about current Symtropy performance. Do not gate CI performance on noisy shared runners; first establish stable hardware, variance and repeated baselines. Correctness contracts can run in ordinary CI.

## How we claim progress over the industry

Use Cesium's pixel-error policy as an initial reference point, then compare engines on a matched workload. A valid improvement means lower measured cost **at equal-or-better oracle error and invariant compliance**, not merely a higher allowable screen-space error or fewer details drawn. Publish the receipt and limitations alongside any claimed win.

## Reference boundary

- [Cesium3DTileset API](https://cesium.com/learn/cesiumjs/ref-doc/Cesium3DTileset.html?classFilter=cesi)
- [Cesium LOD guidance](https://cesium.com/learn/cesiumjs-learn/3d-guassian-splat-tilesets-lods/)
- [OGC 3D Tiles 1.1](https://docs.ogc.org/cs/22-025r4/22-025r4.html)

No benchmark values are filled in before measurements exist.
