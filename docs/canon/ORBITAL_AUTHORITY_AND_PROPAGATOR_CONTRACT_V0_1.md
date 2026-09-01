# Orbital Authority and Propagator Contract v0.1

Status: design freeze candidate

## Purpose

Define how Symtropy represents authoritative spacecraft, debris, stations, asteroids, and other interbody objects across very different orbital simulation fidelities without teleporting state, duplicating ownership, or making the local rigid-body engine responsible for whole-system propagation.

This contract extends the Stellar Causal Substrate v0.1 and the continuation/evidence contracts. It owns no specific numerical solver.

## Core rule

A physical object has one authoritative causal identity.

Propagators and local physics are representations of that identity.

Changing propagator is a representation transfer, not a new object and not an arrival event.

## Ownership boundary

Recommended future domain crate:

`crates/domains/symtropy-orbit`

The crate should remain CPU-first and Bevy-independent.

It may depend on dependency-light math and simulation-contract crates, but local presentation/rigid-body hydration should occur through a bridge.

### Orbital authority owns

For each authoritative orbital/interbody object:

- stable object identity;
- current canonical `SimInstant`;
- canonical reference-frame identity;
- translational state required by the active representation;
- mass/inertial resource identity relevant to trajectory evolution;
- active maneuver/control schedule identity;
- current propagation representation identity;
- body/system relationship required by that representation;
- continuation-significant solver/frontier/event state;
- pending encounter/refinement commitments;
- authoritative state/continuation digest;
- ancestry of accepted representation-transfer receipts.

### Orbital authority does not own

- local rigid-body collision/contact internals after a qualified local handoff;
- visual interpolation;
- camera-relative floating origin;
- network packet queues;
- remote observer knowledge;
- planet terrain/ecology/city state;
- gameplay UI predictions;
- deterministic ephemeris models unless those models are promoted into mutable authority state.

## Representation families

v0.1 should recognize representation identity without assuming one universal fidelity ladder.

Candidate families:

### Analytic two-body

Use when one dominant gravitational source and frozen-model assumptions satisfy the declared error budget.

Typical state:

- epoch;
- central-body/frame identity;
- position/velocity or canonical orbital elements;
- gravitational-parameter/model identity;
- scheduled deterministic maneuvers.

This representation is an approximation contract unless a specific solver establishes a stronger claim.

### Patched-conic / influence-region

Use for efficient mission-scale transfers when qualified.

Sphere/influence transitions are representation changes. Crossing a boundary does not teleport the object or reset history.

Boundary-selection policy and body ephemeris identity are causal inputs.

### Numerical N-body

Use when third-body effects, resonances, encounter prediction, or long-horizon accuracy require them.

Recommended first high-fidelity family: a deterministic, fixed-policy symplectic method appropriate to the problem class, with explicit step/config identity.

Do not claim bitwise physical truth merely because the algorithm is deterministic. Qualification must state error/invariant budgets.

### Encounter / close-operations dynamics

Use around docking, collision, landing approach, debris encounters, interception, or other situations where local rigid-body/contact physics matters.

This is where existing `symtropy-physics` should be reused.

The orbital authority provides qualified boundary state to the local physics representation; the return transfer reconciles translational/angular state, mass/resources, damage and any other declared conserved quantities.

### Landed / attached

When an object becomes owned by a planetary/local authority, that is an authority transfer, not merely another orbit representation.

Arrival/capture/landing must have an explicit receipt.

## Frame semantics

Every orbital state is meaningless without a reference frame.

Consume the canonical frame graph from the reference-frame program.

Never infer transforms from frame-name strings.

At minimum distinguish:

- body-fixed;
- body-centered inertial;
- system barycentric/inertial;
- object-local;
- local encounter frames.

A propagated state must bind the transform/ephemeris authority or deterministic forcing identity needed to interpret it.

## Time semantics

All authoritative orbital evolution uses `SimInstant`.

Wall-clock time may pace presentation but never advances orbit truth.

A local fixed-step physics clock must map to the shared simulation timeline through an explicit timebase contract.

For long-duration catch-up, the owner may use larger analytical/numerical steps only under a versioned propagation policy.

## Maneuver semantics

A maneuver is a causal input, not a direct overwrite of destination state.

Recommended maneuver record contains:

- stable maneuver identity;
- object identity;
- scheduled/trigger `SimInstant`;
- reference frame;
- maneuver model/type;
- commanded delta-v/thrust/control profile or typed input digest;
- propulsion/resource policy identity;
- causal parents;
- authorization/control identity when gameplay requires it.

The authority commits the resulting state and resource change.

Deleting or changing an accepted maneuver requires an explicit supersession/cancellation event.

## Encounter scheduling and causal backpressure

The orbital domain should integrate with CUF adaptive fidelity.

Coarse propagation may return a refinement request when:

- predicted closest approach crosses a configured threshold;
- numerical uncertainty exceeds budget;
- maneuver execution approaches;
- atmospheric/body boundary interaction becomes relevant;
- collision probability becomes non-negligible;
- player/AI control requires finer state;
- downstream authority transfer is approaching.

A scheduler may raise fidelity but does not mutate orbital truth.

## Event-driven dormant propagation

Do not step every distant spacecraft at local gameplay frequency.

For each dormant object, retain or derive the next meaningful event horizon, for example:

- maneuver time;
- influence-region boundary;
- closest approach;
- observation/update requirement;
- predicted collision/refinement threshold;
- destination capture window.

