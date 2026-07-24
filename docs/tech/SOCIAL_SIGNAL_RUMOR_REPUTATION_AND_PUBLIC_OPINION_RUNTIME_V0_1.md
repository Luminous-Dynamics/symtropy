---
title: Social Signal, Rumor, Reputation, and Public Opinion Runtime
version: 0.1
status: implementation-spec
scope: social claim propagation, rumor mutation, domain reputation, media channels, public opinion aggregation, correction, privacy, persistence, and debugging
owner: engineering/simulation/narrative
related:
  - ../canon/INFORMATION_ECOLOGY_RUMOR_MEDIA_AND_REPUTATION_CONTRACT_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - GROUNDED_DIALOGUE_VOICE_AND_GENERATIVE_SAFETY_RUNTIME_V0_1.md
  - CHRONICLE_EVENT_SCHEMA.md
---

# Social Signal, Rumor, Reputation, and Public Opinion Runtime

## Purpose

This specification turns social information into bounded, deterministic, inspectable simulation.

It does not simulate every conversation word-by-word. It preserves the causal information needed for NPC belief, reputation, institutional action, public reaction, and later explanation.

## Authority Boundary

The runtime owns:

```text
claim identities
transmission attempts
receiver interpretation
rumor mutation lineage
domain reputation evidence
media publication state
public-opinion summaries
correction and retraction state
privacy and disclosure enforcement
```

It does not own:

```text
physical world truth
NPC private cognition beyond declared interfaces
institutional legal authority
Chronicle acceptance
language generation
player inventory or permissions
```

# 1. Core Data Model

```rust
struct ClaimRecord {
    claim_id: ClaimId,
    proposition_id: PropositionId,
    canonical_truth_ref: Option<WorldFactRef>,
    source_id: SourceId,
    source_kind: SourceKind,
    created_tick: ChronicleTick,
    created_at: LocationId,
    evidence: SmallVec<[EvidenceRef; 4]>,
    confidence: QuantizedConfidence,
    disclosure: DisclosurePolicy,
    privacy_tags: SmallVec<[PrivacyTag; 4]>,
    mutation_parent: Option<ClaimId>,
    distortion: DistortionVector,
    status: ClaimStatus,
}
```

The optional `canonical_truth_ref` is restricted to authoritative systems and test tooling. Ordinary NPC and media logic receives observations, evidence references, confidence, contradictions, and permissions—not hidden truth labels.

```rust
struct ClaimPossession {
    holder: AgentOrInstitutionId,
    claim_id: ClaimId,
    acquired_from: SourceId,
    acquired_tick: ChronicleTick,
    belief_confidence: QuantizedConfidence,
    remembered_source_confidence: QuantizedConfidence,
    disclosure_intent: DisclosureIntent,
    affective_charge: i16,
    salience: u16,
}
```

```rust
struct TransmissionEvent {
    event_id: EventId,
    sender: AgentOrInstitutionId,
    receivers: ReceiverSet,
    claim_id: ClaimId,
    channel: ChannelId,
    intent: TransmissionIntent,
    observed_evidence_refs: SmallVec<[EvidenceRef; 4]>,
    authority_context: AuthorityContext,
    result: TransmissionResult,
}
```

# 2. Proposition Layer

A proposition is a typed claim target, not arbitrary text.

Examples:

```text
AgentPerformedAction
DeviceCausedFailure
InstitutionWithheldRecord
RouteIsUnsafe
ResourceIsContaminated
PersonIsCompetentInDomain
PolicyIsLegitimate
AlienSignalMeansBoundary
PredictionOfFutureState
MoralInterpretation
```

Typed propositions allow deterministic comparison, contradiction detection, evidence linking, and grounded language rendering.

```rust
struct Proposition {
    subject: EntityRef,
    predicate: PredicateId,
    object: Option<EntityOrValueRef>,
    qualifiers: SmallVec<[Qualifier; 4]>,
    time_scope: TimeScope,
}
```

# 3. Channels

A channel defines reach, latency, privacy, mutation pressure, persistence, and authority.

```rust
struct SocialChannelProfile {
    channel_id: ChannelId,
    reach: ReachProfile,
    latency_ticks: TickRange,
    bandwidth: u16,
    privacy: ChannelPrivacy,
    persistence: PersistenceClass,
    mutation_pressure: MutationPressure,
    censorship_points: SmallVec<[CensorshipPoint; 4]>,
    technical_dependencies: SmallVec<[SystemDependency; 4]>,
}
```

