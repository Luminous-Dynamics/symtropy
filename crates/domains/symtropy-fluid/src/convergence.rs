// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Measurement-first convergence tools for the transparent CPU continuum solver.
//!
//! V0.1 deliberately records Taylor–Green errors without assigning a promotion
//! threshold. The purpose is to establish reproducible evidence first; an
//! observed-order gate belongs to a later qualification tranche after real
//! resolution/timestep ladders have been inspected.

use std::fmt;

use crate::reference::{
    PeriodicMac2d, PeriodicMacConfig, ReferenceDiagnosticError, ReferenceStateError,
    ReferenceStepError,
};

/// Versioned identity of the analytical comparator used by this module.
pub const TAYLOR_GREEN_COMPARATOR_ID: &str = "taylor-green-periodic-2d-exact-v0.1";

/// Exact/comparator error at one time for the standard square periodic
/// Taylor–Green vortex.
#[derive(Clone, Debug, PartialEq)]
pub struct TaylorGreenError {
    pub comparator_id: &'static str,
    pub time_s: f64,
    pub velocity_rms_error_mps: f64,
    pub velocity_max_error_mps: f64,
    /// Relative error against the comparator's kinetic energy using the same
    /// discrete diagnostic/quadrature path as the numerical state.
    pub kinetic_energy_relative_error: f64,
}

/// Reproducible result of one fixed-step Taylor–Green case.
#[derive(Clone, Debug, PartialEq)]
pub struct TaylorGreenCaseReport {
    pub comparator_id: &'static str,
    pub solver_profile: String,
    pub nx: usize,
    pub ny: usize,
    pub steps: usize,
    pub dt_s: f64,
    pub final_time_s: f64,
    pub maximum_observed_advective_cfl: f64,
    pub maximum_observed_divergence_rms_per_s: f64,
    pub maximum_observed_pressure_residual_rms_pa_per_m2: f64,
    pub final_error: TaylorGreenError,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaylorGreenErrorKind {
    /// V0.1 intentionally supports the canonical one-mode square periodic case
    /// only. This keeps the analytical claim narrower than the generic MAC grid.
    NonCanonicalDomain,
    InvalidAmplitude,
    InvalidFinalTime,
    InvalidStepCount,
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
    MissingMeasuredEnergy,
    NonFiniteComparison,
}

impl fmt::Display for TaylorGreenErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalDomain => write!(
                f,
                "Taylor-Green V0.1 requires nx == ny and length_x_m == length_y_m"
            ),
            Self::InvalidAmplitude => write!(f, "Taylor-Green amplitude must be finite and > 0"),
            Self::InvalidFinalTime => write!(f, "final_time_s must be finite and > 0"),
            Self::InvalidStepCount => write!(f, "Taylor-Green case requires at least one step"),
            Self::State(source) => write!(f, "invalid Taylor-Green state: {source}"),
            Self::Step(source) => write!(f, "Taylor-Green step failed: {source}"),
            Self::Diagnostic(source) => write!(f, "Taylor-Green diagnostic failed: {source}"),
            Self::MissingMeasuredEnergy => {
                write!(
                    f,
                    "Taylor-Green comparator requires measured kinetic energy"
                )
            }
            Self::NonFiniteComparison => {
                write!(f, "Taylor-Green comparison produced non-finite data")
            }
        }
    }
}

impl std::error::Error for TaylorGreenErrorKind {}

