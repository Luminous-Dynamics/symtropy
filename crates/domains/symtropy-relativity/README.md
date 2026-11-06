# symtropy-relativity

A narrow Rust research crate for analytic Lorentzian-spacetime reference calculations.

## First slice

- Positive, finite mass validation in kilograms.
- Signed dimensionless Kerr spin chi = cJ/(GM^2), bounded to [-1, 1].
- SI-valued gravitational radius, Schwarzschild horizon, photon sphere, and ISCO.
- Schwarzschild Kretschmann scalar with units m^-4.
- Explicitly scoped static-observer lapse and gravitational redshift outside a Schwarzschild horizon.
- Kerr inner/outer Boyer-Lindquist horizons and angle-dependent stationary-limit surface.
- Analytic limiting-case and invalid-domain tests.

## Deliberate limits

This is not a metric/tensor calculus, geodesic integrator, GRHD/GRMHD solver, gravitational-wave waveform generator, or numerical-relativity evolution code. It makes no observational-validation or state-of-the-art performance claim.

Next gates are metric/inverse/determinant/signature fixtures; Christoffel and curvature tensors; independent coordinate-invariance checks; geodesic integration with normalization and Killing-constant drift; then waveform and numerical-relativity comparisons.

## Run

cargo test -p symtropy-relativity
