# Verified scalar midpoint-step certificate v0.1

This is the first executable slice of [PHYS-NONLINEAR-STEP-CERTIFICATE-001 #5022](https://github.com/Luminous-Dynamics/symtropy/issues/5022).

## What is certified

The crate encloses the scalar reduced multiplier equation used by the constrained dumbbell midpoint reference:

    A(lambda) = I - (h^2 * lambda / 4) W
    rbar(lambda) = A(lambda)^-1 r_free
    phi(lambda) = (2/h) rbar(lambda)^T (rbar(lambda) - r0)

It evaluates the function and derivative with outward-rounded binary64 intervals. A unique root is reported only when the interval derivative excludes zero and the interval-Newton image is strictly contained in the candidate interval.

Under the interval-Newton inclusion theorem, this establishes existence and uniqueness of a root of the declared reduced equation in the certified interval. If the interval system may be singular, arithmetic overflows, the derivative interval contains zero, or inclusion fails, the outcome is INCONCLUSIVE.

## Physical state error envelope

The certified multiplier interval is propagated through the declared one-step reconstruction:

    u_mid(lambda) = v_free + (h * lambda / 2) B rbar(lambda)
    q_next(lambda) = q0 + h u_mid(lambda)
    v_next(lambda) = 2 u_mid(lambda) - v0

where B is the supplied binary64 mass-inverse constraint transpose. The output intervals are compared with the returned f64 candidate and bounded in Euclidean component radii, the generalized-coordinate mass metric for position, and the kinetic mass metric for velocity.

## Important claim boundary

v0 certifies the exact scalar reduced equation and one-step map defined by the supplied binary64 coefficients. It does not independently prove that the supplied W and B are exactly J M^-1 J^T and M^-1 J^T for a separate upstream M/J model; that coefficient provenance remains a distinct obligation. The fixture uses dyadic unequal masses and a simple difference Jacobian so the intended W/B relationships can be checked exactly in a later provenance layer.

The interval layer is finite-dimensional and f64. It is not yet a general interval matrix library, arbitrary multibody root verifier, or global Newton convergence theorem. No production solver authority or scientific-search pruning authority is created.

## Reproduction

    cargo fmt --manifest-path research/verified-midpoint-step-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/verified-midpoint-step-001/Cargo.toml
    cargo clippy --locked --manifest-path research/verified-midpoint-step-001/Cargo.toml --all-targets -- -D warnings
