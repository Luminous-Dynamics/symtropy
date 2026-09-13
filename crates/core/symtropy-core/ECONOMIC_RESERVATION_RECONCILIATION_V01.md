# ECON-04C — Reservation Resolution / Partition Reconciliation v0.1

Status: frozen source contract for qualification

## Purpose

ECON-04C closes the authority gap between exact `LotId`-bound stock reservations and ECON-03's multi-resolution / multi-partition economic representation.

ECON-03 deliberately aggregates stock by:

```text
CommoditySpecId × owner × custodian × location
```

At coarse resolution, `LotId` is therefore not part of the ordinary stock conservation key. ECON-04C forbids treating that aggregation as sufficient evidence for an active reservation whose meaning depends on one exact lot.

## Governing resolution theorem

For reservation manifest `R` bound to ECON-03 snapshot `S`:

```text
if R.active is non-empty
and S.detail == Aggregated:
    S must contain a DetailRetentionRef
```

A coarse snapshot with active exact-lot reservations and no retained-detail reference fails closed.

A pure ECON-03 resolution transition must additionally satisfy both conservation equalities:

```text
economic_manifest_before == economic_manifest_after

and

reservation_manifest_before == reservation_manifest_after
```

Creating, releasing, resizing, reassigning, or otherwise changing a reservation is an economic mutation, not a representation/fidelity transition. Likewise, changing stock, accounts, currencies, monetary supply, or canonical economic history cannot be hidden inside a reservation-side fidelity rebind.

`ReservationResolutionBinding` therefore carries the exact `EconomicConservationManifest` and exact `ReservationConservationManifest` it was attached to. `rebind_unchanged(...)` requires both to remain identical.

## Exact-stock / snapshot binding

Before accepting a reservation overlay for an ECON-03 snapshot, ECON-04C independently reconstructs the snapshot's stock conservation projection from the supplied exact `StockLedger`:

```text
exact StockLedger
    ↓ group by
CommoditySpecId × owner × custodian × location
    ↓
StockConservationRecord[]
```

That reconstructed stock vector must equal `snapshot.manifest().stock` exactly.

This prevents a valid reservation ledger from being attached to a coarse stock manifest from another economic instant. Because ECON-04B separately proves each active reservation against the same exact `StockLedger`, every active reserved lot is thereby anchored to stock that is actually represented in the bound ECON-03 snapshot.

ECON-04C does not independently reconstruct finance in this helper. Instead, the complete ECON-03 manifest captured at the source binding must remain byte-for-byte semantically unchanged across a pure rebind.

## Reservation conservation manifest

`ReservationConservationManifest` retains exactly:

```text
canonical active StockReservation records
+
exact append-only StockReservationLedgerEntry history
```

The active set preserves exact:

- `StockReservationId`;
- `LotId`;
- `CommoditySpecId`;
- reserved quantity;
- authorizing owner;
- holder;
- purpose identity;
- authorization identity.

The history preserves released-ID lineage and prevents a coarse representation from forgetting an old reservation identity and later reusing it.

## Retained-detail boundary

ECON-03 v0.1 already permits an aggregated snapshot to bind `DetailRetentionRef` as external evidence of retained fine detail.

ECON-04C requires such a reference when active reservations exist, because the exact `LotId` cannot be reconstructed uniquely from an aggregate stock conservation key.

When no active reservations exist, ECON-04C does not strengthen ECON-03's existing detail-discard policy merely because the reservation reconciliation layer is present.

ECON-04C does not cryptographically verify the external retained-detail archive. It inherits ECON-03's v0.1 evidence boundary.

## Partition theorem

Each ECON-04B active reservation is assigned to the same ECON-03B partition that owns its live lot's stock conservation key:

```text
lot
  ↓
CommoditySpecId × owner × custodian × location
  ↓
exactly one ECON-03B stock partition
  ↓
reservation authority for that LotId
```

No independent reservation-placement policy exists in v0.1.

Before partitioning reservation authority, ECON-04C reconstructs the ECON-03B global manifest and again requires its stock projection to equal the supplied exact `StockLedger`. This prevents a reservation partition from being generated against a partition set for another economic instant.

This prevents stock authority from living in region A while its authoritative encumbrance silently lives in region B.

