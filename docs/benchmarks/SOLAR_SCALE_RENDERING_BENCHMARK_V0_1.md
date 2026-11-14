# Solar-Scale Rendering Benchmark v0.1

**Status:** Research contract; targets are proposed, not measured  
**Scope:** Planetary-to-system-scale navigation, apparent-size representation selection, large-world coordinates and viewport rendering  
**Claim ceiling:** Specification only until exact-head results and a reproducible receipt exist

## Why this benchmark exists

A solar-system viewer is not qualified by a screenshot or average FPS. The hard problem is preserving spatial meaning while traversing many orders of magnitude without projection error, representation thrash, identity loss, precision drift or unbounded scene costs.

The paradigm shift is to treat rendering as a **perceptual and numerical contract with a resource budget**, not merely a draw loop:

1. **Scientific geometry:** positions and apparent sizes agree with independent reference calculations.
2. **Semantic continuity:** changing representation cannot erase identity, selection, labels or task-critical content.
3. **Temporal stability:** small camera movements do not cause rapid LOD oscillation or visible jumps.
4. **Bounded cost:** a huge catalog does not imply every record becomes a live render entity.
5. **Reproducibility:** comparisons bind source, fixtures, toolchain, device settings and raw samples.

This contract sits alongside the scale-policy, hierarchy-selection and solar-scale-viewer implementation. It does not claim those features have already met its proposed targets.

## 1. Deterministic workload matrix

Synthetic correctness fixtures must stay separate from real astronomical/catalog data. A synthetic pass is not evidence that an ephemeris or real catalog is accurate.

| ID | Workload | Coverage | Required outputs |
|---|---|---|---|
| GEO-01 | Projection oracle | Perspective and orthographic projection; radius, distance and FOV boundaries; near-limb views; invalid numeric inputs | Absolute pixel-diameter error; rejected-input counts |
| SCALE-02 | Multi-decade scale sweep | Scripted approach/retreat through local-body, planetary, inner-system and outer-system scales | Position/projection error; transition locations; identity continuity |
| LOD-03 | Boundary jitter | Cross every representation threshold; oscillate just inside/outside it; reverse direction | Transitions per object; flicker/churn duration; area discontinuity |
| ID-04 | Semantic persistence | Focused, selected, labelled and task-critical bodies crossing each band | Lost identities, selections, labels and protected objects; expected zero |
| CATALOG-05 | Sparse visibility | 10k, 100k and 1M catalog records; fixed visible fixtures at 1%, 0.1% and 0.01% | CPU/GPU frame time; visible instances; culling/LOD cost; allocations |
| PREC-06 | Large-world precision | Planetary, AU and outer-system magnitudes; origin rebasing and round trips | Absolute/relative position error; screen-space drift; NaN/Inf counts |
| REC-07 | Replay determinism | Replay identical camera/control traces 100 times from the same source and fixture digest | Semantic state/receipt digest agreement; timing distribution |
| DEG-08 | Degraded device | CPU/software rendering or constrained GPU budget, low memory and resize events | Graceful degradation; bounded memory; no semantic loss |

## 2. Fair comparison method

### Correctness rules

- Use an independent reference implementation for projection and coordinate checks. It must not call the production helper being tested.
- Measure screen-space quantities in physical render pixels, not logical UI points.
- Record invalid, unsupported and unknown cases. Never silently remove them from the denominator.
- Measure both absolute and relative error. Relative error is unstable near zero; absolute error alone can hide large-scale drift.
- Rebasing may change local coordinates but must preserve canonical object identity and world location within the declared tolerance.
- A selected/task-critical object may use a labelled proxy when too small to resolve; apparent size is not an existence or relevance predicate.
- Replay determinism applies to semantic outputs, not wall-clock timings.

### Performance rules

- Fix camera paths, catalog, visible set, resolution, quality settings, random seed, backend and asset-cache policy.
- Separate cold-start/streaming cost from steady state. Warm up before measurement.
- On each reference device, collect at least 5 warm-ups and 101 measured frames per scene/path. Publish p50, p95 and p99 plus raw samples.
- Report CPU frame time, GPU frame time where available, culling/LOD time, draw/instance counts, peak resident memory and GPU memory. FPS alone is insufficient.
- Use identical workload and quality settings for baselines. Faster is not better if visual quality or semantic content silently changed.
- Preserve exact source commit, dirty-tree status, fixture digest, renderer/backend, OS, compiler, GPU/driver, resolution, power mode and command line in every receipt.
- CI without a real GPU can qualify schema, deterministic CPU references and invariants. It must not emit a GPU-performance pass when timing is missing or software-rendered.

### Baselines

1. **Independent reference:** straightforward double-precision projection and coordinate calculations with fixed fixtures.
2. **Current Symtropy renderer:** same scene semantics and settings; measure progress between commits.
3. **Naive full-catalog diagnostic:** submit every record instead of culling/selecting representations to make the cost of missing scalability visible.
4. **External context:** SPECviewperf 15.0.1 offers standardized professional-graphics workloads, but its application-specific scores are not directly comparable to a solar viewer. Published global-illumination or Gaussian-splatting results are not direct competitors unless hardware, scene, quality and workload match.

