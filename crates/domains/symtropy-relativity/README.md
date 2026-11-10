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
- Noise-weighted frequency-domain waveform comparison: one-sided PSD inner product, constant-phase-maximized mismatch, explicit best phase offset, separate amplitude L2 and frequency-dependent phase residual diagnostics, and strict grid/PSD/domain validation. The caller must pre-align waveforms; no hidden time maximization, interpolation, resampling, or PSD estimation.
- Kerr equatorial prograde/retrograde circular-null-orbit reference observables: analytic photon-orbit radius and signed frequency, metric null/radial circularity residuals, normalized null radial-potential R/R-prime residuals, impact parameter, and coordinate-time radial Lyapunov exponent. Extremal prograde Boyer-Lindquist horizon limit returns explicit unavailable impact/Lyapunov values rather than a fabricated finite result.
- A 20-case independently generated Kerr light-ring reference grid spanning zero, positive, negative, near-extremal, and extremal spin.
- Adaptive canonical-Hamiltonian Kerr null-trajectory integration with step-doubled RK4 in Boyer–Lindquist and horizon-regular ingoing Kerr charts; records accepted/rejected steps and Hamiltonian drift without hidden null renormalization.
- **114 authored Rust tests** across scalar physics, Kerr limits, metric matrices, connection compatibility, curvature, second-order automatic differentiation, horizon-regular charts, posterior transforms, geodesic integration, separated Kerr null-geodesic formulae, waveform metrics, and light-ring references.

## Conventions

Public physical parameters use SI units. Metric coordinates are `x^mu = (c*t, r, theta, phi)`: `c*t` and `r` are in metres, angles are in radians, and `ds^2` is in square metres. The signature is `(-,+,+,+)`.

Kerr spin is the signed dimensionless quantity `chi = c*J/(G*M^2)`; the geometrized rotation parameter is `a = chi*G*M/c^2`. Schwarzschild coordinates and Boyer-Lindquist coordinates are restricted to the exterior and to `0 < theta < pi`.

The metric determinant is coordinate-dependent, not a curvature scalar. Generic `MetricComponents4::new` checks finiteness and symmetry but does not certify Lorentzian signature. The analytic Schwarzschild/Kerr constructors represent the known exterior solutions.

Metric derivatives use `d[alpha][mu][nu] = partial_alpha g_(mu nu)`. Second derivatives use `d2[alpha][beta][mu][nu] = partial_alpha partial_beta g_(mu nu)`. Connection coefficients use `Gamma[rho][mu][nu] = Gamma^rho_(mu nu)`; Riemann storage is `R[rho][sigma][mu][nu] = R^rho_(sigma mu nu)`.

Riemann sign convention:
`R^rho_(sigma mu nu) = partial_mu Gamma^rho_(nu sigma) - partial_nu Gamma^rho_(mu sigma) + Gamma^rho_(mu lambda) Gamma^lambda_(nu sigma) - Gamma^rho_(nu lambda) Gamma^lambda_(mu sigma)`.
Ricci convention: `R_(sigma nu) = R^rho_(sigma rho nu)`.

All derivative arrays must use the same chart, coordinate order, units, and signature. Derivatives are supplied explicitly or produced by the fixed-size Jet2 AD path; no hidden finite-difference approximation is performed.

## Waveform comparison conventions

The waveform metric uses the discrete one-sided PSD convention `4*df*sum(conjugate(h_ref)*h_candidate/S_n)`. Frequency grids must be identical and uniform. The caller supplies an explicit inclusive frequency band and a reference-amplitude floor for phase diagnostics. The reported mismatch maximizes over a single constant phase only; the phase offset is returned, and the frequency-dependent phase residual remains visible. Time alignment is not optimized or modified, so the caller must record the alignment procedure. PSD estimation, interpolation, whitening and windowing are outside this API.

For Kerr:
- `Omega_H = a*c/(r_plus^2 + a^2)` is in radians per second and retains the sign of the spin.
- `kappa_t = c*sqrt(r_g^2-a^2)/(2*r_g*r_plus)` is in inverse seconds.
- `f_2OmegaH = abs(Omega_H)/pi` is the nonnegative Hz reference corresponding to angular-frequency magnitude `2*abs(Omega_H)`.

The direct-wave reference frequency is a theory-derived scale motivated by a published GW250114 analysis. It is not itself a waveform, a detection, or a verified universal feature.

