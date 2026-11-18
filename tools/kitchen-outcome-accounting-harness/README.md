# Kitchen Outcome Accounting Harness

This minimal Cargo package compiles the exact tracked source file:

`crates/domains/symtropy-kitchen-planner/src/outcome_accounting.rs`

It runs that module's unit tests without resolving the complete Symtropy/Bevy workspace.

## Run

```sh
cargo fmt --manifest-path tools/kitchen-outcome-accounting-harness/Cargo.toml --all -- --check
cargo test --manifest-path tools/kitchen-outcome-accounting-harness/Cargo.toml
cargo clippy --manifest-path tools/kitchen-outcome-accounting-harness/Cargo.toml --all-targets -- -D warnings
```

## Claim ceiling

A passing run validates the pure Rust arithmetic/evidence-completeness kernel and its included tests at the exact checkout. It does not qualify the full Symtropy workspace, hardware/module capability, instrument truth, calibration, food safety, sensory quality, energy savings or physical kitchen performance.
