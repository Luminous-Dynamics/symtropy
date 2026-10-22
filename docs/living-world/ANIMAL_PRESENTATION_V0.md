# Animal Presentation V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define one-way animal presentation so meshes, rigs, animation, fur/feathers, facial motion, breathing, and secondary motion reveal canonical animal state without becoming behavior, physiology, or identity authority.

## Core invariant

> Animation expresses canonical intent/state; animation state does not decide canonical animal truth.

## Presentation recipe

A future animal presentation recipe may derive from:

- canonical body plan/proportions;
- developmental/life stage;
- physiology/fatigue;
- canonical locomotor mode and trajectory;
- contact/support state;
- damage/impairment;
- current action intent;
- emotional/arousal proxy where canonically modeled;
- coat/skin/feather material state;
- phenotypic microvariation;
- presentation fidelity/salience.

## Animation stack

A high-quality renderer may combine:

- authored locomotion;
- motion matching;
- trajectory warping;
- gait-phase correction;
- terrain-aware foot targets;
- IK;
- pelvis/spine/head stabilization;
- breathing;
- blinking/saccades;
- ears/tail/whiskers;
- muscle/corrective deformation;
- skin/fur/feather secondary motion.

None of these presentation systems may independently mutate canonical behavior/physiology.

## Causal visual cues

Visible state should derive from biology where meaningful:

- fatigue -> breathing/posture changes;
- injury -> guarding/limp/scar cues;
- fear/arousal -> species-appropriate posture/attention cues;
- wetness -> coat/feather material response;
- age/development -> body proportions/coat changes;
- disease/poor condition -> qualified material/posture cues.

Presentation-only stylistic noise can enrich appearance but cannot invent future-bearing injury, fear memory, pregnancy, disease, or other biology.

## LOD

Presentation fidelity may range from impostor/proxy through hero rig/fur detail. Ecological fidelity is a separate axis.

Important causal cues—missing limb, severe limp, distinctive scar, coat state, group/narrative identity—should survive at perceptually relevant LODs.

## Interaction

A player cannot hit an animal merely because a stale render bone exists. Canonical interaction validates current body/contact authority.

Likewise, an animation footstep/attack event may request a canonical interaction only through an explicit validated bridge; animation notifications are not authority by themselves.

## Temporal interpolation

Rendering may interpolate canonical states for smoothness. Interpolation cannot advance hunger, healing, gait cost, behavior choice, or locomotor position authority.

## Qualification direction

At minimum measure/test:

1. renderer teardown/rebuild leaves canonical animal state unchanged;
2. FPS/animation-rate changes cannot alter canonical travel/energy/behavior;
3. foot-slip, floating, penetration, and trajectory mismatch metrics remain bounded in validation scenes;
4. injury/fatigue/body-plan cues map deterministically from canonical state;
5. interaction validates canonical state rather than stale rig references;
6. important causal cues persist across presentation LOD;
7. headless and rendered executions produce identical canonical ecology;
8. adding a secondary-motion system cannot change replayed animal behavior.

## Non-goals

This contract does not select an animation framework, mocap library, fur renderer, muscle solver, or art style.