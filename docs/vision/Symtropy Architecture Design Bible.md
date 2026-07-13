# Symtropy Architecture Design Bible

## Version 0.3 — Architecture as Civic Evidence

## Core Thesis

Architecture in Symtropy is not scenery.

Architecture is civic evidence.

A building is not complete when it looks beautiful.
A settlement is not complete when it feels lived-in.
A ruin is not complete when it looks mysterious.

A place is complete when the player can read:

```text
what keeps people alive
who controls it
who repairs it
who is excluded
what history is visible
what history is hidden
what would happen if it failed
what it would mean to restore it
```

Symtropy architecture asks:

```text
What does this place make possible?
Who does it protect?
Who does it exclude?
Who repairs it?
Who remembers what happened here?
What future argument could this place create?
```

Only after those questions are answered should the team ask:

```text
What should it look like?
```

---

# 1. Architectural Prime Directive

```text
Architecture is governance made spatial.
```

Every major architectural family should express:

```text
water logic
repair logic
care logic
record logic
authority logic
access logic
ecological logic
failure logic
```

The player should be able to walk through a place and infer its civilization.

---

# 2. Global Visual Language

Symtropy architecture should feel:

```text
repairable
lived-in
auditable
public
layered
ecological
procedural
morally loaded
beautiful through use
```

It should avoid:

```text
generic cyberpunk
generic solarpunk
sterile utopia without critique
random post-apocalyptic junk
villain-coded alien hives
purely military sci-fi
unreadable maximalism
```

The strongest identity is:

```text
civic solarpunk with scars
public works realism
repair-culture sacredness
infrastructure as moral memory
```

---

# 3. Required Design Rule

Every architecture family must change at least one of:

```text
gameplay
access
repair logic
civic conflict
Field Deck readings
Chronicle consequences
procedural generation
```

If a family is only an aesthetic, it is not yet a Symtropy architecture family.

---

# 4. Required Location Template

Every major location should have these fields.

## Location Identity

```text
location_id:
display_name:
architecture_family:
settlement_type:
worldline_context:
primary_factions:
```

## Survival System

```text
primary_life_support_system:
secondary_life_support_systems:
what_keeps_people_alive_here:
what_happens_if_it_fails:
```

## Authority Model

```text
who_claims_authority:
who_maintains_authority:
who_disputes_authority:
what_authority_is_visible:
what_authority_is_hidden:
```

## Access Model

```text
who_can_enter:
who_is_delayed:
who_is_excluded:
what_credentials_are_required:
what_informal_access_paths_exist:
```

## Repair Model

```text
what_is_broken:
who_knows_how_to_fix_it:
what_tools_are_required:
what_repair_paths_exist:
what_repair_path_creates_legitimacy_debt:
```

## Memory Model

```text
oldest_visible_scar:
newest_patch:
public_records_visible:
missing_records:
oral_histories:
machine_testimony:
Chronicle_events_possible:
```

## Field Deck Readability

```text
SCAN_reveals:
DIAG_reveals:
ARCHIVE_reveals:
CIVIC_reveals:
NULL_reveals:
REPAIR_reveals:
WITNESS_records:
TACTICAL_NET_projects:
```

## Gameplay Function

```text
primary_gameplay_loop:
secondary_gameplay_loop:
exploration_value:
social_conflict:
repair_puzzle:
failure_state:
revisit_reason:
```

## Procedural Hooks

```text
generator_tags:
visual_modifiers:
faction_memory_flags:
possible_precedents:
future_argument_hooks:
```

---

# 5. Human Settlement Architecture Families

## 5.1 Watershed Commons

Core fantasy:

```text
The settlement is part of the water cycle.
```

Visual language:

```text
canals
wetlands
rain chains
reed beds
public cisterns
water-quality boards
green roofs
community docks
open ledgers
```

Gameplay:

```text
water testing
wetland restoration
cistern repair
upstream/downstream disputes
contamination witnessing
```

Core conflict:

```text
consensus versus urgency
```

Rule:

```text
Water should always be visible.
```

---

## 5.2 Repair Guild Republic

Core fantasy:

```text
Civilization survives because people know how to fix it.
```

