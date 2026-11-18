# HAPS Fleet Reliability

A standalone Rust evaluator for exact static Bernoulli service-availability screens with independent node states, shared common-cause events and shared dependencies.

This is deliberately not a semi-Markov, repair, maintenance, weather-forecast or flight model. It rejects temporal analysis methods rather than silently treating them as static probabilities.

## Run

```sh
cargo fmt --manifest-path tools/haps-fleet-reliability/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml
cargo run --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml -- \
  docs/research/aviation/examples/synthetic_haps_fleet_common_cause_reliability_v1.json
```

## Model

Each scenario combines:
- one independent Bernoulli availability state per HAPS node;
- one independent Bernoulli state per common-cause event, which disables every affected node;
- one independent Bernoulli state per shared dependency.

A service is available when at least the declared number of nodes remain active and every required shared dependency is up. The report also emits a single-node comparator and a deliberately naïve k-out-of-n estimate that ignores common-cause events.

Exact enumeration is bounded to 20 binary variables (1,048,576 scenarios). Probability mass must sum to 1 within tolerance.

## Evidence boundary

The checked-in fleet cases are synthetic. The 32-scenario fixtures are known-answer arithmetic tests, not measured HAPS or fleet uptime, a 30-day observation, a repair-time model, or a semi-Markov result. The published multi-HAPS semi-Markov reliability paper remains a baseline to reproduce on a matched model boundary.
