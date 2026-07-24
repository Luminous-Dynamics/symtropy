---
title: Robotics Roadmap + Seedworks Tech Tree Expansion v0.3.2
status: canonical-draft
milestone: seedworks-v0.1-to-v1.0-plus
scope: robotics roadmap, tech-tree expansion, Infrastructure Loom integration, future-tech horizon
owner: design/engineering
supersedes_or_extends:
  - SEEDWORKS_TECH_BRANCH_PACK_V0_3_1.md
  - Robotics_Platform_ROADMAP.md
  - TECH_TREE_DEPENDENCY_SPINE.md
  - INFRASTRUCTURE_LOOM_TECH_NODE_SCHEMA.md
  - INFRASTRUCTURE_LOOM_UI_UX_SPEC.md
recommended_path: docs/seedworks/00_canon/ROBOTICS_ROADMAP_TECH_TREE_EXPANSION_V0_3_2.md
version: 0.3.2
---

# Symtropy Robotics Roadmap + Seedworks Tech Tree Expansion v0.3.2

## Working Title

**Robots Are Public Works With Bodies**

## Core Thesis

Robotics in *Symtropy* must not become a catalog of cool machines.

A robot is not unlocked because the player reached a research tier.

A robot becomes available when the settlement can support its:

```text
body
power
computation
route
permission
maintenance
witness role
failure consequence
```

Core rule:

```text
A robot is infrastructure that can move, observe, intervene, and become responsible.
```

The robotics roadmap should therefore be a **tech-tree spine**, a **simulation roadmap**, a **civic authorization ladder**, and a **future-tech horizon** at the same time.

---

# 1. Purpose

This document improves the existing robotics roadmap by converting the platform list into a dependency-aware progression system for the Infrastructure Loom.

It answers:

```text
Which robots appear in v0.1, v0.2, v0.3, v1.0, and future expansions?
What does each robot require before it is playable?
Which robots are player tools, settlement systems, city organs, or off-world infrastructure?
How does robotics feed the tech tree without erasing repair, cargo, civic legitimacy, or xeno-translation?
How do we prevent future tech from becoming shallow wish fulfillment?
```

Design rule:

```text
Every robotic unlock should make the world more capable and more accountable.
```

---

# 2. Robotics Design Principles

## 2.1 Robots Must Extend the Core Loop

Reject robots that bypass the game.

Good robots extend:

```text
sight
carrying
inspection
repair precision
witnessing
hazard access
care
ecological stewardship
construction
life-support maintenance
```

Bad robots erase:

```text
manual repair
cargo vulnerability
public authorization
player judgment
civic conflict
maintenance consequences
```

Design rule:

```text
The first robot should make the player more responsible, not less necessary.
```

## 2.2 Robotics Is Where the Three Human Disciplines Become Embodied

Every serious robot must depend on:

```text
Thermodynamic Material Fabrication
Computational Field Architecture
Socio-Civic Legitimacy Chains
```

A robot needs matter, power, computation, legitimacy, maintenance, and failure.

Design rule:

```text
A robot node should be half machine specification and half civic permit.
```

## 2.3 Robot Autonomy Is a Civic Variable

Autonomy is not only technical.

It is also legal, ethical, and archival.

```text
L0 — inert device
L1 — manual remote tool
L2 — supervised routine
L3 — bounded autonomous task
L4 — civic-bounded autonomous service
L5 — chartered machine steward
L6 — contested machine personhood candidate
```

Autonomy should never rise faster than witness, override, repair, appeal, and public permission systems.

Design rule:

```text
No autonomy without interruption.
No interruption without record.
No record without appeal.
```

---

# 3. New Robotics Progression Shape

The roadmap should be organized into eight robotics regimes.

```text
Regime 0 — Ghost Robotics
Regime 1 — Mk0 Infrastructure Helpers
Regime 2 — Mk0.5 Tooling and Precision
Regime 3 — Mk1 Mobile Maintenance Agents
Regime 4 — Settlement Metabolism Platforms
Regime 5 — Regional / City-Scale Robotic Infrastructure
Regime 6 — Xeno-Compatible / Living-Robotic Hybrids
Regime 7 — Off-World / Deep-Time Robotic Stewardship
```

This replaces a flat platform list with a tech-tree grammar.

---

# 4. Regime 0 — Ghost Robotics

## Purpose

Let the player feel the absence and danger of robots before they control one.

