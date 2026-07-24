---
title: Strategic Conflict, Campaign, and Occupation Simulation
version: 0.1
status: implementation-spec
scope: strategic conflict state, force readiness, territorial capability, supply, diplomacy, occupation, and simulation LOD
owner: simulation/networking/gameplay
related:
  - canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md
  - tech/COMBAT_THREAT_AND_SYSTEMIC_ENCOUNTER_DESIGN_V0_1.md
  - tech/PROCEDURAL_FACTION_EVOLUTION.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
---

# Strategic Conflict, Campaign, and Occupation Simulation

## Purpose

This specification defines the minimum simulation needed to turn encounters, fleets, routes, settlements, and faction choices into coherent campaigns without running a frame-level war simulation across the entire galaxy.

## 1. State Domains

```rust
struct StrategicConflictState {
    conflicts: BTreeMap<ConflictId, Conflict>,
    forces: BTreeMap<ForceId, ForceState>,
    control: BTreeMap<RegionCellId, ControlVector>,
    routes: BTreeMap<RouteId, StrategicRouteState>,
    agreements: BTreeMap<AgreementId, AgreementState>,
    occupations: BTreeMap<SiteId, OccupationState>,
    displacement: BTreeMap<PopulationId, DisplacementState>,
}
```

## 2. Conflict

```rust
struct Conflict {
    parties: Vec<ConflictParty>,
    aims: Vec<WarAimRef>,
    start_event: EventId,
    escalation_level: EscalationLevel,
    exhaustion: BTreeMap<ActorId, f32>,
    negotiation_windows: Vec<NegotiationWindow>,
    protected_sites: BTreeSet<SiteId>,
    conflict_profile: ConflictProfileId,
}
```

Escalation levels:

```text
dispute
coercion
limited violence
organized campaign
regional war
system-scale war
```

Escalation affects mobilization, mission generation, civilian behavior, diplomatic pressure, and what actions require explicit worldline permission.

## 3. Forces

A force is a persistent organization, not every body in a formation.

```rust
struct ForceState {
    owner: ActorId,
    force_type: ForceType,
    personnel_equivalent: f32,
    equipment: EquipmentProfile,
    readiness: ReadinessVector,
    location: StrategicLocation,
    assigned_operation: Option<OperationId>,
    supply_source: Option<NodeId>,
    doctrine: DoctrineId,
    cohesion: f32,
    civilian_relation: f32,
}
```

At full simulation, a force can instantiate embodied units. At summary scale, outcomes use readiness, terrain, intelligence, objectives, doctrine, and deterministic event sampling.

## 4. Control Vector

```rust
struct ControlVector {
    presence: BTreeMap<ActorId, f32>,
    observation: BTreeMap<ActorId, f32>,
    mobility: BTreeMap<ActorId, f32>,
    supply: BTreeMap<ActorId, f32>,
    administration: BTreeMap<ActorId, f32>,
    legitimacy: BTreeMap<ActorId, f32>,
    resilience: BTreeMap<ActorId, f32>,
}
```

A location may be contested across dimensions. UI should communicate the dimensions relevant to the current activity rather than showing seven bars everywhere.

## 5. Route and Supply Graph

Strategic supply moves across typed edges:

```text
road
rail
river
coastal lane
air corridor
orbital transfer
subsurface tunnel
signal network
```

Each edge has:

```rust
struct StrategicRouteState {
    capacity: f32,
    travel_time: SimDuration,
    condition: f32,
    hazard: f32,
    interdiction: BTreeMap<ActorId, f32>,
    access_rules: AccessRuleSet,
    seasonal_modifiers: Vec<Modifier>,
}
```

Supply is aggregated into classes rather than tracking every cartridge at strategic scale:

```text
sustenance
energy
munitions
parts
medical
signal
special mission cargo
```

Local play may physicalize specific cargo whose loss or delivery matters.

## 6. Operations

```rust
struct Operation {
    sponsor: ActorId,
    objective_graph: ObjectiveGraphId,
    participating_forces: Vec<ForceId>,
    start_window: TimeWindow,
    supply_budget: SupplyVector,
    intelligence: IntelligenceEstimate,
    abort_conditions: Vec<Condition>,
    success_conditions: Vec<Condition>,
    civilian_constraints: Vec<Constraint>,
}
```

Operations generate player activities through the canonical mission grammar. Player outcomes update the operation rather than replacing the entire campaign simulation.

## 7. Resolution Modes

### Full Resolution

Used when players are present.

Inputs:

