---
title: Knowledge, Archive, and Historical Evidence Runtime
version: 0.1
status: implementation-spec
scope: evidence objects, provenance, archival custody, interpretation, preservation, access, redaction, historical claims, archive LOD
owner: simulation/data/chronicle/narrative/security/engineering
related:
  - ../canon/ARCHIVES_HISTORIOGRAPHY_HERITAGE_AND_COLLECTIVE_MEMORY_CONTRACT_V0_1.md
  - CHRONICLE_EVENT_SCHEMA.md
  - MULTIPLAYER_TRUTH_MODEL.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
  - SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
---

# Knowledge, Archive, and Historical Evidence Runtime

## Purpose

Define how evidence, records, archives, interpretations, access rights, preservation state, redaction, missing context, historical claims, and worldline ancestry are represented and validated.

The runtime must support uncertainty and contested interpretation without turning the past into arbitrary fiction.

## Prime Directive

> **Store evidence, provenance, custody, and interpretation separately.**

# 1. Core Data Model

```rust
struct EvidenceObject {
    evidence_id: EvidenceId,
    evidence_type: EvidenceType,
    origin_event: Option<EventId>,
    created_by: Option<AgentOrDeviceId>,
    created_at: TimeCoordinate,
    observed_at: Option<TimeCoordinate>,
    physical_location: Option<LocationId>,
    content_ref: ContentRef,
    integrity: IntegrityState,
    provenance: ProvenanceGraphId,
    custody: CustodyChainId,
    privacy_class: PrivacyClass,
    cultural_protocols: Vec<CulturalProtocolId>,
    uncertainty: UncertaintyState,
    schema_version: SchemaVersion,
}

struct HistoricalClaim {
    claim_id: ClaimId,
    proposition: StructuredProposition,
    supporting_evidence: Vec<EvidenceWeight>,
    contradicting_evidence: Vec<EvidenceWeight>,
    interpretation_method: MethodRef,
    author_or_institution: AgentOrInstitutionId,
    confidence: Confidence,
    scope: ClaimScope,
    publication_state: PublicationState,
    supersedes_or_contests: Vec<ClaimId>,
}

struct ArchiveState {
    archive_id: ArchiveId,
    institution_id: Option<InstitutionId>,
    holdings: ArchiveIndex,
    preservation_profile: PreservationProfile,
    access_policy: AccessPolicyId,
    classification_policy: ClassificationPolicyId,
    custody_authority: CustodyAuthority,
    replication_peers: Vec<ArchiveId>,
    translation_coverage: TranslationCoverage,
    known_gaps: Vec<GapRecord>,
    capture_risk: Scalar,
    maintenance_backlog: Scalar,
    continuity_status: ContinuityStatus,
}
```

# 2. Evidence Types

Supported categories include:

```text
signed event
source-chain entry
device transaction
sensor record
physical artifact
biological trace
environmental scar
image/audio record
personal testimony
oral history
administrative record
ritual memory
machine testimony
archaeological context
derived reconstruction
statistical summary
translation
simulation or forensic model
```

Each type has different failure modes and evidentiary limits.

A signature authenticates origin and integrity. It does not establish that the statement is true.

# 3. Provenance Graph

```rust
struct ProvenanceNode {
    node_id: ProvenanceNodeId,
    transform: ProvenanceTransform,
    input_refs: Vec<ContentRef>,
    output_ref: ContentRef,
    actor_or_tool: ActorOrToolId,
    timestamp: TimeCoordinate,
    parameters_hash: Hash,
    method_version: VersionId,
    confidence_delta: ConfidenceDelta,
}
```

Transforms include:

- copying;
- compression;
- translation;
- transcription;
- redaction;
- restoration;
- format migration;
- aggregation;
- inference;
- reconstruction;
- publication;
- declassification.

Every derived object must preserve its source references and method version.

# 4. Custody Chain

Custody answers who physically or logically controlled an evidence object and when.

```rust
struct CustodyEvent {
    evidence_id: EvidenceId,
    from: Option<CustodianId>,
    to: CustodianId,
    time: TimeCoordinate,
    transfer_basis: TransferBasis,
    condition_before: IntegrityState,
    condition_after: IntegrityState,
    witness_refs: Vec<WitnessRef>,
    signature: Signature,
}
```

Broken custody does not automatically invalidate evidence. It raises uncertainty and creates possible manipulation paths.

# 5. Integrity and Preservation

Integrity dimensions:

```text
content completeness
media condition
signature validity
format readability
context completeness
translation quality
custody continuity
tamper indicators
sensor calibration
identity certainty
```

Preservation systems consume:

- power;
- cooling or environmental control;
- storage media;
- redundancy;
- maintenance labor;
- format migration;
- cataloging;
- translation;
- physical security;
- privacy controls.

Archive loss should usually be progressive and specific rather than one binary destruction event.

# 6. Access and Privacy

Access policy evaluates:

- requester identity and role;
- purpose;
- jurisdiction;
- consent;
- privacy class;
- cultural protocol;
- danger of disclosure;
- public-interest rule;
- time-based expiry;
- appeal or review;
- redaction alternatives.

```rust
struct AccessDecision {
    request_id: AccessRequestId,
    archive_id: ArchiveId,
    requester: AgentId,
    requested_refs: Vec<EvidenceId>,
    purpose: AccessPurpose,
    decision: AccessDisposition,
    redaction_profile: Option<RedactionProfileId>,
    reasons: Vec<ReasonCode>,
    appeal_path: Option<AppealPath>,
    audit_event: EventId,
}
```

Private cognition, intimate relationships, medical data, child records, witness identities, and culturally restricted knowledge require fail-closed handling.

