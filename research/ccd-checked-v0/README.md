# Continuous Collision Detection Input Contract v0

This slice strengthens the analytical sphere-sphere and sphere-halfspace CCD primitives without breaking the existing Option-returning API.

## Paradigm change: no-hit is not invalid-input

The legacy APIs return Option<CcdHit>. That type cannot distinguish a valid query with no collision from a malformed query (NaN position, negative radius, negative timestep, zero plane normal). The new checked APIs return Result<Option<CcdHit>, CcdInputError> so callers can fail closed and preserve diagnostic meaning.

Legacy functions remain source-compatible and delegate to the checked path. They return None for invalid inputs. New engineering code should call the checked variants.

## Checked API contract

- Positions, radii and plane offsets use one consistent length unit.
- Velocities use that length unit per second.
- Timestep is a finite, non-negative number of seconds.
- Radii are finite and non-negative.
- All position, velocity and normal components are finite.
- A plane normal must be nonzero; it is normalized internally and the plane offset is scaled by the same norm.
- A returned plane contact normal is unit length.
- A TOI is expressed in seconds in the closed interval [0, dt], not as a normalized fraction. The old field comment incorrectly described it as [0, 1]; it is corrected.
- Non-finite derived outputs and detected arithmetic overflow are explicit errors in the checked APIs.

## Tests added

1. Sphere-sphere head-on: exact TOI and geometric contact-distance residual.
2. Sphere-sphere grazing: tangent contact with zero discriminant.
3. Invalid vs no-hit negative control: negative radius, negative timestep and NaN position are errors; a valid separated trajectory is Ok(None).
4. Non-unit plane normal and scaled offset: a plane encoded as 2*y - 4 = 0 still produces the same y=2 contact plane and a unit normal.
5. Zero normal and non-finite offset: typed input errors.
6. Sphere-plane TOI residual: the impact center lies exactly one radius from the plane, and the returned contact point lies on the plane.

All previous CCD tests continue to exercise the backwards-compatible API.

## Current limitation

The checked routines validate the inputs and returned values for the analytic sphere-sphere and sphere-halfspace query families. They do not prove the entire physics world's CCD integration is tunnelling-free for rotating convex bodies, mesh contacts, deformables, or multi-contact scenes. They also do not replace a robust general conservative-advancement algorithm.

## Next qualification cases

- Adversarial scale sweep from micrometre-scale to kilometre-scale geometry.
- Grazing/tangent and nearly tangent collisions with controlled geometric condition numbers.
- Time-of-impact residual across 1D/2D/3D/4D and randomized generated cases.
- Translation and rotation sweeps; compare swept convex geometry to a trusted reference.
- Tunnelling failure rate vs speed, feature size, timestep and solver profile.
- Differential validation against Rapier, MuJoCo and PhysX only where contact semantics and models match.

No performance benchmark or industry-superiority claim is made by this slice.