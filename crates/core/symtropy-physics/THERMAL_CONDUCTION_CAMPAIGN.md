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
bodies at each campaign end time. The executable obtains `C_A` and `C_B` through
the core finite-derived-capacity validation path, then evaluates the closed-form
reference with normalized capacity weights and `G/C_A + G/C_B` to avoid avoidable
overflow in the reference arithmetic. The analytical reference remains independent
of the numerical conduction update itself.

## What the executable checks

For four successively halved timesteps, the campaign records:

- maximum absolute temperature error against the closed-form transient
- strict complete-accounting first-law closure error
- relative first-law closure error
- maximum per-reservoir state-versus-ledger reconciliation residual
- relative per-reservoir reconciliation residual
- minimum entropy production over every committed conduction step
- closed-pair entropy change over the full interval
- number of conductive ledger entries
- number of steps that invoked the equilibrium safety limiter
- observed temporal convergence order between adjacent resolutions

The executable fails if:

1. Either endpoint cannot supply complete, finite modeled sensible-energy accounting.
2. Relative first-law closure exceeds `1e-12`.
3. Per-reservoir state-versus-ledger reconciliation exceeds `1e-12` relative to
   the initial modeled-energy scale.
4. Any committed passive-conduction step violates the constant-`c_p` second-law
   entropy check beyond `1e-12 J/K`.
5. The full closed-pair interval violates the same second-law check.
6. Any convergence case invokes the equilibrium limiter. The limiter is a Tier-A
   safety mechanism; this Tier-B campaign is intended to measure the unconstrained
   explicit transient.
7. Transient temperature error does not monotonically improve when the timestep
   is halved.
8. Observed temporal order falls outside `[0.8, 1.2]`, the predeclared envelope
   around the expected first-order explicit update.

## Why reservoir reconciliation matters

A total-energy sum can close even when bookkeeping assigns equal-and-opposite
energy changes to the wrong reservoirs. Therefore the campaign separately checks
for each thermal body

`measured reservoir delta - ledger net delta = residual`.

This binds the state transition to the causal ledger rather than treating a small
global first-law residual as sufficient evidence by itself.

Likewise, checking entropy only at the beginning and end could hide a locally
unphysical step that is compensated later. The campaign therefore evaluates the
second-law diagnostic after every successfully committed audited conduction step
as well as across the complete interval.

The campaign intentionally does **not** claim validation of spatial diffusion,
contact-area conductance, temperature-dependent properties, phase change,
radiation, convection, fluid advection, or general solver-coupled thermodynamics.

## Why this campaign matters

A visually plausible temperature curve is not sufficient evidence for a thermal
solver. This case simultaneously tests four independent requirements:

- **first law:** the isolated pair does not create or destroy modeled energy;
- **causal accounting:** each body's measured energy change agrees with its ledger
  entries, not merely with the global sum;
- **second law:** every committed passive-conduction step and the full interval
  have non-negative entropy production within tolerance;
- **numerical convergence:** the transient approaches an independent analytical
  solution at the expected first-order rate as temporal resolution increases.

The same evidence shape should be reused for later thermal regimes: declare the
validity domain, compare to an independent reference, expose complete accounting
and per-reservoir residuals, and show convergence rather than selecting one
attractive timestep.
