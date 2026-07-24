---
title: Choked Valve Court Implementation Roadmap v0.1
status: build-roadmap
project: Symtropy
domain: Vertical Slice / Implementation Roadmap / Southern Africa / Field Deck / Chronicle
recommended_path: docs/earth-atlas/southern-africa/CHOKED_VALVE_COURT_IMPLEMENTATION_ROADMAP_V0_1.md
depends_on:
  - SOUTHERN_AFRICA_CHOKED_VALVE_COURT_VERTICAL_SLICE_V0_1.md
  - CHOKED_VALVE_COURT_GHOST_MINE_CONTINUITY_PATCH_V0_2.md
  - CHRONICLE_MVP_SPEC_V0_1.md
  - CHRONICLE_SCALE_ESCALATION_RULES_V0_1.md
  - FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
  - ORIGIN_BIAS_FIELD_DECK_SCHEMA_V0_1.md
  - ROAD_CHOIR_BRIDGE_CITIZEN_APPEAL_COURTS_V0_1.md
---

# Choked Valve Court Implementation Roadmap v0.1

## Working Title

**Build the Valve Before the World**

## Purpose

This roadmap converts **The Choked Valve Court** from design documentation into an implementation sequence.

The goal is not to build all of Symtropy at once.

The goal is to prove the smallest playable version of the civilization-scale loop:

```text
pressure creates crisis
crisis creates interpretation
interpretation creates authority
authority changes repair
repair creates memory
memory changes what can happen next
```

Acceptance target:

```text
The player does not just fix a valve.
The player changes who has the right to open it.
```

---

# 1. Vertical Slice Scope

## Included in v0.1 Prototype

```text
one valve court site
one lower cistern water crisis
one Field Deck readout
three factions
two repair paths
one evidence item
one Chronicle event
one follow-up consequence
```

## Deferred

```text
full Road Choir appeal system
full toxic legacy district simulation
full Ghost Mine tunnel combat
full worldline variants
full multiplayer truth reconciliation
full death/reconstitution integration
full xeno-contact systems
complex NPC schedules
large open-world traversal
```

Design rule:

```text
Prototype the loop, not the encyclopedia.
```

---

# 2. Minimum Playable Slice

## Required Player Experience

A player should be able to:

```text
arrive at the valve court
scan the water crisis
hear conflicting authority claims
choose a repair approach
perform a repair interaction
record a Chronicle event
see one future permission change
```

## Minimal Narrative

```text
The lower cistern is falling below safe level.
The Basin Court wants witnessed acoustic calibration.
A Hearth representative wants immediate water.
A Cold Perimeter officer wants emergency bypass.
The player chooses between witnessed repair and emergency bypass.
```

## Minimal Mechanical Choice

### Option A — Witnessed Calibration

```text
slower
requires witness
lower legitimacy debt
unlocks public repair precedent
```

### Option B — Emergency Bypass

```text
faster
saves time
creates legitimacy debt
unlocks follow-up pressure anomaly
```

---

# 3. Prototype Milestones

## Milestone 0 — Data Stubs

Goal:

```text
Define static data and IDs.
```

Implement:

```text
RegionId
SiteId
FactionId
FieldDeckMode
ChronicleEventClass
WaterState
AuthorityClaim
```

Acceptance:

```text
The site can load with named factions, pressure values, and one valve state.
```

---

## Milestone 1 — Water Pressure State

Goal:

```text
Make the valve crisis measurable.
```

Implement:

```rust
struct ValveCourtWaterState {
    main_pressure: f32,
    lower_cistern_level: f32,
    upper_reservoir_level: f32,
    mine_drain_rate: f32,
    leak_rate: f32,
    contamination_risk: f32,
    acoustic_gate_alignment: f32,
    emergency_bypass_integrity: f32,
}
```

Initial prototype values:

```yaml
main_pressure: 0.42
lower_cistern_level: 0.18
upper_reservoir_level: 0.64
mine_drain_rate: 0.31
leak_rate: 0.08
contamination_risk: 0.22
acoustic_gate_alignment: 0.42
emergency_bypass_integrity: 0.71
```

Acceptance:

```text
The Field Deck can show lower cistern level and pressure state.
Changing valve alignment changes pressure.
```

---

## Milestone 2 — Field Deck Core Readout

Goal:

```text
Expose the crisis through the Field Deck.
```

Implement raw readout:

```sh
$ read /dev/sym/water/court/main_pressure

PRESSURE_STATE: CHOKED_FLOW
LOWER_CISTERN_LEVEL: 18%
MINE_DRAIN_RATE: 0.31
ACOUSTIC_GATE_ALIGNMENT: 0.42
PUBLIC_AUTHORITY: BASIN_COURT_WITNESS_REQUIRED
```

Acceptance:

```text
The player can inspect the system and understand that it is mechanically and civically constrained.
```

