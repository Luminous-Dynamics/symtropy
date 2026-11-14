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


## Multiphysics and spectral field views (optional)

Enable the `field-visualization` feature to use `FieldVisualizationPlugin`. This adds a renderer for **producer-supplied data**, not another physics solver. It can display scalar samples as false-colour markers and vector samples as direction arrows over the same scene.

```toml
symtropy-render-bridge = { path = "../symtropy-render-bridge", features = ["field-visualization"] }
```

The typed layer model supports these standard quantities and unit pairings:

| Quantity | Scalar/vector | Units |
|---|---|---|
| Temperature | Scalar | K or °C |
| Electric potential | Scalar | V |
| Electric field | Magnitude or vector | V/m |
| Magnetic flux density (B) | Magnitude or vector | T |
| Magnetic field intensity (H) | Magnitude or vector | A/m |
| Poynting energy-flux magnitude/vector | Scalar or vector | W/m² |
| Acoustic pressure and equivalent stress | Scalar | Pa |
| Strain magnitude | Scalar | Dimensionless |
| Density | Scalar | kg/m³ |
| Linear velocity | Vector | m/s |
| Force | Vector | N |
| Spectral radiance | Scalar | W/(m²·sr·nm) or W/(m²·sr·m) |
| Domain-specific value | Scalar/vector | Explicit custom unit string |

A metadata record names the layer, quantity, unit, source, calibration status, coordinate frame and spatial unit, timestamp, fixed display range, color map, and optional wavelength band/frequency. Positions in metres/centimetres/millimetres are converted using `world_units_per_meter`; `WorldUnit` positions are already in scene coordinates. A *calibrated sensor* declaration requires a calibration reference. Spectral-radiance layers require a wavelength band. Invalid quantity/unit pairs, invalid ranges, non-finite samples, and malformed spectra fail validation rather than becoming plausible-looking colors.

### Example: temperature samples and an electric-field vector

```rust
use symtropy_render_bridge::{
    CalibrationStatus, DisplayRange, FieldColorMap, FieldDataOrigin,
    FieldLayerMetadata, FieldLayerSet, FieldUnit, FieldVisualizationPlugin,
    PhysicalQuantity, ScalarFieldLayer, ScalarFieldSample, SpatialUnit,
    VectorFieldLayer, VectorFieldSample,
};

let temperature = ScalarFieldLayer {
    metadata: FieldLayerMetadata {
        id: "thermal-probe".to_owned(),
        quantity: PhysicalQuantity::Temperature,
        unit: FieldUnit::Celsius,
        origin: FieldDataOrigin::MeasuredSensor,
        calibration: CalibrationStatus::Uncalibrated,
        source_label: "demo thermal sensor stream".to_owned(),
        calibration_reference: None,
        coordinate_frame: "world".to_owned(),
        spatial_unit: SpatialUnit::Meter,
        world_units_per_meter: 1.0,
        timestamp_unix_ns: Some(1_800_000_000_000_000_000),
        wavelength_band: None,
        frequency_hz: None,
        display_range: DisplayRange { min: 10.0, max: 90.0 },
        color_map: FieldColorMap::Thermal,
    },
    enabled: true,
    marker_radius: 0.04,
    samples: vec![ScalarFieldSample {
        position: [0.0, 1.0, 0.0],
        value: 63.5,
        uncertainty: Some(0.8),
    }],
};

let electric_field = VectorFieldLayer {
    metadata: FieldLayerMetadata {
        id: "em-e-field".to_owned(),
        quantity: PhysicalQuantity::ElectricFieldVector,
        unit: FieldUnit::VoltPerMeter,
        origin: FieldDataOrigin::Simulated,
        calibration: CalibrationStatus::NotApplicable,
        source_label: "electromagnetic solver output".to_owned(),
        calibration_reference: None,
        coordinate_frame: "world".to_owned(),
        spatial_unit: SpatialUnit::Meter,
        timestamp_unix_ns: Some(1_800_000_000_000_000_000),
        wavelength_band: None,
        frequency_hz: Some(2.4e9),
        display_range: DisplayRange { min: 0.0, max: 500.0 },
        color_map: FieldColorMap::ViridisLike,
    },
    enabled: true,
    vector_scale: 0.8,
    max_vector_length: 1.0,
    samples: vec![VectorFieldSample {
        position: [0.0, 1.0, 0.0],
        vector: [120.0, 20.0, 0.0],
        magnitude_uncertainty: None,
    }],
};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(FieldVisualizationPlugin)
    .insert_resource(FieldLayerSet {
        scalar_layers: vec![temperature],
        vector_layers: vec![electric_field],
        ..default()
    })
    .run();
```

The plugin draws read-only glyphs. Switching views means toggling a layer's `enabled` flag or replacing its supplied sample frame. Current drawing limits are 100,000 samples per input layer, at most 256 scalar and 256 vector layers, up to 2,048 scalar markers per layer / 8,192 total, and up to 1,024 vector glyphs per layer / 4,096 total. These are safety ceilings, not claims about achieving a particular frame rate.