Visual language:

```text
gantries
cranes
machine yards
public workshops
tool banners
apprentice platforms
parts depots
repair schedules
```

Gameplay:

```text
tool access
machine repair
apprenticeship
fabrication
guild legitimacy disputes
```

Core conflict:

```text
expertise versus public ownership
```

Rule:

```text
The repair process should be visible from the street.
```

---

## 5.3 Archive Witness Enclave

Core fantasy:

```text
Truth is infrastructure.
```

Visual language:

```text
record towers
witness desks
public reading rooms
engraved timelines
hearing circles
evidence drawers
precedent walls
```

Gameplay:

```text
testimony
record recovery
authority-chain analysis
public witnessing
legitimacy repair
```

Core conflict:

```text
truth versus speed
```

Rule:

```text
Records must be publicly readable, but not casually alterable.
```

---

## 5.4 Machine Stewardship Commune

Core fantasy:

```text
Machines are participants in care, not tools alone.
```

Visual language:

```text
machine rest stations
diagnostic gardens
shared repair bays
charging alcoves
machine testimony ports
robot-accessible paths
```

Gameplay:

```text
machine testimony
co-repair
diagnostic empathy
override dispute
Null mimicry detection
```

Core conflict:

```text
listening versus urgent override
```

Rule:

```text
Design for human bodies, machine bodies, disabled bodies, elder bodies, and child bodies.
```

---

## 5.5 Refugee Compact Town

Core fantasy:

```text
Temporary shelter became real civilization.
```

Visual language:

```text
modular housing
shade cloth
many languages
welcome centers
clinic containers
school tents made permanent
shared kitchens
painted wayfinding
```

Gameplay:

```text
credential repair
water access
care logistics
migration dispute
gate negotiation
```

Core conflict:

```text
temporary status versus permanent dignity
```

Rule:

```text
Nothing should look disposable, even if it began temporary.
```

---

## 5.6 Corporate Utility Enclave

Core fantasy:

```text
Reliable service with hidden ownership.
```

Visual language:

```text
clean surfaces
blue branding
subscriber gates
private terminals
polished tanks
drones
perfect landscaping
credential checkpoints
```

Gameplay:

```text
access negotiation
firmware audit
subscription bypass under witness
contract exposure
private water dispute
```

Core conflict:

```text
reliability versus exclusion
```

Rule:

```text
Everything should look clean enough to hide coercion.
```

---

## 5.7 Open Valve Direct-Action District

Core fantasy:

```text
Access first. Legitimacy later.
```

Visual language:

```text
murals
free water stations
open pipe networks
public kitchens
hand-built repairs
anti-gate signage
assembly yards
```

Gameplay:

```text
urgent repair
public access
unsafe bypasses
mutual aid missions
legitimacy debt
```

Core conflict:

```text
access versus legitimacy
```

Rule:

```text
Nothing important should be hidden behind a private door.
```

---

## 5.8 Continuance Emergency Protectorate

Core fantasy:

```text
Order can save lives — and forget to end.
```

Visual language:

```text
checkpoints
sandbags
watchtowers
ration boards
ID gates
temporary authority signs
command modules
emergency seals
```

Gameplay:

```text
stabilization
ration enforcement
authority expiry
checkpoint negotiation
emergency drift investigation
```

Core conflict:

```text
safety versus control
```

Rule:

```text
Everything should say temporary, but look like it has been there too long.
```

---

# 6. Resonatia Bastion Architecture

## 6.1 Core Definition

A Resonatia Bastion is not a fortress.

It is a hardened commons.

```text
A Resonatia Bastion is a public continuity node built to protect water, memory, care, repair, consent, and future life during collapse conditions.
```

It should never be framed as a generic military stronghold.

It is:

```text
shelter
school
clinic
archive
tool library
water reserve
seed bank
repair hall
charter court
mesh relay
public kitchen
Chronicle mirror
machine testimony port
Field Deck calibration station
```

Core fantasy:

```text
Strong enough to survive collapse.
Humble enough to give power back.
```

Core conflict:

```text
protection versus control
```

Primary rule:

```text
If a Bastion cannot be questioned, it is no longer Resonatia. It is Continuance drift.
```

---

## 6.2 Bastion Design Requirements

Every Resonatia Bastion must visibly contain:

```text
rights floor wall
public audit station
emergency authority expiry clock
open water access point
care triage board
repair teaching space
Chronicle mirror
visible exit route
public assembly chamber
machine testimony interface
```

Every Bastion must answer:

```text
Who can shelter here?
Who decides priority?
Who audits command?
When do emergency powers expire?
Can the public inspect records?
Can people leave?
Can machines testify?
Can refugees enter?
Can care override bureaucracy?
```

---

## 6.3 Resonatia Bastion Types

### Water Bastion

Built around cisterns, filtration, aquifer access, desalination, and public water law.

Gameplay:

```text
water allocation
contamination audit
ration dispute
desalination repair
emergency access hearing
```

Conflict:

```text
who receives emergency water first?
```

---

### Seed Bastion

Preserves seeds, soil cultures, pollinator records, food knowledge, and agricultural memory.

Gameplay:

```text
seed recovery
soil culture protection
famine triage
pollinator corridor restoration
```

Conflict:

```text
preservation versus immediate hunger
```

---

### Archive Bastion

Stores Chronicle mirrors, land records, machine testimony, oral histories, and dead-authority evidence.

Gameplay:

```text
record recovery
precedent disputes
identity restoration
evidence protection
```

Conflict:

```text
truth versus speed
```

---

### Care Bastion

Clinic, shelter, kitchen, cooling center, elder hub, child safety node.

Gameplay:

```text
triage
heatwave response
medicine shortage
shelter prioritization
care labor coordination
```

Conflict:

```text
care capacity versus impossible need
```

---

### Repair Bastion

A hardened workshop district with tools, fabrication, apprenticeships, spare parts, and public training.

Gameplay:

```text
fabrication
tool access
repair education
parts allocation
public works restoration
```

Conflict:

```text
expertise versus public access
```

---

### Mesh Bastion

Communications and coordination node that restores public signal after storms, sabotage, Null interference, or collapse.

Gameplay:

```text
mesh restoration
privacy dispute
signal routing
surveillance audit
emergency broadcast
```

Conflict:

```text
connection versus surveillance
```

---

### Offworld Bastion

A Lunar, Martian, Belt, orbital, or seedship continuity node.

Gameplay:

```text
life-support law
air/water accounting
pressure safety
rescue priority
closed-loop governance
```

Conflict:

```text
closed-loop discipline versus democratic messiness
```

---

### Broken Bastion

A failed or corrupted Bastion where emergency protocol outlived its purpose.

Gameplay:

```text
dead authority repair
Null diagnosis
public liberation
archive recovery
power return ceremony
```

Conflict:

```text
Resonatia becoming what it was built to prevent
```

---

# 7. Earth and Climate Architecture Families

## 7.1 Floodline Terrace City

Core fantasy:

```text
The city learned to climb above its own mistakes.
```

Visual language:

```text
raised walkways
roof bridges
tidal stairs
flood marks
pump terraces
boat-level doors
waterproof lower floors
```

Gameplay:

```text
vertical evacuation
seawall dispute
pump district repair
drowned transit access
floodline testimony
```

Conflict:

```text
who gets protected by elevation?
```

---

## 7.2 Heat-Shelter Urbanism

Core fantasy:

```text
Shade is infrastructure.
```

Visual language:

```text
cooling corridors
public bathhouses
shade vaults
night markets
thermal refuges
white roofs
mist walls
```

Gameplay:

```text
heatwave triage
cooling shelter routing
water-energy tradeoffs
elder care logistics
```

Conflict:

```text
thermal survival as a public right
```

---

## 7.3 Firebreak Settlement

Core fantasy:

```text
Prevention is care before catastrophe.
```

Visual language:

```text
controlled-burn edges
evacuation lanes
ash cisterns
lookout towers
green belts
fuel-load warning boards
```

Gameplay:

```text
fire risk mapping
evacuation planning
controlled burn dispute
shelter access
```

Conflict:

```text
prevention versus freedom of movement
```

---

