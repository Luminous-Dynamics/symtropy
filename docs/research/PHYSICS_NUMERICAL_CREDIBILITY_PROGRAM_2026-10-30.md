# Physics Numerical Credibility Program

**Date:** 2026-10-30  
**Status:** Research and implementation charter. This document does not claim a new solver capability.

## Executive decision

Do not expand the physics catalogue first. Build a small, composable numerical foundation and a credibility pipeline that makes every engineering claim executable, measurable, reproducible, and explicitly bounded.

The intended division of responsibility is:

- **Symtropy:** execute physical models, geometry, time integration, constraints, contact, fields, and conservation accounting.
- **Symthaea:** reason over model choice, literature, experiment design, anomaly detection, uncertainty, and explanations. It may propose models or experiments; it does not certify a numerical result.
- **Independent references:** establish correctness through analytical solutions, manufactured solutions, high-precision or independently implemented solvers, and eventually physical measurements.

The first competitive advantage should not be "we have more physics modules." It should be: **a user can tell what a model predicts, why it is credible, where it fails, how uncertain it is, and reproduce the evidence from a pinned revision.**

## 1. Source inspection: current baseline

This programme was scoped against the public main branch on 2026-10-30.

### Symtropy

The current symtropy-physics crate is an N-dimensional rigid-body engine built around RigidBody<D>, PhysicsWorld<D>, GJK/EPA collision queries, contact manifolds, joints, and a semi-implicit Euler integrator. The crate currently depends on nalgebra, serde, and arrayvec; it does not yet expose a general-purpose sparse PDE/DAE solver stack.

Relevant code:

- [symtropy-physics/src/lib.rs](../../crates/core/symtropy-physics/src/lib.rs)
- [integrator.rs](../../crates/core/symtropy-physics/src/integrator.rs)
- [body.rs](../../crates/core/symtropy-physics/src/body.rs)
- [world.rs](../../crates/core/symtropy-physics/src/world.rs)
- [diagnostics.rs](../../crates/core/symtropy-physics/src/diagnostics.rs)
- [RESEARCH_VALIDATION.md](../../crates/core/symtropy-physics/RESEARCH_VALIDATION.md)

Important implementation facts:

1. Translational rigid-body integration is semi-implicit Euler. That is a useful real-time baseline, not a complete adaptive scientific integration suite.
2. Rotational acceleration and angular impulses currently use a mean-of-axis inverse-inertia approximation. The source itself marks this as exact for isotropic bodies and not physically correct for general asymmetric bodies. This affects rotational response and energy accounting.
3. Articulated-chain tests primarily establish finite state and bounded motion; they do not yet establish high-accuracy forward/inverse dynamics against a reference.
4. InvariantSnapshot already reports useful quantities such as momentum, energy, penetration, rotation orthogonality/determinant error, and non-finite body state. This should become part of every physics benchmark record.
5. The validation protocol already defines mathematical, analytical, independent-comparison, and reproducible-campaign tiers; it also requires at least four timesteps for convergence studies. Preserve and extend this contract rather than replacing it.

The current validation document explicitly does **not** certify asymmetric-body inertia, gyroscopic coupling, rotational CCD, general articulation/inverse dynamics, production fluids/deformables/fracture/FEM/MPM, or cross-architecture bit-identical determinism.

### Symthaea

The current symthaea-core physics tree contains many domain representations and formula-level validation tests. Its generic dynamical-system implementation documents Euler, RK4, adaptive RK45, and Verlet, with HDC trajectory encoding layered on top. The current interface returns allocated Vec<f64> state/derivative values and is a useful research bridge, not yet a replacement for a scalable solver library.

Keep the distinction explicit:

- **Knowledge model:** equations, concepts, assumptions, references, and HDC encoding.
- **Executable model:** a numerical implementation with units, inputs, initial/boundary conditions, and solver behavior.
- **Validated model:** measured error, convergence, an independent reference, a validity envelope, uncertainty, and reproducible evidence.

A passing formula test must never promote a knowledge model directly to engineering-ready status.

Symthaea's current Cash–Karp RK45 also uses a single absolute max-norm tolerance, not a component-scaled absolute/relative error norm. That is acceptable as a small pedagogical integrator but is insufficient as the only error controller for mixed-scale engineering state vectors. The proposed interval-alignment fix is necessary but not the end-state solver architecture. Prefer diffsol's explicit absolute/relative tolerance model for research-grade integration, and retain the in-tree integrators for simple baselines and specialized low-dimensional dynamics.

