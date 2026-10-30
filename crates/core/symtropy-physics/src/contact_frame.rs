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
    InvalidFrictionCoefficient,
    InvalidContactRadius,
    NegativeNormalImpulse,
    TangentialImpulseNotOrthogonal,
    FrictionLimitExceeded,
}

/// Coordinates of a bivector in the rolling/torsional contact basis.
#[derive(Clone, Debug)]
pub struct ContactAngularCoordinates {
    pub rolling: Vec<f64>,
    pub torsional: Vec<f64>,
    /// Norm of the component not reconstructed by the contact basis.
    pub reconstruction_error: f64,
}

/// Isotropic N-dimensional contact-wrench friction limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactWrenchFrictionLimits {
    /// Coulomb coefficient for tangential linear impulse.
    tangential_coefficient: f64,
    /// Rolling-friction coefficient; bound has units of impulse times length.
    rolling_coefficient: f64,
    /// Torsional-friction coefficient; bound has units of impulse times length.
    torsional_coefficient: f64,
    /// Declared characteristic rolling length in the same units as geometry.
    rolling_radius: f64,
    /// Declared characteristic torsional length in the same units as geometry.
    torsional_radius: f64,
}

impl ContactWrenchFrictionLimits {
    /// Create limits with explicit, positive characteristic lengths.
    pub fn new(
        tangential_coefficient: f64,
        rolling_coefficient: f64,
        torsional_coefficient: f64,
        rolling_radius: f64,
        torsional_radius: f64,
    ) -> Result<Self, ContactFrameError> {
        if !tangential_coefficient.is_finite()
            || !rolling_coefficient.is_finite()
            || !torsional_coefficient.is_finite()
            || tangential_coefficient < 0.0
            || rolling_coefficient < 0.0
            || torsional_coefficient < 0.0
        {
            return Err(ContactFrameError::InvalidFrictionCoefficient);
        }
        if !rolling_radius.is_finite()
            || !torsional_radius.is_finite()
            || rolling_radius <= 0.0
            || torsional_radius <= 0.0
        {
            return Err(ContactFrameError::InvalidContactRadius);
        }
        Ok(Self {
            tangential_coefficient,
            rolling_coefficient,
            torsional_coefficient,
            rolling_radius,
            torsional_radius,
        })
    }

    /// Tangential Coulomb coefficient.
    pub fn tangential_coefficient(&self) -> f64 {
        self.tangential_coefficient
    }

    /// Rolling-friction coefficient.
    pub fn rolling_coefficient(&self) -> f64 {
        self.rolling_coefficient
    }

    /// Torsional-friction coefficient.
    pub fn torsional_coefficient(&self) -> f64 {
        self.torsional_coefficient
    }

    /// Characteristic rolling length in geometry units.
    pub fn rolling_radius(&self) -> f64 {
        self.rolling_radius
    }

    /// Characteristic torsional length in geometry units.
    pub fn torsional_radius(&self) -> f64 {
        self.torsional_radius
    }

    fn validate_parameters(&self) -> Result<(), ContactFrameError> {
        if !self.tangential_coefficient.is_finite()
            || !self.rolling_coefficient.is_finite()
            || !self.torsional_coefficient.is_finite()
            || self.tangential_coefficient < 0.0
            || self.rolling_coefficient < 0.0
            || self.torsional_coefficient < 0.0
        {
            return Err(ContactFrameError::InvalidFrictionCoefficient);
        }
        if !self.rolling_radius.is_finite()
            || !self.torsional_radius.is_finite()
            || self.rolling_radius <= 0.0
            || self.torsional_radius <= 0.0
        {
            return Err(ContactFrameError::InvalidContactRadius);
        }
        Ok(())
    }

    /// Maximum admissible tangential linear impulse magnitude.
    pub fn tangential_bound(&self, normal_impulse: f64) -> f64 {
        self.tangential_coefficient * normal_impulse
    }

    /// Maximum admissible rolling angular impulse magnitude.
    pub fn rolling_bound(&self, normal_impulse: f64) -> f64 {
        self.rolling_coefficient * self.rolling_radius * normal_impulse
    }

