// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Kinematics for time-dependent full-rank periodic simulation cells.
//!
//! This module separates material/fractional coordinates from the affine cell
//! frame. It does not integrate particle state, implement a barostat, or claim
//! conservation of energy during cell deformation.
//!
//! For fractional position s and its physical-time derivative s_dot:
//!   x = o + B*s
//!   v = o_dot + B_dot*s + B*s_dot
//!
//! The B_dot*s term is the affine velocity induced by a changing cell basis.
//! Omitting it creates a velocity discontinuity under shear or strain.

use std::error::Error;
use std::fmt;

use nalgebra::{SMatrix, SVector};

const MAX_CONDITION_PROXY: f64 = 1.0e8;
const MAX_INVERSE_RESIDUAL: f64 = 1.0e-8;
const SAMPLE_TIME_REL_TOLERANCE: f64 = 1.0e-12;

/// Failure to construct or use a periodic cell kinematics frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicCellKinematicsError {
    InvalidDimension,
    UnsupportedDimension { dimension: usize },
    NonFiniteFrameField { field: &'static str, axis: Option<usize> },
    SingularBasis,
    LeftHandedBasis,
    IllConditionedBasis,
    InverseResidualExceeded,
    IncompatiblePeriodicMask,
    IncompatibleSampleTime,
    NonFiniteState { field: &'static str, axis: Option<usize> },
    NumericRange { field: &'static str },
}

impl fmt::Display for PeriodicCellKinematicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "periodic cell kinematics requires D >= 1"),
            Self::UnsupportedDimension { dimension } => {
                write!(f, "periodic cell kinematics supports D=2 or D=3, got D={dimension}")
            }
            Self::NonFiniteFrameField { field, axis } => {
                write!(f, "non-finite periodic cell frame field {field}")?;
                if let Some(axis) = axis {
                    write!(f, " on axis {axis}")?;
                }
                Ok(())
            }
            Self::SingularBasis => write!(f, "periodic cell basis is singular"),
            Self::LeftHandedBasis => write!(f, "periodic cell basis must have positive orientation"),
            Self::IllConditionedBasis => {
                write!(f, "periodic cell basis exceeds the supported condition-proxy envelope")
            }
            Self::InverseResidualExceeded => {
                write!(f, "periodic cell inverse failed its reconstruction residual check")
            }
            Self::IncompatiblePeriodicMask => {
                write!(f, "periodic remap cannot change the active periodic subspace")
            }
            Self::IncompatibleSampleTime => {
                write!(f, "affine cell remap requires frames at the same physical sample time")
            }
            Self::NonFiniteState { field, axis } => {
                write!(f, "non-finite state field {field}")?;
                if let Some(axis) = axis {
                    write!(f, " on axis {axis}")?;
                }
                Ok(())
            }
            Self::NumericRange { field } => {
                write!(f, "periodic cell kinematics numeric-range failure in {field}")
            }
        }
    }
}

impl Error for PeriodicCellKinematicsError {}

/// A particle/material state expressed in a cell's fractional coordinates.
///
/// fractional_velocity is ds/dt in physical seconds. It is not Cartesian
/// velocity and is not a dimensionless frame-to-frame displacement.
#[derive(Clone, Debug, PartialEq)]
pub struct FractionalCellState<const D: usize> {
    pub fractional_position: SVector<f64, D>,
    pub fractional_velocity: SVector<f64, D>,
}

/// A particle state expressed in Cartesian coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct CartesianCellState<const D: usize> {
    pub position: SVector<f64, D>,
    pub velocity: SVector<f64, D>,
}

