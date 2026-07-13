---
title: Seedworks Tech Tree Audit and Horizon Gates v0.3.3
status: canonical-draft
milestone: seedworks-v0.1-to-future-horizon
scope: tech-tree validation, horizon gates, robotics permission envelopes, corruption branches, Chronicle event map, Loom UI requirements
owner: design/engineering/worldbuilding
extends:
  - SEEDWORKS_TECH_BRANCH_PACK_V0_3_1.md
  - ROBOTICS_ROADMAP_TECH_TREE_EXPANSION_V0_3_2.md
  - TECH_TREE_DEPENDENCY_SPINE.md
  - INFRASTRUCTURE_LOOM_TECH_NODE_SCHEMA.md
  - INFRASTRUCTURE_LOOM_UI_UX_SPEC.md
  - ROBOTICS_PLATFORM_ROADMAP.md
  - WORLD_TIMELINE_2000_2168.md
  - SPACE_HISTORY_2000_2168.md
recommended_path: docs/seedworks/00_canon/SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
---

# Symtropy: Seedworks Tech Tree Audit and Horizon Gates v0.3.3

## Working Title

**The Future Must Pass Inspection**

## Core Thesis

The Seedworks tech tree should no longer be treated as a growing list of unlocks.

It should be treated as a living infrastructure audit.

Every node must prove that the settlement can support a new capability through:

```text
matter
power
computation
legitimacy
maintenance
consequence
```

A technology is valid only when the player can understand:

```text
what built it
what powers it
what computes it
who authorizes it
who maintains it
how it can fail
what record it leaves
why the settlement is ready to bear it
```

Core rule:

```text
No future enters the Loom without an audit trail.
```

Design rule:

```text
The first pipe, the first robot, the first alien filter, the first lunar habitat,
and the first interstellar precursor must all pass the same moral physics:
can the world maintain this without lying to itself?
```

---

# 1. Purpose

This document is the validation and horizon-gate layer for the Seedworks tech tree.

It does not primarily add new technology.

It answers whether existing and future technologies are:

```text
authorable
serializable
visible
playable
explainable
legitimate
maintainable
recoverable
corruptible
testable
```

It exists to prevent four failure modes:

```text
1. Future-tech inflation.
2. Robotics becoming a platform catalog instead of an accountable system.
3. Xeno-tech becoming magical alien loot.
4. The Infrastructure Loom becoming a decorative tree instead of a civic diagnostic map.
```

This document should be used when creating or reviewing:

```text
Seedworks v0.1 nodes
Public Works v0.2 nodes
First Embassy v0.3 nodes
Robotics roadmap nodes
regional infrastructure nodes
living infrastructure nodes
off-world expansion nodes
interstellar precursor nodes
Null corruption states
Chronicle events
Loom UI panels
implementation fixtures
```

Final purpose line:

```text
This is the document that asks every future: prove you can be maintained.
```

---

# 2. Relationship to Existing Documents

## 2.1 Extends the Branch Pack

`SEEDWORKS_TECH_BRANCH_PACK_V0_3_1.md` expands the playable and horizon branches.

This document audits them.

The branch pack asks:

```text
What can the settlement eventually become?
```

This document asks:

```text
What must be true before that future is allowed into the player-facing Loom?
```

## 2.2 Extends the Robotics Expansion

`ROBOTICS_ROADMAP_TECH_TREE_EXPANSION_V0_3_2.md` reframes robots as public works with bodies.

This document adds the validator that makes that statement enforceable.

Robotics nodes must include:

```text
platform class
autonomy level
permission envelope
power draw
script budget
sensor suite
mobility domain
witness capacity
maintenance state
civic restrictions
failure modes
```

## 2.3 Extends the Dependency Spine

`TECH_TREE_DEPENDENCY_SPINE.md` defines the six dependency categories.

This document turns those categories into gate criteria, audit checklists, failure branches, and acceptance tests.

## 2.4 Extends the Infrastructure Loom Schema

`INFRASTRUCTURE_LOOM_TECH_NODE_SCHEMA.md` defines what node data should contain.

This document defines what node data must prove before it should be accepted.

## 2.5 Extends the Loom UI/UX Spec

`INFRASTRUCTURE_LOOM_UI_UX_SPEC.md` defines how the player sees the tech tree.

This document defines what a node must expose to the UI so every lock, unlock, shortcut, risk, and corruption state can explain itself.

---

# 3. Core Audit Principle

Every node must be judged by the same question:

```text
Can this capability exist without hiding the cost of its existence?
```

A good node makes dependency visible.

A bad node hides dependency behind fantasy progression.

A good node can fail meaningfully.

A bad node only changes a stat.

A good node produces memory.

A bad node unlocks and disappears into abstraction.

A good node creates new responsibilities.

A bad node creates only power.

Design rule:

```text
If the player cannot explain why the world is now ready for a capability,
the capability should remain locked, foreshadowed, or roadmap-only.
```

---

# 4. Audit Vocabulary

Use these terms consistently.

## Node

A single technology, facility, procedure, permission, platform, recipe, or future capability.

Examples:

```text
Patch Conduit Mk0
Proof-of-Repair Receipt
Public Works Fabrication Bench
mk0-scout Cable-Crawler
Machine Testimony Review
Shared Tool Embassy
Hybrid Filter Alpha
Regional Proof-of-Repair Compact
Lunar Habitat Public Override Doctrine
Interstellar Precursor Charter
```

## Branch

A set of nodes with a shared capability arc.

Examples:

```text
Public Works Certification Branch
Cargo / Ledger Branch
Robotics Readiness Branch
Civic Access Branch
Xeno-Readiness Branch
Regional Stewardship Branch
Off-World Repair Sovereignty Branch
```

## Gate

A civilizational readiness threshold.

A gate is not a node.

A gate is a validator that decides whether a family of nodes may become visible, playable, or horizon-valid.

Example:

```text
Gate 3 — Public Automation
```

means the settlement is ready for certain automation nodes only if it can authorize, monitor, interrupt, repair, and remember automated action.

## Audit Result

The final review result for a node.

```text
APPROVED_PLAYABLE
APPROVED_VISIBLE_LOCKED
APPROVED_FORESHADOWED
APPROVED_ROADMAP
NEEDS_DEPENDENCY_REWRITE
NEEDS_FAILURE_MODE
NEEDS_CHRONICLE_LINK
NEEDS_CIVIC_LOCK
NEEDS_MAINTENANCE_LOOP
REJECTED_SHALLOW
REJECTED_SCOPE_CREEP
ARCHIVE_DEPRECATED
```

## Horizon

A future-tech cluster beyond the immediate playable milestone.

Horizon nodes may be included only if they remain legible as future consequences of current systems.

---

# 5. Node Fitness Model

Every authored node should receive a fitness score from 0 to 30.

Six categories are scored from 0 to 5.

```text
Material fitness:      0–5
Power fitness:         0–5
Computation fitness:   0–5
Legitimacy fitness:    0–5
Maintenance fitness:   0–5
Consequence fitness:   0–5
```

## 5.1 Score Meaning

```text
0 — absent
1 — named but not actionable
2 — dependency exists but is vague
3 — dependency is playable or inspectable
4 — dependency has failure and recovery
5 — dependency is integrated with Chronicle, UI, and branch consequences
```

## 5.2 Node Approval Thresholds

```text
0–9:
  REJECTED_SHALLOW or ROADMAP_NOTE_ONLY

10–15:
  FORESHADOWED only

16–20:
  VISIBLE_LOCKED candidate

21–25:
  PLAYABLE candidate if player verb and Chronicle link exist

26–30:
  CORE_SPINE candidate
```

## 5.3 Automatic Failure Conditions

A node fails audit regardless of score if it has:

```text
PLAYABLE status with no player verb
robotics node with no permission envelope
xeno node with no consent dependency
fabrication node with no material quality requirement
Chronicle-producing node with no Chronicle event
horizon node with no prerequisite gate
NULL-corrupted node with no visible warning
civic node with no authority source
maintenance node with no inspection interval
Device Bus node with no stable ID or path
```