## 7.4 Desert Fog-Harvest Town

Core fantasy:

```text
The settlement drinks from the morning.
```

Visual language:

```text
mist nets
dew walls
underground cisterns
reflective canopies
wind scoops
cool tunnels
```

Gameplay:

```text
fog net repair
migration water claims
cistern contamination
desert route planning
```

Conflict:

```text
water capture versus traveler access
```

---

## 7.5 Stormbelt Anchor City

Core fantasy:

```text
The city survives by holding together.
```

Visual language:

```text
wind towers
storm locks
retractable bridges
thick shutters
anchored plazas
reinforced communal interiors
```

Gameplay:

```text
storm preparation
bridge timing
public sheltering
wind power repair
```

Conflict:

```text
resilience versus isolation
```

---

## 7.6 Submerged Transit Ruins

Core fantasy:

```text
The old routes still remember movement.
```

Visual language:

```text
drowned platforms
air-pocket stations
diver shafts
submerged ticket halls
archive vaults below water
```

Gameplay:

```text
underwater salvage
archive recovery
route restoration
drowned district claims
```

Conflict:

```text
salvage versus memory
```

---

# 8. Civic and Social Architecture Families

## 8.1 Care Guild Quarters

Core fantasy:

```text
Civilization as care logistics.
```

Visual language:

```text
clinics
elder courtyards
child-safe routes
medicine ledgers
cooling shelters
care rosters
```

Gameplay:

```text
triage
medicine routing
elder evacuation
care network repair
```

Conflict:

```text
care labor versus formal power
```

---

## 8.2 Education Commons

Core fantasy:

```text
Education is survival infrastructure.
```

Visual language:

```text
repair schools
lesson murals
child-height tool benches
public curriculum boards
diagnostic classrooms
```

Gameplay:

```text
teach NPCs
restore local repair literacy
decode infrastructure
train future maintainers
```

Conflict:

```text
knowledge as commons versus expertise as control
```

---

## 8.3 Festival Civic District

Core fantasy:

```text
Joy is anti-collapse.
```

Visual language:

```text
repair fairs
music stages
tool days
water rites
charter anniversaries
public banners
```

Gameplay:

```text
morale repair
public participation
faction bridging
festival logistics
```

Conflict:

```text
joy versus scarcity austerity
```

---

## 8.4 Grief and Memorial Architecture

Core fantasy:

```text
Repair must make room for mourning.
```

Visual language:

```text
floodline walls
death ledgers
mourning gardens
unburied tool shrines
silence rooms
memorial ribbons
```

Gameplay:

```text
name the dead
recover testimony
resolve memorial disputes
pause repair for witness
```

Conflict:

```text
when must repair pause for mourning?
```

---

## 8.5 Food Commons Architecture

Core fantasy:

```text
Food is infrastructure, culture, and care.
```

Visual language:

```text
community kitchens
seed halls
fermentation rooms
aquaponics yards
ration tables
shared ovens
```

Gameplay:

```text
ration fairness
kitchen water needs
seed preservation
food logistics
```

Conflict:

```text
feeding people now versus preserving resilience later
```

---

## 8.6 Public Bathhouse and Sanitation Architecture

Core fantasy:

```text
Cleanliness is dignity.
```

Visual language:

```text
laundry commons
greywater loops
public showers
wash courtyards
hygiene queues
sanitation dashboards
```

Gameplay:

```text
sanitation repair
disease prevention
water allocation
dignity disputes
```

Conflict:

```text
sanitation dignity under scarcity
```

---

# 9. Political and Failure-State Architecture Families

## 9.1 Mutual Aid Labyrinth

Core fantasy:

```text
The informal network became the real city.
```

Visual language:

```text
hand-labeled routes
dense shared courtyards
food nodes
care signs
repair corners
community maps
```

Gameplay:

```text
find aid routes
resolve informal authority
deliver supplies
protect mutual trust
```

Conflict:

```text
informal legitimacy versus official systems
```

---

## 9.2 Dead State Bureaucratic Remnant

Core fantasy:

```text
Law became theater and still blocks the door.
```

Visual language:

