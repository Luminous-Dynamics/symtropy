# Stellar Causal Substrate Contract v0.1

**Status:** design freeze; implementation pending lower-layer Q2 continuation qualification  
**Scope:** interplanetary/stellar reference frames, finite information propagation, delayed knowledge, and conserved physical transit  
**Tracks:** #97, #98, #102, #103; builds on #83/#93/#95/#96

## 1. Purpose

A stellar-scale Symtropy world must not become a collection of locally plausible planets connected by causal teleportation.

This contract freezes four distinct cross-body responsibilities:

1. **reference-frame truth** — how coordinates on different bodies relate at a simulation instant;
2. **information transit** — when a signal, command, telemetry packet, or observation can causally arrive;
3. **epistemic state** — what a remote actor actually knows, versus hidden current authority state;
4. **physical transit** — how ships, cargo, debris, occupants, and resources remain authoritative between local body scopes.

The common stellar substrate owns contracts and orchestration. It does not become the owner of planetary terrain, local ecology, spacecraft internals, markets, residents, or networking transport.

## 2. Fundamental invariants

### 2.1 No causal teleportation

For profiles claiming finite propagation:

> A cross-scope effect may influence a target only after the propagation/transit contract says it is causally eligible at the target `SimInstant`.

Host memory locality, network packet arrival, rendering distance, and player camera location cannot bypass this rule.

### 2.2 No physical teleportation

A physical object leaving one authority scope must remain owned by exactly one authority while in transit.

Conceptually:

```text
local/body authority
        |
        | departure + conservation receipt
        v
interbody transit authority
        |
        | arrival/capture + conservation receipt
        v
local/body authority
```

There is never an interval where the object has no authoritative existence, and never a valid interval where two authorities independently own the same canonical object.

### 2.3 No epistemic omniscience by accident

Current remote authority state is not automatically local knowledge.

```text
Mars authoritative state at T
        !=
Earth knowledge of Mars at T
```

unless the selected profile explicitly declares omniscient/debug semantics.

### 2.4 No coordinate comparison without a frame contract

Coordinates from different `ReferenceFrameId`s are not comparable merely because they have the same numeric shape.

A transform/evidence path is required.

## 3. Hierarchical stellar composition

The stellar substrate composes with `WorldContinuationManifest` rather than replacing it.

Conceptually:

```text
System continuation root
├── stellar/global forcing + frame graph
├── pending information-transit continuation
├── interbody physical-transit continuation
├── Earth body manifest
├── Mars body manifest
├── orbital infrastructure manifest(s)
└── other body/region manifests
```

Unchanged planetary subtrees may remain content-addressed and unloaded while system-level transit continues.

No whole-system high-fidelity ECS is required.

## 4. Shared time semantics

Every cross-body causal record uses the portable `SimInstant` coordinate.

Local tick domains may use fixed/rational timebases, but they must map through the explicit timebase contract tracked in #95.

Wall-clock time is never the canonical arrival clock.

A transit or observation may record several simulation instants with different meanings. These must not be collapsed:

- `emitted_at`;
- `observed_at`;
- `received_at`;
- `checkpoint_at`;
- `maneuver_at`;
- `arrival/capture_at`.

## 5. Reference-frame graph

### 5.1 Frame identity

`ReferenceFrameId` names a frame. It does not define its transform.

Examples:

```text
sol:barycentric
sol:earth:inertial
sol:earth:surface-fixed
sol:mars:inertial
sol:mars:surface-fixed
vehicle:<id>:local
```

Names are opaque identifiers, not parsable physics.

### 5.2 Transform evidence

A transform claim binds at minimum:

```text
source_frame
target_frame
at: SimInstant
transform_model_id
model_or_config_digest
source_state_or_forcing_digest?
result_digest
exactness_or_error_policy_digest
```

The result may bind canonical pose/velocity data or a typed digest over a domain-specific canonical transform representation.

### 5.3 Transform provenance classes

A transform is explicitly one of:

- **Static** — fixed relationship under a frozen model;
- **DeterministicForcing** — analytic ephemeris/rotation generated from exact model/config/time inputs;
- **AuthorityBacked** — depends on mutable canonical orbital/body state.

A deterministic ephemeris is not automatically authority truth.

### 5.4 Composition

A frame graph may compose transforms through intermediate frames only under a declared composition policy.

