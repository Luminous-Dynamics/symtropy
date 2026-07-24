---
title: Atlas Time, Proper Time, Knowledge Time, and Causal Graph Runtime
version: 0.1
status: implementation-spec
scope: authoritative chronology, relativistic clocks, FTL event ordering, knowledge latency, route validation, worldline ancestry, replay
owner: engineering/simulation/networking/research
related:
  - ../canon/ATLAS_METRIC_ENGINEERING_FTL_AND_CAUSALITY_CONTRACT_V0_1.md
  - RELATIVISTIC_NAVIGATION_TIME_DILATION_AND_CAUSAL_COORDINATION_RUNTIME_V0_1.md
  - INFRASTRUCTURE_LOCKED_INTERSTELLAR_TRANSIT_GATE_AUTHORITY_AND_FAILURE_RUNTIME_V0_1.md
  - MULTIPLAYER_WORLDLINE_BRANCHING_HOST_MIGRATION_AND_TEMPORAL_OWNERSHIP_RUNTIME_V0_1.md
  - ../canon/MULTIPLAYER_TEMPORAL_OWNERSHIP_WORLDLINE_AND_RECONNECTION_CONTRACT_V0_1.md
---

# Atlas Time, Proper Time, Knowledge Time, and Causal Graph Runtime

## Purpose

This runtime defines the minimum authoritative time model required for relativistic voyages, Atlas FTL, remote knowledge, multiplayer epoch separation, and deterministic replay.

It does not simulate a universal absolute physical clock. It provides a game-authoritative causal ordering that coexists with local relativistic clocks and incomplete observation.

> **The simulation may know what is true. A character may know only what has reached them. A traveler may experience less time than either place. None of those values may silently substitute for another.**

# 1. Time Domains

## 1.1 Atlas Time

Atlas Time is a monotonically increasing scalar within one authoritative worldline.

It owns:

- event ancestry;
- FTL departure and arrival ordering;
- route-graph validation;
- branch points;
- uniqueness transfer boundaries;
- multiplayer epoch ordering;
- deterministic event replay.

Atlas Time is not directly exposed as a single culturally universal calendar. It may be stored as integer attoseconds, fixed-point seconds, or another deterministic unit.

```rust
struct AtlasInstant {
    worldline_id: WorldlineId,
    ticks: u128,
}
```

Required invariants:

```text
child.ticks > parent.ticks for causally later state transitions
arrival.ticks >= departure.ticks + route.minimum_latency
branch_root.ticks == parent_branch_event.ticks
```

No floating-point value may be the sole authority for ordering.

## 1.2 Local Coordinate and Civic Time

Each system, settlement, ship, station, and institution may define local coordinate and civic representations.

Examples:

- orbital coordinate time;
- settlement day and season;
- ship mission day;
- legal term number;
- religious calendar;
- local stellar year.

```rust
struct LocalClockReading {
    frame_id: FrameId,
    atlas_reference: AtlasInstant,
    coordinate_seconds: Fixed,
    calendar_id: CalendarId,
    display_fields: CalendarFields,
    uncertainty: Duration,
}
```

Local clocks may drift, be reset, be politically disputed, or use different epochs. They may not override Atlas event ancestry.

## 1.3 Proper Time

Every relativistically relevant entity may accumulate proper time.

```rust
struct ProperTimeState {
    entity_id: StableId,
    last_atlas_update: AtlasInstant,
    elapsed_proper_time: Fixed,
    rate_solution: ProperTimeRate,
    solution_provenance: EvidenceRef,
}
```

Proper time affects:

- biological aging;
- metabolism;
- pregnancy and development;
- machine wear where physically appropriate;
- chemical processes;
- subjective voyage duration;
- contracts defined by experienced time;
- relationship and life-course continuity.

The runtime must distinguish deliberate suspension, low-detail simulation, and relativistic time dilation. They are not the same mechanism.

## 1.4 Knowledge Time

Knowledge Time describes the newest confirmed remote state available to an observer.

```rust
struct KnowledgeEnvelope {
    subject_region: RegionId,
    observer: ObserverId,
    observation_atlas_time: AtlasInstant,
    emission_atlas_time: AtlasInstant,
    reception_atlas_time: AtlasInstant,
    channel: InformationChannel,
    evidence_root: Hash,
    confidence: Confidence,
    branch_claim: WorldlineId,
    custody: Vec<CustodyEvent>,
    censorship_flags: Vec<CensorshipFlag>,
}
```

The current simulation state must never be copied into a character's knowledge merely because the data exists on the server.

# 2. Event Model

Every authoritative state transition receives a causal event envelope.

```rust
struct CausalEvent {
    event_id: EventId,
    worldline_id: WorldlineId,
    atlas_time: AtlasInstant,
    parent_events: Vec<EventId>,
    region_id: RegionId,
    actor_refs: Vec<StableId>,
    authority_refs: Vec<AuthorityTokenId>,
    input_hash: Hash,
    output_hash: Hash,
    event_kind: EventKind,
    visibility: VisibilityPolicy,
    evidence_refs: Vec<EvidenceRef>,
}
```

