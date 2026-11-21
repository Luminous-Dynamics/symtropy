// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Frame-explicit centroidal mass properties for analytic shape distributions.

use nalgebra::{SMatrix, SVector};

use crate::{Point, Transform};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassPropertiesError {
    UnsupportedShape,
    InvalidMass,
    InvalidGeometry,
    NonFiniteCentroid,
    NonFiniteTensor,
    AsymmetricTensor,
    NonPositiveSemidefiniteTensor,
    EmptyComponents,
    InvalidTransform,
    InvalidCombinedMass,
}

impl std::fmt::Display for MassPropertiesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedShape => write!(formatter, "shape has no declared analytic mass distribution"),
            Self::InvalidMass => write!(formatter, "mass must be finite and strictly positive"),
            Self::InvalidGeometry => write!(formatter, "shape dimensions must be finite and strictly positive"),
            Self::NonFiniteCentroid => write!(formatter, "mass centroid must be finite"),
            Self::NonFiniteTensor => write!(formatter, "mass second-moment tensor must be finite"),
            Self::AsymmetricTensor => write!(formatter, "mass second-moment tensor must be symmetric"),
            Self::NonPositiveSemidefiniteTensor => write!(formatter, "mass second-moment tensor must be positive semidefinite"),
            Self::EmptyComponents => write!(formatter, "at least one component is required"),
            Self::InvalidTransform => write!(formatter, "component transform must contain a finite translation and proper rotation"),
            Self::InvalidCombinedMass => write!(formatter, "combined mass is not finite and positive"),
        }
    }
}

impl std::error::Error for MassPropertiesError {}

/// Centroidal mass properties in the shape's local frame.
///
/// The tensor is C = integral (x-c)(x-c)^T dm. It is symmetric
/// positive-semidefinite and carries units of mass × length².
#[derive(Clone, Debug, PartialEq)]
pub struct MassProperties<const D: usize> {
    pub mass: f64,
    pub centroid: Point<D>,
    pub centroidal_second_moment: SMatrix<f64, D, D>,
}

impl<const D: usize> MassProperties<D> {
    pub fn new(
        mass: f64,
        centroid: Point<D>,
        centroidal_second_moment: SMatrix<f64, D, D>,
    ) -> Result<Self, MassPropertiesError> {
        if !mass.is_finite() || mass <= 0.0 {
            return Err(MassPropertiesError::InvalidMass);
        }
        if !centroid.0.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::NonFiniteCentroid);
        }
        if !centroidal_second_moment.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::NonFiniteTensor);
        }

        let scale = centroidal_second_moment
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let tolerance = (64.0 * f64::EPSILON * scale).max(f64::MIN_POSITIVE);
        for row in 0..D {
            for column in (row + 1)..D {
                if (centroidal_second_moment[(row, column)]
                    - centroidal_second_moment[(column, row)])
                    .abs()
                    > tolerance
                {
                    return Err(MassPropertiesError::AsymmetricTensor);
                }
            }
        }

        let symmetric =
            (centroidal_second_moment + centroidal_second_moment.transpose()) * 0.5;
        let eigen = nalgebra::linalg::SymmetricEigen::new(symmetric);
        if eigen.eigenvalues.iter().any(|value| *value < -tolerance) {
            return Err(MassPropertiesError::NonPositiveSemidefiniteTensor);
        }

        Ok(Self {
            mass,
            centroid,
            centroidal_second_moment,
        })
    }

    /// Rotate the centroidal tensor into the parent frame and transform its centroid.
    pub fn transformed(
        &self,
        transform: &Transform<D>,
    ) -> Result<Self, MassPropertiesError> {
        validate_transform(transform)?;
        let rotation = transform.rotation.to_matrix();
        let rotated = rotation * self.centroidal_second_moment * rotation.transpose();
        Self::new(
            self.mass,
            transform.transform_point(&self.centroid),
            rotated,
        )
    }
}

/// A component's local mass properties and transform into the assembly frame.
#[derive(Clone, Debug)]
pub struct MassComponent<const D: usize> {
    pub properties: MassProperties<D>,
    pub transform: Transform<D>,
}

/// Combine declared component mass properties using the tensor parallel-axis theorem.
pub fn combine_mass_properties<const D: usize>(
    components: &[MassComponent<D>],
) -> Result<MassProperties<D>, MassPropertiesError> {
    if components.is_empty() {
        return Err(MassPropertiesError::EmptyComponents);
    }

    let mut total_mass = 0.0;
    let mut weighted_centroid = SVector::<f64, D>::zeros();
    for component in components {
        validate_transform(&component.transform)?;
        let properties = MassProperties::new(
            component.properties.mass,
            component.properties.centroid,
            component.properties.centroidal_second_moment,
        )?;
        let world_centroid = component.transform.transform_point(&properties.centroid);
        total_mass += properties.mass;
        weighted_centroid += world_centroid.0 * properties.mass;
    }
    if !total_mass.is_finite() || total_mass <= 0.0 {
        return Err(MassPropertiesError::InvalidCombinedMass);
    }
    let centroid = Point(weighted_centroid / total_mass);

    let mut combined = SMatrix::<f64, D, D>::zeros();
    for component in components {
        let properties = MassProperties::new(
            component.properties.mass,
            component.properties.centroid,
            component.properties.centroidal_second_moment,
        )?;
        let rotation = component.transform.rotation.to_matrix();
        let rotated =
            rotation * properties.centroidal_second_moment * rotation.transpose();
        let component_centroid = component.transform.transform_point(&properties.centroid);
        let displacement = component_centroid.0 - centroid.0;
        combined += rotated + properties.mass * (displacement * displacement.transpose());
    }

    MassProperties::new(total_mass, centroid, combined)
}

