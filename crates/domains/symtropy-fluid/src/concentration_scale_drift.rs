// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Same-run passive Taylor-Green concentration-scale and energy-drift trace.
//!
//! The exact viscous Taylor-Green control decays in amplitude/energy while its
//! spatial mode remains fixed. This module therefore records, from one numerical
//! trajectory, both excess resolved-energy loss and drift of the measured
//! vorticity-gradient concentration length away from the exact discrete control
//! scale. Positive scale drift means the measured vorticity structure moved to
//! a larger characteristic length than the control; negative drift means a
//! smaller characteristic length. Neither sign is an automatic classification.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference::{
    PeriodicMac2d, PeriodicMacConfig, ReferenceConfigError, ReferenceDiagnosticError,
    ReferenceStateError, ReferenceStepError,
};
use crate::vorticity_concentration_scale::{
    VorticityConcentrationScaleError, VorticityConcentrationScaleReport,
    VorticityConcentrationScaleUnavailableReason, VorticityConcentrationScaleValue,
    measure_vorticity_concentration_scale,
};

pub const PASSIVE_CONCENTRATION_SCALE_DRIFT_SCHEMA_ID: &str =
    "passive-taylor-green-concentration-scale-energy-drift-v0.1";
pub const MAX_PASSIVE_CONCENTRATION_SCALE_DRIFT_STEPS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassiveScaleDriftStepContext {
    pub max_advective_cfl: f64,
    pub diffusion_number: f64,
    pub combined_explicit_number: f64,
    pub divergence_rms_before_per_s: f64,
    pub divergence_rms_after_per_s: f64,
    pub pressure_residual_rms_pa_per_m2: f64,
    pub non_finite_state_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassiveConcentrationScaleDriftPoint {
    pub step_index: usize,
    pub time_s: f64,
    pub measured_energy_j: f64,
    pub exact_discrete_energy_j: f64,
    /// `exact_discrete_energy_j - measured_energy_j`; positive means more
    /// resolved energy has been lost numerically than in the exact control.
    pub cumulative_excess_resolved_energy_loss_j: f64,
    pub cumulative_excess_loss_fraction_of_initial: f64,
    pub exact_discrete_concentration_scale_m: f64,
    pub measured_concentration_scale_m: f64,
    /// `measured - exact`; positive means a shift toward a larger/smoother
    /// characteristic vorticity length relative to this exact control.
    pub concentration_scale_drift_m: f64,
    pub relative_concentration_scale_drift: f64,
    pub concentration: VorticityConcentrationScaleReport,
    /// Absent only for the initial state because no numerical step has occurred.
    pub step_context: Option<PassiveScaleDriftStepContext>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassiveConcentrationScaleDriftReport {
    pub schema_id: String,
    pub solver_profile: String,
    pub diagnostic_profile: String,
    pub initial_amplitude_mps: f64,
    pub dt_s: f64,
    pub steps: usize,
    pub exact_energy_decay_rate_per_s: f64,
    pub exact_discrete_concentration_scale_m: f64,
    pub initial_energy_j: f64,
    pub points: Vec<PassiveConcentrationScaleDriftPoint>,
    pub maximum_positive_relative_concentration_scale_drift: f64,
    pub minimum_relative_concentration_scale_drift: f64,
    pub maximum_absolute_relative_concentration_scale_drift: f64,
    pub maximum_absolute_cumulative_excess_loss_fraction: f64,
    pub final_relative_concentration_scale_drift: f64,
    pub final_cumulative_excess_loss_fraction_of_initial: f64,
    pub maximum_observed_advective_cfl: f64,
    pub maximum_observed_diffusion_number: f64,
    pub maximum_observed_combined_explicit_number: f64,
    pub maximum_observed_divergence_rms_per_s: f64,
    pub maximum_observed_pressure_residual_rms_pa_per_m2: f64,
    pub maximum_observed_non_finite_state_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PassiveConcentrationScaleDriftError {
    NonCanonicalDomain,
    InvalidAmplitude,
    InvalidTimeStep,
    InvalidStepCount,
    MissingMeasuredEnergy,
    InvalidExactControl,
    UnavailableConcentrationScale(VorticityConcentrationScaleUnavailableReason),
    NonFiniteDerivedMetric(&'static str),
    Config(ReferenceConfigError),
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
    Concentration(VorticityConcentrationScaleError),
}

impl fmt::Display for PassiveConcentrationScaleDriftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalDomain => write!(
                f,
                "passive concentration-scale drift V0.1 requires a square periodic Taylor-Green domain"
            ),
            Self::InvalidAmplitude => write!(f, "initial amplitude must be finite and > 0"),
            Self::InvalidTimeStep => write!(f, "dt_s must be finite and > 0"),
            Self::InvalidStepCount => write!(
                f,
                "step count must be in 1..={MAX_PASSIVE_CONCENTRATION_SCALE_DRIFT_STEPS}"
            ),
            Self::MissingMeasuredEnergy => write!(f, "kinetic-energy diagnostic is not measured"),
            Self::InvalidExactControl => write!(f, "exact passive control metric is invalid"),
            Self::UnavailableConcentrationScale(reason) => write!(
                f,
                "vorticity concentration scale unavailable for passive control: {reason:?}"
            ),
            Self::NonFiniteDerivedMetric(name) => {
                write!(
                    f,
                    "passive concentration-scale drift metric {name} is non-finite"
                )
            }
            Self::Config(source) => write!(f, "invalid reference configuration: {source}"),
            Self::State(source) => write!(f, "invalid reference state: {source}"),
            Self::Step(source) => write!(f, "reference step failed: {source}"),
            Self::Diagnostic(source) => write!(f, "reference diagnostic failed: {source}"),
            Self::Concentration(source) => {
                write!(f, "concentration-scale measurement failed: {source}")
            }
        }
    }
}

impl std::error::Error for PassiveConcentrationScaleDriftError {}

impl From<ReferenceConfigError> for PassiveConcentrationScaleDriftError {
    fn from(value: ReferenceConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<ReferenceStateError> for PassiveConcentrationScaleDriftError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for PassiveConcentrationScaleDriftError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for PassiveConcentrationScaleDriftError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<VorticityConcentrationScaleError> for PassiveConcentrationScaleDriftError {
    fn from(value: VorticityConcentrationScaleError) -> Self {
        Self::Concentration(value)
    }
}

pub fn run_passive_taylor_green_concentration_scale_drift(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    dt_s: f64,
    steps: usize,
) -> Result<PassiveConcentrationScaleDriftReport, PassiveConcentrationScaleDriftError> {
    validate_request(&config, initial_amplitude_mps, dt_s, steps)?;

    let solver_profile = config.profile_identity();
    let exact_discrete_concentration_scale_m = exact_discrete_concentration_scale(&config)?;
    let k_per_m = std::f64::consts::TAU / config.length_x_m;
    let exact_energy_decay_rate_per_s = 4.0 * config.kinematic_viscosity_m2_s * k_per_m * k_per_m;
    ensure_finite(
        "exact_energy_decay_rate_per_s",
        exact_energy_decay_rate_per_s,
    )?;

    let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude_mps)?;
    let initial_energy_j = measured_energy(&state)?;
    if !initial_energy_j.is_finite() || initial_energy_j <= 0.0 {
        return Err(PassiveConcentrationScaleDriftError::InvalidExactControl);
    }

    let initial_concentration = measure_vorticity_concentration_scale(&state)?;
    let diagnostic_profile = initial_concentration.diagnostic_profile.clone();
    let initial_scale_m = measured_scale(&initial_concentration)?;
    let initial_point = make_point(
        0,
        &state,
        initial_energy_j,
        initial_energy_j,
        exact_discrete_concentration_scale_m,
        initial_scale_m,
        initial_concentration,
        None,
    )?;

    let mut maximum_positive_relative_concentration_scale_drift =
        initial_point.relative_concentration_scale_drift.max(0.0);
    let mut minimum_relative_concentration_scale_drift =
        initial_point.relative_concentration_scale_drift.min(0.0);
    let mut maximum_absolute_relative_concentration_scale_drift =
        initial_point.relative_concentration_scale_drift.abs();
    let mut maximum_absolute_cumulative_excess_loss_fraction = initial_point
        .cumulative_excess_loss_fraction_of_initial
        .abs();
    let mut maximum_observed_advective_cfl = 0.0_f64;
    let mut maximum_observed_diffusion_number = 0.0_f64;
    let mut maximum_observed_combined_explicit_number = 0.0_f64;
    let mut maximum_observed_divergence_rms_per_s = state.divergence_rms_per_s();
    let mut maximum_observed_pressure_residual_rms_pa_per_m2 = 0.0_f64;
    let mut maximum_observed_non_finite_state_count = 0_u64;
    let mut points = Vec::with_capacity(steps + 1);
    points.push(initial_point);

    for step_index in 1..=steps {
        let step = state.step(dt_s)?;
        let context = PassiveScaleDriftStepContext {
            max_advective_cfl: step.max_advective_cfl,
            diffusion_number: step.diffusion_number,
            combined_explicit_number: step.combined_explicit_number,
            divergence_rms_before_per_s: step.projection.divergence_rms_before_per_s,
            divergence_rms_after_per_s: step.projection.divergence_rms_after_per_s,
            pressure_residual_rms_pa_per_m2: step.projection.pressure_residual_rms_pa_per_m2,
            non_finite_state_count: step.non_finite_state_count,
        };

        for (name, value) in [
            ("max_advective_cfl", context.max_advective_cfl),
            ("diffusion_number", context.diffusion_number),
            ("combined_explicit_number", context.combined_explicit_number),
            (
                "divergence_rms_before_per_s",
                context.divergence_rms_before_per_s,
            ),
            (
                "divergence_rms_after_per_s",
                context.divergence_rms_after_per_s,
            ),
            (
                "pressure_residual_rms_pa_per_m2",
                context.pressure_residual_rms_pa_per_m2,
            ),
        ] {
            ensure_finite(name, value)?;
        }

        maximum_observed_advective_cfl =
            maximum_observed_advective_cfl.max(context.max_advective_cfl);
        maximum_observed_diffusion_number =
            maximum_observed_diffusion_number.max(context.diffusion_number);
        maximum_observed_combined_explicit_number =
            maximum_observed_combined_explicit_number.max(context.combined_explicit_number);
        maximum_observed_divergence_rms_per_s =
            maximum_observed_divergence_rms_per_s.max(context.divergence_rms_after_per_s);
        maximum_observed_pressure_residual_rms_pa_per_m2 =
            maximum_observed_pressure_residual_rms_pa_per_m2
                .max(context.pressure_residual_rms_pa_per_m2);
        maximum_observed_non_finite_state_count =
            maximum_observed_non_finite_state_count.max(context.non_finite_state_count);

        let measured_energy_j = measured_energy(&state)?;
        let exact_discrete_energy_j =
            initial_energy_j * (-exact_energy_decay_rate_per_s * state.time_s()).exp();
        if !exact_discrete_energy_j.is_finite() || exact_discrete_energy_j <= 0.0 {
            return Err(PassiveConcentrationScaleDriftError::InvalidExactControl);
        }
        let concentration = measure_vorticity_concentration_scale(&state)?;
        if concentration.diagnostic_profile != diagnostic_profile {
            return Err(PassiveConcentrationScaleDriftError::InvalidExactControl);
        }
        let measured_concentration_scale_m = measured_scale(&concentration)?;
        let point = make_point(
            step_index,
            &state,
            measured_energy_j,
            exact_discrete_energy_j,
            exact_discrete_concentration_scale_m,
            measured_concentration_scale_m,
            concentration,
            Some(context),
        )?;

        maximum_positive_relative_concentration_scale_drift =
            maximum_positive_relative_concentration_scale_drift
                .max(point.relative_concentration_scale_drift);
        minimum_relative_concentration_scale_drift = minimum_relative_concentration_scale_drift
            .min(point.relative_concentration_scale_drift);
        maximum_absolute_relative_concentration_scale_drift =
            maximum_absolute_relative_concentration_scale_drift
                .max(point.relative_concentration_scale_drift.abs());
        maximum_absolute_cumulative_excess_loss_fraction =
            maximum_absolute_cumulative_excess_loss_fraction
                .max(point.cumulative_excess_loss_fraction_of_initial.abs());
        points.push(point);
    }

    let final_point = points
        .last()
        .expect("validated passive scale-drift trace always contains an initial point");
    let final_relative_concentration_scale_drift = final_point.relative_concentration_scale_drift;
    let final_cumulative_excess_loss_fraction_of_initial =
        final_point.cumulative_excess_loss_fraction_of_initial;

    Ok(PassiveConcentrationScaleDriftReport {
        schema_id: PASSIVE_CONCENTRATION_SCALE_DRIFT_SCHEMA_ID.to_owned(),
        solver_profile,
        diagnostic_profile,
        initial_amplitude_mps,
        dt_s,
        steps,
        exact_energy_decay_rate_per_s,
        exact_discrete_concentration_scale_m,
        initial_energy_j,
        points,
        maximum_positive_relative_concentration_scale_drift,
        minimum_relative_concentration_scale_drift,
        maximum_absolute_relative_concentration_scale_drift,
        maximum_absolute_cumulative_excess_loss_fraction,
        final_relative_concentration_scale_drift,
        final_cumulative_excess_loss_fraction_of_initial,
        maximum_observed_advective_cfl,
        maximum_observed_diffusion_number,
        maximum_observed_combined_explicit_number,
        maximum_observed_divergence_rms_per_s,
        maximum_observed_pressure_residual_rms_pa_per_m2,
        maximum_observed_non_finite_state_count,
    })
}

fn validate_request(
    config: &PeriodicMacConfig,
    initial_amplitude_mps: f64,
    dt_s: f64,
    steps: usize,
) -> Result<(), PassiveConcentrationScaleDriftError> {
    config.validate()?;
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(PassiveConcentrationScaleDriftError::NonCanonicalDomain);
    }
    if !initial_amplitude_mps.is_finite() || initial_amplitude_mps <= 0.0 {
        return Err(PassiveConcentrationScaleDriftError::InvalidAmplitude);
    }
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(PassiveConcentrationScaleDriftError::InvalidTimeStep);
    }
    if steps == 0 || steps > MAX_PASSIVE_CONCENTRATION_SCALE_DRIFT_STEPS {
        return Err(PassiveConcentrationScaleDriftError::InvalidStepCount);
    }
    Ok(())
}

