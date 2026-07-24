---
title: Relativistic Navigation, Time Dilation, and Causal Coordination Runtime
version: 0.1
status: implementation-spec
scope: relativistic travel, proper time, causal messaging, navigation uncertainty, traveler chronology, replay and persistence
owner: engineering/simulation/networking/worldline
related:
  - ../canon/INTERSTELLAR_CIVILIZATION_RELATIVISTIC_DISTANCE_AND_LOCAL_SOVEREIGNTY_CONTRACT_V0_1.md
  - LIGHT_DELAY_COMMUNICATION_TIMEKEEPING_AND_ASYNC_COORDINATION_RUNTIME_V0_1.md
  - VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Relativistic Navigation, Time Dilation, and Causal Coordination Runtime

## Purpose

This specification defines the minimum authoritative runtime needed for sub-light interstellar travel without pretending that every ship requires a full numerical relativity engine.

The runtime must preserve the consequences players can reason about:

```text
travel time
proper-time difference
causal message order
energy and propellant cost
navigation uncertainty
crew and habitat consumption
arrival-state uncertainty
worldline ancestry
```

> **Approximate the mathematics only where the player-facing invariants remain exact.**

# 1. Authority Boundary

This runtime owns:

```text
mission trajectories
reference frames
proper-time accumulation
message light cones
navigation solutions
arrival windows
relativistic chronology
transit-state persistence
```

It does not own:

```text
political legitimacy
crew consent
private memory
market prices
contact interpretation
mission purpose
```

It publishes verified physical and chronological state to those systems.

# 2. Core Types

Conceptual schema:

```rust
struct StellarBodyRef {
    system_id: StableId,
    body_id: StableId,
}

struct ReferenceFrameId(StableId);

struct CausalTimestamp {
    frame: ReferenceFrameId,
    coordinate_time_ns: i128,
    uncertainty_ns: u64,
}

struct ProperTimeState {
    entity_id: StableId,
    elapsed_ns: i128,
    integration_error_ns: u64,
}

struct RelativisticTrajectory {
    trajectory_id: StableId,
    origin: StellarBodyRef,
    destination: StellarBodyRef,
    departure: CausalTimestamp,
    segments: Vec<TrajectorySegment>,
    proper_time_estimate_ns: i128,
    coordinate_arrival_estimate: CausalTimestamp,
    navigation_covariance: NavigationCovariance,
    solver_version: ContentHash,
}

struct TrajectorySegment {
    duration_ns: i128,
    acceleration_profile: AccelerationProfile,
    start_state: KinematicState,
    end_state: KinematicState,
    gamma_range: (f64, f64),
    energy_budget: EnergyBudget,
}
```

The runtime may use reduced-order kinematics for gameplay, but every saved trajectory must record the solver and approximation version that produced it.

# 3. Chronology Model

The simulation tracks at least four clocks:

```text
simulation canonical time
local civic time
vehicle proper time
message emission and reception time
```

## Rules

1. Proper time is monotonic for each entity.
2. Coordinate time is monotonic within a named reference frame.
3. Cross-frame comparisons are derived, never assumed.
4. A calendar conversion may change presentation, not historical ordering.
5. A migration may improve precision, but cannot rewrite an already witnessed causal order silently.

# 4. Time-Dilation Approximation

For bounded interstellar gameplay, the runtime may model special-relativistic time dilation through segment integration.

For a segment with velocity fraction `beta = v/c`:

```text
gamma = 1 / sqrt(1 - beta^2)
proper_delta = coordinate_delta / gamma
```

Acceleration segments may be integrated numerically or approximated by validated profiles.

## Required Error Envelope

Each trajectory stores:

```text
estimated proper time
estimated coordinate time
integration tolerance
navigation uncertainty
content and solver version
```

The UI must not present precision finer than the validated error envelope.

# 5. Navigation Solutions

A navigation solution is not a magic destination marker.

It consumes:

```text
origin ephemeris
destination ephemeris
known gravitational sources
propulsion envelope
energy and propellant
crew and habitat constraints
collision and dust model
communication plan
abort options
```

It produces:

```text
trajectory family
arrival distribution
proper-time estimate
energy profile
thermal profile
communication windows
uncertainty growth
failure and diversion cases
```

## Navigation Uncertainty

Uncertainty may arise from:

```text
stale destination observations
unknown small bodies
dust environment
propulsion drift
sensor calibration
reference-frame error
unexpected maneuver
stellar activity
```

The runtime should propagate bounded uncertainty rather than convert all distant objects into current exact positions.

# 6. Causal Message Routing

Messages travel through physical or speculative relay paths.

A route records:

```rust
struct CausalMessageRoute {
    route_id: StableId,
    hops: Vec<RelayHop>,
    earliest_arrival: CausalTimestamp,
    expected_arrival: CausalTimestamp,
    confidence: f32,
    censorship_risk: f32,
    loss_risk: f32,
}
```

A message cannot be consumed before its earliest physically valid arrival.

If a receiving settlement forks before the message arrives, the message must bind to ancestry and delivery policy rather than appear identically in every branch without an explicit rule.

# 7. Knowledge Frontiers

Every agent, institution, market, and fleet receives a `KnowledgeFrontier`:

```rust
struct KnowledgeFrontier {
    observer_id: StableId,
    remote_domain: StableId,
    latest_confirmed_event: Option<EventId>,
    observation_cutoff: CausalTimestamp,
    transit_uncertainty: DurationRange,
    source_confidence: f32,
}
```

