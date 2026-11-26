# Q-drive relativistic exhaust reference

This package provides a small, dependency-free analytical kernel for:

- the relativistic four-momentum of a particle;
- Lorentz boosts between explicitly defined inertial frames;
- invariant rest-mass recovery;
- relativistic momentum and energy carried by a steady exhaust stream.

It is a deliberately narrow submodel for the research contract in [Symtropy #7649](https://github.com/Luminous-Dynamics/symtropy/issues/7649). It is **not** a complete q-drive simulator or a physical plasma-interaction model.

## Rate convention

The exhaust mass-flow input is rest mass crossing the nozzle exit per unit coordinate time in the instantaneous vehicle frame. The exhaust velocity vector is also expressed in that frame.

For speed (u), (eta=u/c), and (gamma=(1-eta^2)^{-1/2}):

```text
momentum_flux = mdot * gamma * u
total_energy_flux = mdot * gamma * c^2
rest_mass_energy_flux = mdot * c^2
kinetic_power = mdot * (gamma - 1) * c^2
```

The force on the vehicle is opposite the exhaust momentum flux. For (u/c \ll 1), the kernel must recover:

```text
thrust -> mdot * u
kinetic_power -> 0.5 * mdot * u^2
```

The particle four-momentum uses the (+---) metric and is represented as energy plus a three-vector momentum. Lorentz boosts transform the particle four-momentum, not a casually defined power or mass-flow rate. Rate transformations require the appropriate flux/current and boundary treatment.

## Explicit non-goals

This package does not model:

- plasma kinetic theory, MHD or a plasma-magnet interaction cross-section;
- harvested power, drag, field storage or field momentum;
- the external medium's energy/momentum recoil;
- radiated photons or complete control-volume boundary flux;
- power conversion, thruster hardware efficiency, tankage or radiator mass;
- vehicle mass evolution, trajectory, braking or mission delta-v;
- a physically demonstrated q-drive.

A successful test run establishes only mathematical and software consistency of these analytical kernels. It does not show that an interaction can harvest the required energy or momentum, and it must not be used to raise the Newtonian speed ceiling in the v0 screening package.

## Validation

The test suite covers source-independent analytical gamma values at 0.05c, 0.1c and 0.2c, low-speed Newtonian limits, energy/momentum separation, Lorentz-invariant rest mass, general three-axis boosts, zero-flow controls, vector direction, and invalid/superluminal input rejection.

The exact-head CI workflow runs rustfmt, unit tests and strict Clippy. No test outcome is implied by a queued workflow.
