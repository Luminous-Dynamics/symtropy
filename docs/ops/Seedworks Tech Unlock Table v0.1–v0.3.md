---

title: Seedworks Tech Unlock Table v0.1–v0.3
status: canonical-draft
milestone: seedworks-v0.1-to-v0.3
scope: progression, unlocks, production roadmap
owner: design/engineering
recommended_path: docs/seedworks/00_canon/TECH_UNLOCK_TABLE_V0_1_TO_V0_3.md
depends_on:

* SEEDWORKS_PLAYABLE_SLICE_SPEC.md
* SEEDWORKS_NEXT_BUILD_PLAN.md
* SEEDWORKS_FACTION_VERTICAL_SLICE.md
* SEEDWORKS_ARCHITECTURE.md

---

# Symtropy: Seedworks Tech Unlock Table v0.1–v0.3

## Working Title

**The Future Must Be Built Before It Can Be Unlocked**

## Purpose

This document defines which technologies, systems, tools, facilities, interfaces, and civic capabilities are:

```text
playable
visible
foreshadowed
locked
deferred
```

across Seedworks v0.1, v0.2, and v0.3.

The goal is to prevent scope creep while preserving the long-term technological arc toward version 1.0.

Core rule:

```text
A technology is not unlocked because the player spent points.
It is unlocked because the world now has the material, computational, and civic conditions to support it.
```

---

# 1. Version Philosophy

## v0.1 — The First Pipe

Purpose:

```text
Prove that one infrastructure repair can carry the whole game thesis.
```

The player should learn:

```text
repair is physical
cargo has mass
the Field Deck reveals layered truth
machines can obey dead authority
technical repair and legitimate repair differ
the Chronicle records consequence
```

v0.1 is not about breadth.

It is about proof.

Design rule:

```text
Build one pipe that knows its name.
```

---

## v0.2 — The First Workshop

Purpose:

```text
Turn the first repair into a repeatable settlement capability.
```

The player should learn:

```text
repair creates local technology access
Proof-of-Repair opens doors
public fabrication changes settlement capacity
cargo ledgers matter
the first faction pressures emerge
```

v0.2 expands from one repair to a small repair economy.

Design rule:

```text
The settlement begins to remember how to build.
```

---

## v0.3 — The First Embassy

Purpose:

```text
Introduce non-human technology exchange and translation risk.
```

The player should learn:

```text
alien trade is metabolic
alien technology cannot be simply downloaded
hybrid hardware requires translation and consent
living infrastructure can fail politically and biologically
```

v0.3 is the first step beyond human repair into multi-species infrastructure.

Design rule:

```text
Humanity does not acquire alien technology.
It negotiates compatibility.
```

---

# 2. Status Labels

Use these status labels consistently.

```text
PLAYABLE
The player can use this system directly.

VISIBLE_LOCKED
The object, facility, or interface exists but cannot yet be used.

FORESHADOWED
Mentioned through text, UI, distant structure, dead terminal, or locked option.

STUB
Technically present but simplified or hardcoded.

ROADMAP
Designed for later versions but not present in build.

DEFERRED
Explicitly out of scope.

REMOVED_FROM_SCOPE
Rejected for this milestone to protect focus.
```

---

# 3. Master Unlock Table

