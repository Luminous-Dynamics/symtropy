# ErgoStream finite quotient / coarse-graining witness v0.1

**Implementation:** `src/quotient.rs`  
**Tracking:** [ERGO-STREAM-MORPHISM-001 #7230](https://github.com/Luminous-Dynamics/symtropy/issues/7230)

## Purpose

An embedding preserves a selected substructure. A quotient does something different: it deliberately merges source states into abstract classes.

A state merge is valid only if the abstract transition is well-defined. For a deterministic finite transition system, the partition must be a congruence for every action:
- states in one class must agree on whether the action is enabled;
- if enabled, every member must transition into the same quotient class.

This module checks that condition and constructs the resulting finite quotient transition system.

## Contract

Input:
- a deterministic finite transition system from `dynamics.rs`;
- a total state-to-class partition.

Validation:
1. Source state IDs and action IDs are unique.
2. Every transition references known states and a known action.
3. There is at most one successor for each source state/action pair; duplicate rows and nondeterministic outcomes are distinct errors.
4. Every source state occurs exactly once in the partition.
5. For every quotient class and every action, all member states have the same enabled/disabled status.
6. If the action is enabled for the class, all members transition into the same destination class.

Output:
- sorted quotient class and action IDs;
- one abstract transition per enabled class/action pair;
- source/quotient state and transition counts.

The generated quotient preserves the declared deterministic action-conditioned transition semantics at the declared resolution.

## Positive fixture

Three source states are partitioned as:
- class 100 = {1, 2};
- class 200 = {3}.

For every action, states 1 and 2 have the same action-enabledness and transition to the same quotient class. The quotient therefore has two abstract states and four abstract transitions.

## Negative controls

- **Successor-class mismatch:** two merged source states have the same action enabled but transition into different quotient classes.
- **Action enablement mismatch:** one merged state enables an action and another does not.
- **Incomplete partition:** at least one source state is omitted.
- **Nondeterminism:** one state/action pair has multiple distinct outcomes.
- **Duplicate transition:** one state/action row is repeated identically.
- **Unknown state:** the partition references a state not present in the source system.

Each case is rejected with a distinct error.

## Limits

This is a finite quotient of one deterministic system. It does not yet:
- map a quotient to an independently observed target system;
- validate probabilistic or set-valued transitions;
- prove approximate bisimulation;
- establish observables or rewards preserved by abstraction;
- recover a quotient partition from raw streams.

A valid quotient is not a proof that all microstate distinctions are irrelevant to every task. The map must remain scoped to the declared actions, transition relation and observables. Future v0.2 work should add task-observable preservation and source-to-target quotient correspondence.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
