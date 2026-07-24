# Symtropy Physics Research Validation Protocol

Symtropy Physics is intended to support open-source game development **and**
reproducible scientific research. Research use requires a stricter standard
than a visually plausible demo: every claim must identify its validity domain,
reference solution, error metric, build environment, and acceptance threshold.

The transformed-collision validity envelope and its remaining limitations are
documented separately in [`ORIENTED_COLLISION_VALIDATION.md`](ORIENTED_COLLISION_VALIDATION.md).

## Current validity envelope

This document describes the `0.2.x` research envelope. It is deliberately
narrower than the public API surface.

### Credible today

- Const-generic 2D, 3D, and selected 4D mathematical primitives.
- Convex support functions and transform-aware GJK intersection tests within
  the implemented simplex limit.
- 2D and 3D EPA penetration estimates for non-degenerate cases.
- Uniform-gravity translational dynamics using semi-implicit Euler.
- General bivector rotation increments through the `SO(D)` exponential map,
  including simultaneous independent 4D rotation planes.
- Deterministic replay studies within a fixed build and execution contract.
- Quantitative invariant snapshots through `PhysicsWorld::invariant_snapshot`.
- Full-transform broadphase bounds and narrowphase support queries for bounded
  convex colliders, plus dedicated 2D/3D oriented-box SAT.
- Exact transformed primitive ray queries for spheres, boxes, capsules, and
  half-spaces.

### Not yet certified

- Exact ray and shape queries for every compound, hull, mesh, and experimental
  collider type; unsupported ray shapes retain a bounding-sphere fallback.
- Rotational continuous collision detection and time-of-impact islands.
- Full clipped multi-point manifolds for rotated boxes and transformed meshes.
- Full body-space/world-space inertia tensors for asymmetric rigid bodies.
- Gyroscopic coupling and torque-free asymmetric-top dynamics.
- General convex continuous collision detection with rotational motion.
- Reduced-coordinate articulations and inverse dynamics.
- Production fluid, deformable, fracture, FEM, or MPM simulation.
- Cross-architecture bit-identical determinism.
- 4D EPA penetration depth beyond the documented approximation path.

Research papers and public benchmark reports must not imply certification of a
capability in the second list merely because a related type or experimental
module exists.

## Validation tiers

### Tier A — Local mathematical contract

A unit or property test verifies an identity or invariant with explicit
numerical tolerance. Examples:

- `RᵀR = I` and `det(R) = +1` for generated rotations.
- Support points maximize the dot product for known primitives.
- Antiparallel vector alignment maps the source vector to the target.
- Wedge products are antisymmetric.

### Tier B — Analytical physical solution

A simulation is compared with a closed-form or independently derived solution.
The report must include absolute and relative error versus timestep.

Required first suite:

1. Uniform-gravity free fall.
2. Constant-force acceleration.
3. Elastic two-body collision.
4. Harmonic oscillator.
5. Simple pendulum at small angle.
6. Torque-free spherical body rotation.
7. Rolling without slipping after rotational inertia closure.
8. Two-body Kepler orbit after the orbital solver is isolated.

The included `free_fall_validation` example is the reference format.

### Tier C — Independent implementation comparison

The same initial conditions are evaluated in at least one independent engine or
reference implementation. Compare physical error at matched tolerances rather
than comparing frame rate alone.

Suitable references depend on the domain:

- Jolt, Box2D, Rapier, or PhysX for game rigid bodies.
- MuJoCo for robotics and constrained dynamics.
- A symbolic or high-precision implementation for mathematical identities.
- Published benchmark datasets for fluids, FEM, and deformables.

### Tier D — Reproducible campaign

A complete campaign includes:

- Immutable source revision.
- Machine-readable scenario configuration.
- Raw result files.
- Analysis script or notebook.
- Environment metadata.
- Expected tolerances and pass/fail rules declared before the run.
- A short limitations statement.

A result that cannot be regenerated from a clean checkout is exploratory, not
research evidence.

## Required run metadata

Every archived result should record at least:

| Field | Example |
|---|---|
| source revision | full Git commit hash |
| crate version | `symtropy-physics 0.2.1` |
| Rust compiler | complete `rustc -Vv` output |
| target | `x86_64-unknown-linux-gnu` |
| build profile | debug or release |
| enabled features | including `deterministic-net` |
| operating system | name and version |
| CPU | model and architecture |
| floating-point contract | FMA, denormals, fast-math settings |
| dimension | 2, 3, or 4 |
| timestep | seconds |
| number of steps | integer |
| solver iterations | integer |
| gravity | complete vector |
| seed | where stochastic behavior exists |
| thread count | integer |
| scenario hash | hash of canonical scenario input |

## Core metrics

At minimum, report the metrics relevant to the scenario:

- Position and velocity error against reference.
- Linear momentum drift.
- Angular momentum drift once full inertia is implemented.
- Mechanical-energy drift.
- Constraint residual and joint drift.
- Maximum penetration depth.
- Rotation orthogonality error `max_abs(RᵀR - I)`.
- Rotation determinant error `|det(R) - 1|`.
- Count of non-finite bodies.
- Runtime, allocations, and peak memory for performance studies.

Do not hide unstable runs by averaging only successful trials. Publish failure
counts and the complete distribution.

## Convergence requirement

A numerical method should be tested at no fewer than four timesteps. Reports
must show whether error decreases at the expected order as `dt` is reduced.

For the current semi-implicit Euler translational integrator, position error is
expected to converge at first order in timestep for constant acceleration.
Failure to show convergence is more important than one attractive result at a
single timestep.

## Conservation snapshots

`PhysicsWorld::invariant_snapshot()` measures:

- Dynamic mass and center of mass.
- Linear momentum.
- Kinetic energy under the currently implemented inertia model.
- Uniform-gravity potential energy.
- Mechanical energy.
- Maximum speeds and contact penetration.
- Rotation-group numerical health.
- Non-finite body state.

Interpret these values according to the modeled system. Dynamic-body momentum
is not expected to remain constant when bodies exchange impulse with static
geometry, external fields, actuators, callbacks, or omitted environment
reservoirs.

## Research contribution checklist

A research-oriented pull request should answer:

1. What physical or mathematical claim is being tested?
2. What is the validity domain?
3. What independent reference is used?
4. Which error metric and tolerance define success?
5. Does error converge as resolution increases?
6. Which conservation laws should hold in this scenario?
7. Are all interventions and energy transfers explicit?
8. Can the result be reproduced from a clean checkout?
9. Are negative and pathological cases included?
10. What remains unproven after this change?

## Priority validation backlog

The next high-value campaigns are:

1. Independent transformed-collider comparison and adversarial orientation corpus.
2. General convex GJK/EPA degeneracy corpus.
3. Persistent contact feature and stack stability study.
4. Full 3D inertia and asymmetric-top benchmark.
5. General convex CCD and tunneling corpus.
6. Cross-architecture deterministic replay matrix.
7. Reduced-coordinate articulation comparison with MuJoCo.
8. One completed fluid method with hydrostatic and dam-break references.

The purpose of this protocol is not to slow experimentation. It allows
experimental work to remain ambitious while keeping published claims precise,
auditable, and useful to other researchers.