```text
actual embodied outcomes
device and construction transactions
captured or destroyed assets
rescues, surrender, withdrawal
```

### Assisted Summary

Used for nearby or important operations without players.

Resolution combines deterministic readiness calculations with bounded seeded variation and emits a causal report.

### Coarse Strategic Update

Used for distant conflicts.

Updates only major position, readiness, supply, displacement, agreement, and leadership changes.

No summary mode may produce an outcome impossible under route capacity, travel time, force state, or declared objectives.

## 8. Exhaustion and Cohesion

Exhaustion accumulates from:

```text
casualties
fatigue
supply loss
civilian suffering
economic disruption
failed operations
broken promises
long duration
internal dissent
```

Cohesion changes through:

```text
leadership credibility
shared success
care and rotation
ideological agreement
unit relationships
perceived legitimacy
```

Low cohesion creates events such as refusal, desertion, splintering, unauthorized violence, or negotiation pressure.

## 9. Diplomacy and Agreement Runtime

```rust
struct AgreementState {
    parties: Vec<ActorId>,
    clauses: Vec<AgreementClause>,
    verification: Vec<VerificationMechanism>,
    active_breaches: Vec<BreachRecord>,
    review_tick: Option<ChronicleTick>,
    expiry_tick: Option<ChronicleTick>,
    public_status: Publicity,
}
```

Clause execution is event-driven. A safe-passage clause may subscribe to checkpoint denials; an arms limit may subscribe to deployment or production events.

Disputes should enter an appeal, investigation, retaliation, or withdrawal process rather than instantly changing a global relationship score.

## 10. Occupation State

```rust
struct OccupationState {
    occupier: ActorId,
    site: SiteId,
    administration_model: AdministrationModel,
    service_continuity: ServiceVector,
    local_participation: f32,
    coercion: f32,
    resistance: f32,
    legitimacy: f32,
    exit_conditions: Vec<Condition>,
    review_tick: ChronicleTick,
}
```

Occupation updates mission opportunities, civilian schedules, service access, trade, faction evolution, intelligence, and Chronicle events.

## 11. Displacement

Population movement uses cohorts, not individual pathfinding at strategic scale.

```rust
struct DisplacementState {
    origin: RegionId,
    destination_candidates: Vec<RegionId>,
    population: u32,
    mobility: f32,
    health: f32,
    documentation_integrity: f32,
    family_cohesion: f32,
    protection_status: ProtectionStatus,
}
```

When players encounter a cohort, selected households and named agents may instantiate from it with preserved histories.

## 12. Intelligence Estimates

Strategic actors store estimates rather than truth.

```rust
struct IntelligenceEstimate {
    observations: Vec<IntelObservation>,
    inferred_force: Distribution<ForceEstimate>,
    confidence: f32,
    staleness: SimDuration,
    deception_risk: f32,
}
```

The debug layer can inspect ground truth; players and NPCs cannot unless authorized.

## 13. Event Interface

Consumes:

```text
EncounterResolved
RouteConditionChanged
ConvoyDelivered
InfrastructureDisabled
ForceSurrendered
CivilianHarmRecorded
AgreementSigned
AgreementBreached
FactionSchism
LeadershipChanged
```

Publishes:

```text
OperationCreated
FrontShifted
ForceReadinessChanged
SupplyCrisis
NegotiationWindowOpened
OccupationEstablished
DisplacementStarted
CeasefireViolated
WarAimSatisfied
ConflictEnded
```

Only selected durable outcomes enter Chronicle / civic truth.

## 14. Determinism and Audit

Each summary resolution records:

```text
state version
input forces and readiness
route and supply assumptions
objective weights
seed / sample index
outcome distribution
selected outcome
emitted events
```

This supports replay, balancing, dispute review, and migration.

## 15. Seedworks Test Campaign

A representative test should include:

```text
two settlements
a contested bridge and relay route
one convoy force
one raider or hostile machine force
one civilian cohort
one ceasefire possibility
```

Players should be able to alter the campaign by:

```text
repairing or destroying the bridge
escorting the convoy
publishing evidence
negotiating passage
capturing a signal node
rescuing civilians
```

## 16. Acceptance Gates

- strategic outcomes cite the route, readiness, objective, and player events that caused them;
- distant simulation cannot teleport forces or supplies;
- contested control supports multiple actors and dimensions;
- a ceasefire clause can be mechanically verified and breached;
- occupation affects services, resistance, and legitimacy;
- campaign state survives save/load and version migration;
- player conflict settings remain authoritative;
- summary simulation stays inside the allocated CPU budget.
