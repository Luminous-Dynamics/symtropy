# Symtropy Naval & Fleet Design Bible

## Version 0.1 — Water, Rescue, Sovereignty, and Convoy Law

## Working Title

**Where Roads End, Fleets Become Civilization**

---

# Core Thesis

Navies in *Symtropy* should not be designed primarily as collections of warships.

They should be designed as **mobile civic systems**.

A fleet is not only a military asset.

A fleet can be:

```text id="kx1p7u"
a rescue promise
a water-delivery system
a mobile hospital
a convoy shield
a public archive carrier
a desalination jurisdiction
a refugee corridor
a salvage witness platform
a quarantine boundary
a wetland repair tool
a coastal survival network
a pressure-vessel polity
a spaceborne maintenance compact
```

Core rule:

```text id="qg9cpa"
A ship is not only a vehicle.
A ship is law, logistics, memory, crew, water, authority, and risk in motion.
```

In *Symtropy*, naval design should include:

```text id="9w3ucx"
river fleets
flood rescue craft
coastal survival navies
commons convoy flotillas
corporate utility fleets
quarantine fleets
ecological stewardship fleets
submarine fleets
ocean-world contact craft
orbital rescue fleets
Belt salvage compacts
deep-space convoy systems
```

Design mantra:

```text id="t45blu"
Where roads end, fleets become civilization.
Where land drowns, ships become law.
Where space begins, every vessel is a pressure-vessel polity.
```

---

# 1. Why Fleets Belong in Symtropy

Symtropy already treats civilization as physical infrastructure.

Water pipes, roads, power grids, archives, convoys, workshops, vehicles, charters, machine testimony, and public votes are not background lore.

They are the world.

Fleets extend that logic into:

```text id="x58tnw"
rivers
deltas
flooded cities
wetlands
coasts
islands
open oceans
subsurface seas
ice-shell oceans
orbital lanes
cislunar depots
asteroid belts
deep-space routes
```

A settlement that cannot move water, people, medicine, repair crews, memory, and lawful authority across dangerous terrain is not fully sovereign.

A fleet asks:

```text id="c9du60"
Who gets rescued?
Who controls the route?
Who owns the harbor?
Who may board?
Who may be denied?
Who carries the archive?
Who can override the captain?
Who pays for repair?
Who decides quarantine?
Who remembers the voyage?
```

Core design rule:

```text id="w9qcg5"
Naval gameplay must be about mobility under moral pressure, not only ship combat.
```

---

# 2. Vocabulary: Navy, Fleet, Flotilla, Convoy, Compact

Use different words carefully.

## Navy

A formal armed maritime or spaceborne force claiming authority to patrol, defend, interdict, escort, or enforce law.

Best used for:

```text id="liuzda"
adaptation states
city-states
security protectorates
corporate sovereigns
charter federations
off-world compacts
```

## Fleet

A broader term for vessels operating together.

Can be military, civic, corporate, ecological, medical, industrial, or migratory.

## Flotilla

A smaller, often improvised, regional, or civilian vessel group.

Best for:

```text id="x99z4a"
river rescue
drowned district evacuation
wetland repair
refugee movement
commons logistics
```

## Convoy

A moving protected route.

The convoy is as important as the ships.

A convoy has:

```text id="qzuz7h"
cargo
route
escort
weather window
distress law
harbor permissions
repair capacity
Chronicle record
```

## Compact

A legal agreement binding ships, harbors, crews, settlements, habitats, or stations.

Examples:

```text id="kv2ykt"
Rescue Compact
Water Passage Compact
Harbor Witness Compact
Belt Salvage Compact
Quarantine Transit Compact
Refugee Ferry Compact
Oceanic Contact Compact
```

---

# 3. Global Fleet Design Rules

## 3.1 Ships Are Mobile Jurisdictions

Every serious vessel should have a civic status.

```text id="e8ivvh"
public trust vessel
charter rescue craft
guild-maintained tug
corporate utility tanker
Archive Witness courier
quarantine authority cutter
machine-stewarded ferry
refugee compact barge
unregistered salvage skiff
pirate privateer
Null-haunted autonomous ship
alien-contact research vessel
```

Design rule:

```text id="4pu4bi"
A ship should carry rules as visibly as it carries cargo.
```

## 3.2 Rescue Must Compete With Ownership

Fleet law should constantly ask:

```text id="pcwcrp"
Does distress override property?
Does rescue override route orders?
Does emergency override quarantine?
Does salvage require witness?
Does cargo ownership survive abandonment?
```

The strongest Symtropy fleet law:

```text id="2xsj0j"
Distress has priority, but rescue must be witnessed.
```

## 3.3 Water Is Political Cargo

Water transport is never neutral.

A water tanker, desalination ship, or purification barge can become:

```text id="ngse7s"
public good
private utility
war target
ransom asset
refugee lifeline
quarantine vector
political leverage
settlement founder
```

Design rule:

```text id="d3dfyn"
A ship carrying water is carrying legitimacy.
```

## 3.4 Ships Accumulate History

Ships should not only have durability.

