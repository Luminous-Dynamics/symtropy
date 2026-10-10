# Animal Sensory Perception V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define non-omniscient animal perception as a causal pipeline from physical/ecological signals into bounded percepts.

## Core invariant

> Animals act on percepts derived from signals they could plausibly receive, not on unrestricted world-state queries.

## Causal pipeline

`physical/ecological event -> signal emission -> propagation/attenuation -> receptor sampling -> percept -> belief/memory -> action policy`

Each stage may be simplified, but skipping stages changes the authority/realism claim and must be explicit.

## Existing signal seam

Symtropy Basin already defines typed signal vocabulary including pheromone, root exudate, fungal pulse, bird alarm, vibration, chemical gradient, and source/readability metadata. Fauna should integrate through compatible signal adapters rather than create a parallel omniscient perception universe.

## Sensory modalities

Candidate modalities include:

- vision;
- hearing/acoustic cues;
- smell/chemical cues;
- vibration/substrate cues;
- touch/contact;
- temperature;
- magnetic/electrical/specialized senses only where species biology supports them.

Species/body plans declare receptor capabilities and thresholds.

## Signal authority

Signals may be physical fields, indices, or event-derived cues depending on their qualified model. A signal value is not automatically exact physical truth.

Perception must preserve the distinction between source authority and derived sensory evidence.

## Occlusion and attenuation

Where relevant, detection should depend on factors such as:

- distance;
- terrain/vegetation obstruction;
- wind direction for scent;
- rain/humidity effects;
- surface/substrate for vibration;
- background noise;
- source intensity;
- receptor orientation/sensitivity;
- animal physiological condition.

V0 may use coarse deterministic attenuation rather than expensive wave simulation.

## No player-centric activation

An animal does not become aware of the player merely because the player enters an activation radius.

Region/fidelity systems may decide how perception is approximated, but any canonical behavioral consequence must be compatible with the information available at that fidelity.

## Uncertainty

Percepts may include confidence/precision rather than binary truth.

A weak scent may indicate likely predator direction without exact coordinates. A distant sound may convey class/threat but poor localization.

Uncertainty is canonical process state when it changes future behavior.

## False/ambiguous signals

Different causes may create similar percepts. Animals may investigate, ignore, flee, orient, or update belief based on experience.

Do not grant semantic labels such as `PlayerIsBehindRock` when only an attenuated acoustic/olfactory cue exists.

## Determinism

For a fixed canonical state, signal scheme, receptor state, and authoritative tick, perception results are deterministic or use explicit keyed stochastic semantics whose scheme is versioned.

Sequential runtime RNG and thread iteration order are not perception authority.

## Qualification direction

At minimum test:

1. no signal/receptor path -> no corresponding canonical percept;
2. attenuation reduces detectability in the authored direction;
3. scent responds to wind/source geometry under a frozen simple fixture;
4. occlusion/noise can degrade confidence without revealing hidden exact target state;
5. renderer visibility/occlusion is not canonical animal vision authority unless explicitly bound through a qualified perception backend;
6. same source history + same receptor state -> replay-identical percept sequence;
7. coarse/offscreen perception is admitted only when it preserves process-required information;
8. the player can remain physically nearby but undetected when sensory conditions justify it.

## Non-goals

This contract does not require full acoustic ray tracing, CFD scent simulation, neural vision models, or universal sensory thresholds.