/// Evidence returned by an instantaneous affine cell remap.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicCellRemap<const D: usize> {
    pub before: CartesianCellState<D>,
    pub after: CartesianCellState<D>,
    pub fractional_state: FractionalCellState<D>,
    /// F = B_after * inverse(B_before).
    pub deformation_gradient: SMatrix<f64, D, D>,
    /// det(B_after) / det(B_before), positive for orientation-preserving cells.
    pub volume_ratio: f64,
    /// Velocity change induced by the new cell frame while fractional state
    /// (position and physical-time derivative) is held fixed.
    pub affine_velocity_delta: SVector<f64, D>,
    /// Cartesian position change introduced by the declared remap.
    pub affine_position_delta: SVector<f64, D>,
}

/// One sample of an evolving periodic cell.
///
/// basis stores lattice vectors in its columns. basis_rate is dB/dt in
/// Cartesian units per physical second. The periodic mask is expressed in
/// fractional basis coordinates and is immutable across an affine remap.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicCellKinematics<const D: usize> {
    sample_time: f64,
    origin: SVector<f64, D>,
    origin_velocity: SVector<f64, D>,
    basis: SMatrix<f64, D, D>,
    basis_rate: SMatrix<f64, D, D>,
    periodic_axes: [bool; D],
    inverse_basis: SMatrix<f64, D, D>,
    determinant: f64,
    condition_proxy: f64,
}

impl<const D: usize> PeriodicCellKinematics<D> {
    /// Construct and validate one time sample of a moving cell.
    pub fn new(
        sample_time: f64,
        origin: SVector<f64, D>,
        origin_velocity: SVector<f64, D>,
        basis: SMatrix<f64, D, D>,
        basis_rate: SMatrix<f64, D, D>,
        periodic_axes: [bool; D],
    ) -> Result<Self, PeriodicCellKinematicsError> {
        if D == 0 {
            return Err(PeriodicCellKinematicsError::InvalidDimension);
        }
        if D != 2 && D != 3 {
            return Err(PeriodicCellKinematicsError::UnsupportedDimension { dimension: D });
        }
        if !sample_time.is_finite() {
            return Err(PeriodicCellKinematicsError::NonFiniteFrameField {
                field: "sample_time",
                axis: None,
            });
        }
        for axis in 0..D {
            if !origin[axis].is_finite() {
                return Err(PeriodicCellKinematicsError::NonFiniteFrameField {
                    field: "origin",
                    axis: Some(axis),
                });
            }
            if !origin_velocity[axis].is_finite() {
                return Err(PeriodicCellKinematicsError::NonFiniteFrameField {
                    field: "origin_velocity",
                    axis: Some(axis),
                });
            }
        }
        for row in 0..D {
            for column in 0..D {
                if !basis[(row, column)].is_finite() {
                    return Err(PeriodicCellKinematicsError::NonFiniteFrameField {
                        field: "basis",
                        axis: Some(row),
                    });
                }
                if !basis_rate[(row, column)].is_finite() {
                    return Err(PeriodicCellKinematicsError::NonFiniteFrameField {
                        field: "basis_rate",
                        axis: Some(row),
                    });
                }
            }
        }

        let determinant = basis.determinant();
        if !determinant.is_finite() {
            return Err(PeriodicCellKinematicsError::NumericRange {
                field: "basis determinant",
            });
        }
        if determinant == 0.0 {
            return Err(PeriodicCellKinematicsError::SingularBasis);
        }
        if determinant < 0.0 {
            return Err(PeriodicCellKinematicsError::LeftHandedBasis);
        }

        let inverse_basis = basis
            .try_inverse()
            .ok_or(PeriodicCellKinematicsError::SingularBasis)?;
        if !inverse_basis.iter().all(|value| value.is_finite()) {
            return Err(PeriodicCellKinematicsError::NumericRange {
                field: "inverse basis",
            });
        }

        // Frobenius condition proxy. This is an explicit conservative envelope,
        // not a spectral condition number or a proof of arbitrary precision.
        let condition_proxy = basis.norm() * inverse_basis.norm();
        if !condition_proxy.is_finite() {
            return Err(PeriodicCellKinematicsError::NumericRange {
                field: "basis condition proxy",
            });
        }
        if condition_proxy > MAX_CONDITION_PROXY {
            return Err(PeriodicCellKinematicsError::IllConditionedBasis);
        }

        let inverse_residual =
            (basis * inverse_basis - SMatrix::<f64, D, D>::identity()).norm();
        if !inverse_residual.is_finite() {
            return Err(PeriodicCellKinematicsError::NumericRange {
                field: "inverse reconstruction residual",
            });
        }
        if inverse_residual > MAX_INVERSE_RESIDUAL {
            return Err(PeriodicCellKinematicsError::InverseResidualExceeded);
        }

        Ok(Self {
            sample_time,
            origin,
            origin_velocity,
            basis,
            basis_rate,
            periodic_axes,
            inverse_basis,
            determinant,
            condition_proxy,
        })
    }

