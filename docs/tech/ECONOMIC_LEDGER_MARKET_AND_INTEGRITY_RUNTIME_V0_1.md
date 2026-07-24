---
title: Economic Ledger, Market, and Integrity Runtime
version: 0.1
status: implementation-spec
scope: asset identity, transaction processing, custody, markets, contracts, price signals, economic audit, and anti-duplication
owner: engineering/simulation/networking/security
related:
  - canon/ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md
  - canon/PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - tech/WORLD_PERSISTENCE_PROTOCOL.md
  - SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
---

# Economic Ledger, Market, and Integrity Runtime

## Purpose

This specification defines the authoritative transaction and market runtime for scarce assets, currencies, contracts, services, and aggregated regional flows.

It does not require every physical item to become a blockchain record. It requires every scarce transfer to have one authoritative custody transition and a replay-safe cause.

# 1. Economic Truth Layers

```text
Physical truth      — where an embodied item is and what condition it has
Custody truth       — which rights bundle currently controls transfer or use
Market truth        — offers, bids, contracts, and settlements
Civic truth         — taxes, public claims, licenses, sanctions, and disputes
Chronicle truth     — only major economic precedents and transformations
```

# 2. Asset Identity

Use unique identities for non-fungible or provenance-sensitive assets and batch identities for fungible matter.

```rust
enum AssetIdentity {
    Unique(AssetId),
    Batch { batch_id: BatchId, quantity: FixedQuantity },
    Claim(ClaimId),
    Information { content_hash: ContentHash, license: LicenseId },
}
```

Batch split and merge transactions preserve total quantity and ancestry.

# 3. Transaction Envelope

```rust
struct EconomicTransaction {
    transaction_id: TransactionId,
    actor: AgentId,
    authority_domain: AuthorityDomainId,
    expected_state_version: u64,
    inputs: Vec<AssetRef>,
    outputs: Vec<AssetMutation>,
    rights_changes: Vec<RightsMutation>,
    consideration: Vec<Consideration>,
    signatures_or_tokens: Vec<AuthorizationRef>,
    idempotency_key: IdempotencyKey,
}
```

Processing result:

```text
committed
rejected
conflict requiring refresh
quarantined for review
```

A retried transaction with the same idempotency key returns the prior result.

# 4. Atomic Custody Transfer

Transfer must atomically:

```text
verify current custody
verify quantity and condition
verify encumbrances and access policy
reserve inputs
commit rights and location changes
emit one settlement event
release or consume inputs
```

No item exists simultaneously in seller inventory, cargo container, and buyer inventory.

# 5. Physicalization Boundary

Local gameplay owns embodied movement. The economic runtime owns custody.

Example cargo handoff:

```text
1. Seller commits cargo to escrow container.
2. Physical system moves sealed container.
3. Buyer inspects or accepts condition.
4. Settlement transaction exchanges custody and payment.
5. Container unlocks under buyer rights.
```

If the vehicle is destroyed, the physical asset state changes but custody claims and insurance may remain disputed.

# 6. Market Orders

```rust
struct MarketOrder {
    order_id: OrderId,
    market: MarketId,
    actor: ActorId,
    side: Side,
    asset_spec: AssetSpecification,
    quantity: FixedQuantity,
    limit: ConsiderationLimit,
    expiry: ChronicleTick,
    escrow: EscrowRef,
    visibility: OrderVisibility,
}
```

Orders require escrow or credible capacity. Fake unlimited bids and offers are rejected.

Market types may use:

```text
posted price
continuous matching
batch auction
request for proposal
sealed bid
negotiated contract
```

# 7. Contracts

```rust
struct EconomicContract {
    parties: Vec<ActorId>,
    deliverables: Vec<Deliverable>,
    acceptance_tests: Vec<Condition>,
    deadlines: Vec<ChronicleTick>,
    compensation: Vec<Consideration>,
    escrow: Vec<AssetRef>,
    breach_process: BreachProcess,
    force_majeure: Vec<Condition>,
    exit_policy: ExitPolicy,
}
```

