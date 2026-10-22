// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validated wire boundary for durable F4 process evidence.
//!
//! `ProcessEvidence` remains the compatibility record for the current wire
//! profile. This module establishes the stronger type required by downstream
//! authorities: a value can become `ValidatedProcessEvidence` only when the
//! redundant legacy fields agree with the canonical `ProcessSpecSnapshot` and
//! the invariants guaranteed by normal F4 constructors still hold after
//! serialization/deserialization.

use serde::{Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{
    CapabilityEvidence, MatterBinding, ProcessEvidence, ProcessExecutionId, ProcessExecutionState,
    ProcessInputSnapshot, ProcessKind, ProcessSpecId, ProcessSpecSnapshot, WorkpieceId,
    WorkpieceLifecycle,
};

/// Durable process evidence that has passed the same semantic/integrity checks
/// required at the authority boundary after wire restoration.
///
/// The inner compatibility record is intentionally private. Callers that must
/// bridge to legacy APIs can borrow it through `as_process_evidence`, while new
/// authority decisions should use the canonical accessors below.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ValidatedProcessEvidence(ProcessEvidence);

impl ValidatedProcessEvidence {
    pub fn new(evidence: ProcessEvidence) -> Result<Self, ProcessEvidenceValidationError> {
        validate_process_evidence(&evidence)?;
        Ok(Self(evidence))
    }

    /// Compatibility escape hatch for APIs not yet migrated to the validated
    /// type. Validation has already happened before this reference is exposed.
    pub fn as_process_evidence(&self) -> &ProcessEvidence {
        &self.0
    }

    /// Compatibility escape hatch for callers that must transfer ownership to
    /// an older API. New authority-bearing code should retain this strong type.
    pub fn into_process_evidence(self) -> ProcessEvidence {
        self.0
    }

    pub fn authority_id(&self) -> &StableId {
        &self.0.authority_id
    }

    pub fn evidence_id(&self) -> &StableId {
        &self.0.evidence_id
    }

    pub fn evidence_revision(&self) -> u64 {
        self.0.revision
    }

    pub fn evidence_digest(&self) -> &str {
        &self.0.digest
    }

    pub fn execution_id(&self) -> &ProcessExecutionId {
        &self.0.execution_id
    }

    pub fn spec_snapshot(&self) -> &ProcessSpecSnapshot {
        &self.0.spec_snapshot
    }

    pub fn process_spec_id(&self) -> &ProcessSpecId {
        &self.0.spec_snapshot.id
    }

    pub fn process_spec_revision(&self) -> u64 {
        self.0.spec_snapshot.revision
    }

    pub fn process_kind(&self) -> ProcessKind {
        self.0.spec_snapshot.kind
    }

    pub fn inputs(&self) -> &[ProcessInputSnapshot] {
        &self.0.inputs
    }

    pub fn admitted_capabilities(&self) -> &[CapabilityEvidence] {
        &self.0.admitted_capabilities
    }

    pub fn resulting_matter(&self) -> &[MatterBinding] {
        &self.0.resulting_matter
    }
}

impl TryFrom<ProcessEvidence> for ValidatedProcessEvidence {
    type Error = ProcessEvidenceValidationError;

    fn try_from(evidence: ProcessEvidence) -> Result<Self, Self::Error> {
        Self::new(evidence)
    }
}

impl<'de> Deserialize<'de> for ValidatedProcessEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let evidence = ProcessEvidence::deserialize(deserializer)?;
        Self::new(evidence).map_err(serde::de::Error::custom)
    }
}