Design rule:

```text
Bad futures should fail at authoring time, not after they confuse the player.
```

---

# 6. The Ten Horizon Gates

The Seedworks tech tree should advance through ten gates.

These gates are not strict chronological chapters.

They are readiness thresholds.

A settlement may be strong in one gate and weak in another.

This creates story, faction pressure, and nonlinear progression.

Gate list:

```text
Gate 1 — Repeatable Repair
Gate 2 — Certified Fabrication
Gate 3 — Public Automation
Gate 4 — Machine Testimony
Gate 5 — Living Infrastructure Consent
Gate 6 — Regional Legitimacy
Gate 7 — Closed-Loop Habitat Stewardship
Gate 8 — Off-World Repair Sovereignty
Gate 9 — Deep-Time Archive Continuity
Gate 10 — Interstellar Readiness
```

Design rule:

```text
Future tech does not climb a ladder.
It passes increasingly severe audits of maintenance, consent, and distance.
```

---

# Gate 1 — Repeatable Repair

## Thesis

The settlement can repair a critical system once, record it, and teach why it worked.

## Gate Question

```text
Can one successful repair become public capacity instead of private heroism?
```

## Nodes Enabled or Upgraded

- Proof-of-Repair Receipt
- Public Works Fabrication Bench visible-lock
- Repair Worker Access Class 1
- Old Waterworks Precedent
- Archive Witness Cartridge trust boost

## Required Evidence

- Patch Conduit Mk0 repaired
- Chronicle event accepted
- Field Deck source chain intact or recovered
- local charter recognizes emergency repair
- repair result remains physically inspectable

## Blockers

- witness chain broken
- repair works physically but is civically disputed
- authority expired with no living charter
- NULL shortcut altered source-chain record

## Failure Pressure

- Heroic repair cannot be repeated
- public trust decays
- fabrication remains locked
- factions dispute whether the repair counts

## Chronicle Events

- `OldWaterworksOutcomeRecorded`
- `ProofOfRepairIssued`
- `EmergencyRepairRecognized`
- `WitnessChainDisputed`

## Loom Access Points

- Field Deck
- Public Works Wall Terminal
- Archive Witness Panel


## Approval Test

```text
Gate 1 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 2 — Certified Fabrication

## Thesis

The settlement can reproduce parts with known quality, known risk, and public accountability.

## Gate Question

```text
Can fabrication produce infrastructure instead of junk?
```

## Nodes Enabled or Upgraded

- Certified Seal Kit
- Certified Pipe Gauge
- Pressure Test Rig
- Public Tool Library
- Cargo Ledger Audit Station
- Repair Worker Access Class 2

## Required Evidence

- Public Works Fabrication Bench active
- stable bench power
- material quality states Q0–Q3
- recipe procedure with quality checks
- tool checkout ledger
- witnessable certification process

## Blockers

- materials unverified
- bench power unstable
- operator lacks access class
- recipe source corrupted
- pressure-test procedure missing

## Failure Pressure

- Uncertified fabrication boom
- tool ledger dispute
- repair works visually but cannot receive certified status
- faction dispute over who controls the bench

## Chronicle Events

- `PublicWorksBenchReopened`
- `CertifiedSealFabricated`
- `PressureTestPassed`
- `ToolLibraryPolicyPublished`
- `RecipeSourceDisputed`

## Loom Access Points

- Fabrication Bench
- Public Works Wall Terminal
- Civic Kiosk


## Approval Test

```text
Gate 2 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 3 — Public Automation

## Thesis

The settlement can allow machines to act without erasing human accountability.

## Gate Question

```text
Can automation be useful before it becomes a dead authority?
```

## Nodes Enabled or Upgraded

- mk0-scout Cable-Crawler
- mk0-gantry Sky-Hook
- mk0-aegis Acoustic Boundary
- Robot Dock
- Route Manifest Logger
- Public Inspection Route Authorization

## Required Evidence

- certified fabrication
- robot maintenance bay or dock
- operator access class
- permission envelope
- route boundary
- interrupt mechanism
- machine action logging

## Blockers

- route authorization missing
- privacy boundary unresolved
- robot has no recall path
- dock damaged
- script budget unverified

## Failure Pressure

- robot crosses route boundary
- machine performs technically correct but illegitimate action
- automation creates labor dispute
- public asks who is responsible

## Chronicle Events

- `RobotRouteAuthorized`
- `RobotRouteDenied`
- `MachineActionLogged`
- `AutomationInterruptionTestPassed`
- `LaborDisputeTriggered`

## Loom Access Points

- Robot Dock
- Civic Kiosk
- Field Deck DIAG
- Field Deck CIVIC


## Approval Test

```text
Gate 3 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 4 — Machine Testimony

## Thesis

The settlement can treat machine records as evidence without surrendering judgment to machines.

## Gate Question

```text
When can a machine witness support truth without replacing public judgment?
```

## Nodes Enabled or Upgraded

- Machine Testimony Review
- Archive-Certified Inspection Crawler
- Remote Witness Request
- Machine Witness Policy
- Route Evidence Packet

## Required Evidence

- robot action logging
- source-chain integrity
- machine clock calibration
- human witness fallback
- public evidence standard
- appeal process

## Blockers

- clock drift
- route manifest incomplete
- sensor occlusion
- witness packet unsigned
- machine authority exceeds civic policy

## Failure Pressure

- accurate log rejected due to authorization gap
- machine witness accepted too easily
- archive dispute escalates
- security faction weaponizes logs

## Chronicle Events

- `MachineTestimonySubmitted`
- `MachineTestimonyAccepted`
- `MachineTestimonyRejected`
- `WitnessAppealOpened`
- `ClockDriftDetected`

## Loom Access Points

- Archive Panel
- Robot Dock
- Civic Kiosk
- Chronicle View


## Approval Test

```text
Gate 4 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 5 — Living Infrastructure Consent

## Thesis

The settlement can enter exchange with living or alien systems without treating them as resources or loot.

## Gate Question

```text
Can human infrastructure receive help from non-human systems without violating their boundaries?
```

## Nodes Enabled or Upgraded

- Shared Tool Embassy
- Translation Pool
- Metabolic Stabilizer
- Rights Forum Terminal
- Hybrid Filter Alpha
- Overgrowth Without Consent warning branch

## Required Evidence

- human biofilter baseline
- Translation Pool calibration
- Rights Forum license
- xeno consent status
- quarantine procedure
- maintenance window
- metabolic compatibility map

## Blockers

- consent boundary not recognized
- metabolic mismatch
- translation confidence below threshold
- human wrapper unsafe
- public legitimacy disputed

## Failure Pressure

- Translation Collapse
- Overgrowth Without Consent
- hybrid system refuses command access
- human faction treats living infrastructure as property

## Chronicle Events

- `SharedToolEmbassyOpened`
- `TranslationPoolCalibrated`
- `RightsForumLicenseGranted`
- `HybridFilterLicensed`
- `TranslationCollapseContained`
- `ConsentBoundaryViolated`

## Loom Access Points

- Shared Tool Embassy
- Rights Forum Terminal
- Field Deck NULL
- Field Deck CIVIC


## Approval Test

