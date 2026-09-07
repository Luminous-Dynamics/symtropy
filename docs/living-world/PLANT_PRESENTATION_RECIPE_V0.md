# Plant Presentation Recipe V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define the one-way boundary from canonical plant biology into renderable geometry/animation/material inputs without allowing render structures to become organism authority.

## Core invariant

> Presentation is a rebuildable projection of canonical plant structure and biological state.

Destroying every mesh/material/instance associated with a plant must not destroy or alter the plant itself.

## Conceptual recipe

A future `PlantPresentationRecipe` may contain derived data such as:

- stable source plant/structure revision reference;
- structural segment hierarchy;
- rest centerlines / radii / taper information;
- branch depth and attachment coordinates;
- foliage attachment clusters;
- organ age/state classes;
- phenological visibility/state;
- damage/deadwood/scar descriptors;
- material-state channels;
- structural stiffness / wind-response metadata;
- geometry/instance LOD hints.

It does **not** own canonical growth, damage, resource, phenology, or identity state.

## Structural geometry

Persistent biological segments should be converted into render geometry through a versioned recipe/mesher.

Changing:

- radial tessellation;
- branch surface reconstruction;
- leaf-card/mesh technique;
- Nanite-like/meshlet representation;
- impostor technology;
- shader implementation

must not change canonical plant topology or save state.

## Stable attachment coordinates

Leaves, scars, fungi, buds, fruit, moss, and similar presentation may reference stable biological attachment coordinates rather than transient triangle indices.

A topology revision invalidates/rebuilds affected presentation references deterministically.

## Wind metadata

The recipe may expose derived branch hierarchy, rest axis, stiffness, mass proxy, attachment position, and phase seeds to presentation wind.

High-frequency presentation deformation remains non-authoritative. Canonical breakage/mechanomorphogenesis uses qualified mechanical authority from the plant/wind contracts.

## Geometry caching

Geometry may be cached/shared when canonical morphology signatures prove the relevant geometry inputs equivalent.

Do not merge two plants into one biological state because they happen to share a mesh cache.

Likewise, damage/age/phenology-specific differences must invalidate only the presentation layers they actually affect.

## Deterministic source revision

A recipe should bind to the exact canonical source revision/schema it was derived from.

A renderer may display a stale recipe briefly under explicit presentation policy, but it may never send stale recipe state back into ecology or use it as canonical targeting without the corresponding realization/authority validation.

## Headless equivalence

Canonical simulation with presentation disabled must produce identical ecological state to simulation with the recipe/render pipeline enabled, subject only to explicitly non-authoritative observability cost.

## Qualification direction

At minimum test:

1. same canonical plant revision -> deterministic presentation recipe for a fixed recipe version;
2. renderer teardown/rebuild leaves canonical plant hashes/state unchanged;
3. changing meshing tessellation changes presentation only;
4. topology damage invalidates affected recipe structure but cannot heal/mutate biology;
5. stable attachment locations survive harmless remeshing;
6. recipe generation does not consume random sequential state used by ecology;
7. headless and rendered canonical ecology remain identical;
8. cached geometry sharing cannot merge organism identity/history.

## Non-goals

This contract does not choose a final meshing algorithm, leaf representation, GPU API, shader language, or art style.