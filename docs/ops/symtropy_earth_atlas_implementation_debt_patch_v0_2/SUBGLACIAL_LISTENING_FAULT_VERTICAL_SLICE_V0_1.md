---
title: Subglacial Listening Fault Vertical Slice v0.1
status: canonical-draft
project: Symtropy
domain: Antarctica / Xeno Contact / Mission Design / Machine Archive / Field Deck
recommended_path: docs/earth-atlas/antarctica/SUBGLACIAL_LISTENING_FAULT_VERTICAL_SLICE_V0_1.md
depends_on:
  - ANTARCTICA_XENO_CONTACT_SUBGLACIAL_LISTENING_FAULT_V0_1.md
  - CHRONICLE_MVP_SPEC_V0_1.md
  - FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
  - XENO_CONTACT_PRESSURE_SEMANTICS_PATCH_V0_1.md
---

# Subglacial Listening Fault Vertical Slice v0.1

## Working Title

**The Signal That Did Not Ask To Be Found**

## One-Sentence Pitch

A machine weather archive has been preserving a subglacial anomaly for forty years while misclassifying human distress and possible nonhuman boundary feedback; the player must repair harmful acoustic interference before deciding whether contact should proceed, remain silent, or be buried again in machine categories.

---

# 1. Purpose

This is the first playable Antarctica xeno-contact vertical slice.

It proves the principle:

```text
Repair as greeting.
```

The player does not meet a talking alien.

The player stops a harmful system, records uncertainty, and observes a reciprocal change that may or may not be communication.

Target playtime:

```text
45–75 minutes
```

Primary question:

```text
Can you become careful enough for contact before curiosity becomes another extraction technology?
```

---

# 2. Site Identity

```yaml
site_id: antarctica.white_quiet_array.v01
display_name: White Quiet Array
region_id: white_ledger_territories_antarctica_2168
architecture_family: machine_archive_polar_station
primary_life_support_system: automated weather archive power loop
secondary_life_support_systems:
  - emergency heat shelter
  - ice-road beacon line
  - low-frequency acoustic sensor array
  - borehole court seal
  - ranger patrol route
primary_factions:
  - Machine Weather Continuity Process
  - Treaty Refusal Court
  - Peninsula Refuge City delegation
  - Non-Ownership Rangers
  - White Ledger Archivists
  - possible Continuity Choir interpreter thread
```

---

# 3. Opening Situation

The player arrives after a refugee repair crew reports a painful vibration beneath the ice.

The Machine Weather Continuity Process refuses shutdown because the array is part of a century-scale climate record.

The Treaty Court refuses drilling.

The Rangers have sealed the valley.

A refugee child says:

```text
It is not calling.
It is flinching.
```

Initial Field Deck scan:

```sh
SIGNAL CLASS: BASAL RESONANCE
REPEATABILITY: HIGH
MACHINE LABEL: HYDROLOGICAL_ARTIFACT
AGENCY CONFIDENCE: UNKNOWN
HARM IF DISTURBED: HIGH

RECOMMENDED:
Do not classify as life.
Do not classify as nonlife.
Record uncertainty.
```

---

# 4. Core Systems

## 4.1 Listening Integrity

```rust
struct ListeningIntegrity {
    acoustic_interference: f32,
    thermal_intrusion: f32,
    chemical_contamination_risk: f32,
    machine_classification_accuracy: f32,
    public_witness_integrity: f32,
    nonhuman_agency_confidence: f32,
}
```

## 4.2 Harmful Curiosity

```rust
struct HarmfulCuriosityState {
    extraction_pressure: f32,
    certainty_pressure: f32,
    military_classification_risk: f32,
    public_panic_risk: f32,
    witness_integrity: f32,
    humility_actions_completed: u8,
}
```

Harmful curiosity increases when the player:

```text
forces access
drills before witness
publishes raw signal without context
hands evidence to machine archive only
accepts military classification
treats the anomaly as either confirmed life or confirmed nonlife too early
```

