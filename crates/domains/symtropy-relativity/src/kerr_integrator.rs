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
//! The affine parameter and canonical state are dimensionless. An optional
//! variational path advances the full canonical fundamental matrix with the
//! same step-doubling controller. The caller supplies the initial canonical
//! state; no observer tetrad or optical screen basis is inferred.

use core::fmt;

use super::{
    KerrBlackHole,
    kerr_geodesic::{
        KerrCanonicalEvaluation, KerrCanonicalHamiltonian, KerrCanonicalLinearization,
        KerrCanonicalState, KerrFiniteRadiusObserver, KerrGeodesicError,
        KerrLocalSkyDirection, KerrMinoCoordinateChart, KerrNullConstants,
        KerrObserverCanonicalSeed, KerrObserverScreenProjection,
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

/// One accepted sample from canonical trajectory plus variational transport.
#[derive(Debug, Clone, PartialEq)]
pub struct KerrVariationalSample {
    pub trajectory: KerrTrajectorySample,
    /// Fundamental matrix Phi = d z(lambda) / d z(0), row-major by state component.
    pub fundamental_matrix: [[f64; 8]; 8],
    /// Local Hamiltonian-flow identity residual max |A^T Omega + Omega A|.
    pub local_symplectic_lie_residual: f64,
    /// Propagated fundamental-matrix residual max |Phi^T Omega Phi - Omega|.
    pub fundamental_symplectic_residual: f64,
}

/// Adaptive canonical trajectory and full 8x8 variational fundamental matrix.
#[derive(Debug, Clone, PartialEq)]
pub struct KerrVariationalTrajectoryResult {
    pub samples: Vec<KerrVariationalSample>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub maximum_scaled_local_error: f64,
    pub maximum_fundamental_symplectic_residual: f64,
    pub final_step: f64,
}

/// A propagated angular perturbation of a finite-observer Kerr ray.
///
/// Variations are derivatives with respect to the two returned orthonormal
/// local-sky angular coordinates (radians), represented in canonical
/// phase-space ordering. They are not yet projected source-screen distances.
#[derive(Debug, Clone, PartialEq)]
pub struct KerrObserverTangentSample {
    pub trajectory: KerrTrajectorySample,
    pub first_phase_space_variation: [f64; 8],
    pub second_phase_space_variation: [f64; 8],
    pub fundamental_symplectic_residual: f64,
}

/// Propagated pair of observer-screen angular variations.
#[derive(Debug, Clone, PartialEq)]
pub struct KerrObserverTangentBundleResult {
    pub black_hole: KerrBlackHole,
    pub coordinate_chart: KerrMinoCoordinateChart,
    pub local_screen_basis: [[f64; 3]; 2],
    pub samples: Vec<KerrObserverTangentSample>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub maximum_scaled_local_error: f64,
    pub maximum_fundamental_symplectic_residual: f64,
    pub final_step: f64,
}

/// Two-dimensional finite-observer screen map at one sampled affine parameter.
///
/// Matrix columns are derivatives with respect to the two initial local-sky
/// angular coordinates (radians); rows are components in the source ZAMO screen
/// basis. Singular values are returned in ascending order. The map is a local
/// optical transfer diagnostic, not a complete lensing/caustic catalogue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrOpticalScreenMap {
    pub affine_parameter: f64,
    pub jacobian: [[f64; 2]; 2],
    pub determinant: f64,
    pub singular_values: [f64; 2],
    pub condition_number: f64,
    pub source_screen_basis: [[f64; 3]; 2],
    pub source_direction_norm_residual: f64,
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
    CoordinateChartMismatch { seed: KerrMinoCoordinateChart, integrator: KerrIntegrationChart },
    SampleIndexOutOfRange { index: usize, length: usize },
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
            Self::CoordinateChartMismatch { seed, integrator } => write!(
                f, "observer seed chart {seed:?} does not match integrator chart {integrator:?}"
            ),
            Self::SampleIndexOutOfRange { index, length } => write!(
                f, "Kerr tangent bundle sample index {index} is outside {length} retained samples"
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

impl KerrObserverTangentBundleResult {
    /// Build the source-screen 2x2 map at one retained affine sample.
    ///
    /// Samples after horizon crossing are not projected because a stationary
    /// ZAMO source observer does not exist inside the outer horizon.
    pub fn optical_screen_map_at(
        &self,
        sample_index: usize,
        null_tolerance: f64,
    ) -> Result<KerrOpticalScreenMap, KerrIntegratorError> {
        let sample = self.samples.get(sample_index).ok_or(
            KerrIntegratorError::SampleIndexOutOfRange {
                index: sample_index,
                length: self.samples.len(),
            },
        )?;
        let state = sample.trajectory.state;
        let observer = KerrFiniteRadiusObserver::new(
            self.black_hole,
            state.coordinates.radius_over_m,
            state.coordinates.theta_rad,
        )?;
        let first: KerrObserverScreenProjection =
            observer.project_canonical_variation_to_screen(
                state,
                sample.first_phase_space_variation,
                self.coordinate_chart,
                null_tolerance,
            )?;
        let second: KerrObserverScreenProjection =
            observer.project_canonical_variation_to_screen(
                state,
                sample.second_phase_space_variation,
                self.coordinate_chart,
                null_tolerance,
            )?;
        let jacobian = [
            [first.components[0], second.components[0]],
            [first.components[1], second.components[1]],
        ];
        let determinant =
            jacobian[0][0] * jacobian[1][1] - jacobian[0][1] * jacobian[1][0];
        let frobenius_squared = jacobian.iter().flatten().map(|value| value * value).sum::<f64>();
        let discriminant = (frobenius_squared * frobenius_squared
            - 4.0 * determinant * determinant).max(0.0).sqrt();
        let large_squared = (0.5 * (frobenius_squared + discriminant)).max(0.0);
        let small_squared = (0.5 * (frobenius_squared - discriminant)).max(0.0);
        let small = small_squared.sqrt();
        let large = large_squared.sqrt();
        let condition_number = if small > 0.0 {
            large / small
        } else {
            f64::INFINITY
        };
        if !determinant.is_finite() || !small.is_finite() || !large.is_finite() {
            return Err(KerrIntegratorError::NonFiniteState);
        }
        Ok(KerrOpticalScreenMap {
            affine_parameter: sample.trajectory.affine_parameter,
            jacobian,
            determinant,
            singular_values: [small, large],
            condition_number,
            source_screen_basis: first.local_screen_basis,
            source_direction_norm_residual: first.direction_norm_residual
                .max(second.direction_norm_residual),
        })
    }

    /// Project every retained sample in the exterior observer domain.
    ///
    /// The first horizon-interior or otherwise unprojectable sample is returned
    /// as an error instead of being assigned a fabricated source screen.
    pub fn sampled_optical_screen_maps(
        &self,
        null_tolerance: f64,
    ) -> Result<Vec<KerrOpticalScreenMap>, KerrIntegratorError> {
        (0..self.samples.len())
            .map(|index| self.optical_screen_map_at(index, null_tolerance))
            .collect()
    }
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

    /// Propagate the two angular tangent directions supplied by a finite
    /// ZAMO observer seed. The full fundamental matrix is integrated and then
    /// applied to each null-shell tangent seed. The returned variations remain
    /// canonical phase-space derivatives; source-screen projection is separate.
    pub fn integrate_observer_tangent_bundle(
        self,
        seed: KerrObserverCanonicalSeed,
        affine_interval: f64,
    ) -> Result<KerrObserverTangentBundleResult, KerrIntegratorError> {
        let expected_chart = match self.chart {
            KerrIntegrationChart::BoyerLindquist => KerrMinoCoordinateChart::BoyerLindquist,
            KerrIntegrationChart::Ingoing => KerrMinoCoordinateChart::IngoingKerr,
        };
        if seed.coordinate_chart != expected_chart {
            return Err(KerrIntegratorError::CoordinateChartMismatch {
                seed: seed.coordinate_chart,
                integrator: self.chart,
            });
        }
        let transported = self.integrate_fundamental_matrix(seed.state, affine_interval)?;
        let mut samples = Vec::with_capacity(transported.samples.len());
        for sample in transported.samples {
            let first = matrix_vector_product(
                &sample.fundamental_matrix, &seed.first_phase_space_variation,
            );
            let second = matrix_vector_product(
                &sample.fundamental_matrix, &seed.second_phase_space_variation,
            );
            samples.push(KerrObserverTangentSample {
                trajectory: sample.trajectory,
                first_phase_space_variation: first,
                second_phase_space_variation: second,
                fundamental_symplectic_residual: sample.fundamental_symplectic_residual,
            });
        }
        Ok(KerrObserverTangentBundleResult {
            black_hole: self.hole,
            coordinate_chart: self.chart,
            local_screen_basis: seed.local_screen_basis,
            samples,
            accepted_steps: transported.accepted_steps,
            rejected_steps: transported.rejected_steps,
            maximum_scaled_local_error: transported.maximum_scaled_local_error,
            maximum_fundamental_symplectic_residual:
                transported.maximum_fundamental_symplectic_residual,
            final_step: transported.final_step,
        })
    }

    /// Integrate the canonical ray and its full fundamental matrix.
    ///
    /// The augmented ODE is dz/dlambda = F(z), dPhi/dlambda = A(z) Phi,
    /// where A is supplied by second-order metric AD. Step-doubling controls
    /// state and matrix errors separately and accepts the two-half-step state.
    /// Phi(0) is the 8x8 identity. This is a canonical phase-space tangent map,
    /// not yet a projected 2x2 optical Jacobi map.
    pub fn integrate_fundamental_matrix(
        self,
        initial_state: KerrCanonicalState,
        affine_interval: f64,
    ) -> Result<KerrVariationalTrajectoryResult, KerrIntegratorError> {
        if !affine_interval.is_finite() || affine_interval <= 0.0 {
            return Err(KerrIntegratorError::InvalidAffineInterval(affine_interval));
        }

        let hamiltonian = KerrCanonicalHamiltonian::new(self.hole);
        let initial_linearization = self.linearize(hamiltonian, initial_state)?;
        let initial_evaluation = initial_linearization.evaluation;
        let initial_diagnostics = self.constraint_diagnostics(initial_state, initial_evaluation)?;
        let mut identity = [[0.0; 8]; 8];
        for index in 0..8 { identity[index][index] = 1.0; }

        let initial_trajectory = KerrTrajectorySample {
            affine_parameter: 0.0,
            state: initial_state,
            hamiltonian: initial_evaluation.hamiltonian,
            energy: initial_evaluation.energy,
            angular_momentum_z_over_m: initial_evaluation.angular_momentum_z_over_m,
            carter_constant_over_m2: initial_evaluation.carter_constant_over_m2,
            normalized_radial_first_integral_residual: initial_diagnostics.0,
            normalized_polar_first_integral_residual: initial_diagnostics.1,
            scaled_local_error: 0.0,
        };
        let mut samples = vec![KerrVariationalSample {
            trajectory: initial_trajectory,
            fundamental_matrix: identity,
            local_symplectic_lie_residual: initial_linearization.symplectic_lie_residual,
            fundamental_symplectic_residual: 0.0,
        }];

        let mut affine = 0.0;
        let mut step = self.options.initial_step.min(self.options.maximum_step);
        let mut accepted_steps = 0usize;
        let mut rejected_steps = 0usize;
        let mut maximum_scaled_local_error: f64 = 0.0;
        let mut maximum_fundamental_symplectic_residual: f64 = 0.0;
        let mut state = initial_state;
        let mut fundamental = identity;

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

            let full = self.variational_rk4_step(hamiltonian, state, fundamental, step)?;
            let half = self.variational_rk4_step(hamiltonian, state, fundamental, 0.5 * step)?;
            let two_half = self.variational_rk4_step(
                hamiltonian, half.0, half.1, 0.5 * step,
            )?;
            let error = self.scaled_variational_step_error(
                state, fundamental, full.0, full.1, two_half.0, two_half.1,
            )?;
            if !error.is_finite() {
                return Err(KerrIntegratorError::NonFiniteState);
            }

            if error <= 1.0 {
                state = two_half.0;
                fundamental = two_half.1;
                affine = if remaining <= step { affine_interval } else { affine + step };
                accepted_steps += 1;

                let linearization = self.linearize(hamiltonian, state)?;
                let evaluation = linearization.evaluation;
                let diagnostics = self.constraint_diagnostics(state, evaluation)?;
                let tangent_residual = fundamental_symplectic_residual(&fundamental);
                if !tangent_residual.is_finite() {
                    return Err(KerrIntegratorError::NonFiniteState);
                }
                maximum_scaled_local_error = maximum_scaled_local_error.max(error);
                maximum_fundamental_symplectic_residual =
                    maximum_fundamental_symplectic_residual.max(tangent_residual);
                samples.push(KerrVariationalSample {
                    trajectory: KerrTrajectorySample {
                        affine_parameter: affine,
                        state,
                        hamiltonian: evaluation.hamiltonian,
                        energy: evaluation.energy,
                        angular_momentum_z_over_m: evaluation.angular_momentum_z_over_m,
                        carter_constant_over_m2: evaluation.carter_constant_over_m2,
                        normalized_radial_first_integral_residual: diagnostics.0,
                        normalized_polar_first_integral_residual: diagnostics.1,
                        scaled_local_error: error,
                    },
                    fundamental_matrix: fundamental,
                    local_symplectic_lie_residual: linearization.symplectic_lie_residual,
                    fundamental_symplectic_residual: tangent_residual,
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

        Ok(KerrVariationalTrajectoryResult {
            samples,
            accepted_steps,
            rejected_steps,
            maximum_scaled_local_error,
            maximum_fundamental_symplectic_residual,
            final_step: step,
        })
    }

    fn linearize(
        self,
        hamiltonian: KerrCanonicalHamiltonian,
        state: KerrCanonicalState,
    ) -> Result<KerrCanonicalLinearization, KerrIntegratorError> {
        match self.chart {
            KerrIntegrationChart::BoyerLindquist => Ok(hamiltonian.linearize(state)?),
            KerrIntegrationChart::Ingoing => Ok(hamiltonian.linearize_ingoing(state)?),
        }
    }

    fn variational_rhs(
        self,
        hamiltonian: KerrCanonicalHamiltonian,
        state: KerrCanonicalState,
        fundamental: [[f64; 8]; 8],
    ) -> Result<[f64; 72], KerrIntegratorError> {
        let linearization = self.linearize(hamiltonian, state)?;
        let mut result = [0.0; 72];
        let field = flatten_derivative(linearization.evaluation.derivative);
        result[..8].copy_from_slice(&field);
        for row in 0..8 {
            for column in 0..8 {
                let mut value = 0.0;
                for inner in 0..8 {
                    value += linearization.jacobian[row][inner] * fundamental[inner][column];
                }
                result[8 + row * 8 + column] = value;
            }
        }
        if result.iter().any(|value| !value.is_finite()) {
            return Err(KerrIntegratorError::NonFiniteState);
        }
        Ok(result)
    }

    fn variational_rk4_step(
        self,
        hamiltonian: KerrCanonicalHamiltonian,
        state: KerrCanonicalState,
        fundamental: [[f64; 8]; 8],
        step: f64,
    ) -> Result<(KerrCanonicalState, [[f64; 8]; 8]), KerrIntegratorError> {
        let y = flatten_variational(state, fundamental);
        let k1 = self.variational_rhs(hamiltonian, state, fundamental)?;
        let y2 = add_scaled_variational(y, k1, 0.5 * step);
        let (state2, fundamental2) = unflatten_variational(y2);
        let k2 = self.variational_rhs(hamiltonian, state2, fundamental2)?;
        let y3 = add_scaled_variational(y, k2, 0.5 * step);
        let (state3, fundamental3) = unflatten_variational(y3);
        let k3 = self.variational_rhs(hamiltonian, state3, fundamental3)?;
        let y4 = add_scaled_variational(y, k3, step);
        let (state4, fundamental4) = unflatten_variational(y4);
        let k4 = self.variational_rhs(hamiltonian, state4, fundamental4)?;

        let mut next = y;
        for index in 0..72 {
            next[index] += step
                * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index])
                / 6.0;
        }
        if next.iter().any(|value| !value.is_finite()) {
            return Err(KerrIntegratorError::NonFiniteState);
        }
        Ok(unflatten_variational(next))
    }

    fn scaled_variational_step_error(
        self,
        initial_state: KerrCanonicalState,
        initial_fundamental: [[f64; 8]; 8],
        full_state: KerrCanonicalState,
        full_fundamental: [[f64; 8]; 8],
        half_state: KerrCanonicalState,
        half_fundamental: [[f64; 8]; 8],
    ) -> Result<f64, KerrIntegratorError> {
        let initial = flatten_variational(initial_state, initial_fundamental);
        let full = flatten_variational(full_state, full_fundamental);
        let half = flatten_variational(half_state, half_fundamental);
        let mut state_sum = 0.0;
        let mut tangent_sum = 0.0;
        for index in 0..72 {
            let scale = self.options.absolute_tolerance
                + self.options.relative_tolerance * initial[index].abs().max(half[index].abs());
            let normalized = (half[index] - full[index]).abs() / (15.0 * scale);
            if !normalized.is_finite() {
                return Err(KerrIntegratorError::NonFiniteState);
            }
            if index < 8 { state_sum += normalized * normalized; }
            else { tangent_sum += normalized * normalized; }
        }
        Ok((state_sum.sqrt() / 8.0_f64.sqrt())
            .max(tangent_sum.sqrt() / 64.0_f64.sqrt()))
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

fn matrix_vector_product(matrix: &[[f64; 8]; 8], vector: &[f64; 8]) -> [f64; 8] {
    let mut result = [0.0; 8];
    for row in 0..8 {
        for column in 0..8 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn flatten_derivative(derivative: super::kerr_geodesic::KerrCanonicalDerivative) -> [f64; 8] {
    [
        derivative.coordinates[0],
        derivative.coordinates[1],
        derivative.coordinates[2],
        derivative.coordinates[3],
        derivative.momenta[0],
        derivative.momenta[1],
        derivative.momenta[2],
        derivative.momenta[3],
    ]
}

fn flatten_variational(
    state: KerrCanonicalState,
    fundamental: [[f64; 8]; 8],
) -> [f64; 72] {
    let mut values = [0.0; 72];
    values[..8].copy_from_slice(&flatten(state));
    for row in 0..8 {
        for column in 0..8 {
            values[8 + row * 8 + column] = fundamental[row][column];
        }
    }
    values
}

fn unflatten_variational(values: [f64; 72]) -> (KerrCanonicalState, [[f64; 8]; 8]) {
    let mut fundamental = [[0.0; 8]; 8];
    for row in 0..8 {
        for column in 0..8 {
            fundamental[row][column] = values[8 + row * 8 + column];
        }
    }
    let mut state_values = [0.0; 8];
    state_values.copy_from_slice(&values[..8]);
    (unflatten(state_values), fundamental)
}

fn add_scaled_variational(
    state: [f64; 72],
    derivative: [f64; 72],
    scale: f64,
) -> [f64; 72] {
    let mut result = state;
    for index in 0..72 { result[index] += scale * derivative[index]; }
    result
}

fn canonical_omega_entry(row: usize, column: usize) -> f64 {
    if row < 4 && column == row + 4 { 1.0 }
    else if row >= 4 && column + 4 == row { -1.0 }
    else { 0.0 }
}

/// max absolute component of Phi^T Omega Phi - Omega.
fn fundamental_symplectic_residual(fundamental: &[[f64; 8]; 8]) -> f64 {
    let mut maximum: f64 = 0.0;
    for row in 0..8 {
        for column in 0..8 {
            let mut value = -canonical_omega_entry(row, column);
            for i in 0..8 {
                for j in 0..8 {
                    value += fundamental[i][row]
                        * canonical_omega_entry(i, j)
                        * fundamental[j][column];
                }
            }
            maximum = maximum.max(value.abs());
        }
    }
    maximum
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
        assert!(result.maximum_normalized_radial_first_integral_residual < 2e-9);
        assert!(result.maximum_normalized_polar_first_integral_residual < 2e-12);
        assert!(result.maximum_relative_energy_drift < 2e-12);
        assert!(result.maximum_relative_angular_momentum_drift < 2e-12);
        assert!(result.maximum_relative_carter_drift < 2e-12);
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
        assert!(result.maximum_normalized_radial_first_integral_residual < 2e-8);
        assert!(result.maximum_normalized_polar_first_integral_residual < 2e-12);
        assert!(result.maximum_relative_energy_drift < 2e-10);
        assert!(result.maximum_relative_angular_momentum_drift < 2e-12);
        assert!(result.maximum_relative_carter_drift < 2e-10);
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


    fn perturb_state(state: KerrCanonicalState, component: usize, delta: f64) -> KerrCanonicalState {
        let mut values = flatten(state);
        values[component] += delta;
        unflatten(values)
    }

    #[test]
    fn variational_fundamental_matrix_tracks_neighbouring_canonical_trajectories() {
        let black_hole = hole(0.7);
        let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let initial = canonical.state_from_separated(
            KerrMinoState::new(0.0, 10.0, 1.0, 0.0),
            constants,
            KerrMinoDirection::Decreasing,
            KerrMinoDirection::Increasing,
        ).unwrap();
        let integrator = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, options(),
        ).unwrap();

        let variational = integrator.integrate_fundamental_matrix(initial, 0.05).unwrap();
        assert!(variational.accepted_steps > 0);
        assert!(variational.samples.len() > 1);
        for index in 0..8 {
            for column in 0..8 {
                let expected = if index == column { 1.0 } else { 0.0 };
                assert!((variational.samples[0].fundamental_matrix[index][column] - expected).abs() < 1e-14);
            }
        }
        assert!(variational.maximum_fundamental_symplectic_residual < 1e-5);
        assert!(variational.samples.iter().all(|sample|
            sample.local_symplectic_lie_residual < 2e-10
                && sample.fundamental_symplectic_residual.is_finite()));

        // Compare the p_theta column of Phi with independently integrated
        // nearby canonical initial states. These perturbations test the full
        // phase-space flow; they are not claimed to remain on the null shell.
        let epsilon = 2e-5;
        let plus = integrator.integrate(perturb_state(initial, 6, epsilon), 0.05).unwrap();
        let minus = integrator.integrate(perturb_state(initial, 6, -epsilon), 0.05).unwrap();
        let plus_final = flatten(plus.samples.last().unwrap().state);
        let minus_final = flatten(minus.samples.last().unwrap().state);
        let tangent = &variational.samples.last().unwrap().fundamental_matrix;
        for row in 0..8 {
            let finite_difference = (plus_final[row] - minus_final[row]) / (2.0 * epsilon);
            let expected = tangent[row][6];
            assert!(
                (finite_difference - expected).abs() <= 2e-4 * (1.0 + finite_difference.abs()),
                "row={row}, finite_difference={finite_difference:.12e}, tangent={expected:.12e}"
            );
        }
    }

    fn normalized_direction(vector: [f64; 3]) -> [f64; 3] {
        let norm = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
        [vector[0] / norm, vector[1] / norm, vector[2] / norm]
    }

    #[test]
    fn finite_observer_tangent_bundle_matches_two_neighbouring_null_rays() {
        let black_hole = hole(0.7);
        let observer = KerrFiniteRadiusObserver::new(black_hole, 10.0, 1.0).unwrap();
        let direction = KerrLocalSkyDirection::new(
            -(1.0_f64 - 0.006_f64.powi(2) - 0.008_f64.powi(2)).sqrt(),
            0.006, -0.008,
        ).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        let integrator = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::Ingoing, options(),
        ).unwrap();
        let interval = 0.05;
        let bundle = integrator.integrate_observer_tangent_bundle(seed, interval).unwrap();
        let final_sample = bundle.samples.last().unwrap();
        assert!(bundle.samples.len() > 1);
        assert!(bundle.maximum_fundamental_symplectic_residual < 1e-5);
        assert!(bundle.samples.iter().all(|sample|
            sample.fundamental_symplectic_residual.is_finite()));

        // Perturb each local unit-sky direction and independently integrate
        // the resulting null ray. Compare the full phase-space derivatives.
        let n = [direction.radial, direction.polar, direction.azimuthal];
        let epsilon = 2e-5;
        for index in 0..2 {
            let basis = seed.local_screen_basis[index];
            let plus = normalized_direction([
                n[0] + epsilon * basis[0],
                n[1] + epsilon * basis[1],
                n[2] + epsilon * basis[2],
            ]);
            let minus = normalized_direction([
                n[0] - epsilon * basis[0],
                n[1] - epsilon * basis[1],
                n[2] - epsilon * basis[2],
            ]);
            let plus_seed = observer.canonical_seed(
                KerrLocalSkyDirection::new(plus[0], plus[1], plus[2]).unwrap(),
            ).unwrap();
            let minus_seed = observer.canonical_seed(
                KerrLocalSkyDirection::new(minus[0], minus[1], minus[2]).unwrap(),
            ).unwrap();
            let plus_final = flatten(
                integrator.integrate(plus_seed.state, interval).unwrap().samples.last().unwrap().state,
            );
            let minus_final = flatten(
                integrator.integrate(minus_seed.state, interval).unwrap().samples.last().unwrap().state,
            );
            let transported = if index == 0 {
                final_sample.first_phase_space_variation
            } else {
                final_sample.second_phase_space_variation
            };
            for component in 0..8 {
                let finite_difference = (plus_final[component] - minus_final[component])
                    / (2.0 * epsilon);
                assert!(
                    (finite_difference - transported[component]).abs()
                        <= 5e-4 * (1.0 + finite_difference.abs()),
                    "screen={index}, component={component}, fd={finite_difference:.8e}, tangent={:.8e}",
                    transported[component]
                );
            }
        }
    }

    #[test]
    fn observer_tangent_bundle_rejects_chart_mismatch() {
        let black_hole = hole(0.7);
        let observer = KerrFiniteRadiusObserver::new(black_hole, 10.0, 1.0).unwrap();
        let direction = KerrLocalSkyDirection::new(-1.0, 0.0, 0.0).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        let wrong_chart = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, options(),
        ).unwrap();
        assert!(matches!(
            wrong_chart.integrate_observer_tangent_bundle(seed, 0.01),
            Err(KerrIntegratorError::CoordinateChartMismatch { .. })
        ));
    }

    #[test]
    fn variational_integrator_rejects_nonpositive_affine_interval() {
        let black_hole = hole(0.5);
        let constants = KerrNullConstants::new(1.0, 0.0, 0.0).unwrap();
        let initial = KerrCanonicalHamiltonian::new(black_hole).state_from_separated(
            KerrMinoState::new(0.0, 10.0, core::f64::consts::FRAC_PI_2, 0.0),
            constants,
            KerrMinoDirection::Decreasing,
            KerrMinoDirection::Increasing,
        ).unwrap();
        let integrator = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, options(),
        ).unwrap();
        assert!(matches!(
            integrator.integrate_fundamental_matrix(initial, 0.0),
            Err(KerrIntegratorError::InvalidAffineInterval(_))
        ));
    }

    #[test]
    fn trajectory_records_all_three_constraint_channels_at_each_sample() {
        let black_hole = hole(0.7);
        let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let initial = canonical.state_from_separated(
            KerrMinoState::new(0.0, 10.0, 1.0, 0.0),
            constants,
            KerrMinoDirection::Decreasing,
            KerrMinoDirection::Increasing,
        ).unwrap();
        let result = KerrNullTrajectoryIntegrator::new(
            black_hole, KerrIntegrationChart::BoyerLindquist, options(),
        ).unwrap().integrate(initial, 0.2).unwrap();
        assert!(result.samples.len() > 1);
        for sample in &result.samples {
            assert!(sample.hamiltonian.abs() < 1e-9);
            assert!(sample.normalized_radial_first_integral_residual < 1e-9);
            assert!(sample.normalized_polar_first_integral_residual < 1e-9);
            assert!((sample.energy - 1.0).abs() < 1e-10);
            assert!((sample.angular_momentum_z_over_m - 2.0).abs() < 1e-10);
            assert!((sample.carter_constant_over_m2 - 3.0).abs() < 1e-9);
        }
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