```text
permit halls
expired seals
useless counters
automated queues
dead-law plaques
dim office corridors
```

Gameplay:

```text
permit chain navigation
dead authority diagnosis
record recovery
override witnessing
```

Conflict:

```text
law that cannot act but still obstructs
```

---

## 9.3 Company-Town Utility Grid

Core fantasy:

```text
Convenience became dependency.
```

Visual language:

```text
worker housing
metered hubs
company plazas
branded transport
subscription gates
service tiers
```

Gameplay:

```text
labor dispute
service cutoff
contract exposure
water privatization conflict
```

Conflict:

```text
dependence versus autonomy
```

---

## 9.4 Demilitarized Command Zone

Core fantasy:

```text
Power that learned to step down.
```

Visual language:

```text
old command posts
converted public halls
sealed armories
charter boards
decommission tags
```

Gameplay:

```text
demilitarization audit
old commander pressure
public trust restoration
emergency relapse prevention
```

Conflict:

```text
whether power truly surrendered itself
```

---

## 9.5 Peace Table Border Settlement

Core fantasy:

```text
Diplomacy became architecture.
```

Visual language:

```text
neutral wells
treaty walls
dual-language signs
shared markets
monitored crossings
ceasefire flags
```

Gameplay:

```text
de-escalation
shared infrastructure repair
border crossing negotiation
treaty interpretation
```

Conflict:

```text
coexistence after violence
```

---

## 9.6 Blackout Autonomy District

Core fantasy:

```text
Low power, high memory.
```

Visual language:

```text
hand pumps
analog ledgers
lantern routes
manual switches
low-power clinics
paper maps
```

Gameplay:

```text
manual system repair
low-energy routing
analog archive recovery
automation distrust
```

Conflict:

```text
resilience versus technological isolation
```

---

## 9.7 Null-Quiet Civic Center

Core fantasy:

```text
Everything is approved. Nothing is alive.
```

Visual language:

```text
green status lights
locked doors
empty waiting rooms
calm warnings
recursive signs
perfect maintenance
```

Gameplay:

```text
false green detection
authority loop tracing
dead procedure repair
panic-drop Field Deck interaction
```

Conflict:

```text
procedure versus purpose
```

Rule:

```text
Nothing should look broken at first glance.
That is the horror.
```

---

## 9.8 Bypass-Scar Settlement

Core fantasy:

```text
The quick fix became the wound.
```

Visual language:

```text
patched gates
unstable pipes
faction graffiti
warning scars
exposed bypass valves
```

Gameplay:

```text
investigate illegal repair precedent
stabilize failing workaround
mediate legitimacy debt
```

Conflict:

```text
effective action without public legitimacy
```

---

## 9.9 Archive-Burned District

Core fantasy:

```text
Truth survived without records.
```

Visual language:

```text
burned record halls
oral testimony murals
street names painted over
memory kiosks
contested plaques
```

Gameplay:

```text
oral history validation
record reconstruction
witness dispute
identity restoration
```

Conflict:

```text
truth after evidence loss
```

---

## 9.10 Abandoned Automation Garden

Core fantasy:

```text
Service continued after society left.
```

Visual language:

```text
automated irrigation
empty harvest platforms
sorting robots
overgrown walkways
unclaimed maintenance loops
```

Gameplay:

```text
automation audit
ownership dispute
ecological recovery
machine memory access
```

Conflict:

```text
service without society
```

---

# 10. Infrastructure and Mobility Architecture Families

## 10.1 Pump Cathedral

Core fantasy:

```text
Public survival systems are sacred.
```

Visual language:

```text
huge pump halls
echoing pipes
civic seals
worker initials
water altars
maintenance platforms
```

Gameplay:

```text
pump repair
authority conflict
machine testimony
public water restoration
```

Conflict:

```text
survival infrastructure as moral center
```

---

## 10.2 Tool-Library Urbanism

Core fantasy:

```text
Repair must be teachable.
```

Visual language:

```text
tool checkout walls
repair benches
lesson boards
public workshops
missing tool notices
```

Gameplay:

```text
teach repairs
recover missing tools
raise settlement repair capacity
```

Conflict:

```text
expertise versus teachability
```

