# Mobility RRVR Trace Contract v0.1

Dependency-free Rust reference implementation for replayability and causality-preserving minimization of fault traces.

## Core invariant

A candidate is accepted only when the replay oracle returns the exact target `FailureSignature`:

- exact property ID;
- exact violation class;
- exact witness event ID.

A different failure is not the same counterexample. A passing trace, invalid trace, non-reproducing baseline or mismatched oracle revision cannot be minimized.

## Causal rules

Each event has a stable identity, strictly increasing logical sequence and predecessor IDs.

- **Removal:** deleting an event also deletes every descendant that depends on it.
- **Retention:** keeping an event also keeps every causal ancestor it requires.
- **Witness:** the exact target witness event must remain in every valid candidate.
- **Budget:** replay calls are bounded and counted.
- **Minimality:** `one_minimal = true` is emitted only after all single-event causal removals have been tested and rejected. If the budget runs out, the report says minimization is incomplete.
- **Parentage:** the minimized bundle names the original bundle as its parent and preserves the exact failure signature and initial-state commitment.

The minimizer is deterministic for a fixed trace, oracle revision, replay seed and call budget. `check_replay_determinism` separately checks repeated oracle verdicts.

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-trace/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-trace/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-trace/Cargo.toml --all-targets --locked -- -D warnings
```

## Benchmark comparison

Compare the same frozen traces/oracles across:
1. sequence-only delta debugging;
2. dependency-aware causal minimization;
3. dependency-aware minimization with exact failure-signature preservation.

Report exact-failure replay rate, event and byte reduction, causal-closure retention, oracle calls, wall time, false-preservation rate, deterministic-output rate and human diagnosis time. Do not collapse them into a single weighted score.

## Non-claims

The crate does not execute a simulator, parse TLC output, capture ROS events, authenticate artifacts, prove physical causality, call hardware or authorize recovery. The replay oracle and artifact owner must be independently qualified. A pass means only that the supplied trace and oracle satisfy this reference contract.
