# Distributed Authority Continuation Contract v0.1

**Status:** design freeze; optional profile for networked/migratable scopes  
**Tracks:** #94  
**Consumed by:** World Continuation Manifest v0.1 / Q2 only when distributed authority is enabled

## 1. Purpose

`AuthorityId` identifies the domain that owns truth. It does not identify which peer, server, process, or replica is currently entitled to advance that truth.

For offline/single-player worlds, that distinction is unnecessary protocol state and MUST NOT be imposed on the semantic continuation root.

For networked or server-migrated worlds, however, failing to bind ownership generation can create split-brain continuation:

```text
server A owns body X
        ↓ handoff
server B owns body X
        ↓ suspend/reconnect
old server A resumes stale snapshot
        ↓
A and B both advance authoritative X
```

This contract prevents that class of failure without moving domain state into the network layer.

## 2. Core invariant

For every distributed authoritative `(world, authority, scope)` tuple:

> At most one ownership generation may be accepted as current, and a transfer of ownership is valid only when it is bound to the exact domain continuation identity being transferred.

Network ownership is authorization to advance an authority. It is not the authority's physical state.

## 3. Required identity

A distributed ownership record binds at minimum:

```text
schema_version
world_instance_id
authority_id
scope
reference_frame
owner_id
authority_epoch
effective_at
accepted_continuation_digest
handoff_policy_digest
parent_handoff_digest?
```

Optional protocols may additionally bind quorum/consensus evidence, lease windows, signatures, or transport/session identities, but those extensions must not weaken the core epoch and state-binding semantics.

## 4. Authority epoch

`authority_epoch` is monotonically ordered for one `(world, authority, scope)` lineage.

Rules:

- an older epoch can never replace a newer accepted epoch;
- two different owners for the same epoch are a conflict and fail closed;
- a later epoch must descend from the accepted handoff lineage according to policy;
- epoch ordering is protocol/canonical ordering, not message-arrival ordering;
- reconnecting an old peer does not revive its prior authority.

The epoch may be an integer generation in v1. A future consensus protocol may use a richer term/ballot identity so long as it defines a total accepted ordering for this purpose.

## 5. Handoff receipt

A handoff receipt binds:

```text
world_instance_id
authority_id
scope
reference_frame
from_owner
from_epoch
to_owner
to_epoch
effective_at
source_continuation_digest
target_accepted_continuation_digest
handoff_policy_digest
parent_handoff_digest?
```

For an ownership-only transfer with no domain evolution between source and acceptance, the source and target continuation identities should normally be equal.

If migration includes an explicit state transformation, that transformation requires its own domain migration/equivalence/representation receipt. Network handoff must not silently authorize state conversion.

## 6. Two-phase adoption principle

The exact network protocol may vary, but semantic adoption should follow the equivalent of:

```text
sender proposes exact continuation identity
        ↓
receiver obtains/verifies snapshot + continuation identity
        ↓
receiver proves it can adopt the proposed generation
        ↓
canonical ownership generation commits
        ↓
old generation becomes stale
        ↓
receiver may advance authoritative state
```

A receiver must not acknowledge exclusive ownership before it has verified the domain state it is accepting.

A sender must not continue authoritative mutation after an exclusive handoff is committed, except under an explicitly different replicated/consensus authority model.

## 7. Split-brain failure rules

Fail closed for:

- stale epoch;
- two owner IDs claiming the same accepted epoch;
- handoff bound to the wrong world identity;
- handoff bound to the wrong authority/scope/frame;
- continuation digest mismatch;
- broken parent handoff lineage;
- unknown required handoff policy/schema;
- replay of an already superseded claim;
- transport message order attempting to override canonical epoch order.

## 8. World Continuation Manifest integration

The hierarchical world continuation manifest includes distributed-ownership evidence only for scopes whose execution profile declares networked/migratable authority.

A domain entry may bind an optional:

```text
distributed_authority_digest
```