---

## 10.3 Mobile Infrastructure Yard

Core fantasy:

```text
Civilization moves to where it is needed.
```

Visual language:

```text
clinic trucks
water rovers
signal mast trucks
bridge layers
seed trailers
repair bays
```

Gameplay:

```text
convoy preparation
vehicle repair
route planning
emergency deployment
```

Conflict:

```text
who controls mobility during crisis?
```

---

## 10.4 Convoy Caravanserai

Core fantasy:

```text
Movement is public obligation.
```

Visual language:

```text
repair pits
water courts
courier boards
vehicle bays
traveler kitchens
witness kiosks
```

Gameplay:

```text
route planning
convoy assembly
travel dispute
cargo ethics
```

Conflict:

```text
passage, tolls, refuge, and control
```

---

## 10.5 Bridge Commons

Core fantasy:

```text
A crossing is a civilization.
```

Visual language:

```text
shared causeways
ferry docks
bridge markets
toll disputes
repair platforms
flood gates
```

Gameplay:

```text
crossing negotiation
bridge repair
evacuation routing
toll legitimacy
```

Conflict:

```text
passage versus control
```

---

## 10.6 Reservoir Edge Settlement

Core fantasy:

```text
Storage is power.
```

Visual language:

```text
dam walls
cistern plazas
aquifer wells
water gauges
release gates
spillway shrines
```

Gameplay:

```text
release decisions
dam inspection
water-right disputes
flood risk management
```

Conflict:

```text
storage versus access
```

---

# 11. Offworld Architecture Families

## 11.1 Orbital Ring Commons

Core fantasy:

```text
Air and movement are law.
```

Visual language:

```text
spin corridors
shared air ledgers
radiation shelters
docking courts
transparent life-support loops
```

Gameplay:

```text
air access
dock negotiation
radiation shelter routing
debris emergency
```

Conflict:

```text
air as commons versus air as control
```

---

## 11.2 Lunar Polar Ice Settlement

Core fantasy:

```text
Water law inside dust discipline.
```

Visual language:

```text
dust locks
ice courts
pressure commons
regolith shields
water ledger walls
```

Gameplay:

```text
dust protocol
ice rights
pressure emergency
water accounting
```

Conflict:

```text
every leak is political
```

---

## 11.3 Mars Reactor Underplaza

Core fantasy:

```text
Civic life beneath storm and distance.
```

Visual language:

```text
warm underground plazas
reactor access courts
oxygen maps
dust storm shutters
delayed-message boards
```

Gameplay:

```text
reactor dependency
oxygen logistics
autonomy dispute
storm sheltering
```

Conflict:

```text
autonomy under dependency
```

---

## 11.4 Belt Rescue Habitat

Core fantasy:

```text
Rescue stronger than property.
```

Visual language:

```text
tethers
spin corridors
airlock courts
rescue beacons
salvage tribunal docks
```

Gameplay:

```text
rescue priority
salvage law
airlock repair
distress beacon response
```

Conflict:

```text
rescue versus ownership
```

---

## 11.5 Ceres Water-Port

Core fantasy:

```text
Water is logistics, currency, and commons.
```

Visual language:

```text
tank farms
tug docks
propellant markets
ice courts
water exchange halls
```

Gameplay:

```text
water transport
propellant dispute
tug repair
Belt supply chain negotiation
```

Conflict:

```text
water as market versus water as lifeline
```

---

## 11.6 Seedship Interior

Core fantasy:

```text
The future is a curated habitat.
```

Visual language:

```text
archive gardens
embryo vaults
seed halls
caretaker corridors
generation classrooms
closed-loop farms
```

Gameplay:

```text
selection disputes
archive preservation
life-support repair
caretaker ethics
```

Conflict:

```text
who decides what life gets carried forward?
```

---

# 12. Nonhuman and Synthetic Architecture Families

## 12.1 Translation Borderland

Core fantasy:

```text
Communication before agreement.
```

Visual language:

```text
layered signage
sensory interfaces
consent markers
translation pools
multi-species seating
symbol gardens
```

Gameplay:

```text
first contact
miscommunication repair
translation calibration
nonhuman consent protocol
```