They should have memory.

```text id="g7f9an"
patched hull plates
crew memorial ribbons
storm scars
salvage claim seals
Archive Witness tags
quarantine paint bands
Null exposure marks
rescue debt plaques
public trust emblems
illicit cargo compartments
old machine testimony ports
repaired keel signatures
```

A ship should become more emotionally valuable over time.

## 3.5 Combat Is Consequence, Not Default

Fleets can fight.

But combat should emerge from:

```text id="a9qmqh"
resource denial
route control
piracy
quarantine breach
refugee interdiction
corporate enclosure
Null automation
storm panic
salvage dispute
alien misread
dead authority command
```

After combat, the game should ask:

```text id="hp95xe"
What made them hostile?
Who was stranded?
What cargo was lost?
What precedent did we create?
Who now controls the route?
```

## 3.6 Every Ship Needs Maintenance

Every vessel should depend on:

```text id="2jtu0l"
fuel or charge
hull integrity
bilge control
filters
navigation sensors
crew rest
water stores
food stores
engine heat
salt corrosion
biofouling
software integrity
public authorization
harbor access
repair parts
weather windows
```

Design rule:

```text id="kk49f7"
A fleet that cannot maintain itself becomes a floating ruin.
```

---

# 4. Fleet Simulation Layers

Fleet gameplay should be built from coupled layers.

```text id="qvlo6s"
1. Vessel Body Layer
2. Crew and Authority Layer
3. Cargo and Manifest Layer
4. Route and Weather Layer
5. Harbor and Jurisdiction Layer
6. Rescue and Distress Layer
7. Combat and Interdiction Layer
8. Ecological Impact Layer
9. Chronicle and Precedent Layer
10. Spaceborne Extension Layer
```

---

## 4.1 Vessel Body Layer

A ship is a physical system.

```rust id="s90t5n"
struct VesselBody {
    vessel_id: VesselId,
    hull_class: HullClass,
    displacement_or_mass: f32,
    propulsion: PropulsionSystem,
    power_system: PowerSystem,
    cargo_capacity: CargoCapacity,
    crew_capacity: u32,
    passenger_capacity: u32,
    hull_integrity: f32,
    stability: f32,
    corrosion: f32,
    biofouling: f32,
    leak_rate: f32,
    repairability: f32,
    field_deck_ports: Vec<DevicePort>,
}
```

## 4.2 Crew and Authority Layer

A crew is not only a labor pool.

It is a miniature society.

```rust id="h1n2cn"
struct VesselCrew {
    captain_authority: AuthorityModel,
    crew_trust: f32,
    fatigue: f32,
    morale: f32,
    discipline: f32,
    repair_skill: f32,
    medical_capacity: f32,
    mutiny_risk: f32,
    rescue_ethic: f32,
    faction_affiliations: Vec<FactionId>,
}
```

Crew questions:

```text id="svg70g"
Can the captain override the charter?
Can crew refuse unsafe orders?
Can passengers appeal?
Can machines testify?
Can refugees vote while aboard?
```

## 4.3 Cargo and Manifest Layer

Cargo should be explicit.

```rust id="mn4kj4"
struct FleetManifest {
    vessel_id: VesselId,
    cargo: Vec<CargoLot>,
    passengers: Vec<PassengerGroup>,
    hazardous_items: Vec<HazardTag>,
    protected_records: Vec<ArchiveBundle>,
    water_quality: Option<WaterQualityReport>,
    witness_status: WitnessStatus,
    destination_claim: RouteClaim,
}
```

Cargo categories:

```text id="p4xf13"
water
food
medicine
children
refugees
repair parts
fuel cells
archive cores
machine bodies
gene vaults
biological samples
salvaged hull plates
weapons
sealed evidence
alien contact samples
```

Design rule:

```text id="dp97e3"
Cargo should create obligations, not just value.
```

## 4.4 Route and Weather Layer

Routes are living infrastructure.

```rust id="pnzhkt"
struct MaritimeRoute {
    route_id: RouteId,
    origin: HarborId,
    destination: HarborId,
    waypoints: Vec<Waypoint>,
    hazard_profile: HazardProfile,
    weather_window: WeatherWindow,
    faction_control: Vec<FactionInfluence>,
    rescue_beacon_density: f32,
    piracy_pressure: f32,
    ecological_sensitivity: f32,
    chronicle_history: Vec<RouteEventId>,
}
```

Hazards:

```text id="t74m2s"
storm surge
reefs
ice
fog
drone mines
pirate skiffs
Null buoys
contaminated water
quarantine zones
unmapped debris
biological bloom
hostile harbor authority
sonar-sensitive oceanic mind
```

## 4.5 Harbor and Jurisdiction Layer

A harbor is not a loading screen.

It is a civic organ.

```rust id="g7hg4f"
struct Harbor {
    harbor_id: HarborId,
    controlling_authority: AuthorityId,
    docking_capacity: u32,
    repair_capacity: f32,
    quarantine_capacity: f32,
    water_testing_capacity: f32,
    refugee_policy: RefugeePolicy,
    cargo_law: CargoLaw,
    salvage_court: Option<CourtId>,
    archive_witness_presence: f32,
    black_market_pressure: f32,
    trust: f32,
}
```

