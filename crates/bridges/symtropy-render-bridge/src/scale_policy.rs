// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Apparent-size-driven representation bands for planetary-to-system views.
//!
//! The calculations operate on the apparent projected diameter of a spherical
//! body. They do not decide which assets exist, implement planetary tiles, or
//! change canonical object state. The host maps the bands to real representations
//! and must protect selected/task-critical content independently of apparent size.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

/// Projection inputs needed to estimate a sphere's apparent pixel diameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApparentSizeProjection {
    /// Pinhole perspective projection.
    Perspective {
        /// Vertical viewport size in physical render pixels.
        viewport_height_px: u32,
        /// Vertical field of view in radians, strictly between 0 and PI.
        vertical_fov_radians: f64,
        /// Distance from camera position to sphere centre, in the same units as radius.
        centre_distance: f64,
    },
    /// Orthographic projection with a known vertical world-space span.
    Orthographic {
        /// Vertical viewport size in physical render pixels.
        viewport_height_px: u32,
        /// World-space distance covered by the viewport height.
        vertical_span: f64,
    },
}

/// Errors from projected apparent-size evaluation or scale-band policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApparentScaleError {
    InvalidRadius,
    InvalidViewportHeight,
    InvalidVerticalFieldOfView,
    InvalidCentreDistance,
    CameraInsideOrOnBody,
    InvalidOrthographicSpan,
    InvalidProjectedDiameter,
    InvalidSizeThresholds,
    InvalidHysteresisFraction,
}

impl fmt::Display for ApparentScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ApparentScaleError {}

/// Compute a sphere's projected diameter in physical render pixels.
///
/// In perspective, the tangent angle of the apparent limb is r/sqrt(d^2-r^2),
/// so the projected diameter is:
///
///     viewport_height * (r/d) / (tan(vertical_fov/2) * sqrt(1-(r/d)^2))
///
/// The ratio form avoids unnecessary squaring of astronomical distances. The
/// camera must be strictly outside the sphere. In orthographic projection the
/// result is 2*r*viewport_height/vertical_span.
///
/// This is an apparent-size calculation, not terrain LOD error and not a
/// substitute for frustum clipping. Atmospheric refraction, rings, ellipsoidal
/// shape, partial occlusion, and special artistic exaggeration are out of scope.
pub fn projected_sphere_diameter_px(
    radius: f64,
    projection: ApparentSizeProjection,
) -> Result<f64, ApparentScaleError> {
    if !radius.is_finite() || radius <= 0.0 {
        return Err(ApparentScaleError::InvalidRadius);
    }

    let viewport_height_px = match projection {
        ApparentSizeProjection::Perspective {
            viewport_height_px, ..
        }
        | ApparentSizeProjection::Orthographic {
            viewport_height_px, ..
        } => viewport_height_px,
    };
    if viewport_height_px == 0 {
        return Err(ApparentScaleError::InvalidViewportHeight);
    }
    let height = f64::from(viewport_height_px);

    let diameter = match projection {
        ApparentSizeProjection::Perspective {
            vertical_fov_radians,
            centre_distance,
            ..
        } => {
            if !vertical_fov_radians.is_finite()
                || vertical_fov_radians <= 0.0
                || vertical_fov_radians >= PI
            {
                return Err(ApparentScaleError::InvalidVerticalFieldOfView);
            }
            if !centre_distance.is_finite() || centre_distance <= 0.0 {
                return Err(ApparentScaleError::InvalidCentreDistance);
            }
            if centre_distance <= radius {
                return Err(ApparentScaleError::CameraInsideOrOnBody);
            }

            let radius_ratio = radius / centre_distance;
            let tangent_limb = radius_ratio / (1.0 - radius_ratio * radius_ratio).sqrt();
            height * tangent_limb / (vertical_fov_radians / 2.0).tan()
        }
        ApparentSizeProjection::Orthographic { vertical_span, .. } => {
            if !vertical_span.is_finite() || vertical_span <= 0.0 {
                return Err(ApparentScaleError::InvalidOrthographicSpan);
            }
            (radius / vertical_span) * (2.0 * height)
        }
    };

    if !diameter.is_finite() || diameter < 0.0 {
        return Err(ApparentScaleError::InvalidProjectedDiameter);
    }
    Ok(diameter)
}

