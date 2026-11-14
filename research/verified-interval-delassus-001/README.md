# Verified interval Delassus coefficient provenance v0.1

Implements the coefficient-provenance obligation in [PHYS-MIDPOINT-COEFFICIENT-PROVENANCE-001 #5028](https://github.com/Luminous-Dynamics/symtropy/issues/5028), stacked on [#5083 — outward-rounded metric uncertainty](https://github.com/Luminous-Dynamics/symtropy/pull/5083).

## Claim

For interval enclosures of a symmetric mass matrix M and constraint Jacobian J, the reference derives outward enclosures of B = M^-1 J^T and W = J M^-1 J^T. It never accepts caller-supplied B or W and never forms M^-1.

## Independent derivations

1. Interval Cholesky M = L L^T, followed by interval triangular solves for each column of B.
2. Direct interval assembly W_direct = J B.
3. Independent whitened rows Y = J L^-T and W_gram = Y Y^T.
4. An interval residual check verifies that each entry of M B - J^T contains zero.
5. The direct and Gram enclosures are intersected entrywise; disjoint enclosures reject the certificate.
6. Interval Cholesky of the resulting W proves a strictly positive Delassus factor-diagonal lower bound. Every represented W is SPD, hence every represented J is full row rank for the represented SPD M.

The receipt retains exact input interval arrays, profile, Cholesky factors, B, Y, W, and derived upper/lower bounds as one immutable value.

## Verified downstream bounds

For constraint residual r, the minimum kinetic correction is bounded by ||r||_2 / sqrt(lambda_min_lower(W)). For multiplier lambda, the induced kinetic norm is bounded by sqrt(||W||_F_upper) * ||lambda||_2. The eigenvalue lower bound is derived from an outward upper bound on ||W^-1||_2 using the interval Cholesky factor and interval triangular solves.

## Fail-closed behavior

- Non-symmetric mass intervals are rejected.
- Mass or Delassus factor diagonal lower endpoints at or below the declared floor yield INCONCLUSIVE.
- Disjoint direct/Gram enclosures are rejected.
- A mass-solve residual interval excluding zero is rejected.
- Dependent or near-dependent constraints that cannot prove strictly positive W remain inconclusive; no regularization or pseudo-inverse is substituted.
- Interval overflow and invalid operations propagate as errors.

## Tests

Six tests cover exact coefficient fixtures, uncertain mass/J sample coverage, dense coordinate re-basing, rank-deficient constraint abstention, non-SPD mass abstention, and residual/multiplier bound production.

## Arithmetic contract

The interval primitive is reused from `verified-interval-metric-envelope`, which pins Rust 1.96.0 and declares the conditional IEEE-754 binary64 / round-to-nearest / no-fast-math contract. This PR is stacked on #5083 so interval arithmetic remains a single source.

## Claim ceiling

v0.1 covers small dense SPD mass intervals and full-row-rank linear holonomic constraints. It does not certify PSD redundant-constraint strata, contact/friction, nonlinear manifolds, sparse articulated solvers, interval-Newton trajectory convergence, or arbitrary hardware/compiler arithmetic. Failure to prove SPD is INCONCLUSIVE, not evidence that the physical model is invalid.
