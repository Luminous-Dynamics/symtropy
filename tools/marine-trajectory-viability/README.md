# Marine Finite-Horizon Trajectory Viability Audit

Standalone Rust 1.96-compatible sampled-trajectory evaluator for Symtropy #6261.

## Commands

```sh
cargo fmt --manifest-path tools/marine-trajectory-viability/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-trajectory-viability/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-trajectory-viability/Cargo.toml
cargo run --manifest-path tools/marine-trajectory-viability/Cargo.toml -- audit-trajectory tools/marine-trajectory-viability/fixtures/synthetic-trajectory-pass-0001.json
cargo run --manifest-path tools/marine-trajectory-viability/Cargo.toml -- audit-trajectory tools/marine-trajectory-viability/fixtures/synthetic-arrival-safe-midvoyage-breach-0001.json
```

## What the evaluator checks

- A digest-bound mission snapshot, eleven-axis service contract, strict shared time grid and exactly five disturbance classes.
- Conservative resource/service floors at every sampled time point, with lower bounds for minimum reserves and upper bounds for maximum exposures.
- Component availability and correlated common-cause outage identity.
- Energy, freshwater and waste-capacity conservation per interval, reconciled to the corresponding start/end resource states.
- No energy credited from an unavailable main generator; no water credited from an unavailable watermaker; no shore charge credited when shore power or the battery inverter is unavailable.
- Minimum conservative margin, time of minimum margin, first breach timestamp and maximum balance residual by resource.

The synthetic suite includes a passing five-scenario path and an adversarial path where arrival freshwater is above its floor but the reserve dips below that floor at the intermediate six-hour point. The latter must return `BLOCKED` and `INFEASIBLE/ABSTAIN`.

## Evidence boundary

This is a sampled finite-horizon audit, **not** a mathematical viability-kernel proof. It does not establish inter-sample safety, validate hydrodynamic or equipment models, estimate scenario probabilities, certify battery/watermaker performance, or authorize navigation. Physical deployment requires measured model validation, uncertainty calibration, additional disturbance coverage, independent review and HIL/PHIL testing.
