# Optional Shape-Local AABB Contract

**Status:** additive API / staged research contract  
**Related:** [#7023 — finite, outward-rounded broadphase bounds](https://github.com/Luminous-Dynamics/symtropy/issues/7023), [#7104 — outward interval arithmetic](https://github.com/Luminous-Dynamics/symtropy/pull/7104), [#7146 — compound support mid-phase](https://github.com/Luminous-Dynamics/symtropy/issues/7146)

## Purpose

The shape layer now has an optional `Shape::local_aabb() -> Option<LocalAabb<D>>` method.

The default is `None`. This is intentional: an arbitrary third-party shape, an unbounded shape, or a shape whose bound formula is not qualified must remain unprunable. A consumer must not substitute the existing `bounding_sphere()` method or treat missing bounds as an empty box.

A returned `Some(LocalAabb)` is a promise that the represented local-space shape is contained by that finite, ordered box. The API is additive and does not change GJK, EPA, contact generation, broadphase, or solver behavior.

## Current built-in providers

| Shape | Provider | Basis | Invalid / unsupported behavior |
|---|---|---|---|
| HyperBox | Yes | Exact represented non-negative half-extents | Non-finite or negative extent returns `None` |
| Sphere | Yes | Center ± non-negative radius, with one representable step outward per rounded endpoint | Negative/non-finite radius, non-finite center, or outward overflow returns `None` |
| Capsule | Yes | Axis extent = half-height + radius; other axes = radius; endpoints widened outward | Invalid axis/dimensions or overflow returns `None` |
| ConvexHull | Yes | Coordinate extrema over the represented vertices; no arithmetic is required | Empty or non-finite point set returns `None` |
| HalfSpace | No | Unbounded | Returns `None` |
| CompoundShape | No | Child transforms and enclosure propagation are not yet qualified | Returns `None` |

## Numerical assumptions

The sphere and capsule constructors perform one binary64 addition or subtraction to form each endpoint and then move one representable value outward. This is conservative under the documented IEEE-754 round-to-nearest binary64 profile for those single operations. If the result or outward neighbor is non-finite, the provider returns `None`.

HyperBox endpoints are the represented half-extents themselves with exact sign changes. ConvexHull endpoints are selected from the represented vertices by comparisons only.

This is a *local geometry* contract. It does not prove that a world transform, rotor construction/reversal, matrix-vector product, quantization, AABB union, or BVH traversal preserves the enclosure. Those are separate stages in #7023. The affine interval helper in #7104 handles an exact represented matrix and translation, but does not yet prove that the represented matrix encloses a higher-level Rotor computation.

## Consumer rules

A consumer may use a local AABB only when it has a qualified transform/enclosure path for the provider and arithmetic profile.

- `Some(bounds)`: proceed only through a qualified outward transform and conservative propagation path.
- `None`: mark the shape unknown/unprunable or use the existing exhaustive path.
- Invalid transformed endpoints, overflow, reversed intervals, unsupported child shapes, or missing evidence must never become a separation decision.
- Do not use this local AABB API as a substitute for CCD/swept bounds; current-pose bounds do not prove swept candidate completeness.

## Tests

The shape-layer tests exercise exact hyperbox bounds, outward sphere/capsule endpoints, vertex-extrema bounds for convex hulls, support-point containment on coordinate directions, and invalid public-field mutations. These tests qualify only the declared local providers; they do not qualify transformed world bounds or the production broadphase.

## Next steps

1. Connect the local contract to the affine interval enclosure under #7104.
2. Independently qualify each supported local provider and transformed provider in the canonical #460 corpus.
3. Preserve explicit unknown/unprunable handling through world unions, Morton encoding, static caches, and LBVH nodes.
4. Only then consider enabling these bounds for production pruning or the compound hierarchy in #7146.
