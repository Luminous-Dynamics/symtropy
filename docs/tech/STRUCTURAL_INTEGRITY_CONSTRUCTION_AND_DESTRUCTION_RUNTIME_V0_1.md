---
title: Structural Integrity, Construction, and Destruction Runtime
version: 0.1
status: implementation-spec
scope: structural graph, load propagation, construction transactions, damage, collapse, utilities, LOD, persistence, and multiplayer authority
owner: physics/construction/engineering/networking
related:
  - canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md
  - tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
  - tech/MULTI_PHYSICS_VIRTUAL_GEOMETRY.md
  - tech/DEVICE_BUS_RUNTIME_SAFETY.md
  - tech/WORLDLINE_MECHANICAL_DELTA_SCHEMA_V0_1.md
  - tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
---

# Structural Integrity, Construction, and Destruction Runtime

## Owned Question

**What runtime can support readable construction, repair, load-bearing structure, bounded destruction, and persistent built history without requiring full finite-element simulation or turning every decorative object into an authoritative physics body?**

## Core Thesis

Use a hierarchical structural graph with authored connection semantics, bounded load cases, conservative safety envelopes, and event-driven damage propagation.

```text
Simulate structure where failure changes play.
Author detail where appearance changes meaning.
Do not confuse visual complexity with mechanical authority.
```

# 1. Runtime Layers

```text
Placement and Project Layer
Material and Component Layer
Structural Graph
Utility Graphs
Damage and Hazard Layer
Presentation and Debris Layer
Persistence and Chronicle Layer
```

Each layer has distinct authority and update cadence.

# 2. Structural Data Model

```rust
struct StructuralAssembly {
    assembly_id: AssemblyId,
    project_id: Option<ProjectId>,
    nodes: Vec<StructuralNodeId>,
    connections: Vec<StructuralConnectionId>,
    foundations: Vec<FoundationContact>,
    load_cases: Vec<LoadCase>,
    service_state: ServiceState,
    authority: ConstructionAuthority,
    ancestry: Vec<ConstructionEventId>,
}
```

```rust
struct StructuralNode {
    node_id: StructuralNodeId,
    component_type: ComponentTypeId,
    transform: QuantizedTransform,
    material: MaterialBatchRef,
    condition: ConditionVector,
    section_properties: SectionProperties,
    mass: Fixed,
    capacity: CapacityEnvelope,
    importance: StructuralImportance,
}
```

```rust
struct StructuralConnection {
    connection_id: StructuralConnectionId,
    a: StructuralNodeId,
    b: StructuralNodeId,
    connection_type: ConnectionType,
    stiffness_class: StiffnessClass,
    capacity: CapacityEnvelope,
    condition: ConditionVector,
    inspectability: Inspectability,
}
```

# 3. Structural Graph Rules

Nodes represent mechanically meaningful components or aggregated modules.

Connections represent:

```text
weld
bolt
hinge
socket
bearing
cable
tendon
adhesive
biological growth joint
pressure seal
magnetic or field coupling
```

Decorative meshes do not become nodes unless they affect load, access, hazard, or salvage.

# 4. Load Cases

Authoritative load cases include:

```text
self weight
stored cargo and occupants
vehicle or machine load
wind or atmospheric flow
water or fluid pressure
internal habitat pressure
thermal expansion
seismic or impact pulse
snow, ash, regolith, or deposition
buoyancy and wave load
thrust or acceleration for spacecraft
```

The solver may use simplified beam, truss, plate, module, or pressure-vessel models selected by component class.

# 5. Solver Strategy

## 5.1 Static and Quasi-Static

Most structures use incremental equilibrium or authored load distribution with conservative capacity checks.

## 5.2 Dynamic Events

Impacts, explosions, earthquakes, vehicle collisions, and rapid depressurization create bounded impulses and event-driven reevaluation.

## 5.3 Safety Envelope

```rust
struct StructuralResult {
    utilization: Fixed,
    deflection_class: DeflectionClass,
    fatigue_increment: Fixed,
    instability_risk: Fixed,
    failure_mode: Option<FailureMode>,
    cause_refs: SmallVec<CauseRef>,
}
```

Player interfaces receive categories and causes, not raw solver matrices.

# 6. Foundations

Foundation models include:

```text
spread footing
pile or anchor
rock attachment
floating platform
suspended cable
buried pressure shell
living root or reef anchor
orbital frame connection
```

Foundation contacts store bearing, sliding, uplift, settlement, thermal, and environmental risk.

Terrain deformation may be represented through bounded project operations rather than arbitrary voxel excavation in early milestones.

# 7. Construction Transactions

Construction is an authoritative transaction with physical staging.

```rust
struct ConstructionTransaction {
    event_id: EventId,
    project_id: ProjectId,
    actor: AgentOrMachineId,
    action: ConstructionAction,
    target: ConstructionTarget,
    consumed_batches: Vec<MaterialBatchRef>,
    tool_state: Vec<ToolStateRef>,
    authority_token: Option<AuthorityToken>,
    result_quality: QualityVector,
}
```

Actions:

```text
survey
excavate
place frame
attach
weld
seal
cure
connect utility
inspect
test
commission
brace
dismantle
demolish
```

Transactions are idempotent under network retry and conserve material batches.

