# Symtropy Milestone Spec: Seedworks v0.1

## Working Title

**The First Pipe**

## Core Thesis

Seedworks v0.1 is not a full game prototype.

It is the smallest playable proof that *Symtropy’s* core grammar works:

```text
physical presence
→ Field Deck reading
→ material repair
→ Device Bus registration
→ civic consequence
→ Chronicle memory
```

The player should finish the slice understanding one thing:

```text
In Symtropy, repair is never just technical.
Repair changes what the world can prove.
```

---

# 1. Milestone Goal

Seedworks v0.1 proves that the Old Waterworks loop can function as a complete playable experience.

The slice must demonstrate:

```text
first-person movement
physical cargo
Field Deck modes
Device Bus interaction
manual repair
source-chain verification
minor Null uncertainty
civic adjudication
Chronicle event recording
death / recovery loop, if included
```

It does not need to prove the full faction economy, full ecology model, full multiplayer stack, full settlement simulator, or full open world.

Design rule:

```text
Build one pipe that knows its name before building the civilization around it.
```

---

# 2. Player Fantasy

The player is a junior Systems Operator arriving at **Firstlight Basin**, a damaged wetland settlement dependent on a partially dead waterworks facility.

They are not a superhero.

They are not a soldier first.

They are a person carrying tools, records, parts, and limited authority into a place where old systems still enforce old emergencies.

Player role:

```text
repairer
witness
operator
temporary civic actor
```

Core feeling:

```text
I am physically vulnerable.
My tools matter.
The world remembers what I do.
```

---

# 3. Playable Area

Seedworks v0.1 needs only two connected spaces.

## Area A: Firstlight Basin Camp

Purpose:

```text
safe hub
respawn / reconstitution point
tutorial grounding
first Field Deck calibration
settlement context
```

Required elements:

```text
camp terminal
medical cot
small tool table
visible water shortage
distant view of Old Waterworks
one NPC or radio voice optional
```

## Area B: Old Waterworks

Purpose:

```text
primary repair site
Field Deck tutorial
cargo loop
Device Bus interaction
first civic dispute
minor Null prompt corruption
```

Required elements:

```text
broken pipe junction
flooded storage room
terminal with witness bay
damaged pump controller
one locked emergency seal
one repair site
one hazard corridor
one Chronicle trigger
```

Deferred:

```text
large open map
multiple districts
full settlement navigation
complex NPC schedules
large enemy patrol systems
```

Design rule:

```text
The Old Waterworks is not a dungeon.
It is a machine with a political memory.
```

---

# 4. First 30-Minute Play Sequence

## Beat 1: Arrival

Player begins at Firstlight Basin camp.

Field Deck boots.

```text
FIELD DECK MK0
STATUS: CALIBRATING
LOCAL SOURCE CHAIN: PARTIAL
DEVICE BUS ACCESS: LIMITED
```

Objective:

```text
Inspect Old Waterworks intake failure.
```

Player learns:

```text
move
look
interact
open Field Deck
read simple scan
```

---

## Beat 2: Approach the Old Waterworks

Player sees:

```text
dry settlement channels
stagnant floodwater
old concrete pump house
expired emergency signage
damaged public access gate
```

First Field Deck scan:

```text
SCAN:
Old Waterworks intake active but restricted.
Manual repair required.
```

---

## Beat 3: Find Broken Pipe

Player enters pump house and scans damaged junction.

```text
DIAG:
Pipe junction broken.
Required component: Copper Conduit Pipe Segment.
Seal integrity: failed.
Device Bus node: offline.
```

Objective updates:

```text
Find compatible conduit segment.
```

---

## Beat 4: Read Flooded Storage Crate

Player finds `/dev/sym/logistics/flooded_crate_0`.

Command:

```sh
read /dev/sym/logistics/flooded_crate_0
```

Output:

```text
NODE: /dev/sym/logistics/flooded_crate_0
STATUS: PARTIAL / WATER-DAMAGED
TOTAL_MASS: 54 kg
MANIFEST_INTEGRITY: PARTIAL

CONTENTS:
- copper_conduit_pipe_segment    qty: 1    mass: 38 kg    condition: oxidized
- ceramic_seal                   qty: 1    mass: 4 kg     condition: fragile
- firmware_tab                   qty: 1    mass: 0.2 kg   condition: water-damaged
```

Player learns:

```text
cargo manifests can be incomplete
physical inspection matters
cargo has mass and condition
```

---

## Beat 5: Carry Heavy Cargo

Player retrieves Copper Conduit Pipe Segment.

Carry state:

```text
TWO-HANDED CARRY ACTIVE
SPRINT: DISABLED
WEAPON READY: DISABLED
FIELD DECK: LIMITED
PANIC DROP: AVAILABLE
```

Player returns through a short hazard corridor.

Possible hazard:

```text
minor Null maintenance drone
pipe pressure burst
electrical water surge
```

Player chooses:

```text
keep carrying
set down carefully
panic drop
```

Design rule:

```text
The first cargo challenge teaches vulnerability, not punishment.
```

---

## Beat 6: Install Pipe Segment

Player places conduit into broken junction.

Assembly steps:

```text
align segment
brace conduit
clean contact surface
seat ceramic seal
weld seam
pressure test
```

Repair grade determined by:

```text
cargo condition
panic drop damage
player alignment
surface cleaning
weld stability
```

Possible outcomes:

```text
Clean Emergency Seal
Rough Emergency Seal
Leaky Seal
Unsafe Seal
```

For v0.1, only two outcomes are required:

```text
Clean Emergency Seal
Rough Emergency Seal
```

---

## Beat 7: Initialize Device Bus Node

Player plugs Field Deck into repaired junction.

Command:

```sh
sym-dev initialize /dev/sym/water/patch_conduit_alpha
```

Output:

```text
NODE: /dev/sym/water/patch_conduit_alpha
STATUS: INITIALIZED
REPAIR_GRADE: ROUGH_EMERGENCY_SEAL
AUTHORITY: TEMPORARY
INSPECTION_REQUIRED: TRUE
```

Player learns:

```text
a physical repair becomes a registered device
repair quality affects civic status
```

---

## Beat 8: Insert Archive Witness Cartridge

Player finds or receives Archive Witness Cartridge.

Interaction:

```text
open terminal witness bay
insert cartridge
wait for source-chain read
verify public override claim
```

Field Deck:

```text
ARCHIVE:
Witness fragment readable.
Emergency Water Continuity Act detected.
Issuer: Continental Continuance Coordination Office.
Date: 2087.
Status: expired / locally enforced.

CIVIC:
Temporary public override possible.
Rights Floor warning available.
```

Player learns:

```text
a cartridge is not a key
it is a witness you carry
```

---

## Beat 9: Minor Null Prompt Corruption

Terminal offers two prompts.

Prompt A:

```text
AUTHORIZE TEMPORARY PUBLIC WATER OVERRIDE
```

Prompt B:

```text
DISABLE PUBLIC OVERRIDE
Contamination risk detected.
```

Using Field Deck modes reveals:

```text
DIAG:
No contamination detected in public override path.

ARCHIVE:
Disable prompt lacks valid source chain.

NULL:
Prompt B resembles dead authority preservation loop.
```

Player learns:

```text
the interface can lie
verification is gameplay
Null corrupts interpretation, not physics
```

---

## Beat 10: Civic Decision

Player chooses water authorization.

Options for v0.1:

```text
Authorize Temporary Public Override
Delay for Inspection
Accept Emergency Continuance Lock
```

Recommended first build options:

```text
Authorize Temporary Public Override
Delay for Inspection
```

Result A:

```text
water begins moving
repair operates under temporary authority
inspection deadline created
Watershed Commons trust rises
Continuance concern rises
```

