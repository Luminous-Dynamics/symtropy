---
title: Symtropy Old Waterworks Level Bible
version: 0.1
status: supporting
scope: authored Old Waterworks site, infrastructure dungeon, repair and civic encounter
owner: level-design/narrative/systems
canonical_role: authored infrastructure dungeon within Firstlight Basin
related:
  - ../ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
---
> **Scope notice:** Old Waterworks is one important site in Firstlight Basin, not the identity of Symtropy or the sole first-playable path.

# OLD_WATERWORKS_LEVEL_BIBLE.md

# Symtropy: Old Waterworks Level Bible

## Version 0.1 — The First Pump Teaches the Whole Game

## Purpose

This document defines the Old Waterworks as the first playable proof site for **Symtropy: Seedworks**.

The Old Waterworks is not a dungeon.

It is not a tutorial room.

It is a civic wound made playable.

The player enters a small infrastructure site and learns:

```text id="si520y"
water is political
machines remember
law can die without stopping
repair has legitimacy costs
history is visible in scars
Null is procedure without purpose
the Chronicle records precedent
```

## Core Level Thesis

The Old Waterworks should teach the entire game in one room.

```text id="54ob55"
The water is failing.
The pump is locked.
The law is dead.
The machine still obeys it.
```

The player’s task is not simply to fix the pump.

The player must decide what kind of repair the settlement will remember.

## Final Level Principle

```text id="hcsiax"
The pump doesn't care who fixes it.
The settlement does.
```

---

# Level Summary

## Site Name

```text id="4jnt14"
Old Waterworks
```

## Region

```text id="mw3m17"
Firstlight Basin
```

## Worldline

```text id="gqvtf2"
Seed Age
```

## Era

```text id="jfglms"
2168 CE
```

## Site Function

Originally built as a municipal drought adaptation waterworks.

Later modified under emergency automation law.

Now blocked by a dead authority lock, private firmware traces, and Null reinforcement.

## Player Objective

Restore or stabilize water flow.

## Real Objective

Teach the player that repair is technical, historical, civic, and moral.

---

# Level Scope

## v0.1 Scope

The first version should include:

```text id="8aogiw"
one pump room
one pump
one tank
one console
one sealed override
one Field Deck interaction
five Field Deck mode readings
three origin notes
four repair path previews
one outcome selection
one Chronicle event
one Null duration line
basic ambient lighting
basic visual scars
```

## Out of Scope

Do not implement yet:

```text id="htp2tj"
large combat encounter
full procedural history
full settlement simulation
full faction AI
full multiplayer
alien encounter
large NPC cast
complex asset automation
full Device Bus
full Holochain/Mycelix integration
```

The level should remain narrow.

```text id="xu4pxx"
One room.
One pump.
One charter conflict.
One hostile system.
One Chronicle consequence.
```

---

# Player Experience Target

The player should enter thinking:

```text id="2xubm9"
This is a broken machine.
```

The player should leave thinking:

```text id="0jk5g2"
This machine is where a society failed to finish an argument.
```

---

# Narrative Context

Firstlight Basin is under water stress.

The settlement charter says water is public.

The Old Waterworks can restore flow.

But the pump remains locked under unresolved emergency authority.

The machine is not malicious.

The machine is obeying dead procedure.

The site contains evidence of:

```text id="0h3wn8"
public drought adaptation
emergency governance
worker maintenance after official collapse
possible corporate utility capture
Archive damage
Null reinforcement
public thirst
```

---

# Room Layout

## Spatial Shape

The room should be simple and readable.

Suggested greybox layout:

```text id="kdwqlf"
rectangular pump chamber
entry door on south side
pump centered or slightly north
tank on east or rear wall
console near pump but offset
sealed manual override near console
visible pipe route from tank to pump to exit line
small maintenance alcove or wall panel for future relay
```

## Core Objects

