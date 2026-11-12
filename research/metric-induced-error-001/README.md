# Metric-induced physical error reference v0.1

First executable slice of [PHYS-METRIC-ERROR-001 #4788](https://github.com/Luminous-Dynamics/symtropy/issues/4788).

## Paradigm shift

Ordinary coordinate component norms are not physical invariants under dense non-orthogonal re-basing. This reference treats tangent vectors and cotangent covectors as distinct frame-typed objects and computes their metric-induced norms:

||v||_M = sqrt(v^T M v)

||alpha||_(M^-1) = sqrt(alpha^T M^-1 alpha)

For q = S y, it implements the correct variance laws:

- tangent: v_q = S v_y;
- covector: alpha_y = S^T alpha_q;
- metric: M_y = S^T M_q S;
- dual metric: M_y^-1 = S^-1 M_q^-1 S^-T.

The canonical pairing and the primal/dual norms are unchanged by this re-basing, up to floating-point rounding.

## Riesz-map identity

For an SPD mass metric M, the Riesz map sends a tangent error e to its residual covector r = M e. Therefore:

||r||_(M^-1)^2 = r^T M^-1 r = e^T M e = ||e||_M^2.

This gives a physically meaningful interpretation of the mass-solve residual: if r = Q - M a_hat and a = M^-1 Q, then ||a - a_hat||_M = ||r||_(M^-1) in exact arithmetic. It is an identity for the admitted SPD metric, not a floating-point forward-error certificate for a general operator.

## Implementation

- Rust frame markers prevent accidental interchange of tangent and cotangent values.
- SPD admission uses symmetry checks and Cholesky factorization with explicit relative tolerances.
- Dual norms use an SPD solve; no matrix inverse is materialized.
- Dense basis transforms use partial-pivoted solves and reject singular bases.
- Derived values are checked for non-finite results.
- No external dependencies.

## Reproduction

cargo fmt --manifest-path research/metric-induced-error-001/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/metric-induced-error-001/Cargo.toml
cargo clippy --locked --manifest-path research/metric-induced-error-001/Cargo.toml --all-targets -- -D warnings

## Authored fixtures

The initial 3D fixture uses a diagonal SPD mass metric and a dense upper-triangular determinant-one basis. Tests cover tangent and cotangent norm invariance, canonical pairing, a coordinate infinity-norm counterexample, the Riesz-map identity, raise/lower duality, wrong covector variance, singular basis rejection, and non-SPD/non-finite metric rejection.

## Prospective benchmark programme

Extend the fixed 3D fixture into a preregistered ladder of dense basis transforms with condition tiers 1, 10, 1e2, 1e3, 1e4, 1e5 and 1e6. For each tier, record raw coordinate residual and infinity norm; tangent metric norm and dual covector norm; canonical pairing defect; metric and basis condition estimates; Cholesky pivot ratio and solve residual; lifted physical-state error against an independent high-precision oracle; false ACCEPT, false INCONCLUSIVE and abstention rates; wall time, memory and factorization reuse.

Prospective targets (not achieved): zero false physical ACCEPT on wrong-variance controls; all singular/non-finite/out-of-envelope fixtures explicitly abstain; all admitted fixtures agree with an independent high-precision oracle within a preregistered envelope; metric-induced quantities remain invariant across well-conditioned rebases.

## Industry comparison

MuJoCo already formulates constrained acceleration using the inertia-weighted norm ||x - M^-1 tau||_M and describes constraint-space solvers in terms of the Delassus operator J M^-1 J^T. The intended differentiation here is not claiming to invent inertia-weighted norms. It is making tangent/cotangent variance, coordinate re-basing, error evidence, and independent negative controls explicit in a reusable verification contract.

This reference is not yet comparable to full multibody engines on throughput, contacts, friction, articulated dynamics or robustness. No SOTA superiority claim is made.

## Claim ceiling

This crate covers small dense SPD metrics and coordinate changes. It does not yet qualify nonlinear charts, singular metrics, contact/friction, nonlinear solver error, stochastic systems, arbitrary multibody dynamics, or formally certified floating-point bounds.
