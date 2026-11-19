# Marine Temporal Mission Viability

Standalone Rust auditor for Symtropy #6271. It extends the non-compensatory service vector from an arrival snapshot to a complete time-indexed voyage trajectory.

## Commands

```sh
cargo fmt --manifest-path tools/marine-temporal-viability/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-temporal-viability/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-temporal-viability/Cargo.toml
cargo run --manifest-path tools/marine-temporal-viability/Cargo.toml -- audit-trajectory tools/marine-temporal-viability/fixtures/synthetic-temporal-viability-good-0001.json
```

## What it checks

- The mission, vessel, occupancy, initial resource state, plan, forecast information set, hard-gate roster, operating envelope and floor contract share an exact digest-bound snapshot.
- Scenario member IDs and source-artifact digests are frozen; every policy must replay every member on the exact same time grid.
- UTC event times, elapsed hours, route progress, sample gap limits and segment boundaries are consistent.
- Every point and every inter-sample segment carries all ten service dimensions, conservative bounds, coverage and provenance.
- Minimum reserve floors use the lower bound; maximum delay/outage/comfort/workload/purchased-energy floors use the upper bound.
- Hard-gate results are checked at every sample and segment; any fail, infeasible or abstain result prevents candidate viability.
- Cumulative essential-service outage and comfort exposure cannot decrease after recovery.
- Any change in energy, freshwater, food, waste capacity or spare coverage must reconcile exactly to typed, timed, evidence-bound resource events.
- A later replenishment cannot erase a prior point or inter-sample floor breach. The report counts paths that pass an arrival-only check but fail trajectory viability.

The synthetic adversarial fixture has a freshwater reserve of 2 days at hour 6, later replenished to 5 days by hour 12. Its arrival state passes the illustrative 3-day floor; its voyage trajectory does not.

## Limits

- All fixtures are synthetic known-answer tests, not real vessel telemetry, a physical resource model or an operational planner.
- The segment envelope is only as trustworthy as the upstream model/evidence. This tool checks the declared conservative bound; it does not prove a numerical model has bounded the true continuous trajectory.
- Floors are contract-specific demonstration values; real limits need actual vessel systems, crew, applicable law and qualified engineering review.
- This is not a stability model, live safety monitor, safety certificate or navigation authority.
