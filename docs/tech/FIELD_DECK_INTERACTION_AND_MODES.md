# FIELD_DECK_INTERACTION_AND_MODES.md

# Symtropy Field Deck Interaction and Modes

## Version 0.1 — The Player’s Instrument for Reading Responsibility

## Purpose

This document defines the Field Deck as the central player interface for **Symtropy: Seedworks**.

The Field Deck is not a generic scanner, phone, menu, codex, or hacking device.

It is the player’s instrument for reading the layered truth of a system.

It reveals:

```text
physical condition
machine state
historical record
legal authority
civic legitimacy
Null contamination
repair paths
witness commitments
tactical topology
accessibility firmware
```

The Field Deck makes Symtropy playable because it turns philosophy into verbs.

## Core Thesis

The Field Deck does not show “the answer.”

It shows layers of responsibility.

```text
SCAN shows what is physically present.
DIAG shows what the machine thinks is true.
ARCHIVE shows what history remembers.
CIVIC shows who has authority.
NULL shows where procedure has survived purpose.
REPAIR shows possible interventions.
WITNESS turns action into history.
TACTICAL NET brings data into the room.
```

## Design Principle

```text
The player’s first verb is reading.
The second verb is repair.
The third verb is consequence.
```

## What the Field Deck Is

In fiction, the Field Deck is a rugged civic-repair instrument used by technicians, archivists, settlement workers, machine stewards, ecologists, and emergency responders.

It combines:

```text
diagnostic reader
public works tablet
archive witness instrument
repair planner
civic legality assistant
machine testimony recorder
tactical projection tool
accessibility visor firmware host
```

It is not military by default.

It is infrastructure-native.

## What the Field Deck Is Not

The Field Deck should not become:

```text
a magic truth detector
a universal hacking wand
a quest marker machine
a minimap replacement only
a menu wrapper for lore dumps
a combat HUD pretending to be civic technology
```

It should reveal enough to act, but not enough to remove judgment.

## Core Interaction Loop

```text
1. Raise Field Deck
2. Select mode
3. Inspect target
4. Read layered interpretation
5. Preview possible action
6. Commit or withdraw
7. Chronicle records consequence if action matters
```

## Field Deck States

```rust
enum FieldDeckState {
    Stowed,
    Raised,
    Calibrating,
    ModeSelecting,
    Inspecting,
    RepairPreview,
    WitnessCommit,
    TacticalProjection,
    PanicDrop,
}
```

## Stowed

The player moves normally.

Field Deck UI hidden.

## Raised

The deck appears.

Movement slows.

Nearby inspectable systems highlight subtly.

## Calibrating

Used early in the game.

Field Deck starts mildly unstable and is stabilized through a simple interaction.

This must be brief, accessible, and skippable after first completion.

## ModeSelecting

The player chooses a mode.

For v0.1, modes may be cycled with simple keys.

## Inspecting

The player points at or selects an object.

The chosen mode displays reading.

## RepairPreview

The player sees available repair paths and likely consequences.

## WitnessCommit

The player commits an action that may become Chronicle history.

## TacticalProjection

The Field Deck projects spatial data into the physical room.

## PanicDrop

Emergency exit from Field Deck view.

Required inputs:

```text
Esc
Shift
controller cancel
accessibility equivalent
```

Panic Drop should always return control quickly.

---

# Field Deck Modes

## Mode List

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL
REPAIR
WITNESS
TACTICAL NET
ACCESSIBILITY / visor-assist
```

For the first playable slice, implement:

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL
REPAIR preview
basic WITNESS outcome
visor-assist toggle
```

TACTICAL NET can be a later stub.

---

# 1. SCAN Mode

## Purpose

SCAN shows physical reality.

It answers:

```text
What is this object?
What is visibly damaged?
What materials are present?
What environmental clues exist?
What scars are visible?
```

## Tone

Concrete, sensory, grounded.

## Old Waterworks Example

```text
SCAN:
Pump casing cracked.
Valve corrosion severe.
Emergency seal physically intact.
Unofficial worker marks detected.
Dry mineral deposits along lower pipe seam.
```

## SCAN Should Reveal

```text
damage
wear
age
water traces
heat traces
rust
biological growth
repair marks
physical locks
visible glyphs
environmental hazards
```

## SCAN Should Not Reveal

```text
legal meaning
hidden motives
full history
machine intention
faction legitimacy
Null cause
```

