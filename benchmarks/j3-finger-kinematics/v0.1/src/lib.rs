#![forbid(unsafe_code)]

//! Geometry-bounded reconstruction of the PACT four-DoF finger.
//!
//! The paper supplies link lengths, joint order and pose angles, but not an
//! explicit transform chain or complete axis/frame convention. The convention
//! implemented here is therefore an explicit reconstruction hypothesis, not
//! a source-transcribed Jacobian or exact PACT reproduction.

use std::error::Error;
use std::fmt::{Display, Formatter};

pub type Vector3 = [f64; 3];
pub type Matrix3x4 = [[f64; 4]; 3];
pub type JointAnglesRad = [f64; 4];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KinematicsError {
    NonFiniteInput,
    NonPositiveAbductionLink,
    NonPositivePhalanxLength,
    NonPositiveDifferenceStep,
    NonFiniteResult,
}

impl Display for KinematicsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(formatter, "kinematic inputs must be finite"),
            Self::NonPositiveAbductionLink => write!(formatter, "abduction link length must be positive"),
            Self::NonPositivePhalanxLength => write!(formatter, "phalanx lengths must be positive"),
            Self::NonPositiveDifferenceStep => write!(formatter, "finite-difference step must be positive"),
            Self::NonFiniteResult => write!(formatter, "kinematic result is not finite"),
        }
    }
}

impl Error for KinematicsError {}

/// Serial-chain dimensions in SI units.
///
/// The abduction link is interpreted as an offset along local +Y from the
/// MCP A/A origin to the MCP F/E origin. This must be checked against CAD or
/// author-supplied geometry before claiming source equivalence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FingerGeometry {
    abduction_link_m: f64,
    phalanx_lengths_m: [f64; 3],
}

impl FingerGeometry {
    pub fn new(
        abduction_link_m: f64,
        phalanx_lengths_m: [f64; 3],
    ) -> Result<Self, KinematicsError> {
        if !abduction_link_m.is_finite()
            || phalanx_lengths_m.iter().any(|length| !length.is_finite())
        {
            return Err(KinematicsError::NonFiniteInput);
        }
        if abduction_link_m <= 0.0 {
            return Err(KinematicsError::NonPositiveAbductionLink);
        }
        if phalanx_lengths_m.iter().any(|length| *length <= 0.0) {
            return Err(KinematicsError::NonPositivePhalanxLength);
        }
        Ok(Self {
            abduction_link_m,
            phalanx_lengths_m,
        })
    }

    /// PACT-reported dimensions under the declared serial-chain interpretation.
    pub fn pact_dimension_reconstruction() -> Self {
        Self {
            abduction_link_m: 0.010,
            phalanx_lengths_m: [0.045, 0.025, 0.020],
        }
    }

    pub fn abduction_link_m(&self) -> f64 {
        self.abduction_link_m
    }

    pub fn phalanx_lengths_m(&self) -> [f64; 3] {
        self.phalanx_lengths_m
    }

    /// Fingertip position in the declared base frame, in metres.
    ///
    /// Joint order is [MCP A/A, MCP F/E, PIP F/E, DIP F/E]. Inputs are radians.
    /// Positive flexion rotates the finger from local +Y toward local +Z.
    /// Positive A/A rotates the finger plane about local +Z.
    pub fn forward_kinematics(
        &self,
        q_rad: JointAnglesRad,
    ) -> Result<Vector3, KinematicsError> {
        validate_angles(q_rad)?;

        let q_aa = q_rad[0];
        let flexion = [
            q_rad[1],
            q_rad[1] + q_rad[2],
            q_rad[1] + q_rad[2] + q_rad[3],
        ];

        let mut local_y = self.abduction_link_m;
        let mut local_z = 0.0;
        for (length, angle) in self.phalanx_lengths_m.iter().zip(flexion) {
            local_y += length * angle.cos();
            local_z += length * angle.sin();
        }

        let (sin_aa, cos_aa) = q_aa.sin_cos();
        finite_vector([-sin_aa * local_y, cos_aa * local_y, local_z])
    }

