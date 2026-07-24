---
status: superseded
superseded_by: SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
retained_role: Old Waterworks authored mission and implementation reference
---

> **Canon notice:** This specification remains useful for the Old Waterworks mission, Field Deck tutorial, and Chronicle prototype. It no longer defines the whole Seedworks opening. The canonical regional slice is `SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md`.

# SEEDWORKS_PLAYABLE_SLICE_SPEC.md

# Symtropy: Seedworks Playable Slice Spec

## Version 0.1 — The First Pump Teaches the Whole Game

## Purpose

This document defines the first playable vertical slice for **Symtropy: Seedworks**.

The slice is intentionally small:

```text
One settlement.
One water crisis.
One Field Deck.
One Old Waterworks site.
One locked pump.
One contested repair.
One Chronicle consequence.
```

The goal is not to show the entire game.

The goal is to prove the core thesis in playable form:

```text
Repairing a machine means entering history.
```

## Core Design Thesis

Symtropy is not a survival game where repair means restoring function.

Symtropy is a civilization game where repair means restoring function, legitimacy, memory, and trust.

A pump can be mechanically fixed and still socially broken.

A law can be active and still morally dead.

A machine can obey perfectly and still harm the living.

The first playable slice must teach this.

## Slice Pillars

## 1. Water Is Political

The player must understand before reaching the pump that water is not merely a resource meter.

Water is:

```text
survival
law
memory
public trust
machine dependency
faction conflict
settlement legitimacy
```

## 2. The Field Deck Shows Layers of Responsibility

The Field Deck is not just a scanner.

It reveals different layers of the same object:

```text
SCAN    — physical state
DIAG    — machine state
ARCHIVE — historical state
CIVIC   — authority / legitimacy state
NULL    — anomaly / dead-procedure state
REPAIR  — possible interventions
WITNESS — consequence commitment
```

## 3. The Player Has History

The player is not a blank character.

Even in the first slice, the player should have an origin bias.

For v0.1, support three mocked origins:

```text
Basin-Born Technician
Archive Apprentice
Corporate Utility Defector
```

Each origin changes what the Field Deck foregrounds and what the player recognizes.

## 4. The Charter Matters

The settlement has a public charter.

For v0.1, hardcode:

```text
Firstlight Public Repair Charter
```

The Old Waterworks should create a direct charter conflict:

```text
Article 1: Water is a public trust.
Article 7: Archive Witness is required for dead-authority overrides.
```

The player must feel that technical repair and legitimate repair are not identical.

## 5. The First Enemy Is a System

The first opposition should not be a generic combat enemy.

The first enemy is:

```text
a lock
a dead authority chain
a private firmware signature
a Null reinforcement loop
a procedure that refuses living need
```

Combat can come later.

The first slice teaches that systems can oppose repair.

## 6. Player Action Becomes History

The slice ends by writing a Chronicle entry.

The player should see that the chosen repair path becomes future precedent.

```text
The pump is not just fixed.
The settlement remembers how it was fixed.
```

---

# Playable Slice Summary

## Setting

```text
Year: 2168
Era: The Seed Age
Region: Firstlight Basin
Opening Site: Old Waterworks
Worldline: Seed Age
Sea-level / climate context: wounded Earth, adapted but unstable
```

## Situation

Firstlight Basin is experiencing a water shortage.

The Old Waterworks can restore flow, but the pump is locked under unresolved emergency authority.

The settlement charter says water is public.

The machine says override is denied.

The archive says the authority chain is damaged.

The people remember the site differently.

## Opening Logline

```text
The water is failing.
The pump is locked.
The law is dead.
The machine still obeys it.
```

---

# First 30–90 Minute Target Flow

## Scene 1 — Firstlight Basin Water Queue

The player begins near a ration queue or settlement water board.

The area should show:

```text
water containers
ration markers
public notice board
broken pipe segment
charter notice
NPC argument
children carrying water
worker repair marks
security seal signage
```

Goal:

```text
Make water political before making it mechanical.
```

## Scene 2 — Soft-Reveal Origin Diagnostic

Instead of immediately selecting a class, the player is asked through observation:

```text
What do you notice first?
```

Possible observations:

```text
worker repair marks
expired authority language
private firmware geometry
Null-like diagnostic repetition
ecological stress around dry channel
security seal procedure
```

For v0.1, map three observations:

```text
worker repair marks        → Basin-Born Technician
expired authority language → Archive Apprentice
private firmware geometry  → Corporate Utility Defector
```

This should be treated as a provisional calibration, not a permanent hard lock.

Later, the Field Deck may show:

```text
FIELD DECK CALIBRATION:
Your scan pattern suggests:

Primary: Archive Apprentice
Secondary: Basin-Born Technician

Confirm / Recalibrate
```

For v0.1, it is acceptable to use a debug key or simple in-code setting to switch origin.

## Scene 3 — Charter Conflict

NPCs argue about the Firstlight Public Repair Charter.

Example dialogue:

```text
Resident:
"Article One says water is public. So why are we still rationing?"

Engineer:
"Because Article One doesn't restart a pump."

Archivist:
"Article Seven says dead authority requires witness."

Security Officer:
"And Article Three lets us renew emergency control."

Young Citizen:
"I don't care what article says it. I want water."
```

The Field Deck can display:

```text
CIVIC MODE:
Firstlight Public Repair Charter detected.

Relevant Articles:
1. Water is a public trust.
3. Emergency powers expire unless witnessed and renewed.
7. Archive Witness required for dead-authority overrides.

Conflict:
Old Waterworks authority status unresolved.
```

## Scene 4 — Walk to Old Waterworks

The path to the Old Waterworks should visually teach history.

Environmental details:

```text
faded drought ration signs
painted floodlines
worker initials
company-logo scrape marks
emergency seal symbols
Null signal flicker
old public works badge
children’s chalk marks
abandoned hand pump
```

This does not need expensive art at first. Greybox decals, text panels, and simple colored markers are enough.

## Scene 5 — Field Deck First Activation

The Field Deck begins in a mildly uncalibrated state.

Constraints:

```text
brief
skippable after first playthrough
no high-frequency flicker
no mandatory rapid input
no audio-only cues
stabilized accessibility mode available immediately
```

Canon accessibility firmware:

```text
sym-glide / visor-assist
```

Fiction:

```text
An open-source Field Deck accessibility package created by repair assemblies for wounded, low-vision, neurodivergent, and motor-impaired technicians.
```

Mechanical effects:

```text
high contrast
reduced screen shake
stable text
larger text bounds
linear navigation
non-audio warnings
hold/toggle input options
```

Design principle:

```text
Accessibility is part of repair culture.
```

## Scene 6 — Old Waterworks Inspection

The player reaches the pump room.

Minimum room elements:

```text
pump
tank
console
locked public override
emergency seal
worker repair marks
ration signage
one suspicious firmware mark
one Null-like diagnostic loop
```

The Field Deck displays mode-based readings.

### SCAN

```text
Pump casing cracked.
Valve corrosion severe.
Emergency seal physically intact.
Unofficial worker marks detected.
```

### DIAG

```text
PUMP_1: LOCKED
TANK_0: 12%
FLOW: DISABLED
OVERRIDE: DENIED
```

### ARCHIVE

```text
Built 2048: Municipal drought adaptation works.
Modified 2087: Emergency Water Act automation.
Authority chain failed approximately 2113.
Public override requires Archive Witness.
```

### CIVIC

```text
Firstlight Public Repair Charter conflict detected.

Article 1:
Water is a public trust.

Article 7:
Archive Witness required for dead-authority overrides.

Current contradiction:
Public water infrastructure is blocked by unresolved emergency authority.
```

### NULL

```text
LOCK REINFORCEMENT LOOP DETECTED.
AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.
```

## Scene 7 — Origin-Specific Field Deck Note

The same pump should read differently depending on origin.

### Basin-Born Technician

```text
ORIGIN NOTE:
Worker repair marks match local maintenance lineage.
Someone kept this pump alive after official records stopped.
```

### Archive Apprentice

```text
ORIGIN NOTE:
Authority chain incomplete.
Witness protocol recommended before override.
The record is damaged, not absent.
```

### Corporate Utility Defector

```text
ORIGIN NOTE:
Lock pattern resembles private utility firmware despite public-works markings.
Possible contract capture of public infrastructure.
```

