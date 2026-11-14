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

## Constraint-space extension: Delassus metric

The crate now includes a separate typed constraint-space layer. Given an SPD mass metric M = L L^T and full-row-rank constraint Jacobian J, it whitens each constraint row:

Y = J L^-T

and builds the Delassus operator as the row Gram matrix:

W = Y Y^T = J M^-1 J^T.

The implementation uses triangular solves only and does not form M^-1. It retains Y as an explicit witness, reconstructs W from Y Y^T, and independently compares that result with the direct Riesz-solve formulation in the test suite. The exact diagonal of W is therefore available as the row-wise squared norm of Y.

For constraint multiplier lambda and constraint residual r, the layer uses the induced dual pair:

- multiplier norm: ||lambda||_W = sqrt(lambda^T W lambda);
- residual norm: ||r||_(W^-1) = sqrt(r^T W^-1 r).

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

## Rank-deficient constraint quotient

The companion module `src/rank_deficient_constraint.rs` covers redundant linear holonomic rows without adding an arbitrary diagonal regularizer.

For a positive-definite mass metric and possibly rank-deficient Jacobian, the Delassus map `W = J M^-1 J^T` is positive semidefinite. Multipliers are equivalent modulo `ker(J^T)`; the induced generalized force and minimum-kinetic correction remain unique. The quotient multiplier norm is `sqrt(lambda^T W lambda)`.

A compatible residual lies in `range(J)=range(W)). The solver returns a quotient representative satisfying `W lambda = r`, and its dual norm is `sqrt(lambda^T r)`. Residuals outside the range are explicitly returned as INCOMPATIBLE or INCONCLUSIVE; they are never silently projected and treated as solved.

**Compatibility tolerances are not measured in raw Euclidean coordinates.** The caller supplies an SPD residual-space metric H. When rows are re-described by `r_new = T r_old`, that metric must be re-described as `H_new = T^-T H_old T^-1`. The same H metric drives rank revelation and residual compatibility. This is what makes the floating-point acceptance rule meaningful under arbitrary nonsingular row scaling/shearing.

The rank is still numerical and profile-bound. Near-threshold cases are not promoted to PASS. The hard-constraint quotient is a different semantic model from `W + epsilon I`; regularization must be declared as a soft-constraint problem with a separate identity.

Run the synthetic fixture:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example redundant_constraint_quotient

The unit tests cover duplicate rows, multiplier null shifts, unique physical correction, incompatible residual rejection, a non-orthogonal constraint-equation re-description with the matching H metric, and rank-zero constraints. This does not yet qualify nonlinear constraints, unilateral/frictional contact, active-set changes, or sparse articulated solvers.
