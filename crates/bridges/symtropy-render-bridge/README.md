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


## Purpose-built multidimensional views

The view contract separates **why a view exists** from **how many dimensions it represents**. Use `ViewProfile::recommended(purpose, dimension)` for Strategic, Operational, Tactical, Scientific, or Diagnostic intent, then choose Planar2D, Oblique25D, Spatial3D, or HyperSlice4D independently. Each profile also declares a navigation policy: planar pan/zoom, oblique pan/orbit, overview orbit, focused orbit, inspection orbit, or 4D slice scrub/orbit.

- Strategic views prioritize terrain, infrastructure, networks, routes, and identity-preserving aggregation.
- Operational views prioritize routes, flows, infrastructure dependencies, and clustered activity.
- Tactical views prioritize local entity geometry, contacts, velocity vectors, and motion trails.
- Scientific views prioritize scalar/vector fields, uncertainty, legends, and source provenance.
- Diagnostic views prioritize contacts, motion and field overlays for debugging.

`ProjectorOblique25D` supplies a configurable oblique orthographic coordinate basis that preserves the distinction between ground depth and elevation. The host camera must align to that basis. `OrientedProjector4D` supports a caller-specified 4D hyperplane normal, offset, and half-thickness; it projects points onto that plane but does **not** compute exact mesh/collider intersections.

These are projection and policy contracts, not a complete view manager. Navigation policy is declared for the host input layer; it does not itself install controls or move cameras. Interactive camera controls, viewport-aware culling, scale-dependent aggregation, cross-view selection, and visual-regression capture still need host integration.

See [PURPOSE_BUILT_MULTIDIMENSIONAL_VIEWS_V0_1.md](../../../docs/research/PURPOSE_BUILT_MULTIDIMENSIONAL_VIEWS_V0_1.md) for invariants, proposed benchmarks, and current boundaries.
