# Construction assurance-chain conformance harness

A Rust-only, no-I/O conformance harness for the end-to-end benchmark in `research/construction-robot-assurance-chain-v0-1`.

## Scope

This package validates the frozen scenario schema and handoff invariants:
- stage identifiers/sequences and predecessor graph;
- produced artifact identity and digest continuity;
- consumed artifacts must come from an ancestor stage, or be explicitly supplied at a root stage;
- command authorization must bind the latest observation, epoch, qualification, configuration, task, timebase and lease;
- a new epoch after qualification invalidates an old lease;
- bench/synthetic/HIL evidence cannot be declared admissible for an unsupported field/cross-site claim.

The harness does not run the individual robot safety or metrology components. Their implementations remain owned by #3561/#3564, #3558, #3542, #3531 and #3522/#3505/#3506. Until those PRs merge, this harness validates explicit stage-result handoffs and expected conformance behavior only; it does not claim that the production evaluators have been integrated.

## Run

```sh
cargo test -p symtropy-construction-assurance-chain
```

The scenario fixtures are synthetic. A passing fixture test means the conformance packet is internally consistent; it is not physical validation, safety certification or an operational permit.
