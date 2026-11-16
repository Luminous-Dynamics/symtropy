// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Frame- and reference-point-aware spatial inertia reference for 3D rigid bodies.
//!
//! A spatial inertia is not just a central 3x3 rotational tensor. It is the
//! rigid body's mass, center-of-mass offset, and central inertia, all tied to
//! an explicit coordinate frame and reference point. The resulting 6x6
//! matrix acts on twists ordered as `[angular_velocity; linear_velocity_at_P]`.
//! This module is a checked reference API and is not wired into `RigidBody`.

use nalgebra::{DMatrix, SMatrix, SVector};
use symtropy_math::Rotor;

const ROTATION_VALIDITY_TOLERANCE: f64 = 1.0e-8;
const TENSOR_RELATIVE_TOLERANCE: f64 = 1.0e-12;

/// Stable identity for the coordinate frame in which a spatial inertia is expressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameId(pub u64);

/// Stable identity for the point about which a spatial inertia is expressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReferencePointId(pub u64);

/// Checked spatial-inertia construction, transformation, or composition failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialInertia3Error {
    InvalidMass,
    NonFinite,
    NonSymmetricCentralInertia,
    CentralInertiaNotPositiveSemidefinite,
    CentralInertiaViolatesTriangleInequality,
    InvalidRotation,
    InvalidTolerance,
    InvalidContactDirection,
    SingularSpatialInertia,
    NegativeContactEffectiveMass,
    FrameMismatch { left: FrameId, right: FrameId },
    FrameIdentityConflict,
    ReferencePointIdentityConflict,
    ReferencePointMismatch { left: ReferencePointId, right: ReferencePointId },
    UnrepresentableSpatialMassMatrix,
    UnrepresentableComposition,
}

/// A 3D rigid body's spatial inertia expressed in one frame and about one point.
///
/// `center_of_mass_from_reference` is the vector from the reference point P
/// to the center of mass G, expressed in `frame`. `central_inertia` is the
/// rotational inertia about G, expressed in the same frame. Twists use the
/// ordering `[omega; v_P]`; spatial momenta use `[angular_momentum_about_P;
/// linear_momentum]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SpatialInertia3 {
    frame: FrameId,
    about: ReferencePointId,
    mass: f64,
    center_of_mass_from_reference: SVector<f64, 3>,
    // Symmetric matrix packed as [Ixx, Iyy, Izz, Ixy, Ixz, Iyz].
    central_inertia_components: [f64; 6],
}

impl SpatialInertia3 {
    /// Construct a spatial inertia from mass, COM offset, and central inertia.
    pub fn new(
        frame: FrameId,
        about: ReferencePointId,
        mass: f64,
        center_of_mass_from_reference: SVector<f64, 3>,
        central_inertia: SMatrix<f64, 3, 3>,
    ) -> Result<Self, SpatialInertia3Error> {
        if !mass.is_finite()
            || !center_of_mass_from_reference.iter().all(|value| value.is_finite())
            || !central_inertia.iter().all(|value| value.is_finite())
        {
            return Err(SpatialInertia3Error::NonFinite);
        }
        if mass <= 0.0 {
            return Err(SpatialInertia3Error::InvalidMass);
        }

        let scale = central_inertia
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let symmetry_tolerance = TENSOR_RELATIVE_TOLERANCE * scale.max(f64::MIN_POSITIVE);
        for row in 0..3 {
            for column in 0..3 {
                if (central_inertia[(row, column)] - central_inertia[(column, row)]).abs()
                    > symmetry_tolerance
                {
                    return Err(SpatialInertia3Error::NonSymmetricCentralInertia);
                }
            }
        }

        let symmetric = SMatrix::<f64, 3, 3>::from_fn(|row, column| {
            0.5 * central_inertia[(row, column)]
                + 0.5 * central_inertia[(column, row)]
        });
        if !symmetric.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        let eigen = nalgebra::linalg::SymmetricEigen::new(DMatrix::from_fn(3, 3, |i, j| {
            symmetric[(i, j)]
        }));
        if !eigen.eigenvalues.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        let mut moments = [eigen.eigenvalues[0], eigen.eigenvalues[1], eigen.eigenvalues[2]];
        moments.sort_by(f64::total_cmp);
        let inertia_scale = moments[2].abs().max(f64::MIN_POSITIVE);
        let tolerance = TENSOR_RELATIVE_TOLERANCE * inertia_scale;
        if moments[0] < -tolerance {
            return Err(SpatialInertia3Error::CentralInertiaNotPositiveSemidefinite);
        }
        if moments[2] > moments[0] + moments[1] + tolerance {
            return Err(SpatialInertia3Error::CentralInertiaViolatesTriangleInequality);
        }

        // Remove only roundoff-sized negative eigenvalues, keeping the stored
        // tensor inside the physically admissible PSD cone.
        let clamped = SVector::<f64, 3>::from_iterator(
            eigen.eigenvalues.iter().map(|value| value.max(0.0)),
        );
        let axes = SMatrix::<f64, 3, 3>::from_fn(|i, j| eigen.eigenvectors[(i, j)]);
        let validated = axes * SMatrix::<f64, 3, 3>::from_diagonal(&clamped) * axes.transpose();
        if !validated.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        Ok(Self {
            frame,
            about,
            mass,
            center_of_mass_from_reference,
            central_inertia_components: tensor_to_components(&validated),
        })
    }

