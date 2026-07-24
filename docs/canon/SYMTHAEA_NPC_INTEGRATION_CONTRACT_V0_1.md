---
title: Symthaea NPC Integration Contract
version: 0.1
status: canonical-draft
scope: bounded Symthaea use in NPC cognition, authority boundaries, simulation tiers, promotion gates
owner: AI/simulation/narrative/engineering
related:
  - ../tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - ../tech/SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - ../tech/SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - ../tech/NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
  - ../ops/FOUR_NPC_BENCHMARK_HOUSEHOLD_PROTOCOL_V0_1.md
  - ../ops/NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
---

# Symthaea NPC Integration Contract

## Owned Question

**Which parts of Symthaea may shape NPC attention, memory, prediction, deliberation, and expression without allowing an opaque cognitive subsystem to own game truth, player-facing authority, or simulation correctness?**

## Core Thesis

Symthaea should not replace Symtropy's NPC runtime.

It should deepen selected agents through a bounded cognition service.

```text
Symtropy owns:
  bodies
  perception sources
  resources
  inventories
  relationships as durable game state
  laws and permissions
  movement and combat
  devices and construction
  Chronicle events
  worldline truth

Symthaea may propose:
  salience
  retrieved memories
  predictions
  surprise
  appraisal
  belief updates
  candidate intentions
  dialogue intent
```

The game must remain complete, deterministic, debuggable, and playable when Symthaea is disabled.

## Prime Directive

```text
Cognition proposes.
The game validates.
The world decides.
The agent remembers.
```

No Symthaea subsystem may directly:

- move an entity;
- transfer an item;
- alter a device;
- issue damage;
- create a law;
- cast a vote;
- modify a relationship score;
- establish a fact;
- write a Chronicle record;
- grant a credential;
- declare a quest complete.

Every consequential effect must pass through an authoritative Symtropy action contract.

# 1. Why Integrate Symthaea

The integration exists to improve experiences that conventional behavior trees and dialogue state machines handle poorly:

- remembering long, irregular histories;
- distinguishing direct evidence from rumor;
- carrying emotional and attentional momentum through time;
- noticing prediction failure;
- forming durable interpretations of the player;
- choosing among conflicting obligations;
- revising beliefs after contradiction;
- expressing grounded thoughts in individual language;
- maintaining continuity through absence, migration, injury, and loss.

It does not exist to produce endless conversation or advertise simulated consciousness.

# 2. Integration Tiers

## Tier 0 — Conventional Ambient Life

Population-scale agents use:

- schedules;
- zone occupancy;
- need pressure;
- reaction tags;
- crowd and emergency policies;
- compact social aggregates.

No Symthaea instance is required.

Target scale:

```text
hundreds to thousands per region in aggregate
```

## Tier 1 — Situated Cognitive Agents

Workers, traders, soldiers, animals, drones, and recurring minor characters may use:

- compact HDC situation encoding;
- short episodic memory;
- bounded appraisal state;
- utility or active-inference-lite arbitration;
- authored speech acts.

Target scale:

```text
dozens to low hundreds active
```

## Tier 2 — Symthaea Citizens

Named citizens may use:

- persistent HDC memory;
- CfC-style temporal appraisal;
- prediction-error learning;
- domain-specific beliefs;
- relationship models;
- bounded planning;
- grounded dialogue frames;
- off-screen consolidation.

Target scale:

```text
8–30 per major active region
```

## Tier 3 — Hero and Institutional Minds

Rare companions, leaders, machine persons, archive courts, alien envoys, or ecological intelligences may receive:

- deeper event-driven deliberation;
- explicit second-order social beliefs;
- long-horizon project models;
- richer memory consolidation;
- institution or collective-body interfaces;
- carefully authored cognitive constraints.

Target scale:

```text
0–6 deeply active in a local dramatic scene
```

Tier 3 is not "smarter rights." It is a production and simulation budget category.

# 3. Approved Symthaea Capabilities

## 3.1 HDC Representation

Approved for:

- concept binding;
- person-place-event association;
- fuzzy retrieval;
- role and context representation;
- compressed situation signatures;
- novelty and similarity estimates.

HDC vectors are internal representations, not authoritative facts.

## 3.2 Continuous-Time Dynamics

CfC/LTC-style state is approved for:

- stress;
- arousal;
- fatigue;
- confidence;
- attentional persistence;
- emotional recovery;
- anticipation;
- behavioral momentum.

Continuous state may influence proposal ranking, but not bypass action validation.

## 3.3 Predictive Error

Approved for:

- surprise detection;
- belief review triggers;
- plan invalidation;
- attention shifts;
- learning-rate modulation;
- memory salience.

Prediction error must not automatically mean threat, guilt, falsehood, or moral failure.

