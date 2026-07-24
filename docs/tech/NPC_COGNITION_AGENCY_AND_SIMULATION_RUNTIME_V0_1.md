---
title: NPC Cognition, Agency, and Simulation Runtime Contract
version: 0.1
status: implementation-spec
scope: NPC decision architecture, perception, memory, planning, simulation LOD, dialogue boundaries, determinism
owner: AI/simulation/narrative/engineering
related:
  - vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
  - recipes/NPC_AI.md
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
---

# NPC Cognition, Agency, and Simulation Runtime Contract

## Owned Question

**How can Symtropy simulate people, machines, animals, factions, and nonhuman agents that appear purposeful and historically situated without requiring every actor to run an expensive, opaque, or narratively uncontrollable mind?**

## Core Thesis

NPC intelligence is not one algorithm.

It is a layered contract among:

```text
perception
body and environment
needs and viability
values and obligations
memory and belief
relationships
available actions
planning horizon
social interpretation
simulation budget
```

The player should experience agents as beings who have reasons, limitations, habits, and histories. The runtime should achieve that through bounded state and legible decisions—not through unlimited language generation or claims of perfect psychological simulation.

```text
Believable agency is not maximum complexity.
Believable agency is coherent action under remembered conditions.
```

## Prime Directives

1. **The visible body is not always the whole agent.** A vehicle crew, swarm, household, machine court, or ecological network may act through a collective policy.
2. **No omniscient NPCs.** Agents act on perceived and believed state, not authoritative world state.
3. **No single intelligence stack for every entity.** Ambient life, named citizens, leaders, machines, and nonhuman systems require different costs and guarantees.
4. **No dialogue model owns truth.** Language may express decisions and memories; it may not invent authoritative facts, resources, permissions, or Chronicle events.
5. **No consciousness score as a gameplay obedience switch.** Symthaea-derived signals may modulate attention, confidence, timing, learning, or motor quality when evidence supports it, but must not silently erase agency or reduce personhood to one scalar.
6. **Important decisions must be inspectable.** Developers need causal traces; players need readable behavior and diegetic explanations.
7. **Off-screen simulation must preserve causes, not animation.** A citizen outside the active region still has obligations and consequences, but does not need full pathfinding or frame-level cognition.

# 1. Agent Runtime Layers

Every active agent is assembled from bounded layers. Not every tier implements every layer at full fidelity.

```text
Sensing
  ↓
Perceptual Working State
  ↓
Needs / Viability / Threat
  ↓
Values / Roles / Obligations
  ↓
Memory and Belief Update
  ↓
Candidate Intent Generation
  ↓
Action Arbitration or Planning
  ↓
Embodied Execution
  ↓
Outcome Appraisal
  ↓
Memory, Relationship, and Learning Update
```

## 1.1 Sensing

Sensing produces observations, not conclusions.

Examples:

```text
saw smoke over the east ridge
heard a generator miss twice
received a signed evacuation order
noticed a friend did not arrive for shift
smelled coolant in a sealed corridor
detected an unfamiliar pressure rhythm
```

Sensors have:

```rust
struct SensorChannel {
    modality: Modality,
    range: f32,
    confidence: f32,
    latency: SimDuration,
    occlusion_model: OcclusionModel,
    noise_profile: NoiseProfile,
    source_provenance: ProvenanceClass,
}
```

Agents should confuse, miss, or reinterpret signals in ways consistent with their bodies, training, equipment, stress, and culture.

## 1.2 Perceptual Working State

The working state is a small cache of currently salient beliefs.

It should include:

```text
nearby hazards
current task and interruption reason
socially important actors present
relevant devices and routes
recent surprising changes
uncertainty and contradiction flags
```

It must remain bounded. The world may contain millions of facts; an agent should reason over tens of salient items.

## 1.3 Needs and Viability

Human and animal agents track embodied needs. Machine and nonhuman agents may track viability conditions instead.

Possible dimensions:

```text
energy
hydration
nutrition
rest
safety
pain
thermal stability
social connection
privacy
role completion
habitat continuity
signal coherence
memory integrity
boundary integrity
```

Needs create pressure. They do not directly dictate action.

A hungry medic may still finish stabilizing a patient. A frightened courier may still cross a checkpoint. A machine steward may refuse an efficient command because it violates a protected maintenance boundary.

## 1.4 Values, Roles, and Obligations

Values define what an agent treats as worth protecting. Roles define expected competencies and responsibilities. Obligations are concrete commitments with scope and expiry.

```rust
struct Obligation {
    id: ObligationId,
    beneficiary: AgentOrInstitutionId,
    action_class: ActionClass,
    urgency: f32,
    legitimacy: f32,
    expiry: Option<ChronicleTick>,
    breach_cost: ConsequenceVector,
}
```

Examples:

```text
finish the clinic shift
return a borrowed vehicle
protect a child during evacuation
honor a convoy rescue compact
keep a confidential testimony sealed
maintain a quarantine boundary
avoid damaging a spawning habitat
```

