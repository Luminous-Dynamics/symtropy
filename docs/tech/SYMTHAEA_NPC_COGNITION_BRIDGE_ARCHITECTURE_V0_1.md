---
title: Symthaea NPC Cognition Bridge Architecture
version: 0.1
status: implementation-spec
scope: crate boundaries, cognition transactions, HDC/CfC integration, grounding, determinism, performance, observability
owner: AI/simulation/engineering
related:
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - ../ops/NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
---

# Symthaea NPC Cognition Bridge Architecture

## Purpose

This document defines the technical boundary between Symtropy's authoritative NPC simulation and selected Symthaea cognitive components.

The bridge must make cognition:

```text
bounded
versioned
grounded
replayable
observable
optional
budgeted
```

## Architecture Summary

```text
World and NPC Runtime
  → Perception Compiler
  → Grounded Cognitive Snapshot
  → Symthaea NPC Bridge
      → HDC Encoder / Retriever
      → Temporal Appraisal State
      → Prediction and Surprise
      → Bounded Deliberation
      → Dialogue Intent
  → Proposal Validator
  → Authoritative Action Runtime
  → Outcome Event
  → Memory and Belief Update
```

Symthaea does not receive raw unrestricted access to the ECS or database.

# 1. Recommended Crate Boundary

```text
symtropy-npc-core
symtropy-npc-memory
symtropy-npc-social
symtropy-npc-dialogue
symtropy-npc-observability
symthaea-npc-bridge
```

## `symtropy-npc-core`

Owns:

- authoritative agent state;
- needs and viability;
- roles and obligations;
- available actions;
- plan lifecycle;
- action validation;
- simulation LOD;
- deterministic fallback policies.

## `symtropy-npc-memory`

Owns:

- memory identifiers;
- episodic records;
- semantic beliefs;
- relationship episodes;
- provenance;
- confidence;
- retention and forgetting;
- migration and persistence.

## `symtropy-npc-social`

Owns:

- relationships;
- norms;
- domain trust;
- second-order beliefs;
- group membership;
- reputation packets;
- attachment, grief, conflict, and reconciliation state.

## `symthaea-npc-bridge`

Owns only adapters to approved Symthaea primitives.

It must be feature-gated and replaceable.

# 2. Grounded Snapshot

The bridge receives a finite snapshot.

```rust
struct AgentCognitionSnapshot {
    schema_version: u32,
    agent: AgentIdentityView,
    embodiment: EmbodimentView,
    perceived_entities: Vec<PerceivedEntity>,
    local_conditions: LocalConditionView,
    current_needs: NeedVector,
    current_appraisal: AppraisalVector,
    obligations: Vec<ObligationView>,
    active_plan: Option<PlanView>,
    relationship_context: Vec<RelationshipView>,
    accessible_beliefs: Vec<BeliefView>,
    retrieved_memories: Vec<MemoryView>,
    available_action_templates: Vec<ActionTemplateId>,
    dialogue_permissions: DialoguePermissionSet,
    content_version: ContentVersion,
    deterministic_seed: u64,
}
```

The snapshot contains only information the agent is allowed to know.

A hidden object absent from perception and memory must not appear merely because it exists in ECS state.

# 3. Perception Compiler

The perception compiler converts game signals into agent-relative observations.

Each observation records:

```rust
struct Percept {
    percept_id: PerceptId,
    source: PerceptSource,
    subject: EntityRef,
    predicate: PredicateId,
    value: PerceptValue,
    confidence: f32,
    observed_at: SimTime,
    spatial_context: Option<SpatialRef>,
    evidence_refs: Vec<EvidenceRef>,
}
```

Source classes include:

- direct sensory observation;
- instrument observation;
- trusted testimony;
- rumor;
- archive record;
- inference;
- prediction;
- generated hypothesis.

The bridge must preserve these distinctions.

# 4. HDC Encoding

HDC represents compositional situations, not canonical truth.

Recommended binding grammar:

```text
agent ⊗ role
person ⊗ relationship
event ⊗ place ⊗ time-band
claim ⊗ source-class ⊗ confidence-band
need ⊗ urgency
obligation ⊗ beneficiary
action ⊗ expected-outcome
```

Use separate namespaces for:

- entities;
- roles;
- actions;
- places;
- values;
- emotions;
- evidence classes;
- time bands;
- cultural concepts.

## Retrieval Contract

Retrieval returns candidate memory IDs and similarity scores.

It does not return reconstructed facts without database lookup.

```rust
struct RetrievalCandidate {
    memory_id: MemoryId,
    similarity: f32,
    cue_contribution: Vec<CueContribution>,
}
```

# 5. Temporal Appraisal

CfC/LTC-style dynamics may maintain continuous state:

```rust
struct TemporalAppraisalState {
    stress: f32,
    arousal: f32,
    fatigue: f32,
    confidence: f32,
    vigilance: f32,
    grief_activation: f32,
    social_openness: f32,
    goal_persistence: f32,
    recovery_rate: f32,
}
```

State evolves from authoritative events and elapsed simulation time.

It must not be driven by generated text.

## Update Inputs

- physical harm;
- unmet needs;
- safety;
- social support;
- humiliation;
- achievement;
- surprise;
- sleep;
- chronic overload;
- grief cues;
- role success or failure.

