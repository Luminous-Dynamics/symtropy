# AeroWeft Service-Quality Oracle — Synthetic Slice 0.1

This dependency-free Rust workspace is a tiny exact reference implementation for [AEROWEFT-SERVICE-QUALITY-001 #5248](https://github.com/Luminous-Dynamics/symtropy/issues/5248).

Golden fixture:
fixtures/aeroweift-service-quality/tiny-demand-queue-v0.json

## What it evaluates

- Three anonymous synthetic cohorts, each with one potential trip.
- A deterministic finite choice rule based on maximum acceptable UAM mean waiting time.
- Simultaneous arrivals, identical FCFS service servers and a five-minute service duration.
- Exact integer-seconds queue sums and rational mean-wait threshold comparisons.
- One-server and two-server demand-update maps.
- Exact fixed-point enumeration.
- Exact cycle detection, with canonical cycle ordering.
- A monotonicity control showing that adding a server cannot increase mean wait for the same deterministic arrival trace.

## Fixture-local result

One server:
- demand map: 0→3, 1→3, 2→2, 3→1;
- one fixed point: demand 2;
- one cycle: 1↔3.

Two servers:
- stable fixed point: demand 3;
- mean wait at the fixed point: 100 seconds (5/3 minutes).

The cycle is intentional. A finite best-response update must not be declared converged merely because the iteration has stopped changing in a convenient way. Initial-state dependence and all repeated states are explicit.

These results are properties of the synthetic threshold table, not a passenger forecast, calibrated choice model or real vertiport capacity result.

## Deliberate limitations

This is not a multinomial-logit estimator, stated/revealed preference model, general queueing library, passenger forecast, bilevel optimizer, capacity planner, flight scheduler or operational UAM system. It models one homogeneous finite service queue solely to validate the fixture's exact feedback semantics.

Future adapters should consume the exact #5186 event trace and pass qualified service outcomes to #1601. Do not create a second queue/scheduling or multimodal optimization stack.

## Exact-head commands

cargo fmt --manifest-path tools/aeroweift-service-quality-oracle/Cargo.toml -- --check

cargo clippy --locked --manifest-path tools/aeroweift-service-quality-oracle/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path tools/aeroweift-service-quality-oracle/Cargo.toml

Source and tests are authored; no test pass is claimed until the exact-head workflow completes.