---

## Milestone 3 — Authority Claims

Goal:

```text
Make repair contested.
```

Implement:

```rust
struct AuthorityClaim {
    faction_id: FactionId,
    claim_type: AuthorityClaimType,
    desired_action: DesiredAction,
    fear: String,
    trust_delta_on_success: f32,
    trust_delta_on_failure: f32,
}
```

Initial factions:

```text
Basin Court Steward
Hearth Pump Elder
Cold Perimeter Officer
```

Claims:

```yaml
Basin Court:
  desired_action: witnessed_calibration
  fear: emergency bypass becomes private rule

Hearth Pump Elder:
  desired_action: restore_water_before_evening
  fear: beautiful procedure leaves children thirsty

Cold Perimeter Officer:
  desired_action: command_bypass
  fear: the next crisis will not wait for witness
```

Acceptance:

```text
The player sees that no repair is socially neutral.
```

---

## Milestone 4 — Two Repair Paths

Goal:

```text
Create the first meaningful choice.
```

### Witnessed Calibration

Requirements:

```text
inspect acoustic ledger
obtain witness
calibrate valve
wait through slower pressure recovery
```

Effects:

```text
lower_cistern_level increases slowly
legitimacy_debt +0.05
Basin Court trust +0.12
Hearth urgency frustration +0.04
Cold Perimeter trust -0.05
```

### Emergency Bypass

Requirements:

```text
manual bypass interaction
optional warning confirmation
```

Effects:

```text
lower_cistern_level increases quickly
legitimacy_debt +0.25
Hearth trust +0.10
Cold Perimeter trust +0.08
Basin Court trust -0.14
follow-up pressure anomaly flagged
```

Acceptance:

```text
Both choices solve the immediate crisis.
Neither choice solves the same future.
```

---

## Milestone 5 — Chronicle MVP Event

Goal:

```text
Record the player's choice as public memory.
```

Implement minimal event:

```rust
struct ChronicleEventMvp {
    event_id: String,
    title: String,
    event_class: ChronicleEventClass,
    site_id: String,
    summary_public: String,
    evidence_refs: Vec<String>,
    legitimacy_delta: f32,
    faction_deltas: Vec<(FactionId, f32)>,
    open_questions: Vec<String>,
}
```

Possible events:

```text
The Valve Was Opened Under Witness
The Dry Night Bypass
```

Acceptance:

```text
After repair, the player can inspect the Chronicle event and see why trust changed.
```

---

## Milestone 6 — Field Deck Overlay Precedence

Goal:

```text
Show the same crisis through different local terms.
```

Implement minimal translation stack:

```text
RAW:
lower_cistern_level = 18%

BASIN COURT:
Emergency Clause Threshold

HEARTH:
Household Water Shortfall

COLD PERIMETER:
Civilian Cluster Vulnerability
```

Acceptance:

```text
The Field Deck displays raw data plus at least two cultural interpretations.
```

---

## Milestone 7 — Origin Bias Stub

Goal:

```text
Show origin-specific first warnings.
```

Implement three origins:

```text
Corporate Utility Defector
Refugee Charter Child
Worker-Guild Mechanic
```

Examples:

```text
Corporate Utility Defector:
  foregrounds mine_drain_rate and contract residue

Refugee Charter Child:
  foregrounds lower household risk and appeal routes

Worker-Guild Mechanic:
  foregrounds acoustic gate alignment and repair lineage
```

Acceptance:

```text
The same raw crisis produces different first-line interpretive notes by origin.
```

---

## Milestone 8 — Ghost Mine Persistence Stub

Goal:

```text
Make unresolved causes persist.
```

Implement:

```rust
enum GhostMineKnowledgeState {
    UnknownDrain,
    SuspectedLegacyDrain,
    ConfirmedDeadContract,
    PubliclyWitnessedDeadContract,
    DisabledOrReclaimed,
}
```

For v0.1:

```text
The Ghost Mine cannot be fully explored.
It contributes mine_drain_rate.
It can be suspected through Field Deck anomaly.
Emergency Bypass leaves it unresolved.
Witnessed Calibration may flag it as ongoing anomaly.
```

Acceptance:

```text
The player understands that fixing the valve does not necessarily solve the basin.
```

---

## Milestone 9 — Follow-Up Permission Change

Goal:

```text
Prove that Chronicle memory changes future access.
```

If witnessed repair:

```text
public_repair_precedent = true
future valve access requires fewer approvals
Basin Court allows apprentice calibration
```

If emergency bypass:

```text
legitimacy_debt = active
future valve access requires hearing
Cold Perimeter offers command override
Basin Court distrusts player
```

Acceptance:

```text
A later interaction changes because of the Chronicle event.
```

---

# 4. Implementation Data Model

## 4.1 Slice State

