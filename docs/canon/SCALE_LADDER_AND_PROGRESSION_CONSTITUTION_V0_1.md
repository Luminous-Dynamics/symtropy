---
title: Scale Ladder and Progression Constitution
version: 0.1
status: canonical
scope: progression, technology eras, world expansion
owner: design/systems
related:
  - PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
  - SYMTROPY_GAME_CONSTITUTION_V0_6.md
---

# Scale Ladder and Progression Constitution

## Core Thesis

Symtropy progression is not a ladder from weak character to powerful character.

It is a widening sphere of responsibility and capability.

```text
You first keep yourself alive.
Then you keep a system alive.
Then a settlement.
Then a region.
Then a world.
```

Each expansion of scale must unlock new play while preserving lower-scale relevance.

## Progression Axes

Progression occurs across six partially independent axes:

```text
embodied capability
material capability
mobility
knowledge
institutional capability
historical reach
```

A settlement may be industrially advanced but politically fragile. A player may possess rare knowledge without the logistics to apply it.

## Era 0 — Ruin Literacy

Fantasy:

```text
Learn to survive and read what remains.
```

Unlocks:

```text
hand tools
basic weapon
Field Deck
salvage
field medicine
portable shelter
walking and light vehicle routes
```

Representative play:

```text
ruin exploration
small hostile encounters
hazard navigation
component recovery
first machine contact
```

## Era 1 — Local Metabolism

Fantasy:

```text
Make one settlement capable of continuity.
```

Unlocks:

```text
workshop
power
water
food
care
storage
basic fabrication
local automation
small rovers
public meeting spaces
```

Representative play:

```text
construction
settlement defense
resource chains
local culture
labor and care organization
```

## Era 2 — Regional Network

Fantasy:

```text
Connect places that cannot survive alone.
```

Unlocks:

```text
roads
bridges
convoys
rail or river transport
communication relays
regional trade
specialized settlements
mobile clinics
repair fleets
```

Representative play:

```text
route planning
convoy warfare
migration
regional diplomacy
resource specialization
weather response
```

## Era 3 — Industrial Ecology

Fantasy:

```text
Build abundance without making a dead world.
```

Unlocks:

```text
advanced materials
large factories
robotics
ecosystem engineering
heavy vehicles
research campuses
city districts
large energy systems
```

Representative play:

```text
industrial design
pollution and restoration
labor politics
city-scale construction
machine rights
large raids
```

## Era 4 — Planetary Coordination

Fantasy:

```text
Act at the scale of climate, continents, and planetary society.
```

Unlocks:

```text
orbital infrastructure
continental logistics
planetary observation
climate adaptation
oceanic systems
intercontinental diplomacy
large fleets
biosphere treaties
```

Representative play:

```text
planetary crises
war and federation
major ecological recovery
orbital salvage
global migration
```

## Era 5 — Interplanetary Civilization

Fantasy:

```text
Make distance into a new form of society.
```

Unlocks:

```text
spaceports
orbital habitats
lunar and asteroid industry
Mars transfer systems
deep-space logistics
habitat charters
light-delay institutions
```

Representative play:

```text
ship-as-society management
rescue jurisdiction
closed-loop survival
space labor politics
alien precursor investigation
```

## Era 6 — Xeno Translation

Fantasy:

```text
Build relations across incompatible bodies and worlds.
```

Unlocks:

```text
translation infrastructures
nonhuman treaties
alien-compatible habitats
xeno materials
ecological diplomacy
multi-species settlements
```

Representative play:

```text
first contact
category uncertainty
habitat negotiation
shared construction
cross-species conflict
```

## Era 7 — Worldline Civilization

Fantasy:

```text
Decide what histories may meet.
```

Unlocks:

```text
worldline migration
fork ancestry
Confluence
cross-world trade
historical reconciliation
planetary translation
```

Representative play:

```text
fork diplomacy
conflicting precedent
identity continuity
worldline-scale projects
```

## Era Gate Standard

An era unlock requires more than research points.

```rust
struct EraGate {
    material_capacity: Vec<Capability>,
    knowledge_proofs: Vec<Discovery>,
    infrastructure_preconditions: Vec<Infrastructure>,
    social_preconditions: Vec<Institution>,
    risk_preconditions: Vec<SafetyPractice>,
    chronicle_precedents: Vec<HistoricalProof>,
}
```

Examples:

A spaceport requires:

```text
precision fabrication
fuel or launch energy
weather observation
rescue doctrine
cargo logistics
trained crews
airspace and exclusion law
```

A machine steward requires:

```text
repair capacity
bounded autonomy
testimony interface
appeal process
override safety
public precedent
```

## Horizontal Diversity

Players should not need to climb every branch.

A regional civilization may specialize as:

```text
trade federation
ecological sanctuary
industrial republic
mobile convoy culture
archive civilization
orbital launch society
machine stewardship polity
```

Specialization creates dependency and diplomacy rather than a single optimal tree.

## Prestige Without Obsolescence

Higher eras should make earlier objects storied, upgraded, and repurposed.

A first rover may become:

```text
a convoy scout
a museum object
a ceremonial vehicle
a heavily rebuilt expedition machine
a source of standardized parts
```

A first settlement may remain politically important even after cities and orbital habitats exist.

## World Visibility

The next scale should be visible before it is reachable.

Early Firstlight Basin should show:

```text
orbital wreckage
distant launch traces
regional radio traffic
old rail alignments
unreachable mountain stations
alien or anomalous signals
```

Horizon creates ambition. It must not create premature production scope.

## Anti-Collapse Rule

No era may require every lower simulation to run at full fidelity everywhere.

Use resolution tiers:

```text
active local simulation
regional aggregate simulation
historical event simulation
dormant summarized state
```

Scale is achieved through selective fidelity, not universal detail.

## Final Progression Test

An unlock is meaningful when it changes at least two of:

```text
where players can go
what they can build
what risks they can survive
who they can cooperate with
what kinds of society can exist
what history can be created
```
