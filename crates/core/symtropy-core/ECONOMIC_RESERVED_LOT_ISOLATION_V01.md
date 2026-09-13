# ECON-05B — Reservation-Aware Lot Isolation v0.1

Status: frozen source contract for qualification

## Purpose

ECON-05B closes the mismatch between partial economic reservations and whole-lot physical logistics.

ECON-05A correctly refuses to load a reservation that covers only part of a lot because `StockLedger::Relocate` moves the complete lot. ECON-05B creates a dedicated child lot of exactly the reserved quantity and retargets only that reservation onto the child while conserving all physical and economic totals.

The intended runtime chain is:

```text
partial purchase/procurement reservation
        ↓
ECON-05B isolate reserved quantity onto child lot
        ↓
ECON-04D semantic handoff to transport reservation
        ↓
ECON-05A whole-lot freight load
```

## Governing transition

For parent lot `P` of quantity `N`, target reservation `R` of quantity `Q`, and new child lot `C`:

```text
0 < Q < N
R.LotId == P.LotId
```

ECON-05B builds two candidates:

```text
candidate stock:
    Split(P, C, Q)

candidate reservations:
    Release(R)
    Reserve(R', lot=C, quantity=Q)
```

`R'` must preserve exactly:

- commodity specification;
- quantity;
- authorized owner;
- holder;
- purpose;
- authorization.

Only reservation identity and lot identity change.

The child lot must inherit exactly from the pre-split parent:

- commodity specification;
- owner;
- custodian;
- location.

## Conservation theorem

Let `reserved_before(P)` and `available_before(P)` be the pre-isolation reservation projection.

After isolation:

```text
P.quantity_after + C.quantity == P.quantity_before
C.quantity == Q

reserved_after(P) + reserved_after(C) == reserved_before(P)
available_after(P) + available_after(C) == available_before(P)
```

Every active reservation other than `R` remains byte-for-byte unchanged.

This means lot isolation changes physical partitioning and target lot identity without creating/destroying stock, economic encumbrance, or availability.

## Why the reservation transaction can repair the split

Immediately after the candidate stock split, the old reservation overlay may be stock-relative stale because `R` still points at the now-smaller parent.

`StockReservationLedger::transact(...)` deliberately requires structural ledger validity before event application and full stock-relative validity only for the final candidate. ECON-05B uses that narrow repair semantics to atomically release `R` and reserve `R'` on the child against the split stock candidate.

No stale intermediate candidate is exposed to the caller.

## Coordinator atomicity boundary

Both caller ledgers are cloned first. All split, reservation retargeting, conservation checks, historical receipt construction, and receipt validation happen on the candidates.

Only after every fallible operation succeeds are the caller's `StockLedger` and `StockReservationLedger` replaced. Exclusive mutable borrows prevent caller observation between those two infallible replacements.

Append-only histories themselves do not contain a shared cross-ledger transaction-group identifier. Historical replay therefore proves the exact paired split/release/reserve relationship and conservation, but does not independently prove external durable-storage atomic commit across two separately persisted ledgers.

## Historical receipt theorem

`ReservedLotIsolationReceipt` binds:

- exact stock-history counts before/after isolation;
- exact reservation-history counts before/after isolation;
- exact split sequence;
- exact adjacent release/reserve sequences;
- isolation cause;
- complete source and successor reservations;
- parent quantity before/after;
- conserved total reserved quantity;
- conserved total available quantity.

Validation reconstructs the exact pre/post stock and reservation prefixes and independently re-proves the theorem. Later handoff, loading, reservation release, relocation, depletion, or other append-only events do not erase the historical proof.

## Required adversarial corpus

Qualification must include at least:

- partial reservation isolates to exact child quantity;
- aggregate physical quantity is unchanged;
- child inherits exact commodity/owner/custody/location identity;
- target reservation semantics are unchanged except reservation/lot identity;
- unrelated reservations remain byte-for-byte unchanged;
- aggregate reserved quantity is unchanged;
- aggregate available quantity is unchanged;
- wrong holder fails without mutating either caller ledger;
- duplicate child lot fails without mutating either caller ledger;
- reused successor reservation ID fails without mutation;
- already-whole-lot reservation fails closed as unnecessary isolation;
- exact stock split and adjacent reservation release/reserve events are replayable;
- receipt survives later reservation and stock events;
- end-to-end partial reservation → isolation → ECON-04D handoff → ECON-05A load succeeds.

## Explicit non-claims

ECON-05B does NOT establish:

- physical cutting, packaging, pouring, or material transformation;
- whether the underlying physical material is actually divisible;
- batch-quality homogeneity beyond the existing lot/commodity authority;
- carrier capacity or existence;
- loading, unloading, routing, motion, ETA, fuel, wear, or delivery;
- ownership transfer or monetary settlement;
- legal title, lien, bailment, customs, or insurance semantics;
- distributed-storage atomicity across independently persisted ledgers;
- signatures, delegation, or distributed consensus.

Callers must use ECON-05B only for stock whose existing physical/material authority permits homogeneous lot partitioning.

## Relationship to the logistics stack

```text
ECON-04 / 04B lot + reservation authority
        ↓
ECON-05B reservation-aware lot isolation
        ↓
ECON-04D semantic handoff
        ↓
ECON-05A freight loading
        ↓
ECON-05C transport leg / carrier motion binding
        ↓
ECON-05D capacity, time, cost, risk, spoilage, wear
```

## Qualification rule

Source review and mergeability are not executable qualification.

A qualifying helper must freeze the exact ECON-05B product head and exact ECON-05A parent; run formatting, all-target compile/check, the public isolation corpus including the end-to-end isolation→handoff→load chain, ECON-05A/04D/04C/04B/04 regressions and prerequisite ECON tests, full core tests, strict Clippy, and a static authority audit; then emit machine-readable exact-lineage evidence.
