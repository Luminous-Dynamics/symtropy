---
title: Field Deck Overlay Precedence Rules v0.1
status: canonical-draft
project: Symtropy
domain: Field Deck / UI / Culture Overlays / Truth Layers
recommended_path: docs/systems/field-deck/FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
depends_on:
  - EARTH_ATLAS_2168_TEMPLATE.md
  - IN_WORLD_COMPUTING_AND_SYMTROPYOS.md
  - MULTIPLAYER_TRUTH_MODEL.md
---

# Field Deck Overlay Precedence Rules v0.1

## Working Title

**Truth Layer, Local Vocabulary**

## Purpose

Earth Atlas cultures now define Field Deck overlays with:

```text
visual_temperature
alert_language
prioritized_modes
suppressed_modes
local_terms
failure_bias
```

This creates a problem:

```text
What happens when the player enters overlapping cultural zones?
```

Example:

```text
A Road Choir anchor season inside a Basin Court town
A Peninsula Refuge intake hall guarded by Non-Ownership Rangers
A Hearth Pump Village under Cold Perimeter emergency command
A Machine Archive facility inside Treaty Sanctuary territory
```

This document defines the overlay precedence rules.

---

# 1. Core Rule

```text
The Field Deck core truth layer is never replaced.
Only the interpretive overlay changes.
```

The Field Deck must always preserve access to:

```text
raw sensor data
mode identity
source-chain status
witness status
authority provenance
Null warnings
manual override explanation
```

Culture can reframe truth.

Culture cannot erase truth unless a system is explicitly compromised, captured, or Null-distorted.

---

# 2. Overlay Stack

The Field Deck applies overlays in this order.

```text
1. Core Diagnostic Layer
2. Safety / Hazard Layer
3. Authority / Jurisdiction Layer
4. Site Layer
5. Culture Layer
6. Faction Layer
7. Personal Origin Bias
8. Emergency Posture Modifier
9. Null Distortion / Corruption Layer
```

## 2.1 Core Diagnostic Layer

Always present.

Examples:

```text
pressure
temperature
radiation
contamination
signal integrity
source-chain status
device state
```

Cannot be suppressed.

## 2.2 Safety / Hazard Layer

Overrides aesthetic preferences.

Examples:

```text
toxic plume
water contamination
hypothermia
reactor instability
combat threat
oxygen failure
```

Cannot be hidden by warm or ceremonial overlays.

## 2.3 Authority / Jurisdiction Layer

Shows who claims the right to act.

Examples:

```text
Basin Court
Road Choir host-right
Treaty Court
Machine Archive
Corporate Utility Remnant
Emergency Command
```

## 2.4 Site Layer

Applies site-specific labels.

Example:

```text
The Choked Valve Court labels pressure events as court-relevant valve states.
```

## 2.5 Culture Layer

Changes moral vocabulary.

Example:

```text
Hearth Common:
  "Shared Warmth Shortfall"

Cold Perimeter:
  "Civilian Cluster Vulnerability"
```

## 2.6 Faction Layer

Adds faction-specific interpretation if the player is using a faction-authorized overlay.

## 2.7 Personal Origin Bias

Player origin affects what is foregrounded.

Examples:

```text
Corporate Utility Defector sees hidden contract locks earlier.
Worker-Guild Mechanic sees bad repair lineage earlier.
Refugee Charter Child sees exclusion paths earlier.
```

## 2.8 Emergency Posture Modifier

Temporary crisis overlays.

Examples:

```text
lockdown
evacuation
water emergency
quarantine
whiteout
attack
```

Emergency posture may temporarily elevate suppressed modes but must show that this is happening.

## 2.9 Null Distortion Layer

If active, the Field Deck must indicate possible misclassification.

---

# 3. Conflict Resolution Rules

## Rule 1 — Hazard Beats Culture

A toxic plume is still a toxic plume inside a festival.

```text
Culture may change alert language.
Culture may not suppress hazard truth.
```

## Rule 2 — Authority Beats Faction Styling

If the player is in a Basin Court valve hall, the Field Deck must show Basin Court authority even if a Road Choir overlay is active.

## Rule 3 — Site Beats Region

A machine archive site inside Antarctica may use Machine Archive terminology for device internals while still showing Treaty Sanctuary jurisdiction.

