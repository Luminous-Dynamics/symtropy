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

**New MuJoCo comparator lanes:** (a) a 4.0.0 speculative-contact / CCD lane, with thin-wall translation and rotation cases; (b) a separate 3.17.0 CPU batch lane measuring throughput, scaling, memory and output equivalence against a serial reference at fixed model and hardware. Do not conflate these with the older 3.16 locomotion baseline or compare CPU batch results to GPU throughput as if they were the same execution regime.

### 2.1 Version-pinned comparison baselines (checked 2026-11-09)

| System | Current reference | What changed / why it matters |
|---|---|---|
| MuJoCo | **4.0.0**, released 2026-11-06; **3.17.0** batch API introduced 2026-10-22 | 4.0.0 adds speculative convex contacts intended to prevent tunnelling within a timestep, and stateful elasto-plasticity for joint/tendon springs and 2D/3D flexes. The 3.17.0 batch API runs many simulations of one model on a thread pool and documents bit-identical results to a plain loop across thread counts. The current *unreleased* upstream changelog also lists a fix for a 4.0.0 sparse Cholesky bug that can yield a wrong Hessian factor, incorrect accelerations and divergence with sparse constraint Jacobians. Benchmark CCD/contact, CPU batch scaling and sparse-constraint conditioning as separate profiles; preserve historical receipts and do not attribute an unreleased fix to 4.0.0. |
| Isaac Lab | **3.0.0 GA**, released 2026-10-20 | A single task API spans multiple physics/rendering backends, including kit-less Newton workflows. Pin Isaac Lab plus the exact physics backend, renderer, task and solver path. |
| Newton | **1.7.0**, released 2026-10-14 | Adds composable solver diagnostics, expanded rigid/deformable coupling, mimic-joint support across several solver paths, collision/scene optimizations and controller improvements. It also changes or clarifies imported damping, angular units, joint-coordinate references and MPM material scaling. Pin solver, importer flags, Warp/CUDA, precision, schedule and material migration settings; do not compare headline GPU throughput with CPU-only Rust measurements. |
| Project Chrono | **10.0.0**, released 2026-03-27 | Adds/updates FSI solver paths, peridynamics, checkpointing, implicit-integrator Jacobian strategies and sensor functionality. Use domain-specific modules only where the physical problem overlaps. |
| Rapier | **0.18.0** in Symtropy's current bridge manifest | Preserve 0.18 receipts as historical evidence. A newer Rapier baseline must be a separate, matched migration lane, not an in-place replacement. |