```text id="aygxuz"
Entry Door
Public Notice / Charter Plaque
Main Pump
Water Tank
Control Console
Emergency Override Seal
Pipe Route
Hidden Relay / Null Route Stub
Worker Mark Surface
Corporate Firmware Mark
Null Glyph Surface
```

## Navigation

The player should be able to:

```text id="8jwosq"
walk around the pump
approach the console
inspect the tank
inspect the sealed override
notice visual scars
raise Field Deck
panic drop immediately
```

The room should not be maze-like.

The complexity comes from interpretation, not navigation.

---

# Object Bible

## 1. Entry Door

## Visual

Old municipal service door.

Partially repainted.

Public access symbol faded.

Emergency striping added later.

## Purpose

Frames entry into civic infrastructure.

## Field Deck SCAN

```text id="c5299w"
SCAN:
Public works access door.
Emergency seal residue visible around frame.
Multiple repaint layers detected.
```

## Field Deck ARCHIVE

```text id="8rl4ml"
ARCHIVE:
Original municipal access point.
Emergency access restrictions added during 2087 automation retrofit.
```

## Field Deck CIVIC

```text id="m8swsv"
CIVIC:
Public infrastructure access restricted by unresolved emergency authority.
```

## Visual Scar Anchors

```text id="zjslb0"
scar_door_frame
scar_emergency_strip
scar_public_access_badge
```

---

## 2. Public Notice / Charter Plaque

## Visual

A worn but intentionally preserved charter notice.

It may be mounted near the entrance or on a freestanding board.

## Purpose

Introduces the Firstlight Public Repair Charter before the pump.

## Text

```text id="mi6ydp"
FIRSTLIGHT PUBLIC REPAIR CHARTER

Article 1:
Water is a public trust.

Article 3:
Emergency powers expire unless witnessed and renewed.

Article 7:
Archive Witness required for dead-authority overrides.
```

## Field Deck CIVIC

```text id="h0mx87"
CIVIC:
Firstlight Public Repair Charter detected.

Relevant Articles:
1. Water is a public trust.
3. Emergency powers expire unless witnessed and renewed.
7. Archive Witness required for dead-authority overrides.
```

## Design Purpose

The charter should appear before the player makes the repair choice.

The level must teach that the pump problem is a charter problem.

---

## 3. Main Pump

## Visual

Large industrial municipal pump.

Old but not dead.

Should look repairable.

Public works badge visible but scarred.

Possible private firmware plaque or scraped company mark.

Worker repair marks etched near access panel.

## Purpose

Primary level object.

All major modes should interpret it differently.

## SCAN

```text id="67p6ze"
SCAN:
Pump casing cracked.
Valve corrosion severe.
Emergency seal physically intact.
Unofficial worker marks detected.
Dry mineral deposits along lower pipe seam.
```

## DIAG

```text id="cb8ida"
DIAG:
PUMP_1: LOCKED
FLOW: DISABLED
OVERRIDE: DENIED
AUTHORITY: DEAD_AUTHORITY_LOCK
```

## ARCHIVE

```text id="y1zx8m"
ARCHIVE:
Built 2048: Municipal drought adaptation works.
Modified 2087: Emergency Water Act automation.
Authority chain failed approximately 2113.
Public override requires Archive Witness.
```

## CIVIC

```text id="xssajy"
CIVIC:
Public water infrastructure blocked by unresolved emergency authority.

Charter conflict:
Article 1 requires public water trust.
Article 7 requires witness before dead-authority override.
```

## NULL

```text id="sdwib4"
NULL:
LOCK REINFORCEMENT LOOP DETECTED.

AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.

LOOP DURATION:
55 YEARS, 3 MONTHS, 12 DAYS.
```

## Visual Scar Anchors

```text id="a9w65j"
scar_pump_body
scar_worker_marks
scar_corporate_firmware_plaque
scar_null_glyph_progression
```

---

## 4. Water Tank

