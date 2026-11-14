# Metric-whitened nonlinear step certificate v0.1

This dependency-free Rust reference implements an a-posteriori Krawczyk inclusion test for a small two-dimensional quadratic system. It is the first executable slice of [PHYS-NONLINEAR-STEP-CERTIFICATE-001 #5178](https://github.com/Luminous-Dynamics/symtropy/issues/5178).

## What is certified

Given an approximate root, a metric frame, and a bounded box in whitened state coordinates, the implementation:

1. evaluates the nonlinear residual with outward-rounded binary64 intervals;
2. encloses the Jacobian over the entire state box;
3. builds a point preconditioner from the midpoint Jacobian;
4. encloses the Krawczyk image;
5. proves strict self-inclusion and an infinity-norm contraction bound below one;
6. checks that the preconditioner determinant interval excludes zero;
7. returns a physical metric-radius upper bound for the unique root in the box.

Under those conditions, the Krawczyk fixed-point map is a contraction on the box. Its unique fixed point is a root because the preconditioner is nonsingular.

## Metric covariance

The nonlinear equations are defined in a physical reference frame. MetricFrame2V1 gives the exact integer coordinate map q = S y, a lower state-metric factor L, and a lower residual-metric factor R. The constructor verifies exactly, using checked integer arithmetic, that L L^T = S^T S for the identity physical metric.

The certificate uses the whitened state u = L^T (y-y0) and residual Fhat = R^-1 F. The state norm is induced by G = L L^T; the residual is interpreted in the dual metric. This preserves the physical state metric and residual duality under the declared coordinate basis. The dense unit-triangular shear fixture uses exact integer factors, avoiding an unverified floating-point Cholesky step in the certificate inputs. Integer entries are restricted to the exactly representable binary64 integer range before conversion into interval endpoints.

## Interval arithmetic and assumptions

Every interval operation expands its binary64 result by one representable value in each outward direction. Non-finite endpoints, division by a denominator containing zero, and arithmetic overflow produce an INCONCLUSIVE receipt. The system coefficients and frame integers are interpreted as exact inputs; all nonlinear polynomial and Jacobian evaluations used by the inclusion test are interval-enclosed. A successful receipt retains the complete system coefficients, coordinate frame, metric factors, centre, box and profile. A receiving verifier can replay the calculation and demand bit-exact agreement for every input and output field. This is structural replay, not a cryptographic signature or digest.

This is a small reference implementation. Its arithmetic assumptions, host/compiler and exact input bytes must be bound in a production evidence envelope before the receipt can be treated as a durable cross-system certificate. It does not yet validate the production 6-DOF constrained dumbbell step.

## Reproduction

    cargo fmt --manifest-path research/nonlinear-step-certificate-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/nonlinear-step-certificate-001/Cargo.toml
    cargo clippy --locked --manifest-path research/nonlinear-step-certificate-001/Cargo.toml --all-targets -- -D warnings

## Tests

- outward-rounded interval addition, subtraction, multiplication and division checked against an exact dyadic-rational oracle;
- a 17-value signed dense-shear ladder with invariant physical root enclosure and metric error radius;
- exact binary64 input binding and bit-exact receipt replay with changed-system/frame/box/profile negative controls;
- frame integers outside binary64's exact-integer range rejected before interval evaluation;
- division-by-zero and overflow controls;
- canonical quadratic root enclosure;
- dense non-orthogonal coordinate re-basing with invariant metric error radius, contraction and residual-dual norm;
- bit-exact receipt replay and rejection after changing a single system coefficient, frame identifier, box endpoint or profile threshold;
- wrong metric factor and singular basis rejection;
- changed equation not certified;
- singular Jacobian and out-of-profile basis abstention;
- invalid zero-radius box rejection.

## Claim boundary

v0.1 qualifies a bounded smooth 2D polynomial reference under exact integer frame metrics and the implemented interval arithmetic. It does not yet certify the constrained midpoint solver, contact/friction, a production multibody system, or general nonlinear residual-to-state maps. A numerical Newton residual alone is never reported as a certified state-error bound.