## 1.5 Memory and Belief

The runtime distinguishes:

```text
episodic memory      — a bounded record of experienced events
semantic belief      — a generalized claim believed to be true
relationship memory  — what happened between agents
procedural memory    — learned skill and task familiarity
institutional memory — rules, roles, precedents, and shared records
```

A memory is not automatically truth.

```rust
struct AgentMemory {
    event_ref: Option<EventId>,
    summary: MemorySummary,
    confidence: f32,
    emotional_weight: f32,
    source: MemorySource,
    accessibility: f32,
    contradiction_links: Vec<MemoryId>,
    decay_policy: DecayPolicy,
}
```

Named agents retain a small number of high-weight memories and compress routine events into summaries.

Example:

```text
Raw episodes:
- player delivered medicine during storm
- player stayed after the road reopened
- player repaired clinic refrigeration

Compressed belief:
- player is reliable during care emergencies
```

Compression must preserve contradictory evidence rather than smoothing every relationship into a reputation score.

## 1.6 Candidate Intent Generation

Candidate intents may arise from:

```text
urgent need
assigned work
social request
threat response
curiosity
habit
relationship repair
faction instruction
personal project
opportunism
moral refusal
```

Each intent declares prerequisites, expected outcomes, costs, interruption tolerance, and visible rationale.

## 1.7 Arbitration and Planning

Symtropy uses multiple planners behind one intent contract.

### Reactive Policy

For immediate hazards and simple creatures.

```text
avoid fire
seek cover
flee loud machinery
stabilize balance
```

### Utility Arbitration

For ambient and situated agents choosing among short tasks.

Scores should include:

```text
need pressure
role relevance
relationship weight
risk
travel cost
available tools
social legitimacy
habit
novelty or boredom
```

### Bounded Goal Planning

For named citizens and complex machines.

Plans should be short, interruptible, and built from authored actions. Do not search an unlimited world-state space.

### Deliberative / Cognitive Loop

For rare hero agents, machine persons, nonhuman intelligences, or research prototypes.

A cognitive architecture may propose beliefs, attention shifts, predictions, or candidate intentions. It still acts through the same validated action interface as every other agent.

```text
Cognition may propose.
The world transaction layer disposes.
```

# 2. Simulation Tiers

## Tier 0 — Ambient Population

Budget target:

```text
hundreds to thousands per region
low-frequency schedule and reaction updates
no persistent private plan tree
```

Required state:

```text
archetype
household or work group
current zone
schedule phase
basic needs
mood band
one or two reaction tags
```

## Tier 1 — Situated Agents

Budget target:

```text
dozens to low hundreds in active settlement space
utility arbitration at 1–4 Hz
bounded local memory
```

Required state:

```text
role and skill
needs
short obligations
3–8 relationship edges
compact beliefs
current intent
```

## Tier 2 — Named Citizens

Budget target:

```text
approximately 8–30 per major region
planning at event boundaries or 0.2–1 Hz
persistent relationships and arcs
```

Required state:

```text
biography
values and blind spots
named relationships
episodic and semantic memory
personal projects
belief-change rules
dialogue state
```

## Tier 3 — Hero / Institutional Agents

Budget target:

```text
rare
explicit authored ownership
worldline-persistent
```

Examples:

```text
faction founders
major companions
machine persons
archive courts
alien envoys
distributed ecological intelligences
```

These agents may use specialized cognition, but must still degrade gracefully to deterministic summaries when off-screen.

# 3. Embodiment and Action Contracts

Agents do not directly set arbitrary world state.

They request actions through validated interfaces:

```rust
enum AgentActionRequest {
    Move(RouteIntent),
    Manipulate(DeviceAction),
    Communicate(SpeechAct),
    Transfer(ResourceTransfer),
    Construct(BuildAction),
    Treat(CareAction),
    Attack(CombatAction),
    Vote(CivicAction),
    Refuse(RefusalAction),
    Witness(WitnessAction),
}
```

The owning system returns:

```text
accepted
rejected with reason
partially completed
interrupted
deferred
completed with unexpected consequence
```

The agent appraises the result and updates memory. This prevents an AI layer from bypassing physics, permissions, inventories, civic law, or multiplayer authority.

# 4. Social Cognition

An agent should reason about others through scoped models, not perfect psychological access.

Relationship dimensions may include:

```text
trust
warmth
fear
respect
obligation
resentment
familiarity
dependency
attraction
ideological alignment
```

Not all dimensions apply to all agents.

Agents infer intentions from:

```text
observed behavior
reputation received from trusted sources
role expectations
past relationship
cultural scripts
current stress
visible evidence
```

They can be wrong.

## Belief Change

Beliefs change through:

```text
surprising direct experience
repeated counterexample
trusted testimony
public evidence
social pressure
trauma
successful cooperation
failed prediction
```

A single dialogue choice should rarely rewrite a deep belief unless it reveals decisive evidence or completes a long arc.

# 5. Emotion and Appraisal

Emotion should alter attention, urgency, risk tolerance, memory weighting, voice, posture, and interruption thresholds.