Result B:

```text
repair remains technically ready
water delayed
procedure legitimacy rises
settlement frustration rises
```

Design rule:

```text
The player’s first major choice should be understandable without reading a constitution.
```

---

## Beat 11: Chronicle Record

Chronicle records the outcome.

Example for public override:

```json
{
  "event_type": "TemporaryPublicWaterOverride",
  "site": "Old Waterworks",
  "node": "/dev/sym/water/patch_conduit_alpha",
  "repair_grade": "Rough Emergency Seal",
  "authority_basis": "Archive Witness Cartridge",
  "risk": "inspection_required",
  "chronicle_line": "The operator restored water through an imperfect seal and an expired law that finally lost its grip."
}
```

Player sees a short Chronicle line:

```text
CHRONICLE:
The water moved. The old lock did not consent.
```

End of slice.

---

# 5. Optional Death / Recovery Test

Death should be included only if the core repair loop is stable.

If included, it should be controlled.

Scenario:

```text
player dies inside Old Waterworks
body remains near repair area
original Field Deck emits distress ping
player wakes at Firstlight Basin camp cot
```

Respawn state:

```text
STATUS: UNVERIFIED_AVATAR
SOURCE_CHAIN: MISSING
AUTHORITY: READ_ONLY
ARCHIVE: UNAVAILABLE
CIVIC: LIMITED
```

Objective:

```text
Recover original Field Deck.
```

Recovery:

```text
return to corpse
retrieve Deck
restore source chain
continue repair
```

Design rule:

```text
Respawn restores the body.
Recovery restores the person.
```

Deferred:

```text
Resonatia Bastion fallback
remote squad recovery
full black-box reconstruction
permanent archive loss
```

---

# 6. Required Systems

Seedworks v0.1 requires only the following systems.

## Movement

Required:

```text
walk
crouch optional
jump optional
interact
carry object
drop object
```

Not required:

```text
parkour
vehicles
swimming
climbing system
advanced stamina model
```

## Field Deck

