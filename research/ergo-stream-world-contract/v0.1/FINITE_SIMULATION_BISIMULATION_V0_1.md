# ErgoStream finite simulation and bisimulation witnesses v0.1

**Implementation:** \`src/behaviour.rs\`  
**Tracking:** [ERGO-STREAM-MORPHISM-001 #7230](https://github.com/Luminous-Dynamics/symtropy/issues/7230)

## Purpose

A total structural isomorphism is often too strong for cross-domain transfer. A one-way simulation is weaker: it says every relevant source transition can be reproduced in the target, but the target may have additional behaviour. Bisimulation is stronger: both sides must match one another's transitions.

The implementation represents finite labelled transition systems (LTSs), permits nondeterministic outcomes, and validates a proposed many-to-many relation between source and target states.

## Formal contract

Let \(R \subseteq S_s \times S_t\) be the declared state relation and \(\psi\) the action mapping.

### Simulation

For every related pair \((s,t) \in R\), every in-scope source action \(a\), and every source transition \(s \xrightarrow{a} s'\), there must be a target response:

\[
t \xrightarrow{\psi(a)} t'
\quad\text{such that}\quad
(s',t')\in R.
\]

Additional target transitions are allowed. This is one-way coverage and does not imply behavioural equivalence.

### Bisimulation

Bisimulation also requires the reverse condition: for every related pair and every in-scope target transition under a mapped action, a source transition must exist whose successors are again related. The action mapping must cover the complete declared target action scope.

The relation may be many-to-many. It is not forced to be a bijection or even a function.

## Executable checks

- Non-empty finite source and target state/action sets.
- Unique state and action IDs and valid transition references.
- Duplicate labelled edges rejected; multiple distinct outcomes are allowed.
- Explicit source and target state/action scopes; in-scope transitions may not leave their state scope.
- Every declared scope state must appear in at least one state-relation pair.
- State-relation pairs must be unique and reference the declared scopes.
- Every source action must have a unique target action mapping.
- Action-kind correspondence must be consistent and one-to-one.
- Simulation checks every source transition from every related pair for a target response.
- Bisimulation additionally checks every target transition for a source response and requires action-map coverage of the full target action scope.

## Positive and negative tests

1. Simulation accepts a target with additional transitions that the source does not have.
2. Bisimulation rejects that same target because the extra target transition has no source response.
3. Bisimulation passes after the extra transition and out-of-scope target state are removed.
4. Simulation rejects a source transition with no mapped target response.
5. A source state may relate to multiple target states.
6. Duplicate transition edges are rejected.
7. Missing state-relation coverage is rejected.

## Limits

This is a finite labelled-transition-system validator for a proposed relation. It does not discover state relations, infer action mappings from raw streams, prove causal-mechanism identity or validate continuous-time dynamics. It also does not yet implement weak simulation with silent transitions, probabilistic bisimulation, timed automata, approximate simulation, or partial observation equivalence.

The declared scopes are part of the claim. Bisimulation outside those scopes is not implied.

## Reproduction

\`\`\`sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
\`\`\`