A causal event may depend on multiple parents, such as a route commit requiring evidence from both endpoints.

A merge of evidence is not a merge of worldlines.

# 3. FTL Route Edge

Each active Atlas route is represented as one or more directed causal edges.

```rust
struct AtlasRouteEdge {
    route_id: RouteId,
    worldline_id: WorldlineId,
    origin: EndpointId,
    destination: EndpointId,
    valid_from: AtlasInstant,
    valid_until: Option<AtlasInstant>,
    minimum_latency: Duration,
    nominal_latency: Duration,
    maximum_latency: Duration,
    mass_envelope: Mass,
    information_bandwidth: Bandwidth,
    temporal_offset: Duration,
    route_solution_hash: Hash,
    status: RouteStatus,
}
```

`minimum_latency` must be strictly positive for every FTL route.

`temporal_offset` may represent endpoint clock geometry or route-specific causal adjustment, but the final effective edge weight must remain positive.

# 4. Chronology Protection

## 4.1 No Nonpositive Cycle

Before activating or modifying a route, the runtime constructs the reachable route graph for the relevant worldline and validation horizon.

Every directed cycle must have strictly positive total Atlas duration.

```text
for every directed cycle C:
    sum(effective_latency(edge) for edge in C) > chronology_margin
```

The chronology margin accounts for:

- numerical uncertainty;
- synchronization error;
- route weather;
- endpoint motion;
- adversarial clock manipulation;
- model discrepancy.

If the proof is inconclusive, activation fails closed.

## 4.2 Incremental Validation

A full graph scan may be expensive. The runtime should maintain:

- strongly connected components;
- lower-bound path latencies;
- route dependency ancestry;
- invalidation regions;
- signed route solutions.

Adding one edge requires testing whether a path already exists from destination to origin whose lower-bound latency would create a nonpositive cycle.

## 4.3 Alien or Unknown Routes

Unknown infrastructure does not bypass validation.

It is modeled as an edge with conservative latency bounds and confidence.

If the minimum causal direction cannot be established, the route may be observed or studied but not integrated into the human Atlas graph.

# 5. Transit Transaction

A unique-object transit is a staged transaction.

## 5.1 Prepare

- freeze manifest revision;
- verify identity and source-chain roots;
- reserve destination capacity;
- reserve route mass and energy;
- validate quarantine;
- validate authority;
- compute minimum arrival Atlas Time;
- validate chronology graph;
- create transaction nonce.

## 5.2 Precommit

- lock transferable unique objects;
- write origin and destination intent records;
- establish crash-recovery witness set;
- stop mutable cargo edits;
- publish abort deadline.

## 5.3 Commit

The commit boundary creates exactly one authoritative transfer state.

```rust
struct AtlasTransitCommit {
    transit_id: TransitId,
    manifest_hash: Hash,
    origin_state_hash: Hash,
    destination_reservation_hash: Hash,
    departure_atlas_time: AtlasInstant,
    minimum_arrival_atlas_time: AtlasInstant,
    commit_event: EventId,
    state: CommitState,
}
```

After commit, objects are not independently active at origin.

## 5.4 Transit

During transit the manifest exists in a route-owned state.

It may accumulate:

- proper time;
- radiation and thermal exposure;
- field stress;
- onboard events;
- communications permitted by route geometry;
- abort or diversion options explicitly declared by the route.

## 5.5 Arrival

Arrival requires:

- arrival Atlas Time greater than or equal to the committed minimum;
- destination state compatible with the reservation;
- route solution continuity;
- manifest reconciliation;
- quarantine handoff;
- unique-object activation at destination;
- closure of origin transfer locks.

# 6. Crash Recovery

The runtime must recover one of four states:

```text
not prepared
prepared but not committed
committed and in transit
arrived and reconciled
```

Ambiguity is not resolved by spawning a second instance.

If evidence cannot establish whether arrival completed, the assets remain in disputed custody state until reconciliation.

A player may experience this as a legal, rescue, identity, or infrastructure crisis rather than a technical error screen.

# 7. Knowledge Propagation

## 7.1 Channels

Supported information channels include:

- local observation;
- ordinary electromagnetic communication;
- physical courier;
- Atlas communication lane;
- Atlas passenger or cargo transit;
- trusted archive synchronization;
- alien translated channel;
- rumor or unverified relay.

Each channel has:

- speed or latency;
- bandwidth;
- custody;
- reliability;
- privacy;
- censorship risk;
- branch specificity.

## 7.2 Predictions

IRIS and institutions may predict current remote state from old observations.

Predictions must carry:

- last confirmed observation;
- model version;
- assumptions;
- confidence decay;
- possible discontinuities;
- unavailable private state.

Predicted state is never silently promoted to observed truth.

## 7.3 Contradictory Reports

Conflicting reports remain separate evidence objects.

The runtime may indicate:

- mutually exclusive claims;
- likely clock mismatch;
- possible worldline mismatch;
- stale report;
- forged custody;
- uncertain translation.

