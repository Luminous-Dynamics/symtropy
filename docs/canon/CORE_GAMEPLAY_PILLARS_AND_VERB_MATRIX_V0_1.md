---
title: Core Gameplay Pillars and Verb Matrix
version: 0.1
status: canonical
scope: moment-to-moment, session, and long-loop gameplay
owner: design/gameplay
related:
  - SYMTROPY_GAME_CONSTITUTION_V0_6.md
  - PLAYER_EXPERIENCE_AND_SESSION_RHYTHM_CONTRACT_V0_1.md
---

# Core Gameplay Pillars and Verb Matrix

## Purpose

This document converts the broad Symtropy vision into playable activity.

It exists to prevent two opposite failures:

```text
many systems with no compelling minute-to-minute game
one polished activity that shrinks the entire vision around itself
```

## The Six Playable Pillars

### Traverse

```text
walk
climb
swim
crawl
drive
sail
fly
navigate
route
escort
```

Traversal creates geography, exposure, timing, cargo risk, and discovery.

### Investigate

```text
scan
sample
listen
track
compare
decode
map
question
observe
translate
```

Investigation converts uncertainty into options. It should rarely produce a single omniscient answer.

### Transform

```text
repair
cut
weld
fabricate
build
demolish
reroute
program
automate
cultivate
```

Transformation changes physical capability and landscape.

### Contend

```text
aim
shoot
strike
block
evade
suppress
breach
defend
disable
rescue
```

Contending covers combat and dangerous intervention. It must have strong feel independent of its narrative context.

### Coordinate

```text
carry
load
trade
assign
signal
schedule
teach
share
command
negotiate
```

Coordination makes cooperation, logistics, and social organization playable.

### Belong

```text
rest
eat
celebrate
mourn
care
befriend
argue
vote
ritualize
remember
```

Belonging gives civilization emotional value and makes loss matter.

## Pillar Pairings

Most strong Symtropy activities combine at least two pillars.

| Activity | Primary Pairing | World Result |
|---|---|---|
| Salvage expedition | Traverse + Investigate | New materials, map knowledge, historical clues |
| Convoy defense | Traverse + Contend | Trade continuity, casualties, route security |
| Factory redesign | Investigate + Transform | New production capability and new risks |
| Alien contact | Investigate + Coordinate | Translation confidence, boundaries, treaty possibility |
| Settlement festival | Coordinate + Belong | Relationship change, fatigue, cultural memory |
| Habitat breach | Contend + Transform | Lives saved, structural damage, emergency precedent |
| Ecological restoration | Investigate + Transform | Delayed biome change and land-use conflict |
| City founding | Transform + Belong | New district, identity, migration, obligations |

Activities that use only one pillar should be brief, highly skillful, or intentionally restful.

## Session Rhythm

A typical 45–90 minute session should include:

```text
orientation
commitment
travel or preparation
uncertainty
skillful action
consequence
return or transition
```

Not every session requires combat, a vote, or a large repair.

A healthy rotation includes:

```text
expedition session
construction session
defense session
trade/logistics session
social/cultural session
research/translation session
```

## Player Role Matrix

Roles are practices, not classes.

| Role | Core verbs | Unique leverage | Failure pressure |
|---|---|---|---|
| Explorer | navigate, map, discover | Routes and unknown sites | Isolation, exposure, bad interpretation |
| Engineer | diagnose, build, automate | Reliable physical systems | Maintenance debt, cascading failure |
| Fighter | breach, defend, rescue | Survival under hostile pressure | Injury, collateral damage, escalation |
| Logistician | route, load, schedule | Throughput and preparedness | Bottlenecks, theft, convoy loss |
| Scientist | sample, model, compare | New knowledge and safe experimentation | False certainty, contamination |
| Ecologist | observe, cultivate, restore | Living-system resilience | Delay, invasive effects |
| Medic | triage, stabilize, rehabilitate | Keeps bodies and communities functioning | Scarcity, burnout |
| Trader | value, negotiate, connect | Exchange across difference | Dependency, speculation |
| Civic organizer | assemble, mediate, charter | Collective legitimacy | Delay, capture, exclusion |
| Archivist | verify, preserve, interpret | Durable continuity | Frozen authority, context loss |
| Pilot/driver | operate, route, recover | Mobility and heavy capability | Fuel, weather, mechanical failure |
| Artist/storyteller | perform, commemorate, reinterpret | Culture and shared meaning | Propaganda, exclusion, spectacle debt |

## Activity Quality Standard

Every major activity should specify:

```rust
struct GameplayActivity {
    fantasy: String,
    primary_verbs: Vec<Verb>,
    skill_expression: Vec<SkillPattern>,
    preparation: Vec<Requirement>,
    immediate_feedback: Vec<Feedback>,
    world_consequence: Vec<Consequence>,
    cooperation_hooks: Vec<CoopRole>,
    failure_continuations: Vec<FailureState>,
    repetition_variants: Vec<Variant>,
}
```

### Skill Expression

Avoid reducing mastery to higher statistics.

Skill can come from:

```text
timing
aim
route choice
tool control
pattern recognition
diagnostic reasoning
resource staging
team communication
risk judgment
social reading
```

### Immediate Feedback

Consequences may be delayed, but actions need immediate response through:

```text
sound
motion
force
material change
NPC reaction
instrument reading
route opening
enemy behavior
```

### Failure Continuation

Failure should often create a new situation:

```text
vehicle damaged but recoverable
convoy scattered
enemy alerted
repair temporary
evidence contaminated
faction offended
route closed
injury requiring evacuation
```

Reload-only failure should be reserved for corrupted state, unrecoverable technical faults, or explicit hard modes.

## Progression Across Scales

### Early: Personal Capability

```text
reliable tools
basic weapons
portable power
field shelter
small rover
local knowledge
```

### Middle: Network Capability

```text
workshops
vehicle fleets
farms
factories
regional routes
robots
research stations
civic institutions
```

### Late: Planetary Capability

```text
cities
orbital launch
climate works
large ecologies
fleets
planetary treaties
worldline infrastructure
```

The player should never become so abstract that tools, vehicles, and local places stop mattering.

## Anti-Grind Rules

Repetition is justified only when at least one changes:

```text
terrain
threat
route
tool
social context
material condition
strategic objective
historical consequence
```

Resources should not exist only to lengthen crafting time.

Every recurring chain needs:

```text
automation path
optimization path
trade path
risk path
political path
```

## Interface Rule

The Field Deck should support the activity, not become the activity.

Whenever possible:

```text
world interaction first
instrument interpretation second
menu administration third
```

## Production Test Matrix

A vertical slice is representative only if it proves:

| Pillar | Minimum proof |
|---|---|
| Traverse | Distinct routes and one mobility transition |
| Investigate | A discovery that changes available action |
| Transform | A visible persistent physical change |
| Contend | A dangerous encounter with tactical choice |
| Coordinate | Cargo, co-op, NPC, or system coordination |
| Belong | A moment showing why the settlement matters |

The proof can be compact. It cannot be replaced by exposition.
