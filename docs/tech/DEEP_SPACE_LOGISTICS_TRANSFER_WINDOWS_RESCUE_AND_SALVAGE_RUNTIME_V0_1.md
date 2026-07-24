---
title: Deep-Space Logistics, Transfer Windows, Rescue, and Salvage Runtime
version: 0.1
status: implementation-spec
scope: interplanetary routes, transfer windows, manifests, convoy scheduling, consumables, delay, rescue, towing, salvage, derelicts, casualty evacuation, and cargo conservation
owner: engineering/simulation/logistics/spaceflight
implements:
  - ../canon/INTERPLANETARY_CIVILIZATION_LATENCY_AND_DISTRIBUTED_SOVEREIGNTY_CONTRACT_V0_1.md
  - ../canon/ORBITAL_PLANETARY_INTERFACE_AND_SPACEPORT_GOVERNANCE_CONTRACT_V0_1.md
authority_boundary: owns authoritative interplanetary cargo, route, transit, rescue, and salvage state; does not own local market pricing, fleet political legitimacy, or detailed vehicle physics inside active scenes
related:
  - VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - PLANETARY_INFRASTRUCTURE_NETWORKS_AND_CORRIDOR_RUNTIME_V0_1.md
  - ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Deep-Space Logistics, Transfer Windows, Rescue, and Salvage Runtime

## Purpose

This runtime makes interplanetary movement a conserved, scheduled, hazardous process.

A cargo shipment is not a timer attached to an icon.

It is:

```text
mass
volume
custody
trajectory
energy
crew or automation
life support
maintenance
risk
arrival capacity
```

## Core Invariant

> **Nothing crosses interplanetary distance unless a physical carrier, feasible route, departure event, and destination capacity exist.**

# 1. Route Model

```rust
struct InterplanetaryRoute {
    route_id: RouteId,
    origin: NodeId,
    destination: NodeId,
    departure_window: TimeInterval,
    arrival_window: TimeInterval,
    trajectory_class: TrajectoryClass,
    delta_v: FixedPoint,
    transit_duration: DurationRange,
    navigation_uncertainty: Uncertainty,
    communications_profile: CommProfileId,
    rescue_coverage: RescueCoverage,
    hazard_profile: HazardProfile,
}
```

Trajectory classes may include:

```text
minimum-energy transfer
fast transfer
cycler intercept
ballistic capture
cargo spiral
high-thrust direct
surface-orbit shuttle leg
alien or field-assisted route
```

The simulation need not solve full high-fidelity orbital mechanics for every distant shipment, but every route must preserve launch window, time, mass, energy, interception, and arrival constraints.

# 2. Shipment Envelope

```rust
struct Shipment {
    shipment_id: ShipmentId,
    contract_id: Option<ContractId>,
    carrier: CarrierId,
    origin: NodeId,
    destination: NodeId,
    manifest: ManifestId,
    custody: CustodyChain,
    departure_window: TimeInterval,
    required_arrival: Option<TimeInterval>,
    state: ShipmentState,
    risk_allocation: RiskAllocation,
    insurance_or_mutual_aid: Option<CoverageId>,
}
```

Shipment states:

```text
Planned
AwaitingCargo
AwaitingCarrier
AwaitingWindow
Loading
InspectionHold
Ready
Departed
InTransit
CourseCorrection
Distress
Diverted
Arrived
Unloading
Delivered
PartialDelivery
Lost
SalvagePending
```

# 3. Manifest and Cargo

A manifest contains:

```text
unique assets
fungible material batches
passengers
animals or ecological payloads
medical dependencies
hazard classes
temperature and pressure requirements
ownership and custody
inspection seals
quarantine status
```

Cargo conservation is checked at:

```text
loading
departure
transit transfer
arrival
unloading
salvage
```

Mass changes require explicit causes such as consumption, venting, reaction, waste, damage, jettison, or theft.

# 4. Carrier Readiness

A carrier must satisfy:

```text
propulsion readiness
energy or propellant
navigation
communications
thermal margin
structural condition
crew and automation
life support
spares
cargo interfaces
destination compatibility
```

Readiness is not binary. Degraded carriers may depart under emergency or political pressure with visible risk.

# 5. Transfer Windows

Windows are first-class planning objects.

They influence:

```text
cargo stockpiling
market timing
migration
military planning
rescue feasibility
maintenance deadlines
political negotiations
```

Missing a window may cause:

```text
weeks or months of delay
spoilage
contract default
crew reassignment
settlement shortage
political crisis
```

The game should offer alternate routes, not only failure.

# 6. Transit Consumption

During transit, carriers consume:

```text
energy
propellant where applicable
food
water
oxygen or atmospheric processing capacity
filters
coolant
maintenance parts
medical supplies
crew attention
```

Transit also produces:

```text
waste
heat
wear
radiation dose
fatigue
social conflict
information delay
```

# 7. Distress and Rescue

