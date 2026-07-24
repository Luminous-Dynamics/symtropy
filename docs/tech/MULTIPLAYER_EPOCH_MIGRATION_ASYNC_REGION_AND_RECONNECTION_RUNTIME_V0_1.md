---
title: Multiplayer Epoch Migration, Asynchronous Region, and Reconnection Runtime
version: 0.1
status: implementation-spec
scope: multiplayer time separation, future migration, asynchronous worlds, region advancement, reconnection, host migration, uniqueness
owner: networking/simulation/engineering/design
related:
  - MULTIPLAYER_TRUTH_MODEL.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
  - ATLAS_TIME_PROPER_TIME_KNOWLEDGE_TIME_AND_CAUSAL_GRAPH_RUNTIME_V0_1.md
  - ../canon/ATLAS_METRIC_ENGINEERING_FTL_AND_CAUSALITY_CONTRACT_V0_1.md
  - ../canon/SEED_SHIP_PIONEER_VOYAGE_DEEP_TIME_AND_PLAYER_CONTINUITY_CONTRACT_V0_1.md
---

# Multiplayer Epoch Migration, Asynchronous Region, and Reconnection Runtime

## Purpose

This runtime allows one player to undertake a long interstellar voyage without forcing every connected player to skip the same years and without leaving a duplicate of the traveler behind.

It treats time separation as **forward migration among authoritative regional frontiers**, not as casual time travel.

> **Players may become separated by history. They may not use multiplayer convenience to occupy both sides of that separation.**

# 1. Core Model

A worldline contains multiple regional simulation frontiers.

```rust
struct WorldlineRuntime {
    worldline_id: WorldlineId,
    ancestry_root: Hash,
    regions: BTreeMap<RegionId, RegionFrontier>,
    player_locations: BTreeMap<PlayerId, PlayerTemporalLocation>,
    unique_asset_registry: UniqueAssetRegistry,
    route_graph_version: Hash,
}

struct RegionFrontier {
    region_id: RegionId,
    simulated_through: AtlasInstant,
    snapshot_root: Hash,
    active_session_count: u32,
    pending_causal_inputs: Vec<CausalInputRef>,
    advancement_policy: AdvancementPolicy,
}
```

Regions may be simulated through different Atlas instants while they remain causally disconnected.

A later connection requires advancement or selection of compatible descendant state before exchange.

# 2. Player Temporal Location

```rust
struct PlayerTemporalLocation {
    player_id: PlayerId,
    avatar_id: StableId,
    worldline_id: WorldlineId,
    region_id: RegionId,
    atlas_time: AtlasInstant,
    proper_time: Fixed,
    continuity_root: Hash,
    transit_state: Option<TransitId>,
    authority_tokens: Vec<AuthorityTokenId>,
}
```

Exactly one authoritative temporal location exists for each unique player avatar and continuity root in a worldline.

Offline state remains a location, not an invitation to spawn a second current instance.

# 3. Forward Epoch Migration

A player begins epoch migration when they commit to:

- a relativistic voyage;
- a seed mission;
- deep-time passage;
- an Atlas transit to a later regional frontier;
- a long-duration suspension or reconstitution schedule.

The migration transaction records:

- origin region and Atlas time;
- destination or voyage region;
- minimum future frontier;
- continuity root;
- unique carried assets;
- companion and household participation;
- messages and promises left behind;
- return conditions;
- player-facing disclosure acceptance.

After commit, the avatar no longer acts at origin.

# 4. Players Remaining Behind

Players remaining in the origin region continue at the origin frontier.

They may:

- receive delayed messages from the traveler;
- observe launch and later signal events;
- continue settlement and political history;
- build a future Atlas endpoint;
- undertake their own voyage;
- advance the region through ordinary play;
- create a declared branch.

They may not summon the traveler back into their current past state.

# 5. Joining a Future Player

A player may join another player's future epoch through one of four valid paths.

