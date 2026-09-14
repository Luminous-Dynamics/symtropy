# PHYS-ID-01B2A — Authority-Owned Network Identity Binding v0.1

Status: **source theorem only until exact-head qualification executes**.

This tranche makes `PhysicsAuthorityWorld` the public owner of successful live `NetId` binding. It does not yet seal the legacy raw-world mutation implementation.

## 1. Authority boundary

A stable live physics subject is the tuple:

```text
PhysicalAuthorityId
+ WorldGenerationId
+ NetId
```

`BodyHandle` is deliberately excluded. It is runtime-local allocation state.

The public binding path therefore belongs on the object that owns all three relevant namespaces:

```text
PhysicsAuthorityWorld {
    PhysicalAuthorityId,
    WorldGenerationId,
    PhysicsWorld,
}
```

A caller may still construct a `PhysicsBodySubject` value manually. That value is a **claim/reference**, not a capability or proof. It gains authority only when `PhysicsAuthorityWorld::validate_subject` accepts it against the exact owned world.

## 2. Single-bind theorem

For one live body incarnation and requested `NetId = N`:

```text
None -> N   allowed
N -> N      idempotent
N -> M      rejected when M != N
```

`PhysicsAuthorityWorld::bind_net_id(handle, N)` delegates to the checked mutation coordinator. On success it returns exactly:

```text
PhysicsBodySubject {
    physical_authority_id = authority_world.physical_authority_id(),
    world_generation_id   = authority_world.world_generation_id(),
    net_id                 = N,
}
```

The success postcondition is:

```text
body(handle).net_id() == Some(N)
handle_for_net_id(N) == Some(handle)
net_id_for_handle(handle) == Some(N)
returned_subject.net_id() == N
returned_subject.physical_authority_id()
    == authority_world.physical_authority_id()
returned_subject.world_generation_id()
    == authority_world.world_generation_id()
```

This tranche does **not** grant identity replacement authority. Reincarnation, continuity, transfer, or replacement require a later explicit theorem.

## 3. Deterministic batch theorem

`PhysicsAuthorityWorld::add_bodies_deterministic` delegates to the checked whole-batch coordinator.

Before the first insertion, the entire requested set must reject:

- duplicate requested `NetId`s;
- collisions with an already-live identity;
- stale target indexes;
- conflicting embedded body identity.

If preflight rejects, zero requested bodies are inserted.

On success, each returned ephemeral `BodyHandle` is paired with the exact `PhysicsBodySubject` for the owning authority/generation. Returned handle order inherits the checked deterministic insertion order.

## 4. Authority/generation separation

The same `NetId` may exist in two different world generations or physical authorities. Those subjects are intentionally distinct:

```text
(A, G1, N) != (A, G2, N)
(A1, G, N) != (A2, G, N)
```

A subject from one authority/generation must fail validation against another before body lookup can grant a validated token.

## 5. Public versus privileged tests

Public integration tests exercise only legitimate binding and validation behavior.

Deliberate construction of impossible identity states belongs inside `symtropy-physics` crate-private tests, where tests may use crate-owned fields or legacy raw mutation implementation to prove fail-closed behavior.

This is intentional separation:

```text
public tests:
    permitted authority surface

crate-private adversarial tests:
    corruption construction + rejection theorem
```

Deleting corruption tests merely to make a field private is forbidden by this tranche.

## 6. Transitional compatibility debt

The following are still outside this theorem and remain to be sealed by the successor migration:

- public `PhysicsWorld::set_net_id`;
- public raw `PhysicsWorld::add_bodies_deterministic`;
- public free checked mutation coordinators as a transitional API;
- public structural `PhysicsWorld::bodies` access.

Therefore this tranche establishes the **preferred authority-owned path**, not exclusive mutation authority.

Structural body-storage authority is tracked separately by PHYS-ID-01C / #1005.

## 7. Replay boundary

The current replay subsystem is still `BodyHandle`-based. This tranche does not reinterpret it as durable identity.

Authority-aware replay should occur only after:

1. raw identity mutation is sealed;
2. structural body storage is sealed sufficiently to preserve identity indexes;
3. replay commands/snapshots can name exact authority/generation-bound subjects without inventing cross-generation continuity.

## 8. Explicit non-claims

This tranche does not establish:

- persistence across process restart;
- identity continuity across world generation changes;
- historical pose or observation evidence;
- route, movement, arrival, or delivery evidence;
- cryptographic identity;
- distributed consensus;
- Byzantine-fault tolerance.

`PhysicsBodySubject` remains an exact live-authority/generation reference.

## 9. Qualification gate

An exact-head PASS should require at minimum:

1. exact ancestry and changed-path audit;
2. `cargo fmt --check`;
3. workspace all-target compilation;
4. PHYS-ID-00/01A/B1/B2A identity tests;
5. complete `symtropy-physics` library/tests regression;
6. existing replay harness regression;
7. strict Clippy for `symtropy-physics`;
8. static audit that public PHYS-ID tests no longer construct positive fixtures with legacy `set_net_id`;
9. static audit that deliberate direct/duplicate corruption remains crate-private;
10. unchanged-subject postflight.

Queued, skipped, or runnerless CI is not evidence of qualification.