    pub fn sample_time(&self) -> f64 {
        self.sample_time
    }

    pub fn origin(&self) -> &SVector<f64, D> {
        &self.origin
    }

    pub fn origin_velocity(&self) -> &SVector<f64, D> {
        &self.origin_velocity
    }

    pub fn basis(&self) -> &SMatrix<f64, D, D> {
        &self.basis
    }

    pub fn basis_rate(&self) -> &SMatrix<f64, D, D> {
        &self.basis_rate
    }

    pub fn periodic_axes(&self) -> &[bool; D] {
        &self.periodic_axes
    }

    pub fn determinant(&self) -> f64 {
        self.determinant
    }

    pub fn condition_proxy(&self) -> f64 {
        self.condition_proxy
    }

    /// Map material/fractional position and physical-time rate to Cartesian x,v.
    ///
    /// Cartesian velocity includes origin velocity and the affine basis-rate
    /// contribution basis_rate * fractional_position.
    pub fn to_cartesian(
        &self,
        fractional: &FractionalCellState<D>,
    ) -> Result<CartesianCellState<D>, PeriodicCellKinematicsError> {
        validate_state_vector(&fractional.fractional_position, "fractional_position")?;
        validate_state_vector(&fractional.fractional_velocity, "fractional_velocity")?;

        let position = self.origin + self.basis * fractional.fractional_position;
        let velocity = self.origin_velocity
            + self.basis_rate * fractional.fractional_position
            + self.basis * fractional.fractional_velocity;
        validate_state_vector(&position, "mapped Cartesian position")?;
        validate_state_vector(&velocity, "mapped Cartesian velocity")?;

        Ok(CartesianCellState { position, velocity })
    }

    /// Recover fractional/material position s and physical-time rate ds/dt.
    ///
    /// The inverse velocity formula removes origin and affine basis velocity
    /// before applying inverse(B).
    pub fn to_fractional(
        &self,
        cartesian: &CartesianCellState<D>,
    ) -> Result<FractionalCellState<D>, PeriodicCellKinematicsError> {
        validate_state_vector(&cartesian.position, "Cartesian position")?;
        validate_state_vector(&cartesian.velocity, "Cartesian velocity")?;

        let fractional_position = self.inverse_basis * (cartesian.position - self.origin);
        validate_state_vector(&fractional_position, "fractional position")?;

        let relative_velocity = cartesian.velocity
            - self.origin_velocity
            - self.basis_rate * fractional_position;
        let fractional_velocity = self.inverse_basis * relative_velocity;
        validate_state_vector(&fractional_velocity, "fractional velocity")?;

        Ok(FractionalCellState {
            fractional_position,
            fractional_velocity,
        })
    }

