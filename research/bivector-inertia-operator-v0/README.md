# Bivector-Space Rotational Inertia Operator v0

This is a new, additive mathematical operator for N-dimensional rigid-body
rotation. It is not yet wired into `RigidBody` or the production integrator.

## Why the existing representation is insufficient

The engine stores angular velocity as a `Bivector<D>` in a lexicographic
basis of oriented planes. For spatial dimension D, there are D(D-1)/2
independent rotation planes:

- 2D: (01)
- 3D: (01), (02), (12)
- 4D: (01), (02), (03), (12), (13), (23)

But `RigidBody<D>` currently stores inertia as an `SVector<f64, D>`, and
the integrator/contact angular impulse and kinetic-energy paths reduce that
vector to a scalar mean. This is exact for isotropic spheres, not general
asymmetric rigid bodies. In 4D the dimensional mismatch is especially clear:
six angular-velocity components cannot be represented by a four-entry
spatial-axis vector without losing coupling.

## Derivation from point-mass kinetic energy

Symtropy's `Bivector<D>::to_matrix()` defines the antisymmetric generator B
for angular velocity. A point at center-relative location r has velocity:

```text
v = B r
T_rot = 1/2 Σ_k m_k ||B r_k||²
```

Let `ω` be the vector of lexicographically ordered bivector components and
let `C(r)` be the linear map from those components to `B r`. Then:

```text
T_rot = 1/2 ωᵀ [Σ_k m_k C(r_k)ᵀ C(r_k)] ω
I_biv = Σ_k m_k C(r_k)ᵀ C(r_k)
L_biv = I_biv ω
dim(I_biv) = D(D-1)/2 × D(D-1)/2
```

This construction is symmetric positive semidefinite by construction. The
implementation requires positive definiteness for an invertible rigid-body
operator; degenerate point sets fail explicitly instead of silently receiving
a fabricated inverse.

The mass-point constructor first computes the center of mass and recenters
all points. Its dense matrix retains the off-diagonal products of inertia
between rotation planes.

## Frame transformation

The current Rotor convention stores R and rotates bivector generators by the
adjoint action:

```text
B_world = R B_body Rᵀ
```

Let A be this induced action expressed in the stable bivector basis. The
operator transforms as:

```text
I_world = A I_body Aᵀ
```

The code tests invariance of kinetic energy under this paired transformation.
A later integrator must still make its body/world angular velocity,
momentum, torque and inertia frame conventions explicit.

## Implemented API

- `BivectorInertia::from_matrix`: finite, scale-aware symmetric, positive
  definite validation; stores operator and Cholesky-derived inverse.
- `BivectorInertia::from_point_masses`: center-of-mass point-mass assembly.
- `BivectorInertia::isotropic`: convenience constructor in bivector space.
- `apply`: angular momentum L = Iω.
- `solve`: angular velocity ω = I⁻¹L.
- `kinetic_energy`: 1/2 <ω,Iω>.
- `rotated_by`: frame transformation by Rotor's induced adjoint action.

The operator is limited to D=2..=9, matching the current Bivector implementation
(36 maximum independent components). It uses a dense dynamic matrix in this
research slice; sparse/block/stack-specialized representations can be
benchmarked later without changing the mathematical contract.

## Exact-source verification

`research/bivector-inertia-001` compiles the production inertia operator and
the production Bivector, Point and Rotor source files by path. Its tests cover:

- 2D, 3D and 4D basis dimension and lexicographic order.
- Isotropic symmetric point-mass distributions.
- Non-zero cross-plane coupling in an asymmetric point-mass distribution.
- Applying and solving the operator and calculating rotational kinetic energy.
- Rotating the operator and angular velocity together while preserving energy.
- Invalid matrix dimensions, non-finite/nonsymmetric/non-positive operators,
  unsupported dimensions, empty/invalid/non-finite point-mass sets.

This test layer verifies the operator algebra and mass-point assembly. It does
not verify a time integrator or full rigid-body dynamics.

## Explicit non-goals of v0

- No replacement of legacy `RigidBody.inertia` or `inv_inertia` fields.
- No production integrator change or compatibility migration.
- No gyroscopic Euler equations or torque-free asymmetric-top integration.
- No claim that all 3D shape mass properties or composite-shape mass properties
  are already correct.
- No physical validation of contacts or angular dynamics.

Those remain the next steps under PHYS-ROT-INERTIA-001 (#3753).