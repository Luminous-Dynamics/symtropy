# N-dimensional bivector inertia reference and validation

**Status:** independent reference/evidence API; not connected to `RigidBody` or `PhysicsWorld` production stepping.

## Purpose

Three principal moments are not a general rotational-inertia model once a mass distribution has off-diagonal second moments or when the rotation space has dimension greater than three. The natural generalized velocity state is a bivector in `Lambda^2(R^D)`, with `D(D-1)/2` independent planes. The reference in `src/bivector_inertia.rs` acts directly on that space without constructing a dense square matrix over all bivector coordinates.

## Representation and operator

Let the centered mass second-moment matrix be

`S = integral (r r^T) dm`,

with `S` symmetric positive semidefinite and expressed in the same frame as the stored angular bivector `B`. Symtropy's Rotor convention is `Rotor::from_bivector(B) = exp(-B)`, so physical point velocity is `v_rot(r) = -B r`.

The inertia action is

`I(B) = B S + S B`.

Because `B` is skew and `S` is symmetric, the result is skew and can be stored as a `Bivector<D>`. In principal coordinates, `S = Q diag(lambda) Q^T`, each plane `(i,j)` has inertia eigenvalue `lambda_i + lambda_j`. The inverse therefore rotates the bivector into principal coordinates, divides each plane by `lambda_i + lambda_j`, and rotates it back. A singular plane is accepted only when the requested momentum has no material component in that nullspace; otherwise the operation fails closed.

This uses a `D x D` symmetric eigensolve plus plane-wise application rather than allocating a dense `D(D-1)/2` by `D(D-1)/2` inertia matrix. `Bivector` currently supports `2 <= D <= 9` (36 independent planes at D=9). This is a bounded research/reference contract, not a claim of arbitrary-D scalability.

## Contact effective mass and signs

For a unit impulse direction `n` at COM offset `r`, the stored angular impulse uses the offset-first convention

`delta_L = r wedge n`.

The stored response is `delta_B = I^-1(delta_L)`. The physical contact-point velocity response is

`delta_v_rot = -delta_B r`,

and the rotational contribution to inverse effective mass is `n dot delta_v_rot`. This must be nonnegative for valid positive inertia. The reference rejects non-finite inputs, zero directions, singular excited planes and materially negative effective mass. A global rigid-rotation metamorphic test checks that the scalar response is invariant when inertia, lever arm and impulse direction rotate together. `rotational_inverse_effective_mass_world` accepts world-frame contacts but keeps `S` in the body frame: it rotates only the offset and impulse direction into the body frame, avoiding a fresh world-frame eigendecomposition for every contact query.

## Gyroscopic derivative

For world/spatial-frame `B` and `S`, Rotor's `exp(-B)` convention gives

`S_dot = -B S + S B`.

With `L = B S + S B`, the stored angular momentum coordinates use the same physical axial-vector sign as the offset-first `r wedge j` torque. The reference derivative is therefore

`I(B_dot) = [B, L] + T`,

where `T` is the stored-bivector representation of the physical applied torque. `B`, `S`, and `T` must be expressed in one frame; use `rotated_by` before combining body properties with world-frame angular state. Tests check torque-free momentum/energy derivative, applied torque balance, and body/world covariance.

## Executable test matrix

- Analytical uniform D-ball plane moment for D=2,3,4 and anisotropic ellipsoid principal-plane moments in D=3.
- Off-diagonal second moments coupling distinct rotation planes.
- Forward/inverse round trips in D=3, D=4 (all six planes), and D=9 (all 36 planes).
- Uniform second-moment rescaling by `1e-12`, `1`, and `1e12`: angular state is unchanged, momentum and energy scale linearly with mass, and inverse rotational effective mass scales inversely with mass.
- Body-to-world inertia rotation, kinetic-energy covariance, gyroscopic acceleration covariance and rotational contact effective-mass covariance.
- Analytical isotropic contact effective mass and anisotropic principal-plane contact response, including equivalence of body-frame and transformed world-frame contact queries.
- Off-centre impulse from rest: direct point-velocity response agrees with the scalar rotational inverse effective mass, and rotational kinetic-energy increase equals half the impulse dotted with the contact-point velocity change.
- Torque-free momentum/energy derivative and instantaneous applied-torque balance.
- Parallel-axis composition and point-cloud accumulation with scale-safe huge-offset/tiny-mass controls, explicit underflow rejection, point-mass translation invariance (including small geometry far from world origin), canonical point-cloud input-order invariance, and offset-centering negative control.
- Invalid dimensions outside `2 <= D <= 9`, asymmetric/indefinite/zero-mass/degenerate inputs, invalid tolerance, singular rotation planes, zero impulse direction, improper Rotor, non-finite intermediates and overflow controls.

## Executable CSV qualification campaign

Run:

```bash
cargo run --release --example anisotropic_reference_campaign -p symtropy-physics
```

The example emits deterministic CSV rows with `case`, `parameter`, `observed`, `reference`, absolute/relative error and a pass flag. It includes symmetric-top analytic precession/refinement, asymmetric-top energy convergence, intermediate-axis growth with world-momentum control, off-centre impulse/contact-effective-mass/energy identity, inverse round trips in D=2/3/4/9, mass scaling, body-frame vs explicit world-frame contact queries, point-cloud ordering, huge-offset/tiny-mass point-cloud and parallel-axis composition, and fail-closed underflow controls.

This is a numerical qualification campaign, **not a throughput benchmark** and not a runtime `PhysicsWorld` campaign. It gives a reproducible input/output table for a future matched MuJoCo/Chrono/Pinocchio differential run. The CSV does not itself establish external physical validation or production solver quality. Record the exact source SHA, toolchain, host and release flags when storing a run.

## Mathematical prior art and novelty boundary

The arbitrary-dimensional bivector formulation is established mathematics, not a new claim of this PR. Edward Parker's corrected 2024 paper, *The Ricci decomposition of the inertia tensor for a rigid body in arbitrary spatial dimensions*, derives the rank-4 inertia operator on `Lambda^2(R^D)` and proves its Weyl component vanishes: the physical inertia operator is fully determined by a symmetric rank-2 tensor (the Schouten/Ricci contraction). The paper also notes that the most efficient evaluation can skip materializing the rank-4 operator and apply the rank-2 tensor directly to the angular bivector.

Reference: https://doi.org/10.1007/s10773-023-05505-7 (published 28 Dec 2023; correction published 12 Feb 2024).

Symtropy's `I(B) = B*S + S*B` is the Rotor-sign form of that compact action: `B` is the stored generator for `exp(-B)`, physical point velocity is `-B*r`, and the bivector coordinates use the offset-first `r wedge j` convention. This PR's contribution is an executable Rust reference with fail-closed numerical admission, explicit frame/sign semantics, contact effective-mass and energy identities, scale/covariance/negative controls, and bounded D=2..9 test coverage. Do not claim the underlying N-D inertia formula or its rank-2 sufficiency as novel.


## Promotion boundaries

Passing this reference's tests would establish an independently inspectable N-dimensional inertia operator, not production anisotropic rigid-body dynamics. Promotion still requires:

1. independent analytical and manufactured-solution review of the operator and its signs;
2. differential comparison with an independent mechanics implementation on equivalent 3D and, where supported, higher-D cases;
3. a production state representation and serialization/replay migration;
4. consistent use by integration, contact impulses, warm starting, friction, joints, CCD and energy/work accounting;
5. measured conditioning, timestep-convergence, long-horizon drift and matched-accuracy cost curves.

No production, SOTA, or performance claim follows merely from this reference API existing.