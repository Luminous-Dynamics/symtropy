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

## Fixed-linear constrained KKT residual certificate

For an SPD mass metric M and full-row-rank constraint Jacobian J, the fixed linear saddle-point system is

    [ M  J^T ] [a] = [f]
    [ J   0  ] [λ]   [b]

For a candidate state, independently reconstruct

    r_m = f - (M a_hat + J^T lambda_hat)
    r_c = b - J a_hat
    rho^2 = ||r_m||_(M^-1)^2 + ||r_c||_(W^-1)^2
    W = J M^-1 J^T

The exact-arithmetic inequalities implemented in constrained_kkt_error.rs are

- acceleration: ||a - a_hat||_M <= rho;
- multiplier: ||lambda - lambda_hat||_W <= sqrt(2) rho;
- combined state: sqrt(||a-a_hat||_M^2 + ||lambda-lambda_hat||_W^2) <= ((1+sqrt(5))/2) rho.

The derivation whitens the KKT system into [I B; B^T 0] with B^T B = I. The acceleration constant 1 is sharp on the constraint nullspace; the combined-state constant (1+sqrt(5))/2 is sharp on the two-dimensional constraint-parallel acceleration/multiplier block.

The implementation does not form M^-1 or W^-1. It recomputes both residual blocks from the retained metric/Jacobian and candidate state, then measures them using the mass and Delassus dual norms. Redundant constraints are rejected because W is not SPD.

The residual and all three bounds are checked under two independent representation changes:

- generalized-coordinate re-basing q = S y, with M_y = S^T M_q S, J_y = J_q S, force covectors pulled back by S^T, and accelerations transformed by S^-1;
- constraint-equation re-description r_new = T r_old, with J_new = T J_old, b_new = T b_old, and lambda_new = T^-T lambda_old.

The example runs a 5-by-4 combined S/T ladder. Constraint-space basis changes use a distinct Rust frame type, so old and rebased multiplier/residual coordinates cannot be accidentally passed to the same typed API.

A deterministic example reports residuals and bounds across five generalized-coordinate shears and four independent constraint-equation shears:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example constrained_kkt_residual_ladder

**Claim boundary:** this is a bound for a fixed linear KKT operator with an SPD mass metric and full-row-rank J. The f64 receipt numerically evaluates the exact-arithmetic inequality; it is not an interval-certified real-arithmetic enclosure. It does not automatically bound a nonlinear midpoint step, changing active sets, time-discretization error, redundant constraints, or contact/friction systems.

## Scope and next step

The reference now covers frame-typed dense metrics, full-row-rank linear Delassus constraints, metric-induced operator norms, and a fixed-linear constrained KKT residual-to-state bound. It still does not qualify a nonlinear midpoint residual-to-state map, arbitrary manifold charts, singular chart transitions, unilateral/frictional contact, sparse articulated production solvers, or general solver superiority.

The next step is to connect the fixed-linear certificate to a frozen physical one-step dumbbell fixture in Symtropy #4730. That requires a separate qualification of the nonlinear residual Jacobian / local inverse bound and a clear separation of solver error, constraint linearization error, and time-discretization error.


## Conditional nonlinear local-root envelope (scalar reference)

The next layer is a Newton-Kantorovich calculator for a nonlinear residual F(z)=0 at z0 with nonsingular A=F'(z0). It accepts two independently supplied upper-bound records:

- eta >= ||A^-1 F(z0)|| in a declared physical norm;
- gamma >= Lip(A^-1 F') on the entire candidate ball, in the same norm and for the same problem and linearization.

The calculator rejects mismatched problem, linearization or norm identities. For h=gamma*eta < 1/2 it computes

    r = 2 eta / (1 + sqrt(1 - 2 h))
    q = gamma r

using the cancellation-resistant radius form. It issues a **conditional** local-root receipt only when q is strictly below one and also within the configured contraction profile (default 0.95). The boundary h=1/2, missing/mismatched evidence identity, negative/non-finite bound and overflow are fail-closed or INCONCLUSIVE.

A deterministic boundary ladder is available at:

    cargo run --release --locked --manifest-path research/metric-induced-error-envelope-001/Cargo.toml --example newton_kantorovich_ladder

The word *conditional* is essential. The calculator checks the scalar theorem once eta and gamma are supplied; it does not authenticate either receipt or prove those bounds. In particular, a sampled maximum of Jacobian variation is not a valid gamma bound over the complete ball. A physical local-root claim requires a separate independent verifier to establish the preconditioned Newton correction, Jacobian nonsingularity, uniform Jacobian-Lipschitz bound, norm identity, problem identity and full-ball domain. This v0.1 calculator therefore does not yet certify the dumbbell midpoint integrator or its time-discretization error.

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