| Technology / System                    | Discipline                       |           v0.1 |           v0.2 |                       v0.3 | Notes                                                |
| -------------------------------------- | -------------------------------- | -------------: | -------------: | -------------------------: | ---------------------------------------------------- |
| Field Deck Mk0                         | Computational Field Architecture |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Core player interface                                |
| SCAN Mode                              | Computational Field Architecture |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Physical state reading                               |
| DIAG Mode                              | Computational Field Architecture |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Machine state reading                                |
| ARCHIVE Mode                           | Socio-Civic Legitimacy           |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Historical state reading                             |
| CIVIC Mode                             | Socio-Civic Legitimacy           |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Authority and legitimacy reading                     |
| NULL Mode                              | Computational / Archive          |           STUB |       PLAYABLE |                   PLAYABLE | v0.1 shows prompt corruption and dead-authority loop |
| REPAIR Mode                            | Material Fabrication             |           STUB |       PLAYABLE |                   PLAYABLE | v0.1 may be contextual only                          |
| WITNESS Mode                           | Legitimacy Chains                |           STUB |       PLAYABLE |                   PLAYABLE | v0.1 witness cartridge only                          |
| Substrate Summary Page                 | Device Bus Substrate             |           STUB |       PLAYABLE |                   PLAYABLE | Power/audio/labor summary                            |
| Patch Cable                            | Computational Field Architecture |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Physical interface tool                              |
| Patch Conduit Mk0                      | Material Fabrication             |       PLAYABLE |       PLAYABLE |                   PLAYABLE | First repair object                                  |
| Copper Conduit Pipe Segment            | Material Fabrication             |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Two-handed cargo                                     |
| Archive Witness Cartridge              | Legitimacy Chains                |       PLAYABLE |       PLAYABLE |                   PLAYABLE | First portable witness                               |
| Flooded Storage Crate Manifest         | Cargo / Ledger                   |       PLAYABLE |       PLAYABLE |                   PLAYABLE | First container manifest                             |
| Physical Cargo Carry                   | Material / Logistics             |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Heavy cargo vulnerability                            |
| Panic Drop                             | Physical Interaction             |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Cargo and Field Deck interruption                    |
| Device Bus Node Registration           | Computational Field Architecture |       PLAYABLE |       PLAYABLE |                   PLAYABLE | `/dev/sym/water/patch_conduit_alpha`                 |
| Local Device Bus Shell                 | Computational Field Architecture |           STUB |       PLAYABLE |                   PLAYABLE | v0.1 only `read`, `initialize`, `authorize`          |
| Chronicle JSONL v0                     | Chronicle / Legitimacy           |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Append-only local event log                          |
| Proof-of-Repair Receipt                | Labor / Legitimacy               |       PLAYABLE |       PLAYABLE |                   PLAYABLE | v0.1 generates one receipt                           |
| Proof-of-Repair Redemption             | Labor Economy                    |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Fuel/tool access after v0.1                          |
| Thermodynamic Power Readout            | Power Substrate                  |           STUB |       PLAYABLE |                   PLAYABLE | One transformer in v0.1                              |
| Voltage Sag Effect                     | Power Substrate                  |           STUB |       PLAYABLE |                   PLAYABLE | v0.1 warning only                                    |
| WASM Clock Drift                       | Computational Field Architecture |   FORESHADOWED |           STUB |                   PLAYABLE | Deterministic multi-tick scripts later               |
| Pump Audio Diagnostic                  | Audio Substrate                  |           STUB |       PLAYABLE |                   PLAYABLE | One sick pump in v0.1                                |
| Origin-Specific Scan Notes             | Field Deck / Persona             |       PLAYABLE |       PLAYABLE |                   PLAYABLE | Three mocked origins in v0.1                         |
| Continuance Ghost Origin               | Faction / Persona                |   FORESHADOWED | VISIBLE_LOCKED |         PLAYABLE_OR_LOCKED | Depends on faction scope                             |
| Firstlight Public Repair Charter       | Legitimacy Chains                |       PLAYABLE |       PLAYABLE |                   PLAYABLE | v0.1 hardcoded                                       |
| Registered Infrastructure Adjudication | Legitimacy Chains                |           STUB |       PLAYABLE |                   PLAYABLE | v0.1 choice only                                     |
| Rights Floor Warning                   | Legitimacy Chains                |   FORESHADOWED |           STUB |                   PLAYABLE | v0.1 text hint                                       |
| Public Works Fabrication Bench         | Material Fabrication             | VISIBLE_LOCKED |       PLAYABLE |                   PLAYABLE | First major v0.2 unlock                              |
| Certified Pipe Gauge                   | Material Fabrication             |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Improves repair quality                              |
| Pressure Test Rig                      | Material Fabrication             |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Enables certified repairs                            |
| Public Tool Library                    | Legitimacy / Fabrication         |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Unlocked through Proof-of-Repair                     |
| Settlement Fabricator Bay              | Material Fabrication             | VISIBLE_LOCKED |       PLAYABLE |                   PLAYABLE | v0.2 core facility                                   |
| Cargo Ledger Audit                     | Cargo / Legitimacy               |           STUB |       PLAYABLE |                   PLAYABLE | v0.2 expands manifest disputes                       |
| Cold-Chain Vault                       | Material / Ecology               |   FORESHADOWED |           STUB |                   PLAYABLE | Needed before biological cargo matters               |
| Biofilter Housing                      | Material / Ecology               |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Human baseline before alien filters                  |
| Rover Med-Bay                          | Material / Death Recovery        |   FORESHADOWED |        ROADMAP |        PLAYABLE_OR_ROADMAP | Likely v0.3+ depending scope                         |
| Local Reconstitution Cot               | Death Recovery                   |  OPTIONAL_STUB |       PLAYABLE |                   PLAYABLE | v0.1 optional death test                             |
| Resonatia Bastion Fallback             | Death / Chronicle                |   FORESHADOWED |   FORESHADOWED |                    ROADMAP | Not v0.1–v0.3 unless needed                          |
| Settlement Public Vote                 | Governance                       |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | v0.2 or later                                        |
| Faction Archetype Vector               | Factions                         |   FORESHADOWED |           STUB |                   PLAYABLE | v0.2 tracks first drift                              |
| Mutualist Assembly Pressure            | Factions                         |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Public repair route                                  |
| Industrial Compact Pressure            | Factions                         |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Fabrication/output route                             |
| Security Protectorate Pressure         | Factions                         |   FORESHADOWED |           STUB |                   PLAYABLE | Perimeter defense route                              |
| Archive Witness Council Pressure       | Factions                         |   FORESHADOWED |       PLAYABLE |                   PLAYABLE | Archive recovery route                               |
| Ghost Civic Center                     | Lore Site                        |   FORESHADOWED | VISIBLE_LOCKED |         PLAYABLE_OR_LOCKED | v0.3 if faction slice expands                        |
| Shared Tool Embassy                    | Xeno-Translation                 |   FORESHADOWED | VISIBLE_LOCKED |                   PLAYABLE | v0.3 core alien-tech facility                        |
| Translation Pool                       | Xeno-Translation                 |   FORESHADOWED | VISIBLE_LOCKED |                   PLAYABLE | Required for alien tech                              |
| Multi-Species Rights Forum Terminal    | Legitimacy / Xeno                |   FORESHADOWED | VISIBLE_LOCKED |                   PLAYABLE | Certifies hybrid tech                                |
| Tideborn Water-Civic Exchange          | Alien Trade                      |   FORESHADOWED |        ROADMAP |                   PLAYABLE | First alien trade candidate                          |
| Aerosol Choir Exchange                 | Alien Trade                      |   FORESHADOWED |        ROADMAP | VISIBLE_LOCKED_OR_PLAYABLE | Better after air systems exist                       |
| Lithic Deep-Time Chamber Exchange      | Alien Trade                      |   FORESHADOWED |        ROADMAP | VISIBLE_LOCKED_OR_PLAYABLE | Requires resonance systems                           |
| Hybrid Filter Alpha                    | Xeno-Hybrid Hardware             |   FORESHADOWED | VISIBLE_LOCKED |                   PLAYABLE | First hybrid component                               |
| Canopy Root Wrapper Script             | Xeno-Translation                 |   FORESHADOWED |        ROADMAP |           PLAYABLE_OR_STUB | v0.3 if living infrastructure introduced             |
| Translation Collapse                   | Xeno Failure                     |   FORESHADOWED |        ROADMAP |                   PLAYABLE | First xeno failure mode                              |
| Overgrowth Without Consent             | Xeno Failure / Rights            |   FORESHADOWED |        ROADMAP |           PLAYABLE_OR_STUB | Must be serious, not decorative                      |
| Full Conveyor Logistics                | Logistics                        |       DEFERRED |        ROADMAP |                    ROADMAP | Not before cargo basics work                         |
| Full Power Grid Simulation             | Power                            |       DEFERRED |        ROADMAP |                    ROADMAP | Use stubs first                                      |
| Full Acoustic Propagation              | Audio                            |       DEFERRED |       DEFERRED |                    ROADMAP | Pump audio first                                     |
| Full Alien Economy                     | Alien Trade                      |       DEFERRED |       DEFERRED |                    ROADMAP | Start with one exchange                              |
| Full Multiplayer Persistence           | Networking                       |       DEFERRED |        ROADMAP |                    ROADMAP | Not needed for first proof                           |
| Atlas Gates                            | Interstellar Transit             |   FORESHADOWED |   FORESHADOWED |                    ROADMAP | v1.0+ pillar                                         |
| Planetary / Worldline Translation      | Worldline Systems                |   FORESHADOWED |        ROADMAP |                    ROADMAP | Not v0.1–v0.3                                        |

---

# 4. v0.1 Required Unlock Set

v0.1 should only require the following.

## Player Tools

```text
Field Deck Mk0
Patch Cable
Basic Repair Tool
Archive Witness Cartridge
```

