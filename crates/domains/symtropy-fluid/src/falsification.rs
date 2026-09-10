// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Falsification-oriented continuum verification campaigns.
//!
//! Strict ladders are useful when every case is expected to complete. They are
//! the wrong abstraction for stress/refinement campaigns, however, because a
//! CFL, stability-envelope, projection, forcing, or non-finite failure is itself
//! evidence. This module therefore preserves each attempted case as either a
//! completed measurement point or a typed failure record. Only malformed
//! campaign definitions are returned as outer errors.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::convergence::{
    TaylorGreenCaseReport, TaylorGreenErrorKind, observed_order, run_taylor_green_case,
};
use crate::manufactured::{
    ManufacturedTaylorGreenCaseReport, ManufacturedTaylorGreenError,
    ManufacturedTaylorGreenProfile, run_manufactured_taylor_green_case,
};
use crate::reference::{
    PeriodicMacConfig, ReferenceDiagnosticError, ReferenceStateError, ReferenceStepError,
};
use crate::validation::{ContinuumValidityState, DiagnosticValueError};
use crate::verification_ladder::{
    LadderCaseSpec, MAX_LADDER_CASES, RefinementAxis, VerificationCaseKind,
    VerificationLadderPoint,
};

pub const FALSIFICATION_CAMPAIGN_SCHEMA_ID: &str = "continuum-falsification-campaign-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationFailureKind {
    InvalidInput,
    StateInitializationFailure,
    StabilityLimitExceeded,
    ForcingEvaluationFailure,
    NonFiniteState,
    ProjectionFailure,
    DiagnosticFailure,
    ComparatorFailure,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerificationFailureEvidence {
    pub spec: LadderCaseSpec,
    pub validity_state: ContinuumValidityState,
    pub failure_kind: VerificationFailureKind,
    /// Raw scalar carried by the typed failure when one exists, for example an
    /// observed CFL or diffusion number.
    pub observed_value: Option<f64>,
    /// Declared solver/profile limit associated with `observed_value`, when the
    /// failure exposes one.
    pub configured_limit: Option<f64>,
    /// Stable human-readable detail from the internal typed error. This is
    /// evidence context, not a machine classification key.
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", content = "evidence", rename_all = "snake_case")]
pub enum VerificationCampaignOutcome {
    Completed(VerificationLadderPoint),
    Failed(VerificationFailureEvidence),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CampaignObservedOrderPair {
    pub coarse_outcome_index: usize,
    pub fine_outcome_index: usize,
    pub refinement_ratio: f64,
    pub velocity_rms_order: Option<f64>,
    pub kinetic_energy_order: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerificationCampaignReport {
    pub schema_id: String,
    pub case_kind: VerificationCaseKind,
    pub refinement_axis: RefinementAxis,
    pub outcomes: Vec<VerificationCampaignOutcome>,
    pub adjacent_observed_orders: Vec<CampaignObservedOrderPair>,
    pub completed_count: usize,
    pub failed_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationCampaignError {
    TooFewCases,
    TooManyCases,
    InvalidResolution,
    InvalidStepCount,
    InvalidRefinementSequence,
}

impl fmt::Display for VerificationCampaignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewCases => write!(f, "verification campaign requires at least two cases"),
            Self::TooManyCases => write!(
                f,
                "verification campaign exceeds the bounded case count {MAX_LADDER_CASES}"
            ),
            Self::InvalidResolution => write!(f, "campaign resolution must be at least four"),
            Self::InvalidStepCount => write!(f, "campaign step count must be positive"),
            Self::InvalidRefinementSequence => write!(
                f,
                "campaign cases do not form a strictly refined sequence on the declared axis"
            ),
        }
    }
}

impl std::error::Error for VerificationCampaignError {}

pub fn run_passive_taylor_green_campaign(
    base_config: PeriodicMacConfig,
    initial_amplitude_mps: f64,
    final_time_s: f64,
    refinement_axis: RefinementAxis,
    cases: &[LadderCaseSpec],
) -> Result<VerificationCampaignReport, VerificationCampaignError> {
    validate_case_specs(refinement_axis, cases)?;
    let mut outcomes = Vec::with_capacity(cases.len());

    for &spec in cases {
        let mut config = base_config.clone();
        config.nx = spec.resolution;
        config.ny = spec.resolution;
        let minimum_resolved_length_m = config.dx().min(config.dy());
        let outcome = match run_taylor_green_case(
            config,
            initial_amplitude_mps,
            final_time_s,
            spec.steps,
        ) {
            Ok(report) => VerificationCampaignOutcome::Completed(passive_point(
                report,
                minimum_resolved_length_m,
            )),
            Err(error) => VerificationCampaignOutcome::Failed(classify_passive_failure(
                spec, error,
            )),
        };
        outcomes.push(outcome);
    }

    finish_campaign(
        VerificationCaseKind::PassiveTaylorGreen,
        refinement_axis,
        outcomes,
    )
}

pub fn run_manufactured_taylor_green_campaign(
    base_config: PeriodicMacConfig,
    profile: ManufacturedTaylorGreenProfile,
    final_time_s: f64,
    refinement_axis: RefinementAxis,
    cases: &[LadderCaseSpec],
) -> Result<VerificationCampaignReport, VerificationCampaignError> {
    validate_case_specs(refinement_axis, cases)?;
    let mut outcomes = Vec::with_capacity(cases.len());

    for &spec in cases {
        let mut config = base_config.clone();
        config.nx = spec.resolution;
        config.ny = spec.resolution;
        let minimum_resolved_length_m = config.dx().min(config.dy());
        let outcome = match run_manufactured_taylor_green_case(
            config,
            profile,
            final_time_s,
            spec.steps,
        ) {
            Ok(report) => VerificationCampaignOutcome::Completed(manufactured_point(
                report,
                minimum_resolved_length_m,
            )),
            Err(error) => VerificationCampaignOutcome::Failed(classify_manufactured_failure(
                spec, error,
            )),
        };
        outcomes.push(outcome);
    }

    finish_campaign(
        VerificationCaseKind::ManufacturedTaylorGreen,
        refinement_axis,
        outcomes,
    )
}

fn passive_point(
    report: TaylorGreenCaseReport,
    minimum_resolved_length_m: f64,
) -> VerificationLadderPoint {
    VerificationLadderPoint {
        solver_profile: report.solver_profile,
        manufactured_profile: None,
        resolution: report.nx,
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
    }
}

fn manufactured_point(
    report: ManufacturedTaylorGreenCaseReport,
    minimum_resolved_length_m: f64,
) -> VerificationLadderPoint {
    VerificationLadderPoint {
        solver_profile: report.solver_profile,
        manufactured_profile: Some(report.manufactured_profile),
        resolution: report.nx,
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
    }
}

fn finish_campaign(
    case_kind: VerificationCaseKind,
    refinement_axis: RefinementAxis,
    outcomes: Vec<VerificationCampaignOutcome>,
) -> Result<VerificationCampaignReport, VerificationCampaignError> {
    let mut adjacent_observed_orders = Vec::new();

    for index in 0..outcomes.len().saturating_sub(1) {
        let (
            VerificationCampaignOutcome::Completed(coarse),
            VerificationCampaignOutcome::Completed(fine),
        ) = (&outcomes[index], &outcomes[index + 1])
        else {
            continue;
        };

        let refinement_ratio = match refinement_axis {
            RefinementAxis::Spatial => {
                coarse.minimum_resolved_length_m / fine.minimum_resolved_length_m
            }
            RefinementAxis::Temporal => coarse.dt_s / fine.dt_s,
        };
        if !refinement_ratio.is_finite() || refinement_ratio <= 1.0 {
            return Err(VerificationCampaignError::InvalidRefinementSequence);
        }

        adjacent_observed_orders.push(CampaignObservedOrderPair {
            coarse_outcome_index: index,
            fine_outcome_index: index + 1,
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

    let completed_count = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, VerificationCampaignOutcome::Completed(_)))
        .count();
    let failed_count = outcomes.len() - completed_count;

    Ok(VerificationCampaignReport {
        schema_id: FALSIFICATION_CAMPAIGN_SCHEMA_ID.to_owned(),
        case_kind,
        refinement_axis,
        outcomes,
        adjacent_observed_orders,
        completed_count,
        failed_count,
    })
}

fn validate_case_specs(
    refinement_axis: RefinementAxis,
    cases: &[LadderCaseSpec],
) -> Result<(), VerificationCampaignError> {
    if cases.len() < 2 {
        return Err(VerificationCampaignError::TooFewCases);
    }
    if cases.len() > MAX_LADDER_CASES {
        return Err(VerificationCampaignError::TooManyCases);
    }
    if cases.iter().any(|case| case.resolution < 4) {
        return Err(VerificationCampaignError::InvalidResolution);
    }
    if cases.iter().any(|case| case.steps == 0) {
        return Err(VerificationCampaignError::InvalidStepCount);
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
            return Err(VerificationCampaignError::InvalidRefinementSequence);
        }
    }
    Ok(())
}

fn classify_passive_failure(
    spec: LadderCaseSpec,
    error: TaylorGreenErrorKind,
) -> VerificationFailureEvidence {
    let detail = error.to_string();
    let (failure_kind, validity_state, observed_value, configured_limit) = match &error {
        TaylorGreenErrorKind::NonCanonicalDomain
        | TaylorGreenErrorKind::InvalidAmplitude
        | TaylorGreenErrorKind::InvalidFinalTime
        | TaylorGreenErrorKind::InvalidStepCount => (
            VerificationFailureKind::InvalidInput,
            ContinuumValidityState::ReferenceDomainViolation,
            None,
            None,
        ),
        TaylorGreenErrorKind::State(source) => classify_state_error(source),
        TaylorGreenErrorKind::Step(source) => classify_step_error(source),
        TaylorGreenErrorKind::Diagnostic(source) => classify_diagnostic_error(source),
        TaylorGreenErrorKind::MissingMeasuredEnergy => (
            VerificationFailureKind::DiagnosticFailure,
            ContinuumValidityState::BenchmarkProfileUnavailable,
            None,
            None,
        ),
        TaylorGreenErrorKind::NonFiniteComparison => (
            VerificationFailureKind::ComparatorFailure,
            ContinuumValidityState::NonFiniteState,
            None,
            None,
        ),
    };
    VerificationFailureEvidence {
        spec,
        validity_state,
        failure_kind,
        observed_value,
        configured_limit,
        detail,
    }
}

fn classify_manufactured_failure(
    spec: LadderCaseSpec,
    error: ManufacturedTaylorGreenError,
) -> VerificationFailureEvidence {
    let detail = error.to_string();
    let (failure_kind, validity_state, observed_value, configured_limit) = match &error {
        ManufacturedTaylorGreenError::NonCanonicalDomain
        | ManufacturedTaylorGreenError::InvalidBaseAmplitude
        | ManufacturedTaylorGreenError::InvalidModulation
        | ManufacturedTaylorGreenError::InvalidAngularFrequency
        | ManufacturedTaylorGreenError::InvalidTime
        | ManufacturedTaylorGreenError::InvalidFinalTime
        | ManufacturedTaylorGreenError::InvalidStepCount => (
            VerificationFailureKind::InvalidInput,
            ContinuumValidityState::ReferenceDomainViolation,
            None,
            None,
        ),
        ManufacturedTaylorGreenError::NonFiniteManufacturedValue => (
            VerificationFailureKind::ForcingEvaluationFailure,
            ContinuumValidityState::NonFiniteState,
            None,
            None,
        ),
        ManufacturedTaylorGreenError::State(source) => classify_state_error(source),
        ManufacturedTaylorGreenError::Step(source) => classify_step_error(source),
        ManufacturedTaylorGreenError::Diagnostic(source) => classify_diagnostic_error(source),
        ManufacturedTaylorGreenError::MissingMeasuredEnergy => (
            VerificationFailureKind::DiagnosticFailure,
            ContinuumValidityState::BenchmarkProfileUnavailable,
            None,
            None,
        ),
    };
    VerificationFailureEvidence {
        spec,
        validity_state,
        failure_kind,
        observed_value,
        configured_limit,
        detail,
    }
}

fn classify_state_error(
    error: &ReferenceStateError,
) -> (
    VerificationFailureKind,
    ContinuumValidityState,
    Option<f64>,
    Option<f64>,
) {
    match error {
        ReferenceStateError::Config(_) | ReferenceStateError::WrongFaceCount => (
            VerificationFailureKind::StateInitializationFailure,
            ContinuumValidityState::ReferenceDomainViolation,
            None,
            None,
        ),
        ReferenceStateError::NonFiniteFaceVelocity
        | ReferenceStateError::InvalidTaylorGreenAmplitude => (
            VerificationFailureKind::StateInitializationFailure,
            ContinuumValidityState::NonFiniteState,
            None,
            None,
        ),
    }
}

fn classify_step_error(
    error: &ReferenceStepError,
) -> (
    VerificationFailureKind,
    ContinuumValidityState,
    Option<f64>,
    Option<f64>,
) {
    match error {
        ReferenceStepError::InvalidDt => (
            VerificationFailureKind::InvalidInput,
            ContinuumValidityState::ReferenceDomainViolation,
            None,
            None,
        ),
        ReferenceStepError::CflLimitExceeded { observed, limit }
        | ReferenceStepError::PredictorCflLimitExceeded { observed, limit } => (
            VerificationFailureKind::StabilityLimitExceeded,
            ContinuumValidityState::CflViolation,
            Some(*observed),
            Some(*limit),
        ),
        ReferenceStepError::DiffusionLimitExceeded { observed, limit } => (
            VerificationFailureKind::StabilityLimitExceeded,
            ContinuumValidityState::ReferenceDomainViolation,
            Some(*observed),
            Some(*limit),
        ),
        ReferenceStepError::CombinedExplicitLimitExceeded { observed }
        | ReferenceStepError::PredictorCombinedExplicitLimitExceeded { observed } => (
            VerificationFailureKind::StabilityLimitExceeded,
            ContinuumValidityState::ReferenceDomainViolation,
            Some(*observed),
            Some(crate::reference::MAX_COMBINED_EXPLICIT_NUMBER),
        ),
        ReferenceStepError::NonFiniteAcceleration { .. } => (
            VerificationFailureKind::ForcingEvaluationFailure,
            ContinuumValidityState::NonFiniteState,
            None,
            None,
        ),
        ReferenceStepError::NonFinitePredictor { .. } => (
            VerificationFailureKind::NonFiniteState,
            ContinuumValidityState::NonFiniteState,
            None,
            None,
        ),
        ReferenceStepError::ProjectionScaleNonFinite | ReferenceStepError::NonFiniteProjection => (
            VerificationFailureKind::ProjectionFailure,
            ContinuumValidityState::NonFiniteState,
            None,
            None,
        ),
    }
}

fn classify_diagnostic_error(
    error: &ReferenceDiagnosticError,
) -> (
    VerificationFailureKind,
    ContinuumValidityState,
    Option<f64>,
    Option<f64>,
) {
    let validity_state = match error {
        ReferenceDiagnosticError::InvalidMetric {
            source: DiagnosticValueError::NonFinite,
            ..
        } => ContinuumValidityState::NonFiniteState,
        ReferenceDiagnosticError::InvalidMetric {
            source: DiagnosticValueError::Negative,
            ..
        } => ContinuumValidityState::BenchmarkProfileUnavailable,
    };
    (
        VerificationFailureKind::DiagnosticFailure,
        validity_state,
        None,
        None,
    )
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
    fn campaign_preserves_cfl_failure_and_later_success() {
        let cases = [
            LadderCaseSpec {
                resolution: 12,
                steps: 1,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 64,
            },
        ];
        let report = run_passive_taylor_green_campaign(
            config(),
            0.5,
            0.5,
            RefinementAxis::Temporal,
            &cases,
        )
        .unwrap();

        assert_eq!(report.failed_count, 1);
        assert_eq!(report.completed_count, 1);
        let VerificationCampaignOutcome::Failed(failure) = &report.outcomes[0] else {
            panic!("coarse case should preserve its CFL failure");
        };
        assert_eq!(failure.validity_state, ContinuumValidityState::CflViolation);
        assert_eq!(
            failure.failure_kind,
            VerificationFailureKind::StabilityLimitExceeded
        );
        assert!(failure.observed_value.unwrap() > failure.configured_limit.unwrap());
        assert!(matches!(
            report.outcomes[1],
            VerificationCampaignOutcome::Completed(_)
        ));
    }

    #[test]
    fn observed_orders_never_bridge_across_a_failed_gap() {
        let cases = [
            LadderCaseSpec {
                resolution: 12,
                steps: 1,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 64,
            },
            LadderCaseSpec {
                resolution: 12,
                steps: 128,
            },
        ];
        let report = run_passive_taylor_green_campaign(
            config(),
            0.5,
            0.5,
            RefinementAxis::Temporal,
            &cases,
        )
        .unwrap();

        assert_eq!(report.adjacent_observed_orders.len(), 1);
        assert_eq!(report.adjacent_observed_orders[0].coarse_outcome_index, 1);
        assert_eq!(report.adjacent_observed_orders[0].fine_outcome_index, 2);
    }

    #[test]
    fn invalid_manufactured_profile_is_evidence_not_campaign_erasure() {
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
        let profile = ManufacturedTaylorGreenProfile {
            base_amplitude_mps: 0.1,
            modulation_fraction: 1.0,
            angular_frequency_rad_s: 1.0,
        };
        let report = run_manufactured_taylor_green_campaign(
            config(),
            profile,
            0.01,
            RefinementAxis::Temporal,
            &cases,
        )
        .unwrap();

        assert_eq!(report.completed_count, 0);
        assert_eq!(report.failed_count, 2);
        assert!(report.outcomes.iter().all(|outcome| matches!(
            outcome,
            VerificationCampaignOutcome::Failed(VerificationFailureEvidence {
                validity_state: ContinuumValidityState::ReferenceDomainViolation,
                failure_kind: VerificationFailureKind::InvalidInput,
                ..
            })
        )));
    }

    #[test]
    fn malformed_sequence_still_fails_closed_as_a_campaign_definition() {
        let cases = [
            LadderCaseSpec {
                resolution: 12,
                steps: 8,
            },
            LadderCaseSpec {
                resolution: 16,
                steps: 16,
            },
        ];
        assert_eq!(
            run_passive_taylor_green_campaign(
                config(),
                0.1,
                0.01,
                RefinementAxis::Temporal,
                &cases,
            ),
            Err(VerificationCampaignError::InvalidRefinementSequence)
        );
    }
}