```text
Gate 5 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 6 — Regional Legitimacy

## Thesis

A repair record can travel beyond one settlement without becoming imperial control.

## Gate Question

```text
Can local proof become regional trust without flattening local consent?
```

## Nodes Enabled or Upgraded

- Regional Proof-of-Repair Compact
- Trusted Cargo Corridor
- Technician Passport
- Watershed Repair Council
- Regional Robot Route Reciprocity

## Required Evidence

- portable source chain
- cross-settlement charter recognition
- cargo ledger compatibility
- appeal court or witness council
- regional maintenance contract
- faction dispute protocol

## Blockers

- settlement refuses external authority
- archive formats incompatible
- cargo provenance disputed
- regional inequality hardens
- private utility zone rejects public audit

## Failure Pressure

- Proof-of-Repair fraud
- technician caste formation
- regional capture by utility monopoly
- repair compact becomes coercive

## Chronicle Events

- `RegionalProofRecognized`
- `TrustedCargoCorridorOpened`
- `TechnicianPassportIssued`
- `RegionalCompactDisputed`
- `WatershedCouncilConvened`

## Loom Access Points

- Public Works Wall Terminal
- Civic Kiosk
- Regional Horizon Map
- Cargo Ledger Station


## Approval Test

```text
Gate 6 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 7 — Closed-Loop Habitat Stewardship

## Thesis

The settlement can maintain air, water, food, power, shelter, care, and law as one life-support system.

## Gate Question

```text
Can a habitat become a society instead of a sealed machine?
```

## Nodes Enabled or Upgraded

- Closed-Loop Shelter
- Ecological Quarantine Chamber
- Medical Reconstitution Cot
- Atmospheric Commons
- Habitat Public Override Doctrine
- Care Capacity Ledger

## Required Evidence

- water loop transparency
- air quality audit
- food/care maintenance
- energy reserve
- public override
- emergency expiry doctrine
- resident rights floor
- psychological care plan

## Blockers

- company owns life-support control
- care labor invisible
- emergency powers never expire
- closed-loop automation has no interrupt
- residents cannot leave or appeal

## Failure Pressure

- habitat feels like prison
- life-support denial by dead rule
- care collapse
- automation preserves uptime over life

## Chronicle Events

- `HabitatLoopCertified`
- `PublicOverridePublished`
- `AirWaterAuditPassed`
- `EmergencyExpiryReviewed`
- `CareCapacityWarningIssued`

## Loom Access Points

- Habitat Wall Terminal
- Civic Kiosk
- Field Deck DIAG
- Field Deck WITNESS


## Approval Test

```text
Gate 7 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 8 — Off-World Repair Sovereignty

## Thesis

Seedworks doctrine can function where life-support, latency, radiation, and rescue law make every failure political.

## Gate Question

```text
Can off-world infrastructure be interruptible, auditable, and repair-literate under vacuum constraints?
```

## Nodes Enabled or Upgraded

- Habitat Passport Mode
- Airlock Witness Protocol
- Reactor Audit Covenant
- Lunar Regolith Repair License
- Belt Salvage Witness Packet
- Distress Beacon Override

## Required Evidence

- closed-loop habitat stewardship
- pressure-system device bus
- radiation shelter doctrine
- latency-aware governance
- rescue-first property rules
- machine claim expiry
- life-support transparency

## Blockers

- opaque company habitat
- rescue denied by property claim
- reactor authority expired
- airlock law loop
- propellant hoarding
- autonomous mining system unaudited

## Failure Pressure

- company owns the air
- rescue classified as trespass
- habitat AI follows procedure into harm
- machine stewardship becomes abandonment

## Chronicle Events

- `AirlockWitnessRecorded`
- `HabitatOverrideInvoked`
- `ReactorAuditPublished`
- `DistressBeaconHonored`
- `MachineClaimExpired`

## Loom Access Points

- Habitat Passport Field Deck
- Airlock Panel
- Reactor Audit Console
- Off-World Archive Witness


## Approval Test

```text
Gate 8 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 9 — Deep-Time Archive Continuity

## Thesis

The civilization can maintain memory across long delays, jurisdictional splits, disasters, migrations, and worldline disputes.

## Gate Question

```text
Can history remain usable without becoming a prison?
```

## Nodes Enabled or Upgraded

- Worldline Archive
- Forkable Settlement History
- Deep-Time Witness Vault
- Intergenerational Repair Curriculum
- Dead Authority Expiry Court
- Archive Migration Treaty

## Required Evidence

- portable archives
- source-chain continuity
- witness succession
- public appeal
- dead-authority expiry
- migration identity continuity
- machine testimony limits

## Blockers

- archive capture
- records preserved without trust
- worldline fork denied
- old law dominates living need
- machine witnesses treated as sovereign

## Failure Pressure

- archive becomes ruling class
- dead law outlives people
- repair meaning contested
- history becomes too complex to govern

## Chronicle Events

- `WorldlineForkRecognized`
- `DeadAuthorityExpired`
- `ArchiveMigrationAccepted`
- `DeepTimeWitnessDeposited`
- `HistoryAppealOpened`

## Loom Access Points

- Archive Witness Council
- Worldline Map
- Civic Kiosk
- Field Deck ARCHIVE


## Approval Test

```text
Gate 9 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# Gate 10 — Interstellar Readiness

## Thesis

A civilization should not attempt stellar expansion until it can carry maintenance, legitimacy, consent, and memory across interstellar delay.

## Gate Question

```text
Can a future travel farther than rescue without becoming a prison or a weapon?
```

## Nodes Enabled or Upgraded

- Interstellar Precursor Charter
- Robotic Precursor Ark
- Generation Ship Ethics Hearing
- Nearby-Star Probe Treaty
- Orange-Dwarf Debate Forum
- Deep Xeno Contact Protocol

## Required Evidence

- deep-time archive continuity
- closed-loop habitat stewardship
- off-world repair sovereignty
- rights of exit or meaningful refusal
- non-coercive crew/descendant ethics
- long-delay governance
- xeno-contact quarantine and consent protocol

## Blockers

- no exit rights
- generation ship consent unresolved
- machine mission cannot be interrupted
- archive continuity untrusted
- resource extraction motive dominates
- xeno contact protocol absent

## Failure Pressure

- interstellar prison
- mission success over descendants
- AI probe becomes unaccountable envoy
- contact without consent
- deep-time colonization repeats old violence

## Chronicle Events

- `InterstellarCharterDebated`
- `PrecursorProbeAuthorized`
- `GenerationShipEthicsRejected`
- `DeepXenoProtocolPublished`
- `InterstellarReadinessDenied`

## Loom Access Points

- Atlas Gate Horizon
- Archive Council
- Shared Tool Embassy
- Off-World Wall Terminal


## Approval Test

```text
Gate 10 passes only if the player can inspect why this capability is now supportable,
why it was previously locked, what can still go wrong, and which Chronicle record will remember it.
```

---

# 7. Golden Path Dependency Spine

The tech tree needs one clean path from the first repair to the deepest future.

This is not the only path.

It is the canonical proof that the tree has a readable civilizational arc.

## 7.1 Golden Path

```text
Patch Conduit Mk0
  ↓
Old Waterworks Outcome Recorded
  ↓
Proof-of-Repair Receipt
  ↓
Public Works Fabrication Bench
  ↓
Certified Seal Kit
  ↓
Certified Pipe Gauge
  ↓
Pressure Test Rig
  ↓
Cargo Ledger Audit Station
  ↓
Public Tool Library
  ↓
mk0-scout Cable-Crawler
  ↓
Route Manifest Logger
  ↓
Machine Testimony Review
  ↓
Settlement Public Vote
  ↓
Shared Tool Embassy
  ↓
Translation Pool
  ↓
Hybrid Filter Alpha
  ↓
Regional Proof-of-Repair Compact
  ↓
Trusted Cargo Corridor
  ↓
Autonomous Public Works
  ↓
Closed-Loop Habitat Stewardship
  ↓
Off-World Repair Sovereignty
  ↓
Deep-Time Archive Continuity
  ↓
