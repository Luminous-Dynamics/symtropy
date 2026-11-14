# Water Matched Experiment Admission Evaluator

Read-only Rust evaluator for synthetic baseline-pair input parity and resource-budget disclosure.

## Run from the Symtropy repository root

```sh
cargo test --locked --package water-service-t0
cargo run --locked --package water-service-t0 --bin water-matched-experiment -- \
  research/water-service-engineering/matched-experiments/pairs-v0.1.json \
  research/water-service-engineering/matched-experiments/pairs-v0.1.schema.json \
  research/water-service-engineering/matched-experiments/admission-oracle-v0.1.json \
  artifacts/water-matched-experiment/receipt.json
```

The evaluator admits a pair only when every baseline receives exactly the shared deployment-input object. It separately requires version/config/artifact/runtime provenance, future-covariate denial, and complete development-budget disclosure. Training/compute/engineering budgets may differ across baselines and are intentionally reported rather than force-equalized.

The tool does not score the algorithms or rank vendors. It only rejects unfair or irreproducible pairs. The receipt binds pair corpus, schema, oracle, Cargo manifest, Cargo.lock and exact implementation revision; qualification remains `NotRun`.

Full JSON Schema Draft 2020-12 validation is not implemented; the evaluator checks schema contract requirements and enforces semantic parity rules directly.

## Claim ceiling

Synthetic pair-admission semantics only. No baseline performance, utility outcome, commercial superiority or physical network is qualified.
