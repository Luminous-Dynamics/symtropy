// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Rotational inertia as an operator on the engine's bivector space.
//!
//! For D-dimensional rotation, the angular-velocity space is Λ²(R^D), with
//! D(D-1)/2 components in Bivector's lexicographic (i,j) basis. This module
//! represents inertia as a symmetric positive-definite linear operator on
//! that space, rather than averaging a spatial-axis inertia vector.
//!
//! For a point mass at center-relative position r, Bivector::to_matrix()
//! defines the skew generator B and the induced point velocity is B*r.
//! The rotational kinetic energy is therefore
//! T = 1/2 * Σ m * ||B*r||² = 1/2 * omegaᵀ * I * omega.
//! The mass-point constructor assembles I from this quadratic form.
//!
//! This is an additive math component. It does not yet replace RigidBody's
//! legacy inertia fields or change the production integrator.

use nalgebra::{DMatrix, SVector};
use symtropy_math::{Bivector, Rotor};

/// Construction failures for a bivector-space rotational inertia operator.
#[derive(Clone, Debug, PartialEq)]
pub enum BivectorInertiaError {
    /// The requested dimension is outside Bivector's supported range 2..=9.
    UnsupportedSpatialDimension { dimension: usize },
    /// The supplied operator has the wrong square dimensions.
    MatrixDimensionMismatch {
        expected: usize,
        rows: usize,
        columns: usize,
    },
    /// A matrix entry or point-mass input is non-finite.
    NonFiniteInput,
    /// The supplied matrix is not symmetric within the scale-aware tolerance.
    MatrixNotSymmetric { row: usize, column: usize },
    /// The operator is not positive definite.
    MatrixNotPositiveDefinite,
    /// The point-mass set is empty.
    EmptyPointMassSet,
    /// A point mass has a non-positive or non-finite mass.
    InvalidPointMass { index: usize },
    /// A point-mass position contains a non-finite coordinate.
    NonFinitePointPosition { index: usize },
}

/// Positive-definite rotational inertia operator in the lexicographic
/// bivector basis (01, 02, ..., (D-2)(D-1)).
#[derive(Clone, Debug)]
pub struct BivectorInertia<const D: usize> {
    pairs: Vec<(usize, usize)>,
    matrix: DMatrix<f64>,
    inverse: DMatrix<f64>,
}

impl<const D: usize> BivectorInertia<D> {
    fn supported_pairs() -> Result<Vec<(usize, usize)>, BivectorInertiaError> {
        if !(2..=9).contains(&D) {
            return Err(BivectorInertiaError::UnsupportedSpatialDimension { dimension: D });
        }
        let mut pairs = Vec::with_capacity(D * (D - 1) / 2);
        for i in 0..D {
            for j in (i + 1)..D {
                pairs.push((i, j));
            }
        }
        Ok(pairs)
    }

    /// Construct from a dense operator in Bivector's lexicographic basis.
    ///
    /// The input must be finite, symmetric (within a scale-aware tolerance),
    /// and positive definite. A near-symmetric input is projected onto its
    /// symmetric part only after the symmetry check passes.
    pub fn from_matrix(matrix: DMatrix<f64>) -> Result<Self, BivectorInertiaError> {
        let pairs = Self::supported_pairs()?;
        let expected = pairs.len();
        if matrix.nrows() != expected || matrix.ncols() != expected {
            return Err(BivectorInertiaError::MatrixDimensionMismatch {
                expected,
                rows: matrix.nrows(),
                columns: matrix.ncols(),
            });
        }
        if !matrix.iter().all(|value| value.is_finite()) {
            return Err(BivectorInertiaError::NonFiniteInput);
        }

        let scale = matrix.iter().fold(1.0_f64, |m, value| m.max(value.abs()));
        let symmetry_tolerance = 1e-12 * scale;
        for row in 0..expected {
            for column in (row + 1)..expected {
                if (matrix[(row, column)] - matrix[(column, row)]).abs() > symmetry_tolerance {
                    return Err(BivectorInertiaError::MatrixNotSymmetric { row, column });
                }
            }
        }

        let symmetric = (&matrix + matrix.transpose()) * 0.5;
        let cholesky = symmetric
            .clone()
            .cholesky()
            .ok_or(BivectorInertiaError::MatrixNotPositiveDefinite)?;
        let inverse = cholesky.inverse();

        Ok(Self {
            pairs,
            matrix: symmetric,
            inverse,
        })
    }

