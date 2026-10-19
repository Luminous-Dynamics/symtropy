// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Unit-aware, read-only visualization for measured or simulated physical fields.
//!
//! This module provides a rendering adapter, not a field solver. Thermal values
//! must be ingested as calibrated temperatures or radiometric values; electric
//! and magnetic vectors must come from a declared measurement or simulation.
//! The module never invents field values or infers temperature from PBR color.

use bevy::prelude::*;

pub const MAX_FIELD_SAMPLES: usize = 1_000_000;
const MAX_RENDERED_SAMPLES: usize = 10_000;

/// Declares the physical meaning and canonical unit of scalar samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarQuantity {
    TemperatureKelvin,
    RadiantExitanceWPerM2,
    SpectralRadianceWPerSrM2Um,
    ReflectanceFraction,
    ElectricFieldMagnitudeVPerM,
    MagneticFluxDensityMagnitudeTesla,
    GenericScalar,
}

impl ScalarQuantity {
    pub const fn unit(self) -> &'static str {
        match self {
            Self::TemperatureKelvin => "K",
            Self::RadiantExitanceWPerM2 => "W/m^2",
            Self::SpectralRadianceWPerSrM2Um => "W/(sr m^2 um)",
            Self::ReflectanceFraction => "1",
            Self::ElectricFieldMagnitudeVPerM => "V/m",
            Self::MagneticFluxDensityMagnitudeTesla => "T",
            Self::GenericScalar => "declared-by-caller",
        }
    }

    const fn requires_spectral_band(self) -> bool {
        matches!(
            self,
            Self::SpectralRadianceWPerSrM2Um | Self::ReflectanceFraction
        )
    }

    const fn is_nonnegative(self) -> bool {
        !matches!(self, Self::GenericScalar)
    }
}

/// Spectral identity for a channel. FWHM is bandwidth, not spectral resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralBand {
    pub center_wavelength_nm: f64,
    pub bandwidth_fwhm_nm: f64,
}

