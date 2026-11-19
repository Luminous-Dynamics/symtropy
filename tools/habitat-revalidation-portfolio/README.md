# habitat-revalidation-portfolio

Standalone Rust benchmark for joint-target evidence repair with shared prerequisites and explicit search-budget status.

From repository root:

```sh
cargo fmt --manifest-path tools/habitat-revalidation-portfolio/Cargo.toml -- --check
cargo test --locked --manifest-path tools/habitat-revalidation-portfolio/Cargo.toml
cargo run --locked --manifest-path tools/habitat-revalidation-portfolio/Cargo.toml -- docs/research/fixtures/HABITAT_REVALIDATION_PORTFOLIO_V0_1.json
```

Only synthetic planning contracts are checked.