Queries for current remote state must fail closed or return a prediction clearly marked as such.

Forbidden behavior:

```text
remote NPC reacts to an event whose signal has not arrived
remote market reprices from future production data
remote fleet obeys an order before transmission
remote court treats an unreceived law as binding
```

# 8. Traveler Arrival State

Arrival produces a reconciliation bundle:

```rust
struct RelativisticArrivalBundle {
    traveler_id: StableId,
    origin_departure_event: EventId,
    traveler_proper_time: i128,
    destination_coordinate_time: i128,
    authenticated_archives: Vec<ArchiveRef>,
    credentials: Vec<HistoricalCredential>,
    unread_messages: Vec<MessageId>,
    biological_and_machine_state: ArrivalHealthState,
    legal_status: PendingLocalReview,
}
```

The bundle proves chronology and custody. It does not grant current authority.

# 9. Crewed Transit

Crewed missions integrate with habitat metabolism.

Per transit tick, the runtime publishes:

```text
proper-time delta
life-support demand
radiation exposure
maintenance wear
crew fatigue pressure
communication windows
trajectory deviation
arrival forecast
```

Deep LOD may aggregate ordinary periods, but must preserve:

```text
deaths and births
consent and authority changes
mission revisions
major failures
unique cargo
message creation
worldline branch points
```

# 10. Autonomous Transit

Autonomous probes and arks may run under sparse supervision.

They require:

```text
mission charter
bounded autonomy envelope
self-diagnostic state
repair and replication policy
contact policy
shutdown and quarantine policy
archive lineage
```

The navigation runtime validates movement. Mission and contact systems decide permitted actions.

# 11. Failure Modes

## 11.1 Propulsion Shortfall

Consequences:

```text
missed arrival window
increased proper and coordinate time
consumable deficit
relay loss
new destination uncertainty
```

## 11.2 Navigation Drift

Consequences:

```text
arrival miss distance
destination intercept failure
unplanned flyby
increased rescue impossibility
```

## 11.3 Clock Corruption

The system must preserve:

```text
raw clock evidence
calibration history
alternative reconstructions
uncertainty
```

It may not simply snap all histories to one corrected time.

## 11.4 Relay Loss

Messages remain queued, reroute if permitted, or become permanently undelivered with recorded custody.

## 11.5 Worldline Fork During Transit

The mission remains one physical mission unless the worldline model explicitly creates separate realities.

Fork logic must account for:

```text
which branch receives later messages
which branch contains the physical vehicle
how unique identities remain unique
how claims and obligations diverge
```

# 12. Persistence

Every active trajectory checkpoint stores:

```text
kinematic state
reference frame
proper time
solver version
uncertainty
energy and propellant
habitat state link
message queues
mission authority state
content hashes
```

Replay must reproduce event ordering within declared tolerances.

# 13. Networking

Real-time multiplayer replicates local vessel state through the ordinary simulation layer.

Interstellar chronology is authoritative at the shard/worldline layer.

Clients receive:

```text
validated local physics state
trajectory summaries
knowledge cutoffs
arrival estimates
causal event receipts
```

Clients never calculate authoritative message arrival independently.

# 14. Player Legibility

The Field Deck should expose:

```text
SHIP PROPER TIME
ORIGIN ELAPSED TIME ESTIMATE
DESTINATION ELAPSED TIME ESTIMATE
LAST CONFIRMED DESTINATION STATE
ARRIVAL UNCERTAINTY
NEXT COMMUNICATION WINDOW
ABORT OPTIONS
```

Predictions must be visually distinct from observations.

# 15. Performance and LOD

Suggested regimes:

```text
R0 local maneuvering: ordinary vehicle physics cadence
R1 high-acceleration transit: reduced rigid-body, high navigation cadence
R2 cruise: analytical propagation with event interrupts
R3 deep background: checkpoint-to-checkpoint propagation
R4 archive-only: immutable completed trajectory summary
```

Transitions preserve:

```text
proper time
coordinate chronology
energy and propellant
unique entities
messages
mission and consent events
uncertainty
```

# 16. Verification Tests

Required automated tests:

1. No message arrives before its light-cone bound.
2. Proper time remains monotonic across save/load.
3. Segment integration stays within declared tolerance.
4. Calendar conversion does not reorder events.
5. A remote query cannot access future state.
6. A worldline fork does not duplicate a unique vehicle accidentally.
7. Arrival credentials remain historical until local review.
8. LOD transitions conserve chronology and resources.
9. Solver migration preserves prior witnessed ordering.
10. Distinct clients receive identical authoritative arrival events.

# 17. Representative Fixture

```text
origin and destination: 4 light-years apart
vehicle: 0.8c cruise profile
outbound constitutional message
mid-transit mission amendment
one relay loss
one destination political fork
arrival after unequal proper and civic time
```

The fixture passes when chronology, message order, identity, local authority, and player explanation remain coherent after save, restore, migration, and fork.

# Hard Invariants

```text
no superluminal ordinary message path
no remote current-state query
no negative or decreasing proper time
no authority inferred from clock authenticity
no trajectory without solver version and uncertainty
no LOD transition that loses unique people, cargo, messages, or branch points
no worldline fork that duplicates one physical traveler by accident
```
