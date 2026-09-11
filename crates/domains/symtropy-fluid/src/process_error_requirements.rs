// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Process-owned numerical error requirements for future continuum refinement policy.
//!
//! This module defines *requirements* only. It does not evaluate an error metric,
//! classify a flow, grant refinement authority, or select a semantic/execution
//! profile. A tolerance is meaningful only together with the exact evidence
//! profile, metric identity, semantics, unit, process-requirement profile, and
//! process-requirement revision that own it.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::problem_revision_envelope::{
    ContinuumProblemRevisionEnvelope, ContinuumProblemRevisionEnvelopeError, MAX_REVISION_ID_BYTES,
};
use crate::refinement_policy_contract::MAX_PROFILE_ID_BYTES;

pub const PROCESS_ERROR_REQUIREMENTS_SCHEMA_ID: &str =
    "continuum-process-error-requirements-v0.1";
pub const MAX_ERROR_METRIC_ID_BYTES: usize = 192;
pub const MAX_ERROR_REQUIREMENTS: usize = 64;

/// Semantics of one error-like quantity. These are intentionally error/residual
/// semantics rather than arbitrary physical observables such as raw speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorMetricSemantics {
    AbsoluteError,
    RelativeError,
    ResidualNorm,
    ConservationResidual,
    ScaleDrift,
}

/// Unit identity for one requirement. V0 is deliberately small and may be
/// extended only when a separately defined evidence producer needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorMetricUnit {
    Dimensionless,
    Meters,
    MetersPerSecond,
    MetersPerSecondSquared,
    PerSecond,
    Joules,
    PascalsPerSquareMeter,
}

/// Bit-canonical finite non-negative tolerance. Storing IEEE bits makes policy
/// identity deterministic and prevents JSON decimal formatting from defining
/// equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CanonicalNonNegativeTolerance {
    value_bits: u64,
}

impl CanonicalNonNegativeTolerance {
    pub fn new(value: f64) -> Result<Self, ProcessErrorRequirementsError> {
        if !value.is_finite() {
            return Err(ProcessErrorRequirementsError::NonFiniteTolerance);
        }
        if value < 0.0 {
            return Err(ProcessErrorRequirementsError::NegativeTolerance);
        }
        let canonical = if value == 0.0 { 0.0 } else { value };
        Ok(Self {
            value_bits: canonical.to_bits(),
        })
    }

    pub fn value(self) -> f64 {
        f64::from_bits(self.value_bits)
    }