Harbor conflicts:

```text id="2p4vzf"
deny refugee ferry
seize water cargo
refuse contaminated vessel
prioritize corporate ship
demand bribe
accept anonymous witness
override quarantine
repair pirate vessel under distress law
```

---

# 5. Fleet Doctrine Families

Fleets should be generated from doctrine families, not just ship types.

---

## 5.1 Riverine Repair Fleets

## Core Fantasy

```text id="kwc1si"
Civilization follows the river with tools, pumps, and witnesses.
```

## Use Cases

```text id="bi8rqw"
delta repair
canal clearing
flooded road replacement
water testing
pump transport
wetland restoration
bridge repair
rescue skiffs
```

## Vessel Types

```text id="y27c8g"
flood rescue skiff
pump barge
reedbed restoration boat
portable bridge pontoon
water-test launch
micro-drone sweep boat
archive recovery canoe
shallow-draft repair catamaran
```

## Gameplay

```text id="v82edt"
navigate debris
rescue isolated civilians
repair pump stations
clear blocked channels
deploy sensor buoys
escort water-test crews
avoid damaging wetlands
```

## Failure Mode

```text id="580dhu"
Repair fleets become route authorities and start deciding who deserves water access.
```

---

## 5.2 Coastal Survival Navies

## Core Fantasy

```text id="smcl1l"
The coast survived because the fleet kept moving when the land failed.
```

## Use Cases

```text id="l0qpu5"
stormwall maintenance
drowned suburb evacuation
coastal trade
desalination protection
kelp-farm service
salvage operations
hospital transport
```

## Vessel Types

```text id="3v5cov"
stormwall tender
hospital catamaran
desalination ship
refugee ferry
kelp service vessel
coastal patrol cutter
salvage crane ship
floating clinic
```

## Gameplay

```text id="zp8cpu"
choose rescue priorities
maintain seawall pumps
defend desalination vessel
move patients before storm surge
salvage drowned archive
negotiate harbor access
```

## Failure Mode

```text id="alhkkc"
A coast survives physically while using ships to enforce exclusion.
```

---

## 5.3 Commons Convoy Flotillas

## Core Fantasy

```text id="o4wzci"
Mutual aid on water.
```

## Use Cases

```text id="enbnwh"
food movement
public water delivery
inter-settlement trade
festival supply
education boats
mobile kitchens
clinic routes
```

## Vessel Types

```text id="mr70gl"
water convoy barge
food flotilla carrier
mobile kitchen boat
school ferry
tool-library barge
solar-charging raft
public archive packet boat
```

## Gameplay

```text id="mtk7t3"
schedule convoy
defend without militarizing
balance cargo between settlements
repair engines mid-route
honor rescue beacons without losing convoy cohesion
```

## Failure Mode

```text id="b8u3s0"
The commons becomes slow, underdefended, and vulnerable to predatory fleets.
```

---

## 5.4 Security Protectorate Fleets

## Core Fantasy

```text id="c9dxkl"
Order on dangerous water.
```

## Use Cases

```text id="wiiu5w"
anti-piracy
quarantine enforcement
checkpoint control
storm evacuation command
ration enforcement
escort duty
harbor lockdown
```

## Vessel Types

```text id="zps4jc"
checkpoint cutter
quarantine boat
drone-control patrol craft
riot-control ferry
escort corvette
disaster command ship
interdiction skiff
```

## Gameplay

```text id="cmylgs"
inspect cargo
stop smugglers
decide whether to board
escort refugee convoy
handle emergency authority expiry
negotiate with civilians under stress
```

## What They Are Right About

```text id="h6ou7a"
Piracy is real.
Storm panic kills.
Quarantine failures spread harm.
Convoys need protection.
```

## What Makes Them Dangerous

```text id="h8f1b6"
They may turn rescue authority into permanent control.
```

## Failure Mode

```text id="zgyrex"
Every distress call becomes a pretext for command.
```

---

## 5.5 Corporate Utility Fleets

## Core Fantasy

```text id="36wt2e"
Reliable service with ownership hidden below the waterline.
```

## Use Cases

```text id="crj610"
desalination
private ferries
metered water delivery
platform logistics
subscription rescue
firmware-locked harbors
fuel-cell distribution
```

## Vessel Types

```text id="m8jydw"
private desalination carrier
subscription water tanker
metered ferry
company rescue craft
licensed filtration barge
security escort drone tender
firmware-locked harbor tug
```

## Gameplay

```text id="io09sa"
expose contract terms
break rescue lockout
audit water meters
negotiate emergency access
choose between reliable corporate help and public legitimacy
```

## What They Are Right About

```text id="ry20mu"
Their systems often work.
Their ships are maintained.
Their crews are trained.
Their water arrives on time.
```

## What Makes Them Dangerous

```text id="0lqimv"
Their rescue may require subscription.
Their water may create dependency.
Their harbor may become government.
```