Interstellar Readiness
```

## 7.2 Golden Path Story

The player should be able to summarize the entire path in one sentence:

```text
A pipe repair becomes a public record, the record becomes trusted fabrication,
fabrication makes robots possible, robots create machine testimony, testimony requires public law,
public law enables xeno exchange, xeno exchange forces regional legitimacy,
regional legitimacy matures into habitat stewardship, and habitat stewardship becomes the minimum ethics of the stars.
```

## 7.3 Golden Path Acceptance Test

The Golden Path succeeds if every step can answer:

```text
What changed in the world?
What evidence proves it?
What new responsibility was created?
What failure became possible?
What UI panel explains the lock or unlock?
What Chronicle event preserves the transition?
```

It fails if any step reads like:

```text
research complete
upgrade purchased
new tier unlocked
+5% efficiency
```

Design rule:

```text
The Golden Path should feel like civilization becoming more accountable, not more powerful.
```

---

# 8. Audit Result Matrix

Every node should be reviewed with the following matrix.

| Field | Required? | Pass Condition | Common Failure |
|---|---:|---|---|
| Stable ID | Yes | Unique `tech.*` ID | Display name used as ID |
| Human name | Yes | Clear UI label | Vague lore phrase |
| Milestone | Yes | v0.1/v0.2/v0.3/v1.0/horizon | No milestone boundary |
| Status | Yes | Uses approved enum | Custom status drift |
| Discipline | Yes | At least one, ideally intersections | Single generic category |
| Dependency layer | Yes | Layer 0–8 or horizon gate | Floating node |
| Player verb | Playable only | Inspect, build, deploy, vote, certify, recall, witness | Passive lore only |
| Material dependency | Most nodes | Materials, facility, body, medium, habitat, or archive substrate | Abstract idea with no body |
| Power dependency | Most nodes | Power draw, stability, reserve, or N/A reason | Energy invisible |
| Computation dependency | Most nodes | Field Deck, Device Bus, script budget, source chain, runtime | Magic automation |
| Legitimacy dependency | Yes | Charter, witness, vote, consent, access class, treaty | Tech without permission |
| Maintenance loop | Yes | Inspection, repair, calibration, training, replacement | Unlock is permanent and effortless |
| Consequence | Yes | Chronicle event or world-state change | Node has no memory |
| Failure mode | Yes | At least one named failure | Cannot fail meaningfully |
| Recovery path | Recommended | Quest, audit, vote, repair, appeal | Failure is only punishment |
| UI surface | Yes | Field Deck, Wall, Bench, Kiosk, Dock, Embassy, etc. | Hidden spreadsheet logic |
| NULL interaction | Recommended | Corruptible, spoofable, or safe from NULL with reason | NULL only cosmetic |
| Test fixture | Implementation | JSON/YAML fixture validates | Hand-authored UI state |

---

# 9. Audit Decision Tree

Use this decision tree when reviewing any proposed node.

```text
1. Does the node describe a concrete capability?
   no  → FORESHADOWED, ROADMAP, or reject as lore-only.
   yes → continue.

2. Can the capability be located in the world?
   no  → require material/device/civic/archive substrate.
   yes → continue.

3. Does the node have at least one dependency in the six grammar categories?
   no  → reject shallow.
   yes → continue.

4. If playable, does it have a player verb?
   no  → visible-locked or rewrite.
   yes → continue.

5. Can it fail?
   no  → add failure mode before approval.
   yes → continue.

6. Can the failure be recorded?
   no  → add Chronicle link.
   yes → continue.

7. Does the UI explain the locked state?
   no  → add Loom copy.
   yes → continue.

8. Does the node make future consequences richer?
   no  → consider removing, merging, or deferring.
   yes → approve at the appropriate status.
```

Design rule:

```text
A node that cannot explain itself should not unlock itself.
```

---

# 10. Robotics Permission Envelope System

Robotics needs a formal permission layer.

A robot is not just a chassis plus autonomy.

It is a moving public claim.

## 10.1 Permission Envelope Fields

```text
permission_id
robot_platform
allowed_routes
allowed_tasks
forbidden_tasks
autonomy_level
required_operator_class
required_witness_mode
privacy_boundary
payload_limit
power_limit
speed_limit
contact_limit
recall_method
manual_override
maintenance_interval
log_retention
appeal_authority
emergency_override
failure_escalation
```

## 10.2 Autonomy Levels

```text
L0 — Inert / unpowered display
L1 — Teleoperated only
L2 — Supervised routine
L3 — Bounded autonomous route
L4 — Conditional task autonomy
L5 — Public works autonomy under charter
L6 — Regional autonomous infrastructure role
L7 — Off-world / long-delay stewardship role
L8 — Deep-time machine witness role
```

## 10.3 Permission Classes

```text
R-OBSERVE:
  may observe and log only

R-MARK:
  may mark hazards, routes, and material states

R-CARRY:
  may move approved cargo under manifest

R-REPAIR-SUPPORT:
  may assist human repair but not certify

R-REPAIR-ACTIVE:
  may perform bounded repair actions under witness

R-CERTIFY-SUPPORT:
  may produce evidence packet for human/public certification

R-STEWARD:
  may maintain infrastructure under charter and interrupt rules

R-RESCUE:
  may override property/access limits for life-safety rescue

R-XENO-BOUNDARY:
  may operate only within non-human consent boundary rules

R-OFFWORLD-LIFE-SUPPORT:
  may act on pressure, air, water, or reactor systems only with special public override logging
