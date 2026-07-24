---
title: Habitat Metabolism, Life Support, and Population Runtime
version: 0.1
status: implementation-spec
scope: habitat stocks and flows, atmospheric control, water, food, waste, thermal balance, radiation, maintenance, population cohorts, households, ecological loops, and failure propagation
owner: engineering/simulation/ecology/NPC
implements:
  - ../canon/CLOSED_LOOP_HABITATS_GENERATION_SHIPS_AND_SETTLEMENT_CONTINUITY_CONTRACT_V0_1.md
authority_boundary: owns authoritative habitat physical state and bounded population aggregation; does not own civic legitimacy, private cognition, relationship consent, or vehicle trajectory
related:
  - BIOSPHERE_TROPHIC_AND_ECOLOGICAL_SIMULATION_RUNTIME_V0_1.md
  - STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md
  - BODY_HEALTH_TRAUMA_AND_RECOVERY_RUNTIME_V0_1.md
  - DEMOGRAPHY_GENERATIONAL_AND_CULTURAL_EVOLUTION_RUNTIME_V0_1.md
  - SIMULATION_SCALE_PERFORMANCE_AND_GRACEFUL_DEGRADATION_BUDGETS_V0_1.md
---

# Habitat Metabolism, Life Support, and Population Runtime

## Purpose

This runtime models a habitat as a coupled metabolism rather than a bundle of unrelated resource bars.

It must answer:

```text
What matter and energy exist?
Where are they stored?
Which processes transform them?
Which bodies depend on them?
What maintenance prevents drift?
Which failures are locally survivable?
Which consequences persist after recovery?
```

## Core Invariants

1. Matter is conserved except for declared imports, exports, leakage, venting, or transformation.
2. Energy inputs, storage, work, and waste heat are explicit.
3. Population demand derives from actual inhabitants and body profiles.
4. Aggregation cannot delete named people, rights, households, or private state.
5. Life-support failure propagates through physical dependencies rather than scripted punishment.
6. Background simulation preserves thresholds, expiry, custody, and causal evidence.

# 1. Habitat Graph

```rust
struct HabitatState {
    habitat_id: HabitatId,
    zones: Vec<HabitatZone>,
    conduits: Vec<ConduitEdge>,
    processes: Vec<ProcessNode>,
    stores: Vec<MaterialStore>,
    power_bus: PowerNetworkId,
    thermal_network: ThermalNetworkId,
    population: PopulationState,
    ecology: HabitatEcologyState,
    maintenance: MaintenanceState,
    authority: HabitatAuthorityRefs,
}
```

Zones may be:

```text
habitation
agriculture
industrial
medical
storage
radiation shelter
waste processing
docking
machine-only
alien-compatible
vacant or construction
```

# 2. Material Stocks

Tracked stock families:

```text
oxygen
nitrogen and buffer gases
carbon dioxide
water by quality class
food biomass
nutrient elements
waste solids
waste liquids
industrial feedstocks
medical supplies
coolants
microbial cultures
seed and genetic reserves
```

Each stock has:

```rust
struct MaterialStock {
    material: MaterialId,
    quantity: FixedPoint,
    quality: QualityVector,
    contamination: ContaminationState,
    temperature: Temperature,
    pressure: Option<Pressure>,
    custody: CustodyId,
    location: StoreId,
}
```

Quality cannot be reduced to quantity. Water may exist but be unsafe. Food may contain calories but lack required nutrients.

# 3. Atmospheric Runtime

Atmospheric state is zone-specific:

```rust
struct AtmosphereState {
    pressure: Pressure,
    partial_pressures: GasVector,
    humidity: Ratio,
    aerosols: AerosolVector,
    contaminants: ContaminantVector,
    temperature: Temperature,
    circulation_rate: FlowRate,
}
```

Processes include:

```text
breathing
photosynthesis
scrubbing
combustion
industrial emissions
leakage
fire suppression
microbial activity
alien atmospheric exchange
```

Warnings distinguish:

```text
measurement
inference
prediction
unknown sensor gap
```

# 4. Water Runtime

Water classes:

```text
potable
hygiene
agricultural
industrial
coolant
wastewater
contaminated
biological or alien medium
```

Flows include:

```text
consumption
sanitation
crop transpiration
condensation
filtration
distillation
membrane separation
leakage
storage loss
export
```

Purification creates concentrated waste or energy cost; it never deletes contamination.

# 5. Food and Nutrient Runtime

Food is represented by functional profiles:

```text
energy
protein
fat
carbohydrate
micronutrients
fiber
cultural suitability
allergen or compatibility
storage stability
```

Production processes include:

```text
plant growth
algae
fungal culture
fermentation
cell culture
animal or symbiotic systems
imported stores
```

The runtime preserves planting delay, crop failure, seed stock, labor, light, water, nutrients, and ecological dependencies.

# 6. Waste and Circularity

Waste categories remain inputs with hazards:

```text
organic
human biological
industrial
chemical
radioactive
electronic
medical
alien or unknown
```

Recycling efficiency is process-specific and degrades with contamination, wear, skill, and missing catalysts.

Closed-loop score is diagnostic only. It must not imply perfect closure.

# 7. Thermal and Radiation Runtime

Heat sources:

```text
people
machines
reactors
solar gain
lighting
industry
friction
fire
```

Heat sinks:

```text
radiators
phase-change storage
heat exchangers
planetary ground or atmosphere
exported mass
```

Radiation state tracks:

```text
external environment
shielding geometry
storm events
accumulated dose
hot materials
reactor exposure
medical dose
```

Sheltering changes occupancy, privacy, work, and care—not only dose.

# 8. Population State

