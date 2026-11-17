# Living World Ethical Impact Validator v0.1

A read-only Rust conformance checker for the versioned fixture corpus in
`research/living-world-ethical-impact/v0.1/`.

## What it validates

- JSON parsing into closed, typed Rust structures (`deny_unknown_fields`).
- Version and corpus identity.
- Unique fixture, factset, profile, premise, evidence, party, action and control IDs.
- Referential integrity for premise findings, evidence citations, action-option scopes and anti-derivation controls.
- The twelve distinct scenario families.
- The v0.1 claim ceiling: synthetic-known-answer evidence only, human review required, no action authority, and no thermodynamic observable promoted to qualified.

The checked-in JSON Schema is also parsed and its critical invariants are tested, including the fixed twelve-fixture cardinality, the closed top-level object, the `not_recorded` evidence state and action-scoped findings.

## What it does not validate

This is not a general-purpose JSON Schema implementation. It does not validate empirical evidence, verify signatures or source digests, evaluate whether an ethical premise is morally correct, infer consent, estimate welfare or sentience, calculate ecological or thermodynamic observables, or grant permission to act.

The six profiles are conditional premise examples, not complete ethical theories or a universal ranking. Fixture expected findings are software-test targets authored for these synthetic facts; they are not gold-standard moral truths.

## Run

```sh
cargo fmt --manifest-path research/living-world-ethical-impact-validator/Cargo.toml --all -- --check
cargo clippy --manifest-path research/living-world-ethical-impact-validator/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path research/living-world-ethical-impact-validator/Cargo.toml --all-targets
cargo run --manifest-path research/living-world-ethical-impact-validator/Cargo.toml
```

Direct dependency versions are pinned. This is a standalone Cargo workspace so it does not change Symtropy's production workspace dependency graph.
