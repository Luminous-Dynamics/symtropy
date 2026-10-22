# Solar-Scale Rendering Benchmark v0.1

**Status:** research contract; proposed targets, not measured results  
**Scope:** planetary-to-system-scale navigation, apparent-size representation selection, large-world coordinates and viewport rendering  
**Claim ceiling:** specification only until an exact-head harness runs and preserves its receipt

## Why this benchmark exists

A solar-system viewer is not qualified by a screenshot or average FPS. The hard problem is preserving spatial meaning while moving between scales that differ by many orders of magnitude, without projection errors, representation thrash, identity loss, precision drift or unbounded scene costs.

The paradigm shift is to treat rendering as a **perceptual and numerical contract with a resource budget**, rather than an isolated draw loop. A good result must satisfy all of these simultaneously:

1. **Scientific geometry:** position and apparent size agree with independent reference calculations.
2. **Semantic continuity:** changing representation cannot erase identity, selection, labels or task-critical content.
3. **Temporal stability:** small camera movements do not cause rapid LOD oscillation or visible jumps.
4. **Bounded cost:** an enormous catalog must not imply that every record becomes a live render entity.
5. **Reproducibility:** every comparison binds exact source, scene fixture, toolchain, device, settings and raw samples.

This contract is intended to sit alongside the existing scale-policy, hierarchy-selection and solar-scale-viewer implementation. It does not claim those features have already met the proposed performance targets.

## 1. Workload matrix

All fixture inputs must be deterministic and content-addressed. Keep synthetic correctness fixtures separate from real astronomical/catalog data; a synthetic fixture is not evidence that an ephemeris or real catalog is accurate.

| ID | Workload | Required coverage | Primary outputs |
|---|---|---|---|
| GEO-01 | Projection oracle | Perspective and orthographic projection; radius/distance/FOV boundaries; near-limb views; invalid numeric inputs | Absolute pixel-diameter error; rejected-input counts |
| SCALE-02 | Multi-decade scale sweep | Scripted approach/retreat from local-body, planetary, inner-system and outer-system scales | Position/projection error; transition locations; identity continuity |
| LOD-03 | Boundary jitter | Cross each representation threshold; oscillate just inside/outside it; reverse direction | Transitions per object; flicker/churn duration; area discontinuity |
| ID-04 | Semantic persistence | Focused, selected, labelled and task-critical bodies crossing every representation band | Lost identities, selections, labels and protected objects; expected zero |
| CATALOG-05 | Sparse visibility | Catalog sizes of 10k, 100k and 1M records with fixed visible-set fixtures of 1%, 0.1% and 0.01% respectively | CPU/GPU frame time; visible instances; culling/LOD cost; allocations |
| PREC-06 | Large-world precision | Positions spanning planetary, astronomical-unit and outer-system magnitudes; camera-origin rebasing and round trips | Absolute/relative position error; screen-space drift; NaN/Inf counts |
| REC-07 | Replay determinism | Replay identical camera/control traces 100 times from the same source and fixture digest | Semantic state/receipt digest agreement; timing distribution |
| DEG-08 | Degraded device | Software/CPU renderer or constrained GPU budget, low memory and resize events | Graceful degradation; bounded memory; no semantic loss |

## 2. Reference method and anti-cheating rules

### Correctness

- Use an independent reference implementation for projection and coordinate checks. It must not call the production helper being tested.
- Compare screen-space measurements in physical render pixels, not logical UI points.
- Record invalid, unsupported and unknown cases explicitly. Do not silently drop them from the denominator.
- Test both absolute and relative errors. Relative error becomes misleading near zero; absolute error alone can hide large-scale drift.
- Camera-origin rebasing may change local coordinates, but must not alter canonical object identity or world-space location beyond the stated tolerance.
- A selected or task-critical object may use a labelled proxy when too small to resolve; apparent size is not an existence or relevance predicate.
- Replay checks compare stable semantic state and deterministic outputs. Wall-clock timings may vary and must not be part of the deterministic digest.

### Performance

- Fix the camera path, object catalog, visible set, resolution, quality settings, random seed, backend and asset-cache policy.
- Separate cold-start/streaming from steady state. Warm up before measurement.
- On each reference device, collect at least 5 warm-ups and 101 measured frames per scene/path; publish p50, p95 and p99 plus raw samples.
- Report CPU frame time, GPU frame time where available, LOD/culling time, draw/instance counts, peak resident memory and GPU memory. FPS alone is insufficient.
- Run the same workload and quality mode against each baseline. An optimized implementation cannot reduce work or visual quality without reporting the difference.
- Preserve exact source commit, dirty-tree status, fixture digest, renderer/backend, OS, compiler, GPU/driver, resolution, power mode and command line in every receipt.
- CI without a real GPU can qualify schema, deterministic CPU references and invariants. It must not emit a GPU-performance PASS by substituting software rendering or a missing timing value.

### Baselines

1. **Reference correctness implementation:** straightforward double-precision projection and coordinate calculations with independent fixtures.
2. **Current Symtropy implementation:** identical semantics and workload settings; measure progress between commits.
3. **Naive full-catalog baseline:** diagnostic only, showing the cost of submitting every record instead of culling and representation selection.
4. **External context:** SPECviewperf 15.0.1 is a useful professional-graphics workload reference, but its application-specific scores are not directly comparable with a solar viewer. Published global-illumination or Gaussian-splatting results are also not direct competitors unless hardware, image quality, scene, backend and workload are equivalent.

