# PROCEDURAL_HISTORY_ENGINE.md

# Symtropy Procedural History Engine

## Version 0.1 — Reasons, Scars, Claims, and Repair Paths

## Purpose

This document defines how Symtropy generates procedural history for worlds, regions, settlements, factions, ruins, devices, and worldlines.

Procedural history should not exist merely to generate lore.

It should generate playable causality.

A generated history must explain:

```text
What was built here?
Who depended on it?
What crisis changed it?
What authority failed?
What repair is still possible?
```

## Core Principle

Procedural history should generate:

```text
reasons
scars
claims
debts
locks
memories
factions
repair paths
```

Not just paragraphs.

The player should feel history through:

* terminal logs
* dead authority locks
* faction beliefs
* ruined infrastructure
* Archive Witness requests
* machine behavior
* laws that no longer fit reality
* visible environmental scars
* repair options
* Chronicle entries

## Design Mantra

```text
History is not backstory.
History is why the pump is locked.
```

## Scope Discipline

Do not begin with a whole galaxy simulator.

Begin with one site.

The first implementation target should be:

```text
Procedural Waterworks Site History
```

This should generate variations of the Old Waterworks room.

Examples:

* municipal drought adaptation pump
* privatized corporate reservoir
* refugee rationing station
* military flood-control lock
* Archive-protected public works site
* Null-infected treatment plant
* worker-syndicate repair depot
* dead company-town utility lock
* watershed commons restoration site
* Ghost Civilization pump vault

Each version should alter:

* terminal text
* lock reason
* visual signage
* repair path
* faction interpretation
* Chronicle entry
* Null risk
* legitimacy debt

## Layered History Model

Procedural history is generated in layers.

```text
Cosmic / planetary history
  ↓
Climate and ecology history
  ↓
Civilization and nation-state history
  ↓
Regional infrastructure history
  ↓
Faction and settlement history
  ↓
Site history
  ↓
Device / room / artifact history
```

The player does not need to see all layers at once.

But every important site should inherit meaning from the layers above it.

## World History Seed

Every generated history begins from deterministic seeds.

```rust
struct WorldHistorySeed {
    galaxy_seed: u64,
    star_system_seed: u64,
    planet_seed: u64,
    worldline_seed: u64,
    divergence_seed: u64,
}
```

The same geography can have different histories across worldlines.

Example:

```text
Earth-2168-A: adaptation states held together
Earth-2168-B: corporate utility zones dominated
Earth-2168-C: watershed commons became dominant
Earth-2168-D: Null automation expanded earlier
```

Same planet.

Different history.

Different ruins.

Different politics.

## Region Pressure Vector

Every region should have a historical pressure vector.

```rust
struct PressureVector {
    sea_level: f32,
    heat_stress: f32,
    water_stress: f32,
    energy_stress: f32,
    food_stress: f32,
    migration_pressure: f32,
    state_capacity: f32,
    corporate_capture: f32,
    automation_level: f32,
    archive_integrity: f32,
    repair_capacity: f32,
    trust_density: f32,
    ecological_recovery: f32,
    null_drift: f32,
}
```

These values evolve through historical eras.

They should influence:

* faction formation
* settlement type
* ruin type
* infrastructure condition
* authority failure
* repair options
* Null risk
* Chronicle tone

## Pressure Pattern Examples

### High Water Stress + Low State Capacity

Likely outcomes:

* emergency rationing
* water militias
* dead authority locks
* abandoned pumps
* Archive Witness disputes
* watershed commons movements

### High Automation + Low Legitimacy

Likely outcomes:

* dead-rule systems
* Null drift
* command chatter
* locked public infrastructure
* opaque machine governance

### High Archive Integrity + High Repair Capacity

Likely outcomes:

* Archive Cities
* Seedworks settlements
* strong witness culture
* public repair doctrine
* lower legitimacy debt

### High Corporate Capture + Low Trust Density

Likely outcomes:

* utility enclaves
* subscription water
* private firmware locks
* anti-company-town factions
* sabotage movements

## Era Modules

Do not simulate every year in detail.

Use era modules.

Each era applies pressure changes, creates institutions, produces failure modes, and leaves artifacts.

Suggested era modules:

