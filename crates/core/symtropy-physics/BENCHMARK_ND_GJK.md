# N-D GJK benchmark protocol

This benchmark is a repeatable within-engine latency baseline for the dimension-generic GJK implementation. It is not, by itself, an industry comparison or a SOTA claim.

## Run

```sh
cargo bench --locked -p symtropy-physics --bench gjk_nd_bench
```

Use a release build on an otherwise idle machine. Record the full source SHA, compiler output from rustc -Vv, cargo version, target triple, CPU model, operating system/kernel, power mode, and raw Criterion output. Keep workload setup outside the timed loop. The pseudo-random generator and seed are fixed by dimension.

## Matrix

Dimensions 3 and 4 are specialized-path controls. Dimensions 5, 6, 8, and 16 exercise the generic active-set path. Each dimension includes:
- rotated_overlap: independently oriented boxes whose centres are close enough to guarantee overlap from their inscribed-ball lower bound.
- rotated_separated: boxes with a large centre offset that guarantees separation.

## Required interpretation

- Benchmark only after conformance correctness passes; report indeterminate outcomes as uncertainty, not as successful separation.
- Compare the same shape pairs and semantics across implementations.
- Report per-dimension measurements rather than pooling them into one average.
- Capture support evaluations, GJK iterations, active-set pivots, allocation counts, and p50/p95/p99 latency in the follow-on raw-sample campaign. Criterion's summary alone is not a substitute for those per-query metrics.
- Industry comparisons should use matched 3D workloads against Rapier/Parry and Jolt, with pinned versions, identical inputs, release builds, same hardware, and the same timing boundary. Report the N-D experiments separately because mainstream 3D physics-engine results do not validate higher-dimensional semantics.

## Literature baselines to evaluate (not implied wins)

- [Montanari, Petrinic and Barbieri (2017), *Improving the GJK algorithm for faster and more reliable distance queries*](https://doi.org/10.1145/3072959.3083724): a distance subalgorithm aimed at degenerate geometries reports 15–30% faster contact queries on its evaluated workloads. Use it as a robustness/performance baseline, not as an N-D validation result.
- [Montaut et al. (RSS 2022), *Collision Detection Accelerated: An Optimization Perspective*](https://arxiv.org/abs/2205.09663): derives GJK as a Frank–Wolfe special case and reports up to 2× speedups for accelerated variants on its ShapeNet/collision-pair workloads. The gain is workload-dependent and should be re-evaluated under Symtropy's correctness gates.
- [Performance optimization of GJK collision detection in discrete-element simulations (Computer Physics Communications, 2025)](https://doi.org/10.1016/j.cpc.2025.109768): reports that bounding-volume choices, shape aspect ratios and temporal coherence affect the outcome; Nesterov acceleration is not universally beneficial.
- [Convex Body Collision Detection Using the Signed Distance Function (Computer-Aided Design, 2024)](https://doi.org/10.1016/j.cad.2024.103685): presents an ellipsoid-method approach for SDF-represented bodies and compares it with GJK/MPR. This is a complementary algorithm lane for shapes with a signed-distance representation, not a drop-in support-map competitor.

The field is not waiting for "GJK works in N-D" as a new theorem. The differentiating research opportunity is a dimension-generic, numerically fail-closed implementation with independent oracles, scale/degeneracy coverage, explicit uncertainty, and honest cost curves beyond 3D.

## Claim ceiling

This benchmark isolates narrow-phase query latency for these synthetic box workloads. It does not measure broad-phase scaling, full-world step time, contact resolution fidelity, arbitrary convex shapes, EPA accuracy, or cross-platform determinism.