## Eikonal light-ring reference calculations

`light_ring.rs` provides equatorial Kerr prograde/retrograde circular-null-orbit references. It reports the closed-form photon-orbit radius and signed coordinate frequency, calculates the impact parameter from equatorial metric components, checks the local null and radial circularity conditions, and computes the coordinate-time radial Lyapunov exponent from the null radial potential. For the extremal prograde orbit, the radius and horizon-generator frequency are returned but Boyer-Lindquist impact-parameter / Lyapunov calculations are marked unavailable because the orbit coincides with the horizon. These are geometric/eikonal reference quantities, not exact finite-mode QNM eigenfrequencies.

The posterior transform preserves each input sample ID, order, and paired mass/spin draw. It does not infer parameters, resample, marginalize, or fit waveforms.

- Machine-readable waveform discrepancy / posterior calibration protocol: `benchmarks/waveform_discrepancy_calibration_v1.json`. This is a benchmark specification, not an executed discrepancy model.

- Machine-readable horizon semantics / provenance contract: `benchmarks/horizon_object_contract_v1.json`. It defines distinct stationary, event, apparent/MOTS, marginally trapped tube, dynamical and isolated horizon types; it does not implement horizon finders.

## Deliberate limits

The explicit hand-derived second-derivative fixture covers Schwarzschild in the exterior Schwarzschild chart. A separate second-order forward-mode AD path generates first/second derivatives and curvature for Schwarzschild and Kerr exterior charts. It also supports ingoing Eddington-Finkelstein Schwarzschild and ingoing Kerr coordinates, with tests at/inside the horizons and scalar-invariant comparisons across charts. Kerr vacuum-Ricci and analytic Kretschmann tests are authored. The current geodesic engine is Schwarzschild-only. It includes second-order Christoffel RK4, first-order canonical Hamiltonian RK4, and a step-doubling adaptive controller with separate coordinate/tangent tolerances. It is not yet symplectic, and Kerr geodesics/Carter-constant handling remain future work. There is no GRHD/GRMHD solver, waveform generator, or numerical-relativity evolution code.

The generic metric-component constructor does not certify Lorentzian signature. Curvature output is not yet accompanied by automatic conditioning estimates; tolerances must be interpreted for the declared fixture and coordinate chart. No observational-validation or state-of-the-art performance claim is made.

## Next gates

- Add waveform-metric fixture cross-checks against an independent Python/NumPy reference and validate PSD-weighting / alignment behavior on real SXS test signals.
- Propagate waveform discrepancy through a simple posterior-recovery fixture before any surrogate posterior-accuracy claim.
- Kerr further independent curvature-invariant fixtures and full tensor conditioning/error estimates.
- Cross-check the equatorial Kerr light-ring frequency and Lyapunov references against independent closed-form/eikonal calculations and compare to finite-l QNM spectra without conflating the asymptotic eikonal correspondence with exact QNM values.
- Differential Bianchi identity and higher-derivative consistency checks.
- More exhaustive horizon-regular coordinate and cross-chart scalar-invariant comparisons.
- Extend the Schwarzschild geodesics to horizon-regular coordinate charts, then add Kerr equatorial/inclined geodesics, Carter-constant diagnostics, and an independent formulation with its own convergence suite.
- QNM / waveform and public detector-data replication.

## Versioned benchmark registries

Six versioned machine-readable research manifests are maintained beside the crate:

- `benchmarks/direct_wave_sxs_spin_sweep.json` — the earlier ten-case high-spin counteranalysis plus a pinned 168-entry catalogue (125 aligned/precessing/eccentric SXS and 43 anti-aligned), 13 CCE News waveforms, all-QNM ell=2..6 filter analysis, light-ring and BOB predictors, filter-free subset configuration, and Gaussian-vacuum-perturbation filter controls. The earlier ten-case paper's exact fitting-window boundaries remain null until transcribed from its own supplement.
- `benchmarks/waveform_surrogate_baselines.json` — versioned literature baselines for NRSur7dq4 NN/v2, SEOBNRv5PHM_NNSur7dq10, AESur3dq8, NRSurE_q4NoSpin_22, BOB, SpEC accuracy, SEOBNRv6EPHM, and SEOBNRv6EHM.
- `benchmarks/waveform_surrogate_certification_v1.json` — grouped simulation-ID splits, waveform error distributions, posterior coverage, OOD abstention and active-NR selection protocol.
- `benchmarks/analytic_reference_cases.json` and `benchmarks/geodesic_reference_cases.json` — exact SI Kerr/Schwarzschild and Schwarzschild geodesic reference fixtures.
- `benchmarks/light_ring_reference_cases.json` — independently generated 20-case equatorial Kerr photon-orbit / Lyapunov reference grid.