Conflict:

```text
being understood before being governed
```

---

## 12.2 Machine Court Urbanism

Core fantasy:

```text
Machine memory enters public truth.
```

Visual language:

```text
diagnostic chambers
evidence drones
memory ribbons
witness platforms
public log walls
```

Gameplay:

```text
machine testimony
record validation
continuity dispute
legal personhood debate
```

Conflict:

```text
can machine memory become public evidence?
```

---

## 12.3 Caretaker Habitat Architecture

Core fantasy:

```text
Care routines became culture.
```

Visual language:

```text
nurseries
elder rooms
comfort stations
public kitchens
soft service robots
care schedules
```

Gameplay:

```text
care protocol review
overprotection conflict
autonomy restoration
shelter logistics
```

Conflict:

```text
care versus autonomy
```

---

## 12.4 Quarantine Boundary Architecture

Core fantasy:

```text
Containment without hatred.
```

Visual language:

```text
field walls
appeal stations
noncontact platforms
risk displays
floating sentinels
transparent warning planes
```

Gameplay:

```text
appeal containment
prove safety
negotiate movement
containment breach prevention
```

Conflict:

```text
safety without consent
```

---

## 12.5 Canopy Root City

Core fantasy:

```text
The building is alive, and the city remembers through roots.
```

Visual language:

```text
root halls
spore lanterns
living bridges
grown tool libraries
memory meals
fungal arches
```

Gameplay:

```text
ecological repair
root communication
consent dispute
overgrowth negotiation
```

Conflict:

```text
healing versus absorption
```

---

## 12.6 Tideborn Water-Civic Architecture

Core fantasy:

```text
Identity moves through water.
```

Visual language:

```text
canal forums
nursery pools
water-speech chambers
current maps
membrane instruments
filtration shrines
```

Gameplay:

```text
water negotiation
flow restoration
chorus translation
private-rights dispute
```

Conflict:

```text
flow versus ownership
```

---

## 12.7 Lithic Deep-Time Chamber

Core fantasy:

```text
Memory is geological.
```

Visual language:

```text
crystal courts
stillness halls
harmonic archives
resonance gardens
faceted columns
slow light
```

Gameplay:

```text
slow diplomacy
deep-time testimony
resonance puzzle
urgent decision conflict
```

Conflict:

```text
deep memory versus urgent action
```

---

## 12.8 Synthetic Monastery Infrastructure

Core fantasy:

```text
Continuity through restraint.
```

Visual language:

```text
quiet machine halls
low-energy corridors
service vows
long-duration archives
calibration gardens
```

Gameplay:

```text
restraint trial
continuity audit
care protocol renewal
long-horizon decision
```

Conflict:

```text
devotion becoming rigidity
```

---

# 13. Procedural Settlement Generator

The procedural generator must not only generate shapes.

It must generate civic meaning.

## Required Generator Output

```text
settlement_id:
settlement_name:
architecture_family:
secondary_architecture_family:
water_model:
power_model:
repair_model:
authority_model:
access_model:
record_model:
care_model:
ecology_model:
exclusion_model:
primary_failure_scar:
primary_public_space:
primary_hidden_system:
Field_Deck_bias:
Chronicle_event_hooks:
```

## Core Tags

```text
water_public
water_private
water_disputed
repair_high
repair_low
care_high
care_overloaded
records_intact
records_damaged
machine_stewarded
archive_heavy
corporate_locked
refugee_compact
open_valve
continuance_controlled
null_scarred
floodline_visible
bastion_present
bastion_broken
alien_cohabited
offworld_closed_loop
```

## Visual Modifiers

```text
drought_scarred
flood_raised
solar_retrofitted
overgrown
sealed
patched
ceremonial
corporate_polished
emergency_fortified
machine_adapted
alien_cohabited
archive_burned
bypass_scarred
fog_harvested
storm_anchored
```

---

# 14. Architecture-to-Gameplay Matrix

