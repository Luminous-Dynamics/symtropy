# Thermal Conduction Analytical Campaign

This campaign is the first Tier-B thermodynamics validation executable for
`symtropy-physics`.

Run it from the repository root with:

```bash
cargo run --release -p symtropy-physics --example thermal_conduction_validation
```

The executable prints CSV suitable for archiving as research evidence or feeding
an external analysis script.

## Physical model

Two finite lumped thermal bodies exchange sensible heat through a constant pair
conductance `G` with no external heat or work crossing the accounting boundary.
Each body has constant heat capacity

`C = m c_p`.

The temperature difference obeys

`d(T_A - T_B)/dt = -G (1/C_A + 1/C_B) (T_A - T_B)`

with closed-form solution

`Delta T(t) = Delta T(0) exp[-G (1/C_A + 1/C_B)t]`.

The common equilibrium temperature is

`T_eq = (C_A T_A0 + C_B T_B0) / (C_A + C_B)`.

Those two identities uniquely determine the analytical temperatures of both
bodies at each campaign end time.

## What the executable checks

For four successively halved timesteps, the campaign records:

- maximum absolute temperature error against the closed-form transient
- first-law energy closure error from the double-entry energy ledger
- relative first-law closure error
- closed-pair entropy change
- number of conductive ledger entries
- observed temporal convergence order between adjacent resolutions

The executable fails if:

1. Relative first-law closure exceeds `1e-12`.
2. The closed passive pair violates the constant-`c_p` second-law entropy check
   beyond `1e-12 J/K`.
3. Transient temperature error does not monotonically improve when the timestep
   is halved.

The campaign intentionally does **not** claim validation of spatial diffusion,
contact-area conductance, temperature-dependent properties, phase change,
radiation, convection, or fluid advection.

## Why this campaign matters

A visually plausible temperature curve is not sufficient evidence for a thermal
solver. This case simultaneously tests three independent requirements:

- **first law:** the isolated pair does not create or destroy modeled energy;
- **second law:** passive conduction does not reduce total entropy;
- **numerical convergence:** the transient approaches an independent analytical
  solution as temporal resolution increases.

The same evidence shape should be reused for later thermal regimes: declare the
validity domain, compare to an independent reference, expose conservation
residuals, and show convergence rather than selecting one attractive timestep.