fn validate_transform<const D: usize>(
    transform: &Transform<D>,
) -> Result<(), MassPropertiesError> {
    if !transform.translation.0.iter().all(|value| value.is_finite())
        || !transform
            .rotation
            .to_matrix()
            .iter()
            .all(|value| value.is_finite())
        || !transform.rotation.is_proper_rotation(1.0e-10)
    {
        return Err(MassPropertiesError::InvalidTransform);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bivector, Capsule, ConvexHull, HyperBox, Rotor, Shape, Sphere};
    use std::f64::consts::FRAC_PI_4;

    fn assert_matrix_near<const D: usize>(
        actual: &SMatrix<f64, D, D>,
        expected: &SMatrix<f64, D, D>,
    ) {
        assert!(
            (actual - expected).norm() < 1.0e-10,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn solid_d_ball_second_moments_have_dimension_correct_answers() {
        let mass = 3.0;
        let radius = 2.0;
        let sphere2 = Sphere::<2>::new(Point::origin(), radius)
            .mass_properties(mass)
            .expect("2D solid ball");
        let sphere3 = Sphere::<3>::new(Point::origin(), radius)
            .mass_properties(mass)
            .expect("3D solid ball");
        let sphere4 = Sphere::<4>::new(Point::origin(), radius)
            .mass_properties(mass)
            .expect("4D solid ball");

        assert_matrix_near(
            &sphere2.centroidal_second_moment,
            &(SMatrix::<f64, 2, 2>::identity() * 3.0),
        );
        assert_matrix_near(
            &sphere3.centroidal_second_moment,
            &(SMatrix::<f64, 3, 3>::identity() * 2.4),
        );
        assert_matrix_near(
            &sphere4.centroidal_second_moment,
            &(SMatrix::<f64, 4, 4>::identity() * 2.0),
        );
    }

    #[test]
    fn hyperbox_uses_axis_specific_uniform_density_second_moments() {
        let properties = HyperBox::<3>::new([1.0, 2.0, 3.0])
            .mass_properties(6.0)
            .expect("hyperbox");
        let expected =
            SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([2.0, 8.0, 18.0]));
        assert_matrix_near(&properties.centroidal_second_moment, &expected);
    }

    #[test]
    fn hyperbox_second_moments_match_2d_and_4d_known_answers() {
        let rectangle = crate::HyperBox::<2>::new([1.0, 2.0])
            .mass_properties(6.0)
            .expect("rectangle");
        let expected_2d =
            SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([2.0, 8.0]));
        assert_matrix_near(&rectangle.centroidal_second_moment, &expected_2d);

        let tesseract = crate::HyperBox::<4>::new([1.0, 2.0, 3.0, 4.0])
            .mass_properties(3.0)
            .expect("4D hyperbox");
        let expected_4d =
            SMatrix::<f64, 4, 4>::from_diagonal(&SVector::from([1.0, 4.0, 9.0, 16.0]));
        assert_matrix_near(&tesseract.centroidal_second_moment, &expected_4d);
    }

    #[test]
    fn shape_mass_properties_preserve_the_geometric_centroid() {
        let sphere = Sphere::<3>::new(Point::new([1.0, -2.0, 0.5]), 2.0)
            .mass_properties(1.0)
            .expect("sphere");
        assert_eq!(sphere.centroid, Point::new([1.0, -2.0, 0.5]));
    }

    #[test]
    fn second_moment_scales_linearly_with_mass_and_quadratically_with_size() {
        let base = HyperBox::<3>::new([1.0, 2.0, 3.0])
            .mass_properties(2.0)
            .expect("base");
        let doubled_mass = HyperBox::<3>::new([1.0, 2.0, 3.0])
            .mass_properties(4.0)
            .expect("double mass");
        let doubled_size = HyperBox::<3>::new([2.0, 4.0, 6.0])
            .mass_properties(2.0)
            .expect("double size");
        assert_matrix_near(
            &doubled_mass.centroidal_second_moment,
            &(base.centroidal_second_moment * 2.0),
        );
        assert_matrix_near(
            &doubled_size.centroidal_second_moment,
            &(base.centroidal_second_moment * 4.0),
        );
    }

    #[test]
    fn rotated_asymmetric_box_preserves_products_of_inertia() {
        let properties = HyperBox::<2>::new([1.0, 2.0])
            .mass_properties(3.0)
            .expect("box");
        let rotation =
            Rotor::from_plane_angle(&Bivector::<2>::unit_plane(0, 1), FRAC_PI_4);
        let transform = Transform {
            translation: Point::new([3.0, -2.0]),
            rotation,
        };
        let transformed = properties.transformed(&transform).expect("transform");
        let matrix = transform.rotation.to_matrix();
        let expected = matrix * properties.centroidal_second_moment * matrix.transpose();
        assert_matrix_near(&transformed.centroidal_second_moment, &expected);
        assert!(transformed.centroidal_second_moment[(0, 1)].abs() > 0.1);
    }

    #[test]
    fn compound_parallel_axis_composition_is_order_invariant() {
        let left = Sphere::<3>::new(Point::origin(), 1.0)
            .mass_properties(1.0)
            .expect("left");
        let right = Sphere::<3>::new(Point::origin(), 1.0)
            .mass_properties(1.0)
            .expect("right");
        let left_transform = Transform::from_translation(Point::new([-1.0, 0.0, 0.0]));
        let right_transform = Transform::from_translation(Point::new([1.0, 0.0, 0.0]));
        let first = combine_mass_properties(&[
            MassComponent {
                properties: left.clone(),
                transform: left_transform.clone(),
            },
            MassComponent {
                properties: right.clone(),
                transform: right_transform.clone(),
            },
        ])
        .expect("compound");
        let reversed = combine_mass_properties(&[
            MassComponent {
                properties: right,
                transform: right_transform,
            },
            MassComponent {
                properties: left,
                transform: left_transform,
            },
        ])
        .expect("reversed compound");
        assert_matrix_near(
            &first.centroidal_second_moment,
            &reversed.centroidal_second_moment,
        );
        assert!(first.centroid.0.norm() < 1.0e-12);
        assert!((first.centroidal_second_moment[(0, 0)] - 2.4).abs() < 1.0e-12);
        assert!((first.centroidal_second_moment[(1, 1)] - 0.4).abs() < 1.0e-12);
        assert!((first.centroidal_second_moment[(2, 2)] - 0.4).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_mass_geometry_and_tensor_are_rejected() {
        assert_eq!(
            Sphere::<3>::new(Point::origin(), 1.0).mass_properties(0.0),
            Err(MassPropertiesError::InvalidMass)
        );
        assert_eq!(
            Sphere::<3>::new(Point::origin(), f64::NAN).mass_properties(1.0),
            Err(MassPropertiesError::InvalidGeometry)
        );
        assert_eq!(
            HyperBox::<2>::new([1.0, 0.0]).mass_properties(1.0),
            Err(MassPropertiesError::InvalidGeometry)
        );
        let asymmetric = SMatrix::<f64, 2, 2>::new(1.0, 0.5, 0.0, 1.0);
        assert_eq!(
            MassProperties::new(1.0, Point::origin(), asymmetric),
            Err(MassPropertiesError::AsymmetricTensor)
        );
        let non_psd = SMatrix::<f64, 2, 2>::new(1.0, 2.0, 2.0, 1.0);
        assert_eq!(
            MassProperties::new(1.0, Point::origin(), non_psd),
            Err(MassPropertiesError::NonPositiveSemidefiniteTensor)
        );
    }

    #[test]
    fn rejects_nonfinite_mass_centroid_and_tensor_entries() {
        let tensor = SMatrix::<f64, 2, 2>::identity();
        assert_eq!(
            MassProperties::new(f64::INFINITY, Point::origin(), tensor),
            Err(MassPropertiesError::InvalidMass)
        );
        assert_eq!(
            MassProperties::new(1.0, Point::new([f64::NAN, 0.0]), tensor),
            Err(MassPropertiesError::NonFiniteCentroid)
        );
        let mut invalid_tensor = tensor;
        invalid_tensor[(1, 1)] = f64::NAN;
        assert_eq!(
            MassProperties::new(1.0, Point::origin(), invalid_tensor),
            Err(MassPropertiesError::NonFiniteTensor)
        );
    }

    #[test]
    fn empty_compound_mass_properties_are_rejected() {
        assert_eq!(
            combine_mass_properties::<3>(&[]),
            Err(MassPropertiesError::EmptyComponents)
        );
    }

    #[test]
    fn unsupported_shapes_abstain_instead_of_guessing() {
        let capsule = Capsule::<3>::new(1.0, 0.5, 1);
        assert_eq!(
            capsule.mass_properties(1.0),
            Err(MassPropertiesError::UnsupportedShape)
        );
        let hull = ConvexHull::<2>::hyperbox([1.0, 2.0]);
        assert_eq!(
            hull.mass_properties(1.0),
            Err(MassPropertiesError::UnsupportedShape)
        );
    }

    #[test]
    fn invalid_component_rotation_is_rejected() {
        let properties = Sphere::<2>::unit().mass_properties(1.0).expect("sphere");
        let invalid_rotation = Rotor::from_matrix(SMatrix::<f64, 2, 2>::identity() * 2.0);
        let invalid = Transform {
            translation: Point::origin(),
            rotation: invalid_rotation,
        };
        assert_eq!(
            combine_mass_properties(&[MassComponent {
                properties,
                transform: invalid,
            }]),
            Err(MassPropertiesError::InvalidTransform)
        );
    }
}