impl From<ReferenceStateError> for TaylorGreenErrorKind {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for TaylorGreenErrorKind {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for TaylorGreenErrorKind {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

/// Compare the numerical state to the exact unforced two-dimensional
/// Taylor–Green solution at the state's current time.
///
/// For the canonical square periodic mode,
///
/// `u = A sin(kx) cos(ky) exp(-2 nu k^2 t)`
/// `v = -A cos(kx) sin(ky) exp(-2 nu k^2 t)`.
///
/// Pressure is intentionally not used by V0.1: the velocity/energy comparator
/// is sufficient to establish the first smooth convergence ladder while
/// avoiding an unnecessary pressure-gauge convention.
pub fn compare_taylor_green(
    state: &PeriodicMac2d,
    initial_amplitude_mps: f64,
) -> Result<TaylorGreenError, TaylorGreenErrorKind> {
    validate_amplitude(initial_amplitude_mps)?;
    validate_canonical_config(state.config())?;

    let exact = exact_taylor_green_state(
        state.config().clone(),
        initial_amplitude_mps,
        state.time_s(),
    )?;

    let mut squared_error = 0.0;
    let mut maximum_error = 0.0_f64;
    let mut values = 0usize;
    for (actual, expected) in state
        .u_faces()
        .iter()
        .zip(exact.u_faces())
        .chain(state.v_faces().iter().zip(exact.v_faces()))
    {
        let error = (actual - expected).abs();
        squared_error += error * error;
        maximum_error = maximum_error.max(error);
        values += 1;
    }

    let velocity_rms_error_mps = (squared_error / values as f64).sqrt();
    let actual_energy = measured_kinetic_energy(state)?;
    let exact_energy = measured_kinetic_energy(&exact)?;
    let kinetic_energy_relative_error = (actual_energy - exact_energy).abs() / exact_energy;

    if !velocity_rms_error_mps.is_finite()
        || !maximum_error.is_finite()
        || !kinetic_energy_relative_error.is_finite()
    {
        return Err(TaylorGreenErrorKind::NonFiniteComparison);
    }

    Ok(TaylorGreenError {
        comparator_id: TAYLOR_GREEN_COMPARATOR_ID,
        time_s: state.time_s(),
        velocity_rms_error_mps,
        velocity_max_error_mps: maximum_error,
        kinetic_energy_relative_error,
    })
}

/// Execute one deterministic fixed-step Taylor–Green case and retain raw
/// numerical-quality maxima alongside the final analytical error.
pub fn run_taylor_green_case(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    final_time_s: f64,
    steps: usize,
) -> Result<TaylorGreenCaseReport, TaylorGreenErrorKind> {
    validate_amplitude(initial_amplitude_mps)?;
    validate_canonical_config(&config)?;
    if !final_time_s.is_finite() || final_time_s <= 0.0 {
        return Err(TaylorGreenErrorKind::InvalidFinalTime);
    }
    if steps == 0 {
        return Err(TaylorGreenErrorKind::InvalidStepCount);
    }

    let dt_s = final_time_s / steps as f64;
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(TaylorGreenErrorKind::InvalidFinalTime);
    }

    let solver_profile = config.profile_identity();
    let nx = config.nx;
    let ny = config.ny;
    let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude_mps)?;
    let mut maximum_observed_advective_cfl = 0.0_f64;
    let mut maximum_observed_divergence_rms_per_s = state.divergence_rms_per_s();
    let mut maximum_observed_pressure_residual_rms_pa_per_m2 = 0.0_f64;

    for _ in 0..steps {
        let report = state.step(dt_s)?;
        maximum_observed_advective_cfl =
            maximum_observed_advective_cfl.max(report.max_advective_cfl);
        maximum_observed_divergence_rms_per_s =
            maximum_observed_divergence_rms_per_s.max(report.projection.divergence_rms_after_per_s);
        maximum_observed_pressure_residual_rms_pa_per_m2 =
            maximum_observed_pressure_residual_rms_pa_per_m2
                .max(report.projection.pressure_residual_rms_pa_per_m2);
    }

    let final_error = compare_taylor_green(&state, initial_amplitude_mps)?;
    Ok(TaylorGreenCaseReport {
        comparator_id: TAYLOR_GREEN_COMPARATOR_ID,
        solver_profile,
        nx,
        ny,
        steps,
        dt_s,
        final_time_s: state.time_s(),
        maximum_observed_advective_cfl,
        maximum_observed_divergence_rms_per_s,
        maximum_observed_pressure_residual_rms_pa_per_m2,
        final_error,
    })
}

/// Calculate an observed convergence order from two positive finite errors and
/// a resolution ratio greater than one. This is a measurement helper only; it
/// does not decide whether the order is acceptable.
pub fn observed_order(coarse_error: f64, fine_error: f64, refinement_ratio: f64) -> Option<f64> {
    if !coarse_error.is_finite()
        || !fine_error.is_finite()
        || !refinement_ratio.is_finite()
        || coarse_error <= 0.0
        || fine_error <= 0.0
        || refinement_ratio <= 1.0
    {
        return None;
    }
    let order = (coarse_error / fine_error).ln() / refinement_ratio.ln();
    order.is_finite().then_some(order)
}

fn validate_amplitude(amplitude_mps: f64) -> Result<(), TaylorGreenErrorKind> {
    if !amplitude_mps.is_finite() || amplitude_mps <= 0.0 {
        return Err(TaylorGreenErrorKind::InvalidAmplitude);
    }
    Ok(())
}

fn validate_canonical_config(config: &PeriodicMacConfig) -> Result<(), TaylorGreenErrorKind> {
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(TaylorGreenErrorKind::NonCanonicalDomain);
    }
    Ok(())
}