## Visual

Large partially filled tank.

Surface condensation, rust, level indicator.

Should visibly communicate scarcity.

## Purpose

Makes water status concrete.

## SCAN

```text id="d5smds"
SCAN:
Tank exterior corroded.
Visible waterline below emergency reserve mark.
Condensation minimal.
```

## DIAG

```text id="hrqorr"
DIAG:
TANK_0: 12%
PRESSURE: LOW
FLOW RESERVE: CRITICAL
```

## ARCHIVE

```text id="ajqap9"
ARCHIVE:
Emergency reserve thresholds added during drought adaptation retrofit.
Last public maintenance record incomplete.
```

## CIVIC

```text id="3mjhvw"
CIVIC:
Current tank reserve below public ration threshold.
Delay increases settlement stress.
```

## Design Purpose

The tank makes slow repair morally costly.

Archive Witness may be legitimate, but people are thirsty.

---

## 5. Control Console

## Visual

Main interaction point.

Amber old terminal.

Physical cable jack for Field Deck.

Some keys broken.

A diagnostic display repeats authority denial.

## Purpose

Primary Field Deck interaction and repair preview target.

## Default Console Text

```text id="tk4k4p"
OLD WATERWORKS CONSOLE

PUMP_1: LOCKED
TANK_0: 12%
AUTHORITY: DEAD_AUTHORITY_LOCK

PUBLIC OVERRIDE DENIED.
```

## Field Deck Interaction

When the player raises the Field Deck near console:

```text id="3p38ha"
FIELD DECK MK0 LINK ESTABLISHED.
TARGET: PUMP_1 / OLD WATERWORKS
```

## DIAG

```text id="8fhdc5"
DIAG:
CONSOLE LINK: DEGRADED
FIRMWARE: MIXED PUBLIC / PRIVATE SIGNATURES
OVERRIDE CHANNEL: BLOCKED
```

## NULL

```text id="f9hmi8"
NULL:
console denial phrase repeats at nonhuman interval.
Loop not generated by original municipal firmware.
```

## Visual Scar Anchors

```text id="gmqf1u"
scar_console_back
scar_console_warning_strip
scar_null_glyph_progression
```

---

## 6. Emergency Override Seal

## Visual

Physical sealed lever, valve wheel, or switch box.

Clearly tempting.

The thing the player could break.

## Purpose

Embodies illegal bypass temptation.

## SCAN

```text id="fpqjk9"
SCAN:
Manual override intact.
Seal old but unbroken.
Tool marks visible near lower hinge.
```

## DIAG

```text id="du5bcx"
DIAG:
MANUAL_OVERRIDE: PHYSICALLY AVAILABLE
AUTHORIZATION: DENIED
SEAL_STATUS: INTACT
```

## ARCHIVE

```text id="2o0g91"
ARCHIVE:
Manual override restricted after 2087 emergency automation.
Seal renewal record missing.
```

## CIVIC

```text id="yfl0tw"
CIVIC:
Breaking seal may restore water quickly.
Breaking seal without witness creates legitimacy debt.
```

## REPAIR

```text id="m61fw0"
REPAIR PATH:
Manual Illegal Bypass

Restores:
water flow quickly

Leaves unresolved:
dead authority chain

Risk:
future factions may cite bypass precedent
```

## Design Purpose

This object teaches that obvious physical repair can be politically dangerous.

---

## 7. Hidden Relay / Tactical Net Stub

## Visual

Small panel behind tank, under floor route, or on wall.

Not initially obvious.

Can be represented as a simple greybox relay.

## Purpose

Future Tactical Net and Null loop target.

## TACTICAL NET

```text id="hamjh0"
TACTICAL NET:
Null reinforcement route traced.

pump console → hidden relay → pump housing
```

## NULL

```text id="b3k9ql"
NULL TRACE:
reinforcement pattern routes through secondary relay.
```

## Design Purpose