### Wavelength-resolved spectra

For spectral data, use `SpectrumProfile` rather than collapsing all wavelengths into a single scalar:

```rust
use symtropy_render_bridge::{
    CalibrationStatus, FieldDataOrigin, FieldUnit, SpatialUnit,
    SpectrumBin, SpectrumProfile, validate_spectrum_profile,
};

let spectrum = SpectrumProfile {
    id: "hyperspectral-pixel-17".to_owned(),
    unit: FieldUnit::WattPerSquareMeterSteradianPerNanometer,
    origin: FieldDataOrigin::MeasuredSensor,
    calibration: CalibrationStatus::Calibrated,
    source_label: "calibrated hyperspectral camera".to_owned(),
    calibration_reference: Some("radiance-standard-CERT-123".to_owned()),
    coordinate_frame: "sensor-world".to_owned(),
    spatial_unit: SpatialUnit::Meter,
    timestamp_unix_ns: Some(1_800_000_000_000_000_000),
    position: [1.0, 0.5, 0.0],
    bins: vec![
        SpectrumBin { wavelength_m: 400e-9, spectral_radiance: 1.2, uncertainty: Some(0.1) },
        SpectrumBin { wavelength_m: 550e-9, spectral_radiance: 2.4, uncertainty: Some(0.1) },
        SpectrumBin { wavelength_m: 700e-9, spectral_radiance: 1.7, uncertainty: Some(0.12) },
    ],
};

validate_spectrum_profile(&spectrum)?;
let band = spectrum.wavelength_band()?;
let frequencies_hz = band.frequency_range_hz()?;
```

This profile preserves ordered wavelength-bin values and per-bin uncertainty at one spatial sample. It can back a host UI's spectrum plot or be sampled along an image/volume grid. The current Bevy layer renderer does **not** draw a spectrum chart or a continuous volume; it draws bounded scene glyphs. A future host UI should put an explicit legend beside the view showing quantity, unit, source, calibration status, timestamp, wavelength/frequency band, fixed range, and uncertainty policy.

### Important scientific interpretation limits

- A thermal colour map is only a visual encoding. It does **not** turn visible RGB pixels into temperature, solve heat transfer, infer emissivity, or correct for reflected apparent temperature. For real thermography, bring a temperature/radiance product from a documented sensor/calibration pipeline and preserve its uncertainty and calibration status. Thermal-camera measurement accuracy depends on calibration and parameters such as emissivity. See [NIST's thermal-camera calibration procedure](https://www.nist.gov/publications/calibration-and-measurement-procedures-high-magnification-thermal-camera) and the review [Infrared Thermography for Temperature Measurement and Non-Destructive Testing](https://pmc.ncbi.nlm.nih.gov/articles/PMC4168422/).
- Infrared radiation is one region of the electromagnetic spectrum, not a synonym for temperature. This model supports a wavelength band in metres and derives the corresponding frequency interval using `c / wavelength`; it does not invent spectral sensor samples. See [NASA's electromagnetic-spectrum overview](https://science.nasa.gov/ems/01_intro/).
- Electric field, magnetic flux density and magnetic field intensity are distinct quantities with distinct units: respectively V/m, T and A/m. Poynting flux is treated separately as W/m²; do not relabel a magnetic field magnitude as electric-field strength. See the [German Federal Office for Radiation Protection's field/unit overview](https://www.bfs.de/EN/topics/emf/expansion-grid/basics/emfields/em-fields.html).
- Hyperspectral/radiometric data needs wavelength and radiometric calibration for quantitative interpretation. The layer records calibration metadata, but does not perform that calibration. See [NIST's hyperspectral imager calibration work](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=903023).
- Measured, simulated, derived, imported and illustrative data are labelled separately. The module validates metadata shape and units; it cannot certify that caller-declared provenance is true or that a simulation is physically correct.
- The current renderer is a bounded marker/vector-glyph path, not a GPU volume renderer, finite-element viewer, Maxwell solver or thermodynamics solver. Continuous volume slices, isosurfaces, interpolation, time playback, field equations, unit conversion and sensor-import adapters are follow-on work.

### Candidate benchmark gates

These are proposed acceptance tests, not measured outcomes:
- zero crashes or non-finite output with malformed/missing sensor/solver samples; unit mismatches rejected 100% of the time;
- deterministic glyph extraction for identical data and settings;
- zero change to the underlying simulation state when toggling field layers;
- match the numeric display range/legend to a fixed reference dataset; never auto-rescale between compared runs;
- quantify overlay overhead with 100 / 1,000 / 10,000 visible samples, reporting p50/p95/p99 frame intervals and memory, separately from GPU timestamp timings;
- run a calibrated temperature reference and published analytic electric/magnetic field test cases through an independent solver/measurement pipeline before making accuracy claims.
