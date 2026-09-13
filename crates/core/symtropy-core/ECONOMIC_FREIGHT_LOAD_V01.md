# ECON-05A — Freight Load Binding v0.1

Status: frozen source contract for qualification

## Purpose

ECON-05A establishes the first physical logistics bridge above ECON-04D without creating a second cargo-availability ledger or pretending that the economy kernel already contains a vehicle simulator.

The authoritative split is:

```text
StockLedger             = physical stock existence, quantity, owner, custody, location
StockReservationLedger  = unavailable-stock / economic encumbrance authority
CarrierCargoBinding     = reference to external physical carrier authority
FreightLoadReceipt      = derivable historical proof of one load
```

A load therefore changes the existing stock lot's custody and carrier-local location while leaving its economic reservation unchanged.

## Governing theorem

For live lot `L`, active ECON-04D successor reservation `R`, carrier binding `C`, and load cause `K`, v0.1 requires:

```text
R is still active and byte-for-byte equal to the ECON-04D successor
R.LotId == L.LotId
R.CommoditySpecId == L.CommoditySpecId
R.authorized_owner == L.owner
R.quantity == L.quantity
reserved(L) == L.quantity
available(L) == 0
R.holder == C.carrier_custodian
L.location != C.cargo_location
```

The committed stock transition is:

```text
optional TransferCustody(L, source -> carrier_custodian, K)
Relocate(L, source_location -> carrier_cargo_location, carrier_custodian, K)
```

The custody transition is omitted only when the carrier custodian already holds the lot. Relocation is always required.

After loading:

```text
L.quantity          unchanged
L.CommoditySpecId   unchanged
L.owner             unchanged
L.custodian         == C.carrier_custodian
L.location          == C.cargo_location
R                    unchanged
reserved(L)         unchanged == L.quantity
available(L)        unchanged == 0
```

The stock mutation is one `StockLedger::transact(...)` call, so an optional custody transfer and mandatory relocation commit together or not at all.

## Full-lot isolation boundary

ECON-05A v0.1 loads only an **isolated, fully reserved lot**.

This is deliberate. `StockLedger::Relocate` moves a complete lot, while ECON-04B reservations may cover only part of one. Silently relocating a partly reserved lot would move unrelated or unreserved material. Splitting a reserved lot without simultaneously reconciling reservation identity can also invalidate the reservation overlay.

Therefore v0.1 fails closed unless:

```text
transport reservation quantity == live lot quantity
and
available quantity == 0
```

Partial loading, multi-reservation lot isolation, split/merge reservation retargeting, and multi-vehicle loading require a later explicit theorem rather than an implicit stock split.

## Carrier-local location indirection

`CarrierCargoBinding` contains:

- `carrier_asset_id`;
- `cargo_location_id`;
- `carrier_custodian_id`;
- `physical_authority_id`.

The stock lot relocates to the carrier-local cargo-space `LocationId`. A future transport authority may move the carrier through world space without rewriting every contained lot on every simulation tick.

This is a reference boundary, not a vehicle theorem. ECON-05A does not independently validate the existence, geometry, capacity, mobility, route access, fuel state, driver, or legal authority of the carrier. The opaque `physical_authority_id` binds the load to whichever physical subsystem later establishes those facts.

## ECON-04D continuity

A load must consume an `EncumbranceHandoffReceipt` and the receipt's successor reservation must still be active at the load boundary.

A historically valid handoff is not enough if the successor reservation has since been released or replaced. This prevents stale handoff evidence from authorizing a new load.

The successor holder must equal the carrier custodian. Brokerage, subcontracting, or carrier reassignment must therefore perform another explicit encumbrance handoff before physical loading in v0.1.

## Historical receipt theorem

`FreightLoadReceipt` is evidence derived from authoritative histories, not an independent source of stock, reservation, carrier, or movement authority.

It binds:

- exact stock-history count immediately before loading;
- exact stock-history count immediately after loading;
- exact reservation-history count at loading;
- optional custody-transfer sequence;
- mandatory relocation sequence;
- complete ECON-04D handoff receipt;
- lot, commodity, quantity, and owner identity;
- source custodian and source location;
- carrier binding;
- load cause.

Validation independently reconstructs the frozen stock and reservation prefixes, verifies the exact event grammar, re-validates the ECON-04D receipt, proves the successor reservation was active at the load boundary, and proves quantity/ownership/encumbrance conservation.

Later stock or reservation events may be appended without erasing proof of the historical load.

## Required adversarial corpus

Qualification must include at least:

- full isolated transport reservation loads successfully;
- quantity, commodity identity, and owner are unchanged;
- reserved quantity remains the full lot quantity and availability remains zero;
- carrier holder/custodian mismatch fails without stock mutation;
- stale source custodian fails without mutation;
- stale source location fails without mutation;
- cargo location equal to source location fails without mutation;
- partial reservation over a larger lot fails without mutation;
- released/replaced successor reservation cannot be loaded from stale handoff evidence;
- same-custodian loading emits relocation only, not a no-op custody event;
- different-custodian loading emits exact adjacent custody + relocation events;
- receipt validates immediately after loading;
- receipt survives later reservation release;
- receipt survives later stock depletion/relocation by replaying the exact historical prefixes;
- tampered event order, identities, or load cause fail validation.

## Explicit non-claims

ECON-05A does NOT establish:

- carrier existence beyond an opaque external-authority binding;
- cargo mass/volume capacity;
- axle load, center of mass, packing, securing, or container geometry;
- vehicle dynamics, route, path, road, rail, sea, air, or terrain access;
- travel time, ETA, fuel burn, energy use, wear, maintenance, weather, or risk;
- partial or multi-lot loading;
- one-to-many or many-to-one reservation transformation;
- ownership transfer;
- monetary settlement;
- delivery, acceptance, unloading, or fulfillment;
- legal bailment, lien, insurance, or customs semantics;
- signatures, delegation, or distributed consensus.

Those are later logistics/contract layers.

## Relationship to the ECON stack

```text
ECON-04B exact stock reservation
        ↓
ECON-04C resolution/repartition conservation
        ↓
ECON-04D atomic encumbrance handoff
        ↓
ECON-05A isolated full-lot freight loading
        ↓
ECON-05B transport leg / carrier motion binding
        ↓
ECON-05C capacity, time, cost, risk, spoilage, wear
        ↓
ECON-06 spatial market settlement
```

## Qualification rule

Source review and PR mergeability are not executable qualification.

A qualifying helper must freeze the exact ECON-05A product head and exact ECON-04D parent; run formatting, compile/check, the public ECON-05A adversarial corpus, ECON-04D/04C/04B/04 regressions and prerequisite ECON tests, full core tests, and strict Clippy; statically audit that no parallel cargo-availability ledger or quantity mutation was introduced; assert the one-or-two-event load grammar; and emit machine-readable exact-lineage evidence.