    /// Maximum admissible torsional angular impulse magnitude.
    pub fn torsional_bound(&self, normal_impulse: f64) -> f64 {
        self.torsional_coefficient * self.torsional_radius * normal_impulse
    }
}

/// Contact impulse coordinates in the local normal/tangent/rolling/torsion frame.
///
/// Normal and tangential entries have units of linear impulse. Rolling and
/// torsional entries have units of angular impulse (linear impulse times length).
#[derive(Clone, Debug)]
pub struct ContactWrenchImpulse<const D: usize> {
    pub normal: f64,
    pub tangential: SVector<f64, D>,
    pub rolling: Vec<f64>,
    pub torsional: Vec<f64>,
}

impl<const D: usize> ContactWrenchImpulse<D> {
    /// Validate this wrench against the frame and isotropic friction limits.
    ///
    /// This validates feasibility only; it does not choose the maximum-
    /// dissipation impulse or account for the coupled contact effective metric.
    pub fn validate(
        &self,
        frame: &ContactFrame<D>,
        limits: &ContactWrenchFrictionLimits,
        relative_tolerance: f64,
    ) -> Result<(), ContactFrameError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(ContactFrameError::InvalidTolerance);
        }
        limits.validate_parameters()?;
        if !self.normal.is_finite()
            || !self.tangential.iter().all(|value| value.is_finite())
            || !self.rolling.iter().all(|value| value.is_finite())
            || !self.torsional.iter().all(|value| value.is_finite())
        {
            return Err(ContactFrameError::NonFinite);
        }
        if self.normal < -relative_tolerance {
            return Err(ContactFrameError::NegativeNormalImpulse);
        }
        if self.rolling.len() != frame.rolling_basis.len()
            || self.torsional.len() != frame.torsional_basis.len()
        {
            return Err(ContactFrameError::CoordinateCountMismatch);
        }

        let tangential_scale = self.tangential.norm().max(self.normal.abs()).max(1.0);
        if frame.normal.dot(&self.tangential).abs() > relative_tolerance * tangential_scale {
            return Err(ContactFrameError::TangentialImpulseNotOrthogonal);
        }

        let tangential_norm = self.tangential.norm();
        let tangential_bound = limits.tangential_bound(self.normal.max(0.0));
        if tangential_norm > tangential_bound + relative_tolerance * tangential_scale {
            return Err(ContactFrameError::FrictionLimitExceeded);
        }

        let rolling_norm = self.rolling.iter().map(|value| value * value).sum::<f64>().sqrt();
        let rolling_bound = limits.rolling_bound(self.normal.max(0.0));
        let rolling_scale = rolling_norm.max(rolling_bound).max(1.0);
        if rolling_norm > rolling_bound + relative_tolerance * rolling_scale {
            return Err(ContactFrameError::FrictionLimitExceeded);
        }

        let torsional_norm = self
            .torsional
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let torsional_bound = limits.torsional_bound(self.normal.max(0.0));
        let torsional_scale = torsional_norm.max(torsional_bound).max(1.0);
        if torsional_norm > torsional_bound + relative_tolerance * torsional_scale {
            return Err(ContactFrameError::FrictionLimitExceeded);
        }

        Ok(())
    }

    /// Reconstruct the complete angular impulse bivector.
    pub fn angular_impulse(
        &self,
        frame: &ContactFrame<D>,
    ) -> Result<Bivector<D>, ContactFrameError> {
        frame.reconstruct_angular_components(&self.rolling, &self.torsional)
    }
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

    /// Parallel-transport this frame to a new contact normal using the
    /// shortest rotation that maps the old unit normal to the new one.
    ///
    /// Unlike rebuilding a frame from coordinate axes, this preserves the
    /// transported tangent/rolling/torsional basis and therefore avoids
    /// arbitrary basis pivots during a contact's lifetime. For an exactly
    /// antipodal normal, the first tangent basis vector defines the otherwise
    /// non-unique half-turn plane deterministically.
    pub fn transported_to(
        &self,
        new_normal: SVector<f64, D>,
        relative_tolerance: f64,
    ) -> Result<Self, ContactFrameError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(ContactFrameError::InvalidTolerance);
        }
        if !new_normal.iter().all(|value| value.is_finite()) {
            return Err(ContactFrameError::NonFinite);
        }
        let new_norm = new_normal.norm();
        if !new_norm.is_finite() || new_norm <= relative_tolerance {
            return Err(ContactFrameError::InvalidNormal);
        }

        let target = new_normal / new_norm;
        let cosine = self.normal.dot(&target).clamp(-1.0, 1.0);
        let plane = Bivector::from_wedge(&self.normal, &target);
        let plane_norm = plane.norm();

        let rotation = if plane_norm <= relative_tolerance {
            if cosine >= 0.0 {
                Rotor::identity()
            } else {
                let antipodal_plane =
                    Bivector::from_wedge(&self.normal, &self.tangent_basis[0]);
                Rotor::from_plane_angle(&antipodal_plane, std::f64::consts::PI)
            }
        } else {
            Rotor::from_plane_angle(&plane, cosine.acos())
        };

        let transported = self.rotated_by(&rotation);
        if (transported.normal - target).norm() > 10.0 * relative_tolerance.max(1e-12) {
            return Err(ContactFrameError::InvalidNormal);
        }
        Ok(transported)
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

        // Exercise a generic bivector with independent values in every
        // rotation plane, not only a simple wedge (rank-two) bivector.
        let mut angular = Bivector::<D>::zero();
        for i in 0..D {
            for j in (i + 1)..D {
                angular.set(i, j, ((i + 1) as f64 * 0.37) - ((j + 2) as f64 * 0.21));
            }
        }
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
    fn tangent_projector_is_symmetric_idempotent_and_annihilates_normal() {
        let frame = ContactFrame::<5>::new(
            SVector::new(0.2, -0.3, 0.4, 0.1, -0.5),
            1e-12,
        )
        .unwrap();
        let projector = frame.tangent_projector();
        assert!((projector - projector.transpose()).norm() < 1e-12);
        assert!((projector * projector - projector).norm() < 1e-12);
        assert!((projector * frame.normal()).norm() < 1e-12);
        assert!((projector.trace() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn rolling_and_torsional_bivector_bases_are_mutually_orthonormal() {
        for dimension in [3_usize, 4, 5] {
            match dimension {
                3 => assert_bivector_basis_orthonormal::<3>(),
                4 => assert_bivector_basis_orthonormal::<4>(),
                5 => assert_bivector_basis_orthonormal::<5>(),
                _ => unreachable!(),
            }
        }
    }

    fn assert_bivector_basis_orthonormal<const D: usize>() {
        let normal = SVector::<f64, D>::from_fn(|i, _| (i as f64 + 1.0) * 0.31 - 0.7);
        let frame = ContactFrame::<D>::new(normal, 1e-12).unwrap();
        let basis: Vec<_> = frame
            .rolling_basis()
            .iter()
            .chain(frame.torsional_basis().iter())
            .collect();
        assert_eq!(basis.len(), D * (D - 1) / 2);
        for i in 0..basis.len() {
            for j in 0..basis.len() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (bivector_inner_product(basis[i], basis[j]) - expected).abs() < 1e-12,
                    "D={D}, basis ({i},{j}) is not orthonormal"
                );
            }
        }
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
        let coordinates = frame.decompose_angular(&angular).unwrap();
        let rotated_coordinates = rotated_frame.decompose_angular(&rotated_angular).unwrap();
        assert_eq!(coordinates.rolling.len(), rotated_coordinates.rolling.len());
        assert_eq!(
            coordinates.torsional.len(),
            rotated_coordinates.torsional.len()
        );
        for (before, after) in coordinates
            .rolling
            .iter()
            .chain(coordinates.torsional.iter())
            .zip(
                rotated_coordinates
                    .rolling
                    .iter()
                    .chain(rotated_coordinates.torsional.iter()),
            )
        {
            assert!((before - after).abs() < 1e-10);
        }
        let reconstructed = rotated_frame
            .reconstruct_angular(&rotated_coordinates)
            .unwrap();
        assert!(rotated_coordinates.reconstruction_error < 1e-10);
        assert!(bivector_norm(&rotated_angular.add(&reconstructed.scale(-1.0))) < 1e-10);
        assert!((rotated_frame.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn contact_wrench_bounds_validate_tangential_rolling_and_torsional_impulses() {
        let frame = ContactFrame::<4>::new(SVector::new(0.2, -0.3, 0.4, 0.1), 1e-12).unwrap();
        let limits = ContactWrenchFrictionLimits::new(0.5, 0.02, 0.03, 0.4, 0.2).unwrap();
        let wrench = ContactWrenchImpulse::<4> {
            normal: 10.0,
            tangential: frame.tangent_basis()[0] * 5.0,
            rolling: vec![0.06, 0.04, 0.0],
            torsional: vec![0.04, 0.02, 0.0],
        };
        assert!(wrench.validate(&frame, &limits, 1e-12).is_ok());
        let angular = wrench.angular_impulse(&frame).unwrap();
        let expected_norm =
            (0.06_f64.powi(2) + 0.04_f64.powi(2) + 0.04_f64.powi(2) + 0.02_f64.powi(2))
                .sqrt();
        assert!((angular.norm() - expected_norm).abs() < 1e-12);
    }

    #[test]
    fn contact_wrench_rejects_tangent_projection_and_each_friction_limit() {
        let frame = ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let limits = ContactWrenchFrictionLimits::new(0.5, 0.1, 0.1, 0.5, 0.25).unwrap();
        let base = ContactWrenchImpulse::<3> {
            normal: 2.0,
            tangential: SVector::new(0.0, 0.5, 0.0),
            rolling: vec![0.1, 0.0],
            torsional: vec![0.05],
        };
        assert!(base.validate(&frame, &limits, 1e-12).is_ok());

        let mut non_tangent = base.clone();
        non_tangent.tangential[0] = 1e-3;
        assert_eq!(
            non_tangent.validate(&frame, &limits, 1e-12),
            Err(ContactFrameError::TangentialImpulseNotOrthogonal)
        );

        let mut too_much_tangent = base.clone();
        too_much_tangent.tangential[1] = 1.1;
        assert_eq!(
            too_much_tangent.validate(&frame, &limits, 1e-12),
            Err(ContactFrameError::FrictionLimitExceeded)
        );

        let mut too_much_rolling = base.clone();
        too_much_rolling.rolling[0] = 0.101;
        assert_eq!(
            too_much_rolling.validate(&frame, &limits, 1e-12),
            Err(ContactFrameError::FrictionLimitExceeded)
        );

        let mut too_much_torsion = base.clone();
        too_much_torsion.torsional[0] = 0.051;
        assert_eq!(
            too_much_torsion.validate(&frame, &limits, 1e-12),
            Err(ContactFrameError::FrictionLimitExceeded)
        );

        let mut pulling_contact = base;
        pulling_contact.normal = -0.1;
        assert_eq!(
            pulling_contact.validate(&frame, &limits, 1e-12),
            Err(ContactFrameError::NegativeNormalImpulse)
        );
    }

    #[test]
    fn contact_wrench_friction_model_requires_explicit_physical_length_scales() {
        assert_eq!(
            ContactWrenchFrictionLimits::new(0.5, 0.1, 0.1, 0.0, 0.25).unwrap_err(),
            ContactFrameError::InvalidContactRadius
        );
        assert_eq!(
            ContactWrenchFrictionLimits::new(0.5, -0.1, 0.1, 0.5, 0.25).unwrap_err(),
            ContactFrameError::InvalidFrictionCoefficient
        );
    }

    #[test]
    fn transported_frame_tracks_normal_and_preserves_local_wrench_coordinates() {
        let frame = ContactFrame::<4>::new(SVector::new(1.0, 0.0, 0.0, 0.0), 1e-12).unwrap();
        let target_normal = SVector::new(0.2, 0.3, -0.4, 0.5);
        let transported = frame.transported_to(target_normal, 1e-12).unwrap();
        let target_unit = target_normal.normalize();
        assert!((transported.normal() - target_unit).norm() < 1e-10);

        let mut angular = Bivector::<4>::zero();
        for i in 0..4 {
            for j in (i + 1)..4 {
                angular.set(i, j, (i as f64 + 1.0) * 0.3 - (j as f64 + 0.5) * 0.2);
            }
        }
        let before = frame.decompose_angular(&angular).unwrap();
        // A shortest-rotation transported frame is an orthogonal coordinate
        // transform, so applying that rotation to the physical bivector must
        // preserve its rolling/torsional coordinates.
        let delta_rotation = Rotor::from_plane_angle(
            &Bivector::from_wedge(frame.normal(), &target_unit),
            frame.normal().dot(&target_unit).clamp(-1.0, 1.0).acos(),
        );
        let r = delta_rotation.to_matrix();
        let angular_after =
            Bivector::from_matrix(&(r * angular.to_matrix() * r.transpose()));
        let after = transported.decompose_angular(&angular_after).unwrap();
        for (left, right) in before
            .rolling
            .iter()
            .chain(before.torsional.iter())
            .zip(after.rolling.iter().chain(after.torsional.iter()))
        {
            assert!((left - right).abs() < 1e-10);
        }
        assert!(after.reconstruction_error < 1e-10);
    }

    #[test]
    fn antipodal_contact_frame_transport_uses_deterministic_half_turn() {
        let frame = ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let transported = frame
            .transported_to(SVector::new(-1.0, 0.0, 0.0), 1e-12)
            .unwrap();
        assert!((transported.normal() - SVector::new(-1.0, 0.0, 0.0)).norm() < 1e-12);
        assert_eq!(transported.tangent_basis().len(), 2);
        assert_eq!(transported.rolling_basis().len(), 2);
        assert_eq!(transported.torsional_basis().len(), 1);
    }

    #[test]
    fn contact_wrench_is_invariant_to_consistent_length_unit_conversion() {
        let frame = ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let si_limits =
            ContactWrenchFrictionLimits::new(0.5, 0.1, 0.1, 0.5, 0.25).unwrap();
        let si_wrench = ContactWrenchImpulse::<3> {
            normal: 10.0,
            tangential: SVector::new(0.0, 4.0, 0.0),
            rolling: vec![0.3, 0.1],
            torsional: vec![0.2],
        };
        assert!(si_wrench.validate(&frame, &si_limits, 1e-12).is_ok());

        // m -> cm leaves linear impulse unchanged but multiplies angular
        // impulse and every characteristic length by 100.
        let centimetre_limits =
            ContactWrenchFrictionLimits::new(0.5, 0.1, 0.1, 50.0, 25.0).unwrap();
        let centimetre_wrench = ContactWrenchImpulse::<3> {
            normal: si_wrench.normal,
            tangential: si_wrench.tangential,
            rolling: si_wrench.rolling.iter().map(|value| value * 100.0).collect(),
            torsional: si_wrench
                .torsional
                .iter()
                .map(|value| value * 100.0)
                .collect(),
        };
        assert!(
            centimetre_wrench
                .validate(&frame, &centimetre_limits, 1e-12)
                .is_ok()
        );
    }

    #[test]
    fn contact_wrench_validation_rechecks_model_parameters() {
        let frame = ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let invalid_limits = ContactWrenchFrictionLimits {
            tangential_coefficient: 0.5,
            rolling_coefficient: 0.1,
            torsional_coefficient: 0.1,
            rolling_radius: f64::NAN,
            torsional_radius: 0.25,
        };
        let wrench = ContactWrenchImpulse::<3> {
            normal: 1.0,
            tangential: SVector::new(0.0, 0.0, 0.0),
            rolling: vec![0.0, 0.0],
            torsional: vec![0.0],
        };
        assert_eq!(
            wrench.validate(&frame, &invalid_limits, 1e-12),
            Err(ContactFrameError::InvalidContactRadius)
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
