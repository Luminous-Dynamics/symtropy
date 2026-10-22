# Animal Body Plan V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define canonical animal morphology/topology independently from meshes, animation rigs, skinning, and presentation LOD.

## Core invariant

> Animal biological/body topology is canonical; render skeletons and meshes are derived presentation/control structures.

A renderer or animation retargeting operation cannot add/remove a canonical limb, heal an injury, change age, or alter organism physiology.

## Candidate body-plan state

A future versioned `AnimalBodyPlan` may describe:

- body regions and hierarchical attachments;
- canonical limb/appendage topology;
- joint classes/ranges relevant to canonical contact/damage;
- body proportions / structural dimensions;
- locomotor support points (feet, hooves, paws, fins, etc.);
- sensory-organ attachment classes;
- tissue/region identifiers for damage;
- body-plan/development schema version.

Exact anatomy remains species-specific.

## Structure vs rig

A presentation rig may contain extra helper bones, twist bones, facial controls, fur controls, IK targets, muscle proxies, or deformation joints.

Those are not canonical biological elements unless an explicit body-plan mapping says otherwise.

Likewise, canonical body regions may map to several render bones or no dedicated render bone at lower LOD.

## Developmental morphology

Body proportions may derive from heredity, developmental stage, nutrition/stress history, injury, and other qualified biology.

Mature persistent morphology is not recomputed directly from current environment every frame.

## Damage

Canonical body-region damage addresses biological region identifiers, not triangle/bone indices.

Loss/disability of a limb/region persists through rig/mesh rebuilds and may alter locomotion capability, behavior, metabolic cost, and presentation.

## Collision / contact mapping

Canonical contact capability may use simplified body/contact primitives derived from the body plan. High-detail render mesh collision is not automatically canonical authority.

The mapping from body plan to physical/contact representation is versioned when it affects gameplay outcomes.

## LOD

Presentation LOD can collapse fur, facial bones, muscle deformation, digit detail, etc. without changing body-plan truth.

Ecological/behavioral fidelity is a separate axis. A distant animal may retain canonical injury/body state even if presented as an impostor.

## Qualification direction

At minimum test:

1. same canonical body plan can generate multiple render rigs/LODs without changing biological state;
2. render teardown/rebuild cannot restore a missing/damaged canonical region;
3. extra presentation bones cannot create canonical joints/limbs;
4. body-region references remain stable across harmless remeshing/retargeting;
5. developmental proportion changes follow retained biological state, not FPS/camera;
6. headless simulation requires no render skeleton/assets.

## Non-goals

This contract does not define universal vertebrate/invertebrate anatomy, a final animation rig format, FEM muscles, or rendering technology.