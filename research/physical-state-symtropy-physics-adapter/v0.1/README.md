# Symtropy Physics State Adapter v0.1

A read-only adapter from the native symtropy-physics PhysicsWorld<3> to the physical-state-feedback contract.

## Conservative native-state mapping

The adapter maps native state without inventing missing physical quantities:

- Pose and velocity are tagged PREDICTED with engine-build provenance, never MEASURED.
- Linear acceleration is UNAVAILABLE; the native RigidBody does not retain an acceleration state. The adapter does not finite-difference the velocity.
- Mass is ASSUMED when the body has a positive configured mass. Static bodies' zero mass is a solver sentinel and is returned as UNAVAILABLE.
- Inertia tensor is UNAVAILABLE. Although RigidBody stores a per-axis vector, the current integrator averages inertia across each bivector plane for angular dynamics. The adapter will not claim that this is a full physically applied anisotropic inertia tensor.
- Angular velocity is mapped from the native bivector matrix using the axial dual consistent with Bivector::apply_to_vector: [-B[1,2], B[0,2], -B[0,1]].
- Contact partners come from the last-step contact manifolds. Sensor bodies and snapshots whose contact-cache freshness is not attested expose contact as UNAVAILABLE. The scalar per-point normal solver multiplier is not converted into a vector impulse because the adapter does not assert a per-body impulse convention.
- Stable entity IDs and generations must be supplied by the caller. A local BodyHandle is not promoted into a globally stable physical identity.

## Caller contract

Provide:
- immutable snapshot identity (the digest is caller-supplied in v0.1; canonical snapshot serialization/hash is not implemented);
- a one-to-one map from each current BodyHandle.0 to stable entity ID and generation;
- an explicit contacts_fresh assertion indicating whether the public world.contacts vector belongs to the declared snapshot.

The adapter validates the map and the produced snapshot. Queries are read-only, bind to the exact supplied snapshot digest, and return requested bodies plus their transitive contact context.

## Limitations

No external hardware, independent motion-capture reference, force measurement, canonical snapshot serializer, action-command execution path or physical transfer result is included. This is a native-engine observability adapter, not a physical-fidelity pass.
