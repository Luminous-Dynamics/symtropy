# CCD Qualification Corpus v0

This corpus qualifies the analytical continuous-collision queries in
`symtropy-physics::ccd`. It separates mathematical correctness, robustness,
world integration, and throughput. Passing the unit tests does not establish
that the full physics world prevents all tunnelling.

## Exact domain covered

The current checked kernels solve two constant-linear-velocity problems:

1. Sphere vs sphere in any positive dimension.
2. Sphere vs infinite halfspace with a normalized plane normal.

They do not yet cover rotational sweeps, accelerating trajectories, arbitrary
convex pairs, deformable geometry, or general conservative advancement. These
remain explicit non-goals for this first qualification slice.

All physical values use SI:
- position/radius/plane offset: metres;
- velocity: metres per second;
- time of impact and timestep: seconds;
- normal: dimensionless unit vector.

The result field `toi` is absolute seconds from the beginning of the sweep,
bounded by `[0, dt]`. It is not a dimensionless fraction.

## Numerical formulation

For sphere-sphere motion, use relative position `p`, relative velocity `v`,
combined radius `r`, and time of closest approach

```text
t_c = -(p · v) / |v|²
p_c = p + v t_c
d_c = |p_c|
h = sqrt(((r - d_c) (r + d_c)) / |v|²)
t_enter = t_c - h
t_exit  = t_c + h
```

This avoids the direct subtraction in the standard quadratic discriminant
`b² - 4ac`, which can lose the small radius-dependent term when travel
distance is very large. The implementation uses a scaled/hypot vector norm
and checks every intermediate for non-finite overflow. The analytic formula
is exact for the declared linear-motion model, subject to floating-point
roundoff.

For sphere-halfspace motion, normalize the supplied plane normal and scale
the plane offset by the same norm before evaluating signed distance and
crossing time. This makes the query invariant to positive scaling of the
normal vector.

## Correctness controls

The corpus includes:

- Head-on sphere-sphere exact TOI.
- Large-travel/small-radius conditioning case (1e12 m range, 8 m radius).
- Grazing/tangent contact.
- Perpendicular miss and zero-relative-motion miss.
- Initial overlap and initial plane penetration.
- Plane-normal scaling.
- Negative radius, negative/non-finite timestep, zero plane normal and
  non-finite plane offset rejection.
- Symmetry under swapping sphere A/B (same TOI, opposite normal).
- Galilean invariance under adding the same constant velocity to both bodies.
- Normal normalization and finite hit-point checks.

The checked `try_*` APIs return typed invalid-input errors. Compatibility
wrappers preserve the old `Option` signature and map invalid inputs to None;
new code should use the checked APIs.

## Integration controls still required

The analytic query tests are not enough to establish collision response.
Future integration fixtures must exercise PhysicsWorld from initial state
through event generation and response:

- Fast sphere vs thin static wall, with and without CCD enabled.
- Fast sphere vs sphere, both moving and with unequal radii.
- Restitution and momentum/energy accounting for dynamic-dynamic impacts.
- Static/kinematic wall as an external momentum/energy reservoir.
- Large rotation of a long thin body against a small obstacle.
- Multiple impacts in one frame and impact ordering.
- Repeated TOI advancement with bounded step count and explicit stall/
  inconclusive outcomes.
- CCD on/off paired runs with final-position overlap and missed-event
  negative controls.

Do not require dynamic-body momentum conservation for a collision against a
static wall: the wall is an external reservoir. Use the correct declared
boundary in each balance fixture.

## Industry comparison protocol

### Box2D 3.1

Box2D exposes time-of-impact and shape-cast primitives. Its documentation
describes TOI as a tunnel-prevention method using a swept separating axis; it
also warns that TOI may miss some intermediate or final-overlap situations.
With fixed rotations, its translational shape-cast/TOI result is a stronger
comparison for the current linear sphere cases. Use equivalent geometry,
units, sweep interval and output semantics; retain Box2D's state/result code
rather than assuming every no-hit means a certified separated state.

References:
- https://box2d.org/documentation/md_collision.html
- https://box2d.org/documentation/group__distance.html

### Rapier 0.35.3

Use the pinned stable Rust-engine version for equivalent shape-cast/CCD
scenarios. Keep its broadphase/narrowphase, solver and timestep configuration
in each result receipt. Do not attribute the separately reported Q3 2026
parallel CCD/solver development architecture to the 0.35.3 stable release
without pinning the exact development commit.

Reference:
- https://github.com/dimforge/rapier/blob/master/CHANGELOG.md

### Bullet

Use linear swept-sphere and conservative-advancement tests for the equivalent
motion model. Bullet's classic CCD paper formulates TOI as a lower bound and
uses conservative advancement; this is a useful reference for the next
general-convex stage, not a direct apples-to-apples comparison with this
analytic-only kernel.

Reference:
- https://www.gamedevs.org/uploads/continuous-collision-detection-and-physics.pdf

### MuJoCo

Use MuJoCo 3.17.0 for deterministic batch/replay comparisons, not as the
sole CCD oracle. Its batch API guarantees results bit-identical to a serial
loop at any thread count, which is a useful determinism standard independent
of the TOI algorithm.

References:
- https://mujoco.readthedocs.io/en/3.17.0/changelog.html
- https://mujoco.readthedocs.io/en/3.17.0/programming/simulation.html

## Performance measurement

The Criterion `ccd_bench` measures 512 frozen sphere-sphere and 512
sphere-halfspace queries per iteration. It is a baseline for this Rust kernel,
not a cross-engine result. A valid external comparison must use:
- identical frozen trajectories and shape geometry;
- the same SI units and sweep interval;
- fixed rotation when comparing a translational TOI;
- the same hit/miss semantics and explicit status mapping;
- warm-up and repeated measurement windows;
- query throughput and latency distribution;
- exact source/build/CPU/compiler and competitor version.

No performance numbers or superiority claim are currently reported.

## Improvement targets

Programme targets, not achieved results:

- 100% agreement with analytical hit/miss and TOI fixtures within the
  declared numerical tolerance.
- No false negatives for the exact constant-velocity sphere/sphere and
  sphere/halfspace domains.
- Invalid geometry is distinguishable from a valid miss in checked APIs.
- Unit normals within 1e-12 in the current analytic domain.
- Large-range/small-radius tests remain within the declared absolute and
  relative TOI error.
- Every world-level CCD integration test preserves event identity and the
  selected system boundary for momentum/energy accounting.
- General convex CCD returns a certified TOI interval or an explicit
  inconclusive result; it must not silently turn iteration exhaustion into
  no-hit.

## Paradigm shift: certified TOI intervals

The next major step should be a general shape-cast API returning more than a
single floating-point time:
- a proven separated interval before `t_lower`;
- a contact/overlap witness by `t_upper`;
- a numerical error bound and convergence status;
- the exact shape/sweep/configuration identity.

The interval width is itself a useful precision/latency control. A caller may
choose tolerance, while safety-critical use can require a tighter bracket.
When the algorithm cannot establish separation or contact, it returns
Inconclusive rather than a fabricated no-hit. This is a stronger trust model
than accepting one approximate TOI scalar.
