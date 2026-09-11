// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Excess resolved-energy loss for the passive Taylor-Green control.
//!
//! This module compares the numerical kinetic-energy history against the exact
//! viscous Taylor-Green decay evaluated in the same discrete energy quadrature.
//! A positive excess means the numerical state has lost more resolved kinetic
//! energy than the exact smooth solution. A negative excess means it has lost
//! less. The excess is a numerical-observability quantity, not a claim of pure
//! numerical dissipation: spatial, temporal, projection, and other discretized
//! effects can all contribute.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::convergence::TAYLOR_GREEN_COMPARATOR_ID;
use crate::reference::{
    PeriodicMac2d, PeriodicMacConfig, ReferenceConfigError, ReferenceDiagnosticError,
    ReferenceStateError, ReferenceStepError,
};

pub const EXCESS_ENERGY_DECAY_SCHEMA_ID: &str =
    "passive-taylor-green-excess-resolved-energy-loss-v0.1";
pub const MAX_EXCESS_ENERGY_DECAY_STEPS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExcessEnergyDecayPoint {
    pub step_index: usize,
    pub time_s: f64,
    pub measured_energy_j: f64,
    pub exact_discrete_energy_j: f64,
    pub numerical_to_exact_energy_ratio: f64,
    pub observed_step_energy_loss_j: f64,
    pub exact_step_viscous_energy_loss_j: f64,
    /// Positive means the numerical step lost more resolved kinetic energy than
    /// the exact viscous Taylor-Green control over the same time interval.
    pub step_excess_resolved_energy_loss_j: f64,
    pub observed_cumulative_energy_loss_j: f64,
    pub exact_cumulative_viscous_energy_loss_j: f64,
    /// Equivalent to `exact_discrete_energy_j - measured_energy_j` because both
    /// histories start from the identical discretely sampled initial state.
    pub cumulative_excess_resolved_energy_loss_j: f64,
    pub cumulative_excess_loss_fraction_of_initial: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExcessEnergyDecayReport {
    pub schema_id: String,
    pub comparator_id: String,
    pub solver_profile: String,
    pub initial_amplitude_mps: f64,
    pub dt_s: f64,
    pub steps: usize,
    pub exact_energy_decay_rate_per_s: f64,
    pub initial_energy_j: f64,
    pub points: Vec<ExcessEnergyDecayPoint>,
    pub maximum_positive_step_excess_loss_j: f64,
    pub minimum_step_excess_loss_j: f64,
    pub maximum_absolute_cumulative_excess_loss_fraction: f64,
    pub final_cumulative_excess_resolved_energy_loss_j: f64,
    pub final_cumulative_excess_loss_fraction_of_initial: f64,
    pub maximum_observed_advective_cfl: f64,
    pub maximum_observed_diffusion_number: f64,
    pub maximum_observed_combined_explicit_number: f64,
    pub maximum_observed_divergence_rms_per_s: f64,
    pub maximum_observed_pressure_residual_rms_pa_per_m2: f64,
    pub maximum_observed_non_finite_state_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExcessEnergyDecayError {
    NonCanonicalDomain,
    InvalidAmplitude,
    InvalidTimeStep,
    InvalidStepCount,
    MissingMeasuredEnergy,
    InvalidExactEnergy,
    NonFiniteDerivedMetric(&'static str),
    Config(ReferenceConfigError),
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
}

impl fmt::Display for ExcessEnergyDecayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalDomain => write!(
                f,
                "excess-energy V0.1 requires a square periodic Taylor-Green domain"
            ),
            Self::InvalidAmplitude => write!(f, "initial amplitude must be finite and > 0"),
            Self::InvalidTimeStep => write!(f, "dt_s must be finite and > 0"),
            Self::InvalidStepCount => write!(
                f,
                "step count must be in 1..={MAX_EXCESS_ENERGY_DECAY_STEPS}"
            ),
            Self::MissingMeasuredEnergy => write!(f, "kinetic-energy diagnostic is not measured"),
            Self::InvalidExactEnergy => write!(
                f,
                "exact discrete Taylor-Green energy must remain finite and > 0"
            ),
            Self::NonFiniteDerivedMetric(name) => {
                write!(f, "derived excess-energy metric {name} is non-finite")
            }
            Self::Config(source) => write!(f, "invalid reference configuration: {source}"),
            Self::State(source) => write!(f, "Taylor-Green state failed: {source}"),
            Self::Step(source) => write!(f, "Taylor-Green step failed: {source}"),
            Self::Diagnostic(source) => write!(f, "energy diagnostic failed: {source}"),
        }
    }
}

