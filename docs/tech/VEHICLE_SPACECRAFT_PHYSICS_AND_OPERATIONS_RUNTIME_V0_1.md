---
title: Vehicle and Spacecraft Physics and Operations Runtime
version: 0.1
status: implementation-spec
scope: vehicle simulation profiles, propulsion, control, cargo, crews, damage, routes, orbital operations, networking, LOD, and persistence
owner: physics/vehicles/networking/simulation
related:
  - canon/MOBILITY_VEHICLES_AND_EXPEDITION_OPERATIONS_CONTRACT_V0_1.md
  - tech/Symtropy Vehicle & Mobility Design.md
  - SYMTROPY_SPACECRAFT_DESIGN_BIBLE_V0_1.md
  - tech/NETWORKING_STACK_DECISION.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Vehicle and Spacecraft Physics and Operations Runtime

## Owned Question

**What shared runtime can support distinct ground, water, air, rail, orbital, and interplanetary vehicle experiences while preserving deterministic operations, multiplayer responsiveness, cargo integrity, crew roles, damage, and scalable background travel?**

## Core Thesis

Use a common operational model with domain-specific physics profiles.

```text
Shared identity, cargo, crew, power, damage, and persistence.
Distinct contact, fluid, aerodynamic, guided, and orbital dynamics.
```

Do not force every vehicle into one rigid-body controller. Do not build isolated systems that cannot share logistics, damage, authority, or save semantics.

# 1. Common Vehicle State

```rust
struct VehicleState {
    vehicle_id: VehicleId,
    chassis: ChassisRef,
    domain_profile: VehicleDomainProfile,
    pose: AuthoritativePose,
    velocity: VelocityState,
    modules: Vec<VehicleModuleState>,
    energy: EnergySystemState,
    thermal: ThermalState,
    life_support: Option<LifeSupportState>,
    cargo: CargoManifest,
    crew: CrewManifest,
    damage: VehicleDamageState,
    autonomy: AutonomyState,
    authority: VehicleAuthority,
    route_plan: Option<RoutePlanId>,
    provenance: VehicleProvenance,
}
```

# 2. Domain Profiles

## 2.1 Wheeled and Tracked Ground

Models:

```text
contact patches
suspension
traction envelope
slope and surface response
steering geometry
load transfer
water and mud ingress
```

Use stable approximations rather than tire laboratory simulation.

## 2.2 Legged and Articulated

Models:

```text
support polygon
foot placement classes
terrain affordance
stability reserve
energy and actuator heat
```

Detailed inverse kinematics may be client-presentational while authoritative movement uses bounded locomotion envelopes.

## 2.3 Rail and Guided Transit

Models:

```text
track graph
switch authority
consist mass and braking
grade
schedule and block occupancy
power supply
```

## 2.4 Watercraft

Models:

```text
buoyancy
hydrodynamic drag
propulsion
wave and current classes
flooding compartments
stability and free-surface effects
```

## 2.5 Aircraft

Models:

```text
lift and drag envelopes
thrust
stall and control authority
weather and density
energy reserve
landing requirements
```

High-fidelity aerodynamics are not required for every craft, but failure states must remain physically coherent.

## 2.6 Orbital and Spacecraft

Models:

```text
six-degree-of-freedom local dynamics
patched-conic or numerical orbital propagation
thrust and propellant
attitude control
thermal and radiation exposure
pressure compartments
life support
communications delay
```

During close operations, local physics dominates. During cruise, orbital propagation and operational simulation dominate.

# 3. Module Architecture

Vehicle modules include:

```text
propulsion
energy generation and storage
control surfaces or contact systems
cargo
crew habitat
life support
thermal rejection
sensors
communications
construction and repair tools
weapons or defense
science instruments
docking and towing
```

Modules expose typed ports for:

```text
structure
power
fluid
thermal
data
crew access
cargo access
```

# 4. Power and Energy

```rust
struct EnergySystemState {
    stores: Vec<EnergyStore>,
    generators: Vec<GeneratorState>,
    consumers: Vec<ConsumerState>,
    distribution: DistributionGraph,
    reserve_policy: ReservePolicy,
}
```

Energy updates are operational transactions. Client UI may predict gauges, but authoritative range and depletion belong to the shard.

# 5. Thermal and Life Support

Thermal state tracks bounded zones and heat paths rather than every component temperature.

Life support tracks:

```text
pressure
oxygen or required gas mix
carbon dioxide or waste gas
humidity
water
food and metabolic reserve
contamination
occupancy
```

Leaks and failures propagate by compartment graph.

# 6. Cargo Runtime

```rust
struct CargoItemState {
    asset_ref: EconomicAssetRef,
    container_id: ContainerId,
    transform_or_slot: CargoPlacement,
    restraint_state: RestraintState,
    environmental_state: CargoEnvironment,
    custody: CustodyRef,
}
```

Cargo affects mass, center of mass, stability, drag, and access.

For high-volume fungible cargo, use batch containers rather than one entity per unit.

# 7. Crew Runtime

Crew stations expose tasks rather than permanent classes.

```rust
struct VehicleTask {
    task_id: TaskId,
    station: StationId,
    capability_required: CapabilityVector,
    urgency: Fixed,
    automation_support: AutomationSupport,
    failure_consequence: ConsequenceVector,
}
```

