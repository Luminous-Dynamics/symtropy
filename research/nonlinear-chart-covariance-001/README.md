# Nonlinear chart covariance reference v0.1

This crate is the first differential-geometric qualification slice for [NONLINEAR-CHART-001](https://github.com/Luminous-Dynamics/symtropy/issues/4757).

It uses the six-dimensional globally invertible shear

    q0 = y0 + a sin(y1)
    q1 = y1
    qi = yi, i = 2..5

The determinant of its Jacobian is exactly one. Its non-constant Jacobian and non-zero Hessian expose terms that constant linear coordinate transformations cannot test.

## Implemented identities

For J(y) = D phi(y):

- M_y = J^T M_q J
- Q_y = J^T Q_q for force covectors
- p_y = J^T p_q for canonical momentum covectors
- p_q^T dq = p_y^T dy
- J_h,y = J_h,q J
- qddot = J yddot + D^2 phi(y)[ydot, ydot]

The code validates finite inputs and uses explicit six-dimensional arrays. The tests include central finite-difference Jacobian and acceleration checks, chart round-trip, metric symmetry/positive quadratic form, covector one-form pairing, constraint-Jacobian chain rule, and an omitted-Hessian negative control.

## Reproduction

    cargo fmt --manifest-path research/nonlinear-chart-covariance-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/nonlinear-chart-covariance-001/Cargo.toml
    cargo clippy --locked --manifest-path research/nonlinear-chart-covariance-001/Cargo.toml --all-targets -- -D warnings

## Explicit scope limit

This crate verifies differential pullback identities for one bounded analytic chart. It does not yet integrate a constrained discrete-mechanics step map, prove variational integrator covariance, or establish physical correctness. Those require a follow-up model-level fixture that compares transformed and canonical step maps, multipliers, constraint defects, work/energy identities, and convergence under identical frozen inputs. No benchmark advantage is claimed.
