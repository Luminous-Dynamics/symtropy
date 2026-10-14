// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed contract for future concentration-aware continuum refinement policy.
//!
//! V0.1 is deliberately **non-executable policy**. It defines validated inputs,
//! semantic-profile candidates, information requests, and possible outcomes, but
//! contains no threshold evaluator and grants no runtime refinement authority.
//!
//! The contract encodes the architectural separation from #466/#512:
//!
//! `canonical physics/process requirements -> semantic profile requirement`
//!
//! never:
//!
//! `camera/FPS/GPU load -> cheaper semantic physics`.
//!
//! Execution backend identity is absent by design. CPU/GPU/parallel selection is
//! downstream of an already-selected semantic profile and belongs to a separate
//! backend-equivalence contract.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::validation::ContinuumValidityState;

pub const REFINEMENT_POLICY_CONTRACT_SCHEMA_ID: &str = "continuum-refinement-policy-contract-v0.1";
pub const MAX_POLICY_ID_BYTES: usize = 128;
pub const MAX_PROFILE_ID_BYTES: usize = 256;
pub const MAX_REASON_BYTES: usize = 512;
pub const MAX_REQUIRED_OBSERVABLES: usize = 32;
pub const MAX_PROFILE_CANDIDATES: usize = 32;
pub const MAX_REQUESTED_INFORMATION: usize = 32;

/// Solver-independent observable that a process requirement may demand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredContinuumObservable {
    MaxResolvedSpeed,
    KineticEnergy,
    DivergenceRms,
    MaxVorticity,
    MaxStrainRate,
    MaxPressureGradient,
    AdvectiveCfl,
    MinimumResolvedLength,
    ConcentrationScale,
    SolverResidual,
    ForcingResidual,
    NonFiniteStateCount,
}

/// Why qualification evidence may not currently authorize a semantic decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvidenceFreshness {
    Current {
        evidence_revision: String,
    },
    Stale {
        evidence_revision: String,
        stale_reason: String,
    },
}

/// Authority available to a consumer of this contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementAuthorityState {
    /// Measurements/contracts may be emitted, but no semantic transition may be
    /// executed automatically.
    MeasurementOnly,
    /// A separately qualified policy version may evaluate the contract.
    QualifiedPolicyEvaluationAllowed,
}

/// Qualification state of an available semantic solver profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticProfileQualification {
    QualifiedForDeclaredUse,
    MeasurementOnly,
    QualificationStale,
}

/// Relationship of a candidate semantic profile to the current one.
///
/// These are semantic/model choices, not hardware backends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticProfileCapability {
    FinerSpatialContinuum,
    FinerTemporalContinuum,
    AlternateContinuumDiscretization,
    StrongerProjectionOrImplicitSolve,
    AdaptiveMeshRefinement,
    SubgridOrTurbulenceClosure,
    FreeSurfaceGridParticleHybrid,
    KineticOrMolecularRegime,
    ConservationReconciliation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticProfileCandidate {
    pub semantic_profile_id: String,
    pub qualification: SemanticProfileQualification,
    pub capabilities: Vec<SemanticProfileCapability>,
}

