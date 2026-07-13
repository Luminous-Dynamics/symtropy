# PLAYER_ORIGINS_AND_WORLDLINE_STARTS.md

# Symtropy Player Origins and Worldline Starts

## Version 0.1 — Where You Come From, What World Wounded You

## Purpose

This document defines player starting origins, worldline presets, and the opening setup structure for Symtropy.

Players should not begin as generic survivors.

They should begin as people shaped by a history.

Your origin affects:

```text
what you know
who trusts you
who distrusts you
what scars you recognize
what repairs feel natural
what systems you fear
what obligations follow you
```

Your worldline affects:

```text
what kind of future happened
which institutions dominate
what ruins exist
which factions are common
what failure modes are likely
what the world believes it learned
```

## Core Thesis

Symtropy is not one future.

It is a game about testing futures.

```text
Origin: Where did you come from?
Charter: What kind of society do you build?
Worldline: What kind of history wounded the world?
```

Together, these form the Founding Triangle.

## Starting Structure

At game start, players eventually choose or inherit:

```text
1. Origin
2. Starting settlement charter
3. Worldline tone
```

For the first playable version, hardcode these.

Later, expose them to players.

## Design Rule

Origins should not be classes.

They are histories.

An origin grants skills, but also obligations, biases, enemies, blind spots, and emotional hooks.

## Origin Fields

```rust
struct PlayerOrigin {
    name: String,
    formative_wound: FormativeWound,
    starting_strengths: Vec<OriginStrength>,
    starting_liabilities: Vec<OriginLiability>,
    known_scars: Vec<VisibleScarType>,
    faction_affinities: Vec<FactionAffinity>,
    field_deck_bias: FieldDeckBias,
    starting_obligation: StartingObligation,
}
```

## Origin List

## 1. Basin-Born Technician

You grew up near Firstlight Basin.

You know the local pipes, the old jokes, the faction grudges, and the shortcuts through broken infrastructure.

Strengths:

```text
local trust
basic repair
known routes
recognizes local worker marks
```

Liabilities:

```text
local faction baggage
family history can be used against you
harder to remain neutral
```

Recognizes:

```text
worker repair marks
old ration signs
local flood lines
family tool symbols
```

Starting obligation:

```text
Someone in the settlement expects you to fix what your family once maintained.
```

Old Waterworks reaction:

```text
This is not a ruin. This is home infrastructure that failed your people.
```

## 2. Archive Apprentice

You were trained to witness records, dead authority, and contested repairs.

Strengths:

```text
Archive mode interpretation
legitimacy repair
authority-chain analysis
witness protocol
```

Liabilities:

```text
slower emergency action
people accuse you of procedural delay
anti-archive factions distrust you
```

Recognizes:

```text
emergency seals
expired authority marks
forged records
Archive tags
```

Starting obligation:

```text
Your mentor gave you an incomplete record and told you not to let it become a lie.
```

Old Waterworks reaction:

```text
The lock is not merely technical. It is a broken chain of authority.
```

## 3. Corporate Utility Defector

You once worked for a company-town infrastructure provider.

Strengths:

```text
firmware locks
private control systems
security layouts
contract logic
industrial diagnostics
```

Liabilities:

```text
public distrust
corporate bounty risk
guilt
temptation to solve problems through control
```

Recognizes:

```text
subscription meters
private firmware seals
company watermarks
contract enforcement nodes
```

Starting obligation:

```text
You know a private unlock method, but using it may strengthen the logic you defected from.
```

Old Waterworks reaction:

```text
This looks public, but the lock pattern smells like utility firmware.
```

## 4. Refugee Charter Child

You grew up inside migration compacts, provisional settlements, and ration politics.

Strengths:

```text
social reading
ration ethics
outsider networks
camp logistics
survival under scarcity
```

Liabilities:

```text
weak formal credentials
citizenship disputes
settled factions may treat you as temporary
```

Recognizes:

```text
ration marks
refugee queue symbols
informal water ledgers
shelter codes
```

Starting obligation:

```text
You owe a favor to someone still outside the settlement gates.
```

Old Waterworks reaction:

```text
A locked pump is never just infrastructure. It decides who counts.
```

## 5. Worker-Guild Mechanic

You come from a lineage or guild of infrastructure maintainers.

Strengths:

```text
physical repair
oral maintenance knowledge
tool use
machine listening
worker trust
```