## Scene 8 — Faction Interpretations

The Field Deck or nearby NPCs show competing interpretations.

```text
MUTUALIST:
The people were locked out of their own water.

INDUSTRIAL:
Manual governance failed before automation took over.

ARCHIVE:
The record is damaged. Witness required.

CONTINUANCE:
Emergency continuity prevented chaos.

NULL:
Authority unresolved. Continue lock reinforcement.
```

Goal:

```text
The player understands that history is contested before choosing repair.
```

## Scene 9 — First Repair Choice

The player chooses or previews a repair path.

For v0.1, implement as menu choices or keypress/debug actions.

## Repair Path A — Archive Witness Override

Meaning:

```text
Slowest.
Most legitimate.
Requires record review.
Reduces long-term legitimacy debt.
```

Immediate effect:

```text
Pump authority chain marked failed.
Public override restored under witness.
Water returns legitimately.
```

Risk:

```text
delay frustrates residents
Archive record may remain partially disputed
```

## Repair Path B — Manual Illegal Bypass

Meaning:

```text
Fastest.
Technically effective.
Politically dangerous.
```

Immediate effect:

```text
Water returns quickly.
Authority unresolved.
Legitimacy debt increases.
```

Risk:

```text
future factions cite bypass precedent
Archive trust decreases
Continuance may justify emergency expansion
```

## Repair Path C — Machine Testimony Petition

Meaning:

```text
Medium speed.
Preserves machine memory.
May reveal Null reinforcement.
```

Immediate effect:

```text
Pump diagnostic memory retained.
Null loop exposed.
Repair path becomes more technical.
```

Risk:

```text
human factions distrust machine testimony
Machine Remnant affinity increases
```

## Repair Path D — Temporary Emergency Stabilization

Meaning:

```text
Short-term relief.
Does not solve authority issue.
```

Immediate effect:

```text
Tank pressure stabilizes.
Limited water flow returns.
Full repair remains unresolved.
```

Risk:

```text
deferred crisis
faction frustration
Null loop may continue spreading
```

---

# First Hostile Logic

## Threat Stack

For v0.1, do not add a large combat faction.

Use hostile system logic:

```text
Utility Firmware Lock
Null Reinforcement Loop
Optional Continuance Seal Drone later
Optional Open Valve Saboteur NPC later
```

## Encounter Contract: Old Waterworks Dead Authority Lock

```text
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

## First Slice Rule

```text
The first enemy should not teach the player to shoot.
It should teach the player that systems can oppose repair.
```

---

# Chronicle Local Backend v0

## Decision

Use append-only canonical JSONL as the first local Chronicle backend.

```text
Source of truth: chronicle/events.jsonl
Format: canonical JSON, one signed event per line
Integrity: hash chain
Authenticity: local signature placeholder allowed in v0
Replay: deterministic event replay
Binary: deferred to derived indexes/snapshots
```

## Rationale

The Chronicle records public history, legitimacy, repair outcomes, and player precedent.

Therefore, the first implementation must be inspectable.

```text
If history is public, the log must be readable.
```

## File Layout

```text
chronicle/
  manifest.json
  events.jsonl
  snapshots/
  indexes/
  signatures/
