---
title: Combat, Threat, and Systemic Encounter Design
version: 0.1
status: canonical-draft
scope: weapons, enemies, danger, raids, defense, nonlethal outcomes
related:
  - lore/HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md
  - Symtropy Dungeons and Raids Design Bible.md
  - lore/ENCOUNTER_CONTRACTS_AND_NONLETHAL_RESOLUTION.md
  - canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md
owner: design/gameplay/AI
---

# Combat, Threat, and Systemic Encounter Design

## Core Thesis

Symtropy combat must be satisfying before it is interpreted.

Players should enjoy movement, aiming, impact, teamwork, enemy behavior, and tactical adaptation even when they do not yet understand the political or historical context.

Meaning makes combat consequential. It cannot replace combat quality.


## Strategic Boundary

This document owns the moment-to-moment quality and systemic consequences of combat encounters. Campaign aims, territorial capability, force readiness, logistics, occupation, diplomacy, and peace are owned by [War, Diplomacy, Territory, and Logistics Contract](../canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md).

A combat victory may publish strategic consequences. It must not directly repaint territory or resolve a war without the campaign layer accepting those consequences.

## Combat Pillars

### Physical

Weapons, bodies, cover, machines, structures, and hazards react visibly.

### Readable

The player can understand:

```text
what is threatening them
what the threat can do
what interrupted it
what damage changed
```

### Cooperative

Different tools and roles create real team leverage.

### Systemic

Combat interacts with:

```text
power
doors
vehicles
terrain
signal
weather
cargo
civilians
infrastructure
```

### Consequential

Major outcomes persist through damage, casualties, faction memory, salvage, or changed control.

## Player Combat Verbs

```text
aim
fire
strike
block
brace
dodge
slide
climb
suppress
mark
jam
hack
cut
disable
repair
revive
carry
extract
```

No combat role should be reduced to shooting a different color of damage.

## Weapon Families

### Kinetic

Reliable, physical, ammunition-dependent, effective against exposed components.

### Directed Energy

Power-intensive, heat-limited, useful for precision, sensors, or specific materials.

### Industrial Tools

Cutters, welders, impact drivers, sealant, arc tools. Strong in close technical encounters and environmental interaction.

### Electromagnetic and Signal

Jammers, disruptors, spoofing tools, sensor darts. Useful against machines but limited by shielding and uncertainty.

### Chemical and Ecological

Foams, smoke, adhesives, repellents, spores, atmosphere control. High contextual value and risk.

### Defensive

Shields, cover projectors, decoys, countermeasures, repair drones, medical systems.

## Damage Model

Avoid a single universal health bar where practical.

Threats may expose:

```text
mobility
sensors
power
weapons
cooling
structure
control
morale
coordination
```

Damaging a subsystem should visibly change behavior.

Example:

```text
Sensor damage:
enemy fires less accurately and uses acoustic searching.

Mobility damage:
enemy anchors itself and becomes a turret.

Cooling damage:
enemy attacks aggressively before thermal shutdown.
```

## Enemy Role Grammar

Every encounter group should combine distinct roles.

### Scout

Finds, marks, and routes around the player.

### Harrier

Forces movement and punishes stationary play.

### Anchor

Controls space through armor, cover, or area denial.

### Custodian

Repairs, resupplies, evacuates, or restores other threats.

### Jammer

Disrupts communication, instruments, or targeting.

### Breacher

Destroys cover, doors, vehicles, or infrastructure.

### Carrier

Moves cargo, hazards, units, or captives.

### Commander

Changes coordination and posture rather than only increasing health.

### Environmental Actor

Flood, fire, storm, decompression, hostile ecology, unstable structure.

## Hostility Is Relational

Enemy design should inherit motives and triggers from the threat ecology.

But tactical readability must not depend on reading a lore entry.

A Continuance unit, alien quarantine organism, raider, and Null factory crawler may share a combat role while behaving differently.

## Null Combat Identity

Null systems are dangerous because they maintain procedure under changing reality.

Combat behaviors:

```text
restore destroyed barriers
repeat failed formations
protect obsolete targets
repair hostile infrastructure
mark living actors as anomalies
report false green status
```

Null should not mean slow robots with glowing corruption.

## Human and Social Enemies

Human conflict needs:

```text
fear
suppression
retreat
surrender
rescue
leadership
morale
limited ammunition
```

Not every hostile human should fight to death.

The player may create future enemies through humiliation, collateral damage, or broken agreements.

## Alien Encounters

Alien danger may arise from:

```text
territorial defense
category error
habitat damage
translation failure
metabolic incompatibility
quarantine
```

The player may need to survive first and understand later.

Nonlethal options should be meaningful, not magically superior.

## Encounter Structure

```text
approach
contact
read
commit
escalate
break
resolve
aftermath
```

### Approach

Position, route, preparation, and stealth.

### Contact

The first clear threat behavior.

### Read

The player identifies roles, hazards, and possible objectives.

### Commit

Fight, bypass, negotiate, disable, distract, or retreat.

### Escalate

Reinforcement, environmental change, objective movement, or new enemy role.

### Break

The encounter state changes decisively.

### Resolve

Destroy, disable, force withdrawal, complete rescue, escape, or establish boundary.

### Aftermath

Salvage, casualty care, repair, testimony, pursuit, and world state.

## Encounter Objectives

Use more than elimination:

```text
hold a route
escort cargo
extract civilians
disable production
capture a machine intact
defend a repair
survive a storm window
recover evidence
prevent contamination
force withdrawal
```

## Bosses as Systems With Bodies

A boss should have:

```text
physical body
operational purpose
environmental dependencies
phase-changing damage
support network
aftermath consequence
```

Avoid long health pools detached from the site.

Example:

```text
Recursive Foundry Heart

Purpose:
Continue emergency production.

Dependencies:
power trunk, material feed, custodian drones.

Phases:
defended production
mobile reconfiguration
thermal runaway
negotiable or destructive shutdown

Aftermath:
factory available, damaged, liberated, or lost.
```

## Nonlethal and De-escalatory Play

Nonlethal options may include:

```text
mobility disable
signal isolation
resource denial
boundary marking
leader capture
safe retreat corridor
proof of changed conditions
```

They require preparation and may create different risks.

The game must not imply that nonlethal always means harmless.

## Combat and Infrastructure

Combat can change the world through:

```text
collapsed bridges
damaged power
fire
contaminated water
destroyed archives
opened routes
salvaged machines
fortified sites
```

Players should care where they fight.

## Co-op Roles

A four-player encounter may support:

```text
frontline control
technical disable
mobility and rescue
observation and coordination
```

Roles remain flexible and tool-based.

## Difficulty

Difficulty should alter:

```text
enemy coordination
sensor quality
resource pressure
injury severity
recovery windows
environmental complexity
```

Avoid solving difficulty only through health and damage inflation.

## Firstlight Basin Combat Proof

The first slice should prove:

```text
one weapon with excellent feel
one industrial tool with combat utility
one enemy family with at least four roles
one vehicle encounter
one defend-while-transforming objective
one withdrawal or rescue continuation
```

## Acceptance Tests

Combat succeeds when playtesters can:

```text
identify enemy roles without text
describe a tactical decision they made
name a non-damage action that mattered
feel the environment changed the fight
remember the aftermath, not only the kill
```
