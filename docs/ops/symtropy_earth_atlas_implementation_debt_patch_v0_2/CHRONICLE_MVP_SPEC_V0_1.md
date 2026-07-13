---
title: Chronicle MVP Spec v0.1
status: canonical-draft
project: Symtropy
domain: Chronicle / Civic Truth / Field Deck / Worldline History
recommended_path: docs/systems/chronicle/CHRONICLE_MVP_SPEC_V0_1.md
depends_on:
  - MULTIPLAYER_TRUTH_MODEL.md
  - Symtropy Design Doc: Death, Reconstitution, and Source-Chain Recovery.md
  - PROCEDURAL_HISTORY_ENGINE.md
---

# Chronicle MVP Spec v0.1

## Working Title

**What Society Remembers**

## Purpose

Many Symtropy documents now use Chronicle consequences.

This spec defines the minimum viable Chronicle system needed for Seedworks and the Earth Atlas vertical slices.

The Chronicle is not a quest log.

It is not a lore codex.

It is the durable civic memory layer that records events that should change legitimacy, faction posture, worldline interpretation, and future repair possibilities.

Core question:

```text
What should society remember?
```

---

# 1. Scope for MVP

The MVP Chronicle records only **meaningful public events**.

Do record:

```text
public infrastructure repair
emergency override
public hearing outcome
Archive Witness testimony
source-chain recovery
NPC death with civic consequence
faction schism
settlement charter change
dead authority correction
machine testimony amendment
worldline fork declaration
```

Do not record by default:

```text
every bullet
every footstep
ordinary item pickup
minor crafting
private combat with no civic effect
routine repairs with no contested authority
```

Design rule:

```text
Fast action happens.
Meaningful action becomes history.
```

---

# 2. Chronicle Event Schema

```rust
struct ChronicleEvent {
    event_id: EventId,
    worldline_id: WorldlineId,
    region_id: RegionId,
    site_id: Option<SiteId>,
    timestamp_local: GameTime,
    event_title: String,
    event_class: ChronicleEventClass,
    summary_public: String,
    actor_chain: Vec<ActorRef>,
    witness_set: Vec<WitnessRef>,
    evidence_refs: Vec<EvidenceRef>,
    authority_refs: Vec<AuthorityRef>,
    affected_systems: Vec<SystemRef>,
    legitimacy_delta: LegitimacyDelta,
    faction_deltas: Vec<FactionDelta>,
    field_deck_snapshot: Option<FieldDeckSnapshot>,
    null_drift_delta: f32,
    worldline_flags: Vec<WorldlineFlag>,
    open_questions: Vec<OpenQuestion>,
    addenda: Vec<ChronicleAddendum>,
}
```

## 2.1 Event Classes

```rust
enum ChronicleEventClass {
    RepairPrecedent,
    EmergencyOverride,
    PublicHearing,
    DeadAuthorityCorrection,
    MachineTestimonyAmendment,
    SourceChainRecovery,
    EcologicalWitness,
    XenoContactUncertainty,
    FactionSchism,
    SettlementCharter,
    DeathContinuityEvent,
    WorldlineFork,
}
```

---

# 3. Evidence Model

Evidence is not automatically truth.

Evidence has provenance, integrity, and interpretation.

```rust
struct EvidenceRef {
    evidence_id: EvidenceId,
    evidence_type: EvidenceType,
    source: EvidenceSource,
    chain_of_custody: ChainOfCustodyStatus,
    integrity: f32,
    interpretation_confidence: f32,
    contested_by: Vec<FactionId>,
}
```

Evidence types:

```text
Field Deck scan
machine log
human testimony
vehicle scar ledger
route song witness
water sample
toxic sample
photographic capture
acoustic ledger read
source-chain artifact
machine category audit
nonhuman agency uncertainty record
```

---

# 4. Witness Model

A witness can be:

```text
player
NPC
Archive Witness
Basin Court representative
Mine-Scar Witness
Road Choir route elder
machine archive process
nonhuman ecological indicator
Field Deck source chain
```

