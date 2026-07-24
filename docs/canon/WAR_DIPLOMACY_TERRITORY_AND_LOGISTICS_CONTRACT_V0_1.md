---
title: War, Diplomacy, Territory, and Logistics Contract
version: 0.1
status: canonical
scope: strategic conflict, diplomacy, territorial control, war aims, civilian protection, peace, and campaign consequences
owner: design/systems/narrative/multiplayer
related:
  - tech/COMBAT_THREAT_AND_SYSTEMIC_ENCOUNTER_DESIGN_V0_1.md
  - tech/STRATEGIC_CONFLICT_CAMPAIGN_AND_OCCUPATION_SIMULATION_V0_1.md
  - tech/PROCEDURAL_FACTION_EVOLUTION.md
  - lore/HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md
  - lore/Symtropy Naval & Fleet Design Bible.md
  - SYMTROPY_SPACECRAFT_DESIGN_BIBLE_V0_1.md
  - tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
  - canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
---

# War, Diplomacy, Territory, and Logistics Contract

## Owned Question

**How can Symtropy support exciting warfare, territorial struggle, diplomacy, fleets, insurgency, and peace without reducing civilization to map painting or treating civilian life as decorative collateral?**

## Core Thesis

War in Symtropy is a struggle over **capability, movement, legitimacy, survival, and possible futures**.

It is fought through:

```text
bodies and weapons
routes and vehicles
energy and industry
information and archives
alliances and promises
habitats and ecosystems
fear and morale
law and legitimacy
```

Combat decides what happens at a place. Campaigns decide what a region can sustain. Diplomacy decides whether the result becomes a temporary pause, a durable order, a frozen wound, or another cause of war.

```text
A battle is an event.
A war is a metabolism under organized violence.
A peace is an infrastructure that must be maintained.
```

## Strategic Prime Directives

1. **No abstract armies without physical support.** Forces require people, machines, training, fuel, food, medicine, parts, information, routes, rest, and political permission.
2. **No territory as paint alone.** Control means an actor can observe, move, supply, administer, protect, extract, or credibly deny access.
3. **No permanent war mode by default.** Wars have aims, costs, exhaustion, negotiation windows, and possible endings.
4. **No morality as tactical dullness.** Combat must remain skillful and readable. Consequences enrich the fight; they do not replace it.
5. **No civilian populations as passive numbers.** People flee, hide, resist, collaborate, negotiate, care for one another, and remember occupation.
6. **No faction-wide moral essence.** Commanders, units, institutions, and citizens can disagree, defect, refuse, or split.
7. **No peace through a single dialogue check.** Durable peace requires enforceable boundaries, material arrangements, verification, and institutions capable of surviving bad faith.
8. **No forced PvP through strategic systems.** Multiplayer conflict profiles and consent rules remain authoritative.

# 1. Conflict Scales

## 1.1 Encounter Scale

Duration:

```text
seconds to tens of minutes
```

Owns:

```text
movement
weapons
cover
boarding
vehicle control
local objectives
capture, retreat, surrender
immediate damage
```

Resolved by embodied simulation and local shard authority.

## 1.2 Operation Scale

Duration:

```text
one session to several sessions
```

Owns:

```text
raid or defense plan
convoy movement
reconnaissance
sabotage
rescue
site capture
route opening
withdrawal
```

An operation links encounters through logistics and objectives.

## 1.3 Campaign Scale

Duration:

```text
days to months of worldline time
```

Owns:

```text
fronts
supply networks
force readiness
war aims
alliances
mobilization
civilian displacement
industrial pressure
negotiation
```

## 1.4 Strategic / Worldline Scale

Duration:

```text
months to generations
```

Owns:

```text
borders and access regimes
federations
security architectures
arms limitations
postwar recovery
historical grievance
military institutions
worldline forks
```

The scales exchange consequences, but no higher layer simulates bullets and no lower layer decides treaty legitimacy.

# 2. Why Actors Fight

Wars and lesser conflicts require explicit aims.

Possible aims:

```text
protect a population
secure or reopen a route
stop extraction or ecological harm
recover people or archives
remove an occupation
contain a dangerous process
seize infrastructure
enforce debt or contract
change a regime
prevent secession
win recognition
obtain migration access
control orbit, altitude, pressure, or signal corridors
```

A declared aim may differ from the actual aim. Factions, leaders, and institutions may disagree about both.

## War Aim Contract

```rust
struct WarAim {
    sponsor: ActorId,
    public_claim: ClaimId,
    operational_objectives: Vec<ObjectiveId>,
    acceptable_outcomes: Vec<OutcomeCondition>,
    forbidden_methods: Vec<MethodClass>,
    constituency_support: f32,
    legitimacy_basis: Vec<EvidenceRef>,
    expiry_or_review: Option<ChronicleTick>,
}
```

War aims constrain peace offers, faction cohesion, mission generation, propaganda, and exhaustion.

# 3. Territory as Capability

A location is controlled only to the extent that an actor can sustain relevant functions there.

Control dimensions:

```text
presence          — forces or stewards can physically remain
observation       — activity can be detected reliably
mobility          — routes can be used
supply            — people and machines can be sustained
administration    — rules and services can be enacted
legitimacy        — affected populations recognize or tolerate authority
resilience        — control survives disruption
```

A faction may control a road but not the surrounding villages, an orbital lane but not the habitats below it, or a city center by day and nothing beyond its walls at night.

Territory should therefore be represented as overlapping capabilities, not a single owner field.

# 4. Logistics Is Strategy Made Physical

Every force depends on a supply profile.

```text
energy or fuel
ammunition or replacement tools
food and water
medical capacity
spare parts
maintenance labor
communications
navigation and intelligence
crew rest and rotation
```

A campaign should create meaningful player roles beyond direct combat:

```text
convoy escort
repair and recovery
route surveying
field medicine
signal restoration
counter-intelligence
salvage denial
bridge and port construction
refugee transport
negotiation and exchange
```

## Logistics Rules

- Supply should be disruptable but not require repetitive hauling by one player class.
- Automation can reduce labor while creating infrastructure targets and dependency.
- Shortages should change tactics visibly before silently reducing a number.
- Captured resources retain provenance, contamination, compatibility, and legal claims.
- A force cut off from supply may retreat, forage, negotiate, fragment, surrender, or become predatory.

# 5. Force Readiness

Forces are not only unit counts.

```rust
struct ForceReadiness {
    personnel: f32,
    equipment_condition: f32,
    supply_days: f32,
    mobility: f32,
    intelligence_quality: f32,
    cohesion: f32,
    morale: f32,
    legitimacy: f32,
    medical_capacity: f32,
    fatigue: f32,
}
```

Readiness changes what operations are possible and how units behave. Low cohesion may produce refusal or desertion. Low equipment condition produces jams, breakdowns, and reduced vehicle availability. Low legitimacy increases resistance and intelligence failures.

# 6. Diplomacy as Ongoing Infrastructure

Diplomacy is not a menu of relationship points. It is a network of commitments, channels, verification, and constituencies.

Diplomatic instruments include:

```text
recognition
ceasefire
safe-passage compact
trade and repair agreement
mutual defense
non-aggression
resource-sharing treaty
quarantine protocol
migration corridor
prisoner exchange
archive access
arms limitation
joint ecological stewardship
```

Every agreement should define:

```text
parties
scope
obligations
verification
review and expiry
breach handling
appeal
exit
emergency exceptions
public or secret status
```

## Trust Is Scoped

A faction may trust another to exchange prisoners but not share research, or honor a river treaty while contesting an orbital corridor.

Diplomatic trust must be tracked by domain and precedent rather than one global friendliness score.

# 7. Negotiation and Peace

Peace processes may begin through:

```text
war exhaustion
stalemate
outside mediation
leadership change
shared disaster
mutiny or public pressure
successful prisoner exchange
new evidence
recognition of a larger threat
```

Possible endings:

```text
ceasefire
armistice
withdrawal
limited settlement
federation or compact
partition
autonomy
reparations
joint stewardship
frozen conflict
collapse of one party
worldline fork
```

A peace settlement must address the material causes that remain relevant. Signing a document does not restore destroyed routes, homes, trust, or ecosystems.

# 8. Civilians, Displacement, and Care

Civilian systems track more than casualties.

```text
access to food, water, shelter, medicine, and signal
family separation
route safety
housing damage
fear and trauma
employment and care burden
identity and archive continuity
willingness to remain, flee, resist, or return
```

Players should encounter civilian agency through:

```text
self-organized evacuation
local defense
strikes
shelter networks
smuggling
refusal to collaborate
negotiation with multiple sides
public documentation of abuse
postwar claims
```

The game must not reward indiscriminate harm as an efficient universal strategy. Strategic costs may include resistance, loss of intelligence, diplomatic isolation, internal dissent, Chronicle evidence, and long-term recovery burden.

# 9. Occupation and Administration

Capturing a site does not automatically produce stable rule.

An occupying force must decide:

```text
which institutions remain
who provides services
how movement is controlled
what laws apply
how property and archives are handled
whether local officials participate
how resistance is treated
when authority expires
```

Occupation states:

```text
military presence
provisional administration
collaborative compact
contested occupation
annexation attempt
protectorate drift
withdrawal transition
```

Occupation should generate gameplay around service continuity, legitimacy, sabotage, public safety, testimony, and exit—not only patrol density.

# 10. Surrender, Capture, and Prisoners

Combatants and crews may:

```text
retreat
become incapacitated
surrender
be captured
be exchanged
be paroled
defect
be rescued
```

Rules depend on worldline conflict profile and faction doctrine. Surrender must be tactically legible and protected from trivial exploitation.

Prisoners create obligations:

```text
care
security
identity verification
communication
exchange
trial or release
```

# 11. Intelligence, Secrecy, and Information War

Strategic actors operate with incomplete information.

Sources:

```text
scouts
signals
trade records
witness testimony
captured devices
public statements
orbital observation
local relationships
```

Information has provenance, delay, confidence, and deception risk.

Information operations may:

```text
hide movement
forge orders
jam signal
expose corruption
broadcast evidence
seed rumors
attack archives
create false readiness reports
```

They may not rewrite authoritative player memories or Chronicle records without a traceable attack and recovery path.

# 12. Ecological and Nonhuman Conflict

A campaign may involve beings for whom territory, surrender, or borders work differently.

Examples:

```text
an oceanic mind defending pressure continuity
a migratory polity requiring seasonal passage
a lithic intelligence responding over decades
a swarm whose units are not individual persons
a quarantine system protecting an unknown risk
```

The strategic system must model protected values and viability conditions before assuming conventional war aims.

Some conflicts are resolved through habitat redesign, timing agreements, non-contact boundaries, or translation rather than conquest.

# 13. Fleets and Space Warfare

Naval and spacecraft conflict follows the same logistics and legitimacy rules under harsher constraints.

Important variables:

```text
crew endurance
life support
reaction mass
maintenance windows
orbital geometry
rescue obligation
communications delay
port and depot access
pressure integrity
```

Destroying a ship may also destroy a habitat, archive, hospital, or migration route. Boarding, disabling, surrender, towing, rescue, and capture should often be more strategically valuable than annihilation.

# 14. Player Roles

Players may participate as:

```text
fighter
pilot or driver
engineer
medic
scout
convoy coordinator
signals operator
negotiator
investigator
quartermaster
builder
rescue specialist
public witness
```

No role should be a mandatory bureaucratic chore. Each must involve skillful decisions, physical interaction, or meaningful coordination.

# 15. Anti-Patterns

```text
war as a permanent content treadmill
map color changing without physical consequence
infinite troops spawned from abstract population
supply as repetitive manual hauling only
one relationship score controlling all diplomacy
civilians existing only as casualty counters
occupation becoming free resource output
peace achieved by one charisma roll
all enemies fighting to the death
strategic systems bypassing multiplayer consent
aliens forced into human military categories
```

# 16. Acceptance Gates

A strategic-conflict slice is valid when:

- a local victory changes a route, readiness, or negotiation position rather than simply awarding loot;
- forces cannot operate indefinitely without credible supply and maintenance;
- at least one operation can be won through maneuver, rescue, negotiation, sabotage, or containment rather than annihilation;
- civilian responses alter campaign conditions;
- a ceasefire has explicit obligations and can succeed or fail for traceable reasons;
- territory is represented through overlapping capabilities;
- strategic simulation degrades gracefully outside active regions;
- conflict profile and consent rules remain enforceable;
- the Chronicle records only durable strategic outcomes, not every tactical event.

## Final Rule

```text
Symtropy should make war powerful enough to change civilization,
and expensive enough that peace remains an achievement rather than an absence of content.
```