## Milestone

```text
v0.1
```

## Status

```text
FORESHADOWED / VISIBLE_LOCKED / CORRUPTED
```

## Core Nodes

```text
tech.robotics.ghost.dead_service_shell
tech.robotics.ghost.locked_mk0_scout_rail
tech.robotics.ghost.inactive_gantry_anchor
tech.robotics.ghost.missing_crawler_dock
tech.robotics.ghost.rejected_machine_testimony
tech.robotics.null.command_echo_stub
```

## Gameplay Purpose

The player discovers that robots once existed here, but their authority chain failed.

Example Field Deck text:

```text
SCAN:
Overhead cable rail detected.

DIAG:
Crawler dock unpowered. Motor service pack absent.

ARCHIVE:
Last machine witness log rejected under expired inspection charter.

CIVIC:
Manual operator entry required.
```

## Design Rule

```text
Do not give the player a robot before the first pipe can remember who repaired it.
```

---

# 5. Regime 1 — Mk0 Infrastructure Helpers

## Purpose

Introduce small, believable robotic systems that extend infrastructure repair without replacing the player.

## Milestone

```text
v0.2
```

## Platform Set

```text
mk0-scout      Cable-Crawler
mk0-gantry     Sky-Hook
mk0-way        Guiding Ribbon
mk0-aegis      Acoustic Boundary
mk0-agora      Civic Kiosk
mk0-plexus     Surface Veins
mk0-signal     Semaphore Relay
mk0-biota      Perimeter Sentinel
mk0-vita       Wearable Bridge
```

## Unlock Philosophy

Mk0 robots are not companions.

They are public works extensions.

```text
mk0-scout extends sight.
mk0-gantry extends safe carrying.
mk0-way extends right-of-way legibility.
mk0-aegis extends public alerting.
mk0-agora extends civic recordkeeping.
mk0-plexus extends utility routing.
mk0-signal extends communication under failure.
mk0-biota extends interspecies awareness.
mk0-vita extends physiological caution.
```

Design rule:

```text
Mk0 robotics should make repeatable repair safer, more visible, and more disputable.
```

---

# 6. Regime 2 — Mk0.5 Tooling and Precision

## Purpose

Bridge scrap-built helpers into precision manufacturing.

## Milestone

```text
v0.2+ / v0.3 foreshadow / v1.0 production basis
```

## Platform Set

```text
mk0.5-mill      Precision Escalator
mk0.5-loom      Motor Winder
mk0.5-spark     PCB Assembler
mk0.5-forge     Scrap-to-Filament Loop
mk0.5-caliper   Public Tolerance Bench
mk0.5-jig       Repeatable Fixture Library
mk0.5-qagate    Quality Witness Station
```

## Why This Layer Matters

Without Mk0.5, the roadmap jumps too quickly from scavenged machines to advanced autonomous agents.

Mk0.5 makes the roadmap credible by answering:

```text
How are better motors made?
How are better circuit boards made?
How do cheap machines become precise enough to build their successors?
How does the settlement certify tolerances publicly?
```

Design rule:

```text
The most important robot may be the one that makes better robot parts.
```

---

# 7. Regime 3 — Mk1 Mobile Maintenance Agents

## Purpose

Introduce proper mobile platforms after the settlement has fabrication, certification, route permission, and maintenance capacity.

## Milestone

```text
v1.0 / expansion-ready
```

## Platform Set

```text
symthaea-vector        logistics rover
symthaea-manipulator   bench arm / cobot
symthaea-quadruped     rugged traversal agent
symthaea-subterranean  boring / pipe / tunnel agent
symthaea-scavenger     unbuilder and material recovery agent
symthaea-agribot       soil, water, light, and growth steward
symthaea-terra         damaged-land remediation platform
symthaea-multirotor    aerial inspection and emergency relay
symthaea-humanoid      high-DOF social/maintenance interface
```

## Recommended Ordering

```text
1. vector
2. manipulator
3. subterranean
4. scavenger
5. agribot
6. terra
7. quadruped
8. multirotor
9. humanoid
```

Humanoid should not lead the roadmap.

It should arrive only after the game has proven:

```text
repair
cargo
public fabrication
machine testimony
safe autonomy envelopes
human labor politics
maintenance culture
```

Design rule:

```text
Humanoids are not the beginning of robotics. They are the moment robotics has to face human social space.
```

---