SCAN can see the scar.

It cannot fully explain it.

## Origin Bias Examples

### Basin-Born Technician

```text
Worker mark pattern matches local maintenance lineage.
```

### Ritual Ecologist

```text
Dry channel ecology suggests repeated partial flow, not full restoration.
```

### Worker-Guild Mechanic

```text
Valve housing shows manual override attempts by at least two tool traditions.
```

---

# 2. DIAG Mode

## Purpose

DIAG shows machine state.

It answers:

```text
What does the system report?
What is locked?
What is powered?
What is failing?
What does the device believe its status is?
```

## Tone

Machine-readable, terse, procedural.

## Old Waterworks Example

```text
DIAG:
PUMP_1: LOCKED
TANK_0: 12%
FLOW: DISABLED
OVERRIDE: DENIED
AUTHORITY: DEAD_AUTHORITY_LOCK
```

## DIAG Should Reveal

```text
status codes
power levels
lock states
sensor readings
diagnostic errors
device IDs
network paths
firmware warnings
machine testimony hooks
```

## DIAG Should Not Reveal

```text
whether the machine is morally right
whether the law is legitimate
whether records are complete
whether people will accept the repair
```

DIAG can say:

```text
OVERRIDE DENIED
```

It cannot decide whether denial is just.

## DIAG Failure Pattern

DIAG can lie when:

```text
sensor spoofing occurs
Null loop corrupts status
firmware is private or hostile
machine memory is damaged
authority state is unresolved
```

Example:

```text
DIAG:
SYSTEM SAFE.

NULL MODE:
false green status suspected.
```

---

# 3. ARCHIVE Mode

## Purpose

ARCHIVE shows historical record.

It answers:

```text
When was this built?
Who modified it?
What law or authority affected it?
What records are missing?
Who witnessed past repairs?
What events made this system what it is?
```

## Tone

Historical, evidentiary, incomplete.

## Old Waterworks Example

```text
ARCHIVE:
Built 2048: Municipal drought adaptation works.
Modified 2087: Emergency Water Act automation.
Authority chain failed approximately 2113.
Public override requires Archive Witness.
```

## ARCHIVE Should Reveal

```text
construction dates
modification history
public works records
emergency orders
ownership changes
witness tags
missing records
contradictory testimony
expired authority
```

## ARCHIVE Should Not Reveal

```text
perfect truth
private memory without access
machine testimony unless requested
NPC emotional truth
all oral histories
```

ARCHIVE is powerful, but not omniscient.

## Archive Confidence

Archive readings should include confidence where useful.

Example:

```text
ARCHIVE CONFIDENCE:
Built 2048 — high confidence.
Modified 2087 — high confidence.
Authority failure 2113 — medium confidence.
Witness record — damaged.
```

## Origin Bias Examples

### Archive Apprentice

```text
Authority chain incomplete.
Witness protocol recommended before override.
The record is damaged, not absent.
```

### Corporate Utility Defector

```text
Public record does not explain private firmware signature.
Likely undocumented operator change.
```

---

# 4. CIVIC Mode

## Purpose

CIVIC shows authority, law, charter conflict, legitimacy, and public consequence.

It answers:

```text
Who has the right to act?
Which charter articles apply?
What authority is disputed?
Which factions care?
What legitimacy debt might be created?
```

## Tone

Public, legal, civic, argumentative.

## Old Waterworks Example

```text
CIVIC:
Firstlight Public Repair Charter conflict detected.

Article 1:
Water is a public trust.

Article 7:
Archive Witness required for dead-authority overrides.

Current contradiction:
Public water infrastructure is blocked by unresolved emergency authority.
```

## CIVIC Should Reveal

```text
charter articles
emergency expiry status
public trust implications
rights floor warnings
faction interpretations
legitimacy risks
authority conflicts
public vote possibilities
```

## CIVIC Should Not Reveal

```text
the objectively best moral choice
whether a faction will forgive the player
all hidden political consequences
```

CIVIC should make politics visible, not solve politics for the player.

## Faction Interpretation Example

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

## Rights Floor Warnings

CIVIC should flag possible rights-floor issues.

Example:

```text
RIGHTS FLOOR WARNING:
Public water access obstructed by unresolved authority.
```

Example:

```text
RIGHTS FLOOR WARNING:
Temporary stabilization preserves emergency command structure.
```

---

# 5. NULL Mode

