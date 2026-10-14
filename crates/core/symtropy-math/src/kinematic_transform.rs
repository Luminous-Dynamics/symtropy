// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Time-stamped 3D frame transforms with first and second kinematic derivatives.
//!
//! The pose maps source-frame coordinates into target-frame coordinates.
//! Angular velocity and acceleration are relative source-to-target rates,
//! expressed in the target frame. This is not an astronomical ephemeris or a
//! complete IAU/IERS frame implementation.

use crate::point::Point;
use crate::transform::Transform;
use nalgebra::Vector3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Inertial,
    NonInertial,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KinematicTransformError {
    InvalidTolerance,
    TargetFrameNotInertial,
    MissingFrameMetadata(&'static str),
    NonFiniteInput(&'static str),
    InvalidRotation { orthogonality_error: f64, determinant: f64, tolerance: f64 },
    NonFiniteResult(&'static str),
}

impl std::fmt::Display for KinematicTransformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTolerance => {
                write!(f, "rotation tolerance must be finite and non-negative")
            },
            Self::TargetFrameNotInertial => {
                write!(f, "kinematic derivative transforms require inertial target axes")
            },
            Self::MissingFrameMetadata(name) => write!(f, "{name} must be specified"),
            Self::NonFiniteInput(name) => write!(f, "{name} contains a non-finite value"),
            Self::InvalidRotation { orthogonality_error, determinant, tolerance } => write!(
                f,
                "rotation is not proper: orthogonality error {orthogonality_error}, determinant {determinant}, tolerance {tolerance}"
            ),
            Self::NonFiniteResult(name) => {
                write!(f, "kinematic transform produced non-finite {name}")
            },
        }
    }
}

impl std::error::Error for KinematicTransformError {}

/// A named source-to-target transform evaluated at one epoch.
///
/// Translation, velocity and acceleration of the source origin are expressed
/// in target-frame coordinates. Angular rates describe the source frame
/// relative to the target frame, also expressed in target-frame components.
/// The caller defines the epoch representation and must state its time scale.
#[derive(Debug, Clone)]
pub struct KinematicTransform3 {
    from_frame: String,
    to_frame: String,
    epoch: String,
    target_frame_kind: FrameKind,
    pose: Transform<3>,
    origin_velocity_target_m_s: Vector3<f64>,
    origin_acceleration_target_m_s2: Vector3<f64>,
    angular_velocity_target_rad_s: Vector3<f64>,
    angular_acceleration_target_rad_s2: Vector3<f64>,
}

