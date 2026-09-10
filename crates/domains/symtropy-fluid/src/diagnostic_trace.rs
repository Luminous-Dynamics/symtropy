// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Stepwise diagnostic traces for smooth continuum verification cases.
//!
//! The strict ladder retains one point per resolution/timestep case. This layer
//! retains what happened *inside* each completed case: full solver-independent
//! diagnostics, projection contraction, fixed projection iteration count,
//! analytical error, resolved energy change, and (for manufactured forcing)
//! discrete forcing divergence. These are measurements, not acceptance policy.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::convergence::{TaylorGreenError, TaylorGreenErrorKind, compare_taylor_green};
use crate::manufactured::{
    ManufacturedTaylorGreenError, ManufacturedTaylorGreenProfile,
    manufactured_acceleration_mps2,
};
use crate::reference::{
    PeriodicMac2d, PeriodicMacConfig, ReferenceDiagnosticError, ReferenceStateError,
    ReferenceStepError, ReferenceStepReport,
};
use crate::validation::ContinuumDiagnosticSample;
use crate::verification_ladder::VerificationCaseKind;

pub const DIAGNOSTIC_TRACE_SCHEMA_ID: &str = "continuum-diagnostic-trace-v0.1";
pub const MAX_DIAGNOSTIC_TRACE_STEPS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalErrorSample {
    pub time_s: f64,
    pub velocity_rms_error_mps: f64,
    pub velocity_max_error_mps: f64,
    pub kinetic_energy_relative_error: f64,
}