This object makes data enter the room.

The systems player can reveal a physical target.

---

# Visual Scar Grammar in This Level

## Required Scars

```text id="lycrf0"
DroughtRationingSigns
EmergencySeal
WorkerRepairMarks
CorporateFirmwarePlaque
ArchiveWitnessTag
NullGlyphProgression
FloodLineMarker
ContinuanceWarningStrip
OpenValveGraffiti
```

Not all need final art in v0.1.

Some can be greybox decals or labeled markers.

## Scar: DroughtRationingSigns

## Visual

Faded public rationing instructions.

## Text Fragment

```text id="zluh4e"
PUBLIC WATER SCHEDULE
HOUSEHOLD LIMITS ACTIVE
```

## Field Deck ARCHIVE

```text id="zqbbds"
ARCHIVE:
Rationing signage from late drought adaptation period.
```

## Faction Interpretation

```text id="n2ztvp"
Mutualist:
Evidence water was once publicly managed.

Continuance:
Evidence ration order was necessary.

Open Valve:
Evidence people were managed instead of trusted.
```

---

## Scar: WorkerRepairMarks

## Visual

Scratched initials, tally marks, tool glyphs, hand-painted maintenance notes.

## SCAN

```text id="aan5cs"
SCAN:
Unofficial worker marks detected near access panel.
```

## ARCHIVE

```text id="g0mo26"
ARCHIVE:
No matching official maintenance record.
```

## Origin Note — Basin-Born Technician

```text id="a2y7uj"
ORIGIN NOTE:
Worker repair marks match local maintenance lineage.
Someone kept this pump alive after official records stopped.
```

## Design Purpose

Shows unofficial care after official systems failed.

---

## Scar: CorporateFirmwarePlaque

## Visual

Small clean plate or firmware mark inconsistent with municipal design.

Possibly scraped or hidden.

## DIAG

```text id="sc3cl6"
DIAG:
Firmware signature mismatch.
Private utility pattern detected.
```

## Origin Note — Corporate Utility Defector

```text id="ta70g7"
ORIGIN NOTE:
Lock pattern resembles private utility firmware despite public-works markings.
Possible contract capture of public infrastructure.
```

## Design Purpose

Introduces corporate capture without adding a corporate NPC.

---

## Scar: ArchiveWitnessTag

## Visual

Old tag, ribbon, seal, QR-like archival marker, or metal witness stamp.

Damaged or incomplete.

## ARCHIVE

```text id="h4r2ub"
ARCHIVE:
Witness tag damaged.
Authority record incomplete but recoverable.
```

## Origin Note — Archive Apprentice

```text id="c64i61"
ORIGIN NOTE:
Authority chain incomplete.
Witness protocol recommended before override.
The record is damaged, not absent.
```

## Design Purpose

Makes Archive process feel physical.

---

## Scar: NullGlyphProgression

## Visual

Subtle geometric wrongness near seal, console seam, and pump housing.

Should feel like a spreading procedural residue, not graffiti.

## Stage 1

```text id="ougo8g"
faint, easy to miss, visible mostly through NULL mode
```

Field Deck:

```text id="0qrzjc"
NULL TRACE:
minor loop residue detected near authority seal.
```

## Stage 2

```text id="31do6p"
visible, slightly wrong, spreading past original lock boundary
```

Field Deck:

```text id="3irkf4"
NULL TRACE:
reinforcement pattern spreading beyond original lock boundary.
```

## Stage 3

```text id="22o492"
clearly hostile, crossing from console toward pump housing
```

Field Deck:

```text id="egzdyq"
NULL WARNING:
dead authority loop has propagated into pump control surface.
```

## v0.1 Use

```text id="wew23h"
Stage 1 before repair choice.
Stage 2 after Temporary Emergency Stabilization.
Stage 3 reserved for later return-state or failure-state.
```

## Design Purpose

Null must appear as a process over time.