/// Replays the invariants established by `ProcessSpec::new`,
/// `ProcessExecution::begin`, and `ProcessExecution::complete` after a raw wire
/// value has been restored.
pub fn validate_process_evidence(
    evidence: &ProcessEvidence,
) -> Result<(), ProcessEvidenceValidationError> {
    validate_spec_snapshot(&evidence.spec_snapshot)?;

    if evidence.spec_id != evidence.spec_snapshot.id {
        return Err(ProcessEvidenceValidationError::SpecIdMismatch {
            legacy: evidence.spec_id.clone(),
            canonical: evidence.spec_snapshot.id.clone(),
        });
    }
    if evidence.spec_revision != evidence.spec_snapshot.revision {
        return Err(ProcessEvidenceValidationError::SpecRevisionMismatch {
            legacy: evidence.spec_revision,
            canonical: evidence.spec_snapshot.revision,
        });
    }
    if evidence.kind != evidence.spec_snapshot.kind {
        return Err(ProcessEvidenceValidationError::ProcessKindMismatch {
            legacy: evidence.kind,
            canonical: evidence.spec_snapshot.kind,
        });
    }
    if evidence.outcome != ProcessExecutionState::Completed {
        return Err(ProcessEvidenceValidationError::OutcomeNotCompleted(
            evidence.outcome,
        ));
    }
    if evidence.digest.is_empty() || evidence.digest.len() > 256 {
        return Err(ProcessEvidenceValidationError::InvalidEvidenceDigest(
            evidence.digest.clone(),
        ));
    }
    if evidence.inputs.is_empty() {
        return Err(ProcessEvidenceValidationError::InputRequired);
    }

    for (index, input) in evidence.inputs.iter().enumerate() {
        if evidence.inputs[..index]
            .iter()
            .any(|existing| existing.workpiece_id == input.workpiece_id)
        {
            return Err(ProcessEvidenceValidationError::DuplicateInputWorkpiece(
                input.workpiece_id.clone(),
            ));
        }
        if !evidence
            .spec_snapshot
            .admits_workpiece_state(input.lifecycle)
        {
            return Err(ProcessEvidenceValidationError::InputLifecycleNotAdmitted {
                workpiece_id: input.workpiece_id.clone(),
                lifecycle: input.lifecycle,
            });
        }
        validate_input_matter_bindings(&input.matter_bindings)?;
    }

    for requirement in evidence.spec_snapshot.required_capabilities() {
        if !requirement.is_satisfied_by(&evidence.admitted_capabilities) {
            return Err(ProcessEvidenceValidationError::CapabilityRequirementNotSatisfied {
                capability_id: requirement.capability_id.clone(),
                minimum_value: requirement.minimum_value,
            });
        }
    }

    validate_resulting_matter_bindings(&evidence.resulting_matter)?;

    Ok(())
}

fn validate_spec_snapshot(
    snapshot: &ProcessSpecSnapshot,
) -> Result<(), ProcessEvidenceValidationError> {
    let requirements = snapshot.required_capabilities();
    for (index, requirement) in requirements.iter().enumerate() {
        if requirements[..index]
            .iter()
            .any(|existing| existing.capability_id == requirement.capability_id)
        {
            return Err(
                ProcessEvidenceValidationError::DuplicateSnapshotRequiredCapability(
                    requirement.capability_id.clone(),
                ),
            );
        }
    }
    for pair in requirements.windows(2) {
        if (&pair[0].capability_id, pair[0].minimum_value)
            > (&pair[1].capability_id, pair[1].minimum_value)
        {
            return Err(ProcessEvidenceValidationError::SnapshotCapabilityOrder);
        }
    }

    let states = snapshot.allowed_workpiece_states();
    if states.is_empty() {
        return Err(ProcessEvidenceValidationError::SnapshotAllowedStateRequired);
    }
    for (index, state) in states.iter().enumerate() {
        if states[..index].iter().any(|existing| existing == state) {
            return Err(
                ProcessEvidenceValidationError::DuplicateSnapshotAllowedWorkpieceState(*state),
            );
        }
    }
    for pair in states.windows(2) {
        if lifecycle_rank(pair[0]) > lifecycle_rank(pair[1]) {
            return Err(ProcessEvidenceValidationError::SnapshotLifecycleOrder);
        }
    }

    Ok(())
}