fn exact_discrete_concentration_scale(
    config: &PeriodicMacConfig,
) -> Result<f64, PassiveConcentrationScaleDriftError> {
    let k_per_m = std::f64::consts::TAU / config.length_x_m;
    let discrete_k_per_m = 2.0 * (0.5 * k_per_m * config.dx()).sin() / config.dx();
    let scale_m = 1.0 / (2.0_f64.sqrt() * discrete_k_per_m.abs());
    if !scale_m.is_finite() || scale_m <= 0.0 {
        return Err(PassiveConcentrationScaleDriftError::InvalidExactControl);
    }
    Ok(scale_m)
}

fn make_point(
    step_index: usize,
    state: &PeriodicMac2d,
    measured_energy_j: f64,
    exact_discrete_energy_j: f64,
    exact_discrete_concentration_scale_m: f64,
    measured_concentration_scale_m: f64,
    concentration: VorticityConcentrationScaleReport,
    step_context: Option<PassiveScaleDriftStepContext>,
) -> Result<PassiveConcentrationScaleDriftPoint, PassiveConcentrationScaleDriftError> {
    let cumulative_excess_resolved_energy_loss_j = exact_discrete_energy_j - measured_energy_j;
    let initial_energy_j = if step_index == 0 {
        measured_energy_j
    } else {
        // The caller normalizes with the report's fixed initial value after this
        // helper; use exact time-zero recovery from the current exact energy.
        let config = state.config();
        let k_per_m = std::f64::consts::TAU / config.length_x_m;
        let rate = 4.0 * config.kinematic_viscosity_m2_s * k_per_m * k_per_m;
        exact_discrete_energy_j * (rate * state.time_s()).exp()
    };
    let cumulative_excess_loss_fraction_of_initial =
        cumulative_excess_resolved_energy_loss_j / initial_energy_j;
    let concentration_scale_drift_m =
        measured_concentration_scale_m - exact_discrete_concentration_scale_m;
    let relative_concentration_scale_drift =
        concentration_scale_drift_m / exact_discrete_concentration_scale_m;

    for (name, value) in [
        ("time_s", state.time_s()),
        ("measured_energy_j", measured_energy_j),
        ("exact_discrete_energy_j", exact_discrete_energy_j),
        (
            "cumulative_excess_resolved_energy_loss_j",
            cumulative_excess_resolved_energy_loss_j,
        ),
        (
            "cumulative_excess_loss_fraction_of_initial",
            cumulative_excess_loss_fraction_of_initial,
        ),
        (
            "measured_concentration_scale_m",
            measured_concentration_scale_m,
        ),
        ("concentration_scale_drift_m", concentration_scale_drift_m),
        (
            "relative_concentration_scale_drift",
            relative_concentration_scale_drift,
        ),
    ] {
        ensure_finite(name, value)?;
    }

    Ok(PassiveConcentrationScaleDriftPoint {
        step_index,
        time_s: state.time_s(),
        measured_energy_j,
        exact_discrete_energy_j,
        cumulative_excess_resolved_energy_loss_j,
        cumulative_excess_loss_fraction_of_initial,
        exact_discrete_concentration_scale_m,
        measured_concentration_scale_m,
        concentration_scale_drift_m,
        relative_concentration_scale_drift,
        concentration,
        step_context,
    })
}

