# ECON-03C — Controlled repartition receipts v0.1

## Status

ECON-03C defines controlled changes to **where economic authority is placed** across an ECON-03B partition set while the underlying ECON-03 economic instant remains frozen.

It is not logistics, ownership transfer, account transfer, live distributed migration, or simulation-time advancement.

## Governing theorem

For source partition set `S`, target placement plan `P`, and resulting target partition set `T`:

```text
reconstruct(S) == reconstruct(T)
source_snapshot_id(S) == source_snapshot_id(T)
shared_global(S) == shared_global(T)
```

Only authoritative placement may change.

## Construction theorem

The target partition set is not allowed to supply a new economic manifest.

ECON-03C first reconstructs the exact global manifest from the source ECON-03B partition set and then constructs the target set from that same immutable manifest plus the target placement plan:

```text
source partition set
        ↓ reconstruct
exact ECON-03 manifest
        +
target partition plan
        ↓
target partition set
```

Therefore the repartition plan has no code path to change stock quantity, owner, custodian, location, account totals, currency definitions, monetary supply, or canonical economic history.

## Stock authority movement

For every ECON-03 stock conservation key:

```text
CommoditySpecId
+ owner ActorId
+ custodian ActorId
+ LocationId
```

ECON-03C derives its authoritative source partition and authoritative target partition.

If they differ, one `StockAuthorityMove` is emitted:

```text
exact stock key
from partition
-> to partition
```

The stock record itself is unchanged.

A repartition therefore cannot be used to model physical movement. Changing `LocationId` requires a real economic/logistics transition before a new frozen economic instant is partitioned.

## Account authority movement

For every `FinancialAccountId`, ECON-03C derives its authoritative source and target partition.

If they differ, one `AccountAuthorityMove` is emitted:

```text
exact account ID
from partition
-> to partition
```

Account owner, currency, class, cumulative debits, and cumulative credits remain unchanged.

Repartitioning an account is not a transfer of ownership or value.

## Canonical receipt theorem

`EconomicRepartitionReceipt` records:

- typed `EconomicRepartitionId`;
- exact source `EconomicSnapshotId`;
- evidence/provenance `CausalId`;
- canonical source partition-ID set;
- canonical target partition-ID set;
- canonical stock authority moves;
- canonical account authority moves;
- count of stock authorities that remained in place;
- count of account authorities that remained in place.

Move order is derived from ordered economic identities, not insertion order.

The receipt is not trusted merely because it exists. `validate()` recomputes the complete expected receipt from the supplied source and target partition sets and requires exact equality.

A modified destination partition, omitted movement, invented movement, count change, topology change, ID change, or evidence change causes receipt mismatch.

## No-op rule

A repartition is rejected when all three are simultaneously true:

```text
source partition IDs == target partition IDs
stock authority moves == empty
account authority moves == empty
```

This prevents meaningless authority-transition receipts for a partition set that did not change.

## Topology-only changes

A topology change may still be meaningful even when no current stock or account changes authority.

For example, adding a new empty partition is admitted because:

```text
source partition IDs != target partition IDs
```

while the reconstructed economic manifest remains exact.

This lets later schedulers prepare a partition topology without fabricating economic activity.

## Shared-global theorem

ECON-03B currency state and canonical history remain one shared-global authority throughout repartitioning.

ECON-03C requires exact shared-global equality between source and target.

Therefore repartitioning cannot:

- multiply currency supply;
- fork canonical journal identity;
- fork monetary-event identity;
- fork settlement identity;
- rewrite stock-event identity.

## Frozen-instant theorem

Source and target must bind the exact same `EconomicSnapshotId`.

Repartitioning does not create a new economic instant.

Any intervening economic mutation—production, loss, relocation, trade, settlement, issuance, retirement, wage, tax, contract performance, etc.—requires a newly captured ECON state rather than reuse of the old repartition receipt.

## Read replicas and caches

A repartition receipt concerns **authoritative placement only**.

It does not prohibit old or new regions from holding non-authoritative read replicas, UI caches, AI observations, analytics, or prediction state.

Copied information is not additional authority.

## Atomicity boundary

`repartition_authority()` constructs and validates the complete target partition set and complete receipt before returning either as an admitted result.

A failed target plan, missing authority assignment, duplicate authority, global-manifest mismatch, shared-global mismatch, or receipt mismatch yields no valid target/receipt pair.

V0.1 is a pure value-level construction; it does not claim distributed storage updates are atomically committed across machines.

## Explicit non-claims

ECON-03C v0.1 does not establish:

- live cross-host migration;
- distributed two-phase commit or consensus;
- lock acquisition while simulation time advances;
- geographic boundary quality;
- load-balancing optimality;
- state-transfer networking;
- cache invalidation;
- failure recovery across hosts;
- physical stock movement;
- account ownership/value transfer;
- legal jurisdiction transfer;
- persistent storage durability.

It establishes the deterministic receipt and conservation theorem that those systems must preserve.

## Successor work

- **ECON-04** — commodity units, grades, batches, serialized capital assets, quality and provenance semantics.
- **ECON-05** — spatial logistics and explicit in-transit stock/custody/delivery state.
- A future distributed-runtime layer may use ECON-03C receipts as the semantic subject of authenticated migration protocols.

No successor may treat repartitioning as authority to change the economy being repartitioned.
