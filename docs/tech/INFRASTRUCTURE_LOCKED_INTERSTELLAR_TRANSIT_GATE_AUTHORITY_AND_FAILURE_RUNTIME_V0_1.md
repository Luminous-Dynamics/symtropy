---
title: Infrastructure-Locked Interstellar Transit, Gate Authority, and Failure Runtime
version: 0.1
status: implementation-spec
scope: horizon-only interstellar transit infrastructure, Atlas Gates, public authority, synchronization, quarantine, failure and uniqueness
owner: engineering/worldline/device-bus/interstellar
related:
  - METRIC_TRIM_SAIL_ANCHOR_CORRIDOR_AND_BRIDGE_RUNTIME_V0_1.md
  - ATLAS_TIME_PROPER_TIME_KNOWLEDGE_TIME_AND_CAUSAL_GRAPH_RUNTIME_V0_1.md
  - ../canon/INTERSTELLAR_CIVILIZATION_RELATIVISTIC_DISTANCE_AND_LOCAL_SOVEREIGNTY_CONTRACT_V0_1.md
  - RELATIVISTIC_NAVIGATION_TIME_DILATION_AND_CAUSAL_COORDINATION_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
  - DEVICE_BUS_RUNTIME_SAFETY.md
supersedes:
  - "Symtropy Design Doc - Infrastructure-Locked Interstellar Transit.md"
---

# Infrastructure-Locked Interstellar Transit, Gate Authority, and Failure Runtime

## Purpose

This document defines the **horizon-only** runtime contract for infrastructure capable of shortening interstellar transit.

It does not assert that Atlas Gates are scientifically available in reality, appropriate for the representative build, or required for Symtropy to succeed.

v2.4 selects a specific fictional mechanism: future-directed Atlas metric engineering. This document remains authoritative for the macro-infrastructure transaction, authority, synchronization, quarantine, and uniqueness rules; the metric runtime owns field tiers, positive route latency, route weather, and chronology protection.

> **A gate may shorten distance. It may not erase cost, causality, jurisdiction, or history.**

# 1. Status and Scope

```text
design maturity: D3
implementation maturity: I0
milestone status: horizon only
Seedworks dependency: none
```

Ordinary spacecraft do not carry casual FTL drives.

The default interstellar experience remains:

```text
sub-light probes
arks
relativistic missions
light-delay communication
causal isolation
```

Gate infrastructure may appear only after those systems are independently coherent.

# 2. Gate Model

A gate is a coupled system:

```text
physical structure
power and thermal plant
navigation and target model
causal synchronization system
worldline and identity validator
traffic and quarantine authority
transit aperture
arrival infrastructure
archive and witness service
```

A gate endpoint cannot function as one decorative portal.

# 3. Required Endpoint Pair

Every committed transit requires compatible origin and destination endpoint state unless the chosen mechanism explicitly supports an unpaired destination.

Minimum destination requirements:

```text
validated location and epoch
receiving aperture or arrival envelope
power and thermal reserve
traffic separation
mass and habitat capacity
quarantine capability
worldline compatibility
local consent or lawful emergency exception
```

Writing a target name is not sufficient.

# 4. Gate State Schema

```rust
struct AtlasGateState {
    gate_id: StableId,
    mechanism_profile: MechanismProfileRef,
    physical_state: GatePhysicalState,
    power_reservation: EnergyReservation,
    thermal_margin: ThermalMargin,
    endpoint_state: EndpointSynchronizationState,
    navigation_solution: GateNavigationSolution,
    authority_state: GateAuthorityState,
    quarantine_state: GateQuarantineState,
    transit_queue: Vec<GateTransitRequest>,
    active_commit: Option<GateTransitCommit>,
    archive_root: ArchiveRoot,
    worldline_binding: WorldlineBinding,
}
```

# 5. Mechanism Profile

The fiction must choose a declared profile.

The profile specifies:

```text
what physical resource is consumed
whether coordinate causality is altered
whether endpoints must be synchronized
maximum mass and aperture
failure envelope
information behavior
worldline interaction
biological and machine hazards
```

The runtime may not quietly combine incompatible mechanisms for convenience.

# 6. Transit Request

A request contains:

```rust
struct GateTransitRequest {
    request_id: StableId,
    origin_gate: StableId,
    destination_endpoint: EndpointRef,
    vehicle_and_payload: PhysicalManifest,
    people_and_identity_roots: Vec<IdentityRoot>,
    departure_window: TimeInterval,
    arrival_capacity_reservation: CapacityReservation,
    purpose: TransitPurpose,
    authority_tokens: Vec<ScopedAuthorityToken>,
    quarantine_profile: QuarantineProfile,
    worldline_target: WorldlineRef,
    consent_and_refusal_records: Vec<ConsentRecord>,
}
```

# 7. Staged Commit

Transit uses a staged transaction:

```text
1. request
2. physical and identity manifest freeze
3. destination capacity reservation
4. endpoint synchronization
5. authority and consent validation
6. quarantine review
7. power and thermal staging
8. final witness window
9. commit
10. departure and arrival reconciliation
11. Chronicle and audit publication
```

Any failure before final commit rolls back reserved state without duplicating assets.

# 8. Authority

Gate authority is decomposed:

```text
physical operator
traffic coordinator
power allocator
quarantine authority
identity and worldline validator
local destination authority
emergency rescue authority
public witness or audit role
```

No single actor should silently control every dimension.