Never report an across-the-board SOTA claim from heterogeneous baselines. Make narrow claims such as “lower p95 CPU culling time than the baseline on this fixture/device, with identical semantic state and bounded screen-space error.”

## 3. Proposed acceptance targets

These numbers are **engineering targets to test**, not current results or universal industry standards. Freeze them only after a measured baseline establishes that each is meaningful for the target device.

| Gate | Proposed target | Failure disposition |
|---|---|---|
| GEO-01 projection | p99 absolute projected-diameter error <= 0.5 physical pixel for resolved body proxies; invalid numeric inputs rejected | Fail correctness; do not compensate with artistic scale factors |
| SCALE-02 geometry | p99 screen-space drift <= 0.5 pixel for fixed world points across origin rebases at the same camera pose | Fail precision gate |
| LOD-03 stability | No identity/selection loss; <=1 unintended representation reversal per object during a 120-frame bounded-jitter trace | Fail temporal-stability gate |
| ID-04 semantic persistence | 0 lost selected/task-critical identities and 0 silent object deletion across all bands | Fail closed |
| CATALOG-05 frame budget | p95 total frame time <= 16.7 ms at 2560x1440 on the declared reference GPU for the 100k-catalog / 1k-visible case; p99 <= 33.3 ms | Record miss; preserve correctness |
| CATALOG-05 CPU budget | p95 culling + representation selection <= 4 ms for the 100k catalog on the declared reference CPU | Profile before optimizing |
| Memory | No unbounded growth across a 10-minute deterministic navigation replay; report peak resident/GPU bytes and asset churn | Fail if growth lacks a declared bounded-cache explanation |
| REC-07 repeatability | 100/100 replays agree on canonical semantic digest and fixture/source identity | Fail reproducibility gate |
| DEG-08 fallback | Missing GPU timing is marked unavailable; CPU/software mode must maintain semantics and must not claim GPU parity | Fail closed on misleading receipts |

The p95/p99 timing goals are provisional local budgets. Report each device/backend independently rather than blending weak and strong devices into one number. For RTX 2070 Laptop GPU-class development hardware, include a pinned local profile when available; CI results are not a substitute for that profile.

## 4. Reported metrics

Every benchmark receipt should include:

- Source commit, dirty-tree status, toolchain, target triple and build profile
- Fixture SHA-256, scenario ID, replay-trace digest and exact command
- Device, renderer backend, driver, OS, resolution, refresh/power settings and warm-up count
- Catalog count, visible count, selected count and draw/instance count
- Projection error in pixels p50/p95/p99 and large-world error in metres p50/p95/p99
- LOD transition count, unintended reversal count and identity-loss count
- CPU frame time p50/p95/p99 and GPU frame time p50/p95/p99 (GPU values may be unavailable only when explicitly marked unavailable)
- Peak resident bytes, peak GPU memory bytes and allocation growth
- Pass/fail/unknown per workload and a claim ceiling distinguishing synthetic correctness from measured hardware performance

A missing metric must never be serialized as zero. Preserve raw samples alongside aggregates so a percentile or outlier claim can be independently recomputed.

## 5. Work to beat the state of the art

The differentiator should be **quality-constrained scalability**, not a single high-FPS result:

1. Derive the smallest live render set that preserves all visible, selected and task-critical information.
2. Combine apparent-size bands with screen-space error, temporal hysteresis and semantic protection. Size alone is insufficient for scientific, educational or navigational importance.
3. Use origin rebasing and precision-aware transforms with independent numeric oracles.
4. Make LOD, culling and streaming decisions observable, then optimize the measured bottleneck.
5. Compare quality/performance Pareto frontiers: a configuration must not be called faster if it is materially less accurate or silently omits content.
6. Treat accessibility and interaction stability as correctness constraints, not post-benchmark polish.

The first success criterion is a trusted, repeatable baseline with zero semantic regressions. The second is a measured improvement within the same quality envelope.

## 6. Research anchors

- SPECviewperf 15.0.1: https://gwpg.spec.org/benchmarks/benchmark/specviewperf-15_0_1/. Useful for standardized professional-graphics methodology, not a directly comparable solar-viewer score.
- Dynamic Global Illumination for Interactive Gaussian Splatting Scenes in Real Time (2026): https://hineven.github.io/3DGS_GI/. Its reported results are scene-, implementation- and hardware-specific; use it as an example of workload breakdowns and explicit limitations, not as a direct numeric target.
- Radiance Cascades Compared to Unity's Standard Global Illumination Methods (2026): https://dl.digra.org/index.php/dl/article/view/2775. Relevant for controlled camera paths, dynamic events and artifact analysis when designing image-quality comparisons.

## 7. Completion criteria for a future harness

This document does not itself run or qualify the workloads. The executable harness should be a separate, narrow Rust crate with:

- Deterministic fixtures plus exact-reference and mutation/negative-control tests
- Machine-readable schema validation and fail-closed handling of missing metrics
- A command that emits a receipt bound to source SHA, fixture digest and environment metadata
- Separate CPU correctness and hardware-performance lanes
- No result promotion across source SHAs, fixtures, renderer backends or hardware profiles
- Frozen baselines and artifact retention sufficient to reproduce every reported comparison

A result may claim benchmark performance only after its exact-head receipt and raw artifact exist. Until then, all targets above remain proposed.