/// Information/semantic capability a later evaluator may request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedRefinementInformation {
    FinerSpatialResolution,
    SmallerTimeStep,
    ImprovedProjectionConvergence,
    AlternateContinuumDiscretization,
    AdaptiveMeshRefinement,
    SubgridOrTurbulenceClosure,
    FreeSurfaceRepresentation,
    BoundaryOrForcingRevalidation,
    BenchmarkProfileQualification,
    ConservationReconciliation,
    FreshQualificationEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementReasonKind {
    SpatialUnderResolution,
    TemporalOrCflFailure,
    DivergenceFailure,
    SolverNonConvergence,
    NonFiniteState,
    ReferenceDomainViolation,
    BenchmarkProfileUnavailable,
    MissingRequiredObservable,
    StaleQualificationEvidence,
    ProcessRequirementNotMet,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefinementReason {
    pub kind: RefinementReasonKind,
    pub detail: String,
}

/// Canonical input shape for a future qualified refinement evaluator.
///
/// There are intentionally no renderer, camera, frame-rate, host-load, or
/// backend-selection fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumRefinementPolicyInput {
    pub schema_id: String,
    pub policy_contract_id: String,
    pub current_semantic_profile_id: String,
    pub diagnostic_profile_id: String,
    pub process_requirement_profile_id: String,
    pub validity: ContinuumValidityState,
    pub evidence_freshness: EvidenceFreshness,
    pub authority: RefinementAuthorityState,
    pub required_observables: Vec<RequiredContinuumObservable>,
    pub missing_required_observables: Vec<RequiredContinuumObservable>,
    pub available_semantic_profiles: Vec<SemanticProfileCandidate>,
}

/// Output vocabulary requested by #512. V0.1 defines the shape only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ContinuumRefinementPolicyOutcome {
    CurrentProfileSufficient,
    RefinementRequired {
        reason: RefinementReason,
        requested_information: Vec<RequestedRefinementInformation>,
    },
    AlternativeSemanticProfileRequired {
        reason: RefinementReason,
        candidate_semantic_profile_ids: Vec<String>,
    },
    UnresolvedWithinAvailableProfiles {
        reason: RefinementReason,
    },
    RevalidationRequired {
        stale_evidence_revision: String,
        reason: RefinementReason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefinementPolicyContractError {
    WrongSchemaId,
    EmptyField(&'static str),
    FieldTooLong {
        field: &'static str,
        max_bytes: usize,
    },
    TooManyRequiredObservables,
    DuplicateRequiredObservable,
    TooManyMissingObservables,
    DuplicateMissingObservable,
    MissingObservableWasNotRequired,
    TooManyProfileCandidates,
    DuplicateProfileCandidate,
    CurrentProfileRepeatedAsCandidate,
    EmptyCandidateCapabilities,
    DuplicateCandidateCapability,
    EmptyEvidenceRevision,
    EmptyStaleReason,
    TooManyRequestedInformation,
    DuplicateRequestedInformation,
    EmptyAlternativeCandidates,
    DuplicateAlternativeCandidate,
    EmptyReasonDetail,
    OutcomeReasonTooLong,
}

impl fmt::Display for RefinementPolicyContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSchemaId => write!(f, "unsupported refinement policy contract schema"),
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::FieldTooLong { field, max_bytes } => {
                write!(f, "{field} must not exceed {max_bytes} bytes")
            }
            Self::TooManyRequiredObservables => write!(f, "too many required observables"),
            Self::DuplicateRequiredObservable => write!(f, "required observables must be unique"),
            Self::TooManyMissingObservables => write!(f, "too many missing required observables"),
            Self::DuplicateMissingObservable => write!(f, "missing observables must be unique"),
            Self::MissingObservableWasNotRequired => {
                write!(f, "every missing observable must also be required")
            }
            Self::TooManyProfileCandidates => write!(f, "too many semantic profile candidates"),
            Self::DuplicateProfileCandidate => {
                write!(f, "semantic profile candidates must be unique")
            }
            Self::CurrentProfileRepeatedAsCandidate => {
                write!(
                    f,
                    "current semantic profile must not be repeated as a candidate"
                )
            }
            Self::EmptyCandidateCapabilities => {
                write!(
                    f,
                    "semantic profile candidate must declare at least one capability"
                )
            }
            Self::DuplicateCandidateCapability => {
                write!(f, "semantic profile candidate capabilities must be unique")
            }
            Self::EmptyEvidenceRevision => write!(f, "evidence revision must not be empty"),
            Self::EmptyStaleReason => write!(f, "stale evidence requires a non-empty reason"),
            Self::TooManyRequestedInformation => {
                write!(f, "too many requested-information entries")
            }
            Self::DuplicateRequestedInformation => {
                write!(f, "requested-information entries must be unique")
            }
            Self::EmptyAlternativeCandidates => {
                write!(f, "alternative-profile outcome requires candidates")
            }
            Self::DuplicateAlternativeCandidate => {
                write!(f, "alternative-profile candidates must be unique")
            }
            Self::EmptyReasonDetail => write!(f, "outcome reason detail must not be empty"),
            Self::OutcomeReasonTooLong => write!(f, "outcome reason detail is too long"),
        }
    }
}