    /// Build the exact point-mass inertia operator about the center of mass.
    ///
    /// Each input position may be expressed in any common body-local frame.
    /// The constructor computes the center of mass and recenters all points
    /// before assembling the operator. The resulting matrix preserves
    /// off-diagonal coupling between different rotation planes.
    pub fn from_point_masses(
        point_masses: &[(f64, SVector<f64, D>)],
    ) -> Result<Self, BivectorInertiaError> {
        let pairs = Self::supported_pairs()?;
        if point_masses.is_empty() {
            return Err(BivectorInertiaError::EmptyPointMassSet);
        }

        let mut total_mass = 0.0;
        let mut weighted_center = SVector::<f64, D>::zeros();
        for (index, (mass, position)) in point_masses.iter().enumerate() {
            if !mass.is_finite() || *mass <= 0.0 {
                return Err(BivectorInertiaError::InvalidPointMass { index });
            }
            if !position.iter().all(|value| value.is_finite()) {
                return Err(BivectorInertiaError::NonFinitePointPosition { index });
            }
            total_mass += *mass;
            weighted_center += *position * *mass;
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(BivectorInertiaError::NonFiniteInput);
        }
        let center = weighted_center / total_mass;

        let component_count = pairs.len();
        let mut matrix = DMatrix::<f64>::zeros(component_count, component_count);

        for (mass, position) in point_masses {
            let relative = *position - center;
            // Column p is the point velocity generated by unit angular
            // velocity in bivector plane pairs[p].
            for (p, &(i, j)) in pairs.iter().enumerate() {
                for (q, &(k, l)) in pairs.iter().enumerate().skip(p) {
                    let mut dot = 0.0;
                    for axis in 0..D {
                        let velocity_p = if axis == i {
                            relative[j]
                        } else if axis == j {
                            -relative[i]
                        } else {
                            0.0
                        };
                        let velocity_q = if axis == k {
                            relative[l]
                        } else if axis == l {
                            -relative[k]
                        } else {
                            0.0
                        };
                        dot += velocity_p * velocity_q;
                    }
                    let value = *mass * dot;
                    matrix[(p, q)] += value;
                    if p != q {
                        matrix[(q, p)] += value;
                    }
                }
            }
        }

        Self::from_matrix(matrix)
    }

    /// Construct an isotropic operator whose moment is equal in every plane.
    pub fn isotropic(moment: f64) -> Result<Self, BivectorInertiaError> {
        let pairs = Self::supported_pairs()?;
        let matrix = DMatrix::<f64>::identity(pairs.len(), pairs.len()) * moment;
        Self::from_matrix(matrix)
    }

    /// Number of independent rotation-plane components.
    pub fn component_count(&self) -> usize {
        self.pairs.len()
    }

    /// Stable lexicographic (i,j) basis used by this operator.
    pub fn basis_pairs(&self) -> &[(usize, usize)] {
        &self.pairs
    }

    /// Dense symmetric inertia operator in the lexicographic bivector basis.
    pub fn matrix(&self) -> &DMatrix<f64> {
        &self.matrix
    }

    /// Apply inertia: angular momentum bivector L = I * omega.
    pub fn apply(&self, angular_velocity: &Bivector<D>) -> Bivector<D> {
        let mut result = Bivector::<D>::zero();
        for row in 0..self.pairs.len() {
            let mut value = 0.0;
            for column in 0..self.pairs.len() {
                let (i, j) = self.pairs[column];
                value += self.matrix[(row, column)] * angular_velocity.get(i, j);
            }
            let (i, j) = self.pairs[row];
            result.set(i, j, value);
        }
        result
    }

    /// Solve omega = I^-1 * L for angular velocity from angular momentum.
    pub fn solve(&self, angular_momentum: &Bivector<D>) -> Bivector<D> {
        let mut result = Bivector::<D>::zero();
        for row in 0..self.pairs.len() {
            let mut value = 0.0;
            for column in 0..self.pairs.len() {
                let (i, j) = self.pairs[column];
                value += self.inverse[(row, column)] * angular_momentum.get(i, j);
            }
            let (i, j) = self.pairs[row];
            result.set(i, j, value);
        }
        result
    }

