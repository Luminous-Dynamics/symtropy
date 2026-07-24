---
title: Settlement Founding, Charter, Institution, and Public Service Runtime
version: 0.1
status: implementation-spec
scope: settlement state, founding phases, charters, offices, public services, legitimacy, succession, multiplayer authority
owner: gameplay/simulation/networking/AI
related:
  - ../canon/PLAYER_FOUNDED_CIVILIZATION_SETTLEMENT_LEGACY_AND_WORLDLINE_CONTRACT_V0_1.md
  - ../canon/CIVIC_SUCCESSION_PUBLIC_SERVICE_AND_INSTITUTIONAL_CONTINUITY_CONTRACT_V0_1.md
  - ../canon/SETTLEMENT_AUTONOMY_COLONIZATION_ETHICS_AND_NONEXTRACTIVE_EXPANSION_CONTRACT_V0_1.md
  - ../ops/PLAYER_FOUNDED_SETTLEMENT_CAMPAIGN_PACKET_STANDARD_V0_1.md
  - PLAYER_PROMISE_OFFICE_REPUTATION_AND_LEGACY_RUNTIME_V0_1.md
---

# Settlement Founding, Charter, Institution, and Public Service Runtime

## Purpose

This runtime turns settlement founding into authoritative world state rather than a collection of decorative build flags.

It must support:

- prior claims;
- collective participation;
- provisional authority;
- amendable charters;
- offices and succession;
- public-service obligations;
- budgets and labor;
- legitimacy by constituency;
- multiplayer authority;
- deterministic replay;
- long absence.

# 1. Core Entities

```rust
struct SettlementState {
    settlement_id: SettlementId,
    region_id: RegionId,
    legal_names: Vec<NameRecord>,
    local_names: Vec<NameRecord>,
    phase: FoundingPhase,
    residents: ResidentRegistry,
    households: Vec<HouseholdId>,
    prior_claims: Vec<ClaimId>,
    charter_id: Option<CharterId>,
    institutions: Vec<InstitutionId>,
    public_services: Vec<ServiceId>,
    protected_zones: Vec<ZoneId>,
    external_relations: Vec<RelationId>,
    legitimacy: LegitimacyMap,
    chronicle_head: ChronicleEventId,
    worldline_id: WorldlineId,
}
```

```rust
enum FoundingPhase {
    Presence,
    ProvisionalCamp,
    FoundingCohort,
    ProvisionalCompact,
    ServiceFormation,
    CharterConvention,
    CharteredSettlement,
    MatureSuccession,
    Fragmented,
    Evacuated,
    Dissolved,
}
```

Phase progression is validated from state, not unlocked by a player level.

# 2. Claims

```rust
struct ClaimRecord {
    claim_id: ClaimId,
    claimant: ClaimantId,
    claim_type: ClaimType,
    geometry: Option<SpatialBoundary>,
    resource_scope: ResourceScope,
    temporal_scope: TemporalScope,
    evidence: Vec<EvidenceRef>,
    confidence: f32,
    legal_status: ClaimLegalStatus,
    contested_by: Vec<ClaimId>,
    privacy: PrivacyClass,
}
```

Claim types include:

```text
residency
seasonal use
household ownership
commons use
sacred or memorial use
watershed dependency
migration corridor
machine stewardship
nonhuman habitat
labor occupancy
corporate title
treaty right
refuge duty
abandonment under coercion
public-service easement
```

The runtime must not collapse conflicting claims into one hidden ownership winner before adjudication.

# 3. Participation

```rust
struct FoundingParticipant {
    agent_id: AgentId,
    constituency_ids: Vec<ConstituencyId>,
    standing: ParticipationStanding,
    delegated_authority: Vec<DelegationId>,
    contribution_record: Vec<ContributionId>,
    care_dependencies: Vec<DependencyId>,
    participation_capacity: CapacityWindow,
    dissent_records: Vec<DissentId>,
}
```

Participation standing may derive from residence, labor, dependency, prior claim, representation, office, treaty, or stewardship.

Contribution does not automatically buy votes.

The system must represent people unable to attend because of work, care, illness, distance, language, disability, detention, or network failure.

# 4. Provisional Compacts

