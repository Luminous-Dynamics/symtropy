# Ocean Wave Reference Oracle v0.1

A dependency-free Rust reference for basic linear gravity-wave kinematics and a frozen CSV fixture set. It is deliberately separate from Bevy, the renderer, the SPH solver, and global hydrology/ocean models.

## Run

Use cargo test with the manifest at tools/ocean-wave-reference/Cargo.toml and the locked flag.

## Scope

Implemented:
- deep-water and finite-depth linear gravity-wave dispersion;
- period, phase velocity, group velocity and wavelength;
- one-dimensional analytic elevation;
- frozen reference rows and invalid-input controls.

Not implemented:
- directional spectra, JONSWAP/TMA generation, FFT cascades or nonlinear wave interactions;
- wave breaking, SPH/CFD, ocean circulation, tides, currents or empirical validation;
- Bevy rendering or GPU execution.

## Fixture convention

Each row specifies gravity, wavenumber and depth. Expected values are independently precomputed from the stated linear formulas and stored as decimal constants; tests do not generate expected values using the function under test.

The deep_limit case uses a finite depth with kh = 100, where tanh(kh) is numerically indistinguishable from one in f64. The shallow_limit and intermediate_depth cases exercise different finite-depth regimes.

## Claim boundary

Passing these tests validates only this small analytic implementation against its frozen controls. It does not establish spectral sea-state fidelity, empirical accuracy, CFD quality, GPU parity, ship-response fidelity, or visual competitiveness.

This is the first small executable piece of WATER-PHYS-001 (#2203), and is intended to become a reference oracle for future render and buoyancy adapters rather than a new water-state authority.
