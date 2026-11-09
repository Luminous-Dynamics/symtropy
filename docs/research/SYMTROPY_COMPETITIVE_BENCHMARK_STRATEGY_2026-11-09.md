# Symtropy Competitive Benchmark and Research Strategy

**Date:** 2026-11-09  
**Status:** research strategy; proposed comparisons and milestones, not measured results.

## 1. Competitive thesis

Symtropy should not attempt to win every simulator category. The strongest strategy is to become a rigorous, open, composable physical-analysis substrate with unusually strong evidence lineage, exact configuration binding, independent verification and simulation-to-test correlation.

The competitive proposition is:

> A physical simulation result should be reproducible, independently checkable, uncertainty-aware, traceable to the exact engineering configuration, and explicit about the decisions it can and cannot support.

This is complementary to high-throughput robot-learning platforms and broad multiphysics suites. It is not a claim that Symtropy currently outperforms them.

## 2. Reference systems and fair comparisons

| System | Established strength / scope | What Symtropy should learn or compare | What not to claim |
|---|---|---|---|
| MuJoCo | Articulated rigid-body dynamics, contact-rich robotics, inverse dynamics, optimization and multiple constraint/integration choices | Matched joint/contact/dynamics cases; trajectory, constraint and energy error; solver stability and repeatability | That a small in-house microbenchmark establishes general superiority |
| NVIDIA Isaac Sim / Isaac Lab | Robot simulation and sensor/rendering workflows; GPU-parallel robot learning, domain randomization and multi-backend integration | Import/export fidelity, task parity, cross-backend comparison, reproducible task evaluation and evidence packaging | That Symtropy replaces its rendering, sensor, RL or GPU-cluster ecosystem |
| Newton | GPU-accelerated, extensible and differentiable physics; multiple solvers and robot/deformable workflows | Solver differential tests, gradient checks where relevant, contact robustness, portability and reproducibility | That CPU/Rust and GPU/Python numbers are directly comparable |
| Project Chrono | Broad multiphysics infrastructure including multibody dynamics, FEA, vehicle dynamics, deformables, FSI and granular systems | Interoperability and reference comparisons for selected supported physical regimes | That Symtropy is a general multiphysics or FEA/CFD replacement |

Official references:
- MuJoCo: https://mujoco.org/
- Isaac Lab: https://developer.nvidia.com/isaac/lab
- Isaac Sim: https://developer.nvidia.com/isaac/sim/
- Newton: https://github.com/newton-physics/newton
- Project Chrono: https://www.projectchrono.org/

A fair benchmark freezes the physical problem, geometry, units, material parameters, boundary conditions, initial conditions, actuator models, controller, solver tolerances, time step, hardware and measurement method. If a backend cannot represent the same physical model, report the mismatch rather than silently changing the problem.

## 3. Where the field is moving

### 3.1 Multi-backend and composable simulation

Isaac Lab's current architecture emphasizes common task/scene interfaces over a single physics backend. Newton offers a modular solver ecosystem and OpenUSD interoperability. This validates a federated approach: Symtropy should expose clean model/solver interfaces and exchange portable scene/configuration artifacts rather than attempting to own every layer.

### 3.2 Differentiable and optimization-aware physics

Newton's differentiable workflows and MuJoCo's model-based optimization demonstrate that simulation increasingly participates in design and control optimization, not just forward prediction.

Symtropy should investigate differentiable mechanics only where derivatives are meaningful and validated. Required tests include finite-difference/adjoint agreement, gradient stability near contact transitions, nonsmoothness disclosure, and optimization outcomes checked against forward simulation. Do not treat a gradient as trustworthy simply because an automatic differentiation system returns one.

### 3.3 Richer physical interactions

The frontier includes articulated rigid bodies, contacts, tendons, cables, cloth, deformables, particles and coupled solvers. This is not a reason to immediately implement all of them. It is a reason to define extension boundaries and choose domain-specific solvers based on validated use cases.

### 3.4 Credibility and VVUQ as an engineering discipline

The engineering evidence model should align with established VVUQ practice:
- ASME V&V 10-2019 (R2025) for computational solid mechanics;
- ASME VVUQ 10.2-2021 for uncertainty in solid mechanics V&V;
- ASME VVUQ 20.1-2024 for multivariate validation metrics;
- NASA-STD-7009B (2024) for model/simulation lifecycle credibility and responsible technical authority;
- FMI 3.0.2 as an interoperability reference for dynamic model exchange and co-simulation.

These standards and specifications are reference frameworks, not certifications obtained by implementing a schema.

References:
- ASME VVUQ overview: https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty
- ASME V&V 10: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019/pdf
- ASME VVUQ 10.2: https://www.asme.org/codes-standards/find-codes-standards/the-role-of-uncertainty-quantification-in-verification-and-validation-of-computational-solid-mechanics-models
- NASA-STD-7009B: https://standards.nasa.gov/node/263
- FMI: https://fmi-standard.org/

## 4. Benchmark architecture

Keep four benchmark layers separate and link them through immutable problem IDs.

### A. Solver microbenchmarks

Purpose: isolate algorithmic correctness and cost.

- GJK/EPA support and penetration cases;
- CCD impact cases;
- contact manifold generation;
- friction and restitution;
- joint/constraint solves;
- broadphase scaling;
- replay and snapshot cost.

Report latency distributions, allocations, memory, throughput and scaling. Do not publish only best-case nanoseconds.

### B. Canonical physical problems

Purpose: establish numerical correctness.