Sources: [MuJoCo 4.0.0 changelog](https://mujoco.readthedocs.io/en/4.0.0/changelog.html), [current upstream MuJoCo changelog (upcoming fix is unreleased)](https://mujoco.readthedocs.io/en/latest/changelog.html), [Isaac Lab 3.0.0 GA](https://github.com/isaac-sim/IsaacLab/releases/tag/v3.0.0), [Newton 1.7.0 release](https://github.com/newton-physics/newton/releases/tag/v1.7.0), [Project Chrono 10.0.0 release](https://github.com/projectchrono/chrono/releases/tag/10.0.0), and the repository's `crates/bridges/symtropy-rapier3d-bridge/Cargo.toml`.

**New benchmark admission requirement:** the same asset and timestep do not establish the same physical model. A pairwise mapping must explicitly compare contact law, friction cone, restitution, solver/regularization, integration, damping, inertia, joint constraints, CCD coverage, actuation and observable conventions. See [Cross-Simulator Semantic Equivalence V0.1](CROSS_SIMULATOR_SEMANTIC_EQUIVALENCE_V0_1.md) and [issue #4366](https://github.com/Luminous-Dynamics/symtropy). Unknown and known-non-equivalent cases are not eligible for matched solver-accuracy scores.

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
- NASA-STD-7009B (2024) for model/simulation lifecycle credibility and responsible technical authority, with the NASA-HDBK-7009B implementation handbook published 2026-02-03;
- FMI 3.0.2 as an interoperability reference for dynamic model exchange, co-simulation and scheduled execution. FMI standardizes interfaces; it does not by itself define or qualify the importer’s co-simulation master algorithm.

These standards and specifications are reference frameworks, not certifications obtained by implementing a schema.

References:
- ASME VVUQ overview: https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty
- ASME V&V 10: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019/pdf
- ASME VVUQ 10.2: https://www.asme.org/codes-standards/find-codes-standards/the-role-of-uncertainty-quantification-in-verification-and-validation-of-computational-solid-mechanics-models
- NASA-STD-7009B: https://standards.nasa.gov/node/263
- NASA-HDBK-7009B (published 2026-02-03): https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009
- FMI 3.0.2 specification: https://fmi-standard.org/docs/3.0.2/

## 3.4.1 Sparse constrained systems and numerical conditioning

A useful new challenge is to test the *linear-algebra path underneath constrained dynamics*, not just end-to-end trajectory error. This is timely because MuJoCo's current upstream changelog lists an **unreleased** fix for a sparse Cholesky factorization bug introduced in 4.0.0: rows with different descendants could be merged into one supernode, yielding a wrong Newton-solver Hessian factor for sparse constraint Jacobians, with slow convergence, incorrect accelerations or divergence. The 4.0.0 release remains the pinned released baseline; the upcoming fix must be tested as a separate release/commit once published.

This is not evidence that MuJoCo is generally unreliable. It is evidence that even mature engines need adversarial sparse-system tests and exact release identity. Symtropy should build a comparable numerical-robustness lane around its own constraint solve and any future sparse/matrix-free path.

Proposed cases:
- **Known-spectrum systems:** construct constraint matrices/Jacobians with prescribed singular values and condition numbers; solve against manufactured right-hand sides with known solutions.
- **Topology sweeps:** serial chains, branching trees, loops, sparse grids and deliberately redundant constraints; report graph ordering and sparsity pattern.
- **Mass-ratio and scale sweeps:** vary body mass/inertia ratios, geometry scale and constraint stiffness across declared ranges without mixing unit conversion with solver error.
- **Near-singular controls:** duplicate/dependent constraints, near-collinear axes, vanishing lever arms, near-zero effective mass and rank transitions.
- **Permutation controls:** reorder bodies, constraints and rows/columns while preserving the mathematical problem; the physical solution should remain invariant within numerical tolerance.
- **Precision controls:** compare the same mathematical case under supported f32/f64 paths, with explicit residual and backward-error gates.
- **Failure observability:** record non-finite state, factorization/iteration failure, residual growth, divergence, fallback/preconditioner path and early termination. Failure must not be hidden by reset/recovery.

Report at least:
- normalized constraint residual and backward error;
- forward error against the manufactured/high-precision reference;
- condition estimate and rank classification where available;
- position/velocity constraint drift over time;
- energy/work balance only where the system boundary makes it meaningful;
- iterations, factorization/rebuild counts, p50/p95/p99 step latency and memory;
- sensitivity to ordering, precision, scale, tolerance and solver iteration limit.

Do not require Symtropy to expose the same sparse factorization algorithm as MuJoCo. Compare the resulting physical/numerical contract and diagnostics, not private implementation details. The test owner is [#3749](https://github.com/Luminous-Dynamics/symtropy/issues/3749); results belong to the single [#460 qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460). This must not become a second scenario registry.

## 3.4.2 Application-level humanoid benchmark is a separate tier

NIST's Humanoid Robot Baseline Performance Benchmark is currently a **proposal being developed with industry and research partners**, not a finalized universal certification. Its stated task families include basic mobility and manipulation/dexterity, coordinated loco-manipulation, confined-space whole-body awareness/control, and minimal reasoning/task/scene understanding. These are valuable application-level outcomes, but they are not solver-correctness tests.

Symtropy should maintain two linked scorecards:
1. **Model/solver evidence:** semantics, oracle error, convergence, stability, uncertainty and physical correlation.
2. **Robot-task outcomes:** task completion, time, success/failure modes, payload/environment conditions, energy, recovery and operator intervention, bound to the exact robot/controller/configuration.

A robot can pass a task despite an inaccurate simulator through controller adaptation; a simulator can be numerically excellent while its task model omits perception, actuation or environment effects. Never substitute one scorecard for the other. Track NIST's proposal as an external application benchmark and version the test procedure when NIST publishes revisions.

Reference: [NIST Humanoid Robot Baseline Performance Benchmark proposal](https://www.nist.gov/el/intelligent-systems-division-73500/humanoid-robot-baseline-performance-benchmark).

## 3.5 How to outperform established simulators

The competitive unit should be a **credible engineering decision**, not a raw step, frame or simulation count. A system that runs 10 million steps quickly but cannot establish which physical problem those steps represent has not demonstrated a better engineering result.

Use a quality-constrained performance frontier:

- Fix the scenario, model, precision, hardware class and required accuracy/uncertainty envelope.
- Compare wall-clock time, CPU/GPU time, peak memory and energy only after semantic admission.
- Count a run as useful throughput only when it satisfies the predeclared oracle, stability and evidence gates.
- Plot accuracy, robustness, uncertainty and resource cost as separate Pareto axes. Do not collapse them into a weighted score that can hide a critical failure.

The first competitive scorecard should report:

| Axis | Measure | Hard interpretation |
|---|---|---|
| Semantic admission | Fraction of proposed backend/scenario pairs with complete, reviewed mappings; count of rejected mismatches | Zero known non-equivalent or unknown mappings may enter a matched score |
| Oracle accuracy | Per-observable error against analytical/manufactured references, in physical units | Each case uses its own predeclared tolerance; no cross-unit aggregate |
| Numerical robustness | Convergence, constraint residual, penetration, energy/momentum drift, failure and timeout rate | Any hard-gate failure remains visible; no averaging away |
| Physical prediction | Held-out prediction error and prediction-interval coverage by observable and operating cell | Calibration and validation records must be disjoint |
| Operating envelope | Fraction of required load/temperature/speed/environment/fault cells supported by evidence | Unassessed cells remain unqualified, not implicitly interpolated |
| Reproducibility | Exact replay rate under named environment; independent reproduction rate | Report bitwise and tolerance-based reproducibility separately |
| Performance | p50/p95/p99 latency, throughput at fixed quality, peak memory and energy per accepted case | CPU, GPU and mixed execution are separate lanes |
| Evidence portability | Ability to verify the manifest, mapping receipt, raw trace digest and report outside the producing engine | A report that requires trusting an opaque engine database is not portable |
| Decision efficiency | Time and compute required to resolve a named engineering question to a predeclared evidence level | This is the most meaningful product-level comparison; disclose human review and test-lab time too |

Proposed programme targets (internal research goals, not measured results or universal certification limits):

1. Freeze 30 canonical scenarios with units, frames, initial/boundary conditions, material and solver profiles, reference outputs, tolerances and at least one negative control per hard gate.
2. Require 100% exact input/configuration/source digests for published runs and 100% explicit status for each semantic dimension.
3. Require zero invalid or unknown semantic mappings to reach a matched score; mutate every dimension to prove that admission fails when a consequential property changes.
4. Use at least four refinements for cases where a convergence study is mathematically meaningful; report observed order rather than forcing a universal order on nonsmooth contact events.
5. Start with two engines for the first executable differential lane; extend selected scenarios to three or more only when each adapter's exact model path is independently inspected.
6. Reserve entire conditions/configurations for held-out physical validation. Do not randomly split time samples from one physical test into calibration and validation sets.
7. Publish failures, non-convergence, out-of-domain cases and inconclusive measurements alongside successful cases.

### Mutation-driven semantic testing

A semantic gate needs an adversarial test suite, not only well-formed examples. For each canonical scenario, produce controlled mutations such as:

- change unit scale or gravity while retaining the same numeric fields;
- permute a quaternion or transform convention;
- replace the full inertia tensor with an isotropic approximation;
- change friction-cone/contact formulation or restitution;
- disable/alter CCD for the exact shape pair under test;
- change integrator, timestep policy, damping, substeps or solver regularization;
- change actuator mode, transmission, saturation or delay;
- change importer flags or model-reference conventions while keeping the source asset bytes unchanged.

Expected outcome must be one of: explicit rejection, a reviewed bounded mapping with propagated uncertainty, or a separate non-matched profile. A digest change alone is not enough: the gate must explain which semantic dimension changed and why admission was blocked.

### Current release-specific trap: importer semantics

Newton 1.7.0's release notes document changes affecting imported damping interpretation, angular-unit conversion, joint-coordinate references and MPM material scaling. A backend version string and original USD/URDF digest therefore do not fully identify the interpreted model. The run identity must also bind importer version, importer options, compatibility flags, post-import model digest and the selected solver profile. The same principle applies to every backend: hash and inspect the **effective model after import**, not only the authored source file.

### How Symtropy can win

The attainable differentiation is not "faster than every incumbent". It is to show, on a narrow high-value engineering vertical, that Symtropy can produce a lower-cost, independently reproducible and better-bounded decision. The rotary-joint demonstrator is a credible first vertical because it joins mechanics, actuation losses, thermal response, measurement uncertainty, service history and an exact product configuration. The performance question becomes: how much time and test effort does it take to reach a decision whose limits and evidence can be independently reviewed?

### Shift 7 — From file adapters to a proof-carrying semantic compiler

A neutral API is not enough: two adapters can expose the same function signatures while implementing different physics. The longer-term architecture should treat backend import as a compiler/lowering step with explicit proof obligations.

1. Preserve the authored source artifact and its digest (MJCF, URDF, USD, CAD-derived model or native model).
2. Parse and normalize it into a versioned **Physical Problem IR** containing stable entity/topology identity, units, frames, geometry, mass properties, constraints, contact/friction laws, integration requirements, actuators, boundaries and requested observables.
3. Lower that IR into each backend's effective model. Return the exact imported-model digest, importer version/options, selected solver path, precision, relevant numerical settings and supported/approximated/unsupported semantics.
4. Emit a mapping receipt that binds each lowering to the exact source, IR and effective-model identities. Every nontrivial transformation carries a testable transformation, evidence reference and error/uncertainty bound.
5. Run round-trip and metamorphic controls on units, frames, inertia, topology and observable conventions before admitting paired traces.

The IR is an interlingua for declaring and comparing physical meaning, not a replacement for backend-native models or a promise that every backend can express every field. Unsupported or lossy lowering is explicit. It must not silently invent a default, replace a full inertia tensor, approximate a contact law or coerce an unavailable observable to zero.

Ownership stays compositional: #4366 owns pairwise semantic admission; #1472 declares the exact native capability/model path; #1172 owns scenario/replay identity; #460 owns the corpus and evidence record; #443 owns the Rapier execution adapter. Do not duplicate their registries or execution paths.

### Shift 8 — From arbitrary test volume to active experiment design

Choose the next simulation or physical experiment for its expected ability to reduce uncertainty or change an engineering decision, not because it is easy to generate. A candidate experiment should identify:

- the competing model hypotheses or uncertain parameters it can distinguish;
- the predicted observable differences and measurement resolution required;
- the expected information gain or decision-risk reduction, including test cost;
- the predeclared stopping/acceptance rule and the data reserved for blind validation.

Use multi-fidelity methods deliberately: cheap analytical/reduced models to screen candidates, expensive independent solvers for refinement, and physical experiments for the remaining consequential uncertainty. The acquisition policy itself must be versioned and audited. Do not use the held-out validation set to select experiments and then report it as independent validation.

A useful objective is **expected reduction in decision regret per unit test cost**, subject to hard safety and evidence gates. It is not an instruction to optimize a proxy metric or to trade a safety-critical failure against an efficiency gain.

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