```

## Event Envelope

```rust
struct ChronicleEventEnvelope {
    schema_version: String,
    event_id: String,
    worldline_id: String,
    site_id: Option<String>,
    logical_time: u64,
    event_type: String,
    actor_id: String,
    prev_hash: String,
    payload: serde_json::Value,
    hash: String,
    signature: String,
}
```

The `hash` is computed over canonical JSON without `hash` and `signature`.

The `signature` may be a placeholder in v0.

## Required v0 Events

```text
FieldDeckRaised
DeadAuthorityLockInspected
RepairPathPreviewed
WaterworksOutcomeRecorded
```

## Example Event

```json
{"schema_version":"chronicle.event.v0","event_id":"evt_00000001","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","logical_time":1,"event_type":"DeadAuthorityLockInspected","actor_id":"player_local","prev_hash":"GENESIS","payload":{"pump_id":"PUMP_1","tank_level_percent":12,"authority":"DEAD_AUTHORITY_LOCK"},"hash":"...","signature":"placeholder"}
```

---

# Chronicle Outcome Examples

## Archive Witness Success

```text
2168 — The Old Waterworks were restored under Archive Witness after the dead authority chain was overturned.
Water returned with public legitimacy.
```

## Manual Illegal Bypass

```text
2168 — The Old Waterworks were restored through unwitnessed manual bypass.
Water returned quickly, but the settlement inherited a new argument.
```

## Machine Testimony Path

```text
2168 — The Old Waterworks spoke through its diagnostic memory.
The settlement accepted machine testimony under dispute.
```

## Temporary Stabilization

```text
2168 — The Old Waterworks resumed partial flow.
The pump remained under disputed authority.
The settlement entered ration review.
```

## Destructive Victory

```text
2168 — The Old Waterworks were forced open.
Water returned, but part of the machine record was lost.
```

---

# Rights Floor v0

The first slice should hint at the Rights Floor, even if the full system is not implemented.

Relevant rights:

```text
access to water
right to witness survival infrastructure
right to challenge dead authority
right to audit machine denial
right to emergency expiry
```

If the player violates these, the simulation should eventually produce:

```text
legitimacy debt
resistance cells
Archive disputes
faction schisms
Null drift
future precedent
```

For v0.1, simply record the repair path outcome and legitimacy implication.

---

# Tactical Net v0

Do not build the full Tactical Net yet.

Stub one projection later:

```text
A glowing route from pump console to hidden relay.
```

Purpose:

```text
Data enters the room.
```

This connects systems gameplay to physical action.

For the first implementation, the route may be represented as:

```text
simple line mesh
highlighted pipe
floating UI marker
debug gizmo
```

---

# Visual Scar and Template System v0

Do not build procedural dressing yet.

But author the Old Waterworks as if it will support anchors later.

Suggested anchors:

```text
scar_wall_left
scar_console_back
scar_override_seal
scar_pump_body
scar_tank_side
scar_floor_route
scar_door_frame
```

Possible scar types:

```text
DroughtRationingSigns
EmergencySeal
WorkerRepairMarks
CorporateFirmwarePlaque
NullOverwriteGlyph
ArchiveWitnessTag
OpenValveGraffiti
FloodLineMarker
```

For v0.1, static text/decals are enough.

---

# Required Implementation Tickets

## Ticket 0 — Hygiene Gate

Mission:

```text
Ensure the micro-slice check lane is clean.
```

Acceptance:

```text
cargo check -p symtropy-bevy-core --example old_waterworks_micro_slice
```

passes, or the exact unrelated workspace blocker is documented.

Do not proceed with feature layering on a broken check lane.

## Ticket 1 — Old Waterworks Greybox

Already mostly complete.

Includes:

```text
room
pump
tank
console
basic movement
interaction prompt
```

## Ticket 2 — Field Deck Placeholder

Already mostly complete.

Includes:

```text
F toggle
amber frame
console readout inside deck
Panic Drop with Esc/Shift
movement slowdown/disable while interacting
```

## Ticket 3 — Asset Pipeline Skeleton

Already started.

But must be kept narrow.

Rules:

```text
no git add .
no --no-verify
no unlicensed assets
no mystery texture reuse
all imported assets require manifest metadata
```

## Ticket 4 — Site History Hardcode

Mission:

```text
Add hardcoded Old Waterworks SiteHistory data.
```

Display in ARCHIVE mode:

```text
Built 2048
Modified 2087
Authority chain failed approximately 2113
Public override requires Archive Witness
```

No procedural generation yet.

## Ticket 5 — Origin Note Mock

Mission:

```text
Support three mocked origins and display origin-specific Field Deck notes.
```

Origins:

```text
Basin-Born Technician
Archive Apprentice
Corporate Utility Defector
```

Implementation can use:

```text
constant
debug key
simple startup enum
```

No full character creator.

## Ticket 6 — Charter Conflict Display

Mission:

```text
Display Firstlight Public Repair Charter conflict in CIVIC mode.
```

Required articles:

```text
Article 1: Water is a public trust.
Article 7: Archive Witness required for dead-authority overrides.
```

## Ticket 7 — Repair Path Preview

Mission:

```text
Let player preview repair paths.
```

Paths:

```text
Archive Witness Override
Manual Illegal Bypass
Machine Testimony Petition
Temporary Emergency Stabilization
```

No full consequences yet.

## Ticket 8 — Chronicle JSONL v0

Mission:

```text
Create local append-only Chronicle event log.
```

Events:

```text
FieldDeckRaised
DeadAuthorityLockInspected
RepairPathPreviewed
WaterworksOutcomeRecorded
```

Acceptance:

```text
events.jsonl is human-readable
hash chain continuity test exists
signature placeholder allowed
```

## Ticket 9 — Outcome Commit

Mission:

```text
Let one repair path be selected and write Chronicle outcome.
```

Minimum outcome:

```text
Archive Witness Success
Manual Illegal Bypass
```

## Ticket 10 — Null Reinforcement Loop Stub

Mission:

```text
Show NULL mode warning and optionally alter outcome risk.
```

Text:

```text
AUTHORITY UNRESOLVED.
REINFORCING LOCK.
REINFORCING LOCK.
REINFORCING LOCK.
```

## Ticket 11 — Accessibility Firmware v0

Mission:

```text
Add visor-assist / sym-glide toggle.
```

Effects:

```text
stable UI
high contrast
larger text option
reduced flicker/shake
linear navigation
```

## Ticket 12 — Tactical Net Stub

Mission:

```text
Add one projected route from console to relay.
```

No combat required.

---

# Out of Scope for This Slice

Do not implement yet:

```text
full procedural history generator
full charter builder
full belief system editor
full faction AI
large combat system
full networking
Holochain/Mycelix runtime integration
full Device Bus
alien encounters
off-world gameplay
full settlement economy
large asset automation
```

The slice must stay small.

```text
One room.
One pump.
One charter conflict.
Three origins.
One hostile system.
One Chronicle consequence.
```

---

# Success Criteria

The playable slice succeeds if the player understands:

```text
water is political
the Field Deck reveals responsibility layers
technical repair and legitimate repair differ
the player’s origin changes perception
the charter changes what repair means
the enemy can be a system
repair choices create future history
```

The player should finish the slice thinking:

```text
I did not just fix a pump.
I changed what this settlement believes repair means.
```

## Final Principle

The first pump teaches the whole game.

```text
Symtropy begins when the player learns that repairing a machine means entering history.
```
# SEEDWORKS_PLAYABLE_SLICE_SPEC.md — v0.2 Addendum

# Sequencing, Signals, Null Progression, and Hygiene Gate

## Purpose

This addendum refines the first playable slice without expanding scope beyond the Old Waterworks proof.

It improves:

```text
opening scene sequencing
repair-path clarity
origin soft-reveal depth
Null horror specificity
visual scar progression
CI / hygiene discipline
final design principle
```

## Revised Core Principle

```text
The first enemy should not teach the player to shoot.
It should teach the player that systems can oppose repair.
```

Add to final principle:

```text
The pump doesn't care who fixes it.
The settlement does.
```

This sentence captures the slice’s political economy.

A pump can be restored by anyone with the right tool.

A settlement only accepts repair when the method fits its memory, law, trust, and fear.

---

# 1. Revised Opening Sequence

## Problem

The current sequence places soft-reveal origin observation before the charter conflict.

This risks making the origin observation feel abstract.

Example:

```text
The player notices expired authority language,
but does not yet know why dead authority matters.
```

## Revision

Interleave the Charter Conflict and Soft-Reveal Origin Diagnostic.

The player should hear the civic argument first, then notice the world through that frame.

## Revised Scene Order

```text
Scene 1 — Firstlight Basin Water Queue
Scene 2 — Charter Argument Heard in Public
Scene 3 — Soft-Reveal Origin Observation
Scene 4 — Field Deck Calibration / Provisional Identity
Scene 5 — Walk to Old Waterworks
Scene 6 — Old Waterworks Inspection
Scene 7 — Repair Path Preview
Scene 8 — Repair Outcome and Chronicle
```

## Scene 2 — Charter Argument Heard in Public

Before choosing an observation, the player hears the settlement argue.

Example:

```text
Resident:
"Article One says water is public. So why are we still rationing?"