## Emergency Access

Emergency transit may bypass ordinary scheduling only when:

```text
specific lives or catastrophic harm are at stake
arrival capacity exists
quarantine risk is bounded
local destination authority or compact permits it
scope and expiry are recorded
later review is guaranteed
```

# 9. Consent and Migration

Passengers must receive legible information about:

```text
destination worldline
known political conditions
arrival capacity
quarantine
identity and source-chain handling
possible chronology or continuity effects
right to refuse before commit
```

No faction may use minimum life support, debt, incarceration, or employment dependency to create fake voluntary transit.

# 10. Worldline and Uniqueness

Gate transit is a uniqueness-sensitive operation.

The runtime must define whether the mechanism:

```text
moves one physical entity
reconstructs from transmitted state
creates a branch
links existing worldlines
```

The default safe assumption is movement of one unique physical manifest.

No transit may create transferable duplicates of:

```text
people
source chains
vehicles
cargo
money or claims
unique artifacts
```

If the fiction permits reconstruction or branching, it must use the death, reconstitution, source-chain, and worldline protocols explicitly rather than hiding duplication inside an animation.

# 11. Quarantine

Gate transit can move hazards faster than ordinary causal isolation.

Quarantine evaluates:

```text
biological agents
machine code and autonomous systems
xenotechnology
unknown matter or field state
cognitive or signal hazards
worldline contamination
```

Quarantine is not a generic denial flag. It requires evidence, scope, alternatives, appeal, and emergency care.

# 12. Failure Modes

## 12.1 Synchronization Loss

Transit aborts before commit or enters a declared recovery envelope.

## 12.2 Power Collapse

Staged mass and passengers remain at origin unless the commit boundary has been crossed.

## 12.3 Destination Capacity Loss

The request pauses, reroutes, or aborts. It does not force arrival into nonexistent life support.

## 12.4 Manifest Divergence

Any mismatch in people, cargo, identity roots, or vehicle state blocks commit.

## 12.5 Worldline Mismatch

The system must not guess a destination branch.

## 12.6 Quarantine Breach

Arrival enters isolation and public incident handling. Evidence remains preserved.

## 12.7 Authority Capture

A gate operator may monopolize routes, price access, censor migration, or create political dependence. This is a strategic and civic crisis, not only a maintenance fault.

## 12.8 Partial Transit

The chosen mechanism profile must declare whether partial transit is impossible, recoverable, or catastrophic. The runtime cannot invent ambiguity for drama after the fact.

# 13. Gate Politics

Gate control may create:

```text
migration chokepoints
refuge denial
military dominance
archive censorship
trade monopoly
worldline capture
quarantine abuse
regional abandonment
```

Countermeasures include:

```text
public ledgers
multi-party authority
open standards
destination consent
route plurality
rescue exceptions
exit rights
fork and migration rights
maintenance transparency
```

# 14. Device Bus Interface

Example paths:

```text
/dev/sym/gates/{gate_id}/physical
/dev/sym/gates/{gate_id}/power
/dev/sym/gates/{gate_id}/endpoint
/dev/sym/gates/{gate_id}/queue
/dev/sym/gates/{gate_id}/quarantine
/dev/sym/gates/{gate_id}/authority
/dev/sym/gates/{gate_id}/commit
```

Writes require typed, scoped tokens.

No shell command alone overrides consent, mass reconciliation, destination capacity, or quarantine.

# 15. Persistence and Recovery

Gate checkpoints preserve:

```text
all reservations
manifest hashes
identity roots
endpoint state
power and thermal staging
commit phase
worldline binding
quarantine evidence
authority and consent receipts
```

After crash recovery, the system must determine exactly which side of the commit boundary occurred.

There is no “best guess” duplication recovery.

# 16. Player Legibility

Before commit, the player sees:

```text
WHO AND WHAT WILL MOVE
ORIGIN WORLDLINE
DESTINATION WORLDLINE
ARRIVAL CAPACITY
KNOWN DELAY OR CONTINUITY EFFECT
POWER AND THERMAL MARGIN
QUARANTINE STATUS
AUTHORITY AND CONSENT STATUS
ABORT BOUNDARY
```

The interface must separate:

```text
validated
predicted
unknown
contested
```

# 17. Verification Tests

1. Pre-commit abort releases reservations without movement.
2. Post-commit recovery produces one authoritative outcome.
3. People and cargo remain conserved.
4. Destination capacity cannot be oversubscribed silently.
5. Worldline mismatch fails closed.
6. Quarantine evidence remains reviewable.
7. Consent revocation before commit blocks transit.
8. Emergency authority expires.
9. Gate capture changes access without rewriting physical truth.
10. Save/load at every stage preserves commit semantics.

# 18. Representative Fixture

```text
one origin gate
one destination endpoint
one passenger vessel
mixed cargo and private archives
one worldline fork before departure
one quarantine uncertainty
one destination capacity loss
one emergency rescue request
```

The fixture passes when transit either completes once or aborts coherently, with no duplication, false authority, privacy loss, or arrival into nonexistent capacity.

# Hard Invariants

```text
no casual ship-mounted FTL
no gate without physical infrastructure and declared mechanism
no transit without manifest and destination capacity
no worldline guessing
no person, source chain, vehicle, cargo, or claim duplication
no authority monopoly hidden as technical operation
no quarantine without evidence and review
no irreversible commit without legible consent and abort boundary
```
