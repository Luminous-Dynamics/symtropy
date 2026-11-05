// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Mechanical readiness checks for the non-authoritative refinement-policy
//! contract.
//!
//! This module does **not** decide whether a continuum state is resolved, choose
//! a semantic solver profile, or select an execution backend. It only binds a
//! policy input to the exact diagnostic sample it names, derives availability of
//! required observables, checks the caller's declared-missing set, and preserves
//! evidence freshness / authority as independent gates.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::refinement_policy_contract::{
    ContinuumRefinementPolicyInput, EvidenceFreshness, MAX_POLICY_ID_BYTES, MAX_PROFILE_ID_BYTES,
    MAX_REASON_BYTES, MAX_REQUIRED_OBSERVABLES, RefinementAuthorityState,
    RefinementPolicyContractError, RequiredContinuumObservable,
};
use crate::validation::{
    ContinuumDiagnosticSample, ContinuumSampleError, ContinuumValidityState,
    DiagnosticUnavailableReason, NonNegativeDiagnostic,
};

pub const REFINEMENT_POLICY_PREFLIGHT_SCHEMA_ID: &str =
    "continuum-refinement-policy-preflight-v0.1";

/// Exact evidence for one required observable that is unavailable in the bound
/// diagnostic sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingRequiredObservableEvidence {
    pub observable: RequiredContinuumObservable,
    pub unavailable_reason: DiagnosticUnavailableReason,
}

/// Observable-completeness gate. This is deliberately independent from evidence
/// freshness and policy authority so callers cannot hide multiple blockers in a
/// single score.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RequiredObservableGate {
    Complete,
    Missing {
        missing: Vec<MissingRequiredObservableEvidence>,
    },
}

/// Independent structural gates required before a *separately qualified* policy
/// evaluator may run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefinementPolicyPreflightGates {
    pub evidence_freshness: EvidenceFreshness,
    pub required_observables: RequiredObservableGate,
    pub authority: RefinementAuthorityState,
}

/// Self-contained record proving which exact diagnostic sample was structurally
/// ready (or not ready) for later policy evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumRefinementPolicyPreflightReport {
    pub schema_id: String,
    pub policy_contract_id: String,
    pub current_semantic_profile_id: String,
    pub diagnostic_profile_id: String,
    pub process_requirement_profile_id: String,
    /// Exact IEEE-754 bits from the bound diagnostic sample. This avoids
    /// reformatting/rounding ambiguity in a retained receipt.
    pub sample_time_bits: u64,
    /// Retained for provenance only. Preflight never interprets this validity
    /// state or changes readiness because it says `Resolved`/`UnderResolved`.
    pub declared_validity: ContinuumValidityState,
    /// Canonical sorted set used for the availability check.
    pub required_observables: Vec<RequiredContinuumObservable>,
    pub gates: RefinementPolicyPreflightGates,
}

impl ContinuumRefinementPolicyPreflightReport {
    /// True only when the three mechanical prerequisites for a separately
    /// qualified evaluator are simultaneously satisfied. This says nothing about
    /// whether the physical state itself is resolved.
    pub fn ready_for_qualified_policy_evaluation(&self) -> bool {
        matches!(
            self.gates.evidence_freshness,
            EvidenceFreshness::Current { .. }
        ) && matches!(
            self.gates.required_observables,
            RequiredObservableGate::Complete
        ) && self.gates.authority == RefinementAuthorityState::QualifiedPolicyEvaluationAllowed
    }

