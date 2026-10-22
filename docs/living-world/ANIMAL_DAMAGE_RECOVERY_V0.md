# Animal Damage and Recovery V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define persistent animal injury, impairment, recovery, and scar state as canonical biology rather than transient animation/effects.

## Core invariant

> Canonical injury changes physiology/capability first; presentation expresses it.

A blood decal, hit reaction, limp animation, or ragdoll state cannot independently create/heal canonical damage.

## Damage targets

Damage addresses stable biological body regions from the canonical body plan, not transient render bones/triangles.

Candidate consequences include:

- tissue integrity loss;
- bleeding/fluid loss where modeled;
- pain/stress load;
- locomotor impairment;
- sensory impairment;
- infection susceptibility;
- structural fracture/dislocation state;
- permanent loss/disability;
- scar/remodel state.

## Damage transaction

Conceptually:

`qualified contact/hazard + body region/state -> damage plan -> validate -> atomic physiology/body-plan commit -> presentation cues`

A stale contact cannot damage a different replacement body region through render-index reuse.

## Recovery

Recovery consumes canonical time/resources and depends on physiology, injury type, rest, infection, nutrition, and species policy where modeled.

Recovery is not:

- renderer unload;
- animation completion;
- respawn by LOD;
- instant restoration because the animal left the player radius.

## Persistent impairment

Some injuries may heal fully; others retain future-bearing consequences such as:

- reduced range/strength;
- altered gait;
- chronic pain/stress;
- sensory loss;
- scar tissue;
- increased vulnerability;
- learned avoidance associated with the injury event.

These consequences can force C1/C2/C3 collapse outcomes if coarse state cannot preserve them.

## Death boundary

Fatal injury eventually crosses the exact mortality/stock-settlement architecture rather than simply despawning the entity.

This contract does not itself claim conservation-qualified death; it only requires that injury state leading to death cannot be erased by presentation lifecycle.

## Presentation

Derived cues may include:

- hit reactions;
- guarding/limping;
- posture asymmetry;
- wounds/scars;
- blood/soiling where appropriate;
- reduced locomotor animation range;
- breathing/fatigue cues;
- coat/skin changes.

Presentation interpolation cannot change healing progress.

## Qualification direction

At minimum test:

1. same canonical injury survives render/rig rebuild;
2. injury to a locomotor region restricts compatible canonical movement;
3. healing progress follows authoritative time/process state, not FPS;
4. save/reload preserves recovery trajectory;
5. stale body-region/render references fail closed;
6. permanent impairment cannot disappear under LOD collapse unless coarse state preserves it;
7. hit animation without canonical damage cannot reduce physiology;
8. fatal injury cannot become ordinary entity despawn while leaving population/stock authority unchanged.

## Non-goals

This contract does not define medical realism, gore rendering, universal fracture mechanics, or exact mortality chemistry.