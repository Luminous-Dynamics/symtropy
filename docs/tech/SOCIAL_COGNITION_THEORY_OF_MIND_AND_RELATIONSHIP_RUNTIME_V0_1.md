---
title: Social Cognition, Theory of Mind, and Relationship Runtime
version: 0.1
status: implementation-spec
scope: bounded theory of mind, domain trust, norms, deception, attachment, coalition, grief, reconciliation
owner: AI/simulation/narrative
related:
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - ../vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
  - ../canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
---

# Social Cognition, Theory of Mind, and Relationship Runtime

## Purpose

Symtropy's advanced NPCs need more than memory and mood.

They need bounded models of:

- what others know;
- what others want;
- who is trusted in which domain;
- what a relationship permits;
- what a group expects;
- when someone is hiding information;
- how power changes apparent consent;
- how conflict can be repaired.

This is not unrestricted mind reading.

It is uncertain social inference grounded in observed behavior and testimony.

## Core Principle

```text
An NPC may model another person's perspective.
It may never inspect another person's private cognition directly.
```

# 1. Relationship State

Relationships are multidimensional.

```rust
struct RelationshipState {
    subject: AgentId,
    object: AgentId,
    familiarity: f32,
    affection: f32,
    respect: f32,
    fear: f32,
    resentment: f32,
    obligation: f32,
    dependency: f32,
    attraction: f32,
    ideological_alignment: f32,
    repairability: f32,
    last_meaningful_contact: SimTime,
}
```

No single approval score may replace this state.

## Domain Trust

Trust is contextual.

```rust
struct DomainTrust {
    technical: f32,
    medical: f32,
    political: f32,
    personal: f32,
    tactical: f32,
    ecological: f32,
    archival: f32,
}
```

A skilled engineer may be trusted technically and distrusted politically.

# 2. Social Beliefs

An agent may hold first-order beliefs:

```text
I believe the bridge is unsafe.
```

It may also hold bounded second-order beliefs:

```text
I believe Mara thinks the bridge is safe.
I believe Mara knows that I opposed the repair.
I believe Mara expects me to vote against her.
```

```rust
struct SocialBelief {
    holder: AgentId,
    modeled_agent: AgentId,
    proposition: Proposition,
    order: u8,
    confidence: f32,
    evidence_refs: Vec<EvidenceRef>,
    domain: SocialDomain,
}
```

Initial implementation limits belief order to two.

Higher-order recursive mind models are deferred.

# 3. Perspective Model

Each modeled agent has a compact perspective record.

```rust
struct PerspectiveModel {
    modeled_agent: AgentId,
    inferred_goals: Vec<GoalHypothesis>,
    inferred_values: ValueVector,
    known_information: Vec<ClaimRef>,
    suspected_information: Vec<ClaimRef>,
    expected_norms: Vec<NormId>,
    predicted_reactions: Vec<ReactionHypothesis>,
    confidence: f32,
}
```

The perspective model is lossy and may be wrong.

# 4. Social Prediction Error

NPCs predict how others will respond.

A mismatch updates:

- domain trust;
- inferred goals;
- relationship dimensions;
- norm expectations;
- deception suspicion;
- surprise salience.

Examples:

```text
A rival unexpectedly protects the NPC.
A trusted leader lies publicly.
A frightened worker refuses evacuation.
A machine follows a value rather than an order.
```

Repeated mismatch should matter more than one anomalous act.

# 5. Norms and Roles

Norms are local and historically situated.

```rust
struct Norm {
    norm_id: NormId,
    group_id: GroupId,
    trigger: NormTrigger,
    expected_behavior: BehaviorTemplate,
    protected_value: ValueId,
    enforcement: EnforcementPattern,
    legitimacy: f32,
    controversy: f32,
}
```

NPCs may:

- follow a norm;
- resent a norm;
- exploit a norm;
- publicly defend but privately violate a norm;
- misunderstand a norm;
- challenge a norm;
- attempt reform.

Roles create obligations but not total identities.

# 6. Deception, Concealment, and Privacy

NPCs may intentionally:

- withhold;
- evade;
- misdirect;
- lie;
- reveal selectively;
- maintain a confidence;
- protect another person;
- conceal shame;
- resist interrogation.

Deception requires:

- a claim the agent knows or believes false;
- a motive;
- an audience model;
- expected risk;
- a chosen speech act.

