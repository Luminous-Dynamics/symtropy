// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Same-case iterative sensitivity for the manufactured Taylor-Green control.
//!
//! Every point in a sweep uses the same grid, timestep, viscosity, forcing,
//! initial condition, and requested final time. Only the fixed pressure-
//! projection Jacobi iteration budget changes. This makes velocity differences
//! between points a defensible measure of iterative sensitivity for this
//! executable case. The highest-iteration point is still only a numerical
//! reference, not truth.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::iterative_error::HIGHEST_ITERATION_REFERENCE_POLICY_ID;
use crate::manufactured::{
    ManufacturedTaylorGreenError, ManufacturedTaylorGreenProfile,
    manufactured_acceleration_mps2,
};
use crate::numerical_observability::MAX_NUMERICAL_OBSERVABILITY_POINTS;
use crate::reference::{
    MAX_PRESSURE_ITERATIONS, PeriodicMac2d, PeriodicMacConfig, ReferenceDiagnosticError,
    ReferenceStateError, ReferenceStepError,
};

pub const MANUFACTURED_ITERATIVE_SWEEP_SCHEMA_ID: &str =
    "manufactured-taylor-green-iterative-sensitivity-v0.1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManufacturedIterativePoint {
    pub pressure_iterations: usize,
    pub solver_profile: String,
    pub actual_final_time_s: f64,
    pub maximum_observed_advective_cfl: f64,
    pub maximum_observed_divergence_rms_per_s: f64,
    pub maximum_observed_pressure_residual_rms_pa_per_m2: f64,
    pub analytical_velocity_rms_error_mps: f64,
    pub analytical_velocity_max_error_mps: f64,
    pub analytical_kinetic_energy_relative_error: f64,
    /// Final velocity difference from the highest-iteration run of this exact
    /// manufactured case. Zero at the reference endpoint by construction.
    pub velocity_rms_difference_to_reference_mps: f64,
    pub velocity_max_difference_to_reference_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManufacturedIterativeSweepReport {
    pub schema_id: String,
    pub numerical_reference_policy: String,
    /// Identity of all case inputs that are held fixed across the sweep. The
    /// pressure-iteration count is intentionally excluded and lives per point.
    pub fixed_case_profile: String,
    pub manufactured_profile: String,
    pub resolution: usize,
    pub dt_s: f64,
    pub steps: usize,
    pub requested_final_time_s: f64,
    pub reference_pressure_iterations: usize,
    pub reference_solver_profile: String,
    pub points: Vec<ManufacturedIterativePoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ManufacturedIterativeError {
    InvalidFinalTime,
    InvalidStepCount,
    TooFewPoints,
    TooManyPoints,
    InvalidIterationCount,
    NonIncreasingIterationCounts,
    NonCanonicalDomain,
    NonFiniteDerivedMetric(&'static str),
    Manufactured(ManufacturedTaylorGreenError),
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
    MissingMeasuredEnergy,
}

impl fmt::Display for ManufacturedIterativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFinalTime => write!(f, "final_time_s must be finite and > 0"),
            Self::InvalidStepCount => write!(f, "manufactured iterative sweep requires steps > 0"),
            Self::TooFewPoints => {
                write!(f, "manufactured iterative sweep requires at least two points")
            }
            Self::TooManyPoints => write!(
                f,
                "manufactured iterative sweep exceeds {MAX_NUMERICAL_OBSERVABILITY_POINTS} points"
            ),
            Self::InvalidIterationCount => write!(
                f,
                "pressure iteration counts must be in 1..={MAX_PRESSURE_ITERATIONS}"
            ),
            Self::NonIncreasingIterationCounts => {
                write!(f, "pressure iteration counts must be strictly increasing")
            }
            Self::NonCanonicalDomain => write!(
                f,
                "manufactured iterative V0.1 requires a square periodic grid/domain"
            ),
            Self::NonFiniteDerivedMetric(name) => {
                write!(f, "manufactured iterative metric {name} is non-finite")
            }
            Self::Manufactured(source) => write!(f, "manufactured profile failed: {source}"),
            Self::State(source) => write!(f, "manufactured iterative state failed: {source}"),
            Self::Step(source) => write!(f, "manufactured iterative step failed: {source}"),
            Self::Diagnostic(source) => {
                write!(f, "manufactured iterative diagnostic failed: {source}")
            }
            Self::MissingMeasuredEnergy => {
                write!(f, "manufactured iterative comparison requires measured kinetic energy")
            }
        }
    }
}

impl std::error::Error for ManufacturedIterativeError {}

impl From<ManufacturedTaylorGreenError> for ManufacturedIterativeError {
    fn from(value: ManufacturedTaylorGreenError) -> Self {
        Self::Manufactured(value)
    }
}

impl From<ReferenceStateError> for ManufacturedIterativeError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for ManufacturedIterativeError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for ManufacturedIterativeError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

#[derive(Clone, Debug)]
struct CompletedCandidate {
    pressure_iterations: usize,
    solver_profile: String,
    actual_final_time_s: f64,
    maximum_observed_advective_cfl: f64,
    maximum_observed_divergence_rms_per_s: f64,
    maximum_observed_pressure_residual_rms_pa_per_m2: f64,
    analytical_velocity_rms_error_mps: f64,
    analytical_velocity_max_error_mps: f64,
    analytical_kinetic_energy_relative_error: f64,
    u_faces: Vec<f64>,
    v_faces: Vec<f64>,
}

pub fn run_manufactured_iterative_sensitivity_sweep(
    base_config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    final_time_s: f64,
    steps: usize,
    iteration_counts: &[usize],
) -> Result<ManufacturedIterativeSweepReport, ManufacturedIterativeError> {
    validate_request(&base_config, profile, final_time_s, steps, iteration_counts)?;
    let dt_s = final_time_s / steps as f64;
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(ManufacturedIterativeError::InvalidFinalTime);
    }

    let initial_amplitude = profile.amplitude_mps(0.0)?;
    let manufactured_profile = profile.profile_identity();
    let fixed_case_profile = fixed_case_profile(
        &base_config,
        &manufactured_profile,
        final_time_s,
        steps,
    );
    let forcing_config = base_config.clone();
    let mut candidates = Vec::with_capacity(iteration_counts.len());

    for &pressure_iterations in iteration_counts {
        let mut config = base_config.clone();
        config.pressure_iterations = pressure_iterations;
        let solver_profile = config.profile_identity();
        let exact_config = config.clone();
        let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude)?;
        let mut maximum_observed_advective_cfl = 0.0_f64;
        let mut maximum_observed_divergence_rms_per_s = state.divergence_rms_per_s();
        let mut maximum_observed_pressure_residual_rms_pa_per_m2 = 0.0_f64;

        for _ in 0..steps {
            let step = state.step_with_acceleration(dt_s, |position, time| {
                manufactured_acceleration_mps2(&forcing_config, profile, position, time)
                    .unwrap_or([f64::NAN, f64::NAN])
            })?;
            maximum_observed_advective_cfl =
                maximum_observed_advective_cfl.max(step.max_advective_cfl);
            maximum_observed_divergence_rms_per_s = maximum_observed_divergence_rms_per_s
                .max(step.projection.divergence_rms_after_per_s);
            maximum_observed_pressure_residual_rms_pa_per_m2 =
                maximum_observed_pressure_residual_rms_pa_per_m2
                    .max(step.projection.pressure_residual_rms_pa_per_m2);
        }

        let actual_final_time_s = state.time_s();
        let exact_final_amplitude = profile.amplitude_mps(actual_final_time_s)?;
        let exact = PeriodicMac2d::taylor_green(exact_config, exact_final_amplitude)?;
        let (analytical_velocity_rms_error_mps, analytical_velocity_max_error_mps) =
            velocity_difference(&state, &exact)?;
        let state_energy = measured_energy(&state)?;
        let exact_energy = measured_energy(&exact)?;
        let analytical_kinetic_energy_relative_error =
            (state_energy - exact_energy).abs() / exact_energy;
        ensure_finite(
            "analytical_kinetic_energy_relative_error",
            analytical_kinetic_energy_relative_error,
        )?;

        candidates.push(CompletedCandidate {
            pressure_iterations,
            solver_profile,
            actual_final_time_s,
            maximum_observed_advective_cfl,
            maximum_observed_divergence_rms_per_s,
            maximum_observed_pressure_residual_rms_pa_per_m2,
            analytical_velocity_rms_error_mps,
            analytical_velocity_max_error_mps,
            analytical_kinetic_energy_relative_error,
            u_faces: state.u_faces().to_vec(),
            v_faces: state.v_faces().to_vec(),
        });
    }

    let reference = candidates
        .last()
        .expect("validated sweep contains at least two points");
    let reference_pressure_iterations = reference.pressure_iterations;
    let reference_solver_profile = reference.solver_profile.clone();
    let reference_u = reference.u_faces.clone();
    let reference_v = reference.v_faces.clone();

    let mut points = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let (velocity_rms_difference_to_reference_mps, velocity_max_difference_to_reference_mps) =
            raw_velocity_difference(
                &candidate.u_faces,
                &candidate.v_faces,
                &reference_u,
                &reference_v,
            )?;
        points.push(ManufacturedIterativePoint {
            pressure_iterations: candidate.pressure_iterations,
            solver_profile: candidate.solver_profile,
            actual_final_time_s: candidate.actual_final_time_s,
            maximum_observed_advective_cfl: candidate.maximum_observed_advective_cfl,
            maximum_observed_divergence_rms_per_s: candidate
                .maximum_observed_divergence_rms_per_s,
            maximum_observed_pressure_residual_rms_pa_per_m2: candidate
                .maximum_observed_pressure_residual_rms_pa_per_m2,
            analytical_velocity_rms_error_mps: candidate.analytical_velocity_rms_error_mps,
            analytical_velocity_max_error_mps: candidate.analytical_velocity_max_error_mps,
            analytical_kinetic_energy_relative_error: candidate
                .analytical_kinetic_energy_relative_error,
            velocity_rms_difference_to_reference_mps,
            velocity_max_difference_to_reference_mps,
        });
    }

    Ok(ManufacturedIterativeSweepReport {
        schema_id: MANUFACTURED_ITERATIVE_SWEEP_SCHEMA_ID.to_owned(),
        numerical_reference_policy: HIGHEST_ITERATION_REFERENCE_POLICY_ID.to_owned(),
        fixed_case_profile,
        manufactured_profile,
        resolution: base_config.nx,
        dt_s,
        steps,
        requested_final_time_s: final_time_s,
        reference_pressure_iterations,
        reference_solver_profile,
        points,
    })
}

fn validate_request(
    config: &PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    final_time_s: f64,
    steps: usize,
    iteration_counts: &[usize],
) -> Result<(), ManufacturedIterativeError> {
    config.validate().map_err(ReferenceStateError::Config)?;
    profile.validate()?;
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(ManufacturedIterativeError::NonCanonicalDomain);
    }
    if !final_time_s.is_finite() || final_time_s <= 0.0 {
        return Err(ManufacturedIterativeError::InvalidFinalTime);
    }
    if steps == 0 {
        return Err(ManufacturedIterativeError::InvalidStepCount);
    }
    if iteration_counts.len() < 2 {
        return Err(ManufacturedIterativeError::TooFewPoints);
    }
    if iteration_counts.len() > MAX_NUMERICAL_OBSERVABILITY_POINTS {
        return Err(ManufacturedIterativeError::TooManyPoints);
    }
    if iteration_counts
        .iter()
        .any(|count| *count == 0 || *count > MAX_PRESSURE_ITERATIONS)
    {
        return Err(ManufacturedIterativeError::InvalidIterationCount);
    }
    if iteration_counts.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(ManufacturedIterativeError::NonIncreasingIterationCounts);
    }
    Ok(())
}

fn fixed_case_profile(
    config: &PeriodicMacConfig,
    manufactured_profile: &str,
    final_time_s: f64,
    steps: usize,
) -> String {
    format!(
        "manufactured-iterative-fixed-case-v0.1;n={}x{};L={:016x},{:016x};d={:016x};r={:016x};v={:016x};c={:016x};q={:016x};T={:016x};steps={};m={}",
        config.nx,
        config.ny,
        config.length_x_m.to_bits(),
        config.length_y_m.to_bits(),
        config.slab_depth_m.to_bits(),
        config.density_kg_m3.to_bits(),
        config.kinematic_viscosity_m2_s.to_bits(),
        config.max_advective_cfl.to_bits(),
        config.max_diffusion_number.to_bits(),
        final_time_s.to_bits(),
        steps,
        manufactured_profile,
    )
}

fn velocity_difference(
    actual: &PeriodicMac2d,
    expected: &PeriodicMac2d,
) -> Result<(f64, f64), ManufacturedIterativeError> {
    raw_velocity_difference(
        actual.u_faces(),
        actual.v_faces(),
        expected.u_faces(),
        expected.v_faces(),
    )
}

fn raw_velocity_difference(
    u_faces: &[f64],
    v_faces: &[f64],
    expected_u: &[f64],
    expected_v: &[f64],
) -> Result<(f64, f64), ManufacturedIterativeError> {
    let mut squared_error = 0.0;
    let mut maximum_error = 0.0_f64;
    let mut count = 0usize;
    for (observed, expected) in u_faces
        .iter()
        .zip(expected_u)
        .chain(v_faces.iter().zip(expected_v))
    {
        let error = (observed - expected).abs();
        squared_error += error * error;
        maximum_error = maximum_error.max(error);
        count += 1;
    }
    let rms = (squared_error / count as f64).sqrt();
    ensure_finite("velocity_rms_difference_mps", rms)?;
    ensure_finite("velocity_max_difference_mps", maximum_error)?;
    Ok((rms, maximum_error))
}

fn measured_energy(state: &PeriodicMac2d) -> Result<f64, ManufacturedIterativeError> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(ManufacturedIterativeError::MissingMeasuredEnergy)
}

fn ensure_finite(
    name: &'static str,
    value: f64,
) -> Result<(), ManufacturedIterativeError> {
    if !value.is_finite() {
        return Err(ManufacturedIterativeError::NonFiniteDerivedMetric(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PeriodicMacConfig {
        PeriodicMacConfig {
            nx: 12,
            ny: 12,
            length_x_m: std::f64::consts::TAU,
            length_y_m: std::f64::consts::TAU,
            slab_depth_m: 1.0,
            density_kg_m3: 1.0,
            kinematic_viscosity_m2_s: 0.01,
            pressure_iterations: 64,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }

    fn profile() -> ManufacturedTaylorGreenProfile {
        ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.08,
            modulation_fraction: 0.2,
            angular_frequency_rad_s: 1.5,
        }
    }

    #[test]
    fn highest_iteration_endpoint_is_zero_only_by_reference_construction() {
        let report = run_manufactured_iterative_sensitivity_sweep(
            config(),
            profile(),
            0.004,
            16,
            &[4, 16, 64],
        )
        .unwrap();
        let reference = report.points.last().unwrap();
        assert_eq!(report.reference_pressure_iterations, 64);
        assert_eq!(reference.velocity_rms_difference_to_reference_mps, 0.0);
        assert_eq!(reference.velocity_max_difference_to_reference_mps, 0.0);
        assert!(reference.analytical_velocity_rms_error_mps >= 0.0);
    }

    #[test]
    fn increasing_iterations_reduce_same_case_solution_sensitivity() {
        let report = run_manufactured_iterative_sensitivity_sweep(
            config(),
            profile(),
            0.004,
            16,
            &[1, 4, 16, 64, 256],
        )
        .unwrap();
        assert!(
            report.points[3].velocity_rms_difference_to_reference_mps
                < report.points[0].velocity_rms_difference_to_reference_mps
        );
        assert!(
            report.points[3].maximum_observed_pressure_residual_rms_pa_per_m2
                <= report.points[0].maximum_observed_pressure_residual_rms_pa_per_m2
        );
    }

    #[test]
    fn sweep_is_deterministic() {
        let first = run_manufactured_iterative_sensitivity_sweep(
            config(),
            profile(),
            0.004,
            8,
            &[4, 16, 64],
        )
        .unwrap();
        let second = run_manufactured_iterative_sensitivity_sweep(
            config(),
            profile(),
            0.004,
            8,
            &[4, 16, 64],
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn all_points_use_same_accumulated_final_time() {
        let report = run_manufactured_iterative_sensitivity_sweep(
            config(),
            profile(),
            0.004,
            7,
            &[4, 16, 64],
        )
        .unwrap();
        let first_time = report.points[0].actual_final_time_s;
        assert!(report
            .points
            .iter()
            .all(|point| point.actual_final_time_s.to_bits() == first_time.to_bits()));
    }

    #[test]
    fn malformed_iteration_sequence_fails_closed() {
        assert_eq!(
            run_manufactured_iterative_sensitivity_sweep(
                config(),
                profile(),
                0.004,
                8,
                &[16, 16],
            ),
            Err(ManufacturedIterativeError::NonIncreasingIterationCounts)
        );
    }
}
