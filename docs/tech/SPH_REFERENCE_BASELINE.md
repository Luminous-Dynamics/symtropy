# Native SPH Reference Baseline: equations, evidence, and benchmark ladder

**Status:** research/implementation contract; not a claim that the solver currently passes these tests.  
**Parent:** [HYDRO-LOCAL-03 #452](https://github.com/Luminous-Dynamics/symtropy/issues/452).  
**First tranche:** [HYDRO-SPH-CORE-001 #2879](https://github.com/Luminous-Dynamics/symtropy/issues/2879).

## Executive decision

Do not begin with GPU acceleration, a renderer, or an AI surrogate. First make the CPU SPH path a physically specified, independently testable reference. Keep a deliberately slow all-pairs neighbor path as the correctness oracle; optimize only after the interaction set and equations are verified. Treat fluid simulation, free-surface/boundary modeling, local water ownership, and learned flow prediction as separate claims.

## 1. Source audit: current known defects

Inspected `crates/domains/symtropy-fluid/src/lib.rs` on `main`; blob SHA `8ad464a05133d900f1c4ca33b8f2f157fec6c4ab`.

- `calculate_density_pressure` uses the 3D Poly6 coefficient `315/(64πh^9)` despite `FluidSimulation<const D: usize>`.
- Density is forcibly clamped to `rest_density`, masking under-density and changing the equation.
- Pressure and viscosity force accumulators are initialized to zero and never populated.
- Density neighborhood calculation loops over all N² particle pairs.
- `SphConfig` values, `dt`, gravity, particle mass/position/velocity, and arithmetic outputs are not validated.
- `pressure_stiffness`, `viscosity`, and `FluidParticle.force` do not state units or equation semantics.
- The current update `velocity += force * (dt / density)` treats `force` as a force-density-like value, not a conventional force or acceleration; this contract is undocumented.

This audit is source reading only. No compile, test, or runtime measurement is claimed.

## 2. Declare the first model before coding

Recommended baseline: weakly-compressible SPH (WCSPH) with density summation, an explicitly named equation of state (EOS), a symmetric pressure-gradient operator, a separately specified viscous operator, an explicit boundary treatment, and a named time integrator. The first implementation should support and validate D=2 and D=3; mathematical kernel normalization can be generalized to positive integer D, but that does not prove a meaningful fluid model in arbitrary D.

All quantities must declare their units. In ordinary 3D SI units: position [m], velocity [m/s], mass [kg], density [kg/m³], pressure [Pa], dynamic viscosity [Pa·s] or kinematic viscosity [m²/s] (never an ambiguous blend), acceleration [m/s²], smoothing length [m], and time [s]. If D=2 is treated as a planar model, state explicitly that density is areal mass density [kg/m²] and coefficients/units follow that 2D model.

### Density summation

\[ \rho_i = \sum_j m_j W(\|x_i-x_j\|, h). \]

Do not replace the solved density with `max(rho_i, rho_0)`. Validate masses and the computed density separately. Boundary deficiency (missing kernel support near a wall/free surface) requires an explicit boundary/renormalization policy, not a global clamp.

### Dimension-correct Poly6 reference kernel

For the existing compact-support form as a first reference:

\[ W_D(r,h) = C_D(h) (h^2-r^2)^3_+, \qquad (z)_+ = \max(z,0), \]

\[ C_D(h) = \frac{\Gamma(D/2+4)}{6\pi^{D/2}h^{D+6}}, \qquad D \in \{1,2,3,\ldots\},\ h>0. \]

This coefficient normalizes the kernel over an unbounded D-dimensional Euclidean domain. It does not correct truncated support at boundaries. For numerical safety, compute or validate the scale before use; non-finite normalization, overflow/underflow, D=0, and unsupported numeric ranges must return a typed error or invalid-step outcome, never silently fall back to the 3D factor.

Required kernel references:

- `integral(W_D) ≈ 1` under independent numerical quadrature for D=1,2,3 and a frozen range of h values.
- `W_D ≥ 0`, compact support at r ≥ h, radial symmetry, and monotonic decrease over its positive support.
- Gradient for 0 < r < h follows the derivative of the same kernel: `∇_i W = -6 C_D (h²-r²)² (x_i-x_j)`. Verify it against central finite differences away from the support boundary and compare the derivative to zero outside support.
- Compare against standard SPH kernels (e.g. cubic spline) on fixed canonical workloads before choosing the production kernel. A correct normalization is necessary, not proof of low discretization error.

### Equation of state and pressure forces

One simple linearized WCSPH EOS is:

\[ p_i = c_0^2(\rho_i-\rho_0), \]

where `c0` is an explicit reference sound speed and `pressure_stiffness` must not be described as a dimensionless tuning number if it is being used as `c0²`. Tait EOS is a separate model option; it must be named and validated separately. Do not silently suppress negative pressure or disguise it by changing density. If a tensile-instability policy is needed, it must be a declared model option with its own reference evidence.

A standard symmetric pressure acceleration form is:

\[ a_i^{p} = -\sum_j m_j\left(\frac{p_i}{\rho_i^2}+\frac{p_j}{\rho_j^2}\right)\nabla_i W_{ij}. \]

For the selected symmetric kernel and pair evaluation, the pairwise pressure interactions should be equal and opposite in momentum space up to floating-point error. Add an isolated two-particle oracle and a multi-particle closed-system momentum budget. Do not multiply or divide by density twice: store/update acceleration, force, or force density with an explicit semantic contract and consistent units.

### Viscosity is its own numerical claim

Do not fill the viscosity accumulator with a generic smoothing term and call it Newtonian viscosity. Physical viscosity and artificial shock-stabilization viscosity have different meanings and must not share one ambiguous configuration field.

The first code profile now uses a **Morris-style regularized pair operator** with a spatially uniform dynamic viscosity μ:

\[
\mathbf a_i^{\nu} = \sum_j m_j \frac{\mu_i+\mu_j}{\rho_i\rho_j}
\frac{\mathbf r_{ij}\cdot\nabla_i W_{ij}}{\lVert\mathbf r_{ij}\rVert^2 + \eta^2 h^2}
(\mathbf v_i-\mathbf v_j),
\qquad \mathbf r_{ij}=\mathbf x_i-\mathbf x_j.
\]

With a uniform μ, the code uses μᵢ + μⱼ = 2μ; the regularizer is η²h² with η = 0.01. For the selected Poly6 kernel, r·∇W is non-positive inside support, so the pair term damps relative velocity. The code’s dynamic-viscosity units must not be conflated with a reference implementation whose parameter is supplied as kinematic viscosity. PySPH’s [Morris viscosity implementation](https://github.com/pypr/pysph/blob/main/pysph/sph/surface_tension.py#L2507-L2530) is a structural reference for the regularized pairwise form; comparisons must establish unit conventions at the API boundary.

The pair form, coefficient, sign, units, numerical regularization, and boundary assumptions still require independent shear-flow verification. Test it using Couette/Poiseuille flow or a manufactured shear solution where the chosen boundary discretization supports a valid comparison. Measure the decay of a shear mode and verify dissipative behavior; do not infer correctness from a visually smoother result. Keep artificial viscosity as a separate named option for compressive/high-Mach stabilization if needed.

### Forces, boundary conditions, integrator, and timestep

- Rename/document any acceleration or force-density buffer before other code depends on it; public API stability is not a reason to preserve dimensionally ambiguous semantics in a 0.1 crate.
- Gravity is an acceleration [m/s²]; it is not multiplied into an undefined force quantity.
- Make walls, free surfaces, periodic boundaries, and ghost/boundary particles explicit. Kernel normalization does not constitute boundary validation.
- Declare integrator and timestep policy. At minimum, enforce configured sound-speed/advective and viscous restrictions of the chosen scheme; a useful starting shape is `dt ≤ C_cfl h/(c0+v_max)` plus a viscosity bound proportional to `h²/ν`, with constants derived and frozen for the selected discretization, not guessed per scene.
- Validate finite positive h, positive rest density, non-negative viscosity, positive finite masses, finite initial states/gravity, and finite positive dt. Reject D=0 and invalid arithmetic explicitly.

## 3. Independent oracle ladder

### Gate K — kernel and units

Test D=1,2,3 quadrature; normalization; gradient finite-difference; support edge; scale metamorphism; invalid h; non-finite and extreme-range cases. Store reference outputs independently from the implementation under test.

### Gate E — equation-level mechanics

- One particle in uniform gravity: analytic position/velocity trajectory for the declared integrator and time discretization.
- Density from a known particle configuration compared to a direct reference sum.
- Under-density is observed, not clamped to the rest density.
- Symmetric two-particle pressure interaction checks momentum balance and force sign.
- Viscosity test checks shear decay, sign, units, and convergence against the selected published/manufactured solution.
- Every invalid-input negative control has a typed, reproducible outcome.

### Gate F — canonical fluid cases

Use a staged subset from the SPHERIC and DualSPHysics benchmark families:

1. Static equilibrium and simple manufactured/analytic flow.
2. Couette/Poiseuille flow when the boundary formulation and continuum assumptions fit.
3. Taylor–Green or Gresho-Chan vortex for velocity/vorticity evolution and resolution effects.
4. Periodic-box decay where periodicity and the forcing model are explicit.
5. Dam-break, sloshing, and fluid–body impact only after free-surface and solid-boundary semantics are implemented.

Do not use dam-break as the first correctness oracle: it couples equations, free-surface detection, boundaries, time integration, and impact loads, so a failure is hard to localize.

### Gate N — neighbor acceleration parity

Retain a direct O(N²) neighbor list for small systems. Build uniform-grid/cell-list candidates as an optimization and compare candidate sets exactly against brute force on frozen random and adversarial particle sets, including particles at cell edges, at exactly the support radius, highly nonuniform distributions, duplicate positions, negative coordinates, and large coordinate ranges. Only after set parity passes should the optimized path be used in release benchmarks.

### Gate C — conservation, convergence, and cost

Publish per-case mass and linear momentum residual, kinetic/internal-energy or dissipative budget where defined, density extrema, finite-state counts, minimum inter-particle distance, and failure/unstable timestep counts. Refine spatial resolution and timestep independently where meaningful; show observed trends rather than one best-looking run.

Record p50/p95/p99 wall time, peak memory, particle count, candidate interactions, neighbor-build cost, density cost, pressure cost, viscosity cost, and integration cost separately. Benchmark scaling at increasing N; do not claim GPU benefits until dispatch exists and correctness against the CPU reference is shown.

## 4. Proposed acceptance thresholds

Freeze tolerances per fixture after comparing reference-data uncertainty and expected numerical order. Avoid pretending a single percentage covers every physical case.

- Kernel quadrature relative error ≤ 1e-8 for a sufficiently resolved deterministic quadrature fixture, with quadrature convergence checked so the reference itself is trustworthy.
- Pairwise pressure momentum residual ≤ a preregistered floating-point-scaled tolerance.
- Zero NaN/infinity states, negative masses, silently clamped densities, or false-success outcomes for invalid input.
- Canonical analytic/manufactured solutions show the expected convergence trend across at least three resolutions/time steps where the method predicts one.
- Nearest-neighbor sets from the optimized search are exactly equal to the brute-force reference for the frozen parity corpus.
- Report mass/momentum and energy/dissipation budgets; a threshold is accepted only when justified for the selected discrete scheme and boundaries.
- Report every failed, unstable, out-of-domain, or unresolved case; no selective omission from aggregate metrics.

These are proposed engineering gates, not measured results. Tune on development fixtures, freeze on validation fixtures, and publish untouched confirmation outcomes.

## 5. Compare against the field without mixing unlike goals

| Reference | Appropriate role | What not to claim |
|---|---|---|
| PySPH | Equation/kernel reference implementation and independent CPU comparison on matched SPH schemes | An equation match does not mean equal numerical behavior if neighbor, boundary, kernel, or timestep choices differ |
| SPHERIC benchmarks | Standardized SPH validation cases spanning convergence/stability, boundaries, viscosity/turbulence, free surfaces, coupling | Passing a small subset does not make Symtropy an industrial all-purpose SPH solver |
| DualSPHysics | Free-surface, periodic, sloshing, wave and structure-interaction cases; practical external SPH comparator | Rendering similarity does not establish force/pressure agreement |
| SU2 and NASA Turbulence Modeling Resource | Later aerodynamic/continuum reference cases, turbulence verification and experiment comparison | These are not a like-for-like solver speed comparison with a particle method |
| PDEBench | Scientific-ML benchmark design: score learned models against conventional numerical baselines on matched tasks | Prediction speed alone does not imply physical conservation or calibrated uncertainty |
| GenCFD (Nature Communications, 2026) | Statistical turbulent-flow ensemble generation; evaluate distributional moments/spectra and uncertainty separately | A statistically accurate ensemble is not necessarily a correct deterministic trajectory for a particular initial state |

Useful external anchors:

- PySPH kernels: https://pysph.readthedocs.io/en/main/reference/kernels.html
- PySPH equations: https://pysph.readthedocs.io/en/main/reference/equations.html
- SPHERIC validation tests: https://www.spheric-sph.org/validation-tests
- DualSPHysics test cases: https://github.com/DualSPHysics/DualSPHysics/wiki/7.-Testcases
- NASA Turbulence Modeling Resource: https://tmbwg.github.io/turbmodels/
- SU2: https://su2code.github.io/
- PDEBench: https://proceedings.neurips.cc/paper_files/paper/2022/hash/0a9747136d411fb83f0cf81820d44afb-Abstract-Datasets_and_Benchmarks.html
- GenCFD: https://www.nature.com/articles/s41467-026-76390-x

## 6. Paradigm shift: a multi-fidelity evidence graph

The long-term differentiator should be a replayable **model-evidence graph**, not a claim to replace every CFD package:

1. Symtropy-native SPH supplies fast local/free-surface approximations only inside its validated envelope.
2. An established continuum solver such as SU2 supplies external CFD reference runs for relevant aerodynamic cases.
3. The scenario receipt binds geometry, units, mesh/particles, initial and boundary conditions, solver version, configuration, timestep, precision/backend, and raw-output digest.
4. Symthaea selects experiments and proposes candidate geometries/parameters, but is not allowed to certify its own predictions.
5. An independent verifier checks invariants, reference errors, convergence, uncertainty, and whether a case is in the validated domain.
6. A surrogate may route expensive cases or estimate a distribution, but must abstain when out of domain and never substitute its confidence score for physical validation.

Keep the physical baseline free of implicit Φ/consciousness coupling. If a research experiment intentionally lets Φ alter a constitutive law or control policy, label it as a different model family, compare against the uncoupled physical baseline, and report its own equations and evidence.

## 7. Definition of done for this tranche

- [ ] Current source audit is linked and refreshed against the implementation base SHA.
- [ ] Equation and unit contracts are fixed before tuning.
- [ ] D=1,2,3 kernel fixtures pass an independent normalization oracle.
- [ ] The density floor is removed and EOS/tensile behavior is explicit.
- [ ] Pressure and viscosity no longer have placeholder implementations in the claimed active profile.
- [ ] Pairwise pressure momentum and viscosity dissipation tests pass.
- [ ] Optimized neighbor search exactly matches brute force on a frozen corpus.
- [ ] Canonical cases and convergence reports are reproducible from pinned scenario/config/toolchain digests.
- [ ] External solver comparisons use matched cases and separately report physical accuracy, solver cost, and uncertainty.
- [ ] No SOTA/superiority claim is made before an independently reproducible comparison.

## Claim ceiling

This document is a design/validation contract. It does not implement a solver, establish a pass, or demonstrate that Symtropy outperforms PySPH, DualSPHysics, SU2, OpenFOAM, or any other solver. The immediate aim is a correctly specified reference kernel; engineering-grade CFD and aerospace predictions require much larger validation campaigns.