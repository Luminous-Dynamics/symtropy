// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked body↔world frame transforms for rotational inertia operators.
//!
//! A spatial Rotor acts on bivectors through the adjoint representation. If
//! `A(R)` maps body-frame bivector coefficients to world-frame coefficients,
//! then a body-space inertia operator transforms as
//!
//! `I_world = A I_body A^T`.
//!
//! This reference constructs `A` directly from Symtropy's actual `Rotor<D>` and
//! `Bivector<D>` implementations instead of assuming a 3D cross-product model.

use symtropy_math::{Bivector, Rotor};

use crate::rotational_inertia_operator::{RotationalInertiaError, RotationalInertiaOperator};

const ROTATION_VALIDITY_TOLERANCE: f64 = 1.0e-8;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RotationalFrameError {
    InvalidRotation,
    NonOrthogonalAdjoint,
    UnrepresentableAdjoint,
    Inertia(RotationalInertiaError),
}

impl From<RotationalInertiaError> for RotationalFrameError {
    fn from(value: RotationalInertiaError) -> Self {
        Self::Inertia(value)
    }
}

pub trait RotationalInertiaFrameExt<const D: usize> {
    /// Build the world-frame inertia operator induced by `rotation`.
    fn world_operator_checked(
        &self,
        rotation: &Rotor<D>,
    ) -> Result<RotationalInertiaOperator<D>, RotationalFrameError>;

    /// Compute world angular momentum from world angular velocity while the
    /// stored inertia operator remains body-frame authoritative state.
    fn world_angular_momentum_checked(
        &self,
        rotation: &Rotor<D>,
        angular_velocity_world: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalFrameError>;

    /// Recover world angular velocity from world angular momentum.
    fn world_angular_velocity_from_momentum_checked(
        &self,
        rotation: &Rotor<D>,
        angular_momentum_world: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalFrameError>;

    /// Rotational kinetic energy for world angular velocity and current body
    /// orientation.
    fn world_kinetic_energy_checked(
        &self,
        rotation: &Rotor<D>,
        angular_velocity_world: &Bivector<D>,
    ) -> Result<f64, RotationalFrameError>;
}

impl<const D: usize> RotationalInertiaFrameExt<D> for RotationalInertiaOperator<D> {
    fn world_operator_checked(
        &self,
        rotation: &Rotor<D>,
    ) -> Result<RotationalInertiaOperator<D>, RotationalFrameError> {
        self.validate()?;
        let adjoint = rotation_adjoint_checked(rotation)?;
        let n = RotationalInertiaOperator::<D>::rotational_dimension();
        let body = self.coefficients();

        // temp = A * I_body
        let mut temp = vec![0.0_f64; n * n];
        for row in 0..n {
            for col in 0..n {
                let mut sum = 0.0_f64;
                for k in 0..n {
                    let product = adjoint[row * n + k] * body[k * n + col];
                    sum += product;
                    if !product.is_finite() || !sum.is_finite() {
                        return Err(RotationalFrameError::UnrepresentableAdjoint);
                    }
                }
                temp[row * n + col] = sum;
            }
        }

        // world = temp * A^T
        let mut world = vec![0.0_f64; n * n];
        for row in 0..n {
            for col in 0..n {
                let mut sum = 0.0_f64;
                for k in 0..n {
                    let product = temp[row * n + k] * adjoint[col * n + k];
                    sum += product;
                    if !product.is_finite() || !sum.is_finite() {
                        return Err(RotationalFrameError::UnrepresentableAdjoint);
                    }
                }
                world[row * n + col] = sum;
            }
        }

        // The exact transform preserves symmetry. Average mirrored entries to
        // remove only floating-point multiplication-order asymmetry before the
        // strict inertia validator consumes the result.
        for row in 0..n {
            for col in (row + 1)..n {
                let average = 0.5 * (world[row * n + col] + world[col * n + row]);
                if !average.is_finite() {
                    return Err(RotationalFrameError::UnrepresentableAdjoint);
                }
                world[row * n + col] = average;
                world[col * n + row] = average;
            }
        }

        Ok(RotationalInertiaOperator::<D>::from_dense_checked(world)?)
    }

    fn world_angular_momentum_checked(
        &self,
        rotation: &Rotor<D>,
        angular_velocity_world: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalFrameError> {
        let world_operator = self.world_operator_checked(rotation)?;
        Ok(world_operator.angular_momentum_checked(angular_velocity_world)?)
    }

    fn world_angular_velocity_from_momentum_checked(
        &self,
        rotation: &Rotor<D>,
        angular_momentum_world: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalFrameError> {
        let world_operator = self.world_operator_checked(rotation)?;
        Ok(world_operator.angular_velocity_from_momentum_checked(angular_momentum_world)?)
    }

    fn world_kinetic_energy_checked(
        &self,
        rotation: &Rotor<D>,
        angular_velocity_world: &Bivector<D>,
    ) -> Result<f64, RotationalFrameError> {
        let world_operator = self.world_operator_checked(rotation)?;
        Ok(world_operator.kinetic_energy_checked(angular_velocity_world)?)
    }
}

/// Dense row-major adjoint matrix mapping body-frame bivector coefficients to
/// world-frame bivector coefficients for the supplied Rotor.
pub fn rotation_adjoint_checked<const D: usize>(
    rotation: &Rotor<D>,
) -> Result<Vec<f64>, RotationalFrameError> {
    let rotation_matrix = rotation.to_matrix();
    if rotation_matrix.iter().any(|value| !value.is_finite())
        || !rotation.is_proper_rotation(ROTATION_VALIDITY_TOLERANCE)
    {
        return Err(RotationalFrameError::InvalidRotation);
    }

    let n = Bivector::<D>::num_components();
    let mut adjoint = vec![0.0_f64; n * n];

    for column in 0..n {
        let basis = bivector_basis_from_index::<D>(column)
            .ok_or(RotationalFrameError::UnrepresentableAdjoint)?;
        let basis_matrix = basis.to_matrix();
        let world_matrix = rotation_matrix * basis_matrix * rotation_matrix.transpose();
        if world_matrix.iter().any(|value| !value.is_finite()) {
            return Err(RotationalFrameError::UnrepresentableAdjoint);
        }
        let world_bivector = Bivector::<D>::from_matrix(&world_matrix);
        if !world_bivector.is_finite() {
            return Err(RotationalFrameError::UnrepresentableAdjoint);
        }

        let mut row = 0;
        for i in 0..D {
            for j in (i + 1)..D {
                adjoint[row * n + column] = world_bivector.get(i, j);
                row += 1;
            }
        }
    }

    validate_adjoint_orthogonality(&adjoint, n)?;
    Ok(adjoint)
}

fn bivector_basis_from_index<const D: usize>(index: usize) -> Option<Bivector<D>> {
    let mut current = 0;
    for i in 0..D {
        for j in (i + 1)..D {
            if current == index {
                return Some(Bivector::<D>::unit_plane(i, j));
            }
            current += 1;
        }
    }
    None
}

fn validate_adjoint_orthogonality(
    adjoint: &[f64],
    n: usize,
) -> Result<(), RotationalFrameError> {
    if adjoint.len() != n * n || adjoint.iter().any(|value| !value.is_finite()) {
        return Err(RotationalFrameError::UnrepresentableAdjoint);
    }

    let tolerance = 256.0 * f64::EPSILON * (n.max(1) as f64);
    for row in 0..n {
        for col in 0..n {
            let mut dot = 0.0_f64;
            for k in 0..n {
                let product = adjoint[k * n + row] * adjoint[k * n + col];
                dot += product;
                if !product.is_finite() || !dot.is_finite() {
                    return Err(RotationalFrameError::UnrepresentableAdjoint);
                }
            }
            let expected = if row == col { 1.0 } else { 0.0 };
            if (dot - expected).abs() > tolerance {
                return Err(RotationalFrameError::NonOrthogonalAdjoint);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{SMatrix, SVector};

    use crate::angular_dynamics::{
        angular_vector_to_bivector, rotational_kinetic_energy, PrincipalInertia3,
    };
    use symtropy_math::Bivector;

    #[test]
    fn identity_rotation_has_identity_adjoint() {
        let adjoint = rotation_adjoint_checked(&Rotor::<3>::identity()).unwrap();
        assert_eq!(adjoint.len(), 9);
        for row in 0..3 {
            for col in 0..3 {
                let expected = if row == col { 1.0 } else { 0.0 };
                assert!((adjoint[row * 3 + col] - expected).abs() < 1.0e-14);
            }
        }
    }

    #[test]
    fn three_dimensional_world_energy_matches_principal_inertia_reference() {
        let inertia = PrincipalInertia3::new([1.0, 4.0, 9.0]).unwrap();
        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([1.0, 4.0, 9.0]));
        let operator = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let rotation = Rotor::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            0.73,
        );
        let omega = angular_vector_to_bivector(&SVector::from([0.8, -0.4, 1.1]));

        let expected = rotational_kinetic_energy(&rotation, &omega, inertia).unwrap();
        let actual = operator.world_kinetic_energy_checked(&rotation, &omega).unwrap();
        assert!((actual - expected).abs() < 1.0e-10, "actual={actual}, expected={expected}");
    }

    #[test]
    fn world_apply_and_solve_roundtrip_in_3d() {
        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([1.0, 4.0, 9.0]));
        let operator = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let rotation = Rotor::from_plane_angle(
            &Bivector::<3>::unit_plane(1, 2),
            0.61,
        );
        let omega = angular_vector_to_bivector(&SVector::from([0.3, 0.9, -0.2]));
        let momentum = operator
            .world_angular_momentum_checked(&rotation, &omega)
            .unwrap();
        let recovered = operator
            .world_angular_velocity_from_momentum_checked(&rotation, &momentum)
            .unwrap();
        assert_eq!(omega, recovered);
    }

    #[test]
    fn four_dimensional_adjoint_preserves_six_plane_norm() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.4);
        generator.set(2, 3, -0.7);
        let rotation = Rotor::from_bivector(&generator);
        let adjoint = rotation_adjoint_checked(&rotation).unwrap();
        assert_eq!(adjoint.len(), 36);

        let diagonal = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let operator = RotationalInertiaOperator::<4>::diagonal_checked(&diagonal).unwrap();
        let mut omega = Bivector::<4>::zero();
        omega.set(0, 1, 0.2);
        omega.set(0, 2, -0.3);
        omega.set(0, 3, 0.4);
        omega.set(1, 2, -0.5);
        omega.set(1, 3, 0.6);
        omega.set(2, 3, -0.7);

        let momentum = operator
            .world_angular_momentum_checked(&rotation, &omega)
            .unwrap();
        let recovered = operator
            .world_angular_velocity_from_momentum_checked(&rotation, &momentum)
            .unwrap();
        assert_eq!(omega, recovered);
        assert!(operator.world_kinetic_energy_checked(&rotation, &omega).unwrap() > 0.0);
    }

    #[test]
    fn isotropic_operator_is_rotation_invariant() {
        let operator = RotationalInertiaOperator::<4>::diagonal_checked(&[3.0; 6]).unwrap();
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 2, 0.4);
        generator.set(1, 3, -0.5);
        let rotation = Rotor::from_bivector(&generator);
        let world = operator.world_operator_checked(&rotation).unwrap();
        for (a, b) in operator.coefficients().iter().zip(world.coefficients()) {
            assert!((a - b).abs() < 1.0e-10);
        }
    }

    #[test]
    fn improper_rotation_is_rejected() {
        let operator = RotationalInertiaOperator::<3>::diagonal_checked(&[1.0, 2.0, 3.0]).unwrap();
        let mut reflection = SMatrix::<f64, 3, 3>::identity();
        reflection[(0, 0)] = -1.0;
        let rotation = Rotor::from_matrix(reflection);
        assert_eq!(
            operator.world_operator_checked(&rotation).unwrap_err(),
            RotationalFrameError::InvalidRotation
        );
    }
}
