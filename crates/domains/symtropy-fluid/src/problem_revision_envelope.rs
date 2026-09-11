// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical problem-revision binding for future continuum refinement policy.
//!
//! A valid diagnostic and a valid policy contract are not sufficient if they
//! describe different source, forcing, boundary, snapshot, or process-requirement
//! revisions. This module provides a structural envelope for those identities.
//!
//! It contains no physical threshold, no validity classifier, no semantic-profile
//! selector, and no execution-backend policy.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::refinement_policy_contract::{
    ContinuumRefinementPolicyInput, MAX_POLICY_ID_BYTES, MAX_PROFILE_ID_BYTES,
    RefinementPolicyContractError,
};

pub const CONTINUUM_PROBLEM_REVISION_ENVELOPE_SCHEMA_ID: &str =
    "continuum-problem-revision-envelope-v0.1";
pub const MAX_REVISION_ID_BYTES: usize = 256;
pub const MAX_CONTRACT_ID_BYTES: usize = 256;

/// Explicit revision binding for a problem component that may be absent by the
/// declared physical model.
///
/// `NotApplicable` is not equivalent to unknown/unbound. Its `contract_id`
/// identifies the model contract that makes the component inapplicable (for
/// example, an explicitly unforced passive control).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "binding", rename_all = "snake_case")]
pub enum OptionalProblemRevision {
    Bound { revision_id: String },
    NotApplicable { contract_id: String },
}

/// Exact canonical problem subject to which one future refinement evaluation is
/// allowed to refer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumProblemRevisionEnvelope {
    pub schema_id: String,
    pub policy_contract_id: String,
    pub current_semantic_profile_id: String,
    pub diagnostic_profile_id: String,
    pub process_requirement_profile_id: String,
    pub process_requirement_revision_id: String,
    /// Canonical state revision sampled by the diagnostics.
    pub state_revision_id: String,
    /// Snapshot token shared by all canonical inputs used for this evaluation.
    pub logical_snapshot_id: String,
    /// Revision of initial/source state semantics, including fixture or world
    /// source identity where applicable.
    pub source_revision_id: String,
    /// Exact forcing revision, or an explicit model contract proving that
    /// forcing is not applicable.
    pub forcing_revision: OptionalProblemRevision,
    /// Exact boundary-condition revision, or an explicit model contract proving
    /// that a separate boundary revision is not applicable.
    pub boundary_revision: OptionalProblemRevision,
}