All inputs must be inspectable.

# 6. Prediction and Surprise

The bridge may predict:

- expected local condition;
- expected person response;
- expected plan result;
- expected resource availability;
- expected norm compliance.

A mismatch produces a typed prediction error.

```rust
enum PredictionErrorKind {
    Sensory,
    Social,
    Instrumental,
    Normative,
    Relational,
    Identity,
}
```

Prediction error may trigger review. It may not directly rewrite beliefs.

# 7. Belief Update Proposals

```rust
struct BeliefUpdateProposal {
    belief_id: Option<BeliefId>,
    proposition: Proposition,
    prior_confidence: f32,
    proposed_confidence: f32,
    evidence_refs: Vec<EvidenceRef>,
    contradiction_refs: Vec<BeliefId>,
    update_reason: BeliefUpdateReason,
}
```

The validator applies:

- source reliability;
- cultural prior;
- domain expertise;
- contradiction;
- recency;
- emotional distortion;
- manipulation risk;
- evidence accessibility.

# 8. Candidate Intentions

The bridge selects only from authored action templates.

```rust
struct IntentionProposal {
    action_template: ActionTemplateId,
    target_refs: Vec<EntityRef>,
    expected_need_effect: NeedVectorDelta,
    expected_value_alignment: ValueVector,
    uncertainty: f32,
    urgency: f32,
    memory_refs: Vec<MemoryId>,
    explanation_tags: Vec<ReasonTag>,
}
```

The action runtime remains authoritative.

# 9. Dialogue Frame

Generated language begins from a validated frame.

```rust
struct DialogueFrame {
    speech_act: SpeechAct,
    topic_refs: Vec<TopicRef>,
    allowed_claims: Vec<ClaimRef>,
    prohibited_claim_classes: ClaimMask,
    emotional_tone: ToneVector,
    relationship_stance: RelationshipStance,
    uncertainty_markers: Vec<UncertaintyMarker>,
    disclosure_policy: DisclosurePolicy,
    vocabulary_profile: VocabularyProfileId,
    maximum_length: u16,
}
```

Broca may render this frame.

The validator rejects output that introduces claims outside `allowed_claims`.

# 10. Cognition Scheduling

## Event Queue

Deep cognition requests are prioritized by:

```text
safety
social consequence
plan failure
relationship rupture
novelty
player proximity
story relevance
elapsed reflection debt
```

## Budgets

Each request declares:

- maximum wall time;
- maximum iterations;
- maximum retrieved memories;
- maximum candidate intentions;
- maximum dialogue length;
- fallback deadline.

No request is unbounded.

# 11. Simulation LOD

## Full

Used for active Tier 2–3 agents in consequential scenes.

## Reduced

Updates appraisal, plan, and selected memories without dialogue rendering.

## Summary

Simulates schedule blocks, projects, relationships, and major outcomes.

## Dormant

Preserves state and applies only worldline events that explicitly affect the agent.

Transitions must preserve:

- current obligation;
- active project;
- critical relationship state;
- unresolved grief;
- dangerous misinformation;
- legal or civic status.

# 12. Multiplayer and Network Authority

Only accepted action requests are replicated as game events.

Cognitive traces remain local to the authoritative shard unless:

- needed for moderation;
- explicitly included in a public witness;
- required for deterministic replay;
- exported with informed operator consent.

Rendered dialogue is presentation state. Its underlying speech act and claims are durable enough for replay.

# 13. Failure Modes

The bridge must detect:

- unknown entity references;
- inaccessible facts;
- invalid action templates;
- memory IDs outside the agent;
- confidence overflow;
- cyclic belief updates;
- repeated dialogue;
- runaway reflection;
- culturally impossible vocabulary;
- impossible emotional transitions;
- proposal starvation;
- stale snapshots.

Every failure has a deterministic fallback.

# 14. Observability

Developer tooling must display:

- cognition trigger;
- snapshot hash;
- retrieved memories;
- temporal state;
- predictions;
- prediction errors;
- candidate intentions;
- accepted and rejected proposals;
- dialogue claims;
- total cost;
- degradation path.

A designer should be able to answer:

```text
Why did this NPC notice that?
Why did they remember this event?
Why did they distrust the testimony?
Why did they choose this action?
What would they have done without Symthaea?
```

# 15. Seedworks Performance Target

Initial benchmark:

```text
4 Tier-2 agents
8 Tier-1 agents
64 ambient population aggregates
10 Hz conventional local AI
1–2 Hz Tier-1 appraisal
event-driven Tier-2 cognition
no more than 2 simultaneous deep requests
bounded 16-memory retrieval
bounded 5 candidate intentions
```

Performance is measured with and without language rendering.

# 16. Verification

Required tests:

- inaccessible-fact rejection;
- deterministic proposal replay;
- action-authority separation;
- HDC retrieval relevance;
- appraisal decay;
- contradiction handling;
- LOD transition continuity;
- outage fallback;
- multiplayer replication boundary;
- dialogue claim validation;
- memory provenance preservation;
- ablation equivalence for disabled components.

## Final Rule

```text
The bridge is successful when Symthaea enriches interpretation
without becoming a second, hidden game engine.
```
