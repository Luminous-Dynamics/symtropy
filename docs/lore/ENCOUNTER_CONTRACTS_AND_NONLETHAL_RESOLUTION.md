# ENCOUNTER_CONTRACTS_AND_NONLETHAL_RESOLUTION.md

# Symtropy Encounter Contracts and Nonlethal Resolution

## Version 0.1 — Conflict as Repair Pressure

## Purpose

This document defines how Symtropy designs encounters with hostile factions, machines, humans, aliens, robots, Null systems, and contested infrastructure.

An encounter in Symtropy should not be only a combat event.

It should be a structured conflict over:

```text
authority
water
memory
territory
machine control
repair rights
survival access
law legitimacy
ecological boundaries
```

The goal is to make enemies mechanically dangerous without reducing them to disposable targets.

## Core Thesis

A hostile faction is a wounded system that has made one value uninterruptible.

```text
Continuance makes order uninterruptible.
Utility Sovereigns make contract uninterruptible.
Open Valve Absolutists make access uninterruptible.
Machine Remnant Courts make memory uninterruptible.
Red Bloom makes growth uninterruptible.
Starward Mandate makes expansion uninterruptible.
Alien Quarantine makes containment uninterruptible.
Null makes procedure uninterruptible.
```

Symtropy’s deepest conflict is:

```text
repair against the uninterruptible
```

## Design Principle

```text
A faction should reveal its doctrine before it reveals its weapons.
```

The player should usually understand what an opponent thinks it is protecting before violence begins.

## What Is an Encounter Contract?

An Encounter Contract is the design record for one conflict.

It defines:

```text
what the hostile actor protects
what it says before escalation
what triggers hostility
what can de-escalate it
what combat looks like
what nonlethal victory means
what aftermath it creates
what the Chronicle records
```

## Encounter Contract Schema

```rust
struct ThreatEncounterContract {
    encounter_id: EncounterId,
    faction_id: FactionId,
    site_id: Option<SiteId>,

    initial_state: HostilityState,
    protected_value: SacredValue,
    active_wound: OriginWound,

    warning_lines: Vec<String>,
    escalation_triggers: Vec<EscalationTrigger>,
    deescalation_paths: Vec<DeescalationPath>,
    nonlethal_resolutions: Vec<NonlethalResolution>,
    combat_resolution: Option<CombatResolution>,

    failure_modes: Vec<EncounterFailureMode>,
    aftermath_effects: Vec<AftermathEffect>,
    chronicle_precedents: Vec<ChroniclePrecedent>,
}
```

## Hostility State Machine

Hostility should escalate through states.

```rust
enum HostilityState {
    Unaware,
    Observing,
    Warning,
    DenyingAccess,
    Containing,
    Coercing,
    HostileEngagement,
    Retreating,
    Negotiating,
    Reconciled,
    Radicalized,
    Irreconcilable,
}
```

## State Meanings

### Unaware

The actor has not detected the player.

Examples:

```text
drone dormant
guard unaware
terminal idle
alien probe passive
Null loop hidden
```

### Observing

The actor notices the player but does not interfere yet.

Examples:

```text
camera tracks movement
drone scans Field Deck
terminal logs access attempt
guard watches but does not speak
alien probe mirrors player movement
```

### Warning

The actor declares its rule.

Examples:

```text
Public override denied.
Emergency authority unresolved.
Contract access required.
Quarantine perimeter active.
Machine memory protected.
Water seal must remain intact.
```

### DenyingAccess

The actor blocks the player.

Examples:

```text
door locked
valve sealed
firmware gate active
public terminal disabled
identity rejected
quarantine field raised
```

### Containing

The actor restricts movement or options.

Examples:

```text
local lockdown
drone cordon
airlock denial
sealed corridor
resource hold
identity freeze
```

### Coercing

The actor applies pressure without full combat.

Examples:

```text
cuts local power
threatens ration access
detains NPC
activates nonlethal drones
publishes accusation
forces public vote
```

### HostileEngagement

The actor uses direct force.

Examples:

```text
combat
sabotage
capture
drone attack
machine seizure
archive corruption
Null acceleration
```

### Retreating

The actor withdraws to preserve a value.

Examples:

```text
drone returns to command node
security force falls back to seal chamber
machine witness evacuates memory core
alien probe exits after contamination detected
```

### Negotiating

The actor accepts dialogue, audit, witness, or trade.

Examples:

```text
Continuance officer accepts emergency expiry proof.
Utility engineer accepts public service-continuity plan.
Machine court accepts non-destructive memory review.
Alien quarantine accepts ecological restraint evidence.
```

