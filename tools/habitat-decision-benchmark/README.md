# Habitat decision-impact benchmark

A small standalone Rust benchmark for deciding which synthetic site investigation to commission next. It composes the finite-state, deterministic decision-impact direction tracked in Symtropy #1769 / #1789; it is not a general value-of-information engine.

## Run

```sh
cargo fmt --manifest-path tools/habitat-decision-benchmark/Cargo.toml -- --check
cargo test --locked --manifest-path tools/habitat-decision-benchmark/Cargo.toml
cargo run --locked --manifest-path tools/habitat-decision-benchmark/Cargo.toml -- docs/research/fixtures/HABITAT_DECISION_IMPACT_V0_1.json
```

## Frozen toy world

The input has four mutually exclusive synthetic states describing whether a canyon flood-path constraint and/or a ridge rock-instability constraint is present; three site candidates with invented relative cost units; and three deterministic survey actions whose outcomes must partition the full state set exactly.

For each outcome, the program chooses the lowest-cost candidate that remains feasible in every state consistent with that outcome. It computes maximum residual-state count, worst-case regret relative to the best candidate if the state were known, reduction in that regret relative to no investigation, and regret reduction per declared cost unit. Actions are ranked without priors or outcome probabilities. The top-level value is a deterministic minimax-regret ranking, **not expected value of information**.

This fixture's authored expectation is that the combined survey ranks first (4.8 regret units reduced per synthetic cost unit), then the flood survey (3.0), then the geotechnical survey (0). Those values are intentionally invented benchmark inputs and say nothing about real survey economics.

## Tests and claim ceiling

The package contains a deliberately brute-force reference oracle, exact fixture-byte pinning, a known-answer SHA-256 test, ranking and metric assertions, order-permutation checks, and adversarial tests for missing/overlapping outcome partitions, undeclared hazards, duplicate IDs, unknown fields, altered input bytes, and a state with no feasible candidate.

The fixture is synthetic and deliberately small; its hazard states are a toy finite-state abstraction, not a model of actual geology. It does not recommend real sites or surveys, estimate physical hazard probabilities, simulate flooding or rock stability, establish site/building safety, or replace qualified investigations and engineering review. A structural/benchmark pass is not physical validation.
