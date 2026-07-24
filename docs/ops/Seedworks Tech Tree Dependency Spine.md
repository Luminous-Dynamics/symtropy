---
title: Seedworks Tech Tree Dependency Spine
version: 0.1
status: canonical-draft
milestone: seedworks-v0.1-to-v1.0
scope: technology dependencies, unlock logic, production planning
owner: design/engineering
recommended_path: docs/seedworks/00_canon/TECH_TREE_DEPENDENCY_SPINE.md
---
# Symtropy: Seedworks Tech Tree Dependency Spine

## Working Title

**Nothing Unlocks Alone**

## Purpose

This document defines the causal dependency structure beneath the Seedworks tech tree.

The existing unlock table says when systems appear.

This document explains what each system requires before it becomes playable, safe, legal, and meaningful.

Core rule:

```text
A technology unlocks only when the world can support its body, power, computation, legitimacy, maintenance, and failure.
```

---

# 1. Core Dependency Grammar

Every unlock should be checked against six dependency categories.

```text
MATERIAL:
Can the settlement physically build it?

POWER:
Can the settlement power it without destabilizing other infrastructure?

COMPUTATION:
Can the Field Deck, Device Bus, or controller runtime operate it?

LEGITIMACY:
Is it authorized by charter, witness, public vote, or emergency order?

MAINTENANCE:
Can it be repaired, inspected, and logged after failure?

CONSEQUENCE:
Does its use create Chronicle-worthy state change?
```

Design rule:

```text
A tool that cannot fail meaningfully is not ready for Symtropy.
```

---

# 2. Seedworks Dependency Layers

The tech tree should be treated as eight interlocking layers.

```text
Layer 0 — Survival Repair
Layer 1 — Field Deck / Device Bus
Layer 2 — Cargo and Material Ledgers
Layer 3 — Power / Audio / Labor Substrate
Layer 4 — Public Fabrication
Layer 5 — Robotics and Automation
Layer 6 — Civic and Faction Infrastructure
Layer 7 — Xeno-Translation and Living Infrastructure
Layer 8 — Regional / Interstellar Expansion
```

Each layer depends on earlier layers, but later layers can also reveal weaknesses in earlier ones.

Example:

```text
Robotics requires power.
Robotics also creates new power failures.

Xeno-hybrid filters require civic legitimacy.
They also create new legitimacy disputes.
```

Design rule:

```text
Later tech should not replace earlier systems.
It should make their consequences richer.
```

---

# 3. Layer 0 — Survival Repair

## Purpose

Prove that one physical repair can carry the game.

## Core Unlocks

```text
Basic Repair Tool
Patch Cable
Copper Conduit Pipe Segment
Patch Conduit Mk0
Ceramic Seal
Flooded Storage Crate
Manual Carry
Panic Drop
```

## Dependencies

```text
MATERIAL:
salvaged conduit, seal, basic hand tools

POWER:
none or low

COMPUTATION:
Field Deck Mk0 inspection only

LEGITIMACY:
local emergency need

MAINTENANCE:
manual inspection

CONSEQUENCE:
water either moves or does not
```

## Unlocks Next

```text
Field Deck Device Bus registration
Archive Witness Cartridge
Proof-of-Repair
Public Works Fabrication Bench foreshadow
```

Design rule:

```text
The first repair must be understandable without the full civilization system.
```

---

# 4. Layer 1 — Field Deck / Device Bus

## Purpose

Make infrastructure legible.

## Core Unlocks

```text
Field Deck Mk0
SCAN
DIAG
ARCHIVE
CIVIC
NULL stub
Local Device Bus Shell
Device Bus Node Registration
```

## Dependencies

```text
MATERIAL:
Field Deck body, patch cable, terminal port

POWER:
battery or local terminal power

COMPUTATION:
read/initialize/authorize commands

LEGITIMACY:
operator identity and local repair context

MAINTENANCE:
source-chain recovery after death or device loss

CONSEQUENCE:
player sees that machines have physical, historical, and civic states
```

