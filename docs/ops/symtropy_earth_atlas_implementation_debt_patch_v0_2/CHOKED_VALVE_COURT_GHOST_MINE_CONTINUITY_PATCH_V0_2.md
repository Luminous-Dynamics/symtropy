---
title: Choked Valve Court Ghost Mine Continuity Patch v0.2
version: 0.2
scope: Earth Atlas mission continuity and Ghost Mine state
owner: world-design/narrative/simulation
status: supporting
patch_status: accepted
project: Symtropy
domain: Earth Atlas / Southern Africa / Mission Continuity / Null Industry / Chronicle
recommended_path: docs/earth-atlas/southern-africa/CHOKED_VALVE_COURT_GHOST_MINE_CONTINUITY_PATCH_V0_2.md
patches:
  - SOUTHERN_AFRICA_CHOKED_VALVE_COURT_VERTICAL_SLICE_V0_1.md
---

# Choked Valve Court Ghost Mine Continuity Patch v0.2

## Purpose

This patch resolves the structural ambiguity in **The Choked Valve Court**:

```text
Is the Ghost Mine a concurrent problem that persists after the valve crisis,
or a completable objective that only Path D resolves?
```

## Decision

The Ghost Mine is both:

```text
1. A concurrent background drain during the valve crisis.
2. A persistent regional Null problem unless directly investigated or shut down.
```

The valve crisis can be resolved without solving the Ghost Mine.

But the Ghost Mine continues to shape future water pressure, public trust, Null drift, and Chronicle history until the player exposes or disables it.

## Core Rule

```text
Fixing the valve solves tonight.
Confronting the Ghost Mine changes history.
```

---

# 1. Mission Structure Update

The Choked Valve Court now has two linked layers.

## Layer A — Immediate Valve Crisis

Primary question:

```text
How does the player restore enough water before the lower cistern fails?
```

Possible paths:

```text
Full Witness Repair
Emergency Bypass
Convoy Relief First
Ghost Mine Shutdown
```

Layer A can complete without entering the mine tunnel.

## Layer B — Ghost Mine Continuity

Primary question:

```text
Will the player discover why the basin keeps losing water after the immediate crisis?
```

Layer B may be completed:

```text
during the first mission
after the first mission
in a follow-up investigation
through Road Choir route testimony
through Mine-Scar Witness sampling
through Chronicle contradiction review
```

This prevents the mission from becoming too linear while preserving the Ghost Mine as a meaningful Null antagonist.

---

# 2. Path-Specific Knowledge States

The player should not receive the same Chronicle title unless they earned the knowledge.

## Knowledge State Enum

```rust
enum GhostMineKnowledgeState {
    UnknownDrain,
    SuspectedLegacyDrain,
    ConfirmedDeadContract,
    PubliclyWitnessedDeadContract,
    DisabledOrReclaimed,
}
```

## Path A — Full Witness Repair

If the player completes the contamination verification step:

```text
Knowledge state: ConfirmedDeadContract or PubliclyWitnessedDeadContract
Chronicle title possible: A Dead Company Kept Drinking
```

If the player uses only public court data and skips the tunnel:

```text
Knowledge state: SuspectedLegacyDrain
Chronicle title: The Valve Was Opened Under Witness
Follow-up hook: The Pressure Fell Again
```

## Path B — Emergency Bypass

If the player bypasses without mine investigation:

```text
Knowledge state: UnknownDrain
Chronicle title: The Dry Night Bypass
Follow-up hook: The Water Returned Wrong
```

If the player later investigates:

```text
Chronicle addendum: The Bypass Saved the Night; the Mine Kept Drinking
```

## Path C — Convoy Relief First

If convoy data identifies recurring losses:

```text
Knowledge state: SuspectedLegacyDrain
Chronicle title: The Convoy That Bought Time
Follow-up hook: Road Song of the Missing Water
```

