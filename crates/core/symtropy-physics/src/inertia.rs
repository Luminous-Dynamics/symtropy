//! Rotational inertia on the N-dimensional bivector rotation space.
//!
//! The centered second-moment matrix is S = integral r r^T dm. For an angular-
//! velocity bivector represented by a skew matrix Omega, the corresponding
//! angular-momentum bivector is I(Omega) = Omega*S + S*Omega. This is the
//! dimension-generic form of the familiar 3D spatial inertia tensor, and it
//! naturally couples rotation planes for asymmetric mass distributions.
//!
//! This module currently provides the forward inertia operator and energy
//! calculation. It is not yet wired into RigidBody integration/contact; inverse
//! application, gyroscopic evolution, and CAD-derived mass-property assembly
//! remain separate validated implementation steps.

use nalgebra::{SMatrix, SVector};
use symtropy_math::Bivector;

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
        let min_eigenvalue = nalgebra::linalg::SymmetricEigen::new(symmetric.clone())
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if min_eigenvalue < -tolerance {
            return Err(RotationalInertiaError::NotPositiveSemidefinite);
        }

        Ok(Self {
            second_moment: symmetric,
        })
    }

    /// Construct from a discrete set of point masses.
    ///
    /// Positions may be in any translated world frame; this constructor first
    /// computes the center of mass and then forms the centered second moment.
    /// It is suitable for CAD/mesh mass-lumping and point-cloud reference tests.
    pub fn from_point_masses<I>(
        point_masses: I,
    ) -> Result<Self, RotationalInertiaError>
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
    pub fn uniform_solid_ball(
        mass: f64,
        radius: f64,
    ) -> Result<Self, RotationalInertiaError> {
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

    /// Return the centered second-moment matrix used by this operator.
    pub fn second_moment(&self) -> &SMatrix<f64, D, D> {
        &self.second_moment
    }

    /// Map angular velocity to angular momentum: L = Omega*S + S*Omega.
    ///
    /// The output uses the same lexicographic (i,j), i<j plane ordering as
    /// Bivector<D>. This map is symmetric and positive semidefinite when S
    /// is a physically valid centered mass second-moment matrix.
    pub fn angular_momentum(&self, angular_velocity: &Bivector<D>) -> Bivector<D> {
        let omega = angular_velocity.to_matrix();
        let momentum = omega * self.second_moment + self.second_moment * omega;
        Bivector::from_matrix(&momentum)
    }

    /// Invert angular momentum to angular velocity on the nonsingular rotation subspace.
    ///
    /// The symmetric eigendecomposition S = Q*Lambda*Q^T diagonalizes the
    /// inertia operator on bivectors: in principal coordinates each plane
    /// (i,j) has eigenvalue lambda_i + lambda_j. This avoids constructing a
    /// dense D(D-1)/2 square matrix and preserves coupling in the world frame.
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

        let eigen = nalgebra::linalg::SymmetricEigen::new(self.second_moment.clone());
        let largest_eigenvalue = eigen
            .eigenvalues
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(*value));
        let threshold = relative_tolerance * largest_eigenvalue.max(f64::MIN_POSITIVE);

        let q = eigen.eigenvectors;
        let q_t = q.transpose();
        let momentum_local = q_t * angular_momentum.to_matrix() * q;
        let mut omega_local = nalgebra::SMatrix::<f64, D, D>::zeros();

        for i in 0..D {
            for j in (i + 1)..D {
                let plane_inertia = eigen.eigenvalues[i] + eigen.eigenvalues[j];
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

    /// Rotational kinetic energy, 1/2 <omega, I(omega)> in the bivector inner product.
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
        assert!(
            (translated.angular_momentum(&omega).get(0, 1) - expected).abs() < 1e-12
        );
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
        let inertia = BivectorInertia::<3>::uniform_solid_ellipsoid(
            1.0,
            SVector::new(2.0, 1.0, 0.5),
        )
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
        let second_moment = SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([
            1.0, 2.0, 4.0,
        ]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();

        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 1.0);  // rotation about axis 2: moment 1 + 2
        omega.set(0, 2, 2.0);  // rotation about axis 1: moment 1 + 4
        omega.set(1, 2, -1.0); // rotation about axis 0: moment 2 + 4
        let momentum = inertia.angular_momentum(&omega);

        assert!((momentum.get(0, 1) - 3.0).abs() < 1e-12);
        assert!((momentum.get(0, 2) - 10.0).abs() < 1e-12);
        assert!((momentum.get(1, 2) + 6.0).abs() < 1e-12);
        assert!((inertia.kinetic_energy(&omega) - 14.5).abs() < 1e-12);
    }

    #[test]
    fn off_diagonal_mass_moments_couple_rotation_planes() {
        let second_moment = SMatrix::<f64, 3, 3>::new(
            2.0, 0.25, 0.0,
            0.25, 3.0, 0.0,
            0.0, 0.0, 4.0,
        );
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let omega = Bivector::<3>::unit_plane(0, 2);
        let momentum = inertia.angular_momentum(&omega);

        assert!((momentum.get(0, 2) - 6.0).abs() < 1e-12);
        assert!((momentum.get(1, 2) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn inverse_operator_recovers_coupled_world_frame_angular_velocity() {
        let second_moment = SMatrix::<f64, 3, 3>::new(
            2.0, 0.25, 0.1,
            0.25, 3.0, 0.2,
            0.1, 0.2, 4.0,
        );
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
        let second_moment = SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([
            2.0, 0.0, 0.0,
        ]));
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
        let second_moment = SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([
            1.0, 1.0, -0.01,
        ]));
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn rejects_small_scale_indefinite_second_moment() {
        // PSD tolerance must scale with the physical mass-moment magnitude,
        // not with an arbitrary unit floor.
        let second_moment = SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([
            1e-20, 1e-20, -1e-15,
        ]));
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn rejects_non_symmetric_second_moment() {
        let second_moment = SMatrix::<f64, 3, 3>::new(
            1.0, 0.5, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        );
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotSymmetric
        );
    }
}