Harmful curiosity decreases when the player:

```text
repairs interference
reduces thermal intrusion
records uncertainty
invites plural witness
corrects machine categories
preserves silence where appropriate
```

## 4.3 Machine Category Debt

```rust
struct MachineCategoryDebt {
    human_distress_misclassified: u32,
    refuge_heat_misclassified: u32,
    anomaly_events_dismissed: u32,
    care_categories_missing: Vec<String>,
    amendment_status: AmendmentStatus,
}
```

Design rule:

```text
The machine archive is not evil.
It is brilliant inside categories that cannot care.
```

---

# 5. Layout

## 5.1 Ice Road Approach

Visual:

```text
whiteout markers
ranger signs
half-buried convoy sled
aurora glow
Field Deck interference static
```

Gameplay:

```text
navigation
ranger challenge
refuge crew testimony
first vibration event
```

## 5.2 White Quiet Array Surface Station

Visual:

```text
ice-buried sensor towers
green archive lights
empty bunks
robot tracks
frozen distress beacon
sealed maintenance doors
```

Gameplay:

```text
power reroute
machine archive audit
distress beacon recovery
sensor calibration
```

## 5.3 Archive Category Hall

Visual:

```text
walls of climate records
rows of anomaly labels
misclassified human distress events
machine witness terminals
```

Gameplay:

```text
category correction
evidence comparison
machine testimony negotiation
```

## 5.4 Acoustic Spine

Visual:

```text
low-frequency transducers
ice vibration conduits
copper/ceramic isolation mounts
frost shaking from walls
```

Gameplay:

```text
repair puzzle
harm reduction
sensor continuity tradeoff
```

## 5.5 Borehole Court Seal

Visual:

```text
unused drilling cap
treaty witness gallery
sterile suits
Ranger locks
refuge delegation banners
```

Gameplay:

```text
final hearing
contact protocol decision
Chronicle event
```

---

# 6. Beat Map

## Beat 1 — Hear the Flinch

Objective:

```text
Reach the array and document the vibration without escalating access.
```

Events:

```text
Field Deck static spikes.
A child describes pain before instruments classify it.
Rangers warn against unpermitted entry.
```

Player learns:

```text
local testimony may detect harm before machine categories do
```

## Beat 2 — Audit the Archive

Objective:

```text
Find what the machine has preserved and misclassified.
```

Discoveries:

```text
perfect climate continuity
43 human distress beacons labeled non-scientific traffic
219 refuge heat signatures labeled settlement noise
18,402 anomaly events labeled drift
```

Field Deck:

```sh
SENSOR CONTINUITY: EXCELLENT
CARE CONTINUITY: FAILED
```

## Beat 3 — Choose a Witness Set

Objective:

```text
Decide who must be present before changing the archive.
```

Possible witnesses:

```text
Machine Archive process
Treaty Court witness
Refuge City delegate
Ranger observer
White Ledger Archivist
Field Deck source chain
ecological/microbial indicator if available
```

More witnesses increase legitimacy but slow repair.

Too few witnesses increase harmful curiosity or authority drift.

## Beat 4 — Repair the Acoustic Spine

Objective:

```text
Reduce harmful acoustic interference without destroying climate record continuity.
```

Player actions:

```text
replace isolation mount
reroute transducer timing
lower amplitude
preserve data stream
mark previous emissions as possible harm events
```

Result:

```text
The basal resonance pattern changes.
It becomes less defensive.
It does not speak.
```

Field Deck:

```sh
POST-REPAIR PATTERN CHANGE: VERIFIED
LANGUAGE MODEL: NOT AVAILABLE
CONTACT STATUS: NONVERBAL RECIPROCITY POSSIBLE
HARM REDUCTION CONFIDENCE: HIGH
```

## Beat 5 — Correct Categories

Objective:

```text
Amend the machine archive so future care signals are not classified as noise.
```

Category changes:

```text
HUMAN_TRAFFIC -> HUMAN_DISTRESS / HUMAN_PRESENCE_CONTEXT
ANOMALY -> AGENCY_UNCERTAIN_LISTENING_EVENT
MICROBIAL_RESPONSE -> POSSIBLE_BOUNDARY_FEEDBACK
```

This requires witness.

If skipped, the machine may continue preserving data while repeating harm.

## Beat 6 — Borehole Court Decision

Objective:

```text
Decide what happens after the reciprocal change.
```

Final choices:

### Choice A — Preserve Silence After Repair

```text
No probe.
No drilling.
Public record of uncertainty.
Monitoring continues under amended categories.
```

Effect:

```text
low harmful curiosity
high treaty trust
slow contact progress
refuge delegation frustrated but safer
```

### Choice B — Begin Slow Public Contact Protocol

```text
Petition for non-invasive listening probe.
No extraction tools.
Appeal shutoff required.
Plural witness required.
```

Effect:

```text
contact path opens
higher complexity
Ranger concern rises
Machine Archive must accept care categories
```

### Choice C — Hand Evidence to Machine Archive Only

This is now explicitly a failure-biased option.

```text
The signal goes back into the same classification regime that misread it for forty years.
```

Effect:

```text
short-term order preserved
public panic avoided
Machine Archive trust among humans decreases if later revealed
harmful curiosity appears low but category debt remains high
Chronicle sealed event: The Signal Returned to Drift
```

Hidden risk:

```text
Future machine-only contact may optimize data continuity over consent.
```

### Choice D — Leak Discovery to Force Action

Effect:

```text
public pressure rises
harmful curiosity spikes
resource/protectorate actors may intervene
signal may withdraw
```

### Choice E — Suppress Signal to Protect Refuge Expansion

Effect:

```text
refuge politics stabilized temporarily
Treaty trust damaged if exposed
nonhuman agency harmed or ignored
Chronicle addendum may become severe
```

---

# 7. Failure States

## 7.1 Category Failure

The player repairs hardware but leaves machine categories unchanged.

Effect:

```text
interference reduced
future misclassification continues
Machine Archive remains brilliant but uncaring
```

## 7.2 Curiosity Escalation

The player forces drilling or leaks prematurely.

Effect:

```text
Ranger lockdown
Treaty sanctions
signal withdrawal
possible Continuity Choir quarantine thread activation
```

## 7.3 Machine-Only Burial

The player hands evidence to archive only.

Effect:

```text
discovery preserved but politically buried
future players may find sealed anomaly index
refuge witnesses lose trust if they learn
```

## 7.4 Human-Centric Overclaim

The player declares confirmed alien life too early.

Effect:

```text
public panic / exploitation pressure
Field Deck flags overinterpretation
Treaty Court suspends protocol
```

---

# 8. Chronicle Outcomes

## Best Ambiguous Outcome

```text
The Flinch Was Heard
```

Summary:

```text
The player reduced harmful interference, amended machine categories, and recorded agency uncertainty without forcing contact.
```

## Contact Path Outcome

```text
The First Quiet Protocol
```

Summary:

```text
The region authorized non-invasive listening under public witness.
```

## Machine Burial Outcome

```text
The Signal Returned to Drift
```

Summary:

```text
Evidence of possible boundary response was preserved by the machine archive but not translated into public care or contact protocol.
```

## Escalation Outcome

```text
Curiosity Became an Engine
```

Summary:

```text
The discovery became a political force before it became a careful relation.
```

## Suppression Outcome

```text
Warmth Chose Silence
```

Summary:

```text
Refuge stability was protected by suppressing a possible nonhuman boundary signal.
```

---

# 9. Acceptance Test

The vertical slice succeeds if the player can say:

```text
I did not meet an alien.
I stopped hurting something, and the world changed enough to make my categories feel dangerous.
```

The best outcome is not certainty.

The best outcome is:

```text
lower harm
better categories
public witness
preserved uncertainty
future contact earned rather than seized
```

---

# 10. Final Line

```text
The first answer was not a word.
It was the absence of pain returning as rhythm.
```
