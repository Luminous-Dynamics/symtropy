// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Experimental discrete-gradient reference for quadratic port-Hamiltonian systems.
//!
//! This is a research-grade energy-balance oracle for smooth finite-dimensional
//! systems. It does not solve rigid-body contact, complementarity, or joints,
//! and it is not wired into PhysicsWorld's real-time step path.

use nalgebra::{DMatrix, DVector};

/// Invalid quadratic port-Hamiltonian structure or failed implicit solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortHamiltonianError {
    InvalidDimensions,
    InvalidTolerance,
    InvalidTimeStep,
    NonFiniteInput,
    NonSymmetricEnergyMatrix,
    EnergyMatrixNotPositiveSemidefinite,
    NonSkewInterconnection,
    NonSymmetricDissipation,
    DissipationNotPositiveSemidefinite,
    SingularImplicitStep,
    NonFiniteResult,
    NegativeDissipation,
}

/// A validated quadratic port-Hamiltonian system.
///
/// State energy is H(x) = 0.5 * xᵀ Q x, with Q symmetric positive
/// semidefinite. The dynamics are
///
///     x_dot = (J - R) * ∇H(x) + G * u
///
/// where J is skew-symmetric, R is symmetric positive semidefinite, G maps
/// external port inputs into the state derivative, and y = Gᵀ∇H is the
/// conjugate output. The continuous power balance is
///
///     H_dot = -∇Hᵀ R ∇H + yᵀ u.
///
/// The discrete step below uses the exact discrete gradient of this quadratic
/// Hamiltonian (the midpoint gradient), so the same balance holds for the
/// implicit step up to the linear-solve residual.
#[derive(Clone, Debug)]
pub struct QuadraticPortHamiltonian {
    energy_matrix: DMatrix<f64>,
    interconnection: DMatrix<f64>,
    dissipation: DMatrix<f64>,
    input_matrix: DMatrix<f64>,
    relative_tolerance: f64,
}

/// Signed power/energy evidence for one accepted discrete-gradient step.
#[derive(Clone, Debug)]
pub struct PortHamiltonianStepReceipt {
    pub state_before: DVector<f64>,
    pub state_after: DVector<f64>,
    pub energy_before: f64,
    pub energy_after: f64,
    pub supplied_work: f64,
    pub dissipated_energy: f64,
    /// ΔH - supplied_work + dissipated_energy; ideally zero.
    pub energy_balance_residual: f64,
    /// Relative residual, normalized by the absolute energy-flow magnitudes.
    pub relative_energy_balance_residual: f64,
    pub linear_solve_residual_norm: f64,
    pub relative_linear_solve_residual: f64,
    pub output_power_midpoint: DVector<f64>,
}

impl PortHamiltonianStepReceipt {
    /// Whether the discrete power balance closes within a relative tolerance.
    pub fn closes_within(&self, relative_tolerance: f64) -> bool {
        relative_tolerance.is_finite()
            && relative_tolerance >= 0.0
            && self.relative_energy_balance_residual <= relative_tolerance
    }

    /// Passivity defect: positive values mean energy increased beyond supplied work.
    pub fn passivity_defect(&self) -> f64 {
        (self.energy_after - self.energy_before - self.supplied_work).max(0.0)
    }
}

impl QuadraticPortHamiltonian {
    /// Construct and validate a quadratic port-Hamiltonian system.
    ///
    /// Matrix dimensions are:
    /// - Q, J and R: n × n;
    /// - G: n × m.
    ///
    /// Validation is performed once at construction, not on every time step.
    pub fn new(
        energy_matrix: DMatrix<f64>,
        interconnection: DMatrix<f64>,
        dissipation: DMatrix<f64>,
        input_matrix: DMatrix<f64>,
        relative_tolerance: f64,
    ) -> Result<Self, PortHamiltonianError> {
        let n = energy_matrix.nrows();
        if n == 0
            || energy_matrix.ncols() != n
            || interconnection.nrows() != n
            || interconnection.ncols() != n
            || dissipation.nrows() != n
            || dissipation.ncols() != n
            || input_matrix.nrows() != n
        {
            return Err(PortHamiltonianError::InvalidDimensions);
        }
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(PortHamiltonianError::InvalidTolerance);
        }
        if !energy_matrix.iter().all(|value| value.is_finite())
            || !interconnection.iter().all(|value| value.is_finite())
            || !dissipation.iter().all(|value| value.is_finite())
            || !input_matrix.iter().all(|value| value.is_finite())
        {
            return Err(PortHamiltonianError::NonFiniteInput);
        }

        validate_symmetric_psd(
            &energy_matrix,
            relative_tolerance,
            PortHamiltonianError::NonSymmetricEnergyMatrix,
            PortHamiltonianError::EnergyMatrixNotPositiveSemidefinite,
        )?;

        let interconnection_scale = interconnection.norm().max(1.0);
        let skew_error = (&interconnection + interconnection.transpose()).norm();
        if skew_error > relative_tolerance * interconnection_scale {
            return Err(PortHamiltonianError::NonSkewInterconnection);
        }

        validate_symmetric_psd(
            &dissipation,
            relative_tolerance,
            PortHamiltonianError::NonSymmetricDissipation,
            PortHamiltonianError::DissipationNotPositiveSemidefinite,
        )?;

        Ok(Self {
            energy_matrix,
            interconnection,
            dissipation,
            input_matrix,
            relative_tolerance,
        })
    }

    pub fn state_dimension(&self) -> usize {
        self.energy_matrix.nrows()
    }

    pub fn input_dimension(&self) -> usize {
        self.input_matrix.ncols()
    }

    pub fn hamiltonian(&self, state: &DVector<f64>) -> Result<f64, PortHamiltonianError> {
        if state.len() != self.state_dimension() {
            return Err(PortHamiltonianError::InvalidDimensions);
        }
        if !state.iter().all(|value| value.is_finite()) {
            return Err(PortHamiltonianError::NonFiniteInput);
        }
        let energy = 0.5 * state.dot(&(&self.energy_matrix * state));
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(PortHamiltonianError::NonFiniteResult)
        }
    }

    /// Advance one implicit discrete-gradient step under constant port input.
    ///
    /// For a quadratic Hamiltonian, the exact discrete gradient is Q times
    /// the midpoint state. The implicit linear system is solved directly; no
    /// explicit energy correction or post-hoc normalization is applied.
    pub fn step(
        &self,
        state: &DVector<f64>,
        input: &DVector<f64>,
        dt: f64,
    ) -> Result<PortHamiltonianStepReceipt, PortHamiltonianError> {
        let n = self.state_dimension();
        if state.len() != n || input.len() != self.input_dimension() {
            return Err(PortHamiltonianError::InvalidDimensions);
        }
        if !dt.is_finite() || dt <= 0.0 {
            return Err(PortHamiltonianError::InvalidTimeStep);
        }
        if !state.iter().all(|value| value.is_finite())
            || !input.iter().all(|value| value.is_finite())
        {
            return Err(PortHamiltonianError::NonFiniteInput);
        }

        let identity = DMatrix::<f64>::identity(n, n);
        let structure = &self.interconnection - &self.dissipation;
        let dynamics = structure * &self.energy_matrix;
        let half_step = 0.5 * dt;
        let lhs = &identity - dynamics.scale(half_step);
        let rhs = (&identity
            + (&self.interconnection - &self.dissipation) * &self.energy_matrix * half_step)
            * state
            + (&self.input_matrix * input) * dt;

        let state_after = lhs
            .clone()
            .lu()
            .solve(&rhs)
            .ok_or(PortHamiltonianError::SingularImplicitStep)?;
        if !state_after.iter().all(|value| value.is_finite()) {
            return Err(PortHamiltonianError::NonFiniteResult);
        }

        let linear_solve_residual_norm = (&lhs * &state_after - &rhs).norm();
        let linear_solve_scale = rhs.norm().max(f64::MIN_POSITIVE);
        let relative_linear_solve_residual = linear_solve_residual_norm / linear_solve_scale;

        let midpoint = (state + &state_after) * 0.5;
        let discrete_gradient = &self.energy_matrix * &midpoint;
        let output_power_midpoint = self.input_matrix.transpose() * &discrete_gradient;
        let supplied_work = dt * output_power_midpoint.dot(input);
        let dissipated_energy = dt
            * discrete_gradient.dot(&(&self.dissipation * &discrete_gradient));
        let energy_before = self.hamiltonian(state)?;
        let energy_after = self.hamiltonian(&state_after)?;
        let energy_balance_residual =
            (energy_after - energy_before) - supplied_work + dissipated_energy;

        let balance_scale = energy_before.abs()
            + energy_after.abs()
            + supplied_work.abs()
            + dissipated_energy.abs();
        let relative_energy_balance_residual = if balance_scale <= f64::MIN_POSITIVE {
            0.0
        } else {
            energy_balance_residual.abs() / balance_scale
        };

        if !supplied_work.is_finite()
            || !dissipated_energy.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
            || !linear_solve_residual_norm.is_finite()
            || !relative_linear_solve_residual.is_finite()
        {
            return Err(PortHamiltonianError::NonFiniteResult);
        }
        if dissipated_energy
            < -self.relative_tolerance * balance_scale.max(f64::MIN_POSITIVE)
        {
            return Err(PortHamiltonianError::NegativeDissipation);
        }

        Ok(PortHamiltonianStepReceipt {
            state_before: state.clone(),
            state_after,
            energy_before,
            energy_after,
            supplied_work,
            dissipated_energy,
            energy_balance_residual,
            relative_energy_balance_residual,
            linear_solve_residual_norm,
            relative_linear_solve_residual,
            output_power_midpoint,
        })
    }
}

fn validate_symmetric_psd(
    matrix: &DMatrix<f64>,
    relative_tolerance: f64,
    symmetry_error: PortHamiltonianError,
    psd_error: PortHamiltonianError,
) -> Result<(), PortHamiltonianError> {
    let scale = matrix.norm().max(f64::MIN_POSITIVE);
    if (matrix - matrix.transpose()).norm() > relative_tolerance * scale {
        return Err(symmetry_error);
    }
    let symmetric = (matrix + matrix.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(symmetric).eigenvalues;
    let minimum = eigenvalues
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    if minimum < -relative_tolerance * scale {
        return Err(psd_error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oscillator(
        input_matrix: DMatrix<f64>,
        dissipation: DMatrix<f64>,
    ) -> QuadraticPortHamiltonian {
        QuadraticPortHamiltonian::new(
            DMatrix::identity(2, 2),
            DMatrix::from_row_slice(2, 2, &[0.0, 1.0, -1.0, 0.0]),
            dissipation,
            input_matrix,
            1e-12,
        )
        .unwrap()
    }

    #[test]
    fn undamped_hamiltonian_oscillator_preserves_energy() {
        let system = oscillator(DMatrix::zeros(2, 0), DMatrix::zeros(2, 2));
        let state = DVector::from_vec(vec![1.0, 0.25]);
        let receipt = system.step(&state, &DVector::zeros(0), 0.1).unwrap();

        assert!((receipt.energy_after - receipt.energy_before).abs() < 1e-12);
        assert!(receipt.dissipated_energy.abs() < 1e-12);
        assert!(receipt.supplied_work.abs() < 1e-12);
        assert!(receipt.closes_within(1e-12));
        assert!(receipt.relative_linear_solve_residual < 1e-12);
    }

    #[test]
    fn dissipative_oscillator_loses_exactly_the_recorded_energy() {
        let dissipation = DMatrix::from_diagonal(&DVector::from_vec(vec![0.0, 0.4]));
        let system = oscillator(DMatrix::zeros(2, 0), dissipation);
        let state = DVector::from_vec(vec![1.0, 0.5]);
        let receipt = system.step(&state, &DVector::zeros(0), 0.05).unwrap();

        assert!(receipt.energy_after < receipt.energy_before);
        assert!(receipt.dissipated_energy > 0.0);
        assert!(
            (receipt.energy_before - receipt.energy_after - receipt.dissipated_energy).abs()
                < 1e-12
        );
        assert!(receipt.closes_within(1e-12));
    }

    #[test]
    fn external_port_work_is_separated_from_stored_energy() {
        let input_matrix = DMatrix::from_column_slice(2, 1, &[1.0, 0.0]);
        let system = oscillator(input_matrix, DMatrix::zeros(2, 2));
        let state = DVector::zeros(2);
        let input = DVector::from_vec(vec![0.3]);
        let receipt = system.step(&state, &input, 0.1).unwrap();

        assert!(
            (receipt.energy_after - receipt.energy_before - receipt.supplied_work).abs() < 1e-12
        );
        assert!(receipt.dissipated_energy.abs() < 1e-12);
        assert!(receipt.closes_within(1e-12));
    }

    #[test]
    fn rejects_non_skew_interconnection_and_indefinite_dissipation() {
        let energy = DMatrix::identity(2, 2);
        let invalid_j = DMatrix::from_row_slice(2, 2, &[0.0, 1.0, 0.0, 0.0]);
        assert_eq!(
            QuadraticPortHamiltonian::new(
                energy.clone(),
                invalid_j,
                DMatrix::zeros(2, 2),
                DMatrix::zeros(2, 0),
                1e-12
            )
            .unwrap_err(),
            PortHamiltonianError::NonSkewInterconnection
        );

        let invalid_r = DMatrix::from_diagonal(&DVector::from_vec(vec![0.0, -1.0]));
        assert_eq!(
            QuadraticPortHamiltonian::new(
                energy,
                DMatrix::zeros(2, 2),
                invalid_r,
                DMatrix::zeros(2, 0),
                1e-12
            )
            .unwrap_err(),
            PortHamiltonianError::DissipationNotPositiveSemidefinite
        );
    }
}
