// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Manufactured smooth forced solution for continuum code verification.
//!
//! This module deliberately stays below the 2026 singular-flow lineage. It
//! drives the canonical two-dimensional Taylor-Green spatial mode with a known
//! time-dependent amplitude, so forcing, advection, viscosity, projection, and
//! time integration can be checked against a closed-form velocity field.

use std::fmt;

use crate::convergence::{TAYLOR_GREEN_COMPARATOR_ID, TaylorGreenError};
use crate::reference::{
    PeriodicMac2d, PeriodicMacConfig, ReferenceDiagnosticError, ReferenceStateError,
    ReferenceStepError,
};

/// Versioned identity of this manufactured-solution family.
pub const MANUFACTURED_TAYLOR_GREEN_ID: &str = "manufactured-taylor-green-periodic-2d-v0.1";

/// Time law `q(t) = A * (1 + m * sin(omega * t))` for the Taylor-Green mode.
///
/// `0 <= m < 1` keeps the exact amplitude positive, avoiding a zero-energy
/// denominator in relative-error diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManufacturedTaylorGreenProfile {
    pub base_amplitude_mps: f64,
    pub modulation_fraction: f64,
    pub angular_frequency_rad_s: f64,
}

impl ManufacturedTaylorGreenProfile {
    pub fn validate(&self) -> Result<(), ManufacturedTaylorGreenError> {
        if !self.base_amplitude_mps.is_finite() || self.base_amplitude_mps <= 0.0 {
            return Err(ManufacturedTaylorGreenError::InvalidBaseAmplitude);
        }
        if !self.modulation_fraction.is_finite()
            || self.modulation_fraction < 0.0
            || self.modulation_fraction >= 1.0
        {
            return Err(ManufacturedTaylorGreenError::InvalidModulation);
        }
        if !self.angular_frequency_rad_s.is_finite() || self.angular_frequency_rad_s <= 0.0 {
            return Err(ManufacturedTaylorGreenError::InvalidAngularFrequency);
        }
        Ok(())
    }

    pub fn amplitude_mps(&self, time_s: f64) -> Result<f64, ManufacturedTaylorGreenError> {
        self.validate()?;
        if !time_s.is_finite() || time_s < 0.0 {
            return Err(ManufacturedTaylorGreenError::InvalidTime);
        }
        let value = self.base_amplitude_mps
            * (1.0 + self.modulation_fraction * (self.angular_frequency_rad_s * time_s).sin());
        if !value.is_finite() || value <= 0.0 {
            return Err(ManufacturedTaylorGreenError::NonFiniteManufacturedValue);
        }
        Ok(value)
    }

    pub fn amplitude_derivative_mps2(
        &self,
        time_s: f64,
    ) -> Result<f64, ManufacturedTaylorGreenError> {
        self.validate()?;
        if !time_s.is_finite() || time_s < 0.0 {
            return Err(ManufacturedTaylorGreenError::InvalidTime);
        }
        let value = self.base_amplitude_mps
            * self.modulation_fraction
            * self.angular_frequency_rad_s
            * (self.angular_frequency_rad_s * time_s).cos();
        if !value.is_finite() {
            return Err(ManufacturedTaylorGreenError::NonFiniteManufacturedValue);
        }
        Ok(value)
    }