Representative channels:

```text
face-to-face
household message
workplace conversation
private mesh message
public bulletin
community radio
archive notice
institutional hearing
religious recitation
artistic performance
commercial feed
emergency alert
faction propaganda
```

When power, signal, transport, literacy, translation, or access fails, information routes change rather than all social knowledge freezing.

# 4. Receiver Interpretation

Reception is computed from bounded factors:

```text
prior belief
source trust by domain
relationship to sender
relationship to subject
available evidence
cultural interpretation
current stress
historical similarity
institutional affiliation
translation confidence
```

```rust
struct ReceptionContext {
    receiver: AgentId,
    prior_belief: Option<BeliefState>,
    source_trust: DomainTrust,
    relationship_bias: RelationshipBias,
    evidence_access: EvidenceAccess,
    stress_band: StressBand,
    cultural_frame: CulturalFrameId,
    faction_pressure: PressureVector,
}
```

The runtime returns a belief-update proposal to the NPC cognition layer. The memory and cognition systems remain responsible for accepting, contesting, or deferring it.

# 5. Deterministic Rumor Mutation

Mutation occurs only when a transmission crosses declared pressure thresholds.

```rust
struct DistortionVector {
    compression: i8,
    emotional_emphasis: i8,
    causal_simplification: i8,
    source_laundering: i8,
    false_precision: i8,
    identity_substitution: i8,
    moralization: i8,
    temporal_shift: i8,
}
```

Mutation input:

```text
claim
sender memory quality
sender intent
channel profile
receiver count
stress and uncertainty
local narrative templates
seeded random stream
```

Mutation output must preserve:

```text
parent claim ID
changed fields
mutation cause
seed and algorithm version
```

Freeform generative systems may render the mutated claim but cannot choose the mutation itself.

# 6. Reputation Runtime

Reputation is stored as evidence-weighted domain state between observer and subject.

```rust
struct ReputationState {
    observer: ObserverId,
    subject: SubjectId,
    domain: ReputationDomain,
    estimate: i16,
    confidence: u16,
    evidence_refs: RingBuffer<ReputationEvidence, 24>,
    last_updated: ChronicleTick,
}
```

Recommended domains:

```text
technical competence
medical competence
reliability
honesty
care
fairness
courage
danger
legitimacy
status
```

Reputation updates require evidence records with propagation origin and decay.

```rust
struct ReputationEvidence {
    cause_event: EventId,
    transmission_distance: u8,
    source_credibility: u16,
    directness: EvidenceDirectness,
    domain_match: u16,
    valence: i16,
    strength: u16,
    expiry_or_review: Option<ChronicleTick>,
}
```

No reputation update may silently affect unrelated domains.

# 7. Public Opinion

Public opinion is a query result over situated agents and institutions, not a stored single value.

Queries may aggregate by:

```text
neighborhood
profession
household type
faction
age or life stage
media network
institutional membership
migration history
species or substrate
```

```rust
struct OpinionSnapshot {
    query_id: QueryId,
    proposition_id: PropositionId,
    population_scope: PopulationScope,
    support_distribution: Distribution,
    confidence_distribution: Distribution,
    knowledge_coverage: u16,
    polarization: u16,
    uncertainty: u16,
    dominant_reasons: Vec<ReasonCluster>,
    dissent_clusters: Vec<ReasonCluster>,
    sampled_tick: ChronicleTick,
}
```

The UI must display uncertainty and knowledge coverage. “68% support” is misleading when most people have not encountered the claim.

# 8. Media Runtime

```rust
struct MediaInstitutionState {
    institution_id: InstitutionId,
    ownership: OwnershipModel,
    funding_dependencies: Vec<DependencyRef>,
    editorial_roles: Vec<RoleAssignment>,
    access_scope: AccessScope,
    correction_policy: CorrectionPolicy,
    source_protection_policy: SourceProtectionPolicy,
    archive_policy: ArchivePolicy,
    current_pressures: PressureVector,
}
```

Publishing is an institutional action with:

```text
claim selection
evidence access
editorial framing
disclosure checks
source protection
legal or faction pressure
technical distribution
archive persistence
```

A publication creates a new claim record referencing source claims and evidence. It does not overwrite them.

