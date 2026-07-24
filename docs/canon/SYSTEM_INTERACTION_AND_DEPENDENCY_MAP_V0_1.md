---
title: System Interaction and Dependency Map
version: 0.1
status: superseded
scope: cross-system contracts, authority boundaries, event flow, integration gates
superseded_by:
  - SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_2.md
owner: design/engineering
related:
  - canon/SYMTROPY_GAME_CONSTITUTION_V0_6.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - tech/CHRONICLE_EVENT_SCHEMA.md
  - tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
  - canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
  - canon/SCIENCE_RESEARCH_AND_DISCOVERY_CONTRACT_V0_1.md
  - tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
---

# System Interaction and Dependency Map

## Owned Question

**How do Symtropy’s major systems exchange causes and consequences without collapsing into one giant, tightly coupled simulation?**

## Core Thesis

Symtropy’s identity comes from integration, but integration does not mean every system knows everything.

Each system must own a bounded question, publish stable outputs, consume explicit inputs, and preserve failure boundaries.

```text
Strong integration = meaningful consequences across clear contracts.
Weak integration = shared global state and invisible dependencies.
```

## Integration Prime Directive

Every central player action should be traceable through a bounded causal path:

```text
embodied action
  → local world response
  → system transaction or event
  → simulation consequence
  → agent interpretation
  → visible world change
  → durable history when warranted
```

Not every path reaches the Chronicle.
Not every event affects the planet.
Every meaningful result must have an observable owner.

## System Domains

### 1. Embodied Simulation

Owns:

```text
player and NPC body motion
camera and stance
physical interaction
immediate damage
projectiles and contact
vehicle handling
local hazards
```

Authoritative question:

```text
What physically happened here, now?
```

Publishes:

```text
InteractionCompleted
DamageApplied
ObjectMoved
ToolActionResult
VehicleStateChanged
HazardExposure
BodyStateChanged
```

Must not own:

```text
civic legitimacy
long-term faction meaning
historical truth
planetary economic simulation
```

### 2. Site and Device Simulation

Owns:

```text
device state
power, fluid, signal, and material connections
machine faults
automation execution
local environmental process
construction initialization
```

Authoritative question:

```text
What state change did the local system physically and logically accept?
```

Publishes:

```text
DeviceTransactionCommitted
DeviceTransactionRejected
NetworkTopologyChanged
CapacityChanged
FaultDetected
ConstructionCommissioned
```

Must not own:

```text
whether an action was socially legitimate
what factions believe it meant
whether it becomes durable history
```

### 3. Construction and Fabrication

Owns:

```text
blueprints
material requirements
assembly stages
quality
commissioning
maintenance class
provenance of built objects
```

Authoritative question:

```text
What was built, from what, to what quality, and with what dependencies?
```

Publishes:

```text
BlueprintAcquired
AssemblyStageCompleted
BuildQualityResolved
NodeRegistered
MaintenanceObligationCreated
Decommissioned
```

### 4. Logistics and Economy

Owns:

```text
stocks
cargo commitments
routes
transport capacity
prices or exchange terms
labor availability
contracts
supply-chain risk
```

Authoritative question:

```text
What can move, who has committed it, and what becomes possible when it arrives?
```

Publishes:

```text
CargoReserved
CargoLoaded
DeliveryCompleted
RouteDisrupted
ShortageDeclared
TradeSettled
LaborCommitted
```

### 5. Ecology and Living Systems

Owns:

```text
habitat condition
trophic function
population pressure
contamination
succession
biological infrastructure
nonhuman viability
```

Authoritative question:

```text
What living conditions are changing, on what timescale, and for whom?
```

Publishes:

```text
HabitatConditionChanged
SpeciesEstablished
PopulationPressureChanged
ContaminationSpread
ViabilityThresholdCrossed
AgencySignalObserved
```

### 6. Combat and Threat Ecology

Owns:

```text
hostile posture
tactical roles
encounter objectives
reinforcement logic
morale and withdrawal
threat adaptation
```

Authoritative question:

```text
What is contesting the player, why is it escalating, and how can the encounter resolve?
```

Publishes:

```text
ThreatContact
HostilityEscalated
ObjectiveCompromised
ForceWithdrew
ThreatAdapted
EncounterResolved
```

### 7. NPC and Social Simulation

Owns:

```text
needs
relationships
beliefs
memories
routines
interpretations
group membership
social commitments
```

Authoritative question:

```text
How do living agents understand and respond to what happened?
```

Publishes:

```text
MemoryFormed
BeliefUpdated
RelationshipChanged
CommitmentMade
RoutineChanged
CoalitionFormed
MigrationDecision
```

### 8. Faction and Civic Systems

Owns:

```text
charters
rights
permissions
public authority
collective decisions
institutional memory
faction posture
precedent
```