```

## 10.4 Robotics Audit Rule

```text
A robot is valid only when its permission envelope is more explicit than its motor spec.
```

---

# 11. Robotics Permission Envelope Matrix

| Platform | Earliest Status | Autonomy Ceiling | Primary Permission | Unlock Gate | Key Failure |
|---|---:|---:|---|---|---|
| `mk0-scout` Cable-Crawler | v0.2 visible/playable | L2 | R-OBSERVE / R-MARK | Gate 3 | Route log rejected |
| `mk0-gantry` Sky-Hook | v0.2 visible/playable | L2 | R-CARRY | Gate 3 | Cargo drop / manifest dispute |
| `mk0-aegis` Acoustic Boundary | v0.2 playable/stub | L2 | R-OBSERVE | Gate 3 | Privacy boundary violation |
| `mk0-agora` Civic Kiosk | v0.2 playable | L1 | Civic terminal, not robot | Gate 2 | Vote log integrity failure |
| `mk0-biota` Perimeter Sentinel | v0.3 visible/playable | L2 | R-OBSERVE / R-XENO-BOUNDARY | Gate 5 | Species misclassification |
| `mk0.5-mill` Precision Escalator | v0.4 roadmap | L3 | R-REPAIR-SUPPORT | Gate 2 | Backlash compensation corrupts part |
| `mk0.5-loom` Motor Winder | v0.4 roadmap | L3 | R-REPAIR-SUPPORT | Gate 2 | Motor winding batch defect |
| `mk0.5-spark` PCB Assembler | v0.4 roadmap | L3 | R-REPAIR-SUPPORT | Gate 2 | Firmware slug contamination |
| `quadruped` Maintenance Agent | v1.0 roadmap | L4 | R-MARK / R-CARRY / R-REPAIR-SUPPORT | Gate 3 | Enters restricted shelter route |
| `subterranean` Borer | v1.x roadmap | L4 | R-MARK / R-REPAIR-SUPPORT | Gate 6 | Undermines unrecorded heritage layer |
| `scavenger` Unbuilder | v1.x roadmap | L4 | R-CARRY / R-REPAIR-ACTIVE | Gate 6 | Salvage without witness becomes theft |
| `agribot` Steward | v1.x roadmap | L4 | R-STEWARD | Gate 5 | Ecological optimization violates consent |
| `terra` Remediation Platform | v2.x roadmap | L5 | R-STEWARD | Gate 6 | Remediation shifts contamination downstream |
| `symthaea-stratum` Transit Surface | v2.x roadmap | L5 | Regional infrastructure | Gate 6 | Mobility ration encoded as caste |
| `symthaea-plexus` Utility Manifold | v2.x roadmap | L5 | R-STEWARD | Gate 7 | Self-healing route hides leak evidence |
| `symthaea-cycler` Orbital Tug | v3.x roadmap | L6 | R-OFFWORLD-LIFE-SUPPORT / logistics | Gate 8 | Rescue window missed by contract rule |
| `symthaea-spindle` Zero-G Assembler | v3.x roadmap | L6 | R-REPAIR-ACTIVE | Gate 8 | Structure grows beyond audit boundary |
| `symthaea-regolith` Lunar Extraction | v3.x roadmap | L6 | R-STEWARD | Gate 8 | Dust contamination / heritage violation |
| `symthaea-abyssal` Ocean Steward | v3.x roadmap | L6 | R-STEWARD / R-XENO-BOUNDARY | Gate 5/6 | Treats living record as substrate |
| Robotic Precursor Ark | v4.x horizon | L7–L8 | Deep-time chartered envoy | Gate 10 | Mission success over descendant/encounter ethics |

Design rule:

```text
The higher the robot autonomy, the heavier its civic explanation must become.
```

---

# 12. Future-Tech Gate Compatibility Matrix

Future-tech nodes must be gated by readiness, not spectacle.

| Future-Tech Family | Required Gates | Must Not Unlock If | First Safe Foreshadow |
|---|---|---|---|
| Autonomous public works | Gates 2, 3, 4 | No interrupt, no route log, no repair guild | v0.2 robot dock warnings |
| Living infrastructure | Gates 2, 5 | No consent protocol, no metabolic maintenance | v0.3 Embassy locked branch |
| Regional infrastructure | Gates 1, 2, 3, 6 | Proof cannot travel, cargo ledger incompatible | v0.2 wall terminal horizon |
| Closed-loop habitats | Gates 2, 3, 6, 7 | No public override, no care capacity ledger | v1.0 horizon map |
| Off-world life support | Gates 7, 8 | Company owns air, rescue weaker than ownership | archive logs / skybox references |
| Deep-time archives | Gates 4, 6, 9 | Dead authority cannot expire | Archive Witness Council |
| Interstellar precursors | Gates 7, 8, 9, 10 | No exit/refusal ethics, no long-delay legitimacy | Atlas Gate foreshadow |
| Xeno contact systems | Gates 5, 9, 10 | Consent translation unresolved | Shared Tool Embassy distant signal |

Design rule:

```text
No future-tech node should be more advanced than its legitimacy machinery.
```

---

# 13. Failure and Corruption Branches

The tech tree should include failure branches.

Failure branches are not punishments.

They are alternate evidence states.

They show what happens when a settlement unlocks capacity without enough truth, consent, maintenance, or memory.

## 13.1 Failure Branch Types

```text
technical failure
material failure
power failure
computation failure
legitimacy failure
maintenance failure
witness failure
consent failure
regional failure
closed-loop failure
off-world failure
deep-time failure
```

## 13.2 Null Corruption Types

```text
false shortcut
dead-authority unlock
spoofed witness
unsafe recipe mutation
recursive dependency loop
permission bypass
fake Proof-of-Repair
machine mission persistence
archive capture
consent erasure
```

Design rule:

```text
NULL should tempt the player with a future the world cannot safely bear.
```

---
# 14. Failure Branch Catalog


## Emergency Override Abuse

Type:

```text
legitimacy failure
```

Description:

```text
Emergency power keeps being renewed until it becomes normal government.
```

Affected Nodes:

- `EmergencyRepairToken`
- `FuelDepotTrustConsole`
- `PublicWorksBench`


Recovery Path:

```text
EmergencyExpiryReview
```

Lock / Warning State:

```text
DeadAuthorityLock
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Uncertified Fabrication Boom

Type:

```text
material / legitimacy failure
```

Description:

```text
The settlement produces parts faster than it can certify them.
```

Affected Nodes:

- `PublicWorksFabricationBench`
- `CertifiedSealKit`
- `PublicToolLibrary`


Recovery Path:

```text
FabricationAudit
```

Lock / Warning State:

```text
RecipeQuarantine
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Machine Testimony Rejected

Type:

```text
witness failure
```

Description:

```text
A robot saw the truth, but the route and clock record are civically invalid.
```

Affected Nodes:

- `mk0-scout`
- `RouteManifestLogger`
- `MachineWitnessPolicy`


Recovery Path:

```text
WitnessAppeal
```

Lock / Warning State:

```text
RoutePolicyRepair
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Public Tool Ledger Corrupted

Type:

```text
archive / cargo failure
```

Description:

```text
Tool access becomes a faction fight because checkout, return, or custody is disputed.
```

Affected Nodes:

- `PublicToolLibrary`
- `CargoLedgerAuditStation`


Recovery Path:

```text
LedgerReconciliation
```

Lock / Warning State:

```text
ToolAccessFreeze
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Robot Route Boundary Violation

Type:

```text
robotic legitimacy failure
```

Description:

```text
A robot performs a useful action outside its authorized route.
```

Affected Nodes:

- `RobotDock`
- `PublicInspectionRouteAuthorization`


Recovery Path:

```text
RouteHearing
```

Lock / Warning State:

```text
RobotRecallTest
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Automation Without Appeal

Type:

```text
civic automation failure
```

Description:

```text
Automated decision logic becomes operational before residents have an appeal path.
```

Affected Nodes:

- `CivicKiosk`
- `PublicAutomationPolicy`


Recovery Path:

```text
AppealProtocol
```

Lock / Warning State:

```text
AutomationPause
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Translation Collapse

Type:

```text
xeno-translation failure
```

Description:

```text
Translation confidence falls below safe threshold but the system keeps operating.
```

Affected Nodes:

- `TranslationPool`
- `SharedToolEmbassy`


Recovery Path:

```text
TranslationQuarantine
```

Lock / Warning State:

```text
EmbassySuspension
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Overgrowth Without Consent

Type:

```text
living infrastructure failure
```

Description:

```text
A living system remains useful while violating agreed boundaries.
```

Affected Nodes:

- `HybridFilterAlpha`
- `MetabolicStabilizer`


Recovery Path:

```text
RightsForumAppeal
```

Lock / Warning State:

```text
ConsentBoundaryRepair
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Regional Proof-of-Repair Fraud

Type:

```text
regional legitimacy failure
```

Description:

```text
Portable proof travels farther than its source-chain integrity.
```

Affected Nodes:

- `RegionalProofCompact`
- `TechnicianPassport`


Recovery Path:

```text
RegionalWitnessAudit
```

Lock / Warning State:

```text
PassportSuspension
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Closed-Loop Prison Drift

Type:

```text
habitat failure
```

Description:

```text
A habitat remains physically safe but socially inescapable.
```

Affected Nodes:

- `ClosedLoopShelter`
- `HabitatPublicOverrideDoctrine`


Recovery Path:

```text
ExitRightsHearing
```

Lock / Warning State:

```text
EmergencyPowerExpiry
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Airlock Law Loop

Type:

```text
off-world failure
```

Description:

```text
An airlock denies rescue because old credentials define the living as trespassers.
```

Affected Nodes:

- `HabitatPassportMode`
- `AirlockWitnessProtocol`


Recovery Path:

```text
RescueOverride
```

Lock / Warning State:

```text
DeadAuthorityExpiry
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Archive Sovereignty Capture

Type:

```text
deep-time failure
```

Description:

```text
The archive becomes the authority instead of evidence for authority.
```

Affected Nodes:

- `WorldlineArchive`
- `DeepTimeWitnessVault`


Recovery Path:

```text
ArchiveAppeal
```

Lock / Warning State:

```text
WitnessCouncilReform
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Interstellar Prison Charter

Type:

```text
interstellar ethics failure
```

Description:

```text
A mission can leave but its descendants cannot meaningfully refuse the mission.
```

Affected Nodes:

- `InterstellarPrecursorCharter`
- `GenerationShipEthicsHearing`


Recovery Path:

```text
CharterRejection
```

Lock / Warning State:

```text
MissionRedesign
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

## Machine Envoy Without Consent

Type:

```text
deep xeno failure
```

Description:

```text
A robotic precursor becomes humanity's envoy without legitimate contact rules.
```

