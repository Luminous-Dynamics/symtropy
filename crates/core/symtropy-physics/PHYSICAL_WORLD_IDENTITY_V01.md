# PHYS-ID-00 — Physical World Identity Authority v0.1

Status: frozen source contract for qualification

## Purpose

PHYS-ID-00 closes the authority gap between a caller-authored physical reference and the actual `PhysicsWorld` supplied to downstream systems.

A matching string/ID beside an unrelated world is not authority. The world and its authority identity must be owned by the same physics-side object.

## Identity layers

V0.1 distinguishes five concepts:

```text
PhysicalAuthorityId
    durable physical/persistence lineage identity

WorldGenerationId
    one instantiated runtime generation in that lineage

NetId
    network/live body identity inside that generation

BodyHandle
    ephemeral runtime allocation handle

external economic AssetId
    not owned or interpreted by this crate
```

These concepts are not interchangeable.

`PhysicalAuthorityId` and `WorldGenerationId` reject zero values so an all-zero/default sentinel cannot silently become authority.

## PhysicsAuthorityWorld

`PhysicsAuthorityWorld<D>` owns:

```text
PhysicalAuthorityId
WorldGenerationId
PhysicsWorld<D>
```

Downstream validation therefore derives authority/generation from the supplied authority object itself rather than accepting a detached claimed ID beside an arbitrary `PhysicsWorld`.

V0.1 is a local correctness boundary only. Constructing the wrapper does not prove cryptographic authenticity, persistence, legal authority, or distributed consensus.

## PhysicsBodySubject

A durable-looking body subject contains exactly:

```text
PhysicalAuthorityId
WorldGenerationId
NetId
```

It deliberately contains no `BodyHandle`.

A subject names one body only inside one exact authority/generation namespace. Reusing the same `NetId` in a different world generation does not satisfy the original subject.

## ValidatedNetBody

`ValidatedNetBody<'a, D>` is the current-authority token. It is deliberately non-Serde and lifetime-bound to an immutable borrow of `PhysicsAuthorityWorld<D>`.

For a subject `S`, validation succeeds only when:

```text
S.physical_authority_id == world.physical_authority_id
S.world_generation_id == world.world_generation_id

exactly one live body has body.net_id == Some(S.net_id)
world.handle_for_net_id(S.net_id) == that body.handle
world.net_id_for_handle(that body.handle) == Some(S.net_id)
```

The resolver scans body truth rather than trusting only the index, because the current legacy physics API can still create duplicate/stale identity states. Those mutation hazards are intentionally detected here and assigned to PHYS-ID-01 for repair.

Holding a `ValidatedNetBody` immutably borrows the authority world, so ordinary safe Rust cannot obtain mutable access to that `PhysicsAuthorityWorld` until the token is dropped.

The token may expose the runtime `BodyHandle` for immediate engine calls, but that handle is never part of durable subject identity.

## Fail-closed cases

Validation fails if:

- the physical authority differs;
- the world generation differs;
- no live body carries the requested `NetId`;
- more than one live body carries the requested `NetId`;
- the forward `NetId -> BodyHandle` index is absent;
- the forward index points to another handle;
- reverse `BodyHandle -> NetId` resolution disagrees.

There is no fallback to vector index, nearest body, Bevy entity, or another runtime locator.

## Existing replay boundary

The existing physics replay module remains useful for deterministic command/state replay, but its current `WorldCommand`, `BodySnapshot`, and `WorldSnapshot` identity surfaces are based on runtime `BodyHandle` and do not carry PHYS-ID-00 authority/generation identity.

PHYS-ID-00 does not reinterpret those snapshots as durable cross-generation evidence.

A future PHYS-OBS/PERSIST tranche may reuse their bitwise state-capture semantics while binding observations to `PhysicsBodySubject` and an explicit authority/generation lineage.

## Required adversarial corpus

Qualification must include at least:

- exact authority/generation/`NetId` resolves one body;
- the same `NetId` in a different generation rejects;
- an authority claim detached from the supplied world rejects;
- duplicate live `NetId` rejects even if the index points at one duplicate;
- direct body identity mutation leaving a stale index rejects;
- zero authority/generation roots reject;
- the durable subject contains no `BodyHandle`;
- the validated token has no serialization/deserialization authority surface;
- no pose-history, movement, route, arrival, persistence, or delivery claim enters this tranche.

## PHYS-ID-01 successor

The current underlying `PhysicsWorld` still permits identity inconsistency through legacy mutation surfaces:

- public `RigidBody::net_id`;
- public `PhysicsWorld::bodies` structural mutation;
- infallible `set_net_id` that can overwrite an existing ID;
- incremental `add_bodies_deterministic` validation that can partially insert a batch before a late duplicate is discovered.

PHYS-ID-01 should make assignment typed/fallible, preflight whole operations, define idempotent same-binding behavior, seal direct identity mutation, and audit/remediate structural vector mutation.

PHYS-ID-00 does not hide these hazards; it makes the authority boundary fail closed in their presence.

## Explicit non-claims

PHYS-ID-00 does NOT establish:

- persistence across save/load or restart;
- continuity of one physical body across world generations;
- global `NetId` uniqueness outside the named generation;
- pose history or append-only observations;
- movement, path, route, arrival, distance, speed, or ETA;
- vehicle existence, type, capacity, mobility, fuel, energy, wear, or maintenance;
- cargo containment, ownership, custody, contracts, settlement, or delivery;
- cryptographic signatures, distributed consensus, or legal authority.

## Architectural rule

> A downstream subsystem may consume a source authority's validated fact, but may not manufacture the source authority's identity namespace itself.

For transport, ECON may later consume a `ValidatedNetBody` or stronger physical evidence. ECON must not author physics world identity, body continuity, pose, movement, or route truth.
