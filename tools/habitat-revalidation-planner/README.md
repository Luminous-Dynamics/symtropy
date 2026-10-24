# habitat-revalidation-planner

Standalone Rust planner for synthetic, dependency-safe evidence revalidation.

From repository root:

```sh
cargo fmt --manifest-path tools/habitat-revalidation-planner/Cargo.toml -- --check
cargo test --locked --manifest-path tools/habitat-revalidation-planner/Cargo.toml
cargo run --locked --manifest-path tools/habitat-revalidation-planner/Cargo.toml -- docs/research/fixtures/HABITAT_REVALIDATION_PLAN_V0_1.json
```

No real-world authorization or safety claim is produced.
