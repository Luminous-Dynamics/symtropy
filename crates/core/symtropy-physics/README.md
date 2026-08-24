# symtropy-physics

N-dimensional rigid body physics with GJK+EPA collision detection. Zero heap allocation in the hot path.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
symtropy-physics = "0.2.1"
symtropy-math = "0.2.1"
```

## Usage

```rust
use symtropy_physics::PhysicsWorld;
use symtropy_math::{Point, Sphere};
use nalgebra::SVector;

// Create a 3D world with gravity
let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));

// Add spheres
let a = world.add_sphere(Point::new([0.0, 10.0, 0.0]), 1.0, 1.0);
let b = world.add_sphere(Point::new([0.0, 0.0, 0.0]), 1.0, 1.0);

// Step the simulation
for _ in 0..100 {
    world.step(0.016);
}

// Spheres collided and bounced
```

## Features

- **GJK** intersection test — 102ns for sphere×sphere (benchmarked)
- **EPA** penetration depth — transform-aware contact normals for 2D and 3D
- **Oriented collision geometry** — full rigid transforms in support maps, broadphase bounds, primitive contacts, mesh callbacks, and ray queries
- **Dedicated OBB SAT** — 2D/3D face axes plus 3D edge-cross-edge axes
- **Coulomb friction** — tangential impulse clamped by μ×j_n
- **Body sleeping** — deactivate near-stationary bodies, wake on collision
- **Collision events** — `CollisionEvent<D>` for game logic callbacks
- **PhysicsCallback trait** — inject consciousness or custom force modulation into the physics step
- **Constraints and joints** — distance plus fixed, hinge, ball, and prismatic families with motor support where implemented
- **Const-generic dimensions** — `PhysicsWorld<2>`, `PhysicsWorld<3>`, `PhysicsWorld<4>`
- **Experimental thermodynamics** — validated Kelvin/material primitives, conservative lumped conduction, optional body thermal state, and modeled thermal-energy diagnostics
- **Zero heap** — GJK simplex uses `ArrayVec`, bivectors use `[f64; 6]`
- **WASM compatible**

## Benchmarks

| Operation | Time |
|-----------|------|
| GJK sphere×sphere 3D | 102 ns |
| GJK box×box 3D | 193 ns |
| GJK tesseract 4D | 231 ns |
| Physics step (100 bodies) | 193 µs |

These historical microbenchmarks are useful regression markers, not evidence of
competitive superiority. Cross-engine claims must use matched scenarios, matched
error tolerances, reproducible environment metadata, and the gates defined by the
physics excellence program.

## Open Research and Validation

The engine exposes `PhysicsWorld::invariant_snapshot()` for quantitative
measurement of momentum, mechanical and modeled thermal energy, penetration,
finite-state health, and rotation-group error. Analytical validation examples are
included for the currently modeled regimes.

The output is CSV for CI gates and external analysis. See
[`RESEARCH_VALIDATION.md`](RESEARCH_VALIDATION.md) for the general validity
protocol, [`ORIENTED_COLLISION_VALIDATION.md`](ORIENTED_COLLISION_VALIDATION.md)
for the transformed-geometry contract,
[`THERMODYNAMICS_VALIDATION.md`](THERMODYNAMICS_VALIDATION.md) for the thermal
first-law/second-law contract and benchmark roadmap, and
[`PHYSICS_EXCELLENCE_PROGRAM.md`](PHYSICS_EXCELLENCE_PROGRAM.md) for the
competitive capability matrix, benchmark ladder, claims policy, and execution
order toward top-tier game and real-world simulation.

Public research and competitive claims should explicitly distinguish implemented,
validated, competitive, and leading capabilities.

## PhysicsCallback — Custom Force Modulation

```rust
use symtropy_physics::{PhysicsCallback, CollisionEvent, BodyHandle};

struct MyCallback;
impl PhysicsCallback<3> for MyCallback {
    fn modulate_impulse(&self, impulse: f64, point: &SVector<f64, 3>) -> f64 {
        impulse * 0.5 // Dampen all collisions by 50%
    }
    // ... other methods
}

world.step_with_callback(0.016, &mut MyCallback);
```

Part of the [Symtropy consciousness-physics engine](https://github.com/luminous-dynamics/symtropy).