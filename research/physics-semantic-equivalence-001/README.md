# Physics Semantic Equivalence V0.1

This crate implements the initial fail-closed admission rules for pairwise cross-simulator semantic mappings.

It deliberately does not:
- execute physics;
- parse JSON;
- certify a solver or physical model;
- declare one backend to be ground truth;
- replace #443, #460, #1172 or #1472.

The transport contract is [semantic_mapping_receipt_v0_1.schema.json](../../../schemas/physics/semantic_mapping_receipt_v0_1.schema.json). Adapters deserialize validated records into the typed Rust structs in this crate and call `validate_receipt`.

## Run

```bash
cargo fmt --manifest-path research/physics-semantic-equivalence-001/Cargo.toml -- --check
cargo test --manifest-path research/physics-semantic-equivalence-001/Cargo.toml --locked
cargo clippy --manifest-path research/physics-semantic-equivalence-001/Cargo.toml --locked --all-targets -- -D warnings
```

The validator accepts a comparison only when all nine required dimensions appear exactly once, every required identity is well-formed, every bounded mapping supplies a transformation and finite non-negative bound, and the author differs from the reviewer. Any non-equivalent, unsupported or unassessed dimension blocks a matched score without pretending the schema itself is invalid.

This first slice validates typed Rust values; JSON decoding and full schema-validator integration remain a separate adapter concern.