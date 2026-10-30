# SIM-REL-010 model-indexed policy evaluator

This dependency-free Rust crate is the first executable slice of SIM-REL-010.

It evaluates one supplied deterministic observable-history policy tree independently under every coherent finite world. A policy node selects an action from the public history only; the evaluator never accepts a true-world label or evaluator-target argument. Each world carries its own prior weights, outcomes, and hazard map. The exact report retains per-world results and computes nominal and worst-case summaries.

The evaluator reports exact reduced rational values for:
- expected cumulative action cost (including the cost of an action that immediately causes a hazard);
- probability of at least one campaign hazard;
- probability of a correct exact-identification claim;
- probability of a false identification claim;
- probability of abstention.

The same policy tree is replayed under every world. Action costs and semantic hypothesis identifiers must be shared across the model family. Model outcomes and hazards may differ. Repeated actions on one policy path, cyclic or unreachable nodes, malformed world tables, duplicate world names, and arithmetic overflow fail closed.

## Deliberate non-goals

This crate does not synthesize or optimize policies, construct ambiguity sets, implement a rectangular adversary, handle stochastic observations, or prove that the true physical system belongs to the supplied model family. It is an independent replay/evaluation primitive for the later robust policy optimizer and its exhaustive tiny-domain verifier.

## Qualification

Run:

```sh
cargo fmt --manifest-path tools/sim-rel-model-evaluator/Cargo.toml -- --check
cargo test --locked --manifest-path tools/sim-rel-model-evaluator/Cargo.toml --lib
cargo clippy --locked --manifest-path tools/sim-rel-model-evaluator/Cargo.toml --lib -- -D warnings
```

The dedicated workflow also checks the exact checkout SHA and a clean working tree.
