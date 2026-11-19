# Ocean Thermal Balance Verifier

A small, std-only Rust verifier for 0D steady-state energy accounting across direct seawater, partial economiser + chiller, and chiller-only modes.

## Run locally

    cargo test --manifest-path tools/ocean-thermal-balance-verifier/Cargo.toml
    cargo run --manifest-path tools/ocean-thermal-balance-verifier/Cargo.toml

The package is isolated in a nested Cargo workspace and does not change Symtropy's root workspace members.

## What it checks

IT-side balance:

    IT heat load = direct heat to seawater + chiller evaporator heat

Seawater-side balance:

    mass flow × specific heat × (discharge temperature − intake temperature)
    = direct heat + chiller evaporator heat + chiller electric input
      + declared pump heat fraction + other declared heat

Cooling electricity includes chiller, water-loop pump and separately declared other cooling auxiliaries. The verifier also checks declared site-specific maximum discharge temperature and temperature-rise limits; no universal limit is hardcoded. It reports interval energy in kWh and chiller COP when applicable.

## Important assumptions

- All power values are synchronous interval averages over one declared duration.
- There is no thermal storage change during the interval.
- The supplied direct/chiller heat split closes the IT heat load.
- The pump heat fraction is an explicit caller-supplied boundary allocation.
- Any unlisted heat path, parasitic or storage term makes the case incomplete; the verifier does not infer missing heat as zero.

## Claim ceiling

A PASS establishes only 0D arithmetic consistency under caller-supplied values and declared boundaries. It is not a heat-exchanger design, seawater intake/outfall design, CFD or hydrodynamic plume model, ecological risk model, WUE/PUE certification, environmental authorization, or field validation. The site limits must come from the applicable authority and environmental baseline.

The synthetic cases in the demo are mechanism tests, not a real site or commercial performance result.
