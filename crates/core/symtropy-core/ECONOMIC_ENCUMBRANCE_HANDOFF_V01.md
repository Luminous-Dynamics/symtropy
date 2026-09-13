# ECON-04D — Atomic Encumbrance Handoff v0.1

Status: frozen source contract for qualification

## Purpose

ECON-04D closes the transient-double-spend gap between one economic phase that owns an ECON-04B stock reservation and the next phase that must continue encumbering exactly the same stock.

It deliberately does **not** create a cargo, shipment, freight, or fulfillment ledger. Instead it reuses the already-qualified ECON-04B reservation authority and atomically replaces one complete active reservation with one complete successor reservation.

This makes the handoff useful for later transitions such as procurement → loading, loading → transport, transport → delivery hold, or storage → production without introducing parallel unavailable-quantity authorities.

## Governing theorem

For source reservation `S`, successor reservation `T`, live stock lot `L`, and reservation ledger `R`:

```text
before:
    S is active
    T is not active

atomic transaction:
    Release(S)
    Reserve(T)

after:
    S is not active
    T is active
```

The two reservation events are adjacent in canonical reservation history and are committed by one `StockReservationLedger::transact(...)` call.

The identity-continuity theorem is:

```text
S.LotId              == T.LotId
S.CommoditySpecId    == T.CommoditySpecId
S.quantity           == T.quantity
S.authorized_owner   == T.authorized_owner
S.reservation_id     != T.reservation_id
```

The unavailable-quantity theorem is:

```text
reserved_on_lot_before == reserved_on_lot_after
available_on_lot_before == available_on_lot_after
```

There is therefore no committed intermediate state in which the handed-off quantity becomes newly available.

## Semantic-transition requirement

A handoff is not allowed to be a pure reservation-ID rewrite.

At least one of these successor fields must differ from the source:

- holder;
- purpose;
- authorization reference.

This keeps the transition evidence meaningful and prevents meaningless identity churn from masquerading as a phase handoff.

## Full-handoff-only boundary

ECON-04D v0.1 supports only a **full** reservation handoff.

One source reservation of quantity `Q` becomes one successor reservation of the same quantity `Q`.

Partial handoff, one-to-many split, many-to-one merge, and staged multi-vehicle loading are intentionally deferred. Callers that need independent quantities should establish appropriately sized reservations before invoking this authority.

## Failure atomicity

`handoff_full_reservation(...)` does not mutate the caller's reservation ledger incrementally.

It:

1. validates exact stock and reservation state;
2. derives the source and successor claims;
3. clones the reservation ledger;
4. applies `Release(source)` + `Reserve(successor)` to the clone in one ECON-04B transaction;
5. proves reserved and available quantity conservation;
6. constructs and validates the handoff receipt against the candidate history;
7. replaces the caller ledger only after all checks succeed.

Any error before step 7 leaves the caller ledger unchanged.

## Historical receipt theorem

`EncumbranceHandoffReceipt` is evidence derived from authoritative histories, not a new source of authority.

It binds:

- stock history event count at handoff;
- source-release reservation sequence;
- adjacent successor-reserve sequence;
- handoff causal identity;
- exact source reservation;
- exact successor reservation;
- conserved total reserved quantity on the lot;
- conserved available quantity on the lot.

Receipt validation truncates retained stock and reservation histories to those exact historical boundaries and independently replays the transition.

Later stock or reservation events may therefore be appended without invalidating the historical receipt, provided the original histories remain intact.

## Availability authority

ECON-04D intentionally keeps both source and successor claims inside the same ECON-04B reservation authority.

This avoids a dangerous architecture in which a reservation ledger says stock is available while a separate cargo ledger says it is unavailable.

Downstream ECON-05 logistics may interpret a typed successor reservation as a transport commitment, but ECON-04D itself does not establish shipment semantics.

## Required adversarial corpus

Qualification must include at least:

- successful full handoff replaces source with successor;
- total reserved quantity on the lot is unchanged;
- available quantity on the lot is unchanged;
- unrelated reservations on the same lot remain unchanged;
- release and successor-reserve history entries are adjacent and exact;
- wrong source holder fails without mutation;
- reused source/successor reservation identity fails without mutation;
- previously seen successor reservation ID fails without mutation;
- semantic no-op handoff fails without mutation;
- receipt validates immediately after handoff;
- receipt still validates after later reservation-history appends;
- receipt still validates after later stock-history appends by replaying the bound historical stock prefix;
- source is provably active before the receipt boundary and absent after it;
- successor is provably absent before the receipt boundary and active after it.

## Explicit non-claims

ECON-04D does NOT establish:

- partial reservation handoff;
- one-to-many or many-to-one encumbrance transformation;
- shipment, freight, route, vehicle, loading, unloading, delivery, or ETA semantics;
- physical custody transfer;
- stock relocation;
- ownership transfer;
- monetary settlement or escrow;
- legal lien semantics;
- signatures or delegation proof;
- cross-host consensus;
- that the successor reservation has physically entered a vehicle.

It proves only atomic semantic succession of unavailable-stock authority inside ECON-04B.

## Relationship to the ECON stack

```text
ECON-04B exact stock reservation
        ↓
ECON-04C fidelity / partition conservation
        ↓
ECON-04D atomic full encumbrance handoff
        ↓
ECON-05 spatial logistics
```

## Qualification rule

Source review and PR mergeability are not executable qualification.

A qualifying helper must freeze the exact ECON-04D product head and exact ECON-04C parent, run the dedicated public handoff corpus, regress ECON-04C/04B/04 and prerequisite ECON layers, run formatting/check/strict-Clippy, statically audit that ECON-04D does not mutate stock or create a parallel availability ledger, assert the exact two-event transaction grammar, and emit machine-readable exact-lineage evidence.
