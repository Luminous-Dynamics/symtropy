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


## Adversarial corpus and benchmark boundary

Fifteen authored Rust tests cover: positive whole-trajectory admission; arrival-only acceptance trap with later replenishment; inter-sample segment breach; point-estimate pass with conservative-bound failure; cumulative outage memory; crew workload spike; stale/gapped sampling; missing or mismatched resource events; event-sign semantics; scenario-member source digest identity; weather-blind forecast isolation; hard-gate failure; exact segment/time-grid alignment.

The first fixture contains three matched policy arms, two coherent scenario members per arm, 18 timepoints and 12 inter-sample segments. The adversarial fixture has the integrated candidate cross its freshwater reserve floor at hour 6, consume 3 reserve-days with a timed receipt, then replenish 3 reserve-days at hour 8 and arrive at hour 12 with 5 reserve-days. The trajectory is blocked even though the final arrival snapshot passes.

All benchmark thresholds and data are synthetic. The segment envelopes are checked as declared bounds, not proven physically conservative by this auditor. A measured deployment requires independent evaluation of the model that produces them, sensor/evidence quality, onboard compute latency and qualified vessel-system validation.