impl SpectralBand {
    pub fn validate(self) -> Result<(), FieldValidationError> {
        if !self.center_wavelength_nm.is_finite()
            || !self.bandwidth_fwhm_nm.is_finite()
            || self.center_wavelength_nm <= 0.0
            || self.bandwidth_fwhm_nm <= 0.0
            || self.bandwidth_fwhm_nm >= self.center_wavelength_nm * 2.0
        {
            return Err(FieldValidationError::InvalidSpectralBand);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldSourceKind {
    MeasuredSensor,
    PhysicsSimulation,
    DerivedProduct,
    SyntheticFixture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationState {
    Calibrated,
    Uncalibrated,
    NotApplicable,
}

/// Provenance accompanies field data instead of being painted into a legend only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProvenance {
    pub source_kind: FieldSourceKind,
    /// Sensor, simulation, or transformation identifier.
    pub source_id: String,
    pub calibration_state: CalibrationState,
    pub calibration_id: Option<String>,
    pub calibration_sha256: Option<String>,
}

impl FieldProvenance {
    pub const fn is_uncalibrated_measurement(&self) -> bool {
        matches!(self.source_kind, FieldSourceKind::MeasuredSensor)
            && matches!(self.calibration_state, CalibrationState::Uncalibrated)
    }

    pub fn validate(&self) -> Result<(), FieldValidationError> {
        if self.source_id.trim().is_empty() {
            return Err(FieldValidationError::MissingSourceId);
        }
        if self.source_kind == FieldSourceKind::MeasuredSensor
            && self.calibration_state == CalibrationState::NotApplicable
        {
            return Err(FieldValidationError::InvalidCalibrationState);
        }
        if self.calibration_state == CalibrationState::Calibrated {
            let calibration_id = self
                .calibration_id
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            let digest = self
                .calibration_sha256
                .as_deref()
                .filter(|value| is_sha256(value));
            if calibration_id.is_none() || digest.is_none() {
                return Err(FieldValidationError::MissingCalibrationEvidence);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarRange {
    pub minimum: f64,
    pub maximum: f64,
}

impl ScalarRange {
    pub fn validate(self) -> Result<(), FieldValidationError> {
        if !self.minimum.is_finite() || !self.maximum.is_finite() || self.minimum >= self.maximum {
            return Err(FieldValidationError::InvalidDisplayRange);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarPalette {
    Thermal,
    Viridis,
    Diverging,
    Grayscale,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarFieldSample {
    /// World-space position in metres after caller-applied sensor/world registration.
    pub position_world_m: [f64; 3],
    pub value: f64,
    /// Standard uncertainty in the same unit as the sample value, if available.
    pub standard_uncertainty: Option<f64>,
}

/// A scalar field is data plus physical meaning, a display range, and provenance.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ScalarFieldFrame {
    pub quantity: ScalarQuantity,
    pub samples: Vec<ScalarFieldSample>,
    pub display_range: ScalarRange,
    pub palette: ScalarPalette,
    pub spectral_band: Option<SpectralBand>,
    pub provenance: FieldProvenance,
}

impl ScalarFieldFrame {
    pub fn validate(&self) -> Result<(), FieldValidationError> {
        self.display_range.validate()?;
        self.provenance.validate()?;
        if self.samples.is_empty() || self.samples.len() > MAX_FIELD_SAMPLES {
            return Err(FieldValidationError::InvalidSampleCount);
        }
        if self.quantity.requires_spectral_band() {
            self.spectral_band
                .ok_or(FieldValidationError::MissingSpectralBand)?
                .validate()?;
        } else if let Some(band) = self.spectral_band {
            band.validate()?;
        }

        if self.quantity.is_nonnegative() && self.display_range.minimum < 0.0 {
            return Err(FieldValidationError::InvalidPhysicalValue);
        }
        if self.quantity == ScalarQuantity::ReflectanceFraction
            && (self.display_range.minimum < 0.0 || self.display_range.maximum > 1.0)
        {
            return Err(FieldValidationError::InvalidPhysicalValue);
        }

        for sample in &self.samples {
            validate_position(sample.position_world_m)?;
            if !sample.value.is_finite() || sample.value.abs() > f64::from(f32::MAX) {
                return Err(FieldValidationError::InvalidSampleValue);
            }
            if self.quantity.is_nonnegative() && sample.value < 0.0 {
                return Err(FieldValidationError::InvalidPhysicalValue);
            }
            if self.quantity == ScalarQuantity::ReflectanceFraction
                && !(0.0..=1.0).contains(&sample.value)
            {
                return Err(FieldValidationError::InvalidPhysicalValue);
            }
            if let Some(uncertainty) = sample.standard_uncertainty
                && (!uncertainty.is_finite() || uncertainty < 0.0)
            {
                return Err(FieldValidationError::InvalidUncertainty);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectralScalarQuantity {
    ReflectanceFraction,
    SpectralRadianceWPerSrM2Um,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralChannelSpec {
    pub band: SpectralBand,
    pub display_range: ScalarRange,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpectralSample {
    pub position_world_m: [f64; 3],
    /// One sample per channel, in the frame's declared scalar quantity.
    pub values: Vec<f64>,
    pub standard_uncertainties: Vec<Option<f64>>,
}

/// Maps three chosen measured/simulated bands to display RGB channels.
///
/// This is an explicitly chosen false-colour/pseudocolour composite, not a
/// colourimetric reconstruction of human vision or a complete hyperspectral product.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct SpectralCompositeFrame {
    pub quantity: SpectralScalarQuantity,
    pub channels: Vec<SpectralChannelSpec>,
    pub samples: Vec<SpectralSample>,
    /// Indices of input channels used for display red, green and blue.
    pub rgb_channel_indices: [usize; 3],
    pub provenance: FieldProvenance,
}

impl SpectralCompositeFrame {
    pub fn validate(&self) -> Result<(), FieldValidationError> {
        self.provenance.validate()?;
        if self.channels.len() < 3 || self.channels.len() > 256 {
            return Err(FieldValidationError::InvalidChannelCount);
        }
        if self.samples.is_empty() || self.samples.len() > MAX_FIELD_SAMPLES {
            return Err(FieldValidationError::InvalidSampleCount);
        }
        let [red, green, blue] = self.rgb_channel_indices;
        if red >= self.channels.len()
            || green >= self.channels.len()
            || blue >= self.channels.len()
            || red == green
            || red == blue
            || green == blue
        {
            return Err(FieldValidationError::InvalidSpectralMapping);
        }

        for channel in &self.channels {
            channel.band.validate()?;
            channel.display_range.validate()?;
            if channel.display_range.minimum < 0.0
                || (self.quantity == SpectralScalarQuantity::ReflectanceFraction
                    && channel.display_range.maximum > 1.0)
            {
                return Err(FieldValidationError::InvalidPhysicalValue);
            }
        }
        for sample in &self.samples {
            validate_position(sample.position_world_m)?;
            if sample.values.len() != self.channels.len()
                || sample.standard_uncertainties.len() != self.channels.len()
            {
                return Err(FieldValidationError::ChannelSampleCountMismatch);
            }
            for (channel, value) in self.channels.iter().zip(&sample.values) {
                if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
                    return Err(FieldValidationError::InvalidSampleValue);
                }
                if *value < 0.0
                    || (self.quantity == SpectralScalarQuantity::ReflectanceFraction
                        && *value > 1.0)
                    || (self.quantity == SpectralScalarQuantity::ReflectanceFraction
                        && (channel.display_range.minimum < 0.0
                            || channel.display_range.maximum > 1.0))
                {
                    return Err(FieldValidationError::InvalidPhysicalValue);
                }
            }
            for uncertainty in &sample.standard_uncertainties {
                if let Some(uncertainty) = uncertainty
                    && (!uncertainty.is_finite() || *uncertainty < 0.0)
                {
                    return Err(FieldValidationError::InvalidUncertainty);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralGlyph {
    pub position: [f32; 3],
    pub rgb: [f32; 3],
    pub selected_values: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorQuantity {
    ElectricFieldVoltsPerMeter,
    MagneticFluxDensityTesla,
    VelocityMetersPerSecond,
    GenericVector,
}

impl VectorQuantity {
    pub const fn unit(self) -> &'static str {
        match self {
            Self::ElectricFieldVoltsPerMeter => "V/m",
            Self::MagneticFluxDensityTesla => "T",
            Self::VelocityMetersPerSecond => "m/s",
            Self::GenericVector => "declared-by-caller",
        }
    }

    const fn display_color(self) -> [f32; 3] {
        match self {
            Self::ElectricFieldVoltsPerMeter => [1.0, 0.38, 0.12],
            Self::MagneticFluxDensityTesla => [0.15, 0.82, 0.92],
            Self::VelocityMetersPerSecond => [0.25, 0.9, 0.42],
            Self::GenericVector => [0.92, 0.92, 0.92],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorFieldSample {
    pub position_world_m: [f64; 3],
    /// Components in the declared quantity's unit, in world-axis order.
    pub vector: [f64; 3],
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct VectorFieldFrame {
    pub quantity: VectorQuantity,
    pub samples: Vec<VectorFieldSample>,
    pub provenance: FieldProvenance,
}

impl VectorFieldFrame {
    pub fn validate(&self) -> Result<(), FieldValidationError> {
        self.provenance.validate()?;
        if self.samples.is_empty() || self.samples.len() > MAX_FIELD_SAMPLES {
            return Err(FieldValidationError::InvalidSampleCount);
        }
        for sample in &self.samples {
            validate_position(sample.position_world_m)?;
            for component in sample.vector {
                if !component.is_finite() || component.abs() > f64::from(f32::MAX) {
                    return Err(FieldValidationError::InvalidSampleValue);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarGlyph {
    pub position: [f32; 3],
    pub value: f64,
    pub normalized_value: f32,
    pub color: [f32; 3],
    pub standard_uncertainty: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorGlyph {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub magnitude: f32,
    pub display_length: f32,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldValidationError {
    InvalidDisplayRange,
    InvalidSpectralBand,
    MissingSpectralBand,
    MissingSourceId,
    InvalidCalibrationState,
    MissingCalibrationEvidence,
    InvalidSampleCount,
    InvalidPosition,
    InvalidSampleValue,
    InvalidPhysicalValue,
    InvalidUncertainty,
    InvalidGlyphScale,
    InvalidChannelCount,
    InvalidSpectralMapping,
    ChannelSampleCountMismatch,
}

impl std::fmt::Display for FieldValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FieldValidationError {}

#[derive(Resource, Clone, Debug)]
pub struct FieldVisualizationConfig {
    pub enabled: bool,
    pub show_scalar_fields: bool,
    pub show_spectral_composite: bool,
    pub show_vector_fields: bool,
    pub max_spectral_samples: usize,
    pub max_scalar_samples: usize,
    pub max_vector_samples: usize,
    pub scalar_cross_half_extent: f32,
    pub vector_scale: f32,
    pub max_vector_length: f32,
}

impl Default for FieldVisualizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_scalar_fields: true,
            show_spectral_composite: true,
            show_vector_fields: true,
            max_spectral_samples: 4096,
            max_scalar_samples: 4096,
            max_vector_samples: 2048,
            scalar_cross_half_extent: 0.035,
            vector_scale: 0.2,
            max_vector_length: 1.5,
        }
    }
}

/// Machine-readable status so malformed sensor data never disappears silently.
#[derive(Resource, Clone, Debug, Default)]
pub struct FieldVisualizationStatus {
    pub scalar_error: Option<FieldValidationError>,
    pub spectral_error: Option<FieldValidationError>,
    pub vector_error: Option<FieldValidationError>,
    pub scalar_is_uncalibrated_measurement: bool,
    pub spectral_is_uncalibrated_measurement: bool,
    pub vector_is_uncalibrated_measurement: bool,
    pub scalar_glyph_count: usize,
    pub spectral_glyph_count: usize,
    pub vector_glyph_count: usize,
}

pub struct FieldObservabilityPlugin;

impl Plugin for FieldObservabilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldVisualizationConfig>()
            .init_resource::<FieldVisualizationStatus>()
            .add_systems(Update, draw_field_views);
    }
}

/// Convert a field value to a clipped, named pseudocolor. Returns None for
/// malformed ranges/values; clipping is a display choice, never data mutation.
pub fn scalar_color(value: f64, range: ScalarRange, palette: ScalarPalette) -> Option<[f32; 3]> {
    if !value.is_finite() || range.validate().is_err() {
        return None;
    }
    let t = ((value - range.minimum) / (range.maximum - range.minimum)).clamp(0.0, 1.0);
    let stops: &[[f32; 3]] = match palette {
        ScalarPalette::Thermal => &[
            [0.03, 0.01, 0.12],
            [0.22, 0.02, 0.35],
            [0.62, 0.04, 0.30],
            [0.94, 0.25, 0.08],
            [1.0, 0.78, 0.22],
            [1.0, 0.98, 0.86],
        ],
        ScalarPalette::Viridis => &[
            [0.267, 0.005, 0.329],
            [0.230, 0.322, 0.546],
            [0.128, 0.567, 0.551],
            [0.369, 0.789, 0.383],
            [0.993, 0.906, 0.144],
        ],
        ScalarPalette::Diverging => &[
            [0.12, 0.28, 0.75],
            [0.48, 0.70, 0.88],
            [0.94, 0.94, 0.90],
            [0.91, 0.54, 0.40],
            [0.65, 0.08, 0.13],
        ],
        ScalarPalette::Grayscale => &[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
    };
    let segments = stops.len() - 1;
    let scaled = t * segments as f64;
    let lower = (scaled.floor() as usize).min(segments - 1);
    let fraction = (scaled - lower as f64) as f32;
    Some(std::array::from_fn(|channel| {
        stops[lower][channel] + (stops[lower + 1][channel] - stops[lower][channel]) * fraction
    }))
}

pub fn extract_scalar_glyphs(
    frame: &ScalarFieldFrame,
    max_samples: usize,
) -> Result<Vec<ScalarGlyph>, FieldValidationError> {
    frame.validate()?;
    let cap = max_samples.min(MAX_RENDERED_SAMPLES);
    let mut glyphs = Vec::with_capacity(cap.min(frame.samples.len()));
    for sample in frame.samples.iter().take(cap) {
        let color = scalar_color(sample.value, frame.display_range, frame.palette)
            .ok_or(FieldValidationError::InvalidSampleValue)?;
        let normalized = ((sample.value - frame.display_range.minimum)
            / (frame.display_range.maximum - frame.display_range.minimum))
            .clamp(0.0, 1.0) as f32;
        glyphs.push(ScalarGlyph {
            position: to_f32_position(sample.position_world_m)?,
            value: sample.value,
            normalized_value: normalized,
            color,
            standard_uncertainty: sample.standard_uncertainty,
        });
    }
    Ok(glyphs)
}

pub fn extract_spectral_glyphs(
    frame: &SpectralCompositeFrame,
    max_samples: usize,
) -> Result<Vec<SpectralGlyph>, FieldValidationError> {
    frame.validate()?;
    let cap = max_samples.min(MAX_RENDERED_SAMPLES);
    let [red, green, blue] = frame.rgb_channel_indices;
    let selected = [red, green, blue];
    let mut glyphs = Vec::with_capacity(cap.min(frame.samples.len()));

    for sample in frame.samples.iter().take(cap) {
        let rgb = std::array::from_fn(|output_channel| {
            let input_channel = selected[output_channel];
            let spec = frame.channels[input_channel];
            ((sample.values[input_channel] - spec.display_range.minimum)
                / (spec.display_range.maximum - spec.display_range.minimum))
                .clamp(0.0, 1.0) as f32
        });
        glyphs.push(SpectralGlyph {
            position: to_f32_position(sample.position_world_m)?,
            rgb,
            selected_values: [
                sample.values[red],
                sample.values[green],
                sample.values[blue],
            ],
        });
    }
    Ok(glyphs)
}

pub fn extract_vector_glyphs(
    frame: &VectorFieldFrame,
    max_samples: usize,
    vector_scale: f32,
    max_vector_length: f32,
) -> Result<Vec<VectorGlyph>, FieldValidationError> {
    frame.validate()?;
    if !vector_scale.is_finite()
        || vector_scale < 0.0
        || !max_vector_length.is_finite()
        || max_vector_length < 0.0
    {
        return Err(FieldValidationError::InvalidGlyphScale);
    }

    let cap = max_samples.min(MAX_RENDERED_SAMPLES);
    let mut glyphs = Vec::with_capacity(cap.min(frame.samples.len()));
    for sample in frame.samples.iter().take(cap) {
        let magnitude = sample
            .vector
            .iter()
            .fold(0.0_f64, |accumulator, value| accumulator.hypot(*value));
        if !magnitude.is_finite() || magnitude <= f64::EPSILON {
            continue;
        }
        let direction = sample
            .vector
            .map(|component| (component / magnitude) as f32);
        let magnitude_f32 = magnitude.min(f64::from(f32::MAX)) as f32;
        glyphs.push(VectorGlyph {
            origin: to_f32_position(sample.position_world_m)?,
            direction,
            magnitude: magnitude_f32,
            display_length: (magnitude_f32 * vector_scale).min(max_vector_length),
            color: frame.quantity.display_color(),
        });
    }
    Ok(glyphs)
}

fn draw_field_views(
    scalar_frame: Option<Res<ScalarFieldFrame>>,
    spectral_frame: Option<Res<SpectralCompositeFrame>>,
    vector_frame: Option<Res<VectorFieldFrame>>,
    config: Res<FieldVisualizationConfig>,
    mut status: ResMut<FieldVisualizationStatus>,
    mut gizmos: Gizmos,
) {
    *status = FieldVisualizationStatus::default();
    if !config.enabled {
        return;
    }

    if config.show_scalar_fields
        && let Some(frame) = scalar_frame.as_deref()
    {
        status.scalar_is_uncalibrated_measurement = frame.provenance.is_uncalibrated_measurement();
        match extract_scalar_glyphs(frame, config.max_scalar_samples) {
            Ok(glyphs) => {
                status.scalar_glyph_count = glyphs.len();
                let extent = if config.scalar_cross_half_extent.is_finite() {
                    config.scalar_cross_half_extent.clamp(0.0, 10.0)
                } else {
                    0.0
                };
                if extent > 0.0 {
                    for glyph in glyphs {
                        let p = Vec3::from_array(glyph.position);
                        let color = Color::srgb(glyph.color[0], glyph.color[1], glyph.color[2]);
                        gizmos.line(p - Vec3::X * extent, p + Vec3::X * extent, color);
                        gizmos.line(p - Vec3::Y * extent, p + Vec3::Y * extent, color);
                        gizmos.line(p - Vec3::Z * extent, p + Vec3::Z * extent, color);
                    }
                }
            }
            Err(error) => status.scalar_error = Some(error),
        }
    }

    if config.show_spectral_composite
        && let Some(frame) = spectral_frame.as_deref()
    {
        status.spectral_is_uncalibrated_measurement = frame.provenance.source_kind
            == FieldSourceKind::MeasuredSensor
            && frame.provenance.calibration_state == CalibrationState::Uncalibrated;
        match extract_spectral_glyphs(frame, config.max_spectral_samples) {
            Ok(glyphs) => {
                status.spectral_glyph_count = glyphs.len();
                let extent = if config.scalar_cross_half_extent.is_finite() {
                    config.scalar_cross_half_extent.clamp(0.0, 10.0)
                } else {
                    0.0
                };
                if extent > 0.0 {
                    for glyph in glyphs {
                        let p = Vec3::from_array(glyph.position);
                        let color = Color::srgb(glyph.rgb[0], glyph.rgb[1], glyph.rgb[2]);
                        gizmos.line(p - Vec3::X * extent, p + Vec3::X * extent, color);
                        gizmos.line(p - Vec3::Y * extent, p + Vec3::Y * extent, color);
                        gizmos.line(p - Vec3::Z * extent, p + Vec3::Z * extent, color);
                    }
                }
            }
            Err(error) => status.spectral_error = Some(error),
        }
    }

    if config.show_vector_fields
        && let Some(frame) = vector_frame.as_deref()
    {
        status.vector_is_uncalibrated_measurement = frame.provenance.source_kind
            == FieldSourceKind::MeasuredSensor
            && frame.provenance.calibration_state == CalibrationState::Uncalibrated;
        match extract_vector_glyphs(
            frame,
            config.max_vector_samples,
            config.vector_scale,
            config.max_vector_length,
        ) {
            Ok(glyphs) => {
                status.vector_glyph_count = glyphs.len();
                for glyph in glyphs {
                    if glyph.display_length <= 0.0 {
                        continue;
                    }
                    let p = Vec3::from_array(glyph.origin);
                    let direction = Vec3::from_array(glyph.direction);
                    let color = Color::srgb(glyph.color[0], glyph.color[1], glyph.color[2]);
                    gizmos.line(p, p + direction * glyph.display_length, color);
                }
            }
            Err(error) => status.vector_error = Some(error),
        }
    }
}

fn validate_position(position: [f64; 3]) -> Result<(), FieldValidationError> {
    for component in position {
        if !component.is_finite() || component.abs() > f64::from(f32::MAX) {
            return Err(FieldValidationError::InvalidPosition);
        }
    }
    Ok(())
}

fn to_f32_position(position: [f64; 3]) -> Result<[f32; 3], FieldValidationError> {
    validate_position(position)?;
    Ok(position.map(|component| component as f32))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulation_provenance() -> FieldProvenance {
        FieldProvenance {
            source_kind: FieldSourceKind::PhysicsSimulation,
            source_id: "heat-transfer-fixture-v1".to_owned(),
            calibration_state: CalibrationState::NotApplicable,
            calibration_id: None,
            calibration_sha256: None,
        }
    }

    fn temperature_frame() -> ScalarFieldFrame {
        ScalarFieldFrame {
            quantity: ScalarQuantity::TemperatureKelvin,
            samples: vec![
                ScalarFieldSample {
                    position_world_m: [0.0, 0.0, 0.0],
                    value: 280.0,
                    standard_uncertainty: Some(0.4),
                },
                ScalarFieldSample {
                    position_world_m: [1.0, 0.0, 0.0],
                    value: 320.0,
                    standard_uncertainty: Some(0.8),
                },
            ],
            display_range: ScalarRange {
                minimum: 250.0,
                maximum: 350.0,
            },
            palette: ScalarPalette::Thermal,
            spectral_band: None,
            provenance: simulation_provenance(),
        }
    }

    #[test]
    fn provenance_distinguishes_uncalibrated_sensor_data_from_simulations() {
        let mut provenance = simulation_provenance();
        assert!(!provenance.is_uncalibrated_measurement());
        provenance.source_kind = FieldSourceKind::MeasuredSensor;
        provenance.calibration_state = CalibrationState::Uncalibrated;
        assert!(provenance.is_uncalibrated_measurement());
        provenance.calibration_state = CalibrationState::Calibrated;
        assert!(!provenance.is_uncalibrated_measurement());
    }

    #[test]
    fn thermal_mapping_preserves_units_range_and_uncertainty() {
        let frame = temperature_frame();
        assert!(frame.validate().is_ok());
        assert_eq!(frame.quantity.unit(), "K");
        let glyphs = extract_scalar_glyphs(&frame, 8).expect("valid frame should map");
        assert_eq!(glyphs.len(), 2);
        assert_eq!(glyphs[0].value, 280.0);
        assert_eq!(glyphs[0].standard_uncertainty, Some(0.4));
        assert!(glyphs[0].normalized_value < glyphs[1].normalized_value);
        assert_ne!(glyphs[0].color, glyphs[1].color);
    }

    #[test]
    fn measured_thermal_frame_requires_sensor_and_calibration_evidence_when_claimed() {
        let mut frame = temperature_frame();
        frame.provenance = FieldProvenance {
            source_kind: FieldSourceKind::MeasuredSensor,
            source_id: "thermal-camera-A".to_owned(),
            calibration_state: CalibrationState::Calibrated,
            calibration_id: Some("blackbody-cal-2026-01".to_owned()),
            calibration_sha256: Some("a".repeat(64)),
        };
        assert!(frame.validate().is_ok());
        frame.provenance.calibration_sha256 = Some("not-a-digest".to_owned());
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::MissingCalibrationEvidence)
        );
    }

    #[test]
    fn spectral_radiance_requires_a_declared_wavelength_band() {
        let mut frame = temperature_frame();
        frame.quantity = ScalarQuantity::SpectralRadianceWPerSrM2Um;
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::MissingSpectralBand)
        );
        frame.spectral_band = Some(SpectralBand {
            center_wavelength_nm: 10_000.0,
            bandwidth_fwhm_nm: 1_000.0,
        });
        assert!(frame.validate().is_ok());
    }

    #[test]
    fn spectral_band_rejects_nonpositive_lower_wavelength_edge() {
        assert_eq!(
            SpectralBand {
                center_wavelength_nm: 500.0,
                bandwidth_fwhm_nm: 1000.0,
            }
            .validate(),
            Err(FieldValidationError::InvalidSpectralBand)
        );
    }

    #[test]
    fn reflectance_is_bounded_between_zero_and_one() {
        let mut frame = temperature_frame();
        frame.quantity = ScalarQuantity::ReflectanceFraction;
        frame.display_range = ScalarRange {
            minimum: 0.0,
            maximum: 1.0,
        };
        frame.spectral_band = Some(SpectralBand {
            center_wavelength_nm: 550.0,
            bandwidth_fwhm_nm: 30.0,
        });
        frame.samples[0].value = 1.01;
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::InvalidPhysicalValue)
        );
    }

    #[test]
    fn invalid_ranges_nan_and_negative_uncertainty_fail_closed() {
        assert!(
            scalar_color(
                f64::NAN,
                ScalarRange {
                    minimum: 0.0,
                    maximum: 1.0
                },
                ScalarPalette::Viridis
            )
            .is_none()
        );
        let mut frame = temperature_frame();
        frame.display_range.maximum = frame.display_range.minimum;
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::InvalidDisplayRange)
        );
        frame = temperature_frame();
        frame.samples[0].standard_uncertainty = Some(-0.1);
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::InvalidUncertainty)
        );
    }

    fn spectral_composite_frame() -> SpectralCompositeFrame {
        SpectralCompositeFrame {
            quantity: SpectralScalarQuantity::ReflectanceFraction,
            channels: vec![
                SpectralChannelSpec {
                    band: SpectralBand {
                        center_wavelength_nm: 450.0,
                        bandwidth_fwhm_nm: 20.0,
                    },
                    display_range: ScalarRange {
                        minimum: 0.0,
                        maximum: 1.0,
                    },
                },
                SpectralChannelSpec {
                    band: SpectralBand {
                        center_wavelength_nm: 550.0,
                        bandwidth_fwhm_nm: 20.0,
                    },
                    display_range: ScalarRange {
                        minimum: 0.0,
                        maximum: 1.0,
                    },
                },
                SpectralChannelSpec {
                    band: SpectralBand {
                        center_wavelength_nm: 850.0,
                        bandwidth_fwhm_nm: 30.0,
                    },
                    display_range: ScalarRange {
                        minimum: 0.0,
                        maximum: 1.0,
                    },
                },
            ],
            samples: vec![SpectralSample {
                position_world_m: [2.0, 0.0, 1.0],
                values: vec![0.2, 0.5, 0.9],
                standard_uncertainties: vec![Some(0.01), Some(0.02), None],
            }],
            rgb_channel_indices: [2, 1, 0],
            provenance: simulation_provenance(),
        }
    }

    #[test]
    fn spectral_composites_bind_band_units_and_explicit_rgb_channel_selection() {
        let frame = spectral_composite_frame();
        assert!(frame.validate().is_ok());
        let glyphs = extract_spectral_glyphs(&frame, 8)
            .expect("well-formed bands should produce a display composite");
        assert_eq!(glyphs.len(), 1);
        assert_eq!(glyphs[0].selected_values, [0.9, 0.5, 0.2]);
        assert_eq!(glyphs[0].rgb, [0.9, 0.5, 0.2]);
    }

    #[test]
    fn spectral_composite_rejects_invalid_indices_and_misaligned_channels() {
        let mut frame = spectral_composite_frame();
        frame.rgb_channel_indices = [0, 0, 2];
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::InvalidSpectralMapping)
        );
        frame = spectral_composite_frame();
        frame.samples[0].values.pop();
        assert_eq!(
            frame.validate(),
            Err(FieldValidationError::ChannelSampleCountMismatch)
        );
    }

    #[test]
    fn electromagnetic_vector_glyphs_keep_physical_units_and_cap_display_length() {
        let frame = VectorFieldFrame {
            quantity: VectorQuantity::ElectricFieldVoltsPerMeter,
            samples: vec![VectorFieldSample {
                position_world_m: [1.0, 2.0, 3.0],
                vector: [3.0, 4.0, 0.0],
            }],
            provenance: simulation_provenance(),
        };
        assert!(frame.validate().is_ok());
        assert_eq!(frame.quantity.unit(), "V/m");
        let glyphs = extract_vector_glyphs(&frame, 8, 0.1, 0.25).expect("valid vectors should map");
        assert_eq!(glyphs.len(), 1);
        assert_eq!(glyphs[0].direction, [0.6, 0.8, 0.0]);
        assert_eq!(glyphs[0].display_length, 0.25);
        assert_eq!(glyphs[0].magnitude, 5.0);
    }

    #[test]
    fn field_extractors_are_bounded_deterministic_and_non_mutating() {
        let frame = temperature_frame();
        let before = frame.clone();
        let a = extract_scalar_glyphs(&frame, 1).expect("valid frame should map");
        let b = extract_scalar_glyphs(&frame, 1).expect("valid frame should map");
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
        assert_eq!(frame, before);

        let vectors = VectorFieldFrame {
            quantity: VectorQuantity::MagneticFluxDensityTesla,
            samples: vec![
                VectorFieldSample {
                    position_world_m: [0.0, 0.0, 0.0],
                    vector: [0.0, 0.0, 2.0],
                },
                VectorFieldSample {
                    position_world_m: [1.0, 0.0, 0.0],
                    vector: [0.0, 0.0, 0.0],
                },
            ],
            provenance: simulation_provenance(),
        };
        let first = extract_vector_glyphs(&vectors, 1, 0.2, 1.0).expect("valid vectors should map");
        let second =
            extract_vector_glyphs(&vectors, 1, 0.2, 1.0).expect("valid vectors should map");
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
    }
}