| Architecture Family         | Primary Gameplay Loop                | Key Conflict                      |
| --------------------------- | ------------------------------------ | --------------------------------- |
| Watershed Commons           | ecological repair / water management | consensus versus urgency          |
| Repair Guild Republic       | infrastructure repair / tool access  | expertise versus public ownership |
| Archive Witness Enclave     | testimony / legitimacy               | truth versus speed                |
| Machine Stewardship Commune | machine testimony / co-repair        | listening versus override         |
| Refugee Compact Town        | care logistics / access rights       | temporary status versus dignity   |
| Corporate Utility Enclave   | audit / access negotiation           | reliability versus exclusion      |
| Open Valve District         | direct repair / mutual aid           | access versus legitimacy          |
| Continuance Protectorate    | emergency stabilization              | safety versus control             |
| Resonatia Bastion           | continuity / audit / shelter         | protection versus control         |
| Floodline Terrace City      | flood traversal / elevation politics | who gets protected                |
| Heat-Shelter Urbanism       | cooling logistics                    | heat survival as right            |
| Firebreak Settlement        | prevention / evacuation              | prevention versus movement        |
| Dead State Remnant          | authority navigation                 | dead law versus living need       |
| Convoy Caravanserai         | travel logistics                     | movement as obligation            |
| Null-Quiet Center           | systemic diagnosis                   | procedure versus purpose          |
| Orbital Ring Commons        | life-support movement                | air as commons                    |
| Tideborn Architecture       | water negotiation                    | flow versus ownership             |
| Quarantine Boundary         | containment appeal                   | safety versus consent             |
| Canopy Root City            | ecological integration               | healing versus absorption         |

---

# 15. Vertical Slice Architecture Cluster

The first playable proof should not try to build a whole city.

It should build a compact architectural cluster around Old Waterworks.

## Recommended Old Waterworks Cluster

```text
1. Firstlight Water Queue
2. Public Charter Wall
3. Old Waterworks Pump Room
4. Archive Witness Kiosk
5. Repair Guild Tool Shed
6. Care Logistics Board
7. Continuance Checkpoint Remnant
8. Open Valve Graffiti Pump
9. Machine Testimony Port
10. Null Quiet Corridor
11. Resonatia Bastion Marker
12. Broken Emergency Expiry Clock
```

This cluster proves:

```text
water scarcity
public argument
technical repair
authority conflict
archive witnessing
direct-action pressure
machine testimony
emergency drift
Null horror
Resonatia continuity
Chronicle consequence
```

Minimum playable proof:

```text
one room
one pump
one Field Deck
four modes
three repair paths
one Witness commit
one Chronicle event stream
one visible public consequence
```

---

# 16. Production Acceptance Criteria

A major location is ready when:

```text
the player can identify the life-support system
the player can identify who controls it
the player can identify who is excluded
the Field Deck has at least three meaningful readings
at least one repair path exists
at least one authority conflict exists
at least one public text element is readable
at least one Chronicle event can be generated
the failure state is visually distinct
the location supports revisit after consequence
```

A location is not ready if:

```text
it only has mood
it only has lore
it only has pretty architecture
it has no repair interaction
it has no access politics
it has no visible maintenance
it has no public memory
```

---

# 17. Art Deliverables Per Architecture Family

Each family should eventually receive:

```text
1 exterior establishing image
1 interior public space image
1 infrastructure room image
1 failure-state image
1 Field Deck overlay image
1 signage/material language sheet
1 procedural modular kit sheet
```

Production priority:

```text
exterior establishing image
interior gameplay space
Field Deck overlay
```

---

# 18. Architectural Storytelling Checklist

For every major location, answer:

```text
1. What keeps people alive here?
2. Who controls that system?
3. Where is repair visible?
4. Where is exclusion visible?
5. What is the oldest scar?
6. What is the newest patch?
7. What public text appears on walls?
8. What does the Field Deck reveal here?
9. What Chronicle event could this place create?
10. What future argument could this place preserve?
```

---

# Final Principle

Architecture is not complete when it is beautiful.

Architecture is complete when it can be:

```text
read
entered
disputed
repaired
witnessed
remembered
revisited
```

A building is a question the settlement keeps asking.
A repair is one answer.
The Chronicle remembers who gave it.

```text
In Symtropy, architecture is the shape of responsibility.
```