Affected Nodes:

- `RoboticPrecursorArk`
- `DeepXenoContactProtocol`


Recovery Path:

```text
EnvoyRecallDebate
```

Lock / Warning State:

```text
ContactProtocolRewrite
```

Audit Rule:

```text
This branch must be visible in the Loom before the player can accidentally normalize it.
```

---

# 15. Chronicle Event Map

Every major unlock, failure, and recovery must produce or reference a Chronicle event.

The Chronicle does not merely record story.

It is part of the unlock system.

## 15.1 Core Event Families

```text
Repair events
Fabrication events
Cargo events
Civic events
Robotics events
Machine testimony events
Xeno-translation events
Regional legitimacy events
Habitat stewardship events
Off-world witness events
Deep-time archive events
Interstellar ethics events
NULL corruption events
```

## 15.2 Event Naming Rules

Use concise PascalCase event names.

Good:

```text
ProofOfRepairIssued
PublicWorksBenchReopened
RobotRouteAuthorized
MachineTestimonyRejected
HybridFilterLicensed
DeadAuthorityExpired
```

Avoid:

```text
PlayerUnlockedBench
TechUpgradeComplete
RobotQuestDone
AlienItemObtained
```

Design rule:

```text
Chronicle events should sound like accountable public memory, not achievement popups.
```

## 15.3 Event Table

| Event | Produced By | Consumed By | Failure Variant |
|---|---|---|---|
| `OldWaterworksOutcomeRecorded` | v0.1 repair loop | Proof-of-Repair | `OldWaterworksOutcomeDisputed` |
| `ProofOfRepairIssued` | Archive Witness / Field Deck | Public Works Bench | `ProofOfRepairRejected` |
| `PublicWorksBenchReopened` | v0.2 Civic/Fabrication | Certified recipes | `BenchAccessDenied` |
| `CertifiedSealFabricated` | Fabrication Bench | Pressure Test Rig | `SealCertificationFailed` |
| `PressureTestPassed` | Pressure Test Rig | Repair grade upgrade | `PressureTestFailed` |
| `CargoAuditAccepted` | Cargo Ledger Station | Trusted material use | `CargoAuditDisputed` |
| `ToolLibraryPolicyPublished` | Civic Kiosk | Tool checkout | `ToolLedgerCorrupted` |
| `RobotRouteAuthorized` | Civic Kiosk / Robot Dock | mk0-scout deployment | `RobotRouteDenied` |
| `MachineActionLogged` | Robot runtime | Machine testimony | `MachineLogIncomplete` |
| `MachineTestimonyAccepted` | Witness Council | Remote inspection legitimacy | `MachineTestimonyRejected` |
| `SharedToolEmbassyOpened` | v0.3 Embassy quest | Xeno translation | `EmbassyAccessSuspended` |
| `TranslationPoolCalibrated` | Translation Pool | Hybrid tech | `TranslationConfidenceBelowThreshold` |
| `RightsForumLicenseGranted` | Rights Forum | Hybrid Filter Alpha | `RightsForumLicenseDenied` |
| `HybridFilterLicensed` | Shared Tool Embassy | Living infrastructure | `HybridFilterQuarantined` |
| `RegionalProofRecognized` | Regional compact | Trusted corridor | `RegionalProofFraudDetected` |
| `TrustedCargoCorridorOpened` | Regional ledger | Regional logistics | `CorridorTrustSuspended` |
| `HabitatLoopCertified` | Habitat systems | Closed-loop settlement | `HabitatLoopWarning` |
| `PublicOverridePublished` | Habitat charter | Life-support trust | `OverrideAuthorityExpired` |
| `AirlockWitnessRecorded` | Off-world Field Deck | Rescue legitimacy | `AirlockWitnessDisputed` |
| `DistressBeaconHonored` | Belt compact | Rescue-first law | `DistressBeaconDenied` |
| `DeadAuthorityExpired` | Archive court | NULL recovery | `DeadAuthorityPersists` |
| `WorldlineForkRecognized` | Archive Council | Deep-time continuity | `WorldlineForkDenied` |
| `InterstellarCharterDebated` | Atlas Gate / Council | Precursor authorization | `InterstellarReadinessDenied` |
| `PrecursorProbeAuthorized` | Deep-time council | Robotic precursor | `PrecursorProbeQuarantined` |
| `NullShortcutDetected` | Field Deck NULL | Warning overlay | `NullShortcutAccepted` |

---

# 16. Loom UI Requirements

The audit layer is only useful if the Loom exposes it clearly.

Every node should generate mode-specific text.

## 16.1 Required Node Panels

```text
Node Summary Panel
Readiness Panel
Dependency Panel
Locked Explanation Panel
Failure Mode Panel
Chronicle Link Panel
Permission / Consent Panel
Maintenance Panel
Horizon Gate Panel
NULL Warning Panel
```

## 16.2 Mode-Specific Questions

```text
SCAN:
  What exists physically?

DIAG:
  What can fail technically?

ARCHIVE:
  What made this possible before?

CIVIC:
  What is authorized, disputed, or forbidden?

NULL:
  What is lying, spoofed, recursive, or dead-authority locked?

WITNESS:
  What can be proven, appealed, or remembered?
```

## 16.3 Lock Explanation Template

```text
NODE:
{node_name}

STATUS:
{status}

LOCK TYPE:
physical / power / computation / civic / maintenance / consequence / consent / NULL

MISSING:
- {dependency_1}
- {dependency_2}
- {dependency_3}

WHY THIS MATTERS:
{plain-language consequence explanation}

NEXT ACTION:
{player verb or quest pointer}

CHRONICLE:
{event that may unlock this node}
```

## 16.4 Horizon Gate Panel Template

```text
HORIZON GATE:
Gate {number} — {name}

READINESS:
Material:      {percent}%
Power:         {percent}%
Computation:   {percent}%
Legitimacy:    {percent}%
Maintenance:   {percent}%
Consequence:   {percent}%

BLOCKER:
{most important blocker}

PUBLIC QUESTION:
{gate question}

RISK IF BYPASSED:
{failure branch}
```

Design rule:

```text
The player should never wonder why a future is locked.
They should wonder what kind of civilization they must become to unlock it safely.
```

---

# 17. Data Schema Additions

The existing node schema should receive an audit extension.

## 17.1 Proposed `TechNodeAudit` Struct

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechNodeAudit {
    pub audit_id: String,
    pub node_id: String,
    pub audit_version: String,
    pub material_score: u8,
    pub power_score: u8,
    pub computation_score: u8,
    pub legitimacy_score: u8,
    pub maintenance_score: u8,
    pub consequence_score: u8,
    pub total_score: u8,
    pub result: TechAuditResult,
    pub missing_requirements: Vec<String>,
    pub required_horizon_gates: Vec<String>,
    pub failure_branch_ids: Vec<String>,
    pub chronicle_event_ids: Vec<String>,
    pub loom_surface_ids: Vec<String>,
    pub reviewer_notes: Vec<String>,
}
```

## 17.2 Proposed `TechAuditResult` Enum

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TechAuditResult {
    ApprovedPlayable,
    ApprovedVisibleLocked,
    ApprovedForeshadowed,
    ApprovedRoadmap,
    NeedsDependencyRewrite,
    NeedsFailureMode,
    NeedsChronicleLink,
    NeedsCivicLock,
    NeedsMaintenanceLoop,
    RejectedShallow,
    RejectedScopeCreep,
    ArchiveDeprecated,
}
```

## 17.3 Proposed `HorizonGateState` Struct

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HorizonGateState {
    pub gate_id: String,
    pub name: String,
    pub status: HorizonGateStatus,
    pub readiness: ReadinessSet,
    pub unlocked_node_ids: Vec<String>,
    pub blocked_node_ids: Vec<String>,
    pub required_chronicle_events: Vec<String>,
    pub active_failure_branches: Vec<String>,
    pub public_question: String,
}
```

## 17.4 Proposed `HorizonGateStatus` Enum

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizonGateStatus {
    Hidden,
    Foreshadowed,
    VisibleLocked,
    UnderReview,
    Passed,
    Failed,
    Corrupted,
    Deferred,
}
```