    /// Exact bit-preserving identity for the time law, independent of solver
    /// configuration identity.
    pub fn profile_identity(&self) -> String {
        format!(
            "{MANUFACTURED_TAYLOR_GREEN_ID};A={:016x};m={:016x};w={:016x}",
            self.base_amplitude_mps.to_bits(),
            self.modulation_fraction.to_bits(),
            self.angular_frequency_rad_s.to_bits(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ManufacturedTaylorGreenCaseReport {
    pub manufactured_profile: String,
    pub solver_profile: String,
    pub comparator_id: &'static str,
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
pub enum ManufacturedTaylorGreenError {
    NonCanonicalDomain,
    InvalidBaseAmplitude,
    InvalidModulation,
    InvalidAngularFrequency,
    InvalidTime,
    InvalidFinalTime,
    InvalidStepCount,
    NonFiniteManufacturedValue,
    State(ReferenceStateError),
    Step(ReferenceStepError),
    Diagnostic(ReferenceDiagnosticError),
    MissingMeasuredEnergy,
}

impl fmt::Display for ManufacturedTaylorGreenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalDomain => write!(
                f,
                "manufactured Taylor-Green V0.1 requires a square periodic grid/domain"
            ),
            Self::InvalidBaseAmplitude => {
                write!(f, "base_amplitude_mps must be finite and > 0")
            }
            Self::InvalidModulation => {
                write!(f, "modulation_fraction must be finite and in [0, 1)")
            }
            Self::InvalidAngularFrequency => {
                write!(f, "angular_frequency_rad_s must be finite and > 0")
            }
            Self::InvalidTime => write!(f, "time_s must be finite and >= 0"),
            Self::InvalidFinalTime => write!(f, "final_time_s must be finite and > 0"),
            Self::InvalidStepCount => write!(f, "manufactured case requires at least one step"),
            Self::NonFiniteManufacturedValue => {
                write!(f, "manufactured solution/forcing produced a non-finite value")
            }
            Self::State(source) => write!(f, "invalid manufactured state: {source}"),
            Self::Step(source) => write!(f, "manufactured step failed: {source}"),
            Self::Diagnostic(source) => write!(f, "manufactured diagnostic failed: {source}"),
            Self::MissingMeasuredEnergy => {
                write!(f, "manufactured comparator requires measured kinetic energy")
            }
        }
    }
}

impl std::error::Error for ManufacturedTaylorGreenError {}

impl From<ReferenceStateError> for ManufacturedTaylorGreenError {
    fn from(value: ReferenceStateError) -> Self {
        Self::State(value)
    }
}

impl From<ReferenceStepError> for ManufacturedTaylorGreenError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

impl From<ReferenceDiagnosticError> for ManufacturedTaylorGreenError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

/// Divergence-free body-acceleration amplitude required by the continuous
/// incompressible Navier-Stokes equations for the prescribed `q(t)` mode.
///
/// The nonlinear Taylor-Green self-advection is a pure pressure-gradient term;
/// therefore the solenoidal body forcing only needs
/// `q'(t) + 2 * nu * k^2 * q(t)`.
pub fn forcing_amplitude_mps2(
    config: &PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    time_s: f64,
) -> Result<f64, ManufacturedTaylorGreenError> {
    validate_canonical_config(config)?;
    let q = profile.amplitude_mps(time_s)?;
    let q_dot = profile.amplitude_derivative_mps2(time_s)?;
    let k = std::f64::consts::TAU / config.length_x_m;
    let value = q_dot + 2.0 * config.kinematic_viscosity_m2_s * k * k * q;
    if !value.is_finite() {
        return Err(ManufacturedTaylorGreenError::NonFiniteManufacturedValue);
    }
    Ok(value)
}

/// Evaluate the manufactured divergence-free acceleration field at a point.
pub fn manufactured_acceleration_mps2(
    config: &PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    position_m: [f64; 2],
    time_s: f64,
) -> Result<[f64; 2], ManufacturedTaylorGreenError> {
    validate_canonical_config(config)?;
    if position_m.iter().any(|value| !value.is_finite()) {
        return Err(ManufacturedTaylorGreenError::NonFiniteManufacturedValue);
    }
    let amplitude = forcing_amplitude_mps2(config, profile, time_s)?;
    let k = std::f64::consts::TAU / config.length_x_m;
    let [x, y] = position_m;
    let acceleration = [
        amplitude * (k * x).sin() * (k * y).cos(),
        -amplitude * (k * x).cos() * (k * y).sin(),
    ];
    if acceleration.iter().any(|value| !value.is_finite()) {
        return Err(ManufacturedTaylorGreenError::NonFiniteManufacturedValue);
    }
    Ok(acceleration)
}

/// Execute one deterministic manufactured Taylor-Green case.
///
/// This records error; it intentionally does not decide whether the error or an
/// observed convergence order is acceptable.
pub fn run_manufactured_taylor_green_case(
    config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    final_time_s: f64,
    steps: usize,
) -> Result<ManufacturedTaylorGreenCaseReport, ManufacturedTaylorGreenError> {
    config.validate().map_err(ReferenceStateError::Config)?;
    validate_canonical_config(&config)?;
    profile.validate()?;
    if !final_time_s.is_finite() || final_time_s <= 0.0 {
        return Err(ManufacturedTaylorGreenError::InvalidFinalTime);
    }
    if steps == 0 {
        return Err(ManufacturedTaylorGreenError::InvalidStepCount);
    }
    let dt_s = final_time_s / steps as f64;
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(ManufacturedTaylorGreenError::InvalidFinalTime);
    }

    let initial_amplitude = profile.amplitude_mps(0.0)?;
    let solver_profile = config.profile_identity();
    let manufactured_profile = profile.profile_identity();
    let nx = config.nx;
    let ny = config.ny;
    let forcing_config = config.clone();
    let mut state = PeriodicMac2d::taylor_green(config.clone(), initial_amplitude)?;
    let mut maximum_observed_advective_cfl = 0.0_f64;
    let mut maximum_observed_divergence_rms_per_s = state.divergence_rms_per_s();
    let mut maximum_observed_pressure_residual_rms_pa_per_m2 = 0.0_f64;

    for _ in 0..steps {
        let report = state.step_with_acceleration(dt_s, |position, time| {
            manufactured_acceleration_mps2(&forcing_config, profile, position, time)
                .unwrap_or([f64::NAN, f64::NAN])
        })?;
        maximum_observed_advective_cfl =
            maximum_observed_advective_cfl.max(report.max_advective_cfl);
        maximum_observed_divergence_rms_per_s = maximum_observed_divergence_rms_per_s
            .max(report.projection.divergence_rms_after_per_s);
        maximum_observed_pressure_residual_rms_pa_per_m2 =
            maximum_observed_pressure_residual_rms_pa_per_m2
                .max(report.projection.pressure_residual_rms_pa_per_m2);
    }

    let exact_amplitude = profile.amplitude_mps(state.time_s())?;
    let exact = PeriodicMac2d::taylor_green(config, exact_amplitude)?;
    let final_error = compare_states(&state, &exact)?;

    Ok(ManufacturedTaylorGreenCaseReport {
        manufactured_profile,
        solver_profile,
        comparator_id: TAYLOR_GREEN_COMPARATOR_ID,
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

fn compare_states(
    actual: &PeriodicMac2d,
    exact: &PeriodicMac2d,
) -> Result<TaylorGreenError, ManufacturedTaylorGreenError> {
    let mut squared_error = 0.0;
    let mut maximum_error = 0.0_f64;
    let mut count = 0usize;
    for (observed, expected) in actual
        .u_faces()
        .iter()
        .zip(exact.u_faces())
        .chain(actual.v_faces().iter().zip(exact.v_faces()))
    {
        let error = (observed - expected).abs();
        squared_error += error * error;
        maximum_error = maximum_error.max(error);
        count += 1;
    }
    let velocity_rms_error_mps = (squared_error / count as f64).sqrt();
    let actual_energy = measured_energy(actual)?;
    let exact_energy = measured_energy(exact)?;
    let kinetic_energy_relative_error = (actual_energy - exact_energy).abs() / exact_energy;
    if !velocity_rms_error_mps.is_finite()
        || !maximum_error.is_finite()
        || !kinetic_energy_relative_error.is_finite()
    {
        return Err(ManufacturedTaylorGreenError::NonFiniteManufacturedValue);
    }
    Ok(TaylorGreenError {
        comparator_id: TAYLOR_GREEN_COMPARATOR_ID,
        time_s: actual.time_s(),
        velocity_rms_error_mps,
        velocity_max_error_mps: maximum_error,
        kinetic_energy_relative_error,
    })
}

fn measured_energy(state: &PeriodicMac2d) -> Result<f64, ManufacturedTaylorGreenError> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(ManufacturedTaylorGreenError::MissingMeasuredEnergy)
}

fn validate_canonical_config(
    config: &PeriodicMacConfig,
) -> Result<(), ManufacturedTaylorGreenError> {
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(ManufacturedTaylorGreenError::NonCanonicalDomain);
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
            pressure_iterations: 800,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }

    fn profile() -> ManufacturedTaylorGreenProfile {
        ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.1,
            modulation_fraction: 0.2,
            angular_frequency_rad_s: 2.0,
        }
    }

    #[test]
    fn amplitude_and_derivative_follow_the_declared_time_law() {
        let p = profile();
        assert!((p.amplitude_mps(0.0).unwrap() - 0.1).abs() < 1.0e-15);
        assert!((p.amplitude_derivative_mps2(0.0).unwrap() - 0.04).abs() < 1.0e-15);
    }

    #[test]
    fn zero_modulation_forcing_exactly_balances_continuum_viscous_decay() {
        let p = ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.25,
            modulation_fraction: 0.0,
            angular_frequency_rad_s: 1.0,
        };
        let cfg = config();
        let k = std::f64::consts::TAU / cfg.length_x_m;
        let expected = 2.0 * cfg.kinematic_viscosity_m2_s * k * k * 0.25;
        assert!((forcing_amplitude_mps2(&cfg, p, 0.7).unwrap() - expected).abs() < 1.0e-15);
    }