## Shared-global history

Active reservation projections are partitioned regionally.

Canonical append-only reservation history remains one shared-global authority within a `ReservationPartitionSet`, analogous to ECON-03B's treatment of global currency/history state.

Copied read-only observations may exist elsewhere, but copied authority may not participate in reconstruction.

## Repartition theorem

Changing regional topology or load-balancing placement is valid only when:

```text
reconstruct(reservations_before)
    ==
reconstruct(reservations_after)
```

A repartition may change which `EconomicPartitionId` contains an active claim because its stock group moved. It may not change:

- reservation identity;
- lot identity;
- commodity specification;
- quantity;
- authorizing owner;
- holder;
- purpose/authorization references;
- canonical reservation history.

ECON-03C remains authoritative for proving that the underlying economic partition sets represent the same economic instant. ECON-04C proves that reservation authority follows that already-qualified stock placement without semantic change.

## Canonicality

Partition IDs are canonical and strictly ordered.

Within each partition, active reservations are canonical and strictly ordered by `StockReservationId`.

Global reconstruction rejects duplicate authoritative reservation IDs across partitions.

## Relationship to stock aggregation

Several exact lots can share one ECON-03 `StockConservationKey` and therefore aggregate into one coarse stock quantity.

All such lots share one economic partition because the stock key itself has one authoritative placement. ECON-04C uses that existing placement to co-locate exact active reservation records.

ECON-04C does not claim that an aggregated stock quantity identifies which portion belongs to a reserved lot. That interpretation remains dependent on retained exact detail.

## Required adversarial corpus

Qualification must include at least:

- exact ECON-03 snapshot accepts active reservations without a retention reference;
- aggregated snapshot with active reservations and no retention fails closed;
- aggregated snapshot with active reservations and retained detail is admitted;
- no-active-reservation aggregated snapshot may discard detail under existing ECON-03 policy;
- binding rejects an exact `StockLedger` whose reconstructed stock manifest differs from the bound ECON-03 snapshot;
- pure resolution rebind rejects reservation mutation;
- pure resolution rebind rejects ECON-03 economic-manifest mutation even when reservation state is unchanged;
- exact unchanged reservation manifest and economic manifest survive resolution rebind;
- active reservation co-partitions with its stock conservation key;
- empty economic partitions remain represented in reservation partition topology;
- reservation authority duplicated across partitions is rejected;
- canonical ordering corruption is rejected;
- global reconstruction equals the exact source reservation manifest;
- repartition may change placement but preserves exact reconstructed reservation state/history;
- missing stock partition authority fails closed.

## Explicit non-claims

ECON-04C does NOT establish:

- cryptographic integrity/availability of retained external fine detail;
- a new ECON-03 stock aggregation algorithm;
- an independent reconstruction of the complete financial system during `bind(...)`;
- reservation aggregation that discards exact `LotId` identity;
- cross-host consensus or Byzantine replication;
- market, order, contract, escrow, settlement, or shipment semantics;
- automatic execution of stock mutations when reservations change;
- legal lien semantics;
- ownership of routing or simulation scheduling policy;
- that reservation state is embedded directly inside `EconomicConservationManifest` v0.1.

It is a companion reconciliation authority around the existing ECON-03 and ECON-04B surfaces.

## Relationship to the ECON stack

```text
ECON-00/01   stock authority
      ↓
ECON-03      stock/finance conservation across fidelity
      ↓
ECON-03B/C   partition + repartition authority
      ↓
ECON-04      commodity identity
      ↓
ECON-04B     exact stock reservations
      ↓
ECON-04C     reservation fidelity + partition conservation
      ↓
ECON-04D     atomic encumbrance handoff
      ↓
ECON-05      spatial logistics
```

## Qualification rule

Source review and PR mergeability are not executable qualification.

A qualifying helper must freeze the exact ECON-04C product head and exact ECON-04B parent, run dedicated resolution/partition/repartition reservation corpora including dual-manifest drift attacks, regress ECON-04B and the complete prerequisite ECON stack, run formatting/check/strict-Clippy, statically audit the no-aggregation/no-hidden-stock-mutation boundary and dual-manifest binding, and emit machine-readable exact-lineage evidence.