Generated language may render the deception, but the deception decision is structured.

The game must distinguish:

```text
false belief
uncertainty
memory error
social politeness
strategic concealment
deliberate lie
```

# 7. Power and Consent

Social cognition must account for power.

Relevant asymmetries include:

- employment;
- command;
- debt;
- medical dependency;
- life-support control;
- citizenship;
- age;
- guardianship;
- imprisonment;
- access to records;
- control over reconstitution;
- control over movement.

An apparent agreement under severe dependency is not automatically treated as free consent.

This affects:

- relationship appraisal;
- witness credibility;
- conflict interpretation;
- dialogue framing;
- institutional response.

# 8. Attachment

Attachment is not a romance meter.

It represents durable expectations of safety, proximity, care, and loss.

```rust
struct AttachmentState {
    target: AgentId,
    security: f32,
    proximity_need: f32,
    separation_distress: f32,
    caregiving_commitment: f32,
    abandonment_expectation: f32,
}
```

Attachment may exist among:

- family;
- friends;
- companions;
- crews;
- humans and robots;
- caretakers and animals;
- symbiotic alien partners;
- communities and places.

# 9. Conflict and Repair

Conflict state tracks:

```rust
struct InterpersonalConflict {
    participants: Vec<AgentId>,
    grievances: Vec<Grievance>,
    publicness: f32,
    escalation: ConflictEscalation,
    desired_repair: Vec<RepairNeed>,
    blocked_by: Vec<RepairBarrier>,
}
```

Repair may require:

- acknowledgment;
- changed behavior;
- restitution;
- truth;
- safety;
- time;
- public correction;
- private apology;
- restored autonomy;
- separation.

Apology without changed conditions does not necessarily reduce resentment.

# 10. Grief and Absence

NPCs distinguish:

- temporary absence;
- uncertain disappearance;
- verified death;
- reconstitution with continuity;
- reconstitution with source-chain loss;
- permanent loss;
- institutional erasure.

Grief affects attention, habits, projects, social openness, memory retrieval, and identity.

It must not become a universal sadness debuff.

# 11. Groups and Coalitions

NPCs form groups through:

- shared goals;
- shared wounds;
- material dependence;
- friendship;
- ideology;
- profession;
- kinship;
- geography;
- threat;
- celebration;
- resentment.

Coalition state includes:

- membership confidence;
- loyalty;
- coordination;
- internal trust;
- leadership legitimacy;
- fracture pressure;
- external threat;
- shared project.

Coalitions do not inherit a single group mind unless the fiction defines one.

# 12. Rumor and Reputation

Rumor packets retain:

- original claim;
- source chain;
- transformations;
- emotional charge;
- audience;
- confidence;
- incentives;
- contradiction evidence.

NPCs should not magically merge all reputation reports.

They interpret them according to trust, faction, experience, and interest.

# 13. Dialogue Integration

The social runtime supplies:

- audience model;
- disclosure policy;
- relationship stance;
- norm context;
- power context;
- expected reaction;
- public/private distinction;
- truthfulness class.

A renderer may vary phrasing but not the underlying social act.

# 14. Nonhuman Social Cognition

Nonhuman agents may model:

- habitat boundaries;
- signal reciprocity;
- pattern integrity;
- collective continuity;
- contact permission;
- timescale commitments.

Do not force human friendship, deception, or attachment categories where they do not fit.

# 15. Performance and LOD

Full social models are reserved for named agents.

Reduced agents retain:

- top relationships;
- current conflict;
- group membership;
- domain trust summaries;
- last meaningful contact.

Ambient populations use aggregate social fields.

# 16. Acceptance Tests

Required scenarios:

1. technical trust diverges from political trust;
2. an NPC updates a false belief without losing all confidence in the source;
3. a trusted ally's betrayal changes second-order beliefs;
4. a coerced agreement is recognized as power-laden;
5. a lie differs from a mistaken statement;
6. reconciliation requires the requested form of repair;
7. grief changes routine without erasing personality;
8. rumor propagation preserves source transformations;
9. a coalition fractures through internal contradiction;
10. a nonhuman boundary is modeled without human emotion labels.

## Final Rule

```text
The purpose of theory of mind is not to let NPCs know everything.

It is to let them be wrong about one another
in ways that can become trust, tragedy, forgiveness, or history.
```
