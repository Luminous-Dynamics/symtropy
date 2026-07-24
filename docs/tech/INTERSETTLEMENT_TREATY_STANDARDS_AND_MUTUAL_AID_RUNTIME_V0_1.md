---
title: Intersettlement Treaty, Standards, and Mutual-Aid Runtime
version: 0.1
status: implementation-spec
scope: federation membership, treaty execution, standard negotiation, mutual aid, representation, contribution accounting, jurisdiction conflict, and federal emergency delegation
owner: simulation/engineering/design/networking
related:
  - canon/PLANETARY_FEDERATION_SUBSIDIARITY_AND_SHARED_SOVEREIGNTY_CONTRACT_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - tech/PUBLIC_ADMINISTRATION_SUCCESSION_AND_CORRUPTION_RUNTIME_V0_1.md
  - tech/ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Intersettlement Treaty, Standards, and Mutual-Aid Runtime

## Purpose

This document defines the minimum deterministic runtime needed to represent negotiated coordination among settlements, habitats, fleets, machine polities, and recognized nonhuman communities.

It does not simulate diplomacy through one reputation score. It stores the obligations, scopes, evidence, institutions, logistics, and failure conditions that make agreements materially real.

## Runtime Principle

```text
A treaty is not active because a menu says "allied."
It is active when named parties can prove what they promised,
maintain the institutions that carry the promise,
and detect when capability or consent has changed.
```

# 1. Authoritative Data Domains

The runtime separates:

```text
membership truth
representation truth
treaty text and signatures
operational obligation state
material contribution state
standards compatibility state
jurisdiction and appeals
emergency delegation
public interpretation
Chronicle history
```

No one table owns all of federation truth.

# 2. Core Entity Types

```rust
struct FederationId(ContentHash);
struct MemberPolityId(EntityId);
struct TreatyId(ContentHash);
struct ClauseId(ContentHash);
struct StandardId(ContentHash);
struct ObligationId(ContentHash);
struct DelegationId(ContentHash);
struct ForumId(EntityId);
struct ContributionAccountId(ContentHash);
```

A `MemberPolity` may represent:

```text
settlement
city
orbital habitat
nomadic convoy or fleet
machine polity
recognized nonhuman collective
regional institution
worldline-associated enclave
```

# 3. Federation Record

```rust
struct FederationRecord {
    federation_id: FederationId,
    charter_hash: ContentHash,
    worldline_id: WorldlineId,
    founding_tick: ChronicleTick,
    status: FederationStatus,
    member_ids: Vec<MemberPolityId>,
    chamber_ids: Vec<InstitutionId>,
    common_service_ids: Vec<ServiceId>,
    rights_floor_id: RightsFloorId,
    amendment_procedure_id: ProcedureId,
    secession_procedure_id: ProcedureId,
    audit_policy_id: AuditPolicyId,
}
```

Statuses:

```text
forming
active
degraded
emergency-coordination
constitutional-crisis
partially-suspended
dissolving
historical
```

# 4. Membership State

```rust
struct MembershipRecord {
    federation_id: FederationId,
    member_id: MemberPolityId,
    class: MembershipClass,
    recognized_population: PopulationEstimate,
    represented_systems: Vec<SystemStake>,
    entry_treaty_id: TreatyId,
    active_rights: RightsBitmap,
    active_obligations: Vec<ObligationId>,
    contribution_capacity: CapacityVector,
    compliance_state: ComplianceState,
    dispute_state: Option<DisputeId>,
    review_tick: ChronicleTick,
}
```

Membership changes are atomic civic transactions. A member cannot lose representation while its obligations remain active unless a named emergency clause permits temporary suspension and preserves appeal.

# 5. Treaty Model

Treaties are composed from typed clauses.

```rust
struct Treaty {
    treaty_id: TreatyId,
    title: LocalizedTextId,
    parties: Vec<PartyRef>,
    clauses: Vec<TreatyClause>,
    signatures: Vec<SignatureRef>,
    effective_condition: EffectiveCondition,
    expiry_condition: Option<ExpiryCondition>,
    review_schedule: ReviewSchedule,
    dispute_forum: ForumId,
    withdrawal_procedure: ProcedureId,
    verification_plan: VerificationPlan,
}
```

Clause types:

```text
recognition
mutual aid
resource access
transit
standards adoption
inspection
archive replication
rights guarantee
nonaggression
shared defense
pollution limit
species or habitat protection
contribution
sanction
emergency delegation
withdrawal and asset settlement
```

Every clause defines:

```text
who owes
who benefits
what triggers performance
what evidence proves performance
what exceptions apply
what failure means
what remedy exists
what survives withdrawal
```