/// Apparent-size band that a host can map to a representation type.
///
/// These bands intentionally describe presentation scale, not existence. A
/// subpixel body can still matter scientifically, be selected, or need a
/// labelled proxy. The host must keep identity/selection and semantic
/// protection separate from this visual band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApparentSizeBand {
    /// Below the profile's subpixel threshold.
    Subpixel,
    /// Use a point-like marker or other catalog representation.
    PointSource,
    /// Use a resolved luminous/solid disc.
    ResolvedDisc,
    /// Use a whole-body sphere/ellipsoid proxy.
    BodyProxy,
    /// Use regional features, large structures, or coarse surface tiles.
    RegionalDetail,
    /// Fine body-surface representation may be considered.
    SurfaceDetail,
}

impl ApparentSizeBand {
    fn index(self) -> usize {
        match self {
            Self::Subpixel => 0,
            Self::PointSource => 1,
            Self::ResolvedDisc => 2,
            Self::BodyProxy => 3,
            Self::RegionalDetail => 4,
            Self::SurfaceDetail => 5,
        }
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Subpixel,
            1 => Self::PointSource,
            2 => Self::ResolvedDisc,
            3 => Self::BodyProxy,
            4 => Self::RegionalDetail,
            _ => Self::SurfaceDetail,
        }
    }
}

/// Strictly increasing pixel thresholds between apparent-size bands.
///
/// The five thresholds separate subpixel/point-like, point-like/resolved-disc,
/// disc/body-proxy, body-proxy/regional, and regional/surface detail. They are
/// explicit configuration, not universal perceptual truths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApparentSizeProfile {
    pub subpixel_max_px: f64,
    pub point_source_max_px: f64,
    pub resolved_disc_max_px: f64,
    pub body_proxy_max_px: f64,
    pub regional_detail_max_px: f64,
}

impl ApparentSizeProfile {
    /// Construct a validated apparent-size threshold profile.
    pub fn new(
        subpixel_max_px: f64,
        point_source_max_px: f64,
        resolved_disc_max_px: f64,
        body_proxy_max_px: f64,
        regional_detail_max_px: f64,
    ) -> Result<Self, ApparentScaleError> {
        let profile = Self {
            subpixel_max_px,
            point_source_max_px,
            resolved_disc_max_px,
            body_proxy_max_px,
            regional_detail_max_px,
        };
        let thresholds = profile.thresholds();
        if thresholds
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || thresholds.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ApparentScaleError::InvalidSizeThresholds);
        }
        Ok(profile)
    }

    /// Starting proposal for interactive engineering/planetary navigation.
    ///
    /// These values are hypotheses to tune using fixed-camera capture and
    /// task-readability tests; they are not universal pixel-quality standards.
    pub const fn proposed_default() -> Self {
        Self {
            subpixel_max_px: 0.5,
            point_source_max_px: 1.0,
            resolved_disc_max_px: 8.0,
            body_proxy_max_px: 64.0,
            regional_detail_max_px: 256.0,
        }
    }

    /// Classify a projected diameter with no temporal hysteresis.
    pub fn classify(self, diameter_px: f64) -> Result<ApparentSizeBand, ApparentScaleError> {
        if !diameter_px.is_finite() || diameter_px < 0.0 {
            return Err(ApparentScaleError::InvalidProjectedDiameter);
        }
        self.validate()?;

        let band = if diameter_px < self.subpixel_max_px {
            ApparentSizeBand::Subpixel
        } else if diameter_px < self.point_source_max_px {
            ApparentSizeBand::PointSource
        } else if diameter_px < self.resolved_disc_max_px {
            ApparentSizeBand::ResolvedDisc
        } else if diameter_px < self.body_proxy_max_px {
            ApparentSizeBand::BodyProxy
        } else if diameter_px < self.regional_detail_max_px {
            ApparentSizeBand::RegionalDetail
        } else {
            ApparentSizeBand::SurfaceDetail
        };
        Ok(band)
    }

    fn thresholds(self) -> [f64; 5] {
        [
            self.subpixel_max_px,
            self.point_source_max_px,
            self.resolved_disc_max_px,
            self.body_proxy_max_px,
            self.regional_detail_max_px,
        ]
    }

    fn validate(self) -> Result<(), ApparentScaleError> {
        let values = self.thresholds();
        if values
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || values.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ApparentScaleError::InvalidSizeThresholds);
        }
        Ok(())
    }
}

