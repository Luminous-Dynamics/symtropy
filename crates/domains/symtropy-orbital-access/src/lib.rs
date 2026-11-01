//! Analytic rotating-Earth direct-insertion geometry.
//!
//! This standalone crate is a geometry reference, not a launch vehicle,
//! ascent, orbit propagator, mission-design or range-safety model.

use std::f64::consts::PI;

/// Spherical reference Earth. The radius is the WGS-84 equatorial radius, but
/// this model intentionally treats it as a sphere.
pub const EARTH_SPHERE: EarthModel = EarthModel {
    radius_m: 6_378_137.0,
    rotation_rate_rad_s: 7.292_115_0e-5,
    rotation_angle_at_reference_epoch_rad: 0.0,
    reference_epoch_tai_seconds: 0,
};

/// Uniformly rotating spherical-Earth reference model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EarthModel {
    /// Spherical radius in metres.
    pub radius_m: f64,
    /// Positive eastward rotation rate in radians per second.
    pub rotation_rate_rad_s: f64,
    /// Inertial rotation angle at the reference epoch, in radians.
    pub rotation_angle_at_reference_epoch_rad: f64,
    /// Epoch corresponding to the reference rotation angle, in TAI seconds.
    pub reference_epoch_tai_seconds: i64,
}

/// Named frame and epoch for one evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameEpoch<'a> {
    /// Earth-fixed frame identifier for input geodetic coordinates.
    pub earth_fixed_frame_id: &'a str,
    /// Inertial frame identifier for returned vectors.
    pub inertial_frame_id: &'a str,
    /// Evaluation epoch in TAI seconds.
    pub epoch_tai_seconds: i64,
}

/// Direct-insertion inputs.
///
/// Azimuth is measured clockwise from local north in the Earth-fixed frame.
/// Negative azimuth is west of north. Speed is relative to the rotating
/// surface, not inertial speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchInput {
    pub latitude_rad: f64,
    pub longitude_rad: f64,
    pub azimuth_rad: f64,
    pub horizontal_speed_relative_to_surface_m_s: f64,
}

/// Three-dimensional vector in the named inertial frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    #[must_use]
    pub fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }

    #[must_use]
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y.mul_add(rhs.z, -self.z * rhs.y),
            y: self.z.mul_add(rhs.x, -self.x * rhs.z),
            z: self.x.mul_add(rhs.y, -self.y * rhs.x),
        }
    }

    #[must_use]
    pub fn scale(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    #[must_use]
    pub fn plus(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

/// Result of idealized direct-insertion geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchPlane<'a> {
    pub earth_fixed_frame_id: &'a str,
    pub inertial_frame_id: &'a str,
    pub epoch_tai_seconds: i64,
    pub earth_rotation_angle_rad: f64,
    pub position_inertial_m: Vec3,
    pub velocity_inertial_m_s: Vec3,
    pub angular_momentum_m2_s: Vec3,
    pub north_velocity_m_s: f64,
    pub east_velocity_inertial_m_s: f64,
    pub horizontal_speed_inertial_m_s: f64,
    pub inclination_rad: f64,
    pub inclination_deg: f64,
}

/// Fail-closed validation errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    EmptyEarthFixedFrameId,
    EmptyInertialFrameId,
    InvalidRadius,
    InvalidRotationRate,
    InvalidReferenceRotationAngle,
    InvalidLatitude,
    InvalidLongitude,
    InvalidAzimuth,
    InvalidHorizontalSpeed,
    EpochDifferenceOverflow,
    DegenerateAngularMomentum,
}

impl EarthModel {
    /// Compute inertial rotation angle at an explicitly named epoch.
    pub fn rotation_angle_at_epoch(self, epoch_tai_seconds: i64) -> Result<f64, GeometryError> {
        self.validate()?;
        let elapsed = epoch_tai_seconds
            .checked_sub(self.reference_epoch_tai_seconds)
            .ok_or(GeometryError::EpochDifferenceOverflow)? as f64;
        let angle = self.rotation_angle_at_reference_epoch_rad + self.rotation_rate_rad_s * elapsed;
        if angle.is_finite() {
            Ok(angle)
        } else {
            Err(GeometryError::InvalidReferenceRotationAngle)
        }
    }