    /// Analytic geometric Jacobian d(position)/d(q), in metres per radian.
    ///
    /// This is the derivative of this crate's declared transform chain. It is
    /// not represented as the unpublished PACT source Jacobian.
    pub fn fingertip_jacobian(
        &self,
        q_rad: JointAnglesRad,
    ) -> Result<Matrix3x4, KinematicsError> {
        validate_angles(q_rad)?;

        let q_aa = q_rad[0];
        let flexion = [
            q_rad[1],
            q_rad[1] + q_rad[2],
            q_rad[1] + q_rad[2] + q_rad[3],
        ];

        let mut local_y = self.abduction_link_m;
        for (length, angle) in self.phalanx_lengths_m.iter().zip(flexion) {
            local_y += length * angle.cos();
        }

        let (sin_aa, cos_aa) = q_aa.sin_cos();
        let mut jacobian = [[0.0_f64; 4]; 3];

        // A/A rotates the entire downstream chain about the base +Z axis.
        jacobian[0][0] = -cos_aa * local_y;
        jacobian[1][0] = -sin_aa * local_y;

        // Each flexion coordinate rotates its own and all distal phalanges.
        for joint in 0..3 {
            let mut derivative_y = 0.0;
            let mut derivative_z = 0.0;
            for segment in joint..3 {
                let angle = flexion[segment];
                let length = self.phalanx_lengths_m[segment];
                derivative_y -= length * angle.sin();
                derivative_z += length * angle.cos();
            }

            let column = joint + 1;
            jacobian[0][column] = -sin_aa * derivative_y;
            jacobian[1][column] = cos_aa * derivative_y;
            jacobian[2][column] = derivative_z;
        }

        if jacobian.iter().flatten().all(|value| value.is_finite()) {
            Ok(jacobian)
        } else {
            Err(KinematicsError::NonFiniteResult)
        }
    }

    /// Independent central-difference derivative of forward_kinematics.
    ///
    /// Intended as a regression oracle, not as the production Jacobian.
    pub fn central_difference_jacobian(
        &self,
        q_rad: JointAnglesRad,
        step_rad: f64,
    ) -> Result<Matrix3x4, KinematicsError> {
        validate_angles(q_rad)?;
        if !step_rad.is_finite() {
            return Err(KinematicsError::NonFiniteInput);
        }
        if step_rad <= 0.0 {
            return Err(KinematicsError::NonPositiveDifferenceStep);
        }

        let mut jacobian = [[0.0_f64; 4]; 3];
        for joint in 0..4 {
            let mut plus = q_rad;
            let mut minus = q_rad;
            plus[joint] += step_rad;
            minus[joint] -= step_rad;

            let point_plus = self.forward_kinematics(plus)?;
            let point_minus = self.forward_kinematics(minus)?;
            for axis in 0..3 {
                jacobian[axis][joint] =
                    (point_plus[axis] - point_minus[axis]) / (2.0 * step_rad);
            }
        }

        if jacobian.iter().flatten().all(|value| value.is_finite()) {
            Ok(jacobian)
        } else {
            Err(KinematicsError::NonFiniteResult)
        }
    }

    /// Convert a published pose-table row from degrees to radians.
    pub fn pose_from_degrees(
        &self,
        q_degrees: [f64; 4],
    ) -> Result<JointAnglesRad, KinematicsError> {
        if q_degrees.iter().any(|value| !value.is_finite()) {
            return Err(KinematicsError::NonFiniteInput);
        }
        let q_rad = q_degrees.map(f64::to_radians);
        validate_angles(q_rad)?;
        Ok(q_rad)
    }

    /// Generalized torque implied by a Cartesian point force: tau = J^T f.
    pub fn generalized_torque(
        &self,
        q_rad: JointAnglesRad,
        force_n: Vector3,
    ) -> Result<[f64; 4], KinematicsError> {
        if force_n.iter().any(|value| !value.is_finite()) {
            return Err(KinematicsError::NonFiniteInput);
        }
        let jacobian = self.fingertip_jacobian(q_rad)?;
        let mut torque = [0.0_f64; 4];
        for joint in 0..4 {
            for axis in 0..3 {
                torque[joint] += jacobian[axis][joint] * force_n[axis];
            }
        }
        if torque.iter().all(|value| value.is_finite()) {
            Ok(torque)
        } else {
            Err(KinematicsError::NonFiniteResult)
        }
    }
}

fn validate_angles(q_rad: JointAnglesRad) -> Result<(), KinematicsError> {
    if q_rad.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(KinematicsError::NonFiniteInput)
    }
}

