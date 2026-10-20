// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Conservative screen-space displacement bounds for geometric LOD candidates.
//!
//! These helpers project a caller-supplied world-space geometric deviation
//! bound into pixels under explicit camera assumptions. They do not infer
//! error from a mesh, perform frustum clipping, or account for lens distortion,
//! motion blur, antialiasing, or display scaling.
//!
//! All world-space quantities must use the same length unit. For perspective,
//! the host must supply a lower bound on positive camera-space depth and an
//! upper bound on radial distance from the optical axis for every affected
//! source point. If either bound is unknown, the result must not be claimed
//! as a conservative error ceiling.

use std::f64::consts::PI;

/// Camera model and bounds required for a projected geometric-error estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionModel {
    /// Perspective projection with a vertical field of view.
    ///
    /// The minimum depth must bound every affected source point's positive
    /// camera-space Z. The radial bound must cover sqrt(x*x + y*y) for every
    /// affected source point. Geometric error must be smaller than minimum
    /// depth so perturbed points cannot cross the camera plane.
    Perspective {
        /// Vertical viewport resolution in physical render pixels.
        viewport_height_px: u32,
        /// Vertical field of view in radians, strictly between 0 and PI.
        vertical_fov_radians: f64,
        /// Conservative lower bound on positive camera-space depth.
        min_camera_depth_units: f64,
        /// Conservative upper bound on radial camera-space distance.
        max_radial_distance_units: f64,
    },
    /// Orthographic projection with a known vertical world-space span.
    Orthographic {
        /// Vertical viewport resolution in physical render pixels.
        viewport_height_px: u32,
        /// World-space distance represented by the full viewport height.
        vertical_span_units: f64,
    },
}

/// Invalid projection assumptions or an unrepresentable projected-error bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    /// Geometric error must be finite and non-negative.
    InvalidGeometricError,
    /// A viewport must have at least one vertical render pixel.
    InvalidViewportHeight,
    /// Perspective FOV must be finite and strictly between 0 and PI.
    InvalidVerticalFieldOfView,
    /// Minimum perspective depth must be finite and strictly positive.
    InvalidMinimumDepth,
    /// Maximum radial distance must be finite and non-negative.
    InvalidRadialDistance,
    /// Geometric error reaches or crosses the nearest possible camera plane.
    ErrorReachesCameraPlane,
    /// Orthographic vertical span must be finite and strictly positive.
    InvalidOrthographicSpan,
    /// Floating-point arithmetic produced a non-finite or negative result.
    UnrepresentableResult,
}

