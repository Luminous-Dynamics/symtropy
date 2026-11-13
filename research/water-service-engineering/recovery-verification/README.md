# Water Service Recovery Verification Evaluator

Read-only Rust evaluator for the synthetic v0.1 service-restoration contract.

## Run from the Symtropy repository root

```sh
cargo test --locked --package water-service-t0
cargo run --locked --package water-service-t0 --bin water-recovery-verification -- \
  research/water-service-engineering/recovery-verification/scenarios-v0.1.json \
  research/water-service-engineering/recovery-verification/scenarios-v0.1.schema.json \
  research/water-service-engineering/recovery-verification/disposition-oracle-v0.1.json \
  artifacts/water-recovery-verification/receipt.json
```

The evaluator is part of the existing `water-service-t0` package and adds no hydraulic solver, controller or service-allocation capability. It recomputes all area dispositions, verifies independent measurement-path pairs across five failure domains, enforces freshness/coverage/area binding/stable dwell, and derives the overall restoration state. It compares every area and overall disposition with the separate oracle and recalculates non-authoritative weighted diagnostics.

The receipt hashes the scenario corpus, schema, oracle, Cargo manifest, Cargo.lock and exact implementation revision. Qualification remains `NotRun`.

Full JSON Schema Draft 2020-12 validation is not implemented; the tool checks the schema's root/scenario/area contract and then enforces the semantic rules directly.

## Claim ceiling

Synthetic restoration semantics only. No repair outcome, physical service, potable-water quality, public-health, regulatory or utility operation is qualified. A closed ticket, cleared alarm or pump restart is never independent service proof.
