# Culinary Experience Benchmark Validator

Standalone Rust validator for the culinary experience scenario and metric catalogues.

## Run

From the repository root:

```sh
cargo test --manifest-path tools/culinary-experience-bench/Cargo.toml
cargo run --manifest-path tools/culinary-experience-bench/Cargo.toml -- --help
cargo run --manifest-path tools/culinary-experience-bench/Cargo.toml -- validate docs/research
```

The validator runs offline. It checks cross-file invariants, not recipe quality, food safety, nutritional correctness, sensory acceptability, cultural legitimacy or whether published research generalizes to a local setting.

## What it checks

- Unique scenario IDs and metric IDs.
- Every scenario metric exists in the metric catalogue.
- Required gates are non-empty and uniquely named within each scenario.
- Every metric specifies a definition, unit, direction and denominator.
- Evidence classes, scenario claim limits and adversarial controls are explicit.
- Composite scoring of all dimensions is disabled.
- Safety-relevant metrics keep zero-violation acceptance values.
- The receipt carries SHA-256 digests of the exact two contract files.

A passing report proves only contract consistency. Synthetic or schema-level validation is not a physical cooking result.
