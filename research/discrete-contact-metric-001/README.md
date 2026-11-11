# Discrete effective-metric contact step — research seed

This standalone Rust crate is a small reference implementation for a discrete constrained step in which inertia, damping, position stiffness and unilateral contact are coupled through one effective metric.

## Mathematical contract

For a linearized backward-Euler force model:

- M (v_next - v) = h (f - D v_next - K (q_error + h v_next)) + J^T p
- M_hat = M + h D + h^2 K
- M_hat v_next = M v + h f - h K q_error + J^T p

For each frictionless unilateral contact:

- J v_next >= -gap / h
- p >= 0
- p dot (J v_next + gap/h) = 0

The contact Delassus matrix is J M_hat^-1 J^T. It deliberately uses the same M_hat as the unconstrained step.

## Why this exists

A simulator's step map is the object optimized by a discrete-time controller. Comparing one engine's continuous acceleration with another engine's post-contact discrete transition can conceal different physical and numerical semantics. This seed makes the discrete step explicit, small enough to inspect, and independently testable.

## Scope

- Dense deterministic reference, at most 12 generalized coordinates and 10 contacts.
- Frictionless, zero-restitution unilateral contacts.
- Linearized contact Jacobian and position error over one step.
- Exhaustive small active-set enumeration.
- No CCD, friction, impact restitution, changing contact features, nonlinear geometry, articulated-tree assembly, or production solver integration.
- No performance or superiority claim.

## Initial tests

- Analytic scalar backward-Euler oscillator.
- Single-contact impulse and complementarity.
- Coupled 2-DOF stiffness/contact metric.
- Deterministic two-contact active-set resolution.
- Invalid timestep, non-SPD mass, and asymmetric-matrix negative controls.

## Numerical scale and tolerance boundary

Matrix symmetry and Cholesky pivot checks use tolerances relative to each
matrix's maximum absolute entry. Uniformly changing the mass scale must not
make a mathematically equivalent SPD matrix fail solely because its entries
are smaller than one. The active-contact classification also uses the
solver's nonnegative impulse result rather than an absolute impulse cutoff.

The regression suite checks a one-contact solution at mass scales
`1e-12`, `1` and `1e12`, plus rejection of a small-magnitude but
relatively asymmetric stiffness matrix. This is a narrow scale-covariance
check, not a claim that every tolerance in the reference is dimensionless:
the LCP feasibility and complementarity thresholds are still absolute and
need a separate unit-/scale-normalized residual design before this crate can
serve as a general numerical qualification oracle.

## Comparator snapshot — 2026-11-10

These are capability comparators, not interchangeable ground-truth solvers. Pin exact binaries/commits, precision, backend, contact law, integrator, timestep, and solver tolerances before comparing.

- [MuJoCo](https://mujoco.readthedocs.io/en/4.0.0/changelog.html): the effective-metric `discrete` step map was introduced in 3.13.0 (2026-09-08), using `M + hD + h^2K` and solving constraints against that same metric. It is carried into 4.0.0 (2026-11-06), which adds opt-in speculative contacts for fast convex geometry and ideal / continuum elasto-plasticity. Symtropy must not claim the effective-metric formulation itself as novel. The [upcoming changelog](https://mujoco.readthedocs.io/en/latest/changelog.html) records a sparse-Cholesky/Newton-solver bug introduced in 4.0.0, so keep dense and affected sparse profiles separate and do not treat an unqualified sparse 4.0.0 run as an oracle.
- [Newton 1.8.0rc1](https://github.com/newton-physics/newton/releases/tag/v1.8.0rc1), released 2026-11-04, includes experimental [Steel](https://newton-physics.github.io/newton/latest/api/_generated/newton.solvers.SolverSteel.html): Featherstone articulated-body dynamics with projected Gauss-Seidel constraints and coupled Coulomb friction. In its 2026-11 announcement, Skild reports 3.8x geometric-mean throughput over PhysX across 20 tasks and up to 13x on heterogeneous batches. These are vendor-reported simulation-throughput comparisons at each engine's best measured batch size (Steel also uses the opt-in Newton 1.8 `reuse_kinematics=True` setting), not matched-batch physical-accuracy results; independently reproduce them before making comparative claims. Steel remains experimental and its API may change.
- [Rapier / Dimforge Q3 2026 report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/): reports intra-island parallelism, persistent graph coloring, and parallel broad phase. The report says its engine-comparison charts used early-August builds, so repeat any performance comparison using pinned current builds on one host.
- [Dimforge Nexus 0.7.0](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/): a separate Rust GPU multiphysics lane across CUDA, Metal and WebGPU. Dimforge reports 505,000-body rigid-body pyramids at about 14 ms/frame on an RTX 5080. Treat this as a vendor-reported large-world throughput datapoint, not a matched physical-accuracy result. It belongs in a large homogeneous-scene / GPU scaling lane, not in the same chart as CPU articulated-body or small-island solver timings.
- [Pinocchio 4.0/4.1](https://github.com/stack-of-tasks/pinocchio/blob/devel/CHANGELOG.md): 4.0 added constraint APIs, dense/sparse/Cholesky-expression/rigid-body Delassus operators, and ADMM/PGS constrained solvers; 4.1.0 followed on 2026-07-07. It is a strong articulated mechanics and Delassus reference, not a full collision-world throughput peer.
- [Drake spatial inertia](https://drake.mit.edu/doxygen_cxx/group__multibody__spatial__inertia.html): use its explicit distinction between the point about which inertia is taken and the frame in which it is expressed as a COM/frame contract cross-check.
- [Project Chrono 10.0.0](https://projectchrono.org/news/), released 2026-03-27: use for domain-specific multiphysics, FSI, peridynamics, checkpointing, and implicit-integrator comparisons. Do not rank it against game/robotics engines by generic rigid-body FPS.

Every cross-engine lane needs a semantic-admission record. If collision geometry, restitution, friction cone, compliance, event handling, integrator, constraint target, or contact state differs materially, classify the comparison as approximate or non-equivalent rather than forcing a score.

## Benchmark extension

The next matched comparison should add the same frozen matrix problem to Symtropy's native step, MuJoCo's discrete integrator, and an independently assembled KKT/LCP reference. Compare velocity and impulse, metric/KKT residual, complementarity, next-step gap, energy/work balance, transition derivative, cost, and memory. Only compare a lane after semantic admission confirms contact law, constraint target, integration and damping equivalence.
