# Runtime Physics Authority

## Status

This document defines the staged migration that makes the Symtropy launcher a
continuous dogfood client of `symtropy-physics` rather than a presentation layer
that merely mirrors or carries physics handles.

The first implementation lives in `src/systems/engine_physics.rs` and is scoped
to the 2D `GamePhase::Playing` experience. Old Waterworks remains explicitly a
3D presentation with a 2D horizontal proxy until a dedicated `PhysicsWorld<3>`
runtime is introduced.

## Canonical tile contract

Procedural generation authors the shared tile map as:

| Code | Meaning | Traversable |
| ---: | --- | --- |
| 0 | wall | no |
| 1 | floor | yes |
| 2 | core room | yes |
| 3 | player start | yes |

`SiteLayout` is the authoritative authored map. `TileGrid` is derived state.
The launcher previously populated the grid with `walkable = cell != 1`, which
made walls traversable and ordinary carved floor blocked.

`src/systems/procgen.rs` now owns the canonical `tile_code_is_walkable` predicate
and uses it when constructing `TileGrid`. The FixedUpdate bridge imports the same
predicate and still reconciles derived state from `SiteLayout` defensively, so a
legacy/stale grid cannot silently reintroduce the old inversion.

The 2D renderer still has presentation semantics to normalize separately; that
visual cleanup is not collision authority and should not block R1 physics
validation.

## R1: authoritative 2D fixed-step loop

For `GamePhase::Playing`, every FixedUpdate performs this sequence:

1. Reconcile `TileGrid` with `SiteLayout`.
2. Convert buffered `PlayerInput` into a bounded body velocity.
3. Apply the current motor-authority gain; collapsed/Red state cannot move.
4. Clamp the prospective player motion against the temporary tile boundary
   adapter.
5. Wake dynamic bodies carrying non-zero velocity intent, including FEP NPCs.
6. Advance `PhysicsWorld<2>` exactly once using Bevy fixed time.
7. When `consciousness-runtime` is enabled, rebuild the position-dependent
   harmony field and use `PhysicsWorld::step_with_callback`; otherwise use the
   pure physics `step` path.
8. Synchronize Bevy transforms from authoritative body positions.

The player controller preserves sub-unit input magnitude so continuous AI/FEP
intent is not silently normalized to full speed, while keyboard diagonals are
capped at unit magnitude.

## Temporary boundary adapter

The R1 controller samples the player's bounding circle against `TileGrid` before
committing the velocity components. This is intentionally temporary.

It prevents the restored physics step from immediately allowing traversal
through authored walls, but it remains split authority: wall rejection occurs in
the launcher rather than as a solver contact.

R2 must replace this with static physics colliders (or a formally specified
kinematic boundary adapter) so collision response, provenance, impulse, friction,
and energy accounting all live in the physics substrate.

Before R2 expands static world ownership, `PhysicsWorld` also needs explicit
body-removal/reset and cross-domain experience teardown semantics; see #19.

## Sleeping contract

FEP NPC movement and the restored player controller assign desired linear
velocity directly. `RigidBody::integrate` skips sleeping bodies, and direct field
assignment does not itself wake them. The authority bridge therefore wakes any
dynamic body carrying non-zero velocity intent before stepping.

This should eventually become an explicit controller API on `RigidBody` /
`PhysicsWorld` rather than a launcher-side invariant.

## Full-runtime callback contract

Under `consciousness-runtime`, the physics callback depends on current entity
positions for harmony/friction coupling. The authority bridge rebuilds that field
from authoritative body positions immediately before the step and then calls
`step_with_callback`.

The default lightweight build intentionally uses pure `PhysicsWorld::step`
because its local fallback `ConsciousnessField` is not the production
`PhysicsCallback` implementation.

## Known sequencing debt

The existing plugin currently chains:

`thermodynamic_enforcement_system -> physics_sync_transforms`

The latter now contains the R1 physics step. This means collision callback
bookkeeping occurs after the current tick's thermodynamic finalization. Energy
state mutations and ledger writes still happen deterministically, but some
per-tick HUD/counter accounting can be observed on the following tick or reset
before presentation consumes it.

Do not hide this by duplicating thermodynamic finalization inside the bridge.
The correct follow-up is to split the authority bridge into explicit ordered
systems and establish one tick lifecycle, for example:

`intent -> pre-step fields -> physics step -> post-step thermodynamics -> sync`

with tests proving that collision energy, maintenance, regeneration, and HUD
counters refer to the same fixed tick.

## R2: solver-owned world boundaries

Exit criteria:

- generated walls become deterministic static `symtropy-physics` colliders,
- player-vs-wall and NPC-vs-wall are solver contacts rather than TileGrid
  rejections,
- TileGrid remains navigation/query data, not collision authority,
- player-vs-NPC and NPC-vs-NPC behavior is covered by deterministic scenarios,
- contact events and energy accounting identify the same body handles used by
  gameplay,
- repeated experience enter/exit cycles are bounded by the lifecycle contract in
  #19 rather than accumulating stale bodies.

## R3: true 3D Old Waterworks

Old Waterworks currently creates a 2D sphere body for the player and directly
writes horizontal body/Bevy transforms. R3 requires:

- a dedicated `PhysicsWorld<3>` resource/runtime,
- 3D static colliders for floors, walls, and physical machinery,
- a physical-character/controller path,
- 3D queries for interaction and camera support where appropriate,
- authoritative rigid-body orientation once the asymmetric-inertia migration is
  validated,
- deterministic capture behavior kept separate from simulation authority.

Until then, public descriptions should call it a 3D presentation with a 2D
horizontal physics proxy, not full 3D Symtropy dynamics.

## Validation gates

R1 is not validated merely because source exists. Before promotion:

- format and Clippy must pass,
- default and all-feature launcher builds must pass,
- engine-physics unit tests must pass,
- deterministic same-input runs must produce the same body-state digest,
- player cardinal/diagonal/analog intent must match the speed contract,
- Red/collapsed motor authority must produce zero commanded velocity,
- generated `0` border tiles must be blocked while `1/2/3` are traversable,
- owner playtest follows automated/capture validation rather than replacing it.

## Claim boundary

R1 means the 2D flagship runtime advances `symtropy-physics` authoritatively for
registered dynamic bodies. It does **not** mean:

- walls are already solver-owned,
- Old Waterworks is a true 3D physics world,
- the thermodynamic tick lifecycle is fully reconciled,
- world/experience teardown is production-ready,
- character-controller quality is competitive with Jolt or PhysX,
- the runtime stack has passed CI while the root stacked gate remains queued.
