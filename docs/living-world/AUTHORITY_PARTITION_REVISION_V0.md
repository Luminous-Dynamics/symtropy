# Living World Authority Partition Revision v0

Status: companion authority contract.

## Purpose

Exact authority-share allocation is only safe if already-transferred ownership remains stable while the coarse remainder continues to evolve.

A population stratum can change after one organism becomes Level A because of:

- additional realizations;
- growth or resource consumption;
- mortality;
- reproduction;
- migration;
- disease/state transitions;
- explicit ecological settlement.

The engine must not respond to those changes by silently recomputing the exact extensive shares of already-active organisms.

This contract freezes the distinction between a **partition epoch** and a changing **source revision**.

## Core invariant

> Once an exact extensive share has transferred from coarse authority into a Level-A owner, that exact share is stable until an explicit canonical flux, settlement, collapse, death, or promotion transaction changes its ownership or amount.

Changing the source stratum's later `count`, `biomass`, or representation does not retroactively rebalance existing active shares.

## Why epoch and revision are different

A source revision can advance for several reasons, including a normal reservation that removes one member from coarse authority.

But ordinary sequential reservations must not silently redefine the exact-share formula for the still-unowned members.

Therefore:

- **partition epoch** freezes the origin authority used by one allocation sequence;
- **source revision** identifies the current mutable remainder and advances as transactions commit.

Conceptually:

```text
PartitionEpoch {
    epoch_id,
    origin_count: N0,
    origin_biomass: B0,
    allocation_scheme_version,
    next_slot_ordinal,
}

CoarseRemainder {
    remaining_count,
    remaining_biomass,
    source_revision,
    active_partition_epoch,
}
```

The exact final representation may differ, but the semantic separation is normative.

## No pre-owned prospective slots

Level-P projection owns no authority slot and no exact extensive share.

Before realization, a prospective candidate may identify:

- source scope;
- source revision/schema;
- projection scheme;
- candidate-local presentation facts;

but it does not reserve future biomass.

Therefore a coarse stratum does not have to keep a hidden per-visible-candidate allocation.

Exact ownership is transferred only at the atomic realization/reservation boundary.

## Stable allocation within one epoch

At the start of a partition epoch, freeze:

```text
N0 > 0
B0 >= 0
allocation scheme version
```

For preferred cumulative v0 exact-share allocation, canonical slot `i` receives:

```text
share(i) =
    floor(B0 * (i + 1) / N0)
  - floor(B0 * i / N0)
```

using widened integer arithmetic.

Normal reservations advance `next_slot_ordinal` and remove the corresponding frozen share from the coarse remainder, but **do not rebase `(N0, B0)` merely because a reservation occurred**.

Thus if no ecological mutation other than reservations occurs:

```text
reservation 0 -> share(0)
reservation 1 -> share(1)
reservation 2 -> share(2)
...
```

and the exact telescoping/prefix theorem from `ACTIVE_EXTENSIVE_SHARE_V0.md` remains valid.

## Important non-equivalence

The following tempting shortcut is **not normative**:

```text
share = floor(current_remaining_biomass / current_remaining_count)
```

recomputed independently after every reservation.

That recurrence is not generally the same sequence as the frozen cumulative partition.

For example:

```text
B0 = 2
N0 = 4

frozen cumulative shares = [0, 1, 0, 1]
naive successive floor   = [0, 0, 1, 1]
```

Both conserve the total, but they have different allocation semantics and prefix behavior.

Therefore the implementation must preserve the partition epoch origin while reservations alone consume slots.

## Source remainder authority

After some Level-A transfers, canonical authority is partitioned conceptually as:

```text
origin epoch authority
    = coarse remainder authority
    + active owner A
    + active owner B
    + ...
```

The coarse remainder owns exact current:

```text
remaining_count
remaining_biomass
remaining_joint_state
source_revision
partition_epoch
```

while each active refinement owns its transferred exact share independently.

The sum of all current owners must reconcile to the appropriate canonical conservation scope.

## Reservation transaction within an unchanged epoch

A new realization should conceptually preflight:

```text
source scope matches
source revision/compatibility is acceptable
source schema/capability supports requested realization
compatible stratum exists
remaining_count > 0
extensive resolution is sufficient
partition epoch is active/current
next slot is not already owned
allocation scheme version matches
candidate/source facts are compatible
```

Then compute the next slot's share from the **frozen epoch origin**, not by rebuilding the partition from the current remainder:

```text
i = next_slot_ordinal
share = cumulative_share(B0, N0, i)
```

and commit atomically:

```text
coarse.remaining_count   -= 1
coarse.remaining_biomass -= share
coarse.source_revision   += 1
epoch.next_slot_ordinal  += 1
active owner              = (1, share, epoch, i)
```

No partial state is visible.

## Reservation revision does not automatically rebase epoch

The source revision may advance after every successful reservation so stale concurrent transactions can be detected.

That does **not** imply a new partition epoch.

If the only source changes are consumption of the epoch's own next authority slots, the epoch remains valid until:

- all its slots are consumed/collapsed as defined by the higher-layer transaction model; or
- an ecological mutation changes the unowned remainder in a way that invalidates the frozen unowned-share schedule.

## Intervening ecological mutation

Suppose one active share has already transferred and then the coarse remainder receives or loses exact biomass, changes member count outside reservation, or changes extensive authority schema before another realization.

Example:

```text
E0 / R0: origin (N0=10, B0=1000 mg)
         -> realize A using E0 slot 0
R1:      coarse remainder receives +73 mg growth/resource settlement
```

A's share remains exactly what E0 transferred unless A itself participates in an explicit flux.

But the unowned E0 slot schedule no longer describes the changed remainder.

Therefore the ecological mutation must **close/rebase the unowned remainder into a new partition epoch** before another share is allocated from that changed authority.

Conceptually:

```text
close E0 for future reservations
keep all already-owned E0 active shares unchanged

create E1 from current unowned remainder:
    N1 = current remaining_count
    B1 = current remaining_biomass
    next_slot = 0
```

Future realizations allocate from E1.

No E0 active owner is rescaled.

## What forces an epoch rebase

At minimum, a new partition epoch is required before future reservation when an operation changes the **unowned coarse authority** in a way not represented by consuming the next slot of the existing epoch, including:

- coarse biomass growth/loss/transfer;
- coarse birth/death outside active-slot realization;
- migration into/out of the coarse remainder;
- change to exact quantity unit/schema;
- adoption of richer canonical per-member/extensive information;
- merge/split of canonical strata that changes the source extensive partition.

A pure read-only observation or rendering change does not rebase authority.

## Partition epoch identity

Authority slot coordinates need enough context to prevent collision across rebases.

Conceptually:

```text
AuthoritySlotCoordinate {
    population_scope,
    stratum_key,
    partition_epoch,
    slot_ordinal,
    allocation_scheme_version,
}
```

Slot ordinal alone is never globally meaningful.

An epoch identity must not come from wall-clock time, renderer state, or nondeterministic execution order.

## Existing active handles survive source revision/epoch changes

A source revision bump or new coarse partition epoch does **not** automatically invalidate an already-authoritative Level-A owner.

The active handle has crossed the authority boundary and now owns its exact share.

Instead:

- old **prospective** handles may become stale or require an explicit compatibility proof;
- future reservation intents use the current source revision/epoch;
- already-active owners remain valid under their own active generation/slot coordinate until explicitly changed.

Otherwise every coarse ecological update would invalidate nearby active organisms.

## Prospective candidate compatibility across reservations

Because Level-P candidates do not own slots, one successful realization can advance source revision while other visible candidates from the same projection family still exist.

The future realization layer must not blindly require numeric revision equality if a stronger compatibility proof can establish that another candidate remains valid under the same source-information assumptions and current remaining authority.

Conversely, it must never preserve a candidate merely for visual continuity when current canonical strata prove that candidate is no longer realizable.

Therefore stale-handle policy should distinguish:

- **revision mismatch**;
- **source-schema incompatibility**;
- **candidate no longer supported by remaining canonical state**;
- **candidate already resolved to an active owner**.

