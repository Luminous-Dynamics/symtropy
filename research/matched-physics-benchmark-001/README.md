# Matched physics benchmark contract v0.1

This is the first executable contract slice of [PHYS-METRIC-BENCHMARK-001 #4805](https://github.com/Luminous-Dynamics/symtropy/issues/4805).

## Core principle: comparability is a proof obligation

A benchmark result is not comparable merely because two programs print a number with the same name. Before performance or accuracy can be compared, the adapter must establish that it solved the same case:

- the same physical model and degrees of freedom;
- the same generalized-coordinate and velocity convention;
- the same units, mass/inertia and force-covector semantics;
- the same constraints, initial conditions and boundary conditions;
- the same integration method, timestep, tolerances and solver stopping policy;
- the same observable and output mapping;
- the same frozen input/case digest.

If any required mapping is missing, stale or inconclusive, the result is `NOT_COMPARABLE`. Do not convert an unmatched model into a lower score, a failed score, or evidence of solver superiority.

## Frozen dense re-basing case

`fixtures/dense-rebase-2d-case-v0.1.json` freezes a 2D dense non-orthogonal re-basing:

`q = S y`, `S = [[1, 2], [0, 1]]`, `M_q = diag(2, 3)`, `v_q = [5, 2]`, and `alpha_q = [5, 7]`.

Exact expected values include `v_y = [1, 2]`, `alpha_y = [5, 17]`, `M_y = [[2, 4], [4, 11]]`, canonical pairing 39, tangent metric norm squared 62 and dual cotangent norm squared 173/6. Raw coordinate infinity norms differ (5 versus 2 for the tangent), which is why component norms remain diagnostics rather than physical magnitudes.

The case payload SHA-256 is calculated from canonical JSON: UTF-8, sorted object keys, no insignificant whitespace, non-ASCII preserved. The workflow recomputes that digest and independently checks the algebraic identities.

The case points to the frozen 3D dumbbell physical fixture from [Symtropy #4730](https://github.com/Luminous-Dynamics/symtropy/pull/4730) as the next tier. It does not claim the 2D dense kernel is a physical multibody model.

## Result contract

`schemas/benchmark-result-v0.1.schema.json` requires a semantic-mapping receipt separate from numerical and performance metrics. A result can be:

- `NOT_COMPARABLE`: mapping is missing, mismatched or inconclusive; no comparative claim is allowed;
- `MATCHED`: all required semantic mapping fields pass, the result is completed, and the case digest matches exactly. This only permits a bounded matched-fixture comparison.

V0 fixes `general_sota_superiority=false` and `production_authority=false`. A matched result can support only `MATCHED_FIXTURE_ONLY`, not a general SOTA statement.

The sample result in `fixtures/synthetic-not-run-result-v0.1.json` is deliberately synthetic and `NOT_RUN`; it is a schema contract test, not benchmark evidence.

## Industry comparators and honest boundaries

- **Drake** provides recursive inverse dynamics and the Composite Body Algorithm for mass-matrix construction; its documentation describes the mass-matrix routine as O(n²) and cautions against explicit formation when avoidable. Its generalized-velocity, force and constraint conventions must be mapped exactly before a score is comparable. See [Drake MultibodyPlant](https://drake.mit.edu/pydrake/pydrake.multibody.plant.html).
- **MuJoCo** formulates constraint forces through primal/dual optimization and offers Newton, conjugate-gradient and projected Gauss-Seidel solvers, including dense and sparse representations. The model, cone type, constraint softness, solver and tolerance must be frozen. See [MuJoCo computation](https://mujoco.readthedocs.io/en/latest/computation/).
- **Pinocchio** provides RNEA, CRBA and ABA, plus constrained forward dynamics and impulse dynamics. Coordinate and joint/speed mappings must be explicit. See [Pinocchio dynamics](https://gepettoweb.laas.fr/doc/stack-of-tasks/pinocchio/master/doxygen-html/md_doc_a-features_g-dynamic.html).
- **Project Chrono** may be a later contact/constraint baseline, but its NSC and SMC contact formulations are different model semantics and must never be mixed in one matched row. See [Chrono contact formulations](https://api.projectchrono.org/collisions.html).

These libraries are not direct competitors to the small dense metric-reference microbenchmark. The proper comparison is layered: exact kernel baselines first, then matched physical-step adapters, then larger articulated/contact workloads only when the semantic mapping is auditable.

## Benchmark tiers

1. **Dense kernels:** raw coordinate norms, metric/dual norms, Riesz maps, LU/Cholesky/QR and independent high-precision reference.
2. **Coordinate ladder:** orthogonal transformations, dense unimodular shears, mixed scaling/shear and near-singular controls, with raw component diagnostics shown beside metric-induced invariants.
3. **Constrained mechanics:** begin with the frozen one-link/two-point-mass dumbbell in #4730, then planar and spatial chains. Compare physical one-step outputs, not only internal solver residuals.
4. **Contact and friction:** only after the same contact model, cone geometry, compliance/rigidity, friction law and timestep are mapped.

## Metrics to report together

- **Geometry:** tangent/cotangent norm drift, canonical pairing drift, metric congruence defect, constraint-Jacobian transformation defect.
- **Numerics:** normwise and componentwise backward error, condition estimate, high-precision forward error, bound coverage, rank/abstention status.
- **Physics:** lifted position/velocity/momentum errors, constraint violation, energy/work defect and discrete-step equivariance.
- **Decision quality:** false ACCEPT, false REJECT, INCONCLUSIVE coverage and condition-tier risk/coverage curves.
- **Systems:** factorization and solve time, evidence-layer overhead, allocations, memory, throughput, median/p95/p99, warm/cold separation.

Every measured result should bind source commit, fixture digest, binary/adapter digest, solver version/options, compiler/runtime, floating-point mode, hardware, thread count, seeds, warmups and repetitions.

## How to beat the field without misleading ourselves

Do not try to replace Drake's recursive algorithms, MuJoCo's contact optimization, or Pinocchio's articulated-body algorithms with a tiny dense reference. The potential differentiator is an independent evidence layer around those solvers: it verifies tensor variance, coordinate covariance, residual meaning, condition envelopes, physical observables and exact task semantics, and it can abstain when the evidence is insufficient.

Proposed targets (not results): zero false matched claims on deliberately mismapped adapters; 100% detection of stale fixture digests; complete high-precision coverage on admitted dense cases; zero finite condition claims for rank-deficient controls; and less than 5% median / 10% p99 evidence-only overhead after a same-solver baseline is established. Report every miss and abstention; do not optimize them away.

## Current state

- The Symtropy metric reference [#4797](https://github.com/Luminous-Dynamics/symtropy/pull/4797) has a passing exact-head Rust 1.96 qualification and retained reference-layer microbenchmark artifact.
- The metric-induced operator spectrum work is [#4820](https://github.com/Luminous-Dynamics/symtropy/pull/4820); its strict Clippy issue is being repaired separately.
- This PR provides the first case/result schemas, canonical payload digest check, synthetic non-comparable result and fail-closed mapping controls.
- No Drake, MuJoCo, Pinocchio or Chrono measurement is claimed yet.