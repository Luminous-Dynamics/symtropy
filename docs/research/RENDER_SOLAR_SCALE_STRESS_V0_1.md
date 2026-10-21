# RENDER-SOLAR-STRESS-001 — Numerical and temporal scale stress contract

**Status:** Research regression slice. The tests do not qualify GPU throughput, shader precision, planetary accuracy, or production scene scale.

## Why this is a useful next step

The scale policy already has known-answer projection tests, and the representation latch already has a small hysteresis example. Those establish local behavior. Extreme-scale and long-trace regressions need explicit coverage before expanding the scene or claiming a renderer advantage.

The new tests pin two properties:

1. **Scale invariance:** when radius and camera-centre distance are scaled together, projected apparent diameter must remain invariant. The fixture holds the ratio at (10^{-6}), tests radii from (10^{-150}) through (10^{150}), and requires finite output with relative error at most (10^{-12}) against a reference result.
2. **Temporal stability:** 20,000 consecutive samples alternating around a band boundary at 0.96 px / 1.04 px must not flip a latch configured with 10% hysteresis.

These are deterministic numerical tests, not a claim that a graphics pipeline can preserve precision over the same range in GPU float32. GPU transforms remain a separate qualification boundary.

## Next benchmark stages

### Stage A — representation selection scaling

Generate deterministic hierarchies with (10^2), (10^3), (10^4), then (10^5) nodes. Include balanced trees, deep chains, several independent roots, missing/loading child content, additive nodes, forced refinement, and certified/uncertified replacement coverage.

Record:

- selected node count and held-parent reason counts;
- validation/selection CPU time, p50/p95/p99 across repeated runs;
- bytes per input node and peak resident memory;
- determinism under shuffled input order;
- invariant violations, which must remain zero.

Keep timings informational until runners/hardware are pinned; correctness and fail-closed behavior are gating.

### Stage B — projected-size and camera-path sweep

Capture a fixed camera path over logarithmically spaced distances and resolutions (720p, 1080p, 1440p, 4K). Record the physical projected diameter, selected band, latch transitions, origin-relative position error, rejected precision conversions, and visual-size exaggeration. Require no canonical-position mutation and no unlabelled representation discontinuity.

The CPU reference should calculate projection in f64. The eventual vertex-shader path must be compared against this reference with an explicit pixel-error budget; f64 CPU arithmetic is not evidence of GPU float32 accuracy.

### Stage C — actual render pipeline

Measure CPU frame interval and GPU execution separately. On Vulkan, use timestamp queries around named render passes; do not infer GPU time from wall-clock frame time or assume that queued frames completed on the GPU. Record adapter/device, driver, backend, resolution, scene digest, visible instances, submitted draw/dispatch counts, triangles, upload bytes, resource residency, and thermal/power state.

A comparative result is admissible only if scene contents, camera sequence, output-quality targets, resolution, hardware, graphics backend, driver, warm-up, run count, and measurement method match. Report median and tail latency, frame deadline misses, visual error, temporal popping, and memory alongside throughput.

## Research references

- Khronos Vulkan timestamp query sample: https://docs.vulkan.org/samples/latest/samples/api/timestamp_queries/README.html
- MLPerf's reporting approach is a useful model for explicit workload definitions and quality targets, but it is not itself a planetary renderer benchmark: https://mlcommons.org/benchmarks/inference-datacenter/

## Claim ceiling

This contract adds regression tests and a staged measurement plan. It does not report a speedup, an SOTA result, or scientific ephemeris accuracy. The viewer still needs a qualified GPU path, fixed capture receipts, and a same-hardware comparison before those claims can be made.
