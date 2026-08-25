# Symtropy 3D Asymmetric-Top Validation Protocol

## Status

This document defines the validation contract for the 3D principal-inertia reference implementation in `angular_dynamics.rs`.

The reference is intentionally separate from the production N-D integrator. Passing these tests does **not** mean the live `PhysicsWorld` has full anisotropic rigid-body dynamics yet. The production integrator, angular impulses, contact effective mass, and rotational kinetic-energy accounting must be migrated in later changes and revalidated as a coupled system.

## Why this is P0

The production integrator currently reduces `RigidBody::inv_inertia` to one scalar mean. That makes asymmetric bodies respond isotropically to torque. `RigidBody::kinetic_energy()` likewise uses mean inertia. A long box, capsule, compound body, or vehicle component therefore cannot have physically correct 3D rotational response until the hot path is migrated.

A second issue must be resolved at the same time: Symtropy's `Rotor::from_bivector` integrates orientation as `exp(-B)`, while `Bivector::apply_to_vector(r)` returns `B r`. For orientation-consistent rigid-body kinematics, instantaneous point velocity is `-B r`. The reference API makes that convention explicit rather than silently depending on the existing contact convention.

## Reference state

The reference model uses:

- a proper 3D orientation `R`,
- a Symtropy angular-rate bivector `B`,
- strictly positive body-space principal moments `I1, I2, I3`,
- world angular momentum `L`,
- optional world torque `tau`.

For Symtropy's rotor convention, the physical angular-velocity vector is

`omega = [b12, -b02, b01]`.

The body-space quantities are

`omega_body = R^T omega`

and

`L_body = I_body omega_body`.

World angular momentum is

`L_world = R L_body`.

Rotational kinetic energy is

`K_rot = 1/2 omega_body^T I_body omega_body`.

## Reference step

For one timestep `dt`:

1. compute current `L_world` from `(R, omega, I_body)`,
2. apply angular impulse exactly: `L_world += tau_world dt`,
3. recover `omega_world = I_world(R)^-1 L_world`,
4. integrate orientation through the existing SO(3) exponential map,
5. recover `omega_world` again from the unchanged `L_world` and new orientation.

This is an angular-momentum reference integrator, not a claim of an energy-exact integrator. With zero applied torque, world angular momentum is preserved by construction. Energy error must converge as the timestep is refined.

## Required deterministic unit gates

### A1 — convention round-trip

- axial/vector -> bivector -> axial/vector returns the original vector,
- positive world-z angular velocity produces positive-y instantaneous velocity at a +x offset,
- a small positive world-z rotation moves +x toward +y.

This gate prevents a pair of sign errors from cancelling invisibly.

### A2 — inertia validation

Reject zero, negative, NaN, and infinite principal moments.

The first reference deliberately does not enforce principal-moment triangle inequalities. Shape-derived inertia constructors may add stronger realizability checks later.

### A3 — isotropic free rotation

For `I1 = I2 = I3` and zero torque:

- world angular momentum stays constant,
- world angular velocity stays constant,
- orientation remains a proper rotation.

### A4 — asymmetric free rotation

For `I1 != I2 != I3` and an initial angular velocity not aligned with a principal axis:

- world angular momentum stays constant,
- world angular velocity evolves,
- orientation remains proper.

A constant world angular velocity in this case is a failure; it reproduces the current isotropic approximation.

### A5 — energy convergence

Run the same torque-free asymmetric top at at least two timesteps. The finer timestep must reduce relative rotational-energy drift.

The campaign must report:

- duration,
- timestep,
- inertia,
- initial angular velocity,
- maximum and final `|Delta L|`,
- maximum and final relative `Delta K`,
- maximum orthogonality error,
- determinant error.

### A6 — orientation-dependent inverse inertia

Apply the same world angular momentum to the same anisotropic principal moments under two different body orientations. The recovered world angular velocity must differ when the principal axes differ relative to the world impulse.

### A7 — torque impulse accounting

For finite world torque and timestep:

`L_after - L_before = tau dt`

to numerical tolerance.

This is the accounting contract future actuator and collision impulse paths must preserve.

## Production migration gates

The reference must not be called “production anisotropic rigid-body dynamics” until all of the following are migrated together.

### P1 — body state and energy

- 3D principal inertia has an explicit body-space interpretation.
- `RigidBody::kinetic_energy()` uses the anisotropic 3D formula.
- shape constructors compute physically appropriate principal moments.
- compound/toolhead inertia uses the full parallel-axis theorem instead of collapsing back to one scalar.

### P2 — integrator

- torque changes angular momentum using the same convention as the reference,
- torque-free asymmetric-top behaviour matches the reference within declared tolerance,
- damping and angular clamping are treated as explicit sinks/interventions rather than hidden conservation error.

### P3 — contact kinematics

The contact solver must use orientation-consistent point velocity. The current direct `B.apply_to_vector(r)` convention must not be migrated blindly.

For a contact offset `r`:

`v_point = v_com + omega x r = v_com - B r`.

A dedicated regression must verify point velocity against a finite-difference orientation update.

### P4 — contact effective mass

For an impulse direction `n`, the 3D denominator must use the actual world inverse inertia, not scalar mean inertia. The angular contribution should be derived from the physical torque impulse `r x J`, mapped through `I_world^-1`, and projected back to point velocity.

### P5 — world solver

- normal impulses,
- friction impulses,
- restitution,
- warm starting,
- joints,
- motors,
- CCD contact response

must all share the same angular convention and anisotropic inverse inertia.

## Independent validation targets

After the deterministic reference tests pass, add independent comparisons against at least one mature 3D dynamics implementation. Candidate references include MuJoCo, Jolt, or PhysX for matched free-top and impulse cases.

Matched cases should include:

- spherical/isotropic control,
- rectangular cuboid with analytical principal moments,
- torque-free asymmetric top,
- off-center impulse,
- spinning box contacting a plane,
- compound body after a mass-property update.

The comparison must publish initial conditions, timestep, integrator settings, and error metrics. Visual similarity alone is not sufficient.

## Claim boundary

Before the production migration gates pass, the correct claim is:

> Symtropy contains a 3D principal-inertia/asymmetric-top reference model used to validate a future production angular-dynamics migration.

Do not claim that the general `PhysicsWorld` already has full anisotropic rigid-body dynamics merely because this reference module exists.