impl std::error::Error for ExcessEnergyDecayError {}

impl From<ReferenceConfigError> for ExcessEnergyDecayError {
    fn from(value: ReferenceConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<ReferenceStateError> for ExcessEnergyDecayError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for ExcessEnergyDecayError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for ExcessEnergyDecayError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

pub fn run_passive_taylor_green_excess_energy_decay(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    dt_s: f64,
    steps: usize,
) -> Result<ExcessEnergyDecayReport, ExcessEnergyDecayError> {
    validate_request(&config, initial_amplitude_mps, dt_s, steps)?;

    let solver_profile = config.profile_identity();
    let k = std::f64::consts::TAU / config.length_x_m;
    let exact_energy_decay_rate_per_s = 4.0 * config.kinematic_viscosity_m2_s * k * k;
    ensure_finite(
        "exact_energy_decay_rate_per_s",
        exact_energy_decay_rate_per_s,
    )?;

    let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude_mps)?;
    let initial_energy_j = measured_energy(&state)?;
    if !initial_energy_j.is_finite() || initial_energy_j <= 0.0 {
        return Err(ExcessEnergyDecayError::InvalidExactEnergy);
    }

    let mut points = Vec::with_capacity(steps + 1);
    points.push(ExcessEnergyDecayPoint {
        step_index: 0,
        time_s: 0.0,
        measured_energy_j: initial_energy_j,
        exact_discrete_energy_j: initial_energy_j,
        numerical_to_exact_energy_ratio: 1.0,
        observed_step_energy_loss_j: 0.0,
        exact_step_viscous_energy_loss_j: 0.0,
        step_excess_resolved_energy_loss_j: 0.0,
        observed_cumulative_energy_loss_j: 0.0,
        exact_cumulative_viscous_energy_loss_j: 0.0,
        cumulative_excess_resolved_energy_loss_j: 0.0,
        cumulative_excess_loss_fraction_of_initial: 0.0,
    });

    let mut previous_measured_energy_j = initial_energy_j;
    let mut previous_exact_energy_j = initial_energy_j;
    let mut maximum_positive_step_excess_loss_j = 0.0_f64;
    let mut minimum_step_excess_loss_j = 0.0_f64;
    let mut maximum_absolute_cumulative_excess_loss_fraction = 0.0_f64;
    let mut maximum_observed_advective_cfl = 0.0_f64;
    let mut maximum_observed_diffusion_number = 0.0_f64;
    let mut maximum_observed_combined_explicit_number = 0.0_f64;
    let mut maximum_observed_divergence_rms_per_s = state.divergence_rms_per_s();
    let mut maximum_observed_pressure_residual_rms_pa_per_m2 = 0.0_f64;
    let mut maximum_observed_non_finite_state_count = 0_u64;

    for step_index in 1..=steps {
        let step = state.step(dt_s)?;
        maximum_observed_advective_cfl = maximum_observed_advective_cfl.max(step.max_advective_cfl);
        maximum_observed_diffusion_number =
            maximum_observed_diffusion_number.max(step.diffusion_number);
        maximum_observed_combined_explicit_number =
            maximum_observed_combined_explicit_number.max(step.combined_explicit_number);
        maximum_observed_divergence_rms_per_s = maximum_observed_divergence_rms_per_s
            .max(step.projection.divergence_rms_after_per_s);
        maximum_observed_pressure_residual_rms_pa_per_m2 =
            maximum_observed_pressure_residual_rms_pa_per_m2
                .max(step.projection.pressure_residual_rms_pa_per_m2);
        maximum_observed_non_finite_state_count =
            maximum_observed_non_finite_state_count.max(step.non_finite_state_count);

        let time_s = state.time_s();
        let measured_energy_j = measured_energy(&state)?;
        let exact_discrete_energy_j =
            initial_energy_j * (-exact_energy_decay_rate_per_s * time_s).exp();
        if !exact_discrete_energy_j.is_finite() || exact_discrete_energy_j <= 0.0 {
            return Err(ExcessEnergyDecayError::InvalidExactEnergy);
        }

        let numerical_to_exact_energy_ratio = measured_energy_j / exact_discrete_energy_j;
        let observed_step_energy_loss_j = previous_measured_energy_j - measured_energy_j;
        let exact_step_viscous_energy_loss_j =
            previous_exact_energy_j - exact_discrete_energy_j;
        let step_excess_resolved_energy_loss_j =
            observed_step_energy_loss_j - exact_step_viscous_energy_loss_j;
        let observed_cumulative_energy_loss_j = initial_energy_j - measured_energy_j;
        let exact_cumulative_viscous_energy_loss_j =
            initial_energy_j - exact_discrete_energy_j;
        let cumulative_excess_resolved_energy_loss_j =
            observed_cumulative_energy_loss_j - exact_cumulative_viscous_energy_loss_j;
        let cumulative_excess_loss_fraction_of_initial =
            cumulative_excess_resolved_energy_loss_j / initial_energy_j;

        for (name, value) in [
            ("measured_energy_j", measured_energy_j),
            (
                "numerical_to_exact_energy_ratio",
                numerical_to_exact_energy_ratio,
            ),
            ("observed_step_energy_loss_j", observed_step_energy_loss_j),
            (
                "exact_step_viscous_energy_loss_j",
                exact_step_viscous_energy_loss_j,
            ),
            (
                "step_excess_resolved_energy_loss_j",
                step_excess_resolved_energy_loss_j,
            ),
            (
                "observed_cumulative_energy_loss_j",
                observed_cumulative_energy_loss_j,
            ),
            (
                "exact_cumulative_viscous_energy_loss_j",
                exact_cumulative_viscous_energy_loss_j,
            ),
            (
                "cumulative_excess_resolved_energy_loss_j",
                cumulative_excess_resolved_energy_loss_j,
            ),
            (
                "cumulative_excess_loss_fraction_of_initial",
                cumulative_excess_loss_fraction_of_initial,
            ),
        ] {
            ensure_finite(name, value)?;
        }

        maximum_positive_step_excess_loss_j =
            maximum_positive_step_excess_loss_j.max(step_excess_resolved_energy_loss_j);
        minimum_step_excess_loss_j =
            minimum_step_excess_loss_j.min(step_excess_resolved_energy_loss_j);
        maximum_absolute_cumulative_excess_loss_fraction =
            maximum_absolute_cumulative_excess_loss_fraction
                .max(cumulative_excess_loss_fraction_of_initial.abs());

        points.push(ExcessEnergyDecayPoint {
            step_index,
            time_s,
            measured_energy_j,
            exact_discrete_energy_j,
            numerical_to_exact_energy_ratio,
            observed_step_energy_loss_j,
            exact_step_viscous_energy_loss_j,
            step_excess_resolved_energy_loss_j,
            observed_cumulative_energy_loss_j,
            exact_cumulative_viscous_energy_loss_j,
            cumulative_excess_resolved_energy_loss_j,
            cumulative_excess_loss_fraction_of_initial,
        });

        previous_measured_energy_j = measured_energy_j;
        previous_exact_energy_j = exact_discrete_energy_j;
    }

    let final_point = points
        .last()
        .expect("validated excess-energy trace always contains the initial point");
    let final_cumulative_excess_resolved_energy_loss_j =
        final_point.cumulative_excess_resolved_energy_loss_j;
    let final_cumulative_excess_loss_fraction_of_initial =
        final_point.cumulative_excess_loss_fraction_of_initial;

    Ok(ExcessEnergyDecayReport {
        schema_id: EXCESS_ENERGY_DECAY_SCHEMA_ID.to_owned(),
        comparator_id: TAYLOR_GREEN_COMPARATOR_ID.to_owned(),
        solver_profile,
        initial_amplitude_mps,
        dt_s,
        steps,
        exact_energy_decay_rate_per_s,
        initial_energy_j,
        points,
        maximum_positive_step_excess_loss_j,
        minimum_step_excess_loss_j,
        maximum_absolute_cumulative_excess_loss_fraction,
        final_cumulative_excess_resolved_energy_loss_j,
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
) -> Result<(), ExcessEnergyDecayError> {
    config.validate()?;
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(ExcessEnergyDecayError::NonCanonicalDomain);
    }
    if !initial_amplitude_mps.is_finite() || initial_amplitude_mps <= 0.0 {
        return Err(ExcessEnergyDecayError::InvalidAmplitude);
    }
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(ExcessEnergyDecayError::InvalidTimeStep);
    }
    if steps == 0 || steps > MAX_EXCESS_ENERGY_DECAY_STEPS {
        return Err(ExcessEnergyDecayError::InvalidStepCount);
    }
    Ok(())
}

fn measured_energy(state: &PeriodicMac2d) -> Result<f64, ExcessEnergyDecayError> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(ExcessEnergyDecayError::MissingMeasuredEnergy)
}