```rust
struct ProvisionalCompact {
    compact_id: CompactId,
    adopted_tick: ChronicleTick,
    expires_at: Option<ChronicleTick>,
    membership_rules: Vec<RuleId>,
    authority_rules: Vec<RuleId>,
    emergency_powers: Vec<PowerGrant>,
    rights_floor: RightsFloor,
    contribution_rules: Vec<ContributionRule>,
    care_obligations: Vec<CareRule>,
    dispute_process: ProcessId,
    exit_rules: Vec<ExitRule>,
    review_process: ProcessId,
    ratification: RatificationRecord,
}
```

No provisional compact may be created without an explicit review path.

Emergency extensions create Chronicle events and increase legitimacy risk when review is delayed.

# 5. Charter Model

```rust
struct Charter {
    charter_id: CharterId,
    settlement_id: SettlementId,
    preamble: TextArtifactId,
    clauses: Vec<CharterClause>,
    rights_floor: RightsFloor,
    amendment_process: ProcessId,
    interpretation_process: ProcessId,
    succession_rules: Vec<SuccessionRule>,
    emergency_rules: Vec<EmergencyRule>,
    dissolution_rules: Vec<DissolutionRule>,
    ratification: RatificationRecord,
    effective_tick: ChronicleTick,
    version_parent: Option<CharterId>,
}
```

```rust
struct CharterClause {
    clause_id: ClauseId,
    domain: CharterDomain,
    rule: RuleExpression,
    beneficiaries: Vec<ConstituencyId>,
    burdened_groups: Vec<ConstituencyId>,
    administrative_cost: CostModel,
    enforcement_path: ProcessId,
    appeal_path: Option<ProcessId>,
    sunset: Option<ChronicleTick>,
}
```

Domains include:

- membership and residency;
- public-service access;
- property and commons;
- labor;
- office and representation;
- justice;
- emergency powers;
- ecological boundaries;
- data and privacy;
- machine or nonhuman standing;
- taxation and budgets;
- defense;
- amendment and secession.

Charter evaluation must expose distributional effects. A clause may be legal and still be contested, exclusionary, expensive, or difficult to administer.

# 6. Ratification

Ratification mechanisms may include:

- universal resident vote;
- household or constituency vote;
- assembly consensus;
- supermajority;
- delegated convention;
- treaty ratification;
- multi-species concurrence;
- machine quorum;
- provisional adoption followed by delayed referendum.

The runtime records:

- eligible population;
- actual participation;
- exclusion reasons;
- vote or consent method;
- abstentions;
- dissent;
- coercion indicators;
- network or translation failures;
- legal challenges.

A valid process may still produce low legitimacy if participation was narrow or material dependency distorted consent.

# 7. Institutions

```rust
struct InstitutionState {
    institution_id: InstitutionId,
    settlement_id: SettlementId,
    form: InstitutionForm,
    mandate: Mandate,
    offices: Vec<OfficeId>,
    members: Vec<AgentId>,
    assets: Vec<AssetId>,
    budget_id: BudgetId,
    procedures: Vec<ProcessId>,
    public_obligations: Vec<ObligationId>,
    oversight: Vec<OversightId>,
    legitimacy: LegitimacyMap,
    continuity_plan: ContinuityPlan,
    capture_risk: f32,
    status: InstitutionStatus,
}
```

Institution forms are composable rather than a linear tech tree.

A clinic may be public, cooperative, religious, corporate, household-run, mobile, or mixed. Its obligations depend on charter, funding, contracts, and precedent.

# 8. Offices

```rust
struct CivicOffice {
    office_id: OfficeId,
    institution_id: InstitutionId,
    mandate: Mandate,
    authority_tokens: Vec<AuthorityTokenClass>,
    duties: Vec<ObligationTemplateId>,
    eligibility: Vec<EligibilityRule>,
    selection_process: ProcessId,
    term: TermRule,
    recall_process: Option<ProcessId>,
    conflict_rules: Vec<ConflictRule>,
    succession: SuccessionRule,
    current_holder: Option<AgentId>,
}
```

Authority is issued by the office, not stored permanently on the player profile.

On departure, death, recall, term expiry, or worldline divergence, tokens are revoked or transferred through authoritative procedure.

# 9. Public Services

