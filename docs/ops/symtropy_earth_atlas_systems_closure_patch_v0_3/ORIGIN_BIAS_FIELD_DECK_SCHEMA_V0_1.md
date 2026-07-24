---
title: Origin Bias Field Deck Schema v0.1
status: canonical-draft
project: Symtropy
domain: Field Deck / Player Origins / UI / Cultural Interpretation / Truth Layers
recommended_path: docs/systems/field-deck/ORIGIN_BIAS_FIELD_DECK_SCHEMA_V0_1.md
extends:
  - FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
  - Symtropy Player Origins Full Des.md
  - EARTH_ATLAS_2168_TEMPLATE.md
---

# Origin Bias Field Deck Schema v0.1

## Working Title

**What Your Past Teaches You to Notice**

## Purpose

The Field Deck overlay stack includes **Personal Origin Bias**, but the layer needs a buildable schema.

This document defines how player origin changes what the Field Deck foregrounds without falsifying the underlying truth.

Core principle:

```text
Origin Bias is attention, not destiny.
```

A player's origin may change:

```text
what appears first
which warning tags are early
which vocabulary feels familiar
which NPC assumptions unlock
which risks are over-detected
which risks are under-detected
```

It must not change:

```text
raw diagnostic data
source-chain truth
sensor readings
actual contamination level
actual pressure level
actual Null state
actual authority provenance
```

---

# 1. Design Rule

```text
The Field Deck does not only show what is true.
It also shows what the player has been trained to fear, trust, and repair.
```

Origin Bias is a UI and interpretation layer.

It is not a character class.

It should create different first impressions, not hard-lock moral choices.

---

# 2. Schema

```rust
struct OriginBias {
    origin_id: OriginId,
    display_name: String,
    origin_family: OriginFamily,

    foregrounded_modes: Vec<FieldDeckMode>,
    deprioritized_modes: Vec<FieldDeckMode>,

    early_warning_tags: Vec<SystemTag>,
    familiar_authorities: Vec<AuthorityTag>,
    suspicious_authorities: Vec<AuthorityTag>,

    local_term_affinities: Vec<LocalTermAffinity>,
    dialogue_lenses: Vec<DialogueLens>,

    false_positive_risks: Vec<BiasRisk>,
    false_negative_risks: Vec<BiasRisk>,

    starting_assumptions: Vec<StartingAssumption>,
    unlockable_self_corrections: Vec<SelfCorrection>,
}
```

## 2.1 Origin Family

```rust
enum OriginFamily {
    CorporateUtility,
    RefugeeCharter,
    WorkerGuild,
    ArchiveWitness,
    RoadChoir,
    ColdPerimeter,
    HearthCommon,
    MachineAdjacency,
    BasinCourt,
    MedicalCare,
    ExclusionZone,
    OffWorld,
    UnknownOrFragmented,
}
```

## 2.2 Field Deck Mode

Examples:

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL
REPAIR
WITNESS
CARE
VEHICLE
TACTICAL_NET
SOURCE_CHAIN
```

## 2.3 System Tags

Examples:

```text
contract_lock
subscription_infrastructure
provisional_status
bad_repair_lineage
water_authority_dispute
care_capacity_shortfall
route_debt
toxic_legacy
dead_authority
machine_category_debt
ranger_drift
corporate_capture
static_identity_risk
```

---

# 3. Local Term Affinity

Some players understand certain civic vocabularies faster.

```rust
struct LocalTermAffinity {
    term: String,
    comprehension_bonus: f32,
    emotional_salience: f32,
    tooltip_depth_bonus: u8,
}
```

Example:

```yaml
origin_id: refugee_charter_child
local_term_affinities:
  - term: provisional_status
    comprehension_bonus: 0.25
    emotional_salience: 0.70
    tooltip_depth_bonus: 2
  - term: warmright
    comprehension_bonus: 0.20
    emotional_salience: 0.60
    tooltip_depth_bonus: 1
```

---

# 4. Dialogue Lens

Origin Bias can unlock interpretive prompts.

```rust
struct DialogueLens {
    lens_id: String,
    trigger_tags: Vec<SystemTag>,
    prompt_text: String,
    risk: BiasRiskKind,
}
```

Example:

```text
A Corporate Utility Defector may notice:
"That valve contract looks dead, but the priority queue is still executing."

A Refugee Charter Child may notice:
"This intake hall is warm, but the appeal route is missing."

