# Maritime Explanation Evidence Validator

Rust-first, read-only P0 evaluator for the synthetic explanation contract.

## Run

```sh
cargo test --locked -p maritime-explanation-evidence-validator
cargo clippy --locked -p maritime-explanation-evidence-validator --all-targets -- -D warnings
cargo run --locked -p maritime-explanation-evidence-validator -- benchmarks/maritime-explanation/explanation-corpus-v0.1.json --source-commit <40-hex> --case-sha256 <64-hex>
```

The deterministic report binds the exact source commit and case-corpus digest. P0 validates signal/feature references, prediction-time availability, installed/current evidence, command-vs-physical roles, derived dependency double-counting, simulation promotion, non-isolable fault ambiguity, causal-claim boundaries and two metamorphic assertions.

## Claim ceiling

All fixture values are synthetic. This tool does not run SHAP, Integrated Gradients, LIME, attention, a trained diagnostic model, a physical engine or a human review study. Passing demonstrates only that the evidence/claim contract was enforced against this exact synthetic corpus.
