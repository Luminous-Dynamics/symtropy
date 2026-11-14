# Rotational/translational mass-property consistency

**Status:** experimental hardening, stacked on the contact-verification work.
**Date:** 2026-10-30
**Implementation branch:** `research/mass-property-consistency-2026-10-30`

## Problem

A rotational second-moment matrix (S=\int rr^T dm) does not, by itself, uniquely determine total mass. However, constructors such as uniform solid ellipsoids and point-mass clouds know the mass used to produce (S). Previously that information was discarded. A valid inertia operator built for 2 kg could therefore be attached to a 1 kg `RigidBody`, causing the translational and rotational blocks of the generalized mass matrix to describe different physical objects.

This is more than metadata hygiene: it changes angular acceleration, impulse response, contact effective mass, kinetic energy and any Delassus matrix assembled from the same body.

## Implemented contract

- `BivectorInertia<D>` now carries optional `total_mass` metadata.
- `uniform_solid_ball`, `uniform_solid_ellipsoid`, `from_point_masses`, and `combine_at_common_center` retain known total mass.
- `from_second_moment` remains available but marks mass as unknown. Mass cannot be recovered uniquely from a raw second-moment matrix without additional geometric/density assumptions.
- `from_second_moment_with_mass` is available when the caller knows both S and total mass.
- `validate_body_mass` compares known inertia mass with body translational mass using a relative tolerance.
- `RigidBody::try_set_rotational_inertia_operator` validates before replacing the operator. The legacy setter remains for compatibility.
- `RigidBody::try_merge_toolhead` validates existing and component mass properties before changing collider geometry, COM, mass or inertia. The legacy void merge wrapper is a no-op on invalid input rather than dropping an authoritative inertia operator.
- Fallible free integration, angular impulse, rotational effective mass, and reference Delassus assembly reject a known mass mismatch.
- `InvariantSnapshot` and `InvariantDrift` report the number of known body/inertia mass mismatches. Such a state is not numerically healthy.

## Failure semantics

A mismatch returns `RotationalInertiaError::MassMismatch`. In the fallible world path this is propagated as a typed physics-step error, and the world-owned state is rolled back. No automatic rescaling of S is performed: silently rescaling inertia would change the declared mass distribution and hide an upstream modeling error.

## Compatibility boundary

Raw `from_second_moment` operators keep `total_mass=None` and remain accepted by the legacy API. This is intentionally weaker than the mass-aware path because no truthful total mass is available to validate. New CAD/primitive/point-cloud construction should use mass-aware constructors wherever possible.

The legacy `set_rotational_inertia_operator` can still attach an inconsistent mass-aware operator for source compatibility, but all fallible simulation/contact paths validate it and fail closed. New code should use `try_set_rotational_inertia_operator`.

## Regression coverage

The branch adds:
- metadata retention for ellipsoid and point-cloud constructors;
- mass metadata preservation under body-frame rotation;
- explicit acceptance of equal masses and rejection of mismatches;
- raw-S unknown-mass compatibility;
- parallel-axis combination rejection when component mass arguments disagree with known operator mass;
- setter rejection without replacing the previous valid operator;
- toolhead-merge rejection without changing collider support, COM, mass or inertia;
- transactional integration rejection before body mutation;
- angular-impulse and rotational-effective-mass mismatch rejection;
- Delassus reference rejection for mismatched bodies;
- invariant-snapshot detection and health failure.

These are source-level tests only until the exact PR head completes formatting, Clippy, compilation and physics tests.

## Further work

1. Associate mass-property provenance with its source geometry, density model, units and COM frame, not just its total mass.
2. Reject or explicitly migrate mass mismatches in the legacy setter once downstream call sites are audited.
3. Validate inertia/mass consistency when deserializing replay snapshots and importing CAD/mesh mass properties.
4. Include mass-property provenance and mismatch counts in simulation evidence receipts.
5. Extend tests to compounds, articulated links, mesh tetrahedralization and toolhead merges.

Mass agreement is necessary, not sufficient: it does not prove that S came from the body's actual collider geometry or density distribution. Provenance and geometric mass-property extraction remain separate qualification gates.
