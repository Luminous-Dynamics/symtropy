# Full-group discrete-step equivariance reference v0.1

This is the first executable physical slice of [PHYS-FULL-GROUP-STEP-EQUIV-001 #4940](https://github.com/Luminous-Dynamics/symtropy/issues/4940).

## What is being qualified

The fixture is a point mass constrained to a circle, with an isotropic spring covector and an arbitrary external force covector. The discrete map is a projected velocity-Verlet / SHAKE-RATTLE-shaped reference step:

1. apply a half-force kick;
2. drift;
3. project position onto the circle using the mass metric;
4. apply the second half-force kick;
5. project velocity into the tangent space.

This is a bounded reference integrator, **not** a claim that the implementation is a production RATTLE solver.

The important point is that the *actual discrete map* is checked, not merely the continuous force law.

## Full O(2) action

The exact disconnected component is the reflection \(R=\operatorname{diag}(1,-1)\). A connected representative is a nontrivial planar rotation. Both act on tangent states; force covectors transform by the inverse transpose.

The circular constraint \(g(q)=q^T Gq-L^2\) is invariant under the full orthogonal group. The tests check:
- the mass metric isometry \(P^T M P=M\);
- force-covector duality and external work;
- the circular constraint and tangent-velocity residuals;
- SHAKE/RATTLE-shaped position and velocity multipliers;
- the constraint Jacobian and Delassus scalar \(W=J M^{-1}J^T\);
- kinetic/potential observables;
- commutation of the *discrete step* with connected rotation and disconnected reflection.

## Dense generalized coordinates

For \(q=S y\), the tests use a non-orthogonal unimodular shear. They transform:
- \(M_y=S^T M_q S\);
- tangent states by \(S^{-1}\);
- force covectors by \(S^T\);
- the group action by conjugation \(P_y=S^{-1}P_qS\).

The dense-coordinate action is generally not Euclidean-orthogonal, but it remains an isometry of the transformed mass metric. Both sides of the discrete step are independently evaluated in their declared coordinates and then lifted for physical comparison.

## Reproduction

    cargo fmt --manifest-path research/full-group-step-equivariance-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/full-group-step-equivariance-001/Cargo.toml
    cargo clippy --locked --manifest-path research/full-group-step-equivariance-001/Cargo.toml --all-targets -- -D warnings

## Claim boundary

v0.1 is a one-particle, two-dimensional, one-holonomic-constraint, one-step reference with an isotropic mass/spring model. It does not establish arbitrary multibody/contact/friction equivariance, arbitrary nonlinear integrators, full-trajectory error bounds, global manifold topology, production performance, or scientific-search pruning. The step integrator is explicitly a projected reference map; production RATTLE/midpoint equivalence is deferred.
