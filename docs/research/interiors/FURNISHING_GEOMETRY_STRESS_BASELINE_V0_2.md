# Furnishing adversarial workload baseline on v0.2 brute-force oracle

This measurement-only branch adds the v0.4 stress benchmark to the pre-BVH v0.2 geometry oracle without changing the geometry implementation. The focused workflow now builds and runs both the v0.2 baseline at this PR head and the pinned v0.3 indexed candidate commit `a6fadac278a7005c61ff92626ed41e600a3fdb40` sequentially on the **same Ubuntu 24.04 runner host**. Both execute the exact same benchmark source, workload construction, sizes, iteration counts and expected collision-pair truth. The output is grouped by implementation and includes machine provenance.

Run at the repository root:

    cargo fmt --manifest-path tools/furnishing-layout-audit/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml
    cargo clippy --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --all-targets -- -D warnings
    cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_stress

It covers sparse, tight-functional-clearance and dense-footprint-overlap cases at 25, 100, 1,000 and 10,000 items; the 10,000-item tier intentionally uses one sample per workload to keep the brute-force baseline bounded. The dense test asserts the same independently grid-derived collision-pair count. Because baseline and optimized versions run sequentially on the same host, this gives a stronger comparator than separate GitHub-hosted runs, though one 10k sample remains diagnostic rather than a percentile estimate.

This branch is a benchmark comparator only and should not be merged as a product change. No speedup is claimed until both v0.2 and v0.3 exact-head runs finish with correct counts and their machine details are recorded. The 30-minute cap censors an overlong tier; it must not be turned into a fabricated timing value.