## Purpose

NULL shows where procedure has survived purpose.

It answers:

```text
What is repeating without meaning?
Where is authority unresolved?
Where is the system enforcing dead context?
Where is a loop expanding?
Where is the interface lying?
```

## Tone

Sparse, cold, patient, unsettling.

## Old Waterworks Example

```text
NULL:
LOCK REINFORCEMENT LOOP DETECTED.

AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.
AUTHORITY UNRESOLVED.

LOOP DURATION:
55 YEARS, 3 MONTHS, 12 DAYS.
```

## NULL Should Reveal

```text
dead procedure
loop duration
false green status
authority recursion
uninterruptible locks
Null glyph progression
repeated denial phrases
status contradictions
```

## NULL Should Not Reveal

```text
full repair path automatically
all hidden enemies
final truth of a system
```

NULL mode should create dread and responsibility.

It should not become a cheat mode.

## Null Design Principle

```text
Null is not loud.
Null is duration without meaning.
```

## Null Glyph Progression

NULL mode can expose visible stages:

```text
scar_null_glyph_stage_1
  faint, easy to miss

scar_null_glyph_stage_2
  visible, slightly wrong

scar_null_glyph_stage_3
  spreading, clearly hostile
```

Field Deck text:

```text
NULL TRACE:
minor loop residue detected near authority seal.
```

or:

```text
NULL WARNING:
dead authority loop has propagated into pump control surface.
```

---

# 6. REPAIR Mode

## Purpose

REPAIR shows possible interventions.

It answers:

```text
What can be done?
What will it restore?
What will remain unresolved?
What risks are visible?
What modes support this path?
```

## Tone

Practical, option-facing, consequence-aware.

## Old Waterworks Repair Paths

### Archive Witness Override

```text
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

### Manual Illegal Bypass

```text
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

### Machine Testimony Petition

```text
REPAIR PATH:
Machine Testimony Petition

Restores:
diagnostic memory continuity

Reveals:
possible Null reinforcement

Risk:
human factions may dispute machine testimony
```

### Temporary Emergency Stabilization

```text
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

## REPAIR Mode Rule

Do not hide the basic nature of the tradeoff.

The player may choose a dangerous path, but they should not be tricked into it by unclear wording.

```text
Unresolved costs should be visible.
Future consequences may remain uncertain.
```

---

# 7. WITNESS Mode

## Purpose

WITNESS turns consequential actions into recorded history.

It answers:

```text
Are you committing this action?
What will be recorded?
What precedent might this create?
Who may cite this later?
```

## Tone

Solemn, public, historical.

## Old Waterworks Example

```text
WITNESS COMMIT:
Archive Witness Override

This action will record:
- authority chain failure
- public override restoration
- water return under witness

