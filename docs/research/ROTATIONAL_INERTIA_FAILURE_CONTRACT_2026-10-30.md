# Rotational inertia failure contract

**Status:** experimental API hardening, stacked on the asymmetric-inertia implementation.
**Date:** 2026-10-30
**Parent implementation:** [PR #2927](https://github.com/Luminous-Dynamics/symtropy/pull/2927)

## Problem

An authoritative inertia operator can be singular for a rotation plane (for example, a degenerate mass distribution). The previous integration path incremented a global counter and substituted zero angular acceleration while still advancing translation. Contact effective-mass helpers similarly converted a failed inverse into a numeric zero. Those behaviors keep the process running, but make an invalid physical response easy to mistake for a valid zero response.

## Changes in this increment

- `try_integrate` exposes the inertia error and computes the rotational solve before committing translational or angular state. If the authoritative inertia operation fails, the body pose, velocities, and force/torque accumulators remain unchanged.
- The existing `integrate` signature remains as a compatibility wrapper. On failure, it rejects the whole body step and clears the accumulators; it no longer advances translation while discarding the angular response.
- `try_apply_angular_impulse` returns a singular/invalid inertia error without changing angular velocity.
- `try_rotational_inverse_effective_mass` differentiates a valid zero contribution (such as a static body) from a failed inverse.
- Existing infallible helpers remain for API compatibility and are explicitly documented as lossy wrappers. They must not be used by new solver code that needs a correctness or failure guarantee.

## Validation contract

The regression cases require:

1. A singular authoritative rotational solve returns `SingularInertia`.
2. A failed `try_integrate` call leaves the body's pose, linear velocity, angular velocity, and accumulated force/torque unchanged.
3. The compatibility `integrate` wrapper rejects the complete body step rather than applying only its translational half.
4. Angular impulse and rotational contact effective-mass APIs expose the singular error instead of returning an apparently valid zero.

These source-level regression tests are included in the implementation. They have not been declared passing until the exact branch head completes formatting, compilation, Clippy and the physics test suite.

## Explicit remaining work

This is not yet an island-atomic failure contract. The existing contact-world path still uses the compatibility scalar effective-mass helper; it therefore can record a failure and treat the rotational contribution as zero. Before promotion:

- migrate contact, friction, restitution and joint row assembly to the fallible helpers;
- propagate typed errors from row assembly through the island/world step;
- stage the candidate velocities and poses for the entire island, then commit only after all rows and invariants validate;
- replace a process-global count as the only operational signal with per-world/per-step error receipts and offending body/row identifiers;
- add negative-control tests proving singular inertia cannot produce a completed contact event or an apparently valid solver receipt.

A failure in one body or row must not create a partial island update. Recovery policy (reject step, reduce timestep, regularize only under a declared physical model, or quarantine the body) belongs to the world/application layer and must be recorded, not silently selected inside the inertia operator.

## Promotion rule

No default runtime path may rely on the lossy compatibility wrappers for validated-inertia bodies. Zero response is permitted only when it follows from a valid physical condition (for example, a static body or a zero lever arm), not as a substitute for a failed solve. The implementation remains experimental until the world-level contract and exact-head tests pass.