fn ensure_finite(name: &'static str, value: f64) -> Result<(), ExcessEnergyDecayError> {
    if !value.is_finite() {
        return Err(ExcessEnergyDecayError::NonFiniteDerivedMetric(name));
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
    fn initial_point_has_zero_excess_by_construction() {
        let report = run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.00025, 4)
            .unwrap();
        let initial = &report.points[0];
        assert_eq!(initial.step_index, 0);
        assert_eq!(initial.measured_energy_j, initial.exact_discrete_energy_j);
        assert_eq!(initial.numerical_to_exact_energy_ratio, 1.0);
        assert_eq!(initial.cumulative_excess_resolved_energy_loss_j, 0.0);
        assert_eq!(initial.cumulative_excess_loss_fraction_of_initial, 0.0);
    }

    #[test]
    fn exact_discrete_energy_uses_squared_velocity_decay() {
        let cfg = config();
        let report =
            run_passive_taylor_green_excess_energy_decay(cfg.clone(), 0.08, 0.00025, 4)
                .unwrap();
        let point = report.points.last().unwrap();
        let k = std::f64::consts::TAU / cfg.length_x_m;
        let expected_ratio =
            (-4.0 * cfg.kinematic_viscosity_m2_s * k * k * point.time_s).exp();
        let observed_ratio = point.exact_discrete_energy_j / report.initial_energy_j;
        assert!((observed_ratio - expected_ratio).abs() < 1.0e-14);
    }

    #[test]
    fn cumulative_excess_matches_exact_minus_measured_energy() {
        let report = run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.00025, 8)
            .unwrap();
        for point in &report.points {
            let expected = point.exact_discrete_energy_j - point.measured_energy_j;
            assert!((point.cumulative_excess_resolved_energy_loss_j - expected).abs() < 1.0e-14);
        }
    }

    #[test]
    fn numerical_context_is_retained_from_the_same_run() {
        let report = run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.00025, 8)
            .unwrap();
        assert!(report.maximum_observed_advective_cfl.is_finite());
        assert!(report.maximum_observed_diffusion_number.is_finite());
        assert!(report.maximum_observed_combined_explicit_number.is_finite());
        assert!(report.maximum_observed_divergence_rms_per_s.is_finite());
        assert!(
            report
                .maximum_observed_pressure_residual_rms_pa_per_m2
                .is_finite()
        );
        assert_eq!(report.maximum_observed_non_finite_state_count, 0);
    }

    #[test]
    fn report_is_replay_deterministic() {
        let a = run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.00025, 8)
            .unwrap();
        let b = run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.00025, 8)
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_requests_fail_closed() {
        assert_eq!(
            run_passive_taylor_green_excess_energy_decay(config(), 0.0, 0.00025, 4),
            Err(ExcessEnergyDecayError::InvalidAmplitude)
        );
        assert_eq!(
            run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.0, 4),
            Err(ExcessEnergyDecayError::InvalidTimeStep)
        );
        assert_eq!(
            run_passive_taylor_green_excess_energy_decay(config(), 0.08, 0.00025, 0),
            Err(ExcessEnergyDecayError::InvalidStepCount)
        );

        let mut rectangular = config();
        rectangular.ny = 12;
        assert_eq!(
            run_passive_taylor_green_excess_energy_decay(rectangular, 0.08, 0.00025, 4),
            Err(ExcessEnergyDecayError::NonCanonicalDomain)
        );

        let mut negative_viscosity = config();
        negative_viscosity.kinematic_viscosity_m2_s = -0.01;
        assert_eq!(
            run_passive_taylor_green_excess_energy_decay(
                negative_viscosity,
                0.08,
                0.00025,
                4,
            ),
            Err(ExcessEnergyDecayError::Config(
                ReferenceConfigError::ExpectedNonNegativeFinite("kinematic_viscosity_m2_s")
            ))
        );
    }
}
