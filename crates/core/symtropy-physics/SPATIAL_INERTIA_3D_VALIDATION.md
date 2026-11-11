# Frame- and Point-Aware Spatial Inertia Reference

**Status:** independent 3D reference API; not connected to production `RigidBody` or `PhysicsWorld`.

## Purpose

A central 3×3 rotational-inertia tensor is not a complete rigid-body mass model unless its reference point and expressed-in frame are carried with it. Reference-point changes also change the represented linear velocity, angular momentum, and spatial mass matrix. Treating these as unrelated fields is a common source of COM migration and compound-body errors.

This reference stores exactly ten physical scalar degrees of freedom: mass (1), COM offset (3), and central symmetric inertia (6). The six independent tensor components are packed as `[Ixx, Iyy, Izz, Ixy, Ixz, Iyz]`; matrix views are reconstructed on demand. Frame/point identifiers are metadata, not additional physical degrees of freedom.

This reference packages:
- stable `FrameId` and `ReferencePointId` identities;
- positive total mass;
- the vector from the stored reference point P to the center of mass G, expressed in the stored frame;
- central rotational inertia about G, expressed in that same frame;
- the corresponding 6×6 spatial mass matrix.

## Coordinate and twist convention

Twists are ordered angular first:

`V_P = [omega; v_P]`

where `v_P` is the linear velocity of the stored reference point P. The COM position relative to P is `c = p_G - p_P` and `C = [c]_x`.

Symtropy's Rotor convention is `exp(-B)` and physical point velocity is `-B*r`. In standard axial-vector coordinates this is `omega × r`; therefore:

`v_G = v_P + omega × c = v_P - C*omega`.

Spatial momenta are ordered:

`Pi_P = [h_P; p]`

where `h_P` is angular momentum about P and `p` is linear momentum.

## Spatial mass matrix

With central inertia `I_G`, mass `m`, and `C=[c]_x`, the reference constructs:

```text
M_P = [ I_G + m C^T C     m C ]
      [ -m C              m I ]
```

so `Pi_P = M_P * V_P`, and:

`T = 0.5 * V_P^T * M_P * V_P`

Equivalently:

`T = 0.5 * omega^T * I_G * omega + 0.5 * m * |v_P - C*omega|^2`.

The cross blocks are not optional bookkeeping: they encode COM/linear-angular coupling whenever the stored point is not the COM.

## Reference-point shift

Let `r = p_Q - p_P` be the vector from old reference point P to new reference point Q, expressed in the current frame. Then:

`c_Q = c_P - r`

`v_Q = v_P + omega × r = v_P - [r]_x * omega`.

The central inertia about G does not change. The new spatial matrix is rebuilt from `(m, c_Q, I_G)`. Reusing the same `ReferencePointId` is permitted only for a zero offset; a nonzero move with the same point identity is rejected. The kinetic energy must be identical for corresponding twists. Spatial momentum must transform as:

`p_Q = p_P`

`h_Q = h_P - r × p_P`.

The `shift_reference_point` API changes the inertia's point identity and COM offset. Callers must transform their twist with the matching point-velocity formula before comparing energy or momentum.

## Frame re-expression

`reexpress(new_frame, R_old_to_new)` treats the supplied proper Rotor as a coordinate transform from old-frame components to new-frame components:

`c_new = R*c_old`, `I_new = R*I_old*R^T`, `omega_new = R*omega_old`, `v_new = R*v_old`.

Frame identity is changed; point identity is retained. Reusing the same `FrameId` is permitted only with an identity transform (within the rotation tolerance); an actual rotation with the same frame identity is rejected. Kinetic energy is invariant. Improper or non-finite Rotors are rejected.

## Compound composition

Two spatial inertias can be composed only when both `FrameId` and `ReferencePointId` match exactly. The combined COM offset from that shared point is:

`c = (m1*c1 + m2*c2)/(m1+m2)`.

For `d_i = c_i - c`, the combined central tensor is:

`I_G = I_G1 + m1[(d1·d1)I - d1*d1^T] + I_G2 + m2[(d2·d2)I - d2*d2^T]`.

The implementation forms each weighted outer product as `(sqrt(m)*d)(sqrt(m)*d)^T`, rejecting nonzero terms that underflow or overflow instead of silently dropping them.

## Admission rules

- mass must be finite and strictly positive;
- COM offset and central inertia must be finite;
- central inertia must be symmetric within a relative tolerance, positive semidefinite, and satisfy the 3D principal-moment triangle inequality;
- frame transforms must be finite proper rotations;
- composition requires exact frame and reference-point identity matches;
- all spatial mass matrix and momentum outputs must remain finite;
- invalid or unrepresentable states fail closed.

PSD semidefinite central inertia is allowed, so idealized rods and degenerate mass distributions can be represented when their principal moments obey physical realizability.

## Deterministic unit fixtures

The module currently covers:

1. The 6×6 matrix energy against the independent COM decomposition.
2. Reference-point shift: kinetic energy and linear momentum invariance; angular momentum about P/Q transforms by `-r×p`.
3. Frame re-expression: rotated twist and inertia preserve kinetic energy.
4. Compound composition: combined spatial energy equals the sum of component energies for a common rigid twist.
5. Frame-ID and point-ID mismatch negative controls.
6. Non-PSD and triangle-inequality negative controls.
7. Improper-Rotor rejection.
8. A huge-offset/tiny-mass fixture whose spatial mass matrix remains finite.
9. Shift-then-inverse-shift restores the original point identity, COM offset and central inertia.
10. Uniform mass scaling by `1e-12` and `1e12` scales the 6×6 matrix, spatial momentum and kinetic energy linearly without changing geometry.
11. Six-component central-inertia packing/reconstruction preserves exact matrix symmetry.
12. A deterministic 48-case rotated-positive-second-moment sweep checks 6×6 spatial-matrix symmetry, positive semidefiniteness and positive finite kinetic energy.
13. A nonzero point shift cannot retain the original point identity.
14. A non-identity frame rotation cannot retain the original frame identity.

## Criterion reference-cost measurements

The existing `physics_bench` target now includes seven spatial-inertia reference operations:

- build the 6×6 mass matrix;
- apply a cached 6×6 matrix to a twist;
- checked twist-to-spatial-momentum API (including matrix construction);
- checked kinetic-energy evaluation;
- reference-point shift;
- frame re-expression;
- composition at a common reference.

Run:

```bash
cargo bench -p symtropy-physics --bench physics_bench -- spatial_inertia_3d
```

These microbenchmarks do not claim production throughput or an advantage over Drake/Pinocchio. Record exact source SHA, Rust toolchain, target triple, CPU model, OS, compiler flags, allocator, precision, Criterion warmup/sample settings and host power profile. Publish confidence intervals and raw Criterion output. The cached matrix multiply and checked API call are intentionally separate so validation/build cost is not confused with the 6×6 arithmetic.

## Promotion gates

Before integrating this into production body state:

- add an independent 6×6 spatial-matrix oracle and randomized SPD/PSD property tests;
- add SI and alternate-unit covariance tests;
- validate compound composition against independently generated point-mass clouds;
- bind the representation to collision geometry and versioned serialization;
- migrate force/torque accumulators and impulse application to an explicit point/frame contract;
- validate contact Delassus assembly and world-step work/energy receipts against the same spatial model;
- compare against Drake/MuJoCo/Pinocchio only after the semantic-equivalence contract confirms matching point, frame, inertia and contact semantics.

Until those gates pass, the claim is only that Symtropy has an independently inspectable checked 3D spatial-inertia reference. It does not mean production `RigidBody` or `PhysicsWorld` uses a 6×6 spatial inertia.