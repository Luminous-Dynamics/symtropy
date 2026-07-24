---
title: NPC Memory Consolidation, Learning, and Worldline Continuity Runtime
version: 0.1
status: implementation-spec
scope: episodic memory, semantic consolidation, forgetting, contradiction, provenance, persistence, death, migration
owner: AI/simulation/persistence/narrative
related:
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
  - ../canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
  - ../ops/FOUR_NPC_BENCHMARK_HOUSEHOLD_PROTOCOL_V0_1.md
---

# NPC Memory Consolidation, Learning, and Worldline Continuity Runtime

## Purpose

NPCs should remember enough to become historically situated without retaining every frame forever.

Memory must be:

```text
selective
fallible
provenanced
compressible
migratable
private where appropriate
```

## Core Principle

```text
Memory is not a transcript.
Memory is a changing relationship between evidence, meaning, and identity.
```

# 1. Memory Classes

## Working Memory

Short-lived active context.

Examples:

- current conversation;
- immediate threat;
- active task;
- recently perceived objects;
- unresolved question.

## Episodic Memory

Bounded records of experienced or received events.

```rust
struct EpisodicMemory {
    memory_id: MemoryId,
    agent_id: AgentId,
    event_ref: Option<EventRef>,
    encoded_at: SimTime,
    event_time: TimeRange,
    participants: Vec<EntityRef>,
    place: Option<PlaceRef>,
    summary: PropositionSet,
    sensory_tags: Vec<SensoryTag>,
    appraisal: AppraisalVector,
    source_class: SourceClass,
    confidence: f32,
    privacy: MemoryPrivacy,
    salience: f32,
    rehearsal_count: u16,
    contradiction_refs: Vec<MemoryId>,
}
```

## Semantic Memory

Compressed beliefs, concepts, and learned regularities.

Examples:

- “Tomas repairs quickly but hides uncertainty.”
- “The northern road floods after two days of heavy rain.”
- “Continuance officers usually honor machine safety evidence.”
- “This species avoids high-frequency sonar.”

## Procedural Memory

Learned action competence, habits, and routines.

## Relationship Memory

Episodes specifically tied to interpersonal interpretation.

## Identity Memory

Self-narratives, formative wounds, commitments, and role continuity.

## Institutional Memory

Shared records available through group, archive, profession, machine network, or Chronicle.

# 2. Memory Encoding

An event becomes a candidate memory when at least one is true:

- strong appraisal;
- high novelty;
- protected value affected;
- relationship changed;
- obligation created or discharged;
- prediction failed;
- public consequence;
- repeated pattern;
- deliberate rehearsal;
- identity relevance;
- authored importance tag.

Not every event is stored.

# 3. HDC Memory Index

HDC indexes memories by compositional cues.

It must not replace structured storage.

Each memory receives:

- entity bindings;
- role bindings;
- place;
- event type;
- appraisal;
- source class;
- time band;
- value relevance;
- relationship domain.

Retrieval returns IDs, then structured records are loaded.

# 4. Consolidation

Consolidation occurs during:

- rest;
- sleep;
- travel;
- low-load off-screen periods;
- explicit reflection;
- ritual;
- debrief;
- therapy;
- machine maintenance;
- archive synchronization.

Consolidation may:

- merge repeated episodes;
- form a semantic belief;
- strengthen a habit;
- revise a relationship;
- lower sensory detail;
- retain contradiction;
- create a self-narrative link.

## Deterministic Consolidation

Given the same:

- memory set;
- agent profile;
- model version;
- seed;
- worldline state;

the semantic result must be replayable.

# 5. Forgetting

Forgetting prevents infinite storage and creates human texture.

A memory may lose:

- sensory detail;
- exact time;
- peripheral participants;
- wording;
- confidence;
- causal certainty.

It should preserve longer:

- severe harm;
- strong attachment;
- public identity events;
- repeated patterns;
- unresolved obligations;
- formative experiences;
- player-caused relationship changes;
- death and reconstitution events.

Forgetting must not erase required legal evidence. Personal memory and public record are separate.

# 6. Contradiction

Memories and beliefs may conflict.

The system stores contradiction rather than silently overwriting.

