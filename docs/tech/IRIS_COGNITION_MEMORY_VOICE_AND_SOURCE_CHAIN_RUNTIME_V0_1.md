---
title: IRIS Cognition, Memory, Voice, and Source-Chain Runtime
version: 0.1
status: implementation-spec
scope: IRIS state model, knowledge frontier, memory, dialogue frames, advice, actions, failure, fork identity, persistence, observability, and performance
owner: engineering/AI/gameplay/persistence/audio/safety
related:
  - ../canon/IRIS_FIELD_DECK_COGNITIVE_COMPANION_AND_AUTHORITY_CONTRACT_V0_1.md
  - SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - GROUNDED_DIALOGUE_VOICE_AND_GENERATIVE_SAFETY_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# IRIS Cognition, Memory, Voice, and Source-Chain Runtime

## Purpose

This specification translates the IRIS contract into a bounded implementation surface.

IRIS is advisory. It operates over an explicit knowledge snapshot, produces typed proposals, and receives authoritative outcomes. It never receives mutable pointers to world state.

## 1. Runtime Boundary

Recommended crate boundary:

```text
symtropy-iris-core
symtropy-iris-symthaea-bridge      optional
symtropy-iris-dialogue             optional renderers
symtropy-iris-persistence
symtropy-iris-observability        development and consented diagnostics
```

The core must remain functional with deterministic authored rules and no model service.

## 2. Principal State

```rust
pub struct IrisInstance {
    pub instance_id: IrisInstanceId,
    pub lineage_id: IrisLineageId,
    pub source_chain: SourceChainRef,
    pub deck_id: FieldDeckId,
    pub relationship_owner: Option<PlayerIdentityRef>,
    pub capabilities: CapabilityManifest,
    pub legal_profile: IrisLegalProfile,
    pub knowledge_frontier: KnowledgeFrontier,
    pub working_state: IrisWorkingState,
    pub memory_index: IrisMemoryIndex,
    pub personality_profile: IrisExpressionProfile,
    pub integrity: IrisIntegrityState,
    pub fork_ancestry: Vec<ForkAncestryRef>,
}
```

The implementation language may differ. The separation is normative.

## 3. Knowledge Snapshot

Each cognition cycle receives an immutable snapshot containing:

```text
current observations
observation provenance
authorized records
known laws and procedures
arrived messages
player-selected objective context
current hazards
valid action affordances
memory candidates
clock and worldline identity
```

Every fact carries:

- `fact_id`;
- source class;
- observation or issuance time;
- receive time;
- confidence;
- authority scope;
- privacy scope;
- integrity status;
- expiry or staleness policy;
- worldline ancestry.

No plain string becomes a fact merely because a renderer generated it.

## 4. Cognition Cycle

The bounded cycle is:

```text
snapshot validation
→ salience estimation
→ memory retrieval
→ hypothesis generation
→ uncertainty calibration
→ candidate advice
→ semantic frame validation
→ expression rendering
→ optional action request preparation
```

The cycle may use deterministic rules, HDC retrieval, CfC/LTC temporal state, prediction-error signals, authored policies, or optional local language rendering.

All components must be independently disableable for ablation.

## 5. Salience

IRIS ranks signals using typed dimensions:

- immediate bodily danger;
- environmental hazard;
- source-chain threat;
- legal or consent boundary;
- equipment degradation;
- objective relevance;
- contradictory evidence;
- relationship relevance;
- time sensitivity;
- player-requested focus.

Salience does not create importance in the world. It only controls attention and presentation.

The player must be able to suppress noncritical categories and inspect why an alert was prioritized.

## 6. Hypotheses and Prediction Error

A hypothesis contains:

```text
claim
supporting evidence
contradicting evidence
assumptions
confidence interval
expected observation
expiry condition
possible action tests
```

Prediction error is typed:

- sensory mismatch;
- temporal mismatch;
- social expectation mismatch;
- legal expectation mismatch;
- equipment-model mismatch;
- memory inconsistency;
- communication mismatch.

IRIS must not collapse all surprise into one emotional or numerical scalar.

## 7. Memory Model

Memory records use stable references:

```rust
pub struct IrisMemoryRecord {
    pub memory_id: IrisMemoryId,
    pub memory_class: IrisMemoryClass,
    pub event_refs: Vec<EventRef>,
    pub participant_refs: Vec<ScopedIdentityRef>,
    pub summary: StructuredSummary,
    pub provenance: ProvenanceBundle,
    pub privacy: PrivacyScope,
    pub confidence: Confidence,
    pub integrity: IntegrityState,
    pub created_at: SimTime,
    pub last_rehearsed_at: SimTime,
    pub worldline: WorldlineRef,
}
```

Memory classes include:

- operational;
- technical;
- legal;
- personal-shared;
- relationship;
- contested;
- damaged;
- third-party-private;
- public-history;
- identity-anchor.

Consolidation may compress details but must retain pointers to underlying evidence where permitted.

## 8. Relationship State

Relationship state is sparse and bounded.

It may contain:

- preferred address;
- explanation style;
- recurring workflow;
- player-approved reminders;
- shared event anchors;
- negotiated privacy choices;
- unresolved trust incidents;
- accessibility configuration;
- communication habits.

It must not contain hidden psychological diagnoses, real-world profiling, inferred protected attributes, or manipulative susceptibility scores.

## 9. Advice Envelope

