# Robotics Base Observation Layout v0

**Status:** named input-layout contract for the existing 13-value haptic/HDC projection. This is not the complete Careframe observation contract and it is not evidence of robot safety or perception performance.

## The problem

The generic Bevy robotics bridge previously constructed an anonymous 13-element vector inline:
- position (3)
- quaternion (4)
- linear velocity (3)
- 3D angular-velocity bivector components (xy, xz, yz)

Because the vector had only numeric offsets, the producer and consumer could silently disagree about ordering. The code also retained zero-filled velocity values when a physics body was missing, creating a plausible-looking but unsupported stationary-state observation.

## Contract added by this change

The permissively licensed `symtropy-robotics-bridge-core::observation` module defines:
- `BaseKinematicObservationV0`: named fields with declared units and coordinate semantics;
- `BASE_KINEMATIC_OBSERVATION_FIELD_IDS`: the 13 field identifiers in exact projection order;
- `BASE_KINEMATIC_OBSERVATION_DIM`: the single source of truth for input dimensionality;
- `try_to_ordered_values()`: fixed-order conversion with fail-closed checks for NaN/infinity and a degenerate/non-unit quaternion.

The Bevy bridge now constructs this typed object and uses the same dimension constant for the haptic semantic binder. If the physics body is unavailable or the frame fails validation, it clears the current HDC and legacy observation input rather than substituting zeros or retaining a stale frame.

## Exact 13-value order

1. base position x/y/z in metres (world coordinates);
2. quaternion x/y/z/w;
3. base linear velocity x/y/z in metres/second (world coordinates);
4. base angular-velocity bivector xy/xz/yz in radians/second.

The final three entries are *bivector-plane components*, not an unlabeled Cartesian xyz angular-velocity vector. Downstream adapters must not rename or reinterpret them without an explicit conversion and tests.

## Known scope limitations

This is deliberately a base-kinematics-only HDC input. It does **not** include:
- individual joint positions or velocities;
- tool ID/revision and tool-frame pose;
- task target and task phase;
- explicit safety-authority state;
- object/target/obstacle geometry, visibility or perception confidence;
- proximity detections in the haptic-HDC input (the legacy text path still reports a proximity count).

Therefore it is **not eligible by itself** for the full 18-field matched-semantic-state Careframe comparison. See the separate Careframe contract audit in the research PR: https://github.com/Luminous-Dynamics/symtropy/pull/2723. A complete policy-input adapter must be implemented and instrumented before claiming semantic observation equivalence across state, RGB-derived and point-cloud-derived paths.

## Tests added

- stable field order and dimension;
- exact mapping from named inputs to ordered values;
- non-finite component rejection with a deterministic component index;
- degenerate and non-unit quaternion rejection;
- field-ID uniqueness and non-emptiness.

These are local value/contract tests. They do not prove the HDC model interprets the embedded values correctly, do not add missing sensor channels, and do not establish perception accuracy, safety or sim-to-real performance.