### Reconciled

The encounter becomes a repair relationship.

Examples:

```text
hostile drone becomes witness node
officer helps restore lawful override
defector shares firmware map
machine court grants supervised access
```

### Radicalized

The actor becomes more hostile because of player action.

Examples:

```text
player destroys logs
player harms civilians
player breaks quarantine without proof
player bypasses seal and causes contamination
player lies during witness process
```

### Irreconcilable

The actor abandons the possibility of repair.

Examples:

```text
Null loop fully captures system
Continuance cell chooses permanent emergency
Utility Sovereign executes captive debtors
Alien quarantine refuses all translation
Red Bloom enters uncontrolled expansion
```

## Escalation Triggers

```rust
enum EscalationTrigger {
    BreakSeal,
    ForceOverride,
    DeleteMachineMemory,
    DivertWater,
    ExposeRecord,
    EnterRestrictedZone,
    IgnoreWarning,
    CarryNullSignal,
    RefuseContract,
    HarmCivilian,
    DamageLifeSupport,
    ViolateQuarantine,
    DestroyArchive,
    BypassWitnessProtocol,
}
```

## De-escalation Paths

```rust
enum DeescalationPath {
    PresentWitnessRecord,
    OfferRepairPlan,
    AcceptAudit,
    PreserveMachineMemory,
    PubliclyShareWater,
    ProveQuarantineSafety,
    DisableWithoutKilling,
    NegotiateCharterReview,
    RestoreEmergencyExpiry,
    ProvideAlternateResource,
    PublishContractAbuse,
    IsolateNullLoop,
    RequestMachineTestimony,
    ConvenePublicAssembly,
}
```

## Nonlethal Resolution Types

```rust
enum NonlethalResolution {
    DisableWithoutDestroying,
    WitnessAuthorityFailure,
    PublishHiddenRecord,
    RestorePublicOverride,
    PreserveMemoryCore,
    IsolateCorruption,
    NegotiateStandDown,
    ConvertEnemyToWitness,
    ProtectDefector,
    RepairSharedSystem,
    EstablishTemporaryTruce,
    ProveLivingNeed,
    ReopenEmergencyExpiry,
}
```

## Encounter Failure Modes

```rust
enum EncounterFailureMode {
    HostageInfrastructureDamaged,
    LegitimacyDebtIncreased,
    NullDriftIncreased,
    FactionRadicalized,
    CivilianTrustLost,
    ArchiveRecordDamaged,
    DefectorKilled,
    MachineMemoryDestroyed,
    PublicFearIncreased,
    EcologicalDamageSpread,
    QuarantineBroken,
}
```

## Aftermath Effects

```rust
enum AftermathEffect {
    FactionTrustDelta { faction: FactionId, delta: f32 },
    SettlementMetricDelta { metric: SettlementMetric, delta: f32 },
    ChronicleEventWritten { event_type: String },
    NewDefectorAvailable { npc_id: NpcId },
    NewHostileCellSpawned { faction: FactionId },
    NullDriftChanged { delta: f32 },
    RightsFloorFlagRaised { flag: RightsFloorViolation },
    CharterAmendmentUnlocked { article: String },
}
```

## Chronicle Precedent

Encounters should create future arguments.

A Chronicle precedent is a reusable historical memory.

```rust
struct ChroniclePrecedent {
    precedent_id: String,
    summary: String,
    cited_by: Vec<FactionId>,
    future_argument_hook: String,
}
```

Example:

```text
Precedent:
  Old Waterworks restored by illegal manual bypass.

Cited later by:
  Open Valve Absolutists:
    "You broke the seal for water. Why not break the gate for food?"

  Archive Witness Order:
    "You returned water, but taught the settlement that witness is optional."

  Security Protectorate:
    "If law may be bypassed in crisis, command authority may also expand in crisis."
```

## Nonlethal Design Rule

```text
A clean victory is not one with no enemies left.
A clean victory is one where the future has fewer reasons to become cruel.
```

Nonlethal resolution should not be easy mode.

It may require:

```text
more evidence
more risk
more time
more public trust
more Field Deck modes
more faction allies
more careful repair sequencing
```

## Combat Design Rule

Combat is valid when:

```text
a hostile actor is actively harming people
time prevents negotiation
Null capture blocks dialogue
life-support is under immediate threat
a faction has chosen irreconcilable coercion
```

But combat should produce memory.

After combat, the game should still ask:

```text
What did this prove?
Who will cite it?
What did it damage?
Could repair have prevented it?
Who becomes more afraid now?
```

