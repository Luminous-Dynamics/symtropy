// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! One-step discrete update defect for the manufactured Taylor-Green control.
//!
//! This module starts from an exact manufactured MAC state at an explicit
//! manufactured phase, applies one real reference-solver update using the exact
//! manufactured forcing, and compares the numerical result with the exact
//! manufactured state at the corresponding physical end time. The reference
//! solver's internal clock remains local elapsed time; the manufactured phase is
//! supplied as a deterministic offset to the forcing/comparator.
//!
//! Dividing the face-velocity defect by `dt` yields a local discrete-update
//! residual in m/s^2. It measures the combined defect of advection, viscosity,
//! forcing, projection, and time integration for this exact solver profile; it
//! is not a continuum-theorem residual or pass policy.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::manufactured::{
    ManufacturedTaylorGreenError, ManufacturedTaylorGreenProfile, manufactured_acceleration_mps2,
};
use crate::reference::{
    PeriodicMac2d, PeriodicMacConfig, ReferenceConfigError, ReferenceDiagnosticError,
    ReferenceStateError, ReferenceStepError,
};

pub const MANUFACTURED_UPDATE_DEFECT_SCHEMA_ID: &str =
    "manufactured-taylor-green-one-step-update-defect-v0.1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManufacturedUpdateDefectReport {
    pub schema_id: String,
    pub solver_profile: String,
    pub manufactured_profile: String,
    pub nx: usize,
    pub ny: usize,
    pub dt_s: f64,
    /// Local elapsed solver time after the single update. This should equal
    /// `dt_s`; it is retained separately from the physical manufactured phase.
    pub solver_elapsed_time_s: f64,
    pub manufactured_start_time_s: f64,
    pub manufactured_end_time_s: f64,
    pub velocity_rms_defect_mps: f64,
    pub velocity_max_defect_mps: f64,
    pub update_residual_rms_mps2: f64,
    pub update_residual_max_mps2: f64,
    pub kinetic_energy_relative_error: f64,
    pub max_advective_cfl: f64,
    pub diffusion_number: f64,
    pub combined_explicit_number: f64,
    pub divergence_rms_before_per_s: f64,
    pub divergence_rms_after_per_s: f64,
    pub pressure_residual_rms_pa_per_m2: f64,
    pub non_finite_state_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ManufacturedUpdateDefectError {
    NonCanonicalDomain,
    InvalidStartTime,
    InvalidTimeStep,
    MissingMeasuredEnergy,
    NonFiniteDerivedMetric(&'static str),
    Config(ReferenceConfigError),
    Manufactured(ManufacturedTaylorGreenError),
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
}

impl fmt::Display for ManufacturedUpdateDefectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalDomain => write!(
                f,
                "manufactured update-defect V0.1 requires a square periodic domain"
            ),
            Self::InvalidStartTime => {
                write!(f, "manufactured_start_time_s must be finite and >= 0")
            }
            Self::InvalidTimeStep => write!(f, "dt_s must be finite and > 0"),
            Self::MissingMeasuredEnergy => write!(
                f,
                "manufactured update-defect comparison requires measured kinetic energy"
            ),
            Self::NonFiniteDerivedMetric(name) => {
                write!(f, "manufactured update-defect metric {name} is non-finite")
            }
            Self::Config(source) => write!(f, "invalid reference configuration: {source}"),
            Self::Manufactured(source) => write!(f, "invalid manufactured profile/state: {source}"),
            Self::State(source) => write!(f, "invalid reference state: {source}"),
            Self::Step(source) => write!(f, "reference step failed: {source}"),
            Self::Diagnostic(source) => write!(f, "reference diagnostic failed: {source}"),
        }
    }
}

impl std::error::Error for ManufacturedUpdateDefectError {}

impl From<ReferenceConfigError> for ManufacturedUpdateDefectError {
    fn from(value: ReferenceConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<ManufacturedTaylorGreenError> for ManufacturedUpdateDefectError {
    fn from(value: ManufacturedTaylorGreenError) -> Self {
        Self::Manufactured(value)
    }
}

impl From<ReferenceStateError> for ManufacturedUpdateDefectError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for ManufacturedUpdateDefectError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for ManufacturedUpdateDefectError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

/// Convenience wrapper for the canonical phase-zero one-step defect.
pub fn measure_manufactured_one_step_update_defect(
    config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    dt_s: f64,
) -> Result<ManufacturedUpdateDefectReport, ManufacturedUpdateDefectError> {
    measure_manufactured_one_step_update_defect_at_phase(config, profile, 0.0, dt_s)
}

/// Measure the one-step update defect beginning at an explicit manufactured
/// physical phase while leaving the reference solver's internal time origin at
/// zero. The forcing callback and exact end-state comparator both receive the
/// same phase offset, so no hidden state-time mutation is required.
pub fn measure_manufactured_one_step_update_defect_at_phase(
    config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    manufactured_start_time_s: f64,
    dt_s: f64,
) -> Result<ManufacturedUpdateDefectReport, ManufacturedUpdateDefectError> {
    config.validate()?;
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(ManufacturedUpdateDefectError::NonCanonicalDomain);
    }
    profile.validate()?;
    if !manufactured_start_time_s.is_finite() || manufactured_start_time_s < 0.0 {
        return Err(ManufacturedUpdateDefectError::InvalidStartTime);
    }
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(ManufacturedUpdateDefectError::InvalidTimeStep);
    }