fn validate_input_matter_bindings(
    bindings: &[MatterBinding],
) -> Result<(), ProcessEvidenceValidationError> {
    if bindings.is_empty() {
        return Err(ProcessEvidenceValidationError::InputMatterRequired);
    }
    validate_matter_digests(bindings)?;
    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index].iter().any(|existing| {
            existing.authority_id == binding.authority_id
                && existing.allocation_id == binding.allocation_id
        }) {
            return Err(
                ProcessEvidenceValidationError::DuplicateInputMatterAllocation {
                    authority_id: binding.authority_id.clone(),
                    allocation_id: binding.allocation_id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_resulting_matter_bindings(
    bindings: &[MatterBinding],
) -> Result<(), ProcessEvidenceValidationError> {
    if bindings.is_empty() {
        return Err(ProcessEvidenceValidationError::ResultingMatterRequired);
    }
    validate_matter_digests(bindings)?;
    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index].iter().any(|existing| {
            existing.authority_id == binding.authority_id
                && existing.allocation_id == binding.allocation_id
        }) {
            return Err(
                ProcessEvidenceValidationError::DuplicateResultingMatterAllocation {
                    authority_id: binding.authority_id.clone(),
                    allocation_id: binding.allocation_id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_matter_digests(
    bindings: &[MatterBinding],
) -> Result<(), ProcessEvidenceValidationError> {
    for binding in bindings {
        if binding.binding_digest.is_empty() || binding.binding_digest.len() > 256 {
            return Err(ProcessEvidenceValidationError::InvalidMatterDigest {
                authority_id: binding.authority_id.clone(),
                allocation_id: binding.allocation_id.clone(),
                digest: binding.binding_digest.clone(),
            });
        }
    }
    Ok(())
}

const fn lifecycle_rank(lifecycle: WorkpieceLifecycle) -> u8 {
    match lifecycle {
        WorkpieceLifecycle::Staged => 0,
        WorkpieceLifecycle::InProcess => 1,
        WorkpieceLifecycle::Available => 2,
        WorkpieceLifecycle::Installed => 3,
        WorkpieceLifecycle::Removed => 4,
        WorkpieceLifecycle::Retired => 5,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvidenceValidationError {
    SnapshotAllowedStateRequired,
    DuplicateSnapshotRequiredCapability(StableId),
    SnapshotCapabilityOrder,
    DuplicateSnapshotAllowedWorkpieceState(WorkpieceLifecycle),
    SnapshotLifecycleOrder,
    SpecIdMismatch {
        legacy: ProcessSpecId,
        canonical: ProcessSpecId,
    },
    SpecRevisionMismatch {
        legacy: u64,
        canonical: u64,
    },
    ProcessKindMismatch {
        legacy: ProcessKind,
        canonical: ProcessKind,
    },
    OutcomeNotCompleted(ProcessExecutionState),
    InvalidEvidenceDigest(String),
    InputRequired,
    DuplicateInputWorkpiece(WorkpieceId),
    InputLifecycleNotAdmitted {
        workpiece_id: WorkpieceId,
        lifecycle: WorkpieceLifecycle,
    },
    InputMatterRequired,
    CapabilityRequirementNotSatisfied {
        capability_id: StableId,
        minimum_value: u64,
    },
    ResultingMatterRequired,
    InvalidMatterDigest {
        authority_id: StableId,
        allocation_id: StableId,
        digest: String,
    },
    DuplicateInputMatterAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
    DuplicateResultingMatterAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
}

impl fmt::Display for ProcessEvidenceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SnapshotAllowedStateRequired => write!(
                formatter,
                "canonical process snapshot requires at least one allowed workpiece state"
            ),
            Self::DuplicateSnapshotRequiredCapability(capability_id) => write!(
                formatter,
                "canonical process snapshot repeats required capability {capability_id}"
            ),
            Self::SnapshotCapabilityOrder => write!(
                formatter,
                "canonical process snapshot capability requirements are not constructor-normalized"
            ),
            Self::DuplicateSnapshotAllowedWorkpieceState(state) => write!(
                formatter,
                "canonical process snapshot repeats allowed workpiece state {state:?}"
            ),
            Self::SnapshotLifecycleOrder => write!(
                formatter,
                "canonical process snapshot allowed workpiece states are not constructor-normalized"
            ),
            Self::SpecIdMismatch { legacy, canonical } => write!(
                formatter,
                "process evidence spec id {legacy} disagrees with canonical snapshot {canonical}"
            ),
            Self::SpecRevisionMismatch { legacy, canonical } => write!(
                formatter,
                "process evidence spec revision {legacy} disagrees with canonical snapshot {canonical}"
            ),
            Self::ProcessKindMismatch { legacy, canonical } => write!(
                formatter,
                "process evidence kind {legacy:?} disagrees with canonical snapshot {canonical:?}"
            ),
            Self::OutcomeNotCompleted(outcome) => {
                write!(formatter, "process evidence outcome is not completed: {outcome:?}")
            }
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "process evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::InputRequired => write!(formatter, "process evidence requires at least one input"),
            Self::DuplicateInputWorkpiece(workpiece_id) => {
                write!(formatter, "process evidence repeats input workpiece {workpiece_id}")
            }
            Self::InputLifecycleNotAdmitted {
                workpiece_id,
                lifecycle,
            } => write!(
                formatter,
                "process evidence input {workpiece_id} lifecycle {lifecycle:?} is not admitted by the canonical process snapshot"
            ),
            Self::InputMatterRequired => {
                write!(formatter, "process evidence input requires matter bindings")
            }
            Self::CapabilityRequirementNotSatisfied {
                capability_id,
                minimum_value,
            } => write!(
                formatter,
                "process evidence does not satisfy canonical capability {capability_id} >= {minimum_value}"
            ),
            Self::ResultingMatterRequired => {
                write!(formatter, "process evidence requires resulting matter")
            }
            Self::InvalidMatterDigest {
                authority_id,
                allocation_id,
                digest,
            } => write!(
                formatter,
                "matter binding {authority_id}/{allocation_id} has invalid digest length {}",
                digest.len()
            ),
            Self::DuplicateInputMatterAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "process evidence repeats input matter allocation {authority_id}/{allocation_id}"
            ),
            Self::DuplicateResultingMatterAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "process evidence repeats resulting matter allocation {authority_id}/{allocation_id}"
            ),
        }
    }
}

impl Error for ProcessEvidenceValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityRequirement, ProcessExecution, ProcessSpec, Workpiece};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn matter(name: &str, revision: u64) -> MatterBinding {
        MatterBinding::new(
            id("matter-authority:test"),
            id(&format!("allocation:{name}")),
            revision,
            format!("digest:{name}:{revision}"),
        )
        .unwrap()
    }

    fn evidence() -> ProcessEvidence {
        evidence_with_spec(Vec::new(), vec![WorkpieceLifecycle::Available], Vec::new())
    }

    fn evidence_with_spec(
        requirements: Vec<CapabilityRequirement>,
        allowed_states: Vec<WorkpieceLifecycle>,
        capabilities: Vec<CapabilityEvidence>,
    ) -> ProcessEvidence {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:test")),
            vec![matter("input", 1)],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        let spec = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            4,
            ProcessKind::Clean,
            requirements,
            allowed_states,
        )
        .unwrap();
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:test")),
            &spec,
            &[&workpiece],
            &capabilities,
        )
        .unwrap();
        execution
            .complete(
                id("process-authority:test"),
                id("process-evidence:test"),
                1,
                "digest:process:test",
                vec![matter("result", 2)],
            )
            .unwrap()
    }

    #[test]
    fn valid_process_evidence_round_trips_through_validated_wire() {
        let validated = ValidatedProcessEvidence::new(evidence()).unwrap();
        let encoded = serde_json::to_vec(&validated).unwrap();
        let restored: ValidatedProcessEvidence = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(restored, validated);
        assert_eq!(restored.process_kind(), ProcessKind::Clean);
        assert_eq!(restored.process_spec_revision(), 4);
        assert_eq!(restored.authority_id(), &id("process-authority:test"));
        assert_eq!(restored.evidence_id(), &id("process-evidence:test"));
    }

    #[test]
    fn mismatched_legacy_id_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["spec_id"] = serde_json::Value::String("process-spec:other".into());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn mismatched_legacy_kind_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["kind"] = serde_json::Value::String("inspect".into());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn mismatched_legacy_revision_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["spec_revision"] = 99.into();
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn noncanonical_snapshot_capability_order_is_rejected() {
        let requirements = vec![
            CapabilityRequirement {
                capability_id: id("capability:a"),
                minimum_value: 1,
            },
            CapabilityRequirement {
                capability_id: id("capability:b"),
                minimum_value: 1,
            },
        ];
        let capabilities = vec![
            CapabilityEvidence {
                capability_id: id("capability:a"),
                available_value: 1,
                evidence_id: id("capability-evidence:a"),
            },
            CapabilityEvidence {
                capability_id: id("capability:b"),
                available_value: 1,
                evidence_id: id("capability-evidence:b"),
            },
        ];
        let mut value = serde_json::to_value(evidence_with_spec(
            requirements,
            vec![WorkpieceLifecycle::Available],
            capabilities,
        ))
        .unwrap();
        value["spec_snapshot"]["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn duplicate_snapshot_capability_is_rejected() {
        let requirement = CapabilityRequirement {
            capability_id: id("capability:test"),
            minimum_value: 7,
        };
        let capability = CapabilityEvidence {
            capability_id: id("capability:test"),
            available_value: 9,
            evidence_id: id("capability-evidence:test"),
        };
        let mut value = serde_json::to_value(evidence_with_spec(
            vec![requirement],
            vec![WorkpieceLifecycle::Available],
            vec![capability],
        ))
        .unwrap();
        let duplicate = value["spec_snapshot"]["required_capabilities"][0].clone();
        value["spec_snapshot"]["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn noncanonical_snapshot_lifecycle_order_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["spec_snapshot"]["allowed_workpiece_states"] = serde_json::json!([
            "installed",
            "available"
        ]);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn duplicate_snapshot_lifecycle_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["spec_snapshot"]["allowed_workpiece_states"] =
            serde_json::json!(["available", "available"]);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn malformed_completion_only_fields_are_rejected() {
        let mut bad_outcome = serde_json::to_value(evidence()).unwrap();
        bad_outcome["outcome"] = serde_json::Value::String("in_progress".into());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(bad_outcome).is_err());

        let mut empty_result = serde_json::to_value(evidence()).unwrap();
        empty_result["resulting_matter"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(empty_result).is_err());

        let mut empty_digest = serde_json::to_value(evidence()).unwrap();
        empty_digest["digest"] = serde_json::Value::String(String::new());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(empty_digest).is_err());
    }

    #[test]
    fn wire_cannot_change_admitted_input_lifecycle() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["inputs"][0]["lifecycle"] = serde_json::Value::String("installed".into());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn canonical_capability_requirements_are_revalidated() {
        let requirement = CapabilityRequirement {
            capability_id: id("capability:test"),
            minimum_value: 7,
        };
        let capability = CapabilityEvidence {
            capability_id: id("capability:test"),
            available_value: 9,
            evidence_id: id("capability-evidence:test"),
        };
        let mut value = serde_json::to_value(evidence_with_spec(
            vec![requirement],
            vec![WorkpieceLifecycle::Available],
            vec![capability],
        ))
        .unwrap();
        value["admitted_capabilities"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn nested_matter_constructor_invariants_are_revalidated() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        value["resulting_matter"][0]["binding_digest"] =
            serde_json::Value::String(String::new());
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn authority_scoped_duplicate_input_matter_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        let duplicate = value["inputs"][0]["matter_bindings"][0].clone();
        value["inputs"][0]["matter_bindings"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn authority_scoped_duplicate_resulting_matter_is_rejected() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        let duplicate = value["resulting_matter"][0].clone();
        value["resulting_matter"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn same_local_allocation_from_distinct_authorities_remains_valid() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        let mut second = value["resulting_matter"][0].clone();
        second["authority_id"] = serde_json::Value::String("matter-authority:other".into());
        value["resulting_matter"]
            .as_array_mut()
            .unwrap()
            .push(second);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_ok());
    }
}