impl From<TaylorGreenError> for AnalyticalErrorSample {
    fn from(value: TaylorGreenError) -> Self {
        Self {
            time_s: value.time_s,
            velocity_rms_error_mps: value.velocity_rms_error_mps,
            velocity_max_error_mps: value.velocity_max_error_mps,
            kinetic_energy_relative_error: value.kinetic_energy_relative_error,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionDiagnosticSample {
    pub divergence_rms_before_per_s: f64,
    pub divergence_rms_after_per_s: f64,
    /// `after / before` when the pre-projection divergence is positive.
    /// Lower is stronger contraction. `None` means the ratio is undefined
    /// because the pre-projection divergence was exactly zero.
    pub divergence_reduction_ratio: Option<f64>,
    pub pressure_residual_rms_pa_per_m2: f64,
    /// The V0 reference solver uses a fixed Jacobi count, so the declared
    /// iteration count is evidence even though it is not a convergence verdict.
    pub pressure_iterations_executed: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticTraceStep {
    pub step_index: usize,
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub dt_s: f64,
    pub max_advective_cfl: f64,
    pub diffusion_number: f64,
    pub combined_explicit_number: f64,
    pub projection: ProjectionDiagnosticSample,
    pub diagnostics: ContinuumDiagnosticSample,
    pub analytical_error: AnalyticalErrorSample,
    /// Positive means the resolved kinetic energy decreased over this step;
    /// negative means it increased. This includes physical, numerical, and
    /// forcing contributions and is deliberately not labeled physical
    /// dissipation.
    pub resolved_energy_decay_w: f64,
    /// Discrete MAC divergence of the manufactured body-acceleration field.
    /// Passive cases record `None` because no manufactured forcing is present.
    pub forcing_divergence_rms_per_s2: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticTraceExtrema {
    pub maximum_resolved_speed_mps: f64,
    pub maximum_vorticity_per_s: f64,
    pub maximum_strain_rate_per_s: f64,
    pub maximum_pressure_gradient_pa_per_m: f64,
    pub maximum_divergence_rms_per_s: f64,
    pub maximum_cfl: f64,
    pub maximum_pressure_residual_rms_pa_per_m2: f64,
    pub maximum_non_finite_state_count: u64,
    pub maximum_resolved_energy_decay_w: f64,
    pub maximum_resolved_energy_gain_w: f64,
    pub maximum_forcing_divergence_rms_per_s2: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuumDiagnosticTraceReport {
    pub schema_id: String,
    pub case_kind: VerificationCaseKind,
    pub solver_profile: String,
    pub manufactured_profile: Option<String>,
    pub initial_diagnostics: ContinuumDiagnosticSample,
    pub initial_analytical_error: AnalyticalErrorSample,
    pub steps: Vec<DiagnosticTraceStep>,
    pub extrema: DiagnosticTraceExtrema,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticTraceError {
    InvalidDt,
    InvalidStepCount,
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
    PassiveComparator(TaylorGreenErrorKind),
    Manufactured(ManufacturedTaylorGreenError),
    MissingMeasuredEnergy,
    NonFiniteDerivedMetric(&'static str),
}

impl fmt::Display for DiagnosticTraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDt => write!(f, "diagnostic trace dt must be finite and > 0"),
            Self::InvalidStepCount => write!(
                f,
                "diagnostic trace step count must be in 1..={MAX_DIAGNOSTIC_TRACE_STEPS}"
            ),
            Self::State(source) => write!(f, "diagnostic trace state failed: {source}"),
            Self::Step(source) => write!(f, "diagnostic trace step failed: {source}"),
            Self::Diagnostic(source) => write!(f, "diagnostic trace measurement failed: {source}"),
            Self::PassiveComparator(source) => {
                write!(f, "diagnostic trace analytical comparison failed: {source}")
            }
            Self::Manufactured(source) => {
                write!(f, "diagnostic trace manufactured comparison failed: {source}")
            }
            Self::MissingMeasuredEnergy => {
                write!(f, "diagnostic trace requires measured kinetic energy")
            }
            Self::NonFiniteDerivedMetric(name) => {
                write!(f, "diagnostic trace derived metric {name} is non-finite")
            }
        }
    }
}

impl std::error::Error for DiagnosticTraceError {}

impl From<ReferenceStateError> for DiagnosticTraceError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for DiagnosticTraceError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for DiagnosticTraceError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

pub fn run_passive_taylor_green_diagnostic_trace(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    dt_s: f64,
    steps: usize,
) -> Result<ContinuumDiagnosticTraceReport, DiagnosticTraceError> {
    validate_trace_request(dt_s, steps)?;
    let solver_profile = config.profile_identity();
    let pressure_iterations = config.pressure_iterations;
    let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude_mps)?;
    let initial_diagnostics = state.diagnostics()?;
    let initial_analytical_error = compare_taylor_green(&state, initial_amplitude_mps)
        .map_err(DiagnosticTraceError::PassiveComparator)?
        .into();
    let mut extrema = DiagnosticTraceExtrema::from_initial(&initial_diagnostics)?;
    let mut previous_energy_j = measured_energy(&initial_diagnostics)?;
    let mut trace_steps = Vec::with_capacity(steps);

    for step_index in 0..steps {
        let step = state.step(dt_s)?;
        let diagnostics = state.diagnostics()?;
        let analytical_error = compare_taylor_green(&state, initial_amplitude_mps)
            .map_err(DiagnosticTraceError::PassiveComparator)?
            .into();
        let current_energy_j = measured_energy(&diagnostics)?;
        let resolved_energy_decay_w = (previous_energy_j - current_energy_j) / dt_s;
        ensure_finite("resolved_energy_decay_w", resolved_energy_decay_w)?;
        let projection = projection_sample(&step, pressure_iterations)?;
        let record = DiagnosticTraceStep {
            step_index,
            start_time_s: step.start_time_s,
            end_time_s: step.end_time_s,
            dt_s: step.dt_s,
            max_advective_cfl: step.max_advective_cfl,
            diffusion_number: step.diffusion_number,
            combined_explicit_number: step.combined_explicit_number,
            projection,
            diagnostics,
            analytical_error,
            resolved_energy_decay_w,
            forcing_divergence_rms_per_s2: None,
        };
        extrema.observe(&record)?;
        previous_energy_j = current_energy_j;
        trace_steps.push(record);
    }

    Ok(ContinuumDiagnosticTraceReport {
        schema_id: DIAGNOSTIC_TRACE_SCHEMA_ID.to_owned(),
        case_kind: VerificationCaseKind::PassiveTaylorGreen,
        solver_profile,
        manufactured_profile: None,
        initial_diagnostics,
        initial_analytical_error,
        steps: trace_steps,
        extrema,
    })
}

pub fn run_manufactured_taylor_green_diagnostic_trace(
    config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    dt_s: f64,
    steps: usize,
) -> Result<ContinuumDiagnosticTraceReport, DiagnosticTraceError> {
    validate_trace_request(dt_s, steps)?;
    profile
        .validate()
        .map_err(DiagnosticTraceError::Manufactured)?;
    let solver_profile = config.profile_identity();
    let manufactured_profile = profile.profile_identity();
    let pressure_iterations = config.pressure_iterations;
    let forcing_config = config.clone();
    let initial_amplitude = profile
        .amplitude_mps(0.0)
        .map_err(DiagnosticTraceError::Manufactured)?;
    let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude)?;
    let initial_diagnostics = state.diagnostics()?;
    let initial_analytical_error = compare_manufactured_state(&state, profile)?;
    let mut extrema = DiagnosticTraceExtrema::from_initial(&initial_diagnostics)?;
    let mut previous_energy_j = measured_energy(&initial_diagnostics)?;
    let mut trace_steps = Vec::with_capacity(steps);

    for step_index in 0..steps {
        let forcing_time_s = state.time_s();
        let forcing_divergence_rms_per_s2 = forcing_divergence_rms(
            &forcing_config,
            profile,
            forcing_time_s,
        )?;
        let step = state.step_with_acceleration(dt_s, |position, time| {
            manufactured_acceleration_mps2(&forcing_config, profile, position, time)
                .unwrap_or([f64::NAN, f64::NAN])
        })?;
        let diagnostics = state.diagnostics()?;
        let analytical_error = compare_manufactured_state(&state, profile)?;
        let current_energy_j = measured_energy(&diagnostics)?;
        let resolved_energy_decay_w = (previous_energy_j - current_energy_j) / dt_s;
        ensure_finite("resolved_energy_decay_w", resolved_energy_decay_w)?;
        let projection = projection_sample(&step, pressure_iterations)?;
        let record = DiagnosticTraceStep {
            step_index,
            start_time_s: step.start_time_s,
            end_time_s: step.end_time_s,
            dt_s: step.dt_s,
            max_advective_cfl: step.max_advective_cfl,
            diffusion_number: step.diffusion_number,
            combined_explicit_number: step.combined_explicit_number,
            projection,
            diagnostics,
            analytical_error,
            resolved_energy_decay_w,
            forcing_divergence_rms_per_s2: Some(forcing_divergence_rms_per_s2),
        };
        extrema.observe(&record)?;
        previous_energy_j = current_energy_j;
        trace_steps.push(record);
    }

    Ok(ContinuumDiagnosticTraceReport {
        schema_id: DIAGNOSTIC_TRACE_SCHEMA_ID.to_owned(),
        case_kind: VerificationCaseKind::ManufacturedTaylorGreen,
        solver_profile,
        manufactured_profile: Some(manufactured_profile),
        initial_diagnostics,
        initial_analytical_error,
        steps: trace_steps,
        extrema,
    })
}

fn validate_trace_request(dt_s: f64, steps: usize) -> Result<(), DiagnosticTraceError> {
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(DiagnosticTraceError::InvalidDt);
    }
    if steps == 0 || steps > MAX_DIAGNOSTIC_TRACE_STEPS {
        return Err(DiagnosticTraceError::InvalidStepCount);
    }
    Ok(())
}

fn projection_sample(
    step: &ReferenceStepReport,
    pressure_iterations: usize,
) -> Result<ProjectionDiagnosticSample, DiagnosticTraceError> {
    let before = step.projection.divergence_rms_before_per_s;
    let after = step.projection.divergence_rms_after_per_s;
    let divergence_reduction_ratio = if before > 0.0 {
        let ratio = after / before;
        ensure_finite("projection_divergence_reduction_ratio", ratio)?;
        Some(ratio)
    } else {
        None
    };
    Ok(ProjectionDiagnosticSample {
        divergence_rms_before_per_s: before,
        divergence_rms_after_per_s: after,
        divergence_reduction_ratio,
        pressure_residual_rms_pa_per_m2: step.projection.pressure_residual_rms_pa_per_m2,
        pressure_iterations_executed: pressure_iterations,
    })
}

fn compare_manufactured_state(
    state: &PeriodicMac2d,
    profile: ManufacturedTaylorGreenProfile,
) -> Result<AnalyticalErrorSample, DiagnosticTraceError> {
    let exact_amplitude = profile
        .amplitude_mps(state.time_s())
        .map_err(DiagnosticTraceError::Manufactured)?;
    let exact = PeriodicMac2d::taylor_green(state.config().clone(), exact_amplitude)?;
    let mut squared_error = 0.0;
    let mut maximum_error = 0.0_f64;
    let mut count = 0usize;

    for (observed, expected) in state
        .u_faces()
        .iter()
        .zip(exact.u_faces())
        .chain(state.v_faces().iter().zip(exact.v_faces()))
    {
        let error = (observed - expected).abs();
        squared_error += error * error;
        maximum_error = maximum_error.max(error);
        count += 1;
    }

    let velocity_rms_error_mps = (squared_error / count as f64).sqrt();
    let actual_energy = measured_energy(&state.diagnostics()?)?;
    let exact_energy = measured_energy(&exact.diagnostics()?)?;
    let kinetic_energy_relative_error = (actual_energy - exact_energy).abs() / exact_energy;
    ensure_finite("manufactured_velocity_rms_error_mps", velocity_rms_error_mps)?;
    ensure_finite("manufactured_velocity_max_error_mps", maximum_error)?;
    ensure_finite(
        "manufactured_kinetic_energy_relative_error",
        kinetic_energy_relative_error,
    )?;

    Ok(AnalyticalErrorSample {
        time_s: state.time_s(),
        velocity_rms_error_mps,
        velocity_max_error_mps: maximum_error,
        kinetic_energy_relative_error,
    })
}

fn forcing_divergence_rms(
    config: &PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    time_s: f64,
) -> Result<f64, DiagnosticTraceError> {
    let dx = config.dx();
    let dy = config.dy();
    let cells = config.nx * config.ny;
    let mut ax = vec![0.0; cells];
    let mut ay = vec![0.0; cells];

    for j in 0..config.ny {
        for i in 0..config.nx {
            let index = j * config.nx + i;
            ax[index] = manufactured_acceleration_mps2(
                config,
                profile,
                [i as f64 * dx, (j as f64 + 0.5) * dy],
                time_s,
            )
            .map_err(DiagnosticTraceError::Manufactured)?[0];
            ay[index] = manufactured_acceleration_mps2(
                config,
                profile,
                [(i as f64 + 0.5) * dx, j as f64 * dy],
                time_s,
            )
            .map_err(DiagnosticTraceError::Manufactured)?[1];
        }
    }

    let mut squared = 0.0;
    for j in 0..config.ny {
        let north = if j + 1 == config.ny { 0 } else { j + 1 };
        for i in 0..config.nx {
            let east = if i + 1 == config.nx { 0 } else { i + 1 };
            let index = j * config.nx + i;
            let divergence = (ax[j * config.nx + east] - ax[index]) / dx
                + (ay[north * config.nx + i] - ay[index]) / dy;
            squared += divergence * divergence;
        }
    }
    let value = (squared / cells as f64).sqrt();
    ensure_finite("forcing_divergence_rms_per_s2", value)?;
    Ok(value)
}

fn measured_energy(sample: &ContinuumDiagnosticSample) -> Result<f64, DiagnosticTraceError> {
    sample
        .kinetic_energy_j
        .measured_value()
        .ok_or(DiagnosticTraceError::MissingMeasuredEnergy)
}

fn measured_or_zero(value: Option<f64>) -> f64 {
    value.unwrap_or(0.0)
}

fn ensure_finite(name: &'static str, value: f64) -> Result<(), DiagnosticTraceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(DiagnosticTraceError::NonFiniteDerivedMetric(name))
    }
}

impl DiagnosticTraceExtrema {
    fn from_initial(sample: &ContinuumDiagnosticSample) -> Result<Self, DiagnosticTraceError> {
        let maximum_resolved_speed_mps = measured_or_zero(sample.max_resolved_speed_mps.measured_value());
        let maximum_vorticity_per_s = measured_or_zero(sample.max_vorticity_per_s.measured_value());
        let maximum_strain_rate_per_s = measured_or_zero(sample.max_strain_rate_per_s.measured_value());
        let maximum_pressure_gradient_pa_per_m =
            measured_or_zero(sample.max_pressure_gradient_pa_per_m.measured_value());
        let maximum_divergence_rms_per_s =
            measured_or_zero(sample.divergence_rms_per_s.measured_value());
        let maximum_cfl = measured_or_zero(sample.max_cfl.measured_value());
        let maximum_pressure_residual_rms_pa_per_m2 =
            measured_or_zero(sample.solver_residual.measured_value());

        for (name, value) in [
            ("maximum_resolved_speed_mps", maximum_resolved_speed_mps),
            ("maximum_vorticity_per_s", maximum_vorticity_per_s),
            ("maximum_strain_rate_per_s", maximum_strain_rate_per_s),
            (
                "maximum_pressure_gradient_pa_per_m",
                maximum_pressure_gradient_pa_per_m,
            ),
            (
                "maximum_divergence_rms_per_s",
                maximum_divergence_rms_per_s,
            ),
            ("maximum_cfl", maximum_cfl),
            (
                "maximum_pressure_residual_rms_pa_per_m2",
                maximum_pressure_residual_rms_pa_per_m2,
            ),
        ] {
            ensure_finite(name, value)?;
        }

        Ok(Self {
            maximum_resolved_speed_mps,
            maximum_vorticity_per_s,
            maximum_strain_rate_per_s,
            maximum_pressure_gradient_pa_per_m,
            maximum_divergence_rms_per_s,
            maximum_cfl,
            maximum_pressure_residual_rms_pa_per_m2,
            maximum_non_finite_state_count: sample.non_finite_state_count,
            maximum_resolved_energy_decay_w: 0.0,
            maximum_resolved_energy_gain_w: 0.0,
            maximum_forcing_divergence_rms_per_s2: None,
        })
    }

