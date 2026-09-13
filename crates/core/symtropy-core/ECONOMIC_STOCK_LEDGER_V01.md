# ECON-00/01 — Economic identity and stock ledger v0.1

## Status

This contract defines the first Symtropy economic authority boundary. It is an accounting and provenance kernel, not a market, finance, manufacturing, or physical-control subsystem.

## Governing theorem

For stock admitted to this kernel:

```text
current economic stock state
    == deterministic replay(complete canonical stock-event history)
```

and no supported mutation can silently overwrite quantity, ownership, custody, or location.

A valid stock event means only that the economic ledger has an explicit typed cause and authority transition. It does **not** independently prove that an external industrial, salvage, household, logistics, or qualification claim is true.

## Identity grammar

V0.1 keeps these identities distinct:

- `ActorId` — economic principal;
- `AssetId` — persistent capital/source asset reference;
- `CommoditySpecId` — identity of the unit/specification being counted;
- `LotId` — one homogeneous stock lot;
- `LocationId` — economic location reference;
- `CausalId` — evidence/transaction/process/incident reference.

Identifiers are non-empty, bounded to 256 UTF-8 bytes, and cannot contain leading/trailing whitespace.

## Independent authority dimensions

Every live lot carries separately:

1. owner — who holds the economic claim;
2. custodian — who currently possesses/controls the stock;
3. location — where the stock is represented as present.

Changing one dimension does not silently change either of the others.

Ownership and custody transitions require the caller to bind the expected current authority. Relocation requires both the expected current location and the current custodian. Stale assertions fail closed.

V0.1 does not yet define legal title, liens, leases, beneficial ownership, delegated signing authority, access-control credentials, or jurisdiction-specific property law.

## Stock admission

A lot may enter the ledger only through `StockEvent::Establish` with one explicit origin:

- qualified initial stock;
- industrial production;
- industrial recycling;
- salvage.

Industrial origin records bind the industrial dependency ID and tick plus an evidence reference. This ledger treats those fields as provenance. Industrial qualification/bootstrap evidence remains outside this module.

## Stock depletion

Quantity may leave a lot only through `StockEvent::Deplete` with one explicit cause:

- industrial demand;
- transformation input;
- household consumption;
- loss;
- disposal.

Full depletion removes the lot from current state but never removes its event history.

## Lot splitting

`Split` is the only V0.1 operation that creates a new lot from existing economic stock.

For parent quantity `Q` and child quantity `q`:

```text
0 < q < Q
parent_after = Q - q
child_after  = q
parent_after + child_after = Q
```

The child inherits the exact commodity specification, owner, custodian, and location. A split cannot reclassify stock or transfer authority.

Partial sales/shipments therefore compose as:

```text
split -> transfer ownership/custody/location of the child lot
```

rather than using a hidden partial-transfer mutation.

## Transaction theorem

A transaction is a non-empty ordered vector of stock events.

All events are first applied to a candidate current-state projection. Any invalid event rejects the entire transaction. Only after every event succeeds is canonical append-only history constructed and independently replayed. The transaction commits only if replay exactly equals the candidate state.

Therefore failed transactions leave both materialized state and event history unchanged.

## Persistence theorem

Persisted economic authority is the complete ordered `StockLedgerEntry` sequence, not the materialized lot map alone.

Sequence numbers are canonical, one-based, and gap-free. `StockLedger::from_entries` reconstructs current state by replay. `StockLedger::validate` independently replays history and rejects any mismatch between history and materialized state.

V0.1 intentionally has no history rewrite, delete, reorder, or generic state-set API.

## Determinism and resource bounds

V0.1 uses deterministic ordered maps and checked integer arithmetic.

Hard bounds:

- at most 65,536 live lots;
- at most 262,144 stock events;
- IDs at most 256 UTF-8 bytes.

Overflow and resource-limit violations fail closed.

## Quantity semantics

`quantity` is an unsigned integer expressed in the unit defined by `CommoditySpecId`.

The ledger can sum only lots with the exact same commodity specification. It does not add unlike commodity units and does not convert units.

ECON-04 must later define specification, grade, batch, serialized-asset, quality, and unit semantics explicitly.

## Conservation claim boundary

V0.1 establishes:

- same-spec quantity conservation for `Split`;
- explicit provenance for every admission;
- explicit cause for every depletion;
- absence of an untyped generic quantity setter;
- replay equivalence between complete history and current state.

V0.1 does **not** establish conservation of mass, atoms, energy, exergy, money, or economic value across industrial transformations. Inputs and outputs may use different specifications and units. A future qualified bridge must bind industrial transformation evidence to physical material/energy semantics before such conservation can be claimed.

## Industrial boundary

The existing `industrial_ecology` module remains authoritative for its simulation theorem: configured industrial dependency inventory, demand, production/recycling capacity, prerequisite gating, shortages, and capability availability.

This economic module does not change those values and does not infer qualification from their existence. A later bridge may translate executed, qualified industrial consequences into economic stock events while retaining exact industrial tick/dependency lineage.

## Explicit non-claims

ECON-00/01 v0.1 does not establish:

- prices, bids, asks, markets, or equilibrium;
- money, accounts, double-entry bookkeeping, banking, credit, or monetary authority;
- contracts, escrow, settlement finality, or signatures;
- labor, wages, households, firms, taxation, or government;
- logistics routing, transport capacity, travel time, or delivery evidence;
- commodity quality/grade conversion;
- physical mass/energy conservation across transformations;
- simulation-LOD promotion/demotion conservation;
- legal validity of ownership claims;
- truth of external evidence references.

## Successor boundaries

- **ECON-02**: double-entry financial ledger and explicit currency/monetary authority.
- **ECON-03**: simulation-tier promotion/demotion reconciliation with no duplicate or lost stock/claims.
- **ECON-04**: commodity specifications, units, grades, batches, serialized capital assets, and quality/provenance semantics.
- **ECON-05+**: spatial logistics, markets, contracts, residents, firms, institutions, finance, externalities, technology, observability, and adversarial macro campaigns.

Each successor must consume this authority through explicit typed bridges rather than bypassing it with direct state mutation.