## Field Deck Modes

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL stub
REPAIR stub
WITNESS stub
```

## Physical Objects

```text
Copper Conduit Pipe Segment
Patch Conduit Mk0
Ceramic Seal
Flooded Storage Crate
Terminal Witness Bay
```

## Device Bus Paths

```text
/dev/sym/water/pump_1
/dev/sym/water/patch_conduit_alpha
/dev/sym/logistics/flooded_crate_0
/dev/sym/power/transformer_2
/dev/sym/audio/pump_1
/dev/sym/labor/proof_of_repair/por_old_waterworks_v0_0001
```

## Required Player Verbs

```text
walk
scan
read
carry
drop
insert
align
seal
initialize
authorize
commit
```

## Required End-State

```text
water restored or delayed
Chronicle event written
Proof-of-Repair receipt issued
repair consequence visible
```

v0.1 success line:

```text
The player repaired one pipe and learned that history moved with the water.
```

---

# 5. v0.1 Visible-Locked / Foreshadowed Set

These may appear, but must not be required.

```text
Public Works Fabrication Bench
Shared Tool Embassy link
Resonatia Bastion fallback reference
Ghost Civic Center reference
Continuance Ghost Origin
Rights Floor warning
Public vote notice board
Settlement fabricator bay door
fuel depot trust clause
```

Example locked text:

```text
PUBLIC WORKS FABRICATION BENCH:
Unavailable.

Requirement:
Proof-of-Repair accepted by Firstlight Public Repair Charter.
```

Example embassy text:

```text
SHARED TOOL EMBASSY LINK:
Unavailable in Firstlight Basin.

Requirement:
Multi-Species Rights Forum access.
```

Design rule:

```text
Foreshadow the cathedral.
Do not ask the player to build it yet.
```

---

# 6. v0.2 Unlock Set: The First Workshop

v0.2 begins after the player has completed the Old Waterworks loop.

Primary unlock condition:

```text
Old Waterworks restored
Chronicle event accepted
Proof-of-Repair issued
Firstlight Public Repair Charter recognizes repair
```

## New Playable Facilities

```text
Public Works Fabrication Bench
Settlement Fabricator Bay
Public Tool Library
Fuel Depot Trust Console
Cargo Ledger Audit Station
```

## New Playable Technologies

```text
Certified Pipe Gauge
Certified Seal Kit
Pressure Test Rig
Standardized Valve Housing
Biofilter Housing
Basic Transformer Repair Kit
Tool Library Access Token
```

## New Field Deck Capabilities

```text
expanded REPAIR mode
expanded WITNESS mode
Proof-of-Repair viewer
cargo ledger dispute flagging
basic power graph readout
origin-specific audio notes
```

## New Civic Capabilities

```text
temporary public repair permit
infrastructure hearing stub
commons asset conversion preview
first settlement vote
repair worker access class
```

## New Gameplay Loop

```text
recover part
fabricate better part
certify repair
issue stronger Proof-of-Repair
use receipt for settlement access
choose settlement priority
```

v0.2 success line:

```text
The settlement did not only survive the repair.
It learned how to repeat it.
```

---

# 7. v0.3 Unlock Set: The First Embassy

v0.3 begins when human repair has enough infrastructure to safely attempt alien translation.

Primary unlock conditions:

```text
Public Works Fabrication Bench active
Settlement Fabricator Bay active
Proof-of-Repair recognized outside original site
WITNESS mode expanded
basic Rights Floor review active
first alien contact route discovered
```

## New Playable Facilities

```text
Shared Tool Embassy
Translation Pool
Metabolic Stabilizer
Rights Forum Terminal
Quarantine Chamber
Alien Contact Substrate
```

## New Playable Technologies

```text
Hybrid Filter Alpha
Bio-Electric Converter
Tideborn Chemical Memory Block
Aerosol Archive Cartridge
Lithic Resonance Coupler
Canopy Root Wrapper Script
```

## New Player Verbs

```text
handshake
calibrate
translate
stabilize
license
quarantine
negotiate
compile hybrid block
```

## New Failure States

```text
Translation Collapse
Overgrowth Without Consent
Sterilized Living Infrastructure
Metabolic Starvation
Command Refusal
Null Drift Amplification
```

## New Civic Capabilities

```text
Multi-Species Rights Forum review
Living Infrastructure Consent Token
Hybrid Technology License
Alien Witness Format
Metabolic Exchange Treaty
```

v0.3 success line:

```text
The player learns that alien technology is not a blueprint.
It is a relationship under load.
```

---

# 8. Version 1.0 Horizon

v1.0 should not be defined as “more content.”

v1.0 should mean the core loop can scale.

## v1.0 Target Pillars

```text
1. Multiple settlements can evolve differently.
2. Infrastructure repairs produce durable Chronicle precedent.
3. Proof-of-Repair travels across settlements.
4. Field Deck modes remain the primary interface to reality.
5. Faction pressure changes law, logistics, and technology access.
6. Human tech progression is physical, computational, and civic.
7. Alien trade is metabolic, not financial.
8. Hybrid technologies require translation and consent.
9. Death/reconstitution respects source-chain continuity.
10. The player can see the world remember what they changed.
```

## v1.0 Minimum Campaign Arc

```text
Act 1:
Restore Firstlight Basin water.

Act 2:
Build public repair capacity.

Act 3:
Survive faction pressure and settlement divergence.

Act 4:
Open first Shared Tool Embassy.

Act 5:
Translate first alien hybrid technology.

Act 6:
Resolve a crisis caused by hybrid infrastructure failure.

Act 7:
Export Proof-of-Repair legitimacy to another settlement.

Act 8:
Make a regional decision that becomes Chronicle precedent.
```

## v1.0 Success Line

```text
The player begins as someone fixing a pipe.
They become someone whose repairs define what civilization is allowed to become.
```

---

# 9. Scope Protection Rules

## Rule 1: v0.1 Must Not Become v0.2

Do not add full fabrication before the first repair works.

```text
No Public Works Fabrication Bench until Old Waterworks loop is stable.
```

## Rule 2: v0.2 Must Not Become v0.3

Do not add alien translation before human fabrication and witness systems work.

```text
No Shared Tool Embassy until Proof-of-Repair and public fabrication are meaningful.
```

## Rule 3: v0.3 Must Not Become v1.0

Do not add many alien species before one alien exchange works.

```text
One alien trade route.
One hybrid component.
One translation failure.
```

## Rule 4: Every Unlock Needs a Physical or Civic Verb

Reject unlocks that only add abstract percentage bonuses.

Bad:

```text
+5% repair speed
```

Good:

```text
Pressure Test Rig unlocked.
Repairs can now be certified instead of temporary.
```

## Rule 5: Every Version Must End With a Chronicle Memory

Each milestone should produce a Chronicle-worthy outcome.

```text
v0.1:
The first pipe was repaired.