    /// Express the same fractional/material state in a new cell frame.
    ///
    /// This is an instantaneous affine remap, not time integration. Both frames
    /// must refer to the same physical sample time and the same active periodic
    /// subspace. Evolution of fractional coordinates between different times
    /// is the responsibility of the caller/integrator.
    pub fn affine_remap_to(
        &self,
        next: &Self,
        cartesian: &CartesianCellState<D>,
    ) -> Result<PeriodicCellRemap<D>, PeriodicCellKinematicsError> {
        if self.periodic_axes != next.periodic_axes {
            return Err(PeriodicCellKinematicsError::IncompatiblePeriodicMask);
        }
        let time_scale = self.sample_time.abs().max(next.sample_time.abs()).max(1.0);
        if (self.sample_time - next.sample_time).abs()
            > SAMPLE_TIME_REL_TOLERANCE * time_scale
        {
            return Err(PeriodicCellKinematicsError::IncompatibleSampleTime);
        }

        let fractional_state = self.to_fractional(cartesian)?;
        let after = next.to_cartesian(&fractional_state)?;

        let deformation_gradient = next.basis * self.inverse_basis;
        if !deformation_gradient.iter().all(|value| value.is_finite()) {
            return Err(PeriodicCellKinematicsError::NumericRange {
                field: "deformation gradient",
            });
        }
        let volume_ratio = next.determinant / self.determinant;
        if !volume_ratio.is_finite() || volume_ratio <= 0.0 {
            return Err(PeriodicCellKinematicsError::NumericRange {
                field: "cell volume ratio",
            });
        }

        let affine_position_delta = after.position - cartesian.position;
        let affine_velocity_delta = after.velocity - cartesian.velocity;
        validate_state_vector(&affine_position_delta, "affine position delta")?;
        validate_state_vector(&affine_velocity_delta, "affine velocity delta")?;

        Ok(PeriodicCellRemap {
            before: cartesian.clone(),
            after,
            fractional_state,
            deformation_gradient,
            volume_ratio,
            affine_velocity_delta,
            affine_position_delta,
        })
    }
}

