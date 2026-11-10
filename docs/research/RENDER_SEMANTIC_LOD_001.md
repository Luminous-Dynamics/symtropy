# RENDER-SEMANTIC-LOD-001 — Quality under budget without semantic drift

**Status:** Draft. A pure Rust allocation kernel and unit tests are authored, but have not yet been compiled or run. This is not yet a Bevy mesh-LOD integration or a measured performance result.

## Why this work

Symtropy already has a deterministic PBR/frame benchmark (#2009), multidimensional view profiles (#2107), read-only physics overlays (#2019), capture receipts (#1999/#2008), and a separate water-rendering qualification plan (#2184). This tranche avoids duplicating those systems.

The new idea is to allocate visual quality by declared screen-space error, semantic importance, and calibrated relative representation cost. When over budget, the deterministic policy downgrades the least costly quality loss first. Selected and critical entities remain at their finest supplied representation. If a hard error ceiling or the available cost budget cannot be met, the allocator reports failure rather than silently relaxing the requirement.

## Implemented

- Standalone policy module exposed from symtropy-render-bridge, without new third-party dependencies.
- Stable entity and representation IDs; ordered candidate validation; finite non-negative screen-space errors.
- A declared maximum screen-space error; selected/critical entities pinned to their finest candidate.
- Deterministic heap-based downgrade priority: incremental importance-weighted squared screen error per cost unit saved, with entity ID as the tie-break.
- Stable output ordering, duplicate/malformed-input rejection, infeasible-budget rejection, and unit tests for core behaviors.
- Machine-readable manifest at benchmarks/render/SEMANTIC-LOD-01.json.

This is a policy kernel plus a manifest-driven CPU benchmark runner. The host still supplies projected-error estimates and cost units. It does not currently extract mesh error, read the camera, switch Bevy assets, alter render-graph state, or prove physics-state independence by capture comparison. The runner lives in `crates/apps/symtropy-lod-benchmark`; its dedicated workflow is `.github/workflows/semantic-lod-benchmark.yml`. It emits an exact-source, lockfile-digest, and fixture-digest-bound JSON report. No report from the new runner is considered valid until the exact-head workflow succeeds.

## Benchmark plan

Run deterministic workloads at 100, 1,000, and 10,000 entities with `cargo run --locked -p symtropy-lod-benchmark -- --samples 30 --output target/semantic-lod-report.json`. The harness validates invariants on every run and emits the exact source revision, `Cargo.lock` SHA-256, fixture SHA-256, compiler version, platform, allocator p50/p95/p99, quality-rank histogram, cost budget and realized cost, error-ceiling outcomes, protected-detail outcomes, and an output SHA-256. Measure allocator p50/p95/p99, the quality-rank histogram, cost budget and realized cost, error distribution, and protected-feature outcomes. When integrated into a rendered scene, also report frame intervals and GPU timestamps separately; wgpu GPU timestamps are only available when the adapter supports the required feature.

Proposed maximum projected-error profiles are 1 px for precision inspection, 2 px for interactive detail, and 4 px for strategic overview. These are hypotheses, not established perceptual thresholds.

Reuse the existing #2009 timing profiles: candidate balanced 1080p p95 at or below 16.7 ms and p99 at or below 25 ms; candidate low-power 720p p95 at or below 33.3 ms. These are targets, not measured results. Frame intervals must not be described as GPU pass time.

Hard gates: zero protected-entity downgrades; zero selected candidates over the error ceiling; deterministic results under entity-order permutation; no accepted invalid candidate sets; exact budget satisfaction when feasible; and explicit rejection when infeasible.

## Current engine baseline verified 2026-10-20

The public Symtropy workspace currently pins Bevy 0.19 in the root manifest and render bridge. Bevy 0.20 was announced on October 8, 2026; it includes the Solari renderer path improvements and significant retained scene / renderer work. The existing migration gate is tracked separately in Symtropy issue #1989 with the isolated graphics probe in PR #2012. Do not make this LOD policy silently depend on a renderer-version change.

Bevy's official 0.19 release notes report that the `many_cubes` example on a laptop RTX 4090 moved from 49.47 ms to 18.77 ms with culling enabled between 0.18 and 0.19, and from 93.1 ms to 41.2 ms with culling disabled. The same release notes report 11.8 ms static / 16.2 ms moving for a 55,000-rendered-entity `bevy_city` example. These are vendor-published results on a different GPU and a different workload; they are **context, not direct comparisons** against Symtropy or its RTX 2070 Mobile target.

The architectural implication is important: do not recreate Bevy's GPU-driven draw/batching or occlusion systems inside this project. Treat semantic LOD as a task-aware, explicit quality policy that feeds a Bevy-native presentation path. First compare the current pinned 0.19 backend; once the separate 0.20 migration gate passes, run the same frozen fixtures on 0.20. Keep source, renderer version, adapter, driver, camera, and settings in each receipt. This isolates whether a gain comes from the LOD policy or the engine upgrade.

Nanite already couples fine-grained LOD with occlusion culling and screen-scale detail; Epic also documents cases where highly fragmented aggregate geometry and overdraw weaken those scaling properties. Those cases suggest useful stress fixtures—foliage/porous geometry, nested close surfaces, and dense engineering assemblies—but do not establish that Symtropy will outperform Nanite. The research target remains a matched quality–cost frontier with semantic and state-integrity constraints.

Sources: [Bevy 0.20 release, October 8, 2026](https://bevy.org/news/bevy-0-20/); [Bevy 0.19 renderer performance notes](https://bevy.org/news/bevy-0-19/); [Epic's Nanite scalability and limitations](https://dev.epicgames.com/documentation/en-us/unreal-engine/working-with-naniteenabled-content).

## Comparison with industry

Use Bevy's deferred-rendering and mixed-lighting examples as implementation baselines. Use Unreal Engine's Nanite/Lumen capabilities and Unity HDRP as high-end visual comparison references, not as claims of feature parity. Compare matched assets, fixed camera paths, resolution, lighting intent, hardware, warmed runs, frame-time distributions, and reference captures. 3DMark Steel Nomad can contextualize device stability and thermal behavior, but its score is not directly comparable to Symtropy's own scene.

A credible advantage means preserving task-relevant detail at lower measured cost, or improving detail at matched cost, across reproducible workloads. One screenshot, a passing unit test, or unrelated benchmark scores cannot establish superiority over SOTA.

## Next tranche

1. Complete formatting, strict Clippy, focused render-bridge tests, and the exact-head workload workflow; fix failures without weakening invariants.
2. Inspect the heap-versus-scan parity and timing report. The scan path is an algorithmic reference for the same greedy decision rule, not an engine-level or renderer-performance comparator.
3. Calibrate candidate cost units from measured backend workloads.
4. Add geometric-error projection fixtures and camera-motion hysteresis.
5. Integrate one opt-in Bevy LOD path and compare against the #2009 baseline.
6. Use existing capture receipts to prove camera/LOD changes leave canonical physics results unchanged.

## References

- Bevy deferred rendering: https://bevy.org/examples/3d-rendering/deferred-rendering/
- Bevy mixed lighting: https://bevy.org/examples/3d-rendering/mixed-lighting/
- Unreal Lumen: https://dev.epicgames.com/documentation/en-us/unreal-engine/lumen-global-illumination-and-reflections-in-unreal-engine
- wgpu timestamp queries: https://docs.rs/wgpu/latest/wgpu/enum.QueryType.html
- 3DMark: https://www.3dmark.com/benchmarks
- OGC I3S LOD: https://docs.ogc.org/cs/17-014r5/17-014r5.html
