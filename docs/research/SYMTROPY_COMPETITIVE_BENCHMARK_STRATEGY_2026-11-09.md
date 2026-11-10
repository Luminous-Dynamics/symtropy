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

### 3.2.1 Differentiable contact frontier — separate the forward law from the gradient contract

The 2026 frontier is no longer one category called “differentiable physics”. It includes methods with materially different forward models and derivative meanings. Symtropy should benchmark them as separate lanes rather than combine their headline throughput or policy scores.

| Method | Forward model / derivative | What it demonstrates | Required caveat |
|---|---|---|---|
| **Ostrich** (2026 preprint) | Backward-Euler, hard NCP contact and Coulomb friction solved by non-smooth Newton; implicit-function derivative of the converged residual, reusing the forward Schur complement | Hard-contact optimization at larger steps and GPU-batched worlds; authors report h≈0.1 s, 50× timestep vs MJX on one pallet task, 211× warm-iteration speed vs MJX, and 29× checkpointed-MJX optimization throughput at 8,192 worlds on a 24 GB GPU | Author-reported task/hardware results, not independent reproduction. O(1) gradient memory **per timestep** does not mean O(1) total horizon storage; total stored states remain O(T) unless checkpointed. |
| **NablaEngine / ∇Mimic** (2026 preprint) | GPU fused MuJoCo contact dynamics plus hand-derived adjoint; optimality conditions differentiated with contact set, collision branch, active constraints and actuator clamps held fixed | Authors report median VJP errors vs float64 central differences of 3.6e-5 / 1.0e-5 / 1.9e-6 for position / velocity / action on their held-out state groups, and an 11.5× adjoint speedup vs the compared MuJoCo Warp implementation | These are conditional, within-regime derivatives. The paper identifies unbounded/ill-conditioned derivatives near vanishing-distance self-contact and nearly parallel capsules; its training default freezes some geometry derivatives and clips gradients. Measure exact mode and surrogate/clipped mode separately. |
| **DiffMJX + Contacts From Distance (CFD)** (ICLR 2026) | Adaptive integration for penalty-contact gradient accuracy; additional pre-contact signal only in the backward pass via straight-through estimation | Addresses vanishing gradients before contact while retaining the chosen forward penalty simulation | The backward CFD signal is a surrogate, not the exact derivative of the unchanged forward model. Report its bias and downstream optimization utility separately. |
| **CQDC-Warp** (2026 preprint) | GPU-batched quasi-dynamic convex contact planning with implicit contact-solve gradients and surrogate geometry gradients | Authors report 79.8% success on 17,408 Allegro 17-object in-hand-rotation trials and 2.06e5 / 2.25e5 forward/backward world-steps per second on an RTX 4090 with 2,048 worlds | Quasi-dynamic, contact-guided planning is not a full dynamic hard-contact solver. Its throughput and task success are not directly comparable with hard-dynamics solver accuracy. |

Primary sources:
- Ostrich, arXiv:2609.08800 — https://arxiv.org/abs/2609.08800
- NablaEngine / ∇Mimic, arXiv:2610.41654 — https://arxiv.org/abs/2610.41654
- DiffMJX / CFD, ICLR 2026 — https://proceedings.iclr.cc/paper_files/paper/2026/hash/44039e59aaf6a41b16f1fc5b27bcd409-Abstract-Conference.html
- CQDC-Warp, arXiv:2610.21349 — https://arxiv.org/abs/2610.21349

**The Symtropy opportunity:** differentiate on trustworthy derivative *semantics*, not simply the availability of an adjoint. Provide exact within-branch derivatives where regularity conditions hold; detect contact/active-set/collision-branch transitions; return a typed abstention or a set-/interval-valued directional sensitivity when a unique finite derivative is not justified. Preserve the unmodified forward physical model separately from any backward-only surrogate or gradient clipping policy.

#### Matched benchmark protocol

Use one frozen hard-contact scenario and one immutable forward solver/profile for each paired estimator comparison. Keep a distinct model lane when the forward law is quasi-dynamic, penalty/compliant, speculative or otherwise different.

