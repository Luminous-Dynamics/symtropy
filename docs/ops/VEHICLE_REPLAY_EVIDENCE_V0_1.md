# Scenario-bound physics replay evidence v0.1

**Status:** replay receipt API implemented; no test execution claimed in this session  
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
- exact vector/scalar floating-point bit patterns, including angular-velocity matrix values.

Changing even one command, its order or timestep changes the tape digest.

## Tick snapshots

Each frame is preflight-checked before the first world mutation:
- timestep finite and strictly positive;
- referenced body handles exist in the initial world;
- all force, impulse, velocity and angular-velocity values finite;
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

The full tape is validated before the first frame is applied. Invalid context, empty tape, over-budget tape, invalid timestep, non-finite command, and missing-body references are rejected. A non-finite state discovered after a physics step produces no successful receipt.

A failed post-step execution may already have mutated the in-memory world through that failed step; callers should discard that world instance. The API does not promise transactional rollback of arbitrary physics state.

## Tests

Seven tests are present in `crates/core/symtropy-physics/tests/replay_harness.rs`:
1. Existing bitwise record/replay per tick.
2. Scenario-bound receipt verified in a separate world.
3. Changed tape or context cannot reuse an old receipt.
4. Invalid timestep rejected before world mutation.
5. Missing body rejected before world mutation.
6. Malformed receipt digest rejected.
7. Non-finite initial world rejected before stepping.

These tests are authored but have not been compiled or executed in this session.

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