# 8. Project State

```rust
struct ConstructionProject {
    project_id: ProjectId,
    design_ref: DesignRef,
    site_ref: SiteRef,
    stage_graph: StageGraph,
    material_requirements: Vec<Requirement>,
    labor_requirements: Vec<LaborRequirement>,
    machine_requirements: Vec<MachineRequirement>,
    permits: Vec<AuthorityRequirement>,
    hazards: Vec<ProjectHazard>,
    completion_state: CompletionState,
}
```

Projects support partial utility. A bridge may open to foot traffic before heavy vehicles. A habitat may provide shelter before full climate commissioning.

# 9. Utilities

Structural assemblies reference independent utility graphs:

```text
power
water and process fluid
air and pressure
thermal
data and Device Bus
waste and drainage
fire suppression
```

Damage to one graph can leave the shell intact but unusable.

Utility penetrations may weaken or compromise structural and pressure boundaries.

# 10. Condition and Degradation

Condition vectors include:

```text
corrosion
fatigue
cracking
deformation
seal wear
thermal damage
radiation damage
biological decay
contamination
unauthorized modification
```

Degradation updates at slow cadence except during active hazards.

Maintenance actions restore selected dimensions and may not recover original capacity without replacement.

# 11. Failure Propagation

Failure sequence:

```text
1. Detect capacity or stability breach.
2. Select physically valid failure mode.
3. Change connection or node state.
4. Recompute local dependency closure.
5. Create impulse, debris, utility, and hazard events.
6. Reevaluate supported nodes.
7. Stop when stable or budget boundary reached.
8. Schedule deferred resolution if needed.
```

Possible modes:

```text
yield
buckling
fracture
pullout
connection tear
foundation settlement
seal rupture
progressive collapse
```

A collapse budget limits simultaneous full-detail bodies. Excess debris becomes deterministic aggregate fields after initial presentation.

# 12. Damage and Combat Interface

Combat emits typed damage:

```text
kinetic
blast
cutting
thermal
corrosive
pressure
field disruption
```

The structural runtime converts damage into condition changes according to material and geometry.

Weapons do not directly subtract generic building hit points unless the component is explicitly abstracted.

# 13. Debris and Salvage

Debris has three tiers:

```text
Tier A — mechanically active large fragments
Tier B — collision and navigation obstacles
Tier C — visual and aggregate salvage fields
```

Salvage transactions recover material by provenance and condition. Debris cannot be duplicated by client prediction, rollback, or repeated collection.

# 14. Structural LOD

## LOD 0 — Active Failure

Full local graph, dynamic fragments, hazards, occupants, and rescue paths.

## LOD 1 — Active Structure

Detailed graph, load cases, utilities, and condition.

## LOD 2 — Inactive Site

Aggregated modules and critical connections; slow degradation.

## LOD 3 — Regional Aggregate

Capacity, condition, maintenance backlog, accessibility, and risk only.

LOD transitions preserve critical damage, project state, high-value components, and utility connectivity.

# 15. Multiplayer Authority

The regional shard owns structural transactions and failure state.

Clients may preview placement and tool motion. The server validates:

```text
geometry and anchors
material custody
authority
collision and protected zones
structural envelope
transaction order
```

Protected infrastructure defines modification scopes. Unauthorized changes become rejected attempts or auditable sabotage events, not silent state edits.

# 16. Persistence

Persist:

```text
project stage
structural graph topology
material batch references
connection quality
condition vectors
utility connectivity
inspections and authority
failure ancestry
salvage and demolition events
```

Do not persist transient solver matrices or visual dust particles.

# 17. Observability

Developer overlays:

```text
load paths
utilization
foundation reactions
connection states
utility graph
condition heatmap
failure cause chain
project transaction history
LOD state
```

Player views expose bounded diagnostic layers according to tools, skill, and access.

# 18. Performance Budgets

Representative targets:

```text
active detailed assemblies:        content-budgeted by scene
mechanically meaningful nodes:     <= 2,000 in active local area
connections:                       <= 4,000 in active local area
dynamic collapse fragments:        <= 128 high-detail concurrently
background structural updates:     amortized and event-driven
full graph solve:                   local dependency closure by default
```

Large structures must use module aggregation and critical-path graphs.

# 19. Acceptance Tests

1. **Placement determinism:** identical validated transactions produce identical assemblies.
2. **Material conservation:** build, dismantle, rollback, and migration preserve batch quantities.
3. **Load-path causality:** removing a support affects only its dependency closure unless secondary collapse propagates.
4. **Failure stability:** collapse resolves to a stable or explicitly deferred state within budget.
5. **Utility independence:** a structure can remain standing while power, water, or pressure fail.
6. **Repair quality:** provisional and certified repairs create different capacity and maintenance outcomes.
7. **LOD equivalence:** aggregated sites preserve declared capacity, access, and risk envelopes.
8. **Multiplayer integrity:** duplicate, replayed, or unauthorized construction events fail closed.
9. **Persistence:** save and migration preserve topology, provenance, and failure ancestry.
10. **Player legibility:** a test player can identify the immediate cause of a representative failure.

## Final Rule

```text
The solver exists to make built consequences believable.
It does not exist to turn the entire world into an unbounded physics experiment.
```