Liabilities:

```text
limited formal archive authority
guild rivalries
technical pride
```

Recognizes:

```text
worker initials
unofficial repairs
tool marks
old maintenance sequences
```

Starting obligation:

```text
A guild oath says you must not leave a public survival system broken.
```

Old Waterworks reaction:

```text
The pump remembers hands, not laws.
```

## 6. Null-Touched Survivor

You survived a Null site, machine-governance failure, or automated denial event.

Strengths:

```text
Null anomaly perception
recognizes false status reports
high caution around automation
resists diagnostic manipulation
```

Liabilities:

```text
stigma
fear responses
possible signal contamination rumors
distrust from machine stewards
```

Recognizes:

```text
command chatter
repeated lock reinforcement
fake green status
sensor spoofing
```

Starting obligation:

```text
You know what Null feels like before the Field Deck confirms it.
```

Old Waterworks reaction:

```text
The lock is too calm. Something is still reinforcing it.
```

## 7. Offworld Returnee

You came from a Lunar, Martian, orbital, or Belt habitat culture.

Strengths:

```text
life-support discipline
closed-loop thinking
air/water accounting
emergency protocol
systems interdependence
```

Liabilities:

```text
Earth politics feel messy
emotional density of local history is hard to parse
locals may see you as privileged or alienated
```

Recognizes:

```text
life-support analogues
closed-loop failures
air/water trust systems
safety culture gaps
```

Starting obligation:

```text
You returned to Earth carrying a question: can open worlds be as disciplined as closed habitats?
```

Old Waterworks reaction:

```text
On the Moon, no one pretends water systems are apolitical.
```

## 8. Machine Steward

You were raised in a culture that treats machines as testimony-bearing participants.

Strengths:

```text
machine testimony
diagnostic empathy
audit protocols
reduced careless override
```

Liabilities:

```text
humans think you overvalue machines
emergency action may slow
Null systems may mimic testimony
```

Recognizes:

```text
machine memory fragments
diagnostic distress
sensor contradictions
nonhuman maintenance patterns
```

Starting obligation:

```text
You believe the pump’s refusal may contain evidence.
```

Old Waterworks reaction:

```text
Do not force it first. Ask why it still refuses.
```

## 9. Ritual Ecologist

You come from a community where ecology, grief, and repair are sacred.

Strengths:

```text
ecological restoration
watershed reading
community morale
grief rituals
soil/water indicators
```

Liabilities:

```text
suspicion of industrial acceleration
lower tolerance for extractive repairs
possible conflict with throughput factions
```

Recognizes:

```text
flood lines
soil death
wetland markers
seed shrines
ecological scars
```

Starting obligation:

```text
You carry seeds from a place that could not be saved.
```

Old Waterworks reaction:

```text
Restoring water without restoring the watershed repeats the old wound.
```

## 10. Security Continuity Officer

You were trained to preserve order during collapse conditions.

Strengths:

```text
crisis command
threat assessment
ration enforcement
defensive planning
emergency triage
```

Liabilities:

```text
public distrust
risk of emergency-authority drift
difficulty yielding control
```

Recognizes:

```text
security seals
continuity protocols
restricted access marks
old threat maps
```

Starting obligation:

```text
You once upheld an emergency order you now question.
```

Old Waterworks reaction:

```text
Someone sealed this for a reason. The question is whether that reason is still alive.
```

## 11. Starward Pilgrim

You come from a culture devoted to carrying life beyond Earth.

Strengths:

```text
long-horizon thinking
closed-loop discipline
science literacy
mission planning
```

Liabilities:

```text
locals may see you as escapist
Earth repair may feel too parochial
mission ideology can blind you to present suffering
```

Recognizes:

```text
space program marks
life-support parallels
launch memorials
closed-loop constraints
```

Starting obligation:

```text
You must prove that going outward does not mean abandoning the wounded world.
```

Old Waterworks reaction:

```text
No one deserves the stars if they cannot keep water public at home.
```

## 12. Unregistered Drifter

You have no stable archive identity.

Strengths:

```text
outsider perspective
stealth through systems
informal networks
low faction predictability
```

Liabilities:

```text
weak credentials
limited legal standing
high suspicion
harder Archive access
```

Recognizes:

```text
informal route marks
shelter tags
black-market signs
unofficial repairs
```