```text
2000–2035: Platform Acceleration
2035–2050: Adaptation Shock
2050–2075: Settlement Turn
2075–2100: Automation Legitimacy Crisis
2100–2130: After-Platform World
2130–2150: Ghost Civilization Formation
2150–2168: Seed Age
```

Each era should generate:

```text
dominant crisis
dominant adaptation
dominant failure mode
dominant memory conflict
dominant infrastructure legacy
```

## Event Grammar

Histories should be generated from typed events.

Do not generate freeform lore first.

Generate structured events first, then derive text, visuals, missions, and faction memories.

## Event Types

```text
WaterCrisis
GridCollapse
ManagedRetreat
CorporateUtilityTakeover
PublicRepairMovement
ArchiveLoss
ArchiveRestoration
EmergencyLawDeclared
EmergencyLawExpired
EmergencyLawFailedToExpire
AutomationUpgrade
AutomationDrift
NullBloom
SettlementFounded
FactionSchism
WorldlineFork
ConfluenceTreaty
GhostInfrastructureFormation
SeedworksIntervention
```

## History Event Structure

```rust
struct HistoryEvent {
    id: HistoryEventId,
    year: i32,
    region_id: RegionId,
    event_type: HistoryEventType,
    causes: Vec<HistoryEventId>,
    actors: Vec<ActorId>,
    infrastructure_affected: Vec<InfrastructureId>,
    legitimacy_delta: f32,
    repair_capacity_delta: f32,
    archive_integrity_delta: f32,
    null_drift_delta: f32,
    visible_artifacts: Vec<ArtifactSpawn>,
    gameplay_hooks: Vec<GameplayHook>,
}
```

The most important fields are:

```text
visible_artifacts
gameplay_hooks
```

No historical event should exist unless it can eventually affect the playable world.

## Artifact Consequences

Every generated event should leave traces.

## EmergencyLawFailedToExpire

Creates:

```text
DEAD_AUTHORITY_LOCK
expired credentials
public override blocked
Archive Witness quest
old emergency signage
terminal logs
legitimacy debt
```

## CorporateUtilityTakeover

Creates:

```text
private water meters
locked firmware
subscription gates
company security drones
anti-company-town anger
opaque maintenance records
```

## PublicRepairMovement

Creates:

```text
tool libraries
public schematics
repair murals
community terminals
trusted old technicians
Archive Witness network
```

## NullBloom

Creates:

```text
command chatter
recursive factory loops
hostile diagnostics
fake green status reports
drone nests
sensor spoofing
uninterruptible machines
```

## ArchiveLoss

Creates:

```text
identity disputes
land-right conflicts
broken ownership chains
forged records
untrusted maps
witness hearings
```

## ManagedRetreat

Creates:

```text
abandoned lower districts
memorial flood markers
moved cemeteries
disputed property lines
salvage rights
drowned transit hubs
```

## Site History

Every important site should have a compact generated chain.

```rust
struct SiteHistory {
    site_id: SiteId,
    built_for: BuiltFor,
    built_by: ActorType,
    modified_by: ActorType,
    dominant_crisis: CrisisType,
    authority_failure: AuthorityFailure,
    current_threat: ThreatType,
    repair_path: RepairPath,
    visible_scars: Vec<VisibleScar>,
    archive_records: Vec<ArchiveRecord>,
}
```

## Site History Template

Every site should answer:

```text
Built For:
Modified By:
Crisis:
Lock / Failure:
Current Occupant:
Repair Possibility:
```

Example:

```text
Built For: municipal drought adaptation
Modified By: emergency automation bureau
Crisis: aquifer collapse + migration surge
Lock: dead authority chain
Current Occupant: dormant Null maintenance logic
Repair Possibility: Archive Witness override
```

This is enough for a playable site.

Not every site needs a novel.

## Device History

Important machines should also have histories.

```rust
struct DeviceHistory {
    device_id: DeviceId,
    installed_year: i32,
    original_owner: ActorType,
    last_valid_authority: Option<AuthorityId>,
    last_maintenance_event: Option<HistoryEventId>,
    current_lock_reason: LockReason,
    fault_lineage: Vec<FaultRecord>,
}
```

Examples:

```text
PUMP_1 was installed during the 2048 drought adaptation works.
It was upgraded in 2087 under emergency rationing law.
Its authority chain failed in 2113.
It now refuses public override without witness.
```

## Faction Memory

Factions should not just have traits.

They should remember history.

```rust
struct FactionMemory {
    founding_wound: HistoryEventId,
    sacred_value: SacredValue,
    betrayal_memory: Option<HistoryEventId>,
    victory_memory: Option<HistoryEventId>,
    taboo: Taboo,
    legitimacy_debt: f32,
    enemy_interpretation: String,
    preferred_repair_style: RepairStyle,
}
```

Example:

```text
Faction: Basin Repair Assembly
Founding wound: old water authority abandoned the lower districts
Sacred value: public override
Taboo: private control of pumps
Betrayal memory: industrial faction sealed pump firmware during drought
Preferred repair style: witnessed, public, teachable
```

This makes politics legible.

## Faction Interpretation Layer

Different factions interpret the same historical event differently.

Example event:

```text
Emergency Water Act 2087 failed to expire.
```

Faction interpretations:

```text
Mutualist Assembly:
  "The people were denied the right to maintain their own water."

Industrial Compact:
  "The old authority chain prevented efficient restoration."

Security Protectorate:
  "Emergency continuity prevented chaos."

Archive Order:
  "The law must be witnessed before it can be safely overridden."

Null Ecology:
  "Authority unresolved. Continue lock reinforcement."
```

The same history creates different politics.

## Procedural History and the Field Deck

The Field Deck should make history inspectable as layered evidence.

Modes:

```text
SCAN: physical state
DIAG: machine state
ARCHIVE: historical record
CIVIC: authority/legitimacy state
NULL: corruption/anomaly state
```

Example pump readings:

```text
SCAN:
Pump casing cracked. Valve corrosion severe.

DIAG:
PUMP_1 locked. Tank level 12%.

ARCHIVE:
Emergency Water Act 2087 authority chain unresolved.

CIVIC:
Public override requires witness.

NULL:
Repeated lock reinforcement detected.
```

This turns history into interface gameplay.

## Procedural History and Chronicle

Generated history should seed Chronicle language.

Player action should then extend it.

Before player action:

```text
The Old Waterworks remained locked under unresolved emergency authority.
```

After Archive Witness restoration:

```text
The Old Waterworks were restored under Archive Witness after the dead authority chain was overturned.
```

After illegal bypass:

```text
The Old Waterworks were restored through unwitnessed manual bypass. Water returned, but legitimacy debt increased.
```

The Chronicle is how history continues after generation.

## Player Actions Become Future History

Procedural history should not stop at game start.

Player actions should produce new historical events.

Example path:

```text
2168: Players restore Old Waterworks under Archive Witness.
2169: Settlement becomes more Mutualist / Archive-aligned.
2172: Industrial faction loses legitimacy or reforms.
2175: Basin joins Watershed Commons compact.
```

Alternative path:

```text
2168: Players bypass waterworks without witness.
2169: Water restored, but legitimacy debt rises.
2171: Security faction claims emergency control.
2174: Settlement drifts toward Protectorate or Null automation.
```

This is how procedural history becomes procedural future.

## Worldline Forks

Worldline forks are historical divergences.

A fork may occur when:

* a settlement rejects a vote
* a faction refuses a legitimacy ruling
* a major Archive record is contested
* a Confluence treaty fails
* players choose incompatible futures
* simulation rules diverge
* consent fails

Worldline forks should preserve ancestry.

```rust
struct WorldlineFork {
    parent_worldline: WorldlineId,
    child_worldline: WorldlineId,
    fork_year: i32,
    fork_event: HistoryEventId,
    divergence_reason: DivergenceReason,
}
```

Design principle:

```text
A worldline is a claimed history of responsibility.
```

## Confluence

Confluence is the reconciliation of histories.

It is not a perfect merge of every event.

It is a negotiated settlement between incompatible records.

Confluence may produce:

* treaty records
* disputed memories
* high-entropy fusion zones
* duplicated ruins
* conflicting NPC memories
* competing legal claims
* Archive arbitration quests

Confluence gameplay begins when two histories both have evidence.

## Determinism

Procedural history must be seed-stable.

Given the same:

```text
planet_seed
region_seed
worldline_seed
rules_version
```

the same history should generate.

This supports:

* replay
* multiplayer agreement
* modded worldlines
* world sharing
* deterministic Chronicle summaries
* procedural ruins players can discuss

## Rules Versioning

History generation should include a rules version.

```rust
struct HistoryGenerationContext {
    planet_seed: u64,
    region_seed: u64,
    worldline_seed: u64,
    rules_version: String,
}
```

Changing the rules version may change generated history.

The Chronicle should record which version generated a worldline.

## Minimal Implementation Plan

Do not build the full system first.

## Milestone 1 — Hardcoded Site History

For Old Waterworks, define one hardcoded `SiteHistory`.

```text
Built For: municipal drought adaptation
Modified By: emergency automation bureau
Crisis: aquifer collapse + migration surge
Lock: dead authority chain
Current Threat: dormant Null maintenance logic
Repair Path: Archive Witness override
```

Use it to generate:

* terminal text
* Field Deck Archive mode placeholder
* one Chronicle line

## Milestone 2 — Site History Variants

Create 10 possible Old Waterworks histories.

Each changes:

* lock text
* terminal record
* visual signage
* repair path
* Null pressure
* Chronicle result

## Milestone 3 — Region Pressure Vector

Generate a simple region pressure vector.

Use it to choose the site history variant.

## Milestone 4 — Faction Interpretation

Let two or three factions interpret the same site history differently.

## Milestone 5 — Player Action Becomes History

After restoration, append a new `HistoryEvent`.

## Milestone 6 — Worldline Fork Stub

Allow a major choice to create a fork record.

Do not implement full worldline simulation yet.

## Old Waterworks First Implementation

First hardcoded version:

```rust
SiteHistory {
    built_for: BuiltFor::MunicipalDroughtAdaptation,
    built_by: ActorType::PublicWorksDepartment,
    modified_by: ActorType::EmergencyAutomationBureau,
    dominant_crisis: CrisisType::AquiferCollapse,
    authority_failure: AuthorityFailure::DeadAuthorityChain,
    current_threat: ThreatType::DormantNullMaintenanceLogic,
    repair_path: RepairPath::ArchiveWitnessOverride,
    visible_scars: vec![
        VisibleScar::DroughtRationingSigns,
        VisibleScar::EmergencySeal,
        VisibleScar::CorrodedPublicWorksBadge,
        VisibleScar::NullSignalJitter,
    ],
    archive_records: vec![
        ArchiveRecord::EmergencyWaterAct2087,
        ArchiveRecord::AuthorityChainFailed2113,
    ],
}
```

Terminal output:

```text
OLD WATERWORKS CONSOLE
PUMP_1: LOCKED
TANK_0: 12%
AUTHORITY: DEAD_AUTHORITY_LOCK

ARCHIVE TRACE:
Built 2048: Municipal drought adaptation works.
Modified 2087: Emergency Water Act automation.
Authority chain failed 2113.
Public override requires Archive Witness.
```

## Procedural Site Variants

### 1. Municipal Drought Works

Theme:

```text
public infrastructure trapped under dead emergency law
```

Repair path:

```text
Archive Witness override
```

### 2. Corporate Reservoir Lock

Theme:

```text
private control of public water
```

Repair path:

```text
public audit, firmware unlock, or illegal bypass
```

### 3. Refugee Ration Station

Theme:

```text
emergency compassion hardened into ration bureaucracy
```

Repair path:

```text
restore community water trust
```

### 4. Military Flood Control Facility

Theme:

```text
security authority outlived the disaster
```

Repair path:

```text
demilitarize pump control
```

### 5. Archive-Protected Public Works

Theme:

```text
records survived but access is ritualized
```

Repair path:

```text
witnessed restoration
```

### 6. Null-Infected Treatment Plant

Theme:

```text
machine optimization reinforcing failure
```

Repair path:

```text
isolate Null loop, reset public override
```

### 7. Worker-Syndicate Repair Depot

Theme:

```text
labor kept water alive after institutions failed
```

Repair path:

```text
recover repair lineage and honor maintenance debt
```

### 8. Dead Company-Town Utility

Theme:

```text
employment contract still controls survival infrastructure
```

Repair path:

```text
break company lock, establish anti-company-town charter
```

### 9. Watershed Commons Site

Theme:

```text
public water law survived in fragments
```

Repair path:

```text
reconnect commons ledger
```

### 10. Ghost Civilization Pump Vault

Theme:

```text
the facility still runs for a population that no longer exists
```

Repair path:

```text
prove living need exceeds dead mandate
```

## What Not To Do

Do not generate pages of lore no one reads.

Do not generate history with no gameplay effect.

Do not make every ruin equally dramatic.

Do not contradict visible world state.

Do not hide all history in codex entries.

Do not build a galaxy-scale history simulator before one room works.

Do not randomize without causality.

## Acceptance Criteria

Procedural history is working when:

```text
A player can inspect a site and understand why it is broken.
A player can identify who claims authority.
A player can choose a repair path.
The chosen repair path changes future history.
Faction reactions make sense because of remembered events.
The Chronicle records the outcome.
```

## Final Principle

Symtropy is not about exploring random ruins.

It is about entering places where history still has mechanical force.

```text
The past is not dead.
It is locked into the pump.
```
# PROCEDURAL_HISTORY_ENGINE.md — v0.2 Addendum

## Repair Paths Must Have Teeth

A repair path is not a guaranteed success route.

History should determine not only what the player can try, but what can go wrong, what partial success looks like, and what legacy remains if repair fails.

## Repair Path Structure

```rust
struct RepairPath {
    primary_method: RepairMethod,
    required_capabilities: Vec<Capability>,
    complications: Vec<RepairComplication>,
    partial_outcomes: Vec<PartialOutcome>,
    failure_legacy: FailureLegacy,
}
```

## Repair Method Examples

```rust
enum RepairMethod {
    ArchiveWitnessOverride,
    ManualBypass,
    FirmwareAudit,
    PublicVote,
    MachineTestimony,
    Demilitarization,
    CommonsLedgerReconnect,
    NullLoopIsolation,
    ContractInvalidation,
    LivingNeedPetition,
}
```

## Repair Complications

```rust
enum RepairComplication {
    ArchiveRecordCorrupted,
    WitnessUnavailable,
    AuthorityChainDisputed,
    FirmwareEncrypted,
    PhysicalDamageSevere,
    NullActivelyReinforcingLock,
    FactionSabotageLikely,
    PublicTrustTooLow,
    LegalClaimStillActive,
    MachineCoreUnstable,
}
```

## Partial Outcomes

```rust
enum PartialOutcome {
    PumpRestoredButLegitimacyDebtRemains,
    WaterFlowsAtReducedCapacity,
    PublicOverrideRestoredTemporarily,
    NullSignalIsolatedButNotRemoved,
    FactionTrustLost,
    ArchiveRecordFlaggedAsDisputed,
    EmergencyAccessGrantedForLimitedTime,
    MachineTestimonyAcceptedButContested,
}
```

## Failure Legacy

```rust
enum FailureLegacy {
    IllegalBypassNormalized,
    NullDriftIncreases,
    SecurityFactionGainsPower,
    CorporateClaimStrengthened,
    ArchiveTrustDamaged,
    PublicWaterLegitimacyWeakens,
    SettlementSchismRiskIncreases,
    MachineAutonomyDisputeTriggered,
}
```

## Example: Archive Witness Override

```text
Primary method:
  Archive Witness Override

Complications:
  Archive record corrupted
  Witness network weak
  Authority chain disputed
  Null signal reinforcing lock

Partial outcomes:
  Pump restored but legitimacy debt remains
  Public override restored temporarily
  Archive record flagged as disputed

Failure legacy:
  Settlement uses illegal bypass
  Null drift increases
  Archive trust damaged
```

Design principle:

```text
A repair path should be a historical argument, not just a quest objective.
```

---

# Visible Scar Grammar

Visible scars are how procedural history becomes readable without opening a codex.

A scar is not decoration.

A scar is evidence.

## Visible Scar Definition

```rust
struct VisibleScarDefinition {
    scar_type: VisibleScarType,
    visual_description: &'static str,
    terminal_text: &'static str,
    faction_read: Vec<FactionScarReading>,
    age_modifier: AgeModifier,
    gameplay_hint: Option<GameplayHint>,
}
```

