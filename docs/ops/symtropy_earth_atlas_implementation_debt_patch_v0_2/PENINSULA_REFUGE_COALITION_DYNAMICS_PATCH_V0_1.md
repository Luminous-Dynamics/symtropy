---
title: Peninsula Refuge Coalition Dynamics Patch v0.1
status: canonical-patch
project: Symtropy
domain: Antarctica / Peninsula Refuge Cities / Faction Ecology
recommended_path: docs/earth-atlas/antarctica/PENINSULA_REFUGE_COALITION_DYNAMICS_PATCH_V0_1.md
patches:
  - PENINSULA_REFUGE_CITIES_TEXTURE_PASS_V0_1.md
---

# Peninsula Refuge Coalition Dynamics Patch v0.1

## Purpose

The Peninsula Refuge Cities texture pass defines four strong internal factions:

```text
Refuge Charter Advocates
Treaty Continuity Elders
Kitchen Protectionists
Aurora-Born Youth
```

This patch specifies how they align, split, and generate emergent conflict.

Goal:

```text
Make the Peninsula Refuge Cities feel like a living political ecology, not a flat debate stage.
```

---

# 1. Core Dynamic

The Peninsula Refuge Cities do not divide into simple pro-refuge and anti-refuge camps.

Every faction can ally with every other faction under specific pressures.

The central variable is not ideology.

It is:

```text
Which danger feels nearest?
```

Dangers:

```text
people freezing outside the law
Antarctica becoming owned
greenhouse collapse
ranger sovereignty drift
kitchen exclusion
resource extraction using refuge as excuse
aurora-born statelessness
newcomer overload
```

---

# 2. Faction Baselines

## 2.1 Refuge Charter Advocates

Baseline:

```text
Permanent life deserves permanent rights.
```

Common allies:

```text
Aurora-Born Youth
some Kitchen Councils
Peninsula clinic networks
Bridge lawyers
```

Common opponents:

```text
Treaty Continuity Elders
Non-Ownership Rangers
White Ledger Archivists when claims look sovereign
```

Failure mode:

```text
May accidentally create legal language that extraction interests can reuse.
```

## 2.2 Treaty Continuity Elders

Baseline:

```text
If refuge becomes ownership, the powerful will arrive behind it.
```

Common allies:

```text
White Ledger Archivists
Non-Ownership Rangers
some Kitchen Protectionists
Machine Archive custodians
```

Common opponents:

```text
Refuge Charter Advocates
Aurora-Born Youth
Resource lobby infiltrators
```

Failure mode:

```text
May protect Antarctica's abstraction while denying lived belonging.
```

## 2.3 Kitchen Protectionists

Baseline:

```text
Care must remain possible, or everyone suffers.
```

Common allies shift by crisis.

They ally with Refuge Charter Advocates when:

```text
new residents are already inside the heat circle
children need stable food rights
the kitchen needs legal protection from ranger closure
```

They ally with Treaty Continuity Elders when:

```text
new intake exceeds greenhouse capacity
festival fruit or child nutrition is at risk
they fear permanent overload
```

Failure mode:

```text
Care vocabulary becomes gatekeeping.
```

## 2.4 Aurora-Born Youth

Baseline:

```text
A place can be cared for by people who do not own it.
```

Common allies:

```text
Refuge Charter Advocates
greenhouse mechanics
young Rangers who fear sovereign drift
some Machine Archive reformers
```

Unexpected allies:

```text
Treaty Continuity Elders when resisting extraction
Kitchen Protectionists when defending child heat rights
```

Failure mode:

```text
Impatience can be exploited by actors who want permanent settlement as a path to ownership.
```

---

# 3. Coalition Patterns

## Pattern A — Humanitarian Expansion Coalition

Triggered by:

```text
storm refugee arrival
heat-grid surplus
clinic emergency
children outside first door
```

Coalition:

```text
Refuge Charter Advocates
Aurora-Born Youth
Kitchen Councils that have capacity
some clinics
```

Opposition:

```text
Treaty Continuity Elders
Rangers
capacity-focused Kitchen Protectionists
```

Risk:

```text
new housing may be classified as snowclaim
```

## Pattern B — Anti-Extraction Coalition

Triggered by:

```text
resource survey
military logistics proposal
corporate heat infrastructure offer
mineral access petition
```

Coalition:

```text
Treaty Continuity Elders
Aurora-Born Youth
Non-Ownership Rangers
White Ledger Archivists
some Refuge Charter Advocates
```

Opposition:

```text
desperate housing advocates tempted by infrastructure funding
external states
corporate utility actors
```

Risk:

```text
coalition may use anti-extraction fear to block legitimate refuge repairs
```

## Pattern C — Kitchen Closure Coalition

Triggered by:

```text
greenhouse disease
heat shortfall
food loop contamination
festival fruit collapse
```

Coalition:

```text
Kitchen Protectionists
Treaty Continuity Elders
clinic triage boards
some Rangers
```

Opposition:

```text
Refuge Charter Advocates
Aurora-Born Youth
recent arrivals
```

Risk:

```text
short-term care protection becomes class of people who never reach third-door belonging
```

## Pattern D — Ranger Drift Accountability Coalition

Triggered by:

```text
permit denial without appeal
ranger seizure of refuge materials
armed exclusion of children
patrol avoiding witness halls
```

Coalition:

```text
Aurora-Born Youth
Refuge Charter Advocates
some Treaty Continuity Elders
Machine Archive auditors
Field Deck witnesses
```

Opposition:

```text
Ranger hardliners
some Kitchen Protectionists
external treaty enforcers
```

Risk:

```text
weakening rangers may expose region to extraction actors
```

## Pattern E — Memory Without Territory Coalition

Triggered by:

```text
homeland shelf dispute
old nation-state archive claim
diaspora group requests protected memory space
```

Coalition:

```text
Treaty Continuity Elders
Refuge Charter Advocates
Homeland Shelf Teachers
Archive Witnesses
```

Opposition:

```text
Aurora-Born Youth who feel overburdened by lost places
Kitchen Protectionists who need space for present survival
```

Risk:

```text
memory can crowd out living need
```

---

# 4. Coalition State Schema

```rust
struct PeninsulaCoalitionState {
    humanitarian_pressure: f32,
    treaty_purity_pressure: f32,
    greenhouse_capacity: f32,
    ranger_drift: f32,
    extraction_threat: f32,
    aurora_born_belonging_pressure: f32,
    kitchen_gatekeeping_pressure: f32,
    machine_archive_trust: f32,
}
```

Coalition generation rule:

```text
Do not spawn fixed sides.
Spawn coalitions from the two highest pressures and one recent Chronicle event.
```

---

# 5. Example: Third Door Intake Crisis

Inputs:

```text
humanitarian_pressure: high
greenhouse_capacity: medium-low
ranger_drift: medium
recent_chronicle: Ranger denied refuge exception without public witness
```

Generated positions:

```text
Refuge Charter Advocates:
  admit family under provisional warmright

Aurora-Born Youth:
  admit family and challenge ranger authority

Kitchen Protectionists:
  admit children only unless food support is found

Treaty Continuity Elders:
  require non-claim language and public capacity record

Ranger Cadet:
  privately supports admission but fears snowclaim precedent
```

Potential player solution:

```text
secure temporary convoy food support
record non-claim refuge addendum
require ranger appeal hearing
grant third-door heat circle for children and medical guardian
```

---

# 6. Field Deck Coalition Readout

```sh
$ read /dev/sym/civic/peninsula/coalition_state

CURRENT CRISIS:
Third-Door Intake / heat and belonging dispute

DOMINANT PRESSURES:
humanitarian_pressure: high
greenhouse_capacity: strained
ranger_drift: medium

UNSTABLE COALITIONS:
Refuge Charter + Aurora-Born Youth:
  admit now

Kitchen Protectionists + Treaty Elders:
  admit only with capacity witness

Ranger Cadet Split:
  enforcement loyalty weakening

SYSTEMIC WARNING:
This is not a two-sided dispute.
A solution must answer heat, food, treaty, and appeal.
```

---

# 7. Acceptance Test

Peninsula coalition dynamics are ready when:

```text
1. The same faction can be ally or opponent depending on pressure.
2. Kitchen Protectionists sometimes defend refugees and sometimes restrict them.
3. Aurora-Born Youth can ally with Treaty Elders against extraction.
4. Treaty Elders can oppose Rangers when Rangers become sovereign.
5. A player solution must satisfy at least two pressures, not merely pick a side.
```

---

# 8. Final Line

```text
In the Peninsula Cities, everyone is afraid of becoming the thing that ruins refuge.
They disagree about which ruin comes first.
```