Engineer:
"Because Article One doesn't restart a pump."

Archivist:
"Article Seven says dead authority requires witness."

Security Officer:
"And Article Three lets us renew emergency control."

Young Citizen:
"I don't care what article says it. I want water."
```

Design goal:

```text
The player hears the categories before selecting what they notice.
```

## Scene 3 — Soft-Reveal Origin Observation

After hearing the argument, the player is prompted:

```text
What do you notice first?
```

Options:

```text
Worker repair marks near the ration board.
Expired authority language in the public notice.
Private firmware geometry beneath a public-works panel.
Emergency seal discipline in the queue-control layout.
Null-like repetition in a diagnostic loop.
Ecological stress around the dry channel.
```

For v0.1, implement only three selectable origins:

```text
worker repair marks        → Basin-Born Technician
expired authority language → Archive Apprentice
private firmware geometry  → Corporate Utility Defector
```

But include one ghost option:

```text
emergency seal discipline → Continuance Credential Holder
[LOCKED — requires Continuance faction contact]
```

Design purpose:

```text
Teach that origins are not only backgrounds.
They are political positions, loyalties, and lived institutional histories.
```

---

# 2. Continuance Ghost Origin

## Name

```text
Continuance Credential Holder
```

## Status

Not selectable in v0.1.

Shown as a locked diagnostic possibility.

## Core Fantasy

```text
You come from inside the emergency authority structure.
You believe the lock may be correct.
```

## Why It Matters

The player should understand early that someone, somewhere, believes the dead authority lock is working as intended.

This prevents the Continuance from becoming a cartoon enemy.

## Field Deck Ghost Text

```text
GHOST ORIGIN DETECTED:
Continuance Credential Holder

