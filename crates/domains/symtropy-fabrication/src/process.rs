// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed fabrication processes with deterministic preconditions and evidence.
//!
//! Processes describe intentional work. They never construct prefab inventory
//! items and never mutate conserved matter directly; a physical adapter must
//! commit the corresponding matter transition and return revisioned evidence.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{MatterBinding, Workpiece, WorkpieceId, WorkpieceLifecycle};

/// Stable identity of one process specification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessSpecId(StableId);

impl ProcessSpecId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for ProcessSpecId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Stable identity of one concrete process execution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessExecutionId(StableId);

impl ProcessExecutionId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for ProcessExecutionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Broad algebraic family used for planning and UI without replacing the
/// concrete process kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFamily {
    Prepare,
    Separate,
    Shape,
    Join,
    Treat,
    Connect,
    Configure,
    InspectTest,
}

/// Initial finite process vocabulary. This list should grow by adding physical
/// verbs, not by adding one process per craftable object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    Clean,
    Cut,
    Drill,
    Grind,
    Bend,
    Form,
    Align,
    Clamp,
    Fasten,
    Weld,
    Seal,
    Splice,
    Terminate,
    Coat,
    HeatTreat,
    Configure,
    Calibrate,
    Inspect,
    PressureTest,
    ContinuityTest,
}

impl ProcessKind {
    pub const fn family(self) -> ProcessFamily {
        match self {
            Self::Clean | Self::Align => ProcessFamily::Prepare,
            Self::Cut | Self::Drill | Self::Grind => ProcessFamily::Separate,
            Self::Bend | Self::Form => ProcessFamily::Shape,
            Self::Clamp | Self::Fasten | Self::Weld | Self::Seal => ProcessFamily::Join,
            Self::Coat | Self::HeatTreat => ProcessFamily::Treat,
            Self::Splice | Self::Terminate => ProcessFamily::Connect,
            Self::Configure | Self::Calibrate => ProcessFamily::Configure,
            Self::Inspect | Self::PressureTest | Self::ContinuityTest => {
                ProcessFamily::InspectTest
            }
        }
    }
}

/// Deterministic capability threshold. Integer units are defined by the stable
/// capability identity (for example torque, heat input, dimensional resolution)
/// so the foundation avoids float-dependent admission decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    pub capability_id: StableId,
    pub minimum_value: u64,
}

/// Evidence that a tool/machine/operator currently provides one capability.
/// F5 may introduce richer envelopes while preserving this admission contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEvidence {
    pub capability_id: StableId,
    pub available_value: u64,
    pub evidence_id: StableId,
}

impl CapabilityRequirement {
    pub fn is_satisfied_by(&self, evidence: &[CapabilityEvidence]) -> bool {
        evidence.iter().any(|candidate| {
            candidate.capability_id == self.capability_id
                && candidate.available_value >= self.minimum_value
        })
    }
}

/// Declarative process contract. A process specification is reusable knowledge;
/// it is not a one-shot execution and not a recipe output constructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub id: ProcessSpecId,
    /// Exact specification revision admitted by an execution.
    pub revision: u64,
    pub kind: ProcessKind,
    pub required_capabilities: Vec<CapabilityRequirement>,
    pub allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

impl ProcessSpec {
    pub fn new(
        id: ProcessSpecId,
        revision: u64,
        kind: ProcessKind,
        required_capabilities: Vec<CapabilityRequirement>,
        allowed_workpiece_states: Vec<WorkpieceLifecycle>,
    ) -> Result<Self, ProcessError> {
        if allowed_workpiece_states.is_empty() {
            return Err(ProcessError::NoAllowedWorkpieceStates);
        }
        Ok(Self {
            id,
            revision,
            kind,
            required_capabilities,
            allowed_workpiece_states,
        })
    }

