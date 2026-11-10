// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Frame-explicit mass properties and rotational inertia operators.
//!
//! In N dimensions, angular velocity and torque live in the bivector space
//! so(D), whose dimension is P = D(D-1)/2. A single scalar or one value per
//! spatial coordinate axis cannot represent a general rotational inertia
//! operator when the mass distribution is anisotropic or has products of
//! inertia.
//!
//! This module provides the validated data model only. The operator is stored
//! in body-frame bivector coordinates with lexicographic plane order
//! (0,1), (0,2), ..., (D-2,D-1). The current integrator/body/contact paths are
//! not silently switched to this representation by adding this module.

use nalgebra::DMatrix;
use nalgebra::SVector;
use symtropy_math::Bivector;

/// Maximum bivector dimension supported by symtropy-math (D <= 9).
const MAX_ROTATIONAL_DOF: usize = 36;

/// Number of independent bivector coordinates in so(D).
pub const fn rotational_dof_count<const D: usize>() -> usize {
    D.saturating_mul(D.saturating_sub(1)) / 2
}

/// Errors from constructing or evaluating physical mass properties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MassPropertiesError {
    /// The dimension has fewer than two axes or the operator size overflowed.
    InvalidDimension,
    /// The supplied row-major matrix has the wrong number of elements.
    MatrixSizeMismatch { expected: usize, actual: usize },
    /// The matrix contains a NaN or infinity.
    NonFiniteMatrix,
    /// The matrix is not symmetric within the declared relative tolerance.
    NotSymmetric,
    /// The symmetric matrix is not positive definite.
    NotPositiveDefinite,
    /// A vector does not have the expected bivector-coordinate count.
    VectorSizeMismatch { expected: usize, actual: usize },
    /// A vector or computed result contains a NaN or infinity.
    NonFiniteValue,
    /// Mass must be finite and strictly positive for dynamic mass properties.
    InvalidMass,
    /// Center of mass contains a NaN or infinity.
    InvalidCenterOfMass,
    /// A plane index is out of range or not ordered with i < j.
    InvalidPlaneIndex,
}

/// Validated symmetric positive-definite inertia operator in body-frame
/// bivector coordinates.
///
/// For angular-velocity coordinates `omega`, rotational kinetic energy is:
///
/// `T_rot = 0.5 * omegaᵀ * I_body * omega`.
///
/// The operator is represented as a dense row-major P-by-P matrix, where
/// P = D(D-1)/2. This is suitable for the small dimensions supported by this
/// crate; larger-D sparse/block structures can be introduced separately.
#[derive(Clone, Debug, PartialEq)]
pub struct RotationalInertiaOperator<const D: usize> {
    matrix_row_major: Vec<f64>,
    /// Cached lower Cholesky factor in row-major order. Construction is the
    /// only factorization; real-time solves reuse this validated factor.
    cholesky_lower_row_major: Vec<f64>,
}

