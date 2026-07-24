---
title: Public Administration, Succession, and Corruption Runtime
version: 0.1
status: implementation-spec
scope: offices, civil service, delegated authority, succession transactions, procurement, corruption pressure, audit, continuity LOD
owner: simulation/governance/engineering/narrative/security
related:
  - ../canon/CIVIC_SUCCESSION_PUBLIC_SERVICE_AND_INSTITUTIONAL_CONTINUITY_CONTRACT_V0_1.md
  - INSTITUTIONAL_COLLECTIVE_COGNITION_AND_PUBLIC_REASON_RUNTIME_V0_1.md
  - MULTIPLAYER_TRUTH_MODEL.md
  - CHRONICLE_EVENT_SCHEMA.md
  - ../canon/JUSTICE_HARM_ACCOUNTABILITY_AND_REPAIR_CONTRACT_V0_1.md
---

# Public Administration, Succession, and Corruption Runtime

## Purpose

Define a bounded runtime for offices, public-service capability, delegated authority, leadership transitions, procurement, conflicts of interest, institutional capture, and administrative continuity.

This runtime is not a government-roleplay engine. It exists so essential services do not reduce to a leader NPC, a policy menu, or an abstract legitimacy bar.

## Core Architecture

```text
people and machines
  → roles and qualifications
  → offices and service units
  → scoped authority tokens
  → procedures and transactions
  → service outputs
  → records, review, and Chronicle consequences
```

> **An institution acts through validated role transactions, not through a free-form collective intention.**

# 1. Authoritative State

```rust
struct OfficeState {
    office_id: OfficeId,
    institution_id: InstitutionId,
    purpose: PurposeId,
    jurisdiction: Jurisdiction,
    scopes: Vec<AuthorityScope>,
    limits: Vec<AuthorityLimit>,
    holder: Option<AgentId>,
    deputies: Vec<DeputyAssignment>,
    term: TermState,
    review_rule: ReviewRule,
    continuity_plan: ContinuityPlanId,
    active_delegations: Vec<DelegationId>,
    open_obligations: Vec<ObligationId>,
    records_cursor: RecordCursor,
    conflict_flags: Vec<ConflictFlag>,
    legitimacy: LegitimacyState,
    service_dependency_ids: Vec<ServiceUnitId>,
    schema_version: SchemaVersion,
}

struct ServiceUnitState {
    service_id: ServiceUnitId,
    mission: ServiceMission,
    staffing: StaffingState,
    qualifications: QualificationCoverage,
    operating_capacity: Scalar,
    fatigue: Scalar,
    backlog: BacklogState,
    procedures: Vec<ProcedureId>,
    critical_dependencies: Vec<DependencyId>,
    degraded_modes: Vec<DegradedMode>,
    continuity_status: ContinuityStatus,
}
```

Office state and service-unit state must remain separate.

Removing an officeholder may reduce authorization or coordination, but it must not automatically delete staff competence, physical assets, procedures, or all service capacity.

# 2. Authority Tokens

All public authority that affects authoritative state should be represented by scoped, inspectable tokens.

```rust
struct AuthorityToken {
    token_id: TokenId,
    issuer: AuthorityIssuer,
    holder: AgentOrRole,
    scopes: Vec<AuthorityScope>,
    jurisdiction: Jurisdiction,
    issued_at: ChronicleTick,
    expires_at: Option<ChronicleTick>,
    review_at: Option<ChronicleTick>,
    delegation_chain: Vec<DelegationEdge>,
    emergency_basis: Option<EmergencyBasis>,
    revocation_rule: RevocationRule,
    signature: Signature,
}
```

Authority checks must evaluate:

- scope;
- jurisdiction;
- expiry;
- delegation chain;
- conflicts of interest;
- emergency limits;
- co-signature requirements;
- current institutional status;
- rights-floor constraints.

No dialogue output, relationship value, or cognition proposal may mint an authority token.

# 3. Succession Transaction

A succession is an atomic, multi-domain transaction rather than a single pointer change.

