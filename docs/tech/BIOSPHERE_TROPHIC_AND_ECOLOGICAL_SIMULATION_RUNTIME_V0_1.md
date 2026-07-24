---
title: Biosphere, Trophic, and Ecological Simulation Runtime
version: 0.1
status: implementation-spec
scope: ecological data model, trophic simulation, habitat fields, species proxies, interventions, terraforming, LOD, persistence, and observability
owner: simulation/ecology/engineering
related:
  - canon/LIVING_WORLDS_ECOLOGY_AND_TERRAFORMATION_CONTRACT_V0_1.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/Symtropy Design Doc Earth Species.md
  - canon/SCIENCE_RESEARCH_AND_DISCOVERY_CONTRACT_V0_1.md
  - lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
  - tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Biosphere, Trophic, and Ecological Simulation Runtime

## Owned Question

**What bounded, deterministic, inspectable runtime can produce ecological causality from organisms to planets without simulating every individual or turning ecosystems into arbitrary scripted state changes?**

## Core Thesis

The ecological runtime is a layered field-and-population simulation.

It tracks conservation-relevant flows, functional guilds, habitat compatibility, disturbance, and selected visible organisms. It does not attempt to reproduce real-world ecology at organism-level resolution.

```text
Render individuals where embodiment matters.
Simulate populations where abundance matters.
Simulate functions where complexity would otherwise explode.
Preserve causal events at every scale.
```

# 1. Authority Boundary

The ecological runtime owns:

```text
habitat fields
functional-guild state
selected population cohorts
disturbance and recovery
species introductions and removals
ecological thresholds
regional and planetary aggregate deltas
```

It does not own:

```text
NPC beliefs about ecology
market prices
charter legitimacy
combat damage resolution
scientific certainty
Chronicle interpretation
```

Those systems consume ecological observations or events through typed interfaces.

# 2. Spatial Hierarchy

```rust
struct EcologicalRegion {
    region_id: RegionId,
    parent_id: Option<RegionId>,
    resolution: EcologicalResolution,
    cells: Vec<EcoCellId>,
    corridors: Vec<CorridorId>,
    hydrology_links: Vec<FlowLinkId>,
    atmospheric_links: Vec<FlowLinkId>,
    aggregate_state: EcoStateVector,
}
```

Supported resolutions:

```text
Patch        — meters to hundreds of meters
Landscape    — kilometers
Biome        — tens to thousands of kilometers
Planetary    — global aggregate and circulation graph
```

Cells are not necessarily square terrain tiles. Wetlands, caves, reefs, orbital habitats, and atmospheric layers may use different adjacency graphs.

# 3. Habitat Fields

Each cell stores bounded fields.

```rust
struct HabitatFields {
    temperature: Fixed,
    water_availability: Fixed,
    salinity: Fixed,
    oxygen_or_relevant_oxidant: Fixed,
    acidity: Fixed,
    substrate_depth: Fixed,
    nutrient_vector: SmallVec<Fixed>,
    toxin_vector: SmallVec<Fixed>,
    radiation: Fixed,
    light_or_energy_flux: Fixed,
    structural_cover: Fixed,
    disturbance: Fixed,
    human_or_machine_pressure: Fixed,
}
```

Use fixed-point or carefully specified deterministic arithmetic for authoritative simulation. Continuous-looking values may be quantized internally.

# 4. Functional Guilds

Most biodiversity is represented through functional guilds.

```rust
struct FunctionalGuildState {
    guild_id: GuildId,
    biomass: Fixed,
    activity: Fixed,
    reproductive_capacity: Fixed,
    genetic_or_functional_diversity: Fixed,
    disease_pressure: Fixed,
    adaptation_fit: Fixed,
    extinction_debt: Fixed,
}
```

Typical guilds:

```text
primary producers
decomposers
nitrogen or substrate cyclers
filter feeders
browsers and grazers
small predators
apex regulators
pollinators and dispersers
biofilm formers
reef or structure builders
pathogen and parasite guilds
```

