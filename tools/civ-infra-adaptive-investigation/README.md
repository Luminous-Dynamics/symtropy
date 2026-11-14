# CIV-INFRA Adaptive Investigation Evaluator v0.1

A standalone Rust reference evaluator for small adaptive investigation policies over conservative state-to-possible-outcome compatibility.

## Run

```sh
cargo fmt --manifest-path tools/civ-infra-adaptive-investigation/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-adaptive-investigation/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-adaptive-investigation/Cargo.toml -- \
  tools/civ-infra-adaptive-investigation/fixtures/adaptive-investigation-v0.1.json \
  /tmp/adaptive-investigation-report.json
```

## Semantics

- A state can remain compatible with multiple outcomes; cross-outcome overlap is valid.
- Every open-support action produces an explicit out-of-model-support branch that preserves the complete current state set and terminates in review.
- Outcomes with no states compatible with the current path are impossible under the admitted finite world and do not create a leaf.
- Supported leaves report decision-signature classes and discordant pairs. Support-failure leaves are counted separately and are not silently dropped from the report.
- No probabilities, expected cost, or expected value of information is inferred.
- Policy-graph depth is checked with memoized longest-path analysis. Runtime expansion is capped at 100,000 operations, 10,000 output leaves, and 100,000 aggregate leaf-state memberships; exceeding any bound returns an error instead of a partial ranking.

The current fixture is fully synthetic. Model refs are traceability labels; the crate does not retrieve external report bytes or qualify a measurement model. A passing test means only that the evaluator matches this finite contract, not that a bridge/tunnel survey or engineering design is physically valid.
