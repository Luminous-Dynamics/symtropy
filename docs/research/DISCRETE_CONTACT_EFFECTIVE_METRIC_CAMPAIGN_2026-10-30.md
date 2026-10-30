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
- **Rapier 0.36.1 (2026-10-10):** the current 2D/3D release includes soft bodies. Q3 2026 describes intra-island parallel solving, persistent graph coloring, and parallel broad phase; Dimforge reports up to 2x single-thread and 4x multi-thread improvement over its previous Rapier version on an M4 Max with eight workers (not a cross-engine comparison). The documented 0.36 guide uses four default solver iterations and suggests 8–12 for demanding stacks/machinery. Record iteration counts, PGS/stabilization, warm starts, CCD policy, and parallelism; defaults are not equivalent error tolerances.
- **ComFree-Sim (2026, arXiv:2603.12185):** a serious alternative to global complementarity solves. It predicts smooth velocity, then computes analytical dual-cone impedance corrections per contact and per polyhedral cone face; its 6D model includes tangential, torsional and rolling friction. On an AMD 32-core + RTX 4090 setup, its paper reports near-linear contact-count scaling, about 3x speed in a dense-contact sweep, and about 2x throughput in batched Allegro-hand environments versus MJWarp. This is a workload-specific published result, not a universal engine ranking: their contact-count benchmark uses 512 environments, 750 steps and dt=0.002 s, impedance parameters affect penetration, and the authors note the method often benefits from a smaller timestep than MJWarp. Benchmark its decoupling, polyhedral-cone and impedance choices explicitly.
- **Dimforge Nexus 0.7 / Physmark:** Nexus is a Rust-authored GPU engine with AoSoA-style data layout and Soft-TGS. Dimforge reports a 505,000-body rigid-body pyramid at about 14 ms/frame on an RTX 5080, without sleeping-island optimization. Physmark separates large rigid-body scenes, many parallel robot environments and MPM workloads, and stores replayable video captures. Treat these as important GPU scale/workload comparators, but reproduce on matched hardware, scenes and fidelity metrics rather than comparing the 5080 result directly to CPU runs.
- **Differentiable contact is a separate quality axis:** ICLR 2026 DiffMJX combines adaptive time integration with backward-only contacts-from-distance gradients to improve hard-contact gradient usefulness without altering the forward physical contact result. Bundled Contact Gradients uses local randomized smoothing around stiff-contact configurations and reports zero-shot transfer to a Unitree G1. Compare these on gradient accuracy/variance and policy transfer—not as substitutes for forward-contact accuracy.
- **PhysX 5.8:** include TGS and PGS separately. NVIDIA documents TGS as its recommended direction, using time substeps and reporting better convergence, high-mass-ratio handling, joint-drive accuracy, and high-frequency response than PGS. Record position/velocity iteration and friction-patch settings.
- **Newton:** include for GPU and differentiable robotics workloads. It is built on Warp/OpenUSD, developed by NVIDIA, Google DeepMind and Disney Research, and can use MuJoCo Warp. It is not a direct apples-to-apples baseline for a Rust, CPU-first, dimension-generic core; compare on matched robotics/contact workloads where the backend and precision are supported.
- **Project Chrono 10.0.0 (2026-03-27):** use for engineering/multiphysics comparisons, not game-engine frame-time claims. Relevant capabilities include FSI, peridynamics, checkpointing, and implicit-integrator Jacobian update strategies.

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
