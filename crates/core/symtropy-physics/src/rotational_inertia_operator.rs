// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked rotational-inertia operator on bivector space.
//!
//! In D spatial dimensions, angular velocity and angular momentum live in the
//! grade-2 / rotation space with `D*(D-1)/2` independent components. A correct
//! general inertia model is therefore a positive-definite self-adjoint operator
//! on that rotational space, not merely `D` axis scalars.
//!
//! This implementation is intentionally a reference/evidence layer. It uses a
//! dense heap-backed matrix so the semantics are correct for every currently
//! supported `Bivector<D>` dimension without requiring unstable generic const
//! expressions. A later production layout may specialize 2D/3D/4D for speed.

use nalgebra::SMatrix;
use symtropy_math::Bivector;

#[derive(Clone, Debug, PartialEq)]
pub struct RotationalInertiaOperator<const D: usize> {
    /// Row-major dense matrix over lexicographically ordered bivector planes.
    coefficients: Vec<f64>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RotationalInertiaError {
    DimensionMismatch { expected: usize, actual: usize },
    NonFiniteCoefficient,
    NonSymmetricOperator,
    NonPositiveDefiniteOperator,
    NonFiniteBivector,
    UnrepresentableArithmetic,
}

impl<const D: usize> RotationalInertiaOperator<D> {
    #[inline]
    pub fn rotational_dimension() -> usize {
        Bivector::<D>::num_components()
    }

    /// Construct from a row-major dense matrix over bivector components.
    pub fn from_dense_checked(coefficients: Vec<f64>) -> Result<Self, RotationalInertiaError> {
        let n = Self::rotational_dimension();
        let expected = n
            .checked_mul(n)
            .ok_or(RotationalInertiaError::UnrepresentableArithmetic)?;
        if coefficients.len() != expected {
            return Err(RotationalInertiaError::DimensionMismatch {
                expected,
                actual: coefficients.len(),
            });
        }
        let operator = Self { coefficients };
        operator.validate()?;
        Ok(operator)
    }

    /// Construct a diagonal inertia operator in lexicographic bivector-plane
    /// order: e01, e02, ..., e12, ...
    pub fn diagonal_checked(diagonal: &[f64]) -> Result<Self, RotationalInertiaError> {
        let n = Self::rotational_dimension();
        if diagonal.len() != n {
            return Err(RotationalInertiaError::DimensionMismatch {
                expected: n,
                actual: diagonal.len(),
            });
        }
        let mut dense = vec![0.0; n * n];
        for (index, value) in diagonal.iter().copied().enumerate() {
            if !value.is_finite() || value <= 0.0 {
                return Err(RotationalInertiaError::NonPositiveDefiniteOperator);
            }
            dense[index * n + index] = value;
        }
        Self::from_dense_checked(dense)
    }

    pub fn coefficients(&self) -> &[f64] {
        &self.coefficients
    }

    /// Revalidate after serialization or external mutation of a reconstructed
    /// coefficient vector.
    pub fn validate(&self) -> Result<(), RotationalInertiaError> {
        let n = Self::rotational_dimension();
        let expected = n
            .checked_mul(n)
            .ok_or(RotationalInertiaError::UnrepresentableArithmetic)?;
        if self.coefficients.len() != expected {
            return Err(RotationalInertiaError::DimensionMismatch {
                expected,
                actual: self.coefficients.len(),
            });
        }
        if self.coefficients.iter().any(|value| !value.is_finite()) {
            return Err(RotationalInertiaError::NonFiniteCoefficient);
        }

        for row in 0..n {
            for col in (row + 1)..n {
                let a = self.coefficients[row * n + col];
                let b = self.coefficients[col * n + row];
                let scale = a.abs().max(b.abs()).max(1.0);
                let tolerance = 32.0 * f64::EPSILON * scale;
                if !tolerance.is_finite() || (a - b).abs() > tolerance {
                    return Err(RotationalInertiaError::NonSymmetricOperator);
                }
            }
        }

        self.cholesky_lower().map(|_| ())
    }

