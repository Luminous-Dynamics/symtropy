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
- **Constraints** — distance constraints (more coming)
- **Const-generic dimensions** — `PhysicsWorld<2>`, `PhysicsWorld<3>`, `PhysicsWorld<4>`
- **Zero heap** — GJK simplex uses `ArrayVec`, bivectors use `[f64; 6]`
- **WASM compatible**

## Benchmarks

| Operation | Time |
|-----------|------|
| GJK sphere×sphere 3D | 102 ns |
| GJK box×box 3D | 193 ns |
| GJK tesseract 4D | 231 ns |
| Physics step (100 bodies) | 193 µs |

## Open Research and Validation

The engine exposes `PhysicsWorld::invariant_snapshot()` for quantitative
measurement of momentum, energy, penetration, finite-state health, and
rotation-group error. An analytical free-fall convergence study is included:

```bash
cargo run --release --example free_fall_validation
cargo run --release --example oriented_collision_validation
```

The output is CSV for CI gates and external analysis. See
[`RESEARCH_VALIDATION.md`](RESEARCH_VALIDATION.md) for the general validity
protocol and [`ORIENTED_COLLISION_VALIDATION.md`](ORIENTED_COLLISION_VALIDATION.md)
for the transformed-geometry contract, limitations, and external campaign. Public
research claims should follow that protocol and explicitly distinguish proven,
experimental, and planned capabilities.

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
