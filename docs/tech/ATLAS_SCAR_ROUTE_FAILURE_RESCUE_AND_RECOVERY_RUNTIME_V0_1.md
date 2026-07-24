---
title: Atlas Scar, Route Failure, Rescue, and Recovery Runtime
version: 0.1
status: implementation-spec
scope: Atlas route failures, fault states, committed transit recovery, rescue triage, scar persistence, restoration and evidence
owner: engineering/simulation/production/safety
related:
  - ../canon/ATLAS_ROUTE_WARFARE_RESCUE_QUARANTINE_AND_FAILURE_CONTRACT_V0_1.md
  - METRIC_TRIM_SAIL_ANCHOR_CORRIDOR_AND_BRIDGE_RUNTIME_V0_1.md
  - ATLAS_TIME_PROPER_TIME_KNOWLEDGE_TIME_AND_CAUSAL_GRAPH_RUNTIME_V0_1.md
  - INFRASTRUCTURE_LOCKED_INTERSTELLAR_TRANSIT_GATE_AUTHORITY_AND_FAILURE_RUNTIME_V0_1.md
---

# Atlas Scar, Route Failure, Rescue, and Recovery Runtime

## Purpose

This runtime defines legible route failure states and prevents Atlas incidents from becoming either random instant death or harmless visual effects.

# 1. Fault Domains

```rust
enum AtlasFaultDomain {
    Clock,
    Navigation,
    FieldLattice,
    Power,
    Thermal,
    Structural,
    Software,
    Authority,
    Quarantine,
    Manifest,
    Destination,
    Worldline,
    Environmental,
    Adversarial,
}
```

One incident may involve several domains with causal ordering.

# 2. Severity Classes

## F0 — Advisory

Operation remains within certified envelope but requires review.

## F1 — Degraded

Capacity or confidence reduced. Scheduled traffic may continue under limits.

## F2 — Unsafe to Open

No new transit commitment. Maintenance and rescue preparation begin.

## F3 — Active Transit Emergency

A committed manifest is endangered or destination reconciliation is uncertain.

## F4 — Endpoint Emergency

Field, power, thermal, or structural state threatens the endpoint and nearby population.

## F5 — Persistent Scar Event

Failure produces long-lived physical, ecological, causal, or institutional consequences.

# 3. Failure Graph

Faults propagate through explicit dependencies.

Example:

```text
coolant contamination
→ radiator efficiency loss
→ thermal reserve depletion
→ field-lattice temperature rise
→ coherence instability
→ route aperture reduction
→ committed vessel recovery decision
```

The game should expose enough of the chain for professional diagnosis and later accountability.

# 4. Active Transit Recovery States

```rust
enum TransitRecoveryState {
    StableInTransit,
    HoldForDestination,
    ReturnToOriginPrepared,
    DivertToCertifiedAlternate,
    EmergeInDeclaredOrdinarySpaceEnvelope,
    RemoteStabilization,
    RescueInterceptRequired,
    ArrivalDisputed,
    Irrecoverable,
}
```

Options exist only if prepared by the route solution and infrastructure.

# 5. Rescue Capacity

```rust
struct AtlasRescueCapacity {
    available_mass: Mass,
    available_energy: Energy,
    medical_beds: u32,
    quarantine_units: u32,
    rescue_craft: Vec<StableId>,
    trained_crews: Vec<CrewId>,
    safe_destinations: Vec<EndpointId>,
    response_deadline: AtlasInstant,
}
```

Rescue plans must reconcile capacity with victims rather than assume unlimited emergency throughput.

# 6. Triage

Triage inputs may include:

- number of lives;
- recoverability;
- immediate medical risk;
- children and dependents;
- critical repair roles;
- promised evacuation order;
- quarantine risk;
- rescue-team exposure;
- effect on other stranded groups;
- nonhuman requirements;
- evidence uncertainty.

Policies are declared and reviewable. IRIS may calculate consequences but does not choose whose life matters.

# 7. Endpoint Isolation

Emergency isolation may:

- shut field zones;
- sever power;
- jettison thermal reservoirs;
- move population shelters;
- close communications;
- preserve only information lanes;
- physically separate damaged lattice sectors.

Isolation actions have costs and may create later scars.

# 8. Scar State

```rust
struct AtlasScarState {
    scar_id: StableId,
    origin_incident: EventId,
    location: SpatialRegion,
    created_at: AtlasInstant,
    physical_residuals: Vec<PhysicalResidual>,
    ecological_residuals: Vec<EcologicalResidual>,
    temporal_uncertainty: Duration,
    navigation_exclusion: SpatialRegion,
    affected_populations: Vec<PopulationRef>,
    legal_status: ScarLegalStatus,
    memory_refs: Vec<EvidenceRef>,
    remediation_projects: Vec<ProjectId>,
    monitoring_requirements: Vec<MonitoringRule>,
}
```

# 9. Scar Evolution

A scar can:

- decay;
- stabilize;
- migrate;
- interact with new mass distributions;
- affect communications;
- become inhabited;
- be exploited economically;
- be misrepresented politically;
- trigger delayed ecological effects;
- become sacred or memorialized;
- reveal alien structure.

Scar evolution continues during player absence.

# 10. Evidence Preservation

Incidents preserve:

- raw sensor streams;
- clock states;
- route solution version;
- operator actions;
- authority tokens;
- worker refusals;
- maintenance history;
- software hashes;
- manifest revisions;
- quarantine evidence;
- communications;
- crash-recovery decisions.

Private medical and identity data remain access-controlled.

# 11. Investigation

Investigations distinguish:

- proximate technical cause;
- maintenance cause;
- organizational cause;
- political pressure;
- design limitation;
- sabotage;
- unavoidable uncertainty;
- response quality;
- preventable harm.

The system must support multiple legitimate interpretations without losing measured facts.

# 12. Restoration

Restoration stages:

1. secure and evacuate;
2. stabilize residual fields;
3. account for people and manifests;
4. establish exclusion and monitoring;
5. investigate;
6. repair power and cooling;
7. rebuild or replace lattice;
8. resurvey geometry;
9. renegotiate authority and treaties;
10. run low-energy tests;
11. certify limited operation;
12. reopen or permanently retire.

A repaired endpoint may never regain its former route class.

# 13. Retirement

Permanent retirement requires:

- committed-transit reconciliation;
- hazard stabilization;
- worker and resident transition;
- archive custody;
- destination notification;
- route graph removal;
- memorial or ruin policy;
- long-term monitoring;
- treatment of dependent communities.

# 14. Gameplay Loops

Players may participate as:

- field engineer;
- rescue pilot;
- dispatcher;
- medic;
- clock auditor;
- investigator;
- worker organizer;
- route advocate;
- military planner;
- scar ecologist;
- survivor;
- returning historical witness.

# 15. Determinism and Replay

Every incident must replay from:

- preincident snapshot;
- event inputs;
- deterministic seeds;
- player and NPC actions;
- external route state.

The same replay must not randomly choose different victims or duplicate transit assets.

# 16. Acceptance Tests

1. A cooling fault propagates through a traceable failure graph.
2. A route refuses new traffic before active failure.
3. A committed transit enters exactly one recovery state.
4. Rescue capacity limits are conserved.
5. Triage excludes hidden player priority.
6. Evidence survives host migration and crash.
7. Private data remain restricted during public investigation.
8. A scar evolves during a five-year absence.
9. Restoration requires both engineering and authority gates.
10. Retirement preserves dependent-community consequences.

# Production Maxim

> **A failed route is not an erased fast-travel point. It is an event that changes the people, geometry, and politics around it.**