    fn observe(&mut self, step: &DiagnosticTraceStep) -> Result<(), DiagnosticTraceError> {
        self.maximum_resolved_speed_mps = self.maximum_resolved_speed_mps.max(measured_or_zero(
            step.diagnostics.max_resolved_speed_mps.measured_value(),
        ));
        self.maximum_vorticity_per_s = self.maximum_vorticity_per_s.max(measured_or_zero(
            step.diagnostics.max_vorticity_per_s.measured_value(),
        ));
        self.maximum_strain_rate_per_s = self.maximum_strain_rate_per_s.max(measured_or_zero(
            step.diagnostics.max_strain_rate_per_s.measured_value(),
        ));
        self.maximum_pressure_gradient_pa_per_m = self.maximum_pressure_gradient_pa_per_m.max(
            measured_or_zero(
                step.diagnostics
                    .max_pressure_gradient_pa_per_m
                    .measured_value(),
            ),
        );
        self.maximum_divergence_rms_per_s = self.maximum_divergence_rms_per_s.max(
            measured_or_zero(step.diagnostics.divergence_rms_per_s.measured_value()),
        );
        self.maximum_cfl = self
            .maximum_cfl
            .max(measured_or_zero(step.diagnostics.max_cfl.measured_value()));
        self.maximum_pressure_residual_rms_pa_per_m2 = self
            .maximum_pressure_residual_rms_pa_per_m2
            .max(measured_or_zero(step.diagnostics.solver_residual.measured_value()));
        self.maximum_non_finite_state_count = self
            .maximum_non_finite_state_count
            .max(step.diagnostics.non_finite_state_count);
        self.maximum_resolved_energy_decay_w = self
            .maximum_resolved_energy_decay_w
            .max(step.resolved_energy_decay_w.max(0.0));
        self.maximum_resolved_energy_gain_w = self
            .maximum_resolved_energy_gain_w
            .max((-step.resolved_energy_decay_w).max(0.0));
        if let Some(value) = step.forcing_divergence_rms_per_s2 {
            self.maximum_forcing_divergence_rms_per_s2 = Some(
                self.maximum_forcing_divergence_rms_per_s2
                    .unwrap_or(0.0)
                    .max(value),
            );
        }

        for (name, value) in [
            ("maximum_resolved_speed_mps", self.maximum_resolved_speed_mps),
            ("maximum_vorticity_per_s", self.maximum_vorticity_per_s),
            ("maximum_strain_rate_per_s", self.maximum_strain_rate_per_s),
            (
                "maximum_pressure_gradient_pa_per_m",
                self.maximum_pressure_gradient_pa_per_m,
            ),
            (
                "maximum_divergence_rms_per_s",
                self.maximum_divergence_rms_per_s,
            ),
            ("maximum_cfl", self.maximum_cfl),
            (
                "maximum_pressure_residual_rms_pa_per_m2",
                self.maximum_pressure_residual_rms_pa_per_m2,
            ),
            (
                "maximum_resolved_energy_decay_w",
                self.maximum_resolved_energy_decay_w,
            ),
            (
                "maximum_resolved_energy_gain_w",
                self.maximum_resolved_energy_gain_w,
            ),
        ] {
            ensure_finite(name, value)?;
        }
        Ok(())
    }
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
            pressure_iterations: 400,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }

    #[test]
    fn passive_trace_retains_stepwise_diagnostics_and_analytical_error() {
        let report = run_passive_taylor_green_diagnostic_trace(config(), 0.08, 0.0005, 4)
            .unwrap();
        assert_eq!(report.steps.len(), 4);
        assert!(report.initial_analytical_error.velocity_rms_error_mps < 1.0e-14);
        assert!(report.extrema.maximum_resolved_speed_mps > 0.0);
        assert!(report.extrema.maximum_vorticity_per_s > 0.0);
        assert!(report.extrema.maximum_strain_rate_per_s > 0.0);
        assert_eq!(report.extrema.maximum_non_finite_state_count, 0);
        assert!(report.steps.iter().all(|step| {
            step.projection.pressure_iterations_executed == 400
                && step.analytical_error.velocity_rms_error_mps.is_finite()
                && step.resolved_energy_decay_w.is_finite()
        }));
    }

    #[test]
    fn manufactured_trace_measures_discrete_forcing_solenoidality() {
        let profile = ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.08,
            modulation_fraction: 0.2,
            angular_frequency_rad_s: 1.5,
        };
        let report = run_manufactured_taylor_green_diagnostic_trace(
            config(),
            profile,
            0.0005,
            4,
        )
        .unwrap();
        assert_eq!(report.steps.len(), 4);
        assert!(
            report
                .extrema
                .maximum_forcing_divergence_rms_per_s2
                .unwrap()
                < 1.0e-12
        );
        assert!(report.steps.iter().all(|step| {
            step.forcing_divergence_rms_per_s2.unwrap().is_finite()
                && step.analytical_error.velocity_rms_error_mps.is_finite()
        }));
    }

    #[test]
    fn trace_replays_deterministically() {
        let a = run_passive_taylor_green_diagnostic_trace(config(), 0.08, 0.0005, 3)
            .unwrap();
        let b = run_passive_taylor_green_diagnostic_trace(config(), 0.08, 0.0005, 3)
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_trace_request_fails_closed() {
        assert_eq!(
            run_passive_taylor_green_diagnostic_trace(config(), 0.08, f64::NAN, 4),
            Err(DiagnosticTraceError::InvalidDt)
        );
        assert_eq!(
            run_passive_taylor_green_diagnostic_trace(config(), 0.08, 0.001, 0),
            Err(DiagnosticTraceError::InvalidStepCount)
        );
    }
}
