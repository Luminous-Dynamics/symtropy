# Fauna Perception Causality Benchmark V0

## Purpose

Define qualification scenarios that detect false omniscience and verify that animal percepts arise from qualified signal causality rather than exact hidden world queries.

## Core question

Can we change what an animal could physically/ecologically sense while keeping hidden world truth fixed, and observe the expected change in canonical percept/action state?

If not, the perception architecture is not meaningfully causal.

## Controlled scenario family

Freeze a source/animal/habitat configuration and vary one cause at a time.

### P0 — hidden source, no emission

The source/threat/resource exists in canonical world state, but no compatible signal reaches the animal.

Expected: no percept attributable to that source.

This is the primary false-omniscience regression.

### P1 — source emits, clear path

Compatible emission exists and path/receptor conditions support detection.

Expected: provenance-bearing percept appears.

### P2 — same source/emission, occluded path

Only propagation/occlusion changes.

Expected: perceived strength/detection changes according to the qualified propagation model.

### P3 — same signal, receptor impaired

Only receptor orientation/sensitivity/physiology changes.

Expected: sensing degrades without changing source truth.

### P4 — same source/path, environmental masking

Canonical noise/competing signal increases.

Expected: detectability changes according to the qualified masking model.

### P5 — unsupported channel

Signal exists but receptor lacks the required modality/channel.

Expected: no percept from that channel.

## Provenance evidence

A successful percept should retain enough evidence to trace:

`source emission -> propagation/path authority -> receptor evaluation -> percept`

The benchmark should fail if a percept names a source that has no compatible causal signal lineage.

## Behavior separation

Perception qualification and behavior qualification are separate.

A correct percept does not prove the animal chose a biologically good action. Conversely, a plausible action does not prove sensing was non-omniscient.

Record both when behavior is present:

- canonical percept state;
- bounded memory/belief update;
- chosen homeostatic intent;
- resulting movement/ecological action.

## Presentation independence

Repeat at minimum:

- headless;
- rendered;
- low/high presentation LOD;
- different animation frame rates.

Canonical percept outcome must remain invariant where only presentation changes.

Presentation audio/particles/decals may visualize the same signal but cannot become a second canonical emission.

## Fidelity

If coarse signal propagation is claimed equivalent to fine propagation for a scenario, compare the declared preserved observables and tolerance.

Exact provenance/identity requirements cannot be waived by a numeric error budget.

## Save/reload

For future-bearing memories derived from a percept, compare uninterrupted execution with save/reload after detection.

Reload may not:

- invent a percept;
- forget a persistent memory earlier than its canonical semantics allow;
- change source provenance;
- reroll derived future-bearing memory state.

## Metrics

Useful metrics include:

- false-positive percept rate under P0;
- false-negative rate where P1 guarantees qualified detectability;
- monotonicity under controlled attenuation;
- source-provenance validity;
- replay divergence count;
- headless/rendered state hash equality;
- downstream action divergence attributable to perception rather than hidden query paths.

## Evidence claim boundary

Passing supports claims about causal sensing/non-omniscience under the tested signal model.

It does not prove real-world sensory calibration, animal cognition quality, ecological realism outside the tested scenarios, or presentation realism.

## Initial implementation relationship

PR #246 freezes the first integer/fixed-point non-omniscience oracle using Basin `SignalKind`; #247 is its exact execution lane. Future product work under #249 should eventually become the subject of this benchmark.
