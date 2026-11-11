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
relatively asymmetric stiffness matrix. The later **Unit-aware contact
residual contract** section supersedes the original residual limitation: the
active-set admission path now uses dimensionless dual, primal, complementarity
and effective-metric equation residuals. Arbitrary mixed generalized-coordinate
scaling, ill-conditioned coordinates and a normalized energy/work ledger remain
open research limits.

## Comparator snapshot — 2026-11-11

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

## Unit-aware contact residual contract

Absolute residuals remain useful diagnostics, but they cannot decide physical admission across different unit systems. The solver now uses dimensionless acceptance residuals while retaining its absolute residual fields for traceability:

- Effective-metric equation: for each generalized coordinate, divide `|M_hat v+ - M v - h f + h K q_error - J^T p|` by the largest absolute contributing term on that coordinate; report the maximum.
- Dual feasibility: for each contact row i, divide `max(-p_i, 0)` by its own characteristic impulse scale `max(|p_i|, (|q_i| + Σ_{j ≠ i}|W_ij||p_j|)/|W_ii|)`, where `W = J M_hat^-1 J^T` and `q = target - J v_free`. Aggregate by the maximum row residual. This prevents a large unrelated contact impulse from hiding a smaller row's negative impulse.
- Primal feasibility: per contact row, divide the negative part of `J_i v+ - target_i` by `max(|J_i v+|, |target_i|)`.
- Complementarity: for each row with positive impulse, divide `|p_i s_i|` by `|p_i| max(|J_i v+|, |target_i|)`; zero-impulse rows satisfy complementarity by construction.

Zero-scale handling is explicit: an exactly zero residual maps to zero; a nonzero violation with no nonzero reference scale maps to infinity and is rejected. The active-set candidate checks use these normalized criteria rather than absolute, dimensioned cutoffs. The dual scale is row-local, not the global impulse-vector infinity norm. Absolute residual outputs remain available and must not be interpreted as cross-unit acceptance scores.

The regression suite checks the one-contact step at uniform mass scales `1e-12`, `1`, and `1e12`; normalized feasibility/complementarity invariance under uniform velocity and impulse rescaling; and a mixed-scale negative control where a local negative impulse is 100% infeasible but would appear negligible under global-vector normalization. This is not a proof of invariance under arbitrary mixed generalized-coordinate scaling or badly conditioned coordinate changes. Those still require coordinate-aware characteristic scales and separate metamorphic tests.

Still outstanding: a normalized energy/work ledger (including contact work and frictional dissipation), error bounds under nonzero restitution and friction, and independent KKT residual comparison on a broader corpus. The current reference remains frictionless, zero restitution, fixed-J, single-step and small-island only.

## Benchmark lanes — keep unlike workloads separate

1. **Small constrained islands (CPU reference lane):** 1–10 contacts, 1–12 generalized DOFs; active-set enumeration versus independent KKT/LCP and MuJoCo discrete-step fixtures. Score physical error and normalized residuals before latency.
2. **Many homogeneous worlds (throughput lane):** fixed-size batches of identical scenes, reporting worlds/s and simulated-seconds/s at fixed batch sizes. Separate CPU and GPU.
3. **Heterogeneous robot batches:** mixed morphologies, joint counts, contact topologies and task lengths; report throughput, tail latency, padding/occupancy and per-world quality. This is the relevant lane for Newton Steel's heterogeneous-batch claims.
4. **Large homogeneous rigid-body scenes:** 10k–500k bodies, pyramids, broad-phase-heavy and contact-dense scenes; report broad-phase, narrow-phase, solver and synchronization costs separately. This is the lane for Rapier/Nexus large-scene work, not the same leaderboard as small islands.
5. **Engineering multiphysics:** FSI, granular/peridynamic, thermal and coupled implicit cases; compare against domain-specific Chrono capabilities with matched constitutive laws and boundary conditions.

Every throughput lane must publish both fixed-batch comparisons and each engine's peak-throughput configuration. Pin hardware, driver, backend, precision, compiler, threads, solver/integrator settings, timestep, contact law, warmup, batch size and exact source revisions. Report p50/p95/p99/worst-case step time, memory, iterations, failure/rejection rate, and quality residuals alongside throughput.

