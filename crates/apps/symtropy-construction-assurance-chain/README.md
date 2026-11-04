# Construction assurance-chain conformance harness

A Rust-only, no-I/O conformance harness for the benchmark in `research/construction-robot-assurance-chain-v0-1`.

## Scope

The harness validates:
- stage identifiers/sequences, predecessor DAG and artifact digest continuity;
- exact per-command binding to the latest prior observation, worksite epoch and qualification in that command's own causal timeline;
- causal ancestry from each referenced stage to the command stage;
- direct consumption of the referenced observation/epoch/qualification artifacts;
- lease generation, task, robot configuration, timebase, expiry and command binding;
- computed command rejection reason codes, which must match both the command attempt and command-stage receipt;
- claim evidence-origin/scope constraints.

The key rule is **per-command causal frontier, not scenario-global final state**. A later epoch invalidates commands after that event, but cannot retroactively invalidate a command that was valid before the event. It also cannot authorize a command after the change using a qualification issued before the change.

The harness does not run the individual robot safety or metrology components. Their implementations remain owned by #3561/#3564, #3558, #3542, #3531 and #3522/#3505/#3506. Until those PRs merge, this harness validates explicit stage-result handoffs and expected conformance behavior only; it does not claim production evaluators are integrated.

## Run

```sh
cargo test -p symtropy-construction-assurance-chain
```

The fixtures are synthetic. A passing fixture test means the packet is internally consistent, not physical validation, safety certification or an operational permit.