    pub fn validate(&self) -> Result<(), RefinementPolicyPreflightError> {
        if self.schema_id != REFINEMENT_POLICY_PREFLIGHT_SCHEMA_ID {
            return Err(RefinementPolicyPreflightError::WrongSchemaId);
        }
        validate_id(
            "policy_contract_id",
            &self.policy_contract_id,
            MAX_POLICY_ID_BYTES,
        )?;
        validate_id(
            "current_semantic_profile_id",
            &self.current_semantic_profile_id,
            MAX_PROFILE_ID_BYTES,
        )?;
        validate_id(
            "diagnostic_profile_id",
            &self.diagnostic_profile_id,
            MAX_PROFILE_ID_BYTES,
        )?;
        validate_id(
            "process_requirement_profile_id",
            &self.process_requirement_profile_id,
            MAX_PROFILE_ID_BYTES,
        )?;
        validate_freshness(&self.gates.evidence_freshness)?;

        let sample_time_s = f64::from_bits(self.sample_time_bits);
        if !sample_time_s.is_finite() || sample_time_s < 0.0 {
            return Err(RefinementPolicyPreflightError::InvalidSampleTime);
        }

        if self.required_observables.len() > MAX_REQUIRED_OBSERVABLES {
            return Err(RefinementPolicyPreflightError::TooManyRequiredObservables);
        }
        if !strictly_sorted_unique(&self.required_observables) {
            return Err(RefinementPolicyPreflightError::NonCanonicalRequiredObservables);
        }
        let required = self
            .required_observables
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        match &self.gates.required_observables {
            RequiredObservableGate::Complete => {}
            RequiredObservableGate::Missing { missing } => {
                if missing.is_empty() {
                    return Err(RefinementPolicyPreflightError::EmptyMissingGate);
                }
                if missing.len() > MAX_REQUIRED_OBSERVABLES {
                    return Err(RefinementPolicyPreflightError::TooManyRequiredObservables);
                }
                if !strictly_sorted_unique_by_observable(missing) {
                    return Err(RefinementPolicyPreflightError::NonCanonicalMissingObservables);
                }
                if missing
                    .iter()
                    .any(|entry| !required.contains(&entry.observable))
                {
                    return Err(RefinementPolicyPreflightError::MissingObservableWasNotRequired);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefinementPolicyPreflightError {
    Contract(RefinementPolicyContractError),
    Sample(ContinuumSampleError),
    WrongSchemaId,
    EmptyField(&'static str),
    FieldTooLong {
        field: &'static str,
        max_bytes: usize,
    },
    EmptyEvidenceRevision,
    EmptyStaleReason,
    InvalidSampleTime,
    TooManyRequiredObservables,
    DiagnosticProfileMismatch,
    DeclaredMissingObservableMismatch,
    NonCanonicalRequiredObservables,
    EmptyMissingGate,
    NonCanonicalMissingObservables,
    MissingObservableWasNotRequired,
}

impl fmt::Display for RefinementPolicyPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(source) => write!(f, "invalid refinement-policy input: {source}"),
            Self::Sample(source) => write!(f, "invalid continuum diagnostic sample: {source}"),
            Self::WrongSchemaId => write!(f, "unsupported refinement-policy preflight schema"),
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::FieldTooLong { field, max_bytes } => {
                write!(f, "{field} must not exceed {max_bytes} bytes")
            }
            Self::EmptyEvidenceRevision => write!(f, "evidence revision must not be empty"),
            Self::EmptyStaleReason => write!(f, "stale evidence reason must not be empty"),
            Self::InvalidSampleTime => {
                write!(
                    f,
                    "sample_time_bits must decode to a finite non-negative time"
                )
            }
            Self::TooManyRequiredObservables => write!(f, "too many required observables"),
            Self::DiagnosticProfileMismatch => write!(
                f,
                "policy diagnostic_profile_id does not exactly match the bound diagnostic sample"
            ),
            Self::DeclaredMissingObservableMismatch => write!(
                f,
                "declared missing-required-observable set does not match the bound diagnostic sample"
            ),
            Self::NonCanonicalRequiredObservables => {
                write!(f, "required-observable evidence must be sorted and unique")
            }
            Self::EmptyMissingGate => write!(f, "missing-observable gate must contain evidence"),
            Self::NonCanonicalMissingObservables => {
                write!(f, "missing-observable evidence must be sorted and unique")
            }
            Self::MissingObservableWasNotRequired => {
                write!(
                    f,
                    "missing-observable evidence must refer to a required observable"
                )
            }
        }
    }
}

impl std::error::Error for RefinementPolicyPreflightError {}

impl From<RefinementPolicyContractError> for RefinementPolicyPreflightError {
    fn from(value: RefinementPolicyContractError) -> Self {
        Self::Contract(value)
    }
}

impl From<ContinuumSampleError> for RefinementPolicyPreflightError {
    fn from(value: ContinuumSampleError) -> Self {
        Self::Sample(value)
    }
}

/// Bind one validated policy input to one validated diagnostic sample and derive
/// the mechanical readiness gates. No validity/refinement thresholds are used.
pub fn preflight_refinement_policy_input(
    input: &ContinuumRefinementPolicyInput,
    sample: &ContinuumDiagnosticSample,
) -> Result<ContinuumRefinementPolicyPreflightReport, RefinementPolicyPreflightError> {
    input.validate()?;
    sample.validate()?;

    if input.diagnostic_profile_id != sample.diagnostic_profile {
        return Err(RefinementPolicyPreflightError::DiagnosticProfileMismatch);
    }

    let mut required_observables = input.required_observables.clone();
    required_observables.sort_unstable();

    let missing = required_observables
        .iter()
        .filter_map(|observable| {
            unavailable_reason(sample, *observable).map(|unavailable_reason| {
                MissingRequiredObservableEvidence {
                    observable: *observable,
                    unavailable_reason,
                }
            })
        })
        .collect::<Vec<_>>();

    let derived_missing = missing
        .iter()
        .map(|entry| entry.observable)
        .collect::<BTreeSet<_>>();
    let declared_missing = input
        .missing_required_observables
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if derived_missing != declared_missing {
        return Err(RefinementPolicyPreflightError::DeclaredMissingObservableMismatch);
    }

    let observable_gate = if missing.is_empty() {
        RequiredObservableGate::Complete
    } else {
        RequiredObservableGate::Missing { missing }
    };

    let report = ContinuumRefinementPolicyPreflightReport {
        schema_id: REFINEMENT_POLICY_PREFLIGHT_SCHEMA_ID.to_owned(),
        policy_contract_id: input.policy_contract_id.clone(),
        current_semantic_profile_id: input.current_semantic_profile_id.clone(),
        diagnostic_profile_id: input.diagnostic_profile_id.clone(),
        process_requirement_profile_id: input.process_requirement_profile_id.clone(),
        sample_time_bits: sample.time_s.to_bits(),
        declared_validity: input.validity,
        required_observables,
        gates: RefinementPolicyPreflightGates {
            evidence_freshness: input.evidence_freshness.clone(),
            required_observables: observable_gate,
            authority: input.authority,
        },
    };
    report.validate()?;
    Ok(report)
}

fn unavailable_reason(
    sample: &ContinuumDiagnosticSample,
    observable: RequiredContinuumObservable,
) -> Option<DiagnosticUnavailableReason> {
    match observable {
        RequiredContinuumObservable::MaxResolvedSpeed => {
            diagnostic_reason(&sample.max_resolved_speed_mps)
        }
        RequiredContinuumObservable::KineticEnergy => diagnostic_reason(&sample.kinetic_energy_j),
        RequiredContinuumObservable::DivergenceRms => {
            diagnostic_reason(&sample.divergence_rms_per_s)
        }
        RequiredContinuumObservable::MaxVorticity => diagnostic_reason(&sample.max_vorticity_per_s),
        RequiredContinuumObservable::MaxStrainRate => {
            diagnostic_reason(&sample.max_strain_rate_per_s)
        }
        RequiredContinuumObservable::MaxPressureGradient => {
            diagnostic_reason(&sample.max_pressure_gradient_pa_per_m)
        }
        RequiredContinuumObservable::AdvectiveCfl => diagnostic_reason(&sample.max_cfl),
        RequiredContinuumObservable::MinimumResolvedLength => {
            diagnostic_reason(&sample.minimum_resolved_length_m)
        }
        RequiredContinuumObservable::ConcentrationScale => {
            diagnostic_reason(&sample.concentration_scale_m)
        }
        RequiredContinuumObservable::SolverResidual => diagnostic_reason(&sample.solver_residual),
        RequiredContinuumObservable::ForcingResidual => diagnostic_reason(&sample.forcing_residual),
        RequiredContinuumObservable::NonFiniteStateCount => None,
    }
}

fn diagnostic_reason(value: &NonNegativeDiagnostic) -> Option<DiagnosticUnavailableReason> {
    match value {
        NonNegativeDiagnostic::Measured(_) => None,
        NonNegativeDiagnostic::Unavailable(reason) => Some(*reason),
    }
}

fn strictly_sorted_unique(values: &[RequiredContinuumObservable]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

fn strictly_sorted_unique_by_observable(values: &[MissingRequiredObservableEvidence]) -> bool {
    values
        .windows(2)
        .all(|window| window[0].observable < window[1].observable)
}

fn validate_freshness(value: &EvidenceFreshness) -> Result<(), RefinementPolicyPreflightError> {
    match value {
        EvidenceFreshness::Current { evidence_revision } => {
            if evidence_revision.trim().is_empty() {
                return Err(RefinementPolicyPreflightError::EmptyEvidenceRevision);
            }
            if evidence_revision.len() > MAX_PROFILE_ID_BYTES {
                return Err(RefinementPolicyPreflightError::FieldTooLong {
                    field: "evidence_revision",
                    max_bytes: MAX_PROFILE_ID_BYTES,
                });
            }
        }
        EvidenceFreshness::Stale {
            evidence_revision,
            stale_reason,
        } => {
            if evidence_revision.trim().is_empty() {
                return Err(RefinementPolicyPreflightError::EmptyEvidenceRevision);
            }
            if evidence_revision.len() > MAX_PROFILE_ID_BYTES {
                return Err(RefinementPolicyPreflightError::FieldTooLong {
                    field: "evidence_revision",
                    max_bytes: MAX_PROFILE_ID_BYTES,
                });
            }
            if stale_reason.trim().is_empty() {
                return Err(RefinementPolicyPreflightError::EmptyStaleReason);
            }
            if stale_reason.len() > MAX_REASON_BYTES {
                return Err(RefinementPolicyPreflightError::FieldTooLong {
                    field: "stale_reason",
                    max_bytes: MAX_REASON_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn validate_id(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), RefinementPolicyPreflightError> {
    if value.trim().is_empty() {
        return Err(RefinementPolicyPreflightError::EmptyField(field));
    }
    if value.len() > max_bytes {
        return Err(RefinementPolicyPreflightError::FieldTooLong { field, max_bytes });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{PeriodicMac2d, PeriodicMacConfig};

    fn sample_and_input(
        authority: RefinementAuthorityState,
    ) -> (ContinuumDiagnosticSample, ContinuumRefinementPolicyInput) {
        let state = PeriodicMac2d::taylor_green(PeriodicMacConfig::default(), 1.0).unwrap();
        let sample = state.diagnostics().unwrap();
        let input = ContinuumRefinementPolicyInput {
            schema_id: crate::refinement_policy_contract::REFINEMENT_POLICY_CONTRACT_SCHEMA_ID
                .to_owned(),
            policy_contract_id: "continuum-refinement-policy-contract-v0.1".to_owned(),
            current_semantic_profile_id: "periodic-mac2d-reference-v0.1".to_owned(),
            diagnostic_profile_id: sample.diagnostic_profile.clone(),
            process_requirement_profile_id: "preflight-fixture-v0.1".to_owned(),
            validity: ContinuumValidityState::UnderResolved,
            evidence_freshness: EvidenceFreshness::Current {
                evidence_revision: "exact-head:test".to_owned(),
            },
            authority,
            required_observables: vec![
                RequiredContinuumObservable::MaxResolvedSpeed,
                RequiredContinuumObservable::KineticEnergy,
                RequiredContinuumObservable::NonFiniteStateCount,
            ],
            missing_required_observables: vec![],
            available_semantic_profiles: vec![],
        };
        (sample, input)
    }

    #[test]
    fn readiness_is_mechanical_and_does_not_reinterpret_under_resolved() {
        let (sample, input) =
            sample_and_input(RefinementAuthorityState::QualifiedPolicyEvaluationAllowed);
        let report = preflight_refinement_policy_input(&input, &sample).unwrap();
        assert_eq!(
            report.declared_validity,
            ContinuumValidityState::UnderResolved
        );
        assert!(report.ready_for_qualified_policy_evaluation());
        report.validate().unwrap();
    }

    #[test]
    fn measurement_only_authority_is_an_independent_blocker() {
        let (sample, input) = sample_and_input(RefinementAuthorityState::MeasurementOnly);
        let report = preflight_refinement_policy_input(&input, &sample).unwrap();
        assert!(matches!(
            report.gates.required_observables,
            RequiredObservableGate::Complete
        ));
        assert!(matches!(
            report.gates.evidence_freshness,
            EvidenceFreshness::Current { .. }
        ));
        assert!(!report.ready_for_qualified_policy_evaluation());
    }

    #[test]
    fn stale_and_missing_evidence_remain_independently_visible() {
        let (sample, mut input) = sample_and_input(RefinementAuthorityState::MeasurementOnly);
        input.evidence_freshness = EvidenceFreshness::Stale {
            evidence_revision: "exact-head:test".to_owned(),
            stale_reason: "solver profile changed".to_owned(),
        };
        input.required_observables = vec![RequiredContinuumObservable::ForcingResidual];
        input.missing_required_observables = vec![RequiredContinuumObservable::ForcingResidual];

        let report = preflight_refinement_policy_input(&input, &sample).unwrap();
        assert!(matches!(
            report.gates.evidence_freshness,
            EvidenceFreshness::Stale { .. }
        ));
        assert!(matches!(
            report.gates.required_observables,
            RequiredObservableGate::Missing { .. }
        ));
        assert_eq!(
            report.gates.authority,
            RefinementAuthorityState::MeasurementOnly
        );
        assert!(!report.ready_for_qualified_policy_evaluation());
    }

    #[test]
    fn declared_missing_set_must_match_the_bound_sample() {
        let (sample, mut input) =
            sample_and_input(RefinementAuthorityState::QualifiedPolicyEvaluationAllowed);
        input.required_observables = vec![RequiredContinuumObservable::ForcingResidual];
        input.missing_required_observables = vec![];
        assert_eq!(
            preflight_refinement_policy_input(&input, &sample).unwrap_err(),
            RefinementPolicyPreflightError::DeclaredMissingObservableMismatch
        );

        let (sample, mut input) =
            sample_and_input(RefinementAuthorityState::QualifiedPolicyEvaluationAllowed);
        input.required_observables = vec![RequiredContinuumObservable::MaxResolvedSpeed];
        input.missing_required_observables = vec![RequiredContinuumObservable::MaxResolvedSpeed];
        assert_eq!(
            preflight_refinement_policy_input(&input, &sample).unwrap_err(),
            RefinementPolicyPreflightError::DeclaredMissingObservableMismatch
        );
    }

    #[test]
    fn diagnostic_profile_binding_fails_closed() {
        let (sample, mut input) =
            sample_and_input(RefinementAuthorityState::QualifiedPolicyEvaluationAllowed);
        input.diagnostic_profile_id.push_str(";different");
        assert_eq!(
            preflight_refinement_policy_input(&input, &sample).unwrap_err(),
            RefinementPolicyPreflightError::DiagnosticProfileMismatch
        );
    }

    #[test]
    fn retained_report_revalidates_time_and_freshness_bounds() {
        let (sample, input) =
            sample_and_input(RefinementAuthorityState::QualifiedPolicyEvaluationAllowed);
        let report = preflight_refinement_policy_input(&input, &sample).unwrap();

        let mut invalid_time = report.clone();
        invalid_time.sample_time_bits = f64::NAN.to_bits();
        assert_eq!(
            invalid_time.validate().unwrap_err(),
            RefinementPolicyPreflightError::InvalidSampleTime
        );

        let mut oversized_revision = report;
        oversized_revision.gates.evidence_freshness = EvidenceFreshness::Current {
            evidence_revision: "x".repeat(MAX_PROFILE_ID_BYTES + 1),
        };
        assert_eq!(
            oversized_revision.validate().unwrap_err(),
            RefinementPolicyPreflightError::FieldTooLong {
                field: "evidence_revision",
                max_bytes: MAX_PROFILE_ID_BYTES,
            }
        );
    }
}
