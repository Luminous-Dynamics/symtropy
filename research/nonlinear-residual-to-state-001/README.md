# Nonlinear residual-to-state reference v0.1

This dependency-free Rust workspace is the first executable slice of [PHYS-NONLINEAR-RESIDUAL-TO-STATE-001 #4877](https://github.com/Luminous-Dynamics/symtropy/issues/4877).

## Paradigm shift: residuals are not state errors

A small value of F(z_hat) is not, by itself, a bound on the distance from z_hat to a root. The conversion depends on the inverse Jacobian and on how much the Jacobian can vary throughout a neighbourhood. The relevant quantities must use declared domain and codomain norms.

For a candidate z_hat, this reference evaluates the classical Newton-Kantorovich majorant from supplied bounds:

- beta >= ||J^-1|| from the codomain norm to the domain norm;
- eta >= ||J^-1 F(z_hat)|| in the domain norm;
- L >= Lip(DF) over the entire declared neighbourhood;
- h = beta * L * eta;
- when h <= 1/2, r = 2 eta / (1 + sqrt(1 - 2h)).

The algebraically equivalent expression avoids cancellation near h=0.

## Evidence tiers and claim boundary

The API binds the bounds to system, candidate, domain metric, codomain metric, operator, neighbourhood and norm-profile identities.

- FloatingPointEstimate produces NUMERICALLY_ESTIMATED / DIAGNOSTIC_ONLY.
- ConservativeUpperBound produces CONSERVATIVE_BOUND_QUALIFIED / CONDITIONAL_KANTOROVICH_BALL only when h and neighbourhood containment pass.

The Rust module checks the supplied numbers and their identities. It does not independently prove that beta, eta or L are conservative. That obligation belongs to the producer of those bounds and must be backed by an analytic or validated-arithmetic receipt. This v0.1 does not implement interval Newton/Krawczyk and never emits VALIDATED_UNIQUE_ROOT.

## Metric-coordinate covariance fixture

The scalar fixture computes beta, eta and L from one-dimensional domain/codomain metrics, residual, Jacobian and a second-derivative upper bound. It then applies q=s*y, transforming the domain metric by s^2, the Jacobian by s and the second derivative by s^2. The metric-induced beta, eta, L and state radius should remain invariant even though raw coordinate residual/Jacobian components change.

## Reproduction

    cargo fmt --manifest-path research/nonlinear-residual-to-state-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/nonlinear-residual-to-state-001/Cargo.toml
    cargo clippy --locked --manifest-path research/nonlinear-residual-to-state-001/Cargo.toml --all-targets -- -D warnings

The tests include an exact-root scalar polynomial fixture, a dense-rebasing scalar ladder, tiny-residual/near-singular Jacobian, h=1/2 boundary, neighbourhood escape, zero-L linear case, and invalid-input controls.

v0.1 is a scalar theorem-kernel and evidence contract. It does not qualify the full constrained dumbbell midpoint system, a general block product metric, or interval-certified root existence/uniqueness.
