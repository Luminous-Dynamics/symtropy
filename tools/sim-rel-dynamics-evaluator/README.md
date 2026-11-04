# SIM-REL-011 finite-state dynamics evaluator

This dependency-free Rust crate evaluates a supplied finite-horizon adaptive policy across one or more complete stateful scientific worlds.

## What it models

Each semantic hypothesis has a finite hidden physical state and one transition row per experiment action. A safe action returns a binary observation and advances that hypothesis' physical state. A hazardous transition ends the campaign without producing an observation. Action cost is charged on every attempt, including one that causes a hazard.

The same policy tree is evaluated under every world. The policy selects actions from public observation history only; it receives no true hypothesis, latent state, or model identity. Repeated actions are allowed and use the current physical state. Reset/washout behavior is represented by an explicit action in the transition table, with its own cost and risk.

## Exact reports

The evaluator reports reduced rational values per world for:
- expected cumulative experiment cost;
- probability of a campaign hazard;
- probability of a correct exact-identification claim;
- probability of a false identification claim;
- probability of abstention.

It also reports nominal and worst-case summaries across the supplied finite world family.

## Deliberate non-goals

This crate evaluates policy trees; it does not synthesize or optimize them. It currently uses deterministic finite-state transitions, binary observations, fixed positive action costs, and finite policy horizons. It does not model stochastic observations, infer latent state beliefs, construct ambiguity sets, establish model-family coverage, or certify physical safety. Measurement-noise state (SIM-REL-008) is separate from experiment-induced physical state.

## Qualification

Run:

```sh
cargo fmt --manifest-path tools/sim-rel-dynamics-evaluator/Cargo.toml -- --check
cargo test --locked --manifest-path tools/sim-rel-dynamics-evaluator/Cargo.toml --lib
cargo clippy --locked --manifest-path tools/sim-rel-dynamics-evaluator/Cargo.toml --lib -- -D warnings
```

The dedicated workflow checks the exact checkout SHA and clean working tree.
