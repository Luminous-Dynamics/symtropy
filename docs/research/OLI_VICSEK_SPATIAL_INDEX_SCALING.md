# OLI Vicsek spatial-index scaling benchmark v0

**Status:** benchmark contract frozen; harness, exact-head Rust run and hardware measurements pending  
**Related implementation:** [Symtropy PR #4670](https://github.com/Luminous-Dynamics/symtropy/pull/4670)  
**Tracking issue:** [OLI-PERF-001 #4739](https://github.com/Luminous-Dynamics/symtropy/issues/4739)  
**Machine-readable manifest:** `benchmarks/oli/vicsek-spatial-index-scaling-v0.json`

## Why the density parameterization matters

A misleading scaling benchmark can be created by increasing particle count while also increasing the average number of neighbours per particle. That changes both N and the physical workload.

The primary track fixes expected local degree approximately by expressing interaction radius in units of mean inter-particle spacing:

[
ell = rac{1}{sqrt{ho}},qquad
L = sqrt{rac{N}{ho}},qquad
R = kell,
]

where N is particle count, ρ is number density, L is box size and k is a registered radius-spacing factor. At fixed ρ and k, the expected local neighbourhood size is approximately constant as N grows.

A separate dense-neighbour stress regime intentionally allows the local candidate set to grow toward N so worst-case behavior is visible.

## Workload matrix

- N = 128, 256, 512, 1,024, 2,048, 4,096, 8,192, 16,384.
- Densities ρ = 0.5, 2, 8 particles per unit area.
- Interaction radius k = 1, 2, 4 mean spacings.
- Metric-radius reciprocal, forward visual-sector and rear visual-sector perception.
- Both current-position and next-position update conventions.
- Angular-noise widths 0, 0.4 and 2 radians.
- 10 independent seeds per case; 50 warm-up steps, 200 timed steps, 10 repetitions.

The dense-neighbour, pathological-fallback and skewed-occupancy regimes are specified separately in the JSON manifest.

## Timing contract

Do not report only a candidate-query microbenchmark. Record:
1. model initialization;
2. cell-index construction;
3. candidate enumeration and sorting;
4. exact visibility predicates;
5. state update and noise;
6. fast metrics;
7. full graph topology at registered checkpoints;
8. end-to-end step and fixed-length trajectory.

Retain raw samples, median, p90, p95, min/max, standard deviation, particle-updates per second, trajectory-steps per second, candidate counts, cell occupancy and fallback frequency. Pin CPU model, compiler, flags, target triple, operating system, commit SHA, power mode and thermal conditions. Do not pool results across hardware.

## Exactness contract

The cell index is only a pruning mechanism. The final minimum-image and visual-sector predicates remain authoritative. Candidate IDs are sorted so floating-point summation order is stable. The matched all-pairs reference must yield:
- exact particle state and PRNG state parity;
- exact directed-edge/reciprocity and local-order parity;
- complete candidate coverage for every brute-force radius neighbour;
- exact fallback behavior for zero/tiny radii and grid/pathological cases.

The current source includes candidate completeness tests, one-step trajectory/RNG parity for metric and visual-sector models, and all-pairs directed-metric parity tests. Those tests still require Rust fmt, compile and execution.

## Proposed targets

These are proposed targets, not measured outcomes:
- at least 5× end-to-end trajectory-step throughput improvement for N ≥ 4,096 at bounded density;
- no more than 10% regression for N ≤ 256 after crossover/fallback tuning;
- exact same-platform state parity;
- worst-case dense-neighbour regressions disclosed instead of hidden.

If measurements fail the 5× target, the benchmark should identify whether time is dominated by candidate enumeration, hash construction, sorting, exact visibility, fast metrics, or memory allocation. A faster neighbor query that leaves the actual per-tick metric cost quadratic is not a successful end-to-end optimization.

## Claim ceiling

This benchmark measures a deterministic simulation implementation. It does not establish that the model describes a natural flock, a living system, or intelligence. No speedup claim should be made until the manifest is executed on pinned hardware with raw timing samples and exactness receipts.
