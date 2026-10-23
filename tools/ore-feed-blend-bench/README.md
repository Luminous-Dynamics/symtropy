# Ore Feed Blend Benchmark

Standalone Rust validator for a frozen synthetic ore-feed blend corpus.

From repository root:

    cargo fmt --manifest-path tools/ore-feed-blend-bench/Cargo.toml -- --check
    cargo test --manifest-path tools/ore-feed-blend-bench/Cargo.toml
    cargo run --quiet --manifest-path tools/ore-feed-blend-bench/Cargo.toml -- \
      docs/tech/ore-feed-blend-corpus-v0.1.json \
      --source-commit "$(git rev-parse HEAD)"

The bounded exhaustive search is a small-instance reference oracle, not a production mine planner. Analyte IDs are exact: copper-grade evidence cannot satisfy a zinc target, and contaminant analytes cannot be silently substituted. All fixture values are synthetic; the crate does not predict real recovery, comminution, economics, or physical performance.