    /// Apply `L = I omega` in bivector-component coordinates.
    pub fn angular_momentum_checked(
        &self,
        angular_velocity: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        self.validate()?;
        let omega = bivector_components(angular_velocity)?;
        let momentum = self.multiply_vector_checked(&omega)?;
        components_to_bivector(&momentum)
    }

    /// Solve `omega = I^-1 L` using a checked Cholesky factorization.
    pub fn angular_velocity_from_momentum_checked(
        &self,
        angular_momentum: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        self.validate()?;
        let rhs = bivector_components(angular_momentum)?;
        let lower = self.cholesky_lower()?;
        let n = Self::rotational_dimension();

        let mut y = vec![0.0_f64; n];
        for row in 0..n {
            let mut value = rhs[row];
            for col in 0..row {
                let product = lower[row * n + col] * y[col];
                value -= product;
                if !value.is_finite() || !product.is_finite() {
                    return Err(RotationalInertiaError::UnrepresentableArithmetic);
                }
            }
            let diagonal = lower[row * n + row];
            let solved = value / diagonal;
            if !solved.is_finite() {
                return Err(RotationalInertiaError::UnrepresentableArithmetic);
            }
            y[row] = solved;
        }

        let mut x = vec![0.0_f64; n];
        for row in (0..n).rev() {
            let mut value = y[row];
            for col in (row + 1)..n {
                let product = lower[col * n + row] * x[col];
                value -= product;
                if !value.is_finite() || !product.is_finite() {
                    return Err(RotationalInertiaError::UnrepresentableArithmetic);
                }
            }
            let diagonal = lower[row * n + row];
            let solved = value / diagonal;
            if !solved.is_finite() {
                return Err(RotationalInertiaError::UnrepresentableArithmetic);
            }
            x[row] = solved;
        }

        components_to_bivector(&x)
    }

    /// Rotational kinetic energy `0.5 * omega^T I omega` in bivector space.
    pub fn kinetic_energy_checked(
        &self,
        angular_velocity: &Bivector<D>,
    ) -> Result<f64, RotationalInertiaError> {
        self.validate()?;
        let omega = bivector_components(angular_velocity)?;
        let momentum = self.multiply_vector_checked(&omega)?;
        let mut quadratic = 0.0_f64;
        for (omega_component, momentum_component) in omega.iter().zip(momentum.iter()) {
            let term = omega_component * momentum_component;
            quadratic += term;
            if !term.is_finite() || !quadratic.is_finite() {
                return Err(RotationalInertiaError::UnrepresentableArithmetic);
            }
        }
        let energy = 0.5 * quadratic;
        if !energy.is_finite() || energy < 0.0 {
            return Err(RotationalInertiaError::UnrepresentableArithmetic);
        }
        Ok(energy)
    }

    fn multiply_vector_checked(&self, vector: &[f64]) -> Result<Vec<f64>, RotationalInertiaError> {
        let n = Self::rotational_dimension();
        if vector.len() != n {
            return Err(RotationalInertiaError::DimensionMismatch {
                expected: n,
                actual: vector.len(),
            });
        }
        let mut result = vec![0.0_f64; n];
        for row in 0..n {
            let mut sum = 0.0_f64;
            for col in 0..n {
                let product = self.coefficients[row * n + col] * vector[col];
                sum += product;
                if !product.is_finite() || !sum.is_finite() {
                    return Err(RotationalInertiaError::UnrepresentableArithmetic);
                }
            }
            result[row] = sum;
        }
        Ok(result)
    }