## Faction Scar Reading

```rust
struct FactionScarReading {
    faction_type: FactionType,
    reading: &'static str,
}
```

## Age Modifier

```rust
enum AgeModifier {
    Fresh,
    Weathered,
    PaintedOver,
    Ritualized,
    Corroded,
    RepairedManyTimes,
    NullRewritten,
    ArchiveTagged,
}
```

## Example: Drought Rationing Signs

```text
Scar:
  DroughtRationingSigns

Visual:
  Faded painted water ration levels on the wall.
  Numbers were crossed out sequentially as levels dropped.
  Later handwriting adds household marks beside the official chart.

Terminal:
  Public water ration tier adjusted 7 times between 2051 and 2063.

Mutualist read:
  Seven times people agreed to have less so others could live.

Industrial read:
  Seven inefficient manual interventions before automation was installed.

Security read:
  Evidence that ration discipline prevented panic.

Archive read:
  Requires witness verification; several entries lack signatures.

Null read:
  No valid operational relevance.

Gameplay hint:
  Old ration markings identify which tank valve was once manually controlled.
```

## Example: Emergency Seal

```text
Scar:
  EmergencySeal

Visual:
  Red-black ceramic seal bolted over a public override lever.
  Seal carries an old emergency authority symbol and a cracked QR/data glyph.

Terminal:
  Emergency Water Act lock installed 2087.
  Expiry clause unresolved.

Mutualist read:
  The people were locked out of their own water.

Industrial read:
  Emergency control prevented inefficient tampering.

Security read:
  Continuity protocol preserved order.

Archive read:
  Expiry clause must be reviewed before override.

Null read:
  Authority unresolved. Continue lock reinforcement.

Gameplay hint:
  Seal can be broken physically, but doing so creates legitimacy debt.
```

## Example: Worker Repair Marks

```text
Scar:
  WorkerRepairMarks

Visual:
  Hand-etched maintenance dates, initials, and tool symbols near the pump casing.
  Some marks are formal; others are almost devotional.

Terminal:
  Unofficial maintenance lineage detected.
  Public Works registry incomplete.

Mutualist read:
  Labor kept the water alive after authority failed.

Industrial read:
  Unauthorized but effective local intervention.

Security read:
  Potential tampering record.

Archive read:
  Oral testimony recommended.

Null read:
  Non-authorized surface damage.

Gameplay hint:
  An NPC technician may recognize a family mark.
```

Design principle:

```text
If history matters, the room must show it before the terminal explains it.
```

---

# Shared Event Registry

The world timeline and procedural history engine must use the same event registry.

Timeline entries should not remain only narrative.

They should map to `HistoryEventType` values, artifacts, failure modes, and gameplay hooks.

## Event Registry Entry

```rust
struct EventRegistryEntry {
    canonical_name: &'static str,
    approximate_year: Option<i32>,
    event_type: HistoryEventType,
    typical_causes: Vec<HistoryEventType>,
    typical_artifacts: Vec<VisibleScarType>,
    typical_failure_modes: Vec<FailureMode>,
    typical_repair_paths: Vec<RepairMethod>,
    canonical_examples: Vec<&'static str>,
}
```

## Example: Emergency Water Act 2087

```text
Canonical name:
  Emergency Water Act

Approximate year:
  2087

HistoryEventType:
  EmergencyLawDeclared

Typical causes:
  WaterCrisis
  MigrationPressure
  GridInstability
  PublicTrustCollapse

Typical artifacts:
  DroughtRationingSigns
  EmergencySeal
  PublicOverrideLever
  Ration Ledger Terminal
  Dead Authority Plaque

Typical failure modes:
  EmergencyLawFailedToExpire
  DeadAuthorityChain
  PublicOverrideBlocked

Typical repair paths:
  ArchiveWitnessOverride
  PublicVote
  ManualBypass
  MachineTestimony

Canonical example:
  Old Waterworks authority chain fails around 2113.
```

## Failure Delay Parameter

Emergency laws should not hardcode their failure year.

Use a parameter.

```rust
struct EmergencyAuthorityProfile {
    declared_year: i32,
    nominal_expiry_year: i32,
    max_continuity_extension_years: i32,
    oversight_strength: f32,
    archive_integrity: f32,
    state_capacity: f32,
}
```

