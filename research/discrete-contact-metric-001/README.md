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
and effective-metric equation residuals. One moderate diagonal generalized-
coordinate scaling is now covered by a separate metamorphic test. Arbitrary dense
coordinate mixing, extreme conditioning and a normalized energy/work ledger remain
open research limits.

## Comparator snapshot — 2026-11-20

These are capability comparators, not interchangeable ground-truth solvers. Pin exact releases/commits, precision, backend/device, contact law, integrator, timestep, and solver tolerances before comparing. A moving main-branch feature must never be attributed to a tagged release.

- [MuJoCo 4.0.1](https://mujoco.readthedocs.io/en/4.0.1/changelog.html): its `discrete` integrator (introduced in 3.13.0 on 2026-09-08) uses `M + hD + h^2K` and solves constraints against the same effective metric. Version 4.0.0 (2026-11-06) added opt-in speculative contacts and stateful elasto-plasticity; version 4.0.1 (2026-11-11) fixed a sparse-Cholesky/Newton Hessian-factor bug introduced in 4.0.0 that could cause slow convergence, wrong accelerations or divergence with sparse constraint Jacobians. Pin 4.0.1 for the current comparator; keep dense and sparse solver profiles separate, and do not treat an affected 4.0.0 sparse Newton run as a ground-truth oracle. The effective-metric formula is established industry practice and is not a novel Symtropy claim. [Integrator guidance](https://mujoco.readthedocs.io/en/4.0.1/computation/index.html)
- [MuJoCo mjBatch](https://mujoco.readthedocs.io/en/3.17.0/changelog.html): introduced in 3.17.0, it batches independent simulations with persistent state and reports bit-identical results to a serial loop across thread counts. This is an independent-world throughput comparator, not evidence about one large coupled contact island.
- [Newton 1.8.0](https://github.com/newton-physics/newton/releases/tag/v1.8.0): released 2026-11-11; includes experimental [SolverSteel](https://newton-physics.github.io/newton/1.8.0/api/_generated/newton.solvers.SolverSteel.html), targeting articulated rigid-body/contact workloads. Pin the 1.8.0 [solver feature matrix](https://newton-physics.github.io/newton/1.8.0/solvers/index.html): its supported joints, drive formulations and friction/compliance options are solver- and device-specific. In particular, CPU execution uses world-serial portable sweeps; several features and colored solving require CUDA. Do not attribute Newton-main / Skild integration-branch features to the 1.8.0 release. Vendor throughput numbers are simulation-only claims on named hardware/configurations, not physical-accuracy or end-to-end policy-training evidence; reproduce them before ranking.
- [Rapier 0.35.3](https://github.com/dimforge/rapier/blob/master/CHANGELOG.md): current checked stable Rust release. Version 0.35.2 added an energy-triggered fallback re-solve when the semi-implicit Coriolis term injects more energy than applied forces near a near-singular mass matrix; 0.35.3 fixed multibody friction-loss handling. Its `parallel` feature runs broad phase, narrow phase and solver through Rayon, with SIMD-batched contact processing. Dimforge's Q3 2026 report describes persistent graph coloring and intra-island parallel work; pin the exact reported development commit before assigning those newer capabilities to a release.
- [Nexus 0.7.0](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/): a separate Rust GPU multiphysics lane across CUDA, Metal and WebGPU. Dimforge reports 505,000-body rigid-body pyramids at about 14 ms/frame on an RTX 5080. Treat this as a vendor-reported large-world throughput datapoint, not a matched physical-accuracy result. It belongs in a large homogeneous-scene / GPU scaling lane, not in the same chart as CPU articulated-body or small-island solver timings.
- [Drake v1.59.0](https://drake.mit.edu/from_binary.html): latest stable binary release listed by Drake. Retain it for frame-explicit multibody/spatial inertia, composite-rigid-body mass matrices and constrained-dynamics comparisons. Keep the point about which inertia is taken and the frame in which it is expressed explicit.
- [Pinocchio 4.0/4.1](https://github.com/stack-of-tasks/pinocchio/blob/devel/CHANGELOG.md): 4.0 added constraint APIs, dense/sparse/Cholesky-expression/rigid-body Delassus operators, and ADMM/PGS constrained solvers; 4.1.0 followed on 2026-07-07. It is a strong articulated-mechanics and Delassus reference, not a full collision-world throughput peer.
- [Project Chrono 10.0.0](https://projectchrono.org/news/), released 2026-03-27: use for domain-specific multiphysics, SPH/TDPF FSI, peridynamics, checkpointing, and implicit-integrator comparisons. Do not rank it against game/robotics engines by generic rigid-body FPS.
- [SUNDIALS 7.9.0](https://computing.llnl.gov/projects/sundials/sundials-software), released 2026-09: adds extended super-time-stepping methods (ExtSTS) to ARKODE MRIStep. Use it as an independent smooth ODE/DAE and multirate integration reference, not as a rigid-contact engine.

Every cross-engine lane needs a semantic-admission record. If collision geometry, restitution, friction cone, compliance, event handling, integrator, constraint target, contact state, precision or device semantics differ materially, classify the comparison as approximate or non-equivalent rather than forcing a score. Keep single-world accuracy, many-world batching, many-island parallelism and one-large-island parallelism in separate workload classes.

## Benchmark extension

The next matched comparison should add the same frozen matrix problem to Symtropy's native step, MuJoCo's discrete integrator, and an independently assembled KKT/LCP reference. Compare velocity and impulse, metric/KKT residual, complementarity, next-step gap, energy/work balance, transition derivative, cost, and memory. Only compare a lane after semantic admission confirms contact law, constraint target, integration and damping equivalence.

## Unit-aware contact residual contract

Absolute residuals remain useful diagnostics, but they cannot decide physical admission across different unit systems. The solver now uses dimensionless acceptance residuals while retaining its absolute residual fields for traceability:

- Effective-metric equation: for each generalized coordinate, divide `|M_hat v+ - M v - h f + h K q_error - J^T p|` by the largest absolute contributing term on that coordinate; report the maximum.
- Dual feasibility: for each contact row i, divide `max(-p_i, 0)` by its own characteristic impulse scale `max(|p_i|, (|q_i| + Σ_{j ≠ i}|W_ij||p_j|)/|W_ii|)`, where `W = J M_hat^-1 J^T` and `q = target - J v_free`. Aggregate by the maximum row residual. This prevents a large unrelated contact impulse from hiding a smaller row's negative impulse.
- Primal feasibility: per contact row, divide the negative part of `J_i v+ - target_i` by `max(|J_i v+|, |target_i|, |J_i v_free|)`. The pre-contact free normal velocity supplies a characteristic scale when an active contact drives attained velocity to approximately zero.
- Complementarity: for each row with positive impulse, divide `|p_i s_i|` by `|p_i| max(|J_i v+|, |target_i|, |J_i v_free|)`; zero-impulse rows satisfy complementarity by construction.

Zero-scale handling is explicit: an exactly zero residual maps to zero; a nonzero violation with no nonzero reference scale maps to infinity and is rejected. The active-set candidate checks use these normalized criteria rather than absolute, dimensioned cutoffs. The dual scale is row-local, not the global impulse-vector infinity norm. Absolute residual outputs remain available and must not be interpreted as cross-unit acceptance scores.

The regression suite checks the one-contact step at uniform mass scales `1e-12`, `1`, and `1e12`; normalized feasibility/complementarity invariance under uniform velocity and impulse rescaling; a mixed-scale negative control where a local negative impulse is 100% infeasible but would appear negligible under global-vector normalization; and a two-DOF covariance test under `x = S y`, `S = diag(1e-2, 1e2)`. The covariance fixture exposed a near-zero active-contact slack roundoff case; primal and complementarity scaling now include the pre-contact free normal velocity so an approximately zero attained velocity does not turn an O(1e-16) absolute residual into an O(1) relative violation. The coordinate test transforms mass/stiffness, generalized force, velocity and contact Jacobian consistently, then checks recovered physical velocity, contact impulse, slack, active set and normalized residuals. It covers one moderate diagonal scale transform only—not arbitrary dense coordinate mixing, extreme conditioning or all semantically meaningful coordinate changes.

Non-finite admission is fail-closed: any NaN/infinite impulse, row velocity/slack, contributing term, or overflowed effective-metric equation returns an infinite normalized residual. Regression controls cover NaN, infinity, and overflow from finite matrix/velocity inputs so an IEEE `NaN` cannot be silently discarded by a `max()` reduction.

## Discrete energy / work ledger

The current implementation emits a separate `DiscreteEnergyLedger` for each accepted step. Let
`Δv = v_next - v` and `Δq = q_next - q = h v_next`. Multiplying the discrete momentum equation by
`v_next` gives the algebraic balance

```
ΔT + ΔV
  + h * v_nextᵀ D v_next
  + 1/2 * Δvᵀ M Δv
  + 1/2 * Δqᵀ K Δq
  = h * fᵀ v_next + pᵀ J v_next.
```

The ledger records kinetic and potential energy before/after, external force work,
physical damping dissipation, the two increment terms introduced by the implicit
update, and the actual contact-impulse work `pᵀ J v_next`. It also splits contact work into
the contact-target contribution `pᵀ(-gap/h)` and the complementarity product
`pᵀ slack`. Their reconstruction residual checks that the reported contact work
matches the declared unilateral target and slack.

The mass/stiffness increment terms are method-induced discrete terms; they are not
physical damping. A positive-gap contact can have negative impulse work because
the discrete constraint prevents predicted penetration before the next state.
The frictionless, zero-restitution model does not contain a frictional dissipation
term, and this ledger must not be generalized to frictional contact without adding
the corresponding impulse/work law.

New regressions cover the no-contact limit, a touching contact with zero impulse
work, a positive-gap pre-contact impulse whose work is included in the balance,
and invariance of the energy ledger under a dense non-orthogonal unimodular
coordinate re-basing. The existing solver residuals and complementarity checks
remain separate from the energy identity; a small energy residual is not a
substitute for satisfying the contact equations.

Still outstanding: error bounds under nonzero restitution and friction, independent
KKT residual comparison on a broader corpus, and an energy/work ledger for
frictional/contact-law extensions. The current reference remains frictionless,
zero restitution, fixed-J, single-step and small-island only.

## Benchmark lanes — keep unlike workloads separate

1. **Small constrained islands (CPU reference lane):** 1–10 contacts, 1–12 generalized DOFs; active-set enumeration versus independent KKT/LCP and MuJoCo discrete-step fixtures. Score physical error and normalized residuals before latency.
2. **One large connected island (intra-island solver parallelism):** one fixed connected constraint/contact graph with 1k–100k rows where feasible; vary graph degree, friction rows, contact-law stiffness and conflict density. Measure coloring/propagation phases, synchronization, iterations, residual convergence, speedup and parallel efficiency.
3. **Many disconnected islands in one world:** hold total bodies/constraints fixed while varying island count and size distribution. This stresses island discovery, scheduling, wake/sleep and load balance—not independent-world batching.
4. **Many homogeneous worlds (throughput lane):** fixed-size batches of identical scenes, reporting worlds/s and simulated-seconds/s at fixed batch sizes. Separate CPU and GPU; include warm/cold setup and persistent-state costs.
5. **Heterogeneous robot batches:** mixed morphologies, joint counts, contact topologies and task lengths; report throughput, tail latency, padding/occupancy and per-world quality. This is the relevant lane for Newton Steel's heterogeneous-batch claims.
6. **Large homogeneous rigid-body scenes:** 10k–500k bodies, pyramids, broad-phase-heavy and contact-dense scenes; report broad-phase, narrow-phase, solver and synchronization costs separately. This is the lane for Rapier/Nexus large-scene work, not the same leaderboard as CPU articulated-body or small-island timings.
7. **Engineering multiphysics:** FSI, granular/peridynamic, thermal and coupled implicit cases; compare against domain-specific Chrono capabilities with matched constitutive laws and boundary conditions. Use SUNDIALS only for the smooth stiff ODE/DAE portions where its equations and observables match.

For lanes 2–3, sweep 1/2/4/8/physical-core threads; for GPU backends also record device, occupancy, launch count, transfer bytes and graph-capture mode. Freeze both contact graph topology and topology-churn fixtures. Include sparse chains, dense stacks, grids, branched articulations and frictional multi-contact.

Never infer one performance dimension from another: independent-world batch throughput does not measure one-large-island solver parallelism, and a many-island world does not measure contact conflict within one connected island.

Every throughput lane must publish the exact requested case set and one result per case; no failed, unsupported, abstained or not-run case may disappear from the denominator. Report both raw completed ticks/s and quality-qualified ticks/s, p50/p95/p99/worst-case step time, max error-to-bound, constraint/complementarity residuals, memory, iterations and scoped energy-per-qualified-tick. Keep CPU/GPU, single-world, batch and connected-island Pareto frontiers separate unless the hardware and semantics are deliberately matched.

### Contact residual reference-scale correction

For active contacts, the attained normal velocity is intentionally near the target and can be approximately zero. Normalizing slack only by the attained or target speed is therefore ill-conditioned: a roundoff-sized negative slack divided by a roundoff-sized attained velocity can appear to be a 100% violation. The primal and complementarity residuals now include the pre-contact free normal velocity `J_i v_free` in their row-local characteristic speed. This preserves a meaningful scale when contact resolution removes most of the incoming normal speed. A regression uses a `-9e-17` residual against a `2 m/s` incoming free normal velocity and requires both dimensionless residuals below `1e-15`.

## Independent energy/work ledger replay

The contact reference now exposes `audit_discrete_energy_ledger`. This is a separate audit path from the solver's ledger-construction helper. Given the frozen `DiscreteStepInput` and accepted `DiscreteStepResult`, it:

- independently derives the next position as `q + h v_next` and checks the returned position;
- independently derives each normal velocity as `J_i v_next`, the unilateral target as `-gap_i/h`, and slack as their difference;
- reconstructs kinetic/potential changes, external force work, damping dissipation, mass/stiffness increment terms, contact impulse work and the contact-target/complementarity decomposition;
- compares all 15 published ledger fields under an explicit absolute/relative audit profile;
- rejects non-finite intermediates, mismatched shapes, altered position/slack and any ledger-field mismatch.

The audit receipt is a replay record, not a formal proof: it uses f64 arithmetic and a declared tolerance profile. It does not certify the solver's contact complementarity, KKT accuracy, physical-model fit, frictional dissipation or time-discretization error. Those remain separate evidence obligations.

### Canonical replay artifact bytes

`canonical_energy_audit_payload` emits a deterministic versioned byte payload only after the independent audit succeeds. It binds:

- the exact input matrices, vectors, contact Jacobian/gaps and timestep;
- the accepted next velocity/position, contact impulse and independently checked slack;
- all 15 published energy/work fields;
- the audit profile and the replayed receipt/check counts.

The payload deliberately excludes the solver's effective-metric/free-velocity summaries, active-set labels and normalized solver residuals: those are separate evidence obligations and are not certified by the energy-ledger replay.

Encoding is fixed by the `LUMINOUS-ENERGY-AUDIT-V0.1` magic/version prefix, little-endian u64 lengths, row-major matrices, and exact IEEE-754 f64 bit patterns. The returned bytes can be passed to an approved SHA-256 implementation and the digest placed in the EnergyWorkLedger evidence receipt. The payload deliberately does not replace the higher-level immutable binding for model revision, fixture identity, units, coordinate frames or task semantics; those remain independently bound by the evidence composer.

New regressions require byte-identical replay for identical exact inputs, different bytes after an external-force mutation, and refusal to serialize a tampered published ledger.