```rust
struct SuccessionTransaction {
    succession_id: SuccessionId,
    office_id: OfficeId,
    trigger: SuccessionTrigger,
    outgoing_holder: Option<AgentId>,
    proposed_interim: Option<AgentId>,
    proposed_successor: Option<AgentId>,
    credential_actions: Vec<CredentialAction>,
    record_handover: RecordHandover,
    obligation_handover: Vec<ObligationTransfer>,
    emergency_powers_expired: Vec<AuthorityScope>,
    service_continuity_actions: Vec<ContinuityAction>,
    challenges: Vec<SuccessionChallenge>,
    public_notice: NoticeId,
    validation_result: ValidationResult,
}
```

The transaction must be able to partially succeed.

Example:

```text
interim dispatch authority accepted
procurement authority withheld pending review
records handed over with missing annex warning
medical privacy access not transferred
three open obligations remain disputed
```

# 4. Qualification and Competence Coverage

Formal authority and operational competence are different maps.

```rust
struct QualificationCoverage {
    required_domains: Vec<SkillDomain>,
    available_staff: Vec<QualifiedAgent>,
    redundancy_by_domain: Map<SkillDomain, u8>,
    tacit_knowledge_risk: Map<SkillDomain, Scalar>,
    certification_expiry: Map<AgentId, ChronicleTick>,
    training_pipeline: Vec<ApprenticeshipId>,
}
```

A service unit can remain legally authorized but practically incapable.

It can also remain operational through experienced staff while formal leadership is disputed.

The causal-explanation layer must expose this distinction.

# 5. Administrative Procedures

Procedures are authored templates with bounded variation.

Examples:

- emergency procurement;
- ordinary procurement;
- credential renewal;
- public-record publication;
- safety inspection;
- staffing transfer;
- succession review;
- conflict disclosure;
- whistleblower intake;
- public appeal;
- service continuity activation.

```rust
struct AdministrativeProcedure {
    procedure_id: ProcedureId,
    required_roles: Vec<RoleRequirement>,
    required_inputs: Vec<InputRequirement>,
    authority_checks: Vec<AuthorityCheck>,
    privacy_rules: Vec<PrivacyRule>,
    deadlines: Vec<Deadline>,
    allowed_discretion: Vec<DiscretionWindow>,
    output_events: Vec<EventTemplate>,
    review_path: Option<ReviewPath>,
}
```

Procedures must support compassionate discretion where canon permits it, but all discretion must be explainable and reviewable.

# 6. Public-Service Capacity

Public service is simulated through capabilities, not individual keystrokes.

Core variables:

```text
staff coverage
skill redundancy
fatigue
backlog
resource availability
record quality
procedure clarity
public trust by domain
cross-unit coordination
infrastructure condition
political interference
```

The runtime should generate visible consequences:

- delayed permits;
- missed inspections;
- shorter clinic hours;
- inconsistent route dispatch;
- repair backlogs;
- better emergency response after drills;
- more resilient service after cross-training;
- public frustration;
- informal workarounds;
- staff resignation or organizing.

# 7. Corruption Pressure Model

The runtime tracks corruption pressure, opportunities, attempted acts, evidence, and institutional response separately.

```rust
struct CorruptionPressure {
    concentrated_discretion: Scalar,
    scarcity_value: Scalar,
    audit_weakness: Scalar,
    retaliation_risk: Scalar,
    patronage_density: Scalar,
    personal_dependency: Scalar,
    procurement_opacity: Scalar,
    emergency_urgency: Scalar,
    normalized_impunity: Scalar,
}
```

A high pressure value does not make every official corrupt.

Agent choices also depend on:

- values;
- obligations;
- fear;
- relationships;
- material security;
- faction norms;
- perceived detection;
- expected harm;
- available alternatives.

## 7.1 Corruption Events

```rust
struct IntegrityIncident {
    incident_id: IncidentId,
    suspected_pattern: IntegrityPattern,
    actors: Vec<AgentId>,
    affected_transaction_ids: Vec<TransactionId>,
    evidence_refs: Vec<EvidenceRef>,
    confidence: Confidence,
    privacy_scope: PrivacyScope,
    investigation_status: InvestigationStatus,
    adjudication_status: AdjudicationStatus,
    institutional_remedy_ids: Vec<RemedyId>,
}
```

Suspicion must never automatically create guilt or reputation certainty.

