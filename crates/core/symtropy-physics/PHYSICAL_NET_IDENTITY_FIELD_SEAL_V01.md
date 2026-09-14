# PHYS-ID-01B1 — Direct NetId Write Seal v0.1

## Status

This contract is a narrow successor to PHYS-ID-00 and PHYS-ID-01A.

It closes one specific public bypass: downstream safe Rust code may observe a `RigidBody`'s `NetId`, but may no longer assign `RigidBody::net_id` directly.

It does **not** yet claim that every legacy network-identity mutation path in `PhysicsWorld` has been sealed.

## Governing theorem

For code outside `symtropy-physics` using the ordinary public Rust API:

```text
RigidBody network identity
    is readable
    but not directly writable
```

The public observation surface is:

```text
RigidBody::net_id() -> Option<NetId>
```

The backing field is crate-controlled.

Therefore a downstream caller cannot create a stale `RigidBody::net_id` / `PhysicsWorld::net_id_map` pair merely by assigning the body field through safe public Rust.

## Authority separation

The layers remain distinct:

```text
PhysicsBodySubject
    = PhysicalAuthorityId
    + WorldGenerationId
    + NetId

ValidatedNetBody
    = current lifetime-bound validation

RigidBody::net_id()
    = read-only live body observation
```

None of these surfaces turns `BodyHandle` into durable identity.

## Downstream migration

Known ECON live-carrier identity code now observes body identity through `RigidBody::net_id()` rather than direct field access.

Public integration tests likewise use the getter.

Adversarial tests that deliberately manufacture stale/corrupt identity state were not deleted. They were moved behind the `symtropy-physics` crate boundary where crate-controlled identity mutation remains available to the test module.

This preserves the fail-closed theorem without exposing the corruption primitive as downstream API.

## Preserved privileged attacks

Crate-internal tests retain at least these attacks:

1. mutate body identity behind an existing forward index and require current-authority validation to reject the stale state;
2. construct a body carrying an embedded `NetId` conflicting with a deterministic-batch request and require zero insertion;
3. preserve the PHYS-ID-01A stale-current-index rejection theorem.

Moving an attack behind the authority boundary is not removal of the attack.

## Explicit remaining compatibility debt

V0.1 deliberately leaves these surfaces unchanged:

- `PhysicsWorld::set_net_id` remains public;
- `PhysicsWorld::add_bodies_deterministic` remains public;
- `PhysicsWorld::bodies` remains publicly structurally accessible.

Those are **not** covered by the B1 claim.

The next migration should separate them further:

- **PHYS-ID-01B2** — make checked single/batch identity mutation canonical and seal the legacy world identity mutators;
- **PHYS-ID-01C** — audit and encapsulate structural `PhysicsWorld::bodies` mutation if arbitrary vector mutation is not part of the supported engine contract.

Keeping those changes separate avoids coupling a simple identity-field authority correction to a broad world-storage refactor.

## Replay boundary

The existing replay system remains unchanged. Its commands and body snapshots still use runtime `BodyHandle` and therefore are not promoted to durable cross-generation identity or observation evidence by this tranche.

## Non-claims

PHYS-ID-01B1 does not establish:

- complete sealing of all `NetId` mutation;
- save/load persistence;
- cross-generation continuity;
- incarnation identity;
- cryptographic/global authority authenticity;
- append-only physical observations;
- movement, route, arrival, distance, speed, ETA, cargo delivery, or settlement;
- encapsulation of the full `PhysicsWorld::bodies` container.

## Success criterion

A successful qualification must prove at minimum:

- `RigidBody` contains no `pub net_id:` field;
- a public read-only `net_id()` getter exists;
- downstream ECON carrier code uses the getter;
- public integration tests do not assign `.net_id =`;
- privileged corruption attacks still execute inside `symtropy-physics`;
- PHYS-ID-00 and PHYS-ID-01A regressions remain green;
- the full workspace still compiles after the visibility change.
