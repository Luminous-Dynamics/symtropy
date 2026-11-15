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

## Operator conditioning extension

The stacked operator track in [PHYS-METRIC-OPERATOR-CONDITION-001 #4818](https://github.com/Luminous-Dynamics/symtropy/issues/4818) adds separate frame-typed maps:

- `TangentToTangentOperator`: whitened matrix (B=L_Y^T A L_X^{-T});
- `TangentToCotangentOperator`: whitened matrix (B=L_Y^{-1} A L_X^{-T}).

A one-sided Jacobi SVD estimates the singular spectrum without forming (B^T B). Rank-deficient maps receive no finite condition estimate; maps beyond the declared condition envelope are marked separately. The mass Riesz map (M:T\to T^*), paired with the same metric on both sides, should whiten to identity and have condition 1. This is a check of the Riesz isomorphism, not a claim that arbitrary physical operators are well-conditioned.

The initial extension covers square dense operators only. It is not a formal interval enclosure and does not yet qualify rectangular constraint maps or nonlinear residual-to-state error.

## Rectangular operators and the constraint coisometry

The operator module now also supports frame-typed rectangular maps. A rectangular receipt reports the nonzero-space singular spectrum, maximum possible rank, domain nullity, codomain cokernel dimension and a condition estimate only when the map reaches full possible rank. A full-row-rank map from an N-dimensional tangent space to a K-dimensional constraint space can therefore have a legitimate K-value singular spectrum and an N-K-dimensional kernel; that kernel is not misreported as rank deficiency.

For a full-row-rank constraint Jacobian J and mass metric M, define the Delassus cometric W = J M^-1 J^T and the constraint residual metric G_C = W^-1. The correctly whitened Jacobian is:

`B = L_C^T J L_M^-T`, with `G_C = L_C L_C^T`.

Then `B B^T = L_C^T W L_C = I_K`. The Jacobian is a metric coisometry: it has K singular values equal to one, condition one on its nonzero singular subspace, domain nullity N-K, and zero codomain cokernel. This is a stronger algebraic regression than checking that a generic SVD returns plausible numbers.

The new tests cover:
- a 2-by-3 full-row-rank Delassus-whitened Jacobian with singular spectrum (1,1);
- invariance of that coisometry under a dense generalized-coordinate re-basing;
- a rank-deficient 2-by-3 map whose kernel and cokernel are separately reported and which receives no finite condition claim;
- the transposed Jacobi path for a 3-by-2 map;
- a rectangular tangent-to-cotangent map using the codomain dual metric.

This remains a bounded small-dense f64 qualification. It does not prove all constraint systems are coisometries: full row rank, the exact Delassus metric, and the correct variance laws are required. It does not qualify redundant constraint elimination, contact/friction or sparse articulated solvers.

## Scope and next step

v0.1 is a small dense linear-algebra reference. It does not yet implement a backward/forward error envelope for a general operator, the constrained Delassus solve, a nonlinear midpoint solver, arbitrary tangent bundles/charts, singular chart transitions, contact/friction, sparse articulated systems or production solver claims.

The next step is to connect these frame-typed norms to the mass solve M a = f, constraint-space/Delassus solves, and the physical one-step dumbbell fixture in Symtropy #4730. Every residual must identify its domain/codomain and tensor variance; raw coordinate norms remain diagnostic only.

## Reproducible microbenchmark v0.1

The `metric-induced-microbenchmark` binary uses one frozen 6-DOF diagonal-mass / dense unit-triangular rebasing fixture. It reports per-operation median, p95 and p99 nanoseconds plus a numerical checksum for:

- raw Euclidean and infinity component norms;
- tangent and dual-cotangent metric norms in both coordinate frames;
- tangent pushforward and covector pullback;
- dense metric rebasing and metric admission / Cholesky factorization;
- the combined metric/dual norm path.

Run it with:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --bin metric-induced-microbenchmark -- benchmark.json

The JSON records the fixture ID, crate version, target architecture/OS, available parallelism, iteration count, sample count and raw timing summary. Record the CPU model, OS image, compiler commit and machine configuration alongside each retained report; the program intentionally does not guess those facts from portable runtime APIs.

These timings are **only the reference-layer microbenchmark**. They are not comparable to Drake, MuJoCo or Pinocchio solver timings, do not establish a general overhead bound, and do not measure physical integration. Matched solver comparisons are tracked separately in Symtropy #4805.