/// Stateful band selection with symmetric threshold hysteresis.
///
/// This small state should belong to the presentation layer, not canonical
/// body state. Upgrading requires crossing a threshold times (1+h); downgrading
/// requires crossing the same threshold times (1-h). Hard semantic protection
/// and the geometric-error ceiling always override representation policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleBandLatch {
    band: ApparentSizeBand,
    hysteresis_fraction: f64,
}

impl ScaleBandLatch {
    /// Start with an explicit representation band and hysteresis fraction
    /// between 0 and 0.5 (exclusive).
    pub fn new(
        initial_band: ApparentSizeBand,
        hysteresis_fraction: f64,
    ) -> Result<Self, ApparentScaleError> {
        if !hysteresis_fraction.is_finite() || !(0.0..0.5).contains(&hysteresis_fraction) {
            return Err(ApparentScaleError::InvalidHysteresisFraction);
        }
        Ok(Self {
            band: initial_band,
            hysteresis_fraction,
        })
    }

    pub fn band(self) -> ApparentSizeBand {
        self.band
    }

    /// Update from current apparent pixel diameter and a validated profile.
    ///
    /// Large one-frame changes may cross several bands, but ordinary jitter
    /// near a boundary will not alternate representations each frame.
    pub fn update(
        &mut self,
        diameter_px: f64,
        profile: ApparentSizeProfile,
    ) -> Result<ApparentSizeBand, ApparentScaleError> {
        profile.validate()?;
        if !diameter_px.is_finite() || diameter_px < 0.0 {
            return Err(ApparentScaleError::InvalidProjectedDiameter);
        }

        let thresholds = profile.thresholds();
        let mut index = self.band.index();

        while index < 5 {
            let promote_at = thresholds[index] * (1.0 + self.hysteresis_fraction);
            if diameter_px >= promote_at {
                index += 1;
            } else {
                break;
            }
        }

        while index > 0 {
            let demote_below = thresholds[index - 1] * (1.0 - self.hysteresis_fraction);
            if diameter_px < demote_below {
                index -= 1;
            } else {
                break;
            }
        }

        self.band = ApparentSizeBand::from_index(index);
        Ok(self.band)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: ApparentSizeProfile = ApparentSizeProfile::proposed_default();

    #[test]
    fn perspective_diameter_matches_known_answer() {
        let diameter = projected_sphere_diameter_px(
            100.0,
            ApparentSizeProjection::Perspective {
                viewport_height_px: 1_080,
                vertical_fov_radians: PI / 2.0,
                centre_distance: 1_000.0,
            },
        )
        .expect("valid perspective projection");
        let expected = 1_080.0 * 0.1 / (1.0 - 0.1_f64.powi(2)).sqrt();
        assert!((diameter - expected).abs() < 1e-10);
    }

    #[test]
    fn orthographic_diameter_matches_known_answer() {
        let diameter = projected_sphere_diameter_px(
            25.0,
            ApparentSizeProjection::Orthographic {
                viewport_height_px: 1_000,
                vertical_span: 1_000.0,
            },
        )
        .expect("valid orthographic projection");
        assert!((diameter - 50.0).abs() < 1e-12);
    }

    #[test]
    fn perspective_diameter_grows_with_radius_resolution_and_fov_zoom() {
        let base = ApparentSizeProjection::Perspective {
            viewport_height_px: 1_080,
            vertical_fov_radians: 1.0,
            centre_distance: 10_000.0,
        };
        let base_diameter = projected_sphere_diameter_px(100.0, base).expect("base projection");
        let larger_body = projected_sphere_diameter_px(200.0, base).expect("larger body");
        let higher_resolution = projected_sphere_diameter_px(
            100.0,
            ApparentSizeProjection::Perspective {
                viewport_height_px: 2_160,
                vertical_fov_radians: 1.0,
                centre_distance: 10_000.0,
            },
        )
        .expect("higher resolution");
        let zoomed = projected_sphere_diameter_px(
            100.0,
            ApparentSizeProjection::Perspective {
                viewport_height_px: 1_080,
                vertical_fov_radians: 0.5,
                centre_distance: 10_000.0,
            },
        )
        .expect("narrower FOV");

        assert!(larger_body > base_diameter);
        assert!((higher_resolution - 2.0 * base_diameter).abs() < 1e-10);
        assert!(zoomed > base_diameter);
    }

    #[test]
    fn perspective_projection_is_invariant_to_common_world_unit_scale() {
        let radius = 6.371e6_f64;
        let distance = 4.217e8_f64;
        let projection = ApparentSizeProjection::Perspective {
            viewport_height_px: 2_160,
            vertical_fov_radians: 1.0,
            centre_distance: distance,
        };
        let reference = projected_sphere_diameter_px(radius, projection)
            .expect("reference projection");

        for scale in [1e-12_f64, 1e-6, 1.0, 1e6, 1e12] {
            let diameter = projected_sphere_diameter_px(
                radius * scale,
                ApparentSizeProjection::Perspective {
                    viewport_height_px: 2_160,
                    vertical_fov_radians: 1.0,
                    centre_distance: distance * scale,
                },
            )
            .expect("scaled units preserve the projection domain");
            assert!(
                (diameter - reference).abs() <= reference.abs() * 1e-12,
                "common unit scale {scale:e} changed apparent diameter: {diameter} vs {reference}"
            );
        }
    }

    #[test]
    fn perspective_projection_is_finite_and_monotonic_across_scale_decades() {
        let radius = 6.371e6_f64;
        let mut previous = f64::INFINITY;
        for distance_multiplier in [1.01_f64, 1.1, 2.0, 10.0, 1e3, 1e6, 1e12] {
            let diameter = projected_sphere_diameter_px(
                radius,
                ApparentSizeProjection::Perspective {
                    viewport_height_px: 2_160,
                    vertical_fov_radians: 1.0,
                    centre_distance: radius * distance_multiplier,
                },
            )
            .expect("finite distant-sphere projection");
            assert!(diameter.is_finite() && diameter > 0.0);
            assert!(
                diameter < previous,
                "apparent diameter did not decrease at distance multiplier {distance_multiplier}"
            );
            previous = diameter;
        }
    }

    #[test]
    fn orthographic_projection_is_invariant_to_common_world_unit_scale() {
        let radius = 25.0_f64;
        let vertical_span = 1_000.0_f64;
        let reference = projected_sphere_diameter_px(
            radius,
            ApparentSizeProjection::Orthographic {
                viewport_height_px: 1_440,
                vertical_span,
            },
        )
        .expect("reference orthographic projection");

        for scale in [1e-12_f64, 1e-6, 1.0, 1e6, 1e12] {
            let diameter = projected_sphere_diameter_px(
                radius * scale,
                ApparentSizeProjection::Orthographic {
                    viewport_height_px: 1_440,
                    vertical_span: vertical_span * scale,
                },
            )
            .expect("scaled orthographic units");
            assert!(
                (diameter - reference).abs() <= reference.abs() * 1e-12,
                "common unit scale {scale:e} changed orthographic diameter"
            );
        }
    }

    #[test]
    fn apparent_diameter_decreases_with_perspective_distance() {
        let near = projected_sphere_diameter_px(
            1.0,
            ApparentSizeProjection::Perspective {
                viewport_height_px: 1_080,
                vertical_fov_radians: 1.0,
                centre_distance: 100.0,
            },
        )
        .expect("near body");
        let far = projected_sphere_diameter_px(
            1.0,
            ApparentSizeProjection::Perspective {
                viewport_height_px: 1_080,
                vertical_fov_radians: 1.0,
                centre_distance: 1_000.0,
            },
        )
        .expect("far body");
        assert!(near > far);
    }

    #[test]
    fn rejects_invalid_projection_domains() {
        assert_eq!(
            projected_sphere_diameter_px(
                0.0,
                ApparentSizeProjection::Orthographic {
                    viewport_height_px: 1_000,
                    vertical_span: 100.0,
                }
            ),
            Err(ApparentScaleError::InvalidRadius)
        );
        assert_eq!(
            projected_sphere_diameter_px(
                1.0,
                ApparentSizeProjection::Perspective {
                    viewport_height_px: 0,
                    vertical_fov_radians: 1.0,
                    centre_distance: 10.0,
                }
            ),
            Err(ApparentScaleError::InvalidViewportHeight)
        );
        assert_eq!(
            projected_sphere_diameter_px(
                1.0,
                ApparentSizeProjection::Perspective {
                    viewport_height_px: 1_000,
                    vertical_fov_radians: PI,
                    centre_distance: 10.0,
                }
            ),
            Err(ApparentScaleError::InvalidVerticalFieldOfView)
        );
        assert_eq!(
            projected_sphere_diameter_px(
                1.0,
                ApparentSizeProjection::Perspective {
                    viewport_height_px: 1_000,
                    vertical_fov_radians: 1.0,
                    centre_distance: 1.0,
                }
            ),
            Err(ApparentScaleError::CameraInsideOrOnBody)
        );
        assert_eq!(
            projected_sphere_diameter_px(
                1.0,
                ApparentSizeProjection::Orthographic {
                    viewport_height_px: 1_000,
                    vertical_span: f64::INFINITY,
                }
            ),
            Err(ApparentScaleError::InvalidOrthographicSpan)
        );
    }

    #[test]
    fn profile_validates_monotonic_thresholds_and_classifies() {
        assert_eq!(
            ApparentSizeProfile::new(1.0, 0.5, 8.0, 64.0, 256.0),
            Err(ApparentScaleError::InvalidSizeThresholds)
        );
        assert_eq!(
            ApparentSizeProfile::new(0.5, f64::NAN, 8.0, 64.0, 256.0),
            Err(ApparentScaleError::InvalidSizeThresholds)
        );

        assert_eq!(PROFILE.classify(0.2), Ok(ApparentSizeBand::Subpixel));
        assert_eq!(PROFILE.classify(0.75), Ok(ApparentSizeBand::PointSource));
        assert_eq!(PROFILE.classify(4.0), Ok(ApparentSizeBand::ResolvedDisc));
        assert_eq!(PROFILE.classify(20.0), Ok(ApparentSizeBand::BodyProxy));
        assert_eq!(
            PROFILE.classify(100.0),
            Ok(ApparentSizeBand::RegionalDetail)
        );
        assert_eq!(PROFILE.classify(300.0), Ok(ApparentSizeBand::SurfaceDetail));
    }

    #[test]
    fn hysteresis_prevents_chatter_near_a_band_boundary() {
        let mut latch =
            ScaleBandLatch::new(ApparentSizeBand::PointSource, 0.1).expect("valid latch");

        assert_eq!(
            latch.update(1.05, PROFILE),
            Ok(ApparentSizeBand::PointSource)
        );
        assert_eq!(
            latch.update(1.11, PROFILE),
            Ok(ApparentSizeBand::ResolvedDisc)
        );
        assert_eq!(
            latch.update(0.95, PROFILE),
            Ok(ApparentSizeBand::ResolvedDisc)
        );
        assert_eq!(
            latch.update(0.89, PROFILE),
            Ok(ApparentSizeBand::PointSource)
        );
    }

    #[test]
    fn hysteresis_can_cross_multiple_bands_on_a_large_camera_move() {
        let mut latch = ScaleBandLatch::new(ApparentSizeBand::Subpixel, 0.05).expect("valid latch");
        assert_eq!(
            latch.update(500.0, PROFILE),
            Ok(ApparentSizeBand::SurfaceDetail)
        );
        assert_eq!(latch.update(0.01, PROFILE), Ok(ApparentSizeBand::Subpixel));
    }

    #[test]
    fn rejects_bad_hysteresis_and_bad_diameters() {
        assert_eq!(
            ScaleBandLatch::new(ApparentSizeBand::Subpixel, 0.5),
            Err(ApparentScaleError::InvalidHysteresisFraction)
        );
        let mut latch = ScaleBandLatch::new(ApparentSizeBand::Subpixel, 0.05).expect("valid latch");
        assert_eq!(
            latch.update(f64::NAN, PROFILE),
            Err(ApparentScaleError::InvalidProjectedDiameter)
        );
        let invalid_profile = ApparentSizeProfile {
            subpixel_max_px: 1.0,
            point_source_max_px: 0.5,
            resolved_disc_max_px: 2.0,
            body_proxy_max_px: 64.0,
            regional_detail_max_px: 256.0,
        };
        assert_eq!(
            latch.update(1.0, invalid_profile),
            Err(ApparentScaleError::InvalidSizeThresholds)
        );
    }
}
