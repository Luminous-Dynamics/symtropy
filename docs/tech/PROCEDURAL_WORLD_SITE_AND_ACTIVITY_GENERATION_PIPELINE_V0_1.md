---
title: Procedural World, Site, and Activity Generation Pipeline
version: 0.1
status: implementation-spec
scope: deterministic generation of worlds, regions, sites, factions, activities, histories, content selection, validation, and runtime realization
owner: procedural-generation/simulation/content/engineering
related:
  - tech/PROCEDURAL_HISTORY_ENGINE.md
  - tech/PROCEDURAL_FACTION_EVOLUTION.md
  - canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
  - ops/CONTENT_AUTHORING_VALIDATION_AND_PROVENANCE_STANDARD_V0_1.md
---

# Procedural World, Site, and Activity Generation Pipeline

## Owned Question

**How does Symtropy turn deterministic seeds, authored content, physical constraints, history, and current world state into coherent places and activities without generating disconnected lore, impossible infrastructure, repetitive missions, or untestable combinatorial chaos?**

## Core Thesis

Procedural generation is a compiler from causes to playable affordances.

```text
physics constrains geography
geography constrains ecology and movement
history transforms geography
infrastructure creates dependencies
societies interpret dependencies
current pressure creates opportunities
content packages realize the result
validators reject incoherence
```

Generation must produce structured state first. Text, art variation, mission framing, and Field Deck interpretation are derived from that state.

# 1. Generation Products

The pipeline may produce:

```text
planet and region topology
climate and biosphere templates
resource and hazard geography
settlement and infrastructure networks
historical event chains
factions and institutions
sites and interior layouts
art and architectural variation
NPC roles and relationships
activities and contracts
revisit states
```

Not every product is generated at world creation. Some are lazily realized or evolved from simulation.

# 2. Deterministic Input Bundle

```rust
struct GenerationInputBundle {
    galaxy_seed: u64,
    worldline_seed: u64,
    region_seed: Option<u64>,
    content_lock: ContentLock,
    schema_lock: SchemaLock,
    generator_versions: GeneratorVersionSet,
    worldline_profile: WorldlineProfileRef,
    authored_overrides: Vec<AuthoredOverrideRef>,
}
```

The same input bundle must reproduce the same authoritative generated state.

Cosmetic variation may use separate nonauthoritative seeds when it does not affect gameplay, navigation, evidence, or persistence.

# 3. Pipeline Stages

## Stage 0 — Scope and Budget

Resolve target scale, platform budget, content packs, player count, worldline profile, and milestone gates.

## Stage 1 — Physical Substrate

Generate or load:

```text
astronomical conditions
planet shape and gravity
terrain and hydrology
atmosphere and climate
geological hazards
orbital conditions
```

## Stage 2 — Living Substrate

Generate biosphere compatibility, functional guilds, habitat connectivity, native or introduced species, and ecological agency candidates.

## Stage 3 — Historical Pressure

Run or select typed history events that alter:

```text
population
infrastructure
settlement distribution
ecological state
institutional memory
claims and authority
```

## Stage 4 — Infrastructure Network

Place networks according to actual source, sink, route, terrain, capacity, and historical need.

```text
water
power
food
transport
industry
communications
care
archive
defense
```

A facility must connect to at least one dependency and one beneficiary unless it is explicitly abandoned, symbolic, experimental, or Null-drifted.

## Stage 5 — Settlements and Factions

Generate settlements from viable support envelopes and historical actors. Factions derive from institutions, wounds, protected values, resources, and conflicts.

## Stage 6 — Sites

Select sites where dependencies, scars, opportunities, threats, or symbolic history become spatially playable.

## Stage 7 — Activity Sources

Current pressures, NPC intentions, contracts, discoveries, and world-state transitions create opportunity graphs.

## Stage 8 — Presentation Realization

Choose architecture kits, props, damage layers, ambient life, signage, soundscapes, text, NPC casting, and encounter composition.

## Stage 9 — Validation and Repair

Run structural, navigational, causal, content, performance, and narrative validators. Repair within bounded rules or reject the seed/package combination.

# 4. Constraint Graph

```rust
struct GenerationConstraintGraph {
    nodes: Vec<ConstraintNode>,
    edges: Vec<ConstraintEdge>,
}
```

Constraint classes:

```text
physical
ecological
infrastructure
historical
social
architectural
gameplay
accessibility
performance
content compatibility
```

Hard constraints reject. Soft constraints score and guide selection.

# 5. Site Generation Grammar

A site is generated from:

```rust
struct SiteIntent {
    site_role: SiteRole,
    dependencies: Vec<InfrastructureRef>,
    beneficiaries: Vec<ActorRef>,
    history_chain: Vec<HistoryEventId>,
    current_pressure: PressureVector,
    architecture_family: ArchitectureFamilyId,
    gameplay_requirements: GameplayRequirementSet,
    content_budget: SiteBudget,
}
```

Spatial generation proceeds:

```text
functional zones
critical routes
maintenance routes
public/private thresholds
hazard and failure geometry
encounter spaces
alternative paths
accessibility paths
visual landmarks
prop and evidence placement
```

The layout must remain explainable from the site’s function and history.

# 6. Activity Generation

Activities are not randomly selected quest templates.

Inputs:

```text
world pressure
actor goals
available resources
site affordances
relationship state
recent repetition history
player capabilities
worldline conflict profile
```

Output:

```rust
struct GeneratedOpportunity {
    source: OpportunitySource,
    stakes: ConsequenceVector,
    objective_graph: ObjectiveGraph,
    valid_approaches: Vec<ApproachClass>,
    role_slots: Vec<RoleSlot>,
    failure_continuations: Vec<Continuation>,
    reward_vector: RewardVector,
    expiry_or_evolution: OpportunityLifetime,
    provenance: GenerationProvenance,
}
```

Generation must prove that the opportunity changes or reveals world state. Repetition controls compare mechanical structure, setting, cause, and consequence—not only title text.

# 7. Authored and Procedural Boundaries

Use authorship for:

```text
core emotional beats
major characters
critical first-contact precedents
signature architecture
unique puzzles
high-stakes moral framing
voice performance
```

Use procedural systems for:

```text
state combinations
route and logistical variation
site condition
minor actors
pressure-derived opportunities
revisit states
ambient evidence
resource and weather context
```

Hybrid content provides authored modules with procedural bindings, variants, and constraints.

# 8. Generation Provenance

Every generated authoritative object carries:

```text
generator ID and version
seed scope
content package IDs
source history or pressure events
repair or fallback rules applied
validation results
authored overrides
```

This provenance is required for reproduction, bug reports, migrations, and moderation.

# 9. Validators

## 9.1 Physical and Structural

```text
reachable placement
valid terrain or anchor
support and pressure envelope
utility compatibility
```

## 9.2 Network and Infrastructure

```text
source/sink connectivity
capacity plausibility
maintenance access
failure isolation
```

## 9.3 Navigation

```text
player path
NPC path
accessible alternative
combat and evacuation path
vehicle clearance where required
```

## 9.4 Causal

Every scar, lock, faction claim, and objective references valid causes.

## 9.5 Content

```text
asset presence
license and provenance
localization keys
animation and audio coverage
required variants
```

## 9.6 Gameplay

```text
approach viability
failure continuation
solo viability
co-op role fairness
reward validity
anti-softlock
```

## 9.7 Performance

Estimate entity, draw, physics, AI, audio, network, and memory budgets before realization.

# 10. Repair and Fallback

Validators may apply bounded repairs:

```text
move a prop within authored zone
swap a compatible module
add a maintenance route
reduce ambient density
replace an unavailable asset variant
choose alternate opportunity source
```

They may not silently rewrite the historical cause or moral meaning of a site.

If hard validation fails, reject and regenerate from the nearest safe stage rather than patching arbitrary state.

# 11. Golden Seeds and Fuzzing

Maintain:

```text
golden tutorial seeds
golden stress seeds
golden cultural seeds
golden accessibility seeds
known-bad regression seeds
random fuzz corpus
```

Golden seeds are content-locked and reviewed by humans. Fuzz seeds search for crashes, softlocks, impossible networks, repetitive activities, and budget overflow.

# 12. Runtime Generation

Runtime generation is limited to bounded domains:

```text
opportunity creation
revisit state
ambient population
weather variation
resource regeneration or movement
minor site realization
```

Major geography, settlement identity, and canonical history changes occur through explicit simulation or worldline events, not arbitrary rerolls.

# 13. Multiplayer and Persistence

The authoritative shard generates and signs authoritative content state. Clients receive realized state and provenance references.

Generated objects enter checkpoints and journals through stable IDs. Generator updates do not mutate existing authoritative objects unless a migration explicitly does so.

Worldline forks may use new generators for future content while preserving inherited generated state.

# 14. Observability

Developer tools must provide:

```text
seed and content lock
stage outputs
constraint failures
repair actions
site intent graph
activity source and objective graph
budget estimates
asset selections
generation time
```

A generated bug must be reproducible from one exported bundle.

# 15. Acceptance Tests

1. Identical input bundles reproduce byte-equivalent authoritative generation products.
2. Every major site has valid dependencies, beneficiaries, history, and revisit reason.
3. Generated opportunities cite a current world cause and have at least one continuation.
4. Accessibility and solo paths survive generation variation.
5. Invalid content packages fail before world state is committed.
6. Golden seeds remain stable until an intentional versioned update.
7. Fuzzing finds no hard softlocks across the declared seed sample.
8. Runtime generation stays within frame and background-job budgets.
9. Existing worldlines remain stable when generator versions change.
10. Provenance is sufficient to reproduce and inspect any generated object.

## Final Rule

```text
Procedural generation should multiply authored meaning—not replace meaning with quantity.
```
