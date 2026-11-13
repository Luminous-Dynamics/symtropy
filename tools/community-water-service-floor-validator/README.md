# Community Water Service Floor Validator

Standalone, read-only Rust evaluator for the synthetic v0.1 non-compensatory community service-floor corpus.

## Run from repository root

```sh
cargo test --manifest-path tools/community-water-service-floor-validator/Cargo.toml
cargo run --manifest-path tools/community-water-service-floor-validator/Cargo.toml -- \
  research/water-service-engineering/community-service-floors/scenarios-v0.1.json \
  research/water-service-engineering/community-service-floors/scenarios-v0.1.schema.json \
  research/water-service-engineering/community-service-floors/disposition-oracle-v0.1.json \
  artifacts/community-water-service-floor/receipt.json
```

The validator recomputes every area disposition and then the overall result; it does not trust the expected labels in the scenario file. It enforces the exact required-area set, current evidence and complete coverage for a floor-positive result, the inclusive lower-bound comparison, the upper-bound breach comparison, the mixed breach/unknown disposition and weighted-diagnostic arithmetic.

The Cargo package is an isolated workspace. It does not join another workspace, does not call a hydraulic solver and has no control/actuation surface. The receipt includes source digests, manifest/lockfile digests and exact implementation revision; qualification remains NotRun.

Full JSON Schema Draft 2020-12 engine validation is not implemented by the Rust evaluator: it checks schema declaration and root contract fields, then enforces its semantic rules.

## Claim ceiling

Only synthetic service-floor contract semantics are evaluated. The tool does not establish a legal service standard, physical delivery, customer coverage, water quality, fairness, utility performance or an allocation decision.
