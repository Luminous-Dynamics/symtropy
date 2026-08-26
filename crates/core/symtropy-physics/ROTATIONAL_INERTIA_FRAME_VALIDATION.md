# Symtropy Rotational Inertia Frame Validation Contract

## Status

This tranche defines the checked body-to-world frame transformation for rotational inertia operators using Symtropy's actual `Rotor<D>` action on `Bivector<D>`.

It is a reference/evidence layer. The production integrator and contact solver remain unchanged.

## Mathematical contract

A body orientation `R` acts on an antisymmetric bivector generator by conjugation:

`B_world = R B_body R^T`.

In bivector coefficient coordinates this is a linear orthogonal map `A(R)` — the adjoint representation of the spatial rotation on rotation space.

If `I_body` is the body-frame rotational inertia operator, then

`I_world = A I_body A^T`.

Angular momentum and inverse response are therefore

`L_world = I_world omega_world`

and

`omega_world = I_world^-1 L_world`.

This construction works in 2D, 3D, 4D, and higher supported dimensions without assuming that angular velocity is a 3D pseudovector.

## Adjoint construction

`rotation_adjoint_checked()` builds `A(R)` directly from the actual runtime algebra:

1. validate the Rotor as finite and proper;
2. enumerate the lexicographic bivector basis;
3. convert each basis bivector to its antisymmetric matrix;
4. conjugate it by the Rotor matrix;
5. convert the transformed matrix back to a `Bivector<D>`;
6. place those coordinates into one column of the adjoint matrix;
7. verify the resulting adjoint is orthogonal within a machine-scale bound.

This makes the frame convention executable rather than inferred from notation.

## Required gates

### F1 — identity

The identity Rotor must produce the identity adjoint exactly to numerical tolerance.

### F2 — proper-Rotor admission

Finite reflections or other improper matrices passed through the direct Rotor matrix constructor must be rejected.

### F3 — adjoint orthogonality

`A^T A ≈ 1` must hold on the active rotational subspace. Failure indicates an invalid Rotor, bivector basis mapping, or unrepresentable transformation.

### F4 — 3D reference agreement

For the same body principal inertia, Rotor, and world angular velocity, world kinetic energy through the N-D operator/adjoint path must agree with the independent asymmetric-top 3D reference.

This binds the generic bivector-space semantics to the already audited 3D sign/orientation convention.

### F5 — world apply/solve round trip

For valid body inertia and orientation:

`omega_world -> L_world -> omega_world_recovered`

must round-trip within declared numerical tolerance.

### F6 — 4D six-plane frame behavior

A 4D Rotor containing multiple rotation planes must produce a 6×6 adjoint over

`[e01,e02,e03,e12,e13,e23]`.

A six-component angular state must survive world apply/solve round-trip under an anisotropic six-plane inertia operator.

### F7 — isotropic invariance

An isotropic rotational operator must remain invariant under every proper Rotor:

`A (lambda 1) A^T = lambda 1`.

This is a strong control on the adjoint implementation.

## Numerical contract

The transformation rejects:

- non-finite/improper Rotors;
- non-finite transformed basis generators;
- non-orthogonal adjoint matrices;
- overflow during `A I`, `(A I) A^T`, or subsequent inertia validation.

The world operator is explicitly re-symmetrized only across mirrored entries after the mathematically symmetric transform, then passed back through the strict positive-definite operator validator.

## Production implication

The production solver does not need to materialize `I_world` every time if performance profiling shows that is expensive.

Equivalent exact implementations may transform queries into body rotational space instead:

`omega_body = A^T omega_world`

`L_body = I_body omega_body`

`L_world = A L_body`.

Similarly, inverse response may rotate momentum into body space, solve with `I_body`, then rotate the resulting angular velocity back out.

The reference intentionally defines semantics independently of the eventual optimized dataflow.

## Contact implication

A contact impulse produces an angular impulse bivector in world space. The physically correct angular velocity increment must be obtained from the **world inverse-inertia operator**, not by multiplying the bivector by one mean scalar.

That exact angular-impulse response is the next reference gate before migrating contact effective mass.

## Claim boundary

After executable validation, the correct claim is:

> Symtropy can transform a checked body-space rotational inertia operator into world rotational space through the adjoint action of its actual N-D Rotor representation, with independent 3D reference agreement and explicit six-plane 4D behavior.

Do not claim the live integrator/contact solver uses this frame transformation yet.
