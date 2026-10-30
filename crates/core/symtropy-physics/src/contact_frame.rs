// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Rotation-covariant N-dimensional contact frames and bivector decomposition.
//!
//! For a unit contact normal n and tangent space T=n-perp, the angular
//! bivector space decomposes orthogonally as Λ²(R^D) = (n∧T) ⊕ Λ²T.
//! The first subspace represents rolling directions; the second represents
//! torsional directions.

use nalgebra::{SMatrix, SVector};
use symtropy_math::{Bivector, Rotor};

/// Failure while constructing or using an N-dimensional contact frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactFrameError {
    InvalidDimension,
    InvalidNormal,
    InvalidTolerance,
    NonFinite,
    CoordinateCountMismatch,
}

/// Coordinates of a bivector in the rolling/torsional contact basis.
#[derive(Clone, Debug)]
pub struct ContactAngularCoordinates {
    pub rolling: Vec<f64>,
    pub torsional: Vec<f64>,
    /// Norm of the component not reconstructed by the contact basis.
    pub reconstruction_error: f64,
}

/// Orthonormal contact frame for a unit normal in R^D.
#[derive(Clone, Debug)]
pub struct ContactFrame<const D: usize> {
    normal: SVector<f64, D>,
    tangent_basis: Vec<SVector<f64, D>>,
    rolling_basis: Vec<Bivector<D>>,
    torsional_basis: Vec<Bivector<D>>,
}