Contracts can subscribe to device, cargo, construction, labor, or Chronicle events. A contract does not complete because a dialogue model says it did.

# 8. Currency Ledger

Currencies use integer or fixed-point units.

Required operations:

```text
issue
transfer
reserve
release
redeem
retire
freeze under authorized dispute
migrate
```

Every issuance references an issuer policy version. Currency balances are derived from transactions or stored with verifiable checkpoints.

# 9. Regional Flow Aggregation

Strategic simulation may aggregate bulk resources.

```rust
struct RegionalFlow {
    commodity: CommodityClass,
    source: NodeId,
    sink: NodeId,
    quantity_per_tick: FixedQuantity,
    route: RouteId,
    loss_rate: FixedRate,
    contract_or_policy: FlowAuthority,
}
```

When a player intercepts or escorts a specific shipment, the system materializes a bounded cargo lot and reconciles its outcome back into the aggregate flow.

# 10. Price Signals

The runtime may compute suggested or historical price bands using:

```text
recent completed trades
local inventory coverage
open order depth
route cost
loss and spoilage
risk premium
public subsidy or tax
```

Price suggestions are advisory. NPC and player actors may value goods differently.

# 11. Integrity Controls

## Idempotency

Every mutation is safe under retry.

## Optimistic Concurrency

Transactions include expected state versions. Conflicts refresh rather than overwrite.

## Quarantine

Unknown version, invalid ancestry, duplicate custody, or impossible quantity sends assets to a non-usable quarantine state pending recovery.

## Reconciliation

Periodic checks verify:

```text
asset quantity conservation
single custody
currency issuance and retirement balance
escrow completeness
market order backing
regional flow reconciliation
```

## Rollback Rule

Gameplay rollback may reverse local prediction. Durable economic commits require compensating transactions or restoration to a checkpoint that also restores every dependent authority domain.

# 12. Fraud and Intended Deception

The runtime distinguishes:

```text
technical invalidity — rejected or quarantined
in-world counterfeit — valid object with deceptive claims
disputed quality — valid transfer subject to contract evidence
platform abuse — handled by moderation and security policy
```

In-world deception remains playable because it uses valid state and leaves evidence.

# 13. Offline and Partition Handling

During partition:

```text
local non-transfer interactions may continue
new cross-authority transfers pause or use bounded escrow
markets display staleness
currency issuance pauses unless local authority permits it
reconnection reconciles by transaction identity and ancestry
```

Never merge two independent spends of the same custody claim.

# 14. Privacy

Public markets may expose orders and completed prices. Private contracts expose only required parties and auditors. Exact personal inventories and wealth are not globally visible by default.

# 15. Audit Trace

For every committed transaction record:

```text
transaction ID
authority domain
policy and schema versions
input and output identities
rights changes
authorization references
causal game event
result hash
```

Player-facing views summarize this through receipts, seals, provenance, and dispute evidence.

# 16. Tests

Required property and integration tests:

```text
split/merge conserves quantity
retry is idempotent
concurrent sale commits once
save/load preserves custody
rollback cannot duplicate assets
worldline export/import preserves ancestry
expired order cannot settle
escrow cannot be spent twice
regional materialization reconciles exactly
currency issuance minus retirement equals supply
```

# 17. Seedworks Minimum

Implement:

```text
unique tools and vehicles
batch scrap, food, medicine, and fuel
one local currency or credit claim
posted-price and contract-board exchange
escrowed cargo contract
public workshop subsidy
reconciliation command and audit report
```

# 18. Acceptance Gates

- all scarce transfers use idempotent authoritative transactions;
- a disconnect during trade cannot duplicate or destroy custody silently;
- physical cargo and economic ownership remain consistent;
- markets cannot post unbacked infinite orders;
- aggregate flows reconcile with materialized missions;
- invalid migrated assets are quarantined rather than accepted;
- audit traces identify the cause of each balance or custody change.