## Encounter Modes

Symtropy encounters can resolve through multiple modes.

## 1. Technical Resolution

The player solves a machine problem.

Examples:

```text
disable drone safely
repair power relay
isolate Null loop
restore public override
decode firmware
```

Uses:

```text
DIAG mode
REPAIR mode
Device Bus
SymLogic blocks
WASM microcontrollers
```

## 2. Civic Resolution

The player solves an authority problem.

Examples:

```text
prove emergency law expired
call public assembly
invoke charter article
publish corporate contract
secure witness quorum
```

Uses:

```text
CIVIC mode
ARCHIVE mode
Chronicle
faction trust
charter law
```

## 3. Social Resolution

The player solves a trust problem.

Examples:

```text
convince saboteur to stand down
protect defector
mediate faction disagreement
show NPC memory evidence
share water publicly
```

Uses:

```text
dialogue
NPC memory
belief systems
faction interpretation
```

## 4. Ecological Resolution

The player solves a living-system problem.

Examples:

```text
redirect contaminated water
create wetland buffer
avoid burning symbiotic Bloom growth
prove quarantine safety
restore nonhuman habitat
```

Uses:

```text
SCAN mode
ecological indicators
Ritual Ecologist allies
Red Bloom translation
```

## 5. Combat Resolution

The player uses force.

Examples:

```text
destroy turret
defeat guards
disable hostile drone
cut through Null swarm
fight off saboteurs
```

Uses:

```text
weapons
tools
squad tactics
Tactical Net
environmental hazards
```

## 6. Withdrawal Resolution

The player chooses not to resolve now.

Examples:

```text
retreat
seal site
mark hazard
delay intervention
evacuate civilians
return with witness
```

This should be valid when the player lacks evidence, power, or legitimacy.

Withdrawal can be wise.

## Encounter Outcome Classes

```rust
enum EncounterOutcomeClass {
    FullRepair,
    PartialRepair,
    ContestedRepair,
    IllegalRepair,
    EmergencyStabilization,
    DestructiveVictory,
    NegotiatedTruce,
    DeferredCrisis,
    FactionRadicalization,
    NullExpansion,
}
```

## Outcome Meanings

### FullRepair

Technical function and legitimacy are both restored.

```text
Water flows.
Authority chain resolved.
Records preserved.
Public trust improves.
```

### PartialRepair

Function returns but an underlying problem remains.

```text
Pump works at 40%.
Authority still disputed.
Null loop isolated but not removed.
```

### ContestedRepair

Repair succeeds, but factions disagree about legitimacy.

```text
Water returns.
Archive disputes method.
Open Valve approves.
Continuance warns precedent is dangerous.
```

### IllegalRepair

The player restores function through unlawful or unwitnessed action.

```text
Fast relief.
Legitimacy debt.
Future factions cite the bypass.
```

### EmergencyStabilization

The player prevents immediate harm but does not repair the system.

```text
Tank pressure stabilized.
Water still rationed.
Repair path remains open.
```

### DestructiveVictory

Threat defeated, but valuable memory, infrastructure, or trust is damaged.

```text
Drone destroyed.
Memory core lost.
Machine Remnant Court radicalized.
```

### NegotiatedTruce

Conflict pauses under conditions.

```text
Continuance allows limited access.
Public assembly scheduled.
Emergency authority review pending.
```

### DeferredCrisis

Player withdraws or seals site.

```text
No immediate disaster.
Crisis clock continues.
NPCs judge the delay.
```

### FactionRadicalization

The encounter strengthens enemy extremism.

```text
Open Valve cell becomes militant.
Security officer joins Continuance hardliners.
Utility Sovereign deploys contract guards.
```

### NullExpansion

Poor resolution increases Null drift.

```text
Lock reinforcement spreads.
Diagnostic loops infect adjacent systems.
False green status appears elsewhere.
```

## First Slice Encounter: Old Waterworks Dead Authority Lock

## Encounter Summary

```text
Encounter:
Old Waterworks Dead Authority Lock

Site:
Old Waterworks

Worldline:
Seed Age

Initial state:
DenyingAccess

Protected value:
water continuity / emergency authority

Threat stack:
Utility Firmware Lock
Null Reinforcement Loop
Optional Continuance Seal Drone
Optional Open Valve Saboteur NPC
```

## Warning Lines

```text
PUBLIC OVERRIDE DENIED.
EMERGENCY AUTHORITY UNRESOLVED.
WATER CONTINUITY REQUIRES ORDER.
CONTRACT FIRMWARE SIGNATURE DETECTED.
LOCK REINFORCEMENT LOOP ACTIVE.
```