impl<const D: usize> ContactFrame<D> {
    /// Construct a deterministic orthonormal frame from an arbitrary nonzero normal.
    ///
    /// Modified Gram-Schmidt with deterministic largest-residual pivoting is
    /// used to avoid choosing a fragile fixed coordinate axis near alignment.
    pub fn new(
        normal: SVector<f64, D>,
        relative_tolerance: f64,
    ) -> Result<Self, ContactFrameError> {
        if D < 2 {
            return Err(ContactFrameError::InvalidDimension);
        }
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(ContactFrameError::InvalidTolerance);
        }
        if !normal.iter().all(|value| value.is_finite()) {
            return Err(ContactFrameError::NonFinite);
        }
        let normal_norm = normal.norm();
        if !normal_norm.is_finite() || normal_norm <= relative_tolerance {
            return Err(ContactFrameError::InvalidNormal);
        }
        let unit_normal = normal / normal_norm;
        let tangent_basis = Self::build_tangent_basis(&unit_normal, relative_tolerance)?;
        Ok(Self::from_orthonormal_parts(unit_normal, tangent_basis))
    }

    fn build_tangent_basis(
        normal: &SVector<f64, D>,
        relative_tolerance: f64,
    ) -> Result<Vec<SVector<f64, D>>, ContactFrameError> {
        let mut used_axes = vec![false; D];
        let mut tangent_basis: Vec<SVector<f64, D>> = Vec::with_capacity(D - 1);

        for _ in 0..(D - 1) {
            let mut best_axis = None;
            let mut best_vector = SVector::<f64, D>::zeros();
            let mut best_norm = 0.0_f64;

            for axis in 0..D {
                if used_axes[axis] {
                    continue;
                }
                let mut candidate = SVector::<f64, D>::zeros();
                candidate[axis] = 1.0;
                candidate -= normal * normal[axis];

                // Reorthogonalize to suppress loss of orthogonality.
                for _ in 0..2 {
                    for tangent in &tangent_basis {
                        candidate -= tangent * tangent.dot(&candidate);
                    }
                }

                let candidate_norm = candidate.norm();
                if candidate_norm > best_norm {
                    best_axis = Some(axis);
                    best_vector = candidate;
                    best_norm = candidate_norm;
                }
            }

            if !best_norm.is_finite() || best_norm <= relative_tolerance {
                return Err(ContactFrameError::InvalidNormal);
            }
            let axis = best_axis.ok_or(ContactFrameError::InvalidNormal)?;
            used_axes[axis] = true;
            tangent_basis.push(best_vector / best_norm);
        }

        Ok(tangent_basis)
    }

    fn from_orthonormal_parts(
        normal: SVector<f64, D>,
        tangent_basis: Vec<SVector<f64, D>>,
    ) -> Self {
        let rolling_basis = tangent_basis
            .iter()
            .map(|tangent| Bivector::from_wedge(&normal, tangent))
            .collect();
        let mut torsional_basis = Vec::new();
        for i in 0..tangent_basis.len() {
            for j in (i + 1)..tangent_basis.len() {
                torsional_basis.push(Bivector::from_wedge(
                    &tangent_basis[i],
                    &tangent_basis[j],
                ));
            }
        }
        Self {
            normal,
            tangent_basis,
            rolling_basis,
            torsional_basis,
        }
    }

    /// Unit contact normal.
    pub fn normal(&self) -> &SVector<f64, D> {
        &self.normal
    }

    /// Orthonormal basis of T=n-perp, with D-1 vectors.
    pub fn tangent_basis(&self) -> &[SVector<f64, D>] {
        &self.tangent_basis
    }

    /// Orthonormal rolling bivector basis n∧T, with D-1 elements.
    pub fn rolling_basis(&self) -> &[Bivector<D>] {
        &self.rolling_basis
    }

    /// Orthonormal torsional bivector basis Λ²T, with (D-1)(D-2)/2 elements.
    pub fn torsional_basis(&self) -> &[Bivector<D>] {
        &self.torsional_basis
    }

    /// Orthogonal projector onto the contact tangent space.
    pub fn tangent_projector(&self) -> SMatrix<f64, D, D> {
        SMatrix::<f64, D, D>::identity() - self.normal * self.normal.transpose()
    }

    /// Project a vector onto the contact tangent space.
    pub fn project_tangent(&self, vector: &SVector<f64, D>) -> SVector<f64, D> {
        vector - self.normal * self.normal.dot(vector)
    }

    /// Rotate the complete contact frame while preserving its local basis orientation.
    pub fn rotated_by(&self, rotation: &Rotor<D>) -> Self {
        let normal = rotation.rotate_vector(&self.normal);
        let tangent_basis = self
            .tangent_basis
            .iter()
            .map(|tangent| rotation.rotate_vector(tangent))
            .collect();
        Self::from_orthonormal_parts(normal, tangent_basis)
    }

    /// Decompose a bivector into rolling and torsional contact coordinates.
    ///
    /// The returned reconstruction error should be at roundoff scale because
    /// the two subspaces form an orthogonal basis of Λ²(R^D).
    pub fn decompose_angular(
        &self,
        angular: &Bivector<D>,
    ) -> Result<ContactAngularCoordinates, ContactFrameError> {
        if !angular.is_finite() {
            return Err(ContactFrameError::NonFinite);
        }
        let rolling: Vec<_> = self
            .rolling_basis
            .iter()
            .map(|basis| bivector_inner_product(angular, basis))
            .collect();
        let torsional: Vec<_> = self
            .torsional_basis
            .iter()
            .map(|basis| bivector_inner_product(angular, basis))
            .collect();
        let reconstructed = self.reconstruct_angular_components(&rolling, &torsional)?;
        let residual = angular.add(&reconstructed.scale(-1.0));
        Ok(ContactAngularCoordinates {
            rolling,
            torsional,
            reconstruction_error: bivector_norm(&residual),
        })
    }

    /// Reconstruct an angular bivector from rolling and torsional coordinates.
    pub fn reconstruct_angular(
        &self,
        coordinates: &ContactAngularCoordinates,
    ) -> Result<Bivector<D>, ContactFrameError> {
        self.reconstruct_angular_components(&coordinates.rolling, &coordinates.torsional)
    }

    fn reconstruct_angular_components(
        &self,
        rolling: &[f64],
        torsional: &[f64],
    ) -> Result<Bivector<D>, ContactFrameError> {
        if rolling.len() != self.rolling_basis.len()
            || torsional.len() != self.torsional_basis.len()
        {
            return Err(ContactFrameError::CoordinateCountMismatch);
        }
        if !rolling.iter().all(|value| value.is_finite())
            || !torsional.iter().all(|value| value.is_finite())
        {
            return Err(ContactFrameError::NonFinite);
        }

        let mut result = Bivector::zero();
        for (coefficient, basis) in rolling.iter().zip(&self.rolling_basis) {
            for i in 0..D {
                for j in (i + 1)..D {
                    result.set(i, j, result.get(i, j) + coefficient * basis.get(i, j));
                }
            }
        }
        for (coefficient, basis) in torsional.iter().zip(&self.torsional_basis) {
            for i in 0..D {
                for j in (i + 1)..D {
                    result.set(i, j, result.get(i, j) + coefficient * basis.get(i, j));
                }
            }
        }
        Ok(result)
    }
}

fn bivector_inner_product<const D: usize>(
    left: &Bivector<D>,
    right: &Bivector<D>,
) -> f64 {
    let mut result = 0.0;
    for i in 0..D {
        for j in (i + 1)..D {
            result += left.get(i, j) * right.get(i, j);
        }
    }
    result
}