# 6. Obligation Lifecycle

```text
proposed
accepted
not-yet-effective
active
partially-performed
performed
impeded
breached
contested
remedied
waived
expired
inherited by successor
```

An obligation may fail because of unwillingness, inability, physical destruction, authority loss, contradictory law, or force majeure. These causes must remain distinct.

```rust
struct ObligationState {
    obligation_id: ObligationId,
    clause_id: ClauseId,
    obligor: PartyRef,
    beneficiary: Vec<PartyRef>,
    due_window: TickRange,
    required_capability: CapabilityVector,
    reserved_assets: Vec<AssetReservation>,
    evidence_refs: Vec<EvidenceRef>,
    state: ObligationStatus,
    impediments: Vec<Impediment>,
    remedies: Vec<RemedyRef>,
}
```

# 7. Mutual-Aid Runtime

Mutual aid is not free resource teleportation.

Aid requests include:

```rust
struct AidRequest {
    request_id: ContentHash,
    requester: MemberPolityId,
    incident_id: IncidentId,
    requested_capabilities: CapabilityVector,
    urgency: UrgencyClass,
    receiving_capacity: CapacityVector,
    route_constraints: Vec<RouteConstraint>,
    rights_constraints: RightsBitmap,
    preferred_sources: Vec<MemberPolityId>,
    evidence_refs: Vec<EvidenceRef>,
}
```

Aid offers reserve actual crews, vehicles, materials, beds, energy, data, or shelter.

The scheduler must account for:

```text
travel time
route condition
crew rest
care and maintenance burden
local reserve floor
interoperability
recipient capacity
security risk
language and access needs
return obligations
```

The system may recommend allocation, but political authority approves extraordinary diversion unless a preauthorized emergency clause applies.

# 8. Contribution Accounting

Contributions are multi-dimensional.

```rust
struct ContributionLedger {
    account_id: ContributionAccountId,
    member_id: MemberPolityId,
    period: ChroniclePeriod,
    assessed: ContributionVector,
    delivered: ContributionVector,
    credited_external_benefits: ContributionVector,
    recognized_harm_cost: HarmVector,
    exemptions: Vec<ExemptionRef>,
    disputed_items: Vec<DisputeId>,
}
```

Vectors may include:

```text
energy
materials
currency or clearing credit
labor-hours
care-hours
transport capacity
archive hosting
scientific observation
habitat restoration
security duty
emergency reserve
```

A member cannot satisfy all obligations through money when the federation requires embodied capability.

# 9. Standards Runtime

Standards represent interoperability, safety, measurement, signaling, data, rescue, cargo, life support, identity, and machine interfaces.

```rust
struct StandardRecord {
    standard_id: StandardId,
    domain: StandardDomain,
    version: SemanticVersion,
    specification_hash: ContentHash,
    maintainers: Vec<InstitutionId>,
    conformance_tests: Vec<TestId>,
    mandatory_scope: Option<AuthorityScope>,
    equivalent_profiles: Vec<StandardId>,
    waiver_procedure: ProcedureId,
    sunset_review: ChronicleTick,
}
```

Conformance states:

```text
unknown
self-declared
tested
certified
conditionally-compatible
waived
nonconforming
unsafe
obsolete
```

Standards must support equivalent compliance. A local polity may meet a safety outcome through a different implementation if verification succeeds.

# 10. Compatibility Graph

Interoperability is represented through a graph rather than a universal technology tier.

Nodes:

```text
power interfaces
cargo containers
vehicle couplings
medical records
identity proofs
airlock procedures
emergency signals
water-quality units
translation protocols
archive formats
```

Edges store:

```text
direct compatibility
adapter required
translator required
manual procedure
unsafe
unknown
```

This graph directly affects travel, trade, rescue, construction, and diplomacy.

# 11. Representation Runtime

Representatives possess scoped mandates.

```rust
struct RepresentativeMandate {
    representative_id: AgentId,
    constituency: ConstituencyRef,
    chamber_id: InstitutionId,
    authorized_topics: TopicBitmap,
    instruction_mode: InstructionMode,
    delegation_chain: Vec<AuthorityRef>,
    start_tick: ChronicleTick,
    expiry_tick: ChronicleTick,
    recall_procedure: ProcedureId,
    disclosure_policy: DisclosurePolicyId,
}
```

A representative's speech is not automatically the member polity's binding action. Binding acts require a valid mandate and procedural commit.

Inactive, absent, dead, or source-chain-compromised representatives enter explicit continuity procedures rather than silently retaining power.

# 12. Jurisdiction Conflict Runtime

Conflicts are created when rules overlap or consequences cross boundaries.

```rust
struct JurisdictionCase {
    case_id: DisputeId,
    claimant_rules: Vec<RuleRef>,
    affected_parties: Vec<PartyRef>,
    affected_systems: Vec<SystemRef>,
    immediate_risk: RiskVector,
    rights_floor_questions: Vec<RightId>,
    externality_evidence: Vec<EvidenceRef>,
    provisional_orders: Vec<OrderId>,
    hearing_forum: ForumId,
    appeal_path: Vec<ForumId>,
}
```

The runtime must preserve local rules, federal rules, and the reasoning used to choose a provisional order. It must not overwrite losing law as if it never existed.

# 13. Emergency Delegation Runtime

```rust
struct EmergencyDelegation {
    delegation_id: DelegationId,
    incident_id: IncidentId,
    granting_parties: Vec<AuthorityRef>,
    receiving_institution: InstitutionId,
    permitted_actions: ActionBitmap,
    prohibited_actions: ActionBitmap,
    nonderogable_rights: RightsBitmap,
    evidence_threshold: EvidenceThreshold,
    start_tick: ChronicleTick,
    review_interval: ChronicleDuration,
    expiry: ExpiryCondition,
    independent_witnesses: Vec<WitnessRef>,
    return_transactions: Vec<AuthorityReturn>,
}
```

The system automatically schedules review and return checks. A missed review does not silently renew authority; it moves the delegation to `legitimacy-contested` and restricts expansion.

# 14. Federation Health State

Federation simulation tracks separate dimensions:

```text
operational capacity
member trust
rights-floor compliance
contribution fairness
administrative competence
standard compatibility
shared-service resilience
representation legitimacy
capture pressure
exit pressure
conflict intensity
```

These values generate pressure and opportunities but do not directly choose political outcomes.

# 15. Event Interface

Important events:

```text
FederationFounded
MemberJoined
MemberAssociated
MemberSuspended
MemberWithdrew
TreatyRatified
TreatyClauseActivated
ObligationPerformed
ObligationImpeded
TreatyBreached
AidRequested
AidDispatched
AidFailed
StandardAdopted
StandardWaived
JurisdictionCaseOpened
EmergencyAuthorityDelegated
EmergencyAuthorityReviewed
AuthorityReturned
FederalServiceDegraded
```

Events are partitioned:

- operational updates remain regional simulation truth;
- signed decisions become civic truth;
- founding, secession, major breach, or constitutional change becomes Chronicle truth;
- worldline-recognition consequences become worldline truth.

# 16. Simulation Levels of Detail

## Active Diplomatic Scene

Full participants, testimony, procedure, physical documents, communication delays, and player interaction.

## Active Federation Region

Detailed obligations, routes, institutions, representatives, and services.

## Background Planetary Layer

Aggregated contribution vectors, treaty states, pressure, service capability, and scheduled events.

## Dormant Worldline

Checkpoint plus causal journal; no invented detailed conversations.

LOD transitions preserve:

```text
signed obligations
material reservations
rights restrictions
open disputes
representation mandates
emergency expiry
Chronicle commitments
```

# 17. Failure and Recovery

The runtime must survive:

```text
network partition
settlement destruction
leadership death
identity compromise
archive loss
mod removal
schema migration
worldline fork
federation dissolution
```

Recovery never fabricates signatures or performed obligations. Unknown state becomes explicit uncertainty or disputed reconstruction.

# 18. Performance Budget

Representative regional proof target:

```text
5 member polities
20 active treaty clauses
30 active obligations
8 shared standards
3 mutual-aid services
2 open jurisdiction cases
1 emergency delegation
```

Planetary background target:

```text
50–200 member polities
thousands of dormant clauses
hundreds of active obligations
updates in scheduled batches
```

The system degrades by reducing forecast frequency and aggregating minor obligations, never by dropping active rights, expiry, aid, or breach state.

# 19. Acceptance Tests

- treaty activation requires valid signatures and effective conditions;
- aid consumes real reserved capability and traverses a route;
- inability and bad-faith refusal remain distinguishable;
- expired representation cannot commit binding action;
- emergency delegation automatically reaches review and return logic;
- equivalent technical compliance can satisfy a standard;
- network partition cannot duplicate contributions or aid assets;
- a worldline fork preserves treaty ancestry while allowing future divergence;
- a federation can dissolve without deleting member identity, property, evidence, or outstanding claims;
- player-facing causal traces explain every binding action.

## Final Line

```text
The runtime does not simulate unity.
It simulates the work required for difference to remain cooperative.
```