## Unlocks Next

```text
Chronicle JSONL v0
Archive Witness Cartridge
Registered Infrastructure Adjudication
Proof-of-Repair
```

Design rule:

```text
The Field Deck should not be magic vision.
It should be an accountable instrument.
```

---

# 5. Layer 2 — Cargo and Material Ledgers

## Purpose

Make logistics physical.

## Core Unlocks

```text
Physical Cargo Carry
Flooded Storage Crate Manifest
Cargo Condition States
Panic Drop
Cargo Ledger Audit stub
```

## Dependencies

```text
MATERIAL:
physical cargo objects, crates, seals, manifests

POWER:
minimal unless container is active or cold-chain

COMPUTATION:
manifest readout, condition state, divergence flag

LEGITIMACY:
ownership, public need, evidence status

MAINTENANCE:
recount, flag, quarantine, transfer

CONSEQUENCE:
cargo movement changes repair quality and civic trust
```

## Unlocks Next

```text
Sky-Hook gantry
Cargo Ledger Audit Station
Cold-Chain Vault
Scavenger Unbuilder
Public Tool Library
```

Design rule:

```text
Inventory becomes gameplay when carrying the part changes the story.
```

---

# 6. Layer 3 — Power / Audio / Labor Substrate

## Purpose

Make infrastructure truth measurable beneath the visible repair.

## Core Unlocks

```text
Thermodynamic Power Readout
Voltage Sag Effect
Pump Audio Diagnostic
Substrate Summary Page
Proof-of-Repair Receipt
```

## Dependencies

```text
MATERIAL:
transformer housing, pump casing, sensor ports

POWER:
local grid state

COMPUTATION:
Device Bus substrate nodes

LEGITIMACY:
witness basis for repair record

MAINTENANCE:
diagnostic readouts and repair evidence

CONSEQUENCE:
repair succeeds, delays, or becomes disputed based on substrate truth
```

## Unlocks Next

```text
Power Graph Readout
WASM Clock Drift
mk0-aegis Acoustic Boundary
Proof-of-Repair Redemption
Public Works Fabrication Bench
```

Design rule:

```text
The pump should not only be broken.
It should be able to explain how it is broken.
```

---

# 7. Layer 4 — Public Fabrication

## Purpose

Turn one repair into repeatable settlement capacity.

## Core Unlocks

```text
Public Works Fabrication Bench
Settlement Fabricator Bay
Certified Pipe Gauge
Certified Seal Kit
Pressure Test Rig
Biofilter Housing
Basic Transformer Repair Kit
Public Tool Library
```

## Dependencies

```text
MATERIAL:
tooling, parts, seals, benches, clean workspace

POWER:
stable transformer and battery support

COMPUTATION:
repair validation, fabrication recipes, QA state

LEGITIMACY:
Proof-of-Repair accepted by charter

MAINTENANCE:
certification and inspection loop

CONSEQUENCE:
settlement can repeat repair without relying on heroics
```

## Unlocks Next

```text
Robotics Mk0 platforms
Cold-Chain Vault
Cargo Ledger Audit Station
Settlement Vote
Repair Worker Access Class
```

Design rule:

```text
Fabrication is not crafting.
It is the settlement learning to trust its own hands again.
```

---

# 8. Layer 5 — Robotics and Automation

## Purpose

Make infrastructure embodied, mobile, and politically accountable.

## Core Unlocks

```text
mk0-scout Cable-Crawler
mk0-gantry Sky-Hook
mk0-aegis Acoustic Boundary
mk0-agora Civic Kiosk
mk0-biota Perimeter Sentinel
agribot Soil Steward
scavenger Unbuilder
mk0.5-tooling-cluster
```

## Dependencies

