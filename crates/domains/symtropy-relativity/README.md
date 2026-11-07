# symtropy-relativity

A bounded Rust research crate for analytic Lorentzian-spacetime reference calculations.

## Current first slices

- Positive finite mass and signed dimensionless Kerr spin validation.
- SI-valued Schwarzschild gravitational radius, event horizon, photon sphere, ISCO, Kretschmann scalar, static-observer lapse and redshift.
- SI-valued Kerr inner/outer Boyer-Lindquist horizons and angle-dependent ergosurface.
- Kerr horizon angular velocity, surface-gravity decay rate, and the reference frequency for the proposed (2\Omega_H) direct-wave component.
- Schwarzschild and Kerr exterior metric components in the declared ((ct,r,\theta,\phi)) basis.
- Symmetric finite 4x4 metric component representation, pivoted determinant and inverse, and inverse-identity residual diagnostic.
- Analytic, limiting-case, chart-domain, singular-matrix, and invalid-input tests.

## Conventions

Public physical parameters use SI units. The metric component basis is (x^\mu=(ct,r,\theta,\phi)): (ct) and (r) are in metres, angles are in radians, and (ds^2) is in square metres, with signature ((-+++)). Kerr spin is the signed dimensionless (\chi=cJ/(GM^2)\); (a=\chi GM/c^2\). Schwarzschild coordinates and Boyer-Lindquist coordinates are restricted to the exterior and to (0<\theta<\pi).

The metric determinant is coordinate-dependent, not a curvature scalar. Generic `MetricComponents4::new` checks finiteness and symmetry but does not certify Lorentzian signature. The analytic Schwarzschild/Kerr constructors are the known exterior solutions.

For Kerr:
- `Omega_H = a*c/(r_plus^2 + a^2)` is in rad/s.
- `kappa_t = c*sqrt(r_g^2-a^2)/(2*r_g*r_plus)` is in s^-1.
- `f_2OmegaH = Omega_H/pi` is the Hz reference corresponding to angular frequency `2*Omega_H`.

The direct-wave reference frequency is a theory-derived scale motivated by a published GW250114 analysis. It is not itself a waveform, a detection, or a verified universal feature.

## Deliberate limits

This crate is not yet a complete tensor-calculus library, a Christoffel/Riemann/Ricci/Einstein tensor engine, geodesic integrator, GRHD/GRMHD solver, gravitational-wave waveform generator, or numerical-relativity evolution code. It makes no observational-validation or state-of-the-art performance claim.

Next gates are metric derivative/connection calculations, curvature tensors and coordinate-invariant checks, geodesics with normalization/Killing-constant drift, then waveform/ringdown reproduction and numerical-relativity comparisons.

## Run

`cargo test -p symtropy-relativity`
