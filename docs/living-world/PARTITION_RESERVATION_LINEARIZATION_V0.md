# Partition Reservation Linearization V0

## Status

Normative Living World authority contract. This document refines `AUTHORITY_PARTITION_REVISION_V0`, `ACTIVE_EXTENSIVE_SHARE_V0`, and `ACTIVE_REALIZATION_TRANSACTION_V0` with the concurrency semantics required to consume exact authority slots safely.

It does not change PR #201's arithmetic oracle.

## Problem

A frozen partition epoch gives deterministic exact shares for authority slots, but arithmetic alone does not establish exclusive ownership under concurrent realization.

Two observers may simultaneously prepare against the same state:

```text
epoch E, next_slot = 7

observer A prepares slot 7
observer B prepares slot 7
```

Or an ecological mutation may rebase the unowned remainder while an older realization plan is still in flight:

```text
plan P prepared against epoch E
      |
      +-- ecological mutation commits -> epoch E+1
      |
      `-- stale plan P tries to commit
```

If these races are not explicitly linearized, exact arithmetic can still duplicate or misprice ecological authority.

## Core rule

Preparing a realization does not consume a slot.

Slot ownership transfers only at one canonical **linearization point** inside the realization transaction, where the implementation atomically validates the expected partition/source state and commits the reservation.

Conceptually a prepared plan binds at least:

```text
PartitionReservationExpectation {
    population_or_stratum_scope,
    partition_epoch_id,
    expected_source_state_token,
    expected_allocation_coordinate,
    exact_share,
    realization_request_id,
}
```

The exact Rust shape is non-normative.

## Epoch identity

`partition_epoch_id` must not be merely the current `(remaining_count, remaining_biomass)` tuple.

Two different authority histories can have the same numeric remainder.

The epoch therefore needs a non-aliased/versioned identity or source-state binding sufficient to distinguish semantically different partition histories.

A rebase closes the previous epoch for new commits.

## Allocation coordinate

V0 may use a monotonic `next_slot` within an epoch, which is attractive because PR #201's cumulative partition has strong prefix properties.

If a future policy permits non-prefix/arbitrary slot reservation, it must explicitly qualify:

- uniqueness of each slot coordinate;
- remaining-unowned accounting;
- deterministic selection;
- interaction with projection identity;
- rebase semantics;
- bounded metadata for sparse reserved slots.

Do not silently move from prefix allocation to arbitrary slot allocation while retaining the same partition scheme version.

## Prepare

`PREPARE/VALIDATE/PLAN` may read the current epoch and compute the expected exact share.

They must not:

- decrement source count;
- decrement source biomass;
- advance `next_slot`;
- publish active ownership;
- advance quantization residuals;
- mutate derived fields.

The resulting plan is prospective and may become stale.

## Commit / linearization point

At commit, atomically validate at least:

1. source scope still matches;
2. source authority stamp/token still matches;
3. partition epoch is still open and identical to the expected epoch;
4. expected allocation coordinate is still available/current;
5. exact share recomputes/validates under the frozen epoch scheme;
6. realization request has not already committed under a different result;
7. source count/stock can settle atomically with active-owner publication.

Only then may the transaction:

```text
consume authority slot
+ transfer exact share
+ decrement/coarsen source authority as defined
+ publish Level-A owner
```

as one canonical commit.

## Concurrent realization

If A and B both prepare the same next slot, at most one may commit it.

The loser must:

- observe a stale/conflict result;
- consume no count/biomass;
- publish no active owner;
- either retry from the new canonical state or return the already-committed result when request identity proves it is an idempotent retry.

Scheduler/thread timing must not allow double ownership.

## Rebase race

An ecological mutation that changes unowned count/stock closes the old epoch and creates a new one over the post-mutation unowned remainder.

Any uncommitted reservation plan from the old epoch becomes stale.

It may not be "adjusted" by substituting the new epoch's share while preserving the old candidate transaction silently.

Allowed responses:

- reject/retry against the new epoch;
- an explicitly qualified compatibility path that revalidates every affected assumption before forming a new plan.

Already committed active shares remain unchanged.

## Idempotent retry

`realization_request_id` or equivalent canonical transaction identity must distinguish:

- a duplicate retry of a previously committed realization;
- a new competing realization request.

A duplicate retry must not consume the next slot again.

It should return/reference the committed result when policy permits, or fail with an explicit already-committed identity. Either behavior preserves single ownership.

## Crash boundary

If persistence is introduced around Level-A realization, the durable commit must not allow a crash state in which:

- the source slot/count is consumed but the active owner is absent;
- the active owner exists but source authority was not removed;
- the partition cursor advanced without a corresponding authoritative owner/settlement.

Persistence/journaling integration may choose the mechanism later, but these partially committed states are invalid.

## Qualification fixtures

Minimum future executable evidence should include:

1. two plans prepared for the same next slot -> exactly one commits;
2. loser observes zero mutation and can reprepare against the new state;
3. duplicate retry of winner consumes no second slot;
4. ecological rebase after prepare but before commit invalidates old plan;
5. already committed shares do not change during rebase;
6. stale epoch ID cannot commit even when `(remaining_count, remaining_stock)` numerically matches again;
7. source-state-token change invalidates the plan;
8. commit atomically transfers source count/share and publishes active ownership;
9. injected failure before linearization leaves everything unchanged;
10. injected failure after the durable linearization point recovers to exactly one owner rather than zero/two owners once persistence is integrated;
11. execution/thread ordering cannot produce duplicate slot ownership;
12. changing allocation from prefix to arbitrary-slot policy requires a new explicit scheme/version.

## Relationship to PR #201

PR #201 remains the mathematical oracle for exact share calculation and partition-epoch rebasing.

It deliberately does not solve concurrency.

The future Level-A product layer composes that arithmetic with this reservation protocol.

## Non-goals

This contract does not define:

- distributed consensus;
- network leases;
- Bevy entity ownership;
- persistent Level-I organism identity;
- exact database/journal technology;
- arbitrary-slot projection policy.

It freezes the rule that **an exact authority partition is not safely consumable until slot acquisition and epoch closure have one atomic, replayable canonical linearization point**.