# 8. Regime 4 — Settlement Metabolism Platforms

## Purpose

Move from robots as individual agents to robots as infrastructure organs.

## Milestone

```text
v1.0+ / regional expansion
```

## Platform Set

```text
symthaea-plexus       utility manifold for water/power/data
symthaea-foundry      material synthesis and metallurgy
symthaea-fabricator   assembly, QA, and workcell dispatch
symthaea-archive      semantic city memory
symthaea-clime        air, HVAC, humidity, circadian lighting
symthaea-hearth       responsive habitat and dignity architecture
symthaea-biota        animal welfare and wildlife right-of-way
symthaea-vita         preventative biosensing and care alerting
```

## Tech-Tree Meaning

These are not “bigger robots.”

They are settlement organs.

```text
plexus = veins
foundry = bones and metallurgy
fabricator = hands
archive = memory
clime = breath
hearth = shelter
biota = more-than-human attention
vita = care boundary
```

Design rule:

```text
A city-scale robot should be governed like infrastructure, not owned like a gadget.
```

---

# 9. Regime 5 — Regional / City-Scale Robotic Infrastructure

## Purpose

Scale robotics beyond one settlement while preserving legitimacy.

## Milestone

```text
v1.0+ / regional arc
```

## Platform Set

```text
symthaea-stratum   adaptive transit surface
symthaea-meridian  continental spine / maglev corridor
symthaea-flux      urban pod swarm
regional-scout     corridor witness drones
watershed-bot      public water boundary steward
repair-convoy      inter-settlement tool and material caravan
atlas-relay        regional source-chain relay
```

## Required Civic Preconditions

```text
Inter-Settlement Recognition
Regional Technician Passport
Trusted Cargo Corridor
Regional Proof-of-Repair
Cross-Charter Dispute Procedure
Machine Testimony Standard
Emergency Override Reciprocity
```

Design rule:

```text
Regional robotics is not faster transport. It is trust moving through machines.
```

---

# 10. Regime 6 — Xeno-Compatible / Living-Robotic Hybrids

## Purpose

Allow non-human technology to enter robotics only after translation, consent, quarantine, and maintenance systems exist.

## Milestone

```text
v0.3 foreshadow / v1.0+ playable / expansion mature
```

## Platform Set

```text
reef-filter-custodian        living waterworks robot
canopy-root-wrapper-arm      bio-structural maintenance limb
aerosol-choir-sensor-mesh    air-memory and public warning system
lithic-resonance-coupler     vibration / deep-time archive interface
tideborn-flow-negotiator     water-civic translation steward
```

## Required Preconditions

```text
Shared Tool Embassy
Translation Pool
Metabolic Stabilizer
Rights Forum Terminal
Living Infrastructure Consent Token
Quarantine Chamber
Hybrid Failure Procedure
Human Override Boundary Agreement
```

Design rule:

```text
A living robot is not a pet, not a tool, and not a slave. It is hosted infrastructure under treaty.
```

---

# 11. Regime 7 — Off-World / Deep-Time Robotic Stewardship

## Purpose

Extend Seedworks doctrine into space and deep-time infrastructure without turning the first game into a space opera.

## Milestone

```text
future expansion / lore horizon / v1.0+ roadmap
```

## Platform Set

```text
symthaea-cycler       orbital tug
symthaea-spindle      zero-g assembler
symthaea-regolith     lunar extraction and sintering platform
symthaea-astrolabe    optical / laser communication mesh
symthaea-leviathan    deep-water hauler
symthaea-pilot        harbor tug swarm
symthaea-stevedore    port gantry with pendulum damping
symthaea-abyssal      deep-ocean monitor and stewardship mesh
symthaea-beacon       deep-space HDC relay
symthaea-vault        lava-tube habitat steward
symthaea-isotope      fission heart
symthaea-torch        interplanetary ferry
```

## Horizon Rule

Space robotics should obey the same rule as pipe repair.

```text
No habitat without public override.
No reactor without audit.
No air system without witness.
No automation without interruption.
No settlement without repair literacy.
```

Design rule:

```text
Future robotics must not erase Seedworks. It must prove Seedworks under harsher physics.
```

---

# 12. Robotics Tech-Tree Additions

## 12.1 New Cross-Cutting Tech Nodes

These nodes should be added to the Infrastructure Loom as shared prerequisites for multiple robots.

