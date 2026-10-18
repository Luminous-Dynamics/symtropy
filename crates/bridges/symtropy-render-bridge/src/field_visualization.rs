// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Typed, bounded, read-only visualization for scalar and vector physical fields.
//!
//! This module maps already-computed or measured data into a visual encoding.
//! It does not simulate electromagnetism, infer temperature from RGB, calibrate
//! sensors, or upgrade caller-declared provenance into measurement evidence.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
const MAX_FIELD_SAMPLES: usize = 100_000;
const MAX_FIELD_LAYERS: usize = 256;
const MAX_SCALAR_MARKERS_PER_LAYER: usize = 2_048;
const MAX_VECTOR_GLYPHS_PER_LAYER: usize = 1_024;
const MAX_SCALAR_MARKERS_TOTAL: usize = 8_192;
const MAX_VECTOR_GLYPHS_TOTAL: usize = 4_096;

/// A physical quantity with a corresponding display unit.
///
/// Scalar magnitude and vector quantities are distinct variants so an electric
/// field vector cannot be silently treated as a scalar potential or magnitude.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PhysicalQuantity {
    Temperature,
    ElectricPotential,
    ElectricFieldMagnitude,
    ElectricFieldVector,
    MagneticFluxDensityMagnitude,
    MagneticFluxDensityVector,
    MagneticFieldIntensityMagnitude,
    MagneticFieldIntensityVector,
    PoyntingMagnitude,
    PoyntingVector,
    AcousticPressure,
    EquivalentStress,
    StrainMagnitude,
    Density,
    VelocityVector,
    ForceVector,
    SpectralRadiance,
    CustomScalar,
    CustomVector,
}

/// Explicit unit labels. Values are supplied in the selected unit; this crate
/// never performs an implicit unit conversion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FieldUnit {
    Kelvin,
    Celsius,
    Volt,
    VoltPerMeter,
    Tesla,
    AmperePerMeter,
    WattPerSquareMeter,
    Pascal,
    Dimensionless,
    KilogramPerCubicMeter,
    MeterPerSecond,
    Newton,
    WattPerSquareMeterSteradianPerNanometer,
    WattPerSquareMeterSteradianPerMeter,
    Custom(String),
}

impl FieldUnit {
    pub fn symbol(&self) -> &str {
        match self {
            Self::Kelvin => "K",
            Self::Celsius => "°C",
            Self::Volt => "V",
            Self::VoltPerMeter => "V/m",
            Self::Tesla => "T",
            Self::AmperePerMeter => "A/m",
            Self::WattPerSquareMeter => "W/m²",
            Self::Pascal => "Pa",
            Self::Dimensionless => "1",
            Self::KilogramPerCubicMeter => "kg/m³",
            Self::MeterPerSecond => "m/s",
            Self::Newton => "N",
            Self::WattPerSquareMeterSteradianPerNanometer => "W/(m²·sr·nm)",
            Self::WattPerSquareMeterSteradianPerMeter => "W/(m²·sr·m)",
            Self::Custom(symbol) => symbol,
        }
    }

    fn is_compatible_with(&self, quantity: PhysicalQuantity) -> bool {
        match quantity {
            PhysicalQuantity::Temperature => matches!(self, Self::Kelvin | Self::Celsius),
            PhysicalQuantity::ElectricPotential => matches!(self, Self::Volt),
            PhysicalQuantity::ElectricFieldMagnitude | PhysicalQuantity::ElectricFieldVector => {
                matches!(self, Self::VoltPerMeter)
            }
            PhysicalQuantity::MagneticFluxDensityMagnitude
            | PhysicalQuantity::MagneticFluxDensityVector => matches!(self, Self::Tesla),
            PhysicalQuantity::MagneticFieldIntensityMagnitude
            | PhysicalQuantity::MagneticFieldIntensityVector => {
                matches!(self, Self::AmperePerMeter)
            }
            PhysicalQuantity::PoyntingMagnitude | PhysicalQuantity::PoyntingVector => {
                matches!(self, Self::WattPerSquareMeter)
            }
            PhysicalQuantity::AcousticPressure | PhysicalQuantity::EquivalentStress => {
                matches!(self, Self::Pascal)
            }
            PhysicalQuantity::StrainMagnitude => matches!(self, Self::Dimensionless),
            PhysicalQuantity::Density => matches!(self, Self::KilogramPerCubicMeter),
            PhysicalQuantity::VelocityVector => matches!(self, Self::MeterPerSecond),
            PhysicalQuantity::ForceVector => matches!(self, Self::Newton),
            PhysicalQuantity::SpectralRadiance => matches!(
                self,
                Self::WattPerSquareMeterSteradianPerNanometer
                    | Self::WattPerSquareMeterSteradianPerMeter
            ),
            PhysicalQuantity::CustomScalar | PhysicalQuantity::CustomVector => {
                matches!(self, Self::Custom(symbol) if !symbol.trim().is_empty())
            }
        }
    }
}

/// How values entered the visualization pipeline. This is provenance, not proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FieldDataOrigin {
    MeasuredSensor,
    Simulated,
    Derived,
    ImportedDataset,
    SyntheticIllustration,
}

/// Calibration status is always explicit; rendered colors do not imply calibration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CalibrationStatus {
    Calibrated,
    PartiallyCalibrated,
    Uncalibrated,
    NotApplicable,
}

/// Coordinate scale used by sample positions. Field-unit conversion and spatial
/// coordinate conversion are deliberately separate concerns.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SpatialUnit {
    Meter,
    Centimeter,
    Millimeter,
    WorldUnit,
}

/// A spectral band described in vacuum wavelength metres.
///
/// The interval is ordered from shorter to longer wavelength. Frequency bounds
/// are derived from c / wavelength; they are not independent measurements.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WavelengthBand {
    pub min_wavelength_m: f64,
    pub max_wavelength_m: f64,
}

impl WavelengthBand {
    pub fn validate(&self) -> Result<(), String> {
        if !self.min_wavelength_m.is_finite()
            || !self.max_wavelength_m.is_finite()
            || self.min_wavelength_m <= 0.0
            || self.max_wavelength_m <= self.min_wavelength_m
        {
            return Err("wavelength band must be finite, positive, and ordered".to_owned());
        }
        Ok(())
    }

    pub fn frequency_range_hz(&self) -> Result<(f64, f64), String> {
        self.validate()?;
        Ok((
            SPEED_OF_LIGHT_M_PER_S / self.max_wavelength_m,
            SPEED_OF_LIGHT_M_PER_S / self.min_wavelength_m,
        ))
    }
}

/// Fixed display bounds. Auto-ranging is intentionally omitted from this first
/// implementation because silently changing the range makes runs incomparable.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisplayRange {
    pub min: f64,
    pub max: f64,
}

impl DisplayRange {
    pub fn validate(&self) -> Result<(), String> {
        if !self.min.is_finite() || !self.max.is_finite() || self.max <= self.min {
            return Err("display range requires finite min < max".to_owned());
        }
        Ok(())
    }

    pub fn normalize(&self, value: f64) -> f64 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
}

/// Maps data values to display colors, not to physical radiance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FieldColorMap {
    Thermal,
    ViridisLike,
    Diverging,
    Grayscale,
}

impl FieldColorMap {
    pub fn map_normalized(self, normalized: f64) -> [f32; 3] {
        let t = if normalized.is_finite() {
            normalized.clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let stops: &[(f32, [f32; 3])] = match self {
            Self::Thermal => &[
                (0.0, [0.025, 0.02, 0.16]),
                (0.2, [0.02, 0.12, 0.58]),
                (0.4, [0.0, 0.72, 0.82]),
                (0.6, [1.0, 0.86, 0.12]),
                (0.8, [0.94, 0.18, 0.025]),
                (1.0, [1.0, 0.96, 0.88]),
            ],
            Self::ViridisLike => &[
                (0.0, [0.267, 0.005, 0.329]),
                (0.25, [0.230, 0.322, 0.546]),
                (0.5, [0.128, 0.567, 0.551]),
                (0.75, [0.369, 0.789, 0.383]),
                (1.0, [0.993, 0.906, 0.144]),
            ],
            Self::Diverging => &[
                (0.0, [0.12, 0.28, 0.88]),
                (0.5, [0.96, 0.96, 0.92]),
                (1.0, [0.78, 0.08, 0.10]),
            ],
            Self::Grayscale => &[(0.0, [0.0, 0.0, 0.0]), (1.0, [1.0, 1.0, 1.0])],
        };

        let mut upper = 1;
        while upper < stops.len() && t > stops[upper].0 {
            upper += 1;
        }
        let lower = upper.saturating_sub(1);
        let (start_t, start_rgb) = stops[lower];
        let (end_t, end_rgb) = stops[upper.min(stops.len() - 1)];
        let local = if end_t > start_t {
            ((t - start_t) / (end_t - start_t)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        [
            start_rgb[0] + (end_rgb[0] - start_rgb[0]) * local,
            start_rgb[1] + (end_rgb[1] - start_rgb[1]) * local,
            start_rgb[2] + (end_rgb[2] - start_rgb[2]) * local,
        ]
    }
}

/// Provenance and physical semantics shared by scalar and vector layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldLayerMetadata {
    pub id: String,
    pub quantity: PhysicalQuantity,
    pub unit: FieldUnit,
    pub origin: FieldDataOrigin,
    pub calibration: CalibrationStatus,
    /// Human-readable acquisition/simulation source; never interpreted as truth.
    pub source_label: String,
    /// A traceable reference or procedure, when calibration was performed.
    pub calibration_reference: Option<String>,
    /// Named coordinate frame, for example "world", "sensor-local", or a CAD datum.
    pub coordinate_frame: String,
    pub spatial_unit: SpatialUnit,
    /// Scene coordinates per physical metre; ignored for already-projected WorldUnit data.
    pub world_units_per_meter: f64,
    /// Unix epoch timestamp in nanoseconds when supplied by the source.
    pub timestamp_unix_ns: Option<u64>,
    /// Optional wave/spectral acquisition band in vacuum wavelength metres.
    pub wavelength_band: Option<WavelengthBand>,
    /// Optional temporal frequency for a harmonic field, in hertz.
    pub frequency_hz: Option<f64>,
    /// Fixed scale bounds in the declared field unit.
    pub display_range: DisplayRange,
    pub color_map: FieldColorMap,
}

impl FieldLayerMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("field layer id must not be empty".to_owned());
        }
        if self.source_label.trim().is_empty() {
            return Err(format!("field layer {:?} has no source label", self.id));
        }
        if self.coordinate_frame.trim().is_empty() {
            return Err(format!("field layer {:?} has no coordinate frame", self.id));
        }
        if !self.unit.is_compatible_with(self.quantity) {
            return Err(format!(
                "unit {} is incompatible with quantity {:?}",
                self.unit.symbol(),
                self.quantity
            ));
        }
        self.display_range.validate()?;
        if !self.world_units_per_meter.is_finite() || self.world_units_per_meter <= 0.0 {
            return Err("world_units_per_meter must be finite and positive".to_owned());
        }

        if self.calibration == CalibrationStatus::Calibrated
            && self.origin == FieldDataOrigin::MeasuredSensor
            && self
                .calibration_reference
                .as_deref()
                .is_none_or(|reference| reference.trim().is_empty())
        {
            return Err("a calibrated sensor layer requires a calibration reference".to_owned());
        }
        if let Some(band) = self.wavelength_band {
            band.validate()?;
        }
        if let Some(frequency) = self.frequency_hz
            && (!frequency.is_finite() || frequency <= 0.0)
        {
            return Err("frequency_hz must be finite and positive".to_owned());
        }
        if let (Some(frequency), Some(band)) = (self.frequency_hz, self.wavelength_band) {
            let (minimum_frequency, maximum_frequency) = band.frequency_range_hz()?;
            if frequency < minimum_frequency || frequency > maximum_frequency {
                return Err("frequency_hz falls outside the declared wavelength band".to_owned());
            }
        }
        if self.quantity == PhysicalQuantity::SpectralRadiance && self.wavelength_band.is_none() {
            return Err("spectral radiance requires an explicit wavelength band".to_owned());
        }
        Ok(())
    }
}

/// One wavelength-resolved radiance observation. Wavelengths are vacuum
/// metres; radiance values use the single declared unit for the whole profile.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectrumBin {
    pub wavelength_m: f64,
    pub spectral_radiance: f64,
    /// Standard uncertainty expressed in the profile's spectral-radiance unit.
    pub uncertainty: Option<f64>,
}

/// Wavelength-resolved spectrum at one 3D scene position.
///
/// This is measured/imported/solver-produced data, not a radiative-transfer
/// solver. Bins must be strictly ordered by wavelength and use one spectral
/// radiance unit throughout the profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpectrumProfile {
    pub id: String,
    pub unit: FieldUnit,
    pub origin: FieldDataOrigin,
    pub calibration: CalibrationStatus,
    pub source_label: String,
    pub calibration_reference: Option<String>,
    pub coordinate_frame: String,
    pub spatial_unit: SpatialUnit,
    pub timestamp_unix_ns: Option<u64>,
    pub position: [f64; 3],
    pub bins: Vec<SpectrumBin>,
}

pub fn validate_spectrum_profile(profile: &SpectrumProfile) -> Result<(), String> {
    if profile.id.trim().is_empty()
        || profile.source_label.trim().is_empty()
        || profile.coordinate_frame.trim().is_empty()
    {
        return Err(
            "spectrum profile requires an id, source label, and coordinate frame".to_owned(),
        );
    }
    if profile.unit != FieldUnit::WattPerSquareMeterSteradianPerNanometer
        && profile.unit != FieldUnit::WattPerSquareMeterSteradianPerMeter
    {
        return Err("spectrum profile requires a spectral-radiance unit".to_owned());
    }
    if profile.calibration == CalibrationStatus::Calibrated
        && profile.origin == FieldDataOrigin::MeasuredSensor
        && profile
            .calibration_reference
            .as_deref()
            .is_none_or(|reference| reference.trim().is_empty())
    {
        return Err("a calibrated measured spectrum requires a calibration reference".to_owned());
    }
    validate_finite_position(profile.position)?;
    if profile.bins.len() < 2 || profile.bins.len() > 4_096 {
        return Err("spectrum profile must contain 2..=4096 wavelength bins".to_owned());
    }
    let mut previous_wavelength = 0.0;
    for bin in &profile.bins {
        if !bin.wavelength_m.is_finite() || bin.wavelength_m <= previous_wavelength {
            return Err(
                "spectrum wavelengths must be finite, positive, and strictly increasing".to_owned(),
            );
        }
        if !bin.spectral_radiance.is_finite() || bin.spectral_radiance < 0.0 {
            return Err("spectral radiance must be finite and non-negative".to_owned());
        }
        if bin
            .uncertainty
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err("spectrum uncertainty must be finite and non-negative".to_owned());
        }
        previous_wavelength = bin.wavelength_m;
    }
    Ok(())
}

impl SpectrumProfile {
    pub fn wavelength_band(&self) -> Result<WavelengthBand, String> {
        validate_spectrum_profile(self)?;
        Ok(WavelengthBand {
            min_wavelength_m: self.bins[0].wavelength_m,
            max_wavelength_m: self
                .bins
                .last()
                .expect("validated profile has wavelength bins")
                .wavelength_m,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalarFieldSample {
    pub position: [f64; 3],
    pub value: f64,
    /// Standard uncertainty in the same unit as value, if supplied.
    pub uncertainty: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VectorFieldSample {
    pub position: [f64; 3],
    pub vector: [f64; 3],
    /// Standard uncertainty of vector magnitude, in the declared unit.
    pub magnitude_uncertainty: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalarFieldLayer {
    pub metadata: FieldLayerMetadata,
    pub enabled: bool,
    pub marker_radius: f32,
    pub samples: Vec<ScalarFieldSample>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VectorFieldLayer {
    pub metadata: FieldLayerMetadata,
    pub enabled: bool,
    pub vector_scale: f32,
    pub max_vector_length: f32,
    pub samples: Vec<VectorFieldSample>,
}

/// Global drawing budgets plus named scalar/vector layers.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldLayerSet {
    pub enabled: bool,
    pub max_scalar_markers_per_layer: usize,
    pub max_vector_glyphs_per_layer: usize,
    pub max_scalar_markers_total: usize,
    pub max_vector_glyphs_total: usize,
    pub scalar_layers: Vec<ScalarFieldLayer>,
    pub vector_layers: Vec<VectorFieldLayer>,
}

impl Default for FieldLayerSet {
    fn default() -> Self {
        Self {
            enabled: true,
            max_scalar_markers_per_layer: 2_048,
            max_vector_glyphs_per_layer: 1_024,
            max_scalar_markers_total: 8_192,
            max_vector_glyphs_total: 4_096,
            scalar_layers: Vec::new(),
            vector_layers: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarFieldGlyph {
    pub position: [f32; 3],
    pub value: f64,
    pub normalized_value: f32,
    pub uncertainty: Option<f64>,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorFieldGlyph {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub magnitude: f64,
    pub normalized_magnitude: f32,
    pub color: [f32; 3],
}

/// Validate and prepare up to max_samples finite scalar samples for rendering.
pub fn extract_scalar_glyphs(
    layer: &ScalarFieldLayer,
    max_samples: usize,
) -> Result<Vec<ScalarFieldGlyph>, String> {
    validate_scalar_layer(layer)?;
    let mut glyphs = Vec::with_capacity(max_samples.min(layer.samples.len()));
    if max_samples == 0 {
        return Ok(glyphs);
    }

    for sample in layer.samples.iter().take(max_samples) {
        let position = project_position(
            sample.position,
            layer.metadata.spatial_unit,
            layer.metadata.world_units_per_meter,
        )?;
        if !sample.value.is_finite() {
            return Err(format!(
                "layer {:?} contains a non-finite scalar value",
                layer.metadata.id
            ));
        }
        if sample
            .uncertainty
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(format!(
                "layer {:?} contains invalid uncertainty",
                layer.metadata.id
            ));
        }
        let normalized = layer.metadata.display_range.normalize(sample.value);
        glyphs.push(ScalarFieldGlyph {
            position,
            value: sample.value,
            normalized_value: normalized as f32,
            uncertainty: sample.uncertainty,
            color: layer.metadata.color_map.map_normalized(normalized),
        });
    }
    Ok(glyphs)
}

/// Validate and prepare bounded vector glyphs. Direction carries orientation;
/// magnitude is separately mapped through the fixed declared display range.
pub fn extract_vector_glyphs(
    layer: &VectorFieldLayer,
    max_samples: usize,
) -> Result<Vec<VectorFieldGlyph>, String> {
    validate_vector_layer(layer)?;
    let mut glyphs = Vec::with_capacity(max_samples.min(layer.samples.len()));
    if max_samples == 0 {
        return Ok(glyphs);
    }

    for sample in layer.samples.iter().take(max_samples) {
        let position = project_position(
            sample.position,
            layer.metadata.spatial_unit,
            layer.metadata.world_units_per_meter,
        )?;
        if sample.vector.iter().any(|component| !component.is_finite()) {
            return Err(format!(
                "layer {:?} contains a non-finite vector",
                layer.metadata.id
            ));
        }
        if sample
            .magnitude_uncertainty
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(format!(
                "layer {:?} contains invalid magnitude uncertainty",
                layer.metadata.id
            ));
        }
        let magnitude = sample
            .vector
            .iter()
            .fold(0.0_f64, |norm, value| norm.hypot(*value));
        if !magnitude.is_finite() {
            return Err(format!(
                "layer {:?} contains an unrepresentable vector magnitude",
                layer.metadata.id
            ));
        }
        let direction = if magnitude > f64::EPSILON {
            [
                (sample.vector[0] / magnitude) as f32,
                (sample.vector[1] / magnitude) as f32,
                (sample.vector[2] / magnitude) as f32,
            ]
        } else {
            [0.0; 3]
        };
        let normalized = layer.metadata.display_range.normalize(magnitude);
        glyphs.push(VectorFieldGlyph {
            position,
            direction,
            magnitude,
            normalized_magnitude: normalized as f32,
            color: layer.metadata.color_map.map_normalized(normalized),
        });
    }
    Ok(glyphs)
}

fn validate_scalar_layer(layer: &ScalarFieldLayer) -> Result<(), String> {
    layer.metadata.validate()?;
    if !matches!(
        layer.metadata.quantity,
        PhysicalQuantity::Temperature
            | PhysicalQuantity::ElectricPotential
            | PhysicalQuantity::ElectricFieldMagnitude
            | PhysicalQuantity::MagneticFluxDensityMagnitude
            | PhysicalQuantity::MagneticFieldIntensityMagnitude
            | PhysicalQuantity::PoyntingMagnitude
            | PhysicalQuantity::AcousticPressure
            | PhysicalQuantity::EquivalentStress
            | PhysicalQuantity::StrainMagnitude
            | PhysicalQuantity::Density
            | PhysicalQuantity::SpectralRadiance
            | PhysicalQuantity::CustomScalar
    ) {
        return Err(format!(
            "quantity {:?} is not a scalar-layer quantity",
            layer.metadata.quantity
        ));
    }
    if layer.samples.len() > MAX_FIELD_SAMPLES {
        return Err(format!(
            "field layer exceeds maximum of {MAX_FIELD_SAMPLES} samples"
        ));
    }
    if !layer.marker_radius.is_finite() || layer.marker_radius < 0.0 {
        return Err("scalar marker radius must be finite and non-negative".to_owned());
    }
    Ok(())
}

fn validate_vector_layer(layer: &VectorFieldLayer) -> Result<(), String> {
    layer.metadata.validate()?;
    if !matches!(
        layer.metadata.quantity,
        PhysicalQuantity::ElectricFieldVector
            | PhysicalQuantity::MagneticFluxDensityVector
            | PhysicalQuantity::MagneticFieldIntensityVector
            | PhysicalQuantity::PoyntingVector
            | PhysicalQuantity::VelocityVector
            | PhysicalQuantity::ForceVector
            | PhysicalQuantity::CustomVector
    ) {
        return Err(format!(
            "quantity {:?} is not a vector-layer quantity",
            layer.metadata.quantity
        ));
    }
    if !layer.metadata.display_range.min.is_finite() || layer.metadata.display_range.min < 0.0 {
        return Err("vector magnitude display minimum must be non-negative".to_owned());
    }
    if layer.samples.len() > MAX_FIELD_SAMPLES {
        return Err(format!(
            "field layer exceeds maximum of {MAX_FIELD_SAMPLES} samples"
        ));
    }
    if !layer.vector_scale.is_finite()
        || layer.vector_scale < 0.0
        || !layer.max_vector_length.is_finite()
        || layer.max_vector_length < 0.0
    {
        return Err("vector scale and maximum length must be finite and non-negative".to_owned());
    }
    Ok(())
}

fn project_position(
    position: [f64; 3],
    spatial_unit: SpatialUnit,
    world_units_per_meter: f64,
) -> Result<[f32; 3], String> {
    let source_to_meters = match spatial_unit {
        SpatialUnit::Meter => 1.0,
        SpatialUnit::Centimeter => 0.01,
        SpatialUnit::Millimeter => 0.001,
        SpatialUnit::WorldUnit => return project_world_position(position),
    };
    let scale = source_to_meters * world_units_per_meter;
    let scaled = position.map(|value| value * scale);
    project_world_position(scaled)
}

fn project_world_position(position: [f64; 3]) -> Result<[f32; 3], String> {
    let mut projected = [0.0; 3];
    for (index, value) in position.iter().enumerate() {
        if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
            return Err("field sample has a non-finite or unrepresentable position".to_owned());
        }
        projected[index] = *value as f32;
    }
    Ok(projected)
}

fn validate_finite_position(position: [f64; 3]) -> Result<(), String> {
    if position.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err("spectrum profile has a non-finite position".to_owned())
    }
}

/// Optional read-only renderer. Input layers are producer-supplied observations;
/// the plugin never simulates or modifies their values.
pub struct FieldVisualizationPlugin;

impl Plugin for FieldVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldLayerSet>()
            .add_systems(Update, draw_field_layers);
    }
}

fn draw_field_layers(mut gizmos: Gizmos, layers: Res<FieldLayerSet>) {
    if !layers.enabled
        || layers.scalar_layers.len() > MAX_FIELD_LAYERS
        || layers.vector_layers.len() > MAX_FIELD_LAYERS
    {
        return;
    }

    let scalar_total_budget = layers
        .max_scalar_markers_total
        .min(MAX_SCALAR_MARKERS_TOTAL);
    let mut total_scalar_markers = 0;
    for layer in layers.scalar_layers.iter().filter(|layer| layer.enabled) {
        let remaining = scalar_total_budget.saturating_sub(total_scalar_markers);
        if remaining == 0 {
            break;
        }
        let glyphs = match extract_scalar_glyphs(
            layer,
            layers
                .max_scalar_markers_per_layer
                .min(MAX_SCALAR_MARKERS_PER_LAYER)
                .min(remaining),
        ) {
            Ok(glyphs) => glyphs,
            Err(_) => continue,
        };
        total_scalar_markers += glyphs.len();
        let extent = layer.marker_radius.clamp(0.0, 10.0);
        if extent <= 0.0 {
            continue;
        }
        for glyph in glyphs {
            let point = Vec3::from_array(glyph.position);
            let color = Color::srgb(glyph.color[0], glyph.color[1], glyph.color[2]);
            gizmos.line(point - Vec3::X * extent, point + Vec3::X * extent, color);
            gizmos.line(point - Vec3::Y * extent, point + Vec3::Y * extent, color);
            gizmos.line(point - Vec3::Z * extent, point + Vec3::Z * extent, color);
        }
    }

    let vector_total_budget = layers.max_vector_glyphs_total.min(MAX_VECTOR_GLYPHS_TOTAL);
    let mut total_vector_glyphs = 0;
    for layer in layers.vector_layers.iter().filter(|layer| layer.enabled) {
        let remaining = vector_total_budget.saturating_sub(total_vector_glyphs);
        if remaining == 0 {
            break;
        }
        let glyphs = match extract_vector_glyphs(
            layer,
            layers
                .max_vector_glyphs_per_layer
                .min(MAX_VECTOR_GLYPHS_PER_LAYER)
                .min(remaining),
        ) {
            Ok(glyphs) => glyphs,
            Err(_) => continue,
        };
        total_vector_glyphs += glyphs.len();
        let scale = layer.vector_scale.clamp(0.0, 100.0);
        let max_length = layer.max_vector_length.clamp(0.0, 100.0);
        if scale <= 0.0 || max_length <= 0.0 {
            continue;
        }
        for glyph in glyphs {
            let direction = Vec3::from_array(glyph.direction);
            if direction.length_squared() <= f32::EPSILON {
                continue;
            }
            let start = Vec3::from_array(glyph.position);
            let length = (glyph.normalized_magnitude * scale).clamp(0.0, max_length);
            if length <= 0.0 {
                continue;
            }
            let end = start + direction * length;
            let color = Color::srgb(glyph.color[0], glyph.color[1], glyph.color[2]);
            gizmos.line(start, end, color);

            let helper = if direction.dot(Vec3::Y).abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let side = direction.cross(helper).normalize_or_zero();
            let head_length = (length * 0.22).min(0.12);
            let head_width = head_length * 0.55;
            gizmos.line(
                end,
                end - direction * head_length + side * head_width,
                color,
            );
            gizmos.line(
                end,
                end - direction * head_length - side * head_width,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(
        quantity: PhysicalQuantity,
        unit: FieldUnit,
        range: DisplayRange,
    ) -> FieldLayerMetadata {
        FieldLayerMetadata {
            id: "test-layer".to_owned(),
            quantity,
            unit,
            origin: FieldDataOrigin::Simulated,
            calibration: CalibrationStatus::NotApplicable,
            source_label: "deterministic test fixture".to_owned(),
            calibration_reference: None,
            coordinate_frame: "world".to_owned(),
            spatial_unit: SpatialUnit::Meter,
            world_units_per_meter: 1.0,
            timestamp_unix_ns: Some(1_000_000),
            wavelength_band: None,
            frequency_hz: None,
            display_range: range,
            color_map: FieldColorMap::Thermal,
        }
    }

    #[test]
    fn temperature_layer_keeps_units_and_fixed_scale_explicit() {
        let layer = ScalarFieldLayer {
            metadata: metadata(
                PhysicalQuantity::Temperature,
                FieldUnit::Celsius,
                DisplayRange {
                    min: 0.0,
                    max: 100.0,
                },
            ),
            enabled: true,
            marker_radius: 0.03,
            samples: vec![
                ScalarFieldSample {
                    position: [0.0, 0.0, 0.0],
                    value: 0.0,
                    uncertainty: Some(0.5),
                },
                ScalarFieldSample {
                    position: [1.0, 0.0, 0.0],
                    value: 100.0,
                    uncertainty: Some(0.5),
                },
            ],
        };
        let glyphs = extract_scalar_glyphs(&layer, 10).expect("valid temperature layer");
        assert_eq!(glyphs.len(), 2);
        assert_eq!(glyphs[0].normalized_value, 0.0);
        assert_eq!(glyphs[1].normalized_value, 1.0);
        assert_ne!(glyphs[0].color, glyphs[1].color);
    }

    #[test]
    fn spatial_units_are_converted_before_scene_projection() {
        let mut layer = ScalarFieldLayer {
            metadata: metadata(
                PhysicalQuantity::Temperature,
                FieldUnit::Celsius,
                DisplayRange {
                    min: 0.0,
                    max: 100.0,
                },
            ),
            enabled: true,
            marker_radius: 0.03,
            samples: vec![ScalarFieldSample {
                position: [100.0, 200.0, 300.0],
                value: 25.0,
                uncertainty: None,
            }],
        };
        layer.metadata.spatial_unit = SpatialUnit::Centimeter;
        layer.metadata.world_units_per_meter = 1.0;
        let glyph = extract_scalar_glyphs(&layer, 1).expect("valid centimeter coordinates")[0];
        assert_eq!(glyph.position, [1.0, 2.0, 3.0]);

        layer.metadata.spatial_unit = SpatialUnit::WorldUnit;
        let glyph = extract_scalar_glyphs(&layer, 1).expect("valid world coordinates")[0];
        assert_eq!(glyph.position, [100.0, 200.0, 300.0]);
    }

    #[test]
    fn unit_mismatch_fails_closed() {
        let layer = ScalarFieldLayer {
            metadata: metadata(
                PhysicalQuantity::ElectricFieldMagnitude,
                FieldUnit::Tesla,
                DisplayRange {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            enabled: true,
            marker_radius: 0.03,
            samples: vec![],
        };
        assert!(extract_scalar_glyphs(&layer, 10).is_err());
    }

    #[test]
    fn calibrated_sensor_data_requires_traceable_reference() {
        let mut meta = metadata(
            PhysicalQuantity::Temperature,
            FieldUnit::Kelvin,
            DisplayRange {
                min: 200.0,
                max: 400.0,
            },
        );
        meta.origin = FieldDataOrigin::MeasuredSensor;
        meta.calibration = CalibrationStatus::Calibrated;
        assert!(meta.validate().is_err());
        meta.calibration_reference = Some("blackbody calibrator certificate REF-001".to_owned());
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn harmonic_frequency_must_agree_with_declared_spectral_band() {
        let mut meta = metadata(
            PhysicalQuantity::ElectricFieldVector,
            FieldUnit::VoltPerMeter,
            DisplayRange {
                min: 0.0,
                max: 100.0,
            },
        );
        meta.frequency_hz = Some(2.4e9);
        meta.wavelength_band = Some(WavelengthBand {
            min_wavelength_m: 0.12,
            max_wavelength_m: 0.13,
        });
        assert!(meta.validate().is_ok());
        meta.frequency_hz = Some(1.0e12);
        assert!(meta.validate().is_err());
    }

    #[test]
    fn spectral_radiance_requires_wavelength_band_and_correct_units() {
        let mut meta = metadata(
            PhysicalQuantity::SpectralRadiance,
            FieldUnit::WattPerSquareMeterSteradianPerNanometer,
            DisplayRange {
                min: 0.0,
                max: 10.0,
            },
        );
        assert!(meta.validate().is_err());
        meta.wavelength_band = Some(WavelengthBand {
            min_wavelength_m: 8.0e-6,
            max_wavelength_m: 14.0e-6,
        });
        assert!(meta.validate().is_ok());
        let (low, high) = meta
            .wavelength_band
            .expect("test band")
            .frequency_range_hz()
            .expect("valid band");
        assert!(low < high);
    }

    fn spectrum_profile() -> SpectrumProfile {
        SpectrumProfile {
            id: "spectral-probe-1".to_owned(),
            unit: FieldUnit::WattPerSquareMeterSteradianPerNanometer,
            origin: FieldDataOrigin::MeasuredSensor,
            calibration: CalibrationStatus::Calibrated,
            source_label: "calibrated hyperspectral sensor".to_owned(),
            calibration_reference: Some("traceable radiance standard CERT-123".to_owned()),
            coordinate_frame: "sensor-world".to_owned(),
            spatial_unit: SpatialUnit::Meter,
            timestamp_unix_ns: Some(1_800_000_000_000_000_000),
            position: [0.0, 0.0, 0.0],
            bins: vec![
                SpectrumBin {
                    wavelength_m: 400.0e-9,
                    spectral_radiance: 1.2,
                    uncertainty: Some(0.1),
                },
                SpectrumBin {
                    wavelength_m: 550.0e-9,
                    spectral_radiance: 2.4,
                    uncertainty: Some(0.1),
                },
                SpectrumBin {
                    wavelength_m: 700.0e-9,
                    spectral_radiance: 1.7,
                    uncertainty: Some(0.12),
                },
            ],
        }
    }

    #[test]
    fn spectrum_profile_preserves_ordered_wavelength_data() {
        let profile = spectrum_profile();
        assert!(validate_spectrum_profile(&profile).is_ok());
        let band = profile.wavelength_band().expect("valid profile has a band");
        assert_eq!(band.min_wavelength_m, 400.0e-9);
        assert_eq!(band.max_wavelength_m, 700.0e-9);
    }

    #[test]
    fn spectrum_rejects_duplicate_wavelength_and_wrong_unit() {
        let mut profile = spectrum_profile();
        profile.bins[1].wavelength_m = profile.bins[0].wavelength_m;
        assert!(validate_spectrum_profile(&profile).is_err());

        let mut profile = spectrum_profile();
        profile.unit = FieldUnit::VoltPerMeter;
        assert!(validate_spectrum_profile(&profile).is_err());
    }

    #[test]
    fn calibrated_spectrum_requires_reference_and_rejects_non_finite_radiance() {
        let mut profile = spectrum_profile();
        profile.calibration_reference = None;
        assert!(validate_spectrum_profile(&profile).is_err());

        let mut profile = spectrum_profile();
        profile.bins[1].spectral_radiance = f64::NAN;
        assert!(validate_spectrum_profile(&profile).is_err());
    }

    #[test]
    fn vector_field_uses_direction_and_separate_magnitude_scale() {
        let layer = VectorFieldLayer {
            metadata: metadata(
                PhysicalQuantity::ElectricFieldVector,
                FieldUnit::VoltPerMeter,
                DisplayRange {
                    min: 0.0,
                    max: 100.0,
                },
            ),
            enabled: true,
            vector_scale: 1.0,
            max_vector_length: 1.0,
            samples: vec![VectorFieldSample {
                position: [1.0, 2.0, 3.0],
                vector: [3.0, 4.0, 0.0],
                magnitude_uncertainty: Some(0.2),
            }],
        };
        let glyphs = extract_vector_glyphs(&layer, 5).expect("valid vector layer");
        assert_eq!(glyphs.len(), 1);
        assert!((glyphs[0].magnitude - 5.0).abs() < 1e-9);
        assert!((glyphs[0].direction[0] - 0.6).abs() < 1e-6);
        assert!((glyphs[0].direction[1] - 0.8).abs() < 1e-6);
        assert!((glyphs[0].normalized_magnitude - 0.05).abs() < 1e-6);
    }

    #[test]
    fn invalid_numeric_samples_are_rejected() {
        let layer = ScalarFieldLayer {
            metadata: metadata(
                PhysicalQuantity::Temperature,
                FieldUnit::Kelvin,
                DisplayRange {
                    min: 200.0,
                    max: 400.0,
                },
            ),
            enabled: true,
            marker_radius: 0.03,
            samples: vec![ScalarFieldSample {
                position: [0.0, f64::NAN, 0.0],
                value: 300.0,
                uncertainty: None,
            }],
        };
        assert!(extract_scalar_glyphs(&layer, 10).is_err());
    }

    #[test]
    fn wavelength_and_frequency_are_consistent_for_visible_band() {
        let band = WavelengthBand {
            min_wavelength_m: 380.0e-9,
            max_wavelength_m: 700.0e-9,
        };
        let (min_frequency, max_frequency) = band.frequency_range_hz().expect("valid optical band");
        assert!(min_frequency < max_frequency);
        assert!((min_frequency - SPEED_OF_LIGHT_M_PER_S / 700.0e-9).abs() < 1.0);
        assert!((max_frequency - SPEED_OF_LIGHT_M_PER_S / 380.0e-9).abs() < 1.0);
    }

    #[test]
    fn glyph_budgets_are_honoured() {
        let layer = ScalarFieldLayer {
            metadata: metadata(
                PhysicalQuantity::Temperature,
                FieldUnit::Kelvin,
                DisplayRange {
                    min: 200.0,
                    max: 400.0,
                },
            ),
            enabled: true,
            marker_radius: 0.03,
            samples: (0..5)
                .map(|index| ScalarFieldSample {
                    position: [index as f64, 0.0, 0.0],
                    value: 250.0,
                    uncertainty: None,
                })
                .collect(),
        };
        assert_eq!(extract_scalar_glyphs(&layer, 2).expect("bounded").len(), 2);
        assert!(
            extract_scalar_glyphs(&layer, 0)
                .expect("zero budget")
                .is_empty()
        );
    }

    #[test]
    fn colormap_output_is_bounded_and_deterministic() {
        for map in [
            FieldColorMap::Thermal,
            FieldColorMap::ViridisLike,
            FieldColorMap::Diverging,
            FieldColorMap::Grayscale,
        ] {
            let a = map.map_normalized(0.37);
            let b = map.map_normalized(0.37);
            assert_eq!(a, b);
            assert!(
                a.iter()
                    .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
            );
        }
    }
}