1. **Smooth stratum:** compare analytic/implicit gradients with central finite-difference step ladders and high-precision references; report directional-derivative bias, observed finite-difference convergence, VJP error, and gradient variance across seeds.
2. **Fixed-mode contact stratum:** test separating, sticking, sliding, clamping and active joint-limit cases with the mode/active set explicitly frozen and recorded.
3. **Nonsmooth transition stratum:** contact creation/separation, impact, grazing, friction-cone edge, collision-feature switch, nearly parallel capsules, vanishing-distance self-contact and actuator clamp changes. Score detection and honest abstention/set-valued output; do not reward a method for returning a finite but unjustified single gradient.
4. **Surrogate-gradient stratum:** compare DiffMJX/CFD, contact-geometry surrogate, gradient clipping and smoothing methods by bias vs the hard-forward reference, pre-contact utility, gradient variance and final hard-forward task performance. Label every surrogate and smoothing parameter.
5. **Optimization stratum:** equalize task, initial states, compute/time budget, policy/controller capacity and evaluation seeds. Evaluate the final policy through a common independently checked hard-forward model and held-out physical trials where possible.

Required reporting:
- forward trajectory/model discrepancy and contact complementarity/friction residuals;
- derivative bias, variance, MSE and non-finite/overflow frequency;
- percentage of cases classified smooth, transition, unbounded or unsupported;
- false-confidence rate: a claimed single derivative where the reference/branch test shows no unique finite derivative;
- branch recall and transition detection;
- wall-clock, GPU/CPU time, peak memory and worlds per second;
- optimization success, sample efficiency and hard-forward held-out performance.

A derivative benchmark must never infer forward-model correctness from a useful optimizer update. Conversely, a physically accurate forward trajectory does not prove that its backward derivative is valid at mode transitions.

### 3.2.2 New differentiable-contact and mixed-coordinate releases (2026-11-10)

The latest public work strengthens the case for a taxonomy of gradients, not a single differentiable-simulator leaderboard.

| Method / release | Exact scope | Competitive evidence | Caveats and Symtropy benchmark response |
|---|---|---|---|
| **NablaEngine 0.2.0** (released 2026-11-07) | Fused CUDA simulation and analytic adjoints with MuJoCo contact dynamics; extends beyond the Unitree G1 to Go1, Go2, A1, H1, ANYmal C, Spot, Booster T1, ROBOTIS OP3 and Berkeley Humanoid; adds static boxes/heightfields, runtime terrain upload and an exact box–box adjoint | Release notes say the additional robot models were validated against MuJoCo Warp and float64 MuJoCo; the G1 0.1.0 results are bitwise unchanged | Pin release/commit, exact robot model, solver, terrain and GPU. Record its capacity/overflow policy: the 0.1.0 release notes specify 64 contacts and 192 constraint rows per world, with excess rows dropped but counted/flagged. An overflowed world is **not** a successful matched result. |
| **Bundled Contact Gradients (BCG)** (2026 preprint) | Contact-local randomized smoothing: when stiff contact is detected, evaluate a bundle of perturbed local rollouts and aggregate gradient signals | Authors report reduced gradient variance and zero-shot transfer of dynamic motions to a real Unitree G1 | This is a stochastic/local smoothed gradient estimator, not the exact point derivative of the original contact transition. Measure bias, variance, bundle size, perturbation scale, random-seed sensitivity, compute cost and real-hardware transfer. |
| **DiffGIPC** (2026 preprint) | GPU differentiable Incremental Potential Contact (IPC) with CCD/geometric barriers and coupled FEM + affine-body adjoints; each affine body has 12 motion DoFs independent of its surface vertex count; retains collision, friction and joint coupling | Authors report peak combined forward/backward speedups over DiffIPC of about 150× in full-space FEM and 4,133× with affine bodies | This is a rigid–deformable / nearly-rigid mixed-coordinate inverse-design lane, not a direct rigid-body simulator contest. Measure non-penetration, CCD validity, friction/contact residuals, adjoint error, shape/placement gradients, memory and model-form differences. |

