# Symtropy Thermodynamics Validation Contract

This document defines the research and engineering contract for thermodynamics in
`symtropy-physics`. The goal is not merely to make objects become visually hot or
cold. Thermal behavior should participate in the same deterministic, auditable
physical world as mechanics and should expose explicit energy transfers and
well-defined validity limits.

The thermodynamic layer is experimental until the analytical campaigns below are
completed. Public claims must distinguish implemented primitives from validated
physical regimes.

## Current modeled scope

The initial thermodynamic foundation models lumped sensible heat with constant
material properties:

- Absolute temperature in kelvin.
- Constant specific heat capacity `c_p`.
- Constant thermal conductivity metadata.
- Surface emissivity metadata reserved for future radiation.
- Explicit effective thermal mass for body-attached thermal state.
- Pairwise conductive exchange through an effective conductance `UA` in W/K.
- Conservative pair transfer: heat removed from one participant is added to the
  other participant.
- Equilibrium limiting so one large explicit step cannot cross the pair's thermal
  equilibrium and reverse temperature ordering.
- Bitwise snapshot coverage for body-attached thermal state.
- Revalidation of material, state, and body thermal reservoirs after public-field
  mutation.
- Explicit diagnostic reporting of invalid attached thermal reservoirs instead of
  silently treating them as zero energy.
- World diagnostics for modeled sensible thermal energy and combined mechanical
  plus thermal energy.

The sensible-energy model is

`E_sensible = m c_p (T - T_ref)`.

For the current invariant snapshot, `T_ref = 0 K` is used as a deterministic
accounting reference. This is a modeled energy budget, not a claim that constant
`c_p` extrapolated to absolute zero represents exact physical internal energy.

## State validity and accounting completeness

Construction is not permanent proof that an attached thermal reservoir remains
valid. Thermal material/state fields are currently public, so callers can mutate a
previously valid value into a finite but unphysical one after construction.

Therefore every accounting, coupling, serialization, or validation boundary that
relies on thermal state must distinguish three cases:

1. no thermal reservoir is attached;
2. a valid thermal reservoir is attached, including the valid special case `T = 0 K`;
3. a thermal reservoir is attached but invalid under the current model contract.

These cases are not interchangeable. In particular, an invalid reservoir must never
be silently omitted and interpreted as zero energy, and an absent reservoir must not
be considered bitwise-equivalent to a present reservoir at `0 K`.

`ThermalMaterial::validate`, `ThermalState::validate`, and `ThermalBody::validate`
exist to re-check state after construction. `PhysicsWorld::invariant_snapshot()`
reports `invalid_thermal_body_count`; modeled energy accounting is complete only when
that count is zero. If any attached reservoir is invalid, the reported modeled
thermal/total energy may be useful for diagnosis but is **partial accounting** and
must not be used as evidence that a first-law conservation gate passed.

This is an epistemic invariant as much as a numerical one:

`invalid reservoir != zero-energy reservoir != absent reservoir`.

## Not yet certified

The following capabilities are intentionally outside the current validity envelope:

- Temperature-dependent heat capacity or conductivity.
- Anisotropic heat conduction.
- Spatial conduction within extended rigid bodies.
- Contact-area-derived conductance.
- Convective heat transfer.
- Thermal radiation and view factors.
- Fluid heat advection or compressible thermodynamics.
- Latent heat, melting, freezing, boiling, condensation, or sublimation.
- Multi-phase mixtures.
- Thermal expansion and thermoelastic stress.
- Temperature-dependent strength, friction, viscosity, or fracture toughness.
- Mechanical dissipation converted into heat.
- Chemical reactions, combustion, ionization, or plasma physics.
- Entropy transport across open boundaries.
- Relativistic or quantum thermodynamic regimes.

No gameplay system should imply that one of these effects is physically modeled
merely because a temperature field exists.

## Conservation contract: first law

Every closed thermodynamic scenario should declare the energy reservoirs it
contains and the boundary terms it permits.

Before evaluating a first-law residual, the scenario must establish that modeled
energy accounting is complete for the declared reservoirs. For body-attached
thermal state, this requires zero invalid thermal bodies. A conservation residual
computed from a snapshot with incomplete modeled-energy accounting is invalid as a
pass/fail result regardless of how small the numerical residual appears.

For a closed mechanical-plus-thermal system, the target budget is

`Delta(E_mechanical + E_internal) = 0`