# 9. Correction, Retraction, and Restitution

```rust
struct CorrectionCase {
    case_id: CaseId,
    target_claims: Vec<ClaimId>,
    correction_claim: ClaimId,
    issuing_party: AgentOrInstitutionId,
    affected_parties: Vec<AgentId>,
    record_actions: Vec<RecordAction>,
    restitution_actions: Vec<RestitutionAction>,
    distribution_plan: DistributionPlan,
    status: CorrectionStatus,
}
```

Correction propagation follows ordinary channels but receives special UI and institution support. It does not force belief acceptance.

The system tracks:

```text
who received the original claim
who received the correction
which records changed
which material harms remain
which relationships remain damaged
```

# 10. Privacy and Security

Every transmission must pass:

```text
disclosure policy
relationship permission
institutional authority
worldline safety profile
child/dependent-person safeguards
medical privacy
source protection
alien contact boundary
```

Private cognition is never converted into rumor unless an authorized in-world event exposes it.

Synthetic media must be tagged at the semantic event layer, not only through visual watermarks.

# 11. Levels of Detail

## L0 — Full Situated

Named nearby agents retain individual claim possession, source memory, confidence, disclosure intent, and transmission events.

## L1 — District Summary

Agents retain important claim holdings and reputation evidence; low-salience conversation compresses into network updates.

## L2 — Regional Aggregate

Population cohorts retain opinion distributions, media reach, major rumor lineages, and institutional records.

## L3 — Dormant Worldline

Only durable claims, dominant narratives, corrections, major reputation effects, and scheduled transformations persist.

LOD transitions must preserve:

```text
claim provenance
privacy
major dissent
named-agent evidence
unresolved correction cases
Chronicle links
```

# 12. Persistence and Networking

Persistent identifiers include:

```text
claim IDs
proposition IDs
mutation lineage
publication IDs
correction cases
reputation evidence IDs
```

Real-time transmission may be shard-authoritative. Durable publications, official corrections, and historically significant accusations may become Chronicle candidates after validation.

The system must survive:

```text
disconnect during publication
save/load during rumor spread
worldline fork
partial Chronicle availability
mod removal
schema migration
```

# 13. Observability

Debug tooling should provide:

```text
claim lineage graph
transmission graph
receiver belief proposal trace
reputation evidence ledger
media ownership and pressure view
original-versus-correction reach
public-opinion uncertainty view
privacy redaction report
```

Player-facing explanations remain narrower and rights-respecting.

# 14. Representative Test

Seed: `firstlight.social-signal.001`

Scenario:

```text
route sensor fails ambiguously
Morrow-7 reports uncertainty
an exhausted driver interprets silence as concealment
political rival amplifies accusation
community radio publishes before evidence access
archive desk finds contradictory maintenance record
private health fact must remain sealed
public correction is issued
workshop trust improves slowly; one household remains resentful
```

Assertions:

- no agent learns the private health fact;
- fixed seed reproduces rumor lineage;
- correction reaches a different network than the original rumor;
- competence, honesty, and danger reputation change independently;
- public opinion query reports coverage and uncertainty;
- generative dialogue cannot invent a new source or evidence item;
- save/load preserves lineages and correction state.

# 15. Performance Budget

Representative district target:

```text
12 named agents with full claim state
40–120 ambient agents in district summary
8 active rumor lineages
4 media channels
2 institutions
24 reputation domains per named observer-subject neighborhood, sparse storage
```

Social propagation should run event-driven, not every frame.

Degradation order:

```text
reduce low-salience transmission detail
compress ambient-agent source memory
reduce opinion query frequency
retain named-agent claims, privacy, major rumor lineage, correction cases, and durable evidence
```

# Acceptance Criteria

- Fixed inputs and seed produce deterministic propagation and mutation.
- Every received claim has a valid acquisition path.
- Domain reputation updates reference evidence and do not spill globally.
- Public opinion reports uncertainty and coverage.
- Privacy checks fail closed.
- Corrections do not erase original consequences.
- Media ownership and technical dependencies affect reach.
- LOD transitions preserve provenance and named-agent continuity.
- All optional language generation can be disabled without changing social truth.
- The runtime exposes bounded causal traces for failure triage.

# Final Rule

> **Social simulation becomes believable when every belief has a path, every reputation has evidence, every rumor has a lineage, and every correction must travel through the same wounded society as the original harm.**