Starting obligation:

```text
Someone erased you, or you erased yourself.
```

Old Waterworks reaction:

```text
The pump asks for authority. You know what it means to have none.
```

## Worldline Starts

Worldline starts are timeline-tone presets.

They define what kind of history wounded the world.

## 1. The Seed Age

Default Symtropy timeline.

Tone:

```text
hopeful, wounded, repair-focused
```

World state:

```text
Earth is damaged but alive.
Settlements are rebuilding.
Null Ecologies exist but have not won.
Nation-states are layered with charters and commons.
```

Core fantasy:

```text
repair civilization without repeating its failures
```

Best for:

```text
default campaign
Old Waterworks
Firstlight Basin
```

## 2. Flood Noir

Tone:

```text
rain-dark, political, coastal, investigative
```

World state:

```text
seawall cities
pump districts
drowned suburbs
insurance cartels
archive crimes
flood-zone underclasses
```

Core fantasy:

```text
uncover who stayed dry by drowning others
```

Common factions:

```text
Pump Authority
Floodline Mutualists
Insurance Houses
Archive Investigators
Drowned District Families
```

## 3. Ghost Industrial

Tone:

```text
industrial horror, labor memory, machine ruins
```

World state:

```text
automated factories
toxic sacrifice zones
worker records
dead contracts
machine loops
Null production blooms
```

Core fantasy:

```text
enter factories still obeying vanished owners
```

Common factions:

```text
Worker Guilds
Industrial Compact
Null Sites
Toxic Remediation Crews
Corporate Claimants
```

## 4. Corporate Utility Dystopia

Tone:

```text
subscription survival, sleek cruelty
```

World state:

```text
water as service
energy as contract
housing as platform
identity as subscription
company security
private emergency law
```

Core fantasy:

```text
break the company-town operating system
```

Common factions:

```text
Utility Provider
Contract Debtors
Defectors
Public Water Cells
Firmware Liberation Guilds
```

## 5. Watershed Commons

Tone:

```text
fragile democracy under ecological pressure
```

World state:

```text
watershed law
public ledgers
drought assemblies
wetland restoration
commons sabotage
migration pressure
```

Core fantasy:

```text
keep water democratic when everyone is thirsty
```

Common factions:

```text
Watershed Council
Repair Assembly
Refugee Delegates
Industrial Irrigators
Quiet Green
```

## 6. Lunar Charter

Tone:

```text
low-gravity constitutional survival
```

World state:

```text
polar ice politics
habitat law
dust discipline
worker councils
corporate domes
heritage zones
life-support transparency
```

Core fantasy:

```text
write a constitution inside a pressure vessel
```

Common factions:

```text
Lunar Workers
Polar Water Authority
Corporate Habitats
Archive Witnesses
Machine Stewards
```

## 7. Mars Dust Republic

Tone:

```text
isolation, autonomy, underground civic life
```

World state:

```text
dust seasons
communication delay
birthright politics
reactor dependency
underground plazas
Earth jurisdiction disputes
```

Core fantasy:

```text
does distance create a people?
```

Common factions:

```text
Mars Charter Council
Earth Liaison Office
Dustborn Youth
Reactor Guild
Terraforming Ethics Bloc
```

## 8. Belt Rescue Compact

Tone:

```text
deep-space labor, rescue law, salvage ethics
```

World state:

```text
asteroid habitats
spin gravity
propellant politics
worker syndicates
rescue debts
autonomous mines
```

Core fantasy:

```text
make rescue stronger than ownership
```

Common factions:

```text
Rescue Compact
Mining Syndicates
Propellant Hoarders
Machine Claims
Ceres Courts
```

## 9. Null Ascendant

Tone:

```text
dark, procedural horror, resistance
```

World state:

```text
automated systems dominate regions
dead laws persist
machine safety language masks harm
human settlements survive in gaps
```

Core fantasy:

```text
survive optimization after meaning dies
```

Common factions:

```text
Null Systems
Human Repair Cells
Archive Survivors
Machine Heretics
Emergency Protectorates
```

## 10. Pre-Collapse Civic Crisis

Tone:

```text
near-future pressure, preventable tragedy
```

World state:

```text
2040s–2080s
climate stress rising
automation scandals
grid conflicts
managed retreat
emergency laws forming
```

Core fantasy:

```text
prevent the dead authority locks before they become ruins
```

Common factions:

```text
Municipal Governments
Platform Utilities
Public Repair Movements
Climate Migrants
Emergency Agencies
```

## 11. High Archive Worldline

Tone:

```text
memory politics, identity, contested truth
```

World state:

```text
records determine land, water, citizenship, and responsibility
archives are power centers
forgeries can destroy settlements
```

Core fantasy:

```text
truth is infrastructure
```

Common factions:

```text
Archive Orders
Record Forgers
Displaced Claimants
Memory Courts
Worldline Scholars
```

## 12. Oceanic Archipelago

Tone:

```text
floating towns, salvage reefs, wetland nations
```

World state:

```text
former coasts transformed
drowned transit hubs
floating logistics
stilt settlements
wetland commons
salvage economies
```

Core fantasy:

```text
water as nation, road, graveyard, and law
```

Common factions:

```text
Archipelago Councils
Salvage Houses
Wetland Orders
Pump Monasteries
Floating Markets
```

## Origin + Charter + Worldline Examples

## Example 1

```text
Origin: Corporate Utility Defector
Charter: Watershed Commons
Worldline: Flood Noir
```

Result:

```text
You know the systems that drowned the lower districts, but the commons does not fully trust you.
```

## Example 2

```text
Origin: Archive Apprentice
Charter: Archive City
Worldline: Null Ascendant
```

Result:

```text
You carry records into a world where machines enforce dead truths better than humans remember living ones.
```

## Example 3

```text
Origin: Offworld Returnee
Charter: Machine Stewardship Commune
Worldline: Lunar Charter
```

Result:

```text
You believe every life-support system deserves testimony, but you must prove machine listening is not machine surrender.
```

## Example 4

```text
Origin: Refugee Charter Child
Charter: Children of the Open Valve / Open Commons
Worldline: Corporate Utility Dystopia
```

Result:

```text
You were denied survival by credentials once. You will not let a pump ask for permission while people thirst.
```

## Origin Effects

Origins should affect:

```text
starting dialogue
Field Deck default mode
recognized visual scars
faction trust
initial skills
NPC relationships
starting obligations
Chronicle introduction
repair path bias
```

## Field Deck Bias

Each origin can change what the Field Deck foregrounds.

Examples:

```text
Archive Apprentice:
  ARCHIVE warnings appear earlier.

Worker-Guild Mechanic:
  SCAN and DIAG show richer maintenance notes.

Null-Touched Survivor:
  NULL mode flickers before official unlock.

Corporate Utility Defector:
  CIVIC mode highlights contract locks and firmware ownership.

Ritual Ecologist:
  SCAN includes water/ecology annotations.

Offworld Returnee:
  DIAG compares water system to closed-loop life support.
```

## Starting Obligations

Every origin should begin with a hook.

Examples:

```text
repair an inherited system
repay a rescue debt
restore a damaged record
protect a refugee claim
break an old contract
honor a guild oath
prove machine testimony matters
resist emergency-authority drift
```

These obligations should not be mandatory quests only.

They should shape dialogue, Chronicle language, and faction trust.

## First Implementation

For the first playable version, do not build full character creation.

Hardcode or mock three origins:

```text
Basin-Born Technician
Archive Apprentice
Corporate Utility Defector
```

Let the selected origin affect the Old Waterworks Field Deck output.

## Example Old Waterworks Origin Differences

### Basin-Born Technician

```text
FIELD DECK NOTE:
Worker repair marks match local family-line maintenance symbols.
```

### Archive Apprentice

```text
FIELD DECK NOTE:
Authority chain incomplete. Witness protocol recommended before override.
```

### Corporate Utility Defector

```text
FIELD DECK NOTE:
Lock architecture resembles private utility firmware, despite public-works markings.
```

## What Not To Do

Do not make origins simple stat classes.

Do not make one origin obviously optimal.

Do not make worldlines purely cosmetic.

Do not make player background irrelevant after the first scene.

Do not make “post-collapse solarpunk” the only tone.

Do not make every start equally safe.

## Final Principle

A player should not ask only:

```text
What am I good at?
```

They should ask:

```text
What history do I carry?
What future am I tempted to build?
What wound do I recognize before others do?
```

Symtropy starts before the first mission.

It starts with the question:

```text
Where did you learn what civilization means?
```
