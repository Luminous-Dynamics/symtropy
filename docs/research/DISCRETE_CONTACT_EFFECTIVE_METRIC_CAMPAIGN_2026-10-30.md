# Research campaign: a unified discrete contact/effective-mass solve

**Status:** research specification and benchmark contract; no solver superiority claim.
**Date:** 2026-10-30
**Related work:** [Physics numerical credibility PR #2927](https://github.com/Luminous-Dynamics/symtropy/pull/2927).

## 1. Research hypothesis

The next meaningful step beyond a more accurate rotational inertia tensor is to remove the split between:

- free rigid-body integration,
- normal contact impulses,
- tangential friction impulses,
- joint constraints,
- damping/stiffness response, and
- post-step energy accounting.

These subsystems currently use related but separately assembled approximations. The hypothesis is that an island-level **discrete effective-mass metric** can make them share one physically consistent step operator, reducing disagreement between the dynamics update and the constraint solver.

This is a hypothesis to test. MuJoCo already has a discrete integrator that combines the constraint solve and implicit velocity update in an effective metric. Symtropy must demonstrate a concrete advantage—dimension-generic mechanics, reproducible numerical evidence, better error/cost, or more transparent failure handling—not claim the idea as novel.

## 2. Mathematical contract

For one dynamic body in spatial dimension D, use generalized velocity

[
u = (v, B) in mathbb R^D oplus Lambda^2(mathbb R^D),
qquad
n_u = D + rac{D(D-1)}{2}.
]

Here B is Symtropy's stored rotor bivector. Physical point velocity at world offset r is

[
v_p = v - B r.
]

The generalized inverse-mass action must use the same translational mass and world-rotated bivector inertia operator used by free integration. Bodies with no validated operator may use an explicitly identified legacy mode, but benchmark receipts must never mix the two silently.

For an island of bodies, stack all generalized velocities into u, define the generalized mass operator M, and form the contact/joint Jacobian J from the same kinematic convention. Let v_free be the velocity predicted by forces and the selected smooth-force integration policy.

A research discrete step can be written as

[
u^+ = argmin_u ; rac12 (u-u_{free})^T M_{eff}(u-u_{free})
]

subject to unilateral contact, joint and other admissible constraints, where

[
M_{eff}=M+hD+h^2K
]

is a candidate positive-definite effective metric incorporating implicit damping D and stiffness K where those terms are valid. For a frictionless unilateral-contact prototype, the equivalent reduced system is

[
Wlambda + b ge 0,quad lambda ge 0,quad
lambda^T(Wlambda+b)=0,
qquad W=J M_{eff}^{-1}J^T.
]

Friction requires a maximum-dissipation formulation or a documented cone approximation. The prototype must not pretend a normal-only LCP is a complete Coulomb-friction solver.

The critical consistency requirement is that the same M_eff governs both constraint response and the committed velocity update. Do not solve constraints with one effective inertia and then advance the state with a materially different operator without measuring and reporting that splitting error.

## 2.1 Reference implementation status

The first executable reference layer is now being developed separately in [PR #2998 — small-island Delassus and LCP reference oracle](https://github.com/Luminous-Dynamics/symtropy/pull/2998). It assembles the frictionless contact Delassus matrix from the same translational and bivector-inertia response used by Symtropy, then exhaustively enumerates active sets for up to eight rows. It deliberately remains a validation oracle, does not assemble the bias vector, and does not replace the runtime TGS/sequential-impulse path. Its initial regressions include asymmetric off-diagonal coupling, redundant rows without regularization, and a production single-contact impulse comparison. Exact-head CI and randomized oracle-versus-runtime campaigns remain outstanding. The same PR now includes a Criterion microbenchmark for assembly and oracle solve at 1/2/4/8 rows; it is a reference-layer cost curve, not an engine-level benchmark.

## 3. Architecture experiment

Keep the existing sequential impulse/TGS path as the baseline. Add a separately feature-gated research path in stages:

1. **Contact row contract.** Define a typed row with body handles, world contact offsets, unit normal/tangent direction, right-hand side/bias, unilateral/bilateral kind, and provenance. Validate finite values and normalization.
2. **Generalized response.** Implement a single operation that maps an impulse on a row to the resulting linear and stored-bivector velocity changes using the same inverse-mass and `BivectorInertia` operators as free integration.
3. **Delassus assembly.** Build W_ij by applying the generalized response for row j and evaluating row i's relative point velocity. Verify symmetry and positive semidefiniteness for valid unconstrained mass operators.
4. **Reference solve.** Start with tiny islands (1–8 rows) and a dense reference solver. Compare to a direct KKT/LCP oracle or exhaustive active-set enumeration for small cases. No custom sparse solver until the dense oracle validates the formulation.
5. **Friction.** Add a maximum-dissipation or explicit cone approximation only after normal-contact results are stable. Measure cone residual and frictional work.
6. **Stiff terms.** Add damping/stiffness contributions to M_eff only when a positive-definiteness check and energy/work accounting validate them.
7. **Promotion decision.** Keep this path behind a research feature until matched-work benchmarks justify any runtime default change.

## 4. P0 benchmark contract

Inputs, reference outputs, tolerances, and promotion limits must be committed before performance runs.

| Campaign | Scenario | Required measurements | Initial promotion gate |
|---|---|---|---|
| Algebra | 1–8 row islands, random well-conditioned SPD body operators | W symmetry, eigenvalue floor, solve residual, complementarity residual | symmetry <= 1e-12 relative; residuals <= 1e-10 |
| Analytic contact | sphere-plane, asymmetric ellipsoid-plane, two-contact block | impulse, post-impact velocity, restitution, angular response | effective-mass/impulse identity <= 1e-10 |
| Stacking | 1/10/100 box stacks, equal and extreme mass ratios | penetration max/p95, jitter RMS, energy drift, collapse rate | zero unexplained energy creation; no stack collapse in canonical set |
| Friction | sliding block, oblique impact, multi-point patch | friction cone violation, slip speed, dissipated work | non-positive frictional work within numerical tolerance |
| Gyroscopic | torque-free asymmetric top, forced top, rolling ellipsoid | world angular momentum, kinetic energy, orientation error | error converges under dt refinement; report long-horizon drift |
| CCD | high-spin thin rod, fast projectile, two moving bodies | tunneling rate, TOI error, extra substeps, wall time | zero misses in the frozen adversarial suite |
| Performance | small, medium and large islands | p50/p95 step time, allocations, matrix assembly/solve cost | compare only at matched physical error and scene settings |

Run a step-size and island-size sweep. Publish work-precision curves, not a single frames-per-second result. Include failed and ill-conditioned cases in the result files.

## 5. Industry comparison protocol

Pin engine version, scene assets, timestep, collision margins, solver settings, friction/restitution, CCD flags, thread count, machine/OS/compiler, and feature flags in every receipt. Compare matched physical error and equivalent workload—not default settings or raw FPS.

### Current baselines as of 2026-10-30

- **MuJoCo 3.17.0 (2026-10-22):** the `discrete` integrator (introduced in 3.13) merges constraints and implicit velocity updates in the shared effective metric \\(\\widehat M=M+hD+h^2K\\). In 3.17 it is also used for flex bending; the 3.14 IPC experiment is frictionless and limited to 2D flexes, with contact multipliers not fully represented by saved state. MuJoCo 3.17 adds `mjbatch`: many simulations of one model share a thread pool, and results are documented as bit-identical to a serial loop at any thread count. Benchmark `implicitfast` and `discrete` separately; include batch throughput and replay fidelity. Do not describe the discrete metric as a Symtropy novelty.
- **Rapier 0.36.1 (2026-10-10):** the current 2D/3D release includes soft bodies. Dimforge's Q3 2026 report describes a reworked parallel execution model with intra-island parallelism, persistent graph coloring, and parallel broad phase; its own historical measurements report up to 2x single-thread and 4x multi-thread speedups versus its previous implementation. The report explicitly notes that the engine comparison graphs were run on early-August builds, so those charts are not current cross-engine rankings. Record iteration counts, internal PGS/stabilization iterations, warm starts, CCD, and thread configuration.
- **Nexus 0.7.0 (Dimforge, 2026 Q3):** a Rust GPU multiphysics engine using Rust-authored GPU shaders through rust-gpu, rust-cuda/cuda-oxide or WebGPU. The Q3 report claims 505,000 rigid bodies at about 14 ms/frame on an RTX 5080 for a pyramid scene, with no sleeping-island optimization; the same report says CUDA was about 10% faster than WebGPU on that system. These are vendor-reported measurements, not independent results. Pin Nexus version, backend, GPU and scene; do not compare the RTX 5080 headline to a CPU or laptop-GPU result.
- **Physmark:** a public benchmark suite/reporting platform associated with the Nexus project. It separates large rigid-body piles/stacks, many small parallel robotics environments, and non-rigid MPM workloads, and can record/replay benchmark videos. Treat it as a useful scenario taxonomy and reproducibility reference, not an unquestioned neutral authority: its Q3 2026 report says it is largely AI-assisted and maintained by Dimforge, and warns that some comparison datapoints use early-August engine builds. Re-run all backends at pinned current versions on the same host before accepting rankings.
- **ComFree-Sim (arXiv:2603.12185, 2026-03):** a GPU-parallel analytical contact engine using complementarity-free prediction/correction in the dual cone of Coulomb friction. It extends to 6D contact with tangential, torsional and rolling friction. Its authors report near-linear contact scaling and 2–3x throughput over MJWarp in evaluated dense-contact scenes with comparable physical fidelity. Treat these as paper-reported workload-specific findings, not a general engine ranking. This is a serious alternative to assuming a global LCP is the production endgame; compare it on the same penetration, friction, work and scaling receipts. See [ComFree-Sim paper](https://arxiv.org/abs/2603.12185) and [project page](https://irislab.tech/comfree-sim/).
- **PhysX 5.8:** include TGS and PGS separately. NVIDIA documents TGS as its recommended direction, using time substeps and reporting better convergence, high-mass-ratio handling, joint-drive accuracy, and high-frequency response than PGS. Record position/velocity iteration and friction-patch settings.
- **Newton:** include for GPU and differentiable robotics workloads. It is built on Warp/OpenUSD, developed by NVIDIA, Google DeepMind and Disney Research, and can use MuJoCo Warp. Its solver families include XPBD, VBD, MuJoCo, Featherstone, SemiImplicit, Kamino and implicit MPM. It is not a direct apples-to-apples baseline for a Rust, CPU-first, dimension-generic core; compare on matched robotics/contact workloads where backend and precision are supported.
- **Project Chrono 10.0.0 (2026-03-27):** use for engineering/multiphysics comparisons, not game-engine frame-time claims. Relevant capabilities include FSI, peridynamics, checkpointing, and implicit-integrator Jacobian update strategies.

### Differentiable-contact comparison axis

Forward physical fidelity and backward gradient usefulness are different objectives and must not be collapsed into one score. ICLR 2026's DiffMJX uses adaptive time integration to improve gradient accuracy and adds contacts-from-distance only in the backward pass to provide pre-contact gradient signal while preserving the physical forward contact result. Bundled Contact Gradients (arXiv:2609.30951, 2026-09) instead uses contact-local randomized smoothing to reduce gradient variance around stiff contacts and reports zero-shot transfer of dynamic motions to a Unitree G1. These methods motivate a future differentiable Symtropy campaign, but they are not a requirement for the current LCP oracle. See [DiffMJX / ICLR 2026](https://proceedings.iclr.cc/paper_files/paper/2026/hash/44039e59aaf6a41b16f1fc5b27bcd409-Abstract-Conference.html), [Bundled Contact Gradients](https://arxiv.org/abs/2609.30951), and [BCG project site](https://bundledcontactgradients.github.io/).

### Workload families: don't collapse unlike physics into one score

1. **Large rigid-body worlds:** 1K, 10K, 100K and (where hardware permits) 500K bodies; piles, pyramids, broad contact graphs, sleeping and wake-up storms. Compare CPU Rapier/MuJoCo/PhysX where supported and GPU Nexus/Newton/Isaac/PhysX where supported. Record throughput, p95 frame time, peak memory, active/awake fraction, contact count, island size distribution, and physical defect.
2. **Many-environment robotics:** 1, 64, 256, 1K and 4K copies of small articulated scenes with randomized initial conditions/actions. Measure simulated steps/sec, environment throughput, action/observation latency, gradient quality where differentiable, and trajectory/replay determinism. This is a separate market and architecture from one huge connected pile.
3. **Tightly coupled large islands:** 10, 100, 1K and 10K bodies in one connected contact island, including stacks, joint grids, ragdolls and high mass-ratio mechanisms. This is the key test of Rapier's intra-island parallelism and avoids "many small independent scenes" hiding a poor large-island solver.
4. **Non-rigid / continuum:** MPM, flexes, soft bodies, cloth and tearing. Compare Symtropy only after it implements the same constitutive model, discretization, and material/contact semantics. Otherwise report capability gaps, not a fake performance score.
5. **Dimension scaling:** D=2,3,4 analytic fixtures first, then D=5,6,8 stress tests where compilation and geometry allow. Report the growth in generalized velocity dimension \\(D+D(D-1)/2\\), row count, matrix storage, assembly time and solver time. N-dimensional correctness is a differentiator, but asymptotic cost is part of the truth.

### Comparison methodology


1. **Mathematical oracle:** for 1–8 row islands, use exhaustive active-set enumeration or an independently implemented KKT/LCP reference. For free rigid bodies, use high-accuracy ODE integration and conservation laws. Engines are comparators, not ground truth.
2. **Matched-quality work curves:** sweep timestep, solver tolerance/iteration budget, island size and scene stiffness. Plot state/contact error, invariant drift and residual against total CPU/GPU time. Report p50/p95 latency, memory and allocation count. A faster run with larger physical error is not a win.
3. **Reproducibility:** replay identical inputs across 1/2/4/8 threads, repeated process runs, and supported CPU architectures. First require bit-identical replay for a pinned compiler/architecture/thread-count contract; then quantify cross-architecture divergence separately. Do not conflate deterministic quantization with deterministic execution.
4. **Scaling:** cover three different regimes: large rigid-body worlds (10k/100k/500k bodies), many small parallel environments (256/512/1k/4k), and one large coupled island with dense contact. Measure independent-island throughput and intra-island parallel efficiency separately. Compare CPU and GPU backends on matched hardware; record memory use, allocations and synchronization overhead as well as step time.
5. **Friction and contact fidelity:** compare penetration distributions, sliding/sticking transitions, torsional/rolling resistance, contact-work and energy drift across dt, stiffness/impedance and friction sweeps. Include ComFree-style decoupled dual-cone candidates and coupled LCP baselines; never compare throughput without matching physical tolerances.
6. **Differentiable workloads:** only for engines exposing gradients, report finite-difference gradient error, gradient variance near contact-mode changes, useful pre-contact gradient coverage, and policy/task transfer. Keep backward estimator metrics separate from forward physical fidelity.
7. **Failure reporting:** invalid/singular inertia, ill-conditioned islands, infeasible constraints, CCD misses, non-finite state and exhausted solve budgets must be explicit outcomes. Do not convert them into zero impulses or silently omit them from aggregate metrics.

### Competitive hypothesis and evidence threshold

The plausible Symtropy differentiation is a *dimension-generic, auditable solver portfolio*: exact small-island references and coupled corrections where needed; an optional decoupled dual-cone/impedance fast path for contact-rich throughput; residual-triggered escalation between them; attributable physical-work accounting; and deterministic, replayable receipts. This is a research hypothesis, not a novelty claim or an advantage today. A credible win requires at least one of:

- lower physical error at the same compute budget on a frozen target set;
- lower compute cost at the same error and failure rate;
- broader verified dimensional coverage without a correctness or stability regression;
- stronger replay guarantees under the stated execution contract; or
- a demonstrably better failure envelope on ill-conditioned and adversarial cases.

Before making a general SOTA claim, require a repeatable >=20% win on a named workload family, no statistically material regression on the rest of the matched suite, and complete raw receipts. The threshold is a campaign decision rule, not a statement of current performance.


## Implementation status (2026-10-30)

The first independent normal-contact reference is now implemented in [PR #2988](https://github.com/Luminous-Dynamics/symtropy/pull/2988), in `crates/core/symtropy-physics/src/discrete_contact.rs`. It builds generalized contact rows from translational and bivector coordinates, including the Rotor exp(-B) sign; a direct kinematic test compares J*u with independently evaluated point velocities. It assembles W = J M_eff^-1 J^T from the row Jacobian and a caller-supplied inverse effective-mass metric, validates Delassus symmetry/PSD, recomputes unilateral KKT residuals, and exhaustively enumerates active sets for up to eight rows. It is intentionally not integrated into `PhysicsWorld` and does not yet assemble the inverse effective-mass metric directly from live `RigidBody` inertia operators, integrate impulses, or solve friction. The intended merge/review sequence is to qualify the inertia and failure-propagation foundations first, then compare this reference against independent analytical/QP oracles before building a production island solver.

## 6. Paradigm shifts worth testing

### 6.0 Solver portfolio rather than one universal contact algorithm

Keep the exhaustive small-island LCP as an independent correctness oracle. It should not dictate that all production contact workloads use a global complementarity solve. Test three candidates behind the same row/work receipt:

1. **Reference / exact small islands:** active-set LCP for 1–8 rows; independent algebraic oracle and diagnostic.
2. **Coupled accurate solver:** Delassus/Newton or projected methods for moderate, strongly coupled islands where off-diagonal inertia and joint coupling matter.
3. **Decoupled contact fast path:** dual-cone/impedance or analytical per-contact updates where contact-pair decoupling is acceptable; validate penetration, cone feasibility, energy/work and contact-order sensitivity against the reference and high-accuracy scenes.

Use residual, island coupling, mass ratio, condition estimates and stiffness to escalate between paths. The escalation decision must be deterministic for a fixed receipt and must not let a heuristic certify its own correctness. This is a portfolio hypothesis; it is not a proven faster method.



### 6.1 Adaptive solver effort from physical defect, not fixed iteration count

Estimate the post-solve complementarity residual, energy/work residual, penetration defect and local integration error. Spend extra iterations or substeps only on islands whose defects exceed a declared budget. This could outperform fixed global iteration counts on heterogeneous scenes, but must be tested against worst-case tail latency and starvation.

### 6.2 Conservative-by-construction exchange ledger

Every external force, actuator, damping term, contact impulse, friction impulse and stabilization correction records its expected work/energy exchange. The step report reconciles measured state-energy change against the ledger. Numerical drift becomes an observable residual with an attributable source, not just a scalar total-energy trend.

### 6.3 Multi-fidelity island solve

Use an inexpensive diagonal/block approximation to precondition a more accurate island-level solve. Promote an island to the expensive solve when its condition estimate, residual, mass ratio or contact coupling demands it. Validate both the classifier and the fallback; never let a learned/heuristic choice certify its own correctness.

### 6.4 Dimension-generic mechanics as a real differentiator

Keep D-dimensional translational and bivector rotational mechanics in the same algebraic API. Verify D=2, 3, 4, then test randomized higher dimensions for algebraic consistency. Do not confuse generic compilation with evidence of correct or efficient high-D simulation.

### 6.5 Solver portfolio: exact where coupled, analytical where decoupled

Keep the small-island LCP as an oracle and a fallback for strongly coupled or ill-conditioned islands. In parallel, prototype a ComFree-inspired dual-cone impedance update that solves each contact and cone facet analytically. Do not silently substitute one for the other. The fast path must report primal friction-cone violation, penetration, contact work, energy defect, and an estimate of off-diagonal coupling between contacts. Escalate to a coupled block/LCP solve when any declared defect or coupling threshold is exceeded. Commit only after the selected path passes its independent checks.

The decisive benchmark is not just speed: compare decoupled and coupled solutions against one another and a higher-precision oracle on stacks, shared-body multi-contact, redundant rows, torsional/rolling friction, high mass ratios, and contact-order permutations. Measure the error/throughput frontier and worst-case tail latency.

### 6.6 Keep forward physics and backward gradients separate

A physically credible forward contact solver need not provide useful gradients at contact-mode changes. Prototype backward-only estimators (distance-based straight-through gradients or local randomized smoothing) as separate modes, inspired by DiffMJX and Bundled Contact Gradients. The forward state transition remains governed by the declared physical solver. Measure finite-difference gradient error, gradient variance, pre-contact gradient coverage, and sim-to-real task transfer independently.

### 6.7 Memory layout and batched execution are algorithmic concerns

Benchmark AoSoA/SoA layouts, deterministic contact sorting, stable segmented reductions, and batched world stepping. Target both large single worlds and thousands of small environments. A GPU kernel that is fast for independent pairs but requires costly global synchronization can still lose on one large island; report those regimes separately.

## 7. Required receipts

Each run must persist:

- source commit SHA and dirty-tree status;
- canonical scene/input SHA-256 and scenario schema version;
- engine/backend versions and complete solver settings;
- compiler, target triple, CPU/GPU, OS and feature flags;
- wall-clock distribution (warmups, samples, p50/p95), memory and allocations;
- residuals, invariant drift, constraint violations, failed solves and CCD misses;
- reference fixture version, reference precision, and uncertainty envelope.

A missing required field invalidates the comparison. A queued or skipped workflow is not a passing result.

## 8. Exit decision

Do not replace the existing solver based on a clean demo. Promotion requires:

1. small-island algebra and oracle comparisons pass;
2. cross-path impulse/effective-mass identity passes;
3. contact and friction campaigns meet frozen physical tolerances;
4. high-spin and CCD adversarial suite reports no misses;
5. matched-work performance is competitive with pinned MuJoCo/Rapier/PhysX/Nexus and ComFree-style reference baselines; and
6. the hybrid/portfolio switch demonstrates a repeatable accuracy-throughput win without violating its certification envelope;
7. the new path does not regress legacy scenes or determinism/replay contracts.

Until then this remains an isolated research implementation and benchmark campaign.
