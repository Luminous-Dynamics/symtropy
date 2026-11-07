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

This version requires `WaterQuota`, `ShorePower`, `TerminalHandling` and `InlandTransfer` to use cumulative-window accounting. The latter two represent tonnes-per-window throughput quotas in this model, not simultaneous occupied tonnes. Channel transit slots, lockage slots and berth slots use concurrent-capacity accounting. The mode is explicit in the workload and checked against the typed resource kind; a shift throughput quota cannot be silently treated as a concurrent slot. A future profile that models cranes or other simultaneous handling equipment must give those resources a distinct typed resource identity/unit rather than reinterpreting `Tonnes`.

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
- terminal-handling and inland-transfer throughput quota conflicts across sequential (non-overlapping) operations;
- joint infeasibility when shared water, shore-energy, terminal-handling throughput or inland-transfer throughput is individually feasible for each demand but not for both;
- conflicting shared-resource snapshots and invalid accounting modes;
- missing, duplicate and orphan resource policies;
- duplicate demand identity, mismatched decision cutoffs/tick resolutions and missing time basis;
- input permutation replay, zero-budget rejection and explicit search-budget exhaustion.

The current exact-head source declares **32 unit tests and 3 integration tests**. After the two terminal/inland throughput-quota controls were added, the exact-head workflow passed formatting and all tests on 7 November 2026: https://github.com/Luminous-Dynamics/symtropy/actions/runs/42341759255. The independent fleet enumerator and 20-variant sweep are part of the 32 unit-test corpus; the three integration tests cross-check event-sweep and cumulative quota arithmetic against direct references.

The test-only independent fleet enumerator separately explores each demand's alternatives and integer-tick placements, calculates feasibility using per-tick/per-window accounting, reconstructs the objective vector and computes its own Pareto frontier. It is cross-checked against the production frontier on four hand-checkable cases and a **20-variant frozen parameter sweep**. The sweep changes fast/slow alternative combinations, delivery windows, lock/channel/berth/terminal capacity, and cumulative water/shore-power quotas. Every production frontier must match the independently generated frontier exactly.

A separate integration-test oracle compares the production concurrent-capacity event sweep with a direct per-tick implementation and compares cumulative quota accounting with independent window sums over exhaustive tiny operation pairs.

## Research comparisons

See [`industry-baselines.md`](industry-baselines.md) for the current 2026 industry/research comparison, matched baseline matrix, qualification targets and explicit gaps. The solver-backed reference is tracked in [MARITIME-FLEET-MIP-001 #3833](https://github.com/Luminous-Dynamics/symtropy/issues/3833).

## Deliberate limits

This is an exact tiny-instance reference oracle, not a production optimizer. It requires all demands to be jointly served in a candidate plan; partial-service allocation, participant fairness policies, stochastic arrival/duration distributions, dynamic replanning, multi-chamber vessel packing, hydraulic transients, water-level/tide physics, sediment evolution, AIS/LPMS ingestion, resource reservation and live dispatch remain out of scope.

The implementation is a bounded exhaustive enumerator over its declared finite integer-tick search space. A second test-only implementation now cross-checks the complete fleet frontier on a small set of frozen hand-checkable workloads; this is useful differential evidence, not yet a comprehensive independent qualification corpus or a reproduced external MILP/CP-SAT solver. Expand the cross-check beyond the existing 20-variant two-demand sweep to three-, five- and ten-demand cases, then compare larger matched instances against the solver-backed HiGHS MILP reference tracked in #3833 and the reproduced serial-lock/multi-objective literature baselines.

The production search budget counts operation-start candidate evaluations, not every recursive call or memory allocation; future benchmark reports must preserve that exact definition.

The oracle assumes fixed segment durations, fixed capacity windows and one ordered route per demand. Its `total_idle_wait_ticks` metric only sums idle gaps between consecutive route operations for the same demand; it excludes anchorage/queue wait before the first operation, berth dwell, and end-to-end cargo dwell. Those must be added by the recovery and service-chain owners before comparing door-to-door logistics performance.

It does not establish that upstream evidence is authentic, that a vessel is physically qualified, that a resource operator has authorized use, or that real water/energy availability matches the synthetic profile. It never controls a lock, berth, pump, vessel or shore-power asset.