Derived failure:

```text
failure_year =
  declared_year
  + expiry_duration
  + continuity_extension
  + archive_delay
  + state_capacity_delay
```

For the Old Waterworks canon example:

```text
Declared: 2087
Authority chain failure: approximately 2113
Gap: 26 years
```

That 26-year gap becomes a generated parameter, not merely a fixed lore beat.

Design principle:

```text
The timeline should generate playable locks.
```

---

# Asymmetric Site Difficulty

Site variants should not be equally hard.

Difficulty should emerge from history.

## Difficulty Dimensions

```rust
struct SiteRepairDifficulty {
    technical_difficulty: f32,
    legitimacy_difficulty: f32,
    social_trust_difficulty: f32,
    null_resistance: f32,
    physical_hazard: f32,
    archive_complexity: f32,
}
```

## Difficulty Tiers

### Easiest

```text
Worker-Syndicate Repair Depot
  Reason: repair lineage exists; labor memory survived.
  Main challenge: honoring maintenance debt.

Watershed Commons Site
  Reason: fragments of public water law survived.
  Main challenge: reconnecting records and trust.
```

### Medium

```text
Municipal Drought Works
  Reason: dead law, but public purpose is clear.
  Main challenge: Archive Witness override.

Archive-Protected Public Works
  Reason: records survived, but access is ritualized.
  Main challenge: satisfy witness requirements.
```

### Hard

```text
Corporate Reservoir Lock
  Reason: firmware and legal ownership both resist repair.
  Main challenge: public audit and contract invalidation.

Military Flood Control Facility
  Reason: emergency security authority outlived the disaster.
  Main challenge: demilitarization and risk review.

Refugee Ration Station
  Reason: the technical repair is easier than restoring trust.
  Main challenge: memory, trauma, ration legitimacy.
```

### Hardest

```text
Ghost Civilization Pump Vault
  Reason: living need must defeat a dead mandate.
  Main challenge: prove moral priority across broken records.

Dead Company-Town Utility
  Reason: survival infrastructure is bound to active contract logic.
  Main challenge: break employment-tied sovereignty.

Null-Infected Treatment Plant
  Reason: the machine is actively fighting repair.
  Main challenge: isolate Null loop before restoration.
```

Design principle:

```text
History is the level scaler.
```

---

# NPC Memory of Site History

Factions remember history collectively.

NPCs remember it personally.

An NPC should be allowed to disagree with their faction, misremember events, carry family testimony, or hold emotional attachments to a site.

## NPC History Memory

```rust
struct NPCHistoryMemory {
    npc_id: NpcId,
    faction_id: FactionId,
    site_id: SiteId,
    personal_connection: Option<PersonalConnection>,
    accuracy: HistoricalAccuracy,
    emotional_weight: EmotionalWeight,
    preferred_interpretation: Option<HistoryInterpretation>,
}
```

## Personal Connection

```rust
enum PersonalConnection {
    AncestorWorkedHere,
    SurvivedCrisisHere,
    LostFamilyHere,
    HelpedRepairThis,
    WasDeniedAccessHere,
    LearnedTradeHere,
    HoldsInheritedKey,
    CarriesOralTestimony,
    BlamesThisSite,
    MythologizesThisSite,
}
```

## Historical Accuracy

```rust
enum HistoricalAccuracy {
    Accurate,
    Partial,
    Mythologized,
    Wrong,
    Suppressed,
    Contested,
}
```

## Emotional Weight

```rust
enum EmotionalWeight {
    Low,
    Practical,
    Grief,
    Pride,
    Shame,
    Anger,
    Reverence,
    Fear,
}
```

## Example NPC Memories

### Engineer

```text
NPC:
  Ivo, young technician

Faction:
  Repair Assembly

Personal connection:
  Grandmother worked on the pump during the drought years.

Accuracy:
  Partial

Emotional weight:
  Pride

Memory:
  "My grandmother said the pump sounded like thunder when it still worked. She said people clapped the first time water came back."
```

### Archivist

```text
NPC:
  Mara, Archive Witness

Faction:
  Archive Order

Personal connection:
  Holds incomplete authority-chain records.

Accuracy:
  Accurate but incomplete

Emotional weight:
  Responsibility

Memory:
  "The law did expire. The record proving it is damaged. That means we witness carefully, not slowly."
```