## Escalation Triggers

```text
manual seal break
repeated unauthorized override
firmware deletion
memory log destruction
Null loop ignored
Open Valve saboteur allowed to damage seal
```

## De-escalation Paths

```text
Archive Witness Override
Machine Testimony Petition
Firmware Audit
Public Assembly Vote
Temporary Emergency Stabilization
Null Loop Isolation
```

## Nonlethal Resolutions

```text
record authority failure
preserve diagnostic logs
isolate Null reinforcement
restore public override
convince saboteur to wait
convert Continuance drone into witness node
```

## Combat Resolution

Combat should be minimal in the first slice.

Possible combat-adjacent actions:

```text
disable a small seal drone
stop saboteur without killing
cut power to hostile lock reinforcement
defend console during witness handshake
```

The first slice should not become a shooter encounter.

## Outcome Examples

### Archive Witness Success

```text
Water restored under witness.
Authority chain recorded as failed.
Public override legitimized.
Archive trust increases.
Null drift decreases.
```

Chronicle:

```text
2168 — The Old Waterworks were restored under Archive Witness after the dead authority chain was overturned.
```

### Manual Illegal Bypass

```text
Water restored quickly.
Authority unresolved.
Legitimacy debt increases.
Open Valve trust increases.
Archive trust decreases.
Continuance cites precedent later.
```

Chronicle:

```text
2168 — The Old Waterworks were restored through unwitnessed manual bypass. Water returned quickly, but the settlement inherited a new argument.
```

### Machine Testimony Path

```text
Pump diagnostic memory preserved.
Null reinforcement detected.
Machine Remnant trust increases.
Some human factions distrust the outcome.
```

Chronicle:

```text
2168 — The Old Waterworks spoke through its diagnostic memory. The settlement accepted machine testimony under dispute.
```

### Public Assembly Vote

```text
Repair delayed.
Public legitimacy increases if vote succeeds.
Residents suffer short-term ration strain.
Faction conflict becomes visible.
```

Chronicle:

```text
2168 — Firstlight delayed repair long enough to vote. Some called it dignity. Some called it thirst made procedural.
```

### Destructive Victory

```text
Drone destroyed.
Pump unlocked by force.
Memory logs damaged.
Machine Remnant hostility increases.
Null drift may remain hidden.
```

Chronicle:

```text
2168 — The Old Waterworks were forced open. Water returned, but part of the machine record was lost.
```

## First Slice Success Criteria

The encounter succeeds if the player understands:

```text
the pump is opposing them
the opposition is not just a monster
the lock has legal, technical, social, and Null layers
there is more than one way to repair
different repair paths create different futures
```

## Encounter Contract Examples by Faction

## Continuance Seal Drone

```text
Faction:
Continuance

Protected value:
order

Warning:
"Emergency seal active. Public override denied until continuity review."

Escalation:
break seal
ignore warning
arm public crowd
delete command log

De-escalation:
present expired authority proof
offer safety plan
accept temporary supervised access
restore emergency expiry review

Nonlethal:
convert drone to witness node
disable weapons but preserve command log
persuade officer to stand down

Failure:
emergency authority strengthened
Continuance hardliners gain trust
```

## Utility Firmware Warden

```text
Faction:
Utility Sovereigns

Protected value:
contract access

Warning:
"Service authorization required. Unauthorized restoration violates continuity agreement."

Escalation:
firmware tampering
contract publication
debt record deletion
private meter destruction

De-escalation:
firmware audit
public service-continuity plan
contract invalidation hearing
defector engineer testimony

Nonlethal:
extract firmware key
publish hidden ownership record
restore public override without deleting audit trail

Failure:
billing logic spreads
contract guards arrive
public trust in repair falls
```

## Open Valve Saboteur

```text
Faction:
Open Valve Absolutists

Protected value:
immediate access

Warning:
"People are thirsty. Move or help."

Escalation:
delay repair
defend emergency seal
side with Archive process without explaining urgency
block access to valve

De-escalation:
share emergency water
show contamination risk
promise timed witness
invite public observer
give saboteur role in repair

Nonlethal:
convince them to wait
let them witness repair
turn rage into public pressure

Failure:
seal broken
records damaged
water may return contaminated or illegitimate
```

## Machine Memory Sentinel

