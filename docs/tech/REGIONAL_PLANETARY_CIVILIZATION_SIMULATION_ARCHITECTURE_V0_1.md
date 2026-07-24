---
title: Regional, Planetary, and Civilizational Simulation Architecture
version: 0.1
status: canonical-draft
scope: simulation layers, fidelity, causality, persistence
related:
  - tech/PROCEDURAL_HISTORY_ENGINE.md
  - tech/PROCEDURAL_FACTION_EVOLUTION.md
  - tech/WORLD_PERSISTENCE_PROTOCOL.md
  - Symtropy Player Cities & Society.md
owner: simulation/engineering/design
---

# Regional, Planetary, and Civilizational Simulation Architecture

## Core Thesis

Symtropy simulates connected worlds through selective fidelity.

The game should model enough causality for places to change meaningfully without attempting full-detail simulation of every person, organism, machine, and transaction everywhere.

## Simulation Layers

### Layer 0 — Embodied Scene

High fidelity:

```text
players
nearby NPCs
combat
physics
tools
vehicles
devices
hazards
```

Update rate: frame or device tick.

### Layer 1 — Active Site

Medium-high fidelity:

```text
site machines
local ecology
occupants
cargo
power and fluid networks
structural condition
```

Update rate: seconds.

### Layer 2 — Settlement

Aggregated but causal:

```text
population groups
production
consumption
labor
care
trust
legitimacy
district condition
culture
```

Update rate: minutes or simulation intervals.

### Layer 3 — Region

Network simulation:

```text
routes
trade
migration
weather fronts
faction posture
ecological corridors
resource geography
warfare
signal
```

Update rate: coarse intervals and events.

### Layer 4 — Planet

Strategic and historical simulation:

```text
climate bands
ocean and watershed state
major polities
orbital infrastructure
biosphere trends
continental logistics
world events
```

Update rate: long intervals and event-driven updates.

### Layer 5 — Worldline

Durable history:

```text
fork ancestry
major precedents
settlement identities
treaties
discoveries
migrations
Confluence
```

Update rate: Chronicle events and asynchronous synchronization.

## Fidelity Promotion

Entities promote to higher fidelity when:

```text
players approach
a mission activates
a threshold is crossed
a major actor intervenes
an event requires physical resolution
```

They demote through state compression after:

```text
stability
distance
inactivity
resolved conflict
```

Compression must preserve:

```text
condition
ownership or stewardship
relationships
active obligations
historical scars
pending risks
```

## Domain Model

### Material

```text
energy
water
food
materials
air
shelter
waste
```

### Productive

```text
labor
tools
fabrication
industry
repair capacity
logistics
knowledge
```

### Living

```text
health
population
species
habitat
soil
pollution
disease
```

### Social

```text
trust
legitimacy
culture
care
inequality
belief
migration
```

### Strategic

```text
safety
territory
military capability
route control
diplomacy
signal
```

### Historical

```text
claims
precedents
debts
archives
founding wounds
worldline ancestry
```

## State Type Rules

Use the appropriate representation.

### Stock

Amount accumulated:

```text
battery charge
stored food
available medicine
```

### Flow

Rate:

```text
power generation
water throughput
cargo movement
migration
```

### Capacity

Maximum safe capability:

```text
fabrication throughput
clinic beds
route tonnage
```

### Condition

Integrity or effectiveness:

```text
bridge condition
soil health
machine reliability
```

### Access

Who can use it and under what conditions.

### Confidence

How well the simulation or actors know the state.

### Memory

What actors believe happened and what durable records exist.

Do not collapse these into one percentage.

## Causal Edge

```rust
struct CausalEdge {
    source: StateRef,
    target: StateRef,
    transform: Transform,
    delay: SimDuration,
    spatial_scope: Scope,
    confidence: f32,
    visibility: Visibility,
    provenance: SourceId,
}
```

## Delay and Hysteresis

Living and social systems need delay.

Examples:

```text
repairing irrigation improves crops later
pollution persists after production stops
trust recovers slower than water flow
militarization remains after threat declines
migration responds to perceived future, not only current stock
```

Hysteresis prevents oscillating identities and emergency modes.

## Events and Continuous State

Continuous simulation creates pressure.

Typed events create history.

Example:

```text
water scarcity rises continuously
ration posture activates
a public seizure occurs
the seizure becomes a Chronicle event
faction memory changes
```

## Actor Interpretation

The same state should generate different interpretations.

```rust
struct Interpretation {
    actor_id: ActorId,
    observed_state: StateRef,
    belief_frame: BeliefFrame,
    confidence: f32,
    proposed_action: Action,
}
```

Simulation truth and social meaning are related but distinct.

## Regional Networks

A region should model graphs for:

```text
transport
power
water
signal
trade
ecology
social affiliation
```

Damage or construction changes graph connectivity.

Graph changes should be visible through:

```text
traffic
NPC schedules
prices
availability
migration
enemy movement
ecological response
```

## Conflict Simulation

Off-screen conflict uses posture and capacity, not frame-level combat.

Resolve through:

```text
force
supply
terrain
intelligence
morale
leadership
objectives
outside intervention
```

Promote decisive battles, rescues, sieges, or infiltrations into playable sites.

## Economy

The economy should combine:

```text
physical goods
capacity
time
risk
access
obligation
information
```

Prices may exist, but they should not erase physical logistics or political control.

## Culture State

Culture is not one harmony score.

Track patterns such as:

```text
ritual participation
artistic production
public joy
mourning burden
intergroup contact
rest
cultural confidence
suppression
```

Culture produces behaviors, spaces, schedules, and memories.

## Planetary Simulation

Planetary state should be coarse but causal.

Model:

```text
climate regions
major watersheds
biosphere integrity
industrial load
large migration
orbital access
major polities
planetary hazards
```

Do not simulate weather cells, individual animals, or every city globally unless active.

## Determinism and Reproducibility

The simulation should support:

```text
seeded generation
versioned rules
event logs
state snapshots
deterministic device transactions
debug replay
```

Real-time physics need not be globally deterministic. Durable outcomes must have clear provenance.

## Debugging Tools

Required visualizations:

```text
regional network graph
state trend lines
pressure heatmap
causal edge inspector
event ancestry
actor interpretation comparison
fidelity tier map
```

A complex simulation without explanation tools will become unmaintainable.

## Firstlight Basin State

The first region needs only a bounded model:

```text
reserve power
water continuity
food cooling
medical stability
fabrication capacity
route connectivity
signal integrity
wetland health
public stress
Null activity
```

Each first-thread outcome changes several variables and one network edge.

## Scale Safety

A system belongs in high fidelity only when it creates a nearby player decision.

Everything else should be represented at the cheapest level that preserves:

```text
future consequence
historical continuity
strategic legibility
```

## Acceptance Tests

The architecture succeeds when:

```text
a local action changes regional behavior
an off-screen pressure creates a playable event
a site can demote and later restore without losing its history
players can understand why a major state changed
the simulation can be replayed and inspected
```