```rust
struct ChokedValveCourtSliceState {
    water: ValveCourtWaterState,
    authority_claims: Vec<AuthorityClaim>,
    player_origin_bias: Option<OriginBias>,
    field_deck_context: FieldDeckOverlayContext,
    ghost_mine_knowledge: GhostMineKnowledgeState,
    legitimacy_debt: f32,
    faction_trust: FactionTrustMap,
    chronicle_events: Vec<ChronicleEventMvp>,
    public_repair_precedent: bool,
}
```

## 4.2 Core Tags

```text
water_security
acoustic_legitimacy
emergency_bypass
public_witness
dead_authority
ghost_mine
legitimacy_debt
household_shortfall
civilian_cluster_vulnerability
```

---

# 5. Content Requirements

## 5.1 NPC Minimum Lines

### Basin Court Steward

```text
"A rushed valve becomes a private valve."
```

### Hearth Pump Elder

```text
"A witnessed child still needs water before dark."
```

### Cold Perimeter Officer

```text
"Dead people cannot deliberate."
```

## 5.2 Field Deck Minimum Lines

```text
The valve is not broken in only one way.
It is mechanically misaligned, legally constrained, and historically haunted.
```

## 5.3 Chronicle Minimum Lines

Witnessed:

```text
The valve was restored under public witness. The court's authority survived the crisis, though some households waited longer than they should have.
```

Bypass:

```text
The lower cistern was saved through emergency bypass. The court survived the night, but the right to bypass it became a new dispute.
```

---

# 6. Non-Goals

Do not build yet:

```text
full combat
complex drone AI
full mine tunnel
all four worldline variants
animated tribunal
full settlement simulation
procedural generation
deep NPC schedules
multiplayer dispute resolution
```

Reason:

```text
These are later layers.
The first slice must prove the repair-memory-permission loop.
```

---

# 7. QA Scenarios

## Scenario A — Witnessed Repair

Steps:

```text
scan valve
speak to Basin Court
obtain witness
calibrate valve
record Chronicle
return to valve
```

Expected:

```text
Basin trust increases
public repair precedent set
future valve access easier
Ghost Mine anomaly remains possible
```

## Scenario B — Emergency Bypass

Steps:

```text
scan valve
speak to Hearth or Cold Perimeter
manual bypass
record Chronicle
return to valve
```

Expected:

```text
water restored faster
legitimacy debt active
Basin trust decreases
future hearing required
Cold Perimeter option unlocked
```

## Scenario C — Origin Bias Comparison

Steps:

```text
load same crisis with three origins
open Field Deck
compare first-line warnings
```

Expected:

```text
raw data identical
interpretive priority differs
no moral path is hard-locked
```

## Scenario D — Chronicle Consequence

Steps:

```text
complete either repair path
trigger later valve access
```

Expected:

```text
previous Chronicle event changes permission or dialogue
```

---

# 8. Risks

## Risk 1 — Too Much Lore Before Interaction

Mitigation:

```text
Field Deck readout first.
NPC argument second.
Codex detail optional.
```

## Risk 2 — Bypass Feels Like Wrong Choice

Mitigation:

```text
Both choices save lives.
Both choices create different debts.
```

## Risk 3 — Chronicle Feels Like Quest Log

Mitigation:

```text
Chronicle must alter future permission.
If it does not change access, trust, or memory, it is not a Chronicle event.
```

## Risk 4 — Field Deck Overlay Confuses Players

Mitigation:

```text
Always show raw state above cultural terms.
Use translation stack only after player opens detail view.
```

## Risk 5 — Origin Bias Feels Deterministic

Mitigation:

```text
Origin affects first warning, not allowed choices.
```

---

# 9. Build Readiness Checklist

The slice is ready for prototype when the team has:

```text
[ ] ValveCourtWaterState implemented
[ ] Field Deck raw readout implemented
[ ] Three authority claims implemented
[ ] Two repair paths implemented
[ ] Chronicle MVP event implemented
[ ] Field Deck translation stack implemented
[ ] Three Origin Bias stubs implemented
[ ] GhostMineKnowledgeState stub implemented
[ ] One future permission change implemented
[ ] QA scenarios passing
```

---

# 10. Concept Art / Greybox Targets

## Greybox Site

```text
arrival terrace
valve floor
witness gallery
maintenance panel
public pressure board
```

## Concept Art Priority

1. Choked Valve Court arrival terrace
2. Basal Register wall
3. Acoustic gate calibration
4. Witness gallery hearing
5. Field Deck readout mockup

---

# 11. Final Acceptance Test

The prototype succeeds when a player can truthfully say:

```text
I saw the water was low.
I saw people disagree about who had the right to open the valve.
I chose how to repair it.
The world remembered my choice.
And the next valve treated me differently because of it.
```

---

# 12. Mantra

```text
Build the smallest machine that remembers a repair.
```
