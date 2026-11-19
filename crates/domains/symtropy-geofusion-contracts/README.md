# Symtropy Geothermal–Fusion Contracts

A small, dependency-free Rust library for validating energy-stream boundary records and first-law balances used by the geothermal–fusion integration programme.

## Scope

- Distinguishes electrical and thermal carriers (MW_e/MWh_e versus MW_th/MWh_th).
- Validates declared power × duration = energy.
- Requires temperature-grade data for thermal streams and checks minimum exchanger approach.
- Requires an explicitly declared isolated intermediate loop for geothermal/fusion primary heat streams.
- Rejects cross-connection of geothermal-primary and fusion-primary circuit classes.
- Rejects direct geothermal-primary heat as a D–T plasma-heating input; geothermal-generated electricity may feed a separately identified electrical plasma-heating power supply.
- Evaluates a multi-carrier first-law ledger (for example thermal input, electric output and rejected heat) while preserving the carrier label on every term. Negative storage delta represents discharge.
- Uses overflow-resistant normalized residual arithmetic and explicitly refuses unrepresentable totals/residuals rather than allowing NaN/inf to appear as a balanced result.

## Explicit non-goals

This is not a reservoir, wellbore, thermophysical-property, two-phase-flow, CFD, thermal-hydraulic, plasma, neutronics, tritium, structural, materials, exergy, grid-stability, plant-control or safety solver.

A passing contract validates only declared interface arithmetic and consistency. It does not authenticate evidence, qualify a heat exchanger, establish circuit isolation in hardware, validate source measurements, or authorize physical operation.

## Build and test

```bash
cargo test -p symtropy-geofusion-contracts
cargo clippy -p symtropy-geofusion-contracts --all-targets -- -D warnings
```

The crate is part of the public Symtropy workspace. See the programme architecture and system-level benchmark at [docs/research/geothermal-fusion](../../../docs/research/geothermal-fusion/README.md) on the integration documentation branch and [GEOFUSION-INTEGRATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/6142).
