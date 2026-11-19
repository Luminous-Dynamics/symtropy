# Ocean Thermal Balance Verifier

A small, std-only Rust verifier for 0D steady-state energy accounting across direct seawater, partial economiser + chiller, and chiller-only modes.

## Run locally

    cargo test --manifest-path tools/ocean-thermal-balance-verifier/Cargo.toml
    cargo run --manifest-path tools/ocean-thermal-balance-verifier/Cargo.toml

The package is isolated in a nested Cargo workspace and does not change Symtropy's root workspace members.

## What it checks

IT-side balance:

    IT heat load = direct heat to seawater + chiller evaporator heat
                   + heat diverted to a named recovery sink

Recovery-path balance:

    recovered source heat = useful heat delivered to sink
                             + loss to seawater + loss to ambient

Seawater-side balance:

    mass flow × specific heat × (discharge temperature − intake temperature)
    = direct IT heat + recovery loss to seawater + chiller evaporator heat
      + chiller electric input + declared pump heat fraction + other heat

Cooling electricity includes chiller, water-loop pump and separately declared other cooling auxiliaries. The verifier also checks declared site-specific maximum discharge temperature and temperature-rise limits; no universal limit is hardcoded. It reports interval energy in kWh and chiller COP when applicable.

## Important assumptions

- All power values are synchronous interval averages over one declared duration.
- There is no thermal storage change during the interval.
- The supplied direct/chiller/recovery split closes the IT heat load.
- Direct heat recovery requires a named sink and delivery-observation reference. The sink record is carried, not authenticated.
- Recovered heat is a direct heat-exchanger path only; heat pumps and ORCs require separate electrical-input/conversion accounting and are not modeled here.
- The pump heat fraction is an explicit caller-supplied boundary allocation.
- Any unlisted heat path, parasitic or storage term makes the case incomplete; the verifier does not infer missing heat as zero.

## Claim ceiling

A PASS establishes only 0D arithmetic consistency under caller-supplied values and declared boundaries. It is not a heat-exchanger design, seawater intake/outfall design, CFD or hydrodynamic plume model, ecological risk model, WUE/PUE certification, environmental authorization, or field validation. The site limits must come from the applicable authority and environmental baseline.

Frozen known-answer inputs and expected outputs are stored in `docs/research/port-energy-compute/OCEAN_THERMAL_BALANCE_GOLDEN_FIXTURE_V0_2.json`. It covers three cooling-only modes and one direct heat-recovery-to-sink mode. An independent arithmetic check reproduced the 10 MW recovery case as 6.27 MW useful heat, 1.5675 MW recovery loss to seawater and 2.1625 MW residual cooling rejection, closing exactly.

The synthetic cases in the demo are mechanism tests, not a real site or commercial performance result.
