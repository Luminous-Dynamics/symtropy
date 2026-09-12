# FEP-07 prerequisite — restore authoritative fixed physics stepping

## Why this prerequisite exists

The launcher already registers `thermodynamic_enforcement_system` followed by `physics_sync_transforms` in Bevy `FixedUpdate`, but the pre-FEP-07 `physics_sync_transforms` implementation only copied body positions into Bevy `Transform`s. It did not advance `PhysicsWorld`.

That matters because the FEP movement path writes `RigidBody::linear_velocity`. Without an authoritative world step, changing that velocity is not sufficient to advance body position through the physics engine.

The core engine already owns the correct integration API:

- `PhysicsWorld::step(dt)` for ordinary physics;
- `PhysicsWorld::step_with_callback(dt, callback)` when the consciousness/integration-field callback is enabled.

FEP-07 must therefore restore/verify the physics integration boundary before moving more authored AI into the fixed schedule.

## Authority contract

For the existing launcher fixed chain, `physics_sync_transforms` now means:

1. validate the fixed delta;
2. advance the authoritative 2D physics world exactly once;
3. when `consciousness-runtime` is enabled, perform the step through `ConsciousnessField` as the `PhysicsCallback`;
4. otherwise use the ordinary physics step;
5. export the post-step body positions into Bevy `Transform`s.

A Bevy transform is therefore a projection of the post-integration physics state, not a substitute for the physics step.

## Fail-closed time boundary

`step_physics_world` refuses:

- zero delta;
- negative delta;
- NaN;
- infinity.

An invalid scheduler time sample cannot mutate authoritative physics and then be presented as a legitimate transform update.

## Regression theorem

The unit-level prerequisite test creates a physics body with nonzero linear velocity, executes one valid step, and requires authoritative body position to advance.

This proves the primitive itself is not a no-op. It does **not** yet prove that the complete launcher/FEP schedule is replay-identical.

## Relationship to FEP-07

The full fixed authored-AI target remains:

`thermodynamic_enforcement_system`
→ `fep_behavior_system`
→ `npc_action_system`
→ `npc_movement_system`
→ authoritative physics step
→ transform export

The current prerequisite establishes the last two operations behind the existing `physics_sync_transforms` registration point. The FEP systems are still registered in `Update` on this tranche and must be moved deliberately in the next scheduling change.

## Qualification required

Before treating this as runtime evidence:

- `cargo fmt --check`;
- default launcher check/test so the non-consciousness fallback compiles;
- launcher check/test with `--features fep-ai` so `step_with_callback` compiles against `ConsciousnessField`;
- Clippy on both relevant feature surfaces;
- a small launcher scenario proving a body velocity written before the fixed boundary changes its exported transform afterward.

No such exact-head execution is claimed by this document.
