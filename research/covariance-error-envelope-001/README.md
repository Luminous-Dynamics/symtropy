# Conditioning-aware covariance error envelope v0.1

This standalone, dependency-free Rust reference starts [PHYS-COVARIANCE-ERROR-ENVELOPE-001 #4765](https://github.com/Luminous-Dynamics/symtropy/issues/4765).

## Why raw residuals are insufficient

For a linear system (A x=b), a small residual does not by itself imply a small forward error. The same backward error can correspond to very different solution errors when (A) has different condition numbers.

This reference reports separately:

- normwise backward error;
- componentwise backward error;
- an infinity-norm condition estimate;
- the product (widehat{kappa}_infty eta_infty);
- the classical forward-error expression (widehat{kappa}eta/(1-widehat{kappa}eta)), only when the product is below one;
- a typed status indicating whether the estimate is informative, weak, vacuous, or outside the declared condition envelope.

The condition number is numerically estimated from a pivoted LU factorization and inverse solves. Therefore the reported envelope is a **numerical estimate**, not a formally certified interval bound. Calibration against an independent high-precision oracle is the next research obligation.

## Coordinate covariance primitives

For (q=S y), the module computes the congruence-transformed metric and dual force:

[
M_y=S^T M_q S,qquad Q_y=S^T Q_q.
]

The test fixture checks preservation of the canonical pairing (Q_q^T q=Q_y^T y). The module keeps coordinate re-description separate from physical symmetry actions.

## Reproduction

    cargo fmt --manifest-path research/covariance-error-envelope-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/covariance-error-envelope-001/Cargo.toml
    cargo clippy --locked --manifest-path research/covariance-error-envelope-001/Cargo.toml --all-targets -- -D warnings

## Scope

v0.1 covers small dense linear systems and primal/dual coordinate transformations. It does not yet qualify the nonlinear midpoint solve, the constrained Delassus solve in the physical dumbbell model, arbitrary coordinate charts, or production solver accuracy. A tiny residual is never sufficient by itself to authorize physical equivalence.
