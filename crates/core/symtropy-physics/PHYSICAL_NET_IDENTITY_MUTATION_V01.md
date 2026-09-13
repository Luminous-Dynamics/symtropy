# PHYS-ID-01A — Checked Network Identity Mutation v0.1

Status: frozen source contract for qualification

## Purpose

PHYS-ID-00 detects inconsistent live identity. PHYS-ID-01A adds a checked mutation path that prevents the known expected identity failures before invoking the legacy `PhysicsWorld` mutators.

This tranche deliberately does **not** yet remove or privatize the legacy public mutation surfaces. It creates the safe authority path first.

## Stable identity rule

For one live body incarnation:

```text
None -> NetId(N)   allowed
NetId(N) -> N      idempotent
NetId(N) -> M      rejected
```

Stable identity replacement is not an ordinary setter operation. A later continuity/reincarnation protocol must own any legitimate replacement.

## Single-body binding theorem

`assign_net_id_checked(world, handle, net_id)` validates before mutation:

- `handle` resolves to a live body;
- the requested `NetId` is not held by another live body;
- no stale target index claims the requested `NetId` for another handle;
- if the body already has the requested ID, forward and reverse indices agree exactly;
- if the body already has a different ID, its existing live/index views must first be internally consistent, then replacement is rejected.

Only an unbound body reaches the legacy `set_net_id` call.

All expected rejection paths therefore occur before mutation.

## Deterministic batch theorem

`add_bodies_deterministic_checked(world, batch)` preflights the entire batch before calling the legacy deterministic insertion loop.

It rejects before insertion when:

- a `NetId` is duplicated inside the batch;
- a batch body carries a conflicting embedded `net_id`;
- a requested `NetId` already exists in live body state;
- a requested `NetId` is already present in the world index;
- an existing requested identity is already ambiguous.

After successful preflight, the frozen legacy insertion implementation has no remaining expected duplicate-ID rejection point, so the known partial-insertion failure mode is excluded for callers of the checked coordinator.

## Failure atomicity claim

For all typed preflight errors, caller-visible world identity state is unchanged:

- body count unchanged;
- existing body `net_id` fields unchanged;
- existing forward mappings unchanged;
- no requested new mapping appears.

This claim applies to the checked coordinators and the frozen legacy implementation they invoke. It is not a general transaction theorem for arbitrary future mutations to `PhysicsWorld` internals.

## Relationship to PHYS-ID-00

PHYS-ID-00 remains the read/current-authority validator. PHYS-ID-01A does not replace it.

The intended path is:

```text
checked bind / checked deterministic insert
        ↓
PhysicsAuthorityWorld
        ↓
PhysicsBodySubject
        ↓
ValidatedNetBody
```

Mutation admission and current identity validation remain separate theorems.

## Required adversarial corpus

Qualification must include at least:

- duplicate single assignment rejected without changing either body;
- same binding is idempotent;
- clean `N -> M` reassignment is rejected and preserves `N`;
- stale current index rejects before attempted replacement;
- duplicate batch rejects before inserting the first body;
- collision with an existing world identity rejects before insertion;
- conflicting embedded body identity rejects before insertion;
- successful checked batch preserves deterministic `NetId` ordering.

## Legacy boundary

V0.1 does **not** seal:

- public `RigidBody::net_id`;
- public `PhysicsWorld::bodies`;
- direct calls to `PhysicsWorld::set_net_id`;
- direct calls to `PhysicsWorld::add_bodies_deterministic`.

Those remain migration debt. A successor should migrate callers and then make the unsafe bypasses inaccessible or explicitly unsafe/legacy.

## Explicit non-claims

PHYS-ID-01A does not establish:

- cryptographic/global identity authenticity;
- save/load persistence;
- cross-generation continuity;
- body reincarnation semantics;
- distributed mutation atomicity;
- pose history, movement, route, arrival, or delivery;
- that arbitrary direct legacy callers cannot still create inconsistent identity state.

Its positive claim is narrower: callers using the checked coordinators reject the known expected identity conflicts before mutation and enforce bind-once identity semantics.
