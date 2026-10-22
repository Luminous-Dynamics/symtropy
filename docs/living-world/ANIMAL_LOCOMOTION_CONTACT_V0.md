# Animal Locomotion and Contact V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define the boundary between canonical locomotor intent/contact consequences and presentation animation/IK so animals can move believably over terrain without animation state becoming ecological authority.

## Core invariant

> Canonical movement determines where the animal can go and what contacts matter; animation/IK solves how that motion is presented.

## Canonical locomotor state

Candidate future-bearing state may include:

- body pose/root transform at the chosen ecological fidelity;
- locomotor mode/gait class when it affects costs/capability;
- velocity/trajectory intent;
- support/contact state required for canonical mechanics;
- fatigue/injury locomotor constraints;
- terrain/nav constraints;
- swimming/flying/climbing state where species support it.

Exact representation varies by fidelity.

## Hybrid locomotion pipeline

A high-quality presentation path may use:

`canonical desired trajectory -> gait selection -> animation phase -> anticipated foothold -> terrain query -> foot lock -> IK -> pelvis/spine compensation -> secondary motion`

Only explicitly canonical parts of that pipeline may affect ecology/gameplay outcomes.

## Foot contact

Foot placement should use terrain-aware contact queries and stable contact locks to minimize sliding, floating, and penetration.

A presentation foot target cannot teleport the canonical animal or create support where canonical physics/navigation says none exists.

If detailed foot contact affects gameplay (noise generation, slipping, injury, climbing grip, track formation), that contact must cross an explicit canonical contact boundary rather than being inferred from render bones after the fact.

## Gait / capability

Species/body-plan policy may define supported modes such as walk, trot, gallop, hop, crawl, swim, fly, climb, burrow.

Injury, fatigue, terrain, slope, depth, substrate, load, and physiology may restrict available modes or change cost.

Animation clips do not grant a mode the body/physics model does not support.

## Energy / physiology coupling

Canonical locomotion may generate metabolic/fatigue cost from authoritative motion and species physiology. The cost must not depend on render frame count or animation clip playback speed.

## Terrain coupling

Terrain queries should derive from canonical/qualified terrain state, not visual displacement alone.

Presentation-only microdisplacement may affect visual foot placement but cannot create canonical cliffs, holes, or support.

## Motion continuity across fidelity

Promotion/demotion between coarse movement and detailed locomotion must preserve canonical position/trajectory observables and must not grant free distance, energy, or injury recovery.

## Presentation-only secondary motion

Tail, ears, loose skin, fur, feathers, breathing, muscle jiggle, and similar secondary motion are presentation unless explicitly promoted into a gameplay-relevant canonical process.

## Qualification direction

At minimum test/measure:

1. renderer FPS/animation playback cannot change canonical distance traveled;
2. same locomotor intent + canonical terrain produces replay-equivalent movement for a fixed scheme;
3. detailed promotion/demotion preserves position and energy observables;
4. injury removes/reduces unsupported locomotor modes deterministically;
5. presentation foot IK cannot move canonical root authority;
6. foot-slip distance, floating, and penetration are measured in validation scenes;
7. canonical contact-derived noise/tracks are independent of render bone jitter;
8. visual terrain displacement alone cannot alter canonical navigation/support.

## Non-goals

This contract does not prescribe a specific physics engine, animation library, motion-matching system, gait database, or IK algorithm.