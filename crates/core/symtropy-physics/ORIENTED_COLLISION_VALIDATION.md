# Orientation-Aware Collision Validation

This document defines the research validity envelope introduced by the
transformed-collision series. It complements `RESEARCH_VALIDATION.md` and
prevents “orientation-aware” from being interpreted more broadly than the
implemented evidence supports.

## Coordinate contract

All collider geometry is stored in body-local coordinates. Canonical collision
queries operate through `TransformedShape`, which applies the complete rigid
transform:

1. rotate the world query direction into local coordinates with `R^T`;
2. evaluate the local support function;
3. rotate and translate the support point back into world coordinates.

GJK, EPA, manifold witness generation, broadphase bounds, mesh callbacks, and
primitive ray queries now use this contract. Translation-only entry points are
retained as compatibility wrappers and deliberately construct identity
rotations.

## Implemented and directly tested

- Exact support-derived world AABBs for bounded convex shapes.
- Transform-aware GJK for bounded convex shapes in the dimensions supported by
  the existing simplex implementation.
- Transform-aware EPA and witness-based manifold generation.
- Exact transformed sphere, capsule, box, and generic support-map contacts
  against translated or rotated half-spaces.
- Dedicated 2D/3D oriented-box SAT using both boxes’ face axes and, in 3D,
  edge-cross-edge axes.
- Transform-aware mesh broadphase mapping and mesh narrowphase callbacks.
- Exact local-space ray queries for spheres, hyperboxes, capsules, and
  half-spaces.
- Static/kinematic broadphase cache invalidation when a body is exposed through
  `PhysicsWorld::body_mut`.

Run the compact public-API validation corpus with:

```bash
cargo run --release --example oriented_collision_validation
```

The output is CSV and the process exits unsuccessfully if a declared result
exceeds its tolerance.

## Current limitations

- Rotated 2D/3D OBB SAT currently produces a stable minimum-translation contact
  but not a full clipped multi-point face manifold. Identity-oriented boxes
  retain the existing clipped contact patch.
- Rotated 4D hyperboxes use the generic transformed GJK/EPA path; there is no
  dedicated 4D separating-axis implementation.
- Ray queries for shape kinds without an exact primitive implementation still
  use a transformed bounding-sphere fallback and may produce false positives.
- Continuous collision detection remains sphere-focused and does not yet
  account for rotational sweeps.
- Mesh contacts currently depend on the downstream mesh integration and should
  be independently validated against transformed triangle fixtures.
- Unbounded half-spaces use the existing finite support approximation when they
  reach generic support-based broadphase code. The analytical half-space
  narrowphase is authoritative when exactly one collider is a half-space.
- Scale-aware tolerance policy and a large degeneracy corpus are still pending.

## Required external validation campaign

A publishable orientation campaign should contain at least:

1. Random convex support points compared with brute-force transformed vertices.
2. Random 2D OBB pairs compared with polygon clipping.
3. Random 3D OBB pairs compared with an independent SAT implementation.
4. Rotated convex pairs compared with Jolt, Rapier, or another independent
   engine using matched geometry and tolerances.
5. Grazing, coincident-center, parallel-edge, near-zero-scale, and very-large-
   coordinate adversarial cases.
6. Ray and shape-query comparisons against analytical primitive solutions.
7. Replays across timestep, optimization profile, and supported architecture.

For every run, archive the raw scenario, transform matrices, observed contact
normal and depth, independent reference result, tolerance, compiler metadata,
and source revision.

## Acceptance metrics

At minimum, report:

- intersection classification agreement;
- false-positive and false-negative counts;
- absolute and relative penetration-depth error;
- angular error in contact normals;
- witness-point separation error;
- AABB containment failures and excess-volume ratio;
- GJK/EPA iteration distributions and failure counts;
- runtime distributions separated by shape pair and dimension.

No failed or non-convergent case should be discarded from the published
aggregate. A known failure belongs in the corpus with a stable identifier so a
future patch can prove that it was closed without regressing earlier cases.