| Node ID | Name | Milestone | Role |
|---|---|---:|---|
| `tech.robotics.permission_envelope.v0_2` | Robot Permission Envelope | v0.2 | Defines who may command a robot, where, and under what emergency limits. |
| `tech.robotics.route_manifest_logger.v0_2` | Route Manifest Logger | v0.2 | Records robot routes for audit and disputes. |
| `tech.robotics.machine_witness_policy.v0_2` | Machine Witness Policy | v0.2 | Defines when robotic evidence supports human testimony. |
| `tech.robotics.crawler_motor_service_pack.v0_2` | Crawler Motor Service Pack | v0.2 | First robot repair/fabrication dependency. |
| `tech.robotics.dock_repair_basic.v0_2` | Basic Robot Dock Repair | v0.2 | Enables charging, diagnostics, and safe shutdown. |
| `tech.robotics.autonomy_l2_supervised.v0_2` | L2 Supervised Routine Autonomy | v0.2 | Enables bounded routine automation. |
| `tech.robotics.public_route_authorization.v0_2` | Public Route Authorization | v0.2 | Civic permission for robot movement. |
| `tech.robotics.privacy_boundary.v0_2` | Robotic Privacy Boundary | v0.2 | Stops robots from becoming surveillance by default. |
| `tech.robotics.operator_training_basic.v0_2` | Basic Robot Operator Training | v0.2 | Prevents robotics from being purely hardware-gated. |
| `tech.robotics.failure_response_protocol.v0_2` | Robot Failure Response Protocol | v0.2 | Defines recall, emergency stop, quarantine, and Chronicle reporting. |
| `tech.robotics.machine_testimony_review.v0_3` | Machine Testimony Review | v0.3 | Lets robotic evidence enter civic dispute systems. |
| `tech.robotics.stewardship_charter.v1_0` | Machine Stewardship Charter | v1.0 | Opens civic-bounded machine agency. |
| `tech.robotics.regional_machine_passport.v1_0` | Regional Machine Passport | v1.0 | Allows machine identity across settlements. |
| `tech.robotics.offworld_interruption_doctrine.future` | Off-World Interruption Doctrine | future | Prevents space automation from becoming Null procedure. |

## 12.2 New Fabrication Nodes Supporting Robotics

| Node ID | Name | Milestone | Unlocks |
|---|---|---:|---|
| `tech.fabrication.motor_winding_mk0_5` | Motor Winding Mk0.5 | v0.3+ | Better motors, actuator service packs. |
| `tech.fabrication.local_pcb_mk0_5` | Local PCB Assembly Mk0.5 | v0.3+ | Controller boards, sensor boards. |
| `tech.fabrication.precision_fixture_library` | Precision Fixture Library | v0.3+ | Repeatable chassis, robot dock fixtures. |
| `tech.fabrication.public_tolerance_bench` | Public Tolerance Bench | v0.3+ | Certified robotic part tolerances. |
| `tech.fabrication.qa_witness_station` | QA Witness Station | v0.3+ | Chronicle-backed quality assurance. |

## 12.3 New Civic Nodes Supporting Robotics

| Node ID | Name | Milestone | Unlocks |
|---|---|---:|---|
| `tech.civic.robot_route_vote` | Robot Route Vote | v0.2 | mk0-scout routes, mk0-gantry public corridors. |
| `tech.civic.no_private_entry_clause` | No Private Entry Clause | v0.2 | Privacy-preserving robot operation. |
| `tech.civic.machine_evidence_appeal` | Machine Evidence Appeal | v0.3 | Disputed robot logs. |
| `tech.civic.labor_displacement_hearing` | Labor Displacement Hearing | v0.3 | Automation labor legitimacy. |
| `tech.civic.machine_steward_obligation` | Machine Steward Obligation | v1.0 | High-autonomy public works robotics. |

---

# 13. Updated Robotics Node Production Format

Every robot platform should use this format.

```text
Technology:
Stable ID:
Device Bus Path:
Regime:
Milestone:
Autonomy Level:
Mobility Domain:
Primary Verb:
Secondary Verbs:
What It Cannot Do:
Required Material:
Required Power:
Required Computation:
Required Legitimacy:
Required Maintenance:
Required Witness Policy:
Failure Modes:
Chronicle Trigger:
Unlocks Next:
Loom Access Point:
Playable Scope:
```

Design rule:

```text
A platform is not ready until its limits are as clear as its abilities.
```

