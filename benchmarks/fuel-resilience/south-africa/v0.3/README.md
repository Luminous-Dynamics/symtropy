# SAFRB v0.3 — deterministic min-cost-flow planning baseline

**Status:** standalone research baseline; synthetic fixtures only  
**Research cut:** 2026-11-23  
**Language:** Rust 2024, dependency-free  
**Parent:** SAFRB v0.2 time-expanded network accounting contract

## Purpose

v0.3 adds a conventional optimization comparator. It solves a single-grade, time-expanded network allocation problem using successive shortest augmenting paths and Bellman–Ford shortest paths over a residual graph.

The solve order is:
1. maximize total demand served;
2. among maximum-flow solutions, lexicographically maximize higher-priority demand service;
3. minimize declared abstract route and holding costs.

The implementation uses integer litres and explicit day indices. It enforces node storage capacity, per-edge/day capacity, grade compatibility, supply availability day, transit time and exact demand due day. Routes arriving at or after the horizon are unavailable to the current plan.

## Evidence boundary

- VerifiedUsable supply is an input classification supplied by the caller. The planner does not measure tanks, verify raw evidence digests, validate credentials or prove physical stock.
- Scenario supply is allowed for synthetic counterfactuals and remains labelled in output.
- Reported and Forecast supply is excluded from firm allocation.
- Cost values are abstract integer points per litre, not currency unless a source-defined tariff and unit basis are supplied.
- One solve handles one grade. Run D50 and D500 independently; do not pool.
- This is deterministic min-cost-flow, not robust/stochastic optimization, an MPC controller, hydraulic solver or calibrated national plan.
- The output is advisory. There is no OT write path or allocation authority.

## Run

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --locked
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path Cargo.toml --release --locked
```

## Baseline comparisons enabled

This provides the first reproducible conventional baseline for:
- greedy dispatch (to be added as a separate policy);
- deterministic min-cost max-flow (implemented here);
- LP/MILP implementations from independent solvers;
- two-stage stochastic / scenario recourse;
- robust / distributionally robust optimization;
- Symthaea decision support and active evidence-acquisition policies.

Fair comparison requires identical state snapshots, supply eligibility, demand priorities, topology, capacities, horizon, route costs, scenario set and compute budget. Report total and critical demand served, unserved litres by zone/priority, worst-zone shortfall, CVaR95 under a separate scenario tournament, recovery time, compute, solver status and failures. One deterministic scenario does not establish robust resilience.

## Claim ceiling

A green Rust test run establishes the software behavior of this synthetic optimization implementation only. It does not validate the South African fuel network, source inventory classifications, real route capacity or operational allocation decisions.
