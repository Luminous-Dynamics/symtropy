# Marine Inter-sample Envelope Auditor

Standalone Rust 1.96-compatible evaluator for Symtropy issue #6293. It complements the sampled trajectory and conservation audit in PR #6261.

## Commands

```sh
cargo fmt --manifest-path tools/marine-intersample-envelope/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-intersample-envelope/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-intersample-envelope/Cargo.toml
cargo run --manifest-path tools/marine-intersample-envelope/Cargo.toml -- audit-inter-sample tools/marine-intersample-envelope/fixtures/synthetic-intersample-safe-0001.json
cargo run --manifest-path tools/marine-intersample-envelope/Cargo.toml -- audit-inter-sample tools/marine-intersample-envelope/fixtures/synthetic-intersample-hidden-dip-0001.json
```

## Bound construction

For each metric and adjacent sample interval of duration dt hours, the contract supplies a maximum absolute rate L in metric-units/hour. The endpoint uncertainty bounds form two cones.

For a lower constraint, the conservative inter-sample lower envelope is the minimum over t in [0,dt] of:

`max(a - L*t, b - L*(dt-t))`

where a and b are the lower endpoint bounds.

For an upper constraint, the conservative upper envelope is the maximum over t in [0,dt] of:

`min(a + L*t, b + L*(dt-t))`

where a and b are the upper endpoint bounds.

The implementation uses the exact piecewise closed form, including the endpoint case when the two cones do not intersect inside the interval. It does not use a midpoint heuristic or simply take the min/max of samples. It verifies that the two endpoint uncertainty sets are compatible with the rate ceiling.

## Event and evidence rules

- Rate ceilings are mission-specific, unit-bearing, contract-digest-bound and carry a method, evidence digest and evidence class.
- A synthetic rate-bound record cannot be promoted to measured/shadow/published evidence.
- All five disturbance classes and every time-grid point must be present.
- The scenario and event-manifest digests are recomputed over canonical JSON payloads.
- Known outage/repair/weather/operator/shore-service events must lie exactly on a declared time-grid boundary. A discrete event inside a segment invalidates the input; it must split the segment.
- Initial resource endpoints must match across all scenario branches.
- Endpoint metrics retain measurement intervals, coverage, class, method and evidence digest.

## Synthetic controls

- Safe fixture: five scenarios × two intervals × eleven metrics = 110 computed envelopes. The freshwater floor is 80 L; endpoint intervals at 90 L remain above the floor and the declared 2 L/hour bound yields an inter-sample lower envelope of 83.9 L.
- Hidden-dip fixture: every freshwater endpoint remains 90 L, but the declared 4 L/hour bound yields a conservative inter-sample lower envelope of 77.9 L. This must block.
- A symmetric upper-bound control raises both endpoint workload values to 11.8 hours against a 12-hour ceiling; the 0.1 hour/hour rate ceiling yields an upper envelope of 12.2 hours and blocks.

All values and rate-bound evidence references are synthetic examples, not calibrated vessel data or engineering limits.

## Research context

- Pagnini & Malis, “Segment-safe control barrier functions for model predictive control,” European Journal of Control, online 23 July 2026, DOI https://doi.org/10.1016/j.ejcon.2026.101582. Their work targets inter-sample safety along segments between sampled states; its validation is on nonlinear quadrotor simulations.
- Prajapat et al., “Finite-sample-based reachability for safe control with Gaussian Process dynamics,” Automatica 193 (November 2026), article 113204, DOI https://doi.org/10.1016/j.automatica.2026.113204.
- “Robust data-driven MPC with analytic probabilistic safety barriers,” September 2026, DOI https://doi.org/10.1016/j.rico.2026.100789.
- Kondratyev et al., “Field-Validated Probabilistic Digital Twin of a Shipboard Microgrid for Reliability-Oriented Operation,” Energies 2026, 19(17), 4074, DOI https://doi.org/10.3390/en19174074. The reported evidence includes 162,360 telemetry samples at 4-second cadence across 12 event classes, but from a single operator and predominantly conventional tankers.

These are strong comparators. The present tool is not an equivalent formal-control proof: it checks a transparent declared-rate assumption on scalar mission resources/services, while the cited reachability/CBF work addresses stronger dynamical/control formulations.

## Limits

This is a declared-rate inter-sample envelope audit. It does not independently prove the physical rate ceiling, validate the underlying model, cover unregistered hybrid switches, prove recursive feasibility or a viability kernel, certify vessel safety or authorize navigation. Any real campaign requires rate-ceiling identification over the exact operating domain, uncertainty calibration, high-rate measurement, HIL/PHIL and independent review.
