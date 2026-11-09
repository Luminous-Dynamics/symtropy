# RoboCasa-Mem behavior-conditioned evaluation contract rehearsal

This is a dependency-free Rust **synthetic contract fixture** for one semantic property described in RoboCasa-Mem (arXiv:2611.06475): when multiple execution paths are valid, a later evaluator constraint can be derived from the agent's own earlier interaction rather than from a fixed target.

It does not implement RoboCasa-Mem, reproduce its task evaluator, or run the RoboCasa simulator.

## Contract under test

- The policy receives only a typed `PolicyInput` (episode, instruction, current observation reference and robot-state reference).
- The evaluator separately receives an ordered stream of evaluator-observed `InteractionEvent` values.
- The evaluator derives an `EvaluatorDerivedConstraint` from the latest qualifying prior interaction.
- A later evaluator-observed interaction is checked against that derived constraint.
- Two episodes with identical policy-visible inputs may correctly have different evaluator-private expected entities because their earlier valid choices differed.
- Missing prior interactions fail closed; there is no fixed-target fallback.
- Duplicate/replayed IDs, mixed episodes and reordered/duplicate sequence numbers are rejected rather than silently repaired.
- A separate synthetic process-memory oracle preserves paused/open obligations across interleaved task threads; submission remains blocked until all obligations are satisfied.
- Illegal task transitions, cross-task satisfaction, duplicate openings, late resumption after completion and reordered/replayed events fail closed.

The synthetic example models a prior choice of cup A or cup B followed by a later choice that must remain consistent with the earlier interaction. This is an intentionally minimal semantic fixture, not a copy of any RoboCasa-Mem task.

## Why behavior-conditioned evaluation matters

A fixed-target evaluation can reward a memorized destination even when the task permits multiple valid choices. A behavior-conditioned evaluator can assess history-consistent execution while leaving the valid path open.

The paper describes nine mobile-manipulation tasks spanning:
- Environment memory: spatial, attribute and relational information.
- Process memory: single-task progress and multi-task scheduling.

It reports task success (SR) and stage completion (SCR) separately. The synthetic contract tests none of the native task policies, scene assets, simulator, benchmark scores, or physical robot.

## Run

```bash
cargo fmt --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml -- --check
cargo test --locked --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml
cargo clippy --locked --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml --all-targets -- -D warnings
```

## Nonclaims

Passing these tests means only that this small Rust fixture implements the declared synthetic evaluator semantics. It does not qualify the upstream RoboCasa-Mem release, prove that the upstream evaluator is implemented correctly, demonstrate policy memory capability, establish any benchmark improvement, or establish physical truth/safety.
