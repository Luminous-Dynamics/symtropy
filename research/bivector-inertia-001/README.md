# Bivector-Space Rotational Inertia Verification 001

This standalone Rust crate compiles the exact production files:

- `symtropy-math/src/bivector.rs`
- `symtropy-math/src/point.rs`
- `symtropy-math/src/rotor.rs`
- `symtropy-physics/src/rotational_inertia.rs`

It is intentionally isolated from the full Bevy/workspace dependency graph.
The harness verifies the new inertia operator against Symtropy's actual
bivector basis and rotor action, rather than a copied implementation.

Run:

```sh
cargo fmt --manifest-path research/bivector-inertia-001/Cargo.toml -- --check
cargo test --manifest-path research/bivector-inertia-001/Cargo.toml
cargo clippy --manifest-path research/bivector-inertia-001/Cargo.toml --all-targets -- -D warnings
```

The current tests cover 2D (one plane), 3D (three planes), and 4D (six planes),
point-mass assembly about the center of mass, non-zero cross-plane coupling,
positive-definiteness failures, angular-momentum solve/apply consistency,
rotated-frame energy invariance, and invalid mass/matrix inputs.

## Mathematical contract

For a bivector angular-velocity generator B and center-relative point mass
positions r_k:

```
v_k = B r_k
T_rot = 1/2 * Σ m_k ||B r_k||²
T_rot = 1/2 * omegaᵀ I omega
I : Λ²(R^D) -> Λ²(R^D)
dim(Λ²(R^D)) = D(D-1)/2
```

The point-mass constructor assembles the dense symmetric operator in the
lexicographic (i,j), i<j basis used by Bivector. This preserves cross-plane
terms/products of inertia. The operator is rotated using the induced adjoint
action B_world = R B_body Rᵀ.

## Claim boundary

This verifies the mathematical operator and coordinate transformations. It
does **not** yet integrate the operator into `RigidBody`, replace the legacy
inertia fields, add gyroscopic Euler equations, or validate the full
torque-free asymmetric-top time integrator. Those are subsequent compatibility
and dynamics work packages under PHYS-ROT-INERTIA-001 (#3753).
