# habitat-revalidation-scale

Standalone Rust 2024 tool that materializes a deterministic synthetic graph for a named profile in the habitat revalidation scale matrix. It writes the exact workload bytes plus a separate SHA-256 receipt that pins the matrix digest, seed, generator version, source commit, and emitted byte count.

The generator checks artifact/task counts, dependency references, acyclicity, target reachability, task-to-artifact dependency agreement, gate references, and an independently computed pairwise Pareto frontier for its objective-alternative probe. The workload is a scale input, not an execution of the revalidation planner.

From the repository root:

- cargo fmt --manifest-path tools/habitat-revalidation-scale/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/habitat-revalidation-scale/Cargo.toml
- cargo run --locked --manifest-path tools/habitat-revalidation-scale/Cargo.toml -- docs/research/fixtures/HABITAT_REVALIDATION_SCALE_BENCHMARK_V0_1.json micro_oracle /tmp/habitat-workload.json /tmp/habitat-receipt.json <exact-40-character-source-commit-sha>

The generator requires an exact commit SHA instead of silently recording an unknown provenance value. Repeating the same invocation with the same matrix bytes and source SHA must yield byte-identical workload and receipt files.

All costs, durations, gate IDs, and graph relationships are synthetic. The generator does not make construction estimates, safety claims, permit decisions, or performance claims. Large workload generation is not evidence that the downstream search algorithm scales.
