# ANIMA Minimal Kernel Schema V0

Status: normative implementation-readiness contract. Documentation only.

## Purpose

Freeze the minimum dependency-neutral value/schema boundary the first reusable ANIMA kernel may expose, so biological and synthetic vertical slices share typed causal semantics without importing ECS/render/network/save authority or unrestricted world truth.

## Core invariant

> The reusable kernel consumes bounded qualified agent state and emits typed causal proposals/transitions; it does not own the world, ECS, renderer, network, persistence engine, or actuator authority.

Final crate placement remains subordinate to the dependency/license firewall.

## Dependency boundary

If licensing/dependency review permits, prefer:

`dependency-neutral ANIMA value kernel <- AGPL Living World/Bevy/persistence/network adapters`

and separate platform/robot adapters downstream.

Do not import an AGPL product crate merely to borrow an identifier if that unintentionally changes the reusable kernel's licensing/dependency surface. Product adapters may bind neutral IDs and ticks exactly to existing `StableId` and `SimulationClock` semantics.

## Typed causal identities

Semantically distinct identities are distinct types rather than interchangeable strings, indexes, Bevy entities, or integers.

Candidate identity families include:

- agent;
- emission;
- percept;
- experience;
- belief/hypothesis where needed;
- request;
- intent;
- action;
- policy;
- profile;
- scenario/manifest;
- authority epoch in network adapters.

Persistent agent identity must never be raw `bevy::Entity`.

## Canonical time

Kernel state refers to authoritative simulation ticks rather than render frames or wall-clock `Instant`.

A neutral tick value should map exactly to the product simulation clock through an adapter.

Physical durations and quantities remain physically typed where appropriate; they are not automatically normalized scores.

## Bounded normalized value

Implement the canonical bounded numerical convention through a validated type rather than scattered `f32` values.

The reference convention is millionth-scale unsigned fixed point compatible with the fauna perception oracle:

`0 ..= 1_000_000`

The type requires fail-closed range validation, widened intermediate arithmetic, explicit rounding, stable serialization, and no NaN/Inf semantics.

It must not replace meters, joules, newtons, seconds, angles, or other physical units.

## Freshness

Cross-rate evidence carries explicit freshness rather than only a value.

Required semantic states include equivalents of:

- Current;
- Held with original sample tick;
- Aggregated with interval and aggregation-policy identity;
- Stale with original sample tick;
- Missing with structured reason.

Missing/unknown is distinct from measured zero.

## Raw percept

A raw percept contains receptor-legitimate evidence only.

It may carry:

- percept identity;
- observer identity;
- canonical sample tick;
- modality/carrier;
- emission/source provenance when available;
- bounded detected strength;
- direction/range evidence only where the receptor measured it;
- uncertainty/precision semantics;
- freshness.

It does not normally contain inferred hidden semantic truth such as exact predator identity, friendly-human classification, or exact hidden target coordinates.

A directly encoded communication signal may legitimately convey a symbol because the signal itself carried that information; provenance remains required.

## Experience provenance

The closed admissible experience-source families are equivalent to:

- direct percept;
- bodily/action outcome;
- communication;
- social observation;
- inference from retained evidence.

There is no `WorldTruth` source.

## Experience trace

A future-bearing experience can retain, as policy requires:

- stable experience identity;
- subject agent;
- canonical tick/interval;
- source evidence references;
- involved partner/place/object identities;
- salience/importance;
- confidence/uncertainty;
- bounded appraisal tags actually produced by policy;
- policy/version identity for derived inference or consolidation.

Free-form prose is not canonical memory semantics.

## Belief / expectation

The reference schema must be able to represent uncertainty and error through:

- subject/hypothesis identity;
- supporting evidence;
- contradicting evidence;
- bounded confidence/precision;
- freshness/staleness;
- alternatives where ambiguity matters;
- revision-policy identity.

Unknown remains representable throughout serialization and adapters.

## Relationship expectation

Do not reduce all pair history to one friendship scalar.

Reference vertical slices may use a bounded subset such as familiarity, predictability, safety/trust expectation, affiliation, adverse/conflict association, and learned communication references. Dimensions remain policy/profile-defined rather than claims of universal animal psychology.

## Body capability view

ANIMA does not own canonical morphology or physics. Decision code receives a bounded read-only view containing only capability/state required for appraisal and affordances, such as:

- currently available locomotor/action modes;
- support/stability capability;
- manipulation capabilities;
- injury/fault constraints;
- energy/fatigue/thermal regulation signals;
- relevant body-size/load properties.

Do not expose the complete physics world when a bounded view is sufficient.

## Affordance candidate

An affordance is a relation among qualified observation, body capability, goals/regulation, and uncertainty.

A candidate may include:

- affordance/action kind;
- perceived/known target reference;
- required capabilities;
- supporting evidence;
- bounded feasibility, confidence, risk, cost, and information-gain terms used by policy;
- freshness;
- structured reason codes for unavailable/unsafe alternatives.

World geometry does not own a universal permanent `climbable` or `safe` boolean for all bodies.

## External requests

Player/partner/system input becomes a typed request with requester/evidence/time semantics rather than direct motor authority.

Reference disposition is a closed enum equivalent to:

- Accept;
- Hesitate;
- SeekInformation;
- ChooseAlternative;
- Refuse;
- Unable.

`Unable` denotes capability/constraint failure, not motivational refusal.

## Intent proposal and commitment

Intent candidates are distinct from committed intents.

A candidate carries intent/target semantics plus supporting evidence/appraisal/affordance references and any bounded policy score dimensions. Ordering/ties have explicit deterministic identity.

A committed intent receives stable identity, commit tick, policy/profile identity, and structured causal reason references.

## Action lifecycle

Canonical action phase semantics include equivalents of:

- Proposed;
- Committed;
- Executing;
- Suspended/Waiting;
- Interrupted;
- Completed;
- Failed.

Actions with side effects retain exactly-once settlement identity. Animation completion does not complete a canonical action by itself.

## Structured reasons

Canonical explanation stores stable reason codes and evidence references rather than generated prose.

Candidate codes include categories such as:

- no compatible receptor;
- evidence below threshold;
- stale evidence;
- low support confidence;
- memory hazard association;
- fatigue constraint;
- missing capability;
- unsafe request;
- controller veto;
- information gain preferred.

Human-readable Observatory text is rendered from those structures.

## Deterministic collections

Any collection whose order can influence canonical behavior uses explicit stable sorting/tie rules or ordered structures. `HashMap` iteration, ECS query chunk order, thread completion, and insertion history are not implicit policy.

## Kernel seam

The reusable seam is conceptually closer to:

`AgentQualifiedView + EvidenceBatch + Requests + Policy/Profile refs + Tick -> state deltas + intent/action proposals/transitions + structured trace`

than to an API that receives unrestricted `&mut World` and mutates arbitrary ECS state.

Exact decomposition into attention, memory, belief, appraisal, decision, and action modules may evolve behind these contracts.

## Serialization and adapters

Future-bearing values need explicit stable serialization at their owning product boundary. In-memory binary layout is not evidence format by accident.

Persistence, network, Bevy, robotics, FEP, and HDC layers adapt these values; they do not gain decision authority merely by transporting or storing them.

## Initial implementation order

After the qualified perception product exists:

1. ID/tick/freshness/reason/bounded-value primitives;
2. raw percept/evidence references compatible with product `SignalPerception`;
3. ExperienceTrace plus transparent bounded reference memory;
4. belief revision fixture;
5. body-capability view and affordance candidates;
6. request/disposition and intent proposal/commit;
7. action phase/transition;
8. headless reason trace;
9. species/platform profile adapters.

## Qualification direction

At minimum test:

1. compiler/API prevents accidental identity-family interchange;
2. persistent kernel fixtures require no Bevy `Entity`;
3. public cognition input exposes no unrestricted ECS/world handle;
4. missing/unknown round-trips distinctly from measured zero;
5. source container reordering cannot change canonical decisions;
6. structured reasons round-trip independently of display prose;
7. invalid normalized values fail closed and arithmetic follows the canonical fixed-point policy;
8. every ExperienceTrace fixture has an admissible source chain and no world-truth constructor exists;
9. product adapters preserve existing stable identity and simulation-tick meaning;
10. core tests run headlessly;
11. committed decisions retain sufficient policy/profile identity for replay evidence;
12. persistence/network adapters can serialize/transfer state without receiving cognition authority.

## Non-goals

No universal animal-mind struct, direct ECS/world ownership, renderer/network/save implementation, unrestricted physics-world input, untyped semantic string soup, mandatory HDC/FEP dependency, or finalized public API before perception and dependency/license gates are qualified is introduced here.
