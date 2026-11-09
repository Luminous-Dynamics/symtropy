# Symtropy

**Symtropy is a Rust physics and simulation substrate for games, robotics research and engineering scenarios.** It combines const-generic geometric mathematics, rigid-body dynamics, collision and constraint machinery, replay primitives, and adapters for higher-level systems.

The project also contains an optional consciousness-physics research layer. That layer is an experiment built on the physics core; it is not a prerequisite for ordinary physics use, a theory of robot safety, or evidence that a simulated system is conscious.

> **Maturity and claim boundary:** Symtropy is an actively developed research and software-engineering project. A working API, passing unit test, deterministic replay or successful simulation does not by itself establish physical-model validity, product qualification, functional safety, environmental protection or production readiness. Use the validation envelopes and evidence classes linked below.

## What Symtropy owns

| Layer | Responsibility |
|---|---|
| `symtropy-math` | Const-generic geometric primitives, transforms, rotors, bivectors and support-mapped shapes |
| `symtropy-physics` | Rigid-body stepping, collision detection, contact/constraint handling, joints, queries, events and replay primitives |
| `symtropy-bevy-core` | Bevy integration for the generic physics core |
| `symtropy-robotics-bridge-core` | Dependency-light platform and agent interfaces, motor-planning abstractions and a simple per-joint attenuation helper |
| `symtropy-robotics-bridge` | AGPL integration with Symthaea/Bevy and research embodiments |
| `symtropy-consciousness-physics` | Optional AGPL research coupling for Φ, energy bookkeeping, harmony fields and related experiments |

The math and physics crates are const-generic and have active 2D/3D/4D research and test paths. Generic type support must not be confused with equal qualification of every shape pair, dimension, collision mode or solver path.

## What Symtropy does not own

Symtropy is not an all-in-one CAD, FEA, EDA, CAM, MES or product-lifecycle-management system. It should connect those specialist systems and preserve their revision-bound outputs—not replace them.

| System | Authoritative responsibility |
|---|---|
| **Symthaea** | Cognitive reasoning, planning, perception, belief and human interaction; proposes typed intent, not unrestricted actuator commands |
| **Symtropy** | Physics, mechanics, dynamics and reproducible scenario execution within declared model capabilities |
| **Sol-Atlas** | Spatial intelligence, geometry, reference frames, scene/map relationships, localization and spatial provenance |
| **Mycelix** | Identity, permissions, custody/provenance, attestations and governance |
| **Luminous engineering platform** | Requirements, candidate selection, configuration baselines, EBOM/MBOM/service BOM, technical documents, engineering changes, production and as-built/service records |

The **target** Luminous engineering digital-thread layer is intended to become the product-lifecycle authority; this Symtropy repository does not implement a complete PLM system. The current Luminous workspace contains engineering, digital-twin, simulation and fabrication components, but still needs a unified product identity/configuration model, controlled product structures, engineering changes/effectivity, and as-built/service configuration. Symtropy supplies simulation execution and model outputs. A result is evidence only within its declared model, solver, configuration, uncertainty and context of use.

## Current physics capabilities and known limits

The physics core includes:
- Const-generic geometric and rigid-body types.
- GJK/EPA collision machinery and transformed-shape paths.
- Contact resolution, friction, sleeping and constraint/joint primitives.
- Ray/query support, collision events and replay/snapshot primitives.
- A `PhysicsCallback` extension point for explicitly selected custom force, impulse, friction and observation coupling.

The public validation notes are intentionally narrower than a general-purpose simulator claim:

- [Physics research validity envelope](crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [Orientation-aware collision validation](crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md)
- [Engine-wide physics qualification corpus #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)
- [Exact engine capability/model-path census #1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472)

Known gaps include limitations in general convex/rotational CCD, asymmetric rigid-body inertia and gyroscopic behavior, full contact manifolds for some rotated geometry, mesh-path validation, reduced-coordinate multibody dynamics, and cross-architecture bitwise deterministic guarantees. The exact gap depends on the requested operation and configuration; consult the validation notes and capability-census work before selecting Symtropy as the authority for an engineering question.

**Do not describe Symtropy as a MuJoCo, Drake, Newton, PhysX or FEA replacement.** For articulated robot dynamics, compare against mature independent solvers such as MuJoCo and Newton. Use specialist FEA/thermal/EM tools for structural, thermal and electrical analyses. Symtropy's opportunity is to compose these sources, identify the model path actually used, and preserve reproducible evidence across them.

## Robotics: honest maturity boundary

The robotics bridge is an integration and research surface, not a complete robot-control stack.

- `UniformGainPlanner` is a simple scalar baseline, not a per-joint controller.
- `spawn_robot_body` creates a coarse physics proxy; it is not an articulated humanoid assembly.
- The current humanoid demo's controller is explicitly a PD neutral-pose balance demonstrator with 21 actuators receiving a uniform attenuation gain. It is not a walking, manipulation or whole-body locomotion controller.
- A per-joint gain helper is not an independent, safety-rated safety function. Zeroing torque is not a universal safe state; each physical platform needs an independently engineered safety authority and defined fault response.

The target authority split is cognition → typed, expiring intent → embodiment adapter → platform-specific local controller → independent safety supervision → actuator/hardware or qualified simulation plant → explicit outcome returned to cognition.

Existing work to build on:
- [SYMPTHAEA-CONTROL-BOUNDARY-001 #2828](https://github.com/Luminous-Dynamics/symtropy/issues/2828)
- [Typed embodiment contract PR #2829](https://github.com/Luminous-Dynamics/symtropy/pull/2829) and the follow-on assurance/dispatch/outcome work
- [Matched locomotion benchmark #3357](https://github.com/Luminous-Dynamics/symtropy/issues/3357)
- [Multimodal failover and recoverability #4332](https://github.com/Luminous-Dynamics/symtropy/issues/4332)
- [Actuator electrical/mechanical loss maps #4179](https://github.com/Luminous-Dynamics/symtropy/issues/4179)
- [Repairable humanoid rotary-joint demonstrator #4337](https://github.com/Luminous-Dynamics/symtropy/issues/4337)

## Evidence-driven engineering workflow

Make simulation a falsifiable, configuration-bound engineering process:

`requirement → candidate → controlled configuration → model admission → simulation → prototype → physical measurement → correlation → release → field/service evidence`

Each run should identify:
- Exact source/tree and scenario/configuration identity.
- Geometry, mass/inertia, materials, boundary conditions and component revisions.
- Solver/backend, timestep, substeps, tolerances and numerical settings.
- Supported, approximated, unknown and unsupported model semantics.
- Requested observables, measurement/model uncertainty and acceptance rules.
- Raw input/output digests, warnings, failures and result classification.

A simulation is not promoted to physical qualification by a green dashboard. Synthetic, analytical, differential, hardware-in-the-loop, bench-measured and field-observed evidence remain separate.

The engineering scenario/replay envelope is being developed in [ENG-SYNTH #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172). The humanoid-specific execution strategy is in [the engineering execution plan](docs/research/HUMANOID_ENGINEERING_EXECUTION_PLAN_2026-11-09.md).

## Benchmarks and external comparators

| Question | Reference lane | Symtropy work |
|---|---|---|
| Rigid-body/contact correctness | Analytical fixtures, Rapier differential tests, matched Jolt/PhysX cases where useful | [#460 physics qualification](https://github.com/Luminous-Dynamics/symtropy/issues/460) |
| Articulated robot dynamics | MuJoCo; Newton for GPU/differentiable and multiphysics comparisons | [#1172 engineering scenario substrate](https://github.com/Luminous-Dynamics/symtropy/issues/1172) and [#1472 capability census](https://github.com/Luminous-Dynamics/symtropy/issues/1472) |
| Locomotion and recovery | Version-pinned FlyGym/NeuroMechFly, simple PID/reflex and matched learned baselines | [#3357 locomotion benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3357), [#4332 failover](https://github.com/Luminous-Dynamics/symtropy/issues/4332) |
| Actuator and whole-robot energy | Calibrated electrical/mechanical measurements and matched duty cycles | [#4044 energy accounting](https://github.com/Luminous-Dynamics/symtropy/issues/4044), [#4179 actuator loss maps](https://github.com/Luminous-Dynamics/symtropy/issues/4179) |
| Spatial understanding and execution | ESI-Bench, BEHAVIOR-Spatial, VA-Bench and held-out physical-scene trials | [Humanoid engineering plan](docs/research/HUMANOID_ENGINEERING_EXECUTION_PLAN_2026-11-09.md) |
| Humanoid application readiness | NIST's proposed baseline and Fraunhofer IPA's independent six-area assessment | [Humanoid engineering plan](docs/research/HUMANOID_ENGINEERING_EXECUTION_PLAN_2026-11-09.md) |

A performance comparison is valid only when model semantics, scenario, timestep, solver tolerance, observations, controller rate, hardware/compute budget, seeds and measurement boundary are matched and disclosed.

## Quick start: physics core

For a minimal physics-only project, the published `symtropy-math` and `symtropy-physics` crates are sufficient:

```toml
[dependencies]
symtropy-math = "0.2.1"
symtropy-physics = "0.2.1"
nalgebra = "0.34"
```

```rust
use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

fn main() {
    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
    let _body = world.add_sphere(Point::new([0.0, 2.0, 0.0]), 0.25, 1.0);

    for _ in 0..100 {
        world.step(0.016);
    }
}
```

For custom coupling, implement `PhysicsCallback` with an explicitly defined metric and limits. The generic callback is a mechanism, not an endorsement of any particular metric or control policy.

The Φ / consciousness-physics implementation is an optional **AGPL research dependency**. Its coupling experiments and energy accounting should be kept out of an engineering-deterministic profile unless that research mode is explicitly selected and reported.

## Build and validate

With a Rust 2024 toolchain:

```bash
# Core library regressions
cargo test -p symtropy-math --lib --locked
cargo test -p symtropy-physics --lib --locked

# Reproducible, machine-readable physics checks
cargo run --release -p symtropy-physics --example free_fall_validation
cargo run --release -p symtropy-physics --example oriented_collision_validation
```

The root Bevy launcher uses Bevy 0.19. A full launcher build may require the Linux system libraries and graphics/audio development packages documented in the contributor guide; the physics-only crates do not require the full Bevy runtime.

## Documentation and active plans

- [Architecture guide](docs/tech/ARCHITECTURE.md)
- [Engine guide](docs/tech/ENGINE.md)
- [Formal coupling specification](docs/tech/FORMAL_SPECIFICATION.md) — research model, not a safety standard
- [Research validity protocol](crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [Transformed-collision validation](crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md)
- [Humanoid engineering execution plan](docs/research/HUMANOID_ENGINEERING_EXECUTION_PLAN_2026-11-09.md)
- [Existing operations roadmap](docs/ops/ROADMAP.md)
- [Contributing guide](docs/CONTRIBUTING.md)

## Licensing

Symtropy is dual-track. The permissive crates are separate from the AGPL research and integration layers. See [LICENSING.md](LICENSING.md) for the crate-by-crate position.

- **Apache-2.0 OR MIT:** `symtropy-math`, `symtropy-physics`, `symtropy-bevy-core`, and `symtropy-robotics-bridge-core`.
- **AGPL-3.0-or-later:** `symtropy-consciousness-physics`, `symtropy-bevy`, `symtropy-robotics-bridge`, and the root launcher.

The permissive robotics core is intentionally kept separate from Symthaea and consciousness-physics dependencies. Always verify the exact package and dependency graph for the version being used; do not infer a repository-wide licence from one crate's licence field.