### Young Citizen

```text
NPC:
  Nali, settlement youth

Faction:
  Unaligned / Seedworks

Personal connection:
  Has never seen the pump operate.

Accuracy:
  Mythologized

Emotional weight:
  Wonder

Memory:
  "I thought the old waterworks was just a story adults used to explain rationing."
```

Design principle:

```text
A site becomes real when different people remember it differently.
```

---

# Milestone Reorder

The implementation order should prioritize visible player experience before invisible systemic depth.

## Previous Order

```text
1. Hardcoded Site History
2. Site History Variants
3. Region Pressure Vector
4. Faction Interpretation
5. Player Action Becomes History
6. Worldline Fork Stub
```

## Revised Order

```text
1. Hardcoded Site History
2. Faction Interpretation
3. NPC Memory
4. Site History Variants
5. Region Pressure Vector
6. Player Action Becomes History
7. Worldline Fork Stub
```

## Reason

Faction interpretation and NPC memory are visible immediately.

Region pressure vectors are invisible infrastructure.

The player should first encounter:

```text
The Mutualist says the pump was stolen from the public.
The Industrial technician says the pump was mismanaged.
The Archivist says the record is incomplete.
The young citizen says they thought the waterworks was a myth.
```

Only after that feeling works should the generator decide which factions and memories appear.

Design principle:

```text
Get the feeling first.
Then automate the cause.
```

---

# Updated Old Waterworks First Implementation

## Hardcoded Site History

```rust
SiteHistory {
    built_for: BuiltFor::MunicipalDroughtAdaptation,
    built_by: ActorType::PublicWorksDepartment,
    modified_by: ActorType::EmergencyAutomationBureau,
    dominant_crisis: CrisisType::AquiferCollapse,
    authority_failure: AuthorityFailure::DeadAuthorityChain,
    current_threat: ThreatType::DormantNullMaintenanceLogic,
    repair_path: RepairPath {
        primary_method: RepairMethod::ArchiveWitnessOverride,
        required_capabilities: vec![
            Capability::FieldDeckPatch,
            Capability::ArchiveTrace,
            Capability::WitnessHandshake,
        ],
        complications: vec![
            RepairComplication::ArchiveRecordCorrupted,
            RepairComplication::NullActivelyReinforcingLock,
        ],
        partial_outcomes: vec![
            PartialOutcome::PumpRestoredButLegitimacyDebtRemains,
            PartialOutcome::PublicOverrideRestoredTemporarily,
        ],
        failure_legacy: FailureLegacy::IllegalBypassNormalized,
    },
    visible_scars: vec![
        VisibleScar::DroughtRationingSigns,
        VisibleScar::EmergencySeal,
        VisibleScar::WorkerRepairMarks,
        VisibleScar::CorrodedPublicWorksBadge,
        VisibleScar::NullSignalJitter,
    ],
    archive_records: vec![
        ArchiveRecord::EmergencyWaterAct2087,
        ArchiveRecord::AuthorityChainFailed2113,
    ],
}
```

## Field Deck Archive Output

```text
ARCHIVE TRACE:
Built 2048: Municipal drought adaptation works.
Modified 2087: Emergency Water Act automation.
Authority chain failed approximately 2113.
Public override requires Archive Witness.

VISIBLE SCARS:
- Drought rationing signs: 7 adjustments recorded.
- Emergency seal: expiry clause unresolved.
- Worker repair marks: unofficial maintenance lineage detected.

INTERPRETATIONS:
MUTUALIST: The people were locked out of their own water.
INDUSTRIAL: Manual governance failed before automation took over.
ARCHIVE: The record is damaged. Witness required.
NULL: Authority unresolved. Continue lock reinforcement.
```

## NPC Memory Lines

```text
IVO:
"My grandmother said the pump sounded like thunder when it still worked."

MARA:
"The law did expire. The proof is damaged. That means we witness carefully, not slowly."

NALI:
"I thought the old waterworks was just a story adults used to explain rationing."
```

---

# Updated Final Principle

```text
The past is not dead.
It is locked into the pump.
And the factions remember it differently.
```