fn finite_vector(value: Vector3) -> Result<Vector3, KinematicsError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(value)
    } else {
        Err(KinematicsError::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACT_POSES_DEG: [[f64; 4]; 8] = [
        [0.0, 60.0, 60.0, 30.0],
        [0.0, 45.0, 45.0, 20.0],
        [0.0, 25.0, 30.0, 15.0],
        [0.0, 40.0, 50.0, 25.0],
        [0.0, 55.0, 60.0, 30.0],
        [0.0, 15.0, 20.0, 10.0],
        [0.0, 70.0, 70.0, 35.0],
        [15.0, 45.0, 45.0, 20.0],
    ];

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15e}, expected {expected:.15e}, tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn straight_finger_position_matches_declared_dimensions() {
        let geometry = FingerGeometry::pact_dimension_reconstruction();
        let point = geometry
            .forward_kinematics([0.0, 0.0, 0.0, 0.0])
            .expect("valid pose");
        close(point[0], 0.0, 1.0e-15);
        close(point[1], 0.100, 1.0e-15);
        close(point[2], 0.0, 1.0e-15);
    }

    #[test]
    fn published_eight_pose_rows_have_finite_tip_positions_and_jacobians() {
        let geometry = FingerGeometry::pact_dimension_reconstruction();
        for pose in PACT_POSES_DEG {
            let q = geometry.pose_from_degrees(pose).expect("valid source pose");
            let point = geometry.forward_kinematics(q).expect("finite point");
            let jacobian = geometry.fingertip_jacobian(q).expect("finite Jacobian");
            assert!(point.iter().all(|value| value.is_finite()));
            assert!(jacobian.iter().flatten().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn analytic_jacobian_matches_central_difference_at_all_eight_poses() {
        let geometry = FingerGeometry::pact_dimension_reconstruction();
        for (pose_index, pose) in PACT_POSES_DEG.iter().enumerate() {
            let q = geometry.pose_from_degrees(*pose).expect("valid source pose");
            let analytic = geometry.fingertip_jacobian(q).expect("analytic Jacobian");
            let numeric = geometry
                .central_difference_jacobian(q, 1.0e-6)
                .expect("numeric Jacobian");

            for axis in 0..3 {
                for joint in 0..4 {
                    assert!(
                        (analytic[axis][joint] - numeric[axis][joint]).abs() <= 2.0e-10,
                        "pose {}, axis {}, joint {}: analytic={}, numeric={}",
                        pose_index + 1,
                        axis,
                        joint,
                        analytic[axis][joint],
                        numeric[axis][joint]
                    );
                }
            }
        }
    }

    #[test]
    fn nonzero_abduction_changes_the_fingertip_plane_orientation() {
        let geometry = FingerGeometry::pact_dimension_reconstruction();
        let neutral = geometry
            .forward_kinematics([0.0, 0.4, 0.5, 0.2])
            .expect("neutral");
        let abducted = geometry
            .forward_kinematics([15.0_f64.to_radians(), 0.4, 0.5, 0.2])
            .expect("abducted");
        assert!((abducted[0] - neutral[0]).abs() > 1.0e-4);
        assert!((abducted[1] - neutral[1]).abs() > 1.0e-4);
        close(abducted[2], neutral[2], 1.0e-14);
    }

    #[test]
    fn generalized_torque_obeys_virtual_work_for_small_perturbation() {
        let geometry = FingerGeometry::pact_dimension_reconstruction();
        let q = geometry
            .pose_from_degrees([15.0, 45.0, 45.0, 20.0])
            .expect("pose");
        let force = [2.0, -3.0, 5.0];
        let delta_q = [2.0e-6, -1.0e-6, 1.5e-6, 0.5e-6];
        let torque = geometry
            .generalized_torque(q, force)
            .expect("generalized torque");
        let mut q_next = q;
        for joint in 0..4 {
            q_next[joint] += delta_q[joint];
        }
        let p0 = geometry.forward_kinematics(q).expect("p0");
        let p1 = geometry.forward_kinematics(q_next).expect("p1");
        let delta_p = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let cartesian_work =
            force[0] * delta_p[0] + force[1] * delta_p[1] + force[2] * delta_p[2];
        let generalized_work = (0..4)
            .map(|joint| torque[joint] * delta_q[joint])
            .sum::<f64>();
        close(cartesian_work, generalized_work, 2.0e-12);
    }

    #[test]
    fn rejects_invalid_geometry_angles_and_difference_steps() {
        assert_eq!(
            FingerGeometry::new(0.0, [0.045, 0.025, 0.020]),
            Err(KinematicsError::NonPositiveAbductionLink)
        );
        assert_eq!(
            FingerGeometry::new(0.010, [0.045, -0.025, 0.020]),
            Err(KinematicsError::NonPositivePhalanxLength)
        );

        let geometry = FingerGeometry::pact_dimension_reconstruction();
        assert_eq!(
            geometry.forward_kinematics([f64::NAN, 0.0, 0.0, 0.0]),
            Err(KinematicsError::NonFiniteInput)
        );
        assert_eq!(
            geometry.central_difference_jacobian([0.0; 4], 0.0),
            Err(KinematicsError::NonPositiveDifferenceStep)
        );
        assert_eq!(
            geometry.pose_from_degrees([0.0, f64::INFINITY, 0.0, 0.0]),
            Err(KinematicsError::NonFiniteInput)
        );
    }
}
