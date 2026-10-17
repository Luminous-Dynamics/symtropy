# Habitat decision-impact benchmark

A standalone Rust benchmark for choosing which synthetic site investigation to
commission next. It composes Symtropy's finite-state, deterministic decision-impact
direction in issues #1769 and #1789; it is not a general value-of-information engine.

## Run

```sh
cargo fmt --manifest-path tools/habitat-decision-benchmark/Cargo.toml -- --check
cargo test --locked --manifest-path tools/habitat-decision-benchmark/Cargo.toml
cargo run --locked --manifest-path tools/habitat-decision-benchmark/Cargo.toml -- \
  docs/research/fixtures/HABITAT_DECISION_IMPACT_V0_1.json
```

## Frozen toy world

The input contains four mutually exclusive synthetic states describing whether a
canyon flood-path constraint and/or a ridge rock-instability constraint is present.
It defines three candidate sites with invented relative cost units and three
deterministic survey actions. Each action's outcomes must partition the full state
set exactly: no missing, duplicate, overlapping, or undeclared state assignments.

For each outcome, the program chooses the lowest-cost candidate that remains
feasible across every state consistent with that outcome. It reports maximum
residual-state count, worst-case regret relative to the best candidate if the state
were known, regret reduction relative to no investigation, and regret reduction per
declared cost unit. Ranking uses deterministic minimax regret without priors or
outcome probabilities. It is **not expected value of information**. V0.1 assumes
perfectly discriminating observations and does not model noise, missed detections,
inconclusive results, failed instruments, or correlated evidence. Future versions
must model these as explicit states/outcomes instead of inventing likelihoods.

The frozen authored expectation is that the combined survey ranks first (4.8 regret
units reduced per synthetic cost unit), followed by the flood survey (3.0), then the
geotechnical survey (0). These invented values test the algorithm only; they say
nothing about actual survey costs or the value of investigating a real parcel.

## Tests and claim ceiling

The package includes a deliberately brute-force reference oracle, exact fixture-byte
pinning, a standard SHA-256 known-answer test, exact metric assertions, and
order-permutation checks. Twelve authored unit tests cover these behaviors.
Adversarial tests cover unknown hazards, duplicate IDs, unknown fields, changed input
bytes, undeclared states, incomplete or overlapping outcome partitions, and a state
with no feasible candidate.

The fixture is a small synthetic abstraction, not a geological model. It does not
recommend real sites or surveys, estimate physical-hazard probabilities, simulate
flooding or rock stability, establish site/building safety, or replace qualified
investigations and engineering review. A benchmark pass is not physical validation.
