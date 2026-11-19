# Physics Semantic Equivalence V0.1

This crate implements the initial fail-closed admission rules for pairwise cross-simulator semantic mappings.

It deliberately does not:
- execute physics;
- certify a solver or physical model;
- declare one backend to be ground truth;
- replace #443, #460, #1172 or #1472.

The transport contract is [semantic_mapping_receipt_v0_1.schema.json](../../../schemas/physics/semantic_mapping_receipt_v0_1.schema.json). `parse_receipt_json` rejects duplicate object keys recursively, unknown fields and malformed values before the parsed receipt reaches `validate_receipt`; `validate_json_receipt` combines both operations. Positive and negative JSON fixtures exercise this transport path. `validate_receipt_schema_json` first rejects duplicate JSON object keys, then executes the checked-in Draft 2020-12 schema through the independent Rust `jsonschema` validator; `validate_schema_and_json_receipt` requires both schema conformance and typed semantic admission.

## Run

```bash
cargo fmt --manifest-path research/physics-semantic-equivalence-001/Cargo.toml -- --check
cargo test --manifest-path research/physics-semantic-equivalence-001/Cargo.toml --locked
cargo clippy --manifest-path research/physics-semantic-equivalence-001/Cargo.toml --locked --all-targets -- -D warnings
```

The validator accepts a comparison only when all nine required dimensions appear exactly once, every required identity is well-formed, every bounded mapping supplies a transformation and finite non-negative bound, and the author differs from the reviewer. Any non-equivalent, unsupported or unassessed dimension blocks a matched score without pretending the schema itself is invalid.

The Rust transport adapter and typed validator are exercised against the checked-in positive and negative JSON fixtures. The V0.1 JSON Schema independently requires exactly one occurrence of each of the nine dimensions (not merely an array of length nine). `fixtures/duplicate-dimension.invalid.json` repeats `identity_geometry` and omits `numerical_profile`. The dedicated workflow executes the checked-in Draft 2020-12 schema against the positive and negative fixture corpus, then runs the typed semantic validator. This proves schema execution and the implemented transport/typed-admission path for the covered fixtures; it does not by itself prove exhaustive equivalence for all JSON Schema keywords or physical correctness.