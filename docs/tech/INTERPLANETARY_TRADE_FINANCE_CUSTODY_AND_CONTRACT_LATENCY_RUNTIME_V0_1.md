---
title: Interplanetary Trade, Finance, Custody, and Contract Latency Runtime
version: 0.1
status: implementation-spec
scope: delayed markets, physical delivery, forward contracts, credit, escrow, custody, clearing, currency, insurance, default, sanctions, fraud, and asynchronous dispute resolution
owner: engineering/economy/networking/simulation
implements:
  - ../canon/INTERPLANETARY_CIVILIZATION_LATENCY_AND_DISTRIBUTED_SOVEREIGNTY_CONTRACT_V0_1.md
  - ../canon/ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md
authority_boundary: owns interplanetary economic transaction state and delayed settlement; physical carrier and shipment state remains authoritative in logistics runtime, civic law remains owned by charters and treaties
related:
  - ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - INTERREGIONAL_TRADE_CUSTOMS_CURRENCY_AND_SANCTIONS_RUNTIME_V0_1.md
  - DEEP_SPACE_LOGISTICS_TRANSFER_WINDOWS_RESCUE_AND_SALVAGE_RUNTIME_V0_1.md
  - LIGHT_DELAY_COMMUNICATION_TIMEKEEPING_AND_ASYNC_COORDINATION_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Interplanetary Trade, Finance, Custody, and Contract Latency Runtime

## Purpose

This specification extends Symtropy's economy across worlds that cannot share current prices, inventories, courts, or delivery conditions.

Its core problem is not merely distance cost.

It is that every participant acts on delayed information while the underlying goods remain physical.

## Core Invariant

> **A financial claim may reserve, promise, insure, or value an asset. It may never duplicate the asset or replace physical delivery.**

# 1. Economic Time Layers

Every interplanetary transaction distinguishes:

```text
offer creation time
offer receipt time
acceptance time
acceptance receipt time
contract activation time
cargo reservation time
loading time
departure time
arrival time
inspection time
settlement time
dispute filing and receipt time
```

A price observed on Mars may describe Earth months ago. The interface must make age and source visible.

# 2. Market Views

A market view is local knowledge, not global truth.

```rust
struct MarketView {
    observer: PrincipalId,
    market: MarketId,
    knowledge_cutoff: SystemEpoch,
    received_reports: Vec<MarketReportId>,
    estimated_inventory: EstimateRange,
    estimated_demand: EstimateRange,
    confidence: f32,
    missing_sources: Vec<SourceId>,
}
```

Different worlds may post rationally different prices for the same expected cargo.

# 3. Offer and Acceptance

```rust
struct DelayedOffer {
    offer_id: OfferId,
    issuer: PrincipalId,
    asset_or_service: EconomicSubject,
    quantity: FixedPoint,
    price_terms: PriceTerms,
    delivery_terms: DeliveryTerms,
    known_conditions: ConditionSnapshot,
    valid_until: SystemEpoch,
    acceptance_rule: AcceptanceRule,
    collateral: Option<CollateralRef>,
    authority: AuthorityEnvelope,
}
```

Acceptance rules may be:

```text
first valid acceptance received
issuer confirmation required
bounded quantity allocation
auction close at shared epoch
bilateral negotiation
conditional on route or insurance
```

The runtime must resolve crossing messages deterministically.

# 4. Contract State

```rust
struct InterplanetaryContract {
    contract_id: ContractId,
    parties: Vec<PrincipalId>,
    subject: EconomicSubject,
    quantity: FixedPoint,
    price: PriceTerms,
    delivery: DeliveryTerms,
    custody_requirements: Vec<CustodyRequirement>,
    inspection: InspectionTerms,
    risk_transfer: RiskTransferRule,
    payment_schedule: PaymentSchedule,
    dispute_forum: ForumId,
    force_majeure: Vec<ConditionPredicate>,
    state: ContractState,
}
```

States:

```text
Draft
Offered
AcceptedPendingConfirmation
Active
CargoReserved
Loaded
InTransit
ArrivedPendingInspection
PartiallyPerformed
Performed
Defaulted
Disputed
Restructured
Cancelled
Expired
```

# 5. Forward Contracts and Futures

Forward contracts may coordinate scarce future production and transfer windows.

They require:

```text
specific delivery window
quantity and quality
producer capacity evidence
route feasibility
collateral or mutual guarantee
margin or reserve policy where used
position limits
anti-manipulation audit
```

No purely financial instrument may create more deliverable physical quantity than the declared underlying market and enforceable netting permit.

## Position and Leverage Limits

The runtime tracks:

```text
gross claims
net claims
collateral
physical production capacity
route capacity
concentration
```

Excess leverage can produce default and political pressure, but not material duplication.

# 6. Escrow and Milestones

Escrow may hold:

```text
currency
material batch custody token
unique asset title
bond
insurance reserve
performance guarantee
```

Release milestones may include:

```text
verified loading
departure
midcourse telemetry
arrival
inspection
final custody transfer
```

A custody token cannot move independently of authoritative asset state unless it explicitly represents a disputed claim rather than ownership.

# 7. Payment Systems

Possible systems:

```text
local currencies
mutual credit
clearing-union balances
commodity-linked claims
energy or transport credits
public ration entitlements
interplanetary settlement unit
barter and reciprocal obligations
```

Every currency defines:

```text
issuer
acceptance domain
redemption or tax basis
supply rule
ledger authority
failure and fork behavior
```

No currency is universally accepted by design.

# 8. Clearing Under Delay

A clearing union reduces the need to transport settlement assets for every trade.

```rust
struct ClearingPosition {
    member: PrincipalId,
    asset: SettlementAssetId,
    balance: FixedPoint,
    credit_limit: FixedPoint,
    collateral: Vec<CollateralRef>,
    last_confirmed_epoch: SystemEpoch,
    disputed_delta: FixedPoint,
}
```

Clearing rounds preserve:

```text
signed submissions
knowledge cutoffs
netting rules
credit limits
late-message handling
fork and rollback boundaries
```

A late submission cannot silently rewrite a completed round. It enters correction or dispute.

# 9. Credit and Debt

Credit assessment may use:

```text
production capacity
contract history
custody integrity
public guarantees
route access
reserves
political and disaster risk
```

It must not use prohibited private cognition, protected health, religion, ethnicity, or opaque player profiling.

Debt cannot lawfully collateralize minimum life support, personhood, reproductive rights, or identity continuity.

# 10. Insurance and Mutual Aid

Coverage may address:

```text
cargo loss
late arrival
route closure
rescue cost
quarantine delay
habitat failure
political seizure
```

Claims require evidence from logistics, custody, and communication state.

Insurance cannot pay as total loss while an intact hidden duplicate remains in another branch or unresolved custody state.

Mutual-aid pools may respond without profit pricing but still require contribution, reserves, and governance.

# 11. Default and Restructuring

Default causes include:

```text
missed window
production shortfall
carrier failure
cargo loss
quarantine
war or blockade
currency failure
fraud
message delay
changed law
```

Responses may include:

```text
grace period
partial delivery
replacement cargo
route substitution
price adjustment
debt extension
public guarantee
restructuring
liquidation of nonessential collateral
justice or fraud investigation
```

The runtime distinguishes inability, negligence, opportunism, and fraud.

# 12. Customs, Sanctions, and Political Risk

Contracts reference current and anticipated:

```text
customs rules
quarantine
export controls
sanctions
humanitarian exceptions
seizure risk
```

A legal change becomes effective according to real publication and receipt rules. A ship cannot be punished for violating an order that could not have reached it before departure unless the contract precommitted to that rule.

# 13. Fraud and Manipulation

Threats include:

```text
false inventory
phantom cargo
duplicate title
forged inspection
insider knowledge from privileged communications
relay censorship
wash trading
cornering route capacity
insurance fraud
intentional distress or salvage fraud
```

Detection uses provenance and causal evidence, not an omniscient fraud score.

# 14. Dispute Resolution

Disputes may concern:

```text
quality
quantity
custody
arrival time
force majeure
inspection
payment
law
currency conversion
salvage
```

Forums may use asynchronous filings, local provisional remedies, evidence preservation, and later appeal.

A local court may secure cargo without deciding final interplanetary ownership.

# 15. NPC and Institutional Behavior

Traders and institutions act on local market views and beliefs. They may:

```text
hedge
stockpile
trust a long partner
overreact to delayed news
spread or correct rumor
seek political protection
refuse exploitative terms
```

NPCs never know distant inventory unless a report reached them.

# 16. LOD and Persistence

Background simulation preserves:

```text
contract identity
state
party obligations
asset and custody references
message frontiers
collateral
disputes
expiry
```

Fine order-book activity may aggregate, but unique durable contracts remain traceable.

Worldline forks must:

```text
prevent double settlement of transferable assets
preserve pre-fork claims
mark post-fork incompatibility
retain dispute ancestry
```

# 17. Representative Fixture

Fixture:

```text
Earth-orbit market
Mars settlement
lunar clearing node
water and medical cargo
industrial machine part
one cargo carrier
one insurer or mutual-aid pool
one customs change
one relay outage
```

Scenario:

1. Mars offers a forward contract based on stale Earth inventory.
2. Earth accepts, but a second acceptance crosses in transit.
3. Cargo is reserved and loaded.
4. A customs restriction changes after departure.
5. Relay outage delays market and legal information.
6. Carrier diverts for rescue and arrives late with partial cargo.
7. Parties restructure, inspect, and settle a disputed claim.

Acceptance requires:

- crossing acceptance resolves deterministically;
- no physical cargo duplication;
- market views show age and uncertainty;
- risk transfer follows contract milestones;
- changed law respects publication and receipt timing;
- rescue diversion has economic but not automatic criminal consequences;
- insurance and custody reconcile;
- save/load and fork preserve claims and prevent double payment.

# 18. Anti-Exploit Rules

Reject:

```text
instant system-wide price discovery
financial claim spawning physical goods
zero-risk leverage
late message rewriting settled round
insurance payout plus duplicated asset
currency with undefined issuer or redemption
life support used as ordinary collateral
private cognition used for credit scoring
```

## Final Rule

> **Interplanetary finance may help people coordinate across uncertainty. It may never pretend uncertainty, time, labor, and matter have disappeared.**
