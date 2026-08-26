# Symtropy Checked World 3D Energy Validation

## Status

This contract defines the additive `PhysicsWorld<3>` kinetic-energy evidence surface in `world_energy_3d.rs`.

It is a **measurement view**, not a production solver migration. The existing `PhysicsWorld::total_kinetic_energy()` remains unchanged and continues to use the generic historical body metric.

## Why a world view is needed

A checked body-energy function is necessary but not sufficient for world-level evidence. A world measurement must also define:

- which bodies are included;
- identity uniqueness;
- deterministic aggregation order;
- body-level failure attribution;
- aggregate representability.

Otherwise two worlds containing the same physical bodies can produce slightly different floating-point totals merely because storage order differs, or a duplicate handle can silently be counted twice.

## Canonical measurement

`body_kinetic_energies_3d_checked()`:

1. selects dynamic bodies only;
2. sorts them by `BodyHandle`;
3. rejects duplicate handles;
4. calls `RigidBody<3>::kinetic_energy_3d_checked()` for each body;
5. returns the per-body evidence in canonical handle order.

`total_kinetic_energy_3d_checked()` then performs a deterministic checked sum over that canonical vector.

## Required gates

### W1 — storage-order invariance

Two worlds with identical handle→body state but different `Vec<RigidBody>` storage order must return identical per-body evidence and identical total energy.

The regression deliberately combines a `1e16 J` body with two `1 J` bodies, where naive floating-point summation order can change the low bits.

### W2 — duplicate identity rejection

If two dynamic bodies expose the same `BodyHandle`, the measurement must return `DuplicateBodyHandle` rather than count both.

This is important because `PhysicsWorld::bodies` is public and evidence may be captured after deserialization or direct mutation.

### W3 — body failure attribution

If one body cannot produce checked 3D energy, the world error must include both the body handle and the underlying `RigidBodyEnergyError`.

### W4 — aggregate representability

Individually finite body energies may still overflow the world total. That interval must return `UnrepresentableTotal` rather than `Infinity`.

### W5 — non-invasive authority boundary

No method in this module may mutate body state, stepping state, contacts, caches, or world ordering.

## Relationship to reconciliation

This module creates the correct 3D kinetic-reservoir measurement primitive for a future 3D `EnergyStateSnapshot` profile.

That future profile should carry per-body energy keyed by stable identity and should not silently replace the existing N-D snapshot semantics. It must be introduced under its own version/profile contract so old evidence remains reproducible.

## Remaining work

Before world-level anisotropic dynamics can be called production-ready:

- shape constructors must provide correct principal moments;
- compound bodies must use full parallel-axis mass properties;
- the production integrator must evolve anisotropic angular momentum correctly;
- contacts/joints/friction/CCD must use the same world inverse-inertia convention;
- 3D kinetic measurements must be reconciled against typed transfers in matched campaigns.

## Claim boundary

After this tranche passes, the correct claim is:

> Symtropy has a deterministic, overflow-checked world-level view of the represented 3D kinetic reservoir using exact checked per-body anisotropic energy.

It is not evidence that the production solver itself already evolves those anisotropic dynamics correctly.