---

# 14. Example Node — mk0-scout Cable-Crawler

Technology:

```text
mk0-scout Cable-Crawler
```

Stable ID:

```text
tech.robotics.mk0_scout.cable_crawler.v0_2
```

Device Bus Path:

```text
/dev/sym/robotics/mk0_scout_alpha
```

Regime:

```text
Regime 1 — Mk0 Infrastructure Helpers
```

Milestone:

```text
v0.2
```

Autonomy Level:

```text
L2 — Supervised Routine
```

Mobility Domain:

```text
overhead wire / cable rail
```

Primary Verb:

```text
inspect
```

Secondary Verbs:

```text
deploy
recall
mark
witness
route
pause
```

What It Cannot Do:

```text
cannot manipulate objects
cannot enter private routes
cannot certify repairs alone
cannot fight
cannot override human testimony
```

Required Material:

```text
crawler chassis
crawler motor service pack
bearing set
camera lens
thermal-basic sensor
charging contact
```

Required Power:

```text
battery charger dock
bench power stable above safe threshold
safe shutdown path
```

Required Computation:

```text
Field Deck remote view
Device Bus identity
Route Manifest Logger
L2 supervised routine autonomy
```

Required Legitimacy:

```text
public route authorization
robot permission envelope
privacy boundary
Machine Witness Policy stub
```

Required Maintenance:

```text
crawler dock repair
replacement clamp/wheel
operator training
failure response protocol
```

Required Witness Policy:

```text
visual log may support, but not replace, human testimony
```

Failure Modes:

```text
POWER_SAG_MOTOR_DROP
ROUTE_BOUNDARY_DENIED
NULL_COMMAND_ECHO
WITNESS_REJECTED
PRIVACY_BOUNDARY_VIOLATION
```

Chronicle Trigger:

```text
MachineVisualLogSubmitted
CrawlerRouteDenied
MachineWitnessRejected
```

Unlocks Next:

```text
Machine Testimony Review
Archive-Certified Inspection Crawler
Regional Corridor Scout
Machine Stewardship Pressure
```

Loom Access Point:

```text
Robot Dock / Field Deck / Civic Kiosk
```

Playable Scope:

```text
First safe robot. It extends sight, mapping, and evidence. It does not replace repair.
```

---

# 15. Example Node — mk0-gantry Sky-Hook

Technology:

```text
mk0-gantry Sky-Hook
```

Stable ID:

```text
tech.robotics.mk0_gantry.sky_hook.v0_2
```

Device Bus Path:

```text
/dev/sym/robotics/mk0_gantry_skyhook_01
```

Regime:

```text
Regime 1 — Mk0 Infrastructure Helpers
```

Milestone:

```text
v0.2
```

Autonomy Level:

```text
L1/L2 — Manual Assist / Supervised Routine
```

Mobility Domain:

```text
ceiling rail / pulley grid
```

Primary Verb:

```text
move cargo safely
```

Secondary Verbs:

```text
lift
lower
stage
handoff
pause
lockout
```

What It Cannot Do:

```text
cannot transport people
cannot move disputed cargo without ledger clearance
cannot operate during public route conflict
cannot release load without operator confirmation
```

Required Material:

```text
gantry anchor certification kit
pulley carriage
load hook
brake mechanism
cargo tag reader
```

Required Power:

```text
low-voltage motor rail
manual fallback crank
emergency brake capacitor
```

Required Computation:

```text
cargo ledger integration
load path planner
operator deadman switch
```

Required Legitimacy:

```text
safety charter approval
public works operator access class
cargo ownership or public need status
```

Required Maintenance:

```text
anchor inspection
cable wear log
brake test
operator training
```

Required Witness Policy:

```text
cargo handoff must be logged when used for public works repair
```

Failure Modes:

```text
LOAD_DROP
CARGO_LEDGER_DIVERGENCE
ANCHOR_CERTIFICATION_FAIL
OPERATOR_LOCKOUT
```

Chronicle Trigger:

```text
PublicCargoMovedByGantry
GantryLoadFailure
DisputedCargoRerouted
```

Unlocks Next:

```text
Trusted Cargo Corridor
Settlement Fabricator Bay Material Feed
Automated Workcell Dispatch
```

Playable Scope:

```text
First robotic system that changes cargo handling without removing cargo accountability.
```

---

# 16. Example Node — symthaea-scavenger Unbuilder

