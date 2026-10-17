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

        assert_eq!(
            project_point(INTRINSICS, point),
            project_point(INTRINSICS, scaled)
        );
    }

    #[test]
    fn rejects_zero_and_negative_depth() {
        for z in [0.0, -1.0] {
            assert_eq!(
                project_point(
                    INTRINSICS,
                    CameraPoint3 { x: 0.0, y: 0.0, z }
                ),
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
            PinholeIntrinsics { fx: 0.0, ..INTRINSICS },
            PinholeIntrinsics { fy: -1.0, ..INTRINSICS },
            PinholeIntrinsics { fx: f64::NAN, ..INTRINSICS },
            PinholeIntrinsics { cx: f64::INFINITY, ..INTRINSICS },
        ];

        for intrinsics in invalid {
            assert_eq!(
                project_point(
                    intrinsics,
                    CameraPoint3 { x: 0.0, y: 0.0, z: 1.0 }
                ),
                Err(ProjectionError::InvalidIntrinsics)
            );
        }
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
