# Nonlinear multi-constraint mixing reference v0.1

This standalone Rust workspace starts [PHYS-NONLINEAR-CONSTRAINT-MIXING-001 #5129](https://github.com/Luminous-Dynamics/symtropy/issues/5129), following scalar chart covariance in #5128 and linear constraint-equation covariance in #4876.

## Global nonlinear mixing map

For two constraint residuals g=(g1,g2), this reference uses:

    phi(g1,g2) = (g1, g2 + beta*g1^2)

Its Jacobian is triangular with determinant exactly one:

    T(g) = [[1, 0], [2*beta*g1, 1]]

The inverse is explicit:

    phi^-1(h1,h2) = (h1, h2 - beta*h1^2)

The map is therefore globally invertible and preserves the zero set without introducing additional components.

## Transformation laws

- residual h=phi(g);
- Jacobian J_h=T J_g;
- multiplier lambda_h=T^-T lambda_g;
- Delassus W_h=T W_g T^T;
- acceleration h_ddot=T g_ddot + (0, 2 beta g_dot_1^2).

The code independently computes the Hessian of each composed constraint and checks the acceleration chain rule. It also checks generalized-force and virtual-work invariance. Finite residual-value pairing is explicitly not treated as a covector invariant.

## Projection chart dependence

A separate unit-circle/plane fixture computes a metric-normal Gauss-Newton projection step. The first displacement changes with the nonlinear constraint chart, as expected for a residual-linearized method away from the manifold. Both chart-specific iterations are checked against the same exact Euclidean nearest point on the circle-plane intersection. The oracle is exact only for the identity mass metric.

## Dense coordinate axis

The test fixture also applies q=S y and M_y=S^T M_q S, J_y=J_q S. The Delassus operator and the nonlinear constraint-chart laws must commute with this independent coordinate re-basing.

## Reproduction

    cargo fmt --manifest-path research/nonlinear-constraint-mixing-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/nonlinear-constraint-mixing-001/Cargo.toml
    cargo clippy --locked --manifest-path research/nonlinear-constraint-mixing-001/Cargo.toml --all-targets -- -D warnings

## Scope

v0.1 is a small dense, smooth, full-row-rank two-constraint reference. It does not qualify arbitrary nonlinear maps, redundant constraints, nonsmooth contact, friction, singular manifolds, sparse articulated systems or production solver superiority.