Different legal transform paths must agree under the declared exact/tolerance contract.

Inconsistent cycles fail closed.

## 6. Information transit

Information transit covers causal payloads that do not themselves transfer conserved physical matter ownership, for example:

- commands;
- telemetry;
- distress calls;
- remote sensor observations;
- market messages;
- governance decisions;
- synchronization/control messages that are part of simulation causality.

Raw network packets remain transport/runtime data.

### 6.1 Causal transit record

A canonical transit binds conceptually:

```text
transit_id
world_instance
source_scope
source_frame
target_scope
target_frame
emitted_at
payload_digest
propagation_model_id
propagation_config_digest
path_or_relay_digest?
earliest_or_canonical_receive_at
causal_parents[]
```

The transit identity is serializer-independent.

### 6.2 In-flight continuation state

Pending transit is continuation-significant.

An owning transit queue/state must expose a continuation identity covering every pending record plus any hidden ordering/cursor state that can affect deterministic delivery.

A save must not erase a distress call merely because neither endpoint is loaded.

### 6.3 Delivery

A target may consume a transit only when its canonical receive condition is met.

Delivery must be idempotent under duplicated runtime/network transport.

Packet ordering is not canonical transit ordering.

## 7. Causal knowledge horizons

### 7.1 Observation vs reception

A remote received observation has at least two times:

```text
observed_at   = when source state was measured
received_at   = when receiver causally obtained that measurement
```

Both are identity-significant.

### 7.2 Knowledge is local state

An actor may hold:

- direct/local observations;
- delayed remote observations;
- deterministic predictions;
- inferred beliefs;
- unknowns.

Only observations backed by authority evidence are observations.

Predictions remain predictions even if the hidden future state later matches.

### 7.3 AI and planning

For a physical-information profile, gameplay AI/planners use only causally available knowledge.

An omniscient/debug AI profile is allowed only through an explicit different policy identity.

Host co-location of Earth and Mars simulations cannot grant an Earth actor hidden Mars truth.

### 7.4 Continuation significance

If an agent's remembered knowledge changes future decisions, that knowledge/memory state is continuation-significant for that agent or planning domain.

## 8. Physical interbody transit

Physical transit covers canonical objects whose conserved contents remain meaningful while moving between local authority scopes.

Examples:

- spacecraft;
- cargo;
- passengers;
- robotic probes;
- asteroids/comets when promoted to persistent objects;
- debris;
- transported resources.

### 8.1 Transit authority record

A physical transit record binds conceptually:

```text
object_id
source_handoff_receipt
departure_scope
departure_at
canonical_frame
trajectory_state_digest
current_at
mass_or_matter_digest
inventory_or_resource_digest
energy_propellant_digest
occupant_continuation_refs[]
damage_thermal_radiation_digest?
maneuver_policy_or_plan_digest
forcing_context_digest
representation_identity
continuation_digest
```

Destination intent may be recorded, but intent is not guaranteed arrival.

### 8.2 Conservation handoff

Departure and arrival/capture are explicit authority/conservation transfers.

The receiving domain verifies the exact object continuation/state it is accepting.

A destination cannot create a canonical duplicate because a schedule said a ship was "supposed to arrive."

### 8.3 Failure remains world state

The transit model must support:

- missed capture;
- aborted burns;
- stranded craft;
- loss of communications;
- destruction;
- debris creation;
- rerouting;
- destination changes;
- rescue/interception.

Those outcomes remain causal state rather than being collapsed into an arrival timer.

## 9. Multirate representation

Stellar scale requires representation hierarchy.

A physical transit may move between:

```text
analytical/coarse orbit
        |
        | refinement trigger + transfer receipt
        v
high-fidelity local encounter/maneuver
        |
        | domain-approved coarsening receipt
        v
analytical/coarse orbit
```

The representation change may reduce geometry/integration detail but may not silently alter conserved state or discard consequences.

Work budget/scheduling is not physics.

## 10. Causal cones and relevance

The common layer may compute **causal relevance** without inventing domain effects.

Examples:

- a solar flare makes Earth/Mars radiation domains worth updating;
- an inbound distress signal makes a receiver scope relevant at its receive horizon;
- an approaching spacecraft makes a destination/orbital region worth refining;
- a collision trajectory makes two transit objects mutually relevant.

