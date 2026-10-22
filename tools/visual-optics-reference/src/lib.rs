#![deny(unsafe_code)]
// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small, dependency-free analytical references for visual-technology benchmarks.
//!
//! This crate is intentionally **not** a full optical simulator. The initial
//! implementation covers only an ideal rectified pinhole projection:
//!
//! u = fx * X / Z + cx
//! v = fy * Y / Z + cy
//!
//! Inputs use one declared, consistent length unit. Focal lengths and image
//! coordinates are in pixels. Distortion, pose transforms, finite aperture,
//! focus, photometry, wave optics, sensor response, and physical calibration are
//! outside this model and must not be inferred from its output.

use core::fmt;

pub mod dmtf;

/// Intrinsic parameters for an ideal rectified pinhole camera/projector.
///
/// fx and fy are focal lengths expressed in pixels. cx and cy are the principal
/// point in pixel coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinholeIntrinsics {
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
}

/// A point expressed in the camera/projector coordinate frame.
///
/// The optical axis is +Z. The caller must ensure the point is expressed in
/// the exact frame expected by the intrinsics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPoint3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Continuous image-plane coordinates, in pixels before raster rounding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelPoint {
    pub u: f64,
    pub v: f64,
}

/// An unnormalized viewing ray in camera/projector coordinates.
///
/// The direction is represented with z = 1, so x and y are the horizontal
/// and vertical slope relative to the optical axis. The ray origin is the
/// camera/projector center; this type does not encode world-space pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraRay {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Horizontal and vertical angular extents of an ideal pinhole imager, in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldOfView {
    pub horizontal_radians: f64,
    pub vertical_radians: f64,
}

/// Errors from bounded display transfer-function and relative-luminance calculations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferError {
    /// An input channel was NaN or infinite.
    NonFiniteInput,
    /// An input channel was outside the declared normalized [0, 1] range.
    OutOfRange,
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => f.write_str("transfer-function input must be finite"),
            Self::OutOfRange => {
                f.write_str("input is outside the declared transfer-function domain")
            }
        }
    }
}

impl std::error::Error for TransferError {}

/// Converts a normalized, gamma-encoded sRGB channel into linear-light intensity.
///
/// Uses the piecewise sRGB transfer function defined in the W3C CSS Color
/// specification and accepts only in-gamut channel values in [0, 1]. This is
/// not an HDR transfer function and does not model a physical panel's response.
///
/// # Errors
///
/// Returns an error for non-finite or out-of-range input.
pub fn srgb_channel_to_linear(encoded: f64) -> Result<f64, TransferError> {
    if !encoded.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    if !(0.0..=1.0).contains(&encoded) {
        return Err(TransferError::OutOfRange);
    }
    if encoded == 1.0 {
        return Ok(1.0);
    }

    if encoded <= 0.04045 {
        Ok(encoded / 12.92)
    } else {
        Ok(((encoded + 0.055) / 1.055).powf(2.4))
    }
}

/// Converts normalized linear-light sRGB intensity into an encoded sRGB channel.
///
/// This is the inverse of the piecewise sRGB transfer function for in-gamut
/// values. It does not apply HDR tone mapping, gamut mapping, or physical panel
/// calibration.
///
/// # Errors
///
/// Returns an error for non-finite or out-of-range input.
pub fn linear_channel_to_srgb(linear: f64) -> Result<f64, TransferError> {
    if !linear.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    if !(0.0..=1.0).contains(&linear) {
        return Err(TransferError::OutOfRange);
    }
    if linear == 1.0 {
        return Ok(1.0);
    }

    if linear <= 0.003_130_8 {
        Ok(12.92 * linear)
    } else {
        Ok(1.055 * linear.powf(1.0 / 2.4) - 0.055)
    }
}

const HLG_A: f64 = 0.178_832_77;
const HLG_B: f64 = 0.284_668_92;
const HLG_C: f64 = 0.559_910_73;

