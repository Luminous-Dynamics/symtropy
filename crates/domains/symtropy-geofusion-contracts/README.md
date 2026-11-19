# Symtropy Geothermal–Fusion Contracts

A small, dependency-free Rust library for validating energy-stream boundary records and first-law balances used by the geothermal–fusion integration programme.

## Scope

- Distinguishes electrical and thermal carriers (MW_e/MWh_e versus MW_th/MWh_th).
- Classifies absorption-chiller drive heat, power-cycle thermal input, thermal desalination heat and reverse-osmosis electrical demand as distinct useful-service endpoints.
- Validates declared power × duration = energy.
- Requires temperature-grade data for thermal streams and checks minimum exchanger approach.
- Requires an explicitly declared isolated intermediate loop for geothermal/fusion primary heat streams.
- Rejects cross-connection of geothermal-primary and fusion-primary circuit classes.
- Rejects direct geothermal-primary heat as a D–T plasma-heating input; geothermal-generated electricity may feed a separately identified electrical plasma-heating power supply.
- Evaluates a multi-carrier first-law ledger (for example thermal input, electric output and rejected heat) while preserving the carrier label on every term. Negative storage delta represents discharge.
- Uses overflow-resistant normalized residual arithmetic and explicitly refuses unrepresentable totals/residuals rather than allowing NaN/inf to appear as a balanced result.
- Provides a source-interval allocation ledger to detect over-allocation, duplicate edge IDs and unallocated energy across multiple destinations; curtailed energy is explicit. A zero-output interval is valid when it has zero allocations and zero curtailment; negative energy remains invalid.

## Explicit non-goals

This is not a reservoir, wellbore, thermophysical-property, two-phase-flow, CFD, thermal-hydraulic, plasma, neutronics, tritium, structural, materials, exergy, grid-stability, plant-control or safety solver.

A passing contract validates only declared interface arithmetic and consistency. It does not authenticate evidence, qualify a heat exchanger, establish circuit isolation in hardware, validate source measurements, or authorize physical operation.

## Build and test

```bash
cargo test -p symtropy-geofusion-contracts
cargo clippy -p symtropy-geofusion-contracts --all-targets -- -D warnings
```

The crate is part of the public Symtropy workspace. See the [system architecture and 27-case benchmark documentation](https://github.com/Luminous-Dynamics/symtropy/pull/6143) and [GEOFUSION-INTEGRATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/6142).

## Superhot wellhead state boundary

The `superhot_well` module adds a source-bounded reservoir-to-wellhead case contract:

- Reservoir pressure, absolute temperature and permeability-thickness (mD·m) are distinct from hydraulic transmissivity (m²/s).
- Wellhead pressure, temperature, mass flow and casing inner diameter are retained at an explicit boundary.
- Fluid density and enthalpy must be linked to a named property formulation/database, version, composition, reference state, output digest and the exact pressure / temperature / phase state.
- Single-phase bulk volumetric flow and average casing velocity are computed from the declared density, mass flow and casing ID.
- Two-phase and unknown-phase flows are refused; they require a qualified multiphase hydraulics model.
- An optional velocity comparison requires a named external threshold basis. The comparison is a screening flag, not a universal erosion or safety limit.

The module computes no reservoir productivity, wellbore pressure drop, flashing, phase slip, corrosion, scaling, casing erosion, power-cycle output or net MW. Its synthetic test case is arithmetic only and is not an IDDP-1 reproduction or a real well prediction.
