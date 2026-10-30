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


## 3. Numerical architecture: build versus reuse

Do not write a broad sparse linear algebra package, automatic differentiation system, or stiff ODE solver from scratch before benchmarking existing Rust capabilities.

First evaluate:

- **diffsol:** Rust ODE solvers including variable-order BDF, SDIRK/ESDIRK, and explicit Runge–Kutta; supports mass matrices, adaptive tolerances, events, and forward/adjoint sensitivities.
- **faer:** Rust dense and sparse matrix factorizations, including sparse LLᵀ, LU, and QR paths.
- **SciML / OrdinaryDiffEq:** a solver-selection and performance reference across non-stiff, stiff, symplectic, exponential, and implicit methods.
- **PETSc:** reference for scalable Krylov linear solvers, nonlinear solvers, time stepping, and optimization.
- **MOOSE / FEniCSx / OpenFOAM:** independent reference ecosystems for coupled finite-element, finite-volume, and CFD problems.

An initial Rust-facing solver trait should return more than a state vector. It should expose a structured report containing accepted/rejected steps, local error estimates, nonlinear iterations, linear iterations, Jacobian evaluations, factorization counts, event hits, termination reason, and invariant diagnostics.

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

### Campaign B — Rigid and articulated dynamics (P0/P1)

1. Exact free fall and constant acceleration.
2. Harmonic oscillator and simple pendulum over a timestep sweep.
3. Torque-free spherical, symmetric, and asymmetric tops.
4. Gyroscopic precession and angular momentum conservation.
5. Rolling without slipping with correct inertia.
6. Two-body and multi-body impact, persistent contact, stacks, and restitution.
7. Fast rotating convex-body CCD and adversarial grazing contacts.
8. Articulated chains with forward and inverse dynamics.

Compare the 3D subset against MuJoCo and Rapier using identical geometry, mass properties, initial conditions, and tolerances. Use analytical solutions wherever possible; engine agreement alone is not ground truth.

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
| 2 | Full 3D inertia tensor plus dimension-correct bivector-space inertia, gyroscopic dynamics, and rotational impulse response | Campaign B analytical/reference gates pass |
| 3 | Research solver adapter and structured solver diagnostics | work-precision comparisons against diffsol/SciML |
| 4 | Typed units, model contracts, and conservation exchange ledger | invalid models rejected before execution |
| 5 | One finite-volume or finite-element continuum solver path | Campaign C passes manufactured-solution gates |
| 6 | Motor electrothermal twin + uncertainty-aware experiment loop | held-out measurements and calibrated intervals |

Avoid introducing an all-domain multiphysics framework before stages 1–4 are stable. Keep the interactive real-time path useful throughout.

## 8. Immediate next engineering tasks

1. Add a harmonic-oscillator stability/convergence campaign for the existing semi-implicit Euler integrator. Explicitly demonstrate that symplecticity does not mean unconditional stability.
2. Add a spherical-versus-asymmetric inertia regression suite. For D=3 use the full spatial inertia tensor; for D>3 use a physically constrained inertia operator on the bivector space. The asymmetric-top test should fail until the correct representation and gyroscopic terms are implemented.
3. Reconcile the deterministic-net feature’s claims with the cross-architecture validation status. Quantization is not, by itself, a proof of bit-identical execution.
4. Standardize benchmark receipts and scenario hashes before adding further solver families.
5. Prototype a diffsol adapter behind a research-solver feature, compare it to the existing Symthaea ODE methods, and retain only what the work-precision results justify.

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
