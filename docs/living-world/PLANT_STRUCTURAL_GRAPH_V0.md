# Plant Structural Graph V0

## Status

Normative design contract for future Living World flora. This document does not claim a Rust plant graph already exists.

## Goal

A canonical plant should preserve the biological structure that explains its future growth, damage, mechanics and appearance without storing renderer triangles.

The key boundary is:

```text
canonical plant structure
    -> derived geometry recipe
    -> mesh / instances / impostor / shader
```

The reverse direction is forbidden: rendered geometry is not plant authority.

## V0 topology

Use one rooted append-oriented branching structure per canonical plant individual.

A structural element has one parent except for explicit roots/origins. V0 does not model graft fusion or anastomosing loops inside one plant graph; those require a future topology version.

Conceptually:

```text
PlantStructure {
    structure_schema_version,
    shoot_root,
    root_root,
    elements,
    active_growth_tips,
}
```

The exact Rust representation is open.

## Local structural identity

Plant part identity is scoped to one plant authority, for example a monotonic local ordinal.

It is not automatically a global `StableId` or CRK event identity.

A part coordinate must remain stable once future-bearing state references it, including:

- damage;
- pruning;
- infection;
- load-bearing relationships;
- growth-tip ancestry;
- player interaction;
- causal appearance telemetry.

Renderer-generated mesh indices/instance indices may never substitute for structural part identity.

## Structural element classes

V0 should support at least semantic classes equivalent to:

- shoot/trunk segment;
- branch segment;
- root segment;
- leaf/leaf-cluster attachment;
- bud/growth tip;
- reproductive attachment;
- dead/senescent retained structure.

Exact enums may evolve through schema versioning.

## Minimum segment state

A woody/root segment should eventually retain enough canonical state for future biology and coarse mechanics, including concepts such as:

- parent structural coordinate;
- birth/growth tick;
- organ class;
- rest length;
- radius / cross-sectional structural measure;
- local orientation / growth direction;
- developmental age;
- tissue/wood allocation state;
- vitality / living-vs-dead state;
- damage state;
- stiffness/strength-driving biological parameters or qualified derived inputs;
- child attachment coordinates.

Do not include mesh vertices, GPU buffers, shader handles, Bevy entity IDs, or camera-dependent LOD state.

## Growth topology

Structural growth is monotonic in history even when material later dies or is removed.

Creating a segment is a canonical growth action. Removing/breaking/pruning a segment is a canonical structural event/transition, not vector deletion that erases the fact it existed.

Implementation may compact historical storage later, but any compaction must preserve process-required consequences/provenance.

## Geometry recipe

Presentation derives geometry from structural truth.

Conceptually:

```text
PlantStructure
+ phenotype/development state
+ presentation profile
+ fidelity tier
-> PlantGeometryRecipe
-> render resources
```

A geometry recipe may include derived branch curves, radii, leaf-instance transforms, bark coordinates, LOD clusters, etc.

It is rebuildable cache state.

Deleting/rebuilding every renderer object must leave canonical plant structure unchanged.

## Structural signatures / cache sharing

Many plants may share presentation caches only when the cache key includes every structural/presentation input needed to make the derived geometry equivalent.

Do not cache only by species/seed if drought, wind, damage or age have changed the structure.

## Roots and shoots

V0 treats below-ground and above-ground structure under the same authority principle while permitting distinct authored growth rules.

Root rendering may be absent or heavily simplified while root structural state remains canonical when it affects:

- water/nutrient access;
- stability/uprooting;
- mycelial association;
- excavation damage;
- competition;
- regrowth.

## Leaves

Millions of individual leaves need not all be canonical objects.

A plant may retain leaf attachments/clusters as a coarser sufficient representation while presentation materializes many leaves deterministically.

Individual leaf authority is only required when future-bearing processes actually depend on particular leaves.

This is the same information-relative fidelity principle used elsewhere in Living World.

## Damage

Damage must reference structural biology, not render triangles.

Examples:

```text
branch segment damaged
root segment severed
bud destroyed
bark/tissue wound on segment
primary limb removed
```

Presentation derives fracture surfaces, scars and missing geometry from this state.

A renderer rebuild therefore cannot heal a damaged tree.

## Coarse/fine representation

Large distant forests should not require full structural graphs for every exchangeable young plant if processes do not need them.

Possible fidelity path:

```text
population/stand state
    -> structural summary
    -> explicit plant structural graph
    -> detailed derived geometry
```

Promotion/demotion must follow the existing Living World information/collapse authority rules.

Landmark/ancient/damaged/player-interacted plants may remain exact structural individuals offscreen.

## Determinism

Structural growth decisions derive from authoritative biological state, deterministic developmental drivers, keyed heredity/phenotype rules, and canonical resource settlement.

No canonical part creation may depend on:

- renderer frame number;
- camera distance;
- unordered ECS iteration;
- sequential cosmetic RNG;
- mesh tessellation;
- GPU scheduling.

## First qualification fixture

One hereditary tree baseline under four histories:

```text
control
persistent drought
persistent crosswind
canopy competition / shade
```

The final structural graphs should differ in characteristic, replayable ways while retaining identical hereditary baseline provenance.

The renderer test comes later. The first proof is structural:

> can the graph itself explain why these trees differ?

## Required future tests

1. same biological inputs/history -> identical structural graph;
2. render rebuild -> no structural change;
3. renderer FPS/LOD -> no structural change;
4. every non-root element has one valid parent in V0;
5. no topology cycles in V0;
6. structural part coordinates remain stable after later growth;
7. pruning/damage remains after renderer destruction/recreation;
8. coarse collapse refuses when required structural history cannot be represented;
9. branch geometry can be regenerated from canonical structure;
10. four-history flora benchmark yields explainable structural divergence.

## Non-goals

V0 does not define:

- botanical species parameters;
- mesh algorithms;
- FEM biomechanics;
- photogrammetry assets;
- graft/anastomosis topology;
- every individual leaf;
- full soil/root chemistry.

It freezes the rule that **a persistent plant is a biological branching history from which geometry is derived, not a mesh whose current vertices happen to look tree-like**.