Never claim “SOTA” from heterogeneous baselines. Make scoped claims, such as lower p95 CPU culling time than a baseline on a named fixture/device while preserving semantic state and screen-space error.

## 3. Proposed acceptance targets

These are local engineering targets to test, not current results or universal standards. Freeze each target only after a measured baseline establishes that it is meaningful on target hardware.

| Gate | Proposed target | Failure disposition |
|---|---|---|
| GEO-01 projection | p99 absolute projected-diameter error <= 0.5 physical pixel for resolved body proxies; invalid numeric inputs rejected | Fail correctness; don't compensate with artistic scale factors |
| SCALE-02 geometry | p99 screen-space drift <= 0.5 pixel for fixed world points across origin rebases at the same camera pose | Fail precision gate |
| LOD-03 stability | No identity/selection loss; <=1 unintended representation reversal per object in a 120-frame bounded-jitter trace | Fail temporal gate |
| ID-04 persistence | Zero lost selected/task-critical identities and zero silent object deletion across bands | Fail closed |
| CATALOG-05 frame budget | p95 total frame time <= 16.7 ms at 2560x1440 on the declared reference GPU for 100k catalog / 1k visible; p99 <= 33.3 ms | Record target miss, preserve correctness |
| CATALOG-05 CPU budget | p95 culling + representation selection <= 4 ms for 100k catalog on declared reference CPU | Profile before optimizing |
| Memory | No unbounded growth during a 10-minute deterministic navigation replay; report peak resident/GPU bytes and asset churn | Fail if growth lacks a bounded-cache explanation |
| REC-07 repeatability | 100/100 replays agree on canonical semantic digest and fixture/source identity | Fail reproducibility gate |
| DEG-08 fallback | Missing GPU timing is marked unavailable; CPU/software mode maintains semantics and makes no GPU-parity claim | Fail closed on misleading receipts |

The timing budgets are provisional. Report each device/backend independently rather than blending weak and strong devices into one number. On RTX 2070 Laptop GPU-class development hardware, record a pinned local profile when available; hosted CI results are not a substitute.

## 4. Required receipt

Every receipt should include:

- Source commit, dirty-tree status, toolchain, target triple and build profile
- Fixture SHA-256, scenario ID, replay-trace digest and exact command
- Device, renderer backend, driver, OS, resolution, refresh/power settings and warm-up count
- Catalog, visible, selected and draw-instance counts
- Projection error in pixels p50/p95/p99 and large-world error in metres p50/p95/p99
- LOD transition count, unintended reversal count and identity-loss count
- CPU frame time p50/p95/p99 and GPU frame time p50/p95/p99 (GPU values may be unavailable only when explicitly marked unavailable)
- Peak resident bytes, peak GPU memory bytes and allocation growth
- Pass/fail/unknown per workload and a claim ceiling distinguishing synthetic correctness from measured hardware performance

A missing metric must never serialize as zero. Preserve raw samples so a percentile or outlier can be independently recomputed.

## 5. How to beat the state of the art

The differentiator should be **quality-constrained scalability**, not one high-FPS number:

1. Derive the smallest live render set that preserves all visible, selected and task-critical information.
2. Combine apparent-size bands with screen-space error, temporal hysteresis and semantic protection. Size alone is insufficient for scientific, educational or navigational relevance.
3. Use origin rebasing and precision-aware transforms, independently checked by numeric oracles.
4. Instrument LOD, culling and streaming decisions, then optimize the measured bottleneck.
5. Compare quality/performance Pareto frontiers. A configuration is not better if it is faster only because it is less accurate or silently omits content.
6. Treat accessibility and interaction stability as correctness constraints.

The first success criterion is a trusted baseline with zero semantic regressions. The second is a measured improvement within the same quality envelope.

## 6. Research anchors

- SPECviewperf 15.0.1: https://gwpg.spec.org/benchmarks/benchmark/specviewperf-15_0_1/ — useful standardized professional-graphics methodology, not a directly comparable solar-viewer score.
- Dynamic Global Illumination for Interactive Gaussian Splatting Scenes in Real Time (2026): https://hineven.github.io/3DGS_GI/ — scene-, implementation- and hardware-specific results; a useful example of workload breakdown and explicit limitations, not a direct numeric target.
- Radiance Cascades Compared to Unity's Standard Global Illumination Methods (2026): https://dl.digra.org/index.php/dl/article/view/2775 — controlled camera paths, dynamic events and artifact analysis are useful evaluation practices.

## 7. Future harness completion criteria

This document does not itself run or qualify the workloads. The executable harness should be a separate narrow Rust crate with:

- Deterministic fixtures plus exact-reference and mutation/negative-control tests
- Machine-readable schema validation and fail-closed handling of missing metrics
- A command that emits a receipt bound to source SHA, fixture digest and environment metadata
- Separate CPU correctness and hardware-performance lanes
- No result promotion across source SHAs, fixtures, renderer backends or hardware profiles
- Frozen baselines and artifact retention sufficient to reproduce each comparison

A result may claim benchmark performance only after its exact-head receipt and raw artifact exist. Until then, every target above remains proposed.
