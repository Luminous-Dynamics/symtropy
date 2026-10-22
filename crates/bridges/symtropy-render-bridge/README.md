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


## Unit-aware physical-field views (optional)

Enable the `field-observability` feature to add `FieldObservabilityPlugin`. This is a read-only **field data viewer**, not a thermal simulator, electromagnetic solver, or camera calibration system.

```toml
symtropy-render-bridge = { path = "../../bridges/symtropy-render-bridge", features = ["field-observability"] }
```

A host can insert a `ScalarFieldFrame` and/or a `VectorFieldFrame`. Scalar frames include a typed quantity, declared unit, world-space sample positions, display range, named palette, optional uncertainty and source/calibration provenance. Spectral radiance and reflectance require a wavelength band. A multi-band spectral composite can select three distinct channels and map them to display RGB explicitly; this is false-colour/pseudocolour, not a colourimetric reconstruction of human vision. Vector frames currently support electric-field vectors in V/m, magnetic-flux-density vectors in tesla, velocity vectors in m/s, and generic caller-declared vectors. All paths cap drawn samples and reject malformed numeric data. The pure extraction functions can be tested without a GPU.

Example source semantics:
- Temperature map: ingest calibrated temperature samples in kelvin and register each sample into the Symtropy world frame. The Thermal palette is only a display transfer function; it does not convert RGB material color into temperature or calculate radiative transfer.
- Thermal-camera image: retain apparent/radiometric values, calibration identity, uncertainty, emissivity/reflected-background corrections and sensor geometry in the upstream adapter before claiming true surface temperature. The viewer checks provided metadata; it does not perform those corrections itself.
- Spectral reflectance/radiance: provide the wavelength centre, FWHM, value unit and calibration provenance for every band. One scalar frame represents one band; an adapter can switch bands for inspection. The initial API supports inspected single bands and explicitly mapped three-band composites, but does not claim colorimetric reconstruction, atmospheric correction, sensor-response integration, or full hyperspectral analysis.
- Electromagnetics: insert electric-field or magnetic-flux-density vectors produced by a field solver or sensor. Arrow direction and clipped display length make vectors inspectable, but the visualizer does not solve Maxwell's equations or assert field-source correctness.

The caller-declared sensor and calibration metadata are evidence fields, not hardware attestation. A measured frame explicitly marked uncalibrated can be displayed for exploration, but must not be represented as calibrated metrology. All positions are expected in world-space metres after an upstream coordinate-registration step.

The optional feature does not change simulation state or physics cadence. Candidate validation is: unit/domain validation; deterministic glyph extraction; calibration-provenance negative controls; sample-cap tests; and comparison against known analytic field fixtures before integrating sensor adapters.

Scientific basis: infrared is an electromagnetic band rather than a direct temperature channel, and converting IR signal to temperature needs calibration and emissivity treatment. See NASA's [infrared overview](https://science.nasa.gov/ems/07_infraredwaves/) and NIST's [thermal-camera calibration procedure (NISTIR 8098)](https://doi.org/10.6028/NIST.IR.8098).