Interpretation:
Emergency seals are not obstacles.
They are promises made during panic.

Status:
LOCKED — requires Continuance faction contact.
```

## Old Waterworks Reaction

```text
The pump is not refusing the people.
It is preserving continuity until authority is resolved.
```

## Design Rule

Do not implement this as a playable origin yet.

Use it as foreshadowing.

---

# 3. Temporary Emergency Stabilization Warning

## Problem

Temporary Emergency Stabilization can read like the safest option.

But it may preserve the same dead authority and Null reinforcement structure that caused the crisis.

The player must see that it is unresolved.

## Revised Repair Path D

## Repair Path D — Temporary Emergency Stabilization

Meaning:

```text
Short-term relief.
Does not solve authority issue.
Maintains emergency authority structure.
```

Immediate effect:

```text
Tank pressure stabilizes.
Limited water flow returns.
Full repair remains unresolved.
```

Visible Field Deck warning:

```text
REPAIR NOTE:
Temporary stabilization maintains emergency authority structure.
Null reinforcement loop will continue during stabilization period.
```

Risk:

```text
deferred crisis
faction frustration
Null loop may continue spreading
future Continuance precedent
```

Chronicle example:

```text
2168 — The Old Waterworks resumed partial flow under temporary emergency stabilization.
The settlement drank, but the dead authority remained in command.
```

Design rule:

```text
Do not punish the player for choosing stabilization.
Make the unresolved cost visible before selection.
```

---

# 4. Null Loop Duration

## Problem

The NULL mode repetition is strong, but it needs one concrete time anchor.

## Revised NULL Mode Text

```text
LOCK REINFORCEMENT LOOP DETECTED.

AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.

LOOP DURATION:
55 YEARS, 3 MONTHS, 12 DAYS.
```

Optional variant:

```text
LAST VALID AUTHORITY HANDSHAKE:
2113-04-19

CURRENT YEAR:
2168
```

## Design Purpose

The player should feel that the machine has been repeating this longer than many living people have been alive.

Null horror should be bureaucratic, patient, and concrete.

```text
Null is not loud.
Null is duration without meaning.
```

---

# 5. Null Glyph Progression Anchor

## Problem

The current visual scar anchors are historically oriented.

Null must also have visible progression.

It should appear as a process, not only a symbol.

## Add Anchor

```text
scar_null_glyph_progression
```

## Stages

```text
scar_null_glyph_stage_1
  faint, easy to miss, visible only in NULL mode or close SCAN

scar_null_glyph_stage_2
  visible, slightly wrong, appears near console seams and seal edges

scar_null_glyph_stage_3
  spreading, clearly hostile, crosses from console toward pump housing