fn measured_scale(
    report: &VorticityConcentrationScaleReport,
) -> Result<f64, PassiveConcentrationScaleDriftError> {
    match report.scale {
        VorticityConcentrationScaleValue::Measured { length_m, .. } => Ok(length_m),
        VorticityConcentrationScaleValue::Unavailable(reason) => {
            Err(PassiveConcentrationScaleDriftError::UnavailableConcentrationScale(reason))
        }
    }
}

fn measured_energy(state: &PeriodicMac2d) -> Result<f64, PassiveConcentrationScaleDriftError> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(PassiveConcentrationScaleDriftError::MissingMeasuredEnergy)
}

fn ensure_finite(
    name: &'static str,
    value: f64,
) -> Result<(), PassiveConcentrationScaleDriftError> {
    if !value.is_finite() {
        return Err(PassiveConcentrationScaleDriftError::NonFiniteDerivedMetric(
            name,
        ));
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

    #[test]
    fn initial_state_has_zero_energy_excess_and_exact_scale() {
        let report =
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.00025, 4).unwrap();
        let initial = &report.points[0];
        assert_eq!(initial.step_index, 0);
        assert_eq!(initial.cumulative_excess_resolved_energy_loss_j, 0.0);
        assert_eq!(initial.cumulative_excess_loss_fraction_of_initial, 0.0);
        assert!(initial.step_context.is_none());
        assert!(
            (initial.measured_concentration_scale_m - report.exact_discrete_concentration_scale_m)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn energy_excess_matches_exact_minus_measured() {
        let report =
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.00025, 8).unwrap();
        for point in &report.points {
            let expected = point.exact_discrete_energy_j - point.measured_energy_j;
            assert!((point.cumulative_excess_resolved_energy_loss_j - expected).abs() < 1.0e-14);
        }
    }

    #[test]
    fn post_initial_points_retain_same_step_numerical_context() {
        let report =
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.00025, 4).unwrap();
        assert!(
            report.points[1..]
                .iter()
                .all(|point| point.step_context.is_some())
        );
        assert!(
            report
                .points
                .iter()
                .all(|point| point.relative_concentration_scale_drift.is_finite())
        );
    }

    #[test]
    fn report_is_replay_deterministic() {
        let a =
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.00025, 8).unwrap();
        let b =
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.00025, 8).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_requests_fail_closed() {
        assert_eq!(
            run_passive_taylor_green_concentration_scale_drift(config(), 0.0, 0.00025, 4),
            Err(PassiveConcentrationScaleDriftError::InvalidAmplitude)
        );
        assert_eq!(
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.0, 4),
            Err(PassiveConcentrationScaleDriftError::InvalidTimeStep)
        );
        assert_eq!(
            run_passive_taylor_green_concentration_scale_drift(config(), 0.08, 0.00025, 0),
            Err(PassiveConcentrationScaleDriftError::InvalidStepCount)
        );
    }
}