Design rule:

```text
Horizon gates should be data, not prose trapped in design documents.
```

---

# 18. Example Audited Nodes

## 18.1 Public Works Fabrication Bench

```yaml
id: tech.public_works.fabrication_bench.v0_2
name: Public Works Fabrication Bench
milestone: V0_2
status: VisibleLocked
horizon_gate: gate_2_certified_fabrication
disciplines:
  - ThermodynamicMaterialFabrication
  - SocioCivicLegitimacyChains
  - DeviceBusSubstrate
player_verbs:
  - inspect
  - load_material
  - select_recipe
  - calibrate
  - fabricate
  - certify
required_events:
  - OldWaterworksOutcomeRecorded
  - ProofOfRepairIssued
required_dependencies:
  - stable_bench_power
  - public_tool_access
  - cargo_ledger_stub
  - operator_identity
failure_modes:
  - CHARTER_ACCESS_DENIED
  - NULL_RECIPE_CONTAMINATION
  - PRESSURE_TEST_FAIL
audit:
  material_score: 4
  power_score: 3
  computation_score: 4
  legitimacy_score: 5
  maintenance_score: 4
  consequence_score: 5
  total_score: 25
  result: ApprovedVisibleLocked
```

## 18.2 mk0-scout Cable-Crawler

```yaml
id: tech.robotics.mk0_scout.cable_crawler
name: mk0-scout Cable-Crawler
milestone: V0_2
status: VisibleLocked
horizon_gate: gate_3_public_automation
disciplines:
  - RoboticsAndAutomation
  - ComputationalFieldArchitecture
  - SocioCivicLegitimacyChains
permission_envelope:
  autonomy_level: L2
  primary_permission: R-OBSERVE
  secondary_permission: R-MARK
  allowed_routes:
    - public_inspection_wire_alpha
  required_operator_class: repair_worker_access_class_2
  recall_method: tether_pullback_or_dock_recall
  witness_mode: visual_log_only
player_verbs:
  - deploy
  - inspect
  - mark
  - recall
required_dependencies:
  - public_works_fabrication_bench
  - robot_crawler_motor_service_pack
  - battery_charger
  - overhead_cable_route
  - field_deck_remote_view
  - public_inspection_permission
failure_modes:
  - WITNESS_REJECTED
  - ROUTE_BOUNDARY_DENIED
  - POWER_SAG_MOTOR_DROP
audit:
  material_score: 4
  power_score: 4
  computation_score: 4
  legitimacy_score: 5
  maintenance_score: 3
  consequence_score: 5
  total_score: 25
  result: ApprovedVisibleLocked
```

## 18.3 Hybrid Filter Alpha

```yaml
id: tech.xeno.hybrid_filter_alpha
name: Hybrid Filter Alpha
milestone: V0_3
status: VisibleLocked
horizon_gate: gate_5_living_infrastructure_consent
disciplines:
  - XenoTranslation
  - ThermodynamicMaterialFabrication
  - SocioCivicLegitimacyChains
player_verbs:
  - inspect
  - calibrate
  - license
  - install
  - maintain
xeno_metadata:
  alien_source: Tideborn Water-Civic Exchange
  metabolic_need: flow_continuity_and_ph_stability
  human_wrapper: bio_electric_converter
  consent_dependency: rights_forum_license
  translation_confidence_minimum: 0.78
required_dependencies:
  - biofilter_housing
  - public_works_fabrication_bench
  - translation_pool
  - shared_tool_embassy
  - tideborn_exchange
  - rights_forum_license
  - metabolic_stabilizer
failure_modes:
  - TRANSLATION_COLLAPSE
  - OVERGROWTH_WITHOUT_CONSENT
  - CONSENT_BOUNDARY_VIOLATED
audit:
  material_score: 4
  power_score: 3
  computation_score: 5
  legitimacy_score: 5
  maintenance_score: 4
  consequence_score: 5
  total_score: 26
  result: ApprovedVisibleLocked
```

## 18.4 Interstellar Precursor Charter

```yaml
id: tech.interstellar.precursor_charter
name: Interstellar Precursor Charter
milestone: FutureHorizon
status: Roadmap
horizon_gate: gate_10_interstellar_readiness
disciplines:
  - InterstellarTransit
  - SocioCivicLegitimacyChains
  - DeathAndReconstitution
  - XenoTranslation
player_verbs:
  - review
  - debate
  - witness
  - approve
  - reject
required_dependencies:
  - closed_loop_habitat_stewardship
  - off_world_repair_sovereignty
  - deep_time_archive_continuity
  - generation_ship_ethics_hearing
  - deep_xeno_contact_protocol
  - rights_of_exit_or_refusal
failure_modes:
  - INTERSTELLAR_PRISON_CHARTER
  - MACHINE_ENVOY_WITHOUT_CONSENT
  - MISSION_SUCCESS_OVER_DESCENDANTS
audit:
  material_score: 2
  power_score: 2
  computation_score: 3
  legitimacy_score: 5
  maintenance_score: 4
  consequence_score: 5
  total_score: 21
  result: ApprovedRoadmap
```

---

# 19. Node Audit Backlog

The following backlog should be reviewed before adding more speculative nodes.

## 19.1 v0.1 Core Nodes

| Node | Required Action | Priority |
|---|---|---:|
| Field Deck Mk0 | confirm mode copy and stable ID | P0 |
| SCAN Mode | fixture with physical dependency lines | P0 |
| DIAG Mode | fixture with failure-risk copy | P0 |
| ARCHIVE Mode | fixture with Chronicle link copy | P0 |
| CIVIC Mode | fixture with authority copy | P0 |
| NULL Mode Stub | add corruption warning state | P0 |
| Patch Cable | add cargo/physical interaction details | P0 |
| Patch Conduit Mk0 | add pressure/failure grades | P0 |
| Copper Conduit Pipe Segment | add carry vulnerability and state | P0 |
| Archive Witness Cartridge | add source-chain integrity field | P0 |
| Chronicle JSONL v0 | add event IDs and fixture schema | P0 |
| Proof-of-Repair Receipt | add grade and recognition status | P0 |
| Public Works Fabrication Bench | visible-locked fixture | P0 |

## 19.2 v0.2 Public Works Nodes

| Node | Required Action | Priority |
|---|---|---:|
| Certified Seal Kit | define recipe and quality outcomes | P0 |
| Certified Pipe Gauge | define calibration and drift failure | P0 |
| Pressure Test Rig | define pass/fail impact on repair grade | P0 |
| Public Tool Library | define checkout, return, dispute | P1 |
| Cargo Ledger Audit Station | define cargo source-chain rules | P1 |
| Fuel Depot Trust Console | define Proof-of-Repair redemption | P1 |
| Settlement Fabricator Bay | visible-lock until stable power and safety charter | P2 |
| Cold-Chain Vault | keep stub/foreshadow until biological cargo matters | P2 |

## 19.3 v0.3 Robotics and Xeno Nodes

| Node | Required Action | Priority |
|---|---|---:|
| mk0-scout Cable-Crawler | add permission envelope | P0 |
| Route Manifest Logger | add Chronicle event link | P0 |
| Machine Witness Policy | add acceptance/rejection criteria | P0 |
| Shared Tool Embassy | add consent and quarantine status | P0 |
| Translation Pool | add confidence threshold | P0 |
| Rights Forum Terminal | add license state | P0 |
| Hybrid Filter Alpha | add metabolic maintenance window | P0 |
| Translation Collapse | implement failure branch state | P1 |
| Overgrowth Without Consent | implement consent violation recovery path | P1 |

## 19.4 Horizon Nodes

