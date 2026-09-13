# ECON-03 — Economic resolution conservation v0.1

## Status

ECON-03 defines the first Symtropy authority for changing economic simulation fidelity without silently changing the economy being represented.

It is a reconciliation kernel, not a world scheduler, regional simulator, persistence backend, or aggregation algorithm.

## Governing theorem

For one frozen economic instant:

```text
resolution transition
    != economic mutation
```

and every admitted transition preserves one exact `EconomicConservationManifest`:

```text
manifest(before) == manifest(after)
```

A transition may change representation fidelity and detail availability. It may not itself create, destroy, transfer, settle, consume, issue, retire, or otherwise mutate economic state.

Economic evolution must occur under the appropriate ECON authority before or after the resolution transition and then be captured as a new economic instant.

## Resolution tiers

V0.1 defines four representation labels ordered from finest to coarsest:

1. `ActiveSite`
2. `LocalRegion`
3. `DistantRegion`
4. `PlanetaryAggregate`

These labels do not prescribe world-simulation scheduling, tick rates, AI behavior, physics fidelity, rendering, or geography. They only provide an ordered vocabulary for economic reconciliation.

## Conserved stock theorem

Live stock is grouped only across lots that share the exact tuple:

```text
CommoditySpecId
+ owner ActorId
+ custodian ActorId
+ LocationId
```

The manifest records checked integer quantity for every such group.

Therefore ECON-03 does not permit aggregation to erase economically relevant ownership, custody, location, or commodity-spec distinctions.

V0.1 intentionally does not claim that distinct lot identities are reconstructible from the aggregate quantity alone.

## Complete financial-registry theorem

Transaction history is not accepted as proof of the complete financial registry.

A valid currency or account may have zero supply, zero debit/credit totals, and no transaction history while remaining economically relevant state.

Therefore callers must supply a canonical `FinancialRegistrySnapshot` containing currency and account definitions. ECON-03 reconstructs a complete `FinancialBook` from:

```text
supplied currency registry
+ supplied account registry
+ complete journal history
+ complete monetary history
```

and requires exact `FinancialBook` equality with the source book.

If a zero-balance account, zero-supply currency, owner, class, currency binding, monetary authority, or minor-unit definition is omitted or changed, capture fails closed.

## Financial state conserved

For every account:

```text
account identity
owner identity
currency identity
account class
cumulative debits
cumulative credits
```

For every currency:

```text
currency definition
monetary-authority identity
monetary-authority actor
minor-unit exponent
declared aggregate supply
```

must remain identical during the resolution transition.

## History identity manifest

V0.1 retains identity-bearing history information across resolution transitions:

- stock event sequence + causal identity;
- journal transaction identities;
- monetary event sequence + causal identity;
- ECON-02B settlement identities when the exact source is a `MonetarySettlementLedger`.

These identities make accidental substitution or double-accounting harder to misclassify as a harmless LOD transition.

They are not a cryptographic commitment to the complete byte representation of every historical event. Exact archival content remains an external retained-detail/evidence responsibility in v0.1.

## Detail states

A resolution snapshot is either:

```text
Exact { evidence_id }
```

or:

```text
Aggregated { retention: Option<DetailRetentionRef> }
```

`Exact` means the resolution authority was created or restored from exact ECON state that independently produced the manifest.

`Aggregated` means fine representation detail is not currently authoritative in the active snapshot.

## Demotion theorem

An exact representation may move only to a strictly coarser tier.

The conservation manifest is copied unchanged.

Demotion may either:

1. bind a `DetailRetentionRef` whose source snapshot is exactly the snapshot being demoted; or
2. explicitly discard reversible detail by storing no retention reference.

A retention reference is provenance identity only. ECON-03 v0.1 does not cryptographically verify the external archive/storage system named by that reference.

## Aggregate coarsening theorem

An already aggregated representation may move to a still coarser tier.

Its existing retention reference is carried forward unchanged. Coarsening cannot replace or rebind retained-detail provenance.

This prevents a sequence of LOD transitions from laundering one retained-detail source into another.

## Promotion theorem

Promotion is permitted only when all of the following hold:

```text
current representation is Aggregated
+ target tier is strictly finer
+ current snapshot has retained-detail provenance
+ supplied restoration reference == retained reference exactly
+ supplied exact ECON state validates independently
+ reconstructed manifest == current coarse manifest exactly
```

Only then does the resulting snapshot become `Exact`.

A promotion therefore cannot hide stock loss, financial changes, supply changes, ownership changes, location changes, or history changes behind the phrase “loading more detail.”

## No-false-precision theorem

If detail was demoted with:

```text
retention = None
```

then promotion through ECON-03 v0.1 fails with `DetailUnavailable`.

The kernel does not invent replacement lots, transaction histories, account facts, settlement identities, owner assignments, or locations merely because a finer simulation tier is requested.

An external higher authority may later establish a new qualified detailed state, but it cannot be described as lossless ECON-03 promotion from the discarded snapshot.

## Snapshot and transition identity

Each snapshot ID is unique within one resolution ledger.

Transition history is:

- append-only;
- one-based;
- gap-free;
- bound to the exact previous snapshot ID;
- generation-incrementing;
- replayable from the frozen base snapshot.

`validate()` replays all transitions and requires exact equality with the materialized current snapshot.

## Frozen economic instant

An `EconomicResolutionLedger` represents one economic instant.

Its conservation manifest cannot change through any supported transition.

The following are therefore not resolution operations:

- production or recycling;
- stock consumption, loss, disposal, salvage, or relocation;
- ownership/custody transfer;
- journal posting;
- currency issuance or retirement;
- ECON-02B settlement;
- contract performance;
- wages, taxes, prices, trades, debt, interest, or bankruptcy.

Those changes must occur under their own authorities and produce a new captured economic state.

## Atomicity

A failed demotion, coarsening, or promotion leaves the live resolution ledger unchanged.

Live transitions are appended only after the complete candidate transition history replays successfully from the frozen base snapshot.

## Explicit non-claims

ECON-03 v0.1 does not establish:

- a particular regional simulation algorithm;
- how agents, firms, markets, factories, vehicles, ecology, or infrastructure are aggregated;
- automatic reconstruction of discarded fine detail;
- cryptographic validity of `DetailRetentionRef` evidence;
- byte-identical archival commitment for all underlying ledgers;
- geographic partition correctness;
- distributed consensus between simulation hosts;
- persistence durability of retained detail;
- contract/obligation reconciliation not yet modeled by earlier ECON layers;
- performance or memory improvements from any specific aggregation strategy.

It establishes the authority boundary that later aggregation implementations must satisfy.

## Successor work

- **ECON-03B** — partition/merge reconciliation for multiple regional economic manifests, including border-crossing stock/claims and no double ownership across partitions.
- **ECON-04** — rigorous commodity units, grades, batches, serialized capital assets, quality and provenance semantics.
- **ECON-05** — spatial logistics economics and delivery state.
- Later settlement/authentication work may bind retained-detail and transition evidence to Xenia/Mycelix capabilities/signatures.

No successor should gain authority to bypass this conservation boundary with an untyped representation rewrite.