A cross-repository audit also found a concrete time-alignment defect in Symthaea's generic Cash–Karp RK45 wrapper: a rejected macro-step may produce a shorter accepted substep, while the trait wrapper returns only the new state and the generic simulation loop advances time by the original macro-step. This can make trajectory state and recorded timestamps disagree. A fail-closed interval-wrapper correction and phase-sensitive harmonic-oscillator regression are proposed in [Symthaea PR #9083](https://github.com/Luminous-Dynamics/symthaea/pull/9083). It is not counted as fixed or validated until that change is reviewed and its test is run.


## 2. The paradigm shifts

### 2.1 From a physics catalogue to a model contract

Every executable model should declare a machine-readable contract:

- state variables and parameters, with dimensions and units;
- governing equations or residual form;
- constitutive laws and their provenance;
- domain, assumptions, valid parameter ranges, and excluded regimes;
- initial and boundary conditions;
- conserved quantities and expected external exchanges;
- numerical method, tolerances, and stopping criteria;
- observables, uncertainty sources, and independent references;
- validation level and promotion evidence.

This creates a **credibility compiler**: a model cannot be labelled validated unless required evidence is present and the relevant gates pass. The compiler should reject missing units, inconsistent dimensions, undefined boundary conditions, invalid parameter ranges, and incomplete benchmark provenance before long simulations begin.

### 2.2 Separate real-time simulation from research-grade simulation

Do not force one integrator to serve a game loop, robotics controller, design optimizer, and scientific solver.

Provide two explicit execution profiles:

- **Realtime:** bounded runtime, predictable allocation behavior, robust contact handling, and documented approximations.
- **Research:** adaptive timesteps, local error estimates, stiff and implicit methods, nonlinear convergence diagnostics, higher precision where useful, and complete solver statistics.

They may share state, geometry, material, and conservation contracts. They must not silently share accuracy claims.

### 2.3 Make conservation and coupling first-class interfaces

Multiphysics should not be a collection of callbacks that happen to exchange numbers. Model each subsystem through typed ports carrying:

- a physical quantity and unit;
- a state or flux;
- an orientation/sign convention;
- the interface surface or control volume;
- the exchange rate and time interval;
- the source/sink attribution.

For a closed coupled system, energy and mass ledgers should reconcile across domains. For an open system, every boundary flux, actuator, source, and sink must be explicit. Coupling should support both partitioned and monolithic formulations, with residual and coupling-iteration convergence recorded.

This enables motor electrical loss → heat generation → temperature-dependent resistance, or fluid pressure → structural load → deformation → changed flow geometry, without hiding energy transfer in ad hoc glue.

### 2.4 Make model error and uncertainty part of the result

A simulation result should not be a single curve. It should include:

- discretization and integration error estimates;
- parameter and measurement uncertainty;
- model-form discrepancy;
- sensitivity ranking;
- uncertainty propagation method;
- calibration domain and held-out validation error;
- prediction interval coverage and sharpness.

A calibrated 95% prediction interval that covers only 60% of held-out measurements is a failed model, even if the mean trajectory looks plausible.

### 2.5 Use cognition to improve science, not to replace numerical evidence

Symthaea should propose the next experiment by expected information gain, detect regime changes, identify candidate model discrepancies, and suggest solver/model alternatives. The experiment planner must score proposals against cost, identifiability, safety, and decision value.

Every proposed improvement is then tested by Symtropy and an independent verifier. HDC similarity, confidence, or a generated explanation is never numerical validation.

### 2.6 Treat rotational inertia as an operator on the rotation algebra

The current body state stores one inertia value per spatial axis, while integration, angular impulse application, contact response, and kinetic-energy accounting collapse those values to a scalar mean. That is only physically exact for isotropic mass distributions.

For 3D bodies, implement the full symmetric positive-definite body-space inertia tensor, transform it correctly into world space, and use the gyroscopic term in Euler's rigid-body equations. Contact impulses must use the same mass/inertia model as free integration.

For the project's dimension-generic rotation model, do not simply generalize a 3D tensor into a D-element vector. Angular velocity lives in the bivector space Λ²(R^D), whose dimension is D(D−1)/2. The general rotational inertia is a linear operator on that space (with the physical constraints induced by the body's mass distribution). In 4D this is a 6-component angular-velocity state and a corresponding operator, not a 4-vector of axis inertias.

The kinetic-energy contract should be 1/2 ⟨ω, Iω⟩, and the equations of motion must include the appropriate coadjoint/gyroscopic term. This gives a principled route from the 3D asymmetric top to the existing 4D bivector rotation architecture.

A first mathematical primitive now exists in `crates/core/symtropy-physics/src/inertia.rs`:

- It stores the centered second-moment matrix (S = \int rr^T\,dm).
- It maps Symtropy's **stored rotor bivector** B to angular momentum with I(B)=B*S+S*B. Rotor increments are exp(-B), so physical point velocity is -B*r. In 3D, the equivalent conventional inertia tensor is J=tr(S)I-S.
- It accepts a centered second-moment matrix, a uniform solid D-ball or D-ellipsoid, or a discrete point-mass cloud (which first computes the center of mass, making the result translation-invariant).
- It can rotate body-space mass properties into another frame with S_world=R*S_body*R^T.
- It can invert angular momentum back to the stored rotor bivector B using the eigensystem of S, rejecting singular/ill-conditioned rotation planes.
- It validates finiteness, symmetry, and positive semidefiniteness (with a scale-relative eigenvalue tolerance), projecting roundoff-sized negative eigenvalues onto the PSD boundary so the forward and cached inverse operators use the same matrix.
- It also computes stored-state angular acceleration using the frame-consistent Euler-Arnold equation. Tests cover solid balls, ellipsoids, point clouds, principal moments, cross-plane coupling, rotation covariance, inverse round trips including all six coupled 4D rotation planes, gyroscopic energy and momentum invariants, and invalid/singular mass-moment matrices.

There is an essential sign convention at the Symtropy integration boundary: `Rotor::from_bivector(B)` computes exp(-B), and the integrator left-composes that increment. Thus `RigidBody.angular_velocity` stores B while the physical point-velocity matrix is -B. The `BivectorInertia` angular-momentum, inverse, and angular-acceleration APIs operate directly on the stored B basis; do not negate the state before calling them. The shared `integrator::angular_point_velocity` helper applies the minus sign to obtain physical point velocity.

The primitive is now connected to the runtime path for bodies carrying an authoritative `BivectorInertia`:

- `RigidBody` carries an optional body-space mass-moment operator. The dimension-correct solid-ball constructor populates it; other dynamic bodies can set it explicitly.
- Free angular integration rotates the cached principal-axis basis into world coordinates and evaluates the spatial gyroscopic derivative.
- Angular impulses and normal/friction contact effective-mass terms use the same world-frame inverse operator.
- `RigidBody::kinetic_energy` uses the operator, so the existing world energy snapshot sees the same rotational energy.
- The eigensystem is computed once when constructing mass properties; rotation updates the basis rather than repeating an eigendecomposition at every contact.
- `merge_toolhead` now combines second moments using the parallel-axis theorem, rebases collider-child transforms to the new local COM, and moves the parent origin by the rotated COM offset. A geometry regression checks that the original collider remains fixed in world coordinates.

The integration remains **partial rather than a general CAD mass-properties solution**. `RigidBody::new` retains a legacy fallback unless the caller supplies an operator. The toolhead model is still approximated as a uniform solid D-ball of radius 0.1. General mesh/tetrahedral mass integration, compound-body provenance, and fully articulated dynamics are not complete. Invalid/singular inverse operations are counted, but any nonzero failure counter must block promotion; the simulation must not be treated as validated merely because it continues running.

A cross-path regression now checks that contact effective mass predicts the point-velocity change produced by applying the same angular impulse to a rotated asymmetric body. Separate tests cover analytical moments, energy/momentum invariants, body/world covariance, parallel-axis composition, and COM geometry rebasing. These Rust tests have not yet run in this environment.


### 2.7 Enforce one rotation sign and frame contract

Source inspection found a kinematic sign inconsistency:

- `Rotor::from_bivector(B)` evaluates exp(-B).
- `integrator::integrate` left-composes that rotor increment, so for a world-space offset r the actual point velocity is -B*r.
- Contact, friction, and restitution had been using +B*r.
- The torque accumulator is constructed with the impulse-first wedge convention, so its update into the stored B state must be negative to produce the expected physical point velocity.

The current branch now centralizes the conversion in `integrator::angular_point_velocity`, uses it at all contact/restitution/friction call sites, subtracts angular impulse and applied-torque increments from the stored B state, and adds regression tests comparing the helper with a finite-difference rotor derivative and checking that an off-center impulse produces point velocity in the impulse direction.

This corrects the stored/physical angular-velocity sign relationship and centralizes point velocity in one helper. The full inertia operator now traverses the free-integration, angular-impulse, kinetic-energy, normal-contact and friction paths for operator-backed bodies. Its frame contract is explicit: `B` is the stored world bivector, physical point velocity is `-B*r`, physical angular momentum is the negative of `I(B)`, and the impulse-first wedge accumulator is the physical torque's skew-matrix representation. The gyroscopic derivative accounts for the rotating world second-moment matrix. Remaining validation is empirical: run the full Rust suites, compare asymmetric-top trajectories and contact responses against independent references, prove failure counters remain zero on valid cases, and quantify runtime cost.