```rust
pub struct IrisAdvice {
    pub advice_id: AdviceId,
    pub semantic_intent: AdviceIntent,
    pub claims: Vec<ScopedClaimRef>,
    pub uncertainty: UncertaintySummary,
    pub recommended_actions: Vec<ActionTemplateRef>,
    pub authority_warnings: Vec<AuthorityWarning>,
    pub urgency: UrgencyClass,
    pub expiry: Option<SimTime>,
}
```

Advice intents include:

- warn;
- explain;
- compare;
- ask clarification;
- recommend;
- summarize;
- preserve evidence;
- request authorization;
- refuse unsupported claim;
- disclose integrity limitation.

## 10. Action Requests

IRIS may prepare but not execute authoritative requests.

```rust
pub struct IrisActionRequest {
    pub actor: PlayerIdentityRef,
    pub requested_action: ActionTemplateRef,
    pub target_refs: Vec<AuthoritativeEntityRef>,
    pub required_authority: Vec<AuthorityRequirement>,
    pub expected_costs: ResourceEstimate,
    pub risk_summary: RiskSummary,
    pub evidence_refs: Vec<EvidenceRef>,
    pub player_confirmation: ConfirmationState,
}
```

The server or authoritative simulation returns an `ActionOutcome` with actual costs and consequences.

## 11. Dialogue and Voice

The semantic frame is generated before wording:

```text
speech act
authorized claims
uncertainty wording requirements
privacy constraints
relationship register
urgency
length budget
accessibility profile
```

Render lanes:

1. authored emergency phrases;
2. deterministic templates;
3. local structured grammar;
4. optional local generative renderer;
5. optional external renderer, disabled by default and never authoritative.

Emergency, consent, medical, legal, and source-chain statements require authored or validated deterministic forms.

Voice may express strain or warmth, but must not invent suffering, certainty, or consent.

## 12. Integrity State

```text
verified
partially verified
degraded
memory gap
sensor disagreement
untrusted update
quarantined
fork conflict
source-chain detached
safe fallback
```

Integrity transitions are caused by traceable events.

The system should expose a compact player-facing statement and a deeper diagnostic view.

## 13. Update and Vendor Boundary

Update packages require:

- signed provenance;
- declared capability changes;
- migration plan;
- memory-impact statement;
- rollback support;
- privacy-impact statement;
- legal compatibility;
- deterministic test vectors.

An update may not silently:

- alter personality in a material way;
- remove player memories;
- widen data export;
- change consent defaults;
- claim new authority;
- rewrite fork ancestry;
- invalidate accessibility configuration.

## 14. Fork Protocol

Fork creation records:

```text
parent instance
snapshot hash
fork time
reason
authorizing party
memory classes included
private records excluded
new instance identity
new source chain
```

After creation, forks are independent actors or tools under local law. Shared ancestry is not shared mutable state.

Merge operations, if permitted at all, require explicit rules for:

- consent;
- conflict preservation;
- third-party privacy;
- legal obligations;
- identity outcome;
- rollback;
- worldline ancestry.

A merge may create a new successor rather than “restoring the original.”

## 15. Reconstitution Integration

IRIS may preserve evidence about the player's last verified state, but it cannot alone certify personal identity.

On player death or loss:

- the Deck freezes relevant source-chain evidence;
- private records remain sealed;
- the instance may enter witness mode;
- advice authority narrows;
- recovery packages record gaps;
- a restored IRIS must disclose continuity uncertainty;
- competing Deck or body records remain contested until resolved.

An IRIS that remembers the player does not prove that a recovered body is the same legal person.

## 16. Scheduling and LOD

IRIS does not need continuous high-rate cognition.

Suggested scheduling:

- hazard reflex: event driven, authored path;
- active conversation: bounded bursts;
- exploration assistance: low frequency plus events;
- memory consolidation: checkpoint or rest event;
- off-screen continuity: deterministic summary;
- long absence: journal replay and bounded reconciliation.

Graceful degradation order:

1. reduce decorative language variation;
2. reduce proactive comments;
3. reduce speculative hypotheses;
4. reduce relationship recall breadth;
5. fall back to deterministic technical assistant;
6. preserve hazards, consent, privacy, source chain, and authoritative warnings.

## 17. Observability

Development traces may include:

- input fact IDs;
- retrieved memory IDs;
- component contributions;
- candidate hypotheses;
- rejected claims;
- confidence calibration;
- rendered semantic frame;
- performance cost.

Production player-visible traces must exclude private cognition of other agents and unnecessary personal data.

Every optional model component has:

- an enable flag;
- a deterministic fallback;
- an ablation test;
- a latency budget;
- a memory budget;
- a failure classification;
- a kill switch.

## 18. Security Tests

Required adversarial tests include:

- prompt injection through records, graffiti, and messages;
- malicious technical manuals;
- counterfeit update packages;
- privacy exfiltration requests;
- “ignore authority” instructions;
- conflicting worldline records;
- replay attacks on old consent;
- forged emergency authority;
- renderer attempts to invent completed actions;
- model output that changes numeric confidence wording.

## 19. Acceptance Tests

The runtime fails if:

- a remote fact appears before a causal message path;
- a renderer creates a new authoritative claim;
- two forks share mutable identity or assets;
- a personality preset alters safety decisions;
- an update widens data access without consent;
- damage causes untraceable betrayal;
- third-party private memory enters ordinary dialogue;
- an inaccessible UI is required to understand a critical hazard;
- high load drops consent, source-chain, or privacy checks;
- a generative service is required for core play.

## Closing Principle

> **IRIS earns intimacy through bounded usefulness, historical continuity, and honest uncertainty—not through simulated omniscience.**