- free fall and ballistic motion;
- harmonic oscillator and damped oscillator;
- rigid-body translation/rotation;
- impact against analytical reference;
- pendulum and coupled pendula;
- constrained linkage and closed-chain kinematics;
- rolling/sliding friction with known limits;
- actuator torque-speed and thermal lumped-parameter cases.

For each problem publish analytical or independent reference, expected convergence behaviour, tolerances, units and negative controls.

### C. Cross-simulator differential corpus

Purpose: discover disagreement and semantics drift.

Run matched cases in Symtropy and one or more independent systems such as MuJoCo, Chrono or Newton. A disagreement is an investigation signal, not proof that either system is wrong. Compare only when contact, friction, solver, integrator, actuator and geometry semantics are equivalent or the difference is explicitly parameterized.

Report:
- trajectory/state error;
- contact onset and separation timing;
- impulse/force error;
- constraint residual and drift;
- energy/momentum balance;
- sensitivity to time step and solver tolerances;
- run-to-run and platform reproducibility.

### D. Physical correlation corpus

Purpose: establish applicability to hardware.

Start with the rotary-joint demonstrator:
- static torque/deflection;
- torque-speed response;
- backlash and hysteresis;
- current/torque relation;
- winding and case thermal response;
- vibration and repeatability;
- seal/environment exposure followed by inspection and post-exposure performance.

Freeze the test protocol and measurement uncertainty before evaluating predictions. Keep calibration data separate from held-out validation data. Report each channel and failure mode, not just one aggregate score.

## 5. Research paradigm shifts

### Shift 1 — From solver output to evidence-bearing prediction

Every run emits an immutable manifest, result envelope, uncertainty statement, applicability domain and evidence lineage. A result without these is exploratory output, not engineering evidence.

### Shift 2 — From one simulator as oracle to independent model triangulation

Use analytical references, independent solvers, reduced-order models and physical measurements. Preserve disagreement and explain it. Never let one implementation certify itself.

### Shift 3 — From nominal performance to robust operating envelopes

Evaluate the joint envelope over load, temperature, terrain, wear, actuator degradation, sensing faults and configuration uncertainty. Report feasible regions and boundary confidence, not one peak score.

### Shift 4 — From design-after-failure to fault-prepared embodiment

Continue the existing LOCO-MORPH-FAULT-CODESIGN work (#4360, PRs #4361/#4362). Co-design morphology, passive mechanics, actuation, sensing and control against nominal and degraded performance. Keep viability, safety and physical qualification as independent hard gates.

### Shift 5 — From static digital twin to continuously maintained model credibility

Field observations, repairs, component aging and environmental exposure update the model's applicability and uncertainty. A changed configuration or out-of-domain observation can invalidate a previously accepted claim.

### Shift 6 — From opaque toolchain to open engineering interoperability

Use versioned schemas and adapters for STEP/AP242, URDF/MJCF/USD, FMI/FMU and specialist solver outputs where their semantics fit. Preserve original artifacts, units, frames, source revisions and digests. Do not imply that file-format support equals semantic fidelity.

## 6. Proposed milestone targets

These are internal research targets. They are not universal certification thresholds or current results.

### M0 — Reproducible baseline
- 100% of benchmark cases have frozen inputs, units, frames and expected result definitions.
- 100% of published runs bind exact commit, toolchain, configuration and input digests.
- Every hard gate has at least one negative control.
- All missing or out-of-domain inputs fail closed or produce an explicit indeterminate state.

### M1 — Numerical credibility
- 100% agreement with hand-computable oracle cases within predeclared tolerances.
- Convergence curves for every method where refinement is meaningful.
- No silent constraint, energy or momentum drift outside case-specific acceptance bounds.
- Independent implementation/reference for every high-consequence kernel.

### M2 — Differential simulator qualification
- At least 30 canonical cases spanning primitives, contacts, joints, CCD and dynamics.
- Report matched-backend discrepancies and explain all material disagreements.
- No claim of general superiority; identify regimes where Symtropy is faster, more accurate, more reproducible or easier to audit with evidence.

### M3 — Engineering evidence vertical slice
- One product configuration, three requirements, one EBOM item, one geometry/material revision, one simulation, one test record and one baseline report.
- A relevant revision invalidates or explicitly reassesses downstream evidence.
- Every output links to its exact input and configuration.

### M4 — Rotary-joint physical correlation
- Frozen test protocol and instrumentation uncertainty budget.
- Blind prediction on held-out test conditions.
- Report torque, deflection, speed, current, temperature and vibration errors separately.
- Publish limits and failure cases; no IP rating or product qualification claim without applicable accredited testing.

## 7. First concrete implementation priorities

1. Maintain README as a product contract, not a list of aspirational capabilities.
2. Implement the run manifest and evidence envelope as versioned schemas plus a small Rust validator.
3. Build canonical analytical cases and negative controls before expanding the solver.
4. Add a cross-engine adapter/differential harness; start with a narrow matched MuJoCo corpus.
5. Connect the morphology Pareto work (#4361/#4362) to exact actuator/energy/viability evidence rather than inventing a new score.
6. Define the rotary-joint test and simulation contract before committing to detailed hardware design.
7. Add a minimal Luminous configuration reference and stale-evidence impact test.
8. Only then prioritize new physics capabilities based on benchmark gaps and demonstrated user need.

## 8. Decision rule

A new solver, feature or research programme is prioritized only when it answers at least one of these:

- Does it close a demonstrated physical-model gap?
- Does it improve numerical correctness or robustness?
- Does it reduce uncertainty or extend a validated applicability domain?
- Does it enable a benchmark or interoperability case that existing tools cannot address adequately?
- Does it reduce engineering lifecycle cost while preserving evidence quality?

A feature count, impressive demo or unqualified speed claim is not sufficient.
