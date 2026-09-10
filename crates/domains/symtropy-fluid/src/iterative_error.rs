// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Iterative-convergence error observability for the reference projection.
//!
//! Pressure residuals are useful but live in different units from the velocity
//! errors used by the spatial/temporal verification ladder. This module measures
//! how much the projected velocity itself changes as the fixed Jacobi iteration
//! budget increases. The highest-iteration result is an explicitly declared
//! numerical reference for this sweep only; it is not treated as an exact
//! continuum solution.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::numerical_observability::{
    COMPRESSIVE_MODE_PROFILE_ID, MAX_NUMERICAL_OBSERVABILITY_POINTS,
};
use crate::reference::{
    MAX_PRESSURE_ITERATIONS, PeriodicMac2d, PeriodicMacConfig, ReferenceStateError,
    ReferenceStepError,
};

pub const ITERATIVE_ERROR_SWEEP_SCHEMA_ID: &str = "continuum-iterative-error-sweep-v0.1";
pub const HIGHEST_ITERATION_REFERENCE_POLICY_ID: &str =
    "highest-iteration-result-is-numerical-reference-not-truth-v0.1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IterativeProjectionPoint {
    pub pressure_iterations: usize,
    pub solver_profile: String,
    pub divergence_rms_after_per_s: f64,
    pub pressure_residual_rms_pa_per_m2: f64,
    /// RMS face-velocity difference from the highest-iteration result in this
    /// exact sweep. This is iterative solution change, not continuum error.
    pub velocity_rms_difference_to_reference_mps: f64,
    pub velocity_max_difference_to_reference_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IterativeErrorSweepReport {
    pub schema_id: String,
    pub reference_policy: String,
    pub field_profile: String,
    pub dt_s: f64,
    pub amplitude_mps: f64,
    pub reference_pressure_iterations: usize,
    pub reference_solver_profile: String,
    pub points: Vec<IterativeProjectionPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IterativeErrorSweepError {
    InvalidDt,
    InvalidAmplitude,
    TooFewPoints,
    TooManyPoints,
    InvalidIterationCount,
    NonIncreasingIterationCounts,
    NonFiniteDerivedMetric(&'static str),
    State(ReferenceStateError),
    Projection(ReferenceStepError),
}

impl fmt::Display for IterativeErrorSweepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDt => write!(f, "iterative error sweep dt must be finite and > 0"),
            Self::InvalidAmplitude => {
                write!(f, "iterative error sweep amplitude must be finite and > 0")
            }
            Self::TooFewPoints => write!(f, "iterative error sweep requires at least two points"),
            Self::TooManyPoints => write!(
                f,
                "iterative error sweep exceeds {MAX_NUMERICAL_OBSERVABILITY_POINTS} points"
            ),
            Self::InvalidIterationCount => write!(
                f,
                "pressure iteration counts must be in 1..={MAX_PRESSURE_ITERATIONS}"
            ),
            Self::NonIncreasingIterationCounts => {
                write!(f, "pressure iteration counts must be strictly increasing")
            }
            Self::NonFiniteDerivedMetric(name) => {
                write!(f, "iterative error metric {name} is non-finite")
            }
            Self::State(source) => write!(f, "iterative error state failed: {source}"),
            Self::Projection(source) => write!(f, "iterative error projection failed: {source}"),
        }
    }
}

impl std::error::Error for IterativeErrorSweepError {}

impl From<ReferenceStateError> for IterativeErrorSweepError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

#[derive(Clone, Debug)]
struct ProjectionCandidate {
    pressure_iterations: usize,
    solver_profile: String,
    divergence_rms_after_per_s: f64,
    pressure_residual_rms_pa_per_m2: f64,
    u_faces: Vec<f64>,
    v_faces: Vec<f64>,
}

