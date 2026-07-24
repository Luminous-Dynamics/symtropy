---
title: Player Promise, Office, Reputation, and Legacy Runtime
version: 0.1
status: implementation-spec
scope: commitments, promises, civic office, public and private reputation, attribution, founder memory, inheritance, death, reconstitution
owner: gameplay/AI/narrative/networking
related:
  - ../canon/PLAYER_FOUNDED_CIVILIZATION_SETTLEMENT_LEGACY_AND_WORLDLINE_CONTRACT_V0_1.md
  - ../canon/INFORMATION_ECOLOGY_RUMOR_MEDIA_AND_REPUTATION_CONTRACT_V0_1.md
  - ../canon/CIVIC_SUCCESSION_PUBLIC_SERVICE_AND_INSTITUTIONAL_CONTINUITY_CONTRACT_V0_1.md
  - COMPANION_COORDINATION_DELEGATION_AND_TRUST_RUNTIME_V0_1.md
  - SETTLEMENT_FOUNDING_CHARTER_INSTITUTION_AND_PUBLIC_SERVICE_RUNTIME_V0_1.md
---

# Player Promise, Office, Reputation, and Legacy Runtime

## Purpose

This runtime gives player commitments and public roles durable consequences without reducing identity to a morality score or making historical fame a reward currency.

It distinguishes:

- promises;
- contracts;
- public obligations;
- office duties;
- informal expectations;
- reputation by audience;
- attribution;
- legacy after absence or death.

# 1. Commitments

```rust
struct CommitmentState {
    commitment_id: CommitmentId,
    parties: Vec<AgentOrInstitutionId>,
    commitment_type: CommitmentType,
    terms: Vec<Term>,
    created_tick: ChronicleTick,
    due_conditions: Vec<Condition>,
    privacy: PrivacyClass,
    witnesses: Vec<WitnessRecordId>,
    authority_basis: Option<AuthorityProof>,
    resource_reservations: Vec<ResourceReservationId>,
    status: CommitmentStatus,
    interpretation_disputes: Vec<DisputeId>,
}
```

Commitment types include:

```text
personal promise
professional undertaking
public pledge
contract
care obligation
office duty
emergency undertaking
treaty commitment
repayment
confidentiality
custody
maintenance guarantee
```

A personal promise and a legal contract may overlap but are not identical.

# 2. Formation

A commitment requires:

- identifiable parties;
- understandable terms;
- capacity;
- valid authority where claimed;
- voluntary participation where consent is required;
- evidence appropriate to the context;
- a way to interpret ambiguity.

Dialogue generation may propose wording but cannot silently add binding terms.

# 3. Fulfillment

Fulfillment may be:

- complete;
- partial;
- substituted by agreement;
- delayed with notice;
- impossible through external cause;
- abandoned;
- breached;
- disputed;
- inherited;
- extinguished.

The system records what was actually done, not only the final status.

# 4. Competing Obligations

Players may face obligations that cannot all be fulfilled.

The runtime exposes:

- urgency;
- authority;
- dependency;
- foreseeable harm;
- consent;
- prior commitments;
- available delegation;
- cost of delay;
- who bears the consequence.

There is no universal algorithm that declares the morally correct priority.

# 5. Office Tenure

```rust
struct OfficeTenure {
    tenure_id: OfficeTenureId,
    office_id: OfficeId,
    holder: AgentId,
    start_tick: ChronicleTick,
    expected_end: ChronicleTick,
    authority_tokens: Vec<AuthorityTokenId>,
    duties: Vec<ObligationId>,
    conflicts: Vec<ConflictDisclosureId>,
    actions: Vec<ChronicleEventId>,
    absences: Vec<AbsenceRecord>,
    end_reason: Option<TenureEndReason>,
    handover: Option<HandoverRecordId>,
}
```

Authority tokens expire when tenure ends.

A former mayor may retain fame, contacts, and knowledge but cannot issue current orders.

# 6. Public and Private Reputation

```rust
struct ReputationView {
    subject: AgentId,
    observer: AgentOrGroupId,
    domain: ReputationDomain,
    belief: BeliefDistribution,
    evidence_refs: Vec<EvidenceRef>,
    rumor_refs: Vec<RumorId>,
    personal_experience: Vec<ChronicleEventId>,
    confidence: f32,
    affect: AffectVector,
}
```

Domains include:

- professional competence;
- reliability;
- care;
- courage;
- fairness;
- corruption;
- violence;
- technical style;
- political alignment;
- discretion;
- cultural taste;
- household behavior;
- historical significance.

There is no global reputation scalar.

A player may be trusted as a mechanic and distrusted as an office-holder. A rival may respect competence and resent politics. A household may know private kindness that the public never sees.

# 7. Rumor and Media

Reputation changes through:

