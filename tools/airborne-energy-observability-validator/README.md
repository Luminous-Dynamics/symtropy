# Airborne Energy Observability Validator

A dependency-free Rust reference model for a first-pass battery-electric aircraft mission energy ledger.

## Scope

This crate evaluates a **declared** segment power profile. It does not predict aerodynamic shaft power, battery electrochemistry, voltage sag, thermal propagation, aircraft performance, or flight safety. Those must come from independently justified models and measured data.

The calculator:
- separates planned mission segments from explicit reserve segments;
- converts shaft power to DC-bus power using declared inverter and motor efficiencies;
- adds auxiliary loads at the DC-bus boundary;
- accounts for a single bounded peak interval within each segment;
- checks continuous and peak DC-bus power limits;
- estimates remaining state of charge using a linear SOC-to-energy approximation at a declared pack-DC energy boundary;
- reports energy per passenger-kilometre and per tonne-kilometre when the required inputs exist;
- returns explicit violations rather than converting infeasibility into a successful result.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/airborne-energy-observability-validator/Cargo.toml -- --check
cargo test --locked --manifest-path tools/airborne-energy-observability-validator/Cargo.toml
cargo clippy --locked --manifest-path tools/airborne-energy-observability-validator/Cargo.toml --all-targets -- -D warnings
```

The crate has its own Cargo workspace and lockfile so it can be tested independently of Symtropy's Bevy application workspace.

## Model boundary

- `rated_dc_energy_wh_at_reference` is energy measured at the battery pack DC terminals over the declared 100%-to-0% reference SOC interval, at a named reference temperature and discharge profile.
- `state_of_health` scales that measured capacity linearly. This is an initial approximation, not an electrochemical ageing model.
- `average_shaft_power_w` is the average mechanical power demanded at the propulsor shaft outside the peak interval.
- `peak_shaft_power_w` replaces average shaft power during `peak_duration_s`; it is not added on top of the average.
- `auxiliary_dc_power_w` is already expressed at the pack DC-bus boundary and must include whichever avionics, control, pumps, cooling, cabin, and conversion loads the analyst has elected to include.
- Inverter and motor efficiencies are applied to shaft power only. Propeller/aerodynamic demand must already be reflected in the declared shaft-power input.
- Reserve segments are explicit additional energy scenarios. An empty reserve profile is rejected.
- SOC is linearly related to remaining pack-DC energy. Real batteries require voltage-, temperature-, rate-, imbalance-, and chemistry-aware models before any sizing decision.

## Maturity

This is an analytical bookkeeping model with unit tests. It is not a validated aircraft digital twin, qualified battery model, certification artifact, or operational safety control.