# 7. Correction Without Erasure

A correction creates a new linked record.

```text
original record remains preserved
correction identifies the disputed field
reason and evidence are attached
public index points to current interpretation
historical access can reveal the revision chain
privacy rules still apply
```

Silent mutation of a signed or published record is forbidden.

# 8. Missing Context and Unknown State

The runtime explicitly supports:

```text
record exists, context missing
record referenced, object missing
physical artifact present, origin uncertain
testimony present, identity uncertain
translation incomplete
provenance broken
archive known to be censored
archive known to be incomplete
conflicting calendars or timestamps
worldline ancestry ambiguous
```

Unknown values must not default to the most convenient narrative interpretation.

# 9. Historical Interpretation

Interpretation is a bounded operation over evidence.

```rust
struct InterpretationJob {
    job_id: InterpretationJobId,
    question: StructuredQuestion,
    evidence_scope: Vec<EvidenceId>,
    method: InterpretationMethod,
    assumptions: Vec<Assumption>,
    exclusions: Vec<ExclusionReason>,
    output_claims: Vec<ClaimId>,
    unresolved_questions: Vec<StructuredQuestion>,
    reviewer_ids: Vec<AgentOrInstitutionId>,
}
```

Methods may include:

- provenance reconstruction;
- chronology;
- comparative testimony;
- material forensics;
- causal graph analysis;
- linguistic analysis;
- ecological reconstruction;
- source criticism;
- statistical aggregation.

The runtime should not generate a universal “truth score.” It should expose support, contradiction, assumptions, and uncertainty.

# 10. Archive Capture and Censorship

Capture affects:

- acquisition priorities;
- classification;
- indexing;
- translation funding;
- public display;
- declassification;
- staffing;
- preservation budgets;
- search ranking;
- destruction or neglect.

Capture events must leave causal traces and possible evidence. A captured archive can still contain genuine records.

# 11. Heritage and Memorial State

```rust
struct HeritageState {
    heritage_id: HeritageId,
    subject_refs: Vec<LocationOrPracticeRef>,
    claimant_groups: Vec<GroupId>,
    maintenance_stewards: Vec<StewardId>,
    significance_claims: Vec<ClaimId>,
    contested_claims: Vec<ClaimId>,
    access_rules: AccessPolicyId,
    use_state: HeritageUseState,
    preservation_risk: Scalar,
    living_practice_links: Vec<PracticeId>,
    restitution_obligations: Vec<ObligationId>,
}
```

A memorial stores:

- named and unnamed subjects;
- sponsor;
- evidence basis;
- omissions;
- ritual uses;
- maintenance state;
- protest or reinterpretation history;
- accessibility;
- public narrative version.

# 12. Worldline Forks

On fork:

- evidence objects retain stable ancestry IDs;
- new custody and interpretation chains diverge;
- original signatures remain valid in their original context;
- each worldline may publish different historical claims;
- cross-worldline comparison must preserve which events actually occurred in each branch;
- merged archives cannot collapse contradictory branch histories into one event stream.

# 13. Procedural Generation

Procedural history may generate evidence only through typed provenance rules.

A generated historical event must produce:

- event ID;
- actors;
- affected systems;
- surviving traces;
- missing or destroyed traces;
- records by institution;
- faction interpretations;
- present gameplay hooks.

No record may appear merely because a dialogue scene needs exposition.

# 14. Simulation Levels of Detail

## LOD 0 — Active Investigation

Tracks individual evidence objects, access, testimony, methods, and claims.

## LOD 1 — Active Archive

Tracks holdings classes, preservation, access pressure, staffing, capture, and major disputes.

## LOD 2 — Regional Memory Ecology

Aggregates archive diversity, public trust, translation coverage, loss risk, and narrative conflict.

## LOD 3 — Historical Summary

Preserves major claims, evidence losses, archive lineages, memorials, reparative obligations, and Chronicle events.

LOD transitions must never fabricate provenance or resolve uncertainty.

# 15. Search and Field Deck Presentation

Search results must distinguish:

```text
record title
record type
origin
date confidence
provenance status
access status
known omissions
interpretation status
privacy warning
related disputes
```

Field Deck overlays should avoid presenting inferred history as a physical scan.

# 16. Representative Fixture

One district archive contains:

- a signed evacuation order;
- a sensor log showing the route was already unsafe;
- an oral account claiming officials knew earlier;
- a missing procurement annex;
- a private medical roster that cannot be publicly exposed;
- a memorial that omits migrant deaths;
- a damaged artifact from the collapsed bridge;
- two competing historical claims.

The player can preserve, investigate, publish, redact, challenge, or defer. No path reveals a perfect omniscient transcript.

# 17. Acceptance Tests

Fail the runtime if:

- signatures are treated as truth scores;
- translations lose method and source links;
- a correction mutates the original record;
- archive search bypasses privacy;
- procedural evidence lacks provenance;
- LOD resolves unknown facts;
- forked histories merge into false consensus;
- a memorial has no maintenance or claimant state;
- evidence access creates automatic public reputation changes without transmission;
- optional language generation creates evidence or changes claim confidence.

# 18. Performance and Storage

Use content-addressed storage with deduplication for immutable record payloads.

Store structured metadata and bounded derived summaries separately.

Raw high-volume sensor streams may be:

- sampled;
- aggregated;
- retained only around significant events;
- stored in external archive packages;
- pruned under explicit policy while preserving hashes, summaries, and provenance.

## Final Rule

> **The archive runtime must preserve enough truth to make accountability possible, enough uncertainty to remain honest, and enough privacy to avoid turning memory into domination.**
