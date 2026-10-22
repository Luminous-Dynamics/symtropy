# symtropy-render-bridge

Project `symtropy-physics` N-dimensional physics worlds into Bevy 3D rendering.

## What it does

- **2D projection** — flatten `PhysicsWorld<2>` to Bevy sprites.
- **3D projection** — `PhysicsWorld<3>` to Bevy 3D meshes.
- **4D cross-section slicing** — `PhysicsWorld<4>` projected to 3D via a configurable hyperplane, Miegakure-style. First-class support for ND collider visualisation.

## Licensing

**Apache-2.0 OR MIT.** Zero AGPL dependencies — safe to use in proprietary projects. See the [repository LICENSING.md](https://github.com/luminous-dynamics/symtropy/blob/main/LICENSING.md) for the full dual-track breakdown.

## Quick start

```rust
use bevy::prelude::*;
use symtropy_physics::PhysicsWorld;
use symtropy_render_bridge::{Projector3D, sync_physics_3d};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Projector3D::default())
        .add_systems(FixedUpdate, sync_physics_3d::<3>)
        .run();
}
```

For 4D cross-sections:

```rust
use symtropy_render_bridge::Projector4D;

let projector = Projector4D::with_slice_hyperplane(
    /* normal */ Vec4::new(0.0, 0.0, 0.0, 1.0),
    /* offset */ 0.0,
);
```

## Status

Early — the core projection API is stable; the 4D shader pipeline lands in Phase 2 of the [roadmap](https://github.com/luminous-dynamics/symtropy/blob/main/ROADMAP.md).

## Dependencies

- `symtropy-math` — N-dimensional geometry primitives.
- `symtropy-physics` — `PhysicsWorld<D>` source data.
- `bevy` 0.18 — target renderer.
- `nalgebra` 0.34 — vector math.

## References

- [The Symtropy Book](https://github.com/luminous-dynamics/symtropy/tree/main/book) — overview and tutorials.
- [ten Bosch, M. (2020). *N-Dimensional Rigid Body Dynamics*, SIGGRAPH](https://marctenbosch.com/) — theoretical foundation for 4D rotation.


## Read-only physics observability (optional)

Enable the `debug-gizmos` feature to add `PhysicsObservabilityPlugin`. It can draw contact-point crosses and contact-normal lines from the most recent physics manifolds, plus optional linear-velocity vectors for moving dynamic or kinematic bodies. The plugin is optional, has explicit per-frame contact and velocity limits, sanitizes non-finite data, and reads `PhysicsWorldRes2D` / `PhysicsWorldRes3D` through immutable Bevy resources.

```toml
symtropy-render-bridge = { version = "0.1", features = ["debug-gizmos"] }
```

```rust
use bevy::prelude::*;
use symtropy_render_bridge::{PhysicsObservabilityPlugin, PhysicsOverlayConfig};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(PhysicsObservabilityPlugin)
    .insert_resource(PhysicsOverlayConfig {
        enabled: true,
        show_points: true,
        show_normals: true,
        show_velocities: true,
        max_contact_points: 512,
        max_velocity_vectors: 512,
        velocity_scale: 0.15,
        max_velocity_length: 2.0,
        ..default()
    });
```

The host application must insert one or both of `PhysicsWorldRes2D` and `PhysicsWorldRes3D`, and enable Bevy gizmo rendering. A missing dimension's world resource is fine. Set `enabled = false` to disable overlays; `show_velocities = true` enables the velocity arrows, which are off by default. The pure `extract_contact_glyphs` and `extract_velocity_glyphs` functions can also be used without rendering to test recorded physics state or compare diagnostics.

This view is deliberately read-only: it does not apply forces, mutate bodies or constraints, step the simulation, write to the physics ledger, or change authority. The unit tests validate finite projection, 2D/3D mapping, normalized contact normals, deterministic read-only extraction, invalid-input rejection, and independent contact/velocity bounds. These tests establish the software contract for the overlay; they do not validate the underlying physics solver.