impl std::error::Error for RefinementPolicyContractError {}

impl ContinuumRefinementPolicyInput {
    pub fn validate(&self) -> Result<(), RefinementPolicyContractError> {
        if self.schema_id != REFINEMENT_POLICY_CONTRACT_SCHEMA_ID {
            return Err(RefinementPolicyContractError::WrongSchemaId);
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

        validate_observable_set(&self.required_observables, MAX_REQUIRED_OBSERVABLES, true)?;
        validate_observable_set(
            &self.missing_required_observables,
            MAX_REQUIRED_OBSERVABLES,
            false,
        )?;
        let required = self
            .required_observables
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if self
            .missing_required_observables
            .iter()
            .any(|observable| !required.contains(observable))
        {
            return Err(RefinementPolicyContractError::MissingObservableWasNotRequired);
        }

        validate_freshness(&self.evidence_freshness)?;

        if self.available_semantic_profiles.len() > MAX_PROFILE_CANDIDATES {
            return Err(RefinementPolicyContractError::TooManyProfileCandidates);
        }
        let mut ids = BTreeSet::new();
        for candidate in &self.available_semantic_profiles {
            validate_id(
                "semantic_profile_id",
                &candidate.semantic_profile_id,
                MAX_PROFILE_ID_BYTES,
            )?;
            if candidate.semantic_profile_id == self.current_semantic_profile_id {
                return Err(RefinementPolicyContractError::CurrentProfileRepeatedAsCandidate);
            }
            if !ids.insert(candidate.semantic_profile_id.as_str()) {
                return Err(RefinementPolicyContractError::DuplicateProfileCandidate);
            }
            if candidate.capabilities.is_empty() {
                return Err(RefinementPolicyContractError::EmptyCandidateCapabilities);
            }
            if candidate
                .capabilities
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != candidate.capabilities.len()
            {
                return Err(RefinementPolicyContractError::DuplicateCandidateCapability);
            }
        }
        Ok(())
    }
}

impl ContinuumRefinementPolicyOutcome {
    pub fn validate(&self) -> Result<(), RefinementPolicyContractError> {
        match self {
            Self::CurrentProfileSufficient => Ok(()),
            Self::RefinementRequired {
                reason,
                requested_information,
            } => {
                validate_reason(reason)?;
                if requested_information.len() > MAX_REQUESTED_INFORMATION {
                    return Err(RefinementPolicyContractError::TooManyRequestedInformation);
                }
                if requested_information
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != requested_information.len()
                {
                    return Err(RefinementPolicyContractError::DuplicateRequestedInformation);
                }
                Ok(())
            }
            Self::AlternativeSemanticProfileRequired {
                reason,
                candidate_semantic_profile_ids,
            } => {
                validate_reason(reason)?;
                if candidate_semantic_profile_ids.is_empty() {
                    return Err(RefinementPolicyContractError::EmptyAlternativeCandidates);
                }
                let mut ids = BTreeSet::new();
                for id in candidate_semantic_profile_ids {
                    validate_id("candidate_semantic_profile_id", id, MAX_PROFILE_ID_BYTES)?;
                    if !ids.insert(id.as_str()) {
                        return Err(RefinementPolicyContractError::DuplicateAlternativeCandidate);
                    }
                }
                Ok(())
            }
            Self::UnresolvedWithinAvailableProfiles { reason } => validate_reason(reason),
            Self::RevalidationRequired {
                stale_evidence_revision,
                reason,
            } => {
                validate_id(
                    "stale_evidence_revision",
                    stale_evidence_revision,
                    MAX_PROFILE_ID_BYTES,
                )?;
                validate_reason(reason)
            }
        }
    }
}

fn validate_observable_set(
    values: &[RequiredContinuumObservable],
    max: usize,
    required: bool,
) -> Result<(), RefinementPolicyContractError> {
    if values.len() > max {
        return Err(if required {
            RefinementPolicyContractError::TooManyRequiredObservables
        } else {
            RefinementPolicyContractError::TooManyMissingObservables
        });
    }
    if values.iter().copied().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(if required {
            RefinementPolicyContractError::DuplicateRequiredObservable
        } else {
            RefinementPolicyContractError::DuplicateMissingObservable
        });
    }
    Ok(())
}

