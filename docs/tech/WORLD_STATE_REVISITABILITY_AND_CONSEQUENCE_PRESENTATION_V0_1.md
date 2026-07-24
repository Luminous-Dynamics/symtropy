---
title: World-State Revisitability and Consequence Presentation
version: 0.1
status: implementation-spec
scope: visible state change, site variants, revisit loops, causal presentation, content authoring budgets
owner: world/design/art/simulation
related:
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
---

# World-State Revisitability and Consequence Presentation

## Owned Question

**How does the world visibly, audibly, spatially, socially, and mechanically demonstrate that player actions and simulation changes persist?**

## Core Thesis

A consequence that exists only in a database is not yet a player experience.

```text
state must change
change must be perceivable
causes must be inferable
revisiting must reveal life after the objective
```

# 1. Consequence Channels

Every major outcome should use at least three channels where appropriate.

```text
geometry and access
machine behavior
lighting and power
soundscape
NPC routine
population and traffic
inventory and trade
vegetation and animal behavior
signage and public record
dialogue and rumor
Field Deck evidence
weather or pollution
construction and repair marks
```

# 2. State Variant Classes

## Immediate

Seconds to minutes:

```text
machine starts
alarm stops
route opens
crowd reacts
smoke changes
```

## Short-Term

Hours to days:

```text
cargo arrives
patients move
work crews appear
prices shift
faction presence changes
```

## Seasonal

Days to months:

```text
ecological succession
construction completion
migration
institutional reform
maintenance decay
```

## Historical

Persistent era-defining change:

```text
memorial
new law
ruin preserved or erased
settlement founded
worldline fork
species treaty
```

# 3. Site State Model

```rust
struct SitePresentationState {
    functional_state: FunctionalState,
    occupancy_state: OccupancyState,
    authority_state: AuthorityState,
    ecological_state: EcologicalState,
    damage_state: DamageState,
    memory_markers: Vec<MemoryMarker>,
    active_routines: Vec<RoutineId>,
    audiovisual_profile: PresentationProfile,
}
```

Do not author one monolithic variant for every combination. Use composable layers with conflict rules.

# 4. State Layer Priority

Example priorities:

```text
catastrophic physical state overrides decorative prosperity
quarantine modifies access and crowd behavior
faction occupation changes signage and patrols
power state changes lighting and device availability
ecological state changes growth, water, and fauna
time and weather modify all layers
```

# 5. Revisit Reasons

A site should become worth revisiting through:

```text
new function
new route
new people
new knowledge
new conflict
new maintenance need
cultural event
ecological change
construction progress
relationship request
```

Do not repopulate every cleared space with generic enemies merely to create repeatability.

# 6. Causal Legibility

Players should be able to answer:

```text
what changed
roughly why it changed
who benefited
what remains unresolved
what might happen next
```

Tools:

```text
before/after environmental contrast
NPC explanation from partial perspective
Field Deck causal trace
public notices
visible supply movement
machine logs
Chronicle summary
```

Avoid omniscient banners that flatten uncertainty.

# 7. NPC Return Behavior

NPCs should:

```text
occupy repaired spaces
adopt new routes
perform new work
celebrate or protest
teach others
mourn losses
complain about side effects
```

Named NPC memory and ambient population behavior should agree at the broad level without requiring identical dialogue.

# 8. Maintenance and Decay

Success is not permanent by magic.

Maintenance signals:

```text
wear
consumable depletion
inspection schedule
labor fatigue
weather damage
software drift
political neglect
```

Maintenance should create periodic judgment, not repetitive chore spam.

Use thresholds, automation, service contracts, and delegation so mature infrastructure does not demand constant manual clicking.

# 9. Content Budget Standard

Every representative major site should support:

```text
1 baseline state
1 failure or hostile state
2 materially distinct resolution states
1 delayed revisit layer
1 faction or authority overlay
1 ecological or weather overlay where relevant
```

Not all combinations require bespoke assets.

# 10. Art and Audio Requirements

State changes should alter:

```text
silhouette or traversal when major
animated machinery
surface wear and repair marks
light rhythm and color temperature
ambient population density
mechanical and ecological sound layers
music only when culturally or dramatically justified
```

# 11. Persistence Boundary

Local cosmetic debris may be summarized or regenerated.

Persist:

```text
constructed and destroyed infrastructure
major access changes
named deaths
public laws and ownership
settlement function
mission consequence bindings
important ecological state
memory markers
```

# 12. Debugging

Required tools:

```text
state-layer inspector
causal event trace
variant conflict warning
before/after teleport
simulated time advance
NPC routine overlay
presentation coverage report
```

# 13. Seedworks Minimum

For each of the five regional opening threads, provide:

```text
one immediate response
one settlement-level response
one delayed revisit
one unintended side effect
one NPC memory reaction
one Field Deck causal explanation
```

# 14. Acceptance Evidence

The system succeeds when:

```text
players notice important changes without opening a ledger
players revisit voluntarily
players can connect visible changes to earlier actions
sites feel inhabited after resolution rather than completed and abandoned
state variants remain readable under weather, faction, and damage overlays
maintenance creates decisions without becoming busywork
```
