# Exact Coulomb Delassus Reference v0.1

Dependency-free 3D matrix-free Delassus operator for reduced rigid-body contact coordinates.

## Scope

For contact-local coordinates [normal, tangent_1, tangent_2], this crate evaluates W lambda = J M^-1 J^T lambda by scattering generalized impulses, applying inverse mass/world inverse inertia, and gathering relative contact-point velocity changes.

Contact frames must be orthonormal and right-handed. Inverse inertia tensors must be finite, symmetric and positive semidefinite; singular tensors are allowed. Static bodies use zero inverse mass/inertia.

materialize_dense() constructs the small-system reference matrix through basis-vector applications. It is O(m^2) and must remain a validation path, not the production execution path.

## Run

```sh
cargo fmt --manifest-path research/exact-coulomb-delassus/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/exact-coulomb-delassus/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/exact-coulomb-delassus/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Fixtures

- Analytic centered two-body inverse-mass sum.
- Static-body contribution is zero.
- Matrix-free result equals dense reference for coupled contacts, nonzero lever arms and off-diagonal inertia.
- Dense symmetry and nonnegative quadratic form for PSD mass properties.
- Input-order invariance and invalid input rejection.

## Claim boundary

This is only the linear response operator. It does not implement Coulomb cone feasibility, De Saxce/Feng augmented velocity, complementarity, a cone-QP inner solver, an FBF outer loop, warm starts, or convergence guarantees. No physics or performance improvement is claimed.
