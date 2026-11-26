# Maritime Exact Resource Oracle v0.1

**Status:** synthetic exhaustive micro-instance; exact-head CI pending.  
**Tracking:** [#7519 — MARITIME-EXACT-ORACLE-001](https://github.com/Luminous-Dynamics/symtropy/issues/7519).  
**Related:** #3683, #7231, #7357, #7504, #7510, #7512.

## Purpose

This crate is an exact, dependency-free reference for tiny time-indexed resource allocation problems. It complements MDTR:

- MDTR asks whether the information and decision were justified at the historical cutoff.
- This oracle asks whether a combination of candidate actions fits the declared physical resource capacities and which feasible combinations lie on the Pareto frontier.

It is not a production scheduler, terminal simulator or continuous-time physics model.

## Shared-resource contract

A candidate action belongs to exactly one vessel. It reserves integer units of resources at discrete time slots. A scenario supplies a capacity for each (resource, slot) pair. The exhaustive solver selects exactly one candidate per declared vessel.

The same model represents berth occupancy, quay/yard cranes, AGV fleet, approach bridge, yard slots, tug/pilot windows, onshore-power sockets, grid capacity, rail/gate/inland slots and tide availability. A capacity of zero closes a resource at that time. Reservations are conjunctive; objective improvements never compensate for an over-capacity plan.

## Objective vector

Each candidate contributes five separately reported integer metrics:
1. vessel waiting;
2. end-to-end receiver delay;
3. energy cost;
4. emissions;
5. missed connections.

A plan dominates another only when it is no worse on every axis and strictly better on at least one. Equal or incomparable outcomes remain on the frontier. No arbitrary scalar weights are introduced.

## Current tiny port instance

The frozen fixture has two vessels, six candidate actions and time-indexed capacities for:
- two berths, including a tide-closed slot;
- a shared quay-crane pool;
- AGV fleet;
- constrained bridge corridor;
- yard block;
- shore-power socket.

Exhaustive enumeration should produce four feasible combinations and three Pareto-undominated plans. The tide-closed candidate and early conflicting assignments are excluded by hard capacity checks. The hand-authored output is independently reproduced by a standard-library Python implementation.

## Run locally

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-exact-resource-oracle/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-exact-resource-oracle/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-exact-resource-oracle/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-exact-resource-oracle/Cargo.toml --bin resource-oracle-audit > resource-oracle-receipt.tsv
python3 research/maritime-exact-resource-oracle/scripts/independent_reference.py --receipt resource-oracle-receipt.tsv
```

## Why this is useful for the SOTA tournament

Optimization systems should not be compared only by their own feasibility reports. Each candidate generator (FCFS, rolling horizon, MILP/CP-SAT, robust C&CG, ALNS/RL-ALNS, GNN/MAPPO, HA-QMIX or branch-and-price) should emit the same candidate-action interface. This tiny oracle can exactly adjudicate feasible combinations and Pareto dominance on enumerable cases.

For larger instances, use a solver-provided valid bound and publish timeouts / incumbents. Do not claim an optimality gap when no valid bound exists. The exhaustive oracle is a ground-truth reference only for its declared finite discrete model.

## Limitations and claim ceiling

- Candidate actions are supplied; this crate does not generate an optimization policy.
- Time is discrete; continuous berth positions, crane travel curves, collision geometry, stochastic arrivals and tide hydrodynamics are not modeled yet.
- Resource identities/capacities are supplied by the scenario; the oracle does not prove that they match a real port.
- Objective values are fixture inputs, not independently measured outcomes.
- No cargo custody, evidence rights, authority, receiver acceptance, operational control or live terminal source is connected.
- The fixture is synthetic; a passing result is not real-port performance, global generalization or a SOTA win.

Next increment: add a second topology family with bridge/tide windows and then compare typed candidate outputs from the optimizer families against exact feasible sets and Pareto fronts. Keep physical/authority qualification with the existing #7231/#7357 owners and decision-time evidence with #7505.


## Receipt completeness

The deterministic receipt now emits the canonicalized scenario manifest before the feasible set:
- sorted vessel identities;
- every time-indexed resource capacity, including zero-capacity tide closures;
- every candidate ID, vessel binding and objective vector;
- every candidate reservation by resource and slot;
- complete feasible combination set and Pareto frontier.

The independent Python reference reconstructs the same canonical manifest directly from the fixture, exhaustively enumerates the candidate Cartesian product, computes the Pareto set independently, and compares the entire byte-level receipt. This makes a solver result auditable against the exact capacity and reservation inputs rather than reporting only plan IDs and objective values.


## Sea–rail topology regression

The Rust unit suite now includes a second hand-authored topology family that reserves rail tracks, rail-mounted gantry-crane capacity and receiver-side rail-gate slots. Two vessel choices that each look feasible in isolation cannot both consume the same one-capacity track/time slot. The exact enumerator retains the two cross-track combinations, and both remain Pareto-undominated because they trade energy cost against emissions.

This test is a resource-model regression, not a second independently serialized fixture yet. The primary 2-vessel berth/bridge/AGV/yard/OPS fixture remains the separately checked receipt (4 feasible / 3 Pareto); the sea–rail case is checked in Rust tests only.
