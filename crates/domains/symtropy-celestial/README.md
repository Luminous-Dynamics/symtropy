# symtropy-celestial

Deterministic Newtonian N-body reference dynamics for celestial-mechanics validation.

## Why a separate crate?

The main Symtropy physics engine is designed around rigid bodies, collision detection, joints and replay. Those capabilities do not automatically make it a gravitational orbital or cosmological simulator. This crate establishes a small, auditable numerical reference model before attempting an adapter into the generic engine or a faster backend.

## Model

- SI units: mass in kg, position in m, velocity in m/s, time in s.
- Newtonian pairwise gravity with the declared value of G.
- Direct O(N²) force calculation, intended for small-N validation rather than survey-scale simulation.
- Second-order velocity-Verlet integration.
- Optional Plummer softening with a matching potential-energy formula.
- Energy, linear momentum, angular momentum, center-of-mass and step/time diagnostics.
- Fail-closed validation for invalid masses, duplicated IDs, non-finite states, invalid time steps, singular zero-softening configurations and numerical overflow.
- Candidate state is computed before commit so a failed step does not partially mutate the system.

## Reference tests

The test suite includes an equal-mass circular binary whose period is given by the Newtonian two-body solution:

\[
T = 2\pi\sqrt{\frac{a^3}{G(m_1+m_2)}}.
\]

It measures one-period orbital closure, energy drift, linear/angular momentum and center-of-mass conservation, timestep refinement, invalid inputs, and atomic failure on singular configurations.

Run the focused tests from the Symtropy workspace:

```bash
cargo test -p symtropy-celestial
```

## Benchmark ladder

The implementation-level assertions and not-yet-run independent-validation gates are tracked in [`docs/research/CELESTIAL_DYNAMICS_BENCHMARK_LADDER_2026.md`](../../../docs/research/CELESTIAL_DYNAMICS_BENCHMARK_LADDER_2026.md). The reference solver is not qualified for chaotic long-horizon prediction, planetary ephemerides, relativity, or cosmological N-body use by the two-body fixtures alone.

## Scientific boundaries

This first increment is a Newtonian reference integrator. It is not general relativity, an ephemeris, a relativistic close-encounter solver, a collision or tidal model, a cosmological N-body code, or an implementation of dark-matter structure formation. Positive softening changes the physical force law and must be recorded in a benchmark configuration.

The first gate is agreement with analytic two-body solutions and expected convergence. Only after that passes should we compare trajectories with established scientific codes, add planetary-system fixtures, or build a validated adapter to Symtropy's generic physics world.
