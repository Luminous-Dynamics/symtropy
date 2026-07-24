---
title: Discoverable History, Archive, Ruin, and Environmental Storytelling Runtime
version: 0.1
status: implementation-spec
scope: player discovery of physical traces, records, testimony, interpretation, ruins, heritage, and historical uncertainty
owner: narrative-engineering/world/archives
related:
  - ../canon/ARCHIVES_HISTORIOGRAPHY_HERITAGE_AND_COLLECTIVE_MEMORY_CONTRACT_V0_1.md
  - KNOWLEDGE_ARCHIVE_AND_HISTORICAL_EVIDENCE_RUNTIME_V0_1.md
  - HISTORICAL_TEXTURE_LORE_PROVENANCE_AND_GENERATION_RUNTIME_V0_1.md
  - ../canon/HISTORICAL_CONTENT_AND_PLAYABLE_CAMPAIGN_CONTRACT_V0_1.md
---

# Discoverable History, Archive, Ruin, and Environmental Storytelling Runtime

## Purpose

Symtropy’s history must be encountered through the world rather than delivered primarily through a codex.

Discovery is a process of finding traces, establishing access, interpreting evidence, comparing accounts, and deciding what to preserve, publish, repair, return, or leave undisturbed.

> **A historical trace should first be a thing in the world, then a question, and only later—sometimes—a conclusion.**

## Discovery Object Types

```text
physical trace
functional remnant
ruin layer
artifact
administrative record
sensor record
private archive
oral testimony
machine witness
memorial
ritual practice
architectural adaptation
ecological scar
missing record
forgery
```

Each type has different authority, access, preservation, and interpretation rules.

## Core Record

```rust
struct HistoricalTrace {
    trace_id: TraceId,
    worldline_ancestry: WorldlineAncestry,
    trace_type: TraceType,
    physical_location: Option<WorldLocation>,
    provenance: ProvenanceState,
    custody: CustodyState,
    integrity: IntegrityState,
    access_policy: AccessPolicy,
    privacy_policy: PrivacyPolicy,
    observation_requirements: Vec<Requirement>,
    interpretation_refs: Vec<InterpretationId>,
    related_event_hypotheses: Vec<EventHypothesisId>,
}
```

## Discovery Stages

### 1. Encounter

The player notices a trace through:

- silhouette;
- material mismatch;
- unusual wear;
- sound;
- odor or atmospheric change;
- NPC behavior;
- map discontinuity;
- inaccessible service route;
- ritual avoidance;
- machine alert.

### 2. Access

Access may require:

- physical traversal;
- repair;
- permission;
- protective equipment;
- translation;
- relationship trust;
- legal order;
- community consent;
- waiting for environmental conditions.

Lockpicking is not a universal historical method.

### 3. Observation

Observation produces bounded facts:

```text
material
age estimate
construction sequence
damage pattern
record contents
sensor calibration
biological response
custody marks
```

The interface marks measurement uncertainty.

### 4. Interpretation

Interpretation combines observations with:

- domain expertise;
- cultural knowledge;
- known history;
- competing models;
- testimony;
- missing evidence.

The game may present several supported interpretations. It must not hide a single omniscient answer behind arbitrary skill level.

### 5. Social Circulation

The player may:

- tell a person;
- publish;
- testify;
- submit to an archive;
- return evidence to a community;
- conceal;
- falsify;
- sell;
- destroy;
- preserve privately.

Each path creates different custody, trust, justice, and rumor consequences.

### 6. Consequence

Discovery may alter:

- ownership claims;
- safety procedures;
- public memory;
- reputation;
- institutional legitimacy;
- family relationships;
- ecological protection;
- campaign pressures;
- memorial practice;
- successor-state identity.

## Ruin Layering

A ruin is not one past moment.

A site may contain:

```text
original construction
first adaptation
damage event
emergency repair
occupation
salvage
memorialization
later reuse
current ecology
```