# 8. Procurement and Conflict of Interest

Procurement can create systemic power because infrastructure contracts shape maintenance, data access, spare parts, and future dependency.

Every consequential procurement should record:

- need statement;
- alternatives considered;
- bidder or provider;
- ownership links;
- decision roles;
- declared conflicts;
- price and non-price terms;
- maintenance dependencies;
- lock-in risks;
- emergency basis;
- review status;
- delivery and performance evidence.

The player may discover a conflict of interest without proof of bribery. That should still affect review, disclosure, recusal, and public trust.

# 9. Whistleblowing and Protected Dissent

A whistleblower event requires:

- a report channel;
- privacy classification;
- retaliation risk;
- evidence references;
- receiving institution;
- protection status;
- investigation state;
- public-disclosure threshold.

The game must not assume all whistleblowers are truthful or all institutions are malicious. It must preserve uncertainty and due process while making retaliation and suppression possible systemic failures.

# 10. Institutional Capture

Capture is modeled as persistent influence over roles, procedures, information, appointments, and resource flows.

Capture vectors include:

```text
appointment dominance
procurement dependency
archive control
media influence
credential monopoly
security intimidation
staff patronage
technical lock-in
campaign or faction financing
emergency authority
```

Institutional capture should alter generated options and information visibility, not simply apply a corruption debuff.

# 11. Simulation Levels of Detail

## LOD 0 — Active Procedure

Used for a live succession, hearing, procurement, investigation, or continuity event.

Preserves named agents, evidence, authority checks, and timing.

## LOD 1 — Active Institution

Tracks role coverage, backlog, fatigue, legitimacy, capture pressure, and major decisions.

## LOD 2 — Regional Administration

Aggregates service reliability, staffing, corruption risk, and continuity readiness.

## LOD 3 — Historical Summary

Stores leadership periods, major reforms, failures, scandals, continuity events, and institutional lineage.

LOD transitions must preserve:

- current officeholders;
- authority scopes;
- unresolved obligations;
- open investigations;
- service capacity bands;
- major capture relationships;
- succession state;
- Chronicle-significant events.

# 12. Networking and Persistence

- real-time presence and local animations use local real-time truth;
- office and credential transitions use device/civic transaction truth;
- public appointments, removals, major findings, and reforms use Chronicle truth;
- worldline forks preserve institutional ancestry and disputed claims;
- private reports and protected records must not be globally replicated without authorized disclosure.

# 13. Causal Trace

Every consequential administrative outcome must produce a bounded trace.

Example:

```text
Clinic evening hours reduced
because two certified staff resigned,
training coverage fell below safety threshold,
and interim procurement authority expired before a replacement contract was approved.
```

The trace must distinguish known causes from inferred causes.

# 14. Seedworks Fixture

Representative fixture:

1. the basin logistics coordinator becomes unavailable;
2. a deputy receives temporary dispatch authority but not procurement authority;
3. a convoy route fails during the transition;
4. staff use a continuity runbook;
5. a supplier linked to the deputy offers an emergency contract;
6. a conflict flag triggers review;
7. the player may support transparent recusal, conceal the relationship, choose a slower alternative, or activate mutual aid;
8. services continue at degraded capacity;
9. the handover and review persist through save/load.

# 15. Acceptance Tests

The runtime fails validation if:

- one NPC deletion erases the institution;
- authority expands beyond the token scope;
- emergency authority persists silently;
- a corruption score determines guilt;
- audit reveals protected medical, relationship, or cognitive data;
- service degradation has no legible cause;
- formal office and actual competence are treated as identical;
- succession loses open obligations;
- save/load duplicates credentials;
- LOD aggregation changes a resolved adjudication or active appeal;
- optional generative language changes a procedure outcome.

# 16. Performance Budget

Representative regional target:

- 8–20 active institutions;
- 20–60 offices;
- 100–300 service roles aggregated by unit;
- no per-frame administration planning;
- event-driven procedure evaluation;
- bounded daily or hourly institutional ticks;
- active-case expansion only when player-relevant or system-critical.

## Final Rule

> **The administration runtime must make institutions durable enough to outlive leaders, but interruptible enough to remain accountable to living people.**
