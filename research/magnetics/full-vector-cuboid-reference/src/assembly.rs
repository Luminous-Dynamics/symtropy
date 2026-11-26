// SPDX-License-Identifier: Apache-2.0 OR MIT
#![forbid(unsafe_code)]

//! Rigid assemblies of uniformly polarized cuboids.
//!
//! Source fields are evaluated independently in each cuboid's local frame,
//! transformed into a common global frame, and superposed. Each cuboid has
//! prescribed homogeneous polarization; mutual demagnetizing interaction is
//! not solved.

use std::collections::HashSet;
use std::fmt;

use crate::{CuboidFieldError, FieldVectorTesla, PositionMeters, UniformlyPolarizedCuboid};

const ROTATION_TOLERANCE: f64 = 1.0e-12;

/// A proper rigid transform from a cuboid-local frame to the assembly frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidPose {
    rotation: [[f64; 3]; 3],
    translation: PositionMeters,
}

/// One identified, uniformly polarized finite cuboid and its global pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedCuboid {
    id: u64,
    cuboid: UniformlyPolarizedCuboid,
    pose: RigidPose,
}

/// A non-empty assembly of independently prescribed cuboid sources.
#[derive(Clone, Debug, PartialEq)]
pub struct MagneticAssembly {
    components: Vec<PlacedCuboid>,
}

/// Assembly construction or field-evaluation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblyError {
    EmptyAssembly,
    DuplicateComponentId(u64),
    NonFiniteRotation,
    NonFiniteTranslation,
    NonOrthonormalRotation,
    ImproperRotation,
    NonFiniteObserver,
    ComponentField {
        component_id: u64,
        source: CuboidFieldError,
    },
    NonFiniteFieldSum,
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAssembly => f.write_str("magnetic assembly contains no components"),
            Self::DuplicateComponentId(id) => {
                write!(f, "duplicate magnetic component identifier: {id}")
            }
            Self::NonFiniteRotation => f.write_str("rigid-pose rotation contains a non-finite value"),
            Self::NonFiniteTranslation => {
                f.write_str("rigid-pose translation contains a non-finite value")
            }
            Self::NonOrthonormalRotation => {
                f.write_str("rigid-pose rotation is not orthonormal within tolerance")
            }
            Self::ImproperRotation => {
                f.write_str("rigid-pose transform is a reflection, not a proper rotation")
            }
            Self::NonFiniteObserver => f.write_str("global observer position is non-finite"),
            Self::ComponentField { component_id, source } => {
                write!(f, "component {component_id} field evaluation failed: {source}")
            }
            Self::NonFiniteFieldSum => f.write_str("assembly field superposition is non-finite"),
        }
    }
}

impl std::error::Error for AssemblyError {}

impl RigidPose {
    /// Construct a validated local-to-global rigid pose.
    ///
    /// The matrix must be finite, orthonormal, and have determinant +1 within
    /// ROTATION_TOLERANCE. Scale, shear and reflections are rejected.
    pub fn new(
        rotation: [[f64; 3]; 3],
        translation: PositionMeters,
    ) -> Result<Self, AssemblyError> {
        if !rotation.iter().flatten().all(|value| value.is_finite()) {
            return Err(AssemblyError::NonFiniteRotation);
        }
        if !translation.0.iter().all(|value| value.is_finite()) {
            return Err(AssemblyError::NonFiniteTranslation);
        }

        for (row_index, row_values) in rotation.iter().enumerate() {
            for (other_index, other_values) in rotation.iter().enumerate() {
                let product = dot(*row_values, *other_values);
                let expected = if row_index == other_index { 1.0 } else { 0.0 };
                if (product - expected).abs() > ROTATION_TOLERANCE {
                    return Err(AssemblyError::NonOrthonormalRotation);
                }
            }
        }

        let determinant = determinant3(rotation);
        if !determinant.is_finite() {
            return Err(AssemblyError::NonFiniteRotation);
        }
        if determinant < 0.0 {
            return Err(AssemblyError::ImproperRotation);
        }
        if (determinant - 1.0).abs() > ROTATION_TOLERANCE {
            return Err(AssemblyError::NonOrthonormalRotation);
        }

        Ok(Self { rotation, translation })
    }