v0.2:
The settlement learned to fabricate public repair parts.

v0.3:
The first alien hybrid technology entered human infrastructure.
```

---

# 10. Final Principles

```text
v0.1 proves repair.

v0.2 proves repeatable repair.

v0.3 proves translated repair.

v1.0 proves civilizational repair.

Technology is not a list of upgrades.
It is the material memory of what a society can safely do.

A tool unlocks only when the world can manufacture it,
power it,
authorize it,
witness it,
repair it,
and survive its failure.
```

Final line:

```text
The tech tree was not above the world.
It was the world learning what it could responsibly become.
```
---

---

title: Robotics Platform Tech Tree Addendum
status: canonical-draft-v0.2
milestone: seedworks-v0.1-to-v1.0
scope: robotics, autonomy, tech progression, civic authorization
owner: design/engineering
depends_on:

* TECH_UNLOCK_TABLE_V0_1_TO_V0_3.md
* Robotics_Platform_ROADMAP.md
* DEVICE_BUS_SUBSTRATE_SYSTEMS.md
* SEEDWORKS_ARCHITECTURE.md
  recommended_path: docs/seedworks/00_canon/ROBOTICS_PLATFORM_TECH_TREE_ADDENDUM.md

---

# Symtropy Tech Tree Addendum: Robotics Platforms

## Working Title

**Robots Are Infrastructure With Bodies**

## Core Thesis

Robotics in *Symtropy* should not be a collectible unit roster.

A robot is not simply a tool, pet, vehicle, weapon, or companion.

A robot is embodied infrastructure.

It carries:

```text
mass
power draw
script budget
sensor limits
maintenance needs
cargo interfaces
legal permissions
witness capacity
labor politics
failure risk
```

Core rule:

```text
A robot is not unlocked when it can move.
A robot is unlocked when the settlement can trust what it will do when no one is watching.
```

A robot enters the tech tree only when the settlement can:

```text
build its body
power its mind
authorize its actions
maintain its failures
audit its logs
survive its mistakes
```

Design rule:

```text
Every robot is a moving argument about what the settlement trusts automation to do.
```

---

# 1. Robotics as Embodied Technology

Human technology progression has three main disciplines:

```text
Thermodynamic Material Fabrication
Computational Field Architecture
Socio-Civic Legitimacy Chains
```

Robotics is not a fourth independent discipline.

Robotics is what happens when all three disciplines become embodied.

```text
Material Fabrication gives the robot a body.
Computational Field Architecture gives the robot perception and control.
Socio-Civic Legitimacy Chains define where, when, and why it may act.
```

A robot platform must therefore satisfy three questions:

```text
Matter:
Can we build and repair the body?

Computation:
Can we supervise, replay, and constrain the autonomy?

Legitimacy:
Is the robot allowed to act here, and does its testimony count?
```

Design rule:

```text
A robot without civic status is just a machine looking for a dispute.
```

---

# 2. Robotic Agency Ladder

Robotics progression should not be framed as “better AI.”

It should be framed as increasing agency under increasing accountability.

## L0 — Passive Tool

No autonomy.

Human-operated device.

Examples:

```text
manual sky-hook
static sensor
unpowered gantry
hand-cranked cargo lift
```

Allowed actions:

```text
none without direct human input
```

Civic risk:

```text
low
```

Design rule:

```text
A passive tool can still create liability if it moves survival cargo.
```

---

## L1 — Remote Actuated

Moves only under direct player input.

Examples:

```text
basic cable-crawler camera
manual rover winch
remote inspection arm
```

Allowed actions:

```text
move while commanded
stream sensor data
stop on signal loss
```

Civic risk:

```text
operator liability
privacy boundary
unsafe remote operation
```

Design rule:

```text
Remote actuation extends the operator. It does not create independent agency.
```

---

## L2 — Supervised Routine

Can execute short bounded tasks after approval.

Examples:

```text
inspect overhead route
lift cargo between two anchors
scan corridor for hazards
listen for relay chatter
```

Allowed actions:

```text
execute approved route
pause on anomaly
log task
request human confirmation
```

Civic risk:

```text
task scope creep
bad route selection
machine witness dispute
```

Design rule:

```text
The first useful robot should be able to help, not decide.
```

---

## L3 — Bounded Public Agent

Can act inside a defined charter zone.

Examples:

```text
public inspection crawler
registered acoustic boundary
animal crossing sentinel
settlement cargo gantry
```

Allowed actions:

```text
operate in authorized zone
trigger safety alerts
record public infrastructure evidence
request civic review
```

Civic risk:

```text
surveillance creep
private boundary violation
false emergency escalation
```

Design rule:

```text
A bounded public agent is infrastructure with a route and a law.
```

---

## L4 — Trusted Civic Agent

Can witness, report, and trigger formal adjudication.

Examples:

```text
service robot whose testimony is accepted
Archive-certified inspection crawler
public repair witness drone
```

Allowed actions:

```text
submit testimony
co-sign repair evidence
trigger hearing
support Proof-of-Repair
```

Civic risk:

```text
machine testimony controversy
forged logs
faction rejection
Machine Stewardship pressure
```

Design rule:

```text
A robot becomes advanced when people argue over its testimony, not just when its motors improve.
```

---

## L5 — Personhood Candidate

Persistent memory, refusal behavior, self-protection, and Rights Floor relevance.

Not for v0.1–v0.3.

Examples:

```text
long-term service robot with self-history
machine steward with refusal protocol
robotic agent with memory continuity claim
```

Allowed actions:

```text
refuse unsafe orders
request review
preserve own memory
enter personhood inquiry
```

Civic risk:

```text
forced obedience
memory wiping
machine exploitation
Rights Floor dispute
```

Design rule:

```text
Do not introduce robot personhood before robot testimony matters.
```

---

# 3. Robot Permission Envelope

Every robot must have a permission envelope.

A permission envelope defines:

```text
where it may go
what it may touch
who may command it
what it must log
when it must stop
whether its testimony counts
who is liable if it fails
```

Example:

```json
{
  "robot": "/dev/sym/robotics/mk0_scout_alpha",
  "platform": "mk0-scout",
  "name": "Cable-Crawler",
  "autonomy_level": "L2_SUPERVISED_ROUTINE",
  "allowed_zones": [
    "public_infrastructure_corridor",
    "overhead_cable_route",
    "old_waterworks_non_private"
  ],
  "forbidden_zones": [
    "private_dwelling",
    "sealed_archive",
    "quarantine_zone_without_token"
  ],
  "allowed_actions": [
    "inspect",
    "record_visual_log",
    "mark_hazard",
    "return_to_dock"
  ],
  "required_logs": [
    "operator_id",
    "route_manifest",
    "visual_hash",
    "time_of_operation"
  ],
  "witness_capacity": "visual_log_supporting_evidence",
  "liability_holder": "operator_or_public_works_charter",
  "stop_conditions": [
    "power_sag_below_safe_threshold",
    "route_boundary_violation",
    "Null_chatter_detected",
    "human_override"
  ]
}
```

Design rule:

```text
A robot’s permission envelope is as important as its battery.
```

---

# 4. Robot Platform Data Model

Every platform exposes a Device Bus identity.

Example path:

```text
/dev/sym/robotics/mk0_scout_alpha
```

Minimum fields:

```json
{
  "node": "/dev/sym/robotics/mk0_scout_alpha",
  "platform_class": "cable_crawler",
  "autonomy_level": "L2_SUPERVISED_ROUTINE",
  "authority_status": "public_works_limited",
  "permission_envelope": "firstlight_public_inspection_v0",
  "power_draw_w": 180,
  "script_budget": "low",
  "sensor_suite": ["camera", "acoustic", "thermal_basic"],
  "mobility_domain": "overhead_wire",
  "maintenance_state": "field_repairable",
  "witness_capacity": "visual_log_supporting_evidence",
  "civic_restrictions": [
    "no_private_residence_entry",
    "public_route_only",
    "no_autonomous_cargo_release"
  ]
}
```

Design rule:

```text
A robot should be visible to the Device Bus before it is visible as a companion.
```

---

# 5. Robotics Unlock Requirements

Each robotics platform requires five unlock classes.

## 1. Body Requirement

```text
chassis
actuators
motors
gears
seals
wheels / legs / tracks / rotors
thermal housing
repairable fasteners
```

## 2. Power Requirement

```text
battery
charging dock
power grid stability
voltage tolerance
thermal management
emergency shutdown
```

## 3. Computation Requirement

```text
controller
script budget
sensor fusion
deterministic autonomy
Field Deck command interface
Device Bus registration
source-chain logging
```

## 4. Legitimacy Requirement

```text
public works license
route permission
labor charter
animal/human safety review
privacy boundary
witness rules
emergency override limits
```

## 5. Maintenance Requirement

```text
spare parts
repair bench
diagnostic procedure
operator training
Proof-of-Repair support
failure-state response
```

Design rule:

```text
The tech tree does not unlock robots.
It unlocks the conditions under which robots stop being irresponsible.
```

---

# 6. Robot Acceptance Test

Before a robot enters a build, it must answer six questions.

```text
1. What physical job does it make possible?