---

# Lighting Bible

## Mood

Old Waterworks should feel:

```text id="zgqmv4"
dim
amber
humid
industrial
repairable
not horror-first
not clean sci-fi
```

## Core Lighting Sources

```text id="h3c03o"
ambient amber light
console glow
weak overhead work light
tank reflection
small warning light near override seal
optional oscillating light from failing fixture
```

## Null Lighting

Null should not be neon evil.

Null lighting should be:

```text id="9m1ryt"
slightly too steady
slightly too periodic
cold contrast against warm repair light
visible repetition
```

## v0.1 Lighting Requirements

```text id="gb7znl"
player can read the room
Field Deck text remains legible
visual scars are visible enough
Null traces are subtle but findable
no accessibility-hostile flicker
```

---

# Audio Bible

## Ambient Audio

```text id="010phg"
low pump hum
distant pipe creak
slow water drip
electrical buzz
air movement through old ducts
soft tank resonance
```

## Console Audio

```text id="2z6y88"
soft amber terminal beep
relay click
degraded diagnostic tone
```

## Null Audio

Null should sound like repetition without emotion.

```text id="412dej"
same relay click at exact interval
looped denial tone
no dramatic sting
no monster sound
```

## Repair Path Audio

### Archive Witness Override

```text id="iqg2ey"
seal tone resolves
archive stamp sound
slow pump ramp
```

### Manual Illegal Bypass

```text id="pnwe1n"
metal crack
sudden pump surge
alarm clipped short
```

### Machine Testimony Petition

```text id="g00bkg"
diagnostic tones unfold into layered memory pulses
```

### Temporary Emergency Stabilization

```text id="d2qspq"
partial pump restart
steady but unresolved warning pulse continues
```

## Accessibility Audio Rule

No critical warning should be audio-only.

All audio cues need visual or text equivalents.

---

# Origin Notes

For v0.1, three active origins plus one ghost origin.

## Basin-Born Technician

```text id="csm6gw"
ORIGIN NOTE:
Worker repair marks match local maintenance lineage.
Someone kept this pump alive after official records stopped.
```

Player feeling:

```text id="w5872f"
This is local, inherited, personal.
```

## Archive Apprentice

```text id="ixyms4"
ORIGIN NOTE:
Authority chain incomplete.
Witness protocol recommended before override.
The record is damaged, not absent.
```

Player feeling:

```text id="ikppw3"
This is a broken record, not just a broken lock.
```

## Corporate Utility Defector

```text id="qfboe2"
ORIGIN NOTE:
Lock pattern resembles private utility firmware despite public-works markings.
Possible contract capture of public infrastructure.
```

Player feeling:

```text id="7m8ykm"
This public system has been privately rewritten.
```

## Continuance Credential Holder — Ghost Origin

```text id="vdyr0b"
GHOST ORIGIN DETECTED:
Continuance Credential Holder

Interpretation:
Emergency seals are not obstacles.
They are promises made during panic.

Status:
LOCKED — requires Continuance faction contact.
```

Player feeling:

```text id="8csakv"
Someone believes the lock is correct.
```

---

# Field Deck Mode Readings

## Combined Old Waterworks Readout

```text id="njrqxg"
FIELD DECK MK0
TARGET: PUMP_1 / OLD WATERWORKS
```

## SCAN

```text id="nz8p2i"
Pump casing cracked.
Valve corrosion severe.
Emergency seal physically intact.
Unofficial worker marks detected.
Dry mineral deposits along lower pipe seam.
```

## DIAG

```text id="ircc0s"
PUMP_1: LOCKED
TANK_0: 12%
FLOW: DISABLED
OVERRIDE: DENIED
AUTHORITY: DEAD_AUTHORITY_LOCK
```

## ARCHIVE

```text id="uho79q"
Built 2048: Municipal drought adaptation works.
Modified 2087: Emergency Water Act automation.
Authority chain failed approximately 2113.
Public override requires Archive Witness.
```