- `benchmarks/kerr_mino_trajectory_v1.json` — analytic Schwarzschild critical/double-root fixture, (b=6M) scattering trajectory through a turning point, independent Python/SciPy equation-level references, separated-vs-canonical Hamiltonian cross-check, horizon-regular ingoing evaluator and a G0–G8 qualification ladder. It explicitly marks Rust execution as pending.

These files are literature / experiment specifications. They are not benchmark execution results, model certifications, or evidence that the reported paper metrics have been reproduced by Symtropy.

## Run

`cargo test -p symtropy-relativity`

**Validation status:** 90 tests are authored but were not executed in the editing environment because `rustc` and `cargo` are unavailable there. The new Kerr equations were checked against independent algebraic calculations for the Schwarzschild critical double root, observer-screen map and radial null coordinate-time ratio; this does not validate Rust compilation or a numerical Kerr trajectory. No build/test pass, waveform benchmark result, light-ring Rust result, or geodesic accuracy claim is made.


## Separated Kerr null-geodesic reference

`kerr_geodesic.rs` uses dimensionless geometrized variables (M=Gm/c^2=1), (a/M=\chi), (r/M), (ct/M), (L_z/M), and (Q/M^2). It provides Carter-separated radial/polar potentials and their derivatives, Bardeen asymptotic observer-screen ↔ ((E,L_z,Q)) mapping, and both first-order signed Mino derivatives and a second-order smooth Mino system.