Players and NPCs may take, delegate, queue, or interrupt tasks.

Solo mode combines routine tasks under bounded automation and slows escalation where necessary for fairness.

# 8. Controls and Input

Control layers:

```text
direct embodied control
assisted stabilization
navigation command
route automation
fleet or convoy command
```

A player may move between layers without changing the authoritative vehicle identity.

# 9. Damage Model

Vehicle damage is component and connection based.

```text
structural breach
mobility loss
propulsion degradation
energy fault
thermal overload
sensor loss
control damage
cargo breach
life-support failure
software or autonomy corruption
```

Damage events cite causes and affected modules. Generic hull integrity may summarize but not replace component state.

# 10. Repair and Recovery

Supported actions:

```text
isolate
bypass
patch
replace
cannibalize
recalibrate
tow
recover
abandon
scuttle
```

Repair uses construction and economic transactions. Recovery preserves cargo and vehicle provenance.

# 11. Route Runtime

```rust
struct RoutePlan {
    route_id: RoutePlanId,
    segments: Vec<RouteSegment>,
    expected_cost: CostEnvelope,
    weather_or_window: Vec<ConditionWindow>,
    access_requirements: Vec<AuthorityRequirement>,
    reserve_policy: ReservePolicy,
    alternates: Vec<RoutePlanId>,
}
```

Segments may be terrain paths, rail blocks, waterways, air corridors, launch phases, orbital arcs, or docking sequences.

The runtime may compress safe segments and interrupt on deviations, discoveries, requests, hazards, or player choice.

# 12. Orbital Operations

Orbital state uses:

```text
central body
state vector or orbital elements
epoch
maneuver plan
propellant and thrust profile
uncertainty envelope
```

Maneuver planning converts player intent into executable burns. Execution can be automated, manually supervised, or directly controlled for close operations.

Time acceleration advances deterministic background systems and pauses or reduces when:

```text
collision risk
critical failure
combat or interception
crew emergency
contact event
player-defined interruption
```

# 13. Convoys and Fleets

A convoy is a coordination object.

```rust
struct ConvoyState {
    convoy_id: ConvoyId,
    members: Vec<VehicleId>,
    formation_policy: FormationPolicy,
    route_plan: RoutePlanId,
    cargo_commitments: Vec<ContractId>,
    protection_policy: ProtectionPolicy,
    rescue_policy: RescuePolicy,
    command_authority: AuthorityRef,
}
```

Background convoy simulation aggregates movement, readiness, weather, and threats. Nearby convoys instantiate local vehicles.

# 14. Network Authority

The real-time shard owns:

```text
pose and velocity
direct control results
collisions
local damage
module state
cargo transfer
crew station occupancy
```

Device transaction truth owns programmable control changes. Chronicle owns durable mission, loss, rescue, and public-service outcomes.

Prediction and reconciliation follow domain-specific tolerances. Spacecraft cruise does not need the same replication cadence as a racing bike.

# 15. LOD and Background Travel

## LOD 0 — Direct Control

Full local physics, occupants, cargo, damage, and interaction.

## LOD 1 — Nearby Operational

Simplified physics and full operational systems.

## LOD 2 — Route Simulation

Segment progress, resource use, weather, condition, encounters, and crew tasks.

## LOD 3 — Strategic Aggregate

Convoy or fleet capability, route state, cargo commitments, and arrival windows.

Transitions preserve mass, cargo, damage, crew, energy, and causal event state.

# 16. Persistence

Persist:

```text
vehicle identity and ancestry
module graph
condition and damage
cargo and custody
crew and passengers
energy and life support
route and task state
autonomy configuration
registration and authority
```

Do not persist interpolation caches or transient wheel-contact data.

# 17. Observability

Developer tools:

```text
force and contact overlays
energy flow
thermal zones
life-support compartments
cargo mass and center
control authority
route cost breakdown
network correction metrics
LOD state
failure cause chain
```

# 18. Performance Budgets

Representative target:

```text
player-controlled detailed vehicles: 1–4
nearby detailed AI vehicles:          content-budgeted
background convoys:                   aggregate simulation
module count per Seedworks vehicle:   <= 32 authoritative modules
cargo entities:                       batch where possible
orbital propagation:                  fixed deterministic jobs
```

# 19. Acceptance Tests

1. **Mass and cargo conservation:** transfer, save, rollback, towing, and destruction preserve authoritative quantities.
2. **Domain identity:** ground, water, air, and orbital prototypes produce distinct operational decisions.
3. **Damage continuity:** component loss changes capability before total failure.
4. **Solo/co-op equivalence:** core missions remain completable with bounded automation or multiple players.
5. **Route causality:** weather, load, access, and reserves change expected and realized travel.
6. **LOD equivalence:** direct and route-sim outcomes remain within declared envelopes.
7. **Orbital correctness:** deterministic maneuver tests match reference propagation tolerances.
8. **Network stability:** prediction corrections remain within profile-specific thresholds.
9. **Persistence:** vehicle ancestry, cargo custody, crew, and damage survive migration.
10. **Recovery:** disabled vehicles support tow, rescue, or salvage without state duplication.

## Final Rule

```text
The runtime must preserve the operational truth of travel even when presentation is compressed.
```
