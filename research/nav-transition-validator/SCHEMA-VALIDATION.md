# JSON Schema validation gate

The transition validator now uses the pinned Rust jsonschema 0.61.2 library in explicit Draft 2020-12 mode with format validation enabled.

From the repository root:

```bash
cargo test --all-targets --manifest-path research/nav-transition-validator/Cargo.toml

cargo run --manifest-path research/nav-transition-validator/Cargo.toml -- \
  docs/research/navigation-transition-v0.1/nav_transition_case.schema.json \
  docs/research/navigation-transition-v0.1/fixtures/nav_transition_cases.v0.1.json \
  docs/research/navigation-transition-v0.1/manifest.v0.1.json
```

The CLI must pass both the per-case schema validator and the semantic cross-record validator. A schema pass is not a physical model, measurement, oracle-independence or operational navigation qualification.