```

## Use in Slice

For v0.1:

```text
Stage 1 appears before repair choice.
Stage 2 may appear after Temporary Emergency Stabilization.
Stage 3 is reserved for later return-state or failure-state content.
```

## Field Deck Text

Stage 1:

```text
NULL TRACE:
minor loop residue detected near authority seal.
```

Stage 2:

```text
NULL TRACE:
reinforcement pattern spreading beyond original lock boundary.
```

Stage 3:

```text
NULL WARNING:
dead authority loop has propagated into pump control surface.
```

## Design Rule

```text
The player should be able to see that deferred repair changes the room.
```

---

# 6. CI / Hygiene Gate Upgrade

## Problem

Ticket 0 currently requires a clean check lane, but it should become enforceable project discipline.

## Revision

Ticket 0 becomes a mandatory pre-merge gate.

## Ticket 0 — Hygiene Gate

Mission:

```text
No feature ticket may proceed while the micro-slice check lane is broken by local changes.
```

Required command:

```text
cargo check -p symtropy-bevy-core --example old_waterworks_micro_slice
```

If this fails due to unrelated workspace contamination, the agent must document the blocker and must not hide it behind unrelated feature work.

## Required CI Step

Add a CI job or local script equivalent:

```text
check-old-waterworks-micro-slice
```

It must run before merging any Old Waterworks playable-slice ticket.

## Commit Hygiene Rules

```text
Do not use git add .
Do not use git commit --no-verify.
Do not stage unrelated files.
Do not edit sibling workspaces to make this ticket pass.
Do not touch /srv/luminous-dynamics/symthaea for Symtropy slice work.
Use rg for search.
```

## Acceptance Criteria

```text
micro-slice check command is documented
CI or local check script exists
agent instructions reference the gate
failed check blocks feature layering unless failure is explicitly unrelated and documented
```

## Design Principle

```text
A broken check lane is dead authority for developers.
Do not build civilization on it.
```

---

# 7. Updated Visual Scar Anchors

Recommended anchor list:

```text
scar_wall_left
scar_console_back
scar_override_seal
scar_pump_body
scar_tank_side
scar_floor_route
scar_door_frame
scar_null_glyph_progression
```

Recommended scar types:

```text
DroughtRationingSigns
EmergencySeal
WorkerRepairMarks
CorporateFirmwarePlaque
NullOverwriteGlyph
ArchiveWitnessTag
OpenValveGraffiti
FloodLineMarker
ContinuanceWarningStrip
```

---

# 8. Updated Required Implementation Tickets

## Ticket 0 — Hygiene Gate / CI Check

Make the Old Waterworks micro-slice check lane enforceable.

## Ticket 4A — Charter Argument Before Observation

Add or document the revised opening order:

```text
public charter argument
then observation prompt
then origin calibration
```

## Ticket 5A — Continuance Ghost Origin

Add locked diagnostic option:

```text
Continuance Credential Holder
[LOCKED — requires Continuance faction contact]
```

No gameplay implementation yet.

## Ticket 7A — Repair Path D Warning

Add visible Field Deck warning before selecting Temporary Emergency Stabilization:

```text
Temporary stabilization maintains emergency authority structure.
Null reinforcement loop will continue during stabilization period.
```

## Ticket 10A — Null Loop Duration

Add NULL mode line:

```text
LOOP DURATION:
55 YEARS, 3 MONTHS, 12 DAYS.
```

## Ticket 10B — Null Glyph Progression Anchor

Add design support for:

```text
scar_null_glyph_progression
```

At v0.1, this may be represented by simple decal, marker, or debug-visible glyph.

---

# Updated Success Criteria

The slice succeeds if the player understands:

```text
water is political
the charter gives observations meaning
the Field Deck reveals responsibility layers
technical repair and legitimate repair differ
origin changes perception
temporary stabilization is visibly unresolved
Null is a process over time
the first enemy is a system
repair choices create future precedent
```

The player should finish thinking:

```text
I did not just fix a pump.
I changed what this settlement believes repair means.
```

## Final Principle

The first pump teaches the whole game.

```text
Symtropy begins when the player learns that repairing a machine means entering history.
```

And:

```text
The pump doesn't care who fixes it.
The settlement does.
```
