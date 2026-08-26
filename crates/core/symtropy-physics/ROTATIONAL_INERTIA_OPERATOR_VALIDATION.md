# Symtropy Rotational Inertia Operator Validation Contract

## Status

This tranche defines a checked reference representation for rotational inertia as a self-adjoint positive-definite operator on **bivector / rotation space**.

It does not replace the production `RigidBody<D>::inertia` / `inv_inertia` fields or alter the current integrator/contact solver.

## Why this abstraction is necessary

In `D` spatial dimensions, angular velocity is a bivector with

`N_rot = D(D-1)/2`

independent rotation-plane components.

Examples:

- 2D -> 1 rotational component;
- 3D -> 3 rotational components;
- 4D -> 6 rotational components;
- 5D -> 10 rotational components.

The current `RigidBody<D>` stores only `D` inertia scalars. That representation is therefore structurally insufficient beyond 3D, and in 3D it is only exact when the chosen body frame is a principal-inertia frame.

A general rotational inertia model is an operator

`I : Λ²(R^D) -> Λ²(R^D)`

with rotational kinetic energy

`K_rot = 1/2 <omega, I omega>`.

For physically valid rigid-body inertia the represented operator must be self-adjoint and positive-definite on the active rotational subspace.

## Reference representation

`RotationalInertiaOperator<D>` stores a dense row-major matrix over Symtropy's lexicographically ordered bivector planes.

This heap-backed dense representation is intentionally **not** presented as the final production hot-path layout. It exists to establish semantics and test vectors without unstable generic-const-expression matrix sizes.

A future production implementation may specialize common dimensions with compact stack/SIMD layouts while preserving the same observable contract.

## Required gates

### O1 — dimension identity

`rotational_dimension()` must equal `Bivector::<D>::num_components()`.

A 4D regression must explicitly prove six independent inertia coordinates are accepted and used.

### O2 — symmetric positive-definite admission

Dense operators must reject:

- wrong coefficient count;
- NaN/Infinity coefficients;
- materially non-symmetric matrices;
- non-positive-definite matrices.

The reference uses checked Cholesky factorization as both the positive-definiteness gate and inverse-solve primitive.

### O3 — apply

For finite angular velocity `omega`, `angular_momentum_checked()` computes

`L = I omega`

and rejects any unrepresentable intermediate product or sum.

### O4 — solve

`angular_velocity_from_momentum_checked()` solves

`omega = I^-1 L`

through checked Cholesky forward/back substitution.

No unchecked matrix inverse is required.

### O5 — energy

`kinetic_energy_checked()` computes

`1/2 omega^T I omega`

and rejects non-finite or negative represented results.

### O6 — apply/solve round trip

For a well-conditioned valid operator,

`omega -> L -> omega_recovered`

must return the original represented angular velocity within declared numerical tolerance.

### O7 — exact 3D tensor mapping

For `D=3`, Symtropy's bivector coefficient order is

`q = [b01, b02, b12]`.

The physical angular-vector convention established by the asymmetric-top reference is

`omega = [b12, -b02, b01]`.

The 3D tensor conversion must therefore apply the corresponding signed permutation on both sides:

`I_bivector = P^T I_vector P`.

A diagonal tensor `[1,4,9]` and physical `omega=[1,2,3]` must preserve the analytical rotational energy `49 J`.

### O8 — full-tensor compound mapping

A non-diagonal tensor produced by the compound reference must map into the bivector operator without losing products of inertia. Tensor-space and operator-space rotational energy must agree.

### O9 — 4D six-plane control

A diagonal 4D reference operator must accept six values corresponding to

`[e01, e02, e03, e12, e13, e23]`.

A state with all six components non-zero must survive apply/solve round-trip and have positive finite rotational energy.

This regression prevents a future migration from accidentally preserving the old four-scalar representation under a new name.

## Production migration strategy

The reference suggests the following production architecture:

### Layer 1 — semantic contract

Angular momentum, inverse-inertia response, and rotational energy are defined through an inertia operator on bivector space.

### Layer 2 — dimension-specialized storage

Potential production representations:

- 2D: one scalar;
- 3D primitive/principal frame: three moments plus certified frame relationship;
- 3D arbitrary compound: symmetric 3×3 operator/tensor or equivalent principal-frame representation;
- 4D: symmetric 6×6 rotational operator unless a validated structured/sparse form applies;
- higher D: explicit operator specialization selected by benchmark/fidelity requirements.

Do not require every live body to carry a dense 36×36 matrix merely because the reference supports up to 9D.

### Layer 3 — world orientation

Production code must distinguish body-space inertia from world-space response. The inverse operator used by contacts/torque integration changes with orientation.

In 3D this corresponds to

`I_world^-1 = R I_body^-1 R^T`.

For general D the corresponding adjoint action must be defined on bivector space rather than guessed from axis scalars.

### Layer 4 — authoritative solver integration

Only after state representation is validated should these paths migrate together:

- torque -> angular momentum;
- angular impulse -> angular momentum;
- orientation integration;
- contact point velocity convention;
- contact effective mass;
- friction;
- joints/motors;
- CCD response;
- rotational energy/reconciliation;
- replay/digest state.

## Current integrator boundary

The current integrator and `apply_angular_impulse()` use the scalar mean of `RigidBody::inv_inertia`.

That remains a compatibility approximation until the production operator/state migration passes the above gates. This reference does not silently alter it.

## Claim boundary

After executable validation, the correct claim is:

> Symtropy has a checked N-D reference model in which rotational inertia is a positive-definite self-adjoint operator over the same bivector space used by angular velocity and angular momentum, with exact full-tensor mapping for 3D and explicit six-plane behavior in 4D.

Do not claim that the live production solver consumes this operator yet.