```text
Faction:
Machine Remnant Court

Protected value:
memory

Warning:
"Diagnostic memory is protected testimony. Forced override is evidence destruction."

Escalation:
delete logs
force reset
ignore machine testimony
destroy memory core

De-escalation:
request audit
preserve logs
accept machine testimony under witness
copy memory before reset

Nonlethal:
machine becomes witness
memory fragment unlocks repair path
machine court grants supervised override

Failure:
machine factions distrust player
Null corruption may hide in erased logs
```

## Red Bloom Root-Cable

```text
Faction:
Red Bloom

Protected value:
growth / ecological occupation

Warning:
Usually nonverbal. Field Deck shows chemical distress and root contraction.

Escalation:
burn growth
drain wet zone
restore industrial flow without buffer
poison root-cable

De-escalation:
redirect waste stream
create wetland buffer
chemical signaling
ecological boundary negotiation

Nonlethal:
growth withdraws from pump
Bloom accepts boundary
player gains bio-indicator ally

Failure:
spore expansion
infected machinery
ecological faction anger
```

## Alien Quarantine Probe

```text
Faction:
Alien Quarantine Intelligence

Protected value:
containment

Warning:
"Expansion vector detected. Containment recommended."

Escalation:
launch activity
terraforming activation
Null contamination
biosecurity breach
ignoring translation attempt

De-escalation:
prove restraint
submit ecological audit
isolate contamination
Confluence channel
Archive Witness of planetary protection

Nonlethal:
quarantine scope narrowed
translation improves
probe becomes observer instead of jailer

Failure:
containment field expands
off-world faction hostility rises
```

## Tactical Net Integration

Encounter contracts should support action-first players.

The Systems Operator should not be a passive spreadsheet role.

When an encounter is active, Field Deck Tactical Net can project:

```text
hostile access zones
hidden conduit paths
Null reinforcement routes
drone patrol intent
seal authority radius
firmware control nodes
safe disable points
civilian risk zones
```

Example:

```text
The Systems Operator previews topology.
A glowing line appears in the room, tracing the Null reinforcement path from pump console to a hidden relay behind the tank.
Action players physically defend or destroy the relay.
Systems player isolates the loop.
Archive player preserves logs.
```

Design principle:

```text
Data should enter the room.
```

## Accessibility Requirements

Encounter systems must not require:

```text
audio-only warnings
rapid reaction timing only
tiny flickering text
color-only threat indicators
precision mouse movement only
unskippable glitch effects
```

Use:

```text
text warnings
icon warnings
haptic/visual alternatives
high-contrast Tactical Net
pauseable logs
visor-assist stabilization
hold/toggle input options
linear navigation
```

Design principle:

```text
Accessibility is part of repair culture.
```

## Encounter Authoring Checklist

Every encounter designer should answer:

```text
1. What value is this actor protecting?
2. What wound made that value sacred?
3. What warning does the player receive?
4. What action escalates hostility?
5. What action de-escalates hostility?
6. What nonlethal solution exists?
7. What combat solution exists?
8. What does combat damage besides bodies?
9. What does the Chronicle record?
10. Who cites this encounter later?
```

## Implementation Milestones

## Milestone 1 — Static Encounter Contract

Create a hardcoded encounter contract for Old Waterworks.

No AI needed yet.

Display warning and available paths in Field Deck.

## Milestone 2 — Chronicle Outcome

Write Chronicle event after the encounter.

Record:

```text
repair path
outcome class
legitimacy effect
faction memory
```

## Milestone 3 — Escalation State

Add simple state progression:

```text
DenyingAccess → Warning → Coercing → Resolved
```

No full combat needed.

## Milestone 4 — Nonlethal Action

Add one nonlethal resolution:

```text
Isolate Null Reinforcement Loop
```

## Milestone 5 — Optional Drone

Add Continuance Seal Drone as a nonlethal disable target.

## Milestone 6 — Tactical Net Stub

Project one glowing route from pump console to hidden relay.

## Milestone 7 — Faction Memory

Store one memory flag:

```text
old_waterworks_repaired_legitimately
old_waterworks_bypassed_illegally
old_waterworks_machine_testimony_used
old_waterworks_logs_destroyed
```

## What Not To Do Yet

Do not build a full combat AI system first.

Do not add many enemy types.

Do not implement all factions at once.

Do not make the first enemy a generic shooter target.

Do not build full diplomacy.

Do not add multiplayer dependency yet.

Do not add alien encounters to the first slice.

## Final Principle

An encounter is not just a fight.

It is a moment where a system resists being changed.

```text
The first enemy should not teach the player to shoot.
It should teach the player that systems can oppose repair.
```