Long inactive intervals may advance analytically or in deterministic chunks to the next event.

This is temporal LOD, not deletion of consequences.

## Representation-transfer proof

Any switch between orbital representations should produce or consume a `RepresentationTransferReceipt`.

Domain-specific proof should bind at least:

- source state/continuation identity;
- target state/continuation identity;
- common `SimInstant`;
- common physical object identity;
- reference-frame transform evidence when frames differ;
- mass/resource closure;
- position/velocity continuity under declared tolerance;
- angular momentum/energy diagnostics where meaningful;
- maneuver/event continuity;
- declared approximation/error budget.

A transfer may be `Exact`, `DeterministicApproximate`, or `BoundedEquivalent` under the Stellar Q5 vocabulary.

## State vs continuation identity

Physical orbital state alone may not be enough to resume exactly.

Continuation identity must additionally bind hidden state that can change future evolution, such as:

- next scheduled maneuver cursor;
- adaptive/numerical integrator phase if required;
- event-detection frontier;
- pending refinement lease;
- collision/encounter candidate state when non-rebuildable;
- deterministic keyed-RNG state if any stochastic model is introduced.

Prefer rebuildable indexes where possible. If a cache/index can be reconstructed deterministically from canonical state, identify the rebuild proof instead of persisting implementation detail.

## Determinism and numerical policy

The contract distinguishes deterministic execution from physical accuracy.

Required rules:

- iteration/order over unordered object sets must be canonical;
- no shared mutable RNG stream;
- solver configuration and constants are versioned inputs;
- floating-point canonical state normalizes semantic-equivalent encodings where needed;
- platform/compiler dependence must be measured before claiming bitwise replay;
- error tolerances are frozen evidence, not eyeballed after a run.

For solver families where cross-platform bitwise equality is unrealistic, use bounded-equivalence qualification while still requiring exact continuation metadata and event identities.

## Conservation

Orbital dynamics should not silently create or destroy:

- object identity;
- mass/cargo;
- propellant/resource inventory;
- occupants;
- accepted causal events;
- ownership.

Energy and angular momentum are diagnostics whose conservation expectations depend on thrust, dissipation, external forcing and approximation family.

Do not encode them as universally exact invariants.

## Relationship to physical interbody transit

`InterbodyTransitAuthority` may either be the orbital authority or compose it, but the architecture should avoid two mutable owners of the same trajectory.

Recommended first implementation:

- `symtropy-orbit` owns orbital trajectory state;
- an interbody-transit object owns/links cargo, occupants, damage and logistical resources;
- one stable physical-object/transit ID binds them;
- a single continuation manifest proves the complete traveling object.

If this split produces transactional complexity, merge them later based on measured implementation evidence rather than convenience speculation.

## Relationship to local `symtropy-physics`

Reuse `symtropy-physics` for close/contact dynamics.

Do not run its broadphase/contact solver over solar-system distances.

A bridge should hydrate a selected encounter set into local coordinates, preserve stable object identity, and return an explicit transfer result.

The floating-origin/view transform is presentation/local-physics scaffolding, not astronomical state identity.

## First vertical slice

Earth-like origin body + Mars-like destination body + one cargo vessel.

1. authoritative departure from local/orbital staging;
2. analytic transfer propagation;
3. mid-course maneuver;
4. save/suspend during cruise;
5. exact continuation restore;
6. causal refinement before destination encounter;
7. high-fidelity capture/close approach;
8. either successful destination handoff or continued transit after failed capture.

No step may spawn a duplicate vessel or assume successful arrival.

## Required tests

### Identity/ownership

- physical vessel exists under exactly one authority owner at a time;
- representation changes retain stable object ID;
- destination handoff is accepted exactly once;
- stale source owner cannot advance after handoff.

### Replay

- uninterrupted vs mid-transfer save/resume produce equivalent qualified arrival state;
- pending maneuver cursor survives continuation;
- event ordering is arrival-order independent where declared unordered.

### Propagator transitions

- analytic → numerical transition stays inside frozen state/error budget;
- numerical → local rigid-body hydration stays inside declared transform budget;
- local → orbital return preserves declared conserved state;
- failed equivalence proof rejects transition.

### Physics

- two-body reference cases match independent analytical fixtures within tolerance;
- long-horizon no-thrust orbit reports bounded energy/angular-momentum drift appropriate to solver;
- maneuver resource consumption and trajectory response close under policy;
- missed capture remains an orbit/transit object.

### Scale

- large dormant fleet does not require per-frame/per-local-tick stepping;
- next-event scheduler cost scales with active/relevant transitions rather than entire simulated history;
- adaptive refinement budget is deterministic for equal candidate sets.

## Non-goals for v0.1

- full general relativity;
- high-fidelity aerodynamics;
- perfect multi-century N-body ephemerides;
- spacecraft structural FEM;
- every propulsion technology;
- one universal solver.

The architecture must permit these later without changing object identity or authority semantics.

## Canonical invariants

1. **A physical transit object exists exactly once.**
2. **A propagator change is a representation change, not teleportation.**
3. **Reference-frame transforms are explicit evidence.**
4. **Numerical determinism does not imply physical exactness.**
5. **Dormancy may reduce computation, never erase pending consequence.**
6. **Arrival is an authority handoff that can fail.**
7. **Local rigid-body physics is a qualified close-encounter representation, not the solar-system authority.**