```text
MATERIAL:
motors, frames, gears, bearings, rails, sensors

POWER:
battery chargers, safe voltage, thermal limits

COMPUTATION:
autonomy level, Device Bus identity, route logging

LEGITIMACY:
permission envelope, public works license, privacy boundary, witness rules

MAINTENANCE:
repair bench, spare parts, operator training

CONSEQUENCE:
robots create evidence, safety, labor disputes, and civic precedent
```

## Unlocks Next

```text
Machine Testimony
Machine Stewardship faction path
Regional Technician Passport
Precision Manufacturing
Rover Med-Bay
```

Design rule:

```text
The first robot should extend sight, carrying, listening, or witnessing — not replace the player.
```

---

# 9. Layer 6 — Civic and Faction Infrastructure

## Purpose

Make technology socially consequential.

## Core Unlocks

```text
Settlement Public Vote
Registered Infrastructure Adjudication
Rights Floor Review
Repair Worker Access Class
Faction Archetype Vector
Public Tool Library Governance
Machine Testimony Review
```

## Dependencies

```text
MATERIAL:
public terminals, charters, kiosks, meeting spaces

POWER:
stable enough for public recordkeeping

COMPUTATION:
Chronicle, vote logs, credential tokens

LEGITIMACY:
public access, witness rules, dispute procedures

MAINTENANCE:
audit, appeal, charter amendment

CONSEQUENCE:
settlement evolves toward different faction futures
```

## Unlocks Next

```text
Inter-Settlement Recognition
Regional Proof-of-Repair
Multi-Species Rights Forum
Shared Tool Embassy
```

Design rule:

```text
A technology is mature when people can dispute its use without breaking the world.
```

---

# 10. Layer 7 — Xeno-Translation and Living Infrastructure

## Purpose

Introduce alien technology as negotiated compatibility, not loot.

## Core Unlocks

```text
Shared Tool Embassy
Translation Pool
Metabolic Stabilizer
Rights Forum Terminal
Hybrid Filter Alpha
Bio-Electric Converter
Tideborn Chemical Memory Block
Aerosol Archive Cartridge
Lithic Resonance Coupler
Canopy Root Wrapper Script
```

## Dependencies

```text
MATERIAL:
xeno-compatible containment, filters, membranes, resonance couplers

POWER:
bio-electric conversion, stable thermal envelope

COMPUTATION:
translation runtime, wrapper scripts, signal mapping

LEGITIMACY:
Multi-Species Rights Forum, consent token, living infrastructure license

MAINTENANCE:
quarantine, metabolic care, translation recalibration

CONSEQUENCE:
human infrastructure becomes partly non-human and politically alive
```

## Unlocks Next

```text
Metabolic Exchange Treaties
Hybrid Failure Crisis
Living Infrastructure Rights
Regional Xeno-Trade
```

Design rule:

```text
Alien technology is not acquired.
It is hosted under obligation.
```

---

# 11. Layer 8 — Regional / Interstellar Expansion

## Purpose

Scale the repair grammar beyond one settlement.

## Core Unlocks

```text
Inter-Settlement Recognition
Regional Technician Passport
Trusted Cargo Corridor
Atlas Gate foreshadow
Worldline Translation foreshadow
Orbital / Oceanic / Deep-Space Robotics roadmap
```

## Dependencies

```text
MATERIAL:
transport infrastructure, cargo systems, regional power

POWER:
large-scale grid reliability

COMPUTATION:
mesh synchronization, source-chain portability, deterministic replay

LEGITIMACY:
cross-settlement charter recognition

MAINTENANCE:
regional repair contracts, dispute resolution, archive continuity

CONSEQUENCE:
player’s repair history travels and changes distant systems
```

Design rule:

```text
The stars should not open until one settlement can remember a pipe honestly.
```

---

# 12. Dependency Examples

## Example 1: Public Works Fabrication Bench

Requires:

```text
Patch Conduit Mk0 repaired
Chronicle event accepted
Proof-of-Repair issued
Field Deck WITNESS mode stub
stable enough power
public charter recognizes repair
```

Unlocks:

```text
Certified Pipe Gauge
Certified Seal Kit
Pressure Test Rig
Public Tool Library
mk0-scout foreshadow becomes buildable
```

Failure mode:

```text
fabricated parts cannot be certified if witness chain is broken
```

---

## Example 2: mk0-scout Cable-Crawler

Requires:

```text
Public Works Fabrication Bench
basic motor repair
battery charger
overhead cable route
Field Deck remote view
public inspection permission
```

Unlocks:

```text
remote inspection
machine visual testimony
hazard marking
safer cargo recovery
Machine Stewardship pressure
```

Failure mode:

```text
crawler log is accurate but rejected because route authorization was incomplete
```

---

## Example 3: Hybrid Filter Alpha

Requires:

```text
Biofilter Housing
Public Works Fabrication Bench
Translation Pool
Shared Tool Embassy
Tideborn exchange
Rights Forum license
metabolic stabilizer
Proof-of-Repair recognized beyond Firstlight
```

Unlocks:

```text
high-efficiency water filtration
living infrastructure consent systems
Translation Collapse risk
Overgrowth Without Consent failure state
```

Failure mode:

```text
filter keeps water clean but begins refusing human command access after consent boundary is violated
```

---

# 13. Production Rule

Every new tech-tree entry must be written in this format:

```text
Technology:
Discipline:
Milestone:
Dependency Layer:
Required Material:
Required Power:
Required Computation:
Required Legitimacy:
Required Maintenance:
Player Verb:
Failure Mode:
Chronicle Trigger:
Unlocks Next:
```

Example:

```text
Technology:
mk0-scout Cable-Crawler

Discipline:
Robotics / Computational Field Architecture / Legitimacy Chains

Milestone:
v0.2

Dependency Layer:
Layer 5 — Robotics and Automation

Required Material:
crawler body, motor, overhead cable route

Required Power:
battery dock, transformer stable above safe threshold

Required Computation:
Field Deck remote view, route manifest

Required Legitimacy:
public inspection route authorization

Required Maintenance:
crawler dock, replacement wheel or clamp

Player Verb:
deploy, inspect, mark, recall

Failure Mode:
WITNESS_REJECTED

Chronicle Trigger:
machine visual log submitted to repair dispute

Unlocks Next:
Machine testimony review, Archive-certified inspection crawler
```

Design rule:

```text
No unlock without a verb.
No verb without a failure.
No failure without a record.
```

---

# 14. v1.0 Tech Tree Shape

Version 1.0 should be organized around seven mature arcs.

```text
Arc 1:
Repair → Public Works → Regional Infrastructure

Arc 2:
Field Deck → Device Bus → Source-Chain Portability

Arc 3:
Cargo → Ledger → Trusted Corridor

Arc 4:
Fabrication → Robotics → Autonomous Public Works

Arc 5:
Chronicle → Charter → Faction Evolution

Arc 6:
Biofilter → Shared Tool Embassy → Hybrid Living Infrastructure

Arc 7:
Settlement Repair → Regional Legitimacy → Interstellar Readiness
```

v1.0 is achieved when these arcs interlock.

```text
The player can repair infrastructure.
The settlement can reproduce the repair.
Robots can help without erasing accountability.
Factions can dispute technology without collapsing truth.
Alien systems can enter through consent rather than extraction.
Proof-of-Repair can travel.
The Chronicle remembers.
```

---

# 15. Final Principles

```text
Nothing unlocks alone.

Every technology is a bundle of matter, power, computation, legitimacy, maintenance, and consequence.

The tech tree is not a ladder of upgrades.
It is a living dependency graph.

Earlier systems must remain meaningful after later systems unlock.

Robots should not erase cargo.
Fabrication should not erase repair.
Alien tech should not erase human responsibility.
Interstellar scale should not erase local legitimacy.

The player does not climb the tech tree.
They stabilize one dependency after another until civilization can safely become more capable.
```

Final line:

```text
The future did not unlock because someone researched it.
It unlocked because the world could finally bear the weight of what it was about to become.
```