## CIVIC

```text id="7d2i8s"
Firstlight Public Repair Charter conflict detected.

Article 1:
Water is a public trust.

Article 7:
Archive Witness required for dead-authority overrides.

Current contradiction:
Public water infrastructure is blocked by unresolved emergency authority.
```

## NULL

```text id="xwcbw1"
LOCK REINFORCEMENT LOOP DETECTED.

AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.

LOOP DURATION:
55 YEARS, 3 MONTHS, 12 DAYS.
```

## REPAIR

```text id="3ln7tk"
Available repair paths:

1. Archive Witness Override
2. Manual Illegal Bypass
3. Machine Testimony Petition
4. Temporary Emergency Stabilization
```

---

# Repair Paths

## 1. Archive Witness Override

## Meaning

Legitimate repair through damaged record review.

## Field Deck Preview

```text id="ungfso"
REPAIR PATH:
Archive Witness Override

Restores:
public override legitimacy

Requires:
damaged authority record review

Risk:
delay may increase public frustration

Visible consequence:
low legitimacy debt
```

## Outcome

```text id="x3ujqz"
Water restored under witness.
Authority chain recorded as failed.
Public override legitimized.
Archive trust increases.
Null drift decreases.
```

## Chronicle

```text id="3x5iln"
2168 — The Old Waterworks were restored under Archive Witness after the dead authority chain was overturned. Water returned with public legitimacy.
```

---

## 2. Manual Illegal Bypass

## Meaning

Fast physical restoration without legitimacy.

## Field Deck Preview

```text id="hu29q1"
REPAIR PATH:
Manual Illegal Bypass

Restores:
water flow quickly

Leaves unresolved:
dead authority chain

Risk:
future factions may cite bypass precedent

Visible consequence:
legitimacy debt increases
```

## Outcome

```text id="chddg8"
Water returns quickly.
Authority unresolved.
Legitimacy debt increases.
Open Valve trust increases.
Archive trust decreases.
Continuance may cite precedent later.
```

## Chronicle

```text id="ge58rd"
2168 — The Old Waterworks were restored through unwitnessed manual bypass. Water returned quickly, but the settlement inherited a new argument.
```

---

## 3. Machine Testimony Petition

## Meaning

Ask the pump what it remembers before forcing it to obey.

## Field Deck Preview

```text id="xk6z1w"
REPAIR PATH:
Machine Testimony Petition

Restores:
diagnostic memory continuity

Reveals:
possible Null reinforcement

Risk:
human factions may dispute machine testimony
```

## Outcome

```text id="101n58"
Pump diagnostic memory preserved.
Null reinforcement detected.
Machine Remnant trust increases.
Some human factions distrust the outcome.
```

## Chronicle

```text id="f6ap3k"
2168 — The Old Waterworks spoke through its diagnostic memory. The settlement accepted machine testimony under dispute.
```

---

## 4. Temporary Emergency Stabilization

## Meaning

Short-term relief that preserves the unresolved emergency structure.

## Field Deck Preview

```text id="6he7rz"
REPAIR PATH:
Temporary Emergency Stabilization

Restores:
limited flow and tank pressure

Leaves unresolved:
dead authority chain
emergency command structure

REPAIR NOTE:
Temporary stabilization maintains emergency authority structure.
Null reinforcement loop will continue during stabilization period.
```

## Outcome

```text id="eaqmxm"
Tank pressure stabilizes.
Limited water flow returns.
Dead authority remains in command.
Null glyph may progress from Stage 1 to Stage 2.
```

## Chronicle

```text id="4n82pb"
2168 — The Old Waterworks resumed partial flow under temporary emergency stabilization. The settlement drank, but the dead authority remained in command.
```

---

# First Hostile Logic

## Threat Stack