Authoritative question:

```text
Who may decide, what rule applies, and how does a group formalize consequence?
```

Publishes:

```text
AuthorityGranted
AuthorityContested
PolicyAdopted
EmergencyDeclared
EmergencyExpired
PrecedentEstablished
FactionIdentityShifted
```

### 9. Field Deck and Player Information

Owns:

```text
observation presentation
confidence
provenance
mode-specific interpretation
player identity and credentials
sharing and attention management
```

Authoritative question:

```text
What can this player presently observe, infer, prove, access, and communicate?
```

Publishes player requests and annotations, not world truth by itself:

```text
ScanRequested
EvidencePinned
SharePayloadCreated
WitnessRequested
PlayerAnnotationAdded
```

The Field Deck may expose contradictions. It may not silently resolve them.

### 10. Chronicle and Durable Truth

Owns:

```text
signed civic outcomes
source-chain continuity
historical event persistence
worldline ancestry
accepted public records
```

Authoritative question:

```text
What should this community or worldline remember as a durable claim?
```

Publishes:

```text
ChronicleEventAccepted
ChronicleEventDisputed
SourceChainRebound
WorldlineForkDeclared
TreatyReplicated
```

### 11. Regional, Planetary, and Worldline Simulation

Owns aggregated state above the active scene:

```text
regional networks
migration
large-scale weather
trade flow
war posture
planetary ecology
orbital condition
worldline divergence
```

Authoritative question:

```text
What changes while the player is elsewhere, and what must be promoted into active simulation?
```

Publishes:

```text
RegionPressureChanged
NetworkCapacityChanged
MigrationWave
WarfrontMoved
PlanetaryEvent
WorldlineDelta
```

## Authority Layers

Use the following truth order for state changes:

```text
Local physical truth       — bodies, impacts, immediate interaction
Device transaction truth   — deterministic machine acceptance
Simulation truth           — evolving stocks, flows, conditions
Agent interpretation       — beliefs, memories, decisions
Civic truth                — permissions, policy, public legitimacy
Chronicle truth            — durable historical claim
Worldline truth            — persistent cross-region ancestry and divergence
```

A later layer may interpret an earlier layer. It may not rewrite the earlier fact without creating a new event.

Example:

```text
Physical fact:
A gate was cut open.

Device fact:
The lock controller did not authorize release.

Civic interpretation:
The action was an emergency rescue or an unlawful breach.

Chronicle claim:
The settlement accepted one interpretation after evidence and testimony.
```

## Shared Entity Identity

Cross-system objects use stable identifiers.

Conceptual minimum:

```rust
struct EntityRef {
    entity_id: EntityId,
    worldline_id: WorldlineId,
    region_id: RegionId,
    site_id: Option<SiteId>,
    entity_kind: EntityKind,
}
```

Systems may attach their own components or records. They may not create parallel identities for the same object without an explicit alias or lineage relation.

## Event Envelope

Cross-domain events should use a shared envelope:

```rust
struct SymEvent<T> {
    event_id: EventId,
    event_type: EventType,
    source_system: SystemId,
    worldline_id: WorldlineId,
    region_id: RegionId,
    site_id: Option<SiteId>,
    actor_refs: Vec<EntityRef>,
    subject_refs: Vec<EntityRef>,
    simulation_tick: u64,
    causal_parents: Vec<EventId>,
    confidence: Confidence,
    persistence_class: PersistenceClass,
    payload: T,
}
```

Persistence classes:

```text
ephemeral
site-log
settlement-memory
civic-record
chronicle
worldline
```

Promotion to a more durable class requires an explicit rule or actor action.

## Causal Trace Example: Bridge Restoration

```text
1. Player surveys collapsed span.
   Embodied + Field Deck observation.

2. Logistics system reserves structural material.
   CargoReserved.

3. Vehicle delivers material through hazardous terrain.
   DeliveryCompleted; VehicleStateChanged.

4. Players place anchors and assemble modules.
   AssemblyStageCompleted.

5. Construction system resolves quality and load limit.
   BuildQualityResolved.

6. Regional network gains route capacity.
   NetworkCapacityChanged.

7. Traders and NPC routines respond.
   RoutineChanged; TradeSettled.

8. Threat ecology notices new movement corridor.
   ThreatAdapted.

9. Settlement may establish access policy.
   PolicyAdopted.

10. Only if historically meaningful:
    ChronicleEventAccepted.
```

The bridge is not “integrated” because every system updates a generic bridge score. It is integrated because each system responds through its own contract.

## Causal Trace Example: Alien Wetland Contact