impl KinematicTransform3 {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        from_frame: impl Into<String>,
        to_frame: impl Into<String>,
        epoch: impl Into<String>,
        target_frame_kind: FrameKind,
        pose: Transform<3>,
        origin_velocity_target_m_s: Vector3<f64>,
        origin_acceleration_target_m_s2: Vector3<f64>,
        angular_velocity_target_rad_s: Vector3<f64>,
        angular_acceleration_target_rad_s2: Vector3<f64>,
        tolerance: f64,
    ) -> Result<Self, KinematicTransformError> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(KinematicTransformError::InvalidTolerance);
        }

        if target_frame_kind != FrameKind::Inertial {
            return Err(KinematicTransformError::TargetFrameNotInertial);
        }

        let from_frame = from_frame.into();
        let to_frame = to_frame.into();
        let epoch = epoch.into();
        for (name, value) in [
            ("from_frame", from_frame.as_str()),
            ("to_frame", to_frame.as_str()),
            ("epoch", epoch.as_str()),
        ] {
            let normalized = value.trim().to_ascii_lowercase();
            if normalized.is_empty()
                || matches!(normalized.as_str(), "unknown" | "tbd" | "unspecified" | "?")
            {
                return Err(KinematicTransformError::MissingFrameMetadata(name));
            }
        }

        if pose.translation.0.iter().any(|value| !value.is_finite()) {
            return Err(KinematicTransformError::NonFiniteInput("translation"));
        }
        if pose.rotation.to_matrix().iter().any(|value| !value.is_finite()) {
            return Err(KinematicTransformError::NonFiniteInput("rotation_matrix"));
        }
        for (name, vector) in [
            ("origin_velocity_target_m_s", &origin_velocity_target_m_s),
            ("origin_acceleration_target_m_s2", &origin_acceleration_target_m_s2),
            ("angular_velocity_target_rad_s", &angular_velocity_target_rad_s),
            ("angular_acceleration_target_rad_s2", &angular_acceleration_target_rad_s2),
        ] {
            require_finite(name, vector)?;
        }

        let orthogonality_error = pose.rotation.orthogonality_error();
        let determinant = pose.rotation.determinant();
        if !pose.rotation.is_proper_rotation(tolerance) {
            return Err(KinematicTransformError::InvalidRotation {
                orthogonality_error,
                determinant,
                tolerance,
            });
        }

        Ok(Self {
            from_frame,
            to_frame,
            epoch,
            target_frame_kind,
            pose,
            origin_velocity_target_m_s,
            origin_acceleration_target_m_s2,
            angular_velocity_target_rad_s,
            angular_acceleration_target_rad_s2,
        })
    }

    pub fn from_frame(&self) -> &str { &self.from_frame }
    pub fn to_frame(&self) -> &str { &self.to_frame }
    pub fn epoch(&self) -> &str { &self.epoch }
    pub fn target_frame_kind(&self) -> FrameKind { self.target_frame_kind }

    /// Transform source-frame position into target-frame components.
    pub fn transform_position(
        &self,
        position_source_m: &Point<3>,
    ) -> Result<Point<3>, KinematicTransformError> {
        require_finite("position_source_m", &position_source_m.0)?;
        let result = self.pose.transform_point(position_source_m);
        require_finite_result("position_target_m", &result.0)?;
        Ok(result)
    }

    /// Transform velocity including origin motion and omega cross (R times r).
    pub fn transform_velocity(
        &self,
        position_source_m: &Point<3>,
        velocity_source_m_s: &Vector3<f64>,
    ) -> Result<Vector3<f64>, KinematicTransformError> {
        require_finite("position_source_m", &position_source_m.0)?;
        require_finite("velocity_source_m_s", velocity_source_m_s)?;
        let r = self.pose.rotation.rotate_vector(&position_source_m.0);
        let v = self.pose.rotation.rotate_vector(velocity_source_m_s);
        let result = self.origin_velocity_target_m_s
            + v
            + cross(&self.angular_velocity_target_rad_s, &r);
        require_finite_result("velocity_target_m_s", &result)?;
        Ok(result)
    }

    /// Transform acceleration including Coriolis, angular-acceleration and
    /// centripetal terms, under the source-to-target convention above.
    pub fn transform_acceleration(
        &self,
        position_source_m: &Point<3>,
        velocity_source_m_s: &Vector3<f64>,
        acceleration_source_m_s2: &Vector3<f64>,
    ) -> Result<Vector3<f64>, KinematicTransformError> {
        require_finite("position_source_m", &position_source_m.0)?;
        require_finite("velocity_source_m_s", velocity_source_m_s)?;
        require_finite("acceleration_source_m_s2", acceleration_source_m_s2)?;

        let r = self.pose.rotation.rotate_vector(&position_source_m.0);
        let v = self.pose.rotation.rotate_vector(velocity_source_m_s);
        let a = self.pose.rotation.rotate_vector(acceleration_source_m_s2);
        let omega = &self.angular_velocity_target_rad_s;
        let alpha = &self.angular_acceleration_target_rad_s2;
        let result = self.origin_acceleration_target_m_s2
            + a
            + cross(omega, &v) * 2.0
            + cross(alpha, &r)
            + cross(omega, &cross(omega, &r));
        require_finite_result("acceleration_target_m_s2", &result)?;
        Ok(result)
    }
}

fn cross(a: &Vector3<f64>, b: &Vector3<f64>) -> Vector3<f64> {
    Vector3::new(
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )
}

fn require_finite(name: &'static str, value: &Vector3<f64>) -> Result<(), KinematicTransformError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(KinematicTransformError::NonFiniteInput(name))
    }
}

