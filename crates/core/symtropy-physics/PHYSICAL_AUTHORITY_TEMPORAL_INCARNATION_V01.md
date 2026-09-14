# Physical Authority Temporal Incarnation v0.1

Status: implementation candidate for PHYS-OBS-04A / #1055; executable exact-head qualification required before PASS is claimed.

## Purpose

Prevent two independently constructed live `PhysicsAuthorityWorld` wrappers that
share one loaded incarnation allocator from minting bitwise-equal authority step
stamps merely because they reuse the same `PhysicalAuthorityId`,
`WorldGenerationId`, mutation epoch, and step index.

This theorem adds one authority-owned **allocator-instance-local temporal
incarnation** to every live wrapper and every authority-issued step stamp.

## Temporal-incarnation identity

Conceptually each wrapper owns:

```text
TemporalIncarnationId
```

with these properties:

- minted internally by the authority wrapper;
- opaque outside the crate implementation, with no public constructor;
- non-serializable in this tranche;
- stable across moves, authorized steps, and mutation-epoch changes of one
  wrapper;
- different for independently constructed wrappers sharing one live allocator
  instance;
- allocated with checked monotonic arithmetic and no wraparound.

The ID is **not** a wall-clock timestamp, process identifier, persistent UUID,
cryptographic nonce, or globally unique identifier.

## Allocator-domain scope

The implementation uses one private Rust `static Mutex<u64>` per loaded copy of
the `symtropy-physics` incarnation module.

Ordinary statically linked use has one such allocator for that loaded crate
instance. However, an OS process can theoretically contain multiple independently
loaded copies/versions/dynamic images of the crate. Each such image can own a
distinct static allocator whose numeric sequence starts at 1.

Therefore this theorem deliberately proves uniqueness only within **one live
allocator instance**, not across every possible copy of the crate in an OS
process.

This distinction matters for detached evidence: two stamps from distinct
allocator domains must not be composed merely because their numeric
`TemporalIncarnationId` values match.

A future theorem that composes evidence across independently loaded runtimes must
bind a qualified runtime/session/issuer domain in addition to this ID.

## Constructor law

`PhysicsAuthorityWorld::try_new(...)` mints a fresh temporal incarnation from the
allocator instance linked to that constructor before returning a wrapper.

`PhysicsAuthorityWorld::new(...)` remains source-compatible and delegates to the
fallible constructor. If the allocator is unavailable or exhausted, `new(...)`
fails closed by panicking before returning a wrapper.

Callers cannot supply a temporal-incarnation value to either constructor.

`into_world()` consumes the wrapper and therefore destroys that live temporal
authority. Re-wrapping the resulting raw world through the same loaded authority
implementation mints a different temporal incarnation.

Any future checked raw-world adoption/reconstruction path from PHYS-ID-01D /
#1019 must also mint a fresh incarnation rather than accepting a caller-supplied
or persisted live-wrapper ID.

## Allocation law

The allocator is serialized by a private `Mutex<u64>`. The first admitted value
is 1.

The allocator conservatively reserves `u64::MAX` as a terminal exhausted state:

```text
current <= u64::MAX - 1
    -> mint current
    -> store current + 1

current == u64::MAX
    -> Exhausted
    -> mint nothing
```

Therefore one allocator instance never wraps and never reuses zero after
exhaustion.

A poisoned allocator also fails closed rather than guessing/recovering an ID.

The allocator is construction-path infrastructure only; its numeric order is
not evidence that one wrapper is physically or causally later than another.

## Step-stamp law

An authority step stamp becomes conceptually:

```text
AuthorityStepStamp {
    PhysicalAuthorityId,
    WorldGenerationId,
    TemporalIncarnationId,
    mutation_epoch,
    step_index,
}
```

All fields remain privately constructed and the stamp remains non-serializable.

Full stamp equality now includes temporal incarnation. Consequently, two fresh
wrappers using the same allocator instance with the same authority/generation
and identical numeric epoch/step cannot recreate the same full stamp.

## Explicit lineage comparison

Generic `PartialOrd` / `Ord` are removed from `AuthorityStepStamp`.

Temporal ordering is meaningful only after explicit lineage identity succeeds.
The public helper:

```text
same_temporal_lineage(a, b)
```

requires equality of:

```text
PhysicalAuthorityId
WorldGenerationId
TemporalIncarnationId
mutation_epoch
```

Within one qualified allocator/runtime domain, only after this predicate is true
may a later theorem compare `step_index` as an ordering coordinate.

The incarnation ID itself has equality/hash semantics but no generic ordering
semantics.

## Mutation relationship

A mutation-epoch break does **not** mint a new temporal incarnation. It changes
the mutation epoch inside the same live wrapper incarnation.

Thus:

```text
same wrapper
+ mutation boundary
    -> same TemporalIncarnationId
    -> different mutation_epoch
    -> step_index resets under PHYS-OBS-04/04B rules
```

PHYS-OBS-04B / #1061 remains responsible for fail-closed typed-mutation commit
sequencing. This theorem does not replace that mutation taint.

## Reconstruction relationship

This theorem distinguishes live wrapper incarnations; it does not certify that a
raw world is structurally safe to wrap.

The responsibilities remain separate:

```text
#1005 / PHYS-ID-01C
    complete structural/topological validity

#1019 / PHYS-ID-01D
    checked raw-world adoption

#1055 / PHYS-OBS-04A
    fresh allocator-local live temporal-incarnation identity
```

Until #1019 exists, `PhysicsAuthorityWorld::new/try_new` may still wrap an
arbitrary raw `PhysicsWorld`; a fresh incarnation prevents same-allocator stamp
aliasing but does not bless that raw state as structurally valid.

## Persistence / runtime-domain boundary

A numeric `TemporalIncarnationId` is insufficient durable identity on its own.
Distinct processes, independently loaded crate images, restarted runtimes, or
other allocator domains may mint equal numeric IDs.

Therefore detached/persistent/cross-runtime evidence must bind a qualified
runtime/session/issuer domain in addition to the numeric incarnation before
claiming global uniqueness.

PHYS-EVID-02/#1064 and PHYS-EVID-03/#1065 remain responsible for canonical
persistent representation and authenticated evidence profiles.

## Required executable corpus

Qualification must establish at least:

1. two independently constructed wrappers sharing one allocator and identical
   authority/generation receive different temporal incarnation IDs;
2. their first step may both be `(epoch=0, step=1)` numerically but full stamps
   are unequal;
3. `into_world -> rewrap same authority/generation -> step` through the same
   loaded allocator cannot recreate the old full stamp;
4. authorized steps in one wrapper preserve the same incarnation;
5. raw/non-step mutation epoch breaks preserve incarnation while changing epoch;
6. idempotent operations preserve the complete current stamp where the
   predecessor theorem says they should;
7. concurrent use of one allocator cannot duplicate incarnation IDs;
8. allocator exhaustion fails before minting and never wraps;
9. the public API has no constructor for arbitrary `TemporalIncarnationId` or
   `AuthorityStepStamp`;
10. neither type gains serde authority;
11. `AuthorityStepStamp` does not implement generic `PartialOrd`/`Ord`;
12. explicit `same_temporal_lineage` rejects distinct incarnations even when all
    other numeric fields match.

A future #1019 checked-adoption qualifier must additionally prove that its
construction path mints a fresh incarnation.

A future cross-runtime composition theorem must add a qualified allocator/runtime
identity rather than treating the numeric ID as globally collision-free.

## Evidence consequence

After PHYS-OBS-04A is qualified, an in-process stamped-observation theorem within
one qualified allocator domain may use the full
authority/generation/incarnation/epoch tuple as its lineage identity before
comparing step indexes.

This closes the ordinary wrapper-reconstruction aliasing hole required by
PHYS-OBS-05 / #1054 and PHYS-OBS-06 / #1056 without overclaiming cross-runtime
identity.

It does **not** make an unstamped/caller-constructible observation record into an
authority-issued evidence token; #1059/#1054 still own that boundary.

## Non-claims

No checked raw-world adoption, complete-world structural integrity, uniqueness
across independently loaded allocator instances/processes/restarts, persistence
authenticity, cryptographic provenance, trusted time, elapsed duration,
continuous trajectory/occupancy, route, dwell, arrival, custody, delivery,
distributed consensus, or settlement eligibility.