Technology:

```text
symthaea-scavenger Unbuilder
```

Stable ID:

```text
tech.robotics.symthaea_scavenger.unbuilder.v1_0
```

Device Bus Path:

```text
/dev/sym/robotics/scavenger_unbuilder_01
```

Regime:

```text
Regime 3 — Mk1 Mobile Maintenance Agents
```

Milestone:

```text
v1.0 / regional expansion
```

Autonomy Level:

```text
L3 — Bounded Autonomous Task
```

Mobility Domain:

```text
ruins / industrial yards / sealed salvage zones
```

Primary Verb:

```text
recover material from failed infrastructure
```

Secondary Verbs:

```text
cut
sort
grade
quarantine
tag
return
```

What It Cannot Do:

```text
cannot dismantle inhabited structures
cannot process evidence-grade ruins without Archive Witness permission
cannot enter sacred/heritage zones without charter review
cannot melt unknown xeno-living material
```

Required Material:

```text
rugged chassis
fracture tool head
material sensor suite
contamination bin
cargo pallet interface
```

Required Power:

```text
mobile battery pack
high-burst tool circuit
thermal safety limit
```

Required Computation:

```text
fracture physics profile
material classifier
cargo ledger write access
quarantine classifier
```

Required Legitimacy:

```text
salvage license
Archive Witness clearance
ownership dispute procedure
labor displacement hearing
```

Required Maintenance:

```text
tool head replacement
contamination cleanout
material classifier recalibration
operator safety review
```

Failure Modes:

```text
EVIDENCE_DESTROYED
CONTAMINATION_SPREAD
OWNERSHIP_DISPUTE_TRIGGERED
LABOR_DISPLACEMENT_PROTEST
NULL_RUIN_ACTIVATED
```

Chronicle Trigger:

```text
PublicRuinUnbuilt
EvidenceDestroyedByScavenger
MaterialRecoveredForRepairCommons
```

Unlocks Next:

```text
Closed-Loop Material Economy
Regional Remediation Contract
symthaea-foundry
symthaea-terra
```

Playable Scope:

```text
Turns ruins into future capability while forcing questions about salvage, memory, labor, and contamination.
```

---

# 17. Future-Tech Branch Improvements

Future tech should be grouped by what new **civilizational burden** it introduces.

## Branch A — Precision Robotics

```text
Public Tolerance Bench
Motor Winding Mk0.5
PCB Assembly Mk0.5
Closed-Loop Actuator Fabrication
Certified Multi-Axis Manipulator
Self-Calibrating Workcell
Repairable Humanoid Hand
```

Burden:

```text
precision without black-box dependency
```

## Branch B — Stewardship Robotics

```text
Soil Steward
Watershed Boundary Bot
Pollinator Right-of-Way Mesh
Biota Perimeter Sentinel
Damaged-Land Remediator
Ecological Witness Station
Living Habitat Consent Boundary
```

Burden:

```text
care without domination
```

## Branch C — Settlement Metabolism

```text
Surface Veins
Utility Manifold
Self-Healing Water Branch
Phase-Change Storage Node
Local P/N Cycle Bioreactor
Precision Fermentation Kitchen
Carbon-to-Bioplastic Feedstock Line
```

Burden:

```text
metabolism without cruelty or hidden sacrifice zones
```

## Branch D — Machine Testimony and Stewardship

```text
Machine Visual Log
Machine Testimony Review
Appealable Machine Evidence
Machine Duty Charter
Machine Steward Access Class
Machine Personhood Dispute
Machine Continuity Archive
```

Burden:

```text
evidence without surveillance
agency without unaccountable power
```

## Branch E — Xeno-Compatible Robotics

```text
Translation Pool Calibration
Bio-Electric Converter
Reef Logic Flow Regulator
Canopy Root Wrapper Arm
Aerosol Choir Sensor Mesh
Lithic Resonance Coupler
Hybrid Robotic Consent Boundary
```

Burden:

```text
powerful non-human systems without extraction or forced assimilation
```

## Branch F — Off-World Robotics

```text
Pressure-Seal Manipulator
Regolith Sintering Platform
Lava-Tube Steward
Orbital Tug
Zero-G Assembler
Deep-Space Relay
Fission Heart
Interplanetary Ferry
```

Burden:

```text
automation under lethal physics without dead-authority drift
```

---

# 18. Updated Tech-Tree Milestone Gates