/// Sweep fixed Jacobi iteration budgets while holding the discrete problem
/// completely constant, then express iterative solution change in m/s.
pub fn run_projection_iterative_error_sweep(
    base_config: PeriodicMacConfig,
    amplitude_mps: f64,
    dt_s: f64,
    iteration_counts: &[usize],
) -> Result<IterativeErrorSweepReport, IterativeErrorSweepError> {
    validate_request(amplitude_mps, dt_s, iteration_counts)?;

    // Validate the base profile and geometry before generating the shared field.
    PeriodicMac2d::zeros(base_config.clone())?;
    let (u_faces, v_faces) = compressive_mode_faces(&base_config, amplitude_mps);
    let field_profile = format!(
        "{COMPRESSIVE_MODE_PROFILE_ID};n={}x{};L={:016x},{:016x};a={:016x}",
        base_config.nx,
        base_config.ny,
        base_config.length_x_m.to_bits(),
        base_config.length_y_m.to_bits(),
        amplitude_mps.to_bits(),
    );

    let mut candidates = Vec::with_capacity(iteration_counts.len());
    for &pressure_iterations in iteration_counts {
        let mut config = base_config.clone();
        config.pressure_iterations = pressure_iterations;
        let solver_profile = config.profile_identity();
        let mut state = PeriodicMac2d::from_faces(config, u_faces.clone(), v_faces.clone())?;
        let projection = state
            .project_velocity(dt_s)
            .map_err(IterativeErrorSweepError::Projection)?;
        candidates.push(ProjectionCandidate {
            pressure_iterations,
            solver_profile,
            divergence_rms_after_per_s: projection.divergence_rms_after_per_s,
            pressure_residual_rms_pa_per_m2: projection.pressure_residual_rms_pa_per_m2,
            u_faces: state.u_faces().to_vec(),
            v_faces: state.v_faces().to_vec(),
        });
    }

    let reference = candidates
        .last()
        .expect("validated sweep always contains at least two points");
    let reference_pressure_iterations = reference.pressure_iterations;
    let reference_solver_profile = reference.solver_profile.clone();
    let reference_u = reference.u_faces.clone();
    let reference_v = reference.v_faces.clone();

    let mut points = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let (velocity_rms_difference_to_reference_mps, velocity_max_difference_to_reference_mps) =
            velocity_difference(
                &candidate.u_faces,
                &candidate.v_faces,
                &reference_u,
                &reference_v,
            )?;
        points.push(IterativeProjectionPoint {
            pressure_iterations: candidate.pressure_iterations,
            solver_profile: candidate.solver_profile,
            divergence_rms_after_per_s: candidate.divergence_rms_after_per_s,
            pressure_residual_rms_pa_per_m2: candidate.pressure_residual_rms_pa_per_m2,
            velocity_rms_difference_to_reference_mps,
            velocity_max_difference_to_reference_mps,
        });
    }

    Ok(IterativeErrorSweepReport {
        schema_id: ITERATIVE_ERROR_SWEEP_SCHEMA_ID.to_owned(),
        reference_policy: HIGHEST_ITERATION_REFERENCE_POLICY_ID.to_owned(),
        field_profile,
        dt_s,
        amplitude_mps,
        reference_pressure_iterations,
        reference_solver_profile,
        points,
    })
}

fn validate_request(
    amplitude_mps: f64,
    dt_s: f64,
    iteration_counts: &[usize],
) -> Result<(), IterativeErrorSweepError> {
    if !amplitude_mps.is_finite() || amplitude_mps <= 0.0 {
        return Err(IterativeErrorSweepError::InvalidAmplitude);
    }
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(IterativeErrorSweepError::InvalidDt);
    }
    if iteration_counts.len() < 2 {
        return Err(IterativeErrorSweepError::TooFewPoints);
    }
    if iteration_counts.len() > MAX_NUMERICAL_OBSERVABILITY_POINTS {
        return Err(IterativeErrorSweepError::TooManyPoints);
    }
    if iteration_counts
        .iter()
        .any(|count| *count == 0 || *count > MAX_PRESSURE_ITERATIONS)
    {
        return Err(IterativeErrorSweepError::InvalidIterationCount);
    }
    if iteration_counts.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(IterativeErrorSweepError::NonIncreasingIterationCounts);
    }
    Ok(())
}

