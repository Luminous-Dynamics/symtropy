# Wood Flow Kiln Schedule Benchmark

Standalone Rust validator and small-instance exhaustive schedule oracle.

From repository root, run:

    cargo fmt --manifest-path tools/wood-flow-kiln-bench/Cargo.toml -- --check
    cargo test --manifest-path tools/wood-flow-kiln-bench/Cargo.toml
    cargo run --quiet --manifest-path tools/wood-flow-kiln-bench/Cargo.toml -- \
      docs/tech/wood-flow-kiln-corpus-v0.1.json \
      --source-commit "$(git rev-parse HEAD)"

The frozen fixture set compares a deterministic earliest-start/first-fit baseline with an exhaustive oracle. It applies profile, capacity, shared-power, release-time, horizon, moisture, grade-evidence and inspection gates.

All inputs are synthetic. The tool does not predict real kiln physics, moisture, lumber grade, energy consumption or industrial savings.