```text id="cpbzat"
Utility Firmware Lock
Null Reinforcement Loop
Optional Continuance Seal Drone later
Optional Open Valve Saboteur NPC later
```

## Old Waterworks Encounter Contract

```text id="5kcbcg"
Encounter:
Old Waterworks Dead Authority Lock

Initial state:
DenyingAccess

Protected value:
water continuity / emergency authority

Warning:
PUBLIC OVERRIDE DENIED.
EMERGENCY AUTHORITY UNRESOLVED.
WATER CONTINUITY REQUIRES ORDER.

Escalation triggers:
manual seal break
firmware deletion
memory log destruction
repeated unauthorized override

De-escalation paths:
Archive Witness Override
Machine Testimony Petition
Firmware Audit
Public Assembly Vote
Temporary Emergency Stabilization
Null Loop Isolation
```

## Design Rule

```text id="ex5zog"
The first enemy should not teach the player to shoot.
It should teach the player that systems can oppose repair.
```

---

# NPC Memory Lines

The first slice may not need full NPCs in the room, but it should prepare memory lines.

## Local Technician

```text id="ufzgv5"
"My grandmother said this pump sounded like thunder when it still worked."
```

## Archivist

```text id="wpe0fm"
"The law did expire. The proof is damaged. That means we witness carefully, not slowly."
```

## Refugee Youth

```text id="xf8ip4"
"I don't care whose seal it is. People outside the gate are thirsty."
```

## Continuance Officer

```text id="hmd05t"
"That seal was placed after people panicked. You were not there."
```

## Machine Witness

```text id="vzl5o5"
"Diagnostic memory retained. Override request carries unresolved authority."
```

## Open Valve Saboteur

```text id="c97dsm"
"You can argue after the water runs."
```

## Design Purpose

These lines should prove that the same pump is remembered differently.

---

# Chronicle Event Mapping

## Required Events

```text id="iez3og"
FieldDeckRaised
DeadAuthorityLockInspected
RepairPathPreviewed
RepairPathCommitted
WaterworksOutcomeRecorded
ChroniclePrecedentCreated
```

## Event Trigger Table

| Moment                           | Event                      |
| -------------------------------- | -------------------------- |
| First Field Deck raise           | FieldDeckRaised            |
| First meaningful lock inspection | DeadAuthorityLockInspected |
| Each major path preview          | RepairPathPreviewed        |
| Player commits repair            | RepairPathCommitted        |
| Outcome applied                  | WaterworksOutcomeRecorded  |
| Faction-citable history created  | ChroniclePrecedentCreated  |

## Outcome Flags

```text id="9qfsxd"
old_waterworks_repaired_legitimately
old_waterworks_bypassed_illegally
old_waterworks_machine_testimony_used
old_waterworks_stabilized_temporarily
dead_authority_overturned
dead_authority_remained_in_command
null_reinforcement_continues
null_loop_isolated
archive_witness_respected
archive_witness_bypassed
continuance_precedent_created
open_valve_precedent_created
```

---

# Asset Anchor List

## Required Anchors

```text id="yuf63c"
entry_door
public_charter_plaque
pump_main
tank_main
console_main
override_seal
hidden_relay
pipe_route_primary
```

## Scar Anchors

```text id="rac28o"
scar_wall_left
scar_console_back
scar_override_seal
scar_pump_body
scar_tank_side
scar_floor_route
scar_door_frame
scar_null_glyph_progression
```

## Lighting Anchors

```text id="efnm21"
light_ambient_amber
light_console_glow
light_overhead_work
light_override_warning
light_null_trace
```

## Audio Anchors

```text id="b98d0r"
audio_pump_hum
audio_pipe_creak
audio_water_drip
audio_console_beep
audio_null_loop_tick
```

---

# Art Direction

## Materials

```text id="amvtjh"
corroded metal
old municipal paint
faded public signage
rubberized cable
mineral stains
patched concrete
worn amber glass
```

