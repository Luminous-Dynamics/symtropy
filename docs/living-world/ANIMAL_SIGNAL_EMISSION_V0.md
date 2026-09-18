# Animal Signal Emission V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define how animal actions and physiology generate environmental signals that other agents can perceive, completing the causal loop between embodiment and perception.

## Core invariant

> Canonical/qualified actions create signals from physical/ecological causes; render/audio effects may express those signals but cannot be their sole authority.

## Candidate emissions

Animal state/actions may generate qualified cues such as:

- footfall/movement acoustics;
- substrate vibration;
- scent/chemical traces;
- alarm/contact/courtship vocalization;
- visual motion/display cues;
- disturbed vegetation;
- tracks/footprints;
- waste/dung/urine cues;
- blood/injury scent where modeled;
- heat/body odor proxies;
- water disturbance.

## Event -> signal pipeline

Conceptually:

`canonical action/contact/physiology -> emission intent -> qualified signal source -> propagation/decay -> receiver perception`

Playing a footstep sound does not itself prove a canonical foot contact occurred.

## Source binding

Every future-bearing signal binds to sufficient source semantics:

- source scope/entity/group;
- signal kind/scheme;
- canonical position/region;
- authoritative tick/time window;
- source intensity/quantity semantics;
- decay/propagation policy version where relevant.

Presentation may anonymize/aggregate the source visually, but canonical receivers cannot observe a signal that was never emitted.

## Persistence / traces

Transient sound may disappear quickly; scent, tracks, disturbed vegetation, nests, or waste may persist and decay on canonical ecology time.

Unload/reload must not instantly erase a persistent trace if that trace can still affect future behavior.

## Fidelity

Offscreen/coarse animals may emit aggregate signals if the aggregate preserves receiver-required observables. A herd may create a regional disturbance/scent pressure without simulating every footfall.

Promotion to detailed animation must not suddenly multiply emission magnitude merely because more feet are rendered.

## Anti-feedback

Render audio/particles/decals may be generated from canonical emissions, but those presentation effects do not feed back as duplicate canonical signals.

## Qualification direction

At minimum test:

1. no canonical action/contact -> no corresponding canonical emission;
2. animation-only footstep events cannot mint duplicate movement signals;
3. same action history -> deterministic emission sequence for a fixed scheme;
4. coarse herd vs detailed members preserves declared aggregate signal observables;
5. persistent traces survive save/reload until canonical decay removes them;
6. signal source identity/scope cannot alias after entity/render reuse;
7. receiver perception can trace back to a compatible emitted signal;
8. renderer/audio disabled leaves canonical signal ecology unchanged.

## Non-goals

This contract does not require full acoustic synthesis, CFD odor plumes, exact track deformation, or universal animal communication semantics.