| Node | Required Action | Priority |
|---|---|---:|
| Regional Proof-of-Repair Compact | add cross-settlement recognition rules | P1 |
| Trusted Cargo Corridor | add cargo fraud branch | P1 |
| Autonomous Public Works | require Machine Testimony + public override | P2 |
| Closed-Loop Shelter | require care, air, water, power, exit rights | P2 |
| Off-World Repair Sovereignty | require habitat public override doctrine | P3 |
| Deep-Time Archive Continuity | require dead-authority expiry | P3 |
| Interstellar Precursor Charter | keep roadmap until Gate 10 passes | P4 |

---

# 20. Implementation Tickets

## A1 — Add Audit Result Enum

Implement:

```text
TechAuditResult
```

Acceptance:

```text
all variants serialize and deserialize
invalid string fails schema validation
unit tests cover transition display copy
```

## A2 — Add TechNodeAudit Data

Implement:

```text
TechNodeAudit
```

Acceptance:

```text
audit score fixture loads
score total validates against component scores
missing requirement list appears in authoring error output
```

## A3 — Add Horizon Gate Data

Implement:

```text
HorizonGateState
HorizonGateStatus
```

Acceptance:

```text
Gate 1 fixture loads
Gate 2 remains locked until required events pass
Gate 3 displays robotics blockers
Gate 5 displays xeno consent blockers
```

## A4 — Add Node Fitness Validator

Implement validator rules:

```text
no playable node without player verb
no robot node without permission envelope
no xeno node without consent dependency
no fabrication node without material dependency
no Chronicle-producing node without event link
no NULL node without warning copy
```

Acceptance:

```text
bad fixtures fail loudly
good v0.1 fixtures pass
robot missing permission envelope fails
xeno missing consent dependency fails
```

## A5 — Add Golden Path Fixture

Create one fixture chain from:

```text
Patch Conduit Mk0 → Proof-of-Repair → Public Works Bench → mk0-scout → Shared Tool Embassy
```

Acceptance:

```text
runtime resolver can show locked/unlocked progression
Chronicle events update node status
UI can display next missing dependency
```

## A6 — Add Robotics Permission Envelope Fixtures

Create fixtures for:

```text
mk0-scout
mk0-gantry
mk0-aegis
mk0-biota
```

Acceptance:

```text
Robot Dock panel can render autonomy, route, permission, witness capacity, and failure modes
```

## A7 — Add Failure Branch States

Implement minimal failure branch data:

```text
MachineTestimonyRejected
TranslationCollapse
OvergrowthWithoutConsent
DeadAuthorityLock
```

Acceptance:

```text
failure branch can be active without ending the quest
Loom shows recovery action
Chronicle records failure state
```

## A8 — Add Horizon Gate UI Panel

Implement a reusable panel for gate readiness.

Acceptance:

```text
Gate title renders
six readiness categories render
primary blocker renders
risk if bypassed renders
Chronicle link renders
```

## A9 — Add Authoring Lint Script

Create a CLI or CI check:

```text
seedworks-tech-lint fixtures/tech_nodes/*.json
```

Acceptance:

```text
fails on missing player verb
fails on unresolved dependency
fails on invalid status
prints actionable error message
```

## A10 — Add Design Review Checklist

Create a Markdown template for node review.

Acceptance:

```text
designers can copy template
reviewers can approve or block nodes
review status can link to issue/ticket
```

---

# 21. Acceptance Tests

## 21.1 Document-Level Acceptance

This document succeeds if it helps the team answer:

```text
Which nodes are playable now?
Which nodes are visible but locked?
Which nodes are horizon-only?
Which nodes are shallow and should be rewritten?
Which future-tech nodes need legitimacy gates?
Which robotics nodes need permission envelopes?
Which failures should become branches instead of game-over states?
Which Chronicle events are missing?
Which UI panels must explain the lock?
```

## 21.2 v0.1 Acceptance

```text
The player can open the Loom, inspect Public Works Fabrication Bench,
see that it is locked by missing Proof-of-Repair,
complete Old Waterworks,
return to the Loom,
and see the bench state change because of a Chronicle event.
```

## 21.3 v0.2 Acceptance

```text
The player can fabricate a certified repair part only after material, power,
recipe, tool, and civic requirements are met.
```

## 21.4 v0.3 Acceptance

```text
The player can inspect Hybrid Filter Alpha and understand that it is not an alien item,
but a treaty between human fabrication, translation confidence, metabolic maintenance,
and rights forum consent.
```

## 21.5 Robotics Acceptance

```text
The player can inspect mk0-scout and understand its autonomy level, allowed route,
permission envelope, witness limits, maintenance needs, recall method, and failure modes.
```

## 21.6 Future-Tech Acceptance

```text
The player can inspect an interstellar horizon node and understand why the stars are not a reward tier,
but a civilizational audit that depends on habitat stewardship, off-world repair sovereignty,
deep-time archive continuity, and consent across distance.
```

## 21.7 Failure Branch Acceptance

```text
A failure branch can become active, visible, recoverable, and remembered without reducing the world to a binary success/failure state.
```

---

# 22. Design Review Template

Use this for every new node.

```text
NODE NAME:

STABLE ID:

MILESTONE:

STATUS:

DISCIPLINES:

DEPENDENCY LAYER / HORIZON GATE:

PLAYER VERBS:

MATERIAL REQUIREMENTS:

POWER REQUIREMENTS:

COMPUTATION REQUIREMENTS:

LEGITIMACY REQUIREMENTS:

MAINTENANCE REQUIREMENTS:

CONSEQUENCE / CHRONICLE EVENTS:

FAILURE MODES:

RECOVERY PATHS:

UI SURFACES:

NULL INTERACTION:

ROBOT PERMISSION ENVELOPE, IF APPLICABLE:

XENO CONSENT METADATA, IF APPLICABLE:

AUDIT SCORES:
- Material:
- Power:
- Computation:
- Legitimacy:
- Maintenance:
- Consequence:
- Total:

AUDIT RESULT:

REVIEWER NOTES:
```

Design rule:

```text
If a node cannot survive this template, it is not ready for the Loom.
```

---

# 23. How to Use This Document

## For Designers

Before adding a new node:

```text
1. Place it in a branch.
2. Assign a milestone or horizon gate.
3. Fill the design review template.
4. Add at least one failure mode.
5. Add at least one Chronicle event.
6. Define UI lock copy.
7. Run the audit score.
```

## For Writers

Use gates as story pressure.

Examples:

```text
Gate 3 creates labor and accountability stories.
Gate 5 creates consent and translation stories.
Gate 7 creates habitat rights stories.
Gate 10 creates intergenerational ethics stories.
```

Do not write future tech as destiny.

Write it as a public hearing under physics.

## For Engineers

Use this document to implement:

```text
schema validation
authoring lint
runtime gate states
permission envelope rendering
failure branch state machine
Chronicle event binding
Loom locked-state explanations
```

## For UI/UX

Every node should be able to show:

```text
why locked
why risky
why now possible
what changed
what can fail
what record exists
what the player can do next
```

## For Production

Use gates to protect scope.

```text
v0.1:
  Gate 1 proof only.

v0.2:
  Gate 2 and first Gate 3 visible-locks.

v0.3:
  Gate 3 partial, Gate 4 partial, Gate 5 first playable.

v1.0:
  Gate 1–6 interlocked.

Future expansions:
  Gates 7–10.
```

---

# 24. Final Principles

```text
No node without a body.
No body without power.
No power without consequence.
No computation without source chain.
No automation without interruption.
No robot without permission.
No witness without appeal.
No fabrication without certification.
No xeno-tech without consent.
No habitat without public override.
No archive without expiry of dead authority.
No interstellar future without ethics for those who cannot return.
```

The tech tree is not a reward ladder.

It is a public memory of what the settlement can safely become.

Final line:

```text
The Loom did not ask whether the future was impressive.
It asked whether the future could be maintained without becoming cruel.
```