## 5.1 Natural Advancement

The origin region advances through play or bounded simulation until it reaches a compatible Atlas instant.

## 5.2 Causal Voyage

The joining player undertakes their own journey and arrives later.

## 5.3 Atlas Reconnection

Compatible endpoints open a future-directed route.

## 5.4 New Local Character

The player creates or assumes a different locally valid character in the future region.

This does not move the original avatar or duplicate their possessions.

The interface must distinguish "join as yourself" from "join this world as another person."

# 6. Invitation Semantics

A social invitation is not a temporal authority token.

Before accepting an invitation to a later epoch, the UI discloses:

- Atlas-time difference;
- whether the player's current avatar can travel;
- expected proper time;
- whether return infrastructure exists;
- assets that can travel;
- unresolved offices, custody, dependents, and promises;
- whether the action is reversible before commit;
- whether a new local character is the only non-destructive option.

# 7. Asynchronous Region Advancement

Inactive or low-population regions advance through deterministic event stepping.

## 7.1 Required Preserved State

- named people;
- households;
- companions;
- births and deaths;
- source-chain events;
- offices and authority;
- infrastructure;
- ecology;
- professions;
- public services;
- promises;
- campaigns;
- construction;
- route projects;
- culture;
- migration;
- messages;
- random seeds and branch decisions.

## 7.2 Aggregable State

- repeated routine shifts;
- stable production cycles;
- ordinary consumption;
- low-impact travel;
- routine maintenance;
- common anonymous market exchange.

Aggregation must reconcile conservation and capacity.

## 7.3 Mandatory Interruption Points

- regional catastrophe;
- major political transition;
- companion death or departure;
- player-owned office or custody transition;
- Atlas route activation;
- alien contact;
- worldline fork;
- loss of a unique artifact or settlement;
- event named in a player promise.

# 8. Advancement Proposal

```rust
struct RegionAdvanceProposal {
    region_id: RegionId,
    from: AtlasInstant,
    to: AtlasInstant,
    causal_inputs: Vec<CausalInputRef>,
    deterministic_seed_root: Hash,
    protected_entities: Vec<StableId>,
    interrupt_rules: Vec<InterruptRule>,
    expected_summary: SummaryEnvelope,
}
```

Advancement is first simulated in a reviewable staging state for high-consequence regions.

Production servers may auto-approve bounded low-risk advancement under declared policy.

# 9. Reconnection

Reconnection is not simply putting two region processes online.

It requires:

1. worldline compatibility;
2. causal frontier compatibility;
3. route validity;
4. knowledge synchronization;
5. authority reconciliation;
6. unique-asset reconciliation;
7. migration and quarantine policy;
8. conflict discovery;
9. player-facing history summary.

A destination region may be advanced to the earliest compatible arrival frontier, but its internal history must be generated from prior state and causal inputs rather than copied from another world.

# 10. Conflicting Regional Futures

Two servers or player groups may have advanced descendants from the same ancestor.

They are separate branches unless an explicit merge protocol exists for non-unique authored content.

People, unique infrastructure, offices, property, and source chains do not merge.

Possible player choices:

- continue one branch;
- preserve both as separate worldlines;
- migrate a character through an allowed cross-worldline copy mode that is explicitly non-canonical;
- import authored templates without importing state;
- create a historical comparison archive.

# 11. Host Migration

Host migration transfers simulation service responsibility, not civic or temporal authority.

The new host must receive and verify:

- latest signed snapshot root;
- event log tail;
- region frontiers;
- player temporal locations;
- unique-asset registry;
- route reservations;
- Atlas transit transactions;
- pending messages;
- authority-token issuers;
- deterministic seeds.

A host cannot:

- advance Atlas Time unilaterally beyond policy;
- restore an earlier region while retaining later assets;
- mint authority tokens;
- cancel committed transits without the recovery protocol;
- edit player continuity roots.

