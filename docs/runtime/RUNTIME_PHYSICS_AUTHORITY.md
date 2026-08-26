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

For `GamePhase::Playing`, every FixedUpdate now has four explicitly chained
authority phases:

1. `thermodynamic_pre_step_system`
   - resets per-tick energy counters exactly once;
   - applies maintenance, ambient/well regeneration, collapse/safety state, and
     resonance/offloading needed before motor authority is evaluated.
2. `physics_step_system`
   - reconciles `TileGrid` with `SiteLayout`;
   - converts buffered `PlayerInput` into bounded body velocity;
   - applies the current motor-authority gain;
   - clamps prospective player motion against the temporary tile boundary
     adapter;
   - wakes sleeping dynamic bodies carrying external velocity intent;
   - rebuilds the harmony/callback field under `consciousness-runtime`;
   - advances `PhysicsWorld<2>` exactly once using Bevy fixed time.
3. `thermodynamic_post_step_system`
   - finalizes thermodynamics exactly once after physics callbacks have recorded
     same-tick collision/action/dissipation effects;
   - samples HUD consumption/regeneration only from the completed tick.
4. `physics_sync_transforms`
   - mirrors authoritative body positions to Bevy presentation transforms;
   - performs no control, solver, accounting, or thermodynamic mutation.

The player controller preserves sub-unit input magnitude so continuous AI/FEP
intent is not silently normalized to full speed, while keyboard diagonals are
capped at unit magnitude.

## Same-tick thermodynamic authority

The ordering above is an accounting contract, not presentation polish.

Under the full consciousness runtime, the `PhysicsCallback` consumes entity
energy on collisions and records heat/dissipation during the solver step. If
thermodynamics is finalized before that callback runs, those effects belong to a
physically completed tick but are not present in that tick's finalization/HUD
sample. If the old monolithic thermodynamic system is merely moved after physics,
its `tick_reset()` would instead erase callback-side per-tick counters before
sampling them.

The split therefore enforces:

`reset/pre-step -> authoritative physics + callback effects -> finalize/sample`

with no reset or finalization hidden inside the physics bridge.

The core world still contains a separately known dimensionally-invalid friction
bookkeeping heuristic proportional to impulse. This lifecycle change does not
upgrade that heuristic into physical energy. Measured world friction work remains
a separate prerequisite before coupled mechanical-to-thermal claims are valid.

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
sleeping dynamic body carrying non-zero velocity intent before stepping.

This should eventually become an explicit controller API on `RigidBody` /
`PhysicsWorld` rather than a launcher-side invariant.

## Full-runtime callback contract

Under `consciousness-runtime`, the physics callback depends on current entity
positions for harmony/friction coupling. `physics_step_system` rebuilds that
field from authoritative body positions immediately before the step and then
calls `step_with_callback`.

The default lightweight build intentionally uses pure `PhysicsWorld::step`
because its local fallback `ConsciousnessField` is not the production
`PhysicsCallback` implementation.

## Fixed-tick validation contract

Executable validation must prove more than source ordering. Before this lifecycle
is promoted from draft:

1. **Exactly-once opening/finalization** — one counter reset and one
   `tick_thermodynamics()` finalization per fixed tick.
2. **Same-tick collision accounting** — a deterministic collision callback cost
   is visible in the finalized tick, not one tick later.
3. **No reset-after-collision loss** — callback-side `consumed_this_tick` survives
   until post-step sampling.
4. **Composition** — maintenance, collision/action costs, ambient/well
   regeneration, and offloading all contribute to the same completed tick rather
   than replacing one another.
5. **Motor-authority timing** — pre-step collapse/safety state is visible before
   player intent becomes body velocity.
6. **Presentation isolation** — transform sync cannot alter physics,
   thermodynamic, or accounting state.
7. **Deterministic replay** — identical fixed inputs produce identical per-tick
   physics/thermodynamic traces and final digest.
8. **Feature matrix** — validate default, `fep-ai`, and
   `consciousness-runtime`/full-stack paths independently.

A source-level chain is implementation evidence, not validation evidence. These
gates still require compiler/test execution.

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
- engine-physics and thermodynamic unit tests must pass,
- the fixed-tick validation contract above must execute,
- deterministic same-input runs must produce the same body-state digest,
- player cardinal/diagonal/analog intent must match the speed contract,
- Red/collapsed motor authority must produce zero commanded velocity,
- generated `0` border tiles must be blocked while `1/2/3` are traversable,
- owner playtest follows automated/capture validation rather than replacing it.

## Claim boundary

R1 means the 2D flagship runtime advances `symtropy-physics` authoritatively for
registered dynamic bodies and now has an explicit same-tick thermodynamic phase
ordering in source. It does **not** mean:

- that ordering has passed executable validation yet,
- collision/friction energy values are all physically valid,
- walls are already solver-owned,
- Old Waterworks is a true 3D physics world,
- world/experience teardown is production-ready,
- character-controller quality is competitive with Jolt or PhysX,
- the runtime stack has passed hosted CI while runner execution remains blocked.
