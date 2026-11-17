# Symmetry and coordinate-geometry industry benchmark protocol v0.1

Status: benchmark protocol and fixture requirements. No external-engine performance comparison has yet been executed.
Reference date: 2026-11-16.

## Four independent lanes

A symmetry-discovery model, a multibody dynamics engine, and a manifold optimizer solve different problems. Report four independent lanes: discovery; mathematical qualification; physical step-map qualification; and full scientific-task equivalence. A result in an earlier lane never grants authority in a later lane.

## Verified comparison set

### Symmetry discovery

- [LieFlow — Discovering Symmetry Groups with Flow Matching](https://proceedings.mlr.press/v306/chen26dg.html), ICML 2026 / PMLR 306. Learns a distribution over a hypothesis Lie group and supports continuous and discrete subgroup discovery. Direct discovery baseline for point-cloud and motion subgroup tasks.
- [MLSD — Machine learning symmetry discovery for integrable Hamiltonian dynamics](https://journals.aps.org/pre/abstract/10.1103/jl75-7ynx), Physical Review E 113, 025302 (2026). Demonstrations are for integrable Hamiltonian systems, including Kepler and isotropic-oscillator examples; do not extrapolate to chaotic or stochastic systems.
- [LieStoNet](https://proceedings.mlr.press/v306/liu26aq.html), ICML 2026 / PMLR 306. Discovers Lie-point symmetries for stochastic differential equations from spatiotemporal data, with separate drift/diffusion determining conditions.
- [SymFP](https://arxiv.org/abs/2611.07410), submitted 2026-11-05. Uses singular-value spectra of equivariant representation blocks as symmetry fingerprints. This is a fresh preprint, not yet independently reproduced in this programme.

Matched discovery measures: subgroup precision/recall, group-order error, generator subspace angle, rank/closure/Jacobi residual, false-positive rate with confidence intervals, symmetry-breaking localization error, training/search cost, and proposal abstention rate. Do not compare these papers as though they shared a benchmark task.

### Physics and geometry

- [MuJoCo 4.0.1](https://mujoco.readthedocs.io/en/latest/changelog.html), released 2026-11-11. Pin 4.0.1 rather than 4.0.0: the 4.0.1 notes include a correction to a sparse Cholesky/Newton-solver defect that could produce wrong accelerations or divergence for sparse constraint Jacobians. Engine patch version is part of the benchmark receipt.
- [Drake 1.57.0](https://drake.mit.edu/release_notes/release_notes.html), released 2026-09-10. Its multibody equations distinguish qdot = N(q) v and M(q) vdot + C(q,v)v = tau; fixtures must map generalized positions and generalized velocities correctly rather than assuming qdot = v.
- [Pinocchio 4.1.0 documentation](https://gepetto.github.io/doc/pinocchio/doxygen-html/). Mature articulated rigid-body dynamics and analytical derivatives; use its manifold/tangent conventions in the adapter.
- [Project Chrono 10.0.0](https://projectchrono.org/news/), released 2026-03-27. Includes updated Jacobian strategies for implicit integrators and broad multiphysics capabilities.
- [Ceres Manifold interface](https://ceres-solver.readthedocs.io/latest/nnls_modeling.html). Use Plus/Minus and ambient-versus-tangent Jacobian semantics as a manifold-geometry reference, not as a multibody simulator.

These are versions verified when the protocol was written. Every actual run records executable version, source commit/tag, compiler, build flags, solver settings, precision and host. Latest is not a reproducible identifier.

## Frozen physical fixtures

### A — 3D constrained dumbbell

Use the already frozen unequal-mass, off-axis, active-reaction case from generalized-coordinate covariance #4730. Preserve masses, initial positions and velocities, applied force as a covector, holonomic constraint and Jacobian, timestep and integrator, solver tolerances and iteration limits, physical observables and output rounding policy.

Solve the same input in canonical and dense generalized coordinates. The transformed run must use M_y = S^T M_q S, Q_y = S^T Q_q and J_y = J_q S; lift only after the transformed solve.

### B — six-degree-of-freedom dense metric

Use the dependency-free 6D upper-unitriangular basis and diagonal positive mass metric in this reference crate. Compare optimized two-stage congruence, independent four-index congruence oracle, Cholesky admission receipt, tangent and cotangent induced norms, and lifted physical results. This is a small-matrix correctness/scaling test, not an industry-engine speed comparison.

### C — constrained Delassus operator

For full-row-rank holonomic J, compare two independent constructions: (1) whitened Gram, Y = J L^-T and W = Y Y^T, where M = L L^T; (2) direct reference, solve M x_j = J_j^T then compute W_ij = J_i x_j. Also test multiplier-to-correction and residual-to-minimum-kinetic-correction identities. Reject redundant rows as singular. Rebase generalized coordinates while preserving physical constraint meaning.

### D — articulated engine adapter

After A–C are stable, map a small articulated rigid-body model into Drake, MuJoCo, Pinocchio and Chrono. Do not force every engine to accept an arbitrary dense basis internally; compare the same physical model, map each engine's reported quantities into a declared common tangent/cotangent frame, and apply the dense-rebasing test in the reference layer.

Do not mix contact/friction engines into the first constrained-holonomic benchmark. Add contact only after geometry, friction law, complementarity formulation, restitution, integrator and solver tolerances are matched.

## Measures

Record coordinate basis and transform condition estimates; mass-metric symmetry defect and minimum Cholesky pivot; tangent error in the mass metric and covector residual in the dual metric; Euclidean/infinity component norms beside—not instead of—metric norms; Delassus symmetry/positive-definiteness and constraint residual; momentum one-form pairing; lifted physical error, constraint violation, energy/work and momentum defects; solver convergence, iterations and explicit abstention; wall-clock p50/p95/p99, memory and energy when measurable; certificate generation/replay time and byte size.

Compare every accepted solution against an independent higher-precision or analytic oracle. A small residual alone is not a forward-error certificate. A vacuous condition/error bound must return INCONCLUSIVE.

## Proposed targets (not achieved)

- 100% exact match on finite group/action/orbit fixtures with exhaustive oracle coverage.
- Zero false hard-prune decisions in the frozen finite quotient corpus.
- Zero false physical ACCEPT decisions on wrong variance, wrong metric congruence, wrong Jacobian, singular, non-finite and condition-vacuous controls.
- 100% explicit abstention on singular, malformed, unsupported or declared out-of-envelope inputs.
- Every admitted numerical case covered by its independent high-precision forward-error reference.
- At least 1,000 deterministic seeded invertible coordinate transforms across logarithmic condition tiers, using separate calibration and held-out transform families.
- At least 100 non-isomorphic finite group actions, including non-abelian groups and non-free actions with varying stabilizers.
- At least 2x verified search-cost reduction on one substantial scientific-search task, only after full task invariance and lift verification.
- Publish qualified-work-per-compute, not raw throughput: successful independently verified cases per core-second, with failures and abstentions included and coverage disclosed.
- Include replay overhead and certificate size; no speed claim until independent verifier cost is included.

These are target thresholds for a future benchmark release, not measurements achieved by this crate.

## How to outperform responsibly

Do not attempt to replace MuJoCo, Drake, Pinocchio or Chrono as general-purpose simulators. Their mature strengths include articulated-body algorithms, contact handling, multibody mechanics, and specialized integration/optimization methods.

The opportunity is a capability they are not designed to provide as one integrated guarantee: specialized learners may propose symmetries; an independent checker verifies mathematical structure; a receiving physics verifier checks coordinate and step-map obligations; the scientific task compiler checks objective, constraints and answer semantics before pruning; every quotient member has a replayable witness; and each stage can abstain when evidence is insufficient.

The defensible competitive claim is more qualified useful work per unit compute at a disclosed false-acceptance risk—not a faster simulator or better symmetry discovery without matched evidence.