    #[inline]
    pub const fn frame(&self) -> FrameId {
        self.frame
    }

    #[inline]
    pub const fn about(&self) -> ReferencePointId {
        self.about
    }

    #[inline]
    pub const fn mass(&self) -> f64 {
        self.mass
    }

    #[inline]
    pub const fn center_of_mass_from_reference(&self) -> &SVector<f64, 3> {
        &self.center_of_mass_from_reference
    }

    /// Return the six independent central-inertia components in order
    /// `[Ixx, Iyy, Izz, Ixy, Ixz, Iyz]`.
    #[inline]
    pub const fn central_inertia_components(&self) -> [f64; 6] {
        self.central_inertia_components
    }

    /// Reconstruct the symmetric 3x3 central inertia matrix.
    #[inline]
    pub fn central_inertia(&self) -> SMatrix<f64, 3, 3> {
        components_to_tensor(self.central_inertia_components)
    }

    /// Return the 6x6 spatial mass matrix for twist ordering `[omega; v_P]`.
    ///
    /// With `c = p_G - p_P` and `C = [c]_x`:
    ///
    /// `M_P = [[I_G + m C^T C, m C], [-m C, m I]]`.
    ///
    /// This convention gives `v_G = v_P - C*omega` and
    /// `T = 0.5 * V_P^T * M_P * V_P`.
    pub fn spatial_mass_matrix(&self) -> Result<SMatrix<f64, 6, 6>, SpatialInertia3Error> {
        let c = skew(&self.center_of_mass_from_reference);
        let rotational_shift = checked_mass_weighted_outer(
            self.mass,
            &self.center_of_mass_from_reference,
        )?;
        let rotational_about_p = self.central_inertia()
            + SMatrix::<f64, 3, 3>::identity() * trace(&rotational_shift)
            - rotational_shift;

        let mut matrix = SMatrix::<f64, 6, 6>::zeros();
        matrix.fixed_view_mut::<3, 3>(0, 0).copy_from(&rotational_about_p);
        matrix
            .fixed_view_mut::<3, 3>(0, 3)
            .copy_from(&(c * self.mass));
        matrix
            .fixed_view_mut::<3, 3>(3, 0)
            .copy_from(&(-c * self.mass));
        matrix
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&(SMatrix::<f64, 3, 3>::identity() * self.mass));
        if matrix.iter().all(|value| value.is_finite()) {
            Ok(matrix)
        } else {
            Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix)
        }
    }

    /// Map a twist `[omega; v_P]` to spatial momentum about P.
    pub fn spatial_momentum(
        &self,
        twist: &SVector<f64, 6>,
    ) -> Result<SVector<f64, 6>, SpatialInertia3Error> {
        if !twist.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        let momentum = self.spatial_mass_matrix()? * twist;
        if momentum.iter().all(|value| value.is_finite()) {
            Ok(momentum)
        } else {
            Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix)
        }
    }

    /// Solve the spatial mass equation for an applied wrench, using a
    /// scale-relative eigenspace pseudoinverse for physically singular modes.
    fn spatial_inverse_response(
        &self,
        wrench: &SVector<f64, 6>,
        relative_tolerance: f64,
    ) -> Result<SVector<f64, 6>, SpatialInertia3Error> {
        if !relative_tolerance.is_finite()
            || relative_tolerance <= 0.0
            || relative_tolerance >= 1.0
        {
            return Err(SpatialInertia3Error::InvalidTolerance);
        }
        if !wrench.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        let matrix = self.spatial_mass_matrix()?;
        let dynamic = DMatrix::from_fn(6, 6, |row, column| matrix[(row, column)]);
        let eigen = nalgebra::linalg::SymmetricEigen::new(dynamic);
        if !eigen.eigenvalues.iter().all(|value| value.is_finite())
            || !eigen.eigenvectors.iter().all(|value| value.is_finite())
        {
            return Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix);
        }
        let scale = eigen
            .eigenvalues
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if scale == 0.0 {
            return Err(SpatialInertia3Error::SingularSpatialInertia);
        }
        if eigen
            .eigenvalues
            .iter()
            .any(|value| *value < -relative_tolerance * scale)
        {
            return Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix);
        }
        let threshold = relative_tolerance * scale;
        let wrench_scale = wrench
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let null_tolerance = relative_tolerance * wrench_scale.max(f64::MIN_POSITIVE);
        let mut response = SVector::<f64, 6>::zeros();
        for axis in 0..6 {
            let projection = (0..6)
                .map(|row| eigen.eigenvectors[(row, axis)] * wrench[row])
                .sum::<f64>();
            let moment = eigen.eigenvalues[axis];
            if moment > threshold {
                let coefficient = projection / moment;
                if !coefficient.is_finite() {
                    return Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix);
                }
                for row in 0..6 {
                    response[row] += eigen.eigenvectors[(row, axis)] * coefficient;
                }
            } else if projection.abs() > null_tolerance {
                return Err(SpatialInertia3Error::SingularSpatialInertia);
            }
        }
        if response.iter().all(|value| value.is_finite()) {
            Ok(response)
        } else {
            Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix)
        }
    }

    /// Return the normal inverse effective mass for a unit impulse at a
    /// world/contact point offset from this inertia's reference point P.
    ///
    /// `offset_from_reference` and `normal` are expressed in `frame`.
    /// The impulse wrench is `[r × n; n]`, and the post-impulse point
    /// velocity is `v_P + omega × r`. A singular spatial mode is rejected
    /// when the contact wrench excites it.
    pub fn contact_inverse_effective_mass(
        &self,
        offset_from_reference: &SVector<f64, 3>,
        normal: &SVector<f64, 3>,
        relative_tolerance: f64,
    ) -> Result<f64, SpatialInertia3Error> {
        if !offset_from_reference.iter().all(|value| value.is_finite())
            || !normal.iter().all(|value| value.is_finite())
        {
            return Err(SpatialInertia3Error::NonFinite);
        }
        let direction_scale = normal
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if direction_scale == 0.0 {
            return Err(SpatialInertia3Error::InvalidContactDirection);
        }
        let scaled = normal / direction_scale;
        let scaled_norm = scaled.norm();
        if !scaled_norm.is_finite() || scaled_norm == 0.0 {
            return Err(SpatialInertia3Error::InvalidContactDirection);
        }
        let unit_normal = scaled / scaled_norm;
        let angular_impulse = offset_from_reference.cross(&unit_normal);
        if !angular_impulse.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        let mut wrench = SVector::<f64, 6>::zeros();
        wrench.fixed_rows_mut::<3>(0).copy_from(&angular_impulse);
        wrench.fixed_rows_mut::<3>(3).copy_from(&unit_normal);
        let twist_response = self.spatial_inverse_response(&wrench, relative_tolerance)?;
        let angular_response = twist_response.fixed_rows::<3>(0).into_owned();
        let linear_response = twist_response.fixed_rows::<3>(3).into_owned();
        let point_velocity_response =
            linear_response + angular_response.cross(offset_from_reference);
        if !point_velocity_response.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix);
        }
        let inverse_effective_mass = unit_normal.dot(&point_velocity_response);
        if !inverse_effective_mass.is_finite() {
            return Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix);
        }
        let response_scale = point_velocity_response
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if inverse_effective_mass < -relative_tolerance * response_scale {
            return Err(SpatialInertia3Error::NegativeContactEffectiveMass);
        }
        Ok(inverse_effective_mass.max(0.0))
    }

    /// Compute kinetic energy for a twist represented at the stored point P.
    pub fn kinetic_energy(
        &self,
        twist: &SVector<f64, 6>,
    ) -> Result<f64, SpatialInertia3Error> {
        let momentum = self.spatial_momentum(twist)?;
        let energy = 0.5 * twist.dot(&momentum);
        if energy.is_finite() && energy >= 0.0 {
            Ok(energy)
        } else {
            Err(SpatialInertia3Error::UnrepresentableSpatialMassMatrix)
        }
    }

    /// Shift the reference point from P to Q, with `offset_old_to_new = p_Q - p_P`.
    ///
    /// The COM offset changes to `c_Q = c_P - r`, and a rigid twist changes
    /// from `v_P` to `v_Q = v_P + omega x r = v_P - [r]_x omega`.
    /// The central inertia about G and the expressed-in frame remain unchanged.
    pub fn shift_reference_point(
        &self,
        new_about: ReferencePointId,
        offset_old_to_new: &SVector<f64, 3>,
    ) -> Result<Self, SpatialInertia3Error> {
        if !offset_old_to_new.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::NonFinite);
        }
        if new_about == self.about && offset_old_to_new.iter().any(|value| *value != 0.0) {
            return Err(SpatialInertia3Error::ReferencePointIdentityConflict);
        }
        let new_com_offset = self.center_of_mass_from_reference - offset_old_to_new;
        Self::new(
            self.frame,
            new_about,
            self.mass,
            new_com_offset,
            self.central_inertia(),
        )
    }

    /// Re-express the same physical inertia in a new frame.
    /// `old_to_new` rotates coordinates expressed in the old frame into the new frame.
    pub fn reexpress(
        &self,
        new_frame: FrameId,
        old_to_new: &Rotor<3>,
    ) -> Result<Self, SpatialInertia3Error> {
        let rotation = old_to_new.to_matrix();
        if !rotation.iter().all(|value| value.is_finite())
            || !old_to_new.is_proper_rotation(ROTATION_VALIDITY_TOLERANCE)
        {
            return Err(SpatialInertia3Error::InvalidRotation);
        }
        if new_frame == self.frame
            && (rotation - SMatrix::<f64, 3, 3>::identity()).norm()
                > ROTATION_VALIDITY_TOLERANCE
        {
            return Err(SpatialInertia3Error::FrameIdentityConflict);
        }
        Self::new(
            new_frame,
            self.about,
            self.mass,
            old_to_new.rotate_vector(&self.center_of_mass_from_reference),
            rotation * self.central_inertia() * rotation.transpose(),
        )
    }

    /// Combine two bodies expressed in the same frame and about the same point.
    /// The resulting object retains the full central inertia tensor and COM offset.
    pub fn compose_at_common_reference(
        &self,
        other: &Self,
    ) -> Result<Self, SpatialInertia3Error> {
        if self.frame != other.frame {
            return Err(SpatialInertia3Error::FrameMismatch {
                left: self.frame,
                right: other.frame,
            });
        }
        if self.about != other.about {
            return Err(SpatialInertia3Error::ReferencePointMismatch {
                left: self.about,
                right: other.about,
            });
        }
        let total_mass = self.mass + other.mass;
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(SpatialInertia3Error::UnrepresentableComposition);
        }
        let mass_scale = self.mass.max(other.mass);
        let mass_a = self.mass / mass_scale;
        let mass_b = other.mass / mass_scale;
        let mass_sum = mass_a + mass_b;
        let position_scale = self
            .center_of_mass_from_reference
            .iter()
            .chain(other.center_of_mass_from_reference.iter())
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let combined_com = if position_scale == 0.0 {
            SVector::<f64, 3>::zeros()
        } else {
            let normalized = (self.center_of_mass_from_reference / position_scale) * mass_a
                + (other.center_of_mass_from_reference / position_scale) * mass_b;
            (normalized / mass_sum) * position_scale
        };
        if !combined_com.iter().all(|value| value.is_finite()) {
            return Err(SpatialInertia3Error::UnrepresentableComposition);
        }
        let offset_a = self.center_of_mass_from_reference - combined_com;
        let offset_b = other.center_of_mass_from_reference - combined_com;
        let shift_a = checked_mass_weighted_outer(self.mass, &offset_a)?;
        let shift_b = checked_mass_weighted_outer(other.mass, &offset_b)?;
        let central = self.central_inertia() + other.central_inertia()
            + SMatrix::<f64, 3, 3>::identity() * trace(&shift_a)
            - shift_a
            + SMatrix::<f64, 3, 3>::identity() * trace(&shift_b)
            - shift_b;
        Self::new(self.frame, self.about, total_mass, combined_com, central)
            .map_err(|_| SpatialInertia3Error::UnrepresentableComposition)
    }
}

