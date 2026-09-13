# ECON-03B — Economic partition reconciliation v0.1

## Status

ECON-03B defines how one ECON-03 conservation manifest may be decomposed into multiple authoritative simulation partitions and reconstructed without duplicating or losing economic authority.

It is not a geographic partitioner, transport model, distributed database, or regional scheduler.

## Governing theorem

For a partition set bound to one exact ECON-03 source snapshot:

```text
reconstruct(authoritative partitions + shared-global state)
    == exact source EconomicConservationManifest
```

The theorem is intentionally asymmetric. Not every economic datum is region-owned.

## Partitionable authoritative state

V0.1 partitions exactly two classes of authority:

1. stock conservation records;
2. financial account conservation records.

Every authoritative stock key and every authoritative account ID must be assigned to exactly one declared partition.

Missing assignment fails closed.
Duplicate assignment fails closed.
Assignment to an undeclared partition fails closed.

## Stock partition theorem

A stock conservation record is keyed by the ECON-03 tuple:

```text
CommoditySpecId
+ owner ActorId
+ custodian ActorId
+ LocationId
```

V0.1 assigns the complete record to one partition. It does not split one authoritative record's quantity across several partitions.

Therefore two simultaneously active partitions cannot both claim authoritative ownership of the same exact stock key.

If stock physically moves across a regional boundary, that is an economic/logistics state change, not a partition rewrite. The relevant ECON authority must first produce a new exact state/location, after which a new partition set may be constructed.

## Account partition theorem

Every `FinancialAccountId` appears authoritatively in exactly one partition.

Its complete ECON-03 conservation record moves with it:

```text
account ID
owner
currency
class
cumulative debits
cumulative credits
```

A second region may later maintain read-only/cache/projection views, but such mirrors are outside the authoritative partition set and must never be summed as independent financial claims.

## Shared-global authority

Currency state and canonical history are intentionally **not copied into every region**.

One `SharedGlobalEconomicState` holds exactly once:

- currency definitions;
- monetary-authority identities;
- declared monetary supply;
- stock-history identities;
- journal transaction identities;
- monetary-history identities;
- ECON-02B settlement identities.

This prevents a naive regional decomposition from multiplying global currency supply or canonical transaction history by the number of loaded regions.

For example:

```text
100 currency units globally
+ 4 active regions
!= 400 authoritative currency units
```

The authoritative supply remains 100 in the single shared-global state.

## Complete assignment theorem

An `EconomicPartitionPlan` contains:

```text
declared partition IDs
+ exact stock-key -> partition assignment
+ exact account-ID -> partition assignment
```

Before partitioning, the plan's stock-key set must equal the source manifest's stock-key set exactly.

Likewise, the plan's account-ID set must equal the source account-ID set exactly.

Extra, missing, duplicate, or unknown assignments fail closed.

## Source-snapshot binding

Every partition set binds one exact `EconomicSnapshotId` from ECON-03.

Validation against a different snapshot ID fails even if a caller supplies a superficially similar manifest.

This prevents partition metadata from being silently reused as authority for a different economic instant.

## Merge/reconstruction theorem

Reconstruction unions authoritative regional stock and account records under unique keys/IDs, then combines them with the single shared-global currency/history state.

The reconstructed manifest must be canonical and may contain no duplicate authoritative stock key or account ID.

Validation succeeds only when that reconstructed manifest equals the exact source ECON-03 manifest.

## Canonical structure

V0.1 requires deterministic ordering for:

- partition IDs;
- source stock records;
- source account records;
- source currencies;
- stock-history sequences;
- monetary-history sequences.

Journal transaction IDs and settlement IDs must be unique in the source manifest.

Account currency references must resolve to a currency in the shared-global currency set.

## Cross-region economic activity

ECON-03B does not pretend that cross-region commerce can be localized into one authoritative partition by copying state.

A transaction may involve accounts whose authoritative records live in different partitions. Canonical journal and settlement history remains shared-global.

A future coordination layer may schedule or lock those regional actors, but it must preserve the same global accounting authority rather than minting per-region replicas.

## Read-only mirrors

Read replicas, UI caches, prediction models, AI observations, regional summaries, and network replicas may duplicate information operationally.

They are not represented as authoritative `EconomicPartitionManifest` records.

Only authoritative records participate in reconstruction/conservation arithmetic.

This distinction is load-bearing: duplicated observations are allowed; duplicated ownership/claims are not.

## Geographic non-claim

Partition IDs have no built-in geographic meaning in v0.1.

The plan may correspond to map regions, shards, simulation hosts, administrative areas, interest-management cells, or another future decomposition.

ECON-03B validates economic authority placement, not whether the chosen boundaries are geographically sensible.

## Atomic boundary

Partitioning itself is a pure decomposition of one frozen ECON-03 manifest.

It does not:

- move stock;
- change owner/custodian/location;
- post journal transactions;
- issue/retire currency;
- execute settlement;
- move accounts between owners;
- advance simulation time.

Any such change requires a new exact ECON state and therefore a new partitioning operation.

## Explicit non-claims

ECON-03B v0.1 does not establish:

- geographic boundary correctness;
- distributed locking or consensus;
- cross-host transaction protocols;
- network partition tolerance;
- account migration while economic time advances;
- in-transit logistics semantics;
- replicated-cache invalidation;
- load balancing quality;
- automatic partition discovery;
- partial authoritative stock splitting;
- legal jurisdiction of accounts or assets.

It establishes the partition conservation theorem those systems must preserve.

## Successor work

- **ECON-03C** — controlled repartition/migration receipts between partition sets at one frozen economic instant.
- **ECON-04** — commodity units, grades, batches, serialized capital assets, quality and provenance semantics.
- **ECON-05** — spatial logistics, in-transit custody/location, freight capacity and delivery state.

No successor may treat copied regional state as additional economic authority.