Layers should be physically distinguishable where possible.

## Functional Remnants

Some historical sites still provide services:

- an old pump;
- emergency shelter;
- navigation beacon;
- seed vault;
- bridge;
- machine archive;
- atmospheric seal;
- ritual kitchen.

Preservation cannot assume the site should become a museum. Communities may need to repair, dismantle, or transform it.

## Environmental Storytelling Grammar

Environmental evidence should answer at least two of these questions and raise at least one more:

```text
What happened here?
Who used this place?
What system kept them alive?
What failed or changed?
Who repaired it?
Who was excluded?
What remains useful?
What is remembered incorrectly?
What does the current community need from it?
```

## Archive Interfaces

The Field Deck may organize discovered material by:

- provenance;
- location;
- people;
- institutions;
- event hypothesis;
- confidence;
- access scope;
- contradiction;
- worldline ancestry.

It must not convert all discovery into a linear collectible checklist.

## Historical Hypotheses

```rust
struct EventHypothesis {
    hypothesis_id: EventHypothesisId,
    claim: StructuredClaim,
    supporting_traces: Vec<TraceId>,
    contradicting_traces: Vec<TraceId>,
    assumptions: Vec<Assumption>,
    confidence: ConfidenceRange,
    author_or_school: SourceRef,
    visibility: VisibilityScope,
}
```

Players may hold private working hypotheses without changing public history.

## Testimony

Testimony records:

- speaker identity or protected pseudonym;
- direct observation versus hearsay;
- time elapsed;
- known pressures;
- consent for use;
- audience scope;
- corrections;
- translation chain.

Trauma, age, machine embodiment, or cultural difference do not automatically invalidate testimony.

## Privacy and Sacred Boundaries

Not every archive should be opened.

Protected classes may include:

- medical records;
- intimate correspondence;
- children’s information;
- source-chain material;
- locations of vulnerable people;
- sacred knowledge;
- hazardous technical information;
- noncontact signals.

A historical-completion reward may never override rights.

## Loss, Damage, and Conservation

Evidence can be:

- weathered;
- corrupted;
- copied;
- repaired;
- translated;
- moved;
- stolen;
- destroyed;
- returned.

Copying information does not duplicate unique physical artifacts or authority. Restoration must preserve the original and document intervention.

## NPC Discovery

NPCs discover and interpret history independently.

They may:

- bring evidence to the player;
- conceal it;
- publish first;
- misinterpret it;
- protect it;
- exploit it;
- form a historical movement;
- revise their beliefs.

The player is not the only archaeologist in the world.

## Revisit and Worldline Variation

A discovered site may later be:

- repaired;
- looted;
- flooded;
- protected;
- commercialized;
- returned to descendants;
- incorporated into ordinary life;
- disputed by a successor state;
- transformed into a memorial;
- left intentionally quiet.

Worldline variants preserve trace ancestry even when public interpretation diverges.

## Performance and LOD

Distant historical state may compress to:

```text
trace existence
integrity class
custody holder
public interpretations
active disputes
service function
```

Detailed geometry, loose objects, and local acoustic evidence stream only near the player. Compression may not erase unique artifacts, private access state, or custody history.

## Acceptance Tests

1. A player can discover a major historical conflict without opening a codex.
2. Physical traces never reveal private records without access.
3. Two supported interpretations can coexist.
4. NPCs can discover and circulate evidence off-screen.
5. Repairing a ruin changes both function and heritage state.
6. Destroyed evidence remains recorded as lost rather than silently forgotten.
7. Save/load preserves custody and hypothesis confidence.
8. A worldline fork preserves shared ancestry but independent later custody.
9. A generated description cannot invent evidence.
10. The player can choose preservation, use, return, or noninterference where physically and legally possible.

## Governing Principle

> **History should be legible enough to investigate, uncertain enough to debate, and material enough to damage or repair.**
