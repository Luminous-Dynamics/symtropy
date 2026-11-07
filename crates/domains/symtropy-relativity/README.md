# symtropy-relativity

A bounded Rust research crate for analytic Lorentzian-spacetime reference calculations.

## Current capabilities

- Positive finite mass and signed dimensionless Kerr spin validation, including rejection when derived length scales are not representable.
- SI-valued Schwarzschild gravitational radius, event horizon, photon sphere, ISCO, Kretschmann scalar, static-observer lapse, and redshift.
- SI-valued Kerr inner/outer Boyer-Lindquist horizons and angle-dependent ergosurface.
- Kerr horizon angular velocity, surface-gravity decay rate, and nonnegative direct-wave reference frequency.
- Posterior-preserving transformation from paired remnant mass/spin samples to Kerr horizon observables.
- Schwarzschild/Kerr exterior metric components plus horizon-regular ingoing Eddington-Finkelstein and ingoing Kerr charts in declared coordinate bases.
- Symmetric finite 4x4 metric component representation, pivoted determinant and inverse, and inverse-identity residual diagnostic.
- Explicit first metric derivatives and Levi-Civita connection coefficients, with an analytic Schwarzschild derivative fixture.
- Explicit second metric derivatives, mixed-index Riemann tensor, Ricci tensor/scalar, Einstein tensor, Kretschmann contraction, and algebraic Riemann-symmetry residuals.
- Fixed-size second-order forward-mode automatic differentiation for Schwarzschild, Kerr, ingoing Eddington-Finkelstein, and ingoing Kerr metric values, gradients, and Hessians; AD-generated Kerr curvature is checked against vacuum and analytic Kretschmann fixtures.
- Horizon-regular ingoing charts for Schwarzschild and Kerr, with curvature tests at/inside their horizons and scalar-invariant comparisons against exterior coordinates.
- 50 authored analytic, limiting-case, invalid-domain, matrix, connection, curvature, automatic-differentiation, chart, and posterior-transform tests.

## Conventions

Public physical parameters use SI units. Metric coordinates are `x^mu = (c*t, r, theta, phi)`: `c*t` and `r` are in metres, angles are in radians, and `ds^2` is in square metres. The signature is `(-,+,+,+)`.

Kerr spin is the signed dimensionless quantity `chi = c*J/(G*M^2)`; the geometrized rotation parameter is `a = chi*G*M/c^2`. Schwarzschild coordinates and Boyer-Lindquist coordinates are restricted to the exterior and to `0 < theta < pi`.

The metric determinant is coordinate-dependent, not a curvature scalar. Generic `MetricComponents4::new` checks finiteness and symmetry but does not certify Lorentzian signature. The analytic Schwarzschild/Kerr constructors represent the known exterior solutions.

Metric derivatives use `d[alpha][mu][nu] = partial_alpha g_(mu nu)`. Second derivatives use `d2[alpha][beta][mu][nu] = partial_alpha partial_beta g_(mu nu)`. Connection coefficients use `Gamma[rho][mu][nu] = Gamma^rho_(mu nu)`; Riemann storage is `R[rho][sigma][mu][nu] = R^rho_(sigma mu nu)`.

Riemann sign convention:
`R^rho_(sigma mu nu) = partial_mu Gamma^rho_(nu sigma) - partial_nu Gamma^rho_(mu sigma) + Gamma^rho_(mu lambda) Gamma^lambda_(nu sigma) - Gamma^rho_(nu lambda) Gamma^lambda_(mu sigma)`.
Ricci convention: `R_(sigma nu) = R^rho_(sigma rho nu)`.

All derivative arrays must use the same chart, coordinate order, units, and signature. Derivatives are supplied explicitly; no hidden finite-difference approximation is performed.

For Kerr:
- `Omega_H = a*c/(r_plus^2 + a^2)` is in radians per second and retains the sign of the spin.
- `kappa_t = c*sqrt(r_g^2-a^2)/(2*r_g*r_plus)` is in inverse seconds.
- `f_2OmegaH = abs(Omega_H)/pi` is the nonnegative Hz reference corresponding to angular-frequency magnitude `2*abs(Omega_H)`.

The direct-wave reference frequency is a theory-derived scale motivated by a published GW250114 analysis. It is not itself a waveform, a detection, or a verified universal feature.

The posterior transform preserves each input sample ID, order, and paired mass/spin draw. It does not infer parameters, resample, marginalize, or fit waveforms.

## Deliberate limits

The explicit hand-derived second-derivative fixture covers Schwarzschild in the exterior Schwarzschild chart. A separate second-order forward-mode AD path generates first/second derivatives and curvature for Schwarzschild and Kerr exterior charts. It also supports ingoing Eddington-Finkelstein Schwarzschild and ingoing Kerr coordinates, with tests at/inside the horizons and scalar-invariant comparisons across charts. Kerr vacuum-Ricci and analytic Kretschmann tests are authored. The current geodesic integrator is fixed-step RK4 and has only initial Schwarzschild/Kerr fixtures; adaptive stepping, symplectic/Hamiltonian cross-validation, Carter-constant handling, and extensive inclined Kerr orbit tests remain future work. There is no GRHD/GRMHD solver, waveform generator, or numerical-relativity evolution code.

The generic metric-component constructor does not certify Lorentzian signature. Curvature output is not yet accompanied by automatic conditioning estimates; tolerances must be interpreted for the declared fixture and coordinate chart. No observational-validation or state-of-the-art performance claim is made.

## Next gates

- Kerr further independent curvature-invariant fixtures and full tensor conditioning/error estimates.
- Differential Bianchi identity and higher-derivative consistency checks.
- More exhaustive horizon-regular coordinate and cross-chart scalar-invariant comparisons.
- Timelike/null geodesics with normalization and Killing-constant drift.
- QNM / waveform and public detector-data replication.

## Versioned benchmark registries

Three versioned machine-readable research registries are maintained beside the crate (the two manifests below plus their analysis sub-configurations):

- `benchmarks/direct_wave_sxs_spin_sweep.json` — the earlier ten-case high-spin counteranalysis plus a pinned 168-entry catalogue (125 aligned/precessing/eccentric SXS and 43 anti-aligned), 13 CCE News waveforms, all-QNM ell=2..6 filter analysis, light-ring and BOB predictors, filter-free subset configuration, and Gaussian-vacuum-perturbation filter controls. The earlier ten-case paper's exact fitting-window boundaries remain null until transcribed from its own supplement.
- `benchmarks/waveform_surrogate_baselines.json` — versioned literature baselines for NRSur7dq4 NN/v2, SEOBNRv5PHM_NNSur7dq10, AESur3dq8, NRSurE_q4NoSpin_22, BOB, SpEC accuracy, SEOBNRv6EPHM, and SEOBNRv6EHM.

These files are literature / experiment specifications. They are not benchmark execution results, model certifications, or evidence that the reported paper metrics have been reproduced by Symtropy.

## Run

`cargo test -p symtropy-relativity`

**Validation status:** tests are authored but were not executed in the editing environment because `rustc` and `cargo` are unavailable there.
