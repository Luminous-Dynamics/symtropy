# Analytic CCD qualification corpus v0

This campaign qualifies the analytical sphere/sphere and sphere/halfspace
continuous-collision queries. It is not a claim that the full world prevents
all tunnelling: the current world CCD pass still has a speed threshold, only
sweeps sphere-moving bodies against sphere/halfspace targets, and does not yet
account for angular sweep.

## P0 implementation

The checked APIs are:

- `sphere_sphere_checked(...)`
- `sphere_halfspace_checked(...)`

They return `Result<Option<CcdHit>, CcdQueryError>` so valid no-hit, invalid
input and numerical failure are separate states. Compatibility wrappers keep
the old `Option` signatures; invalid input maps to `None` only for those
legacy callers.

The world integration records each failed query in
`PhysicsWorld::ccd_query_diagnostics`, including moving/target body handles,
primitive kind and error. This is per-step diagnostic evidence; it does not
yet make `PhysicsWorld::step` return a global error.

## Mathematical contract

Sphere/sphere uses relative motion
`|dp + dv*t| = ra + rb`, but solves in normalized time `tau=t/dt` and
normalizes geometry/swept displacement before forming the quadratic. The root
computation uses the stable `q` formulation; near-tangent negative
discriminants inside a scale-relative roundoff bound are treated as tangency.
The result is the earliest root in the closed interval `[0, dt]`.

Sphere/halfspace uses the plane equation `n dot x = offset`. It normalizes
`n` and `offset` together, so a non-unit normal describes the same plane.
A zero normal, negative radius, negative/non-finite timestep or non-finite
geometry/motion is reported explicitly.

The primitive tests cover head-on contact, misses, overlap, halfspace impact,
moving away, contact exactly at the end of the interval, tangency, non-unit
plane normals, scale variants, near-cancellation, invalid inputs and
non-finite inputs. A world-level test verifies that an invalid query appears in
the world diagnostic collection instead of silently falling through as a miss.

## Frozen fixture set

See `manifests/ccd-analytic-v0.json`. The set specifies expected TOI and
acceptance rules independently of timing. It includes:

- head-on sphere/sphere;
- 2D tangent and grazing contact;
- moving-apart and no-hit controls;
- already-overlapping at t=0;
- endpoint impact at t=dt;
- sphere/plane with unit and non-unit plane normal;
- scale variants at micrometre and megametre geometry;
- near-cancellation and invalid-input negative controls.

All expected times are in seconds. Tolerances are specified per fixture and
must not be loosened after observing a candidate failure.

## Performance methodology

Criterion compares the checked and compatibility entry points on the same
fixture shapes and includes scale/tangent cases. It measures query-function
cost only, not the full broadphase, solver, body integration or tunnelling
outcome. A speed regression is acceptable only when explained by the stronger
validation; the primary acceptance is no false negatives/invalid outputs on
the corpus.

## P1 beyond this tranche

A separate programme stage must add:
- angular sweep and shape-specific sweep radius;
- conservative swept AABBs;
- conservative advancement for supported convex shape pairs;
- speculative contacts and solver coupling;
- bounded substeps / motion clamping;
- occlusion rejection for speculative witness contacts;
- engine-level thin-wall and rotating-rod adversarial scenarios.

External comparators:
- Rapier 0.35.3 supports nonlinear CCD for angular and translational motion,
  with configurable maximum CCD substeps.
- MuJoCo's October 2026 speculative contacts sweep geom motion, expand
  broadphase bounds, use conservative advancement/GJK for normalized TOI, and
  reject occluded contacts with a witness-point raycast.
- Box2D documents TOI plus solver substeps and speculative contacts in 2D.

No SOTA or full-engine tunnelling-prevention claim follows from P0.
