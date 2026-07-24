---
title: Light-Delay Communication, Timekeeping, and Asynchronous Coordination Runtime
version: 0.1
status: implementation-spec
scope: delayed messages, communications infrastructure, clocks, causal ordering, asynchronous governance, relay custody, priority, privacy, outages, and replay
owner: engineering/networking/simulation/civic
implements:
  - ../canon/INTERPLANETARY_CIVILIZATION_LATENCY_AND_DISTRIBUTED_SOVEREIGNTY_CONTRACT_V0_1.md
authority_boundary: owns authoritative message envelopes, transmission state, clock conversion, relay custody, delayed delivery, and asynchronous coordination state; does not own message meaning beyond typed validation or local real-time networking
related:
  - MULTIPLAYER_TRUTH_MODEL.md
  - NETWORKING_STACK_DECISION.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
  - CHRONICLE_EVENT_SCHEMA.md
---

# Light-Delay Communication, Timekeeping, and Asynchronous Coordination Runtime

## Purpose

This specification defines how Symtropy transports information when sender and receiver cannot share the same present.

The runtime must preserve:

```text
what was known when a message was created
who authorized it
which route carried it
when it could physically arrive
whether it was altered, delayed, duplicated, censored, or lost
what legal effect it had on receipt
```

## Core Invariant

> **The receiver may learn only what physically and legally reached them.**

No user interface, NPC cognition, market, fleet, or civic system may read distant authoritative state directly.

# 1. Message Envelope

```rust
struct InterplanetaryMessage {
    message_id: MessageId,
    conversation_id: Option<ConversationId>,
    sender: PrincipalId,
    recipients: Vec<RecipientScope>,
    class: MessageClass,
    created_at: LocalTimestamp,
    created_epoch: SystemEpoch,
    knowledge_cutoff: SystemEpoch,
    valid_from: Option<SystemEpoch>,
    expires_at: Option<SystemEpoch>,
    payload_hash: ContentHash,
    payload_schema: SchemaId,
    disclosure: DisclosureScope,
    authority: Option<AuthorityEnvelope>,
    priority: PriorityClass,
    route_policy: RoutePolicy,
    signature: Signature,
}
```

Payloads are content-addressed. The envelope records authority and timing separately from language rendering.

## Message Classes

```text
Distress
CollisionWarning
Medical
LifeSupport
Navigation
Operational
CivicMandate
Treaty
CourtFiling
Trade
Scientific
Chronicle
Personal
Cultural
BulkArchive
```

Class influences queue priority but never permits unauthorized inspection.

# 2. Transmission Graph

Communication uses a physical graph:

```rust
struct CommunicationLink {
    source: RelayId,
    destination: RelayId,
    propagation_delay: Duration,
    bandwidth: BitsPerSecond,
    queue_capacity: Bytes,
    availability: AvailabilityState,
    energy_cost: EnergyRate,
    ownership: OwnershipId,
    operator: OperatorId,
    policy: LinkPolicy,
    error_model: ErrorModel,
}
```

Links may be:

```text
laser
radio
wired habitat network
surface relay
orbital relay
ship-to-ship
courier storage
alien medium
```

Propagation delay is derived from physical distance and medium. Queue delay, scheduling delay, censorship, and equipment failure are separate.

# 3. Route and Custody

A message route contains ordered hops.

```rust
struct MessageTransit {
    message_id: MessageId,
    current_hop: usize,
    route: Vec<RelayId>,
    custody_chain: Vec<CustodyReceipt>,
    bytes_delivered: u64,
    state: TransitState,
    earliest_arrival: SystemEpoch,
    latest_estimated_arrival: Option<SystemEpoch>,
}
```

Transit states:

```text
Queued
Transmitting
Propagating
Stored
Rerouting
HeldByPolicy
HeldForInspection
PartiallyReceived
Delivered
Expired
Corrupted
Lost
Rejected
```

Custody receipts prove possession and forwarding, not truth.

# 4. Priority and Congestion

Priority classes:

```text
P0 collision and immediate life-support warning
P1 distress, rescue, urgent medical, quarantine breach
P2 operational safety and navigation
P3 civic deadlines, court orders, treaty notices
P4 ordinary commercial and scientific traffic
P5 personal and cultural traffic
P6 bulk archive and entertainment replication
```

Rules:

- Safety priority may preempt bandwidth, but may not silently expose private content.
- Long emergencies require review; they cannot permanently starve ordinary personal traffic.
- Congestion creates visible queues and estimated arrival ranges.
- Operators can reserve minimum personal and civic capacity to prevent total institutional capture.

# 5. Clock Model

The runtime distinguishes:

```rust
struct EventClock {
    local_clock: ClockId,
    local_time: LocalTimestamp,
    system_epoch: SystemEpoch,
    uncertainty: Duration,
    synchronization_source: SyncSource,
    calibration_event: Option<EventId>,
}
```

## Required Times

Every durable event records:

```text
occurrence interval
observation time
recording time
send time
relay receipt times
final receive time
local display conversion
```

A clock correction appends calibration evidence. It never rewrites the original timestamp.

## Causal Ordering

The system uses:

```text
per-source sequence numbers
previous-hash chains
explicit references
vector or dotted causal metadata where needed
physical lower bounds from propagation
```

Wall-clock ordering alone never proves causality.

# 6. Knowledge Frontiers

Each settlement, habitat, ship, institution, and NPC context has a knowledge frontier:

```rust
struct KnowledgeFrontier {
    principal: PrincipalId,
    source_scope: SourceScope,
    latest_confirmed_epoch: SystemEpoch,
    latest_message_ids: Vec<MessageId>,
    unresolved_gaps: Vec<GapDescriptor>,
    confidence: f32,
}
```

Systems use local frontiers rather than global state.

Examples:

- A market prices cargo using the latest confirmed production report.
- A fleet follows a command issued before an unknown government collapse.
- A family speaks to a person who has since died, because confirmation has not arrived.
- A court pauses enforcement when a causal gap affects jurisdiction.

# 7. Asynchronous Civic Transactions

Civic messages may carry:

```text
proposal
mandate
ballot
provisional signature
ratification
objection
appeal
revocation
emergency delegation
review finding
```

A transaction state machine tracks:

```rust
struct AsyncCivicTransaction {
    transaction_id: TransactionId,
    participants: Vec<PrincipalId>,
    required_steps: Vec<CivicStep>,
    completed_steps: Vec<CivicReceipt>,
    assumptions: Vec<ConditionSnapshot>,
    activation_rule: ActivationRule,
    sunset_rule: SunsetRule,
    state: AsyncTransactionState,
}
```

States include:

```text
Draft
Mandated
InTransit
ProvisionallyAccepted
AwaitingRatification
Active
ConditionallyActive
Contested
Expired
Revoked
Forked
Reconciled
```

# 8. Delayed Command

A command envelope requires:

```rust
struct DelayedCommand {
    issuer: PrincipalId,
    authority_scope: AuthorityScope,
    objective: ObjectiveId,
    assumptions: Vec<ConditionPredicate>,
    branches: Vec<ContingencyBranch>,
    recipient_discretion: DiscretionBounds,
    issued_at: SystemEpoch,
    expires_at: SystemEpoch,
    reporting_required: bool,
}
```

The local action system validates the command against current conditions.

Possible results:

```text
Accepted
AcceptedWithAdaptation
Deferred
RejectedOutOfScope
RejectedExpired
RejectedChangedConditions
EmergencyOverride
```

Every deviation produces a reviewable reason, not an automatic disloyalty flag.

# 9. Personal Communication

Personal messages preserve:

```text
sender identity at send time
relationship context at send time
privacy scope
attachment provenance
whether delivery confirmation exists
```

NPC dialogue may refer to delayed messages only after receipt. Drafts, unsent recordings, and private archives remain inaccessible unless shared or lawfully recovered.

# 10. Censorship, Inspection, and Adversarial Behavior

Relay operators may attempt:

```text
delay
selective dropping
traffic analysis
metadata stripping
false delivery receipts
priority fraud
payload substitution
route capture
```

Defenses include:

```text
signatures
content hashes
multi-route redundancy
receipt comparison
cover traffic where supported
public queue audits
independent relays
courier fallback
```

The system distinguishes:

```text
network failure
policy hold
lawful inspection
unauthorized censorship
unknown loss
```

# 11. Outages and Store-and-Forward

When links fail, relays retain messages according to class, privacy, retention, and capacity.

Store-and-forward must support:

```text
ships carrying archives
physical courier cartridges
opportunistic relay windows
partial file transfer
forward error correction
resumable content-addressed chunks
```

A recovered relay may contain years of undelivered personal, civic, and scientific history.

# 12. Simulation Levels of Detail

## LOD 0 — Active Link

Per-packet or chunk scheduling, visible transmission, interference, power, and queue behavior.

## LOD 1 — Active Corridor

Messages aggregate by class and route while preserving individual durable envelopes.

## LOD 2 — System Background

Delivery is event-scheduled using deterministic bandwidth and outage intervals.

## LOD 3 — Distant Worldline

Only causal frontiers, unresolved transactions, priority backlogs, and durable message identities remain active.

LOD transitions may not:

```text
deliver early
skip inspection
change privacy
lose authority expiry
duplicate messages
erase causal gaps
```

# 13. Persistence and Replay

Checkpoints store:

```text
relay queues
in-flight propagation events
partial chunks
custody receipts
clock calibrations
knowledge frontiers
civic transaction state
```

Replay must reproduce delivery order given the same physical graph, failures, policies, and inputs.

# 14. Observability

Debug traces expose:

```text
message lineage
route decision
queue reason
earliest physical arrival
policy holds
clock conversion
knowledge frontier used by a decision
```

Private payloads remain redacted unless the viewer has explicit debug authority.

# 15. Representative Fixture

The first fixture contains:

```text
one planet
one moon habitat
one transfer vessel
three relays
one intermittent outage
one civic mandate
one distress call
one personal message chain
one market report
```

Acceptance requires:

1. Distress traffic preempts bulk archive traffic.
2. A civic mandate arrives after conditions change and is lawfully adapted.
3. A personal reply references only received information.
4. Clock correction preserves original timestamps.
5. Save/load preserves in-flight propagation and queue order.
6. A failed relay reroutes without duplicating delivery.
7. A knowledge frontier prevents remote-state leakage.

# 16. Kill Criteria

Remove or simplify any subsystem that:

- turns latency into repetitive waiting rather than consequential planning;
- requires global omniscient state;
- allows priority to erase privacy;
- produces nondeterministic civic outcomes without evidence;
- cannot replay delivery and expiry;
- creates more authoring burden than visible player value.

## Final Rule

> **The communication runtime does not synchronize the solar system. It preserves the truth that the solar system is not synchronized.**