impl<const D: usize> RotationalInertiaOperator<D> {
    /// Construct an algebraically valid operator from a dense row-major matrix.
    ///
    /// The matrix must be finite, symmetric within a relative tolerance of
    /// 1e-12, and positive definite. Asymmetric input is rejected rather than
    /// silently symmetrized, because products of inertia are meaningful data.
    ///
    /// An arbitrary SPD operator is not necessarily realizable by a Euclidean
    /// mass distribution. For physically realizable rigid-body inertia, prefer
    /// [`from_body_second_moment_matrix`], which derives the operator from the
    /// central second-moment tensor.
    pub fn try_from_row_major(matrix_row_major: Vec<f64>) -> Result<Self, MassPropertiesError> {
        let dof = rotational_dof_count::<D>();
        if D < 2 || dof == 0 {
            return Err(MassPropertiesError::InvalidDimension);
        }
        let expected = dof
            .checked_mul(dof)
            .ok_or(MassPropertiesError::InvalidDimension)?;
        if matrix_row_major.len() != expected {
            return Err(MassPropertiesError::MatrixSizeMismatch {
                expected,
                actual: matrix_row_major.len(),
            });
        }
        if !matrix_row_major.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::NonFiniteMatrix);
        }

        let scale = matrix_row_major
            .iter()
            .fold(1.0_f64, |maximum, value| maximum.max(value.abs()));
        let tolerance = 1e-12 * scale;
        for row in 0..dof {
            for column in 0..row {
                let a = matrix_row_major[row * dof + column];
                let b = matrix_row_major[column * dof + row];
                if (a - b).abs() > tolerance {
                    return Err(MassPropertiesError::NotSymmetric);
                }
            }
        }

        if dof > MAX_ROTATIONAL_DOF {
            return Err(MassPropertiesError::InvalidDimension);
        }
        let matrix = DMatrix::from_row_slice(dof, dof, &matrix_row_major);
        let Some(cholesky) = matrix.cholesky() else {
            return Err(MassPropertiesError::NotPositiveDefinite);
        };
        let lower = cholesky.l();
        let mut cholesky_lower_row_major = vec![0.0; expected];
        for row in 0..dof {
            for column in 0..=row {
                cholesky_lower_row_major[row * dof + column] = lower[(row, column)];
            }
        }

        Ok(Self {
            matrix_row_major,
            cholesky_lower_row_major,
        })
    }

    /// Derive the body-frame inertia operator from the central second-moment
    /// matrix S = integral(r r^T dm), in row-major order and kg*m^2.
    ///
    /// For the ordered plane basis E_(i,j), with +1 at (i,j) and -1 at (j,i),
    /// this constructs:
    ///
    /// `I[a,b] = trace(E_a^T * E_b * S)`.
    ///
    /// Consequently, `T_rot = 0.5 * omega^T * I * omega` equals
    /// `0.5 * integral(||Omega r||^2 dm)`. The input second-moment matrix must
    /// be finite, symmetric and positive definite. This gives a physically
    /// realizable inertia operator for a full-dimensional mass distribution;
    /// degenerate lower-dimensional bodies require a separate semidefinite
    /// representation and are not accepted by this dynamic-body constructor.
    pub fn from_body_second_moment_matrix(
        second_moment_row_major: Vec<f64>,
    ) -> Result<Self, MassPropertiesError> {
        if D < 2 {
            return Err(MassPropertiesError::InvalidDimension);
        }
        let expected = D
            .checked_mul(D)
            .ok_or(MassPropertiesError::InvalidDimension)?;
        if second_moment_row_major.len() != expected {
            return Err(MassPropertiesError::MatrixSizeMismatch {
                expected,
                actual: second_moment_row_major.len(),
            });
        }
        if !second_moment_row_major
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(MassPropertiesError::NonFiniteMatrix);
        }

        let scale = second_moment_row_major
            .iter()
            .fold(1.0_f64, |maximum, value| maximum.max(value.abs()));
        let tolerance = 1e-12 * scale;
        for row in 0..D {
            for column in 0..row {
                let a = second_moment_row_major[row * D + column];
                let b = second_moment_row_major[column * D + row];
                if (a - b).abs() > tolerance {
                    return Err(MassPropertiesError::NotSymmetric);
                }
            }
        }

        let second_moment = DMatrix::from_row_slice(D, D, &second_moment_row_major);
        if second_moment.clone().cholesky().is_none() {
            return Err(MassPropertiesError::NotPositiveDefinite);
        }

        let dof = rotational_dof_count::<D>();
        let matrix_len = dof
            .checked_mul(dof)
            .ok_or(MassPropertiesError::InvalidDimension)?;
        let mut operator = vec![0.0; matrix_len];

        let mut planes = Vec::with_capacity(dof);
        for i in 0..D {
            for j in (i + 1)..D {
                planes.push((i, j));
            }
        }

        for (row, &(i, j)) in planes.iter().enumerate() {
            for (column, &(k, l)) in planes.iter().enumerate() {
                let mut value = 0.0;
                for u in 0..D {
                    for v in 0..D {
                        let basis_a_transpose = if v == i && u == j {
                            1.0
                        } else if v == j && u == i {
                            -1.0
                        } else {
                            0.0
                        };
                        if basis_a_transpose == 0.0 {
                            continue;
                        }
                        for w in 0..D {
                            let basis_b = if v == k && w == l {
                                1.0
                            } else if v == l && w == k {
                                -1.0
                            } else {
                                0.0
                            };
                            value += basis_a_transpose * basis_b * second_moment[(w, u)];
                        }
                    }
                }
                operator[row * dof + column] = value;
            }
        }

        Self::try_from_row_major(operator)
    }

    /// Construct a diagonal isotropic operator with the same inertia in every
    /// bivector plane. This is exact for an isotropic mass distribution such
    /// as a uniform sphere, but is not a general anisotropic-body fallback.
    pub fn isotropic(moment_per_plane: f64) -> Result<Self, MassPropertiesError> {
        let dof = rotational_dof_count::<D>();
        if D < 2 || dof == 0 {
            return Err(MassPropertiesError::InvalidDimension);
        }
        if !moment_per_plane.is_finite() {
            return Err(MassPropertiesError::NonFiniteMatrix);
        }
        if moment_per_plane <= 0.0 {
            return Err(MassPropertiesError::NotPositiveDefinite);
        }
        let matrix_len = dof
            .checked_mul(dof)
            .ok_or(MassPropertiesError::InvalidDimension)?;
        let mut matrix = vec![0.0; matrix_len];
        for index in 0..dof {
            matrix[index * dof + index] = moment_per_plane;
        }
        Self::try_from_row_major(matrix)
    }

    /// Number of independent bivector coordinates.
    pub const fn dof(&self) -> usize {
        rotational_dof_count::<D>()
    }

    /// Borrow the validated row-major body-frame operator.
    pub fn as_row_major(&self) -> &[f64] {
        &self.matrix_row_major
    }

    /// Apply the inertia operator to a body-frame angular velocity/acceleration
    /// bivector coordinate vector.
    pub fn apply(&self, vector: &[f64]) -> Result<Vec<f64>, MassPropertiesError> {
        let dof = self.dof();
        if vector.len() != dof {
            return Err(MassPropertiesError::VectorSizeMismatch {
                expected: dof,
                actual: vector.len(),
            });
        }
        if !vector.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::NonFiniteValue);
        }

        let mut result = vec![0.0; dof];
        for (row, result_value) in result.iter_mut().enumerate() {
            let mut value = 0.0;
            for (column, vector_value) in vector.iter().enumerate() {
                value += self.matrix_row_major[row * dof + column] * vector_value;
            }
            if !value.is_finite() {
                return Err(MassPropertiesError::NonFiniteValue);
            }
            *result_value = value;
        }
        Ok(result)
    }

    /// Solve I_body * omega = angular_momentum for omega.
    /// Solve I_body * omega = angular_momentum using the cached Cholesky
    /// factor. This allocating slice API is retained for compatibility;
    /// hot-loop callers should use solve_bivector.
    pub fn solve(&self, angular_momentum: &[f64]) -> Result<Vec<f64>, MassPropertiesError> {
        let dof = self.dof();
        if angular_momentum.len() != dof {
            return Err(MassPropertiesError::VectorSizeMismatch {
                expected: dof,
                actual: angular_momentum.len(),
            });
        }
        if !angular_momentum.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::NonFiniteValue);
        }
        if dof > MAX_ROTATIONAL_DOF {
            return Err(MassPropertiesError::InvalidDimension);
        }

        let mut rhs = [0.0_f64; MAX_ROTATIONAL_DOF];
        rhs[..dof].copy_from_slice(angular_momentum);
        let solution = self.solve_fixed(&rhs)?;
        Ok(solution[..dof].to_vec())
    }

    /// Apply the validated inertia operator directly to a bivector without
    /// allocating. Coordinates use the canonical lexicographic plane order.
    pub fn apply_bivector(
        &self,
        vector: &Bivector<D>,
    ) -> Result<Bivector<D>, MassPropertiesError> {
        if !vector.is_finite() {
            return Err(MassPropertiesError::NonFiniteValue);
        }
        let dof = self.dof();
        if dof > MAX_ROTATIONAL_DOF {
            return Err(MassPropertiesError::InvalidDimension);
        }

        let mut input = [0.0_f64; MAX_ROTATIONAL_DOF];
        for (index, component) in input.iter_mut().take(dof).enumerate() {
            let (i, j) = Self::plane_pair(index)?;
            *component = vector.get(i, j);
        }

        let mut output = [0.0_f64; MAX_ROTATIONAL_DOF];
        for (row, output_value) in output.iter_mut().take(dof).enumerate() {
            let row_start = row * dof;
            *output_value = self.matrix_row_major[row_start..row_start + dof]
                .iter()
                .zip(input.iter().take(dof))
                .map(|(matrix_value, vector_value)| matrix_value * vector_value)
                .sum();
            if !output_value.is_finite() {
                return Err(MassPropertiesError::NonFiniteValue);
            }
        }

        let mut result = Bivector::<D>::zero();
        for (index, value) in output.iter().take(dof).enumerate() {
            let (i, j) = Self::plane_pair(index)?;
            result.set(i, j, *value);
        }
        Ok(result)
    }

    /// Solve I_body * omega = angular_momentum directly in bivector
    /// coordinates without heap allocation or refactorization.
    pub fn solve_bivector(
        &self,
        angular_momentum: &Bivector<D>,
    ) -> Result<Bivector<D>, MassPropertiesError> {
        if !angular_momentum.is_finite() {
            return Err(MassPropertiesError::NonFiniteValue);
        }
        let dof = self.dof();
        if dof > MAX_ROTATIONAL_DOF {
            return Err(MassPropertiesError::InvalidDimension);
        }

        let mut rhs = [0.0_f64; MAX_ROTATIONAL_DOF];
        for (index, value) in rhs.iter_mut().take(dof).enumerate() {
            let (i, j) = Self::plane_pair(index)?;
            *value = angular_momentum.get(i, j);
        }
        let solution = self.solve_fixed(&rhs)?;
        let mut result = Bivector::<D>::zero();
        for (index, value) in solution.iter().take(dof).enumerate() {
            let (i, j) = Self::plane_pair(index)?;
            result.set(i, j, *value);
        }
        Ok(result)
    }

    /// Rotational kinetic energy for a bivector angular velocity without
    /// heap allocation. Units are joules for SI inertia and radians/second.
    pub fn kinetic_energy_bivector_joules(
        &self,
        angular_velocity: &Bivector<D>,
    ) -> Result<f64, MassPropertiesError> {
        let momentum = self.apply_bivector(angular_velocity)?;
        let mut pairing = 0.0_f64;
        for i in 0..D {
            for j in (i + 1)..D {
                pairing += angular_velocity.get(i, j) * momentum.get(i, j);
            }
        }
        let energy = 0.5 * pairing;
        if !energy.is_finite() || energy < 0.0 {
            return Err(MassPropertiesError::NonFiniteValue);
        }
        Ok(energy)
    }

    /// Solve using the cached lower Cholesky factor and fixed stack arrays.
    fn solve_fixed(
        &self,
        rhs: &[f64; MAX_ROTATIONAL_DOF],
    ) -> Result<[f64; MAX_ROTATIONAL_DOF], MassPropertiesError> {
        let dof = self.dof();
        if dof > MAX_ROTATIONAL_DOF {
            return Err(MassPropertiesError::InvalidDimension);
        }
        let mut forward = [0.0_f64; MAX_ROTATIONAL_DOF];
        let mut solution = [0.0_f64; MAX_ROTATIONAL_DOF];

        for (row, rhs_value) in rhs.iter().take(dof).enumerate() {
            let row_start = row * dof;
            let known_coefficients =
                &self.cholesky_lower_row_major[row_start..row_start + row];
            let (completed, remaining) = forward.split_at_mut(row);
            let current = remaining
                .first_mut()
                .ok_or(MassPropertiesError::InvalidDimension)?;
            let known_sum = known_coefficients
                .iter()
                .zip(completed.iter())
                .map(|(coefficient, value)| coefficient * value)
                .sum::<f64>();
            let diagonal = self.cholesky_lower_row_major[row_start + row];
            if !diagonal.is_finite() || diagonal <= f64::MIN_POSITIVE {
                return Err(MassPropertiesError::NotPositiveDefinite);
            }
            *current = (*rhs_value - known_sum) / diagonal;
        }

        for row in (0..dof).rev() {
            let (through_current, following) = solution.split_at_mut(row + 1);
            let current = through_current
                .last_mut()
                .ok_or(MassPropertiesError::InvalidDimension)?;
            let following_sum = following
                .iter()
                .enumerate()
                .map(|(offset, value)| {
                    self.cholesky_lower_row_major[(row + 1 + offset) * dof + row] * value
                })
                .sum::<f64>();
            let diagonal = self.cholesky_lower_row_major[row * dof + row];
            *current = (forward[row] - following_sum) / diagonal;
        }

        if !solution[..dof].iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::NonFiniteValue);
        }
        Ok(solution)
    }

    /// Rotational kinetic energy in joules for body-frame angular velocity
    /// coordinates in radians/second.
    pub fn kinetic_energy_joules(
        &self,
        angular_velocity: &[f64],
    ) -> Result<f64, MassPropertiesError> {
        let momentum_like = self.apply(angular_velocity)?;
        let energy = 0.5
            * angular_velocity
                .iter()
                .zip(momentum_like.iter())
                .map(|(omega, inertia_omega)| omega * inertia_omega)
                .sum::<f64>();
        if !energy.is_finite() || energy < 0.0 {
            return Err(MassPropertiesError::NonFiniteValue);
        }
        Ok(energy)
    }

    /// Map an ordered coordinate-plane pair (i,j), with i < j, to the
    /// lexicographic bivector index used by this operator.
    pub fn plane_index(i: usize, j: usize) -> Result<usize, MassPropertiesError> {
        if D < 2 || i >= j || j >= D {
            return Err(MassPropertiesError::InvalidPlaneIndex);
        }
        let twice_dimension = D
            .checked_mul(2)
            .ok_or(MassPropertiesError::InvalidDimension)?;
        let row_width = twice_dimension
            .checked_sub(i + 1)
            .ok_or(MassPropertiesError::InvalidDimension)?;
        let prefix = i
            .checked_mul(row_width)
            .ok_or(MassPropertiesError::InvalidDimension)?
            / 2;
        prefix
            .checked_add(j - i - 1)
            .ok_or(MassPropertiesError::InvalidDimension)
    }

    /// Inverse of [`plane_index`]: convert a bivector coordinate index to
    /// its ordered coordinate-plane pair (i,j).
    pub fn plane_pair(index: usize) -> Result<(usize, usize), MassPropertiesError> {
        let dof = rotational_dof_count::<D>();
        if index >= dof {
            return Err(MassPropertiesError::InvalidPlaneIndex);
        }
        let mut current = 0;
        for i in 0..D {
            for j in (i + 1)..D {
                if current == index {
                    return Ok((i, j));
                }
                current += 1;
            }
        }
        Err(MassPropertiesError::InvalidPlaneIndex)
    }
}

