// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Adaptive Kerr null-ray integration using canonical Hamilton equations.
//!
//! Advances the independent canonical Hamiltonian system in kerr_geodesic.
//! Uses step-doubled classical RK4 with an RMS scaled local error controller.
//! The local estimate is not a rigorous global error bound and this method
//! is not symplectic.
//!
//! Supports exterior Boyer-Lindquist and horizon-regular ingoing Kerr charts.
//! The affine parameter and canonical state are dimensionless. The caller
//! supplies the initial canonical state; no observer tetrad is inferred.

use core::fmt;

use super::{
    KerrBlackHole,
    kerr_geodesic::{
        KerrCanonicalEvaluation, KerrCanonicalHamiltonian, KerrCanonicalState,
        KerrGeodesicError, KerrNullConstants,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerrIntegrationChart {
    BoyerLindquist,
    Ingoing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrIntegratorOptions {
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub initial_step: f64,
    pub minimum_step: f64,
    pub maximum_step: f64,
    pub maximum_steps: usize,
}

impl KerrIntegratorOptions {
    pub fn validate(self) -> Result<Self, KerrIntegratorError> {
        for (name, value) in [
            ("relative_tolerance", self.relative_tolerance),
            ("absolute_tolerance", self.absolute_tolerance),
            ("initial_step", self.initial_step),
            ("minimum_step", self.minimum_step),
            ("maximum_step", self.maximum_step),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(KerrIntegratorError::InvalidOption { name, value });
            }
        }
        if self.minimum_step > self.initial_step
            || self.initial_step > self.maximum_step
            || self.maximum_steps == 0
        {
            return Err(KerrIntegratorError::InconsistentOptions);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrTrajectorySample {
    pub affine_parameter: f64,
    pub state: KerrCanonicalState,
    pub hamiltonian: f64,
    pub energy: f64,
    pub angular_momentum_z_over_m: f64,
    pub carter_constant_over_m2: f64,
    pub normalized_radial_first_integral_residual: f64,
    pub normalized_polar_first_integral_residual: f64,
    pub scaled_local_error: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KerrTrajectoryResult {
    pub samples: Vec<KerrTrajectorySample>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub maximum_abs_hamiltonian: f64,
    pub maximum_relative_energy_drift: f64,
    pub maximum_relative_angular_momentum_drift: f64,
    pub maximum_relative_carter_drift: f64,
    pub maximum_normalized_radial_first_integral_residual: f64,
    pub maximum_normalized_polar_first_integral_residual: f64,
    pub maximum_scaled_local_error: f64,
    pub final_step: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrIntegratorError {
    Geodesic(KerrGeodesicError),
    InvalidOption { name: &'static str, value: f64 },
    InconsistentOptions,
    InvalidAffineInterval(f64),
    NonFiniteState,
    StepUnderflow { affine_parameter: f64, step: f64 },
    StepLimitExceeded { affine_parameter: f64, accepted_steps: usize },
}

impl From<KerrGeodesicError> for KerrIntegratorError {
    fn from(value: KerrGeodesicError) -> Self {
        Self::Geodesic(value)
    }
}

impl fmt::Display for KerrIntegratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geodesic(error) => write!(f, "{error}"),
            Self::InvalidOption { name, value } => {
                write!(f, "Kerr integrator option {name} must be finite and positive, got {value}")
            }
            Self::InconsistentOptions => {
                write!(f, "Kerr integrator step bounds or maximum_steps are inconsistent")
            }
            Self::InvalidAffineInterval(value) => {
                write!(f, "Kerr affine interval must be finite and positive, got {value}")
            }
            Self::NonFiniteState => write!(f, "Kerr integration generated a non-finite state"),
            Self::StepUnderflow { affine_parameter, step } => write!(
                f,
                "Kerr integration step underflow at affine parameter {affine_parameter} with step {step}"
            ),
            Self::StepLimitExceeded { affine_parameter, accepted_steps } => write!(
                f,
                "Kerr integration exceeded its step budget at affine parameter {affine_parameter} after {accepted_steps} accepted steps"
            ),
        }
    }
}

impl std::error::Error for KerrIntegratorError {}

/// Adaptive canonical Kerr ray integrator.
///
/// Hamilton equations evolve momenta continuously through simple turning
/// points; the integrator does not insert square-root sign flips. This is a
/// trajectory integrator, not yet an observer-to-source shooting solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrNullTrajectoryIntegrator {
    hole: KerrBlackHole,
    chart: KerrIntegrationChart,
    options: KerrIntegratorOptions,
}

impl KerrNullTrajectoryIntegrator {
    pub fn new(
        hole: KerrBlackHole,
        chart: KerrIntegrationChart,
        options: KerrIntegratorOptions,
    ) -> Result<Self, KerrIntegratorError> {
        Ok(Self { hole, chart, options: options.validate()? })
    }

    pub const fn black_hole(self) -> KerrBlackHole { self.hole }
    pub const fn chart(self) -> KerrIntegrationChart { self.chart }
    pub const fn options(self) -> KerrIntegratorOptions { self.options }

    /// Integrate from affine parameter zero to a positive interval endpoint.
    ///
    /// Every accepted state is retained. The step-doubling estimate is the
    /// RMS of the component-wise scaled difference divided by 15. The
    /// accepted state is the two-half-step solution; no null normalization,
    /// Richardson correction or hidden projection is applied.
    pub fn integrate(
        self,
        initial_state: KerrCanonicalState,
        affine_interval: f64,
    ) -> Result<KerrTrajectoryResult, KerrIntegratorError> {
        if !affine_interval.is_finite() || affine_interval <= 0.0 {
            return Err(KerrIntegratorError::InvalidAffineInterval(affine_interval));
        }

        let hamiltonian = KerrCanonicalHamiltonian::new(self.hole);
        let initial_eval = self.evaluate(hamiltonian, initial_state)?;
        let initial_diagnostics = self.constraint_diagnostics(initial_state, initial_eval)?;
        let initial_energy = initial_eval.energy;
        let initial_angular_momentum = initial_eval.angular_momentum_z_over_m;
        let initial_carter = initial_eval.carter_constant_over_m2;
        let mut samples = vec![KerrTrajectorySample {
            affine_parameter: 0.0,
            state: initial_state,
            hamiltonian: initial_eval.hamiltonian,
            energy: initial_energy,
            angular_momentum_z_over_m: initial_angular_momentum,
            carter_constant_over_m2: initial_carter,
            normalized_radial_first_integral_residual: initial_diagnostics.0,
            normalized_polar_first_integral_residual: initial_diagnostics.1,
            scaled_local_error: 0.0,
        }];

        let mut affine = 0.0;
        let mut step = self.options.initial_step.min(self.options.maximum_step);
        let mut accepted_steps = 0usize;
        let mut rejected_steps = 0usize;
        let mut maximum_abs_hamiltonian = initial_eval.hamiltonian.abs();
        let mut maximum_relative_energy_drift: f64 = 0.0;
        let mut maximum_relative_angular_momentum_drift: f64 = 0.0;
        let mut maximum_relative_carter_drift: f64 = 0.0;
        let mut maximum_normalized_radial_first_integral_residual = initial_diagnostics.0;
        let mut maximum_normalized_polar_first_integral_residual = initial_diagnostics.1;
        let mut maximum_scaled_local_error: f64 = 0.0;
        let mut state = initial_state;

        while affine < affine_interval {
            if accepted_steps >= self.options.maximum_steps {
                return Err(KerrIntegratorError::StepLimitExceeded {
                    affine_parameter: affine,
                    accepted_steps,
                });
            }

            let remaining = affine_interval - affine;
            step = step.min(remaining).min(self.options.maximum_step);
            if step < self.options.minimum_step && remaining > self.options.minimum_step {
                return Err(KerrIntegratorError::StepUnderflow {
                    affine_parameter: affine,
                    step,
                });
            }

            let full = self.rk4_step(hamiltonian, state, step)?;
            let half = self.rk4_step(hamiltonian, state, 0.5 * step)?;
            let two_half = self.rk4_step(hamiltonian, half, 0.5 * step)?;
            let error = self.scaled_step_error(state, full, two_half)?;
            if !error.is_finite() {
                return Err(KerrIntegratorError::NonFiniteState);
            }

            if error <= 1.0 {
                state = two_half;
                affine = if remaining <= step { affine_interval } else { affine + step };
                accepted_steps += 1;
                let evaluation = self.evaluate(hamiltonian, state)?;
                let diagnostics = self.constraint_diagnostics(state, evaluation)?;
                maximum_abs_hamiltonian =
                    maximum_abs_hamiltonian.max(evaluation.hamiltonian.abs());
                maximum_relative_energy_drift = maximum_relative_energy_drift.max(
                    relative_drift(initial_energy, evaluation.energy),
                );
                maximum_relative_angular_momentum_drift =
                    maximum_relative_angular_momentum_drift.max(relative_drift(
                        initial_angular_momentum,
                        evaluation.angular_momentum_z_over_m,
                    ));
                maximum_relative_carter_drift = maximum_relative_carter_drift.max(
                    relative_drift(initial_carter, evaluation.carter_constant_over_m2),
                );
                maximum_normalized_radial_first_integral_residual =
                    maximum_normalized_radial_first_integral_residual.max(diagnostics.0);
                maximum_normalized_polar_first_integral_residual =
                    maximum_normalized_polar_first_integral_residual.max(diagnostics.1);
                maximum_scaled_local_error = maximum_scaled_local_error.max(error);
                samples.push(KerrTrajectorySample {
                    affine_parameter: affine,
                    state,
                    hamiltonian: evaluation.hamiltonian,
                    energy: evaluation.energy,
                    angular_momentum_z_over_m: evaluation.angular_momentum_z_over_m,
                    carter_constant_over_m2: evaluation.carter_constant_over_m2,
                    normalized_radial_first_integral_residual: diagnostics.0,
                    normalized_polar_first_integral_residual: diagnostics.1,
                    scaled_local_error: error,
                });
                let factor = if error == 0.0 { 2.0 } else {
                    (0.9 * error.powf(-0.2)).clamp(0.2, 2.0)
                };
                step = (step * factor)
                    .max(self.options.minimum_step)
                    .min(self.options.maximum_step);
            } else {
                rejected_steps += 1;
                let factor = (0.9 * error.powf(-0.2)).clamp(0.1, 0.5);
                let reduced = step * factor;
                if reduced < self.options.minimum_step {
                    return Err(KerrIntegratorError::StepUnderflow {
                        affine_parameter: affine,
                        step: reduced,
                    });
                }
                step = reduced;
            }
        }

        Ok(KerrTrajectoryResult {
            samples,
            accepted_steps,
            rejected_steps,
            maximum_abs_hamiltonian,
            maximum_relative_energy_drift,
            maximum_relative_angular_momentum_drift,
            maximum_relative_carter_drift,
            maximum_normalized_radial_first_integral_residual,
            maximum_normalized_polar_first_integral_residual,
            maximum_scaled_local_error,
            final_step: step,
        })
    }

    fn constraint_diagnostics(
        self,
        state: KerrCanonicalState,
        evaluation: KerrCanonicalEvaluation,
    ) -> Result<(f64, f64), KerrIntegratorError> {
        let constants = KerrNullConstants::new(
            evaluation.energy,
            evaluation.angular_momentum_z_over_m,
            evaluation.carter_constant_over_m2,
        )?;
        let r = state.coordinates.radius_over_m;
        let theta = state.coordinates.theta_rad;
        let spin = self.hole.spin().get();
        let delta = r * r - 2.0 * r + spin * spin;
        let p = evaluation.energy * (r * r + spin * spin)
            - spin * evaluation.angular_momentum_z_over_m;
        let radial_velocity_mino = match self.chart {
            KerrIntegrationChart::BoyerLindquist => delta * state.momenta[1],
            KerrIntegrationChart::Ingoing => delta * state.momenta[1] - p,
        };
        let radial_potential = constants.radial_potential(spin, r)?;
        let polar_potential = constants.polar_potential(spin, theta)?;
        let radial_square = radial_velocity_mino * radial_velocity_mino;
        let polar_square = state.momenta[2] * state.momenta[2];
        let radial_scale = 1.0 + radial_square.abs() + radial_potential.abs();
        let polar_scale = 1.0 + polar_square.abs() + polar_potential.abs();
        let radial_residual = (radial_square - radial_potential).abs() / radial_scale;
        let polar_residual = (polar_square - polar_potential).abs() / polar_scale;
        if !radial_residual.is_finite() || !polar_residual.is_finite() {
            return Err(KerrIntegratorError::NonFiniteState);
        }
        Ok((radial_residual, polar_residual))
    }

    fn evaluate(
        self,
        hamiltonian: KerrCanonicalHamiltonian,
        state: KerrCanonicalState,
    ) -> Result<super::kerr_geodesic::KerrCanonicalEvaluation, KerrIntegratorError> {
        match self.chart {
            KerrIntegrationChart::BoyerLindquist => Ok(hamiltonian.evaluate(state)?),
            KerrIntegrationChart::Ingoing => Ok(hamiltonian.evaluate_ingoing(state)?),
        }
    }

    fn rk4_step(
        self,
        hamiltonian: KerrCanonicalHamiltonian,
        state: KerrCanonicalState,
        step: f64,
    ) -> Result<KerrCanonicalState, KerrIntegratorError> {
        let y = flatten(state);
        let k1 = self.rhs(hamiltonian, state)?;
        let k2 = self.rhs(hamiltonian, unflatten(add_scaled(y, k1, 0.5 * step)))?;
        let k3 = self.rhs(hamiltonian, unflatten(add_scaled(y, k2, 0.5 * step)))?;
        let k4 = self.rhs(hamiltonian, unflatten(add_scaled(y, k3, step)))?;
        let mut next = y;
        for index in 0..8 {
            next[index] += step
                * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index])
                / 6.0;
        }
        if next.iter().any(|value| !value.is_finite()) {
            return Err(KerrIntegratorError::NonFiniteState);
        }
        Ok(unflatten(next))
    }

    fn rhs(
        self,
        hamiltonian: KerrCanonicalHamiltonian,
        state: KerrCanonicalState,
    ) -> Result<[f64; 8], KerrIntegratorError> {
        let derivative = self.evaluate(hamiltonian, state)?.derivative;
        let mut result = [0.0; 8];
        result[..4].copy_from_slice(&derivative.coordinates);
        result[4..].copy_from_slice(&derivative.momenta);
        if result.iter().any(|value| !value.is_finite()) {
            return Err(KerrIntegratorError::NonFiniteState);
        }
        Ok(result)
    }

    fn scaled_step_error(
        self,
        initial: KerrCanonicalState,
        full_step: KerrCanonicalState,
        two_half_steps: KerrCanonicalState,
    ) -> Result<f64, KerrIntegratorError> {
        let initial = flatten(initial);
        let full = flatten(full_step);
        let half = flatten(two_half_steps);
        let mut sum = 0.0;
        for index in 0..8 {
            let scale = self.options.absolute_tolerance
                + self.options.relative_tolerance * initial[index].abs().max(half[index].abs());
            let normalized = (half[index] - full[index]).abs() / (15.0 * scale);
            if !normalized.is_finite() {
                return Err(KerrIntegratorError::NonFiniteState);
            }
            sum += normalized * normalized;
        }
        Ok((sum / 8.0).sqrt())
    }
}

fn relative_drift(initial: f64, current: f64) -> f64 {
    (current - initial).abs() / initial.abs().max(1.0)
}

fn flatten(state: KerrCanonicalState) -> [f64; 8] {
    [
        state.coordinates.time_over_m,
        state.coordinates.radius_over_m,
        state.coordinates.theta_rad,
        state.coordinates.azimuth_rad,
        state.momenta[0],
        state.momenta[1],
        state.momenta[2],
        state.momenta[3],
    ]
}

fn unflatten(values: [f64; 8]) -> KerrCanonicalState {
    KerrCanonicalState {
        coordinates: super::kerr_geodesic::KerrMinoState::new(
            values[0], values[1], values[2], values[3],
        ),
        momenta: [values[4], values[5], values[6], values[7]],
    }
}

fn add_scaled(state: [f64; 8], derivative: [f64; 8], scale: f64) -> [f64; 8] {
    let mut result = state;
    for index in 0..8 { result[index] += scale * derivative[index]; }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DimensionlessSpin, MassKg,
        kerr_geodesic::{KerrMinoDirection, KerrMinoState, KerrNullConstants},
    };

    fn hole(spin: f64) -> KerrBlackHole {
        KerrBlackHole::new(
            MassKg::new(1.988_47e30).unwrap(),
            DimensionlessSpin::new(spin).unwrap(),
        )
    }

    fn options() -> KerrIntegratorOptions {
        KerrIntegratorOptions {
            relative_tolerance: 1e-10,
            absolute_tolerance: 1e-12,
            initial_step: 1e-2,
            minimum_step: 1e-10,
            maximum_step: 5e-2,
            maximum_steps: 200_000,
        }
    }

    #[test]
    fn radial_schwarzschild_null_ray_matches_analytic_coordinate_time_difference() {
        let black_hole = hole(0.0);
        let constants = KerrNullConstants::new(1.0, 0.0, 0.0).unwrap();
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let initial = canonical.state_from_separated(
            KerrMinoState::new(0.0, 10.0, core::f64::consts::FRAC_PI_2, 0.0),
            constants,
            KerrMinoDirection::Decreasing,
            KerrMinoDirection::Increasing,
        ).unwrap();
        let integrator = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, options(),
        ).unwrap();
        let result = integrator.integrate(initial, 0.5).unwrap();
        let final_state = result.samples.last().unwrap().state;
        let r0 = 10.0;
        let r1 = final_state.coordinates.radius_over_m;
        assert!(r1 < r0);
        let expected_delta_t =
            (r0 - r1) + 2.0 * ((r0 / 2.0 - 1.0).ln() - (r1 / 2.0 - 1.0).ln());
        assert!((final_state.coordinates.time_over_m - expected_delta_t).abs() < 2e-8);
        assert!(result.maximum_abs_hamiltonian < 1e-10);
    }

    #[test]
    fn ingoing_chart_integrator_crosses_future_outer_horizon_without_blowup() {
        let black_hole = hole(0.9);
        let constants = KerrNullConstants::new(1.0, 0.0, 0.0).unwrap();
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let horizon = 1.0 + (1.0 - 0.9_f64.powi(2)).sqrt();
        let initial = canonical.ingoing_state_from_separated(
            KerrMinoState::new(0.0, horizon + 0.2, core::f64::consts::FRAC_PI_2, 0.0),
            constants,
            KerrMinoDirection::Increasing,
        ).unwrap();
        let integrator = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::Ingoing, options(),
        ).unwrap();
        let result = integrator.integrate(initial, 1.0).unwrap();
        let final_radius = result.samples.last().unwrap().state.coordinates.radius_over_m;
        assert!(final_radius < horizon);
        assert!(final_radius > 0.0);
        assert!(result.maximum_abs_hamiltonian < 1e-8);
    }

    #[test]
    fn tighter_tolerance_does_not_increase_final_state_error_against_refinement() {
        let black_hole = hole(0.5);
        let constants = KerrNullConstants::new(1.0, 0.0, 0.0).unwrap();
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let initial = canonical.state_from_separated(
            KerrMinoState::new(0.0, 12.0, core::f64::consts::FRAC_PI_2, 0.0),
            constants,
            KerrMinoDirection::Decreasing,
            KerrMinoDirection::Increasing,
        ).unwrap();
        let coarse_options = KerrIntegratorOptions {
            relative_tolerance: 1e-7,
            absolute_tolerance: 1e-9,
            initial_step: 0.1,
            minimum_step: 1e-8,
            maximum_step: 0.1,
            maximum_steps: 100_000,
        };
        let fine_options = KerrIntegratorOptions {
            relative_tolerance: 1e-11,
            absolute_tolerance: 1e-13,
            ..coarse_options
        };
        let reference_options = KerrIntegratorOptions {
            relative_tolerance: 1e-13,
            absolute_tolerance: 1e-15,
            ..coarse_options
        };
        let coarse = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, coarse_options,
        ).unwrap().integrate(initial, 1.0).unwrap();
        let fine = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, fine_options,
        ).unwrap().integrate(initial, 1.0).unwrap();
        let reference = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, reference_options,
        ).unwrap().integrate(initial, 1.0).unwrap();
        let distance = |left: KerrCanonicalState, right: KerrCanonicalState| {
            let a = flatten(left);
            let b = flatten(right);
            a.iter().zip(b.iter()).map(|(x,y)|(x-y).powi(2)).sum::<f64>().sqrt()
        };
        assert!(distance(fine.samples.last().unwrap().state, reference.samples.last().unwrap().state)
            <= distance(coarse.samples.last().unwrap().state, reference.samples.last().unwrap().state));
    }

    #[test]
    fn integrator_rejects_invalid_step_configuration() {
        let invalid = KerrIntegratorOptions { relative_tolerance: -1.0, ..options() };
        assert!(matches!(
            KerrNullTrajectoryIntegrator::new(
                hole(0.0), KerrIntegrationChart::BoyerLindquist, invalid
            ),
            Err(KerrIntegratorError::InvalidOption { .. })
        ));
    }
}