    /// Evaluates every deterministic admission predicate without mutating state.
    pub fn evaluate_preconditions(
        &self,
        workpieces: &[&Workpiece],
        capabilities: &[CapabilityEvidence],
    ) -> Result<(), Vec<ProcessPreconditionFailure>> {
        let mut failures = Vec::new();
        if workpieces.is_empty() {
            failures.push(ProcessPreconditionFailure::WorkpieceRequired);
        }

        for (index, workpiece) in workpieces.iter().enumerate() {
            if workpieces[..index]
                .iter()
                .any(|existing| existing.id == workpiece.id)
            {
                failures.push(ProcessPreconditionFailure::DuplicateWorkpiece {
                    workpiece_id: workpiece.id.clone(),
                });
            }
            if !self.allowed_workpiece_states.contains(&workpiece.lifecycle) {
                failures.push(ProcessPreconditionFailure::WorkpieceState {
                    workpiece_id: workpiece.id.clone(),
                    actual: workpiece.lifecycle,
                });
            }
        }

        for requirement in &self.required_capabilities {
            if !requirement.is_satisfied_by(capabilities) {
                failures.push(ProcessPreconditionFailure::Capability {
                    capability_id: requirement.capability_id.clone(),
                    minimum_value: requirement.minimum_value,
                });
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "failure", rename_all = "snake_case")]
pub enum ProcessPreconditionFailure {
    WorkpieceRequired,
    DuplicateWorkpiece {
        workpiece_id: WorkpieceId,
    },
    WorkpieceState {
        workpiece_id: WorkpieceId,
        actual: WorkpieceLifecycle,
    },
    Capability {
        capability_id: StableId,
        minimum_value: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessExecutionState {
    InProgress,
    Completed,
    Aborted,
}

/// Immutable before-image captured at admission. The physical adapter can use
/// these revisions/digests to reject stale completion against changed matter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessInputSnapshot {
    pub workpiece_id: WorkpieceId,
    pub lifecycle: WorkpieceLifecycle,
    pub matter_bindings: Vec<MatterBinding>,
}

impl From<&Workpiece> for ProcessInputSnapshot {
    fn from(workpiece: &Workpiece) -> Self {
        Self {
            workpiece_id: workpiece.id.clone(),
            lifecycle: workpiece.lifecycle,
            matter_bindings: workpiece.matter_bindings.clone(),
        }
    }
}

/// In-progress process authority. It records exactly what state and capability
/// evidence was admitted, but does not pretend the intended physical
/// transformation has occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessExecution {
    pub id: ProcessExecutionId,
    pub spec_id: ProcessSpecId,
    pub spec_revision: u64,
    pub kind: ProcessKind,
    pub inputs: Vec<ProcessInputSnapshot>,
    pub admitted_capabilities: Vec<CapabilityEvidence>,
    pub state: ProcessExecutionState,
}

impl ProcessExecution {
    pub fn begin(
        id: ProcessExecutionId,
        spec: &ProcessSpec,
        workpieces: &[&Workpiece],
        capabilities: &[CapabilityEvidence],
    ) -> Result<Self, ProcessError> {
        if let Err(failures) = spec.evaluate_preconditions(workpieces, capabilities) {
            return Err(ProcessError::Preconditions(failures));
        }
        Ok(Self {
            id,
            spec_id: spec.id.clone(),
            spec_revision: spec.revision,
            kind: spec.kind,
            inputs: workpieces
                .iter()
                .map(|workpiece| ProcessInputSnapshot::from(*workpiece))
                .collect(),
            admitted_capabilities: capabilities.to_vec(),
            state: ProcessExecutionState::InProgress,
        })
    }

    /// Aborting is a first-class outcome. It records no fabricated result and
    /// leaves physical rollback/partial-change handling to the matter adapter.
    pub fn abort(&mut self) -> Result<(), ProcessError> {
        if self.state != ProcessExecutionState::InProgress {
            return Err(ProcessError::ExecutionClosed(self.id.clone()));
        }
        self.state = ProcessExecutionState::Aborted;
        Ok(())
    }

    /// Completes only when a physical authority supplies revisioned post-process
    /// matter evidence. The return value is evidence, not a prefab item.
    pub fn complete(
        &mut self,
        authority_id: StableId,
        evidence_id: StableId,
        evidence_revision: u64,
        evidence_digest: impl Into<String>,
        resulting_matter: Vec<MatterBinding>,
    ) -> Result<ProcessEvidence, ProcessError> {
        if self.state != ProcessExecutionState::InProgress {
            return Err(ProcessError::ExecutionClosed(self.id.clone()));
        }
        validate_resulting_matter(&resulting_matter)?;
        let evidence_digest = evidence_digest.into();
        if evidence_digest.is_empty() || evidence_digest.len() > 256 {
            return Err(ProcessError::InvalidEvidenceDigest(evidence_digest));
        }

        self.state = ProcessExecutionState::Completed;
        Ok(ProcessEvidence {
            authority_id,
            evidence_id,
            revision: evidence_revision,
            digest: evidence_digest,
            execution_id: self.id.clone(),
            spec_id: self.spec_id.clone(),
            spec_revision: self.spec_revision,
            kind: self.kind,
            inputs: self.inputs.clone(),
            admitted_capabilities: self.admitted_capabilities.clone(),
            resulting_matter,
            outcome: ProcessExecutionState::Completed,
        })
    }
}

/// Durable evidence that a process reached an externally grounded outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEvidence {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
    pub execution_id: ProcessExecutionId,
    pub spec_id: ProcessSpecId,
    pub spec_revision: u64,
    pub kind: ProcessKind,
    pub inputs: Vec<ProcessInputSnapshot>,
    pub admitted_capabilities: Vec<CapabilityEvidence>,
    pub resulting_matter: Vec<MatterBinding>,
    pub outcome: ProcessExecutionState,
}

fn validate_resulting_matter(bindings: &[MatterBinding]) -> Result<(), ProcessError> {
    if bindings.is_empty() {
        return Err(ProcessError::ResultingMatterEvidenceRequired);
    }
    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index].iter().any(|existing| {
            existing.authority_id == binding.authority_id
                && existing.allocation_id == binding.allocation_id
        }) {
            return Err(ProcessError::DuplicateResultingMatterAllocation {
                authority_id: binding.authority_id.clone(),
                allocation_id: binding.allocation_id.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum ProcessError {
    NoAllowedWorkpieceStates,
    Preconditions(Vec<ProcessPreconditionFailure>),
    ExecutionClosed(ProcessExecutionId),
    ResultingMatterEvidenceRequired,
    DuplicateResultingMatterAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
    InvalidEvidenceDigest(String),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAllowedWorkpieceStates => {
                write!(formatter, "process requires at least one allowed workpiece state")
            }
            Self::Preconditions(failures) => {
                write!(formatter, "process preconditions failed: {failures:?}")
            }
            Self::ExecutionClosed(id) => write!(formatter, "process execution {id} is already closed"),
            Self::ResultingMatterEvidenceRequired => {
                write!(formatter, "completed process requires resulting matter evidence")
            }
            Self::DuplicateResultingMatterAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "resulting matter allocation {authority_id}/{allocation_id} is listed more than once"
            ),
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "process evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
        }
    }
}

impl Error for ProcessError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FabricationError, MatterBinding};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn binding(allocation: &str, revision: u64) -> Result<MatterBinding, FabricationError> {
        MatterBinding::new(
            id("matter:universal"),
            id(allocation),
            revision,
            format!("digest:{allocation}:{revision}"),
        )
    }

    fn workpiece(name: &str, revision: u64) -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id(name)),
            vec![binding(&format!("allocation:{name}"), revision).unwrap()],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn weld_spec() -> ProcessSpec {
        ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:weld")),
            4,
            ProcessKind::Weld,
            vec![CapabilityRequirement {
                capability_id: id("capability:weld-heat"),
                minimum_value: 800,
            }],
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap()
    }

    fn welder() -> CapabilityEvidence {
        CapabilityEvidence {
            capability_id: id("capability:weld-heat"),
            available_value: 900,
            evidence_id: id("capability-evidence:welder"),
        }
    }

    #[test]
    fn missing_capability_fails_before_execution_exists() {
        let workpiece = workpiece("workpiece:plate", 7);
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &weld_spec(),
            &[&workpiece],
            &[],
        );
        assert!(matches!(result, Err(ProcessError::Preconditions(_))));
    }

    #[test]
    fn adequate_capability_admits_execution_deterministically() {
        let workpiece = workpiece("workpiece:plate", 7);
        let execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &weld_spec(),
            &[&workpiece],
            &[welder()],
        )
        .unwrap();
        assert_eq!(execution.state, ProcessExecutionState::InProgress);
        assert_eq!(execution.spec_revision, 4);
        assert_eq!(execution.inputs[0].matter_bindings[0].revision, 7);
        assert_eq!(execution.admitted_capabilities[0], welder());
    }

    #[test]
    fn duplicate_workpiece_is_rejected_before_execution() {
        let workpiece = workpiece("workpiece:plate", 7);
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &weld_spec(),
            &[&workpiece, &workpiece],
            &[welder()],
        );
        assert!(matches!(result, Err(ProcessError::Preconditions(_))));
    }

    #[test]
    fn abort_is_valid_and_cannot_later_be_completed() {
        let workpiece = workpiece("workpiece:plate", 7);
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &weld_spec(),
            &[&workpiece],
            &[welder()],
        )
        .unwrap();
        execution.abort().unwrap();
        let complete = execution.complete(
            id("matter:universal"),
            id("process-evidence:weld"),
            2,
            "digest:weld",
            vec![binding("allocation:plate-after", 8).unwrap()],
        );
        assert!(matches!(complete, Err(ProcessError::ExecutionClosed(_))));
    }

    #[test]
    fn completion_preserves_exact_before_and_after_images() {
        let workpiece = workpiece("workpiece:plate", 7);
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &weld_spec(),
            &[&workpiece],
            &[welder()],
        )
        .unwrap();
        let evidence = execution
            .complete(
                id("matter:universal"),
                id("process-evidence:weld"),
                2,
                "digest:weld",
                vec![binding("allocation:plate-after", 8).unwrap()],
            )
            .unwrap();
        assert_eq!(evidence.kind, ProcessKind::Weld);
        assert_eq!(evidence.outcome, ProcessExecutionState::Completed);
        assert_eq!(evidence.spec_revision, 4);
        assert_eq!(evidence.inputs[0].matter_bindings[0].revision, 7);
        assert_eq!(evidence.resulting_matter[0].revision, 8);
    }

    #[test]
    fn duplicate_resulting_allocation_is_rejected_within_same_authority() {
        let workpiece = workpiece("workpiece:plate", 7);
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &weld_spec(),
            &[&workpiece],
            &[welder()],
        )
        .unwrap();
        let result = execution.complete(
            id("matter:universal"),
            id("process-evidence:weld"),
            2,
            "digest:weld",
            vec![
                binding("allocation:plate-after", 8).unwrap(),
                binding("allocation:plate-after", 8).unwrap(),
            ],
        );
        assert!(matches!(
            result,
            Err(ProcessError::DuplicateResultingMatterAllocation { .. })
        ));
    }

    #[test]
    fn same_local_allocation_id_from_distinct_authorities_is_not_a_duplicate() {
        let workpiece = workpiece("workpiece:plate", 7);
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld-multi-authority")),
            &weld_spec(),
            &[&workpiece],
            &[welder()],
        )
        .unwrap();
        let result = execution.complete(
            id("matter:universal"),
            id("process-evidence:weld-multi-authority"),
            2,
            "digest:weld-multi-authority",
            vec![
                MatterBinding::new(
                    id("matter:authority-a"),
                    id("allocation:shared"),
                    8,
                    "digest:a:shared:8",
                )
                .unwrap(),
                MatterBinding::new(
                    id("matter:authority-b"),
                    id("allocation:shared"),
                    8,
                    "digest:b:shared:8",
                )
                .unwrap(),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn patch_conduit_workflow_composes_process_families_without_recipe_outputs() {
        let workflow = [
            ProcessKind::Clean,
            ProcessKind::Align,
            ProcessKind::Clamp,
            ProcessKind::Seal,
            ProcessKind::PressureTest,
        ];
        let families: Vec<_> = workflow.into_iter().map(ProcessKind::family).collect();
        assert_eq!(
            families,
            vec![
                ProcessFamily::Prepare,
                ProcessFamily::Prepare,
                ProcessFamily::Join,
                ProcessFamily::Join,
                ProcessFamily::InspectTest,
            ]
        );
    }
}
