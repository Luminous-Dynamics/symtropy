// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Machine-readable smooth-flow verification ladders.
//!
//! This module preserves raw observables across spatial or temporal refinement.
//! It calculates observed orders but deliberately does not assign a scientific
//! acceptance threshold. Policy should be frozen only after retained evidence
//! demonstrates the regime in which the measured order is meaningful.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::convergence::{TaylorGreenErrorKind, observed_order, run_taylor_green_case};
use crate::manufactured::{
    ManufacturedTaylorGreenError, ManufacturedTaylorGreenProfile,
    run_manufactured_taylor_green_case,
};
use crate::reference::{PeriodicMac2d, PeriodicMacConfig, ReferenceDiagnosticError, ReferenceStepError};

pub const VERIFICATION_LADDER_SCHEMA_ID: &str = "continuum-verification-ladder-v0.1";
pub const ENERGY_TRACE_SCHEMA_ID: &str = "continuum-energy-trace-v0.1";
pub const MAX_LADDER_CASES: usize = 16;
pub const MAX_ENERGY_TRACE_STEPS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefinementAxis {
    Spatial,
    Temporal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationCaseKind {
    PassiveTaylorGreen,
    ManufacturedTaylorGreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LadderCaseSpec {
    pub resolution: usize,
    pub steps: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerificationLadderPoint {
    pub solver_profile: String,
    pub manufactured_profile: Option<String>,
    pub resolution: usize,
    pub steps: usize,
    pub minimum_resolved_length_m: f64,
    pub dt_s: f64,
    pub final_time_s: f64,
    pub velocity_rms_error_mps: f64,
    pub velocity_max_error_mps: f64,
    pub kinetic_energy_relative_error: f64,
    pub maximum_observed_advective_cfl: f64,
    pub maximum_observed_divergence_rms_per_s: f64,
    pub maximum_observed_pressure_residual_rms_pa_per_m2: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservedOrderPair {
    pub coarse_index: usize,
    pub fine_index: usize,
    pub refinement_ratio: f64,
    pub velocity_rms_order: Option<f64>,
    pub kinetic_energy_order: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerificationLadderReport {
    pub schema_id: String,
    pub case_kind: VerificationCaseKind,
    pub refinement_axis: RefinementAxis,
    pub points: Vec<VerificationLadderPoint>,
    pub adjacent_observed_orders: Vec<ObservedOrderPair>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyTraceReport {
    pub schema_id: String,
    pub solver_profile: String,
    pub dt_s: f64,
    pub steps: usize,
    pub energies_j: Vec<f64>,
    pub maximum_step_energy_increase_j: f64,
    pub total_energy_change_j: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VerificationLadderError {
    TooFewCases,
    TooManyCases,
    InvalidResolution,
    InvalidStepCount,
    InvalidRefinementSequence,
    InvalidEnergyTrace,
    MissingMeasuredEnergy,
    Passive(TaylorGreenErrorKind),
    Manufactured(ManufacturedTaylorGreenError),
    Diagnostic(ReferenceDiagnosticError),
    Step(ReferenceStepError),
}

impl fmt::Display for VerificationLadderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewCases => write!(f, "verification ladder requires at least two cases"),
            Self::TooManyCases => write!(
                f,
                "verification ladder exceeds the bounded case count {MAX_LADDER_CASES}"
            ),
            Self::InvalidResolution => write!(f, "ladder resolution must be at least four"),
            Self::InvalidStepCount => write!(f, "ladder step count must be positive"),
            Self::InvalidRefinementSequence => write!(
                f,
                "ladder cases do not form a strictly refined sequence on the declared axis"
            ),
            Self::InvalidEnergyTrace => write!(
                f,
                "energy trace requires finite positive dt and a bounded positive step count"
            ),
            Self::MissingMeasuredEnergy => write!(f, "energy trace requires measured kinetic energy"),
            Self::Passive(source) => write!(f, "passive Taylor-Green case failed: {source}"),
            Self::Manufactured(source) => write!(f, "manufactured Taylor-Green case failed: {source}"),
            Self::Diagnostic(source) => write!(f, "energy diagnostic failed: {source}"),
            Self::Step(source) => write!(f, "energy trace step failed: {source}"),
        }
    }
}

impl std::error::Error for VerificationLadderError {}

impl From<TaylorGreenErrorKind> for VerificationLadderError {
    fn from(value: TaylorGreenErrorKind) -> Self {
        Self::Passive(value)
    }
}

impl From<ManufacturedTaylorGreenError> for VerificationLadderError {
    fn from(value: ManufacturedTaylorGreenError) -> Self {
        Self::Manufactured(value)
    }
}

impl From<ReferenceDiagnosticError> for VerificationLadderError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<ReferenceStepError> for VerificationLadderError {
    fn from(value: ReferenceStepError) -> Self {
        Self::Step(value)
    }
}

pub fn run_passive_taylor_green_ladder(
    base_config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    final_time_s: f64,
    refinement_axis: RefinementAxis,
    cases: &[LadderCaseSpec],
) -> Result<VerificationLadderReport, VerificationLadderError> {
    validate_case_specs(refinement_axis, cases)?;
    let mut points = Vec::with_capacity(cases.len());

    for case in cases {
        let mut config = base_config.clone();
        config.nx = case.resolution;
        config.ny = case.resolution;
        let minimum_resolved_length_m = config.dx().min(config.dy());
        let report = run_taylor_green_case(
            config,
            initial_amplitude_mps,
            final_time_s,
            case.steps,
        )?;
        points.push(VerificationLadderPoint {
            solver_profile: report.solver_profile,
            manufactured_profile: None,
            resolution: case.resolution,
            steps: report.steps,
            minimum_resolved_length_m,
            dt_s: report.dt_s,
            final_time_s: report.final_time_s,
            velocity_rms_error_mps: report.final_error.velocity_rms_error_mps,
            velocity_max_error_mps: report.final_error.velocity_max_error_mps,
            kinetic_energy_relative_error: report.final_error.kinetic_energy_relative_error,
            maximum_observed_advective_cfl: report.maximum_observed_advective_cfl,
            maximum_observed_divergence_rms_per_s: report.maximum_observed_divergence_rms_per_s,
            maximum_observed_pressure_residual_rms_pa_per_m2: report
                .maximum_observed_pressure_residual_rms_pa_per_m2,
        });
    }

    finish_ladder(
        VerificationCaseKind::PassiveTaylorGreen,
        refinement_axis,
        points,
    )
}

pub fn run_manufactured_taylor_green_ladder(
    base_config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    final_time_s: f64,
    refinement_axis: RefinementAxis,
    cases: &[LadderCaseSpec],
) -> Result<VerificationLadderReport, VerificationLadderError> {
    validate_case_specs(refinement_axis, cases)?;
    let mut points = Vec::with_capacity(cases.len());

    for case in cases {
        let mut config = base_config.clone();
        config.nx = case.resolution;
        config.ny = case.resolution;
        let minimum_resolved_length_m = config.dx().min(config.dy());
        let report = run_manufactured_taylor_green_case(config, profile, final_time_s, case.steps)?;
        points.push(VerificationLadderPoint {
            solver_profile: report.solver_profile,
            manufactured_profile: Some(report.manufactured_profile),
            resolution: case.resolution,
            steps: report.steps,
            minimum_resolved_length_m,
            dt_s: report.dt_s,
            final_time_s: report.final_time_s,
            velocity_rms_error_mps: report.final_error.velocity_rms_error_mps,
            velocity_max_error_mps: report.final_error.velocity_max_error_mps,
            kinetic_energy_relative_error: report.final_error.kinetic_energy_relative_error,
            maximum_observed_advective_cfl: report.maximum_observed_advective_cfl,
            maximum_observed_divergence_rms_per_s: report.maximum_observed_divergence_rms_per_s,
            maximum_observed_pressure_residual_rms_pa_per_m2: report
                .maximum_observed_pressure_residual_rms_pa_per_m2,
        });
    }

    finish_ladder(
        VerificationCaseKind::ManufacturedTaylorGreen,
        refinement_axis,
        points,
    )
}

/// Record the raw kinetic-energy history of an unforced Taylor-Green run.
///
/// `maximum_step_energy_increase_j` is retained as an observable rather than a
/// baked-in verdict. For a stable unforced viscous run it should be zero up to
/// floating-point noise; callers may freeze a tolerance in a later policy rung.
pub fn run_unforced_energy_trace(
    config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    dt_s: f64,
    steps: usize,
) -> Result<EnergyTraceReport, VerificationLadderError> {
    if !dt_s.is_finite()
        || dt_s <= 0.0
        || steps == 0
        || steps > MAX_ENERGY_TRACE_STEPS
    {
        return Err(VerificationLadderError::InvalidEnergyTrace);
    }

    let solver_profile = config.profile_identity();
    let mut state = PeriodicMac2d::taylor_green(config, initial_amplitude_mps)
        .map_err(TaylorGreenErrorKind::State)?;
    let mut energies_j = Vec::with_capacity(steps + 1);
    energies_j.push(measured_energy(&state)?);
    let mut maximum_step_energy_increase_j = 0.0_f64;

    for _ in 0..steps {
        state.step(dt_s)?;
        let energy = measured_energy(&state)?;
        let previous = *energies_j.last().ok_or(VerificationLadderError::MissingMeasuredEnergy)?;
        maximum_step_energy_increase_j = maximum_step_energy_increase_j.max(energy - previous);
        energies_j.push(energy);
    }

    let initial_energy = energies_j[0];
    let final_energy = *energies_j
        .last()
        .ok_or(VerificationLadderError::MissingMeasuredEnergy)?;
    Ok(EnergyTraceReport {
        schema_id: ENERGY_TRACE_SCHEMA_ID.to_owned(),
        solver_profile,
        dt_s,
        steps,
        energies_j,
        maximum_step_energy_increase_j,
        total_energy_change_j: final_energy - initial_energy,
    })
}

fn finish_ladder(
    case_kind: VerificationCaseKind,
    refinement_axis: RefinementAxis,
    points: Vec<VerificationLadderPoint>,
) -> Result<VerificationLadderReport, VerificationLadderError> {
    let mut adjacent_observed_orders = Vec::with_capacity(points.len().saturating_sub(1));
    for index in 0..points.len().saturating_sub(1) {
        let coarse = &points[index];
        let fine = &points[index + 1];
        let refinement_ratio = match refinement_axis {
            RefinementAxis::Spatial => {
                coarse.minimum_resolved_length_m / fine.minimum_resolved_length_m
            }
            RefinementAxis::Temporal => coarse.dt_s / fine.dt_s,
        };
        if !refinement_ratio.is_finite() || refinement_ratio <= 1.0 {
            return Err(VerificationLadderError::InvalidRefinementSequence);
        }
        adjacent_observed_orders.push(ObservedOrderPair {
            coarse_index: index,
            fine_index: index + 1,
            refinement_ratio,
            velocity_rms_order: observed_order(
                coarse.velocity_rms_error_mps,
                fine.velocity_rms_error_mps,
                refinement_ratio,
            ),
            kinetic_energy_order: observed_order(
                coarse.kinetic_energy_relative_error,
                fine.kinetic_energy_relative_error,
                refinement_ratio,
            ),
        });
    }

    Ok(VerificationLadderReport {
        schema_id: VERIFICATION_LADDER_SCHEMA_ID.to_owned(),
        case_kind,
        refinement_axis,
        points,
        adjacent_observed_orders,
    })
}

fn validate_case_specs(
    refinement_axis: RefinementAxis,
    cases: &[LadderCaseSpec],
) -> Result<(), VerificationLadderError> {
    if cases.len() < 2 {
        return Err(VerificationLadderError::TooFewCases);
    }
    if cases.len() > MAX_LADDER_CASES {
        return Err(VerificationLadderError::TooManyCases);
    }
    if cases.iter().any(|case| case.resolution < 4) {
        return Err(VerificationLadderError::InvalidResolution);
    }
    if cases.iter().any(|case| case.steps == 0) {
        return Err(VerificationLadderError::InvalidStepCount);
    }

    for pair in cases.windows(2) {
        let coarse = pair[0];
        let fine = pair[1];
        let valid = match refinement_axis {
            RefinementAxis::Spatial => fine.resolution > coarse.resolution,
            RefinementAxis::Temporal => {
                fine.resolution == coarse.resolution && fine.steps > coarse.steps
            }
        };
        if !valid {
            return Err(VerificationLadderError::InvalidRefinementSequence);
        }
    }
    Ok(())
}

fn measured_energy(state: &PeriodicMac2d) -> Result<f64, VerificationLadderError> {
    state
        .diagnostics()?
        .kinetic_energy_j
        .measured_value()
        .ok_or(VerificationLadderError::MissingMeasuredEnergy)
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

    fn manufactured_profile() -> ManufacturedTaylorGreenProfile {
        ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.08,
            modulation_fraction: 0.2,
            angular_frequency_rad_s: 1.5,
        }
    }

    #[test]
    fn temporal_ladder_retains_raw_points_and_orders_without_a_verdict() {
        let cases = [
            LadderCaseSpec {
                resolution: 12,
                steps: 2,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 4,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 8,
            },
        ];
        let report = run_manufactured_taylor_green_ladder(
            config(),
            manufactured_profile(),
            0.002,
            RefinementAxis::Temporal,
            &cases,
        )
        .unwrap();
        assert_eq!(report.schema_id, VERIFICATION_LADDER_SCHEMA_ID);
        assert_eq!(report.points.len(), 3);
        assert_eq!(report.adjacent_observed_orders.len(), 2);
        assert!(report
            .adjacent_observed_orders
            .iter()
            .all(|pair| pair.refinement_ratio > 1.0));
    }

    #[test]
    fn spatial_ladder_retains_resolution_identity() {
        let cases = [
            LadderCaseSpec {
                resolution: 8,
                steps: 8,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 12,
            },
        ];
        let report = run_passive_taylor_green_ladder(
            config(),
            0.08,
            0.002,
            RefinementAxis::Spatial,
            &cases,
        )
        .unwrap();
        assert_eq!(report.points[0].resolution, 8);
        assert_eq!(report.points[1].resolution, 12);
        assert!(report.adjacent_observed_orders[0].refinement_ratio > 1.0);
    }

    #[test]
    fn invalid_refinement_sequences_fail_closed() {
        let cases = [
            LadderCaseSpec {
                resolution: 12,
                steps: 4,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 8,
            },
        ];
        assert_eq!(
            run_passive_taylor_green_ladder(
                config(),
                0.08,
                0.002,
                RefinementAxis::Spatial,
                &cases,
            ),
            Err(VerificationLadderError::InvalidRefinementSequence)
        );
    }

    #[test]
    fn unforced_viscous_energy_trace_does_not_gain_resolved_energy() {
        let report = run_unforced_energy_trace(config(), 0.08, 0.0005, 8).unwrap();
        assert_eq!(report.energies_j.len(), 9);
        assert!(report.total_energy_change_j <= 0.0);
        assert!(report.maximum_step_energy_increase_j <= 1.0e-14);
    }
}
