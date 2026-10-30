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
            .fold(1.0_f64, |m, value| m.max(value.abs()));
        for i in 0..D {
            for j in 0..D {
                if (second_moment[(i, j)] - second_moment[(j, i)]).abs() > 1e-12 * scale {
                    return Err(RotationalInertiaError::NotSymmetric);
                }
            }
        }

        let symmetric = (second_moment + second_moment.transpose()) * 0.5;
        let min_eigenvalue = nalgebra::linalg::SymmetricEigen::new(symmetric.clone())
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if min_eigenvalue < -1e-12 * scale {
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
            total_mass += mass;
            weighted_position += position * *mass;
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
        if !mass.is_finite() || !radius.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        if mass < 0.0 || radius < 0.0 {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }
        let second_moment_scale = mass * radius * radius / (D as f64 + 2.0);
        Self::from_second_moment(SMatrix::<f64, D, D>::identity() * second_moment_scale)
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