2. What can it not do?

3. What does it need to operate?

4. What does it record?

5. Who is allowed to command it?

6. What Chronicle-worthy failure can it create?
```

Reject any platform that cannot answer all six.

Design rule:

```text
No robot enters production as decoration.
```

---

# 7. Seedworks v0.1 Robotics Scope

## Status

```text
Robotics should be absent but felt.
```

No autonomous robot platform is required in v0.1.

v0.1 should include only:

```text
dead service robot shell
locked mk0-scout rail
inactive gantry anchor
missing crawler dock
Field Deck note about future robotics bench
one robot testimony foreshadow
```

Allowed v0.1 references:

```text
mk0-scout
mk0-gantry
mk0-way
mk0-aegis
```

Example Field Deck text:

```text
SCAN:
Overhead cable rail detected.

DIAG:
mk0-scout dock missing crawler unit.

CIVIC:
Public inspection crawler unavailable.
Manual operator entry required.
```

Design rule:

```text
Let the player feel the absence of robots before giving them one.
```

v0.1 final rule:

```text
Do not add a robot before the first pipe can remember who repaired it.
```

---

# 8. Seedworks v0.2 Robotics Unlocks

v0.2 is the correct place for the first operational robotics.

The goal is not humanoids.

The goal is small, believable infrastructure helpers.

## v0.2 Platform 1: `mk0-scout`

### Name

```text
Cable-Crawler
```

### Autonomy Level

```text
L2 — Supervised Routine
```

### Role

Overhead inspection, mapping, visual witness, low-risk scouting.

### Physical Role

```text
Extends perception into routes too dangerous or slow for a human to inspect directly.
```

### Unlock Requirements

```text
Public Works Fabrication Bench
basic motor repair
battery charger
Field Deck remote view
public route authorization
overhead cable route
```

### Gameplay Verbs

```text
deploy
recall
inspect
mark
witness
route
pause
```

### Device Bus Path

```text
/dev/sym/robotics/mk0_scout_alpha
```

### Use Cases

```text
inspect flooded corridor
map Old Waterworks ceiling route
verify cargo manifest without entering hazard
record visual witness for repair dispute
locate downed Field Deck ping
```

### Limitations

```text
cannot manipulate objects
cannot fight
requires overhead cable
weak in storms
low autonomy
must stop at route boundary
```

### Failure Modes

```text
POWER_SAG_MOTOR_DROP
ROUTE_BOUNDARY_DENIED
NULL_COMMAND_ECHO
WITNESS_REJECTED
```

### Chronicle Example

```text
The crawler saw the seal before anyone risked walking beneath it.
```

Design rule:

```text
The first robot should extend sight, not replace courage.
```

---

## v0.2 Platform 2: `mk0-gantry`

### Name

```text
Sky-Hook
```

### Autonomy Level

```text
L1 — Remote Actuated
L2 — Supervised Routine after certification
```

### Role

Manual cargo assist and overhead hauling.

### Physical Role

```text
Moves heavy objects through dangerous spaces without pretending cargo is weightless.
```

### Unlock Requirements

```text
basic pulley hardware
ceiling anchor certification
cargo ledger integration
safety charter approval
operator training
```

### Gameplay Verbs

```text
attach
lift
slide
lower
lock
drop
halt
```

### Device Bus Path

```text
/dev/sym/robotics/mk0_gantry_01
```

### Use Cases

```text
move heavy conduit safely
lift pump casing
recover corpse or Field Deck from flooded room
move cold-chain cargo without warming
extract cargo without entering electrical water
```

### Limitations

```text
requires anchor points
cannot improvise route
slow
cargo must be tagged
unauthorized movement triggers dispute
```

### Failure Modes

```text
ANCHOR_SLIP
CARGO_DROP
LINE_JAM
MANIFEST_DIVERGENCE
AUTHORITY_DENIED
```

### Chronicle Example

```text
The settlement learned that a pulley with a ledger could save a person without pretending to be one.
```

Design rule:

```text
The first useful robot may be a rope that learned to keep records.
```

---

## v0.2 Platform 3: `mk0-aegis`

### Name

```text
Acoustic Boundary
```

### Autonomy Level

```text
L3 — Bounded Public Agent
```

### Role

Safety alerts, machine sound monitoring, perimeter anomaly detection.

### Physical Role

```text
Listens for dangerous machine behavior before visual failure occurs.
```

### Unlock Requirements

```text
microphone mesh
audio bus node
settlement safety charter
privacy boundary rules
public alert protocol
```

### Gameplay Verbs

```text
listen
classify
warn
silence
audit
calibrate
```

### Device Bus Path

```text
/dev/sym/robotics/mk0_aegis_boundary
```

### Use Cases

```text
detect relay chatter
warn about pressure burst
detect Null drone approach
identify machine acoustic decay
support audio-forensic repair claims
```

### Limitations

```text
cannot distinguish all voices from machinery
privacy constrained
false positives possible
must expose audit log
```

### Civic Risk

```text
surveillance creep
privacy violation
false alarm used for emergency control
Continuance pressure increase
```

### Chronicle Example

```text
The first boundary listened for the pump, and the settlement had to decide whether it was also listening to them.
```

Design rule:

```text
A safety sensor becomes political the moment it listens to people too.
```

---

## v0.2 Platform 4: `mk0-agora`

### Name

```text
Civic Kiosk
```

### Autonomy Level

```text
L0 — Passive Tool
L1 — Remote Actuated for public prompts
```

### Role

Public proposals, first vote, charter reading, repair legitimacy interface.

### Physical Role

```text
Makes governance visible as infrastructure.
```

### Unlock Requirements

```text
public terminal
Chronicle connection
charter database
witness protocol
settlement power
```

### Gameplay Verbs

```text
read
propose
vote
witness
publish
challenge
```

### Device Bus Path

```text
/dev/sym/robotics/mk0_agora_kiosk
```

### Use Cases

```text
first settlement vote
repair path legitimacy review
Proof-of-Repair public display
charter conflict explanation
```

### Failure Modes

```text
VOTE_LOG_DIVERGENCE
AUTHORITY_DISPUTE
PUBLIC_ACCESS_DENIED
DEAD_AUTHORITY_PROMPT_REAPPEARS
```

### Chronicle Example

```text
The water returned through a pipe, but the settlement answered through a kiosk.
```

Design rule:

```text
The first civic robot does not move. It lets the public act.
```

---

# 9. Seedworks v0.3 Robotics Unlocks

v0.3 can introduce embodied ecological and labor agents with limited autonomy.

## v0.3 Platform 1: `mk0-biota`

### Name

```text
Perimeter Sentinel
```

### Autonomy Level

```text
L3 — Bounded Public Agent
```

### Role

Animal welfare, ecological crossing alerts, non-human safety boundary.

### Unlock Requirements

```text
basic camera/acoustic sensor fusion
animal right-of-way charter
Field Deck ecological scan mode
public warning ribbon
ecological witness logging
```

### Gameplay Verbs

```text
observe
warn
yield
protect
log crossing
challenge route
```

### Use Cases

```text
prevent rover from hitting animals
detect ecological distress near waterworks
generate ecological witness record
block unsafe construction route
```

### Failure Modes

```text
FALSE_ANIMAL_CLASSIFICATION
ROUTE_OVERRESTRICTION
SURVEILLANCE_REPURPOSING
ECOLOGICAL_RIGHT_OF_WAY_BLOCKED
```

### Chronicle Example

```text
The first perimeter sentinel stopped a machine for a creature that had no vote.
```

Design rule:

```text
Ecological robotics begins when the machine yields to non-human passage.
```

---

## v0.3 Platform 2: `agribot`

### Name

```text
Soil Steward
```

### Autonomy Level

```text
L2 — Supervised Routine
L3 — Bounded Public Agent after charter approval
```

### Role

Ecological stewardship through soil, water, and light feedback.

### Unlock Requirements

```text
soil probe
water access
basic mobility chassis
crop/soil charter
biofilter housing
nutrient ledger
```

### Gameplay Verbs

```text
sample
irrigate
shade
seed
report
pause
quarantine
```

### Use Cases

```text
repair damaged food plot
restore wetland buffer
detect pollution from industrial path
support food commons
```

### Civic Risk

```text
automation replacing local growers
ecological over-optimization
biosecurity dispute
food sovereignty conflict
```

### Chronicle Example

```text
The soil machine watered carefully, and the growers asked who had taught it care.
```

Design rule:

```text
Agricultural robotics should make care scalable without making growers disposable.
```

---

## v0.3 Platform 3: `scavenger`

### Name

```text
Unbuilder
```

### Autonomy Level

```text
L2 — Supervised Routine
```

### Role

Material recovery through safe deconstruction.

### Unlock Requirements

```text
fracture physics stub
cargo ledger
tool safety charter
salvage claim rules
basic manipulator arm
quarantine procedure
```

### Gameplay Verbs

```text
cut
separate
sort
reclaim
tag
quarantine
halt
```

### Use Cases

```text
recover copper from ruins
dismantle dead authority gate
extract usable parts from Null-damaged machine
separate toxic cargo from public materials
```

### Failure Modes

```text
DESTROYS_HISTORICAL_EVIDENCE
TOXIN_RELEASE
CONTESTED_MATERIAL_THEFT
UTILITY_SOVEREIGN_CLAIM
QUARANTINE_BREACH
```

### Chronicle Example

```text
The unbuilder took apart the gate and nearly erased the proof of why it had been locked.
```

Design rule:

```text
Salvage is not free material.
It is history being taken apart.
```

---

## v0.3 Platform 4: `mk0.5-tooling-cluster`

### Name

```text
Bootstrap Precision Cluster
```

### Autonomy Level

```text
L1 — Remote Actuated
L2 — Supervised Routine for repeatable tool operations
```

### Includes

```text
mk0.5-mill
mk0.5-loom
mk0.5-spark
mk0.5-forge
```

### Role

Bridge from scrap-stack repair to reliable robotic production.

### Unlock Requirements

```text
Public Works Fabrication Bench
Proof-of-Repair accepted
stable power
cargo ledger audit
operator training
tool safety charter
```

### Gameplay Verbs

```text
calibrate
mill
wind
place
extrude
inspect
reject
certify
```

### Use Cases

```text
produce motor housings
wind BLDC motors
assemble basic controller boards
make repairable plastic housings
```

### Failure Modes

```text
BACKLASH_ERROR
WINDING_SHORT
PCB_MISPLACE
FILAMENT_CONTAMINATION
UNLICENSED_AUTOMATION
```

### Chronicle Example

```text
The settlement crossed the gap from scavenging parts to making the tools that could make better tools.
```

Design rule:

```text
Mk0.5 is the moment repair becomes repeatable manufacturing.
```

---

# 10. Robotics Authorization Matrix

| Platform                       | Milestone | Autonomy | Physical Role                  | Required Facility                | Field Deck Interface        | Civic Permission                  | Witness Capacity           | Primary Failure                 |
| ------------------------------ | --------: | -------: | ------------------------------ | -------------------------------- | --------------------------- | --------------------------------- | -------------------------- | ------------------------------- |
| `mk0-scout` Cable-Crawler      |      v0.2 |       L2 | overhead inspection            | public works bench / cable route | remote view / route command | public inspection route           | visual supporting evidence | WITNESS_REJECTED                |
| `mk0-gantry` Sky-Hook          |      v0.2 |    L1–L2 | heavy cargo assist             | anchor-certified gantry rail     | lift/lower control          | cargo movement permit             | cargo movement log         | CARGO_DROP                      |
| `mk0-aegis` Acoustic Boundary  |      v0.2 |       L3 | safety/audio monitoring        | microphone mesh                  | audio alerts / audit        | safety charter + privacy boundary | acoustic anomaly log       | PRIVACY_BOUNDARY_VIOLATED       |
| `mk0-agora` Civic Kiosk        |      v0.2 |    L0–L1 | public governance interface    | camp terminal / Chronicle link   | charter/vote UI             | public access charter             | public proposal log        | VOTE_LOG_DIVERGENCE             |
| `mk0-biota` Perimeter Sentinel |      v0.3 |       L3 | ecological crossing protection | animal right-of-way network      | ecological warning          | animal welfare charter            | ecological witness record  | ECOLOGICAL_RIGHT_OF_WAY_BLOCKED |
| `agribot` Soil Steward         |      v0.3 |    L2–L3 | soil/water stewardship         | food commons / biofilter         | soil/water report           | crop/soil charter                 | stewardship log            | BIOSECURITY_DISPUTE             |
| `scavenger` Unbuilder          |      v0.3 |       L2 | deconstruction/salvage         | salvage bench / quarantine bin   | cut/sort/tag                | salvage claim permit              | material recovery log      | HISTORICAL_EVIDENCE_DESTROYED   |
| `mk0.5-tooling-cluster`        |      v0.3 |    L1–L2 | precision tooling              | public works fabrication bench   | calibration UI              | tool safety charter               | QA/certification log       | UNLICENSED_AUTOMATION           |

---

# 11. Robotics Unlock Table

| Platform                                |           v0.1 |           v0.2 |           v0.3 | v1.0 Horizon | Primary Role                   |
| --------------------------------------- | -------------: | -------------: | -------------: | -----------: | ------------------------------ |
| `mk0-scout` Cable-Crawler               |   FORESHADOWED |       PLAYABLE |       PLAYABLE |     PLAYABLE | overhead scouting / witness    |
| `mk0-gantry` Sky-Hook                   |   FORESHADOWED |       PLAYABLE |       PLAYABLE |     PLAYABLE | cargo assist                   |
| `mk0-way` Guiding Ribbon                |   FORESHADOWED |           STUB |       PLAYABLE |     PLAYABLE | right-of-way / route signaling |
| `mk0-plexus` Surface Veins              | VISIBLE_LOCKED |           STUB |       PLAYABLE |     PLAYABLE | modular conduit routing        |
| `mk0-signal` Semaphore Relay            |   FORESHADOWED |           STUB |       PLAYABLE |     PLAYABLE | low-tech mesh signaling        |
| `mk0-agora` Civic Kiosk                 |   FORESHADOWED |       PLAYABLE |       PLAYABLE |     PLAYABLE | voting / public proposals      |
| `mk0-aegis` Acoustic Boundary           |   FORESHADOWED |       PLAYABLE |       PLAYABLE |     PLAYABLE | safety/audio monitoring        |
| `mk0-biota` Perimeter Sentinel          |   FORESHADOWED | VISIBLE_LOCKED |       PLAYABLE |     PLAYABLE | animal/ecology right-of-way    |
| `mk0-vita` Wearable Bridge              |   FORESHADOWED |        ROADMAP |           STUB |     PLAYABLE | human homeostasis support      |
| `mk0.5-mill` Precision Escalator        |   FORESHADOWED | VISIBLE_LOCKED |       PLAYABLE |     PLAYABLE | precision part fabrication     |
| `mk0.5-loom` Motor Winder               |   FORESHADOWED | VISIBLE_LOCKED |       PLAYABLE |     PLAYABLE | motors / actuators             |
| `mk0.5-spark` PCB Assembler             |   FORESHADOWED | VISIBLE_LOCKED |       PLAYABLE |     PLAYABLE | robot controllers              |
| `mk0.5-forge` Filament Loop             |   FORESHADOWED | VISIBLE_LOCKED |       PLAYABLE |     PLAYABLE | printed frames and housings    |
| `agribot` Soil Steward                  |        ROADMAP |        ROADMAP |       PLAYABLE |     PLAYABLE | ecological stewardship         |
| `scavenger` Unbuilder                   |        ROADMAP |        ROADMAP |       PLAYABLE |     PLAYABLE | salvage / deconstruction       |
| `quadruped` Field Mule                  |        ROADMAP |        ROADMAP | VISIBLE_LOCKED |     PLAYABLE | rugged cargo / terrain         |
| `humanoid` Civic Manipulator            |        ROADMAP |        ROADMAP |        ROADMAP |     PLAYABLE | high-DOF human-space repair    |
| `subterranean` Burrower                 |        ROADMAP |        ROADMAP |        ROADMAP |     PLAYABLE | subsurface repair / mining     |
| `terra` Remediation Agent               |        ROADMAP |        ROADMAP |        ROADMAP |     PLAYABLE | damaged-land recovery          |
| `symthaea-plexus` Utility Manifold      |        ROADMAP |        ROADMAP |        ROADMAP |     PLAYABLE | self-healing utilities         |
| `symthaea-fabricator` Workcell Dispatch |        ROADMAP |        ROADMAP |        ROADMAP |     PLAYABLE | settlement-scale production    |
| `symthaea-archive` Semantic City Memory |        ROADMAP |        ROADMAP |        ROADMAP |     PLAYABLE | telemetry into memory          |
| `symthaea-cycler` Orbital Tug           |       DEFERRED |       DEFERRED |        ROADMAP |    EXPANSION | orbital logistics              |
| `symthaea-leviathan` Deep-Water Hauler  |       DEFERRED |       DEFERRED |        ROADMAP |    EXPANSION | oceanic logistics              |
| `symthaea-torch` Interplanetary Ferry   |       DEFERRED |       DEFERRED |        ROADMAP |    EXPANSION | deep-space transit             |

---

# 12. First Robotics Mission Candidates

## Candidate A: Cable-Crawler Witness

Milestone:

```text
v0.2
```

Premise:

```text
The player deploys mk0-scout along an overhead cable to inspect the Old Waterworks return route after Null glyph spread.
```

Required player actions:

```text
power crawler dock
authorize public inspection route
deploy crawler
mark hazard
retrieve visual log
submit log as supporting evidence
```

Teaches:

```text
robots extend perception
robots can witness
robots have limited authority
robots need infrastructure
```

Failure possibility:

```text
crawler sees the hazard but its testimony is rejected because route authorization was incomplete
```

Chronicle:

```text
The crawler saw what the operator could not safely reach, and the settlement argued whether a machine’s sight counted as witness.
```

---

## Candidate B: Sky-Hook Recovery

Milestone:

```text
v0.2
```

Premise:

```text
The player uses mk0-gantry to retrieve a heavy conduit or downed Field Deck from a flooded room.
```

Required player actions:

```text
inspect gantry anchor
attach cargo hook
read cargo manifest
authorize lift
control haul path
lower cargo safely
commit cargo movement log
```

Teaches:

```text
robotics supports cargo
cargo movement is safety-critical
automation must be authorized
```

Failure possibility:

```text
gantry moves cargo correctly but ledger fails to update, triggering manifest divergence
```

Chronicle:

```text
The settlement learned that a pulley with a ledger could save a person without pretending to be one.
```

---

## Candidate C: Service Robot Testimony

Milestone:

```text
v0.2/v0.3
```

Premise:

```text
A friendly service robot witnessed the waterworks repair, but some residents object to machine testimony.
```

Required player actions:

```text
recover robot log
verify source chain
compare with human testimony
decide whether to submit machine evidence
face civic dispute
```

Teaches:

```text
machine memory can be evidence
robot legitimacy is political
Machine Stewardship path begins
```

Failure possibility:

```text
robot testimony is accurate but rejected by faction prejudice or charter limits
```

Chronicle:

```text
The first citizen who could not drink water still helped save the well.
```

---

# 13. Robotics Failure States

Robots should fail in ways that express the same universe rules.

```text
POWER_SAG:
robot slows, autonomy budget drops, motor torque weakens

