# Stone Recovery Benchmark Validator

A standalone Rust validator for a frozen, synthetic quarry-to-product geometry and material-accounting corpus.

From the repository root:

```sh
cargo fmt --manifest-path tools/stone-recovery-bench/Cargo.toml -- --check
cargo test --manifest-path tools/stone-recovery-bench/Cargo.toml
cargo run --quiet --manifest-path tools/stone-recovery-bench/Cargo.toml -- \
  docs/tech/stone-recovery-benchmark-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The validator checks source-volume reconciliation, block bounds, block overlap, declared fracture-plane crossing policy, and the evidence gate for load-bearing use. The receipt binds the exact corpus bytes and a 40-hex source revision.

The four fixtures contain one clean known-answer case and three deliberate negative controls. A result of "validated" means the validator reproduced those declared synthetic expectations; it does not mean the quarry plan is physically valid or optimal.

This is a benchmark contract, not a geology model, quarry optimizer, stone certification system, safety case, extraction permission, or environmental assessment.
