# Full 3D observability oracle

The existing FIM prototype began as a 2D local tangent-plane model. This companion adds a 3D scalar-range geometry oracle to prevent a horizontal projection from hiding an unconstrained vertical axis.

For a unit line-of-sight vector u_i ∈ R³ and independent scalar range variance σ_i²:

J = Σ_i (u_i u_iᵀ / σ_i²)

The symmetric 3×3 information matrix is diagonalized using Jacobi rotations. The full spectrum is reported in ascending order; rank is classified against an explicit relative tolerance. D-, A- and E-optimal subset criteria are available for exhaustive search over at most sixteen candidates.

## Why this changes the benchmark

A horizontal ENU projection is useful when the platform and motion are genuinely planar. It is not a full 3D observability certificate. The 2026 GSFN cross-scenario paper explicitly uses an xy-plane projection of the uncertainty ellipsoid for its DoS availability assessment because its UGV motion is predominantly horizontal; it also notes that strongly non-planar scenarios may degrade this approximation.

The 3D oracle keeps all axes in the objective. A set of east/north/diagonal observations has rank two and zero vertical information; adding a vertical observation restores rank three. A near-planar set with a very weak vertical component remains rank deficient at the declared tolerance.

This is a local linearized geometry benchmark, not a 3D estimator, six-degree-of-freedom pose solver, nonlinear SLAM system, integrity monitor or physical navigation qualification.

## Reproducible commands

Run the standalone crate's 3D demonstration with Cargo using binary name observability_3d_demo. It prints planar and full-3D ranks, eigenvalues and selected candidate IDs.

## Metrics to extend

- minimum 3D information eigenvalue and rank;
- condition number and axis-wise uncertainty directions;
- rotation invariance under frame rotation;
- vertical/horizontal availability transitions;
- 3D-to-2D mode transition and return-to-3D recovery;
- information gain per movement/energy cost;
- later extension to 6D pose and 15-state inertial error-state observability, only where the state/measurement model is explicitly derived.

The 3D FIM output is not itself a protection level or calibrated covariance. Source independence and common-mode errors still need the correlated covariance model and held-out audit.