It may not rewrite records into one convenient answer.

# 8. Epoch Separation

A region has an authoritative simulation frontier.

```rust
struct RegionEpoch {
    region_id: RegionId,
    worldline_id: WorldlineId,
    simulated_through: AtlasInstant,
    persistence_root: Hash,
    active_players: Vec<PlayerId>,
    pending_arrivals: Vec<TransitId>,
    pending_messages: Vec<MessageId>,
}
```

A traveling player may move to a future frontier. Other regions need not immediately advance to the same Atlas Time if no shared causal interaction requires it.

The worldline runtime must still ensure that later reconnection advances or selects compatible descendant states.

# 9. Region Advancement

Inactive regions advance through deterministic bounded simulation.

Advancement preserves:

- unique people;
- households;
- births and deaths;
- offices and authority;
- companion projects;
- infrastructure state;
- ecology;
- public services;
- route construction;
- messages;
- major conflicts;
- private-state boundaries;
- random decisions through recorded seeds.

The system may aggregate repetitive low-impact processes, but it cannot summarize away branch points or unique causal events.

# 10. Worldline Branching

A worldline branch records:

```rust
struct WorldlineBranch {
    branch_id: WorldlineId,
    parent_worldline: WorldlineId,
    branch_event: EventId,
    branch_atlas_time: AtlasInstant,
    reason: BranchReason,
    unique_asset_policy: UniqueAssetPolicy,
}
```

Branches may arise through:

- explicit player world creation;
- multiplayer fork;
- save-derived sandbox;
- incompatible future advancement;
- declared experiment;
- narrative worldline choice.

A branch does not create transferable duplicates between live authoritative worlds.

# 11. UI Contract

Any interstellar destination display must show at least:

```text
current local/Atlas relationship
latest confirmed observation time
estimated present state and confidence
expected departure Atlas Time
minimum and expected arrival Atlas Time
expected traveler proper time
route status and uncertainty
worldline identity
```

When values diverge, the interface must name the divergence rather than choosing one unlabeled date.

# 12. IRIS Contract

IRIS may:

- translate between clocks;
- explain time dilation;
- summarize stale knowledge;
- warn of route cycles;
- compare arrival options;
- remember promises whose deadlines use different clocks;
- identify source-chain and worldline risk.

IRIS may not:

- invent current remote observations;
- hide uncertainty to simplify the interface;
- decide that a time-lagged society is politically obsolete;
- merge branches;
- certify alien causal direction without evidence.

# 13. Multiplayer Ordering

Network messages include:

```rust
struct TemporalNetworkEnvelope {
    worldline_id: WorldlineId,
    sender_region: RegionId,
    sender_atlas_time: AtlasInstant,
    sender_proper_time: Option<Fixed>,
    receiver_region: RegionId,
    earliest_reception: AtlasInstant,
    sequence: u64,
    payload_hash: Hash,
    authority_proof: Option<Hash>,
}
```

The receiver rejects:

- earlier-than-allowed delivery;
- worldline mismatch without an explicit bridge;
- duplicate sequence and payload;
- authority proof issued after its own claimed use;
- host-generated timestamp substitution.

# 14. Performance Strategy

Chronology must scale without simulating every frame everywhere.

Use:

- fixed-point event time;
- region frontiers;
- event queues;
- hierarchical route graphs;
- incremental cycle validation;
- deterministic bulk advancement;
- explicit high-resolution activation windows;
- cached path latency lower bounds;
- signed snapshot roots.

The optimization may reduce computation. It may not change causal order.

# 15. Telemetry and Evidence

The benchmark must export:

- event ancestry hashes;
- route graph versions;
- clock synchronization errors;
- proper-time integration error;
- knowledge age distributions;
- chronology validation results;
- rejected route reasons;
- transfer transaction states;
- crash recovery decisions;
- region advancement seeds;
- worldline branch records.

# 16. Failure Conditions

Immediate failure includes:

- nonmonotonic Atlas event order;
- arrival before committed minimum;
- a nonpositive route cycle;
- duplicate unique assets after replay;
- remote knowledge updated before a valid channel arrival;
- proper-time aging applied twice or omitted;
- branch mismatch silently accepted;
- host clock used as civic or Atlas authority;
- crash recovery guessing by convenience;
- a stale prediction displayed as current observation.

# Acceptance Tests

1. A relativistic voyage produces different proper and Atlas durations.
2. Two settlements display the same Atlas event through different civic calendars.
3. A remote system changes without the player learning immediately.
4. An Atlas message arrives faster than ordinary light while remaining future-directed.
5. Adding a route that creates a nonpositive cycle is rejected.
6. A committed transit survives process crash without duplication.
7. An uncommitted transit aborts with origin assets intact.
8. A worldline mismatch enters quarantine rather than merging.
9. An inactive region advances deterministically to the same state from the same snapshot and event inputs.
10. Multiplayer host migration preserves event ancestry and route reservations.

# Production Maxim

> **Time is not one number. Causality is the agreement among all the numbers that matter.**