Witnesses are not all equivalent.

```rust
struct WitnessRef {
    witness_id: WitnessId,
    witness_type: WitnessType,
    credibility_context: String,
    faction_affiliation: Option<FactionId>,
    authority_scope: Vec<AuthorityScope>,
    bias_tags: Vec<BiasTag>,
}
```

Design rule:

```text
Chronicle truth is stronger when different kinds of witness converge.
```

---

# 5. Reader Model

A Chronicle event matters because someone reads it later.

Readers include:

```text
settlement councils
Basin Courts
Archive Witness Enclaves
Road Choirs
Machine Archives
corporate remnants
Rangers
refuge committees
future players
NPC descendants
worldline migrants
```

```rust
struct ChronicleReaderEffect {
    reader_faction: FactionId,
    visibility: VisibilityLevel,
    trust_change: f32,
    policy_unlocks: Vec<PolicyUnlock>,
    hostility_delta: f32,
    rumor_distortion_risk: f32,
}
```

Visibility levels:

```text
private
party
site-local
regional
faction-network
worldline
confluence
sealed-until-witnessed
```

---

# 6. Consequence Model

Chronicle events can modify:

```text
legitimacy
access rights
repair authorization
faction trust
Null drift
public morale
worldline fork risk
legal precedent
machine category permissions
future mission availability
```

Example:

```yaml
event_title: A Dead Company Kept Drinking
event_class: DeadAuthorityCorrection
effects:
  Basin Court trust: +0.12
  Corporate Utility Remnant hostility: +0.18
  Ghost Mine Null drift: -0.22 if contract disabled
  future_policy_unlocks:
    - dead_contract_review
    - public_aquifer_priority_challenge
  future_mission_hooks:
    - The Mine That Lost Its Claim
```

---

# 7. Addenda

The Chronicle must support later correction.

```rust
struct ChronicleAddendum {
    addendum_id: AddendumId,
    parent_event_id: EventId,
    timestamp_local: GameTime,
    title: String,
    summary: String,
    evidence_refs: Vec<EvidenceRef>,
    changes_interpretation: bool,
    legitimacy_delta: Option<LegitimacyDelta>,
}
```

Design rule:

```text
The Chronicle can grow more truthful without pretending the past was fully understood.
```

Example:

```text
Original event:
  The Dry Night Bypass

Addendum:
  The recurring pressure loss was later traced to a dissolved mine contract.
```

---

# 8. Field Deck UI MVP

The Field Deck should show Chronicle events in three layers.

## 8.1 Immediate Record

```text
EVENT RECORDED:
The Dry Night Bypass

VISIBILITY:
Site-local / disputed

OPEN QUESTIONS:
Recurring pressure loss unresolved.
```

## 8.2 Civic Consequence

```text
CIVIC EFFECT:
Emergency authority increased.
Basin Court legitimacy decreased.
Lower cistern household survival improved.
```

## 8.3 Future Hook

```text
FOLLOW-UP:
Pressure anomaly unresolved.
Mine-Scar Witness requested review.
```

---

# 9. Chronicle and Death

When the player dies, the Chronicle may preserve what the body cannot.

Death-related Chronicle events include:

```text
source-chain recovered
source-chain unrecovered
public testimony accepted
identity disputed
reconstitution witnessed
death record altered by Null
```

Design rule:

```text
The player's continuity is not only biological.
It is what survives verification.
```

---

# 10. Acceptance Tests

Chronicle MVP is ready when:

```text
1. A water repair creates a durable public event.
2. An illegal bypass creates legitimacy debt.
3. Later evidence can add an addendum.
4. Different factions read the same event differently.
5. A machine testimony amendment changes future access.
6. A player can inspect why a permission changed.
7. A Chronicle event can unlock or close a repair path.
8. A death/source-chain event can affect authority.
9. A worldline variant can change which Chronicle precedents already exist.
10. The Chronicle records uncertainty without collapsing it into false certainty.
```

---

# 11. Mantra

```text
History is not a summary of what happened.
History is a machine that decides what can happen next.
```
