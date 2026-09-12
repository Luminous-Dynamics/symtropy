# FEP-06 — deterministic authored event gates

FEP-06 removes ambient per-frame randomness from consequential authored NPC event emission.

## Problem

The Leo relapse path previously emitted its warning/action event through an unseeded `rand::random::<f32>() < 0.01` check every frame while the relapse condition held.

That made whether and how often the event appeared depend on ambient RNG and the number of `Update` frames spent in the condition. Because the emitted `NpcActionEvent` carries gameplay-facing deltas, this is not merely a cosmetic randomness choice.

## Rule

A consequential authored event should be emitted from an explicit state transition or from a stochastic process whose state, seed, cadence, and provenance are owned by the simulation.

An ambient per-frame random draw is not an acceptable authority for such an event.

FEP-06 applies the first form to the current Leo relapse warning:

1. capture the previous allostatic load;
2. update the existing relapse state;
3. emit the warning only if the state crosses upward through `LEO_RELAPSE_ALERT_THRESHOLD`;
4. do not emit merely because the state remains above the threshold.

The transition predicate fails closed for non-finite values.

## Semantics

The threshold is currently `0.8`.

A transition such as `0.79 -> 0.80` emits the event. `0.80 -> 0.81` does not. If the state later falls below the threshold and crosses upward again, that later crossing is a new episode and may emit again.

If the state begins already above the threshold, FEP-06 does not fabricate a historical crossing event.

## Determinism boundary

This tranche does **not** claim that the complete NPC simulation is replay-deterministic.

The FEP behavior/action/movement chain is currently registered on Bevy `Update`, and several state changes use `Time::delta_secs()`. Therefore a different frame-time trajectory can still produce a different state-update trajectory.

FEP-06 establishes the narrower theorem:

> Given the same ordered state trajectory through the authored relapse gate, event emission no longer depends on ambient random draws or the number of repeated frames spent above the threshold.

Future full replay qualification should move authoritative state evolution onto an explicit deterministic simulation cadence or otherwise bind time-step inputs into the replay/evidence contract.

## Stochastic simulation is still allowed

FEP-06 does not ban stochastic behavior. Stochastic processes can be appropriate for ecology, behavior variation, uncertainty, or procedural content when their authority is explicit.

For consequential simulation state, a stochastic process should eventually expose at least:

- deterministic/replayable seed or state;
- explicit update cadence;
- stable ownership of the RNG stream;
- clear distinction between cosmetic and authoritative randomness;
- evidence/provenance sufficient to reconstruct why an event occurred.

## Regression obligations

The current implementation checks that:

- a finite upward threshold crossing returns true;
- already-above states do not repeatedly trigger;
- states that remain below the threshold do not trigger;
- non-finite previous/current/threshold values fail closed;
- the old ambient random event gate is absent from the relapse path.

## Non-goals

FEP-06 does not:

- move the FEP systems to `FixedUpdate`;
- change the existing allostatic-load integration rate;
- add a global deterministic RNG service;
- redefine `NpcActionKind` in this tranche;
- prove replay identity across machines or frame schedules;
- prohibit non-authoritative visual/audio randomness.

## Qualification

Implemented/static only until exact-head formatting, compilation, tests, and Clippy execute successfully. Source inspection and GitHub mergeability are not runtime qualification evidence.