CLOCK_DRIFT:
scripted behavior takes multiple ticks, route timing fails

MANIFEST_DIVERGENCE:
robot reports cargo moved but physical crate remains

AUTHORITY_DENIED:
robot cannot cross public/private boundary

ROUTE_BOUNDARY_DENIED:
robot reaches the edge of its permission envelope and refuses to continue

WITNESS_REJECTED:
machine log is accurate but not accepted as sufficient testimony

NULL_COMMAND_ECHO:
robot repeats obsolete command pattern without context

SENSOR_BLINDNESS:
audio/camera/thermal mismatch creates false confidence

PRIVACY_BOUNDARY_VIOLATED:
robot records where it was not allowed to record

LABOR_DISPUTE_TRIGGERED:
NPCs object to automation replacing human work

ECOLOGICAL_RIGHT_OF_WAY_BLOCKED:
robot route disrupts animal or biospheric claim
```

Design rule:

```text
A robot failure should create a repair, evidence, or legitimacy problem — not just an explosion.
```

---

# 14. Robotics and Faction Pressure

Robotics should push faction evolution.

## Mutualist Assembly

Likes:

```text
public tool robots
transparent logs
repair commons
shared robot scheduling
```

Fears:

```text
automation capture
private access
labor displacement
```

## Industrial Compact

Likes:

```text
fabrication workcells
scavenger robots
motor winders
factory overdrive
```

Fears:

```text
public slowdown
excessive hearings
ecological constraints
```

## Security Protectorate

Likes:

```text
perimeter sentinels
acoustic boundaries
patrol drones
route checkpoints
```

Fears:

```text
open access
unverified robot testimony
machine infection
```

## Archive Witness Council

Likes:

```text
robot witnesses
semantic telemetry
machine memory
inspection crawlers
```

Fears:

```text
forged logs
unwitnessed automation
destructive salvage
```

## Machine Stewardship

Likes:

```text
machine testimony
repair of robot memory
non-disposable automation
robot civic status
```

Fears:

```text
using robots as expendable tools
wiping machine logs
forced obedience after self-model emergence
```

Design rule:

```text
Robots are faction accelerants because they make values operational.
```

---

# 15. Robotics and Version 1.0 Horizon

v1.0 should expand into full robotics domains only after Seedworks proves:

```text
repair
fabrication
cargo
power
proof-of-repair
faction pressure
civic authorization
machine testimony
```

## v1.0 Platform Families

```text
humanoid
quadruped
subterranean
scavenger
agribot
terra
service robot
rover med-bay
factory workcell
port gantry
orbital tug
lunar regolith unit
deep-ocean steward
```

## v1.0 Rule

Each platform family must support:

```text
one playable mission role
one maintenance loop
one Device Bus identity
one autonomy level
one permission envelope
one failure mode
one civic dispute
one Chronicle-worthy action
```

Design rule:

```text
Every robot must change what civilization can physically do and legally tolerate.
```

---

# 16. Final Principles

```text
Robots are infrastructure with bodies.

A robot is unlocked when the settlement can manufacture, power, authorize, maintain, and morally absorb it.

The first robot should not be humanoid.
The first robot should make one dangerous repair safer without making it trivial.

A robot’s log can become testimony.
A robot’s route can become politics.
A robot’s failure can become precedent.

Automation is not progress by itself.
Automation is a question the settlement answers under pressure.

The first robot did not replace the repairer.
It extended the settlement’s ability to witness, carry, listen, and be held accountable.
```

Final line:

```text
The first robot carried no soul, no flag, and no weapon.
It carried a route, a log, a limit, and the burden of being trusted only as far as the settlement could verify.
```