### 2.8 Benchmark the unified contact/inertia contract

The most important new benchmark is not a free-fall speed test. It is **cross-path physical consistency**: the inertia used by free integration, angular impulses, kinetic energy, and contact effective mass must describe the same body in the same frame.

Required P0 tests:

1. **Impulse/effective-mass identity.** For an arbitrary positive-definite second moment, body rotation, contact offset r, and unit direction n, the effective-mass scalar used in the contact denominator must equal n dot delta-v divided by impulse magnitude when the exact same angular impulse is applied. Target: relative discrepancy <= 1e-10 in double precision for well-conditioned inputs.
2. **Asymmetric free top.** Compare symmetric and asymmetric tops against an independent high-accuracy reference. Track world angular-momentum drift, kinetic-energy drift, orientation error, and step-size convergence. Start with a 3D top, then exercise all six rotational degrees of freedom in 4D.
3. **Parallel-axis and COM rebase.** Compare compound second moments against analytically constructed point clouds. Collider geometry must remain fixed in world coordinates when the COM moves. Target: relative second-moment error <= 1e-12 for analytical primitives and geometric displacement <= 1e-12 times scene scale.
4. **Scale and conditioning.** Repeat the same geometry at 1e-9, 1, and 1e9 length scales and at large translated world origins. Record conditioning, inverse failures, and normalized errors. Do not silently accept a nonzero rotational-inertia failure counter.
5. **Contact stability.** Measure maximum and 95th-percentile penetration, resting jitter RMS, restitution error, frictional energy dissipation, stack collapse rate, and cost per solved contact. Compare at matched scene geometry and matched error tolerance.

Proposed promotion targets (not achieved results): valid-case singular/invalid operator failures = 0; impulse/effective-mass identity <= 1e-10; analytical mass-moment and COM errors <= 1e-12 relative where representable; torque-free reference drift <= 1e-6 over the short canonical run and a separately reported long-horizon drift curve; and no tunneling in the defined adversarial CCD set. Thresholds must be calibrated against reference precision and committed before measurement.

### 2.9 Industry comparison: compete on matched work-precision

Pin exact versions and settings in each result receipt. The current comparison baselines are materially stronger than a simple explicit-Euler reference:

- **MuJoCo 3.17.0** (released 22 October 2026) is the current pinned comparison target. Its `discrete` integrator (introduced in 3.13) merges constraints and implicit velocity updates in the effective metric \\(\\widehat M=M+hD+h^2K\\); 3.17 extends use of that path to flex bending. The 3.14 IPC flex-contact experiment is penetration-free through CCD-verified commits but currently limited to 2D flexes and frictionless contact; its persistent multipliers are not fully represented in saved state, so exact replay is not supported for that mode. MuJoCo 3.17 also adds `mjbatch`, documenting bit-identical results to a serial loop across thread counts. Benchmark `implicitfast` and `discrete` separately, and include batch throughput plus replay fidelity. See [3.17 changelog](https://mujoco.readthedocs.io/en/3.17.0/changelog.html) and [computation](https://mujoco.readthedocs.io/en/latest/computation/).
- **Rapier 0.36.1** (10 October 2026) is the current 2D/3D Rust release. Version 0.36 adds soft-body simulation; the Q3 2026 engineering report describes reworked parallelism with intra-island parallel solving, persistent graph coloring and parallel broad phase. Its Rust guide documents 4 default constraint iterations and 8–12 as a reasonable range for demanding stacks/stiff joints. Rapier 0.35.2 also added a multibody safeguard that re-solves with the plain mass matrix when the semi-implicit Coriolis term would inject more energy than the applied forces' work. Compare at matched residual/error and record PGS/stabilization iterations, warm starts, CCD and thread configuration. See [Rapier 0.36.1 package](https://docs.rs/rapier3d/0.36.1/rapier3d/), [integration parameters](https://rapier.rs/docs/user_guides/rust/integration_parameters/), [Q3 2026 report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/), and [changelog](https://github.com/dimforge/rapier/blob/master/CHANGELOG.md).
- **NVIDIA PhysX 5.8** documents Temporal Gauss–Seidel (TGS) as its recommended solver family, with advantages in convergence, high mass ratios, joint-drive accuracy and penetration-correction behavior. Its friction patches and position/velocity iteration split must be represented in the comparison settings; comparing only a single iteration count is misleading. See [PhysX 5.8 simulation](https://nvidia-omniverse.github.io/PhysX/physx/5.8.0/docs/Simulation.html).
- **Newton Physics** is an open-source, GPU-accelerated, differentiable robotics engine built on Warp and OpenUSD, developed by NVIDIA, Google DeepMind and Disney Research and managed by the Linux Foundation. Its documented solver families include XPBD, VBD, MuJoCo, Featherstone, SemiImplicit, Kamino and implicit MPM. This is a major baseline for GPU-parallel robot learning and solver extensibility, although its Warp/Python ecosystem and robot-training workflow are not identical to Symtropy's Rust/N-D target. See [Newton overview](https://newton-physics.github.io/newton/stable/guide/overview.html), [NVIDIA Newton](https://developer.nvidia.com/newton-physics), and [Isaac Lab Newton integration](https://docs.nvidia.com/isaaclab/concepts/newton).
- **Project Chrono 10.0.0** (27 March 2026) is a mature engineering multibody/multiphysics baseline, with FSI/peridynamics improvements, checkpointing and new Jacobian update strategies for implicit integrators. Use it for mechanical/continuum verification cases, not as a drop-in real-time game-engine comparison. See [Chrono 10.0.0 release news](https://projectchrono.org/news/).
- **Symtropy's defensible opportunity** is dimension-generic rotational mechanics (D(D-1)/2 bivector coordinates), a shared mass-moment operator across contact/integration, explicit physical invariants, reproducible simulation receipts, and eventually a unified discrete constraint/integration solve. These are research hypotheses, not evidence of superiority yet.

Do not compare only frames per second or solver iteration count. Compare work-precision curves: error versus wall time, error versus energy/invariant drift, contact penetration versus time, and failure/tunneling probability versus computational budget. Keep interactive and high-accuracy research profiles separate.

## 3. Numerical architecture: build versus reuse

Do not write a broad sparse linear algebra package, automatic differentiation system, or stiff ODE solver from scratch before benchmarking existing Rust capabilities.

First evaluate:

- **diffsol 0.18.0 (published 2026-10-20):** Rust ODE and semi-explicit DAE solvers including variable-order BDF (including singular mass matrices), SDIRK/ESDIRK, and variable-order explicit TSIT45. Its API documents relative and absolute tolerances, optional mass matrices, dense output, event/root handling, quadrature, checkpointing, forward sensitivities and backward/adjoint sensitivities. It supports nalgebra and faer matrix types, including sparse faer solvers; optional features include SuiteSparse KLU and DiffSL JIT backends. The crate is MIT-licensed.
- **faer:** Rust dense and sparse matrix factorizations, including sparse LLᵀ, LDLᵀ-style LBLᵀ, LU, and QR paths. Keep nalgebra for small fixed-size mechanics vectors/matrices; faer is aimed at medium/large matrix workloads and is not a reason to replace nalgebra in the low-dimensional collision/rigid-body hot path.
- **SciML / OrdinaryDiffEq:** a solver-selection and performance reference across non-stiff, stiff, symplectic, exponential, and implicit methods.
- **PETSc:** reference for scalable Krylov linear solvers, nonlinear solvers, time stepping, and optimization.
- **MOOSE / FEniCSx / OpenFOAM:** independent reference ecosystems for coupled finite-element, finite-volume, and CFD problems.

A caveat matters for sparse solver integration: diffsol can attempt to infer Jacobian sparsity through NaN propagation, but its documentation explicitly notes that this is not foolproof when Jacobian code contains value-dependent control flow. For serious production models, supply and test explicit sparsity patterns or Jacobians rather than relying on inference.

The current diffsol README also reports a 2026 Journal of Open Source Software paper (JOSS 11(117), 9384). Treat the library as an implementation candidate to benchmark and independently verify, not as an unquestioned oracle. Pin the exact crate version and backend in every result receipt.

An initial Rust-facing solver trait should return more than a state vector. It should expose a structured report containing accepted/rejected steps, local error estimates, nonlinear iterations, linear iterations, Jacobian evaluations, factorization counts, event hits, termination reason, and invariant diagnostics. It should preserve the backend's dense-output and sensitivity capabilities where required.

Prefer a thin adapter to a mature library where it is reliable and license-compatible. Build a native backend only when a benchmark demonstrates a meaningful gap in portability, integration, reproducibility, latency, or accuracy.

## 4. Benchmark campaigns and target gates

All benchmark inputs and thresholds must be committed before the run. Compare at matched error/tolerance, not only at matched timestep or frames per second.

### Campaign A — Numerical kernel (P0)

| Problem | What it tests | Required evidence |
|---|---|---|
| Harmonic oscillator | order, phase error, energy behavior, timestep stability | at least four dt values; measured order; stability sweep through and beyond dt·ω = 2 for semi-implicit Euler |
| Exponential decay/growth | local error and adaptive controller | tolerance-to-error calibration; accepted/rejected step counts |
| Robertson kinetics | stiff chemistry, positivity, conservation | high-accuracy reference; positivity and mass-balance residual |
| Van der Pol, small ε | stiffness detection and nonlinear solve robustness | failure rate, work-precision curve, solver switches |
| Constrained pendulum | constraint drift and DAE treatment | position/velocity residual, energy behavior, comparison with a high-accuracy reference |
| Heat equation semi-discretization | stiffness scaling and sparse linear solve | error versus mesh/time resolution; memory and solve scaling |

Initial comparison set: current Symthaea RK4/RK45/Verlet where applicable, diffsol, and SciML OrdinaryDiffEq. Include a high-precision reference for small cases. Report work-precision curves, not a single timing.

**Promotion gate:** expected convergence order is demonstrated on smooth problems; error responds monotonically to tighter tolerances within the reference envelope; unstable or failed cases remain in the report; non-finite values and solver termination reasons are explicit.

An initial executable CSV campaign has been added at [examples/numerical_convergence.rs](../../crates/core/symtropy-physics/examples/numerical_convergence.rs). Run it with `RUSTC_VERSION="$(rustc --version)" SYMTROPY_SOURCE_REV="$(git rev-parse HEAD)" cargo run --release -p symtropy-physics --example numerical_convergence`. It reports free-fall and harmonic-oscillator state error, mechanical-energy drift, observed convergence order across four step sizes, 31-run median elapsed time after five warmups, target architecture/OS, package version, optional compiler/source revision metadata, and a stable FNV-1a scenario identifier. The unit tests in integrator.rs additionally check constant-acceleration first-order convergence and a harmonic-oscillator timestep stability boundary. These are initial regression/measurement tools, not yet a full reproducibility package with pinned independent solver fixtures, cryptographic scenario/source digests, run-distribution statistics, or full machine/environment receipts; they have not been run in this environment.

### Campaign B — Rigid and articulated dynamics (P0/P1)

1. Exact free fall and constant acceleration.
2. Harmonic oscillator and simple pendulum over a timestep sweep.
3. Rotor finite-difference point-velocity consistency and off-center angular-impulse direction.
4. Torque-free spherical, symmetric, and asymmetric tops.
4. Gyroscopic precession and angular momentum conservation.
5. Rolling without slipping with correct inertia.
6. Two-body and multi-body impact, persistent contact, stacks, and restitution.
7. Fast rotating convex-body CCD and adversarial grazing contacts.
8. Articulated chains with forward and inverse dynamics.

Compare the 3D subset against MuJoCo and Rapier using identical geometry, mass properties, initial conditions, and tolerances. Use analytical solutions wherever possible; engine agreement alone is not ground truth.

For articulated-dynamics comparison, use MuJoCo's structure as a reference contract rather than comparing only endpoint trajectories:

- Its equations use a joint-space mass matrix M(q), bias forces c(q,v), applied/generalized forces, and constraint forces Jᵀf.
- It builds M(q) with a Composite Rigid-Body algorithm, computes bias forces with Recursive Newton–Euler, exploits sparsity in the articulated mass matrix, and uses sparse factorization/back-substitution.
- Add a forward/inverse dynamics consistency metric: generalized force reconstructed from inverse dynamics should match applied plus constraint forces within a preregistered residual tolerance.
- Compare mass-matrix symmetry, positive definiteness, factorization residuals, constraint-Jacobian residuals, and energy/momentum drift—not just joint-angle error.

MuJoCo's current documentation also describes a `discrete` integrator introduced in September 2026 and still under active development, alongside implicit and implicitfast approaches. Its gyroscopic treatment for standalone free bodies is a useful reference for the fast-spinning asymmetric-top campaign. Benchmark and pin the documented versions/modes rather than assuming one integrator setting represents all of MuJoCo.

For differentiable robotics, distinguish smooth dynamics from contact transitions. MuJoCo documents analytic derivatives for smooth dynamics, while contact-force onset is not differentiable under its default impedance settings. Report derivative error separately for free-flight, constrained motion, and contact-onset cases; do not publish one blanket differentiability score.

**Promotion gate:** correct inertia and gyroscopic equations; bounded constraint drift with a documented solver tolerance; conservation residuals consistent with the modeled exchanges; collision error and tunneling rates reported across adversarial cases.

### Campaign C — Continuum, fluid, and thermal models (P1/P2)

Start with manufactured or analytical solutions before large demonstrations:

- Poisson and linear elasticity patch tests;
- transient heat conduction and layered conduction;
- Poiseuille flow and lid-driven cavity;
- Taylor–Green vortex;
- Sod shock tube;
- hydrostatic column and dam break;
- conjugate heat transfer.

Then add Cook’s membrane, Hertz contact, nonlinear elasticity, plasticity, and fracture. Compare to FEniCSx, MOOSE, OpenFOAM, or published reference data as appropriate. Use mesh refinement and timestep refinement independently so spatial and temporal error are not confused.

**Promotion gate:** a complete conservation/flux balance; documented mesh quality and boundary conditions; mesh/time convergence; comparison to an independent solution; and a validity envelope for each constitutive model.

### Campaign D — Multiphysics and predictive engineering (P2)

Choose one end-to-end case rather than five shallow demos. The best initial candidate is an electric motor thermal twin:

1. electrical circuit and losses;
2. electromagnetic or calibrated torque model;
3. thermal network/conduction;
4. convection boundary model;
5. temperature-dependent resistance and material properties;
6. sensor assimilation and parameter estimation.

Start with synthetic truth and analytical subproblems, then validate on held-out measured traces. A battery electrothermal model or casting solidification model is a second candidate.

**Promotion gate:** held-out predictive error, calibrated intervals, energy balance across domains, sensitivity analysis, and a documented failure envelope.

## 5. Metrics that make comparison fair

Every benchmark record should include, where relevant:

- absolute, relative, and normalized state error;
- observed convergence order;
- conservation residuals for mass, momentum, charge, and energy;
- constraint and nonlinear residuals;
- positivity, boundedness, and non-finite counts;
- rejected steps and failed solves;
- wall time, memory, allocations, and hardware;
- error-versus-runtime and error-versus-memory Pareto curves;
- uncertainty coverage, interval width, and proper scoring rules;
- exact source revision, compiler, features, scenario hash, and environment.

Never average away failures. Publish medians and tails, successful and failed runs, and the complete parameter/timestep range.

### Proposed quantitative promotion targets

These are **initial target gates**, not current results. Freeze exact scenarios and reference data before running them; revise a threshold only through a documented, versioned benchmark decision.

| Area | Initial target | Interpretation |
|---|---|---|
| Closed-form kernels | Relative error ≤ 1e-12 for well-scaled float64 identity/mass-property cases | Catch formula and indexing defects before physical integration |
| Smooth ODE convergence | Observed order within ±0.15 of the method's formal order over at least three consecutive timestep halvings | Do not promote a solver on one timestep |
| Adaptive ODE control | Accepted-step error estimates meet configured absolute/relative tolerances; forced-minimum-step failure is explicit | No silent tolerance violation |
| Articulated forward/inverse consistency | Normalized generalized-force residual ≤ 1e-8 on small, well-conditioned double-precision reference cases | Validate mass matrix, bias terms, constraints and sign conventions together |
| Conservation under refinement | Residual decreases at the expected method order; finest-grid target ≤ 1e-8 only for a declared closed, well-conditioned reference problem | Avoid applying closed-system conservation tests to open/contact systems without accounting for external exchanges |
| PDE/CFD verification | Manufactured-solution error follows expected spatial and temporal order independently | Separate mesh error from time-integration error |
| Uncertainty calibration | Empirical 90%/95% interval coverage must be statistically compatible with nominal coverage on held-out data; report proper scores and interval width | Coverage alone cannot reward arbitrarily wide intervals |
| Solver superiority claim | At matched accuracy, ≥20% lower median wall time or memory, with uncertainty intervals excluding no improvement; alternatively ≥2× lower error at matched cost | Narrow, repeatable superiority claim, not universal SOTA language |
| Active experiment selection | ≥20% fewer physical experiments to reach the same predeclared predictive-uncertainty target, with held-out error and interval calibration non-inferior | A possible differentiated Symthaea research result |

The normalized residual, reference-conditioning, hardware, compiler, mesh, tolerance, and sample-count definitions must be written into each benchmark manifest. The numerical thresholds above are starting hypotheses: for difficult/ill-conditioned problems, the correct acceptance criterion may be convergence to a verified reference rather than a single absolute residual.

## 5.1 Determinism comparison protocol

Use Rapier's published contract precisely. Its default determinism is local to the same machine, engine version, Rust compiler, initial state, and body/collider/joint construction and insertion order. Cross-platform determinism is a separate opt-in mode with additional conditions. Symtropy should benchmark both scopes and publish a CPU/OS/compiler matrix; Q16.16 state quantization alone is not evidence of cross-architecture bit identity.

## 6. Where to beat the state of the art

Do not claim a universal solver-speed or accuracy win over SciML, PETSc, MOOSE, OpenFOAM, MuJoCo, or Rapier without matched benchmark evidence. These projects have mature numerical algorithms, specialist communities, and years of validation.

The realistic path to leadership is a differentiated combination:

1. **Credibility by construction:** executable validity contracts and evidence-gated promotion.
2. **Cross-domain conservation:** auditable exchange ledgers across coupled physical models.
3. **Cognitive experiment design:** Symthaea selects experiments that reduce uncertainty or distinguish competing models, with measured information gain per unit cost.
4. **Reproducible digital twins:** every prediction carries data provenance, calibration state, uncertainty, source digest, and model limitations.
5. **Accessible deployment:** a Rust-first interface that can run local engineering studies, then delegate large sparse solves to proven backends.

The first defensible "better than SOTA" claim should be narrow and decision-relevant, for example: fewer physical experiments needed to reach a target prediction uncertainty, while matching or improving held-out prediction error and keeping calibration valid. That is testable and potentially more useful than a generic FLOPS claim.

## 7. Implementation sequence

| Stage | Deliverable | Exit condition |
|---|---|---|
| 0 | Correct scientific claims; freeze benchmark schema and scenario manifest | all advertised guarantees match the evidence |
| 1 | Numerical-kernel benchmark harness and independent reference fixtures | Campaign A reproducible from clean checkout |
| 2 | Complete promotion of the second-moment-backed bivector inertia path across body state, kinetic energy, integration, contact and compound assembly; add 3D and N-D gyroscopic dynamics | Campaign B analytical/reference gates pass and all valid-case inertia failure counters remain zero |
| 3 | Research solver adapter and structured solver diagnostics | work-precision comparisons against diffsol/SciML |
| 4 | Typed units, model contracts, and conservation exchange ledger | invalid models rejected before execution |
| 5 | One finite-volume or finite-element continuum solver path | Campaign C passes manufactured-solution gates |
| 6 | Motor electrothermal twin + uncertainty-aware experiment loop | held-out measurements and calibrated intervals |

Avoid introducing an all-domain multiphysics framework before stages 1–4 are stable. Keep the interactive real-time path useful throughout.

## 8. Immediate next engineering tasks

1. Promote the initial harmonic-oscillator/free-fall regression tests into a machine-readable benchmark runner with timestep sweeps, error tables, and repeatable result receipts.
2. Run the connected asymmetric-inertia path through the full Rust test/Clippy/build matrix. Compare torque-free and forced asymmetric-top trajectories, angular impulse response, and contact effective mass against independent analytical and engine references.
3. Add a discrete contact/integration experiment: solve the contact impulse and state update under one shared effective-mass metric, then compare it with the existing sequential velocity solve at matched error and wall time.
4. Add adversarial rotating-contact and tunneling campaigns: high spin, thin geometry, simultaneous moving bodies, near-singular inertia, extreme mass ratios, and large world-coordinate offsets.
5. Reconcile the deterministic-net feature’s claims with the cross-architecture validation status. Quantization is not, by itself, a proof of bit-identical execution.
6. Standardize benchmark receipts and scenario hashes before adding further solver families.
7. Prototype a diffsol adapter behind a research-solver feature, compare it to the existing Symthaea ODE methods, and retain only what the work-precision results justify.

## References and comparison baselines

- [Symtropy Physics Research Validation Protocol](../../crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [SciML / OrdinaryDiffEq solver recommendations](https://docs.sciml.ai/DiffEqDocs/stable/solvers/ode_solve/index.html)
- [SciML solver benchmark summaries](https://docs.sciml.ai/SciMLBenchmarksOutput/dev/)
- [diffsol Rust ODE library](https://docs.rs/diffsol/latest/diffsol/)
- [faer Rust linear algebra](https://docs.rs/faer/latest/faer/)
- [PETSc nonlinear solvers](https://petsc.org/main/manualpages/SNES/)
- [MOOSE multiphysics framework](https://mooseframework.inl.gov/)
- [FEniCSx documentation](https://fenicsproject.org/documentation/)
- [MuJoCo computation and derivatives](https://mujoco.readthedocs.io/en/stable/computation.html)
- [Rapier physics engine](https://rapier.rs/docs/)
- [NIST IR 8620: Digital Twins Workshops Summary Report (2026)](https://doi.org/10.6028/NIST.IR.8620)
- [ASME V&V standards resource hub](https://www.asme.org/codes-standards/vvuq-standards)
