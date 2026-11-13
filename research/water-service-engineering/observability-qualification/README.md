# Water Network Observability Qualification Evaluator

Read-only Rust evaluator for the synthetic failure-domain observability corpus.

## Run from the Symtropy repository root

```sh
cargo test --locked --package water-service-t0 --bin water-observability-qualification
cargo run --locked --package water-service-t0 --bin water-observability-qualification -- \
  research/water-service-engineering/observability-qualification/scenarios-v0.1.json \
  research/water-service-engineering/observability-qualification/scenarios-v0.1.schema.json \
  research/water-service-engineering/observability-qualification/disposition-oracle-v0.1.json \
  artifacts/water-observability-qualification/receipt.json
```

The evaluator uses the existing `water-service-t0` package and its dependencies. It is not a sensor-placement optimizer or hydraulic solver.

It recomputes surviving independent channels by inspecting six dependency domains, checks exact topology certificate binding, distinguishes structural observability from numeric estimator qualification, preserves direct instrument readings even when a model certificate is stale, and compares all 48 per-state dispositions plus overall outcomes against the separate oracle.

The receipt hashes scenario, schema, oracle, package manifest, Cargo.lock and exact implementation revision. Qualification is always reported as `NotRun`.

Full JSON Schema Draft 2020-12 engine validation is not implemented; the CLI checks the schema declaration and required root fields, then enforces semantic invariants.

## Claim ceiling

Synthetic observability contract semantics only. No real network, sensor, estimator, hydraulic state or utility operation is qualified.