fn bivector_norm<const D: usize>(value: &Bivector<D>) -> f64 {
    bivector_inner_product(value, value).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SVector;

    fn assert_frame<const D: usize>(normal: SVector<f64, D>) {
        let frame = ContactFrame::<D>::new(normal, 1e-12).unwrap();
        assert_eq!(frame.tangent_basis.len(), D - 1);
        assert_eq!(frame.rolling_basis.len(), D - 1);
        assert_eq!(
            frame.torsional_basis.len(),
            (D - 1) * (D - 2) / 2
        );

        assert!((frame.normal.norm() - 1.0).abs() < 1e-12);
        for tangent in &frame.tangent_basis {
            assert!(frame.normal.dot(tangent).abs() < 1e-12);
            assert!((tangent.norm() - 1.0).abs() < 1e-12);
        }
        for i in 0..frame.tangent_basis.len() {
            for j in 0..frame.tangent_basis.len() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((frame.tangent_basis[i].dot(&frame.tangent_basis[j]) - expected).abs() < 1e-12);
            }
        }

        let angular = Bivector::<D>::from_wedge(
            &SVector::<f64, D>::from_fn(|i, _| (i as f64 + 0.5) * 0.37),
            &SVector::<f64, D>::from_fn(|i, _| (i as f64 + 1.0) * -0.21),
        );
        let coordinates = frame.decompose_angular(&angular).unwrap();
        let reconstructed = frame.reconstruct_angular(&coordinates).unwrap();
        assert!(coordinates.reconstruction_error < 1e-10);
        assert!(
            bivector_norm(&angular.add(&reconstructed.scale(-1.0))) < 1e-10
        );
    }

    #[test]
    fn tangent_and_bivector_decomposition_is_complete_in_2d_3d_4d_and_5d() {
        assert_frame::<2>(SVector::new(1.0, 0.0));
        assert_frame::<3>(SVector::new(0.3, -0.4, 0.5));
        assert_frame::<4>(SVector::new(0.1, -0.2, 0.3, 0.4));
        assert_frame::<5>(SVector::new(-0.2, 0.3, 0.4, -0.1, 0.5));
    }

    #[test]
    fn coordinate_axis_and_near_axis_normals_do_not_break_frame_construction() {
        for axis in 0..4 {
            let normal = SVector::<f64, 4>::from_fn(|index, _| {
                if index == axis { 1.0 } else { 0.0 }
            });
            assert_frame::<4>(normal);
        }
        assert_frame::<4>(SVector::new(1.0, 1e-14, -1e-14, 2e-14));
    }

    #[test]
    fn rotating_contact_frame_preserves_decomposition_and_orthogonality() {
        let frame = ContactFrame::<4>::new(SVector::new(0.2, 0.4, -0.1, 0.3), 1e-12).unwrap();
        let angular = Bivector::<4>::from_wedge(
            &SVector::new(0.2, 0.4, 0.6, 0.8),
            &SVector::new(-0.3, 0.1, 0.5, -0.2),
        );
        let rotation = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(0, 3), 0.71);
        let rotated_frame = frame.rotated_by(&rotation);
        let rotated_angular = Bivector::<4>::from_wedge(
            &rotation.rotate_vector(&SVector::new(0.2, 0.4, 0.6, 0.8)),
            &rotation.rotate_vector(&SVector::new(-0.3, 0.1, 0.5, -0.2)),
        );
        let coordinates = rotated_frame.decompose_angular(&rotated_angular).unwrap();
        let reconstructed = rotated_frame.reconstruct_angular(&coordinates).unwrap();
        assert!(coordinates.reconstruction_error < 1e-10);
        assert!(bivector_norm(&rotated_angular.add(&reconstructed.scale(-1.0))) < 1e-10);
        assert!(
            (rotated_frame.normal.norm() - 1.0).abs() < 1e-12
        );
    }

    #[test]
    fn invalid_normals_and_coordinate_lengths_are_rejected() {
        assert_eq!(
            ContactFrame::<3>::new(SVector::zeros(), 1e-12).unwrap_err(),
            ContactFrameError::InvalidNormal
        );
        assert_eq!(
            ContactFrame::<1>::new(SVector::new(1.0), 1e-12).unwrap_err(),
            ContactFrameError::InvalidDimension
        );

        let frame = ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let bad = ContactAngularCoordinates {
            rolling: vec![1.0],
            torsional: Vec::new(),
            reconstruction_error: 0.0,
        };
        assert_eq!(
            frame.reconstruct_angular(&bad).unwrap_err(),
            ContactFrameError::CoordinateCountMismatch
        );
    }
}
