# Airborne Energy Observability Validator

An isolated Rust package for three separate research contracts:

1. **Receiver-pose observability** — finite synthetic field/receiver pose maps, alias detection, and explicit claim ceilings.
2. **Mission energy ledger** — deterministic battery-electric segment energy accounting, explicit reserve segments, and continuous/peak DC-bus power screening.
3. **Fixed-wing cruise estimator** — tropospheric ISA atmosphere plus a supplied steady-level drag polar to estimate lift, drag, useful thrust power, and propulsor shaft power.

The modules are intentionally separate. A uniquely observable synthetic receiver map does not qualify a wireless-power-transfer aircraft system; a mission energy ledger does not validate an RF/optical link or an aircraft.

## Run from the repository root

```sh
cargo fmt --all -- --check
cargo test --locked -p airborne-energy-observability-validator
cargo clippy --locked -p airborne-energy-observability-validator --all-targets -- -D warnings
```

## Fixed-wing cruise model boundary

`fixedwing::standard_atmosphere` implements the tropospheric ISA approximation from 0 to 11,000 m. `fixedwing::estimate_steady_level_cruise` uses declared aircraft mass, wing area, aspect ratio, Oswald efficiency, zero-lift drag coefficient, maximum lift coefficient, propulsive efficiency, altitude, and true airspeed.

The estimator assumes steady, level, unaccelerated flight and computes lift coefficient, induced drag, total drag, useful thrust power, and shaft power. It uses a simplified parabolic drag polar. Polar values and propulsive efficiency must be sourced from measured data or a qualified reference. This is a low-fidelity analytic estimate, not a validated aero model.

It does not model climb/descent, manoeuvres, takeoff, stall dynamics, VTOL hover, transition, trim drag, compressibility, Reynolds effects, propwash, aeroelasticity, structural margins, or certification criteria. It is not valid as a stand-alone design release calculation.

## Mission-energy model boundary

The v0.1 model is an analytical bookkeeping reference, not an aircraft performance or battery electrochemistry simulator.

- Shaft-power inputs must come from physical measurements or a separately justified aerodynamic/propulsion model. The included fixed-wing cruise estimator is an initial analytical option, not independently validated.
- Motor and inverter efficiencies convert propulsor-shaft power to pack DC-bus power.
- Auxiliary loads are already measured at the pack DC-bus boundary.
- An explicit reserve segment is mandatory; an empty reserve profile is rejected.
- Battery energy is supplied at the pack DC-terminal boundary and scales linearly with state of health and SOC in this first approximation.
- One peak interval per segment replaces average shaft power for its declared duration.
- Voltage sag, current limits, thermal propagation, ageing physics, non-linear SOC curves, generator/hybrid flows, and uncertainty propagation are not implemented yet.

## Evidence maturity

Unit tests demonstrate deterministic calculation and input/contract rejection only. They are not physical validation, HIL evidence, flight testing, certification evidence, or permission to operate an aircraft.
