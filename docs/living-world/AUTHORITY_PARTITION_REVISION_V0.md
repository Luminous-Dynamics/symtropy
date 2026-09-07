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

This contract freezes the revision/epoch semantics around authority partitioning.

## Core invariant

> Once an exact extensive share has transferred from coarse authority into a Level-A owner, that exact share is stable until an explicit canonical flux, settlement, collapse, death, or promotion transaction changes its ownership or amount.

Changing the source stratum's later `count`, `biomass`, or representation does not retroactively rebalance existing active shares.

## No pre-owned prospective slots

Level-P projection owns no authority slot and no exact extensive share.

Before realization, a prospective candidate may identify:

- source scope;
- source revision/schema;
- projection scheme;
- candidate-local presentation facts;

but it does not reserve future biomass.

Therefore a coarse stratum does not have to keep a hidden pre-allocation for every visible candidate.

Exact ownership is computed only at the atomic realization/reservation boundary.

## Source remainder authority

After some Level-A transfers, canonical authority is partitioned conceptually as:

```text
original stratum authority
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
```

while each active refinement owns its transferred exact share independently.

The sum of all current owners must reconcile to the appropriate canonical conservation scope.

## Reservation transaction

A new realization from a current source remainder should conceptually preflight:

```text
source scope matches
source revision is current
source schema/capability supports requested realization
compatible stratum exists
remaining_count > 0
extensive resolution is sufficient
allocation scheme is current
candidate/source facts are compatible
```

Then compute an exact share from the **current canonical remainder state** under the frozen allocation rule and commit atomically:

```text
before:
    coarse remainder: (N, B)

commit one authority slot:
    share = exact_share_for_current_partition(...)

    coarse remainder: (N - 1, B - share)
    active owner:     (1, share)
    source revision:  R -> R + 1
```

No partial state is visible.

## Relationship to cumulative partition

If the only changes to a stratum are sequential reservations, then computing shares from the successive exact remainders must be equivalent to the frozen cumulative partition of the original `(N, B)` authority.

For the preferred cumulative v0 allocation this produces the same exact telescoping ownership sequence and preserves the `< 1 unit` prefix proportionality bound.

This equivalence should be qualified rather than merely assumed.

## Intervening ecological mutation

Suppose one active share has already transferred, and then the coarse remainder receives or loses exact biomass before another realization.

Example:

```text
R0: coarse (N=10, B=1000 mg)
    -> realize A with exact share
R1: coarse remainder receives +73 mg growth/resource settlement
R2: realize B
```

A's share remains exactly what R0 transferred unless A itself participates in an explicit flux.

B's future share is computed from the canonical remainder at R2, not by retroactively pretending the original R0 partition never changed.

The ecological mutation therefore advances the source authority revision/partition epoch.

Any prospective allocation fact that depended on the old source revision is stale.

## Partition epoch

Conceptually, authority handles should bind to a partition epoch/revision sufficient to distinguish:

```text
same population scope
same stratum key
but different current authority remainder
```

A canonical authority slot coordinate therefore needs enough context to prevent collision across rebases, for example conceptually:

```text
AuthoritySlotCoordinate {
    population_scope,
    stratum_key,
    partition_epoch,
    slot_ordinal,
    allocation_scheme_version,
}
```

The exact type is deferred.

Slot ordinal alone is never globally meaningful.

## Existing active handles survive source revision changes

A source revision bump does **not** automatically invalidate an already-authoritative Level-A owner.

The active handle has crossed the authority boundary and now owns its exact share.

Instead:

- old **prospective** handles against the changed source may become stale;
- future reservation intents must use the new source revision;
- already-active owners remain valid under their own active generation/handle until explicitly changed.

This distinction is essential.

Otherwise every coarse ecological update could invalidate nearby active organisms.

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

When an active refinement safely collapses back into compatible coarse authority, it returns its **current owned exact share**, not a newly calculated share under the current source count.

Conceptually:

```text
current coarse remainder Bc
+ active current exact share Ba
--------------------------------
new coarse biomass = Bc + Ba
```

with checked arithmetic and an atomic count/joint-state recombination.

If the active organism changed age, condition, location, disease state, or other canonical axes while active, it may collapse into a different compatible current stratum rather than its source stratum.

Conservation ownership and information placement are both part of the transaction.

## Birth/reproduction

Reproduction is not a share rebalance.

A new organism/cohort obtains exact biomass through explicit canonical source fluxes, for example from parent/resource authority according to the biological model.

Then the affected coarse/active authority states advance revision.

The existence of a new member does not rewrite old ownership history.

## Migration

Migration moves existing exact authority between spatial/population scopes.

If an active organism migrates, its exact share travels with its active owner.

If coarse members migrate, the source/destination coarse states change through explicit exact settlement and their authority revisions advance.

No slot identity is inferred from container order after migration.

## Concurrency

Two realizations racing against the same source revision cannot both commit as if they saw the same unmodified remainder.

At most one can commit against revision `R`; the other must:

- retry/re-evaluate against `R+1`; or
- resolve to an already-existing authoritative refinement when the higher-layer realization key says they targeted the same candidate.

This is an optimistic-concurrency-style semantic requirement, independent of the eventual locking/transaction implementation.

## Qualification requirements

The Living World Observatory should establish at least:

1. sequential reservations without intervening ecology reproduce the frozen cumulative exact partition;
2. an already-active exact share is unchanged when coarse remainder biomass changes;
3. an already-active exact share is unchanged when coarse remainder count changes;
4. source mutation advances revision/partition epoch deterministically;
5. prospective handles against a stale source revision fail before authority mutation;
6. already-authoritative active handles remain valid across unrelated coarse source revision changes;
7. two concurrent commits against one source revision cannot duplicate authority;
8. retry against the new revision produces an exact partition of the new remainder;
9. collapse returns the active owner's current exact quantity, not a recomputed source share;
10. active death transfers owned quantity to the declared destination without survivor rebalance;
11. reproduction adds new authority only through explicit exact source flux;
12. migration preserves the active exact share or explicitly settles coarse source/destination remainders;
13. all transitions retain exact count/biomass conservation and fail closed on arithmetic/revision mismatch.

## Design principle

**Authority is transferred, not continuously re-priced. Once an organism owns exact conserved matter, the rest of the population can evolve without rewriting that organism's accounting history.**
