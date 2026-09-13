# ECON-05C — Live Carrier Identity Bridge v0.1

Status: frozen source contract for qualification

## Purpose

ECON-05C prevents the logistics stack from turning a runtime physics handle into durable economic truth.

The live physics engine currently has two distinct identifiers:

```text
BodyHandle(usize) = runtime allocation handle
NetId(u64)        = stable network identity inside PhysicsWorld
```

`BodyHandle` is allocated monotonically from a fresh `PhysicsWorld` counter and is therefore runtime-local. It must not be stored as durable carrier identity, written into economic evidence, or assumed to survive restart/reconstruction.

`NetId` is the stronger existing physics identity, but the current scene/persistence path does not establish that every `NetId` is restored across save/load. ECON-05C therefore uses it only as a **live resolution key** under an explicit physical-authority namespace. It does not claim persistence that the physical layer has not yet proved.

## Authority split

```text
ECON-05A CarrierCargoBinding
    carrier AssetId
    cargo-space LocationId
    carrier custodian
    physical-authority id

ECON-05C CarrierPhysicsRef
    same carrier AssetId
    same physical-authority id
    physics NetId

PhysicsWorld
    live RigidBody state
    NetId ↔ BodyHandle index
```

The physical authority identifier is opaque to ECON. It should identify the exact world/session/persistence authority that owns the `NetId` namespace.

## Governing live-resolution theorem

For cargo binding `C`, physical reference `P`, physics world `W`, and network identity `N`:

```text
C.carrier_asset_id == P.carrier_asset_id
C.physical_authority_id == P.physical_authority_id
P.net_id == N

exactly one body in W has body.net_id == N
W.handle_for_net_id(N) == that body's BodyHandle
W.net_id_for_handle(that BodyHandle) == N
```

Only then may ECON receive the body handle as a runtime-only resolution result.

The resolver intentionally scans body state instead of trusting only the index. This defends the economic boundary against currently possible inconsistent physics states, including duplicate `NetId` assignment and direct mutation of the public `RigidBody::net_id` field.

## Why this precedes transport legs

A route, arrival, unloading, or delivery theorem is meaningless if the carrier identity can silently refer to a different body after restart, collision, or map corruption.

Therefore the intended stack is now:

```text
ECON-05B reservation-aware lot isolation
        ↓
ECON-04D semantic encumbrance handoff
        ↓
ECON-05A carrier custody + carrier-local cargo location
        ↓
ECON-05C live carrier identity bridge
        ↓
PHYS persistence/history authority (future prerequisite)
        ↓
ECON-05D transport observation / arrival evidence
        ↓
ECON-05E unload / delivery transition
        ↓
capacity, time, cost, risk, spoilage, wear
```

ECON-05C deliberately moves transport-motion work later rather than manufacturing an economic pose/route source of truth.

## Fail-closed cases

Resolution fails if:

- economic and physical carrier `AssetId`s differ;
- economic and physical authority identifiers differ;
- no live body carries the requested `NetId`;
- two or more live bodies carry the requested `NetId`;
- the physics world's `NetId -> BodyHandle` index is missing;
- that index points at a different body;
- reverse `BodyHandle -> NetId` resolution disagrees.

No fallback to `BodyHandle`, Bevy `Entity`, array position, or nearest spatial body is permitted.

## Historical boundary

ECON-05C creates **no historical movement receipt**.

A successful call proves only that, at the instant of resolution in the supplied live `PhysicsWorld`, one body is consistently indexed by the referenced `NetId` and matches the economic carrier/authority binding.

It does not prove:

- that the same `NetId` existed earlier or will exist later;
- save/load persistence;
- body continuity across world reconstruction;
- carrier motion between two observations;
- route traversal;
- arrival at a destination;
- distance, speed, travel time, or ETA.

Those claims require an append-only or otherwise independently replayable physical observation authority.

## Required adversarial corpus

Qualification must include at least:

- unique `NetId` resolves to its live `BodyHandle`;
- economic/physical carrier asset mismatch fails;
- physical-authority mismatch fails;
- unknown `NetId` fails;
- duplicate live `NetId` state fails even if the physics index points to one duplicate;
- direct body `net_id` mutation creating a stale/missing index fails;
- no durable ECON type stores a `BodyHandle`;
- no transport/route/arrival claim is introduced.

## Explicit non-claims

ECON-05C does NOT establish:

- durable `NetId` persistence across restart/save/load;
- global uniqueness outside the named physical authority;
- vehicle type, capacity, geometry, cargo containment, or mobility;
- body pose as economic authority;
- carrier movement, route, road/rail/air/sea access, or arrival;
- energy/fuel use, wear, maintenance, weather, risk, spoilage, or travel time;
- loading/unloading beyond the existing ECON-05A theorem;
- ownership, settlement, contract fulfillment, legal custody, insurance, or customs;
- signatures, distributed consensus, or durable multi-ledger transaction atomicity.

## Qualification rule

Source review and mergeability are not executable qualification.

A qualifier must freeze the exact ECON-05C product head and exact ECON-05B parent; run formatting, all-target compile/check, the public carrier-identity corpus, 05B/05A/04D/04C/04B/04 regressions and prerequisite ECON tests, full core tests, strict Clippy, and static audits proving that ECON-05C stores `NetId` but no `BodyHandle` in durable reference state and introduces no route/motion authority. It must emit machine-readable exact-lineage evidence.