    #[test]
    fn manufactured_acceleration_is_divergence_free_under_continuum_mode() {
        let cfg = config();
        let p = profile();
        let a = manufactured_acceleration_mps2(&cfg, p, [0.0, 0.3], 0.2).unwrap();
        assert_eq!(a[0], 0.0);
        assert!(a[1].is_finite());
    }

    #[test]
    fn short_case_emits_finite_measurements_without_a_pass_policy() {
        let report = run_manufactured_taylor_green_case(config(), profile(), 0.005, 10).unwrap();
        assert_eq!(report.steps, 10);
        assert!(report.final_error.velocity_rms_error_mps.is_finite());
        assert!(report.final_error.velocity_max_error_mps.is_finite());
        assert!(report.final_error.kinetic_energy_relative_error.is_finite());
        assert!(report.maximum_observed_advective_cfl.is_finite());
        assert!(report.maximum_observed_divergence_rms_per_s.is_finite());
        assert!(report
            .maximum_observed_pressure_residual_rms_pa_per_m2
            .is_finite());
    }

    #[test]
    fn manufactured_case_replays_deterministically_within_one_profile() {
        let a = run_manufactured_taylor_green_case(config(), profile(), 0.002, 4).unwrap();
        let b = run_manufactured_taylor_green_case(config(), profile(), 0.002, 4).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn profile_identity_changes_with_manufactured_semantics() {
        let a = profile();
        let mut b = a;
        b.angular_frequency_rad_s *= 2.0;
        assert_ne!(a.profile_identity(), b.profile_identity());
    }

    #[test]
    fn invalid_profile_and_noncanonical_domain_fail_closed() {
        let mut bad = profile();
        bad.modulation_fraction = 1.0;
        assert_eq!(
            bad.validate(),
            Err(ManufacturedTaylorGreenError::InvalidModulation)
        );

        let mut cfg = config();
        cfg.ny = 12;
        assert_eq!(
            run_manufactured_taylor_green_case(cfg, profile(), 0.01, 10),
            Err(ManufacturedTaylorGreenError::NonCanonicalDomain)
        );
    }
}