Guilds exchange matter and modify fields through coefficient tables validated per biome template.

# 5. Visible Species Proxies

A curated species becomes a visible proxy when it has at least one of:

```text
strong player embodiment
cultural importance
unique ecological function
care or welfare gameplay
agency or personhood relevance
high diagnostic value
```

```rust
struct PopulationCohort {
    species_id: SpeciesId,
    region_id: RegionId,
    count_or_biomass: Fixed,
    age_or_stage_distribution: SmallVec<Fixed>,
    health: Fixed,
    reproduction: Fixed,
    movement_policy: MovementPolicy,
    welfare_state: WelfareVector,
    provenance: PopulationProvenance,
}
```

Individual entities are instantiated only near players, during care, hunting, tracking, capture, conflict, reproduction events, or authored encounters.

Aggregate cohorts and instantiated individuals must reconcile through explicit spawn and merge transactions.

# 6. Update Pipeline

Each ecological tick performs:

```text
1. Apply queued disturbances and interventions.
2. Move water, atmosphere, heat, nutrients, and contaminants.
3. Compute habitat fit for guilds and cohorts.
4. Resolve production, consumption, decomposition, predation, and disease.
5. Resolve movement and corridor use.
6. Update diversity, adaptation, and extinction debt.
7. Evaluate thresholds and regime shifts.
8. Emit observations, warnings, and causal events.
9. Aggregate upward to landscape, biome, and planetary state.
```

Different layers use different cadences.

```text
active patch:       seconds to minutes
nearby landscape:   minutes to hours
regional background: hours to days
biome:              days to seasons
planetary:          seasons to years
```

A faster cadence may interpolate presentation but must not create additional ecological production.

# 7. Flows and Conservation

Tracked flow classes include:

```text
water
carbon or local structural element
nitrogen or local nutrient analogues
phosphorus or limiting nutrients
energy availability
biomass
selected contaminants
```

Not every world uses Earth biochemistry. Biome templates define which conserved or limiting quantities matter.

Interventions must express source, sink, transformation, and loss.

```rust
struct EcologicalTransaction {
    transaction_id: EventId,
    source: Option<EcoStoreId>,
    destination: Option<EcoStoreId>,
    material: EcoMaterialId,
    quantity: Fixed,
    efficiency: Fixed,
    waste_products: SmallVec<(EcoMaterialId, Fixed)>,
    cause: CauseRef,
}
```

# 8. Disturbance and Regime Shift

Disturbances include:

```text
fire
flood
drought
storm
harvest
construction
pollution
warfare
invasive release
disease
terraforming pulse
machine-ecology intervention
```

A regime shift requires:

```text
threshold crossed
persistence or hysteresis condition
causal contributors
new attractor or recovery behavior
visible state transition
```

Do not use a single random roll to turn healthy habitat into wasteland.

# 9. Introductions, Invasions, and Quarantine

Species introduction is a staged transaction:

```text
proposal
risk model
contained trial
observation window
replication or expansion
monitoring
containment or recall
```

Runtime state:

```rust
struct IntroductionState {
    introduction_id: IntroductionId,
    organism: SpeciesOrGuildId,
    source_biosphere: BiosphereId,
    target_region: RegionId,
    stage: IntroductionStage,
    escape_probability: Fixed,
    compatibility_uncertainty: Fixed,
    containment_capacity: Fixed,
    observed_effects: Vec<EffectEstimate>,
}
```

Cross-biosphere introductions default to high uncertainty and strict containment.

# 10. Alien and Distributed Agency

The runtime may flag agency hypotheses but cannot declare personhood autonomously.

Potential evidence:

```text
nonrandom corrective action
persistent boundary defense
symbolic or patterned response
memory across disturbances
selective reciprocity
costly signaling
negotiation-like state change
```

```rust
struct AgencyEvidenceState {
    subject_id: EcologicalSubjectId,
    hypotheses: Vec<AgencyHypothesis>,
    evidence_refs: Vec<EventId>,
    confidence: Fixed,
    category_violence_risk: Fixed,
    authorized_classification: AgencyClassification,
}
```

