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

## Operator conditioning extension

The stacked operator track in [PHYS-METRIC-OPERATOR-CONDITION-001 #4818](https://github.com/Luminous-Dynamics/symtropy/issues/4818) adds separate frame-typed maps:

- `TangentToTangentOperator`: whitened matrix (B=L_Y^T A L_X^{-T});
- `TangentToCotangentOperator`: whitened matrix (B=L_Y^{-1} A L_X^{-T}).

A one-sided Jacobi SVD estimates the singular spectrum without forming (B^T B). Rank-deficient maps receive no finite condition estimate; maps beyond the declared condition envelope are marked separately. The mass Riesz map (M:T\to T^*), paired with the same metric on both sides, should whiten to identity and have condition 1. This is a check of the Riesz isomorphism, not a claim that arbitrary physical operators are well-conditioned.

The operator module now supports square and rectangular dense maps in both tangent-to-tangent and tangent-to-cotangent variance classes. The one-sided Jacobi spectrum is a numerical estimate, not a formal interval enclosure, and the current reference does not qualify nonlinear residual-to-state maps.

## Rectangular operators and rank semantics

The stacked rectangular extension in [PR #4866](https://github.com/Luminous-Dynamics/symtropy/pull/4866) adds frame-typed K-by-N tangent-to-tangent and tangent-to-cotangent maps using the same metric-aware whitening laws as the square operators.

A rectangular receipt keeps distinct:
- numerical rank and maximum possible rank;
- domain nullity and codomain nullity;
- the ordinary whole-map condition estimate, defined only for injective/full-column-rank maps;
- the nonzero-spectrum condition estimate, explicitly a condition estimate on the resolved range rather than the whole non-injective map;
- admitted, outside-envelope, rank-deficient and non-injective statuses.

For example, a full-row-rank 2-by-3 constraint map is surjective but not injective: its domain nullity is one, the ordinary whole-map condition is unavailable, and the nonzero-spectrum condition can still be finite. It must not be reported as a finite condition of the full map.

The exact-head test suite covers 2-by-3 full-row-rank and rank-deficient maps, a 3-by-2 full-column-rank map, dense domain/codomain rebasing for both tensor variances, wrong-variance controls, and non-finite/zero-dimensional rejection. These are small dense f64 numerical estimates, not interval enclosures.

The next stronger physical regression is the Delassus coisometry: for a full-row-rank constraint Jacobian J, mass metric M and constraint metric G_C=(J M^-1 J^T)^-1, the whitened Jacobian should satisfy B B^T=I. This tests the physical constraint metric, not merely a generic rectangular SVD.


## Delassus-whitened constraint Jacobian: a metric coisometry

For a full-row-rank constraint Jacobian J: T_q -> C and SPD mass metric M, the Delassus cometric is W = J M^-1 J^T. Treat the constraint residual as a vector with metric G_C = W^-1. If M=L_M L_M^T and G_C=L_C L_C^T, the correctly whitened Jacobian is:

`B = L_C^T J L_M^-T`.

Then:

`B B^T = L_C^T J M^-1 J^T L_C = L_C^T W L_C = I_K`.

Thus a full-row-rank K-by-N constraint Jacobian is a metric coisometry. Its K nonzero singular values are one (condition one on the resolved range), its domain nullity is N-K, and its codomain nullity is zero. The ordinary whole-domain condition estimate remains absent for this non-injective map; do not confuse the range-restricted condition with the full-map condition.

The new tests use the real `DelassusMetricV1` implementation and verify:
- a 2-by-3 full-row-rank Jacobian has singular values (1,1);
- the same coisometry survives dense generalized-coordinate rebasing S;
- it also survives an independent constraint-equation transformation T, with J_new=T J_old and W_new=T W_old T^T;
- a 3-by-3 grid of state shears and constraint shears/row scales retains the unit nonzero spectrum;
- reusing the old W^-1 after changing the constraint-equation basis is a negative control and does not preserve the unit spectrum.

This is an algebraic identity in exact arithmetic. The Rust tests numerically estimate the singular spectrum under the pinned f64/Jacobi profile; they do not provide a formal interval proof, qualify rank-deficient/redundant constraints, or establish equivalence for unilateral/contact/friction constraints.

## Scope and next step

v0.1 is a small dense linear-algebra reference. It does not yet implement a general operator backward/forward error envelope, a nonlinear midpoint solver, arbitrary tangent bundles/charts, singular chart transitions, unilateral/frictional contact, sparse articulated systems or production solver claims.

The next step is integration with the physical one-step dumbbell fixture #4730 and matched industry comparison #4805, including calibrated numerical error envelopes for the mass and Delassus solves. Every residual must identify its domain/codomain, metric and tensor variance; raw coordinate norms remain diagnostic only.

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

## Rectangular constraint-space / Delassus integration

The `constraint_space_receipt` module composes the metric-aware rectangular Jacobian spectrum with the mechanical Delassus operator. It binds the generalized-velocity mass metric M, the independent constraint-rate metric C, and the Jacobian used to construct W = J M^-1 J^T.

For B = L_C^T J L_M^-T, the whitened Delassus identity is B B^T = L_C^T W L_C. The receipt records the rectangular singular values and their squares as eigenvalue estimates of the constraint-metric-whitened Delassus operator. The Delassus cometric is constructed through Cholesky whitening; M inverse is never formed.

Two independent coordinate changes are tested: generalized coordinates q = S y and constraint-rate coordinates c = T d. Their represented quantities obey J_d = T^-1 J_q S, M_y = S^T M_q S, and C_d = T^T C_c T. The Delassus matrix transforms as W_d = T^-1 W_q T^-T.

The full-row-rank path exposes multiplier, residual-dual and minimum-kinetic-correction operations. Rank-deficient and overdetermined maps retain rank/nullity receipts but receive no SPD multiplier-solve authority. Redundant-constraint multiplier equivalence remains the separate quotient-space programme.

### Deterministic constraint-space ladder

`examples/constraint_space_ladder.rs` applies five simultaneous domain and constraint-coordinate rebases. It checks rectangular singular values, whitened Delassus eigenvalue estimates, Delassus matrix congruence, constraint-rate and covector norms, virtual-work pairing, residual dual norms, multiplier covariance, and the lifted minimum-kinetic physical correction.

Run with:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example constraint_space_ladder

The exact-head workflow validates and uploads the five-row CSV receipt. This is a bounded dense mechanics regression, not a qualification of sparse articulated solvers, redundant-constraint quotient algorithms, nonsmooth contact/friction, or production throughput.