fn require_finite_result(
    name: &'static str,
    value: &Vector3<f64>,
) -> Result<(), KinematicTransformError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(KinematicTransformError::NonFiniteResult(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rotor::Rotor;

    fn vec(x: f64, y: f64, z: f64) -> Vector3<f64> {
        Vector3::new(x, y, z)
    }

    fn identity(
        origin_velocity: Vector3<f64>,
        origin_acceleration: Vector3<f64>,
        omega: Vector3<f64>,
        alpha: Vector3<f64>,
    ) -> KinematicTransform3 {
        KinematicTransform3::try_new(
            "source",
            "target",
            "fixture://epoch",
            FrameKind::Inertial,
            Transform::identity(),
            origin_velocity, origin_acceleration, omega, alpha, 1e-12,
        ).unwrap()
    }

    #[test]
    fn inertial_identity_maps_position_velocity_and_acceleration() {
        let t = identity(
            vec(1.0, 2.0, 3.0),
            vec(4.0, 5.0, 6.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
        );
        assert_eq!(t.target_frame_kind(), FrameKind::Inertial);
        let p = Point::new([7.0, 8.0, 9.0]);
        assert_eq!(
            t.transform_position(&p).unwrap(),
            Point::new([7.0, 8.0, 9.0])
        );
        assert_eq!(
            t.transform_velocity(&p, &vec(0.5, 1.0, 1.5)).unwrap(),
            vec(1.5, 3.0, 4.5)
        );
        assert_eq!(
            t.transform_acceleration(
                &p,
                &vec(0.5, 1.0, 1.5),
                &vec(1.0, 1.0, 1.0)
            )
                .unwrap(),
            vec(5.0, 6.0, 7.0)
        );
    }

    #[test]
    fn rotating_frame_adds_transport_velocity() {
        let t = identity(
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 2.0),
            vec(0.0, 0.0, 0.0),
        );
        let velocity = t
            .transform_velocity(&Point::new([1.0, 0.0, 0.0]), &vec(0.0, 0.0, 0.0))
            .unwrap();
        assert_eq!(velocity, vec(0.0, 2.0, 0.0));
    }

    #[test]
    fn acceleration_includes_coriolis_term() {
        let t = identity(
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 2.0),
            vec(0.0, 0.0, 0.0),
        );
        let acceleration = t
            .transform_acceleration(
                &Point::origin(),
                &vec(1.0, 0.0, 0.0),
                &vec(0.0, 0.0, 0.0),
            )
            .unwrap();
        assert_eq!(acceleration, vec(0.0, 4.0, 0.0));
    }

    #[test]
    fn acceleration_includes_centripetal_term() {
        let t = identity(
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 2.0),
            vec(0.0, 0.0, 0.0),
        );
        let acceleration = t
            .transform_acceleration(
                &Point::new([1.0, 0.0, 0.0]),
                &vec(0.0, 0.0, 0.0),
                &vec(0.0, 0.0, 0.0),
            )
            .unwrap();
        assert_eq!(acceleration, vec(-4.0, 0.0, 0.0));
    }

    #[test]
    fn acceleration_includes_angular_acceleration_term() {
        let t = identity(
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 3.0),
        );
        let acceleration = t
            .transform_acceleration(
                &Point::new([1.0, 0.0, 0.0]),
                &vec(0.0, 0.0, 0.0),
                &vec(0.0, 0.0, 0.0),
            )
            .unwrap();
        assert_eq!(acceleration, vec(0.0, 3.0, 0.0));
    }

    #[test]
    fn position_transform_rejects_non_finite_result() {
        let pose = Transform {
            translation: Point::new([f64::MAX, f64::MAX, f64::MAX]),
            rotation: Rotor::<3>::identity(),
        };
        let t = KinematicTransform3::try_new(
            "source",
            "target",
            "fixture://epoch",
            FrameKind::Inertial,
            pose,
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            1e-12,
        )
        .unwrap();
        let result = t.transform_position(&Point::new([f64::MAX, f64::MAX, f64::MAX]));
        assert!(matches!(
            result,
            Err(KinematicTransformError::NonFiniteResult("position_target_m"))
        ));
    }

    #[test]
    fn constructor_rejects_non_finite_tolerance() {
        let result = KinematicTransform3::try_new(
            "source",
            "target",
            "fixture://epoch",
            FrameKind::Inertial,
            Transform::identity(),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0),
            f64::NAN,
        );
        assert!(matches!(
            result,
            Err(KinematicTransformError::InvalidTolerance)
        ));
    }

    #[test]
    fn constructor_rejects_non_proper_rotation() {
        let pose = Transform {
            translation: Point::origin(),
            rotation: Rotor::from_matrix(nalgebra::SMatrix::<f64, 3, 3>::identity() * 2.0),
        };
        let result = KinematicTransform3::try_new(
            "source",
            "target",
            "fixture://epoch",
            FrameKind::Inertial,
            pose,
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0), 1e-12,
        );
        assert!(matches!(
            result,
            Err(KinematicTransformError::InvalidRotation { .. })
        ));
    }

    #[test]
    fn constructor_rejects_non_inertial_target_frame() {
        let result = KinematicTransform3::try_new(
            "source", "rotating-target", "fixture://epoch", FrameKind::NonInertial,
            Transform::identity(),
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0), 1e-12,
        );
        assert!(matches!(
            result,
            Err(KinematicTransformError::TargetFrameNotInertial)
        ));
    }

    #[test]
    fn constructor_rejects_missing_frame_and_nonfinite_rates() {
        let missing = KinematicTransform3::try_new(
            "",
            "target",
            "fixture://epoch",
            FrameKind::Inertial,
            Transform::identity(),
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0), 1e-12,
        );
        assert!(matches!(
            missing,
            Err(KinematicTransformError::MissingFrameMetadata("from_frame"))
        ));

        let nonfinite = KinematicTransform3::try_new(
            "source",
            "target",
            "fixture://epoch",
            FrameKind::Inertial,
            Transform::identity(),
            vec(f64::NAN, 0.0, 0.0), vec(0.0, 0.0, 0.0),
            vec(0.0, 0.0, 0.0), vec(0.0, 0.0, 0.0), 1e-12,
        );
        assert!(matches!(
            nonfinite,
            Err(KinematicTransformError::NonFiniteInput(
                "origin_velocity_target_m_s"
            ))
        ));
    }
}
