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
- Levi-Civita connection coefficients from an explicit first-derivative tensor, with an analytic Schwarzschild derivative fixture and metric-compatibility residual diagnostic. No finite-difference step size is hidden in the API.
- Riemann, Ricci and Einstein tensors from explicit connection derivatives; fully lowered Riemann tensor; algebraic symmetry / first-Bianchi residual; scalar curvature and Kretschmann contraction. An analytic Schwarzschild second-derivative fixture is included.
- Schwarzschild timelike and null geodesics using RK4 and step-doubling adaptive control, with component-wise dimensioned tolerances, normalization, conserved Killing energy/angular momentum diagnostics, and chart-boundary rejection.
- 48 authored analytic, limiting-case, invalid-domain, matrix, connection, curvature, geodesic, and posterior-transform tests.

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

This crate calculates Riemann/Ricci/Einstein curvature for the Schwarzschild analytic fixture and integrates Schwarzschild timelike/null geodesics using fixed-step RK4 and step-doubling adaptive control. Adaptive tolerances are dimension-specific for coordinates and tangent components. It does not yet provide Kerr curvature derivatives, Kerr geodesics, horizon-regular charts, a GRHD/GRMHD solver, a gravitational-wave waveform generator, or numerical-relativity evolution. The generic metric-component constructor does not certify Lorentzian signature. It makes no observational-validation or state-of-the-art performance claim.

Next gates are Kerr curvature and coordinate-invariant cross-chart checks, generic Kerr geodesics and horizon-regular charts, followed by perturbation/waveform reproduction and numerical-relativity comparisons.

## Run

`cargo test -p symtropy-relativity`

**Validation status:** 48 tests are authored but were not executed in the editing environment because `rustc` and `cargo` are unavailable there. Independent array-based numerical checks match Schwarzschild vacuum Ricci, Kretschmann and Riemann algebraic identities to floating-point roundoff; these are not Rust test or build results.


## Versioned benchmark manifests

- `benchmarks/analytic_reference_cases.json`: SI-valued Schwarzschild/Kerr fixtures spanning zero, small, intermediate, retrograde, near-extremal and extremal spin.
- `benchmarks/direct_wave_sxs_spin_sweep.json`: exact ten SXS IDs and final spins from the Kankani–McWilliams counteranalysis, filter specification, competing horizon/News/BOB models and null controls.
- `benchmarks/waveform_surrogate_baselines.json`: versioned literature baselines for NRSur7dq4 NN, NRSur7dq4v2, SEOBNRv5PHM_NNSur7dq10, AESur3dq8, eccentric NR surrogate and SpEC accuracy limits.

These manifests describe expected references and benchmark design. They are not yet automatically loaded by the Rust tests and their literature figures have not been reproduced by this crate.

## Numerical stability additions

- Kerr inner horizon uses a rationalized formula to avoid subtractive cancellation at small spin.
- Kerr surface gravity uses a dimensionless-spin expression that remains stable near extremality.
- Kerr Boyer-Lindquist Delta is evaluated as `(r-r_plus)*(r-r_minus)` near the exterior horizon.
- Levi-Civita connections expose a normalized metric-compatibility residual diagnostic.