## 3.4 Episodic and Semantic Memory

Approved for:

- event storage;
- relationship episodes;
- compression into beliefs and habits;
- retrieval by current context;
- contradiction tracking;
- forgetting and uncertainty.

Memory must retain provenance and confidence.

## 3.5 Grounded Language Rendering

Broca or another renderer may transform a validated dialogue frame into speech.

It may not invent:

- unknown facts;
- unowned items;
- nonexistent relationships;
- unrecorded laws;
- completed actions;
- inaccessible private information;
- hidden system state;
- new canonical lore.

# 4. Cognitive Transaction

Every deep cognition step uses a bounded transaction.

```rust
struct CognitiveRequest {
    request_id: CognitiveRequestId,
    agent_id: AgentId,
    worldline_id: WorldlineId,
    simulation_tick: u64,
    trigger: CognitionTrigger,
    snapshot_hash: ContentHash,
    budget: CognitionBudget,
    permitted_outputs: OutputMask,
}

struct CognitiveProposal {
    request_id: CognitiveRequestId,
    retrieved_memory_ids: Vec<MemoryId>,
    salience_updates: Vec<SalienceProposal>,
    belief_updates: Vec<BeliefUpdateProposal>,
    candidate_intentions: Vec<IntentionProposal>,
    dialogue_frame: Option<DialogueFrame>,
    uncertainty: ConfidenceInterval,
    trace_hash: ContentHash,
}
```

The authoritative NPC runtime may accept, reject, clamp, or defer each proposal.

# 5. Triggered Cognition

Deep cognition is event-driven.

Valid triggers include:

- expectation violated;
- direct social interaction;
- high-stakes choice;
- obligation conflict;
- relationship rupture;
- new evidence;
- public accusation;
- death or recovery;
- plan obstruction;
- faction crisis;
- discovery;
- long absence followed by return;
- first contact;
- deliberate reflection or rest.

Deep cognition should not run continuously merely because an NPC is visible.

# 6. Determinism and Replay

The integration must support:

- explicit model and schema versions;
- deterministic seeds where stochastic choice is used;
- content-addressed input snapshots;
- stable proposal envelopes;
- recorded accept/reject outcomes;
- replay without requiring language text to match byte-for-byte;
- semantic equivalence checks for rendered dialogue.

Simulation truth must be reproducible without depending on external model availability.

# 7. Graceful Degradation

When cognitive budget is unavailable:

1. preserve needs and obligations;
2. preserve current plan;
3. preserve relationship state;
4. preserve critical beliefs;
5. use deterministic authored dialogue;
6. defer reflection;
7. summarize off-screen outcomes.

The game must never pause a settlement because a cognition worker is unavailable.

# 8. Nonhuman and Machine Agents

The same interface may support agents whose meaningful unit is:

- one body;
- a household;
- a swarm pattern;
- an archive court;
- a machine service;
- a habitat;
- a living wetland;
- a symbiotic collective.

The adapter must not assume that all agents have human emotions, individual boundaries, language, or planning horizons.

# 9. Seedworks Scope

The first approved implementation contains:

- four named benchmark agents;
- one shared settlement pressure scenario;
- one week of simulated routine;
- five relationship dimensions;
- episodic and semantic memory;
- one bounded conflict of obligation per agent;
- HDC retrieval;
- CfC-style appraisal;
- prediction-error updates;
- deterministic dialogue frames;
- optional grounded language rendering;
- complete causal traces.

It excludes:

- unrestricted autonomous planning;
- online self-modifying code;
- free-form world editing;
- model-written laws;
- model-decided moral rights;
- unsupervised long-term personality mutation;
- open internet access;
- hidden private player profiling.

# 10. Promotion Gates

A capability may enter representative-build scope only when it demonstrates:

- better long-term consistency than the deterministic baseline;
- no increase in fabricated world facts;
- bounded CPU and memory cost;
- replayable action outcomes;
- readable causal traces;
- acceptable player comprehension;
- no dependence on generated dialogue for core mechanics;
- robust degradation;
- privacy compliance;
- meaningful improvement under causal ablation.

# 11. Kill Criteria

Remove or demote a Symthaea component if:

- players cannot tell that it improves behavior;
- it primarily creates verbose dialogue;
- it reduces factual grounding;
- it breaks replay or multiplayer authority;
- it obscures why an NPC acted;
- it creates unacceptable simulation cost;
- authored baselines perform equally well;
- its contribution disappears under ablation;
- it encourages claims of consciousness unsupported by evidence.

## Final Rule

```text
The goal is not to put a giant mind inside every NPC.

The goal is to let selected inhabitants remember,
interpret, and continue becoming themselves
without surrendering the game to an opaque oracle.
```
