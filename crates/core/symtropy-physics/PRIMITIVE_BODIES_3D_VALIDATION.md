# Symtropy Checked 3D Primitive Body Construction Contract

## Status

This tranche binds validated primitive geometry and analytical mass properties into live `RigidBody<3>` state without changing production stepping or contact response.

It stacks on the analytical mass-property layer and the checked 3D body/world energy views.

## Purpose

A physically meaningful kinetic-energy measurement is only useful when the body's collider geometry, mass, principal inertia, inverse mass, and inverse inertia describe the same object.

The checked constructors provide that atomic binding for centered uniform-density:

- solid spheres,
- solid cuboids (`HyperBox<3>` half-extents),
- solid capsules (`Capsule<3>` half-height/radius/axis convention).

Existing generic constructors remain available for compatibility and are not silently reinterpreted by this tranche.

## Construction invariant

A successful checked constructor must establish all of the following from one validated input set:

1. finite world position;
2. finite positive mass;
3. validated primitive geometry;
4. matching analytical body-space principal moments;
5. finite positive inverse mass;
6. finite positive inverse principal moments;
7. collider geometry identical to the geometry used for mass-property derivation;
8. centered primitive COM convention;
9. live `RigidBody` state equal to the validated values after construction.

If any derived inverse or constructed field is not representable, the constructor returns an error and no partial body is returned.

## Required gates

### C1 — sphere geometry/property identity

For a checked sphere, the collider radius and the radius used by `MassProperties3::solid_sphere` must be the same value.

### C2 — cuboid geometry/property identity

For a checked cuboid, the collider half-extents and the half-extents used by `MassProperties3::solid_cuboid` must be identical.

### C3 — capsule geometry/property identity

For a checked capsule, half-height, radius, and principal axis must match the values used by `MassProperties3::solid_capsule`.

### C4 — checked energy integration

A body produced by a checked constructor must feed directly into `RigidBody<3>::kinetic_energy_3d_checked()` and produce the analytical anisotropic energy implied by its stored moments.

### C5 — fail-closed position and geometry

NaN/infinite position or invalid geometry must return a typed error rather than a partially valid body.

### C6 — inverse representability

A finite positive mass or principal moment is not sufficient if its reciprocal cannot be represented as a finite positive `f64`. Such construction must fail.

### C7 — constructed-state self-check

The completed `RigidBody` must be rechecked against the validated mass, inverse mass, inertia, inverse inertia, and position before being returned.

## Important compound boundary

These constructors deliberately cover only centered primitives whose body frame is also a principal-inertia frame.

A general compound with translated and/or rotated anisotropic children cannot be represented faithfully by merely summing three scalar principal moments in the current collider/body frame. Its correct body-frame inertia is a full symmetric 3×3 tensor:

`I = Σ [ R_i I_i R_i^T + m_i ((d_i·d_i) 1 - d_i d_i^T) ]`

about the compound center of mass.

Unless the aggregate tensor is diagonal in the chosen body frame, storing only `[Ixx, Iyy, Izz]` silently discards products of inertia. A later compound tranche must therefore either:

- add a full 3D inertia-tensor representation to production state, or
- explicitly rotate the body/collider into a certified principal-axis frame and preserve that frame relationship.

The current `merge_toolhead()` scalar approximation is not authoritative compound physics.

## Claim boundary

After this tranche passes executable validation, the correct claim is:

> Symtropy can atomically construct centered uniform-density 3D sphere, cuboid, and capsule bodies whose collider geometry and stored analytical principal inertia are mutually consistent and usable by the checked 3D energy evidence path.

Do not claim that production integration/contact response already uses anisotropic inverse inertia correctly.

## Next gates

1. full 3×3 compound inertia-tensor reference and parallel-axis composition;
2. decide the production representation for non-diagonal body-frame inertia;
3. versioned world/reservoir campaign using checked body/world energy;
4. angular-momentum production integrator migration;
5. anisotropic contact effective mass;
6. friction/joints/CCD migration.