fn exact_taylor_green_state(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    time_s: f64,
) -> Result<PeriodicMac2d, TaylorGreenErrorKind> {
    validate_canonical_config(&config)?;
    validate_amplitude(initial_amplitude_mps)?;
    if !time_s.is_finite() || time_s < 0.0 {
        return Err(TaylorGreenErrorKind::NonFiniteComparison);
    }

    let dx = config.dx();
    let dy = config.dy();
    let k = std::f64::consts::TAU / config.length_x_m;
    let decay = (-2.0 * config.kinematic_viscosity_m2_s * k * k * time_s).exp();
    let amplitude = initial_amplitude_mps * decay;
    let mut u_faces = vec![0.0; config.nx * config.ny];
    let mut v_faces = vec![0.0; config.nx * config.ny];

    for j in 0..config.ny {
        for i in 0..config.nx {
            let index = j * config.nx + i;
            let xu = i as f64 * dx;
            let yu = (j as f64 + 0.5) * dy;
            let xv = (i as f64 + 0.5) * dx;
            let yv = j as f64 * dy;
            u_faces[index] = amplitude * (k * xu).sin() * (k * yu).cos();
            v_faces[index] = -amplitude * (k * xv).cos() * (k * yv).sin();
        }
    }

    Ok(PeriodicMac2d::from_faces(config, u_faces, v_faces)?)
}

fn measured_kinetic_energy(state: &PeriodicMac2d) -> Result<f64, TaylorGreenErrorKind> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(TaylorGreenErrorKind::MissingMeasuredEnergy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PeriodicMacConfig {
        PeriodicMacConfig {
            nx: 16,
            ny: 16,
            length_x_m: std::f64::consts::TAU,
            length_y_m: std::f64::consts::TAU,
            slab_depth_m: 1.0,
            density_kg_m3: 1.0,
            kinematic_viscosity_m2_s: 0.01,
            pressure_iterations: 800,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }

    #[test]
    fn analytical_comparator_matches_canonical_initializer_at_t0() {
        let state = PeriodicMac2d::taylor_green(test_config(), 0.25).unwrap();
        let error = compare_taylor_green(&state, 0.25).unwrap();
        assert!(error.velocity_rms_error_mps < 1.0e-14);
        assert!(error.velocity_max_error_mps < 1.0e-14);
        assert!(error.kinetic_energy_relative_error < 1.0e-14);
    }

    #[test]
    fn exact_discrete_energy_follows_squared_velocity_decay() {
        let config = test_config();
        let initial = exact_taylor_green_state(config.clone(), 0.25, 0.0).unwrap();
        let later = exact_taylor_green_state(config.clone(), 0.25, 0.7).unwrap();
        let initial_energy = measured_kinetic_energy(&initial).unwrap();
        let later_energy = measured_kinetic_energy(&later).unwrap();
        let k = std::f64::consts::TAU / config.length_x_m;
        let expected_ratio = (-4.0 * config.kinematic_viscosity_m2_s * k * k * 0.7).exp();
        assert!(((later_energy / initial_energy) - expected_ratio).abs() < 1.0e-13);
    }

    #[test]
    fn case_runner_emits_finite_measurements_without_assigning_a_pass_policy() {
        let report = run_taylor_green_case(test_config(), 0.10, 0.01, 10).unwrap();
        assert_eq!(report.comparator_id, TAYLOR_GREEN_COMPARATOR_ID);
        assert_eq!(report.steps, 10);
        assert!((report.final_time_s - 0.01).abs() < 1.0e-15);
        assert!(report.final_error.velocity_rms_error_mps.is_finite());
        assert!(report.final_error.velocity_max_error_mps.is_finite());
        assert!(report.final_error.kinetic_energy_relative_error.is_finite());
        assert!(report.maximum_observed_advective_cfl.is_finite());
        assert!(report.maximum_observed_divergence_rms_per_s.is_finite());
        assert!(
            report
                .maximum_observed_pressure_residual_rms_pa_per_m2
                .is_finite()
        );
    }

    #[test]
    fn case_runner_is_replay_deterministic_within_one_backend_profile() {
        let a = run_taylor_green_case(test_config(), 0.10, 0.005, 5).unwrap();
        let b = run_taylor_green_case(test_config(), 0.10, 0.005, 5).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn comparator_rejects_noncanonical_rectangular_case() {
        let mut config = test_config();
        config.ny = 12;
        let state = PeriodicMac2d::zeros(config).unwrap();
        assert_eq!(
            compare_taylor_green(&state, 0.1),
            Err(TaylorGreenErrorKind::NonCanonicalDomain)
        );
    }

    #[test]
    fn observed_order_is_measurement_only_and_fail_closed_on_invalid_inputs() {
        let order = observed_order(0.04, 0.01, 2.0).unwrap();
        assert!((order - 2.0).abs() < 1.0e-12);
        assert_eq!(observed_order(0.0, 0.01, 2.0), None);
        assert_eq!(observed_order(0.04, f64::NAN, 2.0), None);
        assert_eq!(observed_order(0.04, 0.01, 1.0), None);
    }
}
