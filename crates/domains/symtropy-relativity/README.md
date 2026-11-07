# symtropy-relativity

A bounded Rust research crate for analytic Lorentzian-spacetime reference calculations.

## Current capabilities

- Positive finite mass and signed dimensionless Kerr spin validation, including rejection when derived length scales are not representable.
- SI-valued Schwarzschild gravitational radius, event horizon, photon sphere, ISCO, Kretschmann scalar, static-observer lapse, and redshift.
- SI-valued Kerr inner/outer Boyer-Lindquist horizons and angle-dependent ergosurface.
- Kerr horizon angular velocity, surface-gravity decay rate, and nonnegative direct-wave reference frequency.
- Posterior-preserving transformation from paired remnant mass/spin samples to Kerr horizon observables.
- Schwarzschild and Kerr exterior metric components in the declared `(ct, r, theta, phi)` basis.
- Symmetric finite 4x4 metric component representation, pivoted determinant and inverse, and inverse-identity residual diagnostic.
- Levi-Civita connection coefficients from an explicit first-derivative tensor, with an analytic Schwarzschild derivative fixture. No finite-difference step size is hidden in the API.
- 32 authored analytic, limiting-case, invalid-domain, matrix, connection, and posterior-transform tests.

## Conventions

Public physical parameters use SI units. Metric coordinates are `x^mu = (c*t, r, theta, phi)`: `c*t` and `r` are in metres, angles are in radians, and `ds^2` is in square metres. The signature is `(-,+,+,+)`.

Kerr spin is the signed dimensionless quantity `chi = c*J/(G*M^2)`; the geometrized rotation parameter is `a = chi*G*M/c^2`. Schwarzschild coordinates and Boyer-Lindquist coordinates are restricted to the exterior and to `0 < theta < pi`.

The metric determinant is coordinate-dependent, not a curvature scalar. Generic `MetricComponents4::new` checks finiteness and symmetry but does not certify Lorentzian signature. The analytic Schwarzschild/Kerr constructors represent the known exterior solutions.

Metric derivatives use `d[alpha][mu][nu] = partial_alpha g_(mu nu)`; connection coefficients use `Gamma[rho][mu][nu] = Gamma^rho_(mu nu)`. The metric and derivative arrays must use the same chart, coordinate order, units, and signature.

For Kerr:
- `Omega_H = a*c/(r_plus^2 + a^2)` is in radians per second and retains the sign of the spin.
- `kappa_t = c*sqrt(r_g^2-a^2)/(2*r_g*r_plus)` is in inverse seconds.
- `f_2OmegaH = abs(Omega_H)/pi` is the nonnegative Hz reference corresponding to angular-frequency magnitude `2*abs(Omega_H)`.

The direct-wave reference frequency is a theory-derived scale motivated by a published GW250114 analysis. It is not itself a waveform, a detection, or a verified universal feature.

The posterior transform preserves each input sample ID, order, and paired mass/spin draw. It does not infer parameters, resample, marginalize, or fit waveforms.

## Deliberate limits

This crate does not yet implement Riemann/Ricci/Einstein curvature tensors, geodesic integration, a GRHD/GRMHD solver, a gravitational-wave waveform generator, or numerical-relativity evolution. The generic metric-component constructor does not certify Lorentzian signature. It makes no observational-validation or state-of-the-art performance claim.

Next gates are Riemann curvature from connection derivatives, coordinate-invariant checks (including Schwarzschild/Kerr curvature scalars), geodesic integration with normalization/Killing-constant drift, and then waveform/ringdown reproduction and numerical-relativity comparisons.

## Run

`cargo test -p symtropy-relativity`

**Validation status:** 36 tests are authored but were not executed in the editing environment because `rustc` and `cargo` are unavailable there.