A Worker-Guild Mechanic may notice:
"This repair was done fast by someone afraid of being watched."
```

---

# 5. Bias Risk

Origin Bias should include mistakes.

```rust
struct BiasRisk {
    risk_id: String,
    risk_kind: BiasRiskKind,
    description: String,
    mitigation: Vec<SelfCorrectionTrigger>,
}
```

```rust
enum BiasRiskKind {
    OverdetectThreat,
    UnderdetectThreat,
    OvertrustAuthority,
    UndertrustAuthority,
    TechnicalReductionism,
    ProceduralOvertrust,
    CareOverextension,
    StaticIdentityFear,
    CorporateParanoia,
    ArchivePurism,
    EmergencyCommandDrift,
}
```

Design rule:

```text
Every origin gives insight and distortion.
```

---

# 6. Self-Correction

Players can learn beyond their origin.

```rust
struct SelfCorrection {
    correction_id: String,
    unlocked_by: Vec<ChronicleEventClass>,
    changes: Vec<OriginBiasAdjustment>,
    player_facing_line: String,
}
```

Examples:

```text
A Corporate Utility Defector who repeatedly sees public utilities behave well may reduce corporate-paranoia false positives.

A Worker-Guild Mechanic who causes legitimacy debt through emergency repair may foreground CIVIC mode earlier.

A Refugee Charter Child who sees real ecological capacity limits may distinguish exclusion from system overload.
```

Core rule:

```text
Origin Bias can evolve through Chronicle experience.
```

---

# 7. Example Origins

## 7.1 Corporate Utility Defector

```yaml
origin_id: corporate_utility_defector
display_name: Corporate Utility Defector
origin_family: CorporateUtility

foregrounded_modes:
  - DIAG
  - NULL
  - ARCHIVE

deprioritized_modes:
  - CARE

early_warning_tags:
  - contract_lock
  - subscription_infrastructure
  - dead_authority
  - corporate_capture
  - service_dependency

familiar_authorities:
  - utility_service_board
  - contract_archive

suspicious_authorities:
  - corporate_utility_remnant
  - proprietary_repair_vendor

false_positive_risks:
  - corporate_paranoia
  - undertrust_authority

false_negative_risks:
  - care_capacity_shortfall
  - community_legitimacy

sample_prompt:
  "The pipe is public, but the repair authorization still smells like a service contract."
```

Gameplay effect:

```text
The player detects Ghost Mine contract residue earlier,
but may initially underestimate Hearth or Basin Court legitimacy.
```

---

## 7.2 Refugee Charter Child

```yaml
origin_id: refugee_charter_child
display_name: Refugee Charter Child
origin_family: RefugeeCharter

foregrounded_modes:
  - CARE
  - CIVIC
  - WITNESS

deprioritized_modes:
  - TACTICAL_NET

early_warning_tags:
  - provisional_status
  - appeal_route_missing
  - intake_backlog
  - warmright_risk
  - exclusion_without_witness

familiar_authorities:
  - refuge_committee
  - kitchen_council
  - charter_advocate

suspicious_authorities:
  - ranger_permit_gate
  - emergency_border_posture

false_positive_risks:
  - overdetect_exclusion
  - undertrust_authority

false_negative_risks:
  - ecological_capacity_limit
  - infrastructure_overload

sample_prompt:
  "They are calling it capacity. Check whether there is an appeal route."
```

Gameplay effect:

```text
The player sees third-door disputes and provisional legitimacy earlier,
but may initially read necessary limits as cruelty.
```

---

## 7.3 Worker-Guild Mechanic

```yaml
origin_id: worker_guild_mechanic
display_name: Worker-Guild Mechanic
origin_family: WorkerGuild

foregrounded_modes:
  - REPAIR
  - DIAG
  - SCAN

deprioritized_modes:
  - ARCHIVE

early_warning_tags:
  - bad_repair_lineage
  - tool_mismatch
  - pressure_fatigue
  - brittle_patch
  - unsafe_bypass

familiar_authorities:
  - repair_guild
  - workshop_steward

suspicious_authorities:
  - ceremonial_authority_without_maintenance_record
  - proprietary_service_vendor

false_positive_risks:
  - technical_reductionism

false_negative_risks:
  - legitimacy_debt
  - public_witness_need

sample_prompt:
  "The valve can be opened. That does not mean the repair will survive the hearing."