/// Compute an upper bound on projected geometric displacement, in pixels.
///
/// The world-space error bound must use the same length unit as camera bounds.
///
/// For orthographic projection, the bound is geometric_error times viewport
/// height divided by the world-space vertical span.
///
/// For a pinhole perspective camera, let f be the focal length in pixels,
/// z_min the lower bound on positive camera-space depth, rho_max the upper
/// bound on radial distance from the optical axis, and delta the geometric
/// error bound. When delta is strictly less than z_min, a conservative bound
/// is:
///
///     f * delta * (z_min + rho_max) / (z_min * (z_min - delta))
///
/// This follows by bounding the projected displacement for any 3D point
/// perturbation whose Euclidean norm is at most delta. It is conservative only
/// when the supplied depth and radial bounds cover the complete affected
/// geometry. The caller must independently establish that the source geometry
/// error bound itself is valid for the entire representation transition.
///
/// This does not replace view-frustum clipping and does not establish a mesh
/// simplification error bound. Arithmetic uses ordinary f64 operations rather
/// than directed-rounding interval arithmetic; a certification-grade hard gate
/// must add outward-rounded numerical evaluation and independent qualification.
pub fn projected_error_bound_px(
    world_error_bound_units: f64,
    model: ProjectionModel,
) -> Result<f64, ProjectionError> {
    if !world_error_bound_units.is_finite() || world_error_bound_units < 0.0 {
        return Err(ProjectionError::InvalidGeometricError);
    }

    let viewport_height_px = match model {
        ProjectionModel::Perspective {
            viewport_height_px,
            ..
        }
        | ProjectionModel::Orthographic {
            viewport_height_px,
            ..
        } => viewport_height_px,
    };
    if viewport_height_px == 0 {
        return Err(ProjectionError::InvalidViewportHeight);
    }

    let viewport_height = f64::from(viewport_height_px);
    let projected_error = match model {
        ProjectionModel::Perspective {
            vertical_fov_radians,
            min_camera_depth_units,
            max_radial_distance_units,
            ..
        } => {
            if !vertical_fov_radians.is_finite()
                || vertical_fov_radians <= 0.0
                || vertical_fov_radians >= PI
            {
                return Err(ProjectionError::InvalidVerticalFieldOfView);
            }
            if !min_camera_depth_units.is_finite() || min_camera_depth_units <= 0.0 {
                return Err(ProjectionError::InvalidMinimumDepth);
            }
            if !max_radial_distance_units.is_finite() || max_radial_distance_units < 0.0 {
                return Err(ProjectionError::InvalidRadialDistance);
            }
            if world_error_bound_units >= min_camera_depth_units {
                return Err(ProjectionError::ErrorReachesCameraPlane);
            }

            let tangent = (vertical_fov_radians / 2.0).tan();
            let focal_length_px = viewport_height / (2.0 * tangent);
            let relative_error = world_error_bound_units / min_camera_depth_units;
            let relative_radius = max_radial_distance_units / min_camera_depth_units;

            if !focal_length_px.is_finite()
                || focal_length_px <= 0.0
                || !relative_error.is_finite()
                || !(0.0..1.0).contains(&relative_error)
                || !relative_radius.is_finite()
            {
                return Err(ProjectionError::UnrepresentableResult);
            }

            let amplification = (1.0 + relative_radius) / (1.0 - relative_error);
            let base_error = focal_length_px * relative_error;
            if !amplification.is_finite() || !base_error.is_finite() {
                return Err(ProjectionError::UnrepresentableResult);
            }
            base_error * amplification
        }
        ProjectionModel::Orthographic {
            vertical_span_units,
            ..
        } => {
            if !vertical_span_units.is_finite() || vertical_span_units <= 0.0 {
                return Err(ProjectionError::InvalidOrthographicSpan);
            }
            (world_error_bound_units / vertical_span_units) * viewport_height
        }
    };

    if !projected_error.is_finite()
        || projected_error < 0.0
        || (world_error_bound_units > 0.0 && projected_error == 0.0)
    {
        return Err(ProjectionError::UnrepresentableResult);
    }

    Ok(projected_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthographic_projection_matches_analytic_scale() {
        let error = projected_error_bound_px(
            0.25,
            ProjectionModel::Orthographic {
                viewport_height_px: 1_000,
                vertical_span_units: 100.0,
            },
        )
        .expect("valid orthographic fixture");
        assert!((error - 2.5).abs() < 1e-12);
    }

    #[test]
    fn perspective_projection_matches_declared_upper_bound() {
        let error = projected_error_bound_px(
            1.0,
            ProjectionModel::Perspective {
                viewport_height_px: 1_000,
                vertical_fov_radians: PI / 2.0,
                min_camera_depth_units: 10.0,
                max_radial_distance_units: 0.0,
            },
        )
        .expect("valid perspective fixture");
        let expected = 500.0 * 1.0 * 10.0 / (10.0 * 9.0);
        assert!((error - expected).abs() < 1e-12);
    }

    #[test]
    fn perspective_bound_grows_with_error_and_radial_distance() {
        let model = ProjectionModel::Perspective {
            viewport_height_px: 1_080,
            vertical_fov_radians: 1.0,
            min_camera_depth_units: 20.0,
            max_radial_distance_units: 2.0,
        };
        let small_error = projected_error_bound_px(0.01, model).expect("small bound is valid");
        let larger_error = projected_error_bound_px(0.1, model).expect("larger bound is valid");
        let larger_radius = projected_error_bound_px(
            0.1,
            ProjectionModel::Perspective {
                max_radial_distance_units: 20.0,
                ..model
            },
        )
        .expect("larger radial bound is valid");

        assert!(larger_error > small_error);
        assert!(larger_radius > larger_error);
    }

    #[test]
    fn projection_bound_scales_with_viewport_resolution() {
        let low = projected_error_bound_px(
            0.5,
            ProjectionModel::Orthographic {
                viewport_height_px: 720,
                vertical_span_units: 10.0,
            },
        )
        .expect("720p fixture is valid");
        let high = projected_error_bound_px(
            0.5,
            ProjectionModel::Orthographic {
                viewport_height_px: 1_440,
                vertical_span_units: 10.0,
            },
        )
        .expect("1440p fixture is valid");

        assert!((high - 2.0 * low).abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_geometric_error_and_viewport() {
        let valid = ProjectionModel::Orthographic {
            viewport_height_px: 720,
            vertical_span_units: 10.0,
        };
        assert_eq!(
            projected_error_bound_px(f64::NAN, valid),
            Err(ProjectionError::InvalidGeometricError)
        );
        assert_eq!(
            projected_error_bound_px(-0.1, valid),
            Err(ProjectionError::InvalidGeometricError)
        );
        assert_eq!(
            projected_error_bound_px(
                0.1,
                ProjectionModel::Orthographic {
                    viewport_height_px: 0,
                    vertical_span_units: 10.0,
                }
            ),
            Err(ProjectionError::InvalidViewportHeight)
        );
    }

    #[test]
    fn rejects_invalid_perspective_assumptions() {
        let perspective = ProjectionModel::Perspective {
            viewport_height_px: 720,
            vertical_fov_radians: 1.0,
            min_camera_depth_units: 10.0,
            max_radial_distance_units: 2.0,
        };

        assert_eq!(
            projected_error_bound_px(
                0.1,
                ProjectionModel::Perspective {
                    vertical_fov_radians: 0.0,
                    ..perspective
                }
            ),
            Err(ProjectionError::InvalidVerticalFieldOfView)
        );
        assert_eq!(
            projected_error_bound_px(
                0.1,
                ProjectionModel::Perspective {
                    min_camera_depth_units: 0.0,
                    ..perspective
                }
            ),
            Err(ProjectionError::InvalidMinimumDepth)
        );
        assert_eq!(
            projected_error_bound_px(
                0.1,
                ProjectionModel::Perspective {
                    max_radial_distance_units: -1.0,
                    ..perspective
                }
            ),
            Err(ProjectionError::InvalidRadialDistance)
        );
        assert_eq!(
            projected_error_bound_px(10.0, perspective),
            Err(ProjectionError::ErrorReachesCameraPlane)
        );
    }

    #[test]
    fn rejects_unrepresentable_projected_error() {
        assert_eq!(
            projected_error_bound_px(
                f64::MAX,
                ProjectionModel::Orthographic {
                    viewport_height_px: u32::MAX,
                    vertical_span_units: f64::MIN_POSITIVE,
                }
            ),
            Err(ProjectionError::UnrepresentableResult)
        );
    }

    #[test]
    fn rejects_invalid_orthographic_span() {
        assert_eq!(
            projected_error_bound_px(
                0.1,
                ProjectionModel::Orthographic {
                    viewport_height_px: 720,
                    vertical_span_units: f64::INFINITY,
                }
            ),
            Err(ProjectionError::InvalidOrthographicSpan)
        );
    }
}