fn validate_state_vector<const D: usize>(
    vector: &SVector<f64, D>,
    field: &'static str,
) -> Result<(), PeriodicCellKinematicsError> {
    for axis in 0..D {
        if !vector[axis].is_finite() {
            return Err(PeriodicCellKinematicsError::NonFiniteState {
                field,
                axis: Some(axis),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CartesianCellState, FractionalCellState, PeriodicCellKinematics,
        PeriodicCellKinematicsError,
    };
    use nalgebra::{SMatrix, SVector};

    fn frame2(
        basis: SMatrix<f64, 2, 2>,
        basis_rate: SMatrix<f64, 2, 2>,
        origin: [f64; 2],
        origin_velocity: [f64; 2],
        periodic_axes: [bool; 2],
    ) -> PeriodicCellKinematics<2> {
        PeriodicCellKinematics::new(
            7.5,
            SVector::from(origin),
            SVector::from(origin_velocity),
            basis,
            basis_rate,
            periodic_axes,
        )
        .unwrap()
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.15e} ~= {b:.15e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn identity_frame_roundtrips_cartesian_and_fractional_state() {
        let frame = frame2(
            SMatrix::identity(),
            SMatrix::zeros(),
            [0.0, 0.0],
            [0.0, 0.0],
            [true, true],
        );
        let fractional = FractionalCellState {
            fractional_position: SVector::from([0.25, -0.5]),
            fractional_velocity: SVector::from([1.5, -2.0]),
        };

        let cartesian = frame.to_cartesian(&fractional).unwrap();
        assert_eq!(cartesian.position, fractional.fractional_position);
        assert_eq!(cartesian.velocity, fractional.fractional_velocity);
        assert_eq!(frame.to_fractional(&cartesian).unwrap(), fractional);
    }

    #[test]
    fn changing_basis_rate_contributes_to_cartesian_velocity() {
        let old = frame2(
            SMatrix::identity(),
            SMatrix::zeros(),
            [0.0, 0.0],
            [0.0, 0.0],
            [true, true],
        );
        let next_basis = SMatrix::<f64, 2, 2>::from_row_slice(&[
            1.0, 0.5,
            0.0, 1.0,
        ]);
        let next_basis_rate = SMatrix::<f64, 2, 2>::from_row_slice(&[
            0.0, 0.2,
            0.0, 0.0,
        ]);
        let next = frame2(
            next_basis,
            next_basis_rate,
            [0.0, 0.0],
            [0.0, 0.0],
            [true, true],
        );
        let fractional = FractionalCellState {
            fractional_position: SVector::from([0.4, 0.2]),
            fractional_velocity: SVector::from([0.1, -0.2]),
        };
        let before = old.to_cartesian(&fractional).unwrap();
        let remap = old.affine_remap_to(&next, &before).unwrap();

        close(remap.after.position[0], 0.5, 1e-12);
        close(remap.after.position[1], 0.2, 1e-12);
        // B_dot*s = [0.04, 0], and B*s_dot = [0.0, -0.2].
        close(remap.after.velocity[0], 0.04, 1e-12);
        close(remap.after.velocity[1], -0.2, 1e-12);
        close(remap.affine_velocity_delta[0], -0.06, 1e-12);
        close(remap.affine_velocity_delta[1], 0.0, 1e-12);
    }

    #[test]
    fn affine_remap_reports_deformation_gradient_and_volume_ratio() {
        let old = frame2(
            SMatrix::identity(),
            SMatrix::zeros(),
            [0.0, 0.0],
            [0.0, 0.0],
            [true, true],
        );
        let next_basis =
            SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([2.0, 1.5]));
        let next = frame2(
            next_basis,
            SMatrix::zeros(),
            [0.0, 0.0],
            [0.0, 0.0],
            [true, true],
        );
        let before = CartesianCellState {
            position: SVector::from([0.25, 0.5]),
            velocity: SVector::from([1.0, -2.0]),
        };

        let remap = old.affine_remap_to(&next, &before).unwrap();
        close(remap.volume_ratio, 3.0, 1e-12);
        assert!((remap.deformation_gradient - next_basis).norm() < 1e-12);
        close(remap.after.position[0], 0.5, 1e-12);
        close(remap.after.position[1], 0.75, 1e-12);
        close(remap.after.velocity[0], 2.0, 1e-12);
        close(remap.after.velocity[1], -3.0, 1e-12);
    }

    #[test]
    fn old_new_old_affine_remap_roundtrips_material_state() {
        let old_basis =
            SMatrix::<f64, 2, 2>::from_row_slice(&[2.0, 0.3, 0.0, 1.5]);
        let old_rate =
            SMatrix::<f64, 2, 2>::from_row_slice(&[0.01, 0.02, 0.0, -0.03]);
        let old = frame2(
            old_basis,
            old_rate,
            [3.0, -2.0],
            [0.2, -0.1],
            [true, false],
        );

        let new_basis =
            SMatrix::<f64, 2, 2>::from_row_slice(&[2.1, 0.5, 0.0, 1.4]);
        let new_rate =
            SMatrix::<f64, 2, 2>::from_row_slice(&[0.02, 0.01, 0.0, -0.01]);
        let next = frame2(
            new_basis,
            new_rate,
            [3.1, -1.9],
            [0.25, -0.05],
            [true, false],
        );

        let fractional = FractionalCellState {
            fractional_position: SVector::from([0.3, -0.2]),
            fractional_velocity: SVector::from([0.7, -0.4]),
        };
        let original = old.to_cartesian(&fractional).unwrap();
        let forward = old.affine_remap_to(&next, &original).unwrap();
        let backward = next.affine_remap_to(&old, &forward.after).unwrap();

        assert!((backward.after.position - original.position).norm() < 1e-10);
        assert!((backward.after.velocity - original.velocity).norm() < 1e-10);
        assert_eq!(backward.fractional_state, fractional);
    }

    #[test]
    fn rejects_sample_time_and_periodic_mask_changes() {
        let old = frame2(
            SMatrix::identity(),
            SMatrix::zeros(),
            [0.0, 0.0],
            [0.0, 0.0],
            [true, true],
        );
        let state = CartesianCellState {
            position: SVector::from([0.1, 0.2]),
            velocity: SVector::from([0.0, 0.0]),
        };
        let different_mask = frame2(
            SMatrix::identity(),
            SMatrix::zeros(),
            [0.0, 0.0],
            [0.0, 0.0],
            [true, false],
        );
        assert_eq!(
            old.affine_remap_to(&different_mask, &state),
            Err(PeriodicCellKinematicsError::IncompatiblePeriodicMask)
        );

        let different_time = PeriodicCellKinematics::new(
            7.6,
            SVector::zeros(),
            SVector::zeros(),
            SMatrix::identity(),
            SMatrix::zeros(),
            [true, true],
        )
        .unwrap();
        assert_eq!(
            old.affine_remap_to(&different_time, &state),
            Err(PeriodicCellKinematicsError::IncompatibleSampleTime)
        );
    }

    #[test]
    fn rejects_singular_left_handed_ill_conditioned_and_non_finite_bases() {
        let origin = SVector::<f64, 2>::zeros();
        let velocity = SVector::<f64, 2>::zeros();
        let rate = SMatrix::<f64, 2, 2>::zeros();

        let singular = SMatrix::<f64, 2, 2>::from_row_slice(&[1.0, 2.0, 2.0, 4.0]);
        assert!(matches!(
            PeriodicCellKinematics::new(0.0, origin, velocity, singular, rate, [true, true]),
            Err(PeriodicCellKinematicsError::SingularBasis)
        ));

        let left_handed = SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([-1.0, 1.0]));
        assert!(matches!(
            PeriodicCellKinematics::new(0.0, origin, velocity, left_handed, rate, [true, true]),
            Err(PeriodicCellKinematicsError::LeftHandedBasis)
        ));

        let ill_conditioned =
            SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([1.0, 1.0e-10]));
        assert!(matches!(
            PeriodicCellKinematics::new(0.0, origin, velocity, ill_conditioned, rate, [true, true]),
            Err(PeriodicCellKinematicsError::IllConditionedBasis)
        ));

        let non_finite =
            SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([f64::NAN, 1.0]));
        assert!(matches!(
            PeriodicCellKinematics::new(0.0, origin, velocity, non_finite, rate, [true, true]),
            Err(PeriodicCellKinematicsError::NonFiniteFrameField { field: "basis", .. })
        ));
    }

    #[test]
    fn three_dimensional_skew_cell_roundtrips_fractional_kinematics() {
        let basis = SMatrix::<f64, 3, 3>::from_row_slice(&[
            2.0, 0.4, 0.1,
            0.0, 1.5, 0.3,
            0.0, 0.0, 1.2,
        ]);
        let rate = SMatrix::<f64, 3, 3>::from_row_slice(&[
            0.01, 0.02, 0.0,
            0.0, -0.01, 0.01,
            0.0, 0.0, 0.02,
        ]);
        let frame = PeriodicCellKinematics::new(
            1.25,
            SVector::from([2.0, -1.0, 3.0]),
            SVector::from([0.1, 0.2, -0.1]),
            basis,
            rate,
            [true, true, true],
        )
        .unwrap();
        let fractional = FractionalCellState {
            fractional_position: SVector::from([0.2, 0.4, -0.3]),
            fractional_velocity: SVector::from([1.0, -0.5, 0.25]),
        };
        let cartesian = frame.to_cartesian(&fractional).unwrap();
        let recovered = frame.to_fractional(&cartesian).unwrap();
        assert!((recovered.fractional_position - fractional.fractional_position).norm() < 1e-12);
        assert!((recovered.fractional_velocity - fractional.fractional_velocity).norm() < 1e-12);
    }
}