```text
1. Ecological anomaly affects route and machinery.
2. Field Deck presents low-confidence observations.
3. Science activity gathers samples and patterns.
4. Ecology system raises agency uncertainty.
5. Combat system suppresses automatic extermination classification.
6. NPCs and factions form competing interpretations.
7. Player establishes, violates, or ignores a boundary.
8. Habitat and relationship states change.
9. A treaty or conflict may become civic and Chronicle truth.
```

## Update Cadence Contract

Systems should update at the lowest frequency that preserves their meaningful behavior.

```text
Embodied combat/physics: frame or fixed tick
Device transactions: deterministic local ticks
Active site processes: seconds
NPC immediate behavior: sub-second to seconds
Settlement metabolism: minutes or event-driven
Regional networks: minutes to hours
Planetary ecology and climate: hours to days, accelerated when appropriate
Worldline exchange: asynchronous and event-driven
```

Avoid simulating distant detail at active-scene fidelity.

## Failure Boundaries

Each system must fail in a way that preserves the rest of the game.

Examples:

```text
Chronicle unavailable:
Local play continues; durable commits queue or remain local.

Advanced ecology unavailable:
Use bounded habitat state, not zero ecology.

NPC planner failure:
Fallback routine and explicit debug state, not frozen world logic.

Device script fault:
Transaction rolls back or enters diagnosable fault state.

Network disconnect:
Local shard remains playable within declared authority limits.
```

## Anti-Coupling Rules

Reject designs where:

```text
UI reads internal mutable state without a stable query contract.
Lore text is the only output of simulation.
A faction directly edits device physics to express opinion.
Chronicle storage blocks real-time action.
Every system writes to one global “harmony” value.
Distant simulation requires every active entity component.
One event causes unbounded cascades in a single frame.
```

## Integration Gates

A new system is not integrated until it demonstrates:

```text
one explicit input contract
one explicit output contract
one visible player-facing effect
one degraded-mode behavior
one deterministic or replayable test where required
one cross-system causal trace
one budget for update cost and event volume
```

## Seedworks Critical Path

The first representative build requires these contracts:

```text
Embodied interaction ↔ tools and vehicles
Embodied interaction ↔ combat and hazards
Construction ↔ logistics and device registration
Device state ↔ Field Deck presentation
Site events ↔ settlement simulation
Settlement changes ↔ NPC routines and memory
Threat outcomes ↔ regional posture
Meaningful outcomes ↔ Chronicle commit
```

Planetary and worldline layers may be stubbed behind stable interfaces.

## Debugging Requirements

Development tools should expose:

```text
causal parent graph
event ownership
persistence class
system inputs and outputs
simulation promotion/demotion
actor interpretation source
Field Deck evidence provenance
Chronicle promotion reason
```

A designer must be able to answer:

```text
Why did this happen?
Which system decided it?
What evidence did the player receive?
What will remember it?
```

## Acceptance Tests

The integration architecture is credible when:

```text
A bridge, convoy, habitat, or factory change propagates across at least four domains without direct state sharing.
A player can observe the consequence through the world, not only a debug panel.
A Chronicle outage does not halt local play.
A distant region can be simulated without active-scene entities.
The Field Deck can explain provenance without claiming omniscience.
A causal trace can be replayed from recorded events for deterministic domains.
Designers can identify and disable one system without creating undefined global state.
```

## Final Rule

```text
Systems should meet through consequences, not through entanglement.
```

# v0.2 Integration Addendum — Activity, Knowledge, Authorship, and Safety

The following domains complete the cross-system boundary map.

## Activity Orchestration

Owns:

```text
opportunity lifecycle
objective graphs
role slots
activity closure
failure continuation
consequence bindings
```

Authoritative question:

```text
What bounded intervention is currently available, active, resolved, or abandoned?
```

It may request actions from other domains but must not directly mutate their state. It receives committed events and publishes structured activity state.

## Knowledge and Research

Owns:

```text
observations
claims
hypotheses
models
replication
uncertainty
knowledge diffusion
```

Authoritative question:

```text
What is currently known, by whom, with what evidence and scope?
```

It must not convert scanner output directly into universal truth.

## Player Authorship and Content Provenance

Owns:

```text
blueprint ancestry
mod manifests
creative provenance
creator credit
compatibility declarations
save migrations
```

Authoritative question:

```text
What authored this artifact or rule set, what does it depend on, and where may it operate?
```

## Social Safety and Moderation

Owns:

```text
worldline conflict profile
platform abuse boundaries
moderation powers
appeals
abuse recovery
```

Authoritative question:

```text
Was this action allowed as gameplay, and what recovery or sanction applies when it was abuse?
```

Civic legitimacy does not replace platform moderation.

## Consequence Presentation

Owns:

```text
state overlays
site variants
audiovisual consequence channels
revisit state
causal presentation coverage
```

Authoritative question:

```text
How is an already-authoritative change made perceptible in the world?
```

Presentation does not invent simulation state.