    let solver_profile = config.profile_identity();
    let manufactured_profile = profile.profile_identity();
    let nx = config.nx;
    let ny = config.ny;
    let exact_config = config.clone();
    let forcing_config = config.clone();
    let initial_amplitude_mps = profile.amplitude_mps(manufactured_start_time_s)?;
    let mut numerical = PeriodicMac2d::taylor_green(config, initial_amplitude_mps)?;

    let step = numerical.step_with_acceleration(dt_s, |position, local_time_s| {
        let manufactured_time_s = manufactured_start_time_s + local_time_s;
        manufactured_acceleration_mps2(
            &forcing_config,
            profile,
            position,
            manufactured_time_s,
        )
        .unwrap_or([f64::NAN, f64::NAN])
    })?;
    let solver_elapsed_time_s = numerical.time_s();
    let manufactured_end_time_s = manufactured_start_time_s + solver_elapsed_time_s;
    if !manufactured_end_time_s.is_finite() {
        return Err(ManufacturedUpdateDefectError::NonFiniteDerivedMetric(
            "manufactured_end_time_s",
        ));
    }
    let exact_amplitude_mps = profile.amplitude_mps(manufactured_end_time_s)?;
    let exact = PeriodicMac2d::taylor_green(exact_config, exact_amplitude_mps)?;

    let mut squared_velocity_defect = 0.0_f64;
    let mut velocity_max_defect_mps = 0.0_f64;
    let mut face_count = 0usize;
    for (observed, expected) in numerical
        .u_faces()
        .iter()
        .zip(exact.u_faces())
        .chain(numerical.v_faces().iter().zip(exact.v_faces()))
    {
        let defect = observed - expected;
        if !defect.is_finite() {
            return Err(ManufacturedUpdateDefectError::NonFiniteDerivedMetric(
                "velocity_face_defect_mps",
            ));
        }
        squared_velocity_defect += defect * defect;
        velocity_max_defect_mps = velocity_max_defect_mps.max(defect.abs());
        face_count += 1;
    }

    let velocity_rms_defect_mps = (squared_velocity_defect / face_count as f64).sqrt();
    let update_residual_rms_mps2 = velocity_rms_defect_mps / dt_s;
    let update_residual_max_mps2 = velocity_max_defect_mps / dt_s;
    let numerical_energy_j = measured_energy(&numerical)?;
    let exact_energy_j = measured_energy(&exact)?;
    let kinetic_energy_relative_error = (numerical_energy_j - exact_energy_j).abs() / exact_energy_j;

    for (name, value) in [
        ("solver_elapsed_time_s", solver_elapsed_time_s),
        ("manufactured_start_time_s", manufactured_start_time_s),
        ("manufactured_end_time_s", manufactured_end_time_s),
        ("velocity_rms_defect_mps", velocity_rms_defect_mps),
        ("velocity_max_defect_mps", velocity_max_defect_mps),
        ("update_residual_rms_mps2", update_residual_rms_mps2),
        ("update_residual_max_mps2", update_residual_max_mps2),
        ("kinetic_energy_relative_error", kinetic_energy_relative_error),
        ("max_advective_cfl", step.max_advective_cfl),
        ("diffusion_number", step.diffusion_number),
        ("combined_explicit_number", step.combined_explicit_number),
        (
            "divergence_rms_before_per_s",
            step.projection.divergence_rms_before_per_s,
        ),
        (
            "divergence_rms_after_per_s",
            step.projection.divergence_rms_after_per_s,
        ),
        (
            "pressure_residual_rms_pa_per_m2",
            step.projection.pressure_residual_rms_pa_per_m2,
        ),
    ] {
        ensure_finite(name, value)?;
    }

    Ok(ManufacturedUpdateDefectReport {
        schema_id: MANUFACTURED_UPDATE_DEFECT_SCHEMA_ID.to_owned(),
        solver_profile,
        manufactured_profile,
        nx,
        ny,
        dt_s,
        solver_elapsed_time_s,
        manufactured_start_time_s,
        manufactured_end_time_s,
        velocity_rms_defect_mps,
        velocity_max_defect_mps,
        update_residual_rms_mps2,
        update_residual_max_mps2,
        kinetic_energy_relative_error,
        max_advective_cfl: step.max_advective_cfl,
        diffusion_number: step.diffusion_number,
        combined_explicit_number: step.combined_explicit_number,
        divergence_rms_before_per_s: step.projection.divergence_rms_before_per_s,
        divergence_rms_after_per_s: step.projection.divergence_rms_after_per_s,
        pressure_residual_rms_pa_per_m2: step.projection.pressure_residual_rms_pa_per_m2,
        non_finite_state_count: step.non_finite_state_count,
    })
}

