# ErgoStream finite dynamics-transfer witness v0.1

**Implementation:** `src/dynamics.rs`  
**Tracking:** [ERGO-STREAM-TRANSFER-001 #7171](https://github.com/Luminous-Dynamics/symtropy/issues/7171)

## Purpose

A graph correspondence is necessary for many forms of structural transfer, but it is not sufficient for transfer of a dynamical mechanism. Two systems can have isomorphic state/relation graphs and still evolve differently under corresponding actions.

This module validates a learner-proposed correspondence between two deterministic finite-state transition systems.

For source transition function `delta_s`, target transition function `delta_t`, state mapping `phi` and action mapping `psi`, every in-scope transition must satisfy:

```text
phi(delta_s(x, a)) = delta_t(phi(x), psi(a))
```

The implementation represents both transition systems explicitly as finite state, action and transition lists. It validates total, injective state and action mappings, consistent one-to-one action-kind correspondence, and exact commutation for every source transition. Missing target transitions, extra target transitions (via transition-count mismatch), nondeterminism, incomplete mappings and inconsistent kind mappings are rejected.

## Tests

The fixture contains three source states, two actions and four transitions, with a target system whose state/action IDs and action-kind IDs differ but whose dynamics commute under the proposed mapping.

Negative tests cover:
- a single mapped transition counterexample;
- non-injective state correspondence;
- action-kind mapping collision;
- nondeterministic source dynamics;
- incomplete state or action maps;
- missing target transition.

## What this proves—and what it does not

Passing the finite tests establishes only that this implementation checks the declared deterministic transition correspondence on the fixtures. It does not discover state/action mappings, infer causal mechanisms from raw observations, authenticate intervention execution, estimate uncertainty, or qualify physical-world transfer.

The v0.1 witness uses exact deterministic transitions. It is not a continuous-time CfC/LTC equivalence test. A future continuous-time extension should separately specify:
- source and target time coordinates and any admissible time transformation;
- observation timing, delay, sampling and missingness;
- action semantics and actuator bounds;
- allowed approximation norm / error tolerance;
- uncertainty and stochastic transition semantics;
- held-out rollout horizons and intervention coverage.

A continuous-time claim must be evaluated on trajectory and intervention holdouts; matching one-step transitions or graph structure is insufficient.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
