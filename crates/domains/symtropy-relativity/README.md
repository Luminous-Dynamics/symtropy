# symtropy-relativity

A bounded Rust research crate for analytic and differentiated Lorentzian-spacetime reference calculations.

## Current capabilities

- Positive finite SI mass and signed dimensionless Kerr spin validation, including rejection when derived length scales are not representable.
- Schwarzschild gravitational radius, horizon, photon sphere, ISCO, Kretschmann scalar, static-observer lapse, and redshift.
- Kerr inner/outer Boyer-Lindquist horizons, ergosurface, signed horizon angular velocity, surface-gravity rate, and nonnegative m=2 horizon-rotation comparison frequency (without assuming a non-QNM signal follows it).
- Posterior-preserving transformation of paired remnant mass/spin samples into correlated Kerr horizon observables.
- Schwarzschild and Kerr exterior metrics; horizon-regular ingoing Eddington-Finkelstein Schwarzschild and ingoing Kerr metrics.
- Finite symmetric 4x4 metric components, pivoted determinant/inverse, and inverse-identity residual.
- Explicit first metric derivatives and Levi-Civita connection coefficients, including analytic Schwarzschild derivative fixtures.
- Explicit second metric derivatives, mixed-index Riemann tensor, Ricci tensor/scalar, Einstein tensor, Kretschmann contraction, and algebraic Riemann symmetry diagnostics.
- Fixed-size second-order forward-mode automatic differentiation (Jet2), producing first/second metric derivatives for Schwarzschild/Kerr and their horizon-regular ingoing charts.
- Curvature verification across Schwarzschild and Kerr charts, including horizon and interior fixtures; Kerr vacuum Ricci and analytic Kretschmann comparisons.
- Kerr equatorial prograde/retrograde circular-null-orbit radii and signed coordinate angular frequencies, with Schwarzschild and extremal limits.
- **57 authored Rust tests** across scalar physics, metric matrices, connection, curvature, automatic differentiation, horizon-regular charts, posterior transforms, and light-ring limits.

## Conventions

Public physical parameters use SI units. Metric coordinates are expressed as (ct, r, theta, phi) for Schwarzschild/Boyer-Lindquist charts and (v, r, theta, phi) for ingoing charts. The time-like coordinate and radial coordinate are measured in metres; angles are in radians; ds^2 is in square metres. Signature is (-,+,+,+).

Kerr spin is the signed dimensionless quantity chi = c*J/(G*M^2); the geometrized rotation parameter is a = chi*G*M/c^2. Boyer-Lindquist and Schwarzschild coordinates are exterior-only. Ingoing Eddington-Finkelstein / ingoing Kerr charts are regular at the future horizon and are defined for r > 0 and 0 < theta < pi.

The metric determinant is coordinate-dependent, not a curvature scalar. Generic MetricComponents4::new checks finiteness and symmetry but does not certify Lorentzian signature. The analytic metric constructors supply the intended background solutions.

Metric derivatives use d[alpha][mu][nu] = partial_alpha g_(mu nu). Second derivatives use d2[alpha][beta][mu][nu] = partial_alpha partial_beta g_(mu nu). Connection coefficients use Gamma[rho][mu][nu] = Gamma^rho_(mu nu); Riemann storage is R[rho][sigma][mu][nu] = R^rho_(sigma mu nu).

Riemann sign convention:
R^rho_(sigma mu nu) = partial_mu Gamma^rho_(nu sigma) - partial_nu Gamma^rho_(mu sigma) + Gamma^rho_(mu lambda) Gamma^lambda_(nu sigma) - Gamma^rho_(nu lambda) Gamma^lambda_(mu sigma).

Ricci convention: R_(sigma nu) = R^rho_(sigma rho nu).

All derivative arrays must use the same chart, coordinate order, units, and signature. Explicit derivative-fed curvature is available; the Jet2 module generates first and second derivatives by forward-mode automatic differentiation. No finite-difference step size is hidden in either path.

For Kerr:
- Omega_H = a*c/(r_plus^2 + a^2), in radians per second, retaining spin sign.
- kappa_t = c*sqrt(r_g^2-a^2)/(2*r_g*r_plus), in inverse seconds.
- f_m2_horizon = abs(Omega_H)/pi is a nonnegative m=2 horizon-rotation comparison scale. It does not imply an observed non-QNM residual follows that scale.

The m=2 horizon-rotation frequency is a Kerr background comparison scale only. Recent work disputes a universal identification of the extracted non-QNM merger-ringdown component with horizon frequency/surface gravity and instead investigates light-ring and peak-news dynamics; see the research charter and Symthaea replication campaign.

The posterior transform preserves each input sample ID, order, and paired mass/spin draw. It does not infer parameters, resample, marginalize, or fit waveforms.

## Deliberate limits

The crate now calculates background metrics, Levi-Civita connections, Riemann/Ricci/Einstein tensors, and scalar curvature invariants for its supported differentiated charts. It does not yet include a validated geodesic integrator, Kerr/Schwarzschild perturbation solver, QNM solver, waveform generator, GRHD/GRMHD solver, or numerical-relativity evolution system.

Curvature results do not yet carry automatic conditioning estimates. Numerical tolerances must be interpreted for the declared fixture and coordinate chart. No observational-validation or state-of-the-art performance claim is made.

## Next gates

1. Add conditioning-aware tensor norms and independent symbolic/reference comparisons.
2. Add a Kerr analytic/AD cross-check with a second independent differentiation route and further parameter sweeps.
3. Add regular-chart transition maps and invariant comparisons across additional coordinate systems.
4. Implement timelike/null geodesics using both second-order and Hamiltonian formulations; validate normalization, Killing energy/angular momentum, and convergence.
5. Implement the light-ring radial Lyapunov exponent and compare eikonal QNM predictions.
6. Add curvature Bianchi differential-identity checks where third derivatives are available.
7. Connect the verified background/curvature API to QNM perturbation and GW250114 model-discrimination work.

## Run

`cargo test -p symtropy-relativity`

**Validation status:** the 57 tests are authored but have not been executed in the current editing environment because `rustc` and `cargo` are unavailable.
