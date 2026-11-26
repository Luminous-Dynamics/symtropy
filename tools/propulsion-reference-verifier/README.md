# Propulsion analytic reference verifier (v0.1)

A standalone Rust verifier for two bounded analytical identities: the ideal rocket equation and the rocket nozzle thrust equation.

## Run

From the Symtropy repository root:

```bash
cargo test --manifest-path tools/propulsion-reference-verifier/Cargo.toml
cargo run --manifest-path tools/propulsion-reference-verifier/Cargo.toml -- docs/space-engineering/reference-cases/ideal-rocket-thrust-delta-v-v0.1.json
```

The executable emits a deterministic JSON receipt containing the exact input SHA-256.

## What it checks

- Ideal rocket equation: Δv = Isp × g0 × ln(m0 / mf).
- Rocket thrust: F = mdot × ve + Ae × (pe − pa).
- Specific impulse: Isp = F / (mdot × g0).
- Finite, positive and unit-labelled SI reference inputs.
- Source records and an explicit analytical-only claim ceiling.

## What it does not do

- No combustion, injector, turbomachinery, feed-system, regenerative-cooling, CFD, structural, fatigue or materials model.
- No engine sizing, engine design, safety assessment, hardware test prediction or flight qualification.
- These illustrative numbers are not attributed to Prometheus, RAMPT or any real engine.

References:
- NASA Glenn, Ideal Rocket Equation: https://www1.grc.nasa.gov/beginners-guide-to-aeronautics/ideal-rocket-equation/
- NASA Glenn, Rocket Thrust Equation: https://www1.grc.nasa.gov/beginners-guide-to-aeronautics/rocket-thrust-equation/
