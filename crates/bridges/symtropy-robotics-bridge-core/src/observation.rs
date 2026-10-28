// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Named observation layout for the robotics bridge's base-kinematics HDC input.
//!
//! This is deliberately only the existing 13-value base-kinematics projection.
//! It is not the complete Careframe semantic observation contract: joint
//! encoders, tool identity, task context, scene estimates, and proximity data
//! are not represented by this layout.

use serde::{Deserialize, Serialize};

/// Number of scalar values encoded by BaseKinematicObservationV0.
pub const BASE_KINEMATIC_OBSERVATION_DIM: usize = 13;

/// Stable semantic names, in the exact order emitted by
/// BaseKinematicObservationV0::try_to_ordered_values.
pub const BASE_KINEMATIC_OBSERVATION_FIELD_IDS: [&str; BASE_KINEMATIC_OBSERVATION_DIM] = [
    "robot.base_position.x_m",
    "robot.base_position.y_m",
    "robot.base_position.z_m",
    "robot.base_orientation.quaternion_x",
    "robot.base_orientation.quaternion_y",
    "robot.base_orientation.quaternion_z",
    "robot.base_orientation.quaternion_w",
    "robot.base_linear_velocity.x_m_s",
    "robot.base_linear_velocity.y_m_s",
    "robot.base_linear_velocity.z_m_s",
    "robot.base_angular_velocity_bivector.xy_rad_s",
    "robot.base_angular_velocity_bivector.xz_rad_s",
    "robot.base_angular_velocity_bivector.yz_rad_s",
];

/// The fixed-layout physical base state currently sent through the haptic
/// semantic binder. Angular velocity components are 3D bivector-plane
/// components (xy, xz, yz), not an unlabeled Cartesian xyz vector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BaseKinematicObservationV0 {
    /// Translation in metres, in world coordinates.
    pub base_position_m: [f32; 3],
    /// Unit quaternion in xyzw order.
    pub base_orientation_xyzw: [f32; 4],
    /// Linear velocity in metres per second, in world coordinates.
    pub base_linear_velocity_m_s: [f32; 3],
    /// Angular velocity bivector components in radians per second, ordered xy, xz, yz.
    pub base_angular_velocity_bivector_xy_xz_yz_rad_s: [f32; 3],
}

/// Validation failure for a base-kinematics observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseKinematicObservationError {
    /// A numeric input is NaN or infinite.
    NonFiniteComponent { index: usize },
    /// The quaternion is degenerate or outside the declared unit tolerance.
    QuaternionNotUnit,
}

impl BaseKinematicObservationV0 {
    /// Validates all values and returns the fixed ordered projection used by
    /// the haptic semantic binder. It has no controller or physics side effects.
    pub fn try_to_ordered_values(
        &self,
    ) -> Result<[f32; BASE_KINEMATIC_OBSERVATION_DIM], BaseKinematicObservationError> {
        let ordered = [
            self.base_position_m[0],
            self.base_position_m[1],
            self.base_position_m[2],
            self.base_orientation_xyzw[0],
            self.base_orientation_xyzw[1],
            self.base_orientation_xyzw[2],
            self.base_orientation_xyzw[3],
            self.base_linear_velocity_m_s[0],
            self.base_linear_velocity_m_s[1],
            self.base_linear_velocity_m_s[2],
            self.base_angular_velocity_bivector_xy_xz_yz_rad_s[0],
            self.base_angular_velocity_bivector_xy_xz_yz_rad_s[1],
            self.base_angular_velocity_bivector_xy_xz_yz_rad_s[2],
        ];
        for (index, value) in ordered.iter().enumerate() {
            if !value.is_finite() {
                return Err(BaseKinematicObservationError::NonFiniteComponent { index });
            }
        }

        let norm = self
            .base_orientation_xyzw
            .iter()
            .map(|component| component * component)
            .sum::<f32>()
            .sqrt();
        if norm <= f32::EPSILON || (norm - 1.0).abs() > 1.0e-3 {
            return Err(BaseKinematicObservationError::QuaternionNotUnit);
        }

        Ok(ordered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal() -> BaseKinematicObservationV0 {
        BaseKinematicObservationV0 {
            base_position_m: [1.0, 2.0, 3.0],
            base_orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            base_linear_velocity_m_s: [4.0, 5.0, 6.0],
            base_angular_velocity_bivector_xy_xz_yz_rad_s: [7.0, 8.0, 9.0],
        }
    }

    #[test]
    fn ordered_projection_matches_named_field_contract() {
        let observation = nominal();
        let values = observation.try_to_ordered_values().expect("valid state");
        assert_eq!(values.len(), BASE_KINEMATIC_OBSERVATION_DIM);
        assert_eq!(BASE_KINEMATIC_OBSERVATION_FIELD_IDS.len(), values.len());
        assert_eq!(
            values,
            [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        );
    }

    #[test]
    fn non_finite_values_fail_closed_with_component_index() {
        let mut observation = nominal();
        observation.base_linear_velocity_m_s[1] = f32::NAN;
        assert_eq!(
            observation.try_to_ordered_values(),
            Err(BaseKinematicObservationError::NonFiniteComponent { index: 8 })
        );

        let mut observation = nominal();
        observation.base_angular_velocity_bivector_xy_xz_yz_rad_s[2] = f32::INFINITY;
        assert_eq!(
            observation.try_to_ordered_values(),
            Err(BaseKinematicObservationError::NonFiniteComponent { index: 12 })
        );
    }

    #[test]
    fn degenerate_or_non_unit_quaternion_fails_closed() {
        let mut observation = nominal();
        observation.base_orientation_xyzw = [0.0; 4];
        assert_eq!(
            observation.try_to_ordered_values(),
            Err(BaseKinematicObservationError::QuaternionNotUnit)
        );

        let mut observation = nominal();
        observation.base_orientation_xyzw = [0.0, 0.0, 0.0, 2.0];
        assert_eq!(
            observation.try_to_ordered_values(),
            Err(BaseKinematicObservationError::QuaternionNotUnit)
        );
    }

    #[test]
    fn layout_ids_are_unique_and_non_empty() {
        let unique = BASE_KINEMATIC_OBSERVATION_FIELD_IDS
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), BASE_KINEMATIC_OBSERVATION_DIM);
        assert!(BASE_KINEMATIC_OBSERVATION_FIELD_IDS
            .iter()
            .all(|field_id| !field_id.trim().is_empty()));
    }
}