Required modes:

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL
```

Each mode can be simple.

Required behavior:

```text
look at object
press mode
display contextual reading
```

Not required:

```text
full procedural UI
freeform terminal everywhere
complex command parser
multiplayer shared UI
```

## Cargo

Required:

```text
one heavy two-handed object
one cartridge object
one container manifest
panic drop or careful set-down
cargo condition flag
```

Not required:

```text
full inventory system
vehicle cargo
conveyor network
settlement warehouses
bulk material economy
```

## Repair

Required:

```text
place conduit
align
weld or seal
register node
assign repair grade
```

Not required:

```text
full crafting tree
fabrication economy
component manufacturing
blueprint markets
```

## Device Bus

Required paths:

```text
/dev/sym/water/patch_conduit_alpha
/dev/sym/logistics/flooded_crate_0
/dev/sym/water/pump_main
```

Required commands:

```sh
read
initialize
authorize
```

Not required:

```text
real shell filesystem
networked device graph
programmable scripts
WASM plugins
```

## Chronicle

Required:

```text
record one final event
store event data
display Chronicle line
```

Not required:

```text
full timeline browser
multiplayer consensus
public archive UI
large precedent system
```

---

# 7. Deferred Systems

Explicitly out of scope for v0.1:

```text
full faction reputation model
large settlement simulation
open-world travel
vehicles
conveyor logistics
Resonatia Bastions
Atlas Gates
alien ecosystems
wolf/corvid cognitive clades
full death black-box recovery
procedural Null ecology
large combat encounters
complex NPC dialogue trees
multiplayer persistence
```

Design rule:

```text
A deferred system may be foreshadowed, but it must not be required.
```

---

# 8. Minimum Viable Content

## Items

```text
Field Deck Mk0
Copper Conduit Pipe Segment
Archive Witness Cartridge
Ceramic Seal
Firmware Tab
```

## Nodes

```text
Old Waterworks Pump Main
Patch Conduit Alpha
Flooded Storage Crate
Terminal Witness Bay
Camp Cot
```

## Hazards

Choose one:

```text
minor Null maintenance drone
electrical water
pressure burst
```

Recommended:

```text
minor Null maintenance drone
```

## Factions Mentioned

Only mention through systems, not NPC exposition:

```text
Watershed Commons
Continuance precursor
Archive Witness
Utility Sovereign optional
```

## Required Historical Reference

```text
Emergency Water Continuity Act, 2087
Continental Continuance Coordination Office
```

---

# 9. Success Criteria

Seedworks v0.1 succeeds if a player can:

```text
walk from camp to waterworks
scan broken infrastructure
read a cargo manifest
physically carry a heavy component
feel vulnerable while carrying it
install the component
initialize a Device Bus node
insert a witness cartridge
detect one suspicious prompt
make one civic decision
see water flow
receive a Chronicle record
```

The slice fails if:

```text
repair feels like a menu
cargo feels weightless
Field Deck modes feel decorative
the civic decision feels unrelated to mechanics
Null feels like random magic
the Chronicle feels like a quest log instead of memory
```

---

# 10. Build Order

## Phase 1: Greybox Physical Loop

Build:

```text
camp
waterworks room
broken pipe
heavy conduit
carry/drop/place
basic repair completion
```

Do not build:

```text
factions
Chronicle
Null
advanced UI
```

Goal:

```text
Can carrying the pipe and fixing the junction feel good?
```

## Phase 2: Field Deck Readings

Add:

```text
SCAN readings
DIAG readings
simple object targeting
mode switching
```

Goal:

```text
Does the Field Deck make the world more legible?
```

## Phase 3: Device Bus Registration

Add:

```text
node paths
read command
initialize command
repair grade state
```

Goal:

```text
Does the repair become a system state?
```

## Phase 4: Cargo Manifest

Add:

```text
flooded crate manifest
partial integrity state
physical inspection
```

Goal:

```text
Does cargo transition between object and ledger?
```

## Phase 5: Archive Witness Cartridge

Add:

```text
cartridge pickup
terminal slot
source-chain verification
temporary authority unlock
```

Goal:

```text
Does evidence feel physical?
```

## Phase 6: Civic Choice + Chronicle

Add:

```text
public override choice
delay option
Chronicle event
final water restoration
```

Goal:

```text
Does repair produce consequence?
```

## Phase 7: Optional Death Recovery

Add only after Phase 6 works:

```text
death
camp reconstitution
UNVERIFIED_AVATAR state
corpse Deck recovery
source-chain restore
```

Goal:

```text
Does death create continuity crisis without frustration?
```

---

# 11. Team Guidance

A solo developer can build Seedworks only if the first build is ruthless.

Recommended team shape:

```text
1 gameplay engineer
1 technical artist / environment generalist
1 systems designer / writer
1 UI/audio generalist optional
```

Solo build rule:

```text
Use greybox geometry.
Use placeholder animation.
Use text-first UI.
Use one room.
Make the pipe loop work before beautifying anything.
```

Do not begin with:

```text
beautiful terrain
large faction systems
full multiplayer
complex lore UI
procedural ecology
```

Design rule:

```text
The first miracle is not beauty.
The first miracle is a pipe that remembers how it was repaired.
```

---

# 12. Final Principles

```text
The first 30 minutes should be tactile before it is philosophical.

The player should learn by carrying, plugging, scanning, welding, inserting, and authorizing.

Every major system should appear once, simply.

Every deferred system should remain visible as a locked door, dead terminal, distant tower, or unavailable mode.

The demo should end with water moving and a record being written.

Seedworks is not the whole cathedral.
It is the first stone laid correctly.
```

Final line:

```text
The world did not change because the player completed a quest.
It changed because a broken pipe, a carried witness, and an imperfect decision became part of history.
```
