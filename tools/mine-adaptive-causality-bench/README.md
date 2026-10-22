# Mine adaptive causal replay benchmark

This standalone Rust crate audits policy decision traces against observation-delivery timestamps and counterfactual forks. It is a trace-admissibility contract, not a mine optimizer, geological predictor, or economic model.

A test observed at hour 0 but delivered at hour 4 is unavailable to a decision made at hour 3. Event identity, QC/source admissibility, policy-vs-evaluator visibility, decision chronology, and paired counterfactual traces are all checked fail-closed.

Run:

```sh
cargo fmt --manifest-path tools/mine-adaptive-causality-bench/Cargo.toml -- --check
cargo test --manifest-path tools/mine-adaptive-causality-bench/Cargo.toml
cargo run --quiet --manifest-path tools/mine-adaptive-causality-bench/Cargo.toml -- \
  docs/tech/mine-adaptive-causality-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The receipt binds the complete trace corpus by SHA-256 and the exact source commit. The public synthetic fixture demonstrates known failure conditions; it is not a blinded policy-quality evaluation and contains no real geological data.

Policy quality and economics are deliberately outside this crate. Later versions may compare static, two-stage, rolling-horizon, and decision-aware policies using the same held-out hidden realizations, with the evaluator and any perfect-knowledge bound kept separate from policy inputs.