fn validate_freshness(value: &EvidenceFreshness) -> Result<(), RefinementPolicyContractError> {
    match value {
        EvidenceFreshness::Current { evidence_revision } => {
            validate_id("evidence_revision", evidence_revision, MAX_PROFILE_ID_BYTES)
        }
        EvidenceFreshness::Stale {
            evidence_revision,
            stale_reason,
        } => {
            validate_id("evidence_revision", evidence_revision, MAX_PROFILE_ID_BYTES)?;
            if stale_reason.trim().is_empty() {
                return Err(RefinementPolicyContractError::EmptyStaleReason);
            }
            if stale_reason.len() > MAX_REASON_BYTES {
                return Err(RefinementPolicyContractError::FieldTooLong {
                    field: "stale_reason",
                    max_bytes: MAX_REASON_BYTES,
                });
            }
            Ok(())
        }
    }
}

fn validate_reason(reason: &RefinementReason) -> Result<(), RefinementPolicyContractError> {
    if reason.detail.trim().is_empty() {
        return Err(RefinementPolicyContractError::EmptyReasonDetail);
    }
    if reason.detail.len() > MAX_REASON_BYTES {
        return Err(RefinementPolicyContractError::OutcomeReasonTooLong);
    }
    Ok(())
}

fn validate_id(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), RefinementPolicyContractError> {
    if value.trim().is_empty() {
        return Err(RefinementPolicyContractError::EmptyField(field));
    }
    if value.len() > max_bytes {
        return Err(RefinementPolicyContractError::FieldTooLong { field, max_bytes });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> ContinuumRefinementPolicyInput {
        ContinuumRefinementPolicyInput {
            schema_id: REFINEMENT_POLICY_CONTRACT_SCHEMA_ID.to_owned(),
            policy_contract_id: "continuum-refinement-policy-contract-v0.1".to_owned(),
            current_semantic_profile_id: "periodic-mac2d-reference-v0.1".to_owned(),
            diagnostic_profile_id: "reference+vorticity-scale-v0.1".to_owned(),
            process_requirement_profile_id: "smooth-control-observables-v0.1".to_owned(),
            validity: ContinuumValidityState::UnderResolved,
            evidence_freshness: EvidenceFreshness::Current {
                evidence_revision: "exact-head:abc123".to_owned(),
            },
            authority: RefinementAuthorityState::MeasurementOnly,
            required_observables: vec![
                RequiredContinuumObservable::AdvectiveCfl,
                RequiredContinuumObservable::DivergenceRms,
                RequiredContinuumObservable::ConcentrationScale,
            ],
            missing_required_observables: vec![],
            available_semantic_profiles: vec![SemanticProfileCandidate {
                semantic_profile_id: "periodic-mac2d-reference-v0.2-finer".to_owned(),
                qualification: SemanticProfileQualification::MeasurementOnly,
                capabilities: vec![SemanticProfileCapability::FinerSpatialContinuum],
            }],
        }
    }

    #[test]
    fn valid_contract_has_no_execution_backend_or_renderer_inputs() {
        let input = sample_input();
        input.validate().unwrap();
        let json = serde_json::to_string(&input).unwrap().to_ascii_lowercase();
        for forbidden in [
            "camera",
            "fps",
            "frame_rate",
            "gpu_load",
            "cpu_load",
            "backend_id",
        ] {
            assert!(
                !json.contains(forbidden),
                "forbidden semantic selector leaked: {forbidden}"
            );
        }
    }

    #[test]
    fn missing_observable_must_have_been_required() {
        let mut input = sample_input();
        input.missing_required_observables = vec![RequiredContinuumObservable::ForcingResidual];
        assert_eq!(
            input.validate(),
            Err(RefinementPolicyContractError::MissingObservableWasNotRequired)
        );
    }

    #[test]
    fn duplicate_candidate_and_capability_are_rejected() {
        let mut input = sample_input();
        input.available_semantic_profiles[0].capabilities = vec![
            SemanticProfileCapability::FinerSpatialContinuum,
            SemanticProfileCapability::FinerSpatialContinuum,
        ];
        assert_eq!(
            input.validate(),
            Err(RefinementPolicyContractError::DuplicateCandidateCapability)
        );

        let mut input = sample_input();
        input
            .available_semantic_profiles
            .push(input.available_semantic_profiles[0].clone());
        assert_eq!(
            input.validate(),
            Err(RefinementPolicyContractError::DuplicateProfileCandidate)
        );
    }

    #[test]
    fn current_profile_cannot_masquerade_as_an_alternative() {
        let mut input = sample_input();
        input.available_semantic_profiles[0].semantic_profile_id =
            input.current_semantic_profile_id.clone();
        assert_eq!(
            input.validate(),
            Err(RefinementPolicyContractError::CurrentProfileRepeatedAsCandidate)
        );
    }

    #[test]
    fn stale_evidence_requires_explanation() {
        let mut input = sample_input();
        input.evidence_freshness = EvidenceFreshness::Stale {
            evidence_revision: "exact-head:abc123".to_owned(),
            stale_reason: "".to_owned(),
        };
        assert_eq!(
            input.validate(),
            Err(RefinementPolicyContractError::EmptyStaleReason)
        );
    }

    #[test]
    fn outcome_contract_preserves_reason_and_requested_information() {
        let outcome = ContinuumRefinementPolicyOutcome::RefinementRequired {
            reason: RefinementReason {
                kind: RefinementReasonKind::SpatialUnderResolution,
                detail:
                    "concentration-scale separation is insufficient for the requested observable"
                        .to_owned(),
            },
            requested_information: vec![
                RequestedRefinementInformation::FinerSpatialResolution,
                RequestedRefinementInformation::FreshQualificationEvidence,
            ],
        };
        outcome.validate().unwrap();
    }

    #[test]
    fn there_is_no_generic_particles_shortcut_in_requested_information() {
        let names = [
            RequestedRefinementInformation::FinerSpatialResolution,
            RequestedRefinementInformation::SmallerTimeStep,
            RequestedRefinementInformation::ImprovedProjectionConvergence,
            RequestedRefinementInformation::AlternateContinuumDiscretization,
            RequestedRefinementInformation::AdaptiveMeshRefinement,
            RequestedRefinementInformation::SubgridOrTurbulenceClosure,
            RequestedRefinementInformation::FreeSurfaceRepresentation,
            RequestedRefinementInformation::BoundaryOrForcingRevalidation,
            RequestedRefinementInformation::BenchmarkProfileQualification,
            RequestedRefinementInformation::ConservationReconciliation,
            RequestedRefinementInformation::FreshQualificationEvidence,
        ]
        .into_iter()
        .map(|value| serde_json::to_string(&value).unwrap())
        .collect::<Vec<_>>()
        .join(";")
        .to_ascii_lowercase();
        assert!(!names.contains("particles"));
    }
}
