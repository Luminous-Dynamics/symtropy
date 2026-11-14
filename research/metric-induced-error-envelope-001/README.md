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

## Independent constraint-equation basis covariance

The generalized-coordinate rebase is only one representation freedom. The same physical constraint manifold can be written using any nonsingular linear combination of its constraint equations.

For a constraint-basis transform (T):

[
r_{new}=T r_{old},qquad J_{new}=T J_{old},
]
[
lambda_{new}=T^{-T}lambda_{old},qquad
W_{new}=T W_{old}T^T.
]

The multiplier and residual are dual objects: their pairing (lambda^T r) is invariant. The Delassus-induced dual norms are also invariant:

[
|lambda|_{W}^2=lambda^T Wlambda,qquad
|r|_{W^{-1}}^2=r^T W^{-1}r.
]

The implementation adds `ConstraintEquationRebasisV1`, with separate residual pushforward, multiplier inverse-transpose pullback, and Jacobian left-action APIs. It does not conflate a constraint-equation change with generalized-coordinate rebasing.

The test suite now checks:
- the Delassus congruence (W_{new}=T W_{old}T^T);
- multiplier norm, residual norm and dual-pairing invariance;
- generalized constraint-force and induced velocity-correction invariance;
- minimum-kinetic-norm correction invariance;
- a deterministic 17-shear constraint-basis family;
- a 3×3 grid combining independent generalized-coordinate and constraint-equation rebases;
- wrong multiplier/residual variance and singular constraint-transform negative controls.

A separate row-scaling ladder covers diagonal constraint-basis changes from (10^{-2}) through (10^2). It reports the coordinate condition estimate and raw Jacobian component growth beside invariant Delassus norms, pairing, congruence and minimum correction. A near-singular row scaling is separately tested for profile-based abstention. Scaling is not hidden by restricting the study to determinant-one shears.

A deterministic seven-point report is available:

```sh
cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example constraint_basis_ladder
```

This remains a bounded full-row-rank linear holonomic reference. It does not qualify redundant-row elimination, unilateral/frictional contact, nonlinear constraint-manifold reparameterization, or production solvers.

## Exact-rational two-axis covariance oracle

In addition to floating-point tolerance tests, the constraint-basis branch now includes a dependency-free test-only rational oracle at `src/exact_constraint_oracle.rs`.

It constructs an exact integer mass metric, constraint Jacobian, dense unimodular generalized-coordinate transform (S), and independent constraint-equation transform (T). All derived quantities are represented as normalized integer numerator/denominator pairs. The oracle checks exact equality for:

- (M_y=S^T M_q S) and (J_y=T J_q S);
- (W_y=T W_q T^T);
- dual pairing (lambda^T r);
- Delassus multiplier and residual norms;
- generalized constraint-force covariance;
- induced physical acceleration covariance;
- minimum-kinetic correction covariance and the equality between its kinetic norm and the Delassus residual norm.

This fixture does not establish correctness for arbitrary dimensions or nonlinear manifolds; it provides an exact, independently implemented algebraic reference for one nontrivial 3-DOF / 2-constraint case. The floating-point shear ladders remain necessary for numerical conditioning and rounding behaviour.

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
