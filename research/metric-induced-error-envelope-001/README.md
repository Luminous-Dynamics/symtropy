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


## Nonlinear step-map and adjoint observable sensitivity

The nonlinear-sensitivity module adds a local first-order/second-order propagation contract on top of the frame-typed rectangular metric operator:

`||F(x+e)-F(x)||_Y <= ||DF(x)||_(X->Y) * delta + 0.5 * H_F * delta^2`

where `delta` is a declared input tangent-metric radius and `H_F` must be a uniform metric-induced second-derivative bound over the entire perturbation ball.

For a scalar output observable with output covector `g_Y`, the adjoint pullback gives the first-order sensitivity:

`||DF(x)^T g_Y||_(G_X^-1) * delta`

The observable remainder is a separate bound on the Hessian of the composite observable `phi composed with F`; a bound on the map Hessian alone is not sufficient.

The module distinguishes:
- **Declared analytic remainder bound** — caller supplies a referenced analytic bound.
- **Declared interval remainder bound** — caller supplies a referenced interval enclosure.
- **Empirical estimate only** — sampled/finite-difference curvature is reported, but the total upper-bound field remains absent.
- **First order only** — no curvature evidence; no total nonlinear bound is emitted.

The evidence identifier is a caller-supplied reference, not a verified proof digest. The Jacobian operator norm remains a numerical singular-value estimate under the declared Jacobi profile. Therefore the emitted total is conditional on both pieces of evidence and is not a formal floating-point certificate.

Tests cover an analytic quadratic map, interval-backed remainder arithmetic, the empirical-only and missing-evidence abstention paths, adjoint observable sensitivity, dense domain/codomain re-basing invariance, and the fact that a non-injective Jacobian can still have a useful forward operator-norm estimate.


## Nonlinear state and observable coverage ladder

The `nonlinear_sensitivity_ladder` example evaluates five exact one-dimensional maps
`F(x) = a*x + 0.5*h*x^2` over six symmetric metric-radius tiers. For each 2,000-interval deterministic grid it compares the sampled maximum state and scalar-observable changes to the declared analytic Taylor envelopes. The observable is `phi(y)=g*y`, so its composite curvature is exactly `|g*h|`.

The ladder reports:
- sampled state and observable maxima;
- declared state and observable upper estimates;
- sampled coverage booleans;
- tightness ratios (upper estimate / sampled maximum);
- the evidence-status label.

This is a small exact synthetic diagnostic, not a simulator or industry comparison. The sample grid checks the implementation arithmetic on known maps; it does not prove a general curvature theorem. The analytic constants are known by construction and are labelled as such. The CI workflow runs this ladder on the exact triggering revision.