Scientific and civic systems own classification and rights consequences.

# 11. Terraforming Runtime

Terraforming is represented as coupled programs, not one action.

```rust
struct TerraformingProgram {
    program_id: ProgramId,
    target_conditions: ConditionEnvelope,
    intervention_network: Vec<InterventionId>,
    energy_budget: Quantity,
    material_budget: Quantity,
    time_horizon: SimDuration,
    monitoring_network: Vec<SensorId>,
    abort_conditions: Vec<Condition>,
    affected_agencies: Vec<SubjectId>,
    uncertainty: UncertaintyModel,
}
```

Planetary updates must preserve lag, spatial heterogeneity, and the possibility of overshoot.

# 12. System Interfaces

Outputs:

```text
EcoObservation
EcoThresholdCrossed
PopulationChanged
HabitatConnectivityChanged
ContaminationMoved
DiseaseOutbreak
IntroductionEscaped
RegimeShiftStarted
RegimeShiftStabilized
AgencyEvidenceChanged
TerraformingMilestone
```

Consumers:

```text
Field Deck
science and research
settlement metabolism
markets and logistics
NPC cognition
mission grammar
faction evolution
Chronicle
worldline persistence
```

# 13. Simulation LOD

## LOD 0 — Embodied

Visible organisms, local fluids, player tools, immediate hazards.

## LOD 1 — Patch

Detailed habitat fields, cohorts, and short flow graph.

## LOD 2 — Landscape

Aggregated guilds, corridors, hydrology, and disturbance propagation.

## LOD 3 — Biome

Seasonal state, large migrations, climate fit, and regional pressure.

## LOD 4 — Planetary

Global cycles, biosphere envelopes, terraforming programs, and extinction or recovery trends.

Transitions must conserve represented biomass and retain high-importance individuals or populations.

# 14. Persistence and Multiplayer

Authoritative ecological state belongs to the regional simulation shard. Important deltas enter worldline journals.

Persist:

```text
region aggregate states
cohort provenance
introductions
regime shifts
extinctions and recoveries
agency evidence
terraforming programs
high-importance organisms
```

Do not persist every interpolated individual movement.

Player clients may predict presentation but not authoritative population change, species release, or regime shifts.

# 15. Observability

Developer tools must expose:

```text
cell field overlays
guild flow tables
population provenance
causal contributors to thresholds
introduction spread tree
LOD transitions
mass-balance residuals
regime-shift hysteresis
```

Player-facing explanations should expose observations and probable causes according to Field Deck capability, not developer truth.

# 16. Performance Budgets

Initial representative target:

```text
active detailed patches:          <= 32
functional guilds per patch:      <= 24
visible cohorts per active patch: <= 16
instantiated ecological entities: content-budgeted and pooled
background landscape updates:     amortized across frames
planetary update:                 asynchronous deterministic job
```

Every new ecological variable must name its consumer and measurable gameplay effect.

# 17. Acceptance Tests

1. **Conservation:** closed test systems remain within configured mass-balance tolerance.
2. **Causality:** removing a contaminant source changes downstream state through the flow graph rather than a scripted reward.
3. **Hysteresis:** a regime-shifted system does not instantly recover when one variable crosses back.
4. **LOD equivalence:** detailed and aggregated simulations remain within declared outcome envelopes.
5. **Determinism:** identical seeds, inputs, content locks, and tick schedules produce identical authoritative deltas.
6. **Introduction safety:** staged release, escape, containment, and recall paths are all testable.
7. **Cross-system effect:** an ecological transition changes at least two consuming systems.
8. **Persistence:** save, migration, and rollback preserve cohort provenance and regime-shift causality.
9. **Agency humility:** the runtime reports evidence without silently assigning legal personhood.
10. **Budget:** representative ecology remains within the allocated frame and background-job budgets.

## Final Rule

```text
Simulate enough life for consequences to propagate.
Do not simulate so much life that no one can understand why the world changed.
```
