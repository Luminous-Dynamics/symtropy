---
title: Emergent Campaign Detection, Causal Storylet, and Player History Runtime
version: 0.1
status: implementation-spec
scope: systemic event detection, causal chains, narrative opportunities, NPC initiative, campaign assembly, player history, anti-manipulation boundaries
owner: AI/simulation/narrative/research
related:
  - ../canon/HISTORICAL_CONTENT_AND_PLAYABLE_CAMPAIGN_CONTRACT_V0_1.md
  - ../canon/PLAYER_FOUNDED_CIVILIZATION_SETTLEMENT_LEGACY_AND_WORLDLINE_CONTRACT_V0_1.md
  - PLAYABLE_HISTORY_CONTENT_COMPILER_AND_WORLDLINE_VARIATION_RUNTIME_V0_1.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - PLAYER_PROMISE_OFFICE_REPUTATION_AND_LEGACY_RUNTIME_V0_1.md
---

# Emergent Campaign Detection, Causal Storylet, and Player History Runtime

## Purpose

This runtime detects meaningful systemic history and turns it into legible opportunities without inventing arbitrary drama, assigning every event to the player, or replacing simulation with procedural quest spam.

The system observes authoritative events, identifies causal chains, estimates human significance, proposes bounded storylets, and allows NPCs and institutions to initiate responses.

> **The campaign engine does not create importance from nothing. It notices when the world has already produced obligations, conflicts, losses, hopes, or changes that people would act upon.**

# 1. Inputs

The detector consumes Chronicle events from:

- resource and infrastructure systems;
- professions and work;
- households and life course;
- health and care;
- institutions and offices;
- contracts and promises;
- migration;
- ecology;
- combat and security;
- information ecology;
- companion projects;
- culture;
- construction;
- death and reconstitution;
- worldline transitions.

Events must include provenance, affected entities, authority, visibility, privacy, and confidence.

# 2. Event Features

```rust
struct NarrativeEventFeatures {
    event_id: ChronicleEventId,
    magnitude: f32,
    duration: f32,
    reversibility: f32,
    novelty: f32,
    affected_population: PopulationEstimate,
    named_relationships: Vec<RelationshipId>,
    public_obligations: Vec<ObligationId>,
    broken_expectations: Vec<ExpectationId>,
    material_dependency: f32,
    institutional_dependency: f32,
    cultural_salience: f32,
    privacy: PrivacyClass,
    evidence_confidence: f32,
    causal_parents: Vec<ChronicleEventId>,
}
```

Magnitude alone is insufficient. A small event involving a trusted person or public obligation may be more narratively significant than a large anonymous market movement.

# 3. Causal Clusters

The runtime groups events when they share:

- material dependency;
- affected people;
- institution;
- location;
- promise;
- office;
- cultural interpretation;
- temporal proximity;
- causal ancestry;
- repeating failure pattern.

```rust
struct CausalCluster {
    cluster_id: ClusterId,
    root_events: Vec<ChronicleEventId>,
    dependent_events: Vec<ChronicleEventId>,
    unresolved_obligations: Vec<ObligationId>,
    active_agents: Vec<AgentId>,
    institutions: Vec<InstitutionId>,
    places: Vec<EntityId>,
    public_visibility: VisibilityEstimate,
    privacy_ceiling: PrivacyClass,
    stability: f32,
}
```

The detector must not merge events merely because they share a theme word.

# 4. Significance Model

Candidate significance combines:

```text
causal depth
named human or nonhuman stakes
public obligation
irreversibility
relationship history
institutional consequence
cultural interpretation
opportunity for multiple agents
worldline divergence
```

Negative weights include:

```text
repetition fatigue
low evidence confidence
private information without standing
artificial player targeting
resolved or obsolete pressure
content safety conflict
lack of actionable role
```

The score is a scheduling aid, not a truth value.

# 5. Storylet Candidate

```rust
struct StoryletCandidate {
    storylet_id: StoryletId,
    causal_cluster: ClusterId,
    initiating_agent: AgentId,
    initiating_reason: ReasonFrameId,
    opportunity_roles: Vec<OpportunityRole>,
    likely_sites: Vec<EntityId>,
    required_evidence: Vec<EvidenceRequirement>,
    possible_actions: Vec<ActionTemplateId>,
    anticipated_consequences: Vec<ConsequenceEnvelope>,
    privacy_gate: PrivacyGate,
    expiry: Option<ChronicleTick>,
    repetition_key: RepetitionKey,
}
```