/// Dynamic rigid-body mass and rotational mass properties.
///
/// The inertia operator is body-frame data and must be transformed or used
/// with body-frame angular variables consistently by any integrator.
#[derive(Clone, Debug, PartialEq)]
pub struct MassProperties<const D: usize> {
    mass_kg: f64,
    center_of_mass_m: SVector<f64, D>,
    rotational_inertia: RotationalInertiaOperator<D>,
}

impl<const D: usize> MassProperties<D> {
    /// Construct validated dynamic-body mass properties.
    pub fn try_new(
        mass_kg: f64,
        center_of_mass_m: SVector<f64, D>,
        rotational_inertia: RotationalInertiaOperator<D>,
    ) -> Result<Self, MassPropertiesError> {
        if !mass_kg.is_finite() || mass_kg <= 0.0 {
            return Err(MassPropertiesError::InvalidMass);
        }
        if !center_of_mass_m.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesError::InvalidCenterOfMass);
        }
        Ok(Self {
            mass_kg,
            center_of_mass_m,
            rotational_inertia,
        })
    }

    /// Mass in kilograms.
    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    /// Body-frame center of mass in metres.
    pub fn center_of_mass_m(&self) -> &SVector<f64, D> {
        &self.center_of_mass_m
    }

    /// Validated body-frame rotational inertia operator.
    pub fn rotational_inertia(&self) -> &RotationalInertiaOperator<D> {
        &self.rotational_inertia
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_index_is_bijective_for_three_and_four_dimensions() {
        assert_eq!(RotationalInertiaOperator::<3>::plane_index(0, 1), Ok(0));
        assert_eq!(RotationalInertiaOperator::<3>::plane_index(0, 2), Ok(1));
        assert_eq!(RotationalInertiaOperator::<3>::plane_index(1, 2), Ok(2));

        let expected_4d = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        for (index, pair) in expected_4d.into_iter().enumerate() {
            assert_eq!(RotationalInertiaOperator::<4>::plane_pair(index), Ok(pair));
            assert_eq!(
                RotationalInertiaOperator::<4>::plane_index(pair.0, pair.1),
                Ok(index)
            );
        }
    }

    #[test]
    fn body_second_moment_constructs_physically_realizable_3d_operator() {
        let second_moment = vec![2.0, 0.25, 0.0, 0.25, 3.0, 0.5, 0.0, 0.5, 4.0];
        let inertia =
            RotationalInertiaOperator::<3>::from_body_second_moment_matrix(second_moment).unwrap();
        let matrix = inertia.as_row_major();

        // Plane order: (0,1), (0,2), (1,2). Diagonal moments are sums of
        // the two corresponding coordinate second moments.
        assert!((matrix[0] - 5.0).abs() < 1e-12);
        assert!((matrix[4] - 6.0).abs() < 1e-12);
        assert!((matrix[8] - 7.0).abs() < 1e-12);

        // Products of inertia are represented by the induced off-diagonal
        // coupling terms, not discarded by principal-axis averaging.
        assert!((matrix[1] - 0.5).abs() < 1e-12);
        assert!((matrix[3] - 0.5).abs() < 1e-12);
        assert!((matrix[5] - 0.25).abs() < 1e-12);
        assert!((matrix[7] - 0.25).abs() < 1e-12);
        assert!((matrix[2]).abs() < 1e-12);
        assert!((matrix[6]).abs() < 1e-12);
    }

    #[test]
    fn body_second_moment_rejects_non_symmetric_or_degenerate_mass_distribution() {
        assert_eq!(
            RotationalInertiaOperator::<3>::from_body_second_moment_matrix(vec![
                1.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            Err(MassPropertiesError::NotSymmetric)
        );
        assert_eq!(
            RotationalInertiaOperator::<3>::from_body_second_moment_matrix(vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            Err(MassPropertiesError::NotPositiveDefinite)
        );
    }

    #[test]
    fn diagonal_second_moment_maps_to_expected_four_dimensional_plane_inertias() {
        let inertia = RotationalInertiaOperator::<4>::from_body_second_moment_matrix(vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0,
        ])
        .unwrap();
        let expected = [3.0, 4.0, 5.0, 5.0, 6.0, 7.0];
        let dof = rotational_dof_count::<4>();
        for (index, expected_moment) in expected.into_iter().enumerate() {
            assert!((inertia.as_row_major()[index * dof + index] - expected_moment).abs() < 1e-12);
        }
    }

    #[test]
    fn isotropic_operator_matches_half_i_omega_squared() {
        let inertia = RotationalInertiaOperator::<3>::isotropic(2.0).unwrap();
        let omega = [1.0, -2.0, 3.0];
        let expected = 0.5 * 2.0 * (1.0_f64 + 4.0 + 9.0);
        assert!((inertia.kinetic_energy_joules(&omega).unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn anisotropic_operator_supports_products_of_inertia_and_inverse_solve() {
        let matrix = vec![4.0, 0.5, 0.0, 0.5, 3.0, 0.25, 0.0, 0.25, 2.0];
        let inertia = RotationalInertiaOperator::<3>::try_from_row_major(matrix).unwrap();
        let omega = vec![0.5, -1.0, 2.0];
        let momentum = inertia.apply(&omega).unwrap();
        let recovered = inertia.solve(&momentum).unwrap();

        for (actual, expected) in recovered.iter().zip(omega.iter()) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert!(inertia.kinetic_energy_joules(&omega).unwrap() > 0.0);
    }

    #[test]
    fn bivector_apply_and_cached_solve_match_allocating_reference() {
        let matrix = vec![4.0, 0.5, 0.0, 0.5, 3.0, 0.25, 0.0, 0.25, 2.0];
        let inertia = RotationalInertiaOperator::<3>::try_from_row_major(matrix).unwrap();
        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 0.5);
        omega.set(0, 2, -1.0);
        omega.set(1, 2, 2.0);

        let applied = inertia.apply_bivector(&omega).unwrap();
        let coordinates = [omega.get(0, 1), omega.get(0, 2), omega.get(1, 2)];
        let allocating = inertia.apply(&coordinates).unwrap();
        assert!((applied.get(0, 1) - allocating[0]).abs() < 1.0e-12);
        assert!((applied.get(0, 2) - allocating[1]).abs() < 1.0e-12);
        assert!((applied.get(1, 2) - allocating[2]).abs() < 1.0e-12);

        let solved = inertia.solve_bivector(&applied).unwrap();
        assert!((solved.get(0, 1) - omega.get(0, 1)).abs() < 1.0e-12);
        assert!((solved.get(0, 2) - omega.get(0, 2)).abs() < 1.0e-12);
        assert!((solved.get(1, 2) - omega.get(1, 2)).abs() < 1.0e-12);

        let energy = inertia.kinetic_energy_bivector_joules(&omega).unwrap();
        assert!((energy - inertia.kinetic_energy_joules(&coordinates).unwrap()).abs() < 1.0e-12);
    }

    #[test]
    fn cached_cholesky_solve_handles_four_dimensional_double_rotation() {
        let inertia = RotationalInertiaOperator::<4>::isotropic(2.5).unwrap();
        let mut momentum = Bivector::<4>::zero();
        momentum.set(0, 1, 2.5 * 0.7);
        momentum.set(2, 3, 2.5 * -1.1);
        let omega = inertia.solve_bivector(&momentum).unwrap();
        assert!((omega.get(0, 1) - 0.7).abs() < 1.0e-12);
        assert!((omega.get(2, 3) + 1.1).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_operator_matrices_are_rejected() {
        assert_eq!(
            RotationalInertiaOperator::<3>::try_from_row_major(vec![1.0, 0.0]),
            Err(MassPropertiesError::MatrixSizeMismatch {
                expected: 9,
                actual: 2
            })
        );
        assert_eq!(
            RotationalInertiaOperator::<3>::try_from_row_major(vec![
                1.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            Err(MassPropertiesError::NotSymmetric)
        );
        assert_eq!(
            RotationalInertiaOperator::<3>::try_from_row_major(vec![
                1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            Err(MassPropertiesError::NotPositiveDefinite)
        );
    }

    #[test]
    fn mass_properties_reject_non_physical_mass_and_center() {
        let inertia = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        assert!(matches!(
            MassProperties::<3>::try_new(-1.0, SVector::zeros(), inertia.clone()),
            Err(MassPropertiesError::InvalidMass)
        ));
        assert!(matches!(
            MassProperties::<3>::try_new(1.0, SVector::from([0.0, f64::NAN, 0.0]), inertia),
            Err(MassPropertiesError::InvalidCenterOfMass)
        ));
    }

    #[test]
    fn operator_rejects_non_finite_and_wrong_length_vectors() {
        let inertia = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        assert!(matches!(
            inertia.apply(&[1.0, 2.0]),
            Err(MassPropertiesError::VectorSizeMismatch { .. })
        ));
        assert_eq!(
            inertia.apply(&[1.0, f64::INFINITY, 0.0]),
            Err(MassPropertiesError::NonFiniteValue)
        );
    }
}