Exact compatibility mechanics are deferred to the convergence PR.

## No retroactive rebasing

These operations are forbidden without an explicit settlement transaction:

```text
source count changed
    -> recompute all active shares
```

```text
source biomass changed
    -> rescale all active shares
```

```text
new organism was born
    -> divide existing active owners by new N
```

```text
one active organism died
    -> silently redistribute its exact share among survivors
```

Death transfers its owned quantity to the declared destination such as detritus. It does not trigger an implicit population-wide rebalance.

## Collapse/recombination

When an active refinement safely collapses back into compatible coarse authority, it returns its **current owned exact share**, not a newly calculated share under the current source count or newest partition epoch.

Conceptually:

```text
current coarse remainder Bc
+ active current exact share Ba
--------------------------------
new coarse biomass = Bc + Ba
```

with checked arithmetic and an atomic count/joint-state recombination.

If the active organism changed age, condition, location, disease state, or another canonical axis while active, it may collapse into a different compatible current stratum rather than its source stratum.

Such recombination may itself require closing/rebasing the destination stratum's unowned partition epoch for future reservations.

Conservation ownership and information placement are both part of the transaction.

## Birth/reproduction

Reproduction is not a share rebalance.

A new organism/cohort obtains exact biomass through explicit canonical source fluxes, for example from parent/resource authority according to the biological model.

The affected coarse/active states advance revision, and a coarse remainder whose count/biomass changed outside ordinary slot consumption must begin a new partition epoch before future reservation.

The existence of a new member does not rewrite old ownership history.

## Migration

Migration moves existing exact authority between spatial/population scopes.

If an active organism migrates, its exact share travels with its active owner.

If coarse members migrate, the source/destination coarse states change through explicit exact settlement and their relevant unowned partition epochs rebase before future reservation.

No slot identity is inferred from container order after migration.

## Concurrency

Two realization transactions may observe the same source revision/epoch.

They cannot both consume the same next slot or duplicate authority.

A valid implementation may serialize commit through revision/slot compare-and-swap semantics, a transaction lock, or another deterministic atomic mechanism, but the semantic result must be equivalent to one unique commit order.

The loser must:

- re-evaluate against the new current revision/next slot; or
- resolve to an already-existing authoritative refinement when the higher-layer realization key says both attempts targeted the same candidate.

## Qualification requirements

The Living World Observatory should establish at least:

1. exhaustive small `(N0, B0)` epochs reproduce the frozen cumulative slot sequence exactly;
2. the explicit `B0=2, N0=4` fixture rejects/guards against naive successive-floor semantics;
3. sequential reservations without intervening ecology preserve the same partition epoch origin and exact telescoping prefix sums;
4. every successful reservation advances source revision and consumes exactly one unique slot;
5. an already-active exact share is unchanged when coarse remainder biomass changes;
6. an already-active exact share is unchanged when coarse remainder count changes;
7. non-reservation ecological mutation closes/rebases the unowned remainder into a deterministic new epoch;
8. active shares from the old epoch remain unchanged across that rebase;
9. prospective handles against incompatible source state fail before authority mutation;
10. compatible multi-candidate realization cannot duplicate one slot/owner merely because handles share an older projection revision;
11. two concurrent commits cannot consume the same slot or duplicate authority;
12. future reservation after rebase uses the new epoch origin, not leftover old-epoch arithmetic;
13. collapse returns the active owner's current exact quantity, not a recomputed source share;
14. active death transfers owned quantity to the declared destination without survivor rebalance;
15. reproduction adds new authority only through explicit exact source flux;
16. migration preserves the active exact share or explicitly settles/rebases coarse source/destination remainders;
17. all transitions retain exact count/biomass conservation and fail closed on arithmetic/epoch/revision mismatch.

## Design principle

**Authority is transferred, not continuously re-priced. Partition epochs freeze the arithmetic of unowned authority long enough to make reservation reproducible; ecological mutation can rebase only the remainder, never rewrite already-transferred ownership.**
