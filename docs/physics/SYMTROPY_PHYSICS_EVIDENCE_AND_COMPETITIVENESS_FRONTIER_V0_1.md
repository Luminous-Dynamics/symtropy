# Symtropy Physics — Evidence and Competitiveness Frontier

**Status:** research and benchmark design, 12 November 2026  
**Canonical qualification owner:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)  
**Composes with:** [contact qualification #3775](https://github.com/Luminous-Dynamics/symtropy/issues/3775), [metamorphic verification #2409](https://github.com/Luminous-Dynamics/symtropy/issues/2409), [energy ports #3615](https://github.com/Luminous-Dynamics/symtropy/issues/3615), [rotational inertia #3946](https://github.com/Luminous-Dynamics/symtropy/issues/3946)

This document does **not** create a second benchmark framework. It maps the next research campaigns onto the existing canonical corpus and makes the external comparison rules and claim boundaries explicit.

## 1. Research thesis

The differentiator should not be another general-purpose rigid-body simulator, nor a collection of mechanics features that have not yet been qualified together.

The target is an **evidence-qualified physical prediction**:

```
PhysicalPrediction
├── exact model / geometry / mass-property identities
├── reference frames, COM and coordinate conventions
├── solver, integrator, timestep and contact configuration
├── predicted state and trajectory
├── independently reconstructed physical observables
├── conservation, work and energy-transfer ledger
├── numerical / contact / constraint residuals
├── uncertainty and applicability envelope
├── derivative and active-set validity
├── cross-engine or analytical reference comparison
├── reproducibility receipt
└── ADMIT / ESCALATE / ABSTAIN / REJECT
```

A trajectory is not qualified merely because the engine produced one, a stack remained upright, or a render looked plausible. The result must be reproducible and independently challengeable. Unsupported semantics and unknown outcomes must remain explicit.

This is the product-level paradigm shift: **from simulation output to qualified engineering evidence**.

## 2. Five connected paradigm shifts

### 2.1 One physical system, independently reconstructed

The inertia used to predict angular acceleration must also govern angular impulse response, contact effective mass, hinge reaction, kinetic energy and diagnostics. The same physical configuration must be independently reconstructed at the contact boundary.

For contact point (i), define the Delassus operator from the generalized inverse mass/inertia operator:

[
W_{ij}=J_i M^{-1}J_j^T
]

The contact solver's predicted response must agree with the realized velocity response to the applied impulse. Do not let a reference implementation reuse the candidate's own response routine as its oracle.

The immediate bridge is the composition of:
- [#4811 production contact-response oracle](https://github.com/Luminous-Dynamics/symtropy/pull/4811), currently limited to a homogeneous cube where the legacy mean-inertia approximation is exact;
- [#4698 checked anisotropic references](https://github.com/Luminous-Dynamics/symtropy/pull/4698), including a two-contact anisotropic Delassus reference;
- [#4676 COM migration correction](https://github.com/Luminous-Dynamics/symtropy/pull/4676);
- the future independently qualified asymmetric-body contact trajectory.

### 2.2 Covariance is part of correctness, not an optional robustness test

Keep three distinct metamorphic relations:

1. **Rigid-frame covariance:** rotate and translate the complete scene, including points, forces, normals, moments, frames, constraints and outputs. Invariant scalars must remain invariant; vectors and tensors must transform according to their types.
2. **Dimensional / unit covariance:** represent the same physical system using different mass, length and time units, then compare canonicalized observables.
3. **Generalized-coordinate covariance:** for (x=S y), transform the state, Jacobian and dual forces consistently; do not confuse this with a physical scene rescaling.

For positive representation scales (s_M,s_L,s_T), the dimensional relation includes:

| Quantity | Transformed numerical value |
|---|---|
| Mass | (m'=s_Mm) |
| Position / gap | (x'=s_Lx) |
| Time step | (dt'=s_Tdt) |
| Velocity | (v'=(s_L/s_T)v) |
| Acceleration / gravity | (a'=(s_L/s_T^2)a) |
| Force | (F'=(s_Ms_L/s_T^2)F) |
| Linear impulse | (J'=(s_Ms_L/s_T)J) |
| Rotational inertia | (I'=(s_Ms_L^2)I) |
| Torque | (	au'=(s_Ms_L^2/s_T^2)	au) |
| Energy | (E'=(s_Ms_L^2/s_T^2)E) |
| Power | (P'=(s_Ms_L^2/s_T^3)P) |
| Stiffness | (k'=(s_M/s_T^2)k) |
| Viscous damping coefficient | (c'=(s_M/s_T)c) |
| Compliance | (alpha'=(s_T^2/s_M)alpha) |

Dimensionless friction and restitution coefficients remain invariant. Any other dimensional parameter in a solver profile must be included in the transform or explicitly declared as a fixed physical model parameter.

Do not accept a test that rescales only the mass matrix while leaving the geometry, contact Jacobian, loads or solver parameters unchanged. Generate metamorphic cases near every active-set and cutoff boundary, not only well-conditioned mid-range cases.

### 2.3 Escalation and abstention become deterministic solver behavior

A fast method should handle its declared ordinary regime. When a precondition fails or an independently measured residual exceeds a frozen bound, the system should do one of three things:

- escalate to a declared higher-fidelity solver or reference;
- return a qualified result with an explicit approximation envelope;
- abstain because the physics, condition number or evidence is unsupported.

Record the trigger, fallback path, extra compute, and final admission decision. Do not replace a hard failure with an unreported clamp, confidence scalar or more iterations.

### 2.4 Differentiable physics must report which branch it differentiates

Contact creation, separation, friction transitions, grazing events, active-set changes and constraint-rank changes make ordinary smooth gradients incomplete or misleading.

The result must identify the active set and branch, distinguish derivatives valid within the current branch from event sensitivities, and abstain when the required derivative is undefined or ill-conditioned. This is especially important for trajectory optimization, system identification and differentiable robotics.

### 2.5 Conservation and energy are ledgers, not decorative scalars

Record separately:
- external force and actuator work;
- normal-contact impulse work and restitution energy change;
- tangential friction work and dissipation;
- compliant energy stored/released;
- Baumgarte or stabilization work;
- damping losses;
- numerical energy residual;
- battery/source-terminal energy when hardware is instrumented.

For a tangential impulse (J_t), the mechanical work at the contact is:

[
W_t=J_tcdotrac{v_t^-+v_t^+}{2}
]

Only a finite negative value is positive frictional dissipation. An impulse magnitude multiplied by an arbitrary constant is not joules. Mechanical work is also not battery energy or regenerated energy.

## 3. External analytical accuracy baseline: PBD-R seven-case suite

Abderezaei et al., *Physically Accurate Rigid-Body Dynamics in Particle-Based Simulation*, arXiv:2603.14634 (15 March 2026), introduce a solver-agnostic analytical benchmark and compare PBD-R, standard PBD and MuJoCo. The paper reports that PBD-R improves substantially over standard PBD and achieves competitive accuracy with MuJoCo at lower computation cost. Treat those as the authors' results, not as a prediction about Symtropy.

The seven analytical scenarios are a strong external baseline for PHYS-VALID-06:

| Case | Scenario | Main observable |
|---|---|---|
| 1 | Box pushed at its centre of mass | Translation under constant force and ground friction |
| 2 | Box with applied centre-of-mass torque | Angular acceleration for an isotropic inertia case |
| 3 | Box sliding on an incline | Gravity, normal force and friction |
| 4 | Asymmetric mesh pushed at its centre of mass | Force-driven translation with nontrivial geometry |
| 5 | Asymmetric mesh under torque | Anisotropic rotational response |
| 6 | Asymmetric mesh on an incline | Coupled geometry, gravity and friction |
| 7 | Rod pushing a box | Coupled translation–rotation at contact |

The first six cases have closed-form reference observables. The rod/box case uses a quasi-static pushing reference and must retain its stated assumption envelope. The original paper's geometry representations differ across engines (sphere packing versus mesh/primitive representation), so an adapted benchmark must document the actual geometry and contact assumptions rather than silently treating them as identical.

**Execution rule:** map the paper's initial conditions, forces, friction, inertia, contact point and output observables into the #460 manifest; reconstruct each oracle independently; retain raw trajectories; report errors and timestep refinement. Do not vendor upstream assets until their licence and redistribution permissions are established.

These seven cases should be accompanied by the existing Symtropy-specific fixtures: symmetric and asymmetric tops, intermediate-axis instability, COM recentering, multi-contact Delassus coupling, friction-cone feasibility and mass/length/time covariance.

Reference: https://arxiv.org/abs/2603.14634

## 4. Industry comparison — pinned, workload-specific, and fair

The comparison must separate physical accuracy from throughput. The latest upstream releases change which capabilities should be tested.

| Reference | Current pin / relevant capability | Appropriate Symtropy comparison |
|---|---|---|
| MuJoCo | **4.0.1**, released 11 November 2026. 4.0.1 fixes a sparse-Cholesky/Newton Hessian-factor defect from 4.0.0 and single-shot multicontact cylinder-edge cases. 4.0.0 added speculative contacts and elasto-plasticity. | Matched 3D rigid-body contact, restitution, friction, multicontact, stability and numerical residuals. Pin dense/sparse solver profiles separately. |
| Newton | **v1.8.0 stable**, published 11 November 2026. **SolverSteel remains experimental**: matrix-free projected Gauss–Seidel for articulated rigid bodies, with patch-based Coulomb friction and run-to-run determinism support. | Heterogeneous articulated robotics, contact-rich manipulation, memory and throughput, plus independent analytical accuracy. Treat experimental Steel separately from stable profiles. |
| Rapier | Q3 2026 work includes intra-island parallelism, persistent graph coloring and parallel broadphase. Its published performance graphs used early-August 2026 engine versions. | Rust-native CPU scaling, intra-island scaling, broadphase and contact throughput. Rebuild matched tests at pinned current commits rather than copying old graph rankings. |
| Pinocchio | 4.1.0, July 2026. | Articulated rigid-body algorithms, spatial inertia, derivatives and model-based control. Not a general contact-engine leaderboard. |
| Drake | 1.58.0. | Multibody dynamics and spatial/frame semantics, especially articulated mechanisms and independent model checks. |
| Project Chrono | 10.0.0, March 2026. | Domain-specific multiphysics (FSI, peridynamics, granular and related coupled systems) where the equations and constitutive models match. |

Newton/Steel's public materials report 3.8× geometric-mean throughput over PhysX across 20 robot-learning benchmarks and up to 13× on heterogeneous batches. Those are vendor-reported throughput results, not evidence of 3.8× or 13× better physical accuracy. Newton v1.8.0's stable release notes describe Steel as experimental.

Rapier's Q3 report explicitly warns that its published performance graphs reflect early-August engine versions. Do not treat them as a current, matched-version ranking.

### Required comparison protocol

Every external result must bind:
- exact source release/tag and commit;
- compiler, runtime and dependency lock;
- CPU/GPU, driver, device memory and power profile;
- geometry, mass properties, materials and collision representation;
- frames, units, initial state and scene digest;
- solver/contact formulation, timestep, substeps, iteration budget and warm-start;
- tolerances, diagnostics and all supported model features;
- raw trajectories, failures, uncertainty and reproducibility receipt.

An engine comparison is scoreable only when the semantics are equivalent or the approximation is explicitly bounded. Otherwise classify the result as **non-equivalent**, **unsupported** or **unassessed**, not as a win or loss.

Primary release references:
- MuJoCo: https://mujoco.readthedocs.io/en/latest/changelog.html
- Newton v1.8.0: https://github.com/newton-physics/newton/releases/tag/v1.8.0
- Steel throughput report: https://github.com/skild-ai/steel
- Rapier Q3 2026: https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- Pinocchio: https://github.com/stack-of-tasks/pinocchio/releases
- Drake: https://github.com/RobotLocomotion/drake/releases
- Chrono: https://projectchrono.org/news/

## 5. Benchmark campaigns and proposed targets

The targets below are research objectives, **not achieved Symtropy measurements or universal standards**. Freeze thresholds and holdout cases before confirmatory runs.

| Campaign | Independent observables | Proposed qualification target |
|---|---|---|
| Geometric/frame mechanics | SO(3)/SE(3) reconstruction, adjoint transport, power invariance | (10^{-12}) normalized algebraic error away from declared singularities |
| Dimensional/canonical covariance | Canonicalized trajectories, impulses, energy and contact active sets | (10^{-10}) normalized agreement in well-conditioned analytical cases; explicit UNRESOLVED at overflow/underflow or unsupported conditioning |
| Rotational dynamics | Angular momentum, kinetic energy, precession, torque response | Correct analytical behavior and convergence order over 3+ timestep resolutions |
| Contact | Independent Delassus matrix, primal/dual feasibility, complementarity, friction cone and realized impulse response | Zero false admissions in frozen negative controls; row-local residuals within scenario-declared bounds |
| Energy/work | Kinetic energy change, impulse work, restitution, friction and stabilization transfers | Closure within the declared numerical and measurement uncertainty; no dimensional proxy counted as joules |
| Derivatives | Branch-local finite differences, event time, active-set changes, gradient bias | Error and abstention measured separately by branch and event-boundary distance |
| Performance | Throughput, qualified predictions per compute, memory, latency and energy | At least 20% better throughput at equivalent admitted physical accuracy, or 20% lower physical error at equal compute, in one preregistered workload family |

For integer or floating residuals, retain raw dimensional values as diagnostics alongside normalized values. A global infinity norm must not hide one locally invalid contact behind a different contact's large impulse. Non-finite inputs and intermediate overflow must fail closed or return UNRESOLVED.

### Separate performance scorecards

1. **Small constrained islands:** 1–10 contacts, analytical KKT/LCP references, CPU.
2. **Dense contact and stacks:** contact count, churn, solver iterations, penetration and energy.
3. **Articulated robotics:** fixed and heterogeneous morphologies, joint count, loops and contact topology.
4. **Many homogeneous worlds:** worlds/s and simulated-seconds/s, CPU and GPU separately.
5. **Heterogeneous batches:** different morphologies and task lengths; include memory and load imbalance.
6. **Large rigid-body scenes:** 10K–500K bodies, broadphase stress and contact density.
7. **Multiphysics engineering:** only matched constitutive equations, boundary conditions and coupling semantics.

Report p50, p95, p99 and worst-case time, memory, energy, iterations, numerical residuals, failures and physical error. Do not compress these into one weighted score that can hide a safety or fidelity regression.

A useful secondary measure is:

[
\text{qualified predictions per compute}
=\frac{\text{independently admitted predictions}}
{\text{total compute consumed}}
]

This rewards an engine that knows when its answer is supported, not simply one that emits the largest number of trajectories.

## 6. First end-to-end milestone

The next flagship result should be the **first independently qualified anisotropic rigid-body contact trajectory**. One reproducible experiment should contain:

1. Asymmetric body with independently validated mass properties and products of inertia.
2. A frame-consistent COM and child-collider geometry.
3. Rotor-consistent angular point velocity and off-centre impulse.
4. A contact impulse that creates anisotropic angular response.
5. Contact effective mass reconstructed from the same declared inertia operator.
6. Independent linear and angular momentum transfer checks.
7. Friction feasibility and signed contact work.
8. Energy/work ledger with numerical residual.
9. At least three timestep resolutions and a declared convergence assessment.
10. Exact model, solver, source and evidence receipt.

Do not promote the production migration merely because the reference modules pass. The production path must pass the same independent observables at the exact integrated head.

## 7. Current implementation and claim boundaries

The scale-admission findings have been split so that each change has a narrow invariant and regression:

- [#4822](https://github.com/Luminous-Dynamics/symtropy/pull/4822): finite large-mass bodies are no longer classified as immovable by an absolute inverse-mass cutoff. Exact-head three-platform CI passed run `43184942843`.
- [#4827](https://github.com/Luminous-Dynamics/symtropy/pull/4827): finite sub-(10^{-15}\,\mathrm{N\,s}) normal impulses and their friction share are no longer dropped by an absolute impulse gate. CI pending.
- [#4835](https://github.com/Luminous-Dynamics/symtropy/pull/4835): removes the fixed tangential-speed deadband and tangential effective-inverse-mass floor. CI pending.
- [#4837](https://github.com/Luminous-Dynamics/symtropy/pull/4837): replaces impulse-times-constant friction "dissipation" with midpoint contact impulse work. CI pending.
- [#4676](https://github.com/Luminous-Dynamics/symtropy/pull/4676): COM geometry/reference-point correction, refreshed against current `main`; exact-head CI pending.
- [#4698](https://github.com/Luminous-Dynamics/symtropy/pull/4698): checked anisotropic reference work, refreshed against current `main`; strict-Clippy fixes applied, fresh CI pending.
- [#4811](https://github.com/Luminous-Dynamics/symtropy/pull/4811): production contact-response oracle, now refreshed on the current angular-convention branch; fresh exact-head CI pending.

These items are not all validated or merged. Queued is not pass. A software CI pass is not physical validation. The production inertia migration remains separately gated.

## 8. Admission and reporting rules

Every benchmark result must be one of:

- **PASS:** all declared preconditions and independent observables meet frozen thresholds.
- **FAIL:** an observable or negative control violates its declared threshold.
- **UNRESOLVED:** missing reference, invalid/non-finite intermediate, unsupported semantics, insufficient conditioning evidence or incomplete run.
- **MEASUREMENT-ONLY:** timing or resource measurement without a qualified accuracy claim.
- **PHYSICAL-VALIDATION:** reserved for cases with independently measured physical evidence and declared measurement uncertainty.

Never turn SKIPPED, QUEUED, CANCELLED, stale-head or non-equivalent results into PASS. Preserve escaped mutations and false admissions in the corpus. Holdout tuning invalidates that holdout as confirmatory evidence.

---

**Strategic objective:** Symtropy should compete on *physical credibility per unit compute* and on knowing when it cannot yet support a prediction. The moat is not a claimed novelty in Delassus operators, semi-implicit integration or GPU parallelism; it is the combination of geometric generality, independent physical admission, uncertainty, deterministic escalation and reproducible engineering evidence.