As with watershed connectivity, relevance is not authority state.

## 11. Inactive-world evolution

A body may be unloaded while transit/system state continues.

The inactive-time policy from #85 governs whether the body itself is paused, stepped, coarsely evolved, or event-driven.

Transit crossing the interval is handled deterministically:

```text
body suspended at T0
signal arrives at T1
ship flyby occurs at T2
body resumed/caught up to T3
```

The catch-up must incorporate exactly those causal inputs whose canonical receive/encounter times fall in the interval under the selected policy.

## 12. Distributed/networked runtime

Network transport may carry transit records or authority handoffs, but it does not determine simulation truth.

Issue #94 authority epochs remain separate from information-transit IDs.

For a server migration:

- pending causal transits survive;
- physical-transit ownership survives;
- accepted distributed authority epoch survives;
- runtime socket queues may be discarded/rebuilt.

## 13. Persistence and continuation manifest

The system/root manifest binds, directly or through child manifests:

- reference-frame graph/model identity;
- pending information-transit continuation identity when enabled;
- interbody physical-transit continuation identity when objects are in flight;
- forcing/ephemeris context required to continue;
- distributed authority context when enabled;
- applicable inactive-time policy.

A pure suspend/restore must reproduce these identities before simulation resumes.

## 14. Profile semantics

Not every game/session must use maximum physical latency.

Profiles may include:

### LocalArcade

Cross-body features outside current local scope are absent or explicitly simplified.

### InstantInformation

Information propagation is gameplay-instantaneous under an explicit policy, while physical transit remains conserved.

### FiniteInformation

Signals/observations obey finite propagation.

### PhysicalInterbody

Adds conserved physical transit and qualified orbital/frame semantics.

### StellarPhysical

Enables the full v0.1 stellar causal substrate and later Q5 scale requirements.

Profile identity is continuation-significant.

## 15. Anti-architecture rules

Do not:

- use socket arrival time as `received_at`;
- query hidden current remote authority state from ordinary AI/UI under a finite-information profile;
- spawn a physical traveler at destination without a verified authority-transfer lineage;
- compare vectors from different frames without transform evidence;
- use player distance alone to delete pending transit;
- make an unloaded body implicitly freeze if its manifest says it catches up;
- force all planets/ships into one simulation tick frequency;
- keep two canonical ship copies to simplify handoff;
- call a deterministic predicted state an observation;
- turn the world root into owner of transit-domain mutable state.

## 16. First vertical fixtures

### S1 — Earth/Mars delayed signal

- emit at Earth;
- compute deterministic finite receive time;
- suspend system mid-transit;
- restore;
- prove no early Mars delivery;
- deliver once at canonical receive time;
- prove duplicate host packets do not duplicate canonical delivery.

### S2 — delayed observation

- Mars authority changes;
- Earth only possesses earlier received state;
- Earth AI acts from stale but valid knowledge;
- later observation arrives and updates knowledge;
- hidden current Mars state never leaks into the first decision.

### S3 — conserved cargo transfer

- depart Earth local authority;
- enter interbody transit authority;
- suspend/resume mid-flight;
- perform a maneuver;
- arrive/capture at Mars;
- prove exact once-only authority handoff and resource conservation.

### S4 — missed arrival

- same planned voyage;
- perturb a canonical maneuver/forcing input;
- miss capture;
- prove object remains transit state instead of snapping to Mars.

## 17. Relationship to Q layers

### Q2

Core continuation correctness. Stellar features become Q2-required only when the chosen continuation profile enables them.

### Q3

Native domain integration (e.g. Universal Matter/CUF) remains independent of whether stellar transit is enabled.

### Q4

Can include a small two-body causal slice once local Living Watershed/planet foundations are stable.

### Q5

Stresses:

- many body manifests;
- large pending transit sets;
- long inactive intervals;
- frame-transform consistency;
- multi-year/century travel;
- coarse/refined physical transit equivalence;
- bounded catch-up;
- memory/performance budgets;
- optional relativistic extensions.

## 18. Design outcome

The desired result is:

> planets remain independent living causal worlds, yet signals, knowledge, ships, cargo, disasters, and decisions cross the space between them through explicit time, frame, provenance, conservation, and continuation contracts.

That lets Symtropy scale outward without sacrificing the causal discipline established for a single watershed.