/// Encodes normalized scene-linear light as a normalized BT.2100 HLG signal.
///
/// This is the scene-referred HLG OETF. It does not calculate display luminance,
/// display system gamma, black-level lift, the HLG OOTF/EOTF, or physical panel
/// response. Use an explicit display-system model for those separate steps.
///
/// The published rounded HLG constants cause tiny endpoint round-off; the
/// canonical white endpoint is returned as exactly 1.0.
///
/// # Errors
///
/// Returns an error for non-finite or out-of-range input.
pub fn hlg_scene_to_signal(scene_linear: f64) -> Result<f64, TransferError> {
    if !scene_linear.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    if !(0.0..=1.0).contains(&scene_linear) {
        return Err(TransferError::OutOfRange);
    }
    if scene_linear == 1.0 {
        return Ok(1.0);
    }

    let signal = if scene_linear <= (1.0 / 12.0) {
        (3.0 * scene_linear).sqrt()
    } else {
        HLG_A * (12.0 * scene_linear - HLG_B).ln() + HLG_C
    };

    if !signal.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    Ok(signal.clamp(0.0, 1.0))
}

/// Decodes a normalized BT.2100 HLG signal into normalized scene-linear light.
///
/// This is the inverse of the scene-referred HLG OETF only. It does not
/// implement the display-side OOTF/EOTF and does not return cd/m². The standard
/// publishes rounded constants, so this implementation clamps only the tiny
/// numerical endpoint drift back into the declared [0, 1] domain.
///
/// # Errors
///
/// Returns an error for non-finite or out-of-range input.
pub fn hlg_signal_to_scene(signal: f64) -> Result<f64, TransferError> {
    if !signal.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    if !(0.0..=1.0).contains(&signal) {
        return Err(TransferError::OutOfRange);
    }
    if signal == 1.0 {
        return Ok(1.0);
    }

    let scene_linear = if signal <= 0.5 {
        (signal * signal) / 3.0
    } else {
        (((signal - HLG_C) / HLG_A).exp() + HLG_B) / 12.0
    };

    if !scene_linear.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    Ok(scene_linear.clamp(0.0, 1.0))
}

const PQ_M1: f64 = 2610.0 / 16_384.0;
const PQ_M2: f64 = 2523.0 / 32.0;
const PQ_C1: f64 = 3424.0 / 4_096.0;
const PQ_C2: f64 = 2413.0 / 128.0;
const PQ_C3: f64 = 2392.0 / 128.0;

/// Decodes a normalized SMPTE ST 2084 / ITU-R BT.2100 PQ signal to absolute luminance.
///
/// The output is in candelas per square metre (cd/m²) and is bounded by the
/// nominal 0–10,000 cd/m² PQ range. This is a reference EOTF, not tone mapping,
/// panel calibration, or a measurement of actual physical display luminance.
///
/// # Errors
///
/// Returns an error for non-finite input or a signal outside [0, 1].
pub fn pq_signal_to_nits(signal: f64) -> Result<f64, TransferError> {
    if !signal.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    if !(0.0..=1.0).contains(&signal) {
        return Err(TransferError::OutOfRange);
    }

    let powered = signal.powf(1.0 / PQ_M2);
    let numerator = (powered - PQ_C1).max(0.0);
    let denominator = PQ_C2 - PQ_C3 * powered;
    let normalized = (numerator / denominator).powf(1.0 / PQ_M1);
    Ok(10_000.0 * normalized)
}

/// Encodes absolute luminance as a normalized SMPTE ST 2084 / ITU-R BT.2100 PQ signal.
///
/// Input luminance is in cd/m² and must lie in the nominal 0–10,000 range.
/// Zero is canonicalized to signal zero, selecting black from the interval of
/// very-low PQ codes that decode to zero due to the EOTF's clipped numerator.
///
/// This is an encoding reference, not HDR mastering, tone mapping, gamut mapping,
/// or physical display calibration.
///
/// # Errors
///
/// Returns an error for non-finite input or luminance outside [0, 10,000].
pub fn nits_to_pq_signal(nits: f64) -> Result<f64, TransferError> {
    if !nits.is_finite() {
        return Err(TransferError::NonFiniteInput);
    }
    if !(0.0..=10_000.0).contains(&nits) {
        return Err(TransferError::OutOfRange);
    }
    if nits == 0.0 {
        return Ok(0.0);
    }

    let normalized = (nits / 10_000.0).powf(PQ_M1);
    Ok(((PQ_C1 + PQ_C2 * normalized) / (1.0 + PQ_C3 * normalized)).powf(PQ_M2))
}