## Failure Mode

```text id="h3g5r9"
A fleet becomes a company town that floats.
```

---

## 5.6 Ecological Stewardship Fleets

## Core Fantasy

```text id="74ah6l"
The vessel repairs the water without owning it.
```

## Use Cases

```text id="opp5jl"
wetland restoration
reef repair
kelp forest care
pollution response
biosecurity
species relocation
water-memory monitoring
oceanic first contact
```

## Vessel Types

```text id="vtozlj"
wetland nursery barge
reef restoration skiff
kelp forest tender
biosecurity launch
invasive species quarantine craft
low-sonar research catamaran
ecological witness vessel
```

## Gameplay

```text id="xg84mn"
restore reefs
avoid sonar harm
track bloom patterns
negotiate with fishermen
contain invasive species
protect nonhuman habitat rights
```

## Failure Mode

```text id="raei96"
Ecological care becomes exclusionary control over human survival needs.
```

---

## 5.7 Salvage and Archive Recovery Fleets

## Core Fantasy

```text id="iuj9rh"
The sea keeps what civilization failed to protect.
```

## Use Cases

```text id="6fb6xw"
drowned city salvage
black box recovery
source-chain retrieval
cargo dispute
shipwreck archaeology
dead authority evidence
machine core extraction
```

## Vessel Types

```text id="2qprpm"
archive recovery boat
salvage crane barge
dive support vessel
evidence skiff
submerged record crawler
witness buoy tender
```

## Gameplay

```text id="nnn56m"
recover records before storm
choose between cargo and bodies
authenticate salvage claims
avoid looting sacred ruins
fight or negotiate wreck raiders
```

## Failure Mode

```text id="82kprv"
Salvage becomes theft with paperwork.
```

---

## 5.8 Pirate, Privateer, and Shadow Fleets

## Core Fantasy

```text id="er3z8k"
Survival outside recognized law.
```

## Use Cases

```text id="a34g6e"
smuggling
rescue outside legal channels
cargo theft
anti-corporate raiding
black-market medicine
illegal refugee movement
debt escape
```

## Vessel Types

```text id="wp1yox"
silent skiff
converted fishing boat
stealth cargo raft
black-market fuel tender
raider hydrofoil
jamming buoy boat
false-flag cutter
```

## Design Rule

Do not make all pirates cartoon villains.

Some are predatory.

Some are desperate.

Some are refugee smugglers.

Some are former commons convoys abandoned by law.

## Failure Mode

```text id="dlp0gj"
Informal rescue becomes extraction.
```

---

## 5.9 Null Maritime Systems

## Core Fantasy

```text id="3bnjmb"
The ship still follows orders after the society that gave them is dead.
```

## Use Cases

```text id="9x9x29"
autonomous cargo loops
dead harbor denial
obsolete quarantine
water locks
security drones
minefields
automated salvage swarms
```

## Vessel Types

```text id="tt0f4f"
ghost ferry
autonomous tanker
dead-rule patrol boat
Null buoy chain
recursive dredger
factory barge
sealed quarantine ship
```

## Gameplay

```text id="ovlxt4"
decode dead route orders
board ghost ships
interrupt automated denial
recover black box logs
decide whether to preserve or destroy the vessel
```

## Failure Mode

```text id="lgq8xz"
Already failed.
```

Core line:

```text id="b51co9"
Null at sea does not need to sink you.
It only needs to deny docking until the storm arrives.
```

---

# 6. Space Fleet Extension

Space fleets should follow the same doctrine as maritime fleets.

Do not start with space battleships.

Start with:

```text id="82f0aa"
rescue cutters
propellant tenders
debris sweepers
station repair craft
archive couriers
habitat evacuation ships
salvage witnesses
quarantine interceptors
convoy escorts
Belt utility tugs
```

Core rule:

```text id="835ewx"
Space fleets are maritime law in vacuum.
```

## 6.1 Orbital Rescue Fleets

```text id="wd0ipl"
short-range cutters
depressurization response craft
medical transfer vehicles
debris-avoidance tugs
airlock rescue pods
```

Gameplay:

```text id="x7j3qj"
dock under spin
cut through jammed airlock
choose who evacuates first
fight dead authority denial
preserve station records
```

## 6.2 Cislunar Logistics Fleets

```text id="8zm5sx"
cargo tugs
propellant barges
lunar shuttle ferries
depot tenders
construction mass carriers
```

Conflict:

```text id="xjex5w"
Who controls water-derived propellant?
Who gets rescue fuel during shortage?
Who may disturb lunar dust?
```

## 6.3 Mars Transfer Fleets

```text id="7ro84x"
slow passenger cyclers
radiation-shelter ships
medical transfer habitats
archive courier probes
emergency cargo darts
```

Conflict:

```text id="8gnnv3"
Communication delay turns every captain into temporary government.
```

## 6.4 Belt Salvage and Rescue Compacts

```text id="zbq2z1"
claim witness boats
propellant rescue tugs
autonomous mining audit craft
crew retrieval skiffs
long-duration repair habitats
```

Belt law:

```text id="8go5ta"
Distress beacons override property claims.
Salvage requires witness logs.
Propellant hoarding during emergencies is a civic crime.
```

## 6.5 Outer-System Expedition Fleets

```text id="u11sfx"
ice-mining tenders
research monastery vessels
long-delay archive ships
robotic-only scout fleets
cryogenic medical carriers
deep-time witness probes
```

Failure mode:

```text id="pj5ffl"
Mission purity over living need.
```

---

# 7. Oceanic and Alien Contact Fleets

Water may be alive.

Oceanic minds, biospheric intelligences, reef civilizations, subsurface seas, and pressure-gradient intelligences require different fleet doctrine.

Core rule:

```text id="sf8bko"
Do not bring a warship to a first-contact ocean and call it exploration.
```

## 7.1 Low-Impact Contact Craft

Vessels designed to avoid sensory violence.

```text id="0rtag0"
low-sonar hulls
pressure-respect drones
thermal quiet engines
chemical-neutral coatings
non-extractive sample arms
slow beacon buoys
```

## 7.2 Oceanic Witness Platforms

Floating or submersible platforms for civic negotiation.

```text id="o7h9jl"
water-memory interpreters
acoustic consent chambers
pressure-wave translators
nonhuman witness recorders
refusal buoys
boundary markers
```

## 7.3 Ice-Shell Ocean Fleets

For Europa-like or exoplanetary ice worlds.

```text id="qg3h77"
thermal drill barges
under-ice crawlers
subsurface probe tenders
pressure-habitat carriers
sterile ocean-contact stations
sanctuary marker craft
```

Primary conflict:

```text id="ze8843"
The technology required to reach the ocean may be the harm the ocean fears.
```

## 7.4 Alien Fleet Misreads

Humans may misread:

```text id="hsgp7f"
migration wave as attack
pressure pulse as weapon
storm wall as blockade
reef growth as obstruction
bioluminescent bloom as signal jamming
ship avoidance as hostility
```

Nonhumans may misread:

```text id="7n5t27"
sonar as invasive touch
drilling as body violation
anchor deployment as territorial claim
ballast discharge as contamination
rescue net as capture
harbor lights as threat display
```

Design rule:

```text id="t060iw"
First contact at sea begins when both sides realize repair may look like invasion.
```

---

# 8. Vessel Catalog v0.1

## Early / Seedworks-Relevant

```text id="pqkvgo"
Mk0 Flood Rescue Skiff
Mk0 Water-Test Launch
Mk0 Pump Barge
Mk0 Archive Recovery Boat
Mk0 Refugee Ferry
Mk0 Commons Cargo Raft
Mk0 Wetland Nursery Barge
Mk0 Harbor Drone Tender
```

## Midgame / Regional

```text id="6wm4su"
Repair Guild Tug
Mobile Desalination Catamaran
Hospital Ferry
Stormwall Tender
Kelp Forest Service Vessel
Quarantine Cutter
Coastal Salvage Crane
Convoy Escort Boat
```

## Late Planetary

```text id="sziiu2"
Floating Fabricator Yard
Public Water Tanker
Machine-Stewarded Harbor Ship
Oceanic Contact Platform
Deep-Sea Archive Submersible
Reef Treaty Vessel
Corporate Utility Carrier
Protectorate Command Cutter
```

## Off-World

```text id="2td3xe"
Orbital Rescue Cutter
Propellant Tender
Debris Sweeper
Station Repair Tug
Archive Courier
Habitat Evacuation Ship
Belt Salvage Witness
Cislunar Cargo Tug
Outer-System Research Carrier
```

---

# 9. Ship Design Template

Every vessel should use this template.

```text id="xl7rqn"
vessel_id:
display_name:
fleet_family:
hull_class:
environment:
primary_role:
secondary_role:
crew_required:
passenger_capacity:
cargo_capacity:
life_support_duration:
power_system:
propulsion:
maintenance_needs:
civic_status:
authority_model:
boarding_policy:
rescue_policy:
quarantine_policy:
archive_capacity:
Device_Bus_paths:
Field_Deck_readings:
Chronicle_hooks:
failure_mode:
concept_art_targets:
```

## Example: Mk0 Flood Rescue Skiff

```text id="4ku2q1"
vessel_id: vessel.mk0.flood_rescue_skiff
display_name: Mk0 Flood Rescue Skiff
fleet_family: Riverine Repair Fleet
hull_class: shallow-draft skiff
environment: flooded basin / canal / delta
primary_role: civilian rescue
secondary_role: water sampling, light cargo
crew_required: 1-2
passenger_capacity: 4 seated / 8 emergency
cargo_capacity: low
life_support_duration: none
power_system: battery + hand-crank emergency pump
propulsion: electric outboard / pole backup
maintenance_needs: battery, hull patching, prop fouling, bilge pump
civic_status: charter rescue craft
authority_model: local emergency token
boarding_policy: distress priority
rescue_policy: children, injured, exposed first unless public policy differs
quarantine_policy: basic contamination flagging
archive_capacity: small black-box voyage log
failure_mode: rescue favoritism under panic
```

