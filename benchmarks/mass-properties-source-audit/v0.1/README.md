# Rigid-body mass-property source audit M0 v0.1

**Tracking issue:** [PHYS-COMPOUND-MASS-PROPERTIES-001 #6417](https://github.com/Luminous-Dynamics/symtropy/issues/6417)  
**Engine-wide follow-on:** [PHYS-ROTATIONAL-RESPONSE-PORTS-001 #6443](https://github.com/Luminous-Dynamics/symtropy/issues/6443)  
**Related hinge/motor scope:** [#4516](https://github.com/Luminous-Dynamics/symtropy/issues/4516)  
**Source snapshot:** `main` at `b93beb5f68f6fa0bb0c5d4f58d3afb79a19cb68e` (2026-11-20)

**Status:** M0 source audit from exact public repository files. It is not a production fix, compile/test result, physical measurement or safety qualification.

## Why this audit exists

PRs [#6416](https://github.com/Luminous-Dynamics/symtropy/pull/6416) and [#6418](https://github.com/Luminous-Dynamics/symtropy/pull/6418) are standalone analytical references. They do not change how the production engine computes angular acceleration, contact effective mass, friction or joint response. The source audit exposes those integration points so an M2 migration does not fix one path while leaving other paths physically inconsistent.

## Source-bound findings

The machine-readable `source_inventory_v0.1.json` records the baseline commit, source blob SHAs, line ranges, status, effect and promotion gate. The inventory intentionally distinguishes observed source facts from interpretation.

### 1. Construction and storage

- `RigidBody::new` accepts `SVector<f64, D>` inertia values and computes an elementwise reciprocal. The API does not declare whether the vector is principal moments, a frame, or another approximation; it does not validate finite/positive realizability.
- All `RigidBody` fields are public, including `mass`, `inv_mass`, `inertia`, and `inv_inertia`.
- `PhysicsWorld::bodies` is also public, and `body_mut` returns an unrestricted mutable body reference. These are invariant escape hatches: a caller can change mass/inertia without updating the corresponding inverse values.
- `add_body` and `add_bodies_deterministic` accept existing bodies and insert them without validating those invariants.
- `RigidBody` itself does not derive serde serialization/deserialization in the audited source. This is not proof that no external wrapper or persisted representation exists elsewhere in the monorepo.

### 2. Existing mass-property behavior

- `dynamic_sphere` uses `0.4*m*r²` for every supported dimension. That is the familiar 3D solid-sphere moment, not a general N-dimensional ball formula.
- `kinetic_energy` replaces the inertia vector by its arithmetic mean.
- `merge_toolhead` averages the base inertia, assumes a 0.1 m solid-sphere toolhead, performs a scalar distance correction, and assigns the same result to every axis. It cannot preserve component shape, orientation or products of inertia.
- The 3D `ArticulatedChain` builder gives each link a sphere mass property even though `LinkSpec` has length and radius. This is a model/geometry mismatch, not evidence that the physical link is spherical.

### 3. Runtime response paths

- `integrator::integrate` applies mean inverse inertia to every bivector plane when integrating accumulated torque.
- `integrator::apply_angular_impulse` applies the same scalar mean to angular impulses.
- `PhysicsWorld::resolve_contact` uses that same mean approximation in the normal and tangential friction effective-mass denominators. The actual angular impulses are then routed through `apply_angular_impulse`.
- Hinge and prismatic passive constraint paths directly adjust angular-velocity components. Their Jacobian/effective-mass and action/reaction treatment must be covered by the engine-wide follow-on, not inferred correct from the motor fix.
- PR #6411 improves step-local motor impulse accounting, but explicitly retains the current isotropic mean-inertia approximation for hinge response pending a full operator.

## Paradigm shift required

Move from a vector of vaguely defined per-axis values to one frame-explicit mass-property source of truth:

1. Store a full symmetric second-moment tensor about the center of mass (or a rigorously equivalent typed representation).
2. Preserve local/body frame, units, mass, COM, shape/model class and provenance.
3. Derive world-frame angular response from the current orientation.
4. Derive contact/joint effective mass from the same operator through `J M⁻¹ Jᵀ`.
5. Compute kinetic energy, impulse response and work from one consistent operator.
6. Keep a named legacy-isotropic path for existing sphere-like bodies; never silently use it for anisotropic shapes.
7. Make construction and mutation invariant-preserving. Public writable inverse caches must not be independent sources of truth.

This is a contract and migration direction, not permission to immediately replace the production data model. PR #6418's dense bivector operator is a reference oracle, not yet a hot-loop implementation.

## Required next gates

- **M0 — source inventory:** this artifact; extend to all external call sites and any persistence/wire schemas before migration.
- **M1 — independent oracle:** analytic shape fixtures, full tensor composition, rotated-frame covariance, and Jacobian effective-mass references.
- **M2 — versioned API:** explicit units/frames, derived inverse operator, constructor validation, compatibility and migration plan.
- **M3 — integration:** torque integration and angular impulse path use the same frame-aware response.
- **M4 — solver ports:** contact normal, friction, hinge, prismatic and other angular constraints use the same effective mass.
- **M5 — external comparison:** pinned MuJoCo/Bullet/PhysX or another reference, matched geometry/material/contact/time-step/iteration budget.
- **M6 — physical validation:** instrumented article measurements and uncertainty; engine tests alone do not qualify hardware.

## Benchmark vector

Do not collapse correctness, stability and speed into one score. Report:

- relative angular-velocity and angular-impulse error against analytic answers;
- effective-mass and constraint residual by port;
- linear/angular momentum balance residual;
- energy/work residual with actuator, contact, damping and numerical drift separated;
- rotated-frame covariance residual;
- error versus timestep and solver iterations;
- runtime and memory versus body/contact count and dimension;
- negative-control detection and false-admission rate.

Analytic fixture agreement and negative controls should be binary gates. Numerical tolerances for measured physical articles must derive from sensor/calibration uncertainty and task consequences, not from a post-hoc chosen threshold.

## Claim boundary

This inventory establishes what the inspected files do at the pinned source revision. It does not establish what the engine does on real hardware, prove any performance claim, or qualify any humanoid, exoskeleton or human-contact use.