Primary sources:
- [NablaEngine 0.2.0 release](https://github.com/sailab-ai/NablaEngine/releases/tag/v0.2.0) and [0.1.0 release / capacity notes](https://github.com/sailab-ai/NablaEngine/releases/tag/v0.1.0)
- [Bundled Contact Gradients, arXiv:2609.30951](https://arxiv.org/abs/2609.30951)
- [DiffGIPC, arXiv:2610.19091](https://arxiv.org/abs/2610.19091)

#### Additional benchmark strata

1. **Capacity and coverage:** deliberately exceed per-world contact/constraint limits. Verify overflow counters, warnings, omitted rows and final result status. A dropped contact/row must make the case incomplete or explicitly degraded, never silently marked as a pass.
2. **Cross-robot generalization:** run the same dimensionless task families on G1, Go2/ANYmal/Spot and other supported robots, but bind each robot's exact inertial model, contact geometry, actuator map, controller and sensor timing. Report per-robot results; do not pool different models into one unexplained score.
3. **Mixed rigid–deformable coupling:** use canonical IPC/CCD cases with a nearly-rigid affine body, a deformable mesh, frictional contact and articulated constraints. Compare only with engines that represent the same constitutive law, barrier, friction and temporal discretization.
4. **Gradient estimator ablation:** compare exact within-mode adjoints, IFT hard-contact gradients, finite-difference references, randomized bundles, clipped/frozen gradients and backward-only surrogates under identical forward models. Score both estimator quality and downstream optimization utility.
5. **Held-out physical transfer:** freeze the simulator, estimator and policy before the physical test. Report whether improvements survive outside the calibration trajectories, robot unit and contact/material conditions.

Do not let a high task score conceal a missing contact row, out-of-domain model, invalid derivative or a change in the forward physical law. This adds useful coverage to #3425 while #460 remains the only shared scenario/evidence corpus.

### 3.2.2 Rust-native differentiable physics comparator — phyz

A useful additional comparator is [muni-works/phyz](https://github.com/muni-works/phyz), a pure-Rust articulated and multi-physics stack with Featherstone ABA, contact/collision, GPU batch simulation and a rollout adjoint. It is relevant because it competes on language, integration cost and differentiable engineering workflows—not just on raw engine throughput.

Its public benchmark report is notably candid about where it wins and loses:
- On its reported reduced-coordinate pendulum models, phyz was only **1.08×** and **1.02×** Rapier; the correct reading is “competitive”, not “substantially faster”.
- Its 8-box contact row reported Rapier about **26× faster**, but the report explicitly says this is **not a controlled algorithm comparison**: phyz uses compliant penalty contact while Rapier uses an impulse solver.
- Its GPU batch path was reported slower than one CPU thread below about batch 128, then roughly 32–36× at 4,096–16,384 worlds. The report explicitly labels some GPU figures indicative because that suite was skipped in the generated run.
- Its rollout-adjoint report distinguishes exact inertia/contact-mesh parameter derivatives from one-step state/control Jacobian paths that use finite differences through ABA. It reports ~6.5× forward cost for 1-DOF/10 parameters and ~78.4× for 16 DOF/160 parameters; separate parameter-scaling tests show the parameter channel is nearly independent of parameter count at fixed DOF. Its report quotes adjoint-vs-FD agreement around 1e-10 on its covered cases.

**Version and evidence caveat:** the generated benchmark metadata shown in the repository records a 2026-07-25 run, phyz 0.1.0, Rapier 0.34.0, Rust 1.95.0 and an Apple M4 Max. The repository manifest has since advanced to phyz 0.4.1 and was updated on 2026-11-07. Treat the July measurements as historical methodology/evidence, not as a current phyz 0.4.1 vs Rapier 0.36.1 ranking. Reproduce against exact current commits, manifests, features, compiler, hardware and model digests before making a current comparison.

Sources:
- phyz repository and current workspace manifest: https://github.com/muni-works/phyz
- phyz benchmark protocol/results/caveats: https://github.com/muni-works/phyz/blob/main/BENCHMARKS.md

#### Proposed Symtropy ↔ phyz comparison

1. **Articulated forward dynamics:** start with serial chains, branching trees, off-diagonal inertia, free-flying bases and a 24-DOF hand. Freeze model data, coordinates/layout, mass/inertia, gravity, dt, integrator, damping, armature and output conventions. Compare one-step acceleration/impulse first; then convergence and long-horizon divergence. Include MuJoCo as an additional independent reference where a matching model can be imported.
2. **Adjoint scaling:** separately sweep DOF count and parameter count. Include many inertial parameters at fixed DOF (e.g. welded bodies) and increasing DOF at fixed parameter/DOF ratio. Report forward cost, backward cost, peak memory, derivative error and cost per parameter and per DOF. Do not infer asymptotic complexity from a two-point pendulum table.
3. **Contact differentiation:** freeze the forward contact law before comparing gradients. If penalty, impulse, hard NCP or compliant contact differ, label separate model classes. Compare branch-local exact derivatives, whole-rollout adjoints, finite differences and set-valued/abstaining derivatives only on a matched forward map.
4. **Contact performance:** measure narrow phase, contact solve and total step separately. Do not cite the published 26× phyz/Rapier row as an algorithmic winner because the two contact methods differ.
5. **GPU crossover:** record batch 1, 16, 64, 128, 256, 1,024, 4,096 and 16,384, with p50/p95/p99 latency, throughput, memory and energy. Report CPU single-world latency separately from batched world throughput.
6. **Numerical quality:** use the same analytic oracle and report constraint/contact residual, energy/work balance, observed order under dt refinement, stability/failure, and deterministic replay. Do not equate small trajectory error on a short horizon with long-horizon chaos-free equivalence.

The Symtropy differentiator should be **composable assurance**: exact model identity and semantic admission, solver diagnostics, uncertainty and branch semantics, and reproducible evidence across native and external engines. phyz is a useful peer and possible component/adapter target; its public benchmark is not evidence that either project universally dominates the other.

### 3.3 Richer physical interactions

The frontier includes articulated rigid bodies, contacts, tendons, cables, cloth, deformables, particles and coupled solvers. This is not a reason to immediately implement all of them. It is a reason to define extension boundaries and choose domain-specific solvers based on validated use cases.

### 3.4 Credibility and VVUQ as an engineering discipline

The engineering evidence model should align with established VVUQ and metrology practice:
- ASME V&V 10-2019 (R2025) for computational solid mechanics;
- ASME VVUQ 10.2-2021 for uncertainty in solid mechanics V&V;
- ASME VVUQ 20.1-2024 for multivariate validation metrics;
- NASA-STD-7009B (2024) for model/simulation lifecycle credibility and responsible technical authority, with the NASA-HDBK-7009B implementation handbook published 2026-02-03;
- FMI 3.0.2 as an interoperability reference for dynamic model exchange, co-simulation and scheduled execution. FMI standardizes interfaces; it does not by itself define or qualify the importer’s co-simulation master algorithm;
- NIST's 2026 report on harmonizing ASME computational-model UQ with JCGM metrology practice. Its risk-based framework scales VVUQ effort with model influence and decision consequence, and explicitly compares variance propagation with distribution propagation.

This suggests a practical Symtropy principle: evidence effort should be proportionate to model influence and decision consequence, while hard safety/physics gates remain non-negotiable. A low-consequence exploratory model can carry a lighter evidence package; a joint torque/thermal result that affects product release requires a deeper uncertainty budget, calibration traceability and independent review. These standards and reports are reference frameworks, not certifications obtained by implementing a schema.

References:
- ASME VVUQ overview: https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty
- ASME V&V 10: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019/pdf
- ASME VVUQ 10.2: https://www.asme.org/codes-standards/find-codes-standards/the-role-of-uncertainty-quantification-in-verification-and-validation-of-computational-solid-mechanics-models
- NASA-STD-7009B: https://standards.nasa.gov/node/263
- NASA-HDBK-7009B (published 2026-02-03): https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009
- FMI 3.0.2 specification: https://fmi-standard.org/docs/3.0.2/
- NIST report, *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices* (published 2026-10-05): https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing

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
