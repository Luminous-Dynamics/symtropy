# Symtropy renderer benchmark

`symtropy-render-benchmark` is an asset-free, deterministic PBR reference scene for renderer comparisons. It is intentionally separate from the production physics and agent runtimes.

## Fixture: SCENE-LIGHT-01

- Integer-lattice cube grid; no random numbers, external meshes, textures, or network assets.
- One shared mesh and eight shared PBR material assets.
- Fixed directional key light, explicit shadow-map mode, ambient illumination, neutral ground plane, and fixed camera algorithm.
- Camera framing is derived from the grid dimensions; the camera algorithm identifier remains `auto-grid-camera-v1`.
- A UI label exposes the fixture ID, object count, profile and shadow mode to reviewers.
- The frozen fixture contract lives in `benchmarks/render/SCENE-LIGHT-01.json`.

This fixture is for controlled rendering comparisons. The source commit identifies the scene generator implementation; the fixture digest identifies the frozen fixture contract; the render-settings digest identifies the exact profile, count, shadows, camera ID, and resolution. All three are retained independently.

## Run a qualified capture

The runner requires explicit caller-declared graphics adapter, driver, and backend labels. For example, after replacing these example values with the real machine values:

```sh
export SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER="NVIDIA GeForce RTX 2070"
export SYMTROPY_DEMO_CAPTURE_GPU_DRIVER="exact driver version"
export SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND="Vulkan"
bash scripts/run-render-benchmark.sh
```

The runner requires a clean Git worktree, reads the exact source commit and Bevy version, hashes the frozen fixture JSON, creates a canonical render-settings JSON sidecar and digest, binds output resolution/profile/object count/shadow mode into capture metadata, then launches the app in release mode. It sets the default capture schedule to 1.5, 4.0, and 7.0 seconds. Use the final settled frame for a reference comparison; the scene itself is static after startup.

The capture plugin writes PNGs to `target/render-captures/<profile>`. Once the companion capture-receipt change is integrated, the same run also writes a versioned JSON receipt. The runner additionally requires a fresh `frame-time.json` report before it exits successfully.

## Automated scaling sweep

Once the individual scene and frame-time tests are passing, run a complete same-context workload sweep:

```sh
export SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER="exact adapter name"
export SYMTROPY_DEMO_CAPTURE_GPU_DRIVER="exact driver/version"
export SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND="Vulkan"
bash scripts/run-render-benchmark-sweep.sh
```

The sweep runs object counts 64, 144, 256, 512, 1,024, 2,048, 4,096, 8,192 and 10,000 using one chosen resolution profile and shadow setting. Each run has its own capture/report directory; `summary.json` is written only if all reports are metadata-qualified, have at least 30 samples, and match on fixture digest, source revision, camera, renderer, profile, resolution, shadows, adapter, driver/backend, OS, toolchain, and sampling window. Render-settings digests are allowed to differ because the object count is intentionally changing.

The summary is sorted by object count and reports p50/p95/p99, frame intervals over 16.67 ms and 33.3 ms, ratio of p95 to the smallest workload, and whether each p95 meets the candidate 60 FPS / 30 FPS budgets. It does not mark a scaling curve as good or bad by itself, and it does not claim GPU timestamp timings.

## Frame-time report

The runner collects Bevy `Time<Real>::delta_secs()` in milliseconds from 1.0 to 8.0 seconds after app startup and writes `frame-time.json` in the capture directory. It includes sample count, mean, p50/p95/p99, the observed fraction of frame intervals above 16.67 ms and 33.3 ms, profile/workload, and the caller-declared adapter, driver, backend, OS, toolchain, renderer, fixture and render-settings identities.

**Important measurement boundary:** these are observed Bevy real-frame intervals, not GPU timestamp-query durations and not isolated CPU system costs. They include effects such as scheduling and display synchronization. The report explicitly labels the method and claim ceiling. Compare runs on the same adapter/backend with the same capture settings. For true GPU time or CPU/GPU overlap, add backend timestamp-query instrumentation as a separate measurement lane.

The runner removes an old frame-time report before launching the app and fails if a fresh, non-empty report is not produced. At least 30 sampled frame intervals are required; smaller samples are reported as `insufficient_samples` and make the runner exit nonzero. Metadata omissions downgrade the report to `measured_metadata_incomplete` rather than hiding the measurement.

## Profiles and scaling

| Profile | Resolution | Default objects | Sun shadows |
|---|---:|---:|---|
| `quality-1080p` | 1920×1080 | 256 | On |
| `balanced-1080p` | 1920×1080 | 144 | On |
| `low-power-720p` | 1280×720 | 64 | Off |

Override the workload with `SYMTROPY_RENDER_BENCH_OBJECTS` (1–10,000) and shadow mode with `SYMTROPY_RENDER_BENCH_SHADOWS=on|off`. Any override changes the quality-profile identity or settings digest and should be treated as a distinct benchmark configuration.

For a stress run:

```sh
SYMTROPY_RENDER_BENCH_PROFILE=balanced-1080p \
SYMTROPY_RENDER_BENCH_OBJECTS=10000 \
SYMTROPY_RENDER_BENCH_SHADOWS=off \
scripts/run-render-benchmark.sh
```

Do not compare images or frame times from different configurations as though they were the same workload.

## Benchmark targets and interpretation

Use release builds. Record p50/p95/p99 frame time, CPU/GPU time where observable, VRAM, dropped frames, shader/asset failures, and capture receipts alongside the image-difference report. The proposed 16.7 ms p95 target at 1080p and 33.3 ms p95 at 720p are initial goals—not measured results or guarantees.

This one fixture initially isolates lighting/material and draw workload. It does not replace separate human-animation, city-scale/LOD, physics-observability, or 4D-slice fixtures. Raw PNG hashes verify byte identity, and the companion diff tool computes code-value differences; neither is a perceptual quality score or evidence of physical correctness.
