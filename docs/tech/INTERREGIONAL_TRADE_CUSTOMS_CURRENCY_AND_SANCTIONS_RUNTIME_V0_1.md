---
title: Interregional Trade, Customs, Currency, and Sanctions Runtime
version: 0.1
status: implementation-spec
scope: interregional exchange, customs, cargo provenance, clearing systems, exchange rates, trade corridors, sanctions, embargoes, smuggling, and economic diplomacy
owner: simulation/engineering/economy/design
related:
  - canon/ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md
  - canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md
  - tech/ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - tech/PLANETARY_INFRASTRUCTURE_NETWORKS_AND_CORRIDOR_RUNTIME_V0_1.md
  - tech/INTERSETTLEMENT_TREATY_STANDARDS_AND_MUTUAL_AID_RUNTIME_V0_1.md
---

# Interregional Trade, Customs, Currency, and Sanctions Runtime

## Purpose

This document extends Symtropy's local economic integrity model into interregional and planetary exchange.

It defines how goods, services, energy, knowledge, labor obligations, currencies, standards, and political restrictions move between polities while preserving physical conservation, provenance, rights, and conflict.

## Core Thesis

```text
Trade is not a global market screen.
It is matter crossing routes under law, trust, standards, custody, and risk.
```

A trade relationship is not proved by positive reputation. It is proved by repeated settlement of obligations, delivered cargo, functioning corridors, accepted standards, and institutions capable of resolving failure.

# 1. Economic Domains

The runtime separates:

```text
physical assets
custody
ownership and use rights
market offers
contracts
customs status
currency and clearing
public contribution
sanctions and legal restrictions
Chronicle and treaty evidence
```

No currency balance creates a physical item.

# 2. Trade Entities

```rust
struct TradeRouteId(ContentHash);
struct CustomsZoneId(EntityId);
struct TradeContractId(ContentHash);
struct CurrencyId(ContentHash);
struct ClearingUnionId(ContentHash);
struct SanctionRegimeId(ContentHash);
struct CargoManifestId(ContentHash);
```

# 3. Cargo Manifest

```rust
struct CargoManifest {
    manifest_id: CargoManifestId,
    cargo_refs: Vec<AssetOrBatchRef>,
    declared_quantity: QuantityVector,
    origin: NetworkNodeId,
    destination: NetworkNodeId,
    consignor: ActorId,
    consignee: ActorId,
    custodian_chain: Vec<CustodyTransferRef>,
    provenance_refs: Vec<EvidenceRef>,
    hazard_profile: HazardProfile,
    biosecurity_profile: BiosecurityProfile,
    standards_claims: Vec<ConformanceClaim>,
    declared_value: Vec<ValueClaim>,
    contract_id: Option<TradeContractId>,
}
```

The declared manifest and physical cargo can disagree. Inspections, sensor uncertainty, tampering, spoilage, theft, substitution, and honest clerical error are distinct states.

# 4. Trade Contract

```rust
struct TradeContract {
    contract_id: TradeContractId,
    parties: Vec<PartyRef>,
    deliverables: Vec<Deliverable>,
    payment_terms: Vec<PaymentTerm>,
    delivery_window: TickRange,
    route_constraints: Vec<RouteConstraint>,
    quality_terms: Vec<QualityTerm>,
    force_majeure: Vec<ConditionRef>,
    inspection_rights: Vec<InspectionRight>,
    dispute_forum: ForumId,
    default_remedies: Vec<RemedyRef>,
    transferability: TransferPolicy,
}
```

Contracts may exchange:

```text
materials
energy
transport capacity
maintenance service
care service
software or blueprints
scientific observations
access rights
future delivery
mutual obligations
```

The runtime must not treat people as cargo or transferable labor assets. Employment and service obligations remain consent-bound and exit-capable under the labor contract.

# 5. Offers and Markets

Markets may be:

```text
local spot market
bilateral contract exchange
auction
rationed public exchange
guild or cooperative allocation
federation clearing market
emergency requisition market
barter network
```

An offer declares:

```rust
struct MarketOffer {
    offer_id: ContentHash,
    actor_id: ActorId,
    asset_or_service: OfferSubject,
    quantity: QuantityVector,
    price_terms: Vec<PriceTerm>,
    location: NetworkNodeId,
    delivery_terms: DeliveryTerms,
    quality_evidence: Vec<EvidenceRef>,
    expiry: ChronicleTick,
    visibility: OfferVisibility,
}
```

Matching creates a contract, not instantaneous transfer.

# 6. Currency and Clearing

Currencies may represent:

```text
settlement credit
commodity-backed claim
energy credit
labor or care accounting
federation clearing unit
commercial token
worldline-recognized claim
```

The game must distinguish:

```text
unit of account
medium of exchange
store of value
contribution record
ration entitlement
debt claim
```

One instrument may serve several functions but they remain mechanically explicit.

```rust
struct CurrencyProfile {
    currency_id: CurrencyId,
    issuer: InstitutionId,
    issuance_rule: IssuanceRule,
    redemption_rule: RedemptionRule,
    reserve_refs: Vec<ReserveRef>,
    transfer_scope: TransferScope,
    privacy_policy: PrivacyPolicyId,
    failure_mode: CurrencyFailureMode,
}
```

# 7. Exchange Rates

Exchange rates may be set through:

```text
market exchange
clearing-union rule
fixed treaty band
commodity redemption
administered emergency rate
informal street exchange
```

Rates reflect liquidity, trust, redemption capacity, trade balance, sanctions, route risk, and political decisions.

The runtime avoids high-frequency speculative simulation. Rates update at bounded intervals and are designed to produce strategic consequences rather than finance-game noise.

# 8. Clearing Union

A federation may settle net obligations without moving currency for every trade.

