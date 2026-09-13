# ECON-04B — Stock Reservation / Encumbrance Authority v0.1

Status: frozen source contract for qualification

## Purpose

ECON-04B prevents one live economic stock quantity from being simultaneously promised to multiple downstream uses without modifying the established ECON-00/01 stock authority.

It is an append-only reservation overlay over live `LotId`s. It does not create stock or own physical quantity, title, custody, location, commodity semantics, money, contracts, markets, or transport.

## Governing theorem

For validated stock ledger `S` and reservation ledger `R`:

```text
for every live lot L in S:
    reserved(L) = sum(quantity(r))
                  for every active reservation r where r.lot_id == L.lot_id

    0 <= reserved(L) <= L.quantity

    available_for_new_reservations(L)
        = L.quantity - reserved(L)
```

Every active reservation additionally requires:

```text
reservation.lot_id exists in S
reservation.commodity_spec_id == S[lot].commodity_spec_id
reservation.authorized_owner_id == S[lot].owner_id
reservation.quantity > 0
```

No active reservation may silently follow a changed owner, a replacement specification, or a vanished lot.

## Authority separation

`StockLedger` remains the only ECON authority for:

- live stock quantity;
- commodity specification identity;
- owner;
- custodian;
- location;
- establishment, split, transfer, relocation, and depletion history.

`StockReservationLedger` owns only:

- reservation identity;
- quantity encumbered against one live lot;
- owner identity that authorized that encumbrance;
- reservation holder identity;
- opaque purpose and authorization causal references;
- append-only reservation/release history;
- derivation of quantity currently available for additional reservations.

A reservation MUST NOT mutate `StockLedger`.

## Reservation identity

`StockReservationId` is unique across the reservation ledger's complete retained history, not merely among active reservations.

A released ID cannot be reused. This prevents an old shipment, contract, audit record, or causal reference from being rebound to a later claim with different meaning.

## Reservation construction

`StockReservation::new(...)` is a checked event-payload constructor. It binds:

```text
StockReservationId
LotId
CommoditySpecId
quantity
current authorized owner
holder
purpose CausalId
authorization CausalId
```

The constructor proves the referenced stock ledger is valid, the lot exists, quantity is positive, and the asserted authorizing owner is the current owner.

The constructor does NOT independently prove aggregate availability because multiple prepared reservations may race or be submitted together. Aggregate admission belongs to `StockReservationLedger::transact(...)`.

## Atomic admission

A reservation transaction may contain multiple prepared reservation events.

The complete candidate state is replayed and validated before commit. Therefore:

```text
lot quantity = 100
candidate reserve A = 60
candidate reserve B = 50
```

fails atomically. Neither A nor B becomes authoritative.

There is no partial success from a failed multi-reservation transaction.

## Mutation grammar

The v0.1 reservation authority permits only:

```text
Reserve
ReleasePartial
Release
```

There is deliberately no generic:

```text
set_quantity
increase_quantity
set_holder
set_owner
set_purpose
move_to_lot
```

Increasing an encumbrance requires a distinct `StockReservationId`. Changing holder, purpose, authorization, or target lot requires releasing the prior claim and authoring a new one.

`ReleasePartial` must reduce but not eliminate the active claim. Releasing the complete remaining quantity uses the explicit `Release` event.

## Recovery from stock-relative invalidity

A stock mutation outside this overlay can make an active reservation impossible. Examples include:

- owner changes;
- depletion below reserved quantity;
- full depletion/removal of the lot;
- a lot split that leaves too little quantity on the reserved parent.

This invalidity must be detectable; it must never silently rewrite the reservation.

However, fail-closed behavior must not trap the system permanently. Reservation transactions therefore require structural reservation-ledger validity before execution and complete stock-relative validity **after** execution.

This allows a full `Release` to repair a stale reservation. A partial release that leaves the remaining claim invalid still fails atomically.

## Stock split boundary

A reservation binds one exact `LotId`.

Splitting the underlying stock lot does not implicitly migrate, duplicate, or divide its reservations. The reservation remains bound to the original parent `LotId`.

If the parent's remaining quantity still covers all active reservations, the combined state remains valid. If it does not, validation fails closed.

Moving a reservation to a child lot requires a future explicit typed authority; ECON-04B v0.1 does not invent that transition.

## Ownership boundary

