# Verified scalar midpoint-step certificate v0.1

This is the first executable slice of [PHYS-NONLINEAR-STEP-CERTIFICATE-001 #5022](https://github.com/Luminous-Dynamics/symtropy/issues/5022).

## What is certified

The crate encloses the scalar reduced multiplier equation used by the constrained dumbbell midpoint reference:

    A(lambda) = I - (h^2 * lambda / 4) W
    rbar(lambda) = A(lambda)^-1 r_free
    phi(lambda) = (2/h) rbar(lambda)^T (rbar(lambda) - r0)

It evaluates the function and derivative with outward-rounded binary64 intervals. Each arithmetic operation widens its result by one representable value in each direction; if an operation overflows or a denominator interval contains zero, the certificate abstains. A unique root is reported only when:

1. the interval matrix solve establishes that the reduced matrix is nonsingular throughout the candidate multiplier interval;
2. the derivative enclosure excludes zero; and
3. the interval-Newton image is strictly contained in the interior of the candidate interval.

For a continuously differentiable scalar function, the interval Newton theorem states that every root in X lies in N(X), and strict inclusion N(X) inside X (with derivative interval excluding zero) establishes exactly one root in X. See [FLINT's interval Newton theorem](https://flintlib.org/doc/algorithms/ball-arithmetic/real-root-isolation.html) and the [interval Newton/Krawczyk notes](https://ww2.ii.uj.edu.pl/~zgliczyn/cap07/krawczyk.pdf).

## Physical state error envelope

The certified multiplier interval is propagated through the declared one-step reconstruction:

    u_mid(lambda) = v_free + (h * lambda / 2) B rbar(lambda)
    q_next(lambda) = q0 + h u_mid(lambda)
    v_next(lambda) = 2 u_mid(lambda) - v0

where B is the supplied binary64 mass-inverse constraint transpose. The output intervals are compared with the returned f64 candidate and bounded in Euclidean component radii, the generalized-coordinate mass metric for position, and the kinetic mass metric for velocity.

## Industry comparison

This reference is not a faster multibody solver. Drake documents efficient O(n) recursive inverse dynamics and an O(n²) Composite Body Algorithm for an explicit mass matrix, and warns to avoid forming the full matrix when possible ([Drake MultibodyPlant](https://drake.mit.edu/pydrake/pydrake.multibody.plant.html)). Pinocchio provides RNEA/CRBA/ABA as well as constrained-forward and impulse-dynamics algorithms ([Pinocchio dynamics algorithms](https://gepettoweb.laas.fr/doc/stack-of-tasks/pinocchio/master/doxygen-html/md_doc_a-features_g-dynamic.html)). MuJoCo's Newton and CG constraint solvers use gradient/cost/Newton-decrement termination criteria and a strong-convexity duality-gap bound for warm-started solutions ([MuJoCo solver documentation](https://mujoco.readthedocs.io/en/stable/computation.html)); it also uses the Delassus operator and a whitened constraint Jacobian.

The distinction here is narrower: this reference attempts to certify a unique root of one declared reduced scalar midpoint equation and propagate its interval to a one-step physical error envelope. It does not replace the articulated-body algorithms, contact optimization, warm-start duality-gap certificate, or general solver convergence guarantees in those systems. The next comparison should apply the receiver-side certificate to matched solver outputs and exact model mappings, not compare this small interval kernel's runtime with full simulators.

## Important claim boundary

v0 certifies the exact scalar reduced equation and one-step map defined by the supplied binary64 coefficients. The root receipt is opaque to external construction and is bound to the bit patterns of the complete problem record (timestep, initial/free relative positions, Delassus matrix, initial position/velocity, free midpoint velocity, mass-inverse Jacobian transpose and mass metric). Step-error propagation rechecks that binding and rejects reuse against even one changed input field.

It does not independently prove that the supplied W and B are exactly J M^-1 J^T and M^-1 J^T for a separate upstream M/J model; that coefficient provenance remains a distinct obligation. The fixture uses dyadic unequal masses and a simple difference Jacobian so the intended W/B relationships can be checked exactly in a later provenance layer.

The interval layer is finite-dimensional and f64. It is not yet a general interval matrix library, arbitrary multibody root verifier, or global Newton convergence theorem. No production solver authority or scientific-search pruning authority is created.

## Reproduction

    cargo fmt --manifest-path research/verified-midpoint-step-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/verified-midpoint-step-001/Cargo.toml
    cargo clippy --locked --manifest-path research/verified-midpoint-step-001/Cargo.toml --all-targets -- -D warnings
