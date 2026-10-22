# symtropy-render-bridge

Project `symtropy-physics` N-dimensional physics worlds into Bevy 3D rendering.

## What it does

- **2D projection** — flatten `PhysicsWorld<2>` to Bevy sprites.
- **3D projection** — `PhysicsWorld<3>` to Bevy 3D meshes.
- **4D cross-section slicing** — `PhysicsWorld<4>` projected to 3D via a configurable hyperplane, Miegakure-style. First-class support for ND collider visualisation.

## Solar-scale rendering foundations

The bridge has a precision-aware large-world position primitive for scenes that
span from local detail to planetary and interplanetary distances. Canonical
positions use a signed integer cell address plus an f64 offset inside that cell;
camera-relative conversion subtracts cell addresses before creating render
coordinates. A three-part high/middle/low f32 representation is available for
GPU paths that preserve the split through the view/projection transform.

The optional apparent-scale policy computes a sphere's projected diameter for
perspective or orthographic cameras, classifies it into configurable presentation
bands, and applies hysteresis to avoid chattering at representation thresholds.
These bands are not object deletion rules: selection, scientific identity, and
task-critical geometry remain host responsibilities.

This is not yet a complete solar-system renderer. Ephemerides, reference-frame
transforms, tile streaming, shader integration, depth strategy and an end-to-end
surface-to-system camera traversal still need separate qualification.

See:
- [Solar-scale architecture and coordinate contract](../../../docs/research/RENDER_SOLAR_SCALE_001.md)
- [Apparent-size policy and benchmark plan](../../../docs/research/RENDER_APPARENT_SCALE_002.md)

## Hierarchical representation selection

The optional hierarchy selector chooses which body proxy, regional tile, or
surface representation is drawn using a host-supplied screen-space error limit.
It validates the parent/child forest, requires explicit full-coverage declarations
before replacement refinement, records parent fallbacks during streaming, and
preserves semantic entity IDs while representation-node IDs change. Forced/detail-
pinned requests fail visibly when required children are not ready.

This is a pure selection policy. It does not load assets, generate planetary tiles,
compute camera error, cull, or modify simulation state. See
[the hierarchy qualification note](../../../docs/research/RENDER_HIERARCHY_004.md)
and the frozen [solar-scale traversal fixture](../../../benchmarks/render/SOLAR-SCALE-01.json).

## Solar-scale viewer prototype

The stacked app `symtropy-solar-system-viewer` is a navigable Bevy prototype
for stress-testing cell/local coordinates and apparent-size transitions from
local detail to a solar-system overview. It draws static illustrative bodies
and circular guide paths, reports marker exaggeration and precision-budget
rejections, and explicitly labels the orbital layout as illustrative rather
than ephemeris-backed. It is not a scientific model, and its current camera
uses budget-checked camera-relative f32 transforms rather than the high/middle/
low shader path.

See [the viewer qualification note](../../../docs/research/RENDER_SOLAR_VIEWER_005.md).

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