```rust
struct PublicServiceState {
    service_id: ServiceId,
    service_type: ServiceType,
    provider: InstitutionId,
    assets: Vec<AssetId>,
    workers: Vec<WorkAssignmentId>,
    input_flows: Vec<ResourceFlowId>,
    output_flows: Vec<ResourceFlowId>,
    capacity: CapacityModel,
    demand: DemandModel,
    access_rules: Vec<AccessRule>,
    funding: FundingModel,
    public_obligations: Vec<ObligationId>,
    maintenance_debt: f32,
    contingency: ContinuityPlan,
    status: ServiceStatus,
}
```

Services include real operating burdens:

- staffing;
- shifts;
- consumables;
- maintenance;
- records;
- access disputes;
- outages;
- environmental externalities;
- payment and subsidy;
- emergency prioritization.

The player cannot convert a public service into private inventory without a valid institutional process.

# 10. Budgets and Contributions

Funding may come from:

- taxes;
- fees;
- cooperative shares;
- labor contributions;
- grants;
- external sponsorship;
- rents;
- commons revenue;
- debt;
- gifts;
- emergency requisition.

The runtime distinguishes volunteered contribution, contractual labor, taxation, debt, coercion, and uncompensated care.

Budget proposals expose:

```text
who pays
who receives
what is deferred
maintenance debt
care work assumptions
externalized ecological cost
risk under lower revenue
```

# 11. Legitimacy

Legitimacy is stored by constituency and domain.

```rust
struct LegitimacyMap {
    by_constituency: Map<ConstituencyId, LegitimacyVector>,
}

struct LegitimacyVector {
    procedural: f32,
    service: f32,
    historical: f32,
    relational: f32,
    legal: f32,
    ecological: f32,
    coercion_pressure: f32,
}
```

A government may be procedurally legitimate but unable to provide water. A utility may be trusted for service and hated politically. A founder may be admired historically and denied current authority.

# 12. Succession

Succession triggers on:

- term expiry;
- resignation;
- incapacity;
- death;
- reconstitution uncertainty;
- recall;
- removal;
- office abolition;
- institutional merger;
- secession;
- worldline fork.

Succession must transfer:

- active authority;
- public records;
- unresolved obligations;
- budgets;
- access credentials;
- emergency conditions;
- handover uncertainty.

A successor may inherit duties without inheriting trust.

# 13. Multiplayer Authority

Every settlement-changing action requires an authority proof appropriate to its domain.

```rust
struct CivicActionEnvelope {
    action_id: ActionId,
    actor: AgentId,
    claimed_office: Option<OfficeId>,
    authority_proofs: Vec<AuthorityProof>,
    affected_entities: Vec<EntityId>,
    reserved_resources: ResourceReservationSet,
    required_witnesses: Vec<WitnessRequirement>,
    proposed_tick: ChronicleTick,
    worldline_id: WorldlineId,
}
```

The host cannot silently bypass charter rules.

Conflicting actions resolve through authoritative ordering, reservation, and public failure events—not last-writer-wins mutation.

# 14. Simulation LOD

At high fidelity, simulate workers, queues, service flows, meetings, and individual actions.

At lower fidelity, summarize through conserved quantities and bounded institutional transitions:

```text
population
households
service capacity
resource reserves
maintenance debt
legitimacy
unresolved obligations
migration pressure
ecological load
institution capture risk
```

LOD transitions must preserve named events and irreversible decisions.

# 15. Failure Modes

The runtime must support:

- charter deadlock;
- service insolvency;
- office vacancy;
- emergency-power overrun;
- corruption;
- record loss;
- worker strike;
- resident exit;
- ecological boundary breach;
- secession;
- institutional capture;
- public-service collapse;
- peaceful dissolution;
- evacuation;
- successful reform.

None should be represented only by a settlement health bar.

# 16. Determinism and Replay

Persist:

- all adopted rules;
- ratification evidence;
- authority-token ancestry;
- public-service state;
- claims;
- office transitions;
- budget decisions;
- worldline ancestry;
- summarized absence transitions;
- random seeds used by bounded procedural systems.

Replay must reproduce civic outcomes from authoritative inputs.

# 17. Minimum Prototype

The first implementation should include:

- twelve residents;
- four households;
- one prior land or watershed claim;
- one provisional compact;
- three charter variants;
- four institutions;
- five public services;
- three offices;
- one election or appointment;
- one recall or term transfer;
- one service failure;
- one five-year absence;
- one worldline fork.

# Closing Rule

> **A settlement runtime is successful when authority, service, memory, and succession remain legible after the founder is no longer present.**
