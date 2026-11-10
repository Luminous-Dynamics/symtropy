# Intel Embree 4.4.1 comparator adapter

**Status:** first standalone C++ adapter implementation; not yet compiled or executed in this environment.  
**Role:** benchmark-only; the Rust production visibility evaluator does not link Embree.  
**Pinned upstream:** Intel Embree tag `v4.4.1`, commit `f590db83ef6559387df7f6d8725c34fb7acf851d`, Apache-2.0.

## Why this adapter uses an intersection filter

A simple `rtcIntersect1` call normally returns one closest hit. That is insufficient for this comparator's contract because it must preserve *all* triangles within the nearest-hit tie epsilon.

Embree's own `tutorials/next_hit/next_hit_device.cpp` demonstrates gathering every hit through an intersection filter: the filter records `ray->tfar` and primitive/geometry identity, then rejects the hit (`valid[0] = 0`) so traversal continues. This adapter follows that pattern with the Embree 4 `RTCIntersectArguments`, `RTCRayQueryContext` and argument-filter API. It deduplicates callbacks for the same primitive ID but retains distinct coincident triangle identities.

Upstream references:
- https://github.com/RenderKit/embree/tree/v4.4.1
- https://github.com/RenderKit/embree/blob/v4.4.1/tutorials/next_hit/next_hit_device.cpp
- https://github.com/RenderKit/embree/blob/v4.4.1/include/embree4/rtcore_scene.h
- https://github.com/RenderKit/embree/blob/v4.4.1/include/embree4/rtcore_geometry.h
- https://github.com/RenderKit/embree/blob/v4.4.1/include/embree4/rtcore_ray.h
- https://github.com/RenderKit/embree/blob/v4.4.1/LICENSE.txt

## Requirements

- CMake 3.20 or newer.
- C++17 compiler.
- Intel Embree **exactly 4.4.1**, built/installed with its CMake package config and public headers.
- nlohmann-json **exactly 3.11.3** CMake package.
- OpenSSL 3.0+ Crypto development package.

Embree must be built for the actual benchmark host. Record the source commit and the Embree build's ISA/build options. Do not compare an AVX-512 host-optimized build with a generic baseline and attribute the difference solely to the BVH policy.

## Build

Assume Embree and nlohmann-json have been installed to explicit prefixes. Example:

```sh
cmake -S research/scan-bim-rayquery-comparator-001/embree \
  -B build/scan-bim-embree \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH="/opt/embree-4.4.1;/opt/nlohmann-json-3.11.3"

cmake --build build/scan-bim-embree --config Release --parallel
```

The CMake project requires `embree 4.4.1 EXACT` and checks `EMBREE_VERSION`. The adapter pins the upstream source revision in the receipt. For reproducible benchmark reporting, set the adapter revision at invocation time:

```sh
export SCAN_BIM_ADAPTER_REVISION="$(git rev-parse HEAD)"
```

## Smoke tests

The standalone CMake project registers three CTest tests when `BUILD_TESTING=ON` (the default):

- `embree_known_answers`
- `embree_scale_metamorphism`
- `embree_overlap_stress`

Run them after the build:

```sh
ctest --test-dir build/scan-bim-embree --output-on-failure
```

A fixture mismatch causes the adapter to exit non-zero; CTest therefore fails rather than accepting a partial or softened result. The tests exercise the float32 Embree adapter against the f64 fixture expectations. A float32 disagreement is recorded as a mismatch and is not normalized away.

## Run the same fixtures as the f64 oracle

```sh
build/scan-bim-embree/scan_bim_embree_adapter \
  research/scan-bim-rayquery-comparator-001/fixtures/ray-query-known-answers-v0-1.json \
  --quality high --threads 1 --repetitions 1000 \
  --workload-class analytic_known_answer \
  > embree-base-receipt.json

build/scan-bim-embree/scan_bim_embree_adapter \
  research/scan-bim-rayquery-comparator-001/fixtures/ray-query-known-answers-scale-1e3-v0-1.json \
  --quality high --threads 1 --repetitions 1000 \
  --workload-class analytic_known_answer \
  > embree-scale-1e3-receipt.json

build/scan-bim-embree/scan_bim_embree_adapter \
  research/scan-bim-rayquery-comparator-001/fixtures/ray-query-stress-overlap-v0-1.json \
  --quality high --threads 1 --repetitions 1000 \
  --workload-class dense_overlap \
  > embree-overlap-receipt.json
```

Use the **same fixture bytes** and build configuration for every engine. Run low/medium/high build-quality modes as separate receipts. Compare single-thread and parallel query throughput as separate experiments; this initial adapter invokes one ray at a time, even when Embree's device is configured with multiple build/query worker threads.

## Precision and interpretation

- Embree's public `RTCRay` and triangle vertex buffers use **float32**. The reference oracle uses f64. This adapter refuses values that cannot be represented as finite f32 but does not silently widen fixture tolerances to hide precision differences.
- The fixture's metric distance epsilon and nearest-hit tie epsilon are still applied to Embree's f32 distances. Differences near a clipping/edge/tie boundary should appear as categorized mismatches and cause a `rejected` qualification.
- Build latency includes scene/geometry allocation, vertex/index population, geometry commit and scene commit.
- Per-ray p50/p95/p99 latency measures the `rtcIntersect1` invocation with the all-hit filter enabled, including callback work; the aggregate throughput wall clock also includes per-query adapter setup.
- Primitive-test and BVH-node-visit counters are **null**. Embree does not expose those internal counts through this adapter's public API; do not estimate them from hits, callbacks or timing.
- Peak and steady memory are **null** until measured with a pinned external process-memory profiler. They are not inferred from triangle count.
- This adapter measures a specific engine build, host, compiler, build quality, thread setting and fixture. It cannot establish general SOTA superiority or physical survey accuracy.

## Current validation status

The source is committed, and its interface has been reviewed against the pinned Embree 4.4.1 headers and `next_hit` tutorial. The present authoring environment has a C++ compiler and CMake, but does not have the Embree 4.4.1 or nlohmann-json development headers/libraries installed. **The adapter has not been compiled or executed here.** No correctness or performance result is reported.

Next qualification:
1. Build the exact pinned dependencies in a controlled Nix/devcontainer or CI image.
2. Compile with the documented warnings enabled; fix all diagnostics.
3. Run the three fixtures and validate the emitted JSON against `schema/engine-benchmark-receipt-v0-1.schema.json`.
4. Require exact nearest-hit identity agreement or report mismatches, especially the 24-way tie and 1,000× scale corpus.
5. Repeat measurements and verify determinism; publish p50/p95/p99, build time and host/compiler details. Memory and Embree primitive/node counts remain unavailable until separately measured/exposed.
