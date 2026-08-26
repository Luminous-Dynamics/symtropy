# Symtropy 3D Primitive Mass-Properties Validation

## Status

This contract defines analytical, checked body-space mass properties for centered uniform-density 3D primitives.

It is intentionally separate from production stepping. These formulas establish what inertia **should be stored** for primitive bodies; they do not by themselves make the generic integrator or contact solver anisotropic.

## Authority boundary

The module owns analytical geometry-to-principal-inertia conversion for:

- solid sphere;
- solid cuboid / `HyperBox<3>` geometry expressed as half-extents;
- solid capsule matching the `symtropy_math::Capsule<3>` definition.

Compound bodies are deliberately excluded. Their center of mass and inertia generally require tensor addition, child rotations, and the full parallel-axis theorem; the existing scalar `merge_toolhead()` approximation is not promoted here.

## Uniform solid sphere

For mass `m` and radius `r`:

`Ixx = Iyy = Izz = 2/5 m r^2`.

Required controls:

- `m=10`, `r=2` -> all moments `16`;
- non-positive/non-finite mass or radius is rejected;
- finite geometry whose squared radius/inertia is unrepresentable is rejected.

## Uniform solid cuboid

`HyperBox<3>` stores half-extents `[hx, hy, hz]`; full side lengths are `[2hx, 2hy, 2hz]`.

Therefore:

- `Ixx = m/3 (hy^2 + hz^2)`
- `Iyy = m/3 (hx^2 + hz^2)`
- `Izz = m/3 (hx^2 + hy^2)`

Required analytical control:

`m=12`, half-extents `[1,2,3]` -> moments `[52,40,20]`.

Every half-extent must be finite and strictly positive for this solid-volume primitive contract. Degenerate plates/rods need their own explicit model rather than being smuggled through zero thickness.

## Uniform solid capsule

The math crate defines a capsule as a line segment of length `2h` swept by a sphere of radius `r`. The solid is therefore a cylinder of length `2h` plus two solid hemispheres.

For uniform density, after cancelling the common `pi r^2` factor, mass partitions by:

`m_c : m_s = 2h : 4r/3`

where `m_c` is cylinder mass and `m_s` is the combined mass of the two hemispheres.

For the capsule symmetry axis:

`I_axis = 1/2 m_c r^2 + 2/5 m_s r^2`.

For either perpendicular principal axis:

`I_perp = (m_c/12)(3r^2 + 4h^2) + m_s[83/320 r^2 + (h + 3r/8)^2]`.

The cap term uses the solid-hemisphere center-of-mass offset `3r/8`; `83/320 m_h r^2` is the transverse hemisphere inertia about its own COM, with the two equal hemispheres combined into `m_s`.

Required controls:

- `h=0` reduces to the exact sphere formula;
- changing capsule axis only permutes principal moments;
- `m=10`, `h=2`, `r=0.5`, axis Y gives approximately `I_axis=1.2142857142857142` and `I_perp=18.892857142857142`;
- negative half-height, invalid radius, or axis outside `[0,2]` is rejected;
- mass-fraction computation is scale-normalized so extreme finite dimensions do not overflow merely while forming the volume ratio;
- any unrepresentable geometry or inertia arithmetic fails rather than emitting Infinity.

## Numerical policy

The same representability doctrine applies here as in thermal and angular reference work:

> finite inputs do not imply representable derived physics.

Constructors must validate both inputs and derived moments before returning `MassProperties3`.

## Next integration gate

After these formulas are compiler/test green, add checked `RigidBody<3>` primitive constructors that pair the matching collider geometry with these mass properties atomically. Do not silently alter the existing generic constructors in the same tranche.

Then validate:

- collider dimensions exactly match the mass-property dimensions;
- stored `mass`, `inv_mass`, `inertia`, and `inv_inertia` are mutually consistent;
- exact checked body/world energy sees the expected moments;
- invalid geometry produces no partially constructed body.

## Compound follow-up

A separate tranche should define full compound mass properties. It must account for:

- child mass allocation/density policy;
- child center of mass;
- child orientation;
- rotation of each child inertia tensor into the parent frame;
- `I_parent += R I_child R^T + m[(d·d)I - d d^T]`;
- resulting parent COM and principal-axis representation.

The current scalar `merge_toolhead()` path must remain explicitly approximate until that work passes.

## Claim boundary

After this tranche passes, the correct claim is:

> Symtropy has checked analytical principal moments for centered uniform-density sphere, cuboid, and capsule primitives in 3D.

Do not claim that every live body already uses those moments until the constructor migration is complete.