- direct experience;
- witnesses;
- records;
- institutional reports;
- journalism;
- propaganda;
- gossip;
- art;
- memorials;
- trials;
- jokes;
- school curricula;
- machine archives;
- absence.

False information may spread without changing authoritative history.

# 8. Attribution

```rust
struct AttributionClaim {
    attribution_id: AttributionId,
    event_id: ChronicleEventId,
    claimant: AgentOrInstitutionId,
    credited_agents: Vec<WeightedAgentCredit>,
    blamed_agents: Vec<WeightedAgentBlame>,
    evidence: Vec<EvidenceRef>,
    narrative_frame: FrameId,
    audience: ConstituencyId,
    status: AttributionStatus,
}
```

The simulation may contain multiple public attribution claims about one event.

A founder may receive excessive credit because their name was on the project. Workers may later recover erased credit. A failure may be blamed on a successor despite originating in founder design.

# 9. Legacy

Legacy is the persistence of consequences and interpretations after a person's active participation.

```rust
struct LegacyState {
    subject: AgentId,
    material_traces: Vec<EntityId>,
    institutional_traces: Vec<InstitutionId>,
    legal_precedents: Vec<PrecedentId>,
    cultural_artifacts: Vec<ArtifactId>,
    remembered_promises: Vec<CommitmentId>,
    harms: Vec<HarmRecordId>,
    beneficiaries: Vec<AgentOrGroupId>,
    contested_attributions: Vec<AttributionId>,
    commemorations: Vec<CommemorationId>,
    erasures: Vec<ErasureAttemptId>,
}
```

Legacy is not necessarily fame. A forgotten repair standard may matter more than a statue.

# 10. Founder Myth

Founder myth formation may be driven by:

- political need;
- tourism;
- descendants;
- enemies;
- school simplification;
- corporate branding;
- missing records;
- reconstitution uncertainty;
- symbolic anniversaries;
- later crises.

Myths may:

- exaggerate individual agency;
- erase collaborators;
- sanitize harm;
- invent foresight;
- convert accidents into strategy;
- portray compromise as betrayal;
- turn ordinary habits into sacred tradition.

The game should allow the player to confront, accept, exploit, ignore, or fail to change these myths.

# 11. Inheritance

Possible inherited elements include:

- property;
- debt;
- office obligations;
- unresolved contracts;
- family expectations;
- cultural identity;
- machine keys;
- source-chain claims;
- reputation;
- legal liability;
- public-service guarantees.

Not everything is inheritable.

Political authority, consent, friendship, guilt, professional license, and private access require independent rules.

# 12. Death and Reconstitution

On death:

- offices enter succession;
- promises are classified as extinguished, institutionalized, inheritable, or unresolved;
- public interpretation begins;
- private grief remains separate from public legacy;
- source-chain recovery creates evidence, not automatic continuity.

On reconstitution:

- identity may be verified;
- old office does not automatically return;
- inherited property may require adjudication;
- relationships may have changed;
- a legal death period may remain historically real;
- multiple versions may hold competing memories without duplicate authority.

# 13. Absence

The runtime stores absence as a historical condition:

```rust
struct AbsenceRecord {
    subject: AgentId,
    start_tick: ChronicleTick,
    expected_return: Option<ChronicleTick>,
    stated_reason: Option<ReasonFrameId>,
    delegated_duties: Vec<DelegationId>,
    unresolved_commitments: Vec<CommitmentId>,
    contact_availability: ContactModel,
    return_tick: Option<ChronicleTick>,
}
```

People may interpret the same absence as sacrifice, abandonment, necessity, cowardice, or ordinary travel.

# 14. Player-Facing Legibility

IRIS may summarize:

- current commitments;
- explicit deadlines;
- authority scope;
- public records;
- known reputation differences;
- disputed attribution;
- consequences of absence.

IRIS may not reveal private opinions, hidden trauma, or unobserved plans.

UI wording should prefer:

```text
"Sera expects an answer before the water vote."
"Your emergency authority expired 18 days ago."
"Three public accounts credit you; the repair crew's record credits six workers."
"You promised transport, but no vehicle was reserved."
```

Avoid:

```text
"Loyalty -10"
"Settlement Reputation: Heroic"
"Promise Quest Failed"
```

# 15. Multiplayer

Promises and offices require authoritative records.

A client cannot:

- forge another player's pledge;
- preserve expired office tokens;
- transfer private commitments without permission;
- merge worldline reputations by profile ID;
- inherit host authority.

# 16. Validation

The first proof requires:

- five distinct commitment types;
- one impossible conflict between obligations;
- one office term and handover;
- three constituency-specific reputation views;
- one false rumor;
- one attribution dispute;
- one founder myth;
- one five-year absence;
- one death or reconstitution case;
- one legacy that persists without public fame.

# Closing Rule

> **The player should be remembered for consequences, commitments, and relationships—not because the game declared them important.**
