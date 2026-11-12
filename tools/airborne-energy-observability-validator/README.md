# Airborne Energy Observability Validator

An isolated Rust package for two separate research contracts:

1. **Receiver-pose observability** — finite synthetic field/receiver pose maps, alias detection, and explicit claim ceilings.
2. **Mission energy ledger** — deterministic battery-electric segment energy accounting, explicit reserve segments, and continuous/peak DC-bus power screening.

The modules are intentionally separate. A uniquely observable synthetic receiver map does not qualify a wireless-power-transfer aircraft system; a mission energy ledger does not validate an RF/optical link or an aircraft.

## Run from the repository root

```sh
cargo fmt --all -- --check
cargo test --locked -p airborne-energy-observability-validator
cargo clippy --locked -p airborne-energy-observability-validator --all-targets -- -D warnings
```

## Mission-energy model boundary

The v0.1 model is an analytical bookkeeping reference, not an aircraft performance or battery electrochemistry simulator.

- Shaft-power inputs must come from a separately justified aerodynamic/propulsion model or measurement.
- Motor and inverter efficiencies convert propulsor-shaft power to pack DC-bus power.
- Auxiliary loads are already measured at the pack DC-bus boundary.
- An explicit reserve segment is mandatory; an empty reserve profile is rejected.
- Battery energy is supplied at the pack DC-terminal boundary and scales linearly with state of health and SOC in this first approximation.
- One peak interval per segment replaces average shaft power for its declared duration.
- Voltage sag, current limits, thermal propagation, ageing physics, non-linear SOC curves, generator/hybrid flows, and uncertainty propagation are not implemented yet.

## Evidence maturity

Unit tests demonstrate deterministic calculation and input/contract rejection only. They are not physical validation, HIL evidence, flight testing, certification evidence, or permission to operate an aircraft.
