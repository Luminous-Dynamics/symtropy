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

## Industry comparison and research targets

Mass-metric norms are established mechanics, not a new invention. MuJoCo explicitly formulates constrained acceleration as a Gauss-principle minimization in the inertia-weighted norm and distinguishes motion maps from force pullbacks via the Jacobian transpose ([computation documentation](https://mujoco.readthedocs.io/en/stable/computation.html)). Drake computes the generalized mass matrix with a Composite Body Algorithm and documents when explicit mass-matrix construction should be avoided ([CalcMassMatrix](https://drake.mit.edu/pydrake/pydrake.multibody.plant.html)). Pinocchio implements Featherstone-style rigid-body algorithms and derivatives ([overview](https://gepetto.github.io/doc/pinocchio/doxygen-html/)). Classical numerical analysis already distinguishes backward error, condition, and forward-error bounds ([Higham / SIAM chapter](https://epubs.siam.org/doi/10.1137/1.9780898718027.ch7)).

The differentiator being tested here is not the mass metric itself; it is the combination of frame-typed tangent/cotangent APIs, explicit re-basing laws, dual-norm residual identities, and fail-closed negative controls. It remains a reference implementation rather than a replacement for sparse articulated-body algorithms.

Prospective evaluation targets:

- zero false admissions under wrong tangent/covector variance, wrong metric congruence, or wrong pairing laws;
- dense coordinate changes across a preregistered condition ladder (for example 1, 10, 10², 10³, 10⁴, 10⁵, 10⁶), with physical tangent/cotangent norms invariant inside a measured rounding envelope;
- every admitted fixture checked against an independent high-precision oracle;
- 100% explicit rejection or INCONCLUSIVE status for singular, non-finite, non-positive-definite, or out-of-envelope inputs;
- compare direct Cholesky, whitened coordinates, QR, and partial-pivoted LU on the exact same operators, reporting physical error, backward residual, condition, runtime and memory;
- scale from this 3D dense reference to the 6D constrained dumbbell, then sparse multibody fixtures, without claiming that dense cubic algorithms beat articulated-body O(n) / O(n²) methods.

These are proposed targets, not achieved results.
