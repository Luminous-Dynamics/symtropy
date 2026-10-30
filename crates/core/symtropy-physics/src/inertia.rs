//! Rotational inertia on the N-dimensional bivector rotation space.
//!
//! The centered second-moment matrix is S = integral r r^T dm. Symtropy stores
//! the rotor generator B, whose increment is exp(-B); its physical point
//! velocity is therefore -B*r. In the library's stored-bivector convention,
//! angular momentum is I(B) = B*S + S*B. This is the dimension-generic form of
//! the familiar 3D spatial inertia tensor and naturally couples rotation planes
//! for asymmetric mass distributions.
//!
//! This module provides forward and inverse inertia application, coordinate-
//! frame rotation, and energy calculation. It is not yet wired into RigidBody
//! integration/contact; gyroscopic evolution and CAD-derived mass-property
//! assembly remain separate validated implementation steps.

use nalgebra::{SMatrix, SVector};
use symtropy_math::{Bivector, Rotor};

/// Construction errors for a rotational inertia operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationalInertiaError {
    /// Bivectors require a spatial dimension of at least two.
    InvalidDimension,
    /// The supplied second-moment matrix contains NaN or infinity.
    NonFinite,
    /// The supplied second-moment matrix is not symmetric within tolerance.
    NotSymmetric,
    /// The supplied second-moment matrix has a materially negative eigenvalue.
    NotPositiveSemidefinite,
    /// Uniform-ball mass or radius is negative.
    InvalidMassProperties,
    /// A requested inverse is undefined because at least one rotation plane has zero inertia.
    SingularInertia,
    /// An inverse-operation tolerance is non-finite or non-positive.
    InvalidTolerance,
    /// A contact/impulse direction is zero or non-finite.
    InvalidDirection,
    /// Component offsets do not place the combined center of mass at the origin.
    OffsetsNotCentered,
}

/// Rotational inertia acting on Lambda^2(R^D), the bivector space of rotations.
///
/// The second-moment matrix must be centered at the body's center of mass and
/// use physical mass units. For a uniform solid D-ball of mass m and radius r,
/// S = (m r^2 / (D+2)) I, giving I_plane = 2 m r^2 / (D+2) in every rotation
/// plane.
#[derive(Clone, Debug)]
pub struct BivectorInertia<const D: usize> {
    second_moment: SMatrix<f64, D, D>,
    principal_second_moments: SVector<f64, D>,
    principal_axes: SMatrix<f64, D, D>,
}

impl<const D: usize> BivectorInertia<D> {
    /// Construct from the centered second-moment matrix S = integral r r^T dm.
    ///
    /// Small floating-point asymmetry is removed by symmetrizing the input.
    /// The symmetric matrix is checked for positive semidefiniteness using a
    /// scale-relative eigenvalue tolerance.
    pub fn from_second_moment(
        second_moment: SMatrix<f64, D, D>,
    ) -> Result<Self, RotationalInertiaError> {
        if D < 2 {
            return Err(RotationalInertiaError::InvalidDimension);
        }
        if !second_moment.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }

        let scale = second_moment
            .iter()
            .fold(0.0_f64, |m, value| m.max(value.abs()));
        let tolerance = 1e-12 * scale.max(f64::MIN_POSITIVE);
        for i in 0..D {
            for j in 0..D {
                if (second_moment[(i, j)] - second_moment[(j, i)]).abs() > tolerance {
                    return Err(RotationalInertiaError::NotSymmetric);
                }
            }
        }

        let symmetric = (second_moment + second_moment.transpose()) * 0.5;
        let eigensystem = nalgebra::linalg::SymmetricEigen::new(symmetric.clone());
        let min_eigenvalue = eigensystem
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if min_eigenvalue < -tolerance {
            return Err(RotationalInertiaError::NotPositiveSemidefinite);
        }

        Ok(Self {
            second_moment: symmetric,
            principal_second_moments: eigensystem.eigenvalues,
            principal_axes: eigensystem.eigenvectors,
        })
    }

    /// Combine two inertia operators whose component centers are expressed
    /// relative to the same combined center of mass.
    ///
    /// The second-moment parallel-axis rule is:
    /// S = S_a + m_a*d_a*d_a^T + S_b + m_b*d_b*d_b^T.
    /// The offsets must satisfy m_a*d_a + m_b*d_b = 0 within a
    /// scale-relative tolerance. This prevents accidentally combining
    /// centered operators about an arbitrary point.
    pub fn combine_at_common_center(
        mass_a: f64,
        inertia_a: &Self,
        offset_a: &SVector<f64, D>,
        mass_b: f64,
        inertia_b: &Self,
        offset_b: &SVector<f64, D>,
    ) -> Result<Self, RotationalInertiaError> {
        if !mass_a.is_finite()
            || !mass_b.is_finite()
            || !offset_a.iter().all(|value| value.is_finite())
            || !offset_b.iter().all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        if mass_a < 0.0 || mass_b < 0.0 || mass_a + mass_b <= 0.0 {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }

        let total_mass = mass_a + mass_b;
        let center_residual = offset_a * mass_a + offset_b * mass_b;
        let offset_scale = offset_a.norm().max(offset_b.norm()).max(f64::MIN_POSITIVE);
        if center_residual.norm() > 1e-10 * total_mass * offset_scale {
            return Err(RotationalInertiaError::OffsetsNotCentered);
        }

        let second_moment = inertia_a.second_moment
            + (offset_a * offset_a.transpose()) * mass_a
            + inertia_b.second_moment
            + (offset_b * offset_b.transpose()) * mass_b;
        Self::from_second_moment(second_moment)
    }

    /// Construct from a discrete set of point masses.
    ///
    /// Positions may be in any translated world frame; this constructor first
    /// computes the center of mass and then forms the centered second moment.
    /// It is suitable for CAD/mesh mass-lumping and point-cloud reference tests.
    pub fn from_point_masses<I>(point_masses: I) -> Result<Self, RotationalInertiaError>
    where
        I: IntoIterator<Item = (SVector<f64, D>, f64)>,
    {
        if D < 2 {
            return Err(RotationalInertiaError::InvalidDimension);
        }

        let samples: Vec<_> = point_masses.into_iter().collect();
        if samples.is_empty() {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }

        let mut total_mass = 0.0;
        let mut weighted_position = SVector::<f64, D>::zeros();
        for (position, mass) in &samples {
            if !mass.is_finite() || !position.iter().all(|value| value.is_finite()) {
                return Err(RotationalInertiaError::NonFinite);
            }
            if *mass < 0.0 {
                return Err(RotationalInertiaError::InvalidMassProperties);
            }
            total_mass += *mass;
            weighted_position += *position * *mass;
        }

        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }

        let center_of_mass = weighted_position / total_mass;
        let mut second_moment = SMatrix::<f64, D, D>::zeros();
        for (position, mass) in samples {
            let offset = position - center_of_mass;
            second_moment += (offset * offset.transpose()) * mass;
        }

        Self::from_second_moment(second_moment)
    }

    /// Construct the centered inertia operator for a uniform solid D-ball.
    pub fn uniform_solid_ball(mass: f64, radius: f64) -> Result<Self, RotationalInertiaError> {
        if !radius.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        Self::uniform_solid_ellipsoid(mass, SVector::<f64, D>::from_element(radius))
    }

    /// Construct from a uniform solid D-ellipsoid with the supplied semi-axes.
    ///
    /// For a uniform ellipsoid, the centered second moment along axis i is
    /// mass * a_i^2 / (D + 2). This gives a direct analytical mass-property
    /// path for ellipsoidal CAD primitives without assuming isotropy.
    pub fn uniform_solid_ellipsoid(
        mass: f64,
        semi_axes: SVector<f64, D>,
    ) -> Result<Self, RotationalInertiaError> {
        if !mass.is_finite() || !semi_axes.iter().all(|axis| axis.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        if mass < 0.0 || semi_axes.iter().any(|axis| *axis < 0.0) {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }

        let second_moments = semi_axes.map(|axis| mass * axis * axis / (D as f64 + 2.0));
        Self::from_second_moment(SMatrix::<f64, D, D>::from_diagonal(&second_moments))
    }

    /// Rotate the mass distribution into another coordinate frame.
    ///
    /// If the supplied proper rotation R maps body-frame vectors to world-frame
    /// vectors, the transformed second moment is S_world = R*S_body*R^T.
    /// This lets callers keep immutable body-space mass properties and derive
    /// the corresponding world-space operator for contact and diagnostics.
    pub fn rotated_by(&self, rotation: &Rotor<D>) -> Self {
        let r = rotation.to_matrix();
        Self {
            second_moment: r * self.second_moment * r.transpose(),
            principal_second_moments: self.principal_second_moments,
            principal_axes: r * self.principal_axes,
        }
    }

    /// Return the centered second-moment matrix used by this operator.
    pub fn second_moment(&self) -> &SMatrix<f64, D, D> {
        &self.second_moment
    }

    /// Map Symtropy's stored angular state B to angular momentum:
    /// L = B*S + S*B. Rotor::from_bivector(B) integrates exp(-B), so the
    /// associated physical point velocity is -B*r.
    ///
    /// The output uses the same lexicographic (i,j), i<j plane ordering as
    /// Bivector<D>. This map is symmetric and positive semidefinite when S
    /// is a physically valid centered mass second-moment matrix. Pass the
    /// stored RigidBody.angular_velocity directly; do not negate it here.
    pub fn angular_momentum(&self, angular_velocity: &Bivector<D>) -> Bivector<D> {
        let omega = angular_velocity.to_matrix();
        let momentum = omega * self.second_moment + self.second_moment * omega;
        Bivector::from_matrix(&momentum)
    }

    /// Invert angular momentum to recover Symtropy's stored angular state B.
    ///
    /// The symmetric eigendecomposition S = Q*Lambda*Q^T diagonalizes the
    /// inertia operator on bivectors: in principal coordinates each plane
    /// (i,j) has eigenvalue lambda_i + lambda_j. This avoids constructing a
    /// dense D(D-1)/2 square matrix and preserves coupling in the world frame.
    ///
    /// The returned value is the stored B consumed by Rotor::from_bivector,
    /// which advances orientation with exp(-B). Physical point velocity is
    /// obtained separately with -B*r.
    ///
    /// The relative tolerance is applied to the largest eigenvalue of S. If any
    /// rotation plane is singular or too ill-conditioned at this threshold,
    /// the operation fails rather than amplifying noise.
    pub fn angular_velocity_from_momentum(
        &self,
        angular_momentum: &Bivector<D>,
        relative_tolerance: f64,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(RotationalInertiaError::InvalidTolerance);
        }
        if !angular_momentum.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }

        let largest_eigenvalue = self
            .principal_second_moments
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(*value));
        let threshold = relative_tolerance * largest_eigenvalue.max(f64::MIN_POSITIVE);

        let q = &self.principal_axes;
        let q_t = q.transpose();
        let momentum_local = q_t * angular_momentum.to_matrix() * q;
        let mut omega_local = nalgebra::SMatrix::<f64, D, D>::zeros();

        for i in 0..D {
            for j in (i + 1)..D {
                let plane_inertia =
                    self.principal_second_moments[i] + self.principal_second_moments[j];
                if !plane_inertia.is_finite() || plane_inertia <= threshold {
                    return Err(RotationalInertiaError::SingularInertia);
                }

                let value = momentum_local[(i, j)] / plane_inertia;
                if !value.is_finite() {
                    return Err(RotationalInertiaError::NonFinite);
                }
                omega_local[(i, j)] = value;
                omega_local[(j, i)] = -value;
            }
        }

        let omega_world = q * omega_local * q_t;
        let result = Bivector::from_matrix(&omega_world);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(RotationalInertiaError::NonFinite)
        }
    }

    /// Rotational contribution to the inverse effective mass at a contact point.
    ///
    /// For unit impulse direction n and center-of-mass offset r, the
    /// impulse-first torque accumulator is n wedge r. The induced physical
    /// point-velocity change is I^(-1)(n wedge r) applied to r, because the
    /// stored rotor state changes by the negative of that angular response.
    /// The returned scalar is n dot delta_v and is non-negative for a
    /// nonsingular, physically valid mass distribution.
    pub fn rotational_inverse_effective_mass(
        &self,
        offset: &SVector<f64, D>,
        impulse_direction: &SVector<f64, D>,
        relative_tolerance: f64,
    ) -> Result<f64, RotationalInertiaError> {
        if !offset.iter().all(|value| value.is_finite())
            || !impulse_direction.iter().all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        let direction_norm = impulse_direction.norm();
        if !direction_norm.is_finite() || direction_norm <= 1e-15 {
            return Err(RotationalInertiaError::InvalidDirection);
        }
        let direction = impulse_direction / direction_norm;
        let torque_accumulator = Bivector::from_wedge(&direction, offset);
        let angular_response =
            self.angular_velocity_from_momentum(&torque_accumulator, relative_tolerance)?;
        let point_velocity_change = angular_response.apply_to_vector(offset);
        let effective_mass = direction.dot(&point_velocity_change);
        if !effective_mass.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        if effective_mass < -1e-12 * point_velocity_change.norm().max(1.0) {
            return Err(RotationalInertiaError::NotPositiveSemidefinite);
        }
        Ok(effective_mass.max(0.0))
    }

    /// Compute the derivative of Symtropy's stored angular state B.
    ///
    /// In this convention:
    ///   L = I(B) = B*S + S*B
    ///   dot(L) = [B, L] - torque_accumulator
    ///   dot(B) = I^(-1)(dot(L))
    ///
    /// `torque_accumulator` uses the impulse-first wedge convention and is
    /// the negative of physical torque in the angular-momentum bivector basis.
    /// The supplied inertia, B, and torque must all be expressed in the same
    /// coordinate frame. For world-frame state, call `rotated_by` with the
    /// body's current orientation before evaluating this method.
    pub fn angular_acceleration(
        &self,
        stored_angular_state: &Bivector<D>,
        torque_accumulator: &Bivector<D>,
        relative_tolerance: f64,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        if !stored_angular_state.is_finite() || !torque_accumulator.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }

        let b = stored_angular_state.to_matrix();
        let momentum = self.angular_momentum(stored_angular_state).to_matrix();
        let coadjoint_rate = b * momentum - momentum * b;
        let momentum_rate = coadjoint_rate - torque_accumulator.to_matrix();
        let momentum_rate_bivector = Bivector::from_matrix(&momentum_rate);

        self.angular_velocity_from_momentum(&momentum_rate_bivector, relative_tolerance)
    }

    /// Rotational kinetic energy, 1/2 <B, I(B)> in the stored-bivector convention.
    pub fn kinetic_energy(&self, angular_velocity: &Bivector<D>) -> f64 {
        let momentum = self.angular_momentum(angular_velocity);
        let mut pairing = 0.0;
        for i in 0..D {
            for j in (i + 1)..D {
                pairing += angular_velocity.get(i, j) * momentum.get(i, j);
            }
        }
        0.5 * pairing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_uniform_ball_plane_moment<const D: usize>(expected: f64) {
        let inertia = BivectorInertia::<D>::uniform_solid_ball(3.0, 2.0).unwrap();
        let omega = Bivector::<D>::unit_plane(0, 1);
        let momentum = inertia.angular_momentum(&omega);
        assert!(
            (momentum.get(0, 1) - expected).abs() < 1e-12,
            "D={D}: plane inertia={}, expected={expected}",
            momentum.get(0, 1)
        );
    }

    #[test]
    fn parallel_axis_combination_matches_two_component_mass_moments() {
        let body_a = BivectorInertia::<3>::uniform_solid_ball(2.0, 1.0).unwrap();
        let body_b = BivectorInertia::<3>::uniform_solid_ball(1.0, 0.5).unwrap();
        let offset_a = SVector::new(-1.0, 0.0, 0.0);
        let offset_b = SVector::new(2.0, 0.0, 0.0);
        let combined = BivectorInertia::<3>::combine_at_common_center(
            2.0, &body_a, &offset_a, 1.0, &body_b, &offset_b,
        )
        .unwrap();

        // S_xx = 0.4 + 2*1^2 + 0.0125 + 1*2^2 = 5.4125.
        // S_yy = S_zz = 0.4 + 0.0125 = 0.4125.
        assert!((combined.second_moment()[(0, 0)] - 5.4125).abs() < 1e-12);
        assert!((combined.second_moment()[(1, 1)] - 0.4125).abs() < 1e-12);
        assert!((combined.second_moment()[(2, 2)] - 0.4125).abs() < 1e-12);
        let plane_xy = Bivector::<3>::unit_plane(0, 1);
        assert!((combined.angular_momentum(&plane_xy).get(0, 1) - 5.825).abs() < 1e-12);
    }

    #[test]
    fn parallel_axis_combination_rejects_non_centered_offsets() {
        let body_a = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        let body_b = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        assert_eq!(
            BivectorInertia::<3>::combine_at_common_center(
                1.0,
                &body_a,
                &SVector::new(0.0, 0.0, 0.0),
                1.0,
                &body_b,
                &SVector::new(1.0, 0.0, 0.0),
            )
            .unwrap_err(),
            RotationalInertiaError::OffsetsNotCentered
        );
    }

    #[test]
    fn point_mass_constructor_is_translation_invariant() {
        let origin_cloud = [
            (SVector::<f64, 3>::new(-1.0, 0.0, 0.0), 1.0),
            (SVector::<f64, 3>::new(1.0, 0.0, 0.0), 1.0),
        ];
        let translated_cloud = [
            (SVector::<f64, 3>::new(9.0, -4.0, 12.0), 1.0),
            (SVector::<f64, 3>::new(11.0, -4.0, 12.0), 1.0),
        ];

        let origin = BivectorInertia::<3>::from_point_masses(origin_cloud).unwrap();
        let translated = BivectorInertia::<3>::from_point_masses(translated_cloud).unwrap();

        assert!((origin.second_moment() - translated.second_moment()).norm() < 1e-12);
        let omega = Bivector::<3>::unit_plane(0, 1);
        let expected = 2.0;
        assert!((translated.angular_momentum(&omega).get(0, 1) - expected).abs() < 1e-12);
    }

    #[test]
    fn uniform_ball_has_dimension_correct_plane_inertia() {
        // I_plane = 2*m*r^2/(D+2), with m=3 and r=2.
        assert_uniform_ball_plane_moment::<2>(6.0);
        assert_uniform_ball_plane_moment::<3>(4.8);
        assert_uniform_ball_plane_moment::<4>(4.0);
    }

    #[test]
    fn uniform_ellipsoid_recovers_analytical_three_dimensional_moments() {
        // Unit mass, semi-axes (2, 1, 0.5): S_ii = a_i^2 / 5.
        // Rotation-plane moments are sums of the corresponding S diagonal.
        let inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap();

        let xy = Bivector::<3>::unit_plane(0, 1); // rotation about z
        let xz = Bivector::<3>::unit_plane(0, 2); // rotation about y
        let yz = Bivector::<3>::unit_plane(1, 2); // rotation about x
        assert!((inertia.angular_momentum(&xy).get(0, 1) - 1.0).abs() < 1e-12);
        assert!((inertia.angular_momentum(&xz).get(0, 2) - 0.85).abs() < 1e-12);
        assert!((inertia.angular_momentum(&yz).get(1, 2) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn asymmetric_three_dimensional_second_moments_recover_principal_inertia() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1.0, 2.0, 4.0]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();

        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 1.0); // rotation about axis 2: moment 1 + 2
        omega.set(0, 2, 2.0); // rotation about axis 1: moment 1 + 4
        omega.set(1, 2, -1.0); // rotation about axis 0: moment 2 + 4
        let momentum = inertia.angular_momentum(&omega);

        assert!((momentum.get(0, 1) - 3.0).abs() < 1e-12);
        assert!((momentum.get(0, 2) - 10.0).abs() < 1e-12);
        assert!((momentum.get(1, 2) + 6.0).abs() < 1e-12);
        assert!((inertia.kinetic_energy(&omega) - 14.5).abs() < 1e-12);
    }

    #[test]
    fn off_diagonal_mass_moments_couple_rotation_planes() {
        let second_moment =
            SMatrix::<f64, 3, 3>::new(2.0, 0.25, 0.0, 0.25, 3.0, 0.0, 0.0, 0.0, 4.0);
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let omega = Bivector::<3>::unit_plane(0, 2);
        let momentum = inertia.angular_momentum(&omega);

        assert!((momentum.get(0, 2) - 6.0).abs() < 1e-12);
        assert!((momentum.get(1, 2) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn rotated_inertia_is_covariant_and_preserves_kinetic_energy() {
        let body_inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap();
        let rotation = Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.63);
        let world_inertia = body_inertia.rotated_by(&rotation);

        let mut omega_body = Bivector::<3>::zero();
        omega_body.set(0, 1, 0.7);
        omega_body.set(0, 2, -0.4);
        omega_body.set(1, 2, 1.1);
        let omega_world_matrix =
            rotation.to_matrix() * omega_body.to_matrix() * rotation.to_matrix().transpose();
        let omega_world = Bivector::<3>::from_matrix(&omega_world_matrix);

        let momentum_body = body_inertia.angular_momentum(&omega_body);
        let momentum_world = world_inertia.angular_momentum(&omega_world);
        let expected_world_momentum =
            rotation.to_matrix() * momentum_body.to_matrix() * rotation.to_matrix().transpose();

        assert!((momentum_world.to_matrix() - expected_world_momentum).norm() < 1e-11);
        assert!(
            (body_inertia.kinetic_energy(&omega_body) - world_inertia.kinetic_energy(&omega_world))
                .abs()
                < 1e-11
        );
    }

    fn bivector_pairing(a: &Bivector<3>, b: &Bivector<3>) -> f64 {
        (0..3)
            .flat_map(|i| ((i + 1)..3).map(move |j| (i, j)))
            .map(|(i, j)| a.get(i, j) * b.get(i, j))
            .sum()
    }

    #[test]
    fn gyroscopic_acceleration_is_covariant_between_body_and_world_frames() {
        let body_inertia = BivectorInertia::<3>::uniform_solid_ellipsoid(
            2.0,
            SVector::new(1.5, 0.9, 0.4),
        )
        .unwrap();
        let rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 2),
            0.71,
        );
        let world_inertia = body_inertia.rotated_by(&rotation);

        let mut b_body = Bivector::<3>::zero();
        b_body.set(0, 1, 0.8);
        b_body.set(0, 2, -0.3);
        b_body.set(1, 2, 0.5);
        let mut torque_body = Bivector::<3>::zero();
        torque_body.set(0, 1, -0.2);
        torque_body.set(0, 2, 0.1);
        torque_body.set(1, 2, 0.4);

        let to_world = |value: &Bivector<3>| {
            Bivector::from_matrix(
                &(rotation.to_matrix() * value.to_matrix() * rotation.to_matrix().transpose()),
            )
        };
        let b_world = to_world(&b_body);
        let torque_world = to_world(&torque_body);

        let b_dot_body = body_inertia
            .angular_acceleration(&b_body, &torque_body, 1e-12)
            .unwrap();
        let b_dot_world = world_inertia
            .angular_acceleration(&b_world, &torque_world, 1e-12)
            .unwrap();

        assert!(
            b_dot_world.add(&to_world(&b_dot_body).scale(-1.0)).norm() < 1e-11,
            "body-frame and world-frame angular accelerations differ"
        );
    }

    #[test]
    fn rotational_inverse_effective_mass_matches_isotropic_lever_arm() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(3.0, 2.0).unwrap();
        let offset = SVector::new(1.0, 0.0, 0.0);
        let normal = SVector::new(0.0, 1.0, 0.0);
        let value = inertia
            .rotational_inverse_effective_mass(&offset, &normal, 1e-12)
            .unwrap();
        assert!((value - 1.0 / 4.8).abs() < 1e-12);
    }

    #[test]
    fn rotational_inverse_effective_mass_uses_asymmetric_plane_inertia() {
        let inertia = BivectorInertia::<3>::uniform_solid_ellipsoid(
            1.0,
            SVector::new(2.0, 1.0, 0.5),
        )
        .unwrap();
        let offset = SVector::new(1.0, 0.0, 0.0);
        let normal = SVector::new(0.0, 1.0, 0.0);
        let value = inertia
            .rotational_inverse_effective_mass(&offset, &normal, 1e-12)
            .unwrap();
        // XY plane moment is S_xx + S_yy = 0.8 + 0.2 = 1.0.
        assert!((value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rotational_inverse_effective_mass_rejects_zero_direction() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        assert_eq!(
            inertia
                .rotational_inverse_effective_mass(
                    &SVector::new(1.0, 0.0, 0.0),
                    &SVector::zeros(),
                    1e-12,
                )
                .unwrap_err(),
            RotationalInertiaError::InvalidDirection
        );
    }

    #[test]
    fn torque_free_angular_acceleration_preserves_energy_and_momentum_casimir() {
        let second_moment = SMatrix::<f64, 3, 3>::new(
            1.0, 0.15, 0.05,
            0.15, 2.0, 0.1,
            0.05, 0.1, 3.0,
        );
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let mut b = Bivector::<3>::zero();
        b.set(0, 1, 0.7);
        b.set(0, 2, -0.4);
        b.set(1, 2, 1.1);

        let zero_torque = Bivector::<3>::zero();
        let b_dot = inertia.angular_acceleration(&b, &zero_torque, 1e-12).unwrap();
        let l = inertia.angular_momentum(&b);
        let l_dot = inertia.angular_momentum(&b_dot);

        let energy_rate = bivector_pairing(&b, &l_dot);
        assert!(energy_rate.abs() < 1e-11, "energy_rate={energy_rate}");

        // For the torque-free spatial equation, dot(L) = [B,L].
        let expected_l_dot = Bivector::<3>::from_matrix(
            &(b.to_matrix() * l.to_matrix() - l.to_matrix() * b.to_matrix()),
        );
        assert!(
            l_dot.add(&expected_l_dot.scale(-1.0)).norm() < 1e-11,
            "l_dot={l_dot:?}, expected={expected_l_dot:?}"
        );
        assert!(bivector_pairing(&l, &l_dot).abs() < 1e-11);
    }

    #[test]
    fn isotropic_angular_acceleration_has_negative_stored_torque_sign() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(3.0, 2.0).unwrap();
        let zero_state = Bivector::<3>::zero();
        let mut torque_accumulator = Bivector::<3>::zero();
        torque_accumulator.set(0, 1, -2.0);
        let acceleration = inertia
            .angular_acceleration(&zero_state, &torque_accumulator, 1e-12)
            .unwrap();
        assert!((acceleration.get(0, 1) - 2.0 / 4.8).abs() < 1e-12);
        assert!(acceleration.get(0, 2).abs() < 1e-12);
        assert!(acceleration.get(1, 2).abs() < 1e-12);
    }

    #[test]
    fn inverse_operator_recovers_coupled_world_frame_angular_velocity() {
        let second_moment =
            SMatrix::<f64, 3, 3>::new(2.0, 0.25, 0.1, 0.25, 3.0, 0.2, 0.1, 0.2, 4.0);
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();

        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 0.7);
        omega.set(0, 2, -1.2);
        omega.set(1, 2, 0.35);

        let momentum = inertia.angular_momentum(&omega);
        let recovered = inertia
            .angular_velocity_from_momentum(&momentum, 1e-12)
            .unwrap();
        assert!(
            recovered.add(&omega.scale(-1.0)).norm() < 1e-11,
            "recovered={recovered:?}, expected={omega:?}"
        );
    }

    #[test]
    fn inverse_operator_rejects_singular_rotation_plane() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([2.0, 0.0, 0.0]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let momentum = Bivector::<3>::unit_plane(1, 2);
        assert_eq!(
            inertia
                .angular_velocity_from_momentum(&momentum, 1e-12)
                .unwrap_err(),
            RotationalInertiaError::SingularInertia
        );
    }

    #[test]
    fn inverse_operator_rejects_invalid_tolerance() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        assert_eq!(
            inertia
                .angular_velocity_from_momentum(&Bivector::zero(), 0.0)
                .unwrap_err(),
            RotationalInertiaError::InvalidTolerance
        );
    }

    #[test]
    fn rejects_indefinite_second_moment() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1.0, 1.0, -0.01]));
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn rejects_small_scale_indefinite_second_moment() {
        // PSD tolerance must scale with the physical mass-moment magnitude,
        // not with an arbitrary unit floor.
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1e-20, 1e-20, -1e-15]));
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn rejects_non_symmetric_second_moment() {
        let second_moment = SMatrix::<f64, 3, 3>::new(1.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotSymmetric
        );
    }
}