```rust
struct PopulationState {
    named_agents: Vec<NamedAgentRef>,
    cohorts: Vec<PopulationCohort>,
    households: Vec<HouseholdRef>,
    visitors: Vec<VisitorRef>,
    dependents: Vec<DependentRef>,
    body_profiles: BodyProfileDistribution,
}
```

Cohorts may aggregate ordinary background residents by:

```text
life stage
body/environment requirement
household and community
profession or skill family
care need
legal status
cultural or language community
```

Cohorts never replace named agents in active relationships, offices, incidents, succession, or authored content.

## Demand Calculation

Demand is derived from:

```text
body profile
activity
health
life stage
pregnancy where applicable
environmental adaptation
work exposure
care requirement
```

The runtime must not infer moral worth or citizenship from resource demand.

# 9. Households and Private Space

Household state links to:

```text
assigned volume
sleeping capacity
food preparation
sanitation access
care obligations
private storage
communication privacy
```

Ordinary operational telemetry exposes aggregated environmental need, not private relationship or medical details.

# 10. Habitat Ecology

```rust
struct HabitatEcologyState {
    crop_guilds: Vec<CropGuild>,
    decomposer_guilds: Vec<GuildState>,
    pollination: Option<GuildState>,
    microbiomes: Vec<MicrobiomeState>,
    pathogens: Vec<PathogenState>,
    companion_species: Vec<SpeciesCohort>,
    invasive_pressure: f32,
    ecological_resilience: f32,
}
```

Ecology interacts with air, water, food, waste, health, and culture.

# 11. Maintenance Runtime

Every process and conduit has:

```text
condition
calibration
contamination
consumable life
spare-part requirement
maintenance interval
skill requirement
inspection evidence
failure modes
```

Maintenance debt accumulates causally.

```rust
struct MaintenanceTask {
    asset: AssetId,
    action: MaintenanceAction,
    due_window: TimeInterval,
    labor: SkillRequirement,
    parts: Vec<MaterialRequirement>,
    safety_state: SafetyRequirement,
    consequence_if_deferred: ConsequenceModel,
}
```

# 12. Failure Propagation

Failures occur through graphs:

```text
power loss → circulation loss → local CO2 increase → fatigue → work slowdown
water contamination → crop damage → nutrient deficit → medical load
radiator damage → thermal throttling → food storage loss → political conflict
sensor drift → false confidence → delayed intervention
```

The runtime records the causal chain for player explanation.

# 13. Emergency Modes

Emergency modes may temporarily:

```text
isolate zones
reduce activity
shift occupants
prioritize power
open reserve stocks
change ventilation
activate sheltering
```

Activation requires evidence and authority. Expiry is mandatory. Emergency mode never silently changes citizenship, ownership, reproductive rights, or private-data access.

# 14. Births, Deaths, Migration, and Long Duration

Population transitions consume or release real capacity over time.

Birth requires care and household state; it is not triggered solely by population targets.

Death changes:

```text
household
skills
care burden
culture
succession
resource demand
memorial and Chronicle state
```

Migration moves named people, households, records, medical needs, possessions, and obligations through actual transport.

# 15. Generation-Ship Mission Runtime

A generation ship tracks:

```rust
struct MissionContinuityState {
    founding_mission: MissionDocumentId,
    current_interpretation: MissionInterpretationId,
    renewal_due: SystemEpoch,
    destination_models: Vec<DestinationModel>,
    redirect_options: Vec<TrajectoryOption>,
    dissent_records: Vec<RecordId>,
    active_mandate: Option<MandateId>,
}
```

Mission renewal is a civic transaction. It cannot be inferred from operational continuity.

# 16. LOD Model

## LOD 0 — Active Zone

Per-process flows, local atmosphere, agents, maintenance, tools, and visible failures.

## LOD 1 — Habitat

Zone-level stocks and flows, named agents, household constraints, scheduled tasks.

## LOD 2 — Distant Habitat

Daily or shift-level integration preserving thresholds, cohorts, named continuity, maintenance debt, and incidents.

## LOD 3 — Long Absence

Event-driven metabolism with conservative bounds, critical transitions, succession, and recorded uncertainty.

LOD changes may not create or destroy matter, people, rights, skill, or authority.

# 17. Determinism and Numeric Stability

Use fixed-point or controlled deterministic arithmetic for conserved stocks and scheduled processes where replay matters.

Every integration step records:

```text
start stocks
process transformations
imports and exports
losses
end stocks
residual
```

Residuals outside tolerance fail the simulation audit.

# 18. Representative Fixture

The first fixture contains:

```text
120 residents
8 named agents
24 households
one agricultural loop
one water-recovery loop
one atmosphere loop
one industrial zone
one medical bay
one radiation shelter
one docking interface
```

Thirty simulated days include:

```text
routine maintenance
one specialist injury
one sensor drift
one radiator fault
one migrant arrival request
one radiation storm
one emergency-power activation and expiry
```

Acceptance requires:

1. Stock residuals remain within declared tolerance.
2. Sensor drift creates uncertainty rather than secret truth.
3. Specialist injury affects capability but not instant total collapse when redundancy exists.
4. Households move physically to shelter.
5. Privacy boundaries survive operational monitoring.
6. Migration acceptance checks real capacity.
7. Emergency authority expires and is reviewed.
8. Save/load and LOD transitions preserve population and material state.

# 19. Performance Budgets

The representative habitat must declare budgets for:

```text
active process nodes
flow edges
named agents
cohorts
microbial guilds
maintenance tasks
causal traces
network replication
save growth per simulated year
```

Graceful degradation reduces visual and fine ecological detail before it removes conservation, named continuity, safety, privacy, or authority.

## Final Rule

> **The habitat runtime may approximate detail. It may never approximate away the air, the people, or the obligations that make the habitat real.**
