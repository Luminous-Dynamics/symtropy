# Unified discrete contact and effective-metric campaign

**Status:** research and benchmark contract; not a production solver implementation.
**Date:** 2026-10-31
**Implementation references:** [small-island Delassus/LCP oracle PR #2998](https://github.com/Luminous-Dynamics/symtropy/pull/2998), [mass-property consistency PR #3012](https://github.com/Luminous-Dynamics/symtropy/pull/3012).

## 1. Research question

Can Symtropy use one mechanically consistent generalized mass/effective-mass model for free dynamics, contact, friction, joints and stiffness, then select a solver per constraint island according to measured residual, coupling and cost?

The intended shift is from a solver chosen globally by convention to a deterministic solver portfolio whose choices are justified by an explicit accuracy/cost contract.

This does **not** assume that a global LCP is the best production method. MuJoCo, Rapier, PhysX and GPU engines already have mature discrete/contact solver strategies. The work is to establish a small independent oracle, build a common metric, and determine where each numerical method is best.

## 2. Shared mechanical metric

For rigid bodies with translational and bivector/angular generalized velocity,

\[
u=(v,B)\in\mathbb R^D\oplus\Lambda^2(\mathbb R^D),
\]

the generalized inverse mass is assembled from translational mass and the body-frame centered second moment rotated into the world frame. A contact row evaluates relative velocity at its lever arms, using the same physical point-velocity convention as the integrator.

For a step of duration h with positive damping D and stiffness K, the candidate discrete metric is

\[
M_{\mathrm{eff}}=M+hD+h^2K,
\]

where M is the generalized mass matrix. This is a *candidate model*, not a blanket instruction to insert arbitrary K: only terms that retain a symmetric positive-definite effective metric belong in the first reference contract. Nonsymmetric gyroscopic and non-conservative terms need explicit treatment rather than silent symmetrization.

For contact rows J, the Delassus operator is

\[
W=J M_{\mathrm{eff}}^{-1}J^T.
\]

For frictionless unilateral rows, the reduced problem is

\[
\lambda\ge0,\quad W\lambda+q\ge0,\quad
\lambda^T(W\lambda+q)=0.
\]

A normal-only LCP is not a Coulomb-friction solver. Friction, rolling/twist friction, compliance, restitution and stabilization require their own declared model and validation.

## 3. Solver portfolio hypothesis

Use one common row/metric contract with several implementations behind it:

1. **Small-island reference:** exhaustive active-set LCP for 1–8 frictionless rows. This is the independent oracle already added in the research branch; it is not a production path.
2. **Realtime CPU path:** warm-started block sequential impulses / TGS-style substeps for small and medium islands, with residual and work accounting.
3. **Coupled difficult-island path:** a deterministic projected or convex/dual-cone solve for dense, stiff, high-mass-ratio or strongly coupled islands. Compare projected Gauss–Seidel, block solvers and primal/dual active-set methods before selecting one.
4. **Large-scale/GPU path:** row- or block-parallel Jacobi/AVBD-style methods where throughput justifies their convergence profile. Use the same physical regression suite; never infer equivalence from visually similar piles.
5. **Fallback/recovery:** when the measured residual, complementarity or invariant ledger fails its gate, deterministically escalate iterations/solver, split the step, or reject it. Do not silently loosen tolerances or accept a failed solve.

Solver selection must be deterministic from declared island features (row count, coupling, mass ratio, compliance/stiffness, friction model, conditioning and requested accuracy). A learned policy may suggest configurations, but a simple reference policy and all selection decisions remain auditable.

## 4. Executable benchmark matrix

### A. Mathematical kernel tests

- Analytical solid-ball and ellipsoid mass moments in 2D/3D/4D.
- Asymmetric, off-diagonal and nearly singular mass moments.
- Delassus symmetry and positive semidefiniteness.
- Effective-mass / impulse-response identity.
- Contact-row permutation and global rigid-rotation covariance.
- Unit transformation SI ↔ g/cm/ms.
- Redundant rows and rank-deficient active blocks.
- A known feasible complementarity problem with an independently checked residual.

### B. Rigid contact and stability scenes

- Two-body impacts with variable restitution and asymmetric inertia.
- 2, 4, 8, 16 and 32 point contact patches.
- Resting box stacks, pyramids, rings and high aspect-ratio objects.
- Mass ratios: 1, 10, 100, 1,000 and 1,000,000.
- Frictionless, Coulomb, rolling and twist-friction cases.
- Stiff compliant contacts, limits, motors and joint drives.
- Fast thin-body impacts, rotating sweep, grazing impacts and dynamic/dynamic tunneling.
- Dense islands with redundant constraints and poor conditioning.

### C. Robotics and many-environment workloads

Physmark now separates large rigid-body scenes from small scenes replicated across many environments for robot-learning throughput, and includes MPM non-rigid workloads. Symtropy should adopt those *workload families* while adding its own N-dimensional asymmetric mechanics cases.

Measure 1, 32, 256 and 1,024 parallel environments where the backend supports them. Record throughput per environment as well as aggregate throughput; a high aggregate that starves individual environment latency is not enough.

### D. Generalization / dimensionality

- 2D and 3D compatibility cases.
- 4D coupled bivector tests exercising all six independent rotation planes.
- Higher D only after matrix dimensions, storage and algorithmic complexity are recorded. Do not present 4D correctness as evidence of scalable arbitrary-D performance.

## 4.1 Dimensional similarity campaign

Run the same dimensionless physical scenario after coherent changes of units, with all dimensional inputs transformed consistently. For length, mass and time scales \\(\\alpha,\\beta,\\gamma\\):

\[
\begin{aligned}
x'&=\alpha x,& v'&=\alpha v/\gamma,& a'&=\alpha a/\gamma^2,\\
m'&=\beta m,& F'&=\beta\alpha F/\gamma^2,&
S'&=\beta\alpha^2 S,\\
\tau'&=\beta\alpha^2\tau/\gamma^2,&
\omega'&=\omega/\gamma,&
E'&=\beta\alpha^2 E/\gamma^2.
\end{aligned}
\]

The first free-flight integration fixture uses an asymmetric point-mass inertia distribution, gravity, external force and torque. Its target is normalized position/velocity/orientation/energy error <= 1e-8 over the committed scale set. It includes an extreme coherent length/time scaling that previously caused the absolute angular-rate gate to skip rotation.

Follow-on scale families must cover:
- sphere/half-space and box/half-space contact;
- multi-point contact with asymmetric rotational inertia;
- Coulomb, rolling and twist friction;
- CCD and rotating thin-body impact;
- compliant contacts, joints and motors;
- sleeping/waking and broadphase margins.

Normalize penetration by feature size, contact residual by characteristic velocity/impulse, and energy/work residual by characteristic energy. Every tolerance with physical dimensions must state its units and scale law. Do not infer full engine unit-invariance from the current free-flight test.

## 5. Promotion targets

These are proposed gates, not achieved measurements:

| Metric | Reference target | Production target |
|---|---:|---:|
| Delassus relative symmetry error | ≤ 1e-12 | ≤ 1e-10 |
| Minimum eigenvalue of normalized symmetric W | ≥ -1e-12 | ≥ -1e-9 |
| Normalized complementarity/primal residual | ≤ 1e-10 | ≤ 1e-6 |
| Analytical impulse / effective-mass identity | ≤ 1e-10 relative | ≤ 1e-7 relative |
| Rigid-frame covariance error | ≤ 1e-10 | ≤ 1e-8 |
| Known invalid mass/inertia or singular solves silently accepted | 0 | 0 |
| Tunneling on frozen adversarial CCD suite | 0 | 0 |
| Non-finite state committed | 0 | 0 |

For energy, measure signed work and energy residual rather than imposing a generic “energy must always decrease” rule. Closed conservative scenes, inelastic impacts, restitution, friction, damping and external forcing have different correct balances. Require unexplained normalized energy residual ≤ 1e-6 on canonical isolated cases before tightening the threshold.

For resting contacts, report maximum penetration normalized by the minimum relevant feature size and the tail distribution, not just the mean. Establish the acceptance bound from the contact model and time-step convergence campaign; an initial research target is ≤ 1e-3 of feature size in the stable-resting suite.

## 6. Performance protocol

Every comparison must pin:
- exact engine/version and source revision;
- CPU/GPU model, memory, OS, driver and thermal/power mode;
- compiler/toolchain, flags, precision and backend;
- scene geometry, seed, units, masses, materials and timestep;
- solver family, iterations/substeps, warm-start, CCD, sleeping and accuracy settings;
- number of warm-up and timed runs.

Report p50/p95/p99 step time, simulated seconds per wall-clock second, peak memory, contacts/constraints per second, throughput per environment, residual/penetration/energy error, and failure/tunneling counts. Save replayable trajectories or captures so that faster but incorrect runs can be rejected.

Use separate comparisons:
- CPU vs CPU on identical hardware and scene.
- GPU vs GPU on identical hardware and scene.
- Same engine CPU vs GPU on the same scene.
- Matched-accuracy work-precision curves, not default-settings-only charts.

A performance advantage is credible only when it is measured at the same physical accuracy/failure envelope. First target a Pareto improvement: lower runtime at equal error, or lower error at equal runtime. Set absolute throughput goals only after the baseline has been run.

## 7. Industry baselines as of 2026-10-31

- **MuJoCo 3.17.0** (22 October 2026) adds a batch API for stepping many copies of one model. Its release notes state that batch output is bit-identical to a plain loop at any thread count. MuJoCo's implicitfast remains its documented recommended general-purpose integrator, while discrete couples constraints and implicit updates in the shared effective metric; the docs still describe discrete as new and under active development.
- **Rapier 0.35.3** (28 August 2026) is the current release identified in its changelog. The 0.35.2 line added an energy-injection safeguard for near-singular multibody Coriolis updates. Rapier's CPU/SIMD and island solver is a direct Rust baseline.
- **Nexus 0.7.0 / Physmark**: Dimforge's 25 October 2026 report says Nexus, written in Rust GPU shaders, reached 505,000 rigid bodies at about 14 ms/frame on an RTX 5080 in a pyramid scene, without sleeping-island optimization. This is a vendor-reported result, not a generic engine ranking. Physmark compares three workload families: large piles/stacks, many small robotics environments, and MPM; it records videos and runs multiple engine backends. Use it as a workload/methodology reference and reproduce scenes before comparing claims.
- **NVIDIA PhysX 5.8**: TGS is documented as its generally recommended solver, with improved convergence, high-mass-ratio handling, drive accuracy and high-frequency response relative to PGS.
- **Newton**: open-source, GPU-accelerated and differentiable robotics engine built on Warp/OpenUSD with multiple solver families. It is an essential GPU/robotics baseline, but not an apples-to-apples CPU Rust comparison.
- **Project Chrono 10.0.0**: mature engineering and multiphysics reference for multibody, FSI, peridynamics and implicit-integration validation, rather than a direct real-time game-engine competitor.

The strongest near-term competition is not a raw frame-rate shootout. Symtropy must beat these systems on a clearly defined problem: dimension-generic asymmetric rotational response, auditable mass-property provenance, reproducible discrete mechanics, failure-aware world steps, and an openly measured solver portfolio. Until independent benchmarks demonstrate that advantage, it remains a research hypothesis.

## 8. Immediate implementation order

1. Make the current small-island oracle compile and pass exact-head CI.
2. Fix mass/inertia metadata consistency and geometry/COM merge failure semantics.
3. Turn the current microbenchmarks into a versioned work-precision harness with deterministic inputs and output receipts.
4. Add block/tall-stack, high-mass-ratio and thin-body CCD scenes with reference outcomes.
5. Add a single shared island row contract for contacts and joints; only then experiment with implicit damping/stiffness terms.
6. Prototype a deterministic residual-driven solver escalation policy. Keep the policy transparent and benchmark it against fixed TGS/PGS baselines.
7. Add GPU/many-environment work only after CPU correctness and reference trajectories are stable.
