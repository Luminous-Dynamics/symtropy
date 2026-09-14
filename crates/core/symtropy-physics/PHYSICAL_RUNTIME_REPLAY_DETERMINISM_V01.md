# PHYS-REPLAY-00 — Runtime Replay Determinism Claim v0.1

## Purpose

Freeze the exact claim established by the current `symtropy-physics::replay` implementation and `replay_harness` before authority-aware replay is introduced.

This theorem is intentionally **runtime-local**. It protects the existing determinism oracle from later being reinterpreted as durable physical identity or historical provenance.

## Current subject model

The current replay substrate addresses bodies by ephemeral runtime handle:

```text
WorldCommand.body       = BodyHandle
BodySnapshot.handle     = BodyHandle
CollisionEventSnapshot  = BodyHandle pair
WorldSnapshot ordering  = BodyHandle
```

`BodyHandle` is valid inside this theorem because both record and replay worlds are constructed by the same deterministic runtime construction path.

It is not a durable cross-reconstruction subject identifier.

## Executed harness theorem

The checked-in `replay_harness` constructs one deterministic 3D world twice using the same `build_world()` function.

The construction fixes:

- gravity;
- solver iteration count;
- sleep threshold and sleep ticks;
- one static ground body;
- three dynamic sphere bodies;
- deterministic initial velocities.

It builds one ordered `ReplayTape` of 192 frames at `dt = 1/64`.

The tape applies deterministic combinations of:

- `ApplyForce`;
- `ApplyImpulse`.

For each record frame:

```text
apply commands
-> step world
-> capture WorldSnapshot
```

For the independently reconstructed replay world, the same tape is applied and the resulting `WorldSnapshot` is required to be exactly equal to the recorded snapshot **after every tick**.

The present theorem is therefore:

> Under the harness's equivalent runtime construction and identical ordered command tape, the captured replay state is bitwise-equal at every checked tick.

## Snapshot coverage

The current `BodySnapshot` compares raw `f64::to_bits()` state for:

- translation;
- rotation matrix;
- linear velocity;
- angular velocity;
- body type;
- sleeping state;
- sleep counter;
- runtime `BodyHandle`.

The current `WorldSnapshot` additionally compares the last-step collision-event snapshots, including participant handles, impulse bits, normal bits, and depth bits.

This is substantial deterministic state coverage, but it is **not a declaration that every internal `PhysicsWorld` datum is captured**.

In particular, the theorem must not be broadened merely because omitted internal state may influence later snapshots.

## Explicit same-construction requirement

The current tape is meaningful only because the second world reproduces the same handle allocation expected by the recorded commands.

This theorem does not establish that a tape recorded against one handle allocation can be replayed against a differently allocated world.

That is one reason authority-aware replay must resolve `PhysicsBodySubject` values to current ephemeral handles rather than persisting a historical handle as identity.

## Failure semantics

`apply_commands` fails when a referenced `BodyHandle` is missing.

It does not attempt to repair, remap, or reinterpret a missing handle as another body.

## Explicit non-claims

PHYS-REPLAY-00 does **not** establish:

- persistent replay-file encoding or schema stability;
- replay compatibility across code revisions;
- replay compatibility across Rust/toolchain versions;
- cross-platform equivalence beyond whatever a separately qualified configuration proves;
- `BodyHandle` durability;
- `NetId` identity;
- `PhysicalAuthorityId` or `WorldGenerationId` binding;
- continuity across reconstruction or generation change;
- append-only observation history;
- trusted wall-clock or simulation-time provenance;
- continuous trajectory evidence;
- route compliance;
- cargo custody, arrival, or delivery;
- cryptographic provenance or distributed consensus.

## Successor rule

PHYS-REPLAY-01 / #1020 must **add** authority-aware replay semantics rather than silently changing the meaning of this theorem.

The intended layering is:

```text
PHYS-REPLAY-00
runtime-local bitwise determinism
        ↓
PHYS-REPLAY-01
same-generation authority-subject resolution
        ↓
future observation/history authority
        ↓
future physical transition evidence
```

The existing handle-based replay harness should remain as a low-level regression oracle even after authority-aware replay exists.