    /// Identity rotation and zero translation.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: PositionMeters([0.0, 0.0, 0.0]),
        }
    }

    #[must_use]
    pub const fn rotation(self) -> [[f64; 3]; 3] {
        self.rotation
    }

    #[must_use]
    pub const fn translation(self) -> PositionMeters {
        self.translation
    }

    /// Map a global point into the component-local frame using R^T * (x - t).
    fn to_local_point(self, global: PositionMeters) -> Result<PositionMeters, AssemblyError> {
        if !global.0.iter().all(|value| value.is_finite()) {
            return Err(AssemblyError::NonFiniteObserver);
        }
        let displacement = subtract(global.0, self.translation.0);
        let local = [
            self.rotation[0][0] * displacement[0]
                + self.rotation[1][0] * displacement[1]
                + self.rotation[2][0] * displacement[2],
            self.rotation[0][1] * displacement[0]
                + self.rotation[1][1] * displacement[1]
                + self.rotation[2][1] * displacement[2],
            self.rotation[0][2] * displacement[0]
                + self.rotation[1][2] * displacement[1]
                + self.rotation[2][2] * displacement[2],
        ];
        if !local.iter().all(|value| value.is_finite()) {
            return Err(AssemblyError::NonFiniteObserver);
        }
        Ok(PositionMeters(local))
    }

    /// Rotate a local vector into the global frame using R * v.
    fn to_global_vector(self, local: [f64; 3]) -> [f64; 3] {
        [
            self.rotation[0][0] * local[0]
                + self.rotation[0][1] * local[1]
                + self.rotation[0][2] * local[2],
            self.rotation[1][0] * local[0]
                + self.rotation[1][1] * local[1]
                + self.rotation[1][2] * local[2],
            self.rotation[2][0] * local[0]
                + self.rotation[2][1] * local[1]
                + self.rotation[2][2] * local[2],
        ]
    }
}

impl PlacedCuboid {
    /// Place one prescribed-polarization cuboid in the global assembly frame.
    #[must_use]
    pub const fn new(id: u64, cuboid: UniformlyPolarizedCuboid, pose: RigidPose) -> Self {
        Self { id, cuboid, pose }
    }

    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }

    #[must_use]
    pub const fn cuboid(self) -> UniformlyPolarizedCuboid {
        self.cuboid
    }

    #[must_use]
    pub const fn pose(self) -> RigidPose {
        self.pose
    }

    /// Evaluate this component's field at a global observer.
    pub fn field_at_global(
        self,
        observer: PositionMeters,
    ) -> Result<FieldVectorTesla, AssemblyError> {
        let local_observer = self.pose.to_local_point(observer)?;
        let local_field = self.cuboid.field_at(local_observer).map_err(|source| {
            AssemblyError::ComponentField {
                component_id: self.id,
                source,
            }
        })?;
        let global_field = self.pose.to_global_vector(local_field.0);
        if !global_field.iter().all(|value| value.is_finite()) {
            return Err(AssemblyError::NonFiniteFieldSum);
        }
        Ok(FieldVectorTesla(global_field))
    }
}

impl MagneticAssembly {
    /// Construct a non-empty assembly with unique component identities.
    ///
    /// Physical overlap, mechanical clearance, retention and manufacturing
    /// tolerances are outside this mathematical reference.
    ///
    /// # Errors
    ///
    /// Rejects an empty component list or duplicate component IDs.
    pub fn new(components: Vec<PlacedCuboid>) -> Result<Self, AssemblyError> {
        if components.is_empty() {
            return Err(AssemblyError::EmptyAssembly);
        }
        let mut identifiers = HashSet::with_capacity(components.len());
        for component in &components {
            if !identifiers.insert(component.id) {
                return Err(AssemblyError::DuplicateComponentId(component.id));
            }
        }
        Ok(Self { components })
    }

    #[must_use]
    pub fn components(&self) -> &[PlacedCuboid] {
        &self.components
    }

    /// Evaluate the prescribed-source field of the complete assembly.
    ///
    /// Each cuboid is evaluated independently in its own frame, transformed
    /// into the global frame, and superposed with per-axis Kahan compensation.
    /// This is exact for prescribed source polarizations in the linear
    /// free-space formulation; it does not update polarization in response to
    /// neighbouring magnets.
    pub fn field_at(
        &self,
        observer: PositionMeters,
    ) -> Result<FieldVectorTesla, AssemblyError> {
        if !observer.0.iter().all(|value| value.is_finite()) {
            return Err(AssemblyError::NonFiniteObserver);
        }

        let mut sum = [0.0_f64; 3];
        let mut compensation = [0.0_f64; 3];
        for component in &self.components {
            let field = component.field_at_global(observer)?;
            for (axis, contribution) in field.0.iter().enumerate() {
                let corrected = contribution - compensation[axis];
                let next = sum[axis] + corrected;
                compensation[axis] = (next - sum[axis]) - corrected;
                sum[axis] = next;
            }
            if !sum.iter().all(|value| value.is_finite()) {
                return Err(AssemblyError::NonFiniteFieldSum);
            }
        }
        Ok(FieldVectorTesla(sum))
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn determinant3(matrix: [[f64; 3]; 3]) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HalfDimensionsMeters, PolarizationTesla};