```rust
struct DistressCase {
    case_id: DistressId,
    vessel: CarrierId,
    last_known_state: VesselSnapshot,
    event_time: SystemEpoch,
    report_receive_time: SystemEpoch,
    position_uncertainty: RegionEstimate,
    failure: FailureClass,
    people_at_risk: PopulationRef,
    consumable_horizon: DurationRange,
    possible_responders: Vec<ResponderOption>,
    legal_status: DistressLegalState,
}
```

Rescue planning accounts for:

```text
message delay
current trajectory
intercept windows
responder readiness
consumables
medical capacity
towing capability
political borders
quarantine
risk to rescuers
```

## Rescue Priority

Priority considers:

```text
immediacy of death
number and vulnerability of people
feasibility
responder risk
availability of alternatives
prior commitments
```

It must not reduce people to economic value or faction status.

## Rescue Outcomes

```text
remote repair guidance
supply package intercept
crew transfer
medical evacuation
tow or trajectory correction
shelter in place
derelict abandonment
partial rescue
failed rescue with evidence
```

# 8. Rescue Obligation

Rescue compacts define:

```text
who must respond
minimum readiness
cost sharing
priority channels
quarantine handling
later compensation
review of refusal or delay
```

A duty to attempt rescue is not a promise of success.

# 9. Salvage and Derelicts

A derelict may contain:

```text
living survivors
bodies
private archives
public records
hazardous cargo
unclaimed material
disputed property
alien or unknown life
military secrets
source-chain cores
```

Salvage begins with classification:

```text
active distress
abandoned but owned
lost with recoverable claim
public hazard
historical site
unclaimed derelict
war prize claim
alien or agency-uncertain object
```

## Salvage Rights

Separate:

```text
right to secure hazard
right to rescue
right to inspect
right to tow
right to recover material
right to access records
right to claim ownership
```

Rescue never grants automatic ownership.

## Evidence Preservation

Salvage operations preserve:

```text
scene state
custody
bodies and identity
navigation logs
cargo manifest
source-chain records
cause-of-loss evidence
```

# 10. Convoys and Fleet Logistics

Convoys may share:

```text
navigation
communications
repair capacity
rescue coverage
defense
fuel or energy depots
cargo balancing
```

Aggregation preserves individual vessel custody, people, and failure state.

A convoy can split due to:

```text
trajectory divergence
political dispute
quarantine
mechanical failure
attack
rescue diversion
```

# 11. Depots and Cyclers

System infrastructure may include:

```text
propellant depots
water depots
repair stations
cycler habitats
communication relays
medical refuges
archive beacons
navigation markers
```

Depots are physical stocks with maintenance, contamination, custody, and access rules. They are not infinite route buffs.

# 12. Hazard Model

Hazards include:

```text
radiation event
micrometeoroids
collision
debris
thermal failure
navigation error
communications loss
crew illness
sabotage
piracy or seizure
quarantine breach
alien interaction
```

Risk is expressed as causal possibilities and uncertainty, not one opaque percentage.

# 13. LOD and Event Scheduling

## LOD 0 — Active Encounter

Detailed vehicle physics, crew stations, tools, local cargo, boarding, towing, and repair.

## LOD 1 — Active Transit

Scheduled burns, consumables, maintenance, messages, crew state, and discrete incidents.

## LOD 2 — Background Route

Deterministic event scheduling with bounded uncertainty and conserved cargo.

## LOD 3 — Distant System

Contract milestones, custody, trajectory phase, consumable horizon, critical events, and message frontiers.

LOD changes may not:

```text
teleport cargo
skip people
repair damage without cause
create propellant
erase distress
resolve ownership silently
```

# 14. Persistence

Save state includes:

```text
route and trajectory phase
carrier condition
consumables
crew and passenger references
manifest and custody
messages in flight
active rescue cases
salvage evidence
contract state
```

Worldline forks preserve shipment ancestry and prevent transferable asset duplication.

# 15. Representative Fixture

Fixture:

```text
planetary spaceport
moon habitat
cycler
cargo tug
passenger vessel
rescue cutter
three transfer windows
medical cargo
industrial cargo
one disputed derelict
```

Scenario:

1. Cargo is assembled and inspected.
2. One shipment misses a window.
3. A passenger vessel reports thermal failure after communication delay.
4. The rescue cutter lacks enough medical capacity for everyone.
5. The cargo tug can divert but will default on a settlement-critical contract.
6. The player coordinates rescue, supplies, and later accountability.
7. A derelict contains private archives and disputed salvage rights.

Acceptance requires:

- all mass and custody reconcile;
- rescue feasibility changes with message delay and trajectory;
- people are not valued by cargo price or faction;
- diversion creates real shortage and contract consequences;
- salvage preserves evidence and private-data boundaries;
- save/load reproduces transit and rescue state;
- background LOD cannot resolve distress invisibly.

# 16. Anti-Exploit Rules

Reject:

```text
cargo duplication through rerouting or forks
insurance payout plus intact hidden asset
instant recall after departure
rescue teleportation
salvage ownership from scanning alone
infinite depot supply
zero-cost course correction
passengers collapsed into cargo mass only
```

## Final Rule

> **Distance is not empty space. It is the chain of material promises that must remain true until someone arrives.**
