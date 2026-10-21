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

Unit cases cover concurrent non-overlapping placements, temporal reuse of the same space, exclusion-zone avoidance, exhausted-yard rejection, deterministic replay, empty input and unsupported exclusion geometry. A separate integration test independently enumerates all legal integer placements in 216 tiny three-block cases using cell-set collision checks, then compares exact feasibility with both policies. Frozen expected counts are 154 exact-feasible cases and 62 infeasible cases. Caller-order first-fit solves 150 feasible cases, misses four (4/154 ≈ 2.6%), and rejects the 62 infeasible cases. Decreasing-area first-fit is expected to solve all 154 feasible cases, miss zero, and reject the same 62 infeasible cases, with zero false-feasible outputs for both. One known feasible packing that caller-order first-fit misses is explicitly included and is expected to be solved by the decreasing-area policy. These are authored expectations until exact-head CI passes; the 3-block integer-lattice family is deliberately too small to imply general or industrial superiority.

## Claim boundary

This is a transparent baseline, not a globally optimal bottom-left solver. It does not support irregular polygons, rotation, block transfer routes, cranes, lifting/rigging, multi-capacity vehicles, vertical clearance, stacking, floor bearing, or operational safety. A passing synthetic case does not establish industrial throughput, cost savings or superiority to a published solver.

## Benchmark role

Use the baseline with the frozen finite-yard corpus, then add versioned generated instances and exact tiny-instance solutions. Report feasibility, usable area-time utilization, re-handles, vehicle travel, makespan/tardiness, recovery time, schedule churn, runtime and memory separately. Compare against exact or bounded solvers and published methods only when the same model assumptions, objective definitions, input data and stopping budgets are reproducible.