    /// Dense Cholesky factorization `I = L L^T` with checked arithmetic.
    fn cholesky_lower(&self) -> Result<Vec<f64>, RotationalInertiaError> {
        let n = Self::rotational_dimension();
        let mut lower = vec![0.0_f64; n * n];

        for row in 0..n {
            for col in 0..=row {
                let mut value = self.coefficients[row * n + col];
                for k in 0..col {
                    let product = lower[row * n + k] * lower[col * n + k];
                    value -= product;
                    if !product.is_finite() || !value.is_finite() {
                        return Err(RotationalInertiaError::UnrepresentableArithmetic);
                    }
                }

                if row == col {
                    if value <= 0.0 {
                        return Err(RotationalInertiaError::NonPositiveDefiniteOperator);
                    }
                    let diagonal = value.sqrt();
                    if !diagonal.is_finite() || diagonal <= 0.0 {
                        return Err(RotationalInertiaError::NonPositiveDefiniteOperator);
                    }
                    lower[row * n + col] = diagonal;
                } else {
                    let denominator = lower[col * n + col];
                    let factor = value / denominator;
                    if !factor.is_finite() {
                        return Err(RotationalInertiaError::UnrepresentableArithmetic);
                    }
                    lower[row * n + col] = factor;
                }
            }
        }
        Ok(lower)
    }
}

impl RotationalInertiaOperator<3> {
    /// Convert a symmetric positive-definite 3D body-frame inertia tensor into
    /// the equivalent operator over Symtropy's bivector coefficient order
    /// `[e01, e02, e12]`.
    ///
    /// Symtropy's physical angular vector convention is
    /// `omega = [b12, -b02, b01]`. The signed permutation is applied on both
    /// sides so `q^T I_bivector q == omega^T I_vector omega`.
    pub fn from_body_tensor_3d_checked(
        tensor: &SMatrix<f64, 3, 3>,
    ) -> Result<Self, RotationalInertiaError> {
        if tensor.iter().any(|value| !value.is_finite()) {
            return Err(RotationalInertiaError::NonFiniteCoefficient);
        }

        // q=[b01,b02,b12] -> omega=[q2,-q1,q0]. Each tuple is
        // (physical-vector axis, sign) for one bivector coefficient.
        let map = [(2_usize, 1.0_f64), (1, -1.0), (0, 1.0)];
        let mut dense = vec![0.0_f64; 9];
        for row in 0..3 {
            for col in 0..3 {
                let (physical_row, row_sign) = map[row];
                let (physical_col, col_sign) = map[col];
                let value = row_sign * tensor[(physical_row, physical_col)] * col_sign;
                if !value.is_finite() {
                    return Err(RotationalInertiaError::UnrepresentableArithmetic);
                }
                dense[row * 3 + col] = value;
            }
        }
        Self::from_dense_checked(dense)
    }
}

fn bivector_components<const D: usize>(
    bivector: &Bivector<D>,
) -> Result<Vec<f64>, RotationalInertiaError> {
    if !bivector.is_finite() {
        return Err(RotationalInertiaError::NonFiniteBivector);
    }
    let mut components = Vec::with_capacity(Bivector::<D>::num_components());
    for i in 0..D {
        for j in (i + 1)..D {
            components.push(bivector.get(i, j));
        }
    }
    Ok(components)
}

fn components_to_bivector<const D: usize>(
    components: &[f64],
) -> Result<Bivector<D>, RotationalInertiaError> {
    let expected = Bivector::<D>::num_components();
    if components.len() != expected {
        return Err(RotationalInertiaError::DimensionMismatch {
            expected,
            actual: components.len(),
        });
    }
    if components.iter().any(|value| !value.is_finite()) {
        return Err(RotationalInertiaError::UnrepresentableArithmetic);
    }

    let mut result = Bivector::<D>::zero();
    let mut index = 0;
    for i in 0..D {
        for j in (i + 1)..D {
            result.set(i, j, components[index]);
            index += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SVector;

    use crate::angular_dynamics::{angular_vector_to_bivector, bivector_to_angular_vector};
    use crate::compound_inertia_3d::{CompoundMassPart3, CompoundMassProperties3};
    use crate::mass_properties_3d::MassProperties3;
    use symtropy_math::{Bivector, Point, Rotor, Transform};

    #[test]
    fn three_dimensional_tensor_mapping_preserves_energy() {
        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([1.0, 4.0, 9.0]));
        let operator = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let omega = SVector::from([1.0, 2.0, 3.0]);
        let bivector = angular_vector_to_bivector(&omega);
        let energy = operator.kinetic_energy_checked(&bivector).unwrap();
        assert!((energy - 49.0).abs() < 1.0e-12);
    }

    #[test]
    fn apply_and_solve_roundtrip_in_3d() {
        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([1.0, 4.0, 9.0]));
        let operator = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let omega = angular_vector_to_bivector(&SVector::from([0.3, -0.7, 1.2]));
        let momentum = operator.angular_momentum_checked(&omega).unwrap();
        let recovered = operator
            .angular_velocity_from_momentum_checked(&momentum)
            .unwrap();
        let error = (bivector_to_angular_vector(&recovered)
            - bivector_to_angular_vector(&omega))
        .norm();
        assert!(error < 1.0e-12, "3D operator solve roundtrip error={error:e}");
    }

    #[test]
    fn compound_full_tensor_maps_without_losing_cross_terms() {
        let cuboid = MassProperties3::solid_cuboid(12.0, [1.0, 2.0, 3.0]).unwrap();
        let rotation = Rotor::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            std::f64::consts::FRAC_PI_4,
        );
        let part = CompoundMassPart3 {
            part_id: 1,
            local_transform: Transform {
                translation: Point::origin(),
                rotation,
            },
            properties: cuboid,
        };
        let compound = CompoundMassProperties3::compose(&[part]).unwrap();
        let operator = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(
            &compound.inertia_tensor_body,
        )
        .unwrap();
        let omega_vector = SVector::from([1.0, 1.0, 0.0]);
        let omega_bivector = angular_vector_to_bivector(&omega_vector);
        let tensor_energy = compound.rotational_energy_body_checked(&omega_vector).unwrap();
        let operator_energy = operator.kinetic_energy_checked(&omega_bivector).unwrap();
        assert!((tensor_energy - operator_energy).abs() < 1.0e-12);
    }

    #[test]
    fn four_dimensions_use_all_six_rotation_planes() {
        assert_eq!(RotationalInertiaOperator::<4>::rotational_dimension(), 6);
        let diagonal = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let operator = RotationalInertiaOperator::<4>::diagonal_checked(&diagonal).unwrap();
        let mut omega = Bivector::<4>::zero();
        omega.set(0, 1, 0.2);
        omega.set(0, 2, -0.3);
        omega.set(0, 3, 0.4);
        omega.set(1, 2, -0.5);
        omega.set(1, 3, 0.6);
        omega.set(2, 3, -0.7);

        let momentum = operator.angular_momentum_checked(&omega).unwrap();
        let recovered = operator
            .angular_velocity_from_momentum_checked(&momentum)
            .unwrap();
        assert_eq!(omega, recovered);
        assert!(operator.kinetic_energy_checked(&omega).unwrap() > 0.0);
    }

    #[test]
    fn wrong_dense_or_diagonal_dimension_is_rejected() {
        assert_eq!(
            RotationalInertiaOperator::<4>::diagonal_checked(&[1.0, 2.0, 3.0])
                .unwrap_err(),
            RotationalInertiaError::DimensionMismatch {
                expected: 6,
                actual: 3,
            }
        );
        assert_eq!(
            RotationalInertiaOperator::<3>::from_dense_checked(vec![1.0; 8]).unwrap_err(),
            RotationalInertiaError::DimensionMismatch {
                expected: 9,
                actual: 8,
            }
        );
    }

    #[test]
    fn non_symmetric_or_indefinite_operator_is_rejected() {
        let nonsymmetric = vec![1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        assert_eq!(
            RotationalInertiaOperator::<3>::from_dense_checked(nonsymmetric).unwrap_err(),
            RotationalInertiaError::NonSymmetricOperator
        );

        let indefinite = vec![1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0];
        assert_eq!(
            RotationalInertiaOperator::<3>::from_dense_checked(indefinite).unwrap_err(),
            RotationalInertiaError::NonPositiveDefiniteOperator
        );
    }

    #[test]
    fn non_finite_angular_state_fails_closed() {
        let operator = RotationalInertiaOperator::<3>::diagonal_checked(&[1.0, 2.0, 3.0]).unwrap();
        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, f64::NAN);
        assert_eq!(
            operator.kinetic_energy_checked(&omega),
            Err(RotationalInertiaError::NonFiniteBivector)
        );
    }
}
