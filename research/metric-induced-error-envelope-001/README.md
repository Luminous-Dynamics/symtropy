# Metric-induced coordinate-invariant error reference v0.1

This is the first executable slice of [PHYS-METRIC-ERROR-001 #4788](https://github.com/Luminous-Dynamics/symtropy/issues/4788).

## Paradigm shift

Coordinate components are not physical magnitudes. Under a dense non-orthogonal re-basing, Euclidean and infinity norms of the component arrays generally change even when the physical vector or covector is the same.

This reference gives tangent vectors and cotangent covectors distinct frame-typed Rust representations and uses the mass metric and its dual to measure them:

||v||_M = sqrt(v^T M v), and ||alpha||_(M^-1) = sqrt(alpha^T M^-1 alpha).

For q = S y the correct transformation laws are:
- tangent: v_y = S^-1 v_q;
- covector: alpha_y = S^T alpha_q;
- metric: M_y = S^T M_q S.

Therefore both metric-induced norms and the canonical pairing alpha(v) are invariant under the declared coordinate change, up to floating-point rounding and the declared admission profile.

## The Riesz identity

For an admitted symmetric positive-definite mass metric, the Riesz map M:T->T* identifies tangent and dual norms:

||a||_M = ||Ma||_(M^-1).

This is a structural identity for the mass Riesz map. It does not say that arbitrary residuals, arbitrary operators or nonlinear solver errors have the same tangent and cotangent magnitudes. The residual must first be typed in the correct dual space.

## Numerical method and admission

- Dense coordinate transforms use partial-pivoted LU with a scale-relative pivot rejection threshold.
- Metrics are admitted only after finite-value, symmetry and Cholesky positive-definiteness checks.
- Small admitted symmetry defects are canonicalized once and recorded in MetricAdmissionReceiptV1.
- Tangent norms use M=LL^T and evaluate ||L^T v||_2.
- Cotangent norms evaluate ||L^-1 alpha||_2, avoiding an explicit inverse.
- The reference does not form a matrix inverse in its norm paths.

These are floating-point numerical checks, not formally verified bounds. The Cholesky pivot threshold is a declared profile decision and must be calibrated against condition-number tiers before use in larger models.

## Tests

The dependency-free Rust test suite covers dense non-orthogonal rebasing invariance for tangent and cotangent norms; dual pairing and coordinate-map round trips; equality of tangent mass norm and the dual norm of its Riesz-lowered covector; a deterministic shear family with 17 parameter values; explicit changes in raw Euclidean/infinity component norms while the metric norm stays invariant; wrong tensor-variance and metric-congruence negative controls; singular transforms and invalid metrics; and explicit recording of admitted roundoff asymmetry.

## Operator conditioning follow-on

Square and rectangular metric-induced singular spectra and condition receipts are developed separately in [PHYS-METRIC-OPERATOR-CONDITION-001 #4818](https://github.com/Luminous-Dynamics/symtropy/issues/4818), implementation [#4820](https://github.com/Luminous-Dynamics/symtropy/pull/4820) and rectangular extension [#4866](https://github.com/Luminous-Dynamics/symtropy/pull/4866). That programme uses one-sided Jacobi singular values rather than the conservative Frobenius upper bound prototyped in an intermediate branch experiment. The operator PRs are the authoritative implementation path; this base reference remains focused on typed metric and Delassus norms.

## Constraint-space extension: Delassus metric

The crate now includes a separate typed constraint-space layer. For a full-row-rank constraint Jacobian J, the Delassus operator W = J M^-1 J^T maps constraint multiplier covectors to constraint residual vectors. Thus multipliers are measured with the cometric W, while residual vectors are measured with the dual metric W^-1. The two object types are distinct in the Rust API. Given an SPD mass metric M = L L^T, the implementation whitens each constraint row:

Y = J L^-T

and builds the Delassus operator as the row Gram matrix:

W = Y Y^T = J M^-1 J^T.

The implementation uses triangular solves only and does not form M^-1. It retains Y as an explicit witness, reconstructs W from Y Y^T, and independently compares that result with the direct Riesz-solve formulation in the test suite. The exact diagonal of W is therefore available as the row-wise squared norm of Y.

For constraint multiplier covector lambda and residual vector r, the layer uses the induced dual pair:

- multiplier cometric norm: ||lambda||_W = sqrt(lambda^T W lambda);
- residual metric norm: ||r||_(W^-1) = sqrt(r^T W^-1 r).

The mechanical identities exercised by the tests are:

- ||M^-1 J^T lambda||_M = ||lambda||_W;
- the minimum-kinetic-norm correction satisfying J delta_v = r has norm ||r||_(W^-1).

Under q = S y, the correct Jacobian law is J_y = J_q S and the Delassus operator is invariant: J_y M_y^-1 J_y^T = J_q M_q^-1 J_q^T. Redundant constraint rows are rejected because they make W singular; v0 does not silently regularize them.

This is deliberately compared with the same underlying whitened-Jacobian structure exposed by MuJoCo 3.9.0 as efc_Y = J M^-1/2, plus its diagexact constraint-space inertia diagonal. A Cholesky whitening factor is not necessarily the symmetric principal square root M^-1/2, but it produces the same Gram matrix W. The reference's contribution is frame/type separation, independent reconstruction, coordinate-invariance checks, admission receipts and explicit fail-closed behavior—not a claim to have invented Delassus whitening. See the [MuJoCo 3.9.0 changelog](https://mujoco.readthedocs.io/en/3.9.0/changelog.html) and [computation documentation](https://mujoco.readthedocs.io/en/latest/computation/).

A deterministic seven-point example is available at:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example delassus_coordinate_ladder

The example reports the coordinate-matrix condition estimate, Jacobian component growth, Delassus defect, multiplier/residual norms and minimum kinetic correction norms across dense rebases. It is a bounded algebraic reference, not a contact solver benchmark.

## Congruence complexity ladder

The optimized metric re-basing path computes two matrix products, O(N^3), instead of a four-index accumulation, O(N^4). The independent example compares both methods with the same SPD admission/Cholesky work at dimensions 6, 12, 24 and 48:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example congruence_scaling_ladder

This is an internal algorithmic scaling comparison, not a simulator or industry solver result. Retain the exact runner/compiler/CPU receipt before making a speed claim.

## Scope and next step

v0.1 is a small dense linear-algebra reference. It does not yet implement a general operator backward/forward error envelope, a nonlinear midpoint solver, arbitrary tangent bundles/charts, singular chart transitions, unilateral/frictional contact, sparse articulated systems or production solver claims.

The next step is to connect these frame-typed and constraint-space norms to the physical one-step dumbbell fixture in Symtropy #4730 and its conditioning-aware error envelope in #4765. Every residual must identify its domain/codomain and tensor variance; raw coordinate norms remain diagnostic only.

## Reproducible microbenchmark v0.1

The `metric-induced-microbenchmark` binary uses one frozen 6-DOF diagonal-mass / dense unit-triangular rebasing fixture. It reports per-operation median, p95 and p99 nanoseconds plus a numerical checksum for:

- raw Euclidean and infinity component norms;
- tangent and dual-cotangent metric norms in both coordinate frames;
- tangent pushforward and covector pullback;
- dense metric rebasing and metric admission / Cholesky factorization;
- the combined metric/dual norm path;
- whitened-Jacobian Delassus construction versus direct mass-Riesz solves;
- Delassus multiplier and residual dual norms.

Run it with:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --bin metric-induced-microbenchmark -- benchmark.json

The JSON records the fixture ID, crate version, target architecture/OS, available parallelism, iteration count, sample count and raw timing summary. Record the CPU model, OS image, compiler commit and machine configuration alongside each retained report; the program intentionally does not guess those facts from portable runtime APIs.

These timings are **only the reference-layer microbenchmark**. They are not comparable to Drake, MuJoCo or Pinocchio solver timings, do not establish a general overhead bound, and do not measure physical integration. Matched solver comparisons are tracked separately in Symtropy #4805.

## Industry comparison and benchmark strategy

This work should sit beside mature dynamics solvers as an **assurance and coordinate-covariance layer**, not attempt to replace their optimized multibody algorithms.

- MuJoCo formulates constrained acceleration using an inertia-weighted Gauss principle and solves primal/dual constraint-force problems. This is the closest conceptual physical baseline for the mass metric and its dual pairing: [MuJoCo computation documentation](https://mujoco.readthedocs.io/en/latest/computation.html).
- Drake exposes multibody equations, generalized-velocity mappings, mass-matrix computation via the Composite Body Algorithm, and inverse-dynamics alternatives. Its documentation warns that explicit mass-matrix formation is O(n²) and should be avoided when possible: [Drake MultibodyPlant](https://drake.mit.edu/pydrake/pydrake.multibody.plant.html).
- Pinocchio provides RNEA, CRBA, ABA, sparse Cholesky-based constrained forward dynamics, and derivatives of its principal algorithms. It is an appropriate articulated-body comparator, not a reference implementation of this typed assurance contract: [Pinocchio feature overview](https://gepettoweb.laas.fr/doc/stack-of-tasks/pinocchio/master/doxygen-html/md_doc_a_features_intro.html).
- Variational integrators provide a mature discrete-mechanics foundation, including a discrete Noether theorem and principled treatment of forces, dissipation, and constraints. The next physical benchmark should therefore test the actual discrete step map, not infer it from a continuous-time generator alone: [Marsden and West, Discrete Mechanics and Variational Integrators](https://doi.org/10.1017/S096249290100006X).

The differentiating contribution here is not a faster CRBA/ABA implementation. It is a typed, replayable contract that makes tangent/cotangent variance explicit, verifies metric congruence, and compares physical error measures across coordinate descriptions.

### Recommended benchmark tiers

1. **Algebraic contract:** random but seeded SPD metrics, nonsingular dense basis changes, tangent/cotangent round trips, dual pairing, metric congruence, and Riesz identities.
2. **Condition ladder:** increase the basis-change condition estimate over approximately 10¹, 10², 10³, 10⁴, 10⁵, and 10⁶ where the declared f64 profile still admits it. Record raw component norms, induced norms, metric asymmetry, Cholesky pivots, and abstentions.
3. **Tensor-variance adversaries:** deliberately transform a force as a tangent, transform the metric on the wrong side, or transform a constraint Jacobian with the wrong variance. Every mutation must be detected.
4. **Mechanical operator tier:** apply the norms to the same mass solve, Delassus/constraint solve, and midpoint residual used by the dense-coordinate dumbbell reference. Compare with an independent high-precision oracle.
5. **Discrete symmetry tier:** compare complete one-step maps, constraint reactions, momentum maps, work, and physical observables across transformed coordinates. Use a variational or other geometric integrator as a comparator where the physical model supports it.

### Proposed targets (not achieved)

- Zero false accepts on singular, non-finite, wrong-variance, wrong-congruence, and wrong-Jacobian controls.
- All admitted dense re-bases preserve induced tangent and dual-cotangent norms within preregistered, condition-aware tolerances.
- 100% explicit abstention for singular or out-of-declared-envelope cases.
- Report component-norm drift alongside induced-norm invariance; never hide representation sensitivity.
- Demonstrate identical physical step-map and constraint observables across coordinate descriptions before making any physical-equivalence claim.
- Publish runtime and memory against the solver baselines on matched models. No speed claim is made by this small dense reference.
