# Exact Coulomb Delassus Reference v0.1

Dependency-free 3D matrix-free Delassus operator for reduced rigid-body contact coordinates.

## Scope

For contact-local coordinates [normal, tangent_1, tangent_2], this crate evaluates W lambda = J M^-1 J^T lambda by scattering generalized impulses, applying inverse mass/world inverse inertia, and gathering relative contact-point velocity changes.

Contact frames must be orthonormal and right-handed. Inverse inertia tensors must be finite, symmetric and positive semidefinite; singular tensors are allowed. Static bodies use zero inverse mass/inertia.

materialize_dense() independently assembles the sparse contact Jacobian rows and contracts shared-body inverse-mass/inertia blocks. It is O(m^2 B) in the worst case and must remain a tiny-system validation path, not the production execution path.

## Run

```sh
cargo fmt --manifest-path research/exact-coulomb-delassus/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/exact-coulomb-delassus/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/exact-coulomb-delassus/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Fixtures

- Coulomb impulse-cone projection: interior, boundary, apex and frictionless ray.
- Projection idempotence, non-expansiveness and tangent-plane rotation equivariance.
- De Saxce/Feng residual checks for separation, sticking and sliding, explicit (u=q+Wp) response consistency, plus wrong-direction, negative-normal and friction-disk negative controls.
- Analytic centered two-body inverse-mass sum.
- Static-body contribution is zero.
- Matrix-free result equals dense reference for coupled contacts, nonzero lever arms and off-diagonal inertia.
- Dense symmetry and nonnegative quadratic form for PSD mass properties.
- Input-order invariance and invalid input rejection.

## Claim boundary

The `coulomb_cone` module adds Euclidean projection onto the 3D circular Coulomb impulse cone and an independent local-law residual report. The caller supplies impulse, free velocity, Delassus response (Wp), and post velocity. The checker verifies (u=q+Wp), then checks primal cone feasibility, De Saxce/Feng augmented dual-cone feasibility, complementarity and signed kinetic-energy change. It does not search for a contact impulse or enumerate all admissible branches.

The crate still does not implement an island-level cone-QP solver, an FBF outer loop, IPM/ANM continuation, warm starts, or any general convergence guarantee. No production physics or performance improvement is claimed.
