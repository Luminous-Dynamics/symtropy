# Symtropy: Coupled Physics Contract for Mobile Logistics and Home Automation

**Status:** Research design contract v0.1; simulation integration only  
**Date:** 2026-11-15

## 1. Thesis

The logistics carrier, delivery aircraft, ground robot, home receiver, tote elevator, storage racks, people, animals, and building are one coupled physical environment. Symtropy should provide a deterministic, replayable scenario environment and common physical-world interface. It must not be represented as a universal structural, aerodynamic, thermal or electrical solver.

The initial programme is carrier-to-home material flow:
- EV carrier parked in a defined operating bay;
- drone dock and mechanical retention;
- package transfer from aircraft or truck;
- manipulator or powered transfer;
- wheeled robot navigation;
- secure home receiving vestibule;
- tote/container movement;
- vertical lift, conveyor, carousel and storage rack;
- ambient, chilled and frozen storage with separate thermal models;
- people, animals, doors, egress routes and obstacles.

## 2. Existing capabilities and honest boundaries

The current public Symtropy design documents describe:
- N-dimensional rigid-body mathematics and collision detection;
- rigid-body stepping, joints, continuous collision detection, raycasting and deterministic replay;
- robotics bridges and platform abstractions for vehicles, quadrotors, manipulators, quadrupeds and other platforms;
- a wider vehicle/mobility design that treats vehicles as mobile infrastructure and civic assets.

The public engine documentation explicitly scopes soft-body mechanics out of the core. It is not an FEA package, CFD solver, battery electrochemistry model, refrigeration model, building-code checker or flight-certification tool.

Use Symtropy for system-level orchestration, replay, rigid-body/contact interactions where suitable, scene state, timing, event traces and deterministic replay. Delegate specialist physics to validated external solvers and preserve their versions and evidence.

## 3. Model decomposition

| Subsystem | Physical model | Evidence boundary |
|---|---|---|
| EV carrier | Multi-axle vehicle dynamics, payload and axle loads, brakes, steering, stability | Existing Symthaea bicycle model is a baseline only; build a loaded multi-axle plant |
| Drone | Aircraft-specific flight dynamics, propwash/downwash, docking approach, payload and energy | Existing multirotor examples do not qualify cargo capability |
| Dock | Contact, retention, latch and release state, charging interlocks | Physical latch and electrical HIL tests required |
| Manipulator | Kinematics, contact, forces, grasp stability, reach and workspace | Reuse Symthaea manipulator supervisor; external robot dynamics where needed |
| Ground robot | Localization, nonholonomic/holonomic motion, slope, traction, wheel contact | Wheel slip and real load tests required |
| Tote/conveyor/lift | Rigid body transport, jams, acceleration, collision, load limits | Mechanical supplier and physical tests |
| Storage | Ambient/chilled/frozen thermal response, doors, humidity, power loss | Validated thermal model and calibrated sensors |
| Building | Structural loads, vibration, fire compartments, egress and access | FEA, building engineering, fire engineering and local approvals |
| Humans/animals | Occupancy, reachable zones, protected right-of-way, interactions | Human factors, accessibility and animal welfare validation |

## 4. Reference scene and state

The first reference scene is deliberately small:
- one stationary, immobilized EV carrier;
- one parked aircraft with a compatible dock;
- one package with a frozen manifest;
- one fixed receiving station;
- one tote transfer;
- one wheeled mobile base or fixed conveyor;
- one storage destination;
- one human occupant represented by a conservative occupancy envelope.

Every body or object uses a stable ID. Every physical value has a unit. Every uncertainty has a source and range. The scene manifest binds model versions, initial conditions, seed, fixed time step, solver settings and asset configuration digests.

Do not place a learned policy or cognition score in the physics plant. It may propose a setpoint; the local controller and safety supervisor own actuation.

## 5. Event/state machine

Proposed lifecycle:
1. Receiver advertises ready, unavailable or quarantine.
2. Carrier is parked, immobilized and checked for dock/handling readiness.
3. Aircraft or courier is authenticated and assigned a compatible transfer point.
4. Package identity, manifest and outer condition are checked.
5. Transfer is authorized and physically performed.
6. Package enters a reusable tote or safe quarantine.
7. Tote is moved through approved corridors to a valid storage compartment.
8. Storage confirms identity, location, environment and seal state.
9. Custody receipt is committed with the evidence digest.
10. Any failure moves the package and relevant asset into a defined hold, reject, recovery or human-assistance state.