fn tensor_to_components(tensor: &SMatrix<f64, 3, 3>) -> [f64; 6] {
    [
        tensor[(0, 0)],
        tensor[(1, 1)],
        tensor[(2, 2)],
        0.5 * tensor[(0, 1)] + 0.5 * tensor[(1, 0)],
        0.5 * tensor[(0, 2)] + 0.5 * tensor[(2, 0)],
        0.5 * tensor[(1, 2)] + 0.5 * tensor[(2, 1)],
    ]
}

fn components_to_tensor(components: [f64; 6]) -> SMatrix<f64, 3, 3> {
    let [ixx, iyy, izz, ixy, ixz, iyz] = components;
    SMatrix::new(
        ixx, ixy, ixz,
        ixy, iyy, iyz,
        ixz, iyz, izz,
    )
}

fn skew(vector: &SVector<f64, 3>) -> SMatrix<f64, 3, 3> {
    SMatrix::new(
        0.0, -vector[2], vector[1],
        vector[2], 0.0, -vector[0],
        -vector[1], vector[0], 0.0,
    )
}

fn trace(matrix: &SMatrix<f64, 3, 3>) -> f64 {
    matrix[(0, 0)] + matrix[(1, 1)] + matrix[(2, 2)]
}

fn checked_mass_weighted_outer(
    mass: f64,
    offset: &SVector<f64, 3>,
) -> Result<SMatrix<f64, 3, 3>, SpatialInertia3Error> {
    if mass == 0.0 {
        return Ok(SMatrix::<f64, 3, 3>::zeros());
    }
    let weighted = offset * mass.sqrt();
    if !weighted.iter().all(|value| value.is_finite()) {
        return Err(SpatialInertia3Error::NonFinite);
    }
    for axis in 0..3 {
        if offset[axis] != 0.0 && weighted[axis] == 0.0 {
            return Err(SpatialInertia3Error::UnrepresentableComposition);
        }
    }
    let outer = weighted * weighted.transpose();
    for row in 0..3 {
        for column in 0..3 {
            if weighted[row] != 0.0 && weighted[column] != 0.0 && outer[(row, column)] == 0.0 {
                return Err(SpatialInertia3Error::UnrepresentableComposition);
            }
        }
    }
    if outer.iter().all(|value| value.is_finite()) {
        Ok(outer)
    } else {
        Err(SpatialInertia3Error::UnrepresentableComposition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Bivector;

    fn sample() -> SpatialInertia3 {
        SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(10),
            2.0,
            SVector::new(1.0, 0.0, 0.0),
            SMatrix::from_diagonal(&SVector::new(0.5, 0.6, 0.7)),
        )
        .unwrap()
    }

    fn sample_twist() -> SVector<f64, 6> {
        SVector::from([0.2, -0.3, 0.8, 0.4, 0.1, -0.2])
    }

    #[test]
    fn central_inertia_uses_six_canonical_symmetric_components() {
        let inertia = sample();
        assert_eq!(
            inertia.central_inertia_components(),
            [0.5, 0.6, 0.7, 0.0, 0.0, 0.0]
        );
        let matrix = inertia.central_inertia();
        assert_eq!(matrix[(0, 1)], matrix[(1, 0)]);
        assert_eq!(matrix[(0, 2)], matrix[(2, 0)]);
        assert_eq!(matrix[(1, 2)], matrix[(2, 1)]);
    }

    #[test]
    fn spatial_mass_matrix_matches_com_kinetic_energy_decomposition() {
        let inertia = sample();
        let twist = sample_twist();
        let matrix = inertia.spatial_mass_matrix().unwrap();
        assert!((matrix - matrix.transpose()).norm() < 1.0e-14);
        let energy = inertia.kinetic_energy(&twist).unwrap();
        let omega = twist.fixed_rows::<3>(0).into_owned();
        let velocity_at_p = twist.fixed_rows::<3>(3).into_owned();
        let c = inertia.center_of_mass_from_reference;
        let velocity_at_com = velocity_at_p + omega.cross(&c);
        let expected = 0.5 * omega.dot(&(inertia.central_inertia() * omega))
            + 0.5 * inertia.mass * velocity_at_com.norm_squared();
        assert!((energy - expected).abs() < 1.0e-12);
    }

    #[test]
    fn rotated_physical_second_moments_produce_symmetric_psd_spatial_matrices() {
        for seed in 0..48_u64 {
            let sx = 0.15 + (seed.wrapping_mul(17) % 31) as f64 * 0.025;
            let sy = 0.2 + (seed.wrapping_mul(23) % 29) as f64 * 0.03;
            let sz = 0.25 + (seed.wrapping_mul(11) % 37) as f64 * 0.02;
            let mut generator = Bivector::<3>::zero();
            generator.set(0, 1, seed as f64 * 0.007);
            generator.set(0, 2, seed as f64 * -0.004);
            generator.set(1, 2, seed as f64 * 0.005);
            let rotation = Rotor::from_bivector(&generator);
            let diagonal_second_moment = SMatrix::<f64, 3, 3>::from_diagonal(
                &SVector::new(sx, sy, sz),
            );
            let second_moment =
                rotation.to_matrix() * diagonal_second_moment * rotation.to_matrix().transpose();
            let central_inertia =
                SMatrix::<f64, 3, 3>::identity() * trace(&second_moment) - second_moment;
            let center = SVector::new(
                (seed as f64 * 0.13).sin(),
                (seed as f64 * 0.19).cos(),
                (seed as f64 * 0.07).sin() * 0.5,
            );
            let inertia = SpatialInertia3::new(
                FrameId(7),
                ReferencePointId(3),
                0.5 + (seed % 9) as f64 * 0.17,
                center,
                central_inertia,
            )
            .unwrap();
            let matrix = inertia.spatial_mass_matrix().unwrap();
            let symmetry_error = (matrix - matrix.transpose()).norm();
            assert!(symmetry_error < 1.0e-12, "seed={seed}, symmetry={symmetry_error:e}");
            let eigen = nalgebra::linalg::SymmetricEigen::new(DMatrix::from_fn(6, 6, |i, j| {
                matrix[(i, j)]
            }));
            let minimum = eigen
                .eigenvalues
                .iter()
                .fold(f64::INFINITY, |smallest, value| smallest.min(*value));
            let scale = matrix
                .iter()
                .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
            assert!(
                minimum >= -1.0e-11 * scale.max(f64::MIN_POSITIVE),
                "seed={seed}, min spatial-mass eigenvalue={minimum:e}, scale={scale:e}"
            );
            let twist = SVector::from([
                0.1 + seed as f64 * 0.001,
                -0.3,
                0.2,
                0.4,
                (seed as f64 * 0.11).sin(),
                -0.2,
            ]);
            let energy = inertia.kinetic_energy(&twist).unwrap();
            assert!(energy > 0.0 && energy.is_finite(), "seed={seed}, energy={energy:e}");
        }
    }

    #[test]
    fn shifting_reference_point_preserves_twist_energy_and_momentum() {
        let inertia = sample();
        let twist_p = sample_twist();
        let offset_p_to_q = SVector::new(-0.4, 0.7, 0.2);
        let shifted = inertia
            .shift_reference_point(ReferencePointId(20), &offset_p_to_q)
            .unwrap();
        let omega = twist_p.fixed_rows::<3>(0).into_owned();
        let velocity_p = twist_p.fixed_rows::<3>(3).into_owned();
        let velocity_q = velocity_p + omega.cross(&offset_p_to_q);
        let mut twist_q = SVector::<f64, 6>::zeros();
        twist_q.fixed_rows_mut::<3>(0).copy_from(&omega);
        twist_q.fixed_rows_mut::<3>(3).copy_from(&velocity_q);
        assert!((inertia.kinetic_energy(&twist_p).unwrap()
            - shifted.kinetic_energy(&twist_q).unwrap())
            .abs()
            < 1.0e-12);
        let momentum_p = inertia.spatial_momentum(&twist_p).unwrap();
        let momentum_q = shifted.spatial_momentum(&twist_q).unwrap();
        let expected_linear = momentum_p.fixed_rows::<3>(3).into_owned();
        let angular_p = momentum_p.fixed_rows::<3>(0).into_owned();
        let angular_q = momentum_q.fixed_rows::<3>(0).into_owned();
        assert!((angular_q - (angular_p - offset_p_to_q.cross(&expected_linear))).norm() < 1.0e-12);
        assert!((momentum_q.fixed_rows::<3>(3).into_owned() - expected_linear).norm() < 1.0e-12);
    }

    #[test]
    fn reference_point_shift_and_inverse_shift_restore_the_inertia() {
        let inertia = sample();
        let offset = SVector::new(0.25, -0.6, 0.35);
        let shifted = inertia
            .shift_reference_point(ReferencePointId(20), &offset)
            .unwrap();
        let restored = shifted
            .shift_reference_point(ReferencePointId(10), &(-offset))
            .unwrap();
        assert_eq!(restored.frame(), inertia.frame());
        assert_eq!(restored.about(), inertia.about());
        assert!((restored.center_of_mass_from_reference()
            - inertia.center_of_mass_from_reference())
            .norm()
            < 1.0e-14);
        assert!((restored.central_inertia() - inertia.central_inertia()).norm() < 1.0e-14);
    }

    #[test]
    fn uniform_mass_scaling_scales_spatial_matrix_momentum_and_energy() {
        let inertia = sample();
        let twist = sample_twist();
        let matrix = inertia.spatial_mass_matrix().unwrap();
        let momentum = inertia.spatial_momentum(&twist).unwrap();
        let energy = inertia.kinetic_energy(&twist).unwrap();

        for scale in [1.0e-12, 1.0e12] {
            let scaled = SpatialInertia3::new(
                inertia.frame(),
                inertia.about(),
                inertia.mass() * scale,
                *inertia.center_of_mass_from_reference(),
                inertia.central_inertia() * scale,
            )
            .unwrap();
            let scaled_matrix = scaled.spatial_mass_matrix().unwrap();
            let scaled_momentum = scaled.spatial_momentum(&twist).unwrap();
            let scaled_energy = scaled.kinetic_energy(&twist).unwrap();
            assert!(
                (scaled_matrix - matrix * scale).norm()
                    / (matrix.norm() * scale).max(f64::MIN_POSITIVE)
                    < 2.0e-12,
                "spatial matrix scale covariance failed at {scale:e}"
            );
            assert!(
                (scaled_momentum - momentum * scale).norm()
                    / (momentum.norm() * scale).max(f64::MIN_POSITIVE)
                    < 2.0e-12
            );
            assert!((scaled_energy / (energy * scale) - 1.0).abs() < 2.0e-12);
        }
    }

    #[test]
    fn frame_reexpression_preserves_energy_and_rotates_spatial_momentum() {
        let inertia = sample();
        let twist = sample_twist();
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.43);
        generator.set(0, 2, -0.21);
        let rotation = Rotor::from_bivector(&generator);
        let reexpressed = inertia.reexpress(FrameId(2), &rotation).unwrap();
        let angular = twist.fixed_rows::<3>(0).into_owned();
        let linear = twist.fixed_rows::<3>(3).into_owned();
        let mut rotated_twist = SVector::<f64, 6>::zeros();
        rotated_twist.fixed_rows_mut::<3>(0).copy_from(&rotation.rotate_vector(&angular));
        rotated_twist.fixed_rows_mut::<3>(3).copy_from(&rotation.rotate_vector(&linear));
        assert!((inertia.kinetic_energy(&twist).unwrap()
            - reexpressed.kinetic_energy(&rotated_twist).unwrap())
            .abs()
            < 1.0e-12);
    }

    #[test]
    fn compound_spatial_inertia_matches_sum_of_child_energies() {
        let first = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(10),
            2.0,
            SVector::new(-1.0, 0.0, 0.0),
            SMatrix::from_diagonal(&SVector::new(0.8, 0.8, 0.8)),
        )
        .unwrap();
        let second = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(10),
            3.0,
            SVector::new(2.0, 0.0, 0.0),
            SMatrix::from_diagonal(&SVector::new(0.6, 0.7, 0.8)),
        )
        .unwrap();
        let combined = first.compose_at_common_reference(&second).unwrap();
        let twist = sample_twist();
        let total_energy = first.kinetic_energy(&twist).unwrap()
            + second.kinetic_energy(&twist).unwrap();
        assert!((combined.kinetic_energy(&twist).unwrap() - total_energy).abs() < 1.0e-11);
        assert!((combined.mass() - 5.0).abs() < 1.0e-14);
        assert!((combined.center_of_mass_from_reference()[0] - 0.8).abs() < 1.0e-14);
    }

    #[test]
    fn point_identity_cannot_be_reused_for_a_nonzero_reference_shift() {
        let inertia = sample();
        let result = inertia.shift_reference_point(
            inertia.about(),
            &SVector::new(0.1, 0.0, 0.0),
        );
        assert_eq!(result.unwrap_err(), SpatialInertia3Error::ReferencePointIdentityConflict);
    }

    #[test]
    fn frame_identity_cannot_be_reused_for_a_nontrivial_reexpression() {
        let inertia = sample();
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.3);
        let rotation = Rotor::from_bivector(&generator);
        let result = inertia.reexpress(inertia.frame(), &rotation);
        assert_eq!(result.unwrap_err(), SpatialInertia3Error::FrameIdentityConflict);
    }

    #[test]
    fn contact_effective_mass_matches_sphere_and_anisotropic_analytic_values() {
        let sphere = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(1),
            3.0,
            SVector::zeros(),
            SMatrix::from_diagonal(&SVector::new(1.2, 1.2, 1.2)),
        )
        .unwrap();
        let offset = SVector::new(1.0, 0.0, 0.0);
        let normal = SVector::new(0.0, 1.0, 0.0);
        let sphere_value = sphere
            .contact_inverse_effective_mass(&offset, &normal, 1.0e-12)
            .unwrap();
        assert!((sphere_value - (1.0 / 3.0 + 1.0 / 1.2)).abs() < 1.0e-11);

        let anisotropic = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(1),
            2.0,
            SVector::zeros(),
            SMatrix::from_diagonal(&SVector::new(0.5, 0.6, 0.7)),
        )
        .unwrap();
        let value = anisotropic
            .contact_inverse_effective_mass(&offset, &normal, 1.0e-12)
            .unwrap();
        assert!((value - (0.5 + 1.0 / 0.7)).abs() < 1.0e-11);

        let axial = anisotropic
            .contact_inverse_effective_mass(&offset, &SVector::new(2.0, 0.0, 0.0), 1.0e-12)
            .unwrap();
        assert!((axial - 0.5).abs() < 1.0e-11);
    }

    #[test]
    fn contact_effective_mass_is_covariant_under_frame_reexpression() {
        let inertia = sample();
        let offset = SVector::new(0.7, -0.2, 0.4);
        let normal = SVector::new(-0.3, 0.8, 0.5).normalize();
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.2);
        generator.set(0, 2, -0.35);
        generator.set(1, 2, 0.27);
        let rotation = Rotor::from_bivector(&generator);
        let original = inertia
            .contact_inverse_effective_mass(&offset, &normal, 1.0e-12)
            .unwrap();
        let reexpressed = inertia.reexpress(FrameId(2), &rotation).unwrap();
        let transformed = reexpressed
            .contact_inverse_effective_mass(
                &rotation.rotate_vector(&offset),
                &rotation.rotate_vector(&normal),
                1.0e-12,
            )
            .unwrap();
        assert!((original - transformed).abs() < 2.0e-11);
    }

    #[test]
    fn contact_effective_mass_is_invariant_under_reference_point_shift() {
        let inertia = sample();
        let point_p_offset = SVector::new(0.7, -0.25, 0.4);
        let normal = SVector::new(-0.2, 0.9, 0.35);
        let p_to_q = SVector::new(-0.3, 0.5, 0.2);
        let value_p = inertia
            .contact_inverse_effective_mass(&point_p_offset, &normal, 1.0e-12)
            .unwrap();
        let shifted = inertia
            .shift_reference_point(ReferencePointId(11), &p_to_q)
            .unwrap();
        let point_q_offset = point_p_offset - p_to_q;
        let value_q = shifted
            .contact_inverse_effective_mass(&point_q_offset, &normal, 1.0e-12)
            .unwrap();
        assert!((value_p - value_q).abs() < 2.0e-11);
    }

    #[test]
    fn contact_effective_mass_normalizes_extreme_finite_normal_scale() {
        let inertia = sample();
        let offset = SVector::new(0.7, -0.25, 0.4);
        let normal = SVector::new(-0.2, 0.9, 0.35);
        let expected = inertia
            .contact_inverse_effective_mass(&offset, &normal, 1.0e-12)
            .unwrap();
        let observed = inertia
            .contact_inverse_effective_mass(&offset, &(normal * 1.0e300), 1.0e-12)
            .unwrap();
        assert!((expected - observed).abs() < 2.0e-11);
    }

    #[test]
    fn contact_effective_mass_rejects_an_excited_singular_rotation_mode() {
        let rod = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(1),
            1.0,
            SVector::zeros(),
            SMatrix::from_diagonal(&SVector::new(1.0, 1.0, 0.0)),
        )
        .unwrap();
        let error = rod
            .contact_inverse_effective_mass(
                &SVector::new(1.0, 0.0, 0.0),
                &SVector::new(0.0, 1.0, 0.0),
                1.0e-12,
            )
            .unwrap_err();
        assert_eq!(error, SpatialInertia3Error::SingularSpatialInertia);
    }

    #[test]
    fn composition_rejects_frame_and_reference_point_mismatches() {
        let base = sample();
        let other_frame = SpatialInertia3::new(
            FrameId(2),
            ReferencePointId(10),
            1.0,
            SVector::zeros(),
            SMatrix::identity(),
        )
        .unwrap();
        assert!(matches!(
            base.compose_at_common_reference(&other_frame),
            Err(SpatialInertia3Error::FrameMismatch { .. })
        ));
        let other_point = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(99),
            1.0,
            SVector::zeros(),
            SMatrix::identity(),
        )
        .unwrap();
        assert!(matches!(
            base.compose_at_common_reference(&other_point),
            Err(SpatialInertia3Error::ReferencePointMismatch { .. })
        ));
    }

    #[test]
    fn invalid_or_unphysical_central_inertia_is_rejected() {
        assert_eq!(
            SpatialInertia3::new(
                FrameId(1),
                ReferencePointId(1),
                1.0,
                SVector::zeros(),
                SMatrix::from_diagonal(&SVector::new(1.0, 1.0, -0.1)),
            )
            .unwrap_err(),
            SpatialInertia3Error::CentralInertiaNotPositiveSemidefinite
        );
        assert_eq!(
            SpatialInertia3::new(
                FrameId(1),
                ReferencePointId(1),
                1.0,
                SVector::zeros(),
                SMatrix::from_diagonal(&SVector::new(0.1, 0.1, 1.0)),
            )
            .unwrap_err(),
            SpatialInertia3Error::CentralInertiaViolatesTriangleInequality
        );
    }

    #[test]
    fn improper_frame_rotation_is_rejected() {
        let inertia = sample();
        let mut reflection = SMatrix::<f64, 3, 3>::identity();
        reflection[(0, 0)] = -1.0;
        let rotor = Rotor::from_matrix(reflection);
        assert_eq!(
            inertia.reexpress(FrameId(2), &rotor).unwrap_err(),
            SpatialInertia3Error::InvalidRotation
        );
    }

    #[test]
    fn large_offset_tiny_mass_spatial_matrix_remains_finite() {
        let inertia = SpatialInertia3::new(
            FrameId(1),
            ReferencePointId(1),
            1.0e-310,
            SVector::new(1.0e155, 0.0, 0.0),
            SMatrix::from_diagonal(&SVector::new(0.5e-310, 0.5e-310, 0.5e-310)),
        )
        .unwrap();
        let matrix = inertia.spatial_mass_matrix().unwrap();
        assert!(matrix.iter().all(|value| value.is_finite()));
        assert!((matrix[(1, 1)] - 1.0).abs() < 1.0e-12);
    }
}