impl ContinuumProblemRevisionEnvelope {
    pub fn validate(&self) -> Result<(), ContinuumProblemRevisionEnvelopeError> {
        if self.schema_id != CONTINUUM_PROBLEM_REVISION_ENVELOPE_SCHEMA_ID {
            return Err(ContinuumProblemRevisionEnvelopeError::WrongSchemaId);
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
        validate_id(
            "process_requirement_revision_id",
            &self.process_requirement_revision_id,
            MAX_REVISION_ID_BYTES,
        )?;
        validate_id(
            "state_revision_id",
            &self.state_revision_id,
            MAX_REVISION_ID_BYTES,
        )?;
        validate_id(
            "logical_snapshot_id",
            &self.logical_snapshot_id,
            MAX_REVISION_ID_BYTES,
        )?;
        validate_id(
            "source_revision_id",
            &self.source_revision_id,
            MAX_REVISION_ID_BYTES,
        )?;
        validate_optional_revision("forcing_revision", &self.forcing_revision)?;
        validate_optional_revision("boundary_revision", &self.boundary_revision)?;
        Ok(())
    }

    /// Bind this revision envelope to the exact policy-contract input identities.
    ///
    /// This only checks structural subject identity. It does not decide whether
    /// evidence is fresh, measurements are complete, or the declared validity
    /// state is physically correct.
    pub fn bind_policy_input(
        &self,
        input: &ContinuumRefinementPolicyInput,
    ) -> Result<(), ContinuumProblemRevisionEnvelopeError> {
        self.validate()?;
        input.validate()?;

        for (field, envelope, policy) in [
            (
                "policy_contract_id",
                self.policy_contract_id.as_str(),
                input.policy_contract_id.as_str(),
            ),
            (
                "current_semantic_profile_id",
                self.current_semantic_profile_id.as_str(),
                input.current_semantic_profile_id.as_str(),
            ),
            (
                "diagnostic_profile_id",
                self.diagnostic_profile_id.as_str(),
                input.diagnostic_profile_id.as_str(),
            ),
            (
                "process_requirement_profile_id",
                self.process_requirement_profile_id.as_str(),
                input.process_requirement_profile_id.as_str(),
            ),
        ] {
            if envelope != policy {
                return Err(
                    ContinuumProblemRevisionEnvelopeError::PolicyIdentityMismatch { field },
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContinuumProblemRevisionEnvelopeError {
    PolicyContract(RefinementPolicyContractError),
    WrongSchemaId,
    EmptyField(&'static str),
    FieldTooLong {
        field: &'static str,
        max_bytes: usize,
    },
    PolicyIdentityMismatch {
        field: &'static str,
    },
}

impl fmt::Display for ContinuumProblemRevisionEnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PolicyContract(source) => write!(f, "invalid refinement-policy input: {source}"),
            Self::WrongSchemaId => write!(f, "unsupported continuum problem revision schema"),
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::FieldTooLong { field, max_bytes } => {
                write!(f, "{field} must not exceed {max_bytes} bytes")
            }
            Self::PolicyIdentityMismatch { field } => {
                write!(
                    f,
                    "problem revision envelope does not match policy input field {field}"
                )
            }
        }
    }
}

impl std::error::Error for ContinuumProblemRevisionEnvelopeError {}

impl From<RefinementPolicyContractError> for ContinuumProblemRevisionEnvelopeError {
    fn from(value: RefinementPolicyContractError) -> Self {
        Self::PolicyContract(value)
    }
}

fn validate_optional_revision(
    field: &'static str,
    value: &OptionalProblemRevision,
) -> Result<(), ContinuumProblemRevisionEnvelopeError> {
    match value {
        OptionalProblemRevision::Bound { revision_id } => {
            validate_id(field, revision_id, MAX_REVISION_ID_BYTES)
        }
        OptionalProblemRevision::NotApplicable { contract_id } => {
            validate_id(field, contract_id, MAX_CONTRACT_ID_BYTES)
        }
    }
}

fn validate_id(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ContinuumProblemRevisionEnvelopeError> {
    if value.trim().is_empty() {
        return Err(ContinuumProblemRevisionEnvelopeError::EmptyField(field));
    }
    if value.len() > max_bytes {
        return Err(ContinuumProblemRevisionEnvelopeError::FieldTooLong { field, max_bytes });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refinement_policy_contract::{
        EvidenceFreshness, REFINEMENT_POLICY_CONTRACT_SCHEMA_ID, RefinementAuthorityState,
    };
    use crate::validation::ContinuumValidityState;

    fn policy_input() -> ContinuumRefinementPolicyInput {
        ContinuumRefinementPolicyInput {
            schema_id: REFINEMENT_POLICY_CONTRACT_SCHEMA_ID.to_owned(),
            policy_contract_id: "continuum-refinement-policy-contract-v0.1".to_owned(),
            current_semantic_profile_id: "periodic-mac2d-reference-v0.1".to_owned(),
            diagnostic_profile_id: "periodic-mac2d-reference-v0.1;diag=v0.1".to_owned(),
            process_requirement_profile_id: "passive-control-process-v0.1".to_owned(),
            validity: ContinuumValidityState::Resolved,
            evidence_freshness: EvidenceFreshness::Current {
                evidence_revision: "exact-head:test".to_owned(),
            },
            authority: RefinementAuthorityState::MeasurementOnly,
            required_observables: vec![],
            missing_required_observables: vec![],
            available_semantic_profiles: vec![],
        }
    }

    fn envelope() -> ContinuumProblemRevisionEnvelope {
        let input = policy_input();
        ContinuumProblemRevisionEnvelope {
            schema_id: CONTINUUM_PROBLEM_REVISION_ENVELOPE_SCHEMA_ID.to_owned(),
            policy_contract_id: input.policy_contract_id,
            current_semantic_profile_id: input.current_semantic_profile_id,
            diagnostic_profile_id: input.diagnostic_profile_id,
            process_requirement_profile_id: input.process_requirement_profile_id,
            process_requirement_revision_id: "process-revision:001".to_owned(),
            state_revision_id: "state-revision:001".to_owned(),
            logical_snapshot_id: "snapshot:001".to_owned(),
            source_revision_id: "source:passive-taylor-green-v0.1".to_owned(),
            forcing_revision: OptionalProblemRevision::NotApplicable {
                contract_id: "unforced-passive-control-v0.1".to_owned(),
            },
            boundary_revision: OptionalProblemRevision::Bound {
                revision_id: "periodic-boundary-v0.1".to_owned(),
            },
        }
    }

    #[test]
    fn exact_subject_binding_accepts_explicit_unforced_control() {
        let input = policy_input();
        let envelope = envelope();
        envelope.validate().unwrap();
        envelope.bind_policy_input(&input).unwrap();
    }

    #[test]
    fn not_applicable_is_explicit_not_an_empty_sentinel() {
        let mut envelope = envelope();
        envelope.forcing_revision = OptionalProblemRevision::NotApplicable {
            contract_id: String::new(),
        };
        assert_eq!(
            envelope.validate().unwrap_err(),
            ContinuumProblemRevisionEnvelopeError::EmptyField("forcing_revision")
        );
    }

    #[test]
    fn every_policy_identity_mismatch_fails_closed() {
        let baseline = envelope();

        let mut input = policy_input();
        input.policy_contract_id.push_str("-changed");
        assert_eq!(
            baseline.bind_policy_input(&input).unwrap_err(),
            ContinuumProblemRevisionEnvelopeError::PolicyIdentityMismatch {
                field: "policy_contract_id"
            }
        );

        let mut input = policy_input();
        input.current_semantic_profile_id.push_str("-changed");
        assert_eq!(
            baseline.bind_policy_input(&input).unwrap_err(),
            ContinuumProblemRevisionEnvelopeError::PolicyIdentityMismatch {
                field: "current_semantic_profile_id"
            }
        );

        let mut input = policy_input();
        input.diagnostic_profile_id.push_str("-changed");
        assert_eq!(
            baseline.bind_policy_input(&input).unwrap_err(),
            ContinuumProblemRevisionEnvelopeError::PolicyIdentityMismatch {
                field: "diagnostic_profile_id"
            }
        );

        let mut input = policy_input();
        input.process_requirement_profile_id.push_str("-changed");
        assert_eq!(
            baseline.bind_policy_input(&input).unwrap_err(),
            ContinuumProblemRevisionEnvelopeError::PolicyIdentityMismatch {
                field: "process_requirement_profile_id"
            }
        );
    }

    #[test]
    fn revisions_are_bounded_and_structurally_validated() {
        let mut envelope = envelope();
        envelope.state_revision_id = "x".repeat(MAX_REVISION_ID_BYTES + 1);
        assert_eq!(
            envelope.validate().unwrap_err(),
            ContinuumProblemRevisionEnvelopeError::FieldTooLong {
                field: "state_revision_id",
                max_bytes: MAX_REVISION_ID_BYTES,
            }
        );
    }

    #[test]
    fn serialization_contains_no_renderer_or_backend_selection_fields() {
        let json = serde_json::to_string(&envelope())
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in ["camera", "fps", "gpu_load", "cpu_load", "backend_id"] {
            assert!(!json.contains(forbidden));
        }
    }
}