# 12. Disconnect During Voyage

A disconnected player remains:

- aboard the ship;
- in suspension;
- in route custody;
- in a declared safe autonomous role;
- or represented by a continuity-preserving NPC policy.

The game may not teleport them to a lobby-safe origin that is now in their past.

Player-selected disconnect policy may include:

- continue routine duties;
- enter safe suspension;
- transfer bounded responsibility to a named companion;
- wake only for declared conditions;
- stop voyage advancement at next safe checkpoint.

# 13. Companion and Household Migration

Companions and dependents are not inventory.

Their migration requires:

- willingness;
- care plans;
- employment and household consequences;
- destination capacity;
- authority where applicable;
- transport and medical accommodations;
- departure consequences for origin relationships.

If a companion refuses, the player chooses whether to travel without them, delay, negotiate, or change plans.

# 14. Offices, Property, and Custody

Long migration may affect:

- elected office;
- emergency authority;
- employment;
- housing;
- guardianship;
- evidence custody;
- public-service responsibility;
- contracts;
- property maintenance.

The system requires handover, expiry, proxy, or abandonment rules before departure.

A time-separated player cannot remotely exercise an office whose constituency has lived years beyond the last valid mandate.

# 15. Messages Across Epochs

Messages travel only through valid channels.

A message includes:

- sender Atlas time;
- intended recipient continuity root;
- earliest valid reception;
- channel;
- expiry and privacy;
- whether it may be delivered after sender death;
- branch claim;
- custody.

A later player can send a message to an earlier region only if the channel's reception time remains later than the send event. It cannot arrive in the sender's past.

# 16. Matchmaking and Discovery

Public server discovery must display temporal context.

Examples:

```text
Firstlight / Atlas 221.4 / active regional present
South Cut Descendant / Atlas 247.1 / 25.7 years later
Seed Vessel Ardent / Atlas 229.8 / traveler proper year 6.2
Far Station Branch B / divergent from Atlas 233.0
```

Players must not enter a future world accidentally through a generic low-latency server list.

# 17. Storage and Performance

Use:

- content-addressed snapshots;
- append-only causal logs;
- region-level checkpoints;
- event compaction with preserved roots;
- deterministic advancement kernels;
- route-transaction journals;
- protected unique-entity indexes;
- background validation.

Old fine-grained logs may be compacted only after audit roots and replay equivalence are preserved.

# 18. Abuse and Griefing Controls

Prevent:

- inviting players into an irreversible future without disclosure;
- abandoning disconnected players in lethal transit through host action;
- duplicating inventory through branch reconnect;
- forcing group time skips;
- holding a region hostage by refusing advancement while absent;
- using route closure to erase migrants;
- spawning future copies to farm offices or reputation;
- exploiting host clock changes.

Moderation actions must preserve worldline and custody evidence.

# 19. Evidence Bundle

A multiplayer temporal benchmark exports:

- pre- and post-migration player locations;
- region frontier histories;
- host migration record;
- unique-asset reconciliation;
- route commit logs;
- disconnected-player policy execution;
- companion consent records;
- branch ancestry;
- advancement seeds and summaries;
- message timing proofs.

# 20. Acceptance Tests

1. Player A begins a twenty-year seed voyage while Player B continues at origin.
2. Player A does not remain controllable or lootable at origin.
3. Player B receives only causally valid messages.
4. Player B cannot invite Player A back into the earlier epoch.
5. Player B later joins through a valid voyage or route.
6. Host migration during seed travel preserves location and continuity.
7. Disconnect during suspension does not duplicate or teleport the player.
8. A companion refusal remains effective across the voyage invitation flow.
9. Two independently advanced regions become explicit branches rather than silently merging.
10. A seven-year inactive region advances identically from the same snapshot and inputs.

# Production Maxim

> **Multiplayer may bridge distance between players. It may not abolish the time their characters chose to cross.**
