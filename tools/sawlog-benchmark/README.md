# Sawlog Benchmark Contract Validator

Standalone Rust validator for a strict, synthetic-only sawlog evidence corpus. It is a dataset/physical-accounting contract, not a sawmill optimizer, grading authority, sustainability certification or digital twin.

From the repository root:

    cargo fmt --manifest-path tools/sawlog-benchmark/Cargo.toml -- --check
    cargo test --manifest-path tools/sawlog-benchmark/Cargo.toml
    cargo run --quiet --manifest-path tools/sawlog-benchmark/Cargo.toml -- docs/tech/sawlog-benchmark-corpus-v0.1.json --source-commit "$(git rev-parse HEAD)"

The receipt binds the exact corpus bytes and source commit, keeps reference-volume and section-geometry provenance separate, and reports per-log plus aggregate absolute percentage error. Its frozen corpus includes a deliberate geometry target miss. A schema-valid corpus can therefore return `validated` while one geometry target is missed: model error is benchmark evidence, not malformed input. Unknown fields are rejected. Stations must span the log and be ordered; defects must fit inside it; structural boards require qualified grade evidence and a passing inspection; complete observations must close within declared tolerance; partial observations cannot overrun the gross-volume envelope; unknown observations cannot smuggle in quantitative outputs.

The fixtures are synthetic, with no customer, mill-operator, supplier or proprietary data. Passing validates the schema and declared checks only. See docs/tech/SAWLOG_BENCHMARK_CONTRACT_V0_1.md.