```rust
struct ContradictionSet {
    proposition: Proposition,
    supporting_memories: Vec<MemoryId>,
    opposing_memories: Vec<MemoryId>,
    unresolved_confidence: f32,
    review_trigger: ReviewTrigger,
}
```

NPCs may respond by:

- uncertainty;
- source reassessment;
- reinterpretation;
- denial;
- compartmentalization;
- investigation;
- public accusation;
- belief revision.

# 7. Learning

Learning changes:

- expectations;
- action preferences;
- domain trust;
- skill models;
- danger estimates;
- social predictions;
- value interpretations;
- plan choice.

It must not create capabilities unsupported by embodiment, tools, teaching, or practice.

A medic cannot learn reactor engineering merely by observing one repair.

# 8. Memory Error

Memory errors are typed.

```text
omission
time compression
source confusion
participant confusion
causal simplification
emotion-biased emphasis
rumor contamination
identity-protective distortion
Null contamination
archive mismatch
```

Errors should arise from causes, not random plot convenience.

# 9. Private Memory

Privacy classes:

```text
private
relationship-shared
household-shared
professional
institutional
public
Chronicle
```

NPC private memories are not player collectibles by default.

Access may require:

- voluntary disclosure;
- trusted relationship;
- legal process;
- medical consent;
- archive authority;
- posthumous directive;
- machine testimony protocol.

# 10. Death and Reconstitution

For reconstitutable persons, memory continuity depends on:

- body survival;
- local cognitive state;
- Field Deck or source-chain state;
- last valid sync;
- witness records;
- Chronicle entries;
- backup policy;
- consent directives.

Possible outcomes:

```text
continuous recovery
partial episodic loss
semantic continuity with missing recent events
relationship uncertainty
source-chain dispute
public record intact, personal memory missing
contaminated recovery
permanent death
```

NPCs must react to the returned person according to what continuity is actually verified.

# 11. Worldline Migration

NPC memory persistence uses:

- stable agent IDs;
- stable memory IDs;
- schema versions;
- content hashes;
- provenance references;
- privacy flags;
- migration transforms;
- unresolved-content quarantine.

A worldline fork copies eligible memories up to the fork boundary.

Post-fork experiences diverge.

Confluence does not automatically merge private memories.

# 12. Off-Screen Continuity

When an NPC is not deeply simulated, preserve:

- current project;
- strongest needs;
- major relationships;
- unresolved conflict;
- active obligation;
- critical belief;
- current location class;
- recent major episode;
- scheduled transition.

Summary simulation may create new memories only for explicit major outcomes.

# 13. Chronicle Relationship

Chronicle is not NPC memory.

A Chronicle record may:

- confirm;
- contradict;
- contextualize;
- omit;
- publicly expose;
- legally bind.

NPCs may distrust or misunderstand Chronicle records.

# 14. Memory Budgets

Seedworks target per Tier-2 NPC:

```text
working items:             8–24
high-detail episodes:      64–128
compressed episodes:       256–512
semantic beliefs:          128–256
relationship episodes:     32–96 per major relationship
identity anchors:          8–24
```

Budgets are tuned through ablation and longitudinal simulation.

# 15. Developer Tools

Required tools:

- memory timeline;
- retrieval cue inspector;
- contradiction graph;
- semantic consolidation diff;
- privacy viewer;
- source-chain comparison;
- pre/post migration diff;
- forgetting preview;
- player-history trace;
- memory contamination detector.

# 16. Acceptance Tests

1. repeated episodes consolidate into a belief;
2. one contradictory event creates uncertainty rather than instant reversal;
3. a private memory does not appear in public dialogue;
4. forgetting removes detail but preserves formative meaning;
5. off-screen simulation preserves unresolved obligations;
6. worldline fork produces shared pre-fork and divergent post-fork memory;
7. partial reconstitution produces appropriate continuity uncertainty;
8. Chronicle contradiction remains distinct from personal recollection;
9. HDC retrieval returns relevant IDs without inventing facts;
10. schema migration preserves provenance and privacy.

## Final Rule

```text
An NPC becomes historical not because it remembers everything,
but because what it remembers can change what it expects,
what it fears, whom it trusts, and what future it tries to build.
```