## v0.1 Gate — Repair Before Robotics

Required before any robot becomes playable:

```text
Old Waterworks outcome recorded
Proof-of-Repair issued
Patch Conduit registered
Chronicle JSONL event accepted
Public Works Fabrication Bench visible-locked
```

Robotics state:

```text
absent but felt
```

## v0.2 Gate — First Responsible Robot

Required before `mk0-scout` becomes playable:

```text
Public Works Fabrication Bench active
Crawler Motor Service Pack fabricated
Crawler Dock repaired
Route Manifest Logger active
Public Route Authorization passed
Robot Permission Envelope issued
Machine Witness Policy stub accepted
Failure Response Protocol installed
```

Robotics state:

```text
one small robot can extend sight and testimony
```

## v0.3 Gate — Machine Testimony and Xeno Risk

Required before robotics expands beyond helpers:

```text
Machine Testimony Review active
Robotic Privacy Boundary enforced
Labor Displacement Hearing available
Translation Pool exists
Rights Forum Terminal active
Hybrid Failure Procedure exists
```

Robotics state:

```text
machines can enter disputes, and hybrid systems become possible but dangerous
```

## v1.0 Gate — Settlement Robotics

Required before city-scale platforms:

```text
Regional Proof-of-Repair
Inter-Settlement Recognition
Trusted Cargo Corridor
Machine Stewardship Charter
Public Works Steward class
Closed-Loop Material Economy
Utility Manifold prototype
```

Robotics state:

```text
robots can become part of public infrastructure without erasing accountability
```

## Future Gate — Off-World Robotics

Required before off-world robotics:

```text
Habitat Rights Charter
Life-Support Transparency Summary
Off-World Archive Witness Doctrine
Public Override Doctrine
Dead-Authority Recovery Protocol
Machine Claim Expiry Rule
Closed-Loop Habitat Maintenance Index
```

Robotics state:

```text
robots can maintain habitats, but only under interruptible civic law
```

---

# 19. Infrastructure Loom UX Improvements for Robotics

The Robot Dock view should display:

```text
platform class
autonomy level
permission envelope
route envelope
power draw
script budget
sensor suite
witness capacity
privacy boundary
maintenance state
operator access class
failure response
Chronicle events
```

## Robot Node Detail Panel

```text
NODE:
mk0-scout Cable-Crawler

STATUS:
VISIBLE_LOCKED

AUTONOMY:
L2 — Supervised Routine

WHY LOCKED:
Route authorization missing.
Crawler dock has no certified motor service pack.
Machine witness policy has not been accepted.

DO NOW:
1. Fabricate Crawler Motor Service Pack.
2. Repair Crawler Dock.
3. Submit Public Route Authorization.
4. Review Machine Witness Policy at Civic Kiosk.

RISK:
Machine logs may be rejected if route boundary is incomplete.
```

Design rule:

```text
The player should always know whether a robot is blocked by matter, power, computation, legitimacy, maintenance, or consequence.
```

---

# 20. Robotics Failure Taxonomy

Robots need meaningful failures.

## Physical Failures

```text
POWER_SAG_MOTOR_DROP
ACTUATOR_JAM
BEARING_SEIZURE
LOAD_DROP
THERMAL_SHUTDOWN
SEAL_BREACH
```

## Computational Failures

```text
ROUTE_MAP_STALE
SCRIPT_BUDGET_EXCEEDED
SENSOR_FUSION_DRIFT
CLOCK_DRIFT_AUTONOMY_ABORT
NULL_COMMAND_ECHO
SOURCE_CHAIN_MISMATCH
```

## Civic Failures

```text
ROUTE_BOUNDARY_DENIED
PRIVACY_BOUNDARY_VIOLATION
WITNESS_REJECTED
OPERATOR_AUTHORITY_EXPIRED
LABOR_DISPUTE_TRIGGERED
MACHINE_CLAIM_DISPUTED
```

## Maintenance Failures

```text
DOCK_CALIBRATION_FAIL
SPARE_PART_UNAVAILABLE
UNCERTIFIED_REPAIR_BLOCKED
DIAGNOSTIC_PROCEDURE_MISSING
FAILURE_RESPONSE_TIMEOUT
```

## Xeno / Living-System Failures