## Rule 4 — Emergency Modifies, It Does Not Permanently Replace

A Hearth Village under attack may temporarily foreground TACTICAL_NET.

After the crisis, the overlay should revert or show authority drift.

## Rule 5 — Suppressed Means Deprioritized, Not Deleted

Suppressed modes move lower in the UI and require deliberate access.

They are not removed.

Example:

```text
Road Choir overlay suppresses CIVIC_STATIC_DEFAULTS.
The player can still open CIVIC mode and see static legal claims.
```

## Rule 6 — Conflicting Local Terms Show as Translation Stack

If multiple cultures name the same state differently, the Field Deck can show a stack.

Example:

```sh
RAW STATE:
lower_cistern_level = 18%

HEARTH TERM:
Shared Warmth Shortfall

BASIN COURT TERM:
Emergency Clause Threshold

COLD PERIMETER TERM:
Civilian Cluster Vulnerability
```

## Rule 7 — Null Distortion Must Be Named

If a mode is suppressed because of Null, capture, or corruption, it must be flagged differently from cultural deprioritization.

```text
CULTURAL SUPPRESSION:
This society does not foreground this mode.

NULL SUPPRESSION:
This system may be preventing you from seeing this mode.
```

---

# 4. Overlay Conflict Schema

```rust
struct FieldDeckOverlayContext {
    core_layer: CoreDiagnosticLayer,
    hazard_layer: Option<HazardLayer>,
    authority_layer: AuthorityLayer,
    site_layer: Option<SiteOverlay>,
    culture_layers: Vec<CultureOverlay>,
    faction_layers: Vec<FactionOverlay>,
    origin_bias: Option<OriginBias>,
    emergency_posture: Option<EmergencyPosture>,
    null_distortion: Option<NullDistortion>,
}
```

```rust
struct OverlayResolution {
    displayed_primary_term: String,
    translation_stack: Vec<OverlayTerm>,
    prioritized_modes: Vec<FieldDeckMode>,
    suppressed_modes: Vec<FieldDeckMode>,
    locked_modes: Vec<LockedModeReason>,
    warnings: Vec<OverlayWarning>,
}
```

---

# 5. Example: Road Choir Anchor Season Inside Basin Court Town

```sh
RAW:
water_tanker_level = 31%
lower_cistern_level = 18%
host_right_status = disputed
basin_court_authority = active

PRIMARY DISPLAY:
Emergency Water Host Dispute

TRANSLATION STACK:
Road Choir: Route Debt Unbalanced
Basin Court: Witnessed Allocation Required
Hearth Village: Household Water Shortfall

PRIORITIZED:
DIAG, CIVIC, VEHICLE, WITNESS

SUPPRESSED:
TACTICAL_NET unless emergency posture changes
```

---

# 6. Example: Peninsula Refuge Hall With Ranger Enforcement

```sh
RAW:
habitat_occupancy = 132%
heat_state = strained
ranger_permit_gate = active
refuge_exception_review = overdue

PRIMARY DISPLAY:
Third-Door Heat Dispute

TRANSLATION STACK:
Refuge City: Warmright Risk
Ranger Overlay: Unauthorized Settlement Pressure
Treaty Court: Provisional Overcapacity

PRIORITIZED:
DIAG, CARE, CIVIC, ARCHIVE

WARNING:
Ranger enforcement posture approaching sovereign behavior.
```

---

# 7. Player Control

Players should eventually be able to:

```text
pin a preferred overlay
compare overlays
view raw mode
view local vocabulary
mark suspected bias
record overlay conflict as Chronicle evidence
```

Accessibility rule:

```text
The player should never be trapped inside one society's vocabulary.
```

---

# 8. Acceptance Tests

The overlay system is ready when:

```text
1. The same pump failure can be shown through Hearth, Cold Perimeter, and Basin Court terms.
2. Raw diagnostic data remains accessible in all overlays.
3. A site with overlapping cultures shows a translation stack.
4. Emergency posture can elevate tactical data without permanently changing culture.
5. Null suppression is visually distinct from cultural suppression.
6. Player origin bias foregrounds details without falsifying data.
7. Overlay conflict can become Chronicle evidence.
```

---

# 9. Mantra

```text
The Field Deck does not only show what is true.
It shows what a society has learned to call the truth.
```
