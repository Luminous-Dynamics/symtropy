---
title: Historical Pressure, Faction Clock, and Campaign State Runtime
version: 0.1
status: implementation-spec
scope: causal campaign progression, faction initiative, off-screen state, branching, replay, and worldline persistence
owner: simulation/narrative-engineering
related:
  - ../canon/HISTORICAL_CONTENT_AND_PLAYABLE_CAMPAIGN_CONTRACT_V0_1.md
  - ../canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
  - PROCEDURAL_FACTION_EVOLUTION.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
---

# Historical Pressure, Faction Clock, and Campaign State Runtime

## Purpose

Traditional quest systems often hide a linear script behind a set of journal flags. Symtropy requires a campaign runtime where pressure arises from world state, factions act independently, player absence matters, and outcomes remain deterministic enough to replay and migrate.

> **A campaign clock measures a causal process. It is not a timer disguised as history.**

## Core State

```rust
struct CampaignState {
    campaign_id: CampaignId,
    content_version: ContentVersion,
    worldline_id: WorldlineId,
    phase: CampaignPhase,
    pressures: Vec<PressureState>,
    factions: Vec<FactionCampaignState>,
    commitments: Vec<Commitment>,
    evidence_state: Vec<EvidenceState>,
    active_activities: Vec<ActivityInstance>,
    consequences: Vec<ConsequenceRecord>,
    revisit_tags: Vec<RevisitTag>,
    next_wake_time: SimTime,
}
```

The structure is illustrative. Implementations may differ while preserving the information boundaries.

## Pressure State

A pressure has:

```rust
struct PressureState {
    id: PressureId,
    dimensions: BTreeMap<PressureDimension, FixedPoint>,
    contributors: Vec<CausalContribution>,
    thresholds: Vec<PressureThreshold>,
    recovery_processes: Vec<RecoveryProcess>,
    observability: ObservabilityPolicy,
    last_update: SimTime,
}
```

Examples:

- flood exposure;
- habitat bearing fatigue;
- public distrust;
- clinic overload;
- debt-service stress;
- ecological disturbance;
- succession uncertainty;
- rumor saturation;
- food insecurity;
- emergency-authority normalization.

One scalar cannot safely represent all campaign pressure.

## Contribution Rules

Each contribution declares:

- source event;
- affected dimension;
- magnitude;
- duration or decay;
- spatial scope;
- confidence;
- whether it is authoritative, inferred, or predicted;
- replay identity.

Player actions do not write pressure directly. They create validated world events that contribute to pressure.

## Faction Initiative

```rust
struct FactionCampaignState {
    faction_id: FactionId,
    objectives: Vec<ObjectiveState>,
    available_capabilities: Vec<CapabilityRef>,
    commitments: Vec<CommitmentRef>,
    internal_blocs: Vec<BlocState>,
    planned_initiatives: Vec<InitiativePlan>,
    risk_tolerance: FixedPoint,
    legitimacy_by_domain: BTreeMap<DomainId, FixedPoint>,
    last_decision_trace: DecisionTraceId,
}
```

The typographical field above should be implemented as `internal_blocs`; the conceptual requirement is that factions contain disagreement rather than one mind.

Faction planning consumes:

- known state rather than omniscient state;
- objectives and obligations;
- available people, materials, authority, and routes;
- predicted consequences;
- internal bloc pressure;
- time and communication delay.

It produces candidate initiatives that enter the same authoritative validation path as player actions.

## Initiative Classes

```text
public service
repair and construction
negotiation
information publication
investigation
mutual aid
market action
labor action
security action
evacuation
ritual or memorial action
covert action
institutional reform
```

Covert actions still require capability, opportunity, and traceable effects.

## Campaign Phases

Campaign phases are descriptive summaries, not exclusive scripts.

Recommended phases:

```text
dormant inheritance
visible tension
mobilization
crisis
resolution attempt
transition
revisit
```

A campaign may move backward, split, or remain unresolved.

## Threshold Events

Crossing a threshold may:

- unlock an activity;
- alter faction priorities;
- change site state;
- trigger migration;
- require emergency authority;
- close a transfer window;
- expose or destroy evidence;
- generate a Chronicle candidate;
- create a successor institution.

Threshold processing must be deterministic for the same ordered event history.

## Player Influence

Player influence is represented through:

- completed actions;
- promises and broken commitments;
- evidence shared or withheld;
- resources delivered;
- relationships and trust by domain;
- offices or mandates lawfully held;
- public witness;
- physical presence;
- authored origin knowledge.

No “campaign influence” currency may silently replace these causes.

## Commitment Runtime

A commitment includes:

```text
promisor
beneficiary
terms
scope
created time
knowledge cutoff
required capability
expiry or review
status
evidence
consequences of breach
```

Promises are not completed by dialogue text. They complete when authoritative conditions are satisfied.

## Off-Screen Progression

Campaigns wake on:

- scheduled process updates;
- relevant world events;
- threshold proximity;
- faction initiative completion;
- player entry or departure;
- message arrival;
- save migration;
- worldline fork.

Background updates may aggregate ordinary activity but must preserve:

- named people;
- unique assets;
- active commitments;
- material conservation;
- evidence custody;
- rights and consent state;
- irreversible consequences.

## Levels of Detail

### L0 — Active Scene

Full local simulation and player interaction.

### L1 — Active Region

Named agents, factions, services, pressures, and routes update at bounded cadence.

### L2 — Background Region

Cohort and institutional summaries with explicit named exceptions.

### L3 — Historical Compression

Long absences resolve through deterministic event batches with preserved causal summaries and evidence.

LOD transitions require equivalence tests within declared error envelopes.

## Branching and Merge Rules

Campaign state may branch through:

- worldline fork;
- institutional split;
- geographic separation;
- unresolved competing authorities;
- player-hosted alternate scenarios.

Branches may share ancestry but may not share mutable unique entities after divergence.

Campaign merges are not ordinary state merges. Reconciliation requires an authored or procedural event that resolves duplicate claims, identities, records, and assets.

## Failure Recovery

Runtime failures must not corrupt worldline truth.

Required safeguards:

- append-only campaign event journal;
- deterministic identifiers;
- idempotent threshold processing;
- transactional consequence application;
- bounded retries;
- replay verification;
- safe fallback to last checkpoint;
- versioned migration.

## Observability

Operational traces may include:

- pressure contributions;
- threshold decisions;
- faction candidate scores;
- validation outcomes;
- LOD transitions;
- content-version migration.

Private beliefs, medical information, intimate state, and protected evidence remain outside ordinary telemetry.

## Acceptance Tests

1. Two identical event streams produce identical campaign state.
2. Removing the player still allows bounded faction progress.
3. A faction cannot perform an action without required capability or route.
4. An unknown event cannot affect a faction before information arrives.
5. A service dependency survives leadership defeat.
6. Save/load preserves commitments, clocks, and evidence custody.
7. LOD round trips preserve named people and unique assets.
8. A worldline fork prevents later cross-branch asset mutation.
9. Pressure recovery follows real repair or adaptation.
10. Generated dialogue cannot cross a threshold.

## Governing Principle

> **Campaign state is the history of validated actions under pressure—not a collection of narrative flags.**