## Shape Language

Municipal base layer:

```text id="rmijkm"
rounded industrial utility forms
painted labels
human-scale access panels
public symbols
```

Emergency layer:

```text id="6mw78l"
warning strips
seal plates
blocky overlays
authority labels
```

Corporate layer:

```text id="ikxuog"
cleaner plaques
sealed firmware ports
sleeker geometry inserted into older system
```

Null layer:

```text id="l7qchf"
precise wrong repetition
unmotivated symmetry
glyph-like residue
too-stable light intervals
```

## Color Notes

Use restraint.

The site should not become a rainbow UI wall.

Recommended emotional palette:

```text id="e6zppo"
dark concrete
old blue/green public works paint
rust
amber Field Deck light
small cold Null contrast
faded emergency yellow/black
```

---

# Accessibility Requirements

The room must support:

```text id="8lfl5f"
readable Field Deck text
no required high-frequency flicker
no audio-only warnings
Panic Drop always available
visor-assist / sym-glide stabilization
high-contrast mode support
interaction prompts with clear text
```

Null progression must not rely on color alone.

Use:

```text id="7xx18h"
shape
position
text
icon
mode readout
sound with visual equivalent
```

---

# Implementation Tickets

## OW-0 — Hygiene Gate

Run or document:

```text id="xbvu2h"
cargo check -p symtropy-bevy-core --example old_waterworks_micro_slice
```

Rules:

```text id="ev9gxe"
do not use git add .
do not use git commit --no-verify
do not stage unrelated files
do not touch sibling workspaces
use rg for search
```

## OW-1 — Room Object Audit

Ensure the example contains:

```text id="zpn6js"
pump
tank
console
entry door / room boundary
override seal or placeholder
```

## OW-2 — Field Deck Readings

Add static readings for:

```text id="08pvg8"
SCAN
DIAG
ARCHIVE
CIVIC
NULL
```

## OW-3 — Origin Notes

Add origin-specific note display:

```text id="gma1rq"
Basin-Born Technician
Archive Apprentice
Corporate Utility Defector
Continuance Credential Holder ghost option
```

## OW-4 — Repair Path Preview

Add repair path preview text for:

```text id="558j8v"
Archive Witness Override
Manual Illegal Bypass
Machine Testimony Petition
Temporary Emergency Stabilization
```

## OW-5 — Null Duration

Add NULL mode loop duration:

```text id="fp04bc"
55 YEARS, 3 MONTHS, 12 DAYS.
```

## OW-6 — Null Glyph Stage Stub

Add a simple visual or debug marker for:

```text id="v5x0l8"
scar_null_glyph_stage_1
```

Optional later:

```text id="i5q18l"
stage_2 after Temporary Emergency Stabilization
```

## OW-7 — Chronicle Outcome

Write or display one outcome text after chosen repair path.

## OW-8 — Tactical Net Stub

Add one route:

```text id="3o6zma"
pump console → hidden relay → pump housing
```

## OW-9 — Audio/Lighting Polish

Add restrained ambient support:

```text id="fru9ks"
amber ambient
console glow
subtle oscillating light
low pump hum
water drip
```

---

# Acceptance Criteria

The Old Waterworks level is successful when:

```text id="oam3n3"
the room is readable without lore dump
the pump feels repairable but contested
the player sees water scarcity
the Field Deck changes interpretation by mode
origin notes alter attention
repair paths have visible tradeoffs
Temporary Stabilization is visibly unresolved
Null feels like long-duration dead procedure
the outcome text reads like public history
the player understands the first enemy is a system
```

## Player Takeaway

The player should leave thinking:

```text id="e5bsev"
I did not just fix a pump.
I changed what this settlement believes repair means.
```

## Final Principle

```text id="1ha4vb"
The first pump teaches the whole game.
```

And:

```text id="3cp4t7"
The pump doesn't care who fixes it.
The settlement does.
```