A transfer acknowledgement is not proof that the package physically arrived in its destination compartment. Sensor evidence and the compartment's independent state must agree.

## 6. Simulation campaigns

### Campaign A — dock and handoff
- small positional offsets;
- latch sensor disagreement;
- wet or contaminated contact surfaces;
- charging request before latch confirmation;
- unexpected package mass/dimensions;
- power loss during transfer;
- unexpected human presence.

### Campaign B — material flow
- conveyor jam;
- tote skew;
- lift overload;
- shelf obstruction;
- mislabelled tote;
- dropped or tipped container;
- route blocked by a person or pet;
- elevator unavailable;
- conflicting robots requesting the same aisle.

### Campaign C — home environment
- power outage and backup transition;
- refrigerator/freezer temperature excursion;
- door left open;
- sensor failure and stale data;
- fire/alarm condition;
- egress route obstruction;
- network partition;
- unauthorized entry attempt.

### Campaign D — system-level disruptions
- carrier delayed;
- drone grounded by weather;
- alternate carrier unavailable;
- NestPort full;
- damaged package;
- conflicting demand and priority;
- missing custody acknowledgement;
- stale asset or maintenance state.

Each campaign includes negative controls and identifies what is simulated, analytically checked, physically tested and externally qualified.

## 7. Novel design opportunity: material-flow digital twin

Treat home logistics as a flow of identified containers through constrained physical transitions. Each tote has physical dimensions, mass and centre-of-gravity bounds; contents manifest or opaque shipment reference; permitted temperature/humidity envelope; allowed transfer mechanisms; seal/custody state; source and destination; storage class; hazard/contamination separation; cleaning and return state.

The digital twin can reject impossible plans before motion: a tote does not fit the lift, storage is full, mass exceeds the manipulator's current envelope, the route crosses an occupied zone, or the receiving bay lacks sufficient environmental capacity.

This is more valuable than visualizing a robot moving a box. It provides a machine-checkable material-flow plan.

## 8. Proposed interfaces

Illustrative Rust contract boundaries:
- SceneManifest
- RigidBodyBinding
- AssetConfigurationDigest
- TransferTask
- ToteState
- DockInterlockState
- StorageCompartmentState
- OccupancyConstraint
- PhysicalObservation
- ScenarioEvent
- ReplayReceipt

The interface must reject unit ambiguity, non-finite values, invalid time, stale measurements and unresolved asset configuration. Use typed units where possible. The planner receives a validated scene snapshot and returns a proposal; Symtropy must not turn a proposal into actuator authority.

## 9. Benchmarks

Measure:
- collision/contact invariant violations;
- deterministic replay agreement;
- maximum scene/event divergence;
- transfer completion and package damage;
- dock retention and release state;
- tote transport stability;
- load/axle/torque/workspace margin;
- path completion and deadlock recovery;
- blocked egress time (must be zero in qualified runs);
- human/animal exposure and near misses;
- temperature excursion duration and magnitude;
- energy per successful transfer;
- compute time and memory per simulated second.

Initial research targets:
- deterministic replay equality for identical frozen inputs and declared platform;
- zero accepted impossible loads or storage assignments in the scenario suite;
- zero accepted hard safety-constraint violations;
- 100% explicit incomplete/fail classification for missing or contradictory evidence;
- benchmark performance reported against the same model and scenario with fixed solver budgets.

Do not convert a passing simulation into a claim of physical safety. Require HIL for dock/latch/charger/transfer systems and independent engineering validation for structural, thermal, electrical and aviation-related claims.

## 10. Integration roles

- **Symthaea:** proposal, perception, learned skill and cognitive planning; all commands are bounded by local control and independent safety.
- **Mycelix:** asset/service identity, task authorization, custody events, shared resource governance and maintenance provenance; not high-rate control.
- **Sol-Atlas:** site context, geospatial constraints and evidence freshness; not a certified terrain or cadastral authority by default.
- **AeroWeft:** aircraft and charging configuration envelopes, mission energy and dock geometry; all tied to a frozen aircraft configuration.
- **NestPort/HomeOps:** receiver state, storage compartments, package and tote events.

## 11. Decision

Proceed with one stationary carrier-to-receiver reference scene and one material-flow scenario before adding multiple drones, moving-carrier operation, humanoid assistance, automated unboxing or heavy-lift aircraft. The differentiator is a replayable, evidence-bound digital twin of the entire logistics handoff, not an unsupported claim of universal physical simulation.