    /// Derive the idealized orbital plane from direct horizontal insertion.
    ///
    /// Earth-fixed position and local velocity directions are rotated into the
    /// inertial frame at the supplied epoch. The vehicle's relative eastward
    /// insertion component receives the local Earth-rotation velocity before
    /// the inertial angular-momentum vector is computed.
    pub fn direct_insertion_plane<'a>(
        self,
        frame_epoch: FrameEpoch<'a>,
        input: LaunchInput,
    ) -> Result<LaunchPlane<'a>, GeometryError> {
        self.validate()?;
        if frame_epoch.earth_fixed_frame_id.trim().is_empty() {
            return Err(GeometryError::EmptyEarthFixedFrameId);
        }
        if frame_epoch.inertial_frame_id.trim().is_empty() {
            return Err(GeometryError::EmptyInertialFrameId);
        }
        if !input.latitude_rad.is_finite() {
            return Err(GeometryError::InvalidLatitude);
        }
        if !input.longitude_rad.is_finite() {
            return Err(GeometryError::InvalidLongitude);
        }
        if !input.azimuth_rad.is_finite() {
            return Err(GeometryError::InvalidAzimuth);
        }
        if !input.horizontal_speed_relative_to_surface_m_s.is_finite()
            || input.horizontal_speed_relative_to_surface_m_s <= 0.0
        {
            return Err(GeometryError::InvalidHorizontalSpeed);
        }
        if input.latitude_rad.abs() > PI / 2.0 {
            return Err(GeometryError::InvalidLatitude);
        }

        let phi = input.latitude_rad;
        let lambda = input.longitude_rad;
        let azimuth = input.azimuth_rad;
        let speed = input.horizontal_speed_relative_to_surface_m_s;
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        let cos_lambda = lambda.cos();
        let sin_lambda = lambda.sin();

        let rotation_angle = self.rotation_angle_at_epoch(frame_epoch.epoch_tai_seconds)?;
        let cos_theta = rotation_angle.cos();
        let sin_theta = rotation_angle.sin();

        // Earth-fixed unit basis vectors: local up, north and east.
        let up_ecef = Vec3 {
            x: cos_phi * cos_lambda,
            y: cos_phi * sin_lambda,
            z: sin_phi,
        };
        let north_ecef = Vec3 {
            x: -sin_phi * cos_lambda,
            y: -sin_phi * sin_lambda,
            z: cos_phi,
        };
        let east_ecef = Vec3 {
            x: -sin_lambda,
            y: cos_lambda,
            z: 0.0,
        };

        // Rotate ECEF vectors about inertial +Z into the named inertial frame.
        let rotate_to_inertial = |v: Vec3| Vec3 {
            x: cos_theta * v.x - sin_theta * v.y,
            y: sin_theta * v.x + cos_theta * v.y,
            z: v.z,
        };
        let up = rotate_to_inertial(up_ecef);
        let north = rotate_to_inertial(north_ecef);
        let east = rotate_to_inertial(east_ecef);

        let north_velocity = speed * azimuth.cos();
        let east_velocity =
            speed * azimuth.sin() + self.rotation_rate_rad_s * self.radius_m * cos_phi;
        let velocity = north.scale(north_velocity).plus(east.scale(east_velocity));
        let position = up.scale(self.radius_m);
        let angular_momentum = position.cross(velocity);
        let angular_momentum_norm = angular_momentum.norm();

        if !angular_momentum_norm.is_finite() || angular_momentum_norm <= f64::MIN_POSITIVE {
            return Err(GeometryError::DegenerateAngularMomentum);
        }

        let inclination_rad = (angular_momentum.z / angular_momentum_norm)
            .clamp(-1.0, 1.0)
            .acos();
        let horizontal_speed_inertial_m_s = north_velocity.hypot(east_velocity);

        Ok(LaunchPlane {
            earth_fixed_frame_id: frame_epoch.earth_fixed_frame_id,
            inertial_frame_id: frame_epoch.inertial_frame_id,
            epoch_tai_seconds: frame_epoch.epoch_tai_seconds,
            earth_rotation_angle_rad: rotation_angle,
            position_inertial_m: position,
            velocity_inertial_m_s: velocity,
            angular_momentum_m2_s: angular_momentum,
            north_velocity_m_s: north_velocity,
            east_velocity_inertial_m_s: east_velocity,
            horizontal_speed_inertial_m_s,
            inclination_rad,
            inclination_deg: inclination_rad.to_degrees(),
        })
    }

    fn validate(self) -> Result<(), GeometryError> {
        if !self.radius_m.is_finite() || self.radius_m <= 0.0 {
            return Err(GeometryError::InvalidRadius);
        }
        if !self.rotation_rate_rad_s.is_finite() {
            return Err(GeometryError::InvalidRotationRate);
        }
        if !self.rotation_angle_at_reference_epoch_rad.is_finite() {
            return Err(GeometryError::InvalidReferenceRotationAngle);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: FrameEpoch<'static> = FrameEpoch {
        earth_fixed_frame_id: "Earth-fixed/spherical-reference",
        inertial_frame_id: "Earth-centred-inertial/spherical-reference",
        epoch_tai_seconds: 0,
    };

    fn input(
        latitude_deg: f64,
        longitude_deg: f64,
        azimuth_deg: f64,
        speed_m_s: f64,
    ) -> LaunchInput {
        LaunchInput {
            latitude_rad: latitude_deg.to_radians(),
            longitude_rad: longitude_deg.to_radians(),
            azimuth_rad: azimuth_deg.to_radians(),
            horizontal_speed_relative_to_surface_m_s: speed_m_s,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.12}, expected={expected:.12}, tolerance={tolerance}"
        );
    }

    #[test]
    fn equatorial_due_east_is_zero_inclination() {
        let result = EARTH_SPHERE
            .direct_insertion_plane(FRAME, input(0.0, 0.0, 90.0, 7_800.0))
            .unwrap();
        close(result.inclination_deg, 0.0, 1e-10);
    }

    #[test]
    fn due_east_inclination_is_absolute_site_latitude() {
        for latitude in [-40.0, -2.5, 2.5, 40.0] {
            let result = EARTH_SPHERE
                .direct_insertion_plane(FRAME, input(latitude, 37.0, 90.0, 7_800.0))
                .unwrap();
            close(result.inclination_deg, latitude.abs(), 1e-10);
        }
    }

    #[test]
    fn equatorial_due_north_includes_earth_rotation() {
        let result = EARTH_SPHERE
            .direct_insertion_plane(FRAME, input(0.0, 0.0, 0.0, 7_800.0))
            .unwrap();
        close(result.east_velocity_inertial_m_s, 465.1010849, 1e-6);
        close(result.inclination_deg, 86.5875880313, 1e-9);
        assert!(result.inclination_deg < 90.0);
    }

    #[test]
    fn slightly_west_of_north_cancels_equatorial_rotation_for_polar_plane() {
        let azimuth = -(EARTH_SPHERE.rotation_rate_rad_s * EARTH_SPHERE.radius_m / 7_800.0).asin();
        let result = EARTH_SPHERE
            .direct_insertion_plane(
                FRAME,
                LaunchInput {
                    azimuth_rad: azimuth,
                    ..input(0.0, 0.0, 0.0, 7_800.0)
                },
            )
            .unwrap();
        close(result.east_velocity_inertial_m_s, 0.0, 1e-10);
        close(result.inclination_deg, 90.0, 1e-10);
    }

    #[test]
    fn longitude_and_epoch_rotate_vectors_but_preserve_axisymmetric_inclination() {
        let first = EARTH_SPHERE
            .direct_insertion_plane(FRAME, input(12.0, 0.0, 70.0, 7_800.0))
            .unwrap();
        let second = EARTH_SPHERE
            .direct_insertion_plane(
                FrameEpoch {
                    epoch_tai_seconds: 3_600,
                    ..FRAME
                },
                input(12.0, 42.0, 70.0, 7_800.0),
            )
            .unwrap();
        close(first.inclination_deg, second.inclination_deg, 1e-10);
        assert_ne!(first.position_inertial_m, second.position_inertial_m);
    }

    #[test]
    fn vector_result_matches_scalar_inclination_cross_check() {
        let result = EARTH_SPHERE
            .direct_insertion_plane(FRAME, input(18.0, 121.0, 58.0, 7_650.0))
            .unwrap();
        let cos_inclination = 18.0_f64.to_radians().cos() * result.east_velocity_inertial_m_s
            / result.horizontal_speed_inertial_m_s;
        let scalar_inclination = cos_inclination.clamp(-1.0, 1.0).acos();
        close(result.inclination_rad, scalar_inclination, 1e-12);
    }

    #[test]
    fn disabling_earth_rotation_changes_the_inertial_plane() {
        let still_earth = EarthModel {
            rotation_rate_rad_s: 0.0,
            ..EARTH_SPHERE
        };
        let rotating = EARTH_SPHERE
            .direct_insertion_plane(FRAME, input(0.0, 0.0, 0.0, 7_800.0))
            .unwrap();
        let stationary = still_earth
            .direct_insertion_plane(FRAME, input(0.0, 0.0, 0.0, 7_800.0))
            .unwrap();
        close(stationary.inclination_deg, 90.0, 1e-10);
        assert!(rotating.inclination_deg < stationary.inclination_deg);
    }

    #[test]
    fn non_finite_azimuth_is_rejected() {
        let mut invalid = input(0.0, 0.0, 90.0, 7_800.0);
        invalid.azimuth_rad = f64::NAN;
        let result = EARTH_SPHERE.direct_insertion_plane(FRAME, invalid);
        assert_eq!(result, Err(GeometryError::InvalidAzimuth));
    }

    #[test]
    fn inertial_frame_identity_is_required() {
        let bad_frame = FrameEpoch {
            inertial_frame_id: "  ",
            ..FRAME
        };
        let result = EARTH_SPHERE.direct_insertion_plane(bad_frame, input(0.0, 0.0, 90.0, 7_800.0));
        assert_eq!(result, Err(GeometryError::EmptyInertialFrameId));
    }

    #[test]
    fn epoch_subtraction_overflow_is_rejected() {
        let model = EarthModel {
            reference_epoch_tai_seconds: i64::MIN,
            ..EARTH_SPHERE
        };
        let result = model.rotation_angle_at_epoch(i64::MAX);
        assert_eq!(result, Err(GeometryError::EpochDifferenceOverflow));
    }

    #[test]
    fn invalid_latitude_is_rejected() {
        let result = EARTH_SPHERE.direct_insertion_plane(FRAME, input(91.0, 0.0, 90.0, 7_800.0));
        assert_eq!(result, Err(GeometryError::InvalidLatitude));
    }

    #[test]
    fn zero_speed_is_rejected() {
        let result = EARTH_SPHERE.direct_insertion_plane(FRAME, input(0.0, 0.0, 90.0, 0.0));
        assert_eq!(result, Err(GeometryError::InvalidHorizontalSpeed));
    }

    #[test]
    fn unknown_frame_identity_is_rejected() {
        let bad_frame = FrameEpoch {
            earth_fixed_frame_id: "",
            ..FRAME
        };
        let result = EARTH_SPHERE.direct_insertion_plane(bad_frame, input(0.0, 0.0, 90.0, 7_800.0));
        assert_eq!(result, Err(GeometryError::EmptyEarthFixedFrameId));
    }
}