The initiating agent is not always the player.

NPCs may:

- call a meeting;
- begin repairs;
- publish evidence;
- organize a strike;
- leave;
- form a cooperative;
- seek care;
- ask for mediation;
- close a service;
- create a memorial;
- challenge a charter;
- refuse a project;
- hide misconduct;
- solve the problem without the player.

# 6. Opportunity Roles

A player opportunity may be:

- direct participant;
- professional specialist;
- witness;
- office-holder;
- resident;
- funder;
- transporter;
- investigator;
- mediator;
- defendant;
- affected household member;
- outsider with no standing;
- invited advisor;
- person asked to stay away.

The engine must support the possibility that the player has no legitimate role.

# 7. Campaign Assembly

A campaign is assembled from linked storylets only when:

- a causal spine persists;
- several agents have independent objectives;
- consequences can alter the world;
- ordinary life exists around the pressure;
- the player can enter through more than one role;
- failure or absence produces a continuation;
- repetition controls pass;
- privacy and safety controls pass.

Campaign phases may emerge as:

```text
trace
recognition
organization
contestation
work
public consequence
reinterpretation
inheritance
```

These phases are not mandatory quest beats.

# 8. Anti-Arbitrary-Drama Rules

The runtime may not:

- injure a companion merely because the player has been inactive;
- create betrayal solely to refresh engagement;
- kill a resident to make a meeting feel important;
- manufacture scarcity without resource causality;
- expose private trauma as public content without standing;
- make every systemic problem wait for the player;
- increase conflict because a hidden excitement meter is low;
- turn all success into a new crisis;
- force moral symmetry where evidence is asymmetric;
- produce a villain when institutional failure is sufficient.

# 9. Player History Model

```rust
struct PlayerHistoryState {
    player_id: AgentId,
    public_actions: Vec<ChronicleEventId>,
    private_commitments: Vec<CommitmentId>,
    offices: Vec<OfficeTenureId>,
    built_assets: Vec<AssetContributionId>,
    witnessed_events: Vec<WitnessRecordId>,
    authored_artifacts: Vec<ArtifactId>,
    disputed_attributions: Vec<AttributionDisputeId>,
    absences: Vec<AbsenceRecord>,
    identity_events: Vec<IdentityEventId>,
}
```

The player history is evidence, not a heroic biography.

Different histories may cite different subsets and interpretations.

# 10. Attribution

When a settlement change occurs, attribution may be distributed among:

- initiator;
- designer;
- funder;
- workers;
- office-holder;
- institution;
- households;
- external sponsor;
- ecological event;
- accident;
- previous generation.

The system must avoid crediting the player for work completed by NPCs merely because the player started the project.

# 11. Campaign Fatigue

Track repetition across:

- crisis type;
- initiating role;
- location;
- profession;
- emotional register;
- institutional form;
- resolution method;
- affected companion;
- public hearing structure.

A region should contain:

- ordinary continuity;
- successful maintenance;
- unresolved low-level tensions;
- humor;
- celebration;
- projects that complete without crisis;
- events ignored by the player;
- stories discovered only later.

# 12. Privacy

The detector may know private state for simulation but cannot expose it without a valid information path.

A private coercion indicator, medical condition, romantic conflict, or machine memory may influence an agent's behavior without becoming a player-facing quest objective.

IRIS receives only information authorized by evidence and access rules.

# 13. Absence

During player absence:

- storylets may resolve;
- campaigns may change initiator;
- NPCs may succeed or fail;
- institutions may act;
- evidence may disappear;
- rumors may replace direct knowledge;
- opportunities may expire;
- new inheritance conflicts may emerge.

Return summaries identify consequences without pretending the player witnessed them.

# 14. Worldline Variation

Campaign candidates store:

- causal ancestry;
- branch conditions;
- invariant facts;
- disputed facts;
- branch-specific agents;
- inherited obligations;
- suppressed alternatives.

Two worldlines may produce similar campaigns for different reasons. The compiler must preserve those reasons.

# 15. Validation

The first proof must demonstrate:

- one campaign emerging from infrastructure neglect;
- one from a broken promise;
- one from cultural adoption;
- one resolved without the player;
- one where the player has no standing;
- one private event that remains private;
- one campaign that becomes ordinary institutional work;
- one five-year return interpretation;
- one worldline divergence;
- deterministic replay.

# Closing Rule

> **Emergent narrative is not randomness arranged into drama. It is causality made legible to the people who have reasons to care.**
