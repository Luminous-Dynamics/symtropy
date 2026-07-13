---
title: Green Cover Lie Resolution Mechanic v0.1
status: canonical-draft
project: Symtropy
domain: Southern Africa / Toxic Legacy / Ecological Repair / Mission Mechanics
recommended_path: docs/earth-atlas/southern-africa/GREEN_COVER_LIE_RESOLUTION_MECHANIC_V0_1.md
patches:
  - SOUTHERN_AFRICA_TOXIC_LEGACY_TEXTURE_PASS_V0_1.md
---

# Green Cover Lie Resolution Mechanic v0.1

## Purpose

This document makes the **Green Cover Lie** mission seed implementable.

Problem:

```text
A beautiful remediation garden is hiding contaminated food crops.
The player must expose the flaw without causing a backlash against all ecological repair.
```

The hard part is not discovering contamination.

The hard part is preserving trust in ecological repair while exposing a false success.

---

# 1. Core Thesis

```text
Bad remediation is not proof that remediation is bad.
It is proof that repair without witness becomes performance.
```

The player must separate:

```text
the method
the site
the evidence
the people who depended on it
the institutions that overclaimed success
```

Failure to separate these creates a backlash:

```text
Tailings Gardeners lose legitimacy.
Corporate chemical contractors gain influence.
Mine-Scar Witnesses are accused of sabotage.
Hearth kitchens lose safe-food confidence.
```

---

# 2. Mission Setup

A Tailings Garden has become a regional showcase.

Visual:

```text
green raised beds
children learning soil care
public banners
solar pumps
willow buffer rows
duckweed polishing ponds
beautiful shade cloth
```

Problem:

```text
Field Deck REPAIR mode detects metals entering edible roots.
Public signage says the site is safe.
The garden's success has already been used to justify reopening nearby housing.
```

Primary NPCs:

```text
Tailings Gardener Lead
Mine-Scar Witness Sampler
Red Water Baker
Dust Lung Clinic Nurse
Corporate Remediation Salesperson
Hearth Parent
```

---

# 3. Core Variables

```rust
struct GreenCoverLieState {
    contamination_detected: bool,
    contamination_publicly_confirmed: bool,
    evidence_integrity: f32,
    ecological_repair_trust: f32,
    site_specific_trust: f32,
    corporate_remediation_pressure: f32,
    food_risk: f32,
    child_exposure_risk: f32,
    gardener_defensiveness: f32,
    witness_cooperation: f32,
    alternative_repair_plan_ready: bool,
}
```

---

# 4. Resolution Mechanic: Separate, Preserve, Correct

The player must complete three actions before public exposure.

## Step 1 — Separate the Claim

Identify what is false.

Possible false claims:

```text
food crops are safe
soil is fully remediated
visual greenness equals recovery
test plots represent whole site
root uptake is negligible
water polishing is complete
```

Field Deck:

```sh
FALSE CLAIM DETECTED:
"Edible root crops safe for household distribution."

NOT DISPROVEN:
Willow buffer effectiveness.
Duckweed pond metal capture.
Shade cover dust reduction.
Community garden labor value.
```

This prevents total collapse of trust.

## Step 2 — Preserve the Working Parts

Document which parts work.

Examples:

```text
willow buffers reduce dust migration
duckweed ponds reduce surface metal load
mycorrhizal plots stabilize soil moisture
non-edible cover crops reduce exposed dust
raised beds are safe if sealed properly
```

Chronicle evidence:

```text
The garden failed as food.
It partially succeeded as dust control.
```

## Step 3 — Correct the Harm

Offer a concrete replacement plan before public accusation.

Options:

```text
convert edible beds to non-edible phytoremediation
move food production to sealed raised beds
install root-barrier membranes
route produce through Red Water Baker testing
create color-coded edible/non-edible garden zones
publicly mark uncertain plots instead of destroying them
train citizen samplers
```

---

# 5. Public Hearing Structure

The hearing has three possible frames.

## Frame A — Scandal Frame

```text
"The gardeners poisoned people."
```

Effects:

```text
site trust collapses
ecological repair trust collapses
corporate remediation pressure rises
public anger high
```

## Frame B — Denial Frame

```text
"The readings are exaggerated. The garden is beautiful and needed."
```

Effects:

```text
short-term morale preserved
food risk continues
Mine-Scar Witness trust collapses
future exposure worse
```

## Frame C — Witnessed Correction Frame

```text
"The food claim was false.
The dust-control work partly succeeded.
The site must change purpose under public witness."
```

Effects:

```text
site trust decreases moderately
ecological repair trust preserved
food risk drops
citizen sampling unlocks
Tailings Gardeners remain reformable
```

The player's goal is to create Frame C.

---

# 6. Gameplay Actions

## 6.1 Dual-Sample Protocol

Take paired samples:

```text
edible root tissue
surrounding soil
non-edible cover crop
water inlet
water outlet
sealed raised bed control
```

This proves the problem is specific, not universal.

## 6.2 Public Plot Map

Use Field Deck to generate a map:

```text
red = unsafe edible uptake
yellow = uncertain / non-food only
green = safe sealed bed
blue = water-polishing zone
white = evidence-preserve area
```

## 6.3 Ally Selection

The player should recruit at least two of:

```text
Mine-Scar Witness Sampler
Tailings Gardener Lead
Red Water Baker
Dust Lung Clinic Nurse
Hearth Parent
```

Different allies change the hearing.

## 6.4 Replacement Plan

The player must choose one:

```text
Food Withdrawal + Remediation Conversion
Sealed Bed Rebuild
Phytoremediation Only Zone
Rotating Test Garden
Temporary Closure with Convoy Food Support
```

---

# 7. Failure Modes

## 7.1 Total Discredit

Triggered by exposing contamination without preserving working evidence.

```text
Public reads: all green repair is theater.
Corporate chemical contractors gain power.
Tailings Gardeners fragment.
```

## 7.2 Beautiful Denial

Triggered by suppressing evidence.

```text
The garden remains celebrated.
Food exposure continues.
Later Chronicle addendum is harsher.
```

## 7.3 Evidence Capture

Triggered if corporate remediation actor obtains exclusive evidence.

```text
Public learns the site failed through corporate sales framing.
Repair commons loses authority.
```

## 7.4 Witness Paralysis

Triggered if the player delays too long for perfect evidence.

```text
Produce reaches kitchens.
Mine-Scar Witness credibility rises but clinic burden increases.
```

---

# 8. Chronicle Outcomes

## Best Outcome

```text
The Garden Changed Its Promise
```

Summary:

```text
The player showed that the garden was unsafe as food infrastructure but valuable as dust and soil stabilization.
The site was converted under witness rather than abandoned or hidden.
```

## Scandal Outcome

```text
The Green Cover Lie
```

Summary:

```text
A public garden hid toxic uptake beneath visible beauty.
Ecological repair legitimacy collapsed in the district.
```

## Denial Outcome

```text
False-Clean Harvest
```

Summary:

```text
Contaminated crops remained in circulation after warning signs were ignored.
```

## Corporate Capture Outcome

```text
The Salesman Read the Soil First
```

Summary:

```text
A corporate remediator used real contamination evidence to discredit commons repair.
```

---

# 9. Acceptance Test

The mission succeeds if the player can say:

```text
I did not prove the green was fake.
I proved which green was repair, which green was theater, and how to keep the living part.
```
