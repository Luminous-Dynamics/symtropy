# symtropy-oli-abiotic

Deterministic synthetic abiotic mechanism generators for the Open-World Life & Intelligence benchmark.

## First generator: Gray–Scott reaction–diffusion

Generator ID: `OLI-ABIOTIC-GRAY-SCOTT-001`.

Evidence class: `SYNTHETIC_KNOWN_TRUTH`.

The reference implements the two-field Gray–Scott reaction–diffusion update on a periodic 2D grid. The initial perturbation is deterministic and seeded. The reference profile uses a 64×64 grid, the canonical seeded patch, explicit Euler integration and 500 steps.

A matched **diffusion-only** control uses the same initial condition, timestep and diffusion coefficients while disabling reaction/feed/kill terms.

The benchmark reports spatial concentration means/variances and extrema. It does not infer life from patterns, and does not treat visual spot formation as reproduction or heredity.

## Numerical contract

For the explicit 2D five-point Laplacian, the configured diffusion update is rejected if \(D\,dt > 1/4\). Invalid/non-finite parameters and concentration values outside the declared [0, 1] physical range are rejected transactionally: a failed step does not partially mutate the grid.

The reaction and forcing parameters are separately declared. Removing feed is an intervention whose outcome is measured, not assumed.

## Run

```bash
cargo test -p symtropy-oli-abiotic --locked
```


## Second generator: phase-oscillator synchronization controls

Generator ID: `OLI-ABIOTIC-OSCILLATOR-001`.

The phase-oscillator reference compares three matched causal conditions:
- **Coupled-only:** pairwise coupling enabled, external forcing disabled.
- **Common-input-only:** pairwise coupling disabled, shared external forcing enabled.
- **Independent:** both coupling and external forcing disabled.

The 32 oscillators share identical deterministic initial phases and a fixed natural-frequency population across conditions. The reference profile uses 5,000 simultaneous explicit-Euler updates at \(dt=0.01\).

For the frozen profile, both coupled-only and common-input-only populations achieve an order parameter above 0.99, while the independent population remains below 0.30. Critically, the common-input-only population reaches this synchronization with **coupling strength exactly zero**. This demonstrates that visual/phase synchrony alone cannot establish communication or collective cognition.

Metrics are kept separate:
- instantaneous Kuramoto order parameter;
- mean absolute pairwise phase difference;
- time-averaged phase-lag coherence relative to an external driver;
- mean phase lag to the driver;
- declared coupling and forcing parameters.

External phase locking is `null` when the driver is disabled. It is not computed by merely rotating the instantaneous phase vector: the implementation averages the relative phase over time, because instantaneous order-parameter magnitude is invariant to a global phase rotation.

The phase increment contract includes natural frequency, driver frequency, coupling and forcing bounds. It is a declared temporal-resolution guard, not a claim of a universal nonlinear stability theorem.

## Claim ceiling

This is one discrete synthetic reaction–diffusion mechanism. It establishes only that the declared model can produce spatial organization under the tested update rules and parameter profile. It does not establish that natural reaction–diffusion systems are non-living, or that any observed physical pattern is life, reproduction, agency, intelligence or consciousness.