It should not become a universal stat debuff.

Example appraisal dimensions:

```text
novelty
control
threat
loss
responsibility
social exposure
moral violation
hope
relief
```

Emotional states must decay, transform, or be maintained by causes. NPCs should not remain permanently furious because of one stale flag.

# 6. Dialogue and Generative Language Boundary

Dialogue systems may:

```text
select authored lines
compose from approved semantic frames
summarize remembered events
express uncertainty
ask context-sensitive questions
adapt tone and vocabulary
```

Dialogue systems may not independently create:

```text
new inventory
new laws
new permissions
new historical events
new relationships
new scientific facts
new quest completion
```

Any generated line must be grounded in an approved dialogue frame:

```rust
struct DialogueFrame {
    speaker: AgentId,
    speech_act: SpeechAct,
    grounded_facts: Vec<FactRef>,
    allowed_inferences: Vec<InferenceRef>,
    emotional_tone: ToneVector,
    disclosure_scope: DisclosureScope,
}
```

For networked play, authoritative effects are committed separately from text rendering.

# 7. Symthaea Integration Boundary

Symthaea may supply experimental capabilities such as:

```text
attention selection
predictive error signals
memory retrieval candidates
uncertainty estimates
adaptive timing
motor-intent proposals
social-pattern encoding
```

It must not be treated as proof that an NPC is conscious, nor may a consciousness metric determine legal personhood, rights, or narrative importance.

All experimental cognition must support:

```text
feature flags
baseline comparison
causal ablation
telemetry
safe fallback planner
deterministic replay inputs where required
```

# 8. Determinism and Multiplayer

The authoritative shard owns accepted actions and durable state.

Clients may predict animation and local movement, but not private beliefs or decisions with permanent effects.

For replay and debugging, record:

```text
agent state version
perception digest
selected intent
candidate scores or planner result
action request
world response
random seed or deterministic sample index
```

Private internal state should not be replicated to other players unless revealed through behavior, testimony, or authorized diagnostic interfaces.

# 9. Off-Screen and Long-Distance Simulation

Agents outside active simulation transition through levels of detail.

```text
Full: body, navigation, sensing, action execution
Local Summary: schedules, task outcomes, relationship encounters
Regional Summary: work contribution, migration, risk, major events
Worldline Summary: institutional membership, life milestones, Chronicle-worthy change
```

Promotion back to full simulation reconstructs a coherent local state from the summary. It must not fabricate impossible travel, resources, or relationships.

# 10. Failure Modes to Prevent

```text
omniscient NPCs
quest dispensers waiting forever
all NPCs speaking like philosophers
one global reputation number
instant ideological conversion
AI-generated facts becoming canon
background agents consuming frame budget
leaders immune to ordinary needs
NPCs teleporting resources to satisfy plans
motor paralysis presented as moral or conscious failure
private thoughts leaked through multiplayer replication
```

# 11. Developer Observability

Every named agent should expose an opt-in debug view:

```text
current perceptions
salient beliefs
active needs
current obligation
candidate intents
selected intent and reason
plan steps
recent action results
memory changes
relationship deltas
simulation LOD
```

The debug view is not player-facing truth. It is an engineering instrument.

# 12. Seedworks Implementation Slice

Seedworks should prove:

```text
12–20 Tier 0 ambient agents
8–12 Tier 1 situated agents
4–6 Tier 2 named citizens
1 bounded machine agent
```

Required scenarios:

1. **Competing obligations:** a driver chooses between assigned cargo work and helping an injured friend.
2. **Belief correction:** an NPC updates a belief after direct evidence contradicts a rumor.
3. **Schedule consequence:** restoring a route changes work, leisure, and market routines.
4. **Social memory:** a named citizen remembers how the player handled a dangerous choice.
5. **Graceful degradation:** agents retain coherent outcomes when their site is simulated off-screen.
6. **Action authority:** no cognition layer bypasses inventory, Device Bus, combat, or civic validation.

# 13. Acceptance Gates

The contract is implemented only when:

- a named NPC can explain a recent decision using grounded causes;
- two agents in the same role can choose differently because of memory, relationships, or values;
- off-screen simulation produces no impossible resources or travel;
- dialogue cannot create authoritative facts;
- a saved and replayed decision reproduces from recorded inputs within the supported determinism profile;
- ambient population cost remains inside the region budget;
- experimental cognition can be disabled without breaking core gameplay;
- players can predict broad behavior without seeing hidden numerical scores.

## Final Rule

```text
An NPC should not feel alive because it can say anything.
It should feel alive because what it does belongs to a body, a place, a memory, and a future.
```


# v1.0 Symthaea Integration Clarification

This runtime remains authoritative over agent state and action selection.

The Symthaea bridge may provide bounded cognitive proposals only. It does not supersede perception permissions, action templates, relationship state, game facts, dialogue claims, privacy, worldline authority, or deterministic fallback.

Use the v1.0 benchmark and ablation program before promoting any Symthaea component into representative-build scope.