A reservation binds `authorized_owner_id` to the live owner at reservation creation.

Changing stock ownership underneath an active reservation makes the combined state invalid. The reservation does not silently become an encumbrance authorized by the new owner.

ECON-04B does not itself prevent the independent ECON-00/01 ownership API from being called. Downstream market/contract settlement that promises reservation-aware title transfer must coordinate the two authorities atomically in a later layer.

## Custody and location boundary

Reservations do not bind custodian or location. This is intentional: a reserved lot may later move through physical custody or locations while the same economic claim remains active.

Transport authority must bind those transitions explicitly in ECON-05 rather than making reservation identity double as shipment state.

## Replay theorem

Reservation history is append-only and sequence-numbered.

Independent replay must reconstruct exactly:

```text
active reservations
+
complete seen StockReservationId census
```

The materialized projection is invalid if it differs from replay.

Released reservations disappear from the active map but their IDs remain permanently seen within retained history.

## Availability meaning

`available_quantity()` means:

> live stock quantity not presently encumbered by this authoritative reservation ledger.

It does NOT mean:

- physically reachable;
- saleable under law or contract;
- transportable;
- unspoiled;
- production-qualified;
- affordable;
- market-listed;
- free of external liens unknown to this authority.

Those require separate typed authorities.

## Causal references

`purpose_id` and `authorization_id` are opaque `CausalId` references. ECON-04B preserves their identity but does not establish that the referenced contract, signature, delegation, shipment, production job, or policy is valid.

Higher-level systems remain responsible for that evidence.

## Time boundary

ECON-04B v0.1 has no wall-clock expiry and no implicit timeout.

A claim remains active until an explicit append-only release event. Future expiry semantics, if needed, must use deterministic simulation/economic time and typed authority rather than host wall clock.

## Required adversarial corpus

Qualification must include at least:

- reservation reduces availability without mutating stock;
- multiple reservations sum exactly;
- over-reservation fails atomically;
- atomic multi-reservation detects combined overbooking;
- reservation ID cannot be reused after release;
- partial release only decreases a claim;
- whole remaining quantity cannot masquerade as partial release;
- incorrect holder release fails;
- incorrect owner at creation fails;
- owner change makes an active reservation invalid;
- full release can repair an owner-changed reservation;
- partial release cannot leave an owner-changed reservation active;
- underlying depletion below reserved quantity is detected;
- lot split never implicitly migrates reservation authority;
- append-only history replays exactly;
- zero quantity fails closed;
- invalid stock ledger fails closed;
- failed transactions leave active state and history unchanged.

## Explicit non-claims

ECON-04B does NOT establish:

- a legal lien or security interest;
- cryptographic proof that the authorizing owner actually signed;
- delegation or agency authority;
- contract validity;
- price, quote, order, auction, or market semantics;
- monetary escrow or settlement;
- stock ownership/custody/location changes;
- fulfillment, delivery, shipment, routing, vehicle capacity, or travel time;
- process-input consumption;
- automatic reservation migration across lot splits or merges;
- reservation expiry;
- priority/preemption among reservations;
- distributed consensus on reservation state;
- automatic prevention of arbitrary direct calls to ECON-00/01 stock mutation APIs;
- ECON-03 cross-resolution or cross-partition reservation conservation yet.

Later adapters that claim market, contract, manufacturing, or logistics correctness must require and preserve this reservation authority instead of treating it as optional UI state.

## Relationship to the ECON stack

```text
ECON-00/01  stock identity + physical economic quantity
     ↓
ECON-02     finance / monetary settlement
     ↓
ECON-03     resolution + partition conservation
     ↓
ECON-04     commodity/unit/batch/serialized semantics
     ↓
ECON-04B    stock reservation / encumbrance
     ↓
ECON-05     spatial logistics authority
     ↓
later       markets / contracts / firms / households
```

## Qualification rule

Source review, static audit, PR mergeability, and generic repository CI do not constitute executable qualification.

A qualifying helper must freeze the exact ECON-04B product head and exact ECON-04 parent, assert the intended stack delta, execute the dedicated reservation theorem corpus, regress the complete prerequisite ECON authority surface, run repository formatting/check/test/strict-Clippy gates, audit the mutation/non-authority surface, and preserve machine-readable evidence that distinguishes product failure from qualifier or infrastructure failure.