    fn cuboid(half: [f64; 3], polarization: [f64; 3]) -> UniformlyPolarizedCuboid {
        UniformlyPolarizedCuboid::new(
            HalfDimensionsMeters(half),
            PolarizationTesla(polarization),
        )
        .unwrap()
    }

    fn pose(rotation: [[f64; 3]; 3], translation: [f64; 3]) -> RigidPose {
        RigidPose::new(rotation, PositionMeters(translation)).unwrap()
    }

    fn close_vector(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
        for (axis, (actual_value, expected_value)) in actual.iter().zip(expected.iter()).enumerate()
        {
            assert!(
                (actual_value - expected_value).abs() <= tolerance,
                "axis {axis}: actual={actual_value:.17e}, expected={expected_value:.17e}, tolerance={tolerance:.3e}"
            );
        }
    }

    fn rotate_z(vector: [f64; 3]) -> [f64; 3] {
        [-vector[1], vector[0], vector[2]]
    }

    #[test]
    fn identity_pose_reproduces_single_cuboid_reference() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        let point = PositionMeters([0.03, 0.02, 0.01]);
        let single = magnet.field_at(point).unwrap();
        let placed = PlacedCuboid::new(1, magnet, RigidPose::identity());
        let assembly = MagneticAssembly::new(vec![placed]).unwrap();
        assert_eq!(assembly.field_at(point).unwrap(), single);
    }

    #[test]
    fn translated_rotated_component_matches_explicit_local_transform() {
        let rotation = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let transform = pose(rotation, [0.2, -0.1, 0.05]);
        let magnet = cuboid([0.05, 0.04, 0.03], [0.0, 0.0, 1.0]);
        let component = PlacedCuboid::new(17, magnet, transform);
        let point = PositionMeters([0.31, -0.02, 0.09]);
        let displacement = subtract(point.0, [0.2, -0.1, 0.05]);
        let local = PositionMeters([
            rotation[0][0] * displacement[0]
                + rotation[1][0] * displacement[1]
                + rotation[2][0] * displacement[2],
            rotation[0][1] * displacement[0]
                + rotation[1][1] * displacement[1]
                + rotation[2][1] * displacement[2],
            rotation[0][2] * displacement[0]
                + rotation[1][2] * displacement[1]
                + rotation[2][2] * displacement[2],
        ]);
        let local_field = magnet.field_at(local).unwrap();
        let expected = [
            rotation[0][0] * local_field.0[0]
                + rotation[0][1] * local_field.0[1]
                + rotation[0][2] * local_field.0[2],
            rotation[1][0] * local_field.0[0]
                + rotation[1][1] * local_field.0[1]
                + rotation[1][2] * local_field.0[2],
            rotation[2][0] * local_field.0[0]
                + rotation[2][1] * local_field.0[1]
                + rotation[2][2] * local_field.0[2],
        ];
        close_vector(component.field_at_global(point).unwrap().0, expected, 1.0e-14);
    }

    #[test]
    fn global_rotation_covariance_rotates_component_and_observer() {
        let base_pose = pose(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.02, -0.01, 0.03],
        );
        let component = PlacedCuboid::new(1, cuboid([0.05, 0.04, 0.03], [0.2, 0.5, 0.9]), base_pose);
        let point = PositionMeters([0.08, 0.04, 0.11]);
        let original = component.field_at_global(point).unwrap();

        let rotation = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let rotated_pose = pose(rotation, rotate_z(base_pose.translation().0));
        let rotated_component = PlacedCuboid::new(1, component.cuboid(), rotated_pose);
        let rotated_point = PositionMeters(rotate_z(point.0));
        let rotated_field = rotated_component.field_at_global(rotated_point).unwrap();

        close_vector(rotated_field.0, rotate_z(original.0), 1.0e-13);
    }

    #[test]
    fn four_segment_array_matches_independent_surface_quadrature_fixture() {
        let half = [0.01, 0.01, 0.02];
        let identity = RigidPose::identity();
        let components = vec![
            PlacedCuboid::new(
                1,
                cuboid(half, [1.0, 0.0, 0.0]),
                pose(identity.rotation(), [-0.03, 0.0, 0.0]),
            ),
            PlacedCuboid::new(
                2,
                cuboid(half, [1.0, 0.0, 0.0]),
                pose(identity.rotation(), [0.03, 0.0, 0.0]),
            ),
            PlacedCuboid::new(
                3,
                cuboid(half, [0.0, 1.0, 0.0]),
                pose(identity.rotation(), [0.0, -0.03, 0.0]),
            ),
            PlacedCuboid::new(
                4,
                cuboid(half, [0.0, 1.0, 0.0]),
                pose(identity.rotation(), [0.0, 0.03, 0.0]),
            ),
        ];
        let assembly = MagneticAssembly::new(components).unwrap();
        let field = assembly.field_at(PositionMeters([0.0, 0.0, 0.0])).unwrap();
        close_vector(
            field.0,
            [0.135_645_539_175_420_44, 0.135_645_539_175_420_44, 0.0],
            2.0e-13,
        );
    }

    #[test]
    fn assembly_order_does_not_change_field_beyond_roundoff() {
        let half = [0.01, 0.01, 0.02];
        let make = |id, center, polarization| {
            PlacedCuboid::new(
                id,
                cuboid(half, polarization),
                pose(RigidPose::identity().rotation(), center),
            )
        };
        let a = make(1, [-0.03, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let b = make(2, [0.03, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let c = make(3, [0.0, -0.03, 0.0], [0.0, 1.0, 0.0]);
        let d = make(4, [0.0, 0.03, 0.0], [0.0, 1.0, 0.0]);
        let point = PositionMeters([0.0, 0.0, 0.0]);
        let first = MagneticAssembly::new(vec![a, b, c, d])
            .unwrap()
            .field_at(point)
            .unwrap();
        let reversed = MagneticAssembly::new(vec![d, c, b, a])
            .unwrap()
            .field_at(point)
            .unwrap();
        close_vector(first.0, reversed.0, 1.0e-14);
    }

    #[test]
    fn invalid_rotation_and_translation_are_rejected() {
        assert_eq!(
            RigidPose::new(
                [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                PositionMeters([0.0; 3]),
            ),
            Err(AssemblyError::NonOrthonormalRotation)
        );
        assert_eq!(
            RigidPose::new(
                [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                PositionMeters([0.0; 3]),
            ),
            Err(AssemblyError::ImproperRotation)
        );
        assert_eq!(
            RigidPose::new(
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                PositionMeters([f64::INFINITY, 0.0, 0.0]),
            ),
            Err(AssemblyError::NonFiniteTranslation)
        );
    }

    #[test]
    fn empty_assembly_and_duplicate_ids_are_rejected() {
        assert_eq!(MagneticAssembly::new(vec![]), Err(AssemblyError::EmptyAssembly));
        let component = PlacedCuboid::new(9, cuboid([0.05, 0.04, 0.03], [0.0, 0.0, 1.0]), RigidPose::identity());
        assert_eq!(
            MagneticAssembly::new(vec![component, component]),
            Err(AssemblyError::DuplicateComponentId(9))
        );
    }

    #[test]
    fn component_boundary_error_preserves_component_identity() {
        let component = PlacedCuboid::new(123, cuboid([0.05, 0.04, 0.03], [0.0, 0.0, 1.0]), RigidPose::identity());
        let assembly = MagneticAssembly::new(vec![component]).unwrap();
        assert_eq!(
            assembly.field_at(PositionMeters([0.05, 0.0, 0.0])),
            Err(AssemblyError::ComponentField {
                component_id: 123,
                source: CuboidFieldError::CoordinatePlaneBoundary
            })
        );
    }

    #[test]
    fn non_finite_observer_is_rejected_before_component_evaluation() {
        let component = PlacedCuboid::new(1, cuboid([0.05, 0.04, 0.03], [0.0, 0.0, 1.0]), RigidPose::identity());
        let assembly = MagneticAssembly::new(vec![component]).unwrap();
        assert_eq!(
            assembly.field_at(PositionMeters([f64::NAN, 0.0, 0.0])),
            Err(AssemblyError::NonFiniteObserver)
        );
    }
}
