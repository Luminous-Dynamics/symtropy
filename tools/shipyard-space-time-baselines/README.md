# Shipyard space-time baseline (V0)

A dependency-free, deterministic first-fit/bottom-left-style baseline for the isolated verifier in [space-time PR #2548](https://github.com/Luminous-Dynamics/symtropy/pull/2548). This is an initial reference policy for benchmarking, not a solver or production scheduler.

## Run

```sh
cargo fmt --all --manifest-path tools/shipyard-space-time-baselines/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-space-time-baselines/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/shipyard-space-time-baselines/Cargo.toml --all-targets -- -D warnings
```

The nested Cargo crate has no external dependencies and uses the sibling space-time verifier as a local path dependency.

## Policy contract

Requests are processed in input order. For each rectangular block request, candidate lower-left coordinates are derived from yard and obstacle boundaries and tested in deterministic ascending (y, x) order. Reservations may reuse a floor region only when their half-open occupancy intervals do not overlap. Active rectangular exclusion zones block placement. Any invalid input, unsupported exclusion geometry, or unsatisfied request rejects the entire call; no partial plan is returned.

Every output scenario is passed to the independent space-time verifier. The baseline cannot override a violation or mark the result feasible itself.

## Tests

Unit cases cover concurrent non-overlapping placements, temporal reuse of the same space, exclusion-zone avoidance, exhausted-yard rejection, deterministic replay, empty input and unsupported exclusion geometry. A second cell-set check independently confirms that concurrent placements emitted by the baseline share no lattice cell.

## Claim boundary

This is a transparent baseline, not a globally optimal bottom-left solver. It does not support irregular polygons, rotation, block transfer routes, cranes, lifting/rigging, multi-capacity vehicles, vertical clearance, stacking, floor bearing, or operational safety. A passing synthetic case does not establish industrial throughput, cost savings or superiority to a published solver.

## Benchmark role

Use the baseline with the frozen finite-yard corpus, then add versioned generated instances and exact tiny-instance solutions. Report feasibility, usable area-time utilization, re-handles, vehicle travel, makespan/tardiness, recovery time, schedule churn, runtime and memory separately. Compare against exact or bounded solvers and published methods only when the same model assumptions, objective definitions, input data and stopping budgets are reproducible.