up to declared numerical tolerance.

For an open system,

`Delta E_system = Q_boundary + W_boundary + E_mass_in - E_mass_out`.

Future solvers should record these terms explicitly instead of silently creating
or deleting energy. Numerical stabilization, clipping, solver correction, and
reduced-order transitions must also expose their contribution when it is large
enough to affect the budget.

### Pairwise conduction invariant

For two finite lumped bodies A and B with no external source,

`Q_A + Q_B = 0`.

The final equilibrium temperature has the analytical solution

`T_eq = (C_A T_A0 + C_B T_B0) / (C_A + C_B)`

where `C = m c_p`.

The implementation must never produce a one-step temperature crossing in which
A begins hotter than B and ends colder than B solely because the timestep is too
large.

## Directionality contract: second law

Energy conservation alone is insufficient. A numerically conservative solver can
still transfer heat in an unphysical direction.

For isolated passive conduction:

- Heat must flow from hotter material toward colder material.
- Equal-temperature bodies must exchange zero net heat.
- Temperatures must approach a common equilibrium without crossing it in a single
  monotone pair update.
- Total entropy for the closed pair should not decrease beyond numerical tolerance.

For constant heat capacities, pair entropy change can be evaluated as

`Delta S = C_A ln(T_A1 / T_A0) + C_B ln(T_B1 / T_B0)`.

A future validation harness should report both energy residual and entropy change.
A first-law pass with a significant negative entropy change is a physics failure,
not a success.

## Validation tiers

### Tier A — local thermodynamic identities

Required unit and property tests include:

1. Reject non-finite and sub-zero absolute temperatures.
2. Reject zero or negative thermal mass and heat capacity.
3. Reject negative conductance and invalid emissivity.
4. Revalidation catches invalid post-construction mutation of thermal material,
   state, and thermal mass.
5. Pair conduction conserves sensible energy.
6. Heat-transfer sign matches temperature ordering.
7. Equal temperatures produce zero transfer.
8. Large timesteps stop at pair equilibrium.
9. Body snapshots change bitwise when thermal state changes.
10. Mechanically identical snapshots distinguish absent thermal state from an
    attached valid `0 K` reservoir.
11. Invalid attached reservoirs increment the explicit diagnostic invalid count,
    make numerical health fail, and mark modeled energy accounting incomplete.
12. Combined invariant accounting changes by exactly the applied external heat in
    a source-only test once the audited external-source API exists.

### Tier B — analytical solutions

The first analytical thermodynamics campaign should contain at least:

1. **Two-lump equilibration**
   - unequal masses and heat capacities
   - compare equilibrium temperature with the exact weighted solution
   - measure energy residual and entropy change

2. **Two-lump transient exchange**
   - constant pair conductance
   - compare temperature difference against exponential decay
   - demonstrate temporal convergence over at least four timesteps

3. **Single lump with prescribed heat input**
   - constant source power
   - verify `Delta T = P t / (m c_p)`

4. **1D slab conduction** after a spatial thermal discretization exists
   - compare with an analytical Fourier-series or independently solved reference
   - report spatial and temporal convergence

5. **Newton cooling** after convection exists
   - compare with the lumped analytical exponential solution

6. **Radiative cooling** after radiation exists
   - compare against a high-accuracy independent integration of the
     Stefan-Boltzmann equation

7. **Stefan phase-change problem** after latent heat exists
   - compare moving phase boundary position with the classical reference solution

### Tier C — independent implementation comparison

For each mature thermal regime, compare against at least one independent solver or
reference implementation. Suitable references may include validated finite-volume,
finite-element, multiphysics, or high-precision numerical implementations.

The comparison should match boundary conditions, material laws, geometry, and
resolution rather than comparing visually similar scenes.

### Tier D — reproducible campaign

Thermal campaigns follow the general `RESEARCH_VALIDATION.md` requirements and
must additionally record:

- temperature units and absolute/reference convention
- material property source and assumed temperature range
- thermal mass or cell volume/density
- boundary conditions
- conductance derivation
- heat-source and heat-sink histories
- radiation environment where applicable
- phase model and latent heat values where applicable
- modeled-energy accounting completeness / invalid-reservoir count
- energy residual per step and over the full run
- entropy change where the scenario permits a closed-system check

## Stability and resolution

A thermal solver must state the numerical stability assumptions of its integration
scheme. Spatially resolved diffusion should report the Fourier number

