# Symtropy 3D Compound Inertia Tensor Validation Contract

## Status

This tranche establishes a checked **reference representation** for general 3D compound inertia. It does not yet change `RigidBody<3>` or the production solver.

The purpose is to make the current representation gap explicit and measurable before migrating live angular dynamics.

## Why three moments are not enough

A centered primitive aligned to its own principal axes can be represented by three principal moments:

`diag(I1, I2, I3)`.

A compound child may be translated and rotated relative to the compound body frame. Its contribution is

`R_i I_i R_i^T + m_i ((d_i·d_i) 1 - d_i d_i^T)`.

The aggregate about the compound center of mass is

`I_compound = Σ_i [R_i I_i R_i^T + m_i ((d_i·d_i) 1 - d_i d_i^T)]`.

This is generally a full symmetric 3×3 tensor. Products of inertia are physical state, not numerical noise to discard.

## Canonical composition

Each `CompoundMassPart3` carries a stable `part_id`.

Composition must:

1. reject empty compounds;
2. sort parts by `part_id` before all floating-point accumulation;
3. reject duplicate part identity;
4. revalidate public child mass/COM/inertia state;
5. require finite translations and proper Rotors;
6. compute child COM in compound coordinates;
7. accumulate total mass and COM without unchecked `m*r` products;
8. rotate each child principal tensor into the compound body frame;
9. apply the full parallel-axis theorem;
10. reject every non-finite intermediate before commit;
11. produce an exactly symmetric stored tensor;
12. validate positive definiteness after scale normalization.

Input vector order must not define compound evidence.

## Required gates

### T1 — symmetric dumbbell

Two 1 kg solid unit spheres at `x = ±2` must produce:

- total mass `2`;
- COM at the origin;
- `Ixx = 0.8`;
- `Iyy = Izz = 8.8`.

This is the elementary parallel-axis control.

### T2 — canonical part order

Reversing input storage order while preserving `part_id` values must produce exactly the same composed evidence object.

### T3 — rotated anisotropic child

A 12 kg cuboid with half-extents `[1,2,3]` has principal moments `[52,40,20]`.

Rotating its principal frame 45 degrees in XY while keeping the compound frame fixed must produce approximately:

- `Ixx = 46`;
- `Iyy = 46`;
- `Izz = 20`;
- `|Ixy| = 6`.

The nonzero product of inertia is the decisive representation-gap regression: the current production `[Ixx,Iyy,Izz]` body field cannot represent this tensor in that chosen body frame.

### T4 — full-tensor rotational energy

For non-diagonal `I`, rotational energy is

`K = 1/2 ω^T I ω`.

A regression must prove the cross terms change the result relative to a diagonal-only calculation.

### T5 — proper child transforms

Finite reflection/shear matrices admitted through a direct Rotor constructor must not become compound inertia evidence. Child rotations must satisfy the proper-rotation contract.

### T6 — identity and public-state integrity

Duplicate part IDs and corrupted public mass/COM state must be rejected before accumulation.

### T7 — representability

Finite inputs whose total mass, COM update, rotated tensor, parallel-axis term, tensor sum, or rotational-energy calculation becomes non-finite must fail closed.

### T8 — post-construction evidence validation

`CompoundMassProperties3` has public fields. Any serialized/manually mutated evidence must revalidate total mass, COM, exact tensor symmetry, and positive definiteness before energy or principal-frame decisions are trusted.

## Positive-definiteness check

The reference uses Sylvester's criterion on the tensor after division by its largest absolute element.

Positive scalar normalization preserves definiteness while reducing overflow risk in principal-minor/determinant arithmetic. Extremely ill-conditioned cases that cannot preserve positive minors in represented `f64` fail closed rather than being silently accepted.

## Production representation decision

This tranche deliberately does **not** diagonalize the tensor and stuff the eigenvalues into `RigidBody.inertia`.

Doing that without also preserving the corresponding principal-axis frame would be incorrect: collider/body coordinates and inertia coordinates would diverge.

Before production compound migration, choose and validate one of two representations:

### Option A — full body-frame tensor

Store a full symmetric 3×3 inertia tensor (and checked inverse) in 3D production state. Contact/integrator code evaluates `I_world^-1 = R I_body^-1 R^T`.

### Option B — principal moments + inertia-frame Rotor

Store principal moments plus an explicit Rotor mapping the principal-inertia frame to the collider/body frame. Every angular calculation must include that frame transform.

Option A is the simpler correctness model for arbitrary compounds; Option B may be attractive for diagonal inversion but adds another frame relationship that must survive replay, serialization, collider changes, and toolhead composition.

Do not silently use Option B without storing the frame.

## `merge_toolhead()` boundary

The current `merge_toolhead()` collapses compound inertia to a scalar-like value and therefore cannot be considered authoritative 3D compound physics.

It should remain a compatibility approximation until:

1. child geometry has known mass properties;
2. compound COM is recomputed;
3. full tensor composition passes this reference;
4. production inertia representation is selected;
5. angular momentum/integrator/contact paths consume that representation;
6. replay and energy reconciliation include the new state.

## Claim boundary

After executable validation, the correct claim is:

> Symtropy can deterministically compose checked 3D compound mass properties into a full symmetric body-frame inertia tensor using rotated child inertia and the full parallel-axis theorem.

Do not claim that `RigidBody<3>` or `PhysicsWorld<3>` yet stores/evolves that full tensor.