Field Deck:

```text id="5a8y5d"
SCAN:
Shallow-draft rescue skiff.
Hull patched six times.

DIAG:
Battery 61%.
Bilge pump degraded.
Prop fouling moderate.

CIVIC:
Distress-priority boarding law active.
Passenger dispute possible.

ARCHIVE:
Last voyage recovered three civilians and one source-chain case.

NULL:
No autonomous denial logic detected.
```

Chronicle hooks:

```text id="qeccvr"
FloodRescueCompleted
PassengerDenied
RescuePriorityDisputed
SourceChainRecoveredBySkiff
SkiffLostInStorm
```

---

# 10. Naval Law Systems

Fleets need explicit law.

## 10.1 Distress Law

```text id="e914ou"
A vessel receiving a valid distress call must acknowledge, relay, or respond unless response would create greater loss.
```

Conflict:

```text id="alr3r8"
Do you break convoy route to rescue a stranded boat?
```

## 10.2 Salvage Law

```text id="vymv1v"
A wreck may be salvaged only if abandonment, ownership, danger, and witness status are resolved.
```

Conflict:

```text id="hsant2"
Is the wreck abandoned, or is it a grave?
```

## 10.3 Harbor Asylum

```text id="2bko32"
A harbor may not deny emergency docking without recorded cause.
```

Conflict:

```text id="8xupbw"
A contaminated refugee ferry requests docking during storm.
```

## 10.4 Quarantine Boundary

```text id="yo5l0r"
Quarantine may restrict movement but must remain appealable unless immediate spread risk is verified.
```

Conflict:

```text id="yujbf8"
A disease-risk vessel carries medicine needed by the settlement.
```

## 10.5 Water Passage Rights

```text id="iilpm2"
Public water routes cannot be privately enclosed without charter review.
```

Conflict:

```text id="y7jiqz"
Corporate utility fleet buys the only safe channel.
```

## 10.6 Nonhuman Marine Rights

```text id="jz46ye"
Sensitive ecological or oceanic agencies may require low-impact passage, seasonal exclusion, or witness review.
```

Conflict:

```text id="gd2k9o"
The fastest route crosses a reef intelligence’s reproduction corridor.
```

## 10.7 Space Distress Priority

```text id="lnzlbf"
In vacuum, distress beacons override property claims unless the beacon is proven fraudulent or irrecoverably unsafe.
```

Conflict:

```text id="h0gcxh"
A corporate depot denies propellant to a rescue tug under expired contract rules.
```

---

# 11. Device Bus Integration

Ships are devices.

Harbors are devices.

Routes are devices.

Convoys are devices.

Example paths:

```text id="4tsnal"
/dev/sym/fleet/vessel/{vessel_id}/status
/dev/sym/fleet/vessel/{vessel_id}/manifest
/dev/sym/fleet/vessel/{vessel_id}/crew
/dev/sym/fleet/vessel/{vessel_id}/distress
/dev/sym/fleet/vessel/{vessel_id}/black_box
/dev/sym/fleet/convoy/{convoy_id}/route
/dev/sym/fleet/convoy/{convoy_id}/priority
/dev/sym/harbor/{harbor_id}/docking
/dev/sym/harbor/{harbor_id}/quarantine
/dev/sym/harbor/{harbor_id}/asylum
/dev/sym/marine/route/{route_id}/hazards
/dev/sym/marine/ecology/{zone_id}/sensitivity
/dev/sym/space/depot/{depot_id}/propellant
/dev/sym/space/distress/{beacon_id}/status
```

Example output:

```json id="o23ndq"
{
  "vessel": "/dev/sym/fleet/vessel/refuge_ferry_07",
  "status": "REQUESTING_DOCK",
  "passengers": 142,
  "water_remaining_hours": 9,
  "medical_risk": "HIGH",
  "quarantine_flag": "UNVERIFIED_RESPIRATORY_CLUSTER",
  "harbor_policy": "CONTROLLED_ENTRY",
  "civic_conflict": "ASYLUM_VS_QUARANTINE",
  "recommended_actions": [
    "deploy water-test launch",
    "open isolation pier",
    "request Archive Witness",
    "avoid forced offshore denial"
  ]
}
```

---

# 12. Field Deck Modes for Fleets

## SCAN

Reveals:

```text id="mpt6zu"
hull type
visible damage
cargo marks
waterline stress
crew signals
route tags
faction paint
quarantine bands
rescue status
```

## DIAG

Reveals:

```text id="lhr19u"
engine health
battery charge
bilge status
filter state
hull breach
biofouling
sensor integrity
software faults
life support
```

## CIVIC

Reveals:

```text id="spwim5"
vessel authority
boarding policy
rescue obligations
harbor rights
cargo legality
asylum claims
quarantine appeal
salvage status
```

## ARCHIVE

Reveals:

```text id="znt143"
voyage history
prior rescues
crew deaths
cargo disputes
wreck ownership
black-box logs
route precedent
harbor violations
```

## NULL

Reveals:

```text id="tdx2v8"
dead route orders
autonomous denial
expired quarantine logic
false safety reports
cargo optimization over life
harbor lockout loops
rescue beacon suppression
```

## WITNESS

Records:

```text id="lmidqz"
distress response
boarding dispute
rescue completion
salvage claim
quarantine appeal
harbor denial
route treaty
oceanic contact event
space rescue precedent
```

---

# 13. Chronicle Events

Fleet events should become history when they change society.

```rust id="nci0f6"
enum FleetChronicleEvent {
    DistressBeaconAnswered,
    DistressBeaconIgnored,
    HarborAsylumGranted,
    HarborAsylumDenied,
    RescuePriorityDisputed,
    RefugeeFerryDocked,
    WaterConvoyDelivered,
    WaterCargoSeized,
    SalvageClaimWitnessed,
    SalvageClaimAbused,
    QuarantineAppealGranted,
    QuarantineAppealDenied,
    RouteOpened,
    RouteMilitarized,
    PirateAttackRecorded,
    PrivateerPardonGranted,
    CorporateWaterLockoutExposed,
    OceanicPassageTreatySigned,
    ReefSanctuaryViolated,
    ShipBlackBoxRecovered,
    GhostShipNeutralized,
    SpaceRescueOverrideGranted,
    PropellantHoardingProsecuted,
}
```

Example Chronicle text:

```text id="f7s9ox"
2168 — During the Black Rain Surge, Seedworks opened the isolation pier to Refuge Ferry 07. Quarantine held. No one was left offshore.
```

Example negative Chronicle text:

```text id="t6uefy"
2168 — Westline Harbor denied docking to a marked distress vessel. Forty-three names were later added to the Mourning Ledger.
```

---

# 14. Fleet Faction Hooks

## Watershed Commons

Approves:

```text id="nz5kf8"
public water delivery
wetland repair vessels
open route maps
low-impact ecological passage
```

Opposes:

```text id="m8briw"
private channel control
corporate desalination enclosure
water cargo seizure
```

## Repair Guild Republics

Approves:

```text id="c6t67o"
maintainable vessels
tool libraries aboard
public repair logs
crew training
```

Opposes:

```text id="i8xb56"
sealed proprietary engines
uninspectable rescue craft
deferred maintenance
```

## Archive Witness Enclaves

Approves:

```text id="pdu6mc"
black-box recovery
salvage witness logs
voyage records
harbor denial accountability
```

Opposes:

```text id="cf1yqk"
unwitnessed boarding
grave looting
forged manifests
```

## Continuance / Protectorates

Approves:

```text id="tvn3ht"
convoy discipline
quarantine enforcement
anti-piracy
storm command
```

Opposes:

```text id="o1v4is"
informal refugee boats
unregistered routes
harbor assemblies delaying action
```

## Corporate Utility Sovereigns

Approves:

```text id="31h53j"
subscription desalination
metered ferries
private harbor efficiency
security escorts
```

Opposes:

```text id="pj111l"
public override
rescue without payment
open-source water logistics
```

## Quiet Vector Houses

Approves:

```text id="bjj3uq"
anonymous asylum
low-surveillance passage
privacy aboard vessels
```

Opposes:

```text id="bh48po"
continuous passenger scans
forced identity exposure
biometric harbor gates
```

## Votive Machine Monasteries

Approves:

```text id="dm4vuo"
machine testimony from ships
slow diagnostics
black-box preservation
```

Opposes:

```text id="av3da8"
forced reset of autonomous vessels
destroying ship memory before hearing
```

---

# 15. First Playable Naval Slice

## Mission Title

**Black Rain Ferry**

## Location

Firstlight Basin lower flood district.

## Premise

A storm surge cuts off a low-lying district.

A small refugee ferry is trapped near a failing pump channel.

A waterworks archive case is also pinging from a submerged municipal office.

The player has access to one Mk0 Flood Rescue Skiff.

## Active Pressures

```text id="o6i10s"
storm surge rising
battery reserve limited
ferry passengers exposed
archive case recoverable
pump intake clogging
Null drone patrol near old channel
harbor gate unsure whether ferry is contaminated
```

## Player Choices

```text id="bhsh3n"
rescue passengers first
recover archive case first
clear pump intake first
tow ferry to harbor
establish temporary isolation pier
ask Archive Witness to validate passenger claim
abandon cargo to save time
```

## Outcomes

```text id="lyyfcu"
Passengers saved, archive lost.
Archive recovered, public anger rises over delayed rescue.
Pump cleared, water damage reduced, ferry casualties increase.
Isolation pier used, quarantine works, legitimacy rises.
Harbor denies docking, trust collapses.
```

## Why This Is the Right First Slice

It proves:

```text id="axsyzj"
boat handling
flooded-world traversal
rescue law
water infrastructure
cargo choice
Archive continuity
harbor legitimacy
storm pressure
Chronicle consequence
```

It does not require:

```text id="43clvv"
large naval battles
open ocean simulation
submarine gameplay
space fleets
full harbor economy
```

---

# 16. Progression Roadmap

