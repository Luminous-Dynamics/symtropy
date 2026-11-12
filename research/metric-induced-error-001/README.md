# Metric-induced error reference v0.1

This dependency-free Rust workspace is the first executable slice of [PHYS-METRIC-ERROR-001 #4788](https://github.com/Luminous-Dynamics/symtropy/issues/4788).

## Paradigm shift: norms must respect tensor variance

A tangent vector and a covector do not transform in the same way. For a re-basing q = S y:

- tangent vectors push forward: v_q = S v_y;
- covectors pull back: Q_y = S^T Q_q;
- the metric pulls back by congruence: M_y = S^T M_q S.

The physical tangent and cotangent norms are:

    ||v||_M = sqrt(v^T M v)
    ||Q||_(M^-1) = sqrt(Q^T M^-1 Q)

The canonical pairing Q(v) is unchanged by the paired transformations. Ordinary coordinate infinity norms remain available as diagnostics, but the tests intentionally show that they change under a dense non-orthogonal basis while the metric-induced norm is preserved.

## Riesz residual identity

For the mass/Riesz map M a = Q, let a_hat be an approximate acceleration. Its covector residual is r = M a_hat - Q. Since r = M(a_hat-a), exact arithmetic gives:

    ||r||_(M^-1) = ||a_hat-a||_M

This is the core physical error identity exercised by the reference tests. The equality is an algebraic identity; floating-point evaluation still needs a declared condition/error envelope and independent calibration.

## Frame-typed API

The public API distinguishes Tangent<Frame, N> from Covector<Frame, N>. CoordinateRebasis<From, To, N> exposes directionally correct pushforward/pullback operations and returns frame-specific types. This prevents a number of accidental primal/dual or cross-frame substitutions at compile time.

## Validation

    cargo fmt --manifest-path research/metric-induced-error-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/metric-induced-error-001/Cargo.toml
    cargo clippy --locked --manifest-path research/metric-induced-error-001/Cargo.toml --all-targets -- -D warnings

The workflow pins Rust 1.96.0. The reference covers bounded small dense SPD metrics and invertible coordinate changes; it does not qualify arbitrary nonlinear charts, singular metrics, contact/friction, production solvers or formally certified floating-point bounds.