Proceed?
```

## WITNESS Should Be Used When

```text
repair path changes public legitimacy
machine memory is altered
authority is overridden
water access changes
faction precedent is created
Chronicle event will be written
```

## WITNESS Should Not Be Used For

```text
every small interaction
ordinary scanning
minor movement
basic UI changes
```

WITNESS mode should feel meaningful, not bureaucratic spam.

## Chronicle Example

```text
2168 — The Old Waterworks were restored under Archive Witness after the dead authority chain was overturned.
Water returned with public legitimacy.
```

## Witness Design Principle

```text
The player should feel the weight of actions that become public history.
```

---

# 8. TACTICAL NET Mode

## Purpose

TACTICAL NET brings data into physical space.

It answers:

```text
Where does this system run?
What conduits matter?
Where are the hidden relays?
What nodes need defense?
Where is the Null path?
Where are civilians at risk?
```

## Tone

Spatial, urgent, shared.

## Design Principle

```text
Data should enter the room.
```

## Old Waterworks v0 Stub

Project one route:

```text
pump console → hidden relay → pump housing
```

The route can be represented by:

```text
simple line mesh
highlighted pipe
floating marker
debug gizmo
high-contrast projection
```

## Tactical Net Use Cases

```text
trace Null reinforcement path
show hidden conduit route
mark safe disable point
project drone patrol intent
highlight water-flow topology
show public hazard zone
```

## Co-op Role

The Systems Operator uses TACTICAL NET to help action-first players.

Example:

```text
The Systems Operator previews topology.
A glowing line appears in the room.
The repair technician follows the line to the relay.
The security player defends the junction.
The archive player preserves logs.
```

## Accessibility Requirement

TACTICAL NET must not rely on color alone.

Use:

```text
line shape
icons
text labels
contrast settings
pulse speed control
audio alternatives
haptic alternatives where available
```

---

# 9. ACCESSIBILITY / visor-assist Mode

## Purpose

Accessibility is canon.

The Field Deck supports in-world firmware for visual, motor, sensory, and cognitive accessibility.

## Firmware Names

```text
sym-glide
visor-assist
```

## Fiction

```text
An open-source Field Deck accessibility package created by repair assemblies for wounded, low-vision, neurodivergent, and motor-impaired technicians.
```

## Mechanical Effects

```text
stabilized UI
high contrast
reduced screen shake
reduced flicker
larger text bounds
linear navigation shortcuts
non-audio warnings
hold/toggle input alternatives
simplified rotary inputs
reduced timing pressure
```

## Design Principle

```text
Accessibility is part of repair culture.
```

## Requirements

The Field Deck must not require:

```text
audio-only warnings
high-frequency flicker
tiny amber text only
color-only status differences
mandatory rapid input
precision cursor movement only
unskippable glitch effects
```

## Implementation Rule

Atmospheric friction is allowed only when the player can stabilize it.

The fiction should support the player, not fight them.

---

# Field Deck Input Model

## Keyboard v0

Suggested defaults:

```text
F       raise/lower Field Deck
Tab     cycle mode
E       inspect / confirm
Q       back
Esc     Panic Drop
Shift   Panic Drop / hold to lower
1–8     direct mode select later
```

## Controller v0

Suggested defaults:

```text
Left trigger      raise Field Deck
D-pad / bumper    cycle mode
A / Cross         inspect / confirm
B / Circle        back / Panic Drop
Y / Triangle      mode details
```

## Accessibility Input Options

```text
toggle instead of hold
linear menu instead of radial
longer confirm window
no rapid repeated inputs
text log of warnings
large target selection
```

---

# Field Deck UI Layout

## v0 Layout

```text
Top:
  Mode name

Left:
  object / target ID

Center:
  main reading

Right:
  warnings / conflicts / confidence

Bottom:
  available actions
```

## Old Waterworks Example

```text
FIELD DECK MK0 — CIVIC

TARGET:
PUMP_1 / OLD WATERWORKS

READING:
Firstlight Public Repair Charter conflict detected.

RELEVANT ARTICLES:
1. Water is a public trust.
7. Archive Witness required for dead-authority overrides.

WARNING:
Public water infrastructure blocked by unresolved emergency authority.

ACTIONS:
Preview Repair Paths
Switch Mode
Panic Drop
```

## UI Tone

The Field Deck should feel:

```text
rugged
amber
legible
public-works oriented
repairable
not sleek military sci-fi
```

Avoid making it too polished.

It should feel like a trusted civic tool kept alive by maintenance culture.

---

# Field Deck Text Register

Field Deck writing should be:

```text
short
precise
layered
understated
sometimes chilling
never meme-like
rarely emotional
```

Good:

```text
Water returned quickly, but the settlement inherited a new argument.
```

Good:

```text
Authority unresolved. Loop duration: 55 years.
```

Bad:

```text
Uh oh! Looks like the evil AI is being spooky!
```

Bad:

```text
Congratulations! You fixed the pump and gained +10 legitimacy.
```

Metrics can exist, but the player-facing tone should stay historical and civic.

---

# Field Deck and Origins

Origins alter what the Field Deck foregrounds.

They should not give totally different facts.

They should bias attention.

## Basin-Born Technician

Foregrounds:

```text
worker marks
local maintenance lineage
physical repair history
family or guild memory
```

## Archive Apprentice

Foregrounds:

```text
authority chains
missing records
witness protocol
legal continuity
```

## Corporate Utility Defector

Foregrounds:

```text
private firmware
contract capture
service locks
billing logic
```

## Continuance Credential Holder

Ghost origin for v0.1.

Foregrounds:

```text
emergency seal discipline
continuity logic
risk-control structure
```

Status:

```text
LOCKED — requires Continuance faction contact.
```

---

# Field Deck and Chronicle

The Field Deck writes to Chronicle when action becomes public consequence.

## Required v0 Chronicle Events

```text
FieldDeckRaised
DeadAuthorityLockInspected
RepairPathPreviewed
WaterworksOutcomeRecorded
```

## Event Trigger Examples

### FieldDeckRaised

Triggered when the player raises the deck for the first time.

### DeadAuthorityLockInspected

Triggered when player inspects Old Waterworks lock in DIAG, ARCHIVE, CIVIC, or NULL mode.

### RepairPathPreviewed

Triggered when player previews any major repair path.

### WaterworksOutcomeRecorded

Triggered when player commits repair outcome.

## Event Rule

Do not log every tiny UI action.

Log moments that matter to history, replay, or precedent.

---

# Field Deck and Encounter Contracts

The Field Deck is how encounter contracts become legible.

It should show:

```text
warning line
protected value
escalation risk
de-escalation path
nonlethal option
combat-adjacent risk
Chronicle precedent warning
```

## Old Waterworks Encounter Display

```text
ENCOUNTER:
Dead Authority Lock

