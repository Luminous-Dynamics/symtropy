# Maritime service-chain fleet oracle v0.1

Dependency-light Rust crate for tiny, deterministic multi-demand scheduling over shared corridor resources. It composes validated `ServiceChainCase` records from `maritime-service-chain-contract`; it does not duplicate vessel, route, evidence, participant or port-call schemas.

## Run

```sh
cargo fmt --manifest-path benchmarks/maritime-service-chain-fleet/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-service-chain-fleet/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-service-chain-fleet/Cargo.toml --all-targets -- -D warnings
```

## Resource semantics

Every resource has one explicit `SharedResourcePolicy`:

- `ConcurrentCapacity`: sum of resource use across all overlapping operations must not exceed capacity at any tick. Half-open intervals are used; an operation ending at tick 10 does not overlap one beginning at tick 10.
- `CumulativeWindowQuota`: each operation must fit wholly inside one declared accounting window, and its consumption is added once to that window's quota.

This version requires `WaterQuota` and `ShorePower` to use cumulative-window accounting. Channel slots, lockages, berth slots, terminal handling and inland transfer use concurrent-capacity accounting. The mode is explicit in the workload and checked against the typed resource kind; a water quota cannot be treated as a simultaneous lock slot.

All cases in one workload must share a corridor, decision cutoff and tick resolution, bound to a caller-declared `time_basis_id`. If two cases reference the same resource ID, their complete resource declarations and referenced evidence records must be identical. Conflicting capacities, windows, units, revisions or evidence snapshots reject the workload.

## Exact tiny-instance oracle

The oracle:

- validates each one-demand case before search;
- sorts demands and alternatives canonically, independent of input case ordering;
- enumerates integer-tick placements for every declared alternative;
- preserves within-vessel route precedence;
- enforces shared concurrent capacity and cumulative quota limits across demands;
- returns a non-dominated vector of makespan, total delivery ticks, idle wait, water, propulsion energy, auxiliary energy, shore import and inland-transfer energy. Energy meter boundaries remain separate and are not added together;
- distinguishes `ExhaustiveComplete` from `BudgetExhausted`. A partial frontier is never described as proven optimal or infeasible.

An empty frontier is meaningful only when `ExhaustiveComplete` is returned: it means no joint plan exists within the declared finite discrete model. Invalid or stale evidence fails validation rather than being converted into infeasibility.

## Current qualification

The Rust unit-test corpus covers:
- overlapping lockage conflict, half-open handoff and capacity greater than one;
- capacity changes between declared windows;
- cumulative quota overrun, exact-boundary acceptance, independent accounting windows and a use interval that straddles two windows;
- two complete vessel demands sharing lock, berth, terminal, shore-energy and water resources;
- joint infeasibility when shared water or shore-energy quota is individually feasible for each demand but not for both;
- conflicting shared-resource snapshots and invalid accounting modes;
- missing, duplicate and orphan resource policies;
- duplicate demand identity, mismatched decision cutoffs/tick resolutions and missing time basis;
- input permutation replay, zero-budget rejection and explicit search-budget exhaustion.

The test-only independent fleet enumerator separately explores each demand's alternatives and integer-tick placements, calculates feasibility using per-tick/per-window accounting, reconstructs the objective vector and computes its own Pareto frontier. It is cross-checked against the production frontier on:
- a feasible two-demand mixed-speed workload;
- a two-demand workload with speed alternatives and non-dominated trade-offs;
- a workload that is jointly infeasible under a strict delivery deadline;
- individually valid demands whose combined water or shore-energy quota is infeasible.

A separate integration-test oracle compares the production concurrent-capacity event sweep with a direct per-tick implementation and compares cumulative quota accounting with independent window sums over exhaustive tiny operation pairs.

## Deliberate limits

This is an exact tiny-instance reference oracle, not a production optimizer. It requires all demands to be jointly served in a candidate plan; partial-service allocation, participant fairness policies, stochastic arrival/duration distributions, dynamic replanning, multi-chamber vessel packing, hydraulic transients, water-level/tide physics, sediment evolution, AIS/LPMS ingestion, resource reservation and live dispatch remain out of scope.

The implementation is a bounded exhaustive enumerator over its declared finite integer-tick search space. A second test-only implementation now cross-checks the complete fleet frontier on a small set of frozen hand-checkable workloads; this is useful differential evidence, not yet a comprehensive independent qualification corpus or a reproduced external MILP/CP-SAT solver. Expand the cross-check to at least 20 tiny cases, then compare larger matched instances against published serial-lock MILP and multi-objective algorithm baselines.

The production search budget counts operation-start candidate evaluations, not every recursive call or memory allocation; future benchmark reports must preserve that exact definition.

The oracle assumes fixed segment durations, fixed capacity windows and one ordered route per demand. It does not establish that upstream evidence is authentic, that a vessel is physically qualified, that a resource operator has authorized use, or that real water/energy availability matches the synthetic profile. It never controls a lock, berth, pump, vessel or shore-power asset.