/// Computes normalized relative luminance from gamma-encoded sRGB values.
///
/// The inputs are converted to linear-light first, then combined using the
/// Rec. 709/sRGB primary weighting coefficients. Output is normalized relative
/// luminance Y, not absolute luminance in cd/m² and not a measurement of a
/// physical display.
pub fn relative_luminance_from_srgb(red: f64, green: f64, blue: f64) -> Result<f64, TransferError> {
    let red = srgb_channel_to_linear(red)?;
    let green = srgb_channel_to_linear(green)?;
    let blue = srgb_channel_to_linear(blue)?;
    Ok(0.2126 * red + 0.7152 * green + 0.0722 * blue)
}

/// Why an input could not be projected by the declared ideal model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    /// An intrinsic value was non-finite or a focal length was not positive.
    InvalidIntrinsics,
    /// One or more coordinates were NaN or infinite.
    NonFiniteInput,
    /// Points on or behind the optical plane are outside this model's domain.
    NonPositiveDepth,
    /// Finite inputs generated an unrepresentable projected output.
    NonFiniteOutput,
    /// Image width or height was zero.
    InvalidImageDimensions,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntrinsics => {
                f.write_str("intrinsics must be finite and focal lengths positive")
            }
            Self::NonFiniteInput => f.write_str("point coordinates must be finite"),
            Self::NonPositiveDepth => {
                f.write_str("point depth must be finite and strictly positive")
            }
            Self::NonFiniteOutput => {
                f.write_str("projection produced a non-finite image coordinate")
            }
            Self::InvalidImageDimensions => {
                f.write_str("image width and height must both be non-zero")
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

impl PinholeIntrinsics {
    /// Returns whether all intrinsic values are finite and focal lengths positive.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.fx.is_finite()
            && self.fy.is_finite()
            && self.cx.is_finite()
            && self.cy.is_finite()
            && self.fx > 0.0
            && self.fy > 0.0
    }
}

/// Back-projects a continuous image coordinate into an ideal camera-space ray.
///
/// The returned ray is unnormalized and has z = 1. The caller may scale it
/// by any positive depth to obtain a point along the ray. Pixel coordinates
/// use the same convention as the supplied principal point.
///
/// # Errors
///
/// Returns an error for invalid intrinsics, non-finite coordinates, or an
/// unrepresentable ray.
///
/// # Model limits
///
/// No camera pose, lens distortion, clipping, or physical calibration is applied.
pub fn ray_through_pixel(
    intrinsics: PinholeIntrinsics,
    pixel: PixelPoint,
) -> Result<CameraRay, ProjectionError> {
    if !intrinsics.is_valid() {
        return Err(ProjectionError::InvalidIntrinsics);
    }

    if !pixel.u.is_finite() || !pixel.v.is_finite() {
        return Err(ProjectionError::NonFiniteInput);
    }

    let ray = CameraRay {
        x: (pixel.u - intrinsics.cx) / intrinsics.fx,
        y: (pixel.v - intrinsics.cy) / intrinsics.fy,
        z: 1.0,
    };

    if !ray.x.is_finite() || !ray.y.is_finite() {
        return Err(ProjectionError::NonFiniteOutput);
    }

    Ok(ray)
}

/// Computes the horizontal and vertical field of view of an ideal pinhole imager.
///
/// Width and height are in pixels. The image boundaries are taken as u = 0
/// to u = width and v = 0 to v = height; intrinsics must use the same pixel
/// coordinate convention. The principal point need not be at the exact center.
///
/// This is a geometric model result, not a measurement of a physical device's
/// usable optical field of view. Eye position, distortion, vignetting, fit,
/// occlusion and manufacturer-specific limits are outside this function.
///
/// # Errors
///
/// Returns an error for invalid intrinsics, zero dimensions, or non-finite output.
pub fn field_of_view(
    intrinsics: PinholeIntrinsics,
    width_px: u32,
    height_px: u32,
) -> Result<FieldOfView, ProjectionError> {
    if !intrinsics.is_valid() {
        return Err(ProjectionError::InvalidIntrinsics);
    }

    if width_px == 0 || height_px == 0 {
        return Err(ProjectionError::InvalidImageDimensions);
    }

    let width = f64::from(width_px);
    let height = f64::from(height_px);
    let left = ((0.0 - intrinsics.cx) / intrinsics.fx).atan();
    let right = ((width - intrinsics.cx) / intrinsics.fx).atan();
    let top = ((0.0 - intrinsics.cy) / intrinsics.fy).atan();
    let bottom = ((height - intrinsics.cy) / intrinsics.fy).atan();

    let fov = FieldOfView {
        horizontal_radians: right - left,
        vertical_radians: bottom - top,
    };

    if !fov.horizontal_radians.is_finite()
        || !fov.vertical_radians.is_finite()
        || fov.horizontal_radians <= 0.0
        || fov.vertical_radians <= 0.0
    {
        return Err(ProjectionError::NonFiniteOutput);
    }

    Ok(fov)
}

