# Furnishing adversarial workload baseline on v0.2 brute-force oracle

This measurement-only branch adds the v0.4 stress benchmark to the pre-BVH v0.2 geometry oracle without changing the geometry implementation. The focused workflow now builds and runs both the v0.2 baseline at this PR head and the pinned v0.3 indexed candidate commit `a6fadac278a7005c61ff92626ed41e600a3fdb40` sequentially on the **same Ubuntu 24.04 runner host**. Both execute the exact same benchmark source, workload construction, sizes, iteration counts and expected collision-pair truth. The output is grouped by implementation and includes machine provenance.

Run at the repository root:

    cargo fmt --manifest-path tools/furnishing-layout-audit/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml
    cargo clippy --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --all-targets -- -D warnings
    cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_stress

It covers sparse, tight-functional-clearance and dense-footprint-overlap cases at 25, 100, 1,000 and 10,000 items; the 10,000-item tier intentionally uses one sample per workload to keep the brute-force baseline bounded. The dense test asserts the same independently grid-derived collision-pair count. Because baseline and optimized versions run sequentially on the same host, this gives a stronger comparator than separate GitHub-hosted runs, though one 10k sample remains diagnostic rather than a percentile estimate.

This branch is a benchmark comparator only and should not be merged as a product change. No speedup is claimed until both v0.2 and v0.3 exact-head runs finish with correct counts and their machine details are recorded. The 30-minute cap censors an overlong tier; it must not be turned into a fabricated timing value.


## Matched same-host result — run #39481984586

The baseline workflow now ran v0.2 and the pinned indexed candidate `a6fadac278a7005c61ff92626ed41e600a3fdb40` sequentially on the **same Ubuntu 24.04 runner** (4 vCPUs, AMD EPYC 9V45, 15 GiB reported RAM, Rust 1.96.0). Both used the same benchmark source blob `731d7d4a29d00596bf93cdc30a9519d377d17a55`, the same generated layouts and identical collision truth. See the [workflow run](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39481984586) and [full JSON receipt](evidence/FURNISHING_GEOMETRY_AB_RUN_39481984586.json).

| Workload | Items | v0.2 brute-force | v0.3 indexed | Diagnostic latency ratio | Expected/observed collisions in both |
|---|---:|---:|---:|---:|---:|
| Sparse grid | 25 | 0.198 ms p95 | 0.076 ms p95 | 2.61× | 0/0 |
| Sparse grid | 100 | 2.531 ms p95 | 0.342 ms p95 | 7.40× | 0/0 |
| Sparse grid | 1,000 | 312.496 ms p95 | 4.656 ms p95 | 67.11× | 0/0 |
| Sparse grid | 10,000 | 50,137.250 ms, one sample | 71.502 ms, one sample | 701.20× | 0/0 |
| Tight task clearance | 25 | 0.157 ms p95 | 0.068 ms p95 | 2.31× | 0/0 |
| Tight task clearance | 100 | 1.851 ms p95 | 0.339 ms p95 | 5.46× | 0/0 |
| Tight task clearance | 1,000 | 222.281 ms p95 | 4.414 ms p95 | 50.34× | 0/0 |
| Tight task clearance | 10,000 | 34,611.570 ms, one sample | 69.288 ms, one sample | 499.53× | 0/0 |
| Dense footprint overlap | 25 | 0.158 ms p95 | 0.092 ms p95 | 1.72× | 72/72 |
| Dense footprint overlap | 100 | 1.850 ms p95 | 0.404 ms p95 | 4.58× | 342/342 |
| Dense footprint overlap | 1,000 | 214.408 ms p95 | 5.113 ms p95 | 41.94× | 3,811/3,811 |
| Dense footprint overlap | 10,000 | 35,571.205 ms, one sample | 77.805 ms, one sample | 457.18× | 39,402/39,402 |

All stress rows reported zero unexpected findings. The 10,000-item rows use one sample per implementation, so the ratios are **single-sample latency ratios, not p95 speedups**. On the measured generated workloads, the indexed oracle is much faster while preserving the exact expected collision counts. This is meaningful evidence for this code/workload pair, not a universal “better than SOTA” claim or a production SLA. The implementations were run sequentially, so thermal/cache/order effects remain possible; memory peak and realistic/held-out room cases remain unmeasured.