```rust
struct ClearingPosition {
    union_id: ClearingUnionId,
    member_id: MemberPolityId,
    period: ChroniclePeriod,
    credits: ValueVector,
    debits: ValueVector,
    collateral_refs: Vec<AssetRef>,
    settlement_due: ChronicleTick,
    dispute_refs: Vec<DisputeId>,
}
```

Clearing reduces transaction burden but creates systemic dependency and governance risk.

Failure modes:

```text
member default
false reporting
capture by creditor regions
liquidity freeze
reserve mismatch
political exclusion
```

# 9. Customs Zones

Customs governs entry for reasons including:

```text
biosecurity
hazard safety
standards
revenue
sanctions
species protection
cultural or sacred restrictions
anti-slavery and labor protection
archive or antiquities protection
```

A customs decision must identify its basis.

```rust
struct CustomsDecision {
    decision_id: ContentHash,
    manifest_id: CargoManifestId,
    zone_id: CustomsZoneId,
    decision: CustomsResult,
    reasons: Vec<ReasonRef>,
    inspection_evidence: Vec<EvidenceRef>,
    required_actions: Vec<ActionRef>,
    fees_or_bonds: Vec<ValueTransfer>,
    appeal_forum: Option<ForumId>,
}
```

Results:

```text
cleared
cleared-conditionally
inspection-required
quarantine
reexport-required
seized-pending-review
prohibited
unknown-classification
```

# 10. Inspection

Inspections consume time, labor, equipment, trust, and physical handling.

Methods:

```text
document audit
sensor scan
sample test
container opening
machine testimony
origin verification
witness chain review
nonhuman consent protocol
```

Inspections can damage sensitive goods or violate privacy. Scope and handling rules matter.

# 11. Tariffs, Fees, and Contributions

Charges may reflect:

```text
infrastructure use
inspection cost
environmental impact
public revenue
protective industrial policy
retaliation
emergency scarcity
```

The runtime records the declared purpose. A fee imposed for safety but diverted into unrelated patronage becomes an administrative integrity issue.

# 12. Sanctions and Embargoes

Sanctions are targeted restrictions intended to change behavior without immediate force.

Possible targets:

```text
specific actors
institutions
asset classes
technologies
routes
financial clearing
military goods
luxury goods
all exchange
```

Every regime defines:

```text
legal basis
objective
target
humanitarian exceptions
review interval
success criteria
evasion risk
unintended-harm monitoring
exit conditions
```

Broad sanctions can produce civilian harm, black markets, elite consolidation, and political backlash. These consequences are simulated and remembered.

# 13. Smuggling and Informal Trade

Smuggling emerges when valuable movement conflicts with law, access, scarcity, or survival.

Smuggling may involve:

```text
life-saving medicine
people escaping coercion
weapons
stolen artifacts
banned species
proprietary body-maintenance compounds
uncensored records
sanctioned machine parts
```

The game must not assign one moral value to all smuggling.

The runtime tracks:

```text
route secrecy
network trust
inspection risk
cargo harm
corruption
political protection
community legitimacy
```

# 14. Trade Dependency

Regions track strategic dependency by category:

```text
food
energy
water
medicine
machine parts
computation
transport
care labor
archive hosting
```

Dependency can create cooperation, vulnerability, coercion, or specialization.

A dependency warning should explain:

```text
source concentration
route concentration
reserve duration
substitution difficulty
political risk
maintenance risk
```

# 15. Economic Diplomacy

Trade agreements may create:

```text
market access
standards recognition
customs union
clearing union
shared reserve
technology transfer
labor mobility
procurement access
investment rights
```

These rights and obligations remain separate. A customs union does not automatically imply military alliance or open migration.

# 16. Fraud and Exploit Prevention

The runtime must prevent:

```text
cargo duplication across shards
manifest replay
currency double spend
contract completion without delivery
ownership transfer without authority
sanction bypass through renamed assets
customs state loss during save migration
exchange-rate race duplication
```

It also detects suspicious patterns without treating anomaly as guilt.

# 17. Simulation LOD

## Active Trade Scene

Physical cargo, characters, inspection, negotiation, loading, damage, and theft.

## Active Route

Vehicles or flow packets, custody, risks, checkpoints, and schedule.

## Regional Background

Aggregated contracts, market inventories, route capacity, clearing, and customs queues.

## Planetary Background

Strategic flows, dependencies, rates, sanctions, and scheduled settlement.

LOD preserves physical conservation, custody, open disputes, quarantine, debt, and contractual deadlines.

# 18. Player Experience

Players may:

```text
negotiate a supply contract
escort a convoy
inspect suspicious cargo
build an adapter that opens a market
challenge discriminatory customs policy
smuggle medicine through an unjust blockade
expose a false-origin scheme
stabilize a clearing union
choose sanctions or negotiated monitoring
recover stolen cultural artifacts
```

# 19. Representative Proof

The regional proof includes:

```text
three polities
one traded material
one perishable or care-critical cargo
one standards mismatch
one customs inspection
one payment or clearing method
one default or delay
one informal trade route
```

# 20. Acceptance Tests

- matched offers create enforceable contracts rather than teleportation;
- cargo cannot arrive without a conserved movement or valid abstract flow packet;
- customs decisions preserve evidence and appeal;
- a perishable cargo changes condition during delay;
- currency issuance and redemption follow declared rules;
- sanctions include humanitarian exceptions and produce tracked side effects;
- smuggling has causal networks and risks;
- trade dependency affects diplomacy and crisis response;
- worldline forks preserve contract and currency ancestry without duplicating assets;
- players can trace price, shortage, delay, and default to physical and institutional causes.

## Final Line

```text
A market connects strangers only when routes, promises, and institutions survive the distance between them.
```