fn compressive_mode_faces(config: &PeriodicMacConfig, amplitude_mps: f64) -> (Vec<f64>, Vec<f64>) {
    let mut u_faces = vec![0.0; config.nx * config.ny];
    let mut v_faces = vec![0.0; config.nx * config.ny];
    let dx = config.dx();
    let dy = config.dy();
    let kx = std::f64::consts::TAU / config.length_x_m;
    let ky = std::f64::consts::TAU / config.length_y_m;
    for j in 0..config.ny {
        for i in 0..config.nx {
            let index = j * config.nx + i;
            u_faces[index] = amplitude_mps * (kx * i as f64 * dx).sin();
            v_faces[index] = amplitude_mps * (ky * j as f64 * dy).sin();
        }
    }
    (u_faces, v_faces)
}

fn velocity_difference(
    u_faces: &[f64],
    v_faces: &[f64],
    reference_u: &[f64],
    reference_v: &[f64],
) -> Result<(f64, f64), IterativeErrorSweepError> {
    let mut squared_error = 0.0;
    let mut maximum_error = 0.0_f64;
    let mut count = 0usize;
    for (observed, reference) in u_faces
        .iter()
        .zip(reference_u)
        .chain(v_faces.iter().zip(reference_v))
    {
        let error = (observed - reference).abs();
        squared_error += error * error;
        maximum_error = maximum_error.max(error);
        count += 1;
    }
    let rms = (squared_error / count as f64).sqrt();
    if !rms.is_finite() {
        return Err(IterativeErrorSweepError::NonFiniteDerivedMetric(
            "velocity_rms_difference_to_reference_mps",
        ));
    }
    if !maximum_error.is_finite() {
        return Err(IterativeErrorSweepError::NonFiniteDerivedMetric(
            "velocity_max_difference_to_reference_mps",
        ));
    }
    Ok((rms, maximum_error))
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

    #[test]
    fn highest_iteration_point_is_zero_by_declared_reference_construction() {
        let report =
            run_projection_iterative_error_sweep(config(), 0.08, 0.001, &[1, 4, 16, 64])
                .unwrap();
        let reference = report.points.last().unwrap();
        assert_eq!(report.reference_pressure_iterations, 64);
        assert_eq!(reference.velocity_rms_difference_to_reference_mps, 0.0);
        assert_eq!(reference.velocity_max_difference_to_reference_mps, 0.0);
    }

    #[test]
    fn larger_iteration_budget_reduces_solution_change_for_control_fixture() {
        let report =
            run_projection_iterative_error_sweep(config(), 0.08, 0.001, &[1, 4, 16, 64, 256])
                .unwrap();
        assert!(
            report.points[3].velocity_rms_difference_to_reference_mps
                < report.points[0].velocity_rms_difference_to_reference_mps
        );
        assert!(
            report.points[3].pressure_residual_rms_pa_per_m2
                < report.points[0].pressure_residual_rms_pa_per_m2
        );
    }

    #[test]
    fn iterative_error_sweep_is_deterministic() {
        let first =
            run_projection_iterative_error_sweep(config(), 0.08, 0.001, &[1, 4, 16, 64])
                .unwrap();
        let second =
            run_projection_iterative_error_sweep(config(), 0.08, 0.001, &[1, 4, 16, 64])
                .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn malformed_iteration_sequences_fail_closed() {
        assert_eq!(
            run_projection_iterative_error_sweep(config(), 0.08, 0.001, &[4, 4]),
            Err(IterativeErrorSweepError::NonIncreasingIterationCounts)
        );
        assert_eq!(
            run_projection_iterative_error_sweep(config(), 0.08, 0.001, &[0, 4]),
            Err(IterativeErrorSweepError::InvalidIterationCount)
        );
    }
}