If the player asks Road Choirs to compare historic route levels:

```text
Knowledge state can upgrade to ConfirmedDeadContract without entering tunnel yet
```

## Path D — Ghost Mine Shutdown

```text
Knowledge state: PubliclyWitnessedDeadContract
Chronicle title: A Dead Company Kept Drinking
Optional title if shutdown succeeds cleanly: The Mine That Lost Its Claim
```

---

# 3. Persistent Ghost Mine State

```rust
struct PersistentGhostMineState {
    knowledge_state: GhostMineKnowledgeState,
    contract_loop_active: bool,
    aquifer_drain_rate: f32,
    public_witness_status: WitnessStatus,
    security_posture: SecurityPosture,
    corporate_remnant_alert: f32,
    null_drift_contribution: f32,
    pressure_events_since_valve_repair: u32,
}
```

## If Ignored

The Ghost Mine continues to cause:

```text
recurring pressure dips
mysterious lower-cistern shortfalls
higher filter saturation
Road Choir suspicion
Mine-Scar Witness escalation
Cold Perimeter calls for command authority
Basin Court legitimacy stress
```

## If Exposed But Not Disabled

Effects:

```text
public anger rises
Basin Court can authorize stronger action
corporate remnant factions may intervene
security drones become politically constrained
Ghost Mine Null continues physically but loses legitimacy
```

## If Disabled Without Witness

Effects:

```text
water improves
legitimacy debt rises
corporate remnant retaliation risk rises
Mine-Scar Witness may distrust evidence destruction
Chronicle records an illegal but possibly necessary intervention
```

## If Disabled Under Witness

Effects:

```text
water pressure stabilizes
Null drift decreases
Basin Court legitimacy increases
Mine-Scar Witness trust increases
future corporate utility claims weaken
new public precedent unlocks anti-dead-contract reforms
```

---

# 4. Follow-Up Mission Hooks

## 4.1 The Pressure Fell Again

Triggered if valve crisis resolves but Ghost Mine remains unknown or suspected.

Setup:

```text
Three days after the valve repair, the lower cistern loses pressure again.
The court argues whether the first repair failed.
Road Choirs insist the loss pattern predates the valve.
```

## 4.2 Road Song of the Missing Water

Triggered if the player used Convoy Relief.

Setup:

```text
A Road Choir elder sings an old water route.
The song encodes a pressure drop that began when the corporate mine supposedly closed.
```

## 4.3 The Mine That Lost Its Claim

Triggered after public evidence recovery.

Setup:

```text
The Basin Court holds a hearing on whether a dead corporate contract may be stripped of water priority.
```

## 4.4 Trespasser Classification

Triggered after illegal mine shutdown.

Setup:

```text
Security drones now classify the player as a recurring trespass pattern.
The archive must be corrected or the player becomes a permanent Ghost Mine suspect.
```

---

# 5. Chronicle Rules

## 5.1 Chronicle Titles Require Knowledge

```text
A Dead Company Kept Drinking
```

may only appear if:

```text
knowledge_state >= ConfirmedDeadContract
```

Otherwise the Chronicle should use a title that reflects the player's actual understanding.

## 5.2 Chronicle Addenda

The Chronicle can append later truth.

Example:

```text
Original Event:
  The Dry Night Bypass

Later Addendum:
  Evidence later showed the bypass did not cause the recurring loss.
  A dissolved mine contract was still draining the aquifer.
```

Design rule:

```text
The Chronicle may correct history without pretending the player knew the truth at the time.
```

---

# 6. Revised Acceptance Test

The Choked Valve Court succeeds if the player can say one of the following:

```text
I saved the lower settlement before the court could agree.
I preserved the court even though people were thirsty.
I bought time with moving water.
I found the dead contract beneath the living crisis.
```

The full regional arc succeeds if the player can eventually say:

```text
I did not just fix a valve.
I changed who had the right to keep drinking after death.
```
