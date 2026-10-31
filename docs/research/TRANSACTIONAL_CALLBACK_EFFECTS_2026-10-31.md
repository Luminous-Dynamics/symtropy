# Transactional callback effects

**Status:** first two-phase callback publication path implemented.
**Date:** 2026-10-31
**Parent implementation:** [PR #3012 — mass-property consistency and cumulative contact verification](https://github.com/Luminous-Dynamics/symtropy/pull/3012)

## Problem

The world-step transaction already restores physics-owned body state, contact caches, manifolds and events when an inertia operation fails. However, the contact resolver previously called mutable callback hooks immediately during the solve:

- `record_dissipation`;
- `on_collision`;
- `apply_trauma`.

A later contact could fail after an earlier contact had invoked these methods. The world would roll back, but an external consciousness/telemetry/event store could retain a collision or dissipation record for an impact that was never committed.

## Implemented two-phase model

1. The solver continues to query `modulate_force`, `modulate_impulse`, and `friction_multiplier` while calculating the candidate step.
2. Mutable observations are represented as `PhysicsCallbackEffect` values and accumulated in a local buffer.
3. The world runs all fallible physics phases.
4. On success, `PhysicsCallback::commit_step_effects` receives the ordered buffer once.
5. On failure, the world restores its physics snapshot and drops the buffer without calling mutable effect hooks.

Effect order is preserved. A contact's friction-dissipation event remains before its collision event; the collision event dispatches `on_collision` followed by `apply_trauma`.

## Callback extension contract

The three solver modulation methods take `&self` and are now explicitly required to be pure and deterministic for fixed inputs. They may be called multiple times and a step may later reject. Implementations must not use interior mutability to publish irreversible side effects from these methods.

The default `commit_step_effects` implementation dispatches the batch through the legacy callback hooks, preserving source compatibility. Implementations that require atomic external publication should override the batch method and commit the supplied list as one transaction in their own store.

The physics world cannot guarantee atomicity inside arbitrary external callback code. The default per-effect dispatcher preserves legacy behavior but can still partially publish if a callback panics or its own side effects fail. A callback requiring stronger guarantees must implement atomic batch publication; callback methods currently have no error return.

## Regression target

The added regression creates two independent contact pairs:

- an earlier valid sphere impact with tangential slip, which generates collision and friction-dissipation effects;
- a later block/half-space contact with singular rotational inertia, which rejects the world step.

The expected result is:

- a typed contact-solve rejection;
- exact restoration of the physics-owned state covered by the world snapshot;
- zero callback batch commits;
- zero collision notifications, trauma notifications and dissipation recorded by the callback.

This is the key negative control: an earlier successful contact solve must not leak events when a later contact rejects the *world step*.

A separate existing successful sphere/plane test continues to require friction dissipation to reach the callback after a committed step.

## Limits and next steps

- Callback modulation purity is a documented behavioral contract, not enforceable by Rust's type system because implementations can use interior mutability.
- The compatibility default dispatches effects sequentially. Strong atomicity of external state requires overriding `commit_step_effects`.
- Callback panic handling is not converted into a recoverable world error; a panic during post-commit publication occurs after physics has committed.
- `record_work` is not yet emitted by the solver because actuator/joint work attribution is not implemented.
- A future typed callback transaction should include prepare/commit/abort or a fallible durable sink, plus a deterministic step sequence and receipt digest.

The change closes the specific rollback leak in mutable callback hooks; it does not claim a general transaction across arbitrary external systems.
