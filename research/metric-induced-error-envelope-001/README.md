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

## Scope and next step

v0.1 is a small dense linear-algebra reference. It does not yet implement a backward/forward error envelope for a general operator, the constrained Delassus solve, a nonlinear midpoint solver, arbitrary tangent bundles/charts, singular chart transitions, contact/friction, sparse articulated systems or production solver claims.

The next step is to connect these frame-typed norms to the mass solve M a = f, constraint-space/Delassus solves, and the physical one-step dumbbell fixture in Symtropy #4730. Every residual must identify its domain/codomain and tensor variance; raw coordinate norms remain diagnostic only.
