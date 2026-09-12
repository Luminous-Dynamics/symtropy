# FEP-05 — observer-scoped attention and local diagnostics

FEP-05 constrains authored NPC behavior so that world truth does not silently become agent knowledge, attention, or action eligibility.

## Core rule

An NPC may orient, navigate, or choose an authored target only from information that has crossed an explicit observer-facing presentation boundary.

The following are distinct:

1. authoritative world state;
2. observer-facing presentation;
3. observer-local estimate or memory;
4. target salience;
5. close diagnostic information;
6. authoritative action result.

A hidden world flag may not skip directly from (1) to (4).

## Presentation boundaries

FEP-05 currently permits these presentation surfaces:

- local machine output/operational degradation;
- local noise/risk cues;
- coarse outward distress tiers;
- locally visible hostile-drone presence;
- explicit authored/tutorial assignment, provided the assignment does not itself grant remote target coordinates.

Exact private allostatic state and hidden equipment causes are not target-presentation data.

### Infrastructure

`PowerJunction::output`, `WaterPump::is_running`, and `WaterPump::efficiency` may be projected into coarse degradation salience because they represent observer-facing operation in the current vertical slice.

`WaterPump::is_sabotaged` is a hidden explanatory/diagnostic cause. It must not make a pump remotely salient or navigable.

A fully performing machine therefore contributes zero degradation salience even if an undisclosed internal failure flag exists.

## Local diagnostics

Some hidden state may become available after an actor reaches a capability-appropriate diagnostic boundary.

For the current PR-4 repair behavior, that boundary is `LOCAL_DIAGNOSTIC_RANGE = 30.0` game-space units.

The control-flow order matters:

1. establish physical locality;
2. only then inspect diagnostic state;
3. decide whether the local repair action is applicable;
4. mutate authoritative machine state only as the action result.

A distant pump's `is_sabotaged` value must not participate in PR-4 navigation or remote action eligibility.

## Deterministic target selection

Observer-local target selection must not depend on ECS/query iteration order.

FEP-05 uses bounded, distance-attenuated salience and deterministic coordinate ordering for exact ranking ties. Invalid coordinates, invalid ranges, zero-salience candidates, and out-of-range candidates are not actionable.

## Private social state

A medic does not receive another resident's exact allostatic scalar. Private load is projected through a deliberately coarse distress tier before it may become local target salience.

This means multiple private internal values intentionally map to the same outward cue.

## Non-goals

FEP-05 does not:

- make the FEP layer the authority for physical truth;
- create a universal sensor model;
- reinterpret consciousness-physics provider inputs as NPC memory;
- prove that every gameplay system is observer-scoped;
- claim realistic human perception;
- replace future capability-, occlusion-, line-of-sight-, instrumentation-, or communications-aware adapters.

It narrows one concrete authored-behavior surface and establishes the authority rule future adapters must preserve.

## Regression obligations

The implementation should preserve these properties:

- reversing candidate iteration order does not change the selected target;
- out-of-range and zero-salience targets cannot be selected;
- exact private allostatic load is not exposed to other agents;
- full observable machine output yields no degradation target;
- stopped/degraded visible operation can yield a target without revealing the hidden cause;
- remote sabotage state does not grant PR-4 target knowledge;
- hidden sabotage state may affect repair execution only after local diagnostic range is established;
- observer-frame estimates do not mutate authoritative world-state types.

## Qualification

This branch is implemented/static until exact-head formatting, compilation, tests, and Clippy execute successfully. No runtime PASS is implied by the contract or source review alone.
