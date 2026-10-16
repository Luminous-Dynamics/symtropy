# Static-heavy broadphase crossover benchmark

Status: deterministic benchmark harness committed; **no performance result is claimed until this exact branch is executed on a recorded host/toolchain**.

## Run

```sh
cargo bench -p symtropy-physics --bench physics_bench -- broadphase_static_heavy
```

The harness runs an exact pair-set comparison before emitting each Criterion case. A mismatch, duplicate pair, or non-normalized pair aborts the benchmark instead of publishing timings.

## Matrix

The deterministic fixture matrix contains eight scenes:

- Layout: uniform grid or clustered static geometry.
- Static bodies: 1,000 or 10,000.
- Dynamic bodies: 8 or 32, keeping this workload below the existing 50-active-body crossover.
- Dynamics are placed to overlap selected static bodies. Both the full LBVH and incremental implementations receive the same complete body vector.
- The cache is prebuilt for the steady-state incremental timing. A separate case measures rebuilding the static cache from the current bodies.

Each fixture measures three operations separately: full-LBVH pair generation, incremental pair generation with the existing cache, and static-cache rebuild. The benchmark does not hide cache construction inside the steady-state query measurement.

## Correctness contract

Before timing, compare the canonical full-LBVH and incremental pair sets exactly. Pairs must be normalized and unique. No timing result is published for a fixture whose candidate pairs differ.

This fixture intentionally starts with Static + Dynamic bodies only. Kinematic-pair semantics have a separate correctness repair under review in [PR #1697](https://github.com/Luminous-Dynamics/symtropy/pull/1697); extend the timing matrix to kinematics after that fix is integrated and independently validated.

## How to interpret results

- The comparison is between rebuilding the full LBVH each call and reusing a static cache while recalculating dynamic AABBs.
- The static-cache rebuild case must be accounted for using the actual invalidation frequency. For example, with an average of `F` frames between cache rebuilds, estimate incremental amortized cost as `steady_state_cost + rebuild_cost / F`. Do not count a cache as free.
- Grid and clustered scenes are deliberately different. A single winner should not be generalized to arbitrary scenes.
- Criterion timing distributions are per-case microbenchmark statistics; these cases do not yet report end-to-end frame p95/p99, peak memory, broadphase node visits, collision narrowphase calls, or moving/deforming traces.
- Record the exact commit, Rust toolchain, CPU, OS, build profile, raw Criterion reports, scene counts, and benchmark command. Compare implementations on the same host.

## Proposed acceptance gates

These are project targets, not published industry standards:

1. **Correctness:** exact pair-set equivalence for every fixture; zero missing and zero extra candidates relative to the reference path.
2. **Useful static-heavy improvement:** target at least 15% lower *amortized* broadphase cost in at least two predeclared grid/clustered scenarios, after charging cache rebuild cost at a declared invalidation rate.
3. **Regression budget:** no more than 5% regression on any predeclared critical scenario without an explicitly accepted trade-off.
4. **Decision quality:** if incremental scanning loses on clustered or large-static workloads, keep the full LBVH path for those regimes and consider a spatially indexed static cache rather than asserting that caching alone is faster.
5. **Generalization:** before changing the default crossover, add held-out movement traces, sleeping/waking, kinematic motion, insertion/removal, collision masks, and frame-step measurements.

The paradigm here is **policy selection backed by measured workloads and exact semantic equivalence**. Reusing data is only an optimization when its maintenance cost and query cost are both accounted for.
