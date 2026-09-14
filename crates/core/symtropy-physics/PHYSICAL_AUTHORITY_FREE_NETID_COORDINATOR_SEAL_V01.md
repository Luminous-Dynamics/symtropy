# PHYS-ID-01B2B1 — Free NetId Coordinator Seal v0.1

## Claim

Outside `symtropy-physics`, ordinary safe Rust can no longer call the low-level checked network-identity coordinators directly.

The public stable-identity path established by PHYS-ID-01B2A is:

```text
PhysicsAuthorityWorld
    -> bind_net_id(...)
    -> add_bodies_deterministic(...)
    -> PhysicsBodySubject
```

The low-level mechanics remain available only inside the physics crate:

```text
assign_net_id_checked(...)
add_bodies_deterministic_checked(...)
```

`NetIdentityMutationError` remains public so downstream callers retain typed failure semantics through the authority-owned API.

## Preserved mutation law

For one live body incarnation:

```text
None -> N   allowed
N -> N      idempotent
N -> M      rejected
```

For deterministic batch insertion, duplicate requested IDs, collisions with existing live IDs, conflicting embedded identity, or stale index state are rejected before expected insertion mutation begins.

## Regression requirement

The public mutation corpus must continue to prove through `PhysicsAuthorityWorld`:

- duplicate single binding fails without mutating either body;
- repeated identical binding is idempotent;
- clean reassignment is rejected and preserves the original binding;
- duplicate batch is rejected before insertion;
- collision with an existing live identity is rejected before insertion;
- successful batches preserve deterministic `NetId` ordering and return exact authority/generation-bound subjects.

Privileged corrupt-state construction remains crate-internal.

## Static API law

At this tranche:

```text
public:
    NetIdentityMutationError
    PhysicsAuthorityWorld::bind_net_id
    PhysicsAuthorityWorld::add_bodies_deterministic

not public:
    identity_mutation module
    assign_net_id_checked
    add_bodies_deterministic_checked
```

## Explicit remaining debt

This tranche does **not** complete PHYS-ID-01B2B.

The following raw authority-free methods remain public and are outside this claim:

```text
PhysicsWorld::set_net_id
PhysicsWorld::add_bodies_deterministic
```

Their visibility seal is PHYS-ID-01B2B2.

`PhysicsWorld::bodies` also remains public; structural body-storage and complete private-index integrity are PHYS-ID-01C / #1005.

## Non-claims

No persistence, cross-generation continuity, checked raw-world import, observation history, movement, route, arrival, delivery, cryptographic identity, or distributed-consensus claim is made here.