    /// Rotational kinetic energy: one half of <omega, I * omega>.
    pub fn kinetic_energy(&self, angular_velocity: &Bivector<D>) -> f64 {
        let angular_momentum = self.apply(angular_velocity);
        let mut pairing = 0.0;
        for &(i, j) in &self.pairs {
            pairing += angular_velocity.get(i, j) * angular_momentum.get(i, j);
        }
        0.5 * pairing
    }

    /// Express this operator in a rotated frame using the induced adjoint
    /// action B_world = R * B_body * R^T on antisymmetric generators.
    ///
    /// If A is the matrix of that induced action in the lexicographic
    /// bivector basis, the transformed operator is A * I * A^T.
    pub fn rotated_by(&self, rotation: &Rotor<D>) -> Result<Self, BivectorInertiaError> {
        let count = self.pairs.len();
        let mut adjoint = DMatrix::<f64>::zeros(count, count);
        let rotation_matrix = rotation.to_matrix();

        for (column, &(i, j)) in self.pairs.iter().enumerate() {
            let basis = Bivector::<D>::unit_plane(i, j);
            let transformed_matrix =
                rotation_matrix * basis.to_matrix() * rotation_matrix.transpose();
            let transformed = Bivector::<D>::from_matrix(&transformed_matrix);

            for (row, &(k, l)) in self.pairs.iter().enumerate() {
                adjoint[(row, column)] = transformed.get(k, l);
            }
        }

        let transformed = &adjoint * &self.matrix * adjoint.transpose();
        Self::from_matrix(transformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Bivector;

    fn symmetric_axis_masses<const D: usize>() -> Vec<(f64, SVector<f64, D>)> {
        let mut points = Vec::new();
        for axis in 0..D {
            let mut positive = SVector::<f64, D>::zeros();
            positive[axis] = 1.0;
            let mut negative = SVector::<f64, D>::zeros();
            negative[axis] = -1.0;
            points.push((1.0, positive));
            points.push((1.0, negative));
        }
        points
    }

    #[test]
    fn three_d_symmetric_axis_distribution_is_isotropic() {
        let inertia =
            BivectorInertia::<3>::from_point_masses(&symmetric_axis_masses::<3>()).unwrap();

        assert_eq!(inertia.basis_pairs(), &[(0, 1), (0, 2), (1, 2)]);
        for index in 0..3 {
            assert!((inertia.matrix()[(index, index)] - 4.0).abs() < 1e-12);
            for other in 0..3 {
                if index != other {
                    assert!(inertia.matrix()[(index, other)].abs() < 1e-12);
                }
            }
        }
    }

    #[test]
    fn four_d_operator_has_six_rotation_plane_components() {
        let inertia =
            BivectorInertia::<4>::from_point_masses(&symmetric_axis_masses::<4>()).unwrap();
        assert_eq!(inertia.component_count(), 6);
        assert_eq!(
            inertia.basis_pairs(),
            &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        );
        for index in 0..6 {
            assert!((inertia.matrix()[(index, index)] - 4.0).abs() < 1e-12);
        }
    }

    #[test]
    fn two_d_operator_has_one_rotation_plane_degree_of_freedom() {
        let inertia =
            BivectorInertia::<2>::from_point_masses(&symmetric_axis_masses::<2>()).unwrap();
        assert_eq!(inertia.component_count(), 1);
        assert_eq!(inertia.basis_pairs(), &[(0, 1)]);
        assert!((inertia.matrix()[(0, 0)] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn asymmetric_mass_distribution_preserves_cross_plane_coupling() {
        let mut points = symmetric_axis_masses::<3>();
        points.push((1.0, SVector::from([0.0, 1.0, 1.0])));
        points.push((1.0, SVector::from([0.0, -1.0, -1.0])));

        let inertia = BivectorInertia::<3>::from_point_masses(&points).unwrap();
        // The tilted pair contributes a non-zero I_(01,02) product term.
        // A diagonal-per-axis or scalar-average approximation would lose it.
        assert!((inertia.matrix()[(0, 1)] - 2.0).abs() < 1e-12);
        assert!((inertia.matrix()[(1, 0)] - 2.0).abs() < 1e-12);
        assert!((inertia.matrix()[(0, 1)] - inertia.matrix()[(1, 0)]).abs() < 1e-14);
    }

    #[test]
    fn angular_momentum_solve_and_energy_are_consistent() {
        let inertia =
            BivectorInertia::<4>::from_point_masses(&symmetric_axis_masses::<4>()).unwrap();
        let mut omega = Bivector::<4>::zero();
        omega.set(0, 1, 1.0);
        omega.set(0, 3, -2.0);
        omega.set(1, 2, 0.5);

        let momentum = inertia.apply(&omega);
        let recovered = inertia.solve(&momentum);
        assert_eq!(recovered, omega);
        assert!((inertia.kinetic_energy(&omega) - 0.5 * 4.0 * omega.norm_squared()).abs() < 1e-12);
    }

    #[test]
    fn rotated_operator_preserves_energy_and_basis_norm() {
        let points = {
            let mut values = Vec::new();
            for vector in [
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ] {
                values.push((1.0, SVector::from(vector)));
            }
            values
        };
        let inertia = BivectorInertia::<3>::from_point_masses(&points).unwrap();
        let rotation = Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.73);
        let rotated = inertia.rotated_by(&rotation).unwrap();

        let mut omega_body = Bivector::<3>::zero();
        omega_body.set(0, 1, 0.4);
        omega_body.set(0, 2, -0.8);
        omega_body.set(1, 2, 1.1);
        let omega_matrix =
            rotation.to_matrix() * omega_body.to_matrix() * rotation.to_matrix().transpose();
        let omega_world = Bivector::<3>::from_matrix(&omega_matrix);

        assert!(
            (rotated.kinetic_energy(&omega_world) - inertia.kinetic_energy(&omega_body)).abs()
                < 1e-10
        );
    }

    #[test]
    fn center_of_mass_inertia_is_translation_invariant() {
        let original = vec![
            (2.0, SVector::from([1.0, 0.0, 0.0])),
            (1.0, SVector::from([0.0, 2.0, 0.0])),
            (3.0, SVector::from([0.0, 0.0, 1.0])),
            (1.5, SVector::from([-1.0, -1.0, 0.5])),
        ];
        let translation = SVector::from([17.0, -8.0, 3.5]);
        let translated: Vec<_> = original
            .iter()
            .map(|(mass, position)| (*mass, *position + translation))
            .collect();

        let before = BivectorInertia::<3>::from_point_masses(&original).unwrap();
        let after = BivectorInertia::<3>::from_point_masses(&translated).unwrap();

        assert!(
            (before.matrix() - after.matrix()).norm() < 1e-10,
            "translating every point mass must not change center-of-mass inertia"
        );
    }

    #[test]
    fn malformed_operators_and_mass_distributions_are_rejected() {
        assert!(matches!(
            BivectorInertia::<3>::from_matrix(DMatrix::identity(2, 2)),
            Err(BivectorInertiaError::MatrixDimensionMismatch { .. })
        ));
        assert!(matches!(
            BivectorInertia::<3>::from_matrix(DMatrix::zeros(3, 3)),
            Err(BivectorInertiaError::MatrixNotPositiveDefinite)
        ));
        assert!(matches!(
            BivectorInertia::<3>::from_matrix(DMatrix::from_element(3, 3, f64::NAN)),
            Err(BivectorInertiaError::NonFiniteInput)
        ));
        let mut nonsymmetric = DMatrix::identity(3, 3);
        nonsymmetric[(0, 1)] = 0.25;
        assert!(matches!(
            BivectorInertia::<3>::from_matrix(nonsymmetric),
            Err(BivectorInertiaError::MatrixNotSymmetric { row: 0, column: 1 })
        ));
        assert!(matches!(
            BivectorInertia::<3>::from_point_masses(&[]),
            Err(BivectorInertiaError::EmptyPointMassSet)
        ));
        assert!(matches!(
            BivectorInertia::<3>::from_point_masses(&[(0.0, SVector::zeros())]),
            Err(BivectorInertiaError::InvalidPointMass { index: 0 })
        ));
        assert!(matches!(
            BivectorInertia::<3>::from_point_masses(&[(1.0, SVector::from([f64::NAN, 0.0, 0.0]))]),
            Err(BivectorInertiaError::NonFinitePointPosition { index: 0 })
        ));
        assert!(matches!(
            BivectorInertia::<1>::isotropic(1.0),
            Err(BivectorInertiaError::UnsupportedSpatialDimension { dimension: 1 })
        ));
    }
}
