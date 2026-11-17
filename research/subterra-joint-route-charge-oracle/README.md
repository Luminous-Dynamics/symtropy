# Subterra joint route and charge oracle

A standalone Rust exact-reference solver for tiny underground robot-fleet instances. This package is deliberately isolated from the Bevy workspace and has no external dependencies.

## v0.1 model

The solver advances a **synchronized discrete global clock**. During each tick, every robot chooses one action: wait, traverse a directed edge, service an eligible task, or charge at a station. Active traversals and service operations progress concurrently. The search state contains all robot positions, battery levels, active operations, assigned/completed task masks, and the cumulative multi-objective label.

This jointly searches task assignment, robot routes and charging decisions. It is not a task-allocation stage followed by an independent route-refinement stage.

Hard constraints:
- 1–3 heterogeneous robots;
- finite directed graph with explicit travel distance, energy, duration and edge capacity;
- up to 10 tasks with capability masks and service duration/energy;
- up to 2 charging stations with charging rate and concurrent capacity;
- each mandatory task completed exactly once;
- every robot returns to its specified safe/depot node with its reserve intact;
- independent maximum elapsed time, distance, battery energy consumed and grid energy charged;
- conservative per-robot mission-interruption and robot-loss union bounds in parts per million;
- deterministic search/node cap; limit exhaustion never reports exactness.

The returned Pareto frontier preserves time, distance, battery energy consumed and external charging energy as separate metrics. No guessed scalar objective is used.

## Deliberate v0.1 limitations

- Edge travel and task service use fixed discrete ticks; continuous-time scheduling and physical collision geometry are outside this reference.
- Edge capacity is a simple per-tick occupancy limit; there is no detailed opposing-flow, passing-bay or geometric collision model.
- Charger capacity is enforced for simultaneous charging actions. Charger queues are represented by robots waiting for a later tick; station degradation, connector compatibility and queue policies beyond this exact shared-capacity rule are not modelled.
- Energy is integerized; battery efficiency, temperature, ageing and model uncertainty require a calibrated Symtropy envelope.
- Risk bounds are scenario-level supplied upper bounds; the oracle applies a conservative union bound and does not estimate risk.
- All test inputs are synthetic. A green run is not hardware-in-the-loop or mine qualification.

## Run

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --all-targets --offline
cargo run --manifest-path Cargo.toml --example synthetic_joint_route_charge --offline
cargo clippy --manifest-path Cargo.toml --all-targets --offline -- -D warnings
```

## Claim boundary

A proven frontier is exact only for this finite, discretized graph/task/battery/charger model and declared constraints. It does not prove that the graph, battery, charge, safety or locomotion models match a physical mine. Field claims require independent instrumented test-cell and site validation.