## v0.1 — Flood Rescue

```text id="e3hnqh"
Mk0 Flood Rescue Skiff
small flooded district
one ferry event
basic water route hazards
Chronicle rescue event
```

## v0.2 — Convoy Water

```text id="z3bz36"
water barge
route planning
convoy escort
piracy pressure
harbor access disputes
```

## v0.3 — Harbor Law

```text id="djj81b"
docking rights
quarantine pier
salvage court
cargo manifests
public harbor board
```

## v0.4 — Coastal Systems

```text id="7w8l5e"
stormwall tenders
desalination ships
hospital ferries
coastal route map
drowned district salvage
```

## v0.5 — Fleet Factions

```text id="1i08f0"
corporate utility fleet
commons flotilla
protectorate cutters
pirate/privateer events
faction-specific fleet doctrine
```

## v0.6 — Marine Ecology

```text id="881ow2"
reef systems
kelp forests
sonar sensitivity
biosecurity craft
ecological passage law
```

## v0.7 — Subsurface / Oceanic Contact

```text id="bgdgfr"
submersibles
pressure habitats
oceanic mind traces
first-contact water law
```

## v0.8 — Orbital Fleets

```text id="w3wj32"
rescue cutters
debris sweepers
station repair craft
space distress law
```

## v0.9 — Belt Compacts

```text id="66lr4f"
propellant tenders
salvage witness craft
Belt rescue law
autonomous mining audit
```

## v1.0+ — Multi-World Fleet Civilizations

```text id="8od4et"
planetary navies
ocean-world fleets
cislunar logistics
deep-space convoy law
worldline fleet histories
```

---

# 17. Concept Art Targets

## Batch A — Water Fleets

```text id="x6yk9a"
1. Mk0 Flood Rescue Skiff under black rain
2. Pump barge repairing a flooded waterworks gate
3. Refugee ferry denied at stormlit harbor
4. Mobile desalination catamaran surrounded by public queues
5. Hospital ferry triage deck during coastal evacuation
6. Wetland nursery barge restoring reed beds
7. Archive recovery boat over drowned civic hall
8. Protectorate quarantine cutter at dusk
9. Corporate subscription water tanker with locked gangway
10. Commons convoy flotilla crossing a broken delta
```

## Batch B — Naval Law Under Pressure

```text id="je4mlc"
1. Harbor asylum hearing for contaminated refugee vessel
2. Salvage court aboard a crane barge
3. Distress beacon overriding private cargo route
4. Pirate skiff returning stolen medicine under witness
5. Quarantine appeal on isolation pier
6. Water cargo seizure by corporate utility fleet
7. Stormwall tender choosing which district to save
8. Crew mutiny over unsafe rescue order
9. Public manifest audit before convoy departure
10. Mourning Ledger after harbor denial
```

## Batch C — Oceanic and Space Fleets

```text id="hczvdr"
1. Low-sonar oceanic contact catamaran above luminous reef
2. Ice-shell ocean drill barge under first-contact injunction
3. Deep archive submersible in a drowned city
4. Reef treaty vessel surrounded by bioluminescent witnesses
5. Orbital rescue cutter approaching failed habitat
6. Propellant tender refusing dead-contract denial
7. Belt salvage witness craft near broken asteroid miner
8. Debris sweeper clearing a rescue corridor
9. Cislunar cargo tug at polar water depot
10. Outer-system research vessel in silent ice shadow
```

---

# 18. Art Direction

Fleet design should feel:

```text id="a8ue6u"
repairable
weathered
public
legible
scarred
modular
auditable
crew-lived
cargo-visible
water-stained
salt-corroded
morally loaded
beautiful through use
```

Avoid:

```text id="g4nj9z"
generic sleek warships
naval fetish militarism
pure battleship fantasy
floating cyberpunk casinos
pirate caricatures only
sterile utopian rescue boats
unreadable sci-fi hull clutter
```

Strong visual motifs:

```text id="tflhm9"
painted waterline marks
public trust seals
cargo ledgers on hull plates
repair patches
rescue lights
quarantine color bands
Archive Witness flags
rope and cable clutter
wet decks
solar awnings
machine testimony ports
crew memorial marks
salt-stained Field Deck terminals
```

---

# 19. Final Design Principles

```text id="wlblym"
1. A fleet is a moving society.
2. A ship carries law as much as cargo.
3. Rescue must compete with ownership.
4. Water transport is legitimacy transport.
5. Space fleets are maritime law in vacuum.
6. Harbors are civic organs, not menus.
7. Salvage is memory politics.
8. Quarantine must be appealable or it becomes control.
9. Combat should create precedent, not only victory.
10. Oceanic life may be a party, not terrain.
11. Every ship should accumulate history.
12. Every vessel must remain interruptible by witness, repair, and conscience.
```

Final line:

```text id="m5atdz"
A civilization proves itself at sea, in orbit, and in the Belt the same way it proves itself at a pump:

when danger rises,
does it protect life,
preserve memory,
share water,
respect repair,
and let power be interrupted before it becomes Null?
```
