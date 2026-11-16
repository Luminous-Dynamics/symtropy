# Muon Collider Beam Handoff Validator

A small Rust CLI/library for fail-closed validation of the proposed beam-state handoff envelope.

## Scope

The first version validates:

1. JSON Schema Draft 2020-12 structure using the Rust jsonschema implementation.
2. Stage qualification versus downstream handoff status.
3. Species-aware population accounting and the independently computed balance residual.
4. Correspondence between distribution weighted population and stage surviving population.
5. Confined relative asset paths, exact byte length and SHA-256.
6. The initial text/csv adapter: particle identity, ancestry, finite coordinates, non-negative weights, macro-particle count, weighted population, effective sample size, weighted six-dimensional mean and covariance.

Unknown asset formats are rejected until a named adapter exists. This tool does not validate beam dynamics, cooling performance, accelerator stability, detector performance or collider feasibility.

## Run

From the repository root:

```sh
cargo run --manifest-path tools/beam-handoff-validator/Cargo.toml -- \
  --schema docs/research/muon-collider/beam-state-handoff.schema.json \
  --input docs/research/muon-collider/fixtures/beam-handoff.synthetic.example.json

cargo run --manifest-path tools/beam-handoff-validator/Cargo.toml -- \
  --schema docs/research/muon-collider/beam-state-handoff.schema.json \
  --input docs/research/muon-collider/fixtures/beam-handoff.full-distribution.synthetic.example.json
```

The full-distribution fixture resolves its CSV asset relative to the envelope file, verifies its digest and recomputes declared moments.

## Tests

```sh
cargo test --manifest-path tools/beam-handoff-validator/Cargo.toml
cargo clippy --manifest-path tools/beam-handoff-validator/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/beam-handoff-validator/Cargo.toml -- --check
```

Tests include valid summary/full fixtures and negative controls for missing metadata, digest corruption, population imbalance, hidden species mixing, unqualified handoffs and path traversal.

## Claim ceiling

A pass means the envelope, supported asset adapter and implemented accounting checks passed. It is a software conformance receipt, not a physical validation result.