The second-order equations follow by differentiating the separated first integrals:
- (d^2(r/M)/d\gamma^2 = \tfrac12 R'(r/M));
- (d^2\theta/d\gamma^2 = \tfrac12 \Theta'(\theta)).

The second-order state passes continuously through simple turning points because the corresponding coordinate velocity reaches zero while its acceleration determines the continuation. `mino_constraint_residuals` exposes (v_r^2-R) and (v_\theta^2-\Theta) for integration-drift monitoring. The first-order signed system remains useful for evaluating branch-local analytic derivatives and explicit sign controls.

Finite-radius observer geometry is now explicit through `KerrFiniteRadiusObserver`, which constructs a zero-angular-momentum-observer (ZAMO) orthonormal tetrad in the Kerr exterior. `KerrLocalSkyDirection` requires a unit local 3-direction; `ray_seed` maps it to (E=1), (L_z), Carter (Q), and a Mino initial state. The API rejects non-unit directions, axis/horizon chart boundaries and non-positive Killing energy at infinity (negative-energy ergoregion rays are not represented by the current (E>0) constants type). The local observer map provides the finite-radius starting point; the Bardeen map remains the asymptotic limit.

A second, independent formulation is also present: `KerrCanonicalHamiltonian` builds the dimensionless Boyer–Lindquist metric and its first derivatives from the existing Jet2 automatic-differentiation path, then evaluates (H=\\tfrac12 g^{\\mu\\nu}p_\\mu p_\\nu) and Hamilton's equations. `state_from_separated` maps Carter constants and branch choices into canonical momenta. The cross-check compares the Hamiltonian coordinate vector field to the separated Mino derivatives after multiplication by (\\Sigma), and checks recovered (E,L_z,Q) and the null Hamiltonian residual. This tests two different derivation routes against each other; they still share the underlying Kerr metric.

The second-order system has two adaptive RK4 step-doubling entry points:
- `integrate_adaptive` uses exterior Boyer–Lindquist coordinates and stops at the horizon/chart boundary.
- `integrate_adaptive_ingoing` uses future-ingoing Kerr coordinates and can cross the outer horizon for inward rays while (r>0) and (0<\theta<\pi).

The full-step/two-half-step difference controls local truncation error; radial and polar first-integral residuals are monitored independently and enforced as a separate hard qualification ceiling. Accumulated invariant drift is not misused as a local truncation-error estimate. Every trajectory and finite-observer ray seed carries an explicit coordinate-chart tag, because the first/fourth state slots represent ((t,\phi)) in Boyer–Lindquist mode and ((v,\tilde\phi)) in ingoing mode. Returned trajectories record each accepted endpoint, accepted/rejected step counts, suggested next step, maximum normalized constraint residuals and the requested Mino interval. Simple radial/polar turning points are traversed by the smooth second-order equations.

The Schwarzschild (b=6M) scattering fixture integrates from (r=10M), crosses the analytic outer turning radius (r\approx4.4533631938M), and returns outward by (\gamma=0.6). A separate Kerr (chi=0.9), (L_z/M=2), (Q=0) capture fixture integrates from (r=10M) through (r_+=1.4358898944M) to (r\approx1M) using the ingoing chart. Both fixtures have independent Python/SciPy equation-level references; Rust execution remains pending.

`KerrMinoTrajectory::capture_status` provides a conservative horizon-crossing classifier. It returns `CapturedByHorizon` only when an explicitly ingoing-chart trajectory contains an outside-to-inside (r_+) crossing and both normalized first-integral residual maxima remain below the caller's ceiling. It reports `NoCrossingWithinIntegratedInterval` rather than inferring escape, `NotQualifiableInBoyerLindquist` for exterior-chart paths, and `ConstraintDriftExceeded` when residuals fail the requested limit.

The independent canonical formulation now also has an adaptive trajectory integrator in `kerr_integrator.rs`. `KerrNullTrajectoryIntegrator` uses step-doubled RK4 with declared absolute/relative tolerances in either the exterior Boyer–Lindquist chart or the horizon-regular ingoing Kerr chart. It retains accepted endpoints, rejected/accepted counts, maximum absolute null-Hamiltonian residual, maximum accepted scaled local error and the suggested next step. It does not project or renormalize the null constraint. The local error estimate is a controller diagnostic, not a rigorous global trajectory bound; classical RK4 is not symplectic.

Two independent trajectory routes now exist: the separated second-order Mino system (including radial/polar first-integral residuals) and the canonical Hamiltonian system (including Hamiltonian constraint drift). They should be compared on matched initial data and observables; they are not two independent spacetime metrics.

The canonical evaluator also supports the horizon-regular ingoing Kerr chart. `ingoing_state_from_separated` transforms the future-inward Boyer–Lindquist radial momentum using the rationalized identity (p_r^{\mathrm{in}}=[Q+(L-aE)^2]/[P+\sqrt R]), avoiding cancellation in the (1/\Delta) expression at the future horizon. The ingoing metric/AD path and chart-equivalence transformations are separately tested.

The finite-radius observer is specifically a ZAMO. `KerrFiniteRadiusObserver` provides an orthonormal tetrad; `KerrLocalSkyDirection` validates a unit local sky direction; and `ray_seed` returns normalized Carter constants, Mino initial data, observer location and an explicit preferred coordinate chart. Other observer worldlines require their own tetrads. Negative Killing-energy Penrose-process rays remain outside the current (E>0) constant type.

This is an adaptive separated Kerr ray integrator, but not yet a complete ray-bundle solver. The canonical Hamiltonian path now integrates trajectories via the separate adaptive step-doubling module. No variational Jacobi transport, caustic detection, image-branch topology tracking or wave optics is included yet. The ingoing chart integration is restricted to (r>0) and (0<\theta<\pi); it does not claim passage through the ring singularity or polar axis. Tests are authored but not executed in this environment.

## Schwarzschild geodesic scope

The current geodesic module is deliberately limited to the Schwarzschild exterior chart. It offers two independent formulations:

1. The second-order geodesic equation using the Levi-Civita connection.
2. First-order canonical Hamilton equations using (H=\frac12 g^{\mu\nu}p_\mu p_\nu).

The coordinate formulation also supports step-doubling adaptive integration. Its absolute tolerances are split by coordinate and tangent dimensions; the adaptive error estimate is a local controller, not a rigorous global error bound.

Timelike trajectories use `lambda = c*tau` and target (g(u,u)=-1). Null trajectories use an affine parameter and target (g(u,u)=0). Both formulations expose normalization, Killing energy, axial angular momentum, and Hamiltonian diagnostics. They reject the Schwarzschild horizon and polar-axis chart boundaries rather than crossing a coordinate singularity.

The machine-readable fixtures are in `benchmarks/geodesic_reference_cases.json`. An adaptive separated Kerr Mino stepper is implemented. Adaptive canonical-Hamiltonian integration, finite-radius observer tetrads, variational Jacobi fields and caustic detection remain unimplemented.