```

Gameplay effect:

```text
The player finds repair paths early,
but may need Chronicle experience to appreciate witness authority.
```

---

## 7.4 Road Choir Routekin

```yaml
origin_id: road_choir_routekin
display_name: Road Choir Routekin
origin_family: RoadChoir

foregrounded_modes:
  - VEHICLE
  - SCAN
  - WITNESS

deprioritized_modes:
  - CIVIC_STATIC_DEFAULTS

early_warning_tags:
  - route_debt
  - host_right_dispute
  - vehicle_fatigue
  - stopping_hearing_overdue
  - convoy_labor_dependency

familiar_authorities:
  - route_elder
  - bridge_citizen
  - axle_captain

suspicious_authorities:
  - static_registry
  - settlement_gate_office

false_positive_risks:
  - static_identity_fear

false_negative_risks:
  - durable_archive_need
  - harm_claim_persistence

sample_prompt:
  "A town can say 'later' until the road has to carry its consequences."
```

Gameplay effect:

```text
The player reads route memory and vehicle scars fluently,
but may initially underweight static records.
```

---

## 7.5 Archive Witness Apprentice

```yaml
origin_id: archive_witness_apprentice
display_name: Archive Witness Apprentice
origin_family: ArchiveWitness

foregrounded_modes:
  - ARCHIVE
  - WITNESS
  - SOURCE_CHAIN

deprioritized_modes:
  - TACTICAL_NET

early_warning_tags:
  - chain_of_custody_break
  - testimony_dispute
  - altered_record
  - machine_category_debt
  - dead_authority

familiar_authorities:
  - archive_witness
  - tribunal
  - source_chain_custodian

suspicious_authorities:
  - emergency_override_without_record
  - undocumented_machine_testimony

false_positive_risks:
  - archive_purism
  - procedural_overtrust

false_negative_risks:
  - urgent_care_need
  - repair_window_closing

sample_prompt:
  "The evidence is incomplete. The harm is not waiting."
```

Gameplay effect:

```text
The player protects evidence well,
but may need to learn when incomplete truth is enough to act.
```

---

# 8. Interaction With Overlay Precedence

Origin Bias applies after Culture/Faction overlays but before Emergency Posture and Null Distortion.

Stack position:

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

Important:

```text
Origin Bias may reorder what the player notices.
It may not override hazard, authority, or raw diagnostic truth.
```

Example:

```sh
RAW:
lower_cistern_level = 18%
mine_drain_rate = 0.31
public_authority = Basin Court

BASIN COURT OVERLAY:
Emergency Clause Threshold

ORIGIN BIAS / Corporate Utility Defector:
Foregrounds contract residue.
"Check for dead service priority before blaming court delay."

ORIGIN BIAS / Refugee Charter Child:
Foregrounds lower household risk.
"Appeal route needed before children lose water."
```

---

# 9. Chronicle Interaction

Chronicle events can modify Origin Bias.

Example:

```yaml
chronicle_event: The Garden Changed Its Promise
affected_origin_bias:
  worker_guild_mechanic:
    reduces technical_reductionism
    increases CIVIC/WITNESS salience for ecological repair
  mine_scar_witness_origin:
    reduces witness_paralysis if duplicate-evidence protocol succeeded
```

## Bias Evolution Rule

```text
A player origin is a starting lens.
Chronicle history is how the lens becomes wiser or more damaged.
```

---

# 10. UI Requirements

The Field Deck should show origin influence subtly.

Avoid:

```text
large class icons
RPG stat cards
"because of your origin" popups every time
```

Prefer:

```text
small lens icon
optional tooltip
different first-line interpretation
foregrounded tags
origin-specific comparison hints
```

Example:

```sh
LENS: Worker-Guild Mechanic
Interpretive note available.
```

---

# 11. Acceptance Tests

Origin Bias is ready when:

```text
1. Two player origins see the same raw pump failure but different first warnings.
2. Raw data remains identical across origins.
3. Origin bias produces both insight and distortion.
4. Chronicle events can soften or sharpen origin assumptions.
5. Origin bias can be disabled or compared in accessibility/debug mode.
6. No origin hard-locks a moral path.
7. Origin-specific prompts help players notice systems, not win debates automatically.
```

---

# 12. Mantra

```text
Your origin is not what you are.
It is what the world taught you to notice first.
```
