# FEP-07 prerequisite — restore authoritative fixed physics stepping

## Why this prerequisite exists

The launcher already registers `thermodynamic_enforcement_system` followed by `physics_sync_transforms` in Bevy `FixedUpdate`, but the pre-FEP-07 `physics_sync_transforms` implementation only copied body positions into Bevy `Transform`s. It did not advance `PhysicsWorld`.

That matters because the 2D FEP movement path writes `RigidBody::linear_velocity`. Without an authoritative world step, changing that velocity is not sufficient to advance body position through the physics engine.

The core engine already owns the correct integration API:

- `PhysicsWorld::step(dt)` for ordinary physics;
- `PhysicsWorld::step_with_callback(dt, callback)` when the consciousness/integration-field callback is enabled.

FEP-07 must therefore restore/verify the 2D physics integration boundary before moving more authored AI into the fixed schedule.

## Authority contract

The launcher currently has two different movement authorities that share `PhysicsWorldRes` representation:

- `GamePhase::Playing` (2D) uses dynamic physics bodies and is the phase this prerequisite authorizes to execute `PhysicsWorld` integration;
- `GamePhase::Playing3D` uses the Old Waterworks kinematic controller, which writes its visual transform and shared physics-body position directly.

The existing fixed chain runs in both phases, so `physics_sync_transforms` must preserve that distinction rather than turning 3D bodies into newly dynamic actors by accident.

For `GamePhase::Playing`, `physics_sync_transforms` now means:

1. validate the fixed delta;
2. advance the authoritative 2D physics world exactly once;
3. when `consciousness-runtime` is enabled, perform the step through `ConsciousnessField` as the `PhysicsCallback`;
4. otherwise use the ordinary physics step;
5. export the post-step body positions into Bevy `Transform`s.

For `GamePhase::Playing3D`, this adapter does **not** step the world. It only mirrors the already-authoritative kinematic body representation back to the visual transform, preserving the pre-existing 3D controller contract.

A Bevy transform in the 2D slice is therefore a projection of post-integration physics state, not a substitute for the physics step.

## Fail-closed time boundary

`step_physics_world` refuses:

- zero delta;
- negative delta;
- NaN;
- infinity.

An invalid scheduler time sample cannot mutate authoritative physics and then be presented as a legitimate transform update.

## Regression theorems

The unit-level prerequisite tests require that:

- invalid step durations are rejected;
- a physics body with nonzero linear velocity changes authoritative position after one valid step;
- only `GamePhase::Playing` grants this adapter world-integration authority; `Playing3D`, loading, and menu phases do not.

These prove the primitive is not a no-op and that the new integration authority is phase-scoped. They do **not** yet prove that the complete launcher/FEP schedule is replay-identical.

## Relationship to FEP-07

The full 2D fixed authored-AI target remains:

`thermodynamic_enforcement_system`
→ `fep_behavior_system`
→ `npc_action_system`
→ `npc_movement_system`
→ authoritative physics step
→ transform export

The current prerequisite establishes the last two operations behind the existing `physics_sync_transforms` registration point. The FEP systems are still registered in `Update` on this tranche and must be moved deliberately in the next scheduling change.

A separate player-input boundary is also still required: the 2D `player_movement_system` buffers `PlayerInput`, but this tranche does not invent a speed/force policy or claim that buffered intent is already consumed by the physics world.

## Qualification required

Before treating this as runtime evidence:

- `cargo fmt --check`;
- default launcher check/test so the non-consciousness fallback compiles;
- launcher check/test with `--features fep-ai` so `step_with_callback` compiles against `ConsciousnessField`;
- Clippy on both relevant feature surfaces;
- a 2D launcher scenario proving a body velocity written before the fixed boundary changes its exported transform afterward;
- a 3D regression scenario proving the Old Waterworks kinematic controller is unchanged by this phase-scoped integration gate.

No such exact-head execution is claimed by this document.
