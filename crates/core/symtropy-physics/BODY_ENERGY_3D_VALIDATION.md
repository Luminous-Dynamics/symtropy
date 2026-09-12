# Symtropy Checked 3D Rigid-Body Energy Validation

## Status

This document defines the validation boundary for the checked `RigidBody<3>` energy bridge added on top of the asymmetric-top reference.

The implementation provides **exact represented-state measurement** for a 3D rigid body's translational kinetic energy plus anisotropic rotational kinetic energy. It does **not** change the generic production integrator, contact solver, angular impulses, joints, friction response, CCD, or the historical N-D `RigidBody::kinetic_energy()` compatibility method.

This therefore satisfies only the **measurement portion** of P1 from `ASYMMETRIC_TOP_VALIDATION.md`.

## Model

For a dynamic 3D body:

`K = K_linear + K_rot`

with

`K_linear = 1/2 m |v|^2`

and

`K_rot = 1/2 omega_body^T I_body omega_body`.

`I_body` is interpreted from the body's stored three inertia components as principal moments. The body's existing Rotor defines the world↔body frame transform under the convention validated by the asymmetric-top reference.

Static and kinematic bodies report zero from the checked energy method, matching the existing compatibility API's modeled dynamic-energy boundary.

## Fail-closed requirements

The checked body-energy path must reject rather than emit evidence when:

- dynamic mass is non-finite or non-positive;
- any linear-velocity component is non-finite;
- finite linear state produces an unrepresentable squared speed or kinetic energy;
- any principal moment is invalid;
- the body Rotor is not a proper rotation;
- rotational-energy arithmetic is unrepresentable;
- finite linear and rotational contributions overflow when combined.

The angular failures are inherited from the checked asymmetric-top reference through `RigidBodyEnergyError::Angular`.

## Required deterministic gates

### B1 — analytical anisotropic energy

For:

- `m = 2 kg`,
- `v = [3, 4, 0]`,
- `I = [1, 4, 9]`,
- world/body identity orientation,
- `omega = [1, 2, 3]`,

the checked energy must equal:

- linear: `25 J`,
- rotational: `49 J`,
- total: `74 J`.

The historical mean-inertia compatibility method must be shown to differ materially on this case. This prevents accidental substitution of the old approximation into validation campaigns.

### B2 — orientation-dependent anisotropic energy

For fixed world angular velocity and anisotropic principal inertia, changing body orientation must change the checked rotational energy according to the body-frame projection of `omega`.

This proves the measurement path is using the real body orientation rather than a scalar or world-axis inertia shortcut.

### B3 — non-dynamic control

Static/kinematic modeled energy remains zero without requiring positive dynamic principal moments.

### B4 — malformed live-state rejection

Mutated/public body state with invalid mass, non-finite velocity, or invalid principal inertia must return a typed error rather than NaN/Infinity.

### B5 — derived representability

Finite inputs that overflow linear or rotational kinetic-energy arithmetic must fail explicitly.

## Reconciliation use

Once this tranche is compiler/test green, 3D validation and future friction campaigns should prefer `RigidBody<3>::kinetic_energy_3d_checked()` when measuring a 3D body's kinetic reservoir.

The generic `EnergyStateSnapshot` must **not** silently switch to this method for all dimensions in this tranche. That migration needs its own reviewed change because snapshot semantics currently span N-D worlds and existing evidence baselines.

## Remaining P1 work

This tranche does not complete P1. Still required:

- shape-derived anisotropic principal moments for boxes/capsules/compounds;
- full parallel-axis compound/toolhead inertia rather than scalar collapse;
- production integrator migration to angular-momentum/principal-inertia dynamics;
- exact 3D contact effective mass under the same angular convention;
- replacement or explicit deprecation strategy for the generic scalar-mean energy path;
- world-level reconciliation campaigns using the checked 3D measurement.

## Claim boundary

The correct claim after this tranche passes is:

> Symtropy can measure the represented kinetic energy of a 3D `RigidBody` using checked anisotropic principal-inertia dynamics, while the production solver still uses the historical generic angular approximation.

Do not claim that the live `PhysicsWorld` has full anisotropic 3D dynamics from this measurement bridge alone.
