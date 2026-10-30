# World-step failure propagation and rollback contract

**Status:** experimental implementation stacked on the fallible inertia APIs.
**Date:** 2026-10-30
**Parent work:** [PR #2979](https://github.com/Luminous-Dynamics/symtropy/pull/2979), [PR #2927](https://github.com/Luminous-Dynamics/symtropy/pull/2927)

## Problem

The per-body fallible APIs are insufficient if the world continues to call compatibility wrappers. A failure can occur after earlier bodies or contact points have already been updated. Returning a typed error without rolling back those earlier mutations would still expose a partially committed simulation step.

## Implemented path

- `PhysicsWorld::try_step(dt)` executes the no-op-callback step and returns `Result<(), PhysicsStepError>`.
- `PhysicsWorld::try_step_with_callback(dt, callback)` exposes the same typed error path when callback integration is required.
- Free-body integration uses `integrator::try_integrate`.
- Warm-start angular impulses use `try_apply_angular_impulse`.
- Normal and tangential contact effective-mass calculations use `try_rotational_inverse_effective_mass`.
- Normal and friction angular impulses use `try_apply_angular_impulse`.
- A failed step restores the pre-step body transforms, linear/angular velocities, force/torque accumulators and sleep state; contact manifolds; collision and sensor events; normal warm-start caches; and the static-broadphase dirty bit.
- The contact cache is cloneable specifically to make rollback explicit rather than reconstructing a previous cache heuristically.
- Existing `step` and `step_with_callback` signatures remain. They invoke the fallible path; if it fails they clear accumulators. Callers requiring error visibility and retry semantics must use the `try_` APIs.

## Callback boundary

The physics-owned state can be restored, but arbitrary external side effects from callback methods cannot be rolled back through the current `PhysicsCallback` trait. In particular, a callback may already have observed a force modulation or recorded a collision/dissipation from an earlier contact before a later contact fails. The new docs explicitly state this limitation.

A future fully transactional callback contract should buffer callback observations and publish them only after the physics commit, or let callbacks implement prepare/commit/abort semantics. The no-op `try_step` has no external callback side effects and is the strongest current atomicity path.

## Ordering invariant

All fallible inertia operations occur during free integration, warm-start, and contact solving, before the existing non-fallible joint/constraint solver phases. The current rollback contract relies on that phase ordering. Any future fallible operation added after constraint solving must either be moved before the commit boundary or cause the snapshot to include the affected constraint state.

## Regression target

The included world-level test places a valid body first and a singular-inertia body second. The first body is advanced before the second body fails. The expected result is a typed `SingularInertia` error and exact restoration of the first body's pose, velocity and force accumulator, plus restoration of the failing body's state and torque accumulator.

The per-body tests additionally require that failed `try_integrate` and angular-impulse operations do not mutate their target body.

## Non-goals and promotion blockers

This is not the island-level discrete Delassus solver described in [PR #2966](https://github.com/Luminous-Dynamics/symtropy/pull/2966). It does not prove contact complementarity, friction maximum dissipation, or energy conservation. It also does not provide rollback for arbitrary callback-owned state or internal state hidden inside constraint implementations.

Before promotion:

1. Run formatting, Clippy, compile and physics tests at the exact head.
2. Add contact-failure rollback fixtures (not only integration-stage failure).
3. Add callback buffering or prepare/commit/abort semantics.
4. Include deterministic receipts for rejected steps, including failing body/contact identifiers.
5. Prove the rollback contract remains valid as new fallible solver phases are introduced.

No CI or numerical test pass is claimed by the source changes alone.
