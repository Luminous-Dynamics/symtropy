# NEO reference verifier (v0.1)

A small, independent Rust verifier for the frozen DART/Dimorphos historical reference case.

## Run

From the Symtropy repository root:

```bash
cargo test --manifest-path tools/neo-reference-verifier/Cargo.toml
cargo run --manifest-path tools/neo-reference-verifier/Cargo.toml -- docs/space-engineering/reference-cases/dart-dimorphos-v0.1.json
```

The tool emits a deterministic JSON receipt containing the exact input SHA-256 and a narrowly scoped status.

## What it checks

- Schema version, case type, target and event identity.
- Source identifiers and HTTPS source records.
- Required period-change and along-track velocity reference values.
- Units and uncertainty conventions (period: 3σ; velocity change: 1σ).
- The conditional nature of the reported momentum-enhancement factor β.
- Explicit non-claims and the historical-reference-only claim ceiling.

## What it does not do

- It does not propagate an orbit, fit astrometry, calculate an impact probability, simulate impact hydrodynamics, infer a universal β, or qualify a mitigation mission.
- It does not download or silently refresh external source material.
- The SHA-256 is over the exact local fixture bytes; it is not a digest of the external journal articles.
- A passing receipt validates only this data contract and its frozen reference semantics.

The JSON Schema is in `docs/space-engineering/schemas/`; the fixture is in `docs/space-engineering/reference-cases/`.