or a child/reference to an equivalent canonical ownership record.

The root does not interpret peer topology. It only proves which ownership-generation evidence is required before that domain is allowed to resume mutation.

### 8.1 RequiredExact

Use when the accepted owner/epoch/handoff state itself must survive suspend/resume exactly.

### 8.2 RebuildableWithProof

Use only when a deterministic consensus/assignment protocol can reconstruct the same accepted authority generation before any domain mutation resumes.

A proximity heuristic alone is not a proof of identical ownership reconstruction.

### 8.3 Offline profile

Offline/single-player manifests omit distributed ownership entirely.

The absence of network ownership evidence in an offline profile is not a missing continuation field.

## 9. Interaction with current `SpatialAuthority`

Current `symtropy-net-core::SpatialAuthority` is useful runtime routing state:

- `BodyHandle -> PeerId` claims;
- local-body cache;
- proximity/remote-claim-driven reassignment.

That structure must not be directly serialized as canonical world truth because:

- `BodyHandle` is a runtime handle, not necessarily a stable world scope identity;
- `HashMap`/`HashSet` iteration is not a canonical encoding contract;
- there is no authority epoch;
- there is no continuation-state binding;
- there is no stale handoff rejection.

A future network hardening layer may derive runtime `SpatialAuthority` caches from the canonical ownership records.

## 10. Required tests

### Epoch safety

- accept generation N;
- reject N-1;
- reject a different owner at N;
- accept valid N+1 handoff;
- reject replay of N after N+1.

### State binding

- exact continuation digest accepts;
- wrong continuation digest rejects;
- physical-state-only digest rejects when the domain profile requires a complete continuation digest.

### Message ordering

Apply the same valid handoff messages in different transport arrival orders and require the same final accepted owner/epoch.

### Duplicate delivery

Repeated delivery of an already accepted identical handoff is idempotent or explicitly recognized as duplicate; it must not advance the epoch twice.

### Suspend/resume

- suspend a networked world at accepted owner/epoch;
- restore ownership evidence before authority mutation;
- require the same accepted generation or an explicitly newer valid handoff;
- prove the old owner cannot resume stale authority.

### Crash boundary

Model receiver failure before and after canonical handoff commit and prove that the protocol does not produce two accepted exclusive owners.

### Offline profile

Prove offline continuation manifest construction and restore do not require peer IDs, epochs, or network handoff state.

## 11. Q2 evidence

A Q2 profile that includes distributed authority must add stable fixture IDs for at least:

```text
Q2-NETAUTH-EPOCH-001
Q2-NETAUTH-STATE-001
Q2-NETAUTH-ORDER-001
Q2-NETAUTH-RESUME-001
```

The evidence capsule records:

- ownership record/receipt digests;
- accepted epoch/owner at checkpoints;
- bound domain continuation digest;
- duplicate/reorder/stale rejection results.

A single-player Q2 profile may mark the distributed-authority feature as out of scope rather than skipped.

## 12. Security boundary

This contract defines deterministic continuation semantics, not cryptographic peer authentication.

A production hostile-network deployment should additionally authenticate/sign ownership/handoff evidence using the chosen networking/security layer.

Cryptographic signatures prove who authorized a handoff; the canonical ownership/epoch contract proves what handoff was authorized and which state it applies to.

## 13. Non-goals

v0.1 does not:

- define one mandatory consensus algorithm;
- require networking for offline worlds;
- make `symtropy-net-core` owner of physics/terrain/ecology;
- persist runtime body handles as semantic world IDs;
- use local wall-clock lease expiration as simulation truth;
- let network proximity override a newer canonical epoch;
- solve Byzantine consensus by itself.

## 14. Outcome

For networked worlds, continuation becomes:

> restore this exact domain continuation state **and** prove that this owner/generation is currently entitled to advance it.

For offline worlds, the same world-continuation machinery remains free of unnecessary network state.
