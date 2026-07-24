---
title: Xeno Contact Pressure Semantics Patch v0.1
version: 0.1
scope: Earth Atlas xeno-contact pressure semantics
owner: world-design/xeno/simulation
status: supporting
patch_status: accepted
project: Symtropy
domain: Earth Atlas / Xeno Contact / Pressure Vectors
recommended_path: docs/earth-atlas/00_schema/XENO_CONTACT_PRESSURE_SEMANTICS_PATCH_V0_1.md
patches:
  - EARTH_ATLAS_2168_TEMPLATE.md
  - ANTARCTICA_XENO_CONTACT_SUBGLACIAL_LISTENING_FAULT_V0_1.md
---

# Xeno Contact Pressure Semantics Patch v0.1

## Purpose

Antarctica now has a named xeno-contact hook:

```text
The Subglacial Listening Fault
```

This raises a schema question:

```text
What does xeno_contact_pressure mean in regions that do not yet have named contact hooks?
```

This patch defines the semantics of `xeno_contact_pressure`.

---

# 1. Core Rule

```text
xeno_contact_pressure is not a binary "aliens are here" flag.
It is a measure of how strongly nonhuman, alien, deep-time, or agency-uncertain phenomena can affect regional history, gameplay, and interpretation.
```

A region may have nonzero pressure because of:

```text
direct alien artifact
misclassified signal
biospheric intelligence ambiguity
machine archive anomaly
ancient consent boundary
alien ecological contamination
off-world material pathway
cultural fear of contact
false contact mythology
xeno-quarantine policy
```

---

# 2. Pressure Bands

## 0.00 — None / Not Relevant

```text
No known or meaningful xeno-contact pressure.
```

Do not add alien hooks.

## 0.01–0.05 — Background Cosmological Awareness

The region knows aliens or nonhuman agencies exist elsewhere, but this has little local mechanical force.

Examples:

```text
spaceport rumor
school curriculum
distant Chronicle precedent
imported xeno-safe equipment
```

Gameplay:

```text
minor dialogue
rare Field Deck glossary entries
no required mechanics
```

## 0.06–0.12 — Indirect Contact Pressure

The region is affected by contact-adjacent systems without a local alien site.

Examples:

```text
xeno-biosecurity law affecting seed imports
machine archive categories influenced by alien doctrine
off-world materials requiring quarantine
settlers carrying first-contact trauma
fake alien claims used in politics
```

Gameplay:

```text
policy restrictions
biosecurity checks
faction beliefs
misinformation missions
small Field Deck warnings
```

Southern Africa at `0.09` fits here.

Possible interpretation:

```text
Southern Africa does not host a known alien site.
It has xeno-safe seed law, off-world material controls, and cultural memory of contact debates imported through global networks.
```

## 0.13–0.25 — Named Local Anomaly

The region has at least one named local phenomenon that may involve alien, nonhuman, or agency-uncertain presence.

Examples:

```text
Subglacial Listening Fault
machine archive receiving unknown reciprocal patterns
nonhuman agency preserved by treaty
subsurface artifact with uncertain origin
```

Gameplay:

```text
dedicated mission hook
Field Deck uncertainty mode
faction debate
Chronicle precedent
special hazard/consent mechanics
```

Antarctica at `0.18` fits here.

## 0.26–0.50 — Structural Contact Region

Contact pressure shapes major governance.

Examples:

```text
translation borderland
biospheric intelligence treaty zone
alien quarantine boundary
shared human/nonhuman settlement
```

Gameplay:

```text
rights floor expansion
translation mechanics
nonhuman witness
settlement law altered
major faction identity
```

## 0.51+ — Contact-Defined Region

The region's identity is inseparable from active nonhuman contact.

Use rarely.

Examples:

```text
alien-human confluence city
biospheric intelligence capital
xeno quarantine megazone
multi-species treaty frontier
```

Gameplay:

```text
contact is core loop, not side hook
```

---

# 3. Schema Addition

```rust
struct XenoContactPressure {
    value: f32,
    band: XenoPressureBand,
    source_type: Vec<XenoPressureSource>,
    named_hooks: Vec<String>,
    uncertainty_policy: UncertaintyPolicy,
    gameplay_required: bool,
}
```

```rust
enum XenoPressureSource {
    DirectArtifact,
    MisclassifiedSignal,
    BiosphericAgency,
    MachineArchiveAnomaly,
    OffworldMaterialPathway,
    BiosecurityLaw,
    CulturalMemory,
    FalseContactMyth,
    QuarantinePolicy,
    TranslationTrauma,
}
```

---

# 4. Design Rule: Do Not Over-Alien Earth

Earth Atlas regions should not all secretly contain alien artifacts.

Use nonzero xeno pressure carefully.

Better:

```text
Most regions have 0.00–0.12.
Only a few have named hooks.
Very few are contact-defined.
```

This preserves Earth history as human, ecological, political, and infrastructural history rather than turning the whole planet into an ancient-alien puzzle box.

---

# 5. Antarctica Clarification

```yaml
region_id: white_ledger_territories_antarctica_2168
xeno_contact_pressure:
  value: 0.18
  band: named_local_anomaly
  source_type:
    - MisclassifiedSignal
    - MachineArchiveAnomaly
    - DirectArtifact
    - BiosphericAgency
  named_hooks:
    - Subglacial Listening Fault
  uncertainty_policy: do_not_classify_as_life_or_nonlife_without_witness
  gameplay_required: true
```

---

# 6. Southern Africa Clarification

```yaml
region_id: southern_africa_water_energy_2168
xeno_contact_pressure:
  value: 0.09
  band: indirect_contact_pressure
  source_type:
    - BiosecurityLaw
    - OffworldMaterialPathway
    - CulturalMemory
  named_hooks: []
  uncertainty_policy: xeno_pressure_affects_seed_law_not_local_contact
  gameplay_required: false
```

Possible gameplay effects:

```text
xeno-safe seed handling rules at Tailings Garden sites
off-world filtration membrane provenance checks
imported alien-biosecurity fears used by corporate actors
minor Field Deck glossary entries
```

---

# 7. Acceptance Test

The pressure system is ready when:

```text
1. A region can have low xeno pressure without a local alien site.
2. A region with medium pressure has a named hook.
3. A region with high pressure changes core gameplay.
4. Designers cannot accidentally imply aliens authored Earth history.
5. Field Deck uncertainty is preserved for agency-uncertain phenomena.
```

---

# 8. Final Line

```text
Not every anomaly is an alien.
Not every contact is a conversation.
Not every pressure needs a revelation.
```
