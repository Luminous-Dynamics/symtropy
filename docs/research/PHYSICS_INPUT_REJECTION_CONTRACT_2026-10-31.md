# Physics input rejection contract

**Status:** experimental fail-closed validation for callback and contact inputs.
**Date:** 2026-10-31
**Implementation branch:** `research/physics-input-rejection-2026-10-31`

## Problem

A simulation engine must not silently reinterpret malformed inputs as valid physical data. The callback boundary previously accepted values that could reverse a unilateral contact impulse, create a negative Coulomb friction bound, propagate NaNs into forces, or trigger a panic in `f64::clamp`. A later zeroing/sanitizing guard can hide the original source of the defect.

## Typed step-cause taxonomy

`PhysicsStepError` now carries a `PhysicsStepCause`:

- `RotationalInertia(error)` — existing singular/invalid mass-property failure.
- `InvalidTimeStep` — `dt` is non-finite or not strictly positive.
- `NonFiniteCallbackOutput` — force/impulse/friction callback output is NaN or infinite, or a generated impulse is non-finite.
- `ImpulseDirectionReversal` — callback modulation reverses the sign of a unilateral normal-impulse update.
- `NegativeFrictionCoefficient` — either body's Coulomb friction coefficient is negative or non-finite.
- `NegativeFrictionMultiplier` — callback friction modulation is negative.
- `NonFiniteContactMetric` — effective-mass terms, impulse accumulations, friction bounds or derived contact values overflow or become invalid.

The cause and the existing phase/body-pair fields make failures actionable without parsing logs.

## Accepted callback semantics

- Force modulation may scale or otherwise modulate a force, but its output must remain finite.
- Normal impulse modulation may attenuate or amplify the signed update, but it must preserve the sign of the unilateral solver update. A callback cannot turn a push into a pull.
- Body friction coefficients and callback friction multipliers must be finite and non-negative. Values above one are permitted; overflow in the resulting coefficient/bound is rejected.
- A zero friction multiplier is valid and means no friction.
- The solver rejects invalid `dt` before taking or mutating the world snapshot.

These checks deliberately do not clamp or repair malformed values. They return a typed error and rely on the existing world transaction to restore the state and discard buffered callback effects.

## Regression coverage

The source adds negative controls for:
- non-finite force callback output, with exact body/accumulator rollback;
- reversed normal impulse, with contact-step rollback;
- negative callback friction multiplier, proving the solver rejects rather than panics;
- negative body friction coefficient;
- zero, negative, NaN and infinite timesteps;
- overflow in normal-lambda accumulation, total normal impulse, tangential speed, friction bounds and applied friction impulse.

The existing valid paths remain covered by sphere impact, friction-dissipation and callback-commit tests.

## Compatibility and limits

- `PhysicsStepError.cause` changes from a rotational-inertia-specific error to the broader `PhysicsStepCause` enum. Downstream exhaustive matches need to handle the new variants and wrap the original inertia error as `RotationalInertia(error)`.
- The world-level failure remains transactional for physics-owned state and buffered mutable callback effects. Modulation callbacks are contractually pure/deterministic; Rust cannot prevent interior-mutability side effects in their `&self` implementations.
- This does not yet validate every collider/material parameter at construction time, and it does not replace model-level unit/physical-domain checks.
- Exact-head CI and the regression suite must pass before this contract is promoted.
