# Scenario-bound physics replay evidence v0.1

**Status:** replay receipt API plus kinematic state tape command authored; no test execution claimed in this session  
**Crate:** `symtropy-physics`  
**Code:** `replay::execute_with_evidence`, `replay::verify_replay_evidence`  
**Composition target:** Symthaea SAM #10043 / #10234 / #10237 and Symtropy #3387

## Purpose

The physics crate already provides:
- an ordered `ReplayTape` with timestep and command frames;
- a typed `WorldCommand` vocabulary;
- bitwise `WorldSnapshot` capture for body kinematics and collision events;
- a replay harness that compares every tick.

This tranche adds an evidence envelope around those existing primitives. It does not add a vehicle-specific simulator or scenario language. The envelope binds a caller-declared scenario, engine/build, initial-world profile, environment and execution protocol to:
- exact replay input tape digest;
- initial bitwise world snapshot digest;
- every post-step world snapshot digest;
- final world snapshot digest;
- canonical receipt digest.

## Context profile

`ReplayEvidenceContext` requires:
- stable scenario ID and scenario artifact digest;
- engine artifact/build digest;
- initial world profile digest (body configuration, mass/inertia/collider parameters, gravity and other initial conditions not present in kinematic snapshots);
- environment profile digest (actors, external forces, weather/surface and other external conditions);
- execution protocol digest (timestep/horizon/termination and configuration).

All digests are checked as 64-hex strings. These are structural inputs, not authentication. The caller owns truthful build, model and scenario provenance.

## Tape identity

The tape digest includes:
- physics dimension;
- frame count and exact timestep bit patterns;
- frame order and command order;
- command variant tags and body handles;
- exact vector/scalar floating-point bit patterns, including angular-velocity matrix values and kinematic translation/linear-velocity targets.

Changing even one command, its order or timestep changes the tape digest.

## Tick snapshots

Each frame is preflight-checked before the first world mutation:
- timestep finite and strictly positive;
- referenced body handles exist in the initial world;
- all force, impulse, velocity, angular-velocity and kinematic target values finite;
- each `SetKinematicState` target references an existing `BodyType::Kinematic` body;
- frame count within the declared hard cap.

After each step, a bitwise world snapshot is captured. Initial and post-step snapshots are rejected if any body transform, velocity, angular velocity or collision-event numeric field is non-finite. The per-tick digests bind the exact recorded trajectory of the current `PhysicsWorld` implementation.

The current `WorldSnapshot` captures body type, pose, linear/angular velocity, sleeping state and the last-step collision events. It does not itself capture body shape, mass, inertia, gravity, all solver configuration, or external environment state. Those must be bound by the supplied initial-world/environment/execution profile digests.

## Fresh-world verification

`verify_replay_evidence` re-executes the same tape against a separately constructed world and compares:
- exact context;
- input tape digest;
- initial world snapshot;
- every tick snapshot;
- final snapshot;
- full receipt digest.

A successful comparison means bitwise equality for the named build/world/profile and this replay run. It does not establish cross-platform determinism unless hardware/compiler/feature constraints are independently held constant. The test harness constructs two separate worlds, executes the tape on the first and verifies the second against the receipt.

## Errors and fail-closed behaviour

The full tape is validated before the first frame is applied. Invalid context, empty tape, over-budget tape, invalid timestep, non-finite command, missing-body references, and kinematic-state commands directed at non-kinematic bodies are rejected. A non-finite state discovered after a physics step produces no successful receipt.

A failed post-step execution may already have mutated the in-memory world through that failed step; callers should discard that world instance. The API does not promise transactional rollback of arbitrary physics state.

## Kinematic actor-state replay

The replay command vocabulary now includes `SetKinematicState`, which binds a world-space translation and linear velocity for one kinematic body at a tick boundary. This is intentionally not a general dynamics command:
- it rejects Static and Dynamic bodies;
- it rejects non-finite translation/velocity before the first tape mutation;
- it applies through `PhysicsWorld::body_mut`, which invalidates the cached static/kinematic broadphase before the next step;
- it is included in the command-tape digest and per-tick snapshots;
- the physics integrator still does not advance kinematic bodies; the scenario adapter supplies each kinematic state.

A road actor adapter must separately bind stable scenario actor IDs, route/path profile, actor policy, allowed observation cues, reaction delay and actor transition receipts. A seed or body handle alone is not a durable actor identity. Curved paths, yaw updates and unsupported route semantics are outside this translation-only command and must not be silently inferred.

## Tests

Eleven tests are present in `crates/core/symtropy-physics/tests/replay_harness.rs`:
1. Existing bitwise record/replay per tick.
2. Scenario-bound receipt verified in a separate world.
3. Changed tape or context cannot reuse an old receipt.
4. Invalid timestep rejected before world mutation.
5. Missing body rejected before world mutation.
6. Malformed receipt digest rejected.
7. Non-finite initial world rejected before stepping.
8. Kinematic translation/linear velocity command replays bitwise in a fresh world.
9. A Dynamic body cannot receive a kinematic-state command; rejection occurs before mutation.
10. Non-finite kinematic targets reject before mutation.
11. Changing a kinematic target invalidates the old replay receipt.

These tests are authored but have not been compiled or executed in this session. CI now has an explicit `cargo test -p symtropy-physics --test replay_harness` step because the existing physics `--lib` test step does not execute integration-test targets.

## Composition with SAM

Symtropy remains the physics/execution owner. Symthaea owns scenario search and decision interpretation.

For each SAM scenario/action:
1. Symthaea supplies scenario ID/artifact digest, candidate action and objective.
2. Symtropy constructs a world and freezes initial-world, environment and execution profile digests.
3. Symtropy executes a tape and returns this replay receipt plus domain-specific observables (minimum separation, contact, signal/zone/rule interactions, actor state, etc.).
4. Symthaea consumes the exact receipt in scenario-minimisation and counterfactual-decision reports.
5. Fresh-world replay validates that the result is reproducible under the same declared context.

This module does **not** yet generate pedestrian/cyclist traffic actors, road semantics, traffic signals, sign rules, or intersection conflict observables. It is a generic physics replay evidence layer that those scenario adapters can compose.

## Claim ceiling

A valid receipt establishes only that the named physics build replayed the exact command tape from the declared initial world/profile and emitted the retained bitwise snapshots. It does not prove physical realism, scenario plausibility, actuator fidelity, legal rule correctness, safe driving or SAE Level 4/5 capability.