/// Projects a point using an ideal, distortion-free pinhole model.
///
/// The function fails closed on malformed intrinsics, non-finite input,
/// non-positive depth, and non-finite output. It does not perform coordinate
/// transforms, unit conversion, distortion correction, clipping, or rounding.
///
/// # Errors
///
/// Returns a ProjectionError if an input is outside the model's domain.
pub fn project_point(
    intrinsics: PinholeIntrinsics,
    point: CameraPoint3,
) -> Result<PixelPoint, ProjectionError> {
    if !intrinsics.is_valid() {
        return Err(ProjectionError::InvalidIntrinsics);
    }

    if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
        return Err(ProjectionError::NonFiniteInput);
    }

    if point.z <= 0.0 {
        return Err(ProjectionError::NonPositiveDepth);
    }

    let projected = PixelPoint {
        u: intrinsics.fx * point.x / point.z + intrinsics.cx,
        v: intrinsics.fy * point.y / point.z + intrinsics.cy,
    };

    if !projected.u.is_finite() || !projected.v.is_finite() {
        return Err(ProjectionError::NonFiniteOutput);
    }

    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTRINSICS: PinholeIntrinsics = PinholeIntrinsics {
        fx: 1_000.0,
        fy: 1_000.0,
        cx: 640.0,
        cy: 360.0,
    };

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }

    #[test]
    fn optical_axis_maps_to_principal_point() {
        let actual = project_point(
            INTRINSICS,
            CameraPoint3 {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            },
        )
        .expect("valid optical-axis point should project");

        // Independent closed-form answer for X=Y=0: u=cx and v=cy.
        assert_eq!(actual, PixelPoint { u: 640.0, v: 360.0 });
    }

    #[test]
    fn off_axis_projection_matches_hand_calculated_reference() {
        // Hand calculation: u=1000*0.1/2+640=690;
        // v=1000*(-0.05)/2+360=335.
        let actual = project_point(
            INTRINSICS,
            CameraPoint3 {
                x: 0.1,
                y: -0.05,
                z: 2.0,
            },
        )
        .expect("valid off-axis point should project");

        assert_close(actual.u, 690.0, 1e-12);
        assert_close(actual.v, 335.0, 1e-12);
    }

    #[test]
    fn scaling_all_point_coordinates_preserves_projection() {
        let point = CameraPoint3 {
            x: 0.1,
            y: -0.05,
            z: 2.0,
        };
        let scaled = CameraPoint3 {
            x: point.x * 7.0,
            y: point.y * 7.0,
            z: point.z * 7.0,
        };

        let actual = project_point(INTRINSICS, point).expect("original point should project");
        let scaled_actual = project_point(INTRINSICS, scaled).expect("scaled point should project");

        // Floating-point evaluation can differ by a few ULPs after scaling.
        assert_close(actual.u, scaled_actual.u, 1e-12);
        assert_close(actual.v, scaled_actual.v, 1e-12);
    }

    #[test]
    fn principal_pixel_back_projects_to_optical_axis() {
        assert_eq!(
            ray_through_pixel(INTRINSICS, PixelPoint { u: 640.0, v: 360.0 }),
            Ok(CameraRay {
                x: 0.0,
                y: 0.0,
                z: 1.0
            })
        );
    }

    #[test]
    fn projection_and_back_projection_are_consistent() {
        let pixel = PixelPoint { u: 725.0, v: 295.0 };
        let ray = ray_through_pixel(INTRINSICS, pixel).expect("finite pixel should back-project");
        let point = CameraPoint3 {
            x: ray.x * 3.0,
            y: ray.y * 3.0,
            z: ray.z * 3.0,
        };
        let round_trip = project_point(INTRINSICS, point).expect("ray point should project");

        assert_close(round_trip.u, pixel.u, 1e-12);
        assert_close(round_trip.v, pixel.v, 1e-12);
    }

    #[test]
    fn centered_field_of_view_matches_closed_form() {
        let actual = field_of_view(INTRINSICS, 1280, 720)
            .expect("valid intrinsics and non-zero dimensions should produce FOV");

        assert_close(
            actual.horizontal_radians,
            2.0 * (640.0_f64 / 1_000.0).atan(),
            1e-12,
        );
        assert_close(
            actual.vertical_radians,
            2.0 * (360.0_f64 / 1_000.0).atan(),
            1e-12,
        );
    }

    #[test]
    fn off_center_principal_point_uses_both_image_boundaries() {
        let shifted = PinholeIntrinsics {
            cx: 600.0,
            cy: 340.0,
            ..INTRINSICS
        };
        let actual =
            field_of_view(shifted, 1280, 720).expect("valid shifted intrinsics should produce FOV");

        let expected_horizontal = (680.0_f64 / 1_000.0).atan() - (-600.0_f64 / 1_000.0).atan();
        let expected_vertical = (380.0_f64 / 1_000.0).atan() - (-340.0_f64 / 1_000.0).atan();
        assert_close(actual.horizontal_radians, expected_horizontal, 1e-12);
        assert_close(actual.vertical_radians, expected_vertical, 1e-12);
    }

    #[test]
    fn rejects_zero_image_dimensions_for_field_of_view() {
        assert_eq!(
            field_of_view(INTRINSICS, 0, 720),
            Err(ProjectionError::InvalidImageDimensions)
        );
        assert_eq!(
            field_of_view(INTRINSICS, 1280, 0),
            Err(ProjectionError::InvalidImageDimensions)
        );
    }

    #[test]
    fn rejects_non_finite_pixel_for_back_projection() {
        assert_eq!(
            ray_through_pixel(
                INTRINSICS,
                PixelPoint {
                    u: f64::NAN,
                    v: 0.0
                }
            ),
            Err(ProjectionError::NonFiniteInput)
        );
        assert_eq!(
            ray_through_pixel(
                INTRINSICS,
                PixelPoint {
                    u: 0.0,
                    v: f64::INFINITY
                }
            ),
            Err(ProjectionError::NonFiniteInput)
        );
    }

    #[test]
    fn rejects_zero_and_negative_depth() {
        for z in [0.0, -1.0] {
            assert_eq!(
                project_point(INTRINSICS, CameraPoint3 { x: 0.0, y: 0.0, z }),
                Err(ProjectionError::NonPositiveDepth)
            );
        }
    }

    #[test]
    fn rejects_non_finite_point_coordinates() {
        for point in [
            CameraPoint3 {
                x: f64::NAN,
                y: 0.0,
                z: 1.0,
            },
            CameraPoint3 {
                x: 0.0,
                y: f64::INFINITY,
                z: 1.0,
            },
            CameraPoint3 {
                x: 0.0,
                y: 0.0,
                z: f64::NEG_INFINITY,
            },
        ] {
            assert_eq!(
                project_point(INTRINSICS, point),
                Err(ProjectionError::NonFiniteInput)
            );
        }
    }

    #[test]
    fn rejects_invalid_intrinsics() {
        let invalid = [
            PinholeIntrinsics {
                fx: 0.0,
                ..INTRINSICS
            },
            PinholeIntrinsics {
                fy: -1.0,
                ..INTRINSICS
            },
            PinholeIntrinsics {
                fx: f64::NAN,
                ..INTRINSICS
            },
            PinholeIntrinsics {
                cx: f64::INFINITY,
                ..INTRINSICS
            },
        ];

        for intrinsics in invalid {
            assert_eq!(
                project_point(
                    intrinsics,
                    CameraPoint3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0
                    }
                ),
                Err(ProjectionError::InvalidIntrinsics)
            );
        }
    }

    #[test]
    fn srgb_transfer_matches_known_reference_points() {
        assert_close(
            srgb_channel_to_linear(0.04045).expect("valid encoded value"),
            0.04045 / 12.92,
            1e-15,
        );
        assert_close(
            srgb_channel_to_linear(0.5).expect("valid encoded value"),
            0.214_041_140_482_232_55,
            1e-15,
        );
        assert_close(
            linear_channel_to_srgb(0.003_130_8).expect("valid linear value"),
            12.92 * 0.003_130_8,
            1e-15,
        );
        assert_eq!(srgb_channel_to_linear(0.0), Ok(0.0));
        assert_eq!(srgb_channel_to_linear(1.0), Ok(1.0));
        assert_eq!(linear_channel_to_srgb(0.0), Ok(0.0));
        assert_eq!(linear_channel_to_srgb(1.0), Ok(1.0));
    }

    #[test]
    fn srgb_transfer_round_trips_in_gamut_values() {
        for encoded in [0.0, 0.001, 0.04044, 0.04046, 0.18, 0.5, 1.0] {
            let linear = srgb_channel_to_linear(encoded).expect("encoded channel is in gamut");
            let decoded = linear_channel_to_srgb(linear).expect("linear channel is in gamut");
            assert_close(decoded, encoded, 1e-12);
        }
    }

    #[test]
    fn srgb_transfer_rejects_non_finite_and_out_of_range_channels() {
        for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                srgb_channel_to_linear(input),
                Err(TransferError::NonFiniteInput)
            );
            assert_eq!(
                linear_channel_to_srgb(input),
                Err(TransferError::NonFiniteInput)
            );
        }

        for input in [-0.01, 1.01] {
            assert_eq!(
                srgb_channel_to_linear(input),
                Err(TransferError::OutOfRange)
            );
            assert_eq!(
                linear_channel_to_srgb(input),
                Err(TransferError::OutOfRange)
            );
        }
    }

    #[test]
    fn hlg_oetf_matches_bt2100_reference_points() {
        assert_eq!(hlg_scene_to_signal(0.0), Ok(0.0));
        assert_close(
            hlg_scene_to_signal(1.0 / 12.0).expect("valid scene light"),
            0.5,
            1e-15,
        );
        assert_close(
            hlg_scene_to_signal(0.18).expect("valid scene light"),
            0.672_358_132_598_092_5,
            1e-8,
        );
        assert_close(
            hlg_scene_to_signal(0.75).expect("valid scene light"),
            0.947_098_639_616_496_9,
            1e-8,
        );
        assert_eq!(hlg_scene_to_signal(1.0), Ok(1.0));
    }

    #[test]
    fn hlg_inverse_matches_bt2100_reference_points() {
        assert_eq!(hlg_signal_to_scene(0.0), Ok(0.0));
        assert_close(
            hlg_signal_to_scene(0.25).expect("valid HLG signal"),
            0.020_833_333_333_333_332,
            1e-15,
        );
        assert_close(
            hlg_signal_to_scene(0.5).expect("valid HLG signal"),
            1.0 / 12.0,
            1e-15,
        );
        assert_close(
            hlg_signal_to_scene(0.75).expect("valid HLG signal"),
            0.264_962_559_786_400_15,
            1e-8,
        );
        assert_eq!(hlg_signal_to_scene(1.0), Ok(1.0));
    }

    #[test]
    fn hlg_oetf_and_inverse_round_trip_scene_light() {
        for scene in [0.0, 0.001, 1.0 / 12.0, 0.18, 0.5, 0.75, 0.999, 1.0] {
            let signal = hlg_scene_to_signal(scene).expect("scene value is in range");
            let reconstructed =
                hlg_signal_to_scene(signal).expect("encoded HLG signal is in range");
            assert_close(reconstructed, scene, 1e-7);
        }
    }

    #[test]
    fn hlg_oetf_is_monotonic_over_a_deterministic_grid() {
        let mut previous = hlg_scene_to_signal(0.0).expect("zero is in range");
        for index in 1..=1000 {
            let scene = f64::from(index) / 1000.0;
            let actual = hlg_scene_to_signal(scene).expect("grid value is in range");
            assert!(
                actual >= previous,
                "HLG OETF decreased at scene value {scene}"
            );
            previous = actual;
        }
        assert_eq!(previous, 1.0);
    }

    #[test]
    fn hlg_transfer_rejects_non_finite_and_out_of_range_inputs() {
        for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                hlg_scene_to_signal(input),
                Err(TransferError::NonFiniteInput)
            );
            assert_eq!(
                hlg_signal_to_scene(input),
                Err(TransferError::NonFiniteInput)
            );
        }

        for input in [-0.001, 1.001] {
            assert_eq!(hlg_scene_to_signal(input), Err(TransferError::OutOfRange));
            assert_eq!(hlg_signal_to_scene(input), Err(TransferError::OutOfRange));
        }
    }

    #[test]
    fn pq_eotf_matches_st2084_reference_points() {
        assert_eq!(pq_signal_to_nits(0.0), Ok(0.0));
        assert_close(
            pq_signal_to_nits(0.5).expect("valid PQ signal"),
            92.245_708_994_065_27,
            1e-9,
        );
        assert_close(
            pq_signal_to_nits(0.508_078_421_517_399).expect("valid PQ signal"),
            100.0,
            1e-8,
        );
        assert_close(
            pq_signal_to_nits(0.751_827_096_247_041).expect("valid PQ signal"),
            1_000.0,
            1e-8,
        );
        assert_close(
            pq_signal_to_nits(1.0).expect("valid PQ signal"),
            10_000.0,
            1e-8,
        );
    }

    #[test]
    fn pq_oetf_matches_st2084_reference_points() {
        assert_eq!(nits_to_pq_signal(0.0), Ok(0.0));
        assert_close(
            nits_to_pq_signal(1.0).expect("valid luminance"),
            0.149_945_732_100_180_22,
            1e-12,
        );
        assert_close(
            nits_to_pq_signal(100.0).expect("valid luminance"),
            0.508_078_421_517_399,
            1e-12,
        );
        assert_close(
            nits_to_pq_signal(203.0).expect("valid luminance"),
            0.580_688_881_041_610_9,
            1e-12,
        );
        assert_close(
            nits_to_pq_signal(1_000.0).expect("valid luminance"),
            0.751_827_096_247_041,
            1e-12,
        );
        assert_close(
            nits_to_pq_signal(4_000.0).expect("valid luminance"),
            0.902_572_393_310_937_3,
            1e-12,
        );
        assert_eq!(nits_to_pq_signal(10_000.0), Ok(1.0));
    }

    #[test]
    fn pq_transfer_round_trips_luminance_range() {
        for nits in [
            0.0, 0.001, 0.01, 0.1, 1.0, 100.0, 203.0, 1_000.0, 4_000.0, 10_000.0,
        ] {
            let signal = nits_to_pq_signal(nits).expect("test luminance is in PQ range");
            let decoded = pq_signal_to_nits(signal).expect("encoded signal is in PQ range");
            assert_close(decoded, nits, 1e-7);
        }
    }

    #[test]
    fn pq_transfer_rejects_non_finite_and_out_of_range_inputs() {
        for signal in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                pq_signal_to_nits(signal),
                Err(TransferError::NonFiniteInput)
            );
            assert_eq!(
                nits_to_pq_signal(signal),
                Err(TransferError::NonFiniteInput)
            );
        }
        for signal in [-0.001, 1.001] {
            assert_eq!(pq_signal_to_nits(signal), Err(TransferError::OutOfRange));
        }
        for nits in [-0.001, 10_000.001] {
            assert_eq!(nits_to_pq_signal(nits), Err(TransferError::OutOfRange));
        }
    }

    #[test]
    fn relative_luminance_uses_linear_light_weights() {
        assert_close(
            relative_luminance_from_srgb(1.0, 0.0, 0.0).expect("valid RGB"),
            0.2126,
            1e-15,
        );
        assert_close(
            relative_luminance_from_srgb(0.0, 1.0, 0.0).expect("valid RGB"),
            0.7152,
            1e-15,
        );
        assert_close(
            relative_luminance_from_srgb(0.0, 0.0, 1.0).expect("valid RGB"),
            0.0722,
            1e-15,
        );
        assert_close(
            relative_luminance_from_srgb(1.0, 1.0, 1.0).expect("valid RGB"),
            1.0,
            1e-15,
        );
        assert_close(
            relative_luminance_from_srgb(0.5, 0.5, 0.5).expect("valid RGB"),
            0.214_041_140_482_232_55,
            1e-15,
        );
        assert_eq!(
            relative_luminance_from_srgb(0.0, f64::NAN, 0.0),
            Err(TransferError::NonFiniteInput)
        );
    }

    #[test]
    fn rejects_overflowed_projection_result() {
        let point = CameraPoint3 {
            x: f64::MAX,
            y: 0.0,
            z: f64::MIN_POSITIVE,
        };

        assert_eq!(
            project_point(INTRINSICS, point),
            Err(ProjectionError::NonFiniteOutput)
        );
    }
}