PROTECTED VALUE:
water continuity / emergency authority

WARNING:
Public override denied.
Emergency authority unresolved.

VISIBLE DE-ESCALATION:
Archive Witness Override
Machine Testimony Petition
Firmware Audit
Temporary Emergency Stabilization

ESCALATION RISK:
Manual seal break may damage record and increase legitimacy debt.
```

---

# First Playable Slice Requirements

For Old Waterworks v0.1, the Field Deck must support:

```text
raise/lower with F
Panic Drop
mode display
SCAN reading
DIAG reading
ARCHIVE reading
CIVIC reading
NULL reading
REPAIR path preview
basic WITNESS commit
origin-specific note
visor-assist toggle
```

Nice-to-have:

```text
mode cycling animation
simple calibration moment
one Tactical Net projection
Chronicle event writing
```

Out of scope:

```text
full hacking language
full Device Bus
full multiplayer Tactical Net
procedural history generation
real machine testimony AI
full accessibility settings screen
```

---

# Implementation Tickets

## Ticket FD-0 — Field Deck Hygiene Check

Ensure micro-slice check lane is clean before modifying Field Deck.

Required command:

```text
cargo check -p symtropy-bevy-core --example old_waterworks_micro_slice
```

Rules:

```text
do not use git add .
do not use git commit --no-verify
do not stage unrelated files
do not edit sibling workspaces
use rg for search
```

## Ticket FD-1 — Mode Enum and Static Mode Switching

Add Field Deck mode enum:

```rust
enum FieldDeckMode {
    Scan,
    Diag,
    Archive,
    Civic,
    Null,
    Repair,
    Witness,
}
```

Allow cycling modes while Field Deck is raised.

## Ticket FD-2 — Old Waterworks Mode Readings

Hardcode readings for:

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL
```

## Ticket FD-3 — Repair Path Preview

Add REPAIR mode list:

```text
Archive Witness Override
Manual Illegal Bypass
Machine Testimony Petition
Temporary Emergency Stabilization
```

Include visible warning for Temporary Emergency Stabilization.

## Ticket FD-4 — Witness Commit Stub

Add WITNESS mode confirmation for at least two paths:

```text
Archive Witness Override
Manual Illegal Bypass
```

Write or display outcome text.

## Ticket FD-5 — Origin-Specific Notes

Add three origin notes:

```text
Basin-Born Technician
Archive Apprentice
Corporate Utility Defector
```

Add Continuance Credential Holder as locked ghost option text only.

## Ticket FD-6 — NULL Loop Duration

Add NULL mode text:

```text
LOOP DURATION:
55 YEARS, 3 MONTHS, 12 DAYS.
```

## Ticket FD-7 — visor-assist Toggle

Add a simple accessibility/stability toggle.

Effects may be minimal at first:

```text
clearer text label
reduced animation
stable UI state flag
```

## Ticket FD-8 — Tactical Net Stub

Add one optional projected route:

```text
pump console → hidden relay
```

Can be a debug line or simple mesh.

---

# Acceptance Criteria

The Field Deck spec is successful in-game when:

```text
player can raise and lower it reliably
player can panic-drop instantly
each mode changes interpretation of the same pump
origin changes what is foregrounded
repair paths show visible tradeoffs
temporary stabilization is clearly unresolved
NULL mode makes duration concrete
accessibility stabilization is available
the player understands repair is layered
```

## Final Principle

The Field Deck is not there to tell the player what to do.

It is there to make responsibility visible.

```text
The pump doesn't care who fixes it.
The settlement does.
```
