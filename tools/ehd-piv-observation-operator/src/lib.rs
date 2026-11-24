// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic planar 2D2C particle-image forward operator.
//!
//! This is a small manufactured observation model, not a complete optical or
//! particle-laden-flow simulator. It maps a prescribed 2D velocity field and
//! particle seeds into two quantized synthetic images while exposing finite
//! exposure, out-of-plane illumination attenuation, particle blur, optical
//! PSF, gain, offset, explicit additive noise, masking and sensor saturation.
//!
//! The velocity field is expressed in pixel coordinates and pixel/time units.
//! No extrapolation is permitted outside its rectangular support.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct VelocityField2D {
    pub width: usize,
    pub height: usize,
    pub origin_x_px: f64,
    pub origin_y_px: f64,
    pub spacing_x_px: f64,
    pub spacing_y_px: f64,
    pub u_px_per_time: Vec<f64>,
    pub v_px_per_time: Vec<f64>,
}

impl VelocityField2D {
    fn sample(&self, x_px: f64, y_px: f64) -> Result<(f64, f64), ObservationError> {
        let grid_x = (x_px - self.origin_x_px) / self.spacing_x_px;
        let grid_y = (y_px - self.origin_y_px) / self.spacing_y_px;
        if !grid_x.is_finite()
            || !grid_y.is_finite()
            || grid_x < 0.0
            || grid_y < 0.0
            || grid_x > (self.width - 1) as f64
            || grid_y > (self.height - 1) as f64
        {
            return Err(ObservationError::VelocitySupportExceeded);
        }

        let x0 = grid_x.floor() as usize;
        let y0 = grid_y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = grid_x - x0 as f64;
        let ty = grid_y - y0 as f64;

        let interpolate = |values: &[f64]| {
            let q00 = values[y0 * self.width + x0];
            let q10 = values[y0 * self.width + x1];
            let q01 = values[y1 * self.width + x0];
            let q11 = values[y1 * self.width + x1];
            let lower = q00 * (1.0 - tx) + q10 * tx;
            let upper = q01 * (1.0 - tx) + q11 * tx;
            lower * (1.0 - ty) + upper * ty
        };
        Ok((
            interpolate(&self.u_px_per_time),
            interpolate(&self.v_px_per_time),
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSeed {
    pub x_px: f64,
    pub y_px: f64,
    /// Distance from the illumination-sheet centre in declared sheet units.
    pub z_sheet_units: f64,
    /// Constant prescribed out-of-plane drift in sheet units per time.
    /// This is an input proxy, not a solved third velocity component.
    pub out_of_plane_velocity_sheet_units_per_time: f64,
    pub brightness: f64,
    /// Intrinsic particle-image Gaussian sigma, in pixels.
    pub particle_sigma_px: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IlluminationSheet {
    /// Gaussian sheet standard deviation in sheet units.
    pub sigma_sheet_units: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionConfig {
    /// Time between the nominal start of the two exposures.
    pub frame_separation: f64,
    /// Duration of each exposure. Zero means instantaneous exposure.
    pub exposure_duration: f64,
    /// Midpoint quadrature samples within each exposure.
    pub exposure_samples: usize,
    /// Maximum RK2 integration step in the same time units.
    pub maximum_integration_step: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SensorConfig {
    pub width: usize,
    pub height: usize,
    pub bit_depth: u8,
    pub gain: f64,
    pub offset: f64,
    /// Gaussian optical PSF sigma in pixels.
    pub optical_psf_sigma_px: f64,
    /// True means active pixel; false means masked and forced to zero.
    pub active_pixel_mask: Vec<bool>,
}

impl SensorConfig {
    pub fn maximum_digital_value(&self) -> u16 {
        ((1_u32 << self.bit_depth) - 1) as u16
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameNoise {
    /// Explicit additive sensor noise in digital units, one value per pixel.
    /// V0 deliberately accepts a frozen noise realization instead of hiding
    /// an implicit RNG or non-reproducible stochastic source.
    pub first_frame: Vec<f64>,
    pub second_frame: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageFrame {
    pub width: usize,
    pub height: usize,
    pub bit_depth: u8,
    pub pixels: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationReceipt {
    pub particle_count: usize,
    pub frame_separation: f64,
    pub exposure_duration: f64,
    pub exposure_samples: usize,
    pub maximum_integration_step: f64,
    pub effective_psf_sigma_range_px: Option<(f64, f64)>,
    pub masked_pixel_count: usize,
    pub saturated_pixel_count_first: usize,
    pub saturated_pixel_count_second: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParticleImagePair {
    pub first: ImageFrame,
    pub second: ImageFrame,
    /// Detector-domain floating-point values after gain/offset/noise and
    /// active-pixel masking, but before clipping and ADC quantization.
    /// These are useful for finite-difference sensitivity analysis; they are
    /// not photon-electron or radiometric truth.
    pub analog_first_frame: Vec<f64>,
    pub analog_second_frame: Vec<f64>,
    pub receipt: SimulationReceipt,
}

/// Central finite-difference sensitivity of the concatenated two-frame
/// analog detector observation. Pixel order is first frame row-major,
/// followed by second frame row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePairSensitivity {
    pub parameter_step: f64,
    pub first_frame_pixel_count: usize,
    pub observation_sensitivity: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationError {
    InvalidFieldShape {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    InvalidFieldDimensions,
    FieldSupportOutsideSensor,
    PixelCountExceeded,
    InvalidFieldMetadata {
        field: &'static str,
    },
    NonFiniteFieldValue {
        index: usize,
    },
    InvalidSensorDimensions,
    InvalidSensorMetadata {
        field: &'static str,
    },
    InvalidMaskShape {
        expected: usize,
        actual: usize,
    },
    InvalidAcquisitionMetadata {
        field: &'static str,
    },
    InvalidIlluminationMetadata,
    EmptyParticleSet,
    InvalidParticle {
        index: usize,
        field: &'static str,
    },
    InvalidNoiseShape {
        frame: &'static str,
        expected: usize,
        actual: usize,
    },
    NonFiniteNoise {
        frame: &'static str,
        index: usize,
    },
    NonFiniteSensorSignal {
        index: usize,
    },
    InvalidSensitivityStep,
    ImagePairShapeMismatch,
    InvalidAnalogFrameShape {
        frame: &'static str,
        expected: usize,
        actual: usize,
    },
    NonFiniteAnalogFrameValue {
        frame: &'static str,
        index: usize,
    },
    VelocitySupportExceeded,
    ExcessiveIntegrationSteps,
}

impl fmt::Display for ObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid PIV observation-operator input: {self:?}")
    }
}

impl Error for ObservationError {}

pub fn render_particle_image_pair(
    field: &VelocityField2D,
    particles: &[ParticleSeed],
    illumination: &IlluminationSheet,
    acquisition: &AcquisitionConfig,
    sensor: &SensorConfig,
    noise: &FrameNoise,
) -> Result<ParticleImagePair, ObservationError> {
    validate_inputs(field, particles, illumination, acquisition, sensor, noise)?;

    let pixel_count = sensor.width * sensor.height;
    let mut first_intensity = vec![0.0; pixel_count];
    let mut second_intensity = vec![0.0; pixel_count];
    let mut sigma_min = f64::INFINITY;
    let mut sigma_max = 0.0_f64;
    let render_context = RenderContext {
        field,
        illumination,
        acquisition,
        sensor,
    };

    for particle in particles {
        let sigma = particle
            .particle_sigma_px
            .hypot(sensor.optical_psf_sigma_px);
        sigma_min = sigma_min.min(sigma);
        sigma_max = sigma_max.max(sigma);

        render_exposure(&render_context, particle, 0.0, sigma, &mut first_intensity)?;
        render_exposure(
            &render_context,
            particle,
            acquisition.frame_separation,
            sigma,
            &mut second_intensity,
        )?;
    }

    let (first_pixels, saturated_pixel_count_first, analog_first_frame) =
        digitize(&first_intensity, &noise.first_frame, sensor)?;
    let (second_pixels, saturated_pixel_count_second, analog_second_frame) =
        digitize(&second_intensity, &noise.second_frame, sensor)?;

    let masked_pixel_count = sensor
        .active_pixel_mask
        .iter()
        .filter(|active| !**active)
        .count();

    Ok(ParticleImagePair {
        first: ImageFrame {
            width: sensor.width,
            height: sensor.height,
            bit_depth: sensor.bit_depth,
            pixels: first_pixels,
        },
        second: ImageFrame {
            width: sensor.width,
            height: sensor.height,
            bit_depth: sensor.bit_depth,
            pixels: second_pixels,
        },
        analog_first_frame,
        analog_second_frame,
        receipt: SimulationReceipt {
            particle_count: particles.len(),
            frame_separation: acquisition.frame_separation,
            exposure_duration: acquisition.exposure_duration,
            exposure_samples: acquisition.exposure_samples,
            maximum_integration_step: acquisition.maximum_integration_step,
            effective_psf_sigma_range_px: Some((sigma_min, sigma_max)),
            masked_pixel_count,
            saturated_pixel_count_first,
            saturated_pixel_count_second,
        },
    })
}

/// Estimate the image-domain derivative using two forward predictions
/// evaluated at parameter values theta +/- parameter_step.
///
/// The caller MUST freeze all unrelated nuisance/random inputs, including the
/// particle seed set, per-pixel noise arrays, masks and acquisition state.
/// The returned derivative uses analog detector values before ADC clipping
/// and quantization so sub-LSB changes remain visible. Parameter steps must be
/// large enough to dominate numerical/model noise while remaining local.
pub fn central_difference_image_sensitivity(
    positive_perturbation: &ParticleImagePair,
    negative_perturbation: &ParticleImagePair,
    parameter_step: f64,
) -> Result<ImagePairSensitivity, ObservationError> {
    if !parameter_step.is_finite() || parameter_step <= 0.0 {
        return Err(ObservationError::InvalidSensitivityStep);
    }
    let positive_first = &positive_perturbation.first;
    let positive_second = &positive_perturbation.second;
    let negative_first = &negative_perturbation.first;
    let negative_second = &negative_perturbation.second;

    if positive_first.width != negative_first.width
        || positive_first.height != negative_first.height
        || positive_first.bit_depth != negative_first.bit_depth
        || positive_second.width != negative_second.width
        || positive_second.height != negative_second.height
        || positive_second.bit_depth != negative_second.bit_depth
        || positive_first.width != positive_second.width
        || positive_first.height != positive_second.height
    {
        return Err(ObservationError::ImagePairShapeMismatch);
    }
    let first_count = positive_first
        .width
        .checked_mul(positive_first.height)
        .filter(|count| *count <= 16_777_216)
        .ok_or(ObservationError::PixelCountExceeded)?;
    let second_count = positive_second
        .width
        .checked_mul(positive_second.height)
        .filter(|count| *count <= 16_777_216)
        .ok_or(ObservationError::PixelCountExceeded)?;

    for (frame, values, expected) in [
        (
            "positive_first",
            positive_perturbation.analog_first_frame.as_slice(),
            first_count,
        ),
        (
            "negative_first",
            negative_perturbation.analog_first_frame.as_slice(),
            first_count,
        ),
        (
            "positive_second",
            positive_perturbation.analog_second_frame.as_slice(),
            second_count,
        ),
        (
            "negative_second",
            negative_perturbation.analog_second_frame.as_slice(),
            second_count,
        ),
    ] {
        if values.len() != expected {
            return Err(ObservationError::InvalidAnalogFrameShape {
                frame,
                expected,
                actual: values.len(),
            });
        }
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(ObservationError::NonFiniteAnalogFrameValue { frame, index });
            }
        }
    }

    let denominator = 2.0 * parameter_step;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(ObservationError::InvalidSensitivityStep);
    }
    let mut observation_sensitivity = Vec::with_capacity(first_count + second_count);
    for (positive, negative) in positive_perturbation
        .analog_first_frame
        .iter()
        .zip(negative_perturbation.analog_first_frame.iter())
        .chain(
            positive_perturbation
                .analog_second_frame
                .iter()
                .zip(negative_perturbation.analog_second_frame.iter()),
        )
    {
        let derivative = (positive - negative) / denominator;
        if !derivative.is_finite() {
            return Err(ObservationError::NonFiniteSensorSignal {
                index: observation_sensitivity.len(),
            });
        }
        observation_sensitivity.push(derivative);
    }

    Ok(ImagePairSensitivity {
        parameter_step,
        first_frame_pixel_count: first_count,
        observation_sensitivity,
    })
}

fn validate_inputs(
    field: &VelocityField2D,
    particles: &[ParticleSeed],
    illumination: &IlluminationSheet,
    acquisition: &AcquisitionConfig,
    sensor: &SensorConfig,
    noise: &FrameNoise,
) -> Result<(), ObservationError> {
    if field.width < 2 || field.height < 2 {
        return Err(ObservationError::InvalidFieldDimensions);
    }
    let field_len = field
        .width
        .checked_mul(field.height)
        .filter(|count| *count <= 16_777_216)
        .ok_or(ObservationError::PixelCountExceeded)?;
    for (name, actual) in [
        ("u_px_per_time", field.u_px_per_time.len()),
        ("v_px_per_time", field.v_px_per_time.len()),
    ] {
        if actual != field_len {
            return Err(ObservationError::InvalidFieldShape {
                field: name,
                expected: field_len,
                actual,
            });
        }
    }
    for (name, value) in [
        ("origin_x_px", field.origin_x_px),
        ("origin_y_px", field.origin_y_px),
        ("spacing_x_px", field.spacing_x_px),
        ("spacing_y_px", field.spacing_y_px),
    ] {
        if !value.is_finite() || (name.starts_with("spacing") && value <= 0.0) {
            return Err(ObservationError::InvalidFieldMetadata { field: name });
        }
    }
    for (index, (u, v)) in field
        .u_px_per_time
        .iter()
        .zip(field.v_px_per_time.iter())
        .enumerate()
    {
        if !u.is_finite() || !v.is_finite() {
            return Err(ObservationError::NonFiniteFieldValue { index });
        }
    }

    if sensor.width < 2 || sensor.height < 2 {
        return Err(ObservationError::InvalidSensorDimensions);
    }
    if !(1..=16).contains(&sensor.bit_depth) {
        return Err(ObservationError::InvalidSensorMetadata { field: "bit_depth" });
    }
    for (name, value) in [
        ("gain", sensor.gain),
        ("optical_psf_sigma_px", sensor.optical_psf_sigma_px),
    ] {
        if !value.is_finite()
            || (name == "gain" && value <= 0.0)
            || (name == "optical_psf_sigma_px" && value < 0.0)
        {
            return Err(ObservationError::InvalidSensorMetadata { field: name });
        }
    }
    if !sensor.offset.is_finite() {
        return Err(ObservationError::InvalidSensorMetadata { field: "offset" });
    }
    let pixel_count = sensor
        .width
        .checked_mul(sensor.height)
        .filter(|count| *count <= 16_777_216)
        .ok_or(ObservationError::PixelCountExceeded)?;
    if sensor.active_pixel_mask.len() != pixel_count {
        return Err(ObservationError::InvalidMaskShape {
            expected: pixel_count,
            actual: sensor.active_pixel_mask.len(),
        });
    }
    let field_max_x = field.origin_x_px + (field.width - 1) as f64 * field.spacing_x_px;
    let field_max_y = field.origin_y_px + (field.height - 1) as f64 * field.spacing_y_px;
    if field.origin_x_px < 0.0
        || field.origin_y_px < 0.0
        || !field_max_x.is_finite()
        || !field_max_y.is_finite()
        || field_max_x > (sensor.width - 1) as f64
        || field_max_y > (sensor.height - 1) as f64
    {
        return Err(ObservationError::FieldSupportOutsideSensor);
    }

    if !illumination.sigma_sheet_units.is_finite() || illumination.sigma_sheet_units <= 0.0 {
        return Err(ObservationError::InvalidIlluminationMetadata);
    }
    if !acquisition.frame_separation.is_finite() || acquisition.frame_separation < 0.0 {
        return Err(ObservationError::InvalidAcquisitionMetadata {
            field: "frame_separation",
        });
    }
    if !acquisition.exposure_duration.is_finite() || acquisition.exposure_duration < 0.0 {
        return Err(ObservationError::InvalidAcquisitionMetadata {
            field: "exposure_duration",
        });
    }
    if acquisition.exposure_samples == 0 || acquisition.exposure_samples > 4096 {
        return Err(ObservationError::InvalidAcquisitionMetadata {
            field: "exposure_samples",
        });
    }
    if !acquisition.maximum_integration_step.is_finite()
        || acquisition.maximum_integration_step <= 0.0
    {
        return Err(ObservationError::InvalidAcquisitionMetadata {
            field: "maximum_integration_step",
        });
    }

    if particles.is_empty() {
        return Err(ObservationError::EmptyParticleSet);
    }
    for (index, particle) in particles.iter().enumerate() {
        if particle.x_px < 0.0
            || particle.x_px > (sensor.width - 1) as f64
            || particle.y_px < 0.0
            || particle.y_px > (sensor.height - 1) as f64
        {
            return Err(ObservationError::InvalidParticle {
                index,
                field: "image_support",
            });
        }
        for (name, value, strictly_positive) in [
            ("x_px", particle.x_px, false),
            ("y_px", particle.y_px, false),
            ("z_sheet_units", particle.z_sheet_units, false),
            (
                "out_of_plane_velocity_sheet_units_per_time",
                particle.out_of_plane_velocity_sheet_units_per_time,
                false,
            ),
            ("brightness", particle.brightness, false),
            ("particle_sigma_px", particle.particle_sigma_px, true),
        ] {
            if !value.is_finite() || (strictly_positive && value <= 0.0) {
                return Err(ObservationError::InvalidParticle { index, field: name });
            }
            if name == "brightness" && value < 0.0 {
                return Err(ObservationError::InvalidParticle { index, field: name });
            }
        }
        if particle.particle_sigma_px < 1.0e-6
            || particle.particle_sigma_px > sensor.width.max(sensor.height) as f64
        {
            return Err(ObservationError::InvalidParticle {
                index,
                field: "particle_sigma_px",
            });
        }
    }

    if sensor.optical_psf_sigma_px > sensor.width.max(sensor.height) as f64 {
        return Err(ObservationError::InvalidSensorMetadata {
            field: "optical_psf_sigma_px",
        });
    }

    for (frame, values) in [
        ("first_frame", noise.first_frame.as_slice()),
        ("second_frame", noise.second_frame.as_slice()),
    ] {
        if values.len() != pixel_count {
            return Err(ObservationError::InvalidNoiseShape {
                frame,
                expected: pixel_count,
                actual: values.len(),
            });
        }
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(ObservationError::NonFiniteNoise { frame, index });
            }
        }
    }
    Ok(())
}

struct RenderContext<'a> {
    field: &'a VelocityField2D,
    illumination: &'a IlluminationSheet,
    acquisition: &'a AcquisitionConfig,
    sensor: &'a SensorConfig,
}

fn render_exposure(
    context: &RenderContext<'_>,
    particle: &ParticleSeed,
    exposure_start: f64,
    effective_sigma_px: f64,
    image: &mut [f64],
) -> Result<(), ObservationError> {
    let count = context.acquisition.exposure_samples;
    for sample_index in 0..count {
        let sample_offset =
            (sample_index as f64 + 0.5) * context.acquisition.exposure_duration / count as f64;
        let sample_time = exposure_start + sample_offset;
        let (x_px, y_px) = advect_particle(
            context.field,
            particle.x_px,
            particle.y_px,
            sample_time,
            context.acquisition.maximum_integration_step,
        )?;
        let z = particle.z_sheet_units
            + particle.out_of_plane_velocity_sheet_units_per_time * sample_time;
        let normalized_z = z / context.illumination.sigma_sheet_units;
        let illumination_weight = (-0.5 * normalized_z * normalized_z).exp();
        let brightness = particle.brightness * illumination_weight / count as f64;

        add_gaussian_particle(
            context.sensor,
            image,
            x_px,
            y_px,
            brightness,
            effective_sigma_px,
        );
    }
    Ok(())
}

fn advect_particle(
    field: &VelocityField2D,
    initial_x: f64,
    initial_y: f64,
    total_time: f64,
    maximum_step: f64,
) -> Result<(f64, f64), ObservationError> {
    let mut x = initial_x;
    let mut y = initial_y;
    if total_time == 0.0 {
        field.sample(x, y)?;
        return Ok((x, y));
    }
    let step_count_f = (total_time / maximum_step).ceil();
    if !step_count_f.is_finite() || step_count_f > 1_000_000.0 {
        return Err(ObservationError::ExcessiveIntegrationSteps);
    }
    let step_count = step_count_f as usize;
    let dt = total_time / step_count as f64;

    for _ in 0..step_count {
        let (u1, v1) = field.sample(x, y)?;
        let midpoint_x = x + 0.5 * dt * u1;
        let midpoint_y = y + 0.5 * dt * v1;
        let (u_mid, v_mid) = field.sample(midpoint_x, midpoint_y)?;
        x += dt * u_mid;
        y += dt * v_mid;
        field.sample(x, y)?;
    }
    Ok((x, y))
}

fn add_gaussian_particle(
    sensor: &SensorConfig,
    image: &mut [f64],
    x_px: f64,
    y_px: f64,
    brightness: f64,
    sigma_px: f64,
) {
    if brightness == 0.0 {
        return;
    }
    let radius = (4.0 * sigma_px).ceil() as isize;
    let center_x = x_px.floor() as isize;
    let center_y = y_px.floor() as isize;
    let inverse_variance = 0.5 / (sigma_px * sigma_px);

    for y in (center_y - radius)..=(center_y + radius + 1) {
        if y < 0 || y >= sensor.height as isize {
            continue;
        }
        for x in (center_x - radius)..=(center_x + radius + 1) {
            if x < 0 || x >= sensor.width as isize {
                continue;
            }
            let index = y as usize * sensor.width + x as usize;
            if !sensor.active_pixel_mask[index] {
                continue;
            }
            let dx = x as f64 - x_px;
            let dy = y as f64 - y_px;
            let squared_distance = dx * dx + dy * dy;
            image[index] += brightness * (-squared_distance * inverse_variance).exp();
        }
    }
}

fn digitize(
    intensity: &[f64],
    noise: &[f64],
    sensor: &SensorConfig,
) -> Result<(Vec<u16>, usize, Vec<f64>), ObservationError> {
    let maximum = sensor.maximum_digital_value() as f64;
    let mut saturated = 0usize;
    let mut analog_values = Vec::with_capacity(intensity.len());
    let pixels = intensity
        .iter()
        .zip(noise.iter())
        .zip(sensor.active_pixel_mask.iter())
        .enumerate()
        .map(|(index, ((signal, noise), active))| {
            if !*active {
                analog_values.push(0.0);
                return Ok(0);
            }
            let analog = sensor.gain * signal + sensor.offset + noise;
            if !analog.is_finite() {
                return Err(ObservationError::NonFiniteSensorSignal { index });
            }
            analog_values.push(analog);
            if analog >= maximum {
                saturated += 1;
            }
            Ok(analog.round().clamp(0.0, maximum) as u16)
        })
        .collect::<Result<Vec<u16>, ObservationError>>()?;
    Ok((pixels, saturated, analog_values))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(width: usize, height: usize, u: f64, v: f64) -> VelocityField2D {
        VelocityField2D {
            width,
            height,
            origin_x_px: 0.0,
            origin_y_px: 0.0,
            spacing_x_px: 1.0,
            spacing_y_px: 1.0,
            u_px_per_time: vec![u; width * height],
            v_px_per_time: vec![v; width * height],
        }
    }

    fn sensor(width: usize, height: usize) -> SensorConfig {
        SensorConfig {
            width,
            height,
            bit_depth: 12,
            gain: 1.0,
            offset: 0.0,
            optical_psf_sigma_px: 0.0,
            active_pixel_mask: vec![true; width * height],
        }
    }

    fn acquisition(exposure: f64, samples: usize) -> AcquisitionConfig {
        AcquisitionConfig {
            frame_separation: 1.0,
            exposure_duration: exposure,
            exposure_samples: samples,
            maximum_integration_step: 0.05,
        }
    }

    fn particle(x: f64, y: f64) -> ParticleSeed {
        ParticleSeed {
            x_px: x,
            y_px: y,
            z_sheet_units: 0.0,
            out_of_plane_velocity_sheet_units_per_time: 0.0,
            brightness: 1000.0,
            particle_sigma_px: 0.8,
        }
    }

    fn illumination() -> IlluminationSheet {
        IlluminationSheet {
            sigma_sheet_units: 1.0,
        }
    }

    fn noise(width: usize, height: usize) -> FrameNoise {
        FrameNoise {
            first_frame: vec![0.0; width * height],
            second_frame: vec![0.0; width * height],
        }
    }

    fn centroid_x(image: &ImageFrame) -> f64 {
        let mut weighted = 0.0;
        let mut total = 0.0;
        for y in 0..image.height {
            for x in 0..image.width {
                let value = image.pixels[y * image.width + x] as f64;
                weighted += x as f64 * value;
                total += value;
            }
        }
        weighted / total
    }

    fn variance_x(image: &ImageFrame) -> f64 {
        let mean = centroid_x(image);
        let mut weighted = 0.0;
        let mut total = 0.0;
        for y in 0..image.height {
            for x in 0..image.width {
                let value = image.pixels[y * image.width + x] as f64;
                weighted += (x as f64 - mean).powi(2) * value;
                total += value;
            }
        }
        weighted / total
    }

    fn total_intensity(image: &ImageFrame) -> u64 {
        image.pixels.iter().map(|value| *value as u64).sum()
    }

    fn vector_correlation(left: &[f64], right: &[f64]) -> f64 {
        let dot = left
            .iter()
            .zip(right.iter())
            .map(|(left_value, right_value)| left_value * right_value)
            .sum::<f64>();
        let left_norm = left.iter().map(|value| value * value).sum::<f64>().sqrt();
        let right_norm = right.iter().map(|value| value * value).sum::<f64>().sqrt();
        dot / (left_norm * right_norm)
    }

    #[test]
    fn stationary_particle_pair_is_identical_with_identical_noise() {
        let width = 32;
        let height = 32;
        let image_pair = render_particle_image_pair(
            &field(width, height, 0.0, 0.0),
            &[particle(16.0, 16.0)],
            &illumination(),
            &acquisition(0.2, 5),
            &sensor(width, height),
            &noise(width, height),
        )
        .unwrap();
        assert_eq!(image_pair.first.pixels, image_pair.second.pixels);
    }

    #[test]
    fn uniform_translation_moves_particle_centroid_by_analytic_displacement() {
        let width = 32;
        let height = 32;
        let image_pair = render_particle_image_pair(
            &field(width, height, 2.0, 0.0),
            &[particle(12.0, 16.0)],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor(width, height),
            &noise(width, height),
        )
        .unwrap();
        assert!(
            (centroid_x(&image_pair.second) - centroid_x(&image_pair.first) - 2.0).abs() < 0.02
        );
    }

    #[test]
    fn longer_exposure_increases_motion_smear() {
        let width = 40;
        let height = 32;
        let field = field(width, height, 4.0, 0.0);
        let sensor = sensor(width, height);
        let seeds = [particle(12.0, 16.0)];
        let short = render_particle_image_pair(
            &field,
            &seeds,
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        let long = render_particle_image_pair(
            &field,
            &seeds,
            &illumination(),
            &acquisition(0.8, 9),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        assert!(variance_x(&long.first) > variance_x(&short.first) + 0.2);
    }

    #[test]
    fn out_of_plane_drift_attenuates_particle_through_gaussian_sheet() {
        let width = 32;
        let height = 32;
        let field = field(width, height, 0.0, 0.0);
        let mut central = particle(16.0, 16.0);
        let mut outside = central.clone();
        outside.z_sheet_units = 2.0;
        let sensor = sensor(width, height);
        let central_image = render_particle_image_pair(
            &field,
            &[central.clone()],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        let outside_image = render_particle_image_pair(
            &field,
            &[outside],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        assert!(total_intensity(&outside_image.first) < total_intensity(&central_image.first));
        central.out_of_plane_velocity_sheet_units_per_time = 2.0;
        let drifting = render_particle_image_pair(
            &field,
            &[central],
            &illumination(),
            &acquisition(0.5, 3),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        assert!(total_intensity(&drifting.first) < total_intensity(&central_image.first));
    }

    #[test]
    fn optical_psf_increases_particle_image_width() {
        let width = 32;
        let height = 32;
        let field = field(width, height, 0.0, 0.0);
        let seed = [particle(16.0, 16.0)];
        let narrow = render_particle_image_pair(
            &field,
            &seed,
            &illumination(),
            &acquisition(0.0, 1),
            &sensor(width, height),
            &noise(width, height),
        )
        .unwrap();
        let mut blurred_sensor = sensor(width, height);
        blurred_sensor.optical_psf_sigma_px = 1.2;
        let broad = render_particle_image_pair(
            &field,
            &seed,
            &illumination(),
            &acquisition(0.0, 1),
            &blurred_sensor,
            &noise(width, height),
        )
        .unwrap();
        assert!(variance_x(&broad.first) > variance_x(&narrow.first) + 0.4);
    }

    #[test]
    fn gain_offset_noise_quantization_and_saturation_are_deterministic() {
        let width = 8;
        let height = 8;
        let mut sensor = sensor(width, height);
        sensor.bit_depth = 8;
        sensor.gain = 4.0;
        sensor.offset = 20.0;
        let mut noise = noise(width, height);
        noise.first_frame.fill(7.0);
        noise.second_frame.fill(-3.0);
        let image = render_particle_image_pair(
            &field(width, height, 0.0, 0.0),
            &[particle(4.0, 4.0)],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise,
        )
        .unwrap();
        assert!(image.receipt.saturated_pixel_count_first > 0);
        assert!(image.first.pixels.iter().all(|value| *value <= 255));
        let replay = render_particle_image_pair(
            &field(width, height, 0.0, 0.0),
            &[particle(4.0, 4.0)],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise,
        )
        .unwrap();
        assert_eq!(image, replay);
    }

    #[test]
    fn masked_detector_pixels_are_forced_to_zero() {
        let width = 12;
        let height = 12;
        let mut sensor = sensor(width, height);
        let masked_index = 6 * width + 6;
        sensor.active_pixel_mask[masked_index] = false;
        let image = render_particle_image_pair(
            &field(width, height, 0.0, 0.0),
            &[particle(6.0, 6.0)],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        assert_eq!(image.first.pixels[masked_index], 0);
        assert_eq!(image.second.pixels[masked_index], 0);
        assert_eq!(image.receipt.masked_pixel_count, 1);
    }

    #[test]
    fn out_of_support_advection_fails_instead_of_extrapolating() {
        let width = 8;
        let height = 8;
        let result = render_particle_image_pair(
            &field(width, height, 10.0, 0.0),
            &[particle(4.0, 4.0)],
            &illumination(),
            &acquisition(1.0, 1),
            &sensor(width, height),
            &noise(width, height),
        );
        assert_eq!(result, Err(ObservationError::VelocitySupportExceeded));
    }

    #[test]
    fn unsupported_field_extent_is_rejected_before_rendering() {
        let width = 8;
        let height = 8;
        let mut outside = field(width, height, 0.0, 0.0);
        outside.origin_x_px = 2.0;
        assert_eq!(
            render_particle_image_pair(
                &outside,
                &[particle(4.0, 4.0)],
                &illumination(),
                &acquisition(0.0, 1),
                &sensor(width, height),
                &noise(width, height)
            ),
            Err(ObservationError::FieldSupportOutsideSensor)
        );
    }

    #[test]
    fn rendered_signal_overflow_fails_closed() {
        let width = 8;
        let height = 8;
        let mut first = particle(4.0, 4.0);
        first.brightness = f64::MAX;
        let mut second = first.clone();
        second.brightness = f64::MAX;
        let sensor = sensor(width, height);
        assert_eq!(
            render_particle_image_pair(
                &field(width, height, 0.0, 0.0),
                &[first, second],
                &illumination(),
                &acquisition(0.0, 1),
                &sensor,
                &noise(width, height)
            ),
            Err(ObservationError::NonFiniteSensorSignal {
                index: 4 * width + 4
            })
        );
    }

    #[test]
    fn malformed_noise_and_metadata_are_rejected() {
        let width = 8;
        let height = 8;
        let mut bad_noise = noise(width, height);
        bad_noise.first_frame.pop();
        assert!(matches!(
            render_particle_image_pair(
                &field(width, height, 0.0, 0.0),
                &[particle(4.0, 4.0)],
                &illumination(),
                &acquisition(0.0, 1),
                &sensor(width, height),
                &bad_noise
            ),
            Err(ObservationError::InvalidNoiseShape { .. })
        ));
        let mut bad_acquisition = acquisition(0.0, 1);
        bad_acquisition.maximum_integration_step = f64::NAN;
        assert_eq!(
            render_particle_image_pair(
                &field(width, height, 0.0, 0.0),
                &[particle(4.0, 4.0)],
                &illumination(),
                &bad_acquisition,
                &sensor(width, height),
                &noise(width, height)
            ),
            Err(ObservationError::InvalidAcquisitionMetadata {
                field: "maximum_integration_step"
            })
        );
    }
    #[test]
    fn analog_finite_difference_preserves_sub_lsb_sensitivity() {
        let width = 24;
        let height = 24;
        let mut base = particle(12.0, 12.0);
        base.brightness = 100.0;
        let step = 1.0e-4;
        let mut positive_seed = base.clone();
        positive_seed.brightness += step;
        let mut negative_seed = base;
        negative_seed.brightness -= step;
        let field = field(width, height, 0.0, 0.0);
        let sensor = sensor(width, height);
        let positive = render_particle_image_pair(
            &field,
            &[positive_seed],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        let negative = render_particle_image_pair(
            &field,
            &[negative_seed],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor,
            &noise(width, height),
        )
        .unwrap();
        assert_eq!(positive.first.pixels, negative.first.pixels);
        let sensitivity = central_difference_image_sensitivity(&positive, &negative, step).unwrap();
        assert!(
            sensitivity
                .observation_sensitivity
                .iter()
                .any(|value| value.abs() > 0.0)
        );
        assert!(
            sensitivity
                .observation_sensitivity
                .iter()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn brightness_and_gain_sensitivities_are_locally_collinear() {
        let width = 24;
        let height = 24;
        let mut seed = particle(12.0, 12.0);
        seed.brightness = 1000.0;
        let field = field(width, height, 0.0, 0.0);
        let acquisition = acquisition(0.0, 1);
        let illumination = illumination();
        let frozen_noise = noise(width, height);
        let brightness_step = 0.1;
        let mut bright_plus = seed.clone();
        bright_plus.brightness += brightness_step;
        let mut bright_minus = seed.clone();
        bright_minus.brightness -= brightness_step;
        let sensor = sensor(width, height);
        let brightness_plus = render_particle_image_pair(
            &field,
            &[bright_plus],
            &illumination,
            &acquisition,
            &sensor,
            &frozen_noise,
        )
        .unwrap();
        let brightness_minus = render_particle_image_pair(
            &field,
            &[bright_minus],
            &illumination,
            &acquisition,
            &sensor,
            &frozen_noise,
        )
        .unwrap();
        let brightness_sensitivity = central_difference_image_sensitivity(
            &brightness_plus,
            &brightness_minus,
            brightness_step,
        )
        .unwrap();

        let gain_step = 0.001;
        let mut gain_plus = sensor.clone();
        gain_plus.gain += gain_step;
        let mut gain_minus = sensor;
        gain_minus.gain -= gain_step;
        let gain_plus_pair = render_particle_image_pair(
            &field,
            &[seed.clone()],
            &illumination,
            &acquisition,
            &gain_plus,
            &frozen_noise,
        )
        .unwrap();
        let gain_minus_pair = render_particle_image_pair(
            &field,
            &[seed],
            &illumination,
            &acquisition,
            &gain_minus,
            &frozen_noise,
        )
        .unwrap();
        let gain_sensitivity =
            central_difference_image_sensitivity(&gain_plus_pair, &gain_minus_pair, gain_step)
                .unwrap();

        let dot: f64 = brightness_sensitivity
            .observation_sensitivity
            .iter()
            .zip(gain_sensitivity.observation_sensitivity.iter())
            .map(|(left, right)| left * right)
            .sum();
        let left_norm = brightness_sensitivity
            .observation_sensitivity
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let right_norm = gain_sensitivity
            .observation_sensitivity
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let correlation = dot / (left_norm * right_norm);
        assert!(correlation > 0.999_999);
    }

    #[test]
    fn finite_difference_rejects_invalid_steps_and_mismatched_analog_shapes() {
        let width = 8;
        let height = 8;
        let pair = render_particle_image_pair(
            &field(width, height, 0.0, 0.0),
            &[particle(4.0, 4.0)],
            &illumination(),
            &acquisition(0.0, 1),
            &sensor(width, height),
            &noise(width, height),
        )
        .unwrap();
        assert_eq!(
            central_difference_image_sensitivity(&pair, &pair, f64::NAN),
            Err(ObservationError::InvalidSensitivityStep)
        );
        let mut malformed = pair.clone();
        malformed.analog_first_frame.pop();
        assert_eq!(
            central_difference_image_sensitivity(&malformed, &pair, 0.1),
            Err(ObservationError::InvalidAnalogFrameShape {
                frame: "positive_first",
                expected: width * height,
                actual: width * height - 1
            })
        );
    }
    #[test]
    fn particle_diameter_and_optical_psf_are_confounded_in_single_particle_view() {
        let width = 24;
        let height = 24;
        let step = 1.0e-4;
        let field = field(width, height, 0.0, 0.0);
        let illumination = illumination();
        let acquisition = acquisition(0.0, 1);
        let noise = noise(width, height);
        let sensor = {
            let mut value = sensor(width, height);
            value.optical_psf_sigma_px = 0.5;
            value
        };
        let seed = particle(12.0, 12.0);

        let mut particle_plus = seed.clone();
        particle_plus.particle_sigma_px += step;
        let mut particle_minus = seed.clone();
        particle_minus.particle_sigma_px -= step;
        let particle_sensitivity = central_difference_image_sensitivity(
            &render_particle_image_pair(
                &field,
                &[particle_plus],
                &illumination,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            &render_particle_image_pair(
                &field,
                &[particle_minus],
                &illumination,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            step,
        )
        .unwrap();

        let mut psf_plus = sensor.clone();
        psf_plus.optical_psf_sigma_px += step;
        let mut psf_minus = sensor;
        psf_minus.optical_psf_sigma_px -= step;
        let psf_sensitivity = central_difference_image_sensitivity(
            &render_particle_image_pair(
                &field,
                &[seed.clone()],
                &illumination,
                &acquisition,
                &psf_plus,
                &noise,
            )
            .unwrap(),
            &render_particle_image_pair(
                &field,
                &[seed],
                &illumination,
                &acquisition,
                &psf_minus,
                &noise,
            )
            .unwrap(),
            step,
        )
        .unwrap();

        assert!(
            vector_correlation(
                &particle_sensitivity.observation_sensitivity,
                &psf_sensitivity.observation_sensitivity
            ) > 0.999_99
        );
    }

    #[test]
    fn particle_sheet_position_and_sheet_thickness_are_confounded() {
        let width = 24;
        let height = 24;
        let step = 1.0e-4;
        let field = field(width, height, 0.0, 0.0);
        let acquisition = acquisition(0.0, 1);
        let sensor = sensor(width, height);
        let noise = noise(width, height);
        let mut seed = particle(12.0, 12.0);
        seed.z_sheet_units = 0.7;
        let illumination = IlluminationSheet {
            sigma_sheet_units: 1.2,
        };

        let mut z_plus = seed.clone();
        z_plus.z_sheet_units += step;
        let mut z_minus = seed.clone();
        z_minus.z_sheet_units -= step;
        let z_sensitivity = central_difference_image_sensitivity(
            &render_particle_image_pair(
                &field,
                &[z_plus],
                &illumination,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            &render_particle_image_pair(
                &field,
                &[z_minus],
                &illumination,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            step,
        )
        .unwrap();

        let sheet_plus = IlluminationSheet {
            sigma_sheet_units: illumination.sigma_sheet_units + step,
        };
        let sheet_minus = IlluminationSheet {
            sigma_sheet_units: illumination.sigma_sheet_units - step,
        };
        let sheet_sensitivity = central_difference_image_sensitivity(
            &render_particle_image_pair(
                &field,
                &[seed.clone()],
                &sheet_plus,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            &render_particle_image_pair(
                &field,
                &[seed],
                &sheet_minus,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            step,
        )
        .unwrap();

        assert!(
            vector_correlation(
                &z_sensitivity.observation_sensitivity,
                &sheet_sensitivity.observation_sensitivity
            ) < -0.999_99
        );
    }

    #[test]
    fn uniform_flow_velocity_and_frame_separation_are_confounded() {
        let width = 32;
        let height = 32;
        let step = 1.0e-4;
        let seed = particle(12.0, 16.0);
        let illumination = illumination();
        let sensor = sensor(width, height);
        let noise = noise(width, height);
        let acquisition = acquisition(0.0, 1);
        let velocity_plus = field(width, height, 2.0 + step, 0.0);
        let velocity_minus = field(width, height, 2.0 - step, 0.0);
        let velocity_sensitivity = central_difference_image_sensitivity(
            &render_particle_image_pair(
                &velocity_plus,
                &[seed.clone()],
                &illumination,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            &render_particle_image_pair(
                &velocity_minus,
                &[seed.clone()],
                &illumination,
                &acquisition,
                &sensor,
                &noise,
            )
            .unwrap(),
            step,
        )
        .unwrap();

        let mut time_plus = acquisition.clone();
        time_plus.frame_separation += step;
        let mut time_minus = acquisition;
        time_minus.frame_separation -= step;
        let fixed_field = field(width, height, 2.0, 0.0);
        let time_sensitivity = central_difference_image_sensitivity(
            &render_particle_image_pair(
                &fixed_field,
                &[seed.clone()],
                &illumination,
                &time_plus,
                &sensor,
                &noise,
            )
            .unwrap(),
            &render_particle_image_pair(
                &fixed_field,
                &[seed],
                &illumination,
                &time_minus,
                &sensor,
                &noise,
            )
            .unwrap(),
            step,
        )
        .unwrap();

        assert!(
            vector_correlation(
                &velocity_sensitivity.observation_sensitivity,
                &time_sensitivity.observation_sensitivity
            ) > 0.999_99
        );
    }
}