fn measured_energy(state: &PeriodicMac2d) -> Result<f64, ManufacturedUpdateDefectError> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(ManufacturedUpdateDefectError::MissingMeasuredEnergy)
}

fn ensure_finite(
    name: &'static str,
    value: f64,
) -> Result<(), ManufacturedUpdateDefectError> {
    if !value.is_finite() {
        return Err(ManufacturedUpdateDefectError::NonFiniteDerivedMetric(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PeriodicMacConfig {
        PeriodicMacConfig {
            nx: 16,
            ny: 16,
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

    fn profile() -> ManufacturedTaylorGreenProfile {
        ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.08,
            modulation_fraction: 0.2,
            angular_frequency_rad_s: 1.5,
        }
    }

    #[test]
    fn phase_zero_wrapper_is_finite_and_retains_same_step_context() {
        let dt_s = 0.00025;
        let report = measure_manufactured_one_step_update_defect(config(), profile(), dt_s).unwrap();
        assert_eq!(report.schema_id, MANUFACTURED_UPDATE_DEFECT_SCHEMA_ID);
        assert_eq!(report.nx, 16);
        assert_eq!(report.ny, 16);
        assert_eq!(report.manufactured_start_time_s, 0.0);
        assert!((report.solver_elapsed_time_s - dt_s).abs() < 1.0e-15);
        assert!((report.manufactured_end_time_s - dt_s).abs() < 1.0e-15);
        assert!(report.velocity_rms_defect_mps.is_finite());
        assert!(report.velocity_max_defect_mps.is_finite());
        assert!(report.update_residual_rms_mps2.is_finite());
        assert!(report.update_residual_max_mps2.is_finite());
        assert!(report.kinetic_energy_relative_error.is_finite());
        assert!(report.max_advective_cfl.is_finite());
        assert!(report.diffusion_number.is_finite());
        assert!(report.combined_explicit_number.is_finite());
        assert!(report.divergence_rms_before_per_s.is_finite());
        assert!(report.divergence_rms_after_per_s.is_finite());
        assert!(report.pressure_residual_rms_pa_per_m2.is_finite());
        assert_eq!(report.non_finite_state_count, 0);
    }

    #[test]
    fn arbitrary_phase_offsets_forcing_and_exact_comparator_together() {
        let p = profile();
        let period_s = std::f64::consts::TAU / p.angular_frequency_rad_s;
        let start_time_s = 0.25 * period_s;
        let dt_s = 0.00025;
        let report = measure_manufactured_one_step_update_defect_at_phase(
            config(),
            p,
            start_time_s,
            dt_s,
        )
        .unwrap();
        assert!((report.manufactured_start_time_s - start_time_s).abs() < 1.0e-15);
        assert!((report.solver_elapsed_time_s - dt_s).abs() < 1.0e-15);
        assert!((report.manufactured_end_time_s - (start_time_s + dt_s)).abs() < 1.0e-15);
        assert!(report.update_residual_rms_mps2.is_finite());
        assert_eq!(report.non_finite_state_count, 0);
    }

    #[test]
    fn residual_units_are_velocity_defect_divided_by_dt() {
        let dt_s = 0.00025;
        let report = measure_manufactured_one_step_update_defect(config(), profile(), dt_s).unwrap();
        assert!(
            (report.update_residual_rms_mps2 - report.velocity_rms_defect_mps / dt_s).abs()
                < 1.0e-15
        );
        assert!(
            (report.update_residual_max_mps2 - report.velocity_max_defect_mps / dt_s).abs()
                < 1.0e-15
        );
    }

    #[test]
    fn arbitrary_phase_defect_is_replay_deterministic() {
        let p = profile();
        let start_time_s = 0.37;
        let a = measure_manufactured_one_step_update_defect_at_phase(
            config(),
            p,
            start_time_s,
            0.00025,
        )
        .unwrap();
        let b = measure_manufactured_one_step_update_defect_at_phase(
            config(),
            p,
            start_time_s,
            0.00025,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_requests_fail_closed() {
        assert_eq!(
            measure_manufactured_one_step_update_defect_at_phase(
                config(),
                profile(),
                -0.1,
                0.00025,
            ),
            Err(ManufacturedUpdateDefectError::InvalidStartTime)
        );
        assert_eq!(
            measure_manufactured_one_step_update_defect(config(), profile(), 0.0),
            Err(ManufacturedUpdateDefectError::InvalidTimeStep)
        );

        let mut rectangular = config();
        rectangular.ny = 12;
        assert_eq!(
            measure_manufactured_one_step_update_defect(rectangular, profile(), 0.00025),
            Err(ManufacturedUpdateDefectError::NonCanonicalDomain)
        );

        let mut negative_viscosity = config();
        negative_viscosity.kinematic_viscosity_m2_s = -0.01;
        assert_eq!(
            measure_manufactured_one_step_update_defect(
                negative_viscosity,
                profile(),
                0.00025,
            ),
            Err(ManufacturedUpdateDefectError::Config(
                ReferenceConfigError::ExpectedNonNegativeFinite("kinematic_viscosity_m2_s")
            ))
        );
    }
}