    pub fn validate(self) -> Result<(), ProcessErrorRequirementsError> {
        let value = self.value();
        if !value.is_finite() {
            return Err(ProcessErrorRequirementsError::NonFiniteTolerance);
        }
        if value < 0.0 {
            return Err(ProcessErrorRequirementsError::NegativeTolerance);
        }
        if value == 0.0 && self.value_bits != 0.0_f64.to_bits() {
            return Err(ProcessErrorRequirementsError::NonCanonicalZeroTolerance);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessErrorRequirement {
    /// Stable metric identity defined by a qualified evidence producer.
    pub metric_id: String,
    /// Exact evidence/comparator/operator profile that gives `metric_id` its
    /// numerical meaning.
    pub evidence_profile_id: String,
    pub semantics: ErrorMetricSemantics,
    pub unit: ErrorMetricUnit,
    pub maximum_allowed: CanonicalNonNegativeTolerance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumProcessErrorRequirements {
    pub schema_id: String,
    pub process_requirement_profile_id: String,
    pub process_requirement_revision_id: String,
    /// Canonical sorted requirement set. V0 keys uniqueness by metric ID; a
    /// metric's semantics/unit/evidence profile may not be redefined twice in
    /// one process revision.
    pub requirements: Vec<ProcessErrorRequirement>,
}

impl ContinuumProcessErrorRequirements {
    pub fn validate(&self) -> Result<(), ProcessErrorRequirementsError> {
        if self.schema_id != PROCESS_ERROR_REQUIREMENTS_SCHEMA_ID {
            return Err(ProcessErrorRequirementsError::WrongSchemaId);
        }
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
        if self.requirements.len() > MAX_ERROR_REQUIREMENTS {
            return Err(ProcessErrorRequirementsError::TooManyRequirements);
        }
        if !strictly_sorted_unique_by_metric(&self.requirements) {
            return Err(ProcessErrorRequirementsError::NonCanonicalRequirementOrder);
        }
        for requirement in &self.requirements {
            validate_id(
                "metric_id",
                &requirement.metric_id,
                MAX_ERROR_METRIC_ID_BYTES,
            )?;
            validate_id(
                "evidence_profile_id",
                &requirement.evidence_profile_id,
                MAX_PROFILE_ID_BYTES,
            )?;
            requirement.maximum_allowed.validate()?;
        }
        Ok(())
    }

    /// Bind process-owned tolerances to the exact process profile/revision in a
    /// canonical problem-revision envelope. This does not evaluate any metric.
    pub fn bind_problem_revision(
        &self,
        envelope: &ContinuumProblemRevisionEnvelope,
    ) -> Result<(), ProcessErrorRequirementsError> {
        self.validate()?;
        envelope.validate()?;
        if self.process_requirement_profile_id != envelope.process_requirement_profile_id {
            return Err(ProcessErrorRequirementsError::ProblemRevisionMismatch {
                field: "process_requirement_profile_id",
            });
        }
        if self.process_requirement_revision_id != envelope.process_requirement_revision_id {
            return Err(ProcessErrorRequirementsError::ProblemRevisionMismatch {
                field: "process_requirement_revision_id",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessErrorRequirementsError {
    ProblemRevision(ContinuumProblemRevisionEnvelopeError),
    WrongSchemaId,
    EmptyField(&'static str),
    FieldTooLong {
        field: &'static str,
        max_bytes: usize,
    },
    TooManyRequirements,
    NonCanonicalRequirementOrder,
    NonFiniteTolerance,
    NegativeTolerance,
    NonCanonicalZeroTolerance,
    ProblemRevisionMismatch {
        field: &'static str,
    },
}

impl fmt::Display for ProcessErrorRequirementsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProblemRevision(source) => write!(f, "invalid problem revision envelope: {source}"),
            Self::WrongSchemaId => write!(f, "unsupported process error requirements schema"),
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::FieldTooLong { field, max_bytes } => {
                write!(f, "{field} must not exceed {max_bytes} bytes")
            }
            Self::TooManyRequirements => write!(f, "too many process error requirements"),
            Self::NonCanonicalRequirementOrder => write!(
                f,
                "process error requirements must be strictly sorted and unique by metric_id"
            ),
            Self::NonFiniteTolerance => write!(f, "process error tolerance must be finite"),
            Self::NegativeTolerance => write!(f, "process error tolerance must be non-negative"),
            Self::NonCanonicalZeroTolerance => {
                write!(f, "zero tolerance must use canonical positive-zero bits")
            }
            Self::ProblemRevisionMismatch { field } => {
                write!(f, "process error requirements do not match problem revision field {field}")
            }
        }
    }
}

impl std::error::Error for ProcessErrorRequirementsError {}

impl From<ContinuumProblemRevisionEnvelopeError> for ProcessErrorRequirementsError {
    fn from(value: ContinuumProblemRevisionEnvelopeError) -> Self {
        Self::ProblemRevision(value)
    }
}

fn strictly_sorted_unique_by_metric(requirements: &[ProcessErrorRequirement]) -> bool {
    requirements
        .windows(2)
        .all(|window| window[0].metric_id < window[1].metric_id)
}

fn validate_id(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ProcessErrorRequirementsError> {
    if value.trim().is_empty() {
        return Err(ProcessErrorRequirementsError::EmptyField(field));
    }
    if value.len() > max_bytes {
        return Err(ProcessErrorRequirementsError::FieldTooLong { field, max_bytes });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem_revision_envelope::{
        CONTINUUM_PROBLEM_REVISION_ENVELOPE_SCHEMA_ID, OptionalProblemRevision,
    };

    fn requirements() -> ContinuumProcessErrorRequirements {
        ContinuumProcessErrorRequirements {
            schema_id: PROCESS_ERROR_REQUIREMENTS_SCHEMA_ID.to_owned(),
            process_requirement_profile_id: "estuary-water-level-observables-v0.1".to_owned(),
            process_requirement_revision_id: "requirements:001".to_owned(),
            requirements: vec![
                ProcessErrorRequirement {
                    metric_id: "free-surface-height-absolute-error".to_owned(),
                    evidence_profile_id: "height-comparator-v0.1".to_owned(),
                    semantics: ErrorMetricSemantics::AbsoluteError,
                    unit: ErrorMetricUnit::Meters,
                    maximum_allowed: CanonicalNonNegativeTolerance::new(0.05).unwrap(),
                },
                ProcessErrorRequirement {
                    metric_id: "mass-conservation-relative-error".to_owned(),
                    evidence_profile_id: "water-mass-ledger-comparator-v0.1".to_owned(),
                    semantics: ErrorMetricSemantics::ConservationResidual,
                    unit: ErrorMetricUnit::Dimensionless,
                    maximum_allowed: CanonicalNonNegativeTolerance::new(1.0e-8).unwrap(),
                },
            ],
        }
    }

    fn envelope() -> ContinuumProblemRevisionEnvelope {
        ContinuumProblemRevisionEnvelope {
            schema_id: CONTINUUM_PROBLEM_REVISION_ENVELOPE_SCHEMA_ID.to_owned(),
            policy_contract_id: "continuum-refinement-policy-contract-v0.1".to_owned(),
            current_semantic_profile_id: "coastal-continuum-v0.1".to_owned(),
            diagnostic_profile_id: "coastal-continuum-v0.1;diag=v0.1".to_owned(),
            process_requirement_profile_id: "estuary-water-level-observables-v0.1".to_owned(),
            process_requirement_revision_id: "requirements:001".to_owned(),
            state_revision_id: "state:001".to_owned(),
            logical_snapshot_id: "snapshot:001".to_owned(),
            source_revision_id: "source:001".to_owned(),
            forcing_revision: OptionalProblemRevision::Bound {
                revision_id: "forcing:001".to_owned(),
            },
            boundary_revision: OptionalProblemRevision::Bound {
                revision_id: "boundary:001".to_owned(),
            },
        }
    }

    #[test]
    fn requirements_bind_exact_process_profile_and_revision() {
        let requirements = requirements();
        requirements.validate().unwrap();
        requirements.bind_problem_revision(&envelope()).unwrap();
    }

    #[test]
    fn tolerance_is_bit_canonical_and_rejects_bad_numbers() {
        assert_eq!(
            CanonicalNonNegativeTolerance::new(-0.0).unwrap().value().to_bits(),
            0.0_f64.to_bits()
        );
        assert_eq!(
            CanonicalNonNegativeTolerance::new(f64::NAN).unwrap_err(),
            ProcessErrorRequirementsError::NonFiniteTolerance
        );
        assert_eq!(
            CanonicalNonNegativeTolerance::new(-1.0).unwrap_err(),
            ProcessErrorRequirementsError::NegativeTolerance
        );
    }

    #[test]
    fn duplicate_or_unsorted_metric_identity_fails_closed() {
        let mut duplicate = requirements();
        duplicate.requirements[1].metric_id = duplicate.requirements[0].metric_id.clone();
        assert_eq!(
            duplicate.validate().unwrap_err(),
            ProcessErrorRequirementsError::NonCanonicalRequirementOrder
        );

        let mut reversed = requirements();
        reversed.requirements.reverse();
        assert_eq!(
            reversed.validate().unwrap_err(),
            ProcessErrorRequirementsError::NonCanonicalRequirementOrder
        );
    }

    #[test]
    fn process_revision_mismatch_fails_closed() {
        let requirements = requirements();
        let mut changed = envelope();
        changed.process_requirement_revision_id = "requirements:002".to_owned();
        assert_eq!(
            requirements.bind_problem_revision(&changed).unwrap_err(),
            ProcessErrorRequirementsError::ProblemRevisionMismatch {
                field: "process_requirement_revision_id"
            }
        );
    }

    #[test]
    fn schema_contains_no_renderer_backend_or_universal_resolution_fields() {
        let json = serde_json::to_string(&requirements()).unwrap().to_ascii_lowercase();
        for forbidden in [
            "camera",
            "fps",
            "gpu_load",
            "cpu_load",
            "backend_id",
            "universal_reynolds",
            "resolved_threshold",
        ] {
            assert!(!json.contains(forbidden));
        }
    }
}