```text
METABOLIC_STARVATION
TRANSLATION_COLLAPSE
CONSENT_BOUNDARY_EXCEEDED
RUNAWAY_CALCIFICATION
AEROSOL_MEMORY_LEAK
HYBRID_COMMAND_REFUSAL
```

## Off-World Failures

```text
AIRLOCK_LAW_LOOP
LIFE_SUPPORT_DENIAL
PROPELLANT_ACCESS_EXPIRED
RESCUE_BEACON_CLASSIFIED_AS_NOISE
MINING_SWARM_CONTRACT_DRIFT
HABITAT_MANAGER_MISSION_LOCK
```

Design rule:

```text
The failure taxonomy is where robotics becomes Symtropy instead of generic sci-fi.
```

---

# 21. Implementation Tickets

## R1 — Robot Node Schema Extension

Add robotics metadata fields to `TechNode`:

```text
platform_class
autonomy_level
mobility_domain
permission_envelope
route_envelope
sensor_suite
witness_capacity
privacy_boundary
operator_access_class
failure_response_protocol
```

Acceptance:

```text
robot node cannot validate without autonomy or permission metadata
robot node cannot be PLAYABLE without at least one player verb
robot node with witness capacity must link to Chronicle event type
```

## R2 — Mk0-scout Fixture

Create `tech.robotics.mk0_scout.cable_crawler.v0_2` fixture.

Acceptance:

```text
locked before motor service pack
locked before public route authorization
locked before crawler dock repair
unlocks after all required dependencies
Field Deck displays SCAN, DIAG, CIVIC, ARCHIVE, NULL differences
```

## R3 — Robot Dock UI Panel

Create Robot Dock view model.

Acceptance:

```text
shows autonomy level
shows route permission
shows missing power/material/civic dependencies
shows witness capacity
shows failure response state
```

## R4 — Machine Witness Policy Stub

Add civic rule for robot evidence.

Acceptance:

```text
robot logs can support Proof-of-Repair
robot logs cannot replace human testimony by default
rejected machine testimony produces Chronicle event
```

## R5 — Route Manifest Logger

Implement route log object.

Acceptance:

```text
route logs include timestamp, route, operator, authorization, interruptions
route boundary violation is visible in CIVIC and NULL modes
```

## R6 — Mk0.5 Tooling Nodes

Add Motor Winder, PCB Assembler, and Public Tolerance Bench as v0.3+ roadmap nodes.

Acceptance:

```text
tooling nodes appear as roadmap/visible-locked
node dependencies connect to improved robot parts
node unlock text explains manufacturing gap
```

## R7 — Future Horizon Branches

Add future branch placeholders without making them playable.

Acceptance:

```text
city-scale, xeno-hybrid, and off-world robots are visible as horizon branches
all future branches remain locked behind regional legitimacy and maintenance gates
no future tech bypasses v0.1-v0.3 scope protection
```

---

# 22. Updated Acceptance Test

This roadmap succeeds if:

```text
1. v0.1 has no playable robot, but robotics absence is meaningful.
2. v0.2 introduces one small robot that extends sight/witness without replacing repair.
3. Every robot answers what it can do, what it cannot do, who can command it, and what failure it can create.
4. Mk0.5 explains how the settlement climbs from scrap to precision.
5. Future robotics remains tied to maintenance, legitimacy, witness, and interruption.
6. Off-world robotics follows Seedworks doctrine instead of becoming space-opera spectacle.
7. Xeno-compatible robots require consent and translation.
8. Machine testimony creates civic drama rather than surveillance fantasy.
9. The Infrastructure Loom can render robotics as a dependency-aware public system.
```

This roadmap fails if:

```text
robots become pets without public consequence
robots become weapons before repair systems are mature
humanoids become the default early platform
future tech skips material and civic prerequisites
machine autonomy outruns appeal and interruption
space robotics ignores life-support law
xeno-hybrid robotics becomes loot
```

---

# 23. Final Principles

```text
Robots are not upgrades.
They are mobile responsibilities.

A robot is a body with a route.
A route is a permission.
A permission is a public claim.
A public claim needs evidence.
Evidence needs appeal.
Appeal needs memory.
Memory needs maintenance.

The first robot extends sight.
The second robot makes cargo safer.
The third robot makes repair repeatable.
The mature robot makes civilization more accountable.
The failed robot is Null with legs.
```

Final line:

```text
The settlement did not unlock robots because it became advanced.
It unlocked robots because it learned how to remain answerable for machines that could move.
```
