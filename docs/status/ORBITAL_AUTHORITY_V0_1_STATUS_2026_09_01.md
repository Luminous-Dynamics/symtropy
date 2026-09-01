# Orbital Authority v0.1 Status — 2026-09-01

Status: design-only / not runtime-qualified

## Repository finding

The current public `symtropy-physics` crate is a local rigid-body/contact engine with articulation, bodies, broadphase, CCD, constraints, contact solving, GJK/EPA, integration, islands, joints and related local dynamics.

No existing public domain crate was identified that should be stretched into canonical orbital/ephemeris authority.

Therefore the recommended future ownership split is:

- `symtropy-orbit`: orbital/interbody trajectory authority and propagator representations;
- `symtropy-physics`: high-fidelity local/contact dynamics;
- bridge: qualified orbital ↔ local encounter hydration/reconciliation;
- CUF contracts: identity, time, frames, continuation, representation-transfer evidence;
- transit/logistics layer: cargo/occupants/resources composed with one stable physical transit object.

## Proposed patch campaign

### ORB-0 — dependency-light orbital contracts

Branch candidate: `orbit/v0.1-contracts`

Add stable IDs/types for:

- orbital object;
- propagation representation;
- maneuver;
- encounter/refinement request;
- orbital continuation identity;
- typed state/error budget.

No solver yet.

### ORB-1 — two-body reference propagator

Branch candidate: `orbit/v0.2-two-body`

Implement a small deterministic reference propagator and independent analytical fixtures.

Scope:

- one dominant central body;
- position/velocity + epoch representation;
- frozen gravitational parameter/model identity;
- no hidden global time;
- no Bevy.

Qualification begins with simple circular/elliptic reference cases and round-trip/state-error evidence.

### ORB-2 — maneuver ledger + event horizon

Branch candidate: `orbit/v0.3-maneuvers-events`

Add:

- append/commit/cancel maneuver semantics;
- deterministic maneuver cursor;
- next meaningful event calculation;
- dormant object advance-to-event behavior;
- continuation digest including pending event/cursor state.

### ORB-3 — frame graph integration

Branch candidate: `orbit/v0.4-frame-evidence`

Consume #98 reference-frame transform evidence.

Prove that equal physical state expressed through legal frame paths agrees under the frozen policy.

No frame-name parsing heuristics.

### ORB-4 — interbody transit authority

Branch candidate: `orbit/v0.5-transit-authority`

Compose/implement #103:

- stable physical transit identity;
- source departure receipt;
- orbital trajectory authority;
- mass/cargo/propellant/occupant references;
- destination intent;
- arrival/capture receipt;
- failed-arrival continuation.

### ORB-5 — local encounter bridge

Branch candidate: `orbit/v0.6-local-encounter-bridge`

Hydrate a bounded encounter into `symtropy-physics` without duplicating authority.

Required proof:

- frame transform evidence;
- object-ID preservation;
- state continuity/error budget;
- resource/mass closure;
- return-to-orbit reconciliation.

### ORB-6 — suspend/resume mid-flight

Branch candidate: `orbit/v0.7-continuation`

Bind orbital/transit continuation state into the hierarchical world continuation root.

One-shot transfer and save/reload transfer must satisfy the declared equivalence class.

### ORB-7 — two-body Earth↔Mars-style qualification fixture

Branch candidate: `orbit/v0.8-two-body-fixture`

Fixture:

1. depart origin staging authority;
2. analytical cruise;
3. maneuver;
4. save/reload;
5. refine near destination;
6. attempt capture;
7. successful handoff or continued failed-capture trajectory.

Combine with #97/#102 for delayed command/observation evidence when FiniteInformation is enabled.

### ORB-8 — higher-fidelity numerical propagation

Branch candidate: `orbit/v0.9-nbody`

Add a deterministic N-body/symplectic representation only after ORB-7 proves authority/continuation semantics.

Qualification compares it against frozen reference scenarios and error/invariant budgets.

### ORB-9 — fleet/system stress

Branch candidate: `orbit/v1.0-system-stress`

Measure:

- thousands/millions of dormant transit objects under event-driven scheduling;
- active encounter/refinement budget;
- memory per object/continuation record;
- catch-up performance across long inactive intervals;
- deterministic selection under equal candidate sets.

This is a Q5 scale gate, not a reason to make early solvers complicated.

## Immediate dependency order

The recommended order is:

1. finish/qualify continuation core (#96 → #101 → #105);
2. Q0/Q1 Universal Matter replay (#74);
3. Q2 continuation hardening (#76/#79/#81/#83/#84);
4. frame-graph contract/runtime (#98);
5. ORB-0/ORB-1;
6. ORB-2/ORB-3;
7. physical transit (#103) + ORB-4;
8. local `symtropy-physics` bridge (ORB-5);
9. continuation + two-body fixture (ORB-6/ORB-7);
10. only then higher-fidelity N-body/system work.

## Why not start with N-body

A sophisticated solver does not solve the harder world problem if:

- vessels duplicate at authority handoff;
- frames are ambiguous;
- pending maneuvers vanish on reload;
- local physics hydration changes identity;
- distant objects require high-frequency stepping;
- information teleports between planets.

Therefore authority/continuation/frames come before numerical sophistication.

## Qualification vocabulary

- Q2: orbital authority continuation correctness once orbital state participates in world resume;
- Q3: native cross-domain/frame/continuation integration;
- Q4: bounded two-body living-world/transit vertical slice;
- Q5: stellar physical, epistemic, long-horizon and fleet-scale qualification.

## Current claim boundary

No orbital runtime has been implemented by this tranche.

No claim is made that `symtropy-physics` currently solves astronomical trajectories.

This tranche freezes an architecture intended to let existing local physics remain excellent at the problem it actually solves while adding orbital authority as a separate causal domain.