`Fo = alpha dt / dx^2`

with thermal diffusivity

`alpha = k / (rho c_p)`.

Explicit diffusion solvers must enforce or adapt around their method's stability
limit. Adaptive substepping should be deterministic for a fixed state and should
be visible in diagnostics.

For convection and fluid advection, the relevant CFL-like constraints must be
reported once those solvers exist.

## Material-state architecture

Long term, Symtropy should distinguish three concepts:

1. **Composition** — what the material is made of.
2. **Thermodynamic state** — temperature, phase fractions, and later pressure or
   other state variables where the solver requires them.
3. **Constitutive behavior** — how mechanical, thermal, fluid, and fracture
   properties depend on composition and state.

This avoids encoding "hot steel", "molten steel", and "cold steel" as unrelated
materials. Temperature and phase should alter constitutive behavior while the
material identity remains traceable.

## Preferred phase-change formulation

When phase change is introduced, prefer an enthalpy-based state variable over
manually clamping temperature at phase boundaries.

A useful modeled quantity is

`H = sensible_energy + latent_energy`.

Temperature and phase fraction are then derived from enthalpy and material phase
relations. This makes latent heat part of the energy budget and gives melting,
freezing, boiling, and condensation a common accounting path.

The first implementation should support one-component solid/liquid transitions
before attempting multi-component mixtures or chemistry.

## Thermo-mechanical coupling roadmap

The recommended coupling order is:

1. Body-attached lumped thermal state.
2. Explicit external heat-source API and energy accounting.
3. Contact conduction using deterministic contact geometry.
4. Frictional and inelastic dissipation routed into thermal energy.
5. Spatial thermal cells for terrain and extended bodies.
6. Temperature-dependent material properties.
7. Enthalpy-based phase change.
8. Thermal expansion and thermoelastic stress.
9. Temperature-dependent fracture, yielding, and viscosity.
10. Fluid thermal advection and convection.
11. Radiation.
12. Chemistry only after the lower layers have independent validation.

This ordering is deliberate: later effects depend on trustworthy accounting in the
lower layers.

## Cross-solver energy transfers

The long-term engine should treat solver transitions as explicit energy transfers.
Examples include:

- rigid-body friction -> thermal energy
- fracture work -> new-surface energy + heat
- fluid viscosity -> thermal energy
- plastic deformation -> stored defect energy + heat
- phase change -> latent energy
- radiation -> environment energy exchange
- combustion -> chemical energy -> heat + mechanical work

No solver should silently discard a resolved energy channel if the lost energy is
large enough to matter to the modeled scale.

This is the foundation for a causal matter engine: not every joule must be tracked
at microscopic fidelity, but every modeled macroscopic transfer should have an
explicit destination, source, or declared approximation.

## Initial acceptance thresholds

Thresholds should be tightened empirically after convergence studies, but the
first campaign should predeclare targets rather than choosing them after seeing the
results.

Suggested starting gates for deterministic lumped tests using `f64`:

- Complete modeled-energy accounting for every declared attached thermal reservoir;
  `invalid_thermal_body_count == 0` before a first-law residual can pass.
- Pair energy residual: `|Delta E| / max(|E0|, 1 J) <= 1e-12`.
- Equilibrium temperature analytical error: relative error `<= 1e-12` for a
  directly limited pair step.
- No non-finite thermal state.
- No temperature below absolute zero.
- No negative entropy production for closed passive conduction beyond a small
  floating-point tolerance.
- Transient convergence must improve monotonically as timestep is reduced across
  at least four resolutions.

These are validation targets for small analytical cases, not universal runtime
error guarantees for large coupled simulations.

## Research backlog

After the initial lumped foundation, the highest-value experiments are:

1. Two-body equilibration first-law + second-law campaign.
2. Deterministic external heat-source replay test.
3. Thermal attachment/replacement/removal ownership semantics bound to the energy
   ledger so reservoirs cannot disappear without provenance.
4. Contact-conduction convergence study.
5. Mechanical collision/friction dissipation-to-heat budget closure.
6. 1D spatial conduction campaign.
7. Enthalpy/Stefan phase-change campaign.
8. Thermoelastic expansion benchmark.
9. Temperature-dependent fracture benchmark.
10. Thermally advected fluid benchmark.
11. Radiation benchmark.

A spectacular demo is useful only after these small cases show that the same
couplings obey their declared conservation and convergence contracts.
