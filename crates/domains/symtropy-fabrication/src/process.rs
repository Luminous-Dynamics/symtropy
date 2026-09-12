// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed fabrication processes with deterministic preconditions and evidence.
//!
//! Processes describe intentional work. They never construct prefab inventory
//! items and never mutate conserved matter directly; a physical adapter must
//! commit the corresponding matter transition and return revisioned evidence.

use serde::{Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{MatterBinding, MatterIntegrityError, Workpiece, WorkpieceId, WorkpieceLifecycle};

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

/// Declares what deliberate separation means for matter and serviceability.
///
/// This is intentionally not an inverse-process table. A welded or sealed joint
/// may require destructive or consumable-interface work to release, so callers
/// must select the actual physical separation process rather than assuming that
/// every join can be undone cleanly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeparationSemantics {
    /// Intended to preserve the joined workpieces for reuse, subject to the
    /// resulting matter evidence supplied by the physical authority.
    NonDestructive,
    /// Intended to preserve durable workpieces while consuming or replacing an
    /// interface material such as a gasket, sealant, or sacrificial bond.
    ConsumableInterface,
    /// Intentionally removes or damages material to liberate the workpiece.
    Destructive,
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
    /// Release a clamped interface without asserting that the underlying
    /// workpieces are undamaged; resulting matter evidence remains authoritative.
    ReleaseClamp,
    /// Remove a deliberately releasable fastener or fastening set.
    Unfasten,
    /// Separate a service connection such as an electrical/fluid termination.
    Disconnect,
    /// Separate a mechanical coupling designed for deliberate release.
    Decouple,
    /// Open a sealed interface where the seal/gasket may be consumed.
    Unseal,
    /// Liberate joined workpieces by intentionally cutting material.
    CutFree,
    /// Remove a workpiece from a surrounding fit, seat, pocket, or assembly.
    Extract,
    /// Join mating mechanical or fluid-service interfaces designed to couple.
    Couple,
}

impl ProcessKind {
    pub const fn family(self) -> ProcessFamily {
        match self {
            Self::Clean | Self::Align => ProcessFamily::Prepare,
            Self::Cut
            | Self::Drill
            | Self::Grind
            | Self::ReleaseClamp
            | Self::Unfasten
            | Self::Disconnect
            | Self::Decouple
            | Self::Unseal
            | Self::CutFree
            | Self::Extract => ProcessFamily::Separate,
            Self::Bend | Self::Form => ProcessFamily::Shape,
            Self::Clamp | Self::Fasten | Self::Weld | Self::Seal => ProcessFamily::Join,
            Self::Coat | Self::HeatTreat => ProcessFamily::Treat,
            Self::Splice | Self::Terminate | Self::Couple => ProcessFamily::Connect,
            Self::Configure | Self::Calibrate => ProcessFamily::Configure,
            Self::Inspect | Self::PressureTest | Self::ContinuityTest => ProcessFamily::InspectTest,
        }
    }

    /// Returns explicit matter/serviceability semantics for physical separation
    /// verbs. `None` means this process is not itself a separation operation;
    /// notably join verbs do not claim a universal inverse.
    pub const fn separation_semantics(self) -> Option<SeparationSemantics> {
        match self {
            Self::ReleaseClamp
            | Self::Unfasten
            | Self::Disconnect
            | Self::Decouple
            | Self::Extract => Some(SeparationSemantics::NonDestructive),
            Self::Unseal => Some(SeparationSemantics::ConsumableInterface),
            Self::Cut | Self::Drill | Self::Grind | Self::CutFree => {
                Some(SeparationSemantics::Destructive)
            }
            _ => None,
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

/// Canonical structural identity of one exact process contract.
///
/// This intentionally uses direct structural equality instead of introducing a
/// local hash/crypto authority. A later registry or signature layer can bind
/// this value without changing what counts as the process's semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessSpecSnapshot {
    pub id: ProcessSpecId,
    pub revision: u64,
    pub kind: ProcessKind,
    required_capabilities: Vec<CapabilityRequirement>,
    allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

#[derive(Deserialize)]
struct ProcessSpecSnapshotWire {
    id: ProcessSpecId,
    revision: u64,
    kind: ProcessKind,
    required_capabilities: Vec<CapabilityRequirement>,
    allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

/// Representation-level canonicality failures for exact F4 process snapshots.
///
/// A reusable `ProcessSpec` may be normalized through `ProcessSpec::new`, but a
/// historical `ProcessSpecSnapshot` must already be in this exact canonical
/// representation before it can regain authority after persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessSpecSnapshotError {
    NoAllowedWorkpieceStates,
    DuplicateRequiredCapability(StableId),
    NonCanonicalCapabilityOrder,
    DuplicateAllowedWorkpieceState(WorkpieceLifecycle),
    NonCanonicalLifecycleOrder,
}

impl fmt::Display for ProcessSpecSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAllowedWorkpieceStates => write!(
                formatter,
                "canonical process snapshot requires at least one allowed workpiece state"
            ),
            Self::DuplicateRequiredCapability(capability_id) => write!(
                formatter,
                "canonical process snapshot repeats required capability {capability_id}"
            ),
            Self::NonCanonicalCapabilityOrder => write!(
                formatter,
                "canonical process snapshot capability requirements are not constructor-normalized"
            ),
            Self::DuplicateAllowedWorkpieceState(state) => write!(
                formatter,
                "canonical process snapshot repeats allowed workpiece state {state:?}"
            ),
            Self::NonCanonicalLifecycleOrder => write!(
                formatter,
                "canonical process snapshot allowed workpiece states are not constructor-normalized"
            ),
        }
    }
}

impl Error for ProcessSpecSnapshotError {}

impl<'de> Deserialize<'de> for ProcessSpecSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProcessSpecSnapshotWire::deserialize(deserializer)?;
        let snapshot = Self {
            id: wire.id,
            revision: wire.revision,
            kind: wire.kind,
            required_capabilities: wire.required_capabilities,
            allowed_workpiece_states: wire.allowed_workpiece_states,
        };
        snapshot
            .validate_canonical()
            .map_err(serde::de::Error::custom)?;
        Ok(snapshot)
    }
}

impl ProcessSpecSnapshot {
    /// Proves that this exact representation is the unique constructor-normalized
    /// form used for structural equality at F4/F10 authority boundaries.
    pub fn validate_canonical(&self) -> Result<(), ProcessSpecSnapshotError> {
        for (index, requirement) in self.required_capabilities.iter().enumerate() {
            if self.required_capabilities[..index]
                .iter()
                .any(|existing| existing.capability_id == requirement.capability_id)
            {
                return Err(ProcessSpecSnapshotError::DuplicateRequiredCapability(
                    requirement.capability_id.clone(),
                ));
            }
        }
        for pair in self.required_capabilities.windows(2) {
            if (&pair[0].capability_id, pair[0].minimum_value)
                >= (&pair[1].capability_id, pair[1].minimum_value)
            {
                return Err(ProcessSpecSnapshotError::NonCanonicalCapabilityOrder);
            }
        }

        if self.allowed_workpiece_states.is_empty() {
            return Err(ProcessSpecSnapshotError::NoAllowedWorkpieceStates);
        }
        for (index, state) in self.allowed_workpiece_states.iter().enumerate() {
            if self.allowed_workpiece_states[..index]
                .iter()
                .any(|existing| existing == state)
            {
                return Err(ProcessSpecSnapshotError::DuplicateAllowedWorkpieceState(
                    *state,
                ));
            }
        }
        for pair in self.allowed_workpiece_states.windows(2) {
            if lifecycle_rank(pair[0]) >= lifecycle_rank(pair[1]) {
                return Err(ProcessSpecSnapshotError::NonCanonicalLifecycleOrder);
            }
        }
        Ok(())
    }

    pub fn required_capabilities(&self) -> &[CapabilityRequirement] {
        &self.required_capabilities
    }

    pub fn allowed_workpiece_states(&self) -> &[WorkpieceLifecycle] {
        &self.allowed_workpiece_states
    }

    pub fn admits_workpiece_state(&self, lifecycle: WorkpieceLifecycle) -> bool {
        self.allowed_workpiece_states.contains(&lifecycle)
    }
}

/// Declarative process contract. A process specification is reusable knowledge;
/// it is not a one-shot execution and not a recipe output constructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessSpec {
    pub id: ProcessSpecId,
    /// Exact specification revision admitted by an execution.
    pub revision: u64,
    pub kind: ProcessKind,
    pub required_capabilities: Vec<CapabilityRequirement>,
    pub allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

#[derive(Deserialize)]
struct ProcessSpecWire {
    id: ProcessSpecId,
    revision: u64,
    kind: ProcessKind,
    required_capabilities: Vec<CapabilityRequirement>,
    allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

impl<'de> Deserialize<'de> for ProcessSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProcessSpecWire::deserialize(deserializer)?;
        Self::new(
            wire.id,
            wire.revision,
            wire.kind,
            wire.required_capabilities,
            wire.allowed_workpiece_states,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl ProcessSpec {
    pub fn new(
        id: ProcessSpecId,
        revision: u64,
        kind: ProcessKind,
        mut required_capabilities: Vec<CapabilityRequirement>,
        mut allowed_workpiece_states: Vec<WorkpieceLifecycle>,
    ) -> Result<Self, ProcessError> {
        if allowed_workpiece_states.is_empty() {
            return Err(ProcessError::NoAllowedWorkpieceStates);
        }

        required_capabilities.sort_by(|left, right| {
            (&left.capability_id, left.minimum_value)
                .cmp(&(&right.capability_id, right.minimum_value))
        });
        for pair in required_capabilities.windows(2) {
            if pair[0].capability_id == pair[1].capability_id {
                return Err(ProcessError::DuplicateRequiredCapability(
                    pair[0].capability_id.clone(),
                ));
            }
        }

        allowed_workpiece_states.sort_by_key(|lifecycle| lifecycle_rank(*lifecycle));
        for pair in allowed_workpiece_states.windows(2) {
            if pair[0] == pair[1] {
                return Err(ProcessError::DuplicateAllowedWorkpieceState(pair[0]));
            }
        }

        Ok(Self {
            id,
            revision,
            kind,
            required_capabilities,
            allowed_workpiece_states,
        })
    }

    /// Revalidates a live spec before an authority-bearing consumer uses it.
    /// This catches public field mutation without silently normalizing an object
    /// that is already being presented as live execution knowledge.
    pub fn validate_canonical(&self) -> Result<(), ProcessSpecSnapshotError> {
        self.snapshot().validate_canonical()
    }

    /// Returns the structural process semantics. Authority-bearing consumers
    /// must call `validate_canonical` before accepting a snapshot produced from
    /// a publicly mutable `ProcessSpec`.
    pub fn snapshot(&self) -> ProcessSpecSnapshot {
        ProcessSpecSnapshot {
            id: self.id.clone(),
            revision: self.revision,
            kind: self.kind,
            required_capabilities: self.required_capabilities.clone(),
            allowed_workpiece_states: self.allowed_workpiece_states.clone(),
        }
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
            if let Err(error) = workpiece.validate_matter_integrity() {
                failures.push(match error {
                    MatterIntegrityError::BindingRequired => {
                        ProcessPreconditionFailure::WorkpieceMatterRequired {
                            workpiece_id: workpiece.id.clone(),
                        }
                    }
                    MatterIntegrityError::InvalidDigest {
                        authority_id,
                        allocation_id,
                        digest,
                    } => ProcessPreconditionFailure::InvalidWorkpieceMatterDigest {
                        workpiece_id: workpiece.id.clone(),
                        authority_id,
                        allocation_id,
                        digest,
                    },
                    MatterIntegrityError::DuplicateAllocation {
                        authority_id,
                        allocation_id,
                    } => ProcessPreconditionFailure::DuplicateWorkpieceMatterAllocation {
                        workpiece_id: workpiece.id.clone(),
                        authority_id,
                        allocation_id,
                    },
                });
            }

            for binding in &workpiece.matter_bindings {
                for previous_workpiece in &workpieces[..index] {
                    if previous_workpiece.id != workpiece.id
                        && previous_workpiece.matter_bindings.iter().any(|existing| {
                            existing.authority_id == binding.authority_id
                                && existing.allocation_id == binding.allocation_id
                        })
                    {
                        failures.push(
                            ProcessPreconditionFailure::CrossWorkpieceMatterAllocationAlias {
                                authority_id: binding.authority_id.clone(),
                                allocation_id: binding.allocation_id.clone(),
                                first_workpiece_id: previous_workpiece.id.clone(),
                                second_workpiece_id: workpiece.id.clone(),
                            },
                        );
                        break;
                    }
                }
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
    WorkpieceMatterRequired {
        workpiece_id: WorkpieceId,
    },
    InvalidWorkpieceMatterDigest {
        workpiece_id: WorkpieceId,
        authority_id: StableId,
        allocation_id: StableId,
        digest: String,
    },
    DuplicateWorkpieceMatterAllocation {
        workpiece_id: WorkpieceId,
        authority_id: StableId,
        allocation_id: StableId,
    },
    CrossWorkpieceMatterAllocationAlias {
        authority_id: StableId,
        allocation_id: StableId,
        first_workpiece_id: WorkpieceId,
        second_workpiece_id: WorkpieceId,
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
    /// Canonical immutable semantics admitted at F4 begin.
    pub spec_snapshot: ProcessSpecSnapshot,
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
        spec.validate_canonical()
            .map_err(ProcessError::NonCanonicalSpec)?;
        if let Err(failures) = spec.evaluate_preconditions(workpieces, capabilities) {
            return Err(ProcessError::Preconditions(failures));
        }
        Ok(Self {
            id,
            spec_id: spec.id.clone(),
            spec_revision: spec.revision,
            kind: spec.kind,
            spec_snapshot: spec.snapshot(),
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
            spec_snapshot: self.spec_snapshot.clone(),
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
    /// Exact canonical F4 semantics captured before the process began.
    pub spec_snapshot: ProcessSpecSnapshot,
    pub inputs: Vec<ProcessInputSnapshot>,
    pub admitted_capabilities: Vec<CapabilityEvidence>,
    pub resulting_matter: Vec<MatterBinding>,
    pub outcome: ProcessExecutionState,
}

fn lifecycle_rank(lifecycle: WorkpieceLifecycle) -> u8 {
    match lifecycle {
        WorkpieceLifecycle::Staged => 0,
        WorkpieceLifecycle::InProcess => 1,
        WorkpieceLifecycle::Available => 2,
        WorkpieceLifecycle::Installed => 3,
        WorkpieceLifecycle::Removed => 4,
        WorkpieceLifecycle::Retired => 5,
    }
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
    DuplicateRequiredCapability(StableId),
    DuplicateAllowedWorkpieceState(WorkpieceLifecycle),
    NonCanonicalSpec(ProcessSpecSnapshotError),
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
                write!(
                    formatter,
                    "process requires at least one allowed workpiece state"
                )
            }
            Self::DuplicateRequiredCapability(id) => {
                write!(formatter, "process repeats required capability {id}")
            }
            Self::DuplicateAllowedWorkpieceState(lifecycle) => write!(
                formatter,
                "process repeats allowed workpiece lifecycle {lifecycle:?}"
            ),
            Self::NonCanonicalSpec(error) => {
                write!(formatter, "process spec is not canonical: {error}")
            }
            Self::Preconditions(failures) => {
                write!(formatter, "process preconditions failed: {failures:?}")
            }
            Self::ExecutionClosed(id) => {
                write!(formatter, "process execution {id} is already closed")
            }
            Self::ResultingMatterEvidenceRequired => {
                write!(
                    formatter,
                    "completed process requires resulting matter evidence"
                )
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

    fn requirement(name: &str, minimum_value: u64) -> CapabilityRequirement {
        CapabilityRequirement {
            capability_id: id(name),
            minimum_value,
        }
    }

    fn weld_spec() -> ProcessSpec {
        ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:weld")),
            4,
            ProcessKind::Weld,
            vec![requirement("capability:weld-heat", 800)],
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
    fn process_spec_snapshot_is_canonical_across_input_order() {
        let left = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            7,
            ProcessKind::PressureTest,
            vec![
                requirement("capability:pressure-measurement", 5),
                requirement("capability:pressure-source", 10),
            ],
            vec![WorkpieceLifecycle::Installed, WorkpieceLifecycle::Available],
        )
        .unwrap();
        let right = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            7,
            ProcessKind::PressureTest,
            vec![
                requirement("capability:pressure-source", 10),
                requirement("capability:pressure-measurement", 5),
            ],
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap();

        assert_eq!(left.snapshot(), right.snapshot());
    }

    #[test]
    fn duplicate_capability_identity_is_rejected_even_with_different_thresholds() {
        let result = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            1,
            ProcessKind::Weld,
            vec![
                requirement("capability:heat", 700),
                requirement("capability:heat", 800),
            ],
            vec![WorkpieceLifecycle::Available],
        );
        assert!(matches!(
            result,
            Err(ProcessError::DuplicateRequiredCapability(capability_id))
                if capability_id == id("capability:heat")
        ));
    }

    #[test]
    fn duplicate_allowed_lifecycle_is_rejected() {
        let result = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            1,
            ProcessKind::Inspect,
            Vec::new(),
            vec![WorkpieceLifecycle::Installed, WorkpieceLifecycle::Installed],
        );
        assert!(matches!(
            result,
            Err(ProcessError::DuplicateAllowedWorkpieceState(
                WorkpieceLifecycle::Installed
            ))
        ));
    }

    #[test]
    fn reusable_process_spec_wire_normalizes_input_order() {
        let canonical = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:wire-normalization")),
            2,
            ProcessKind::PressureTest,
            vec![
                requirement("capability:a", 1),
                requirement("capability:b", 2),
            ],
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap();
        let mut value = serde_json::to_value(&canonical).unwrap();
        value["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .reverse();
        value["allowed_workpiece_states"]
            .as_array_mut()
            .unwrap()
            .reverse();

        let restored: ProcessSpec = serde_json::from_value(value).unwrap();
        assert_eq!(restored, canonical);
        assert_eq!(restored.snapshot(), canonical.snapshot());
    }

    #[test]
    fn reusable_process_spec_wire_rejects_duplicate_capability_identity() {
        let mut value = serde_json::to_value(weld_spec()).unwrap();
        let mut duplicate = value["required_capabilities"][0].clone();
        duplicate["minimum_value"] = 801.into();
        value["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ProcessSpec>(value).is_err());
    }

    #[test]
    fn reusable_process_spec_wire_rejects_empty_or_duplicate_allowed_states() {
        let mut empty = serde_json::to_value(weld_spec()).unwrap();
        empty["allowed_workpiece_states"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<ProcessSpec>(empty).is_err());

        let mut duplicate = serde_json::to_value(weld_spec()).unwrap();
        duplicate["allowed_workpiece_states"] = serde_json::json!(["available", "available"]);
        assert!(serde_json::from_value::<ProcessSpec>(duplicate).is_err());
    }

    #[test]
    fn exact_snapshot_wire_rejects_noncanonical_capability_order() {
        let spec = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:snapshot-cap-order")),
            1,
            ProcessKind::Inspect,
            vec![
                requirement("capability:a", 1),
                requirement("capability:b", 1),
            ],
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap();
        let mut value = serde_json::to_value(spec.snapshot()).unwrap();
        value["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(value).is_err());
    }

    #[test]
    fn exact_snapshot_wire_rejects_duplicate_capability_even_with_different_thresholds() {
        let mut value = serde_json::to_value(weld_spec().snapshot()).unwrap();
        let mut duplicate = value["required_capabilities"][0].clone();
        duplicate["minimum_value"] = 999.into();
        value["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(value).is_err());
    }

    #[test]
    fn exact_snapshot_wire_rejects_empty_duplicate_or_reordered_states() {
        let snapshot = weld_spec().snapshot();

        let mut empty = serde_json::to_value(&snapshot).unwrap();
        empty["allowed_workpiece_states"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(empty).is_err());

        let mut duplicate = serde_json::to_value(&snapshot).unwrap();
        duplicate["allowed_workpiece_states"] = serde_json::json!(["available", "available"]);
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(duplicate).is_err());

        let mut reordered = serde_json::to_value(&snapshot).unwrap();
        reordered["allowed_workpiece_states"] = serde_json::json!(["installed", "available"]);
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(reordered).is_err());
    }

    #[test]
    fn execution_rejects_publicly_mutated_noncanonical_spec() {
        let workpiece = workpiece("workpiece:mutated-spec", 1);
        let mut spec = weld_spec();
        spec.allowed_workpiece_states.reverse();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:mutated-spec")),
            &spec,
            &[&workpiece],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::NonCanonicalSpec(
                ProcessSpecSnapshotError::NonCanonicalLifecycleOrder
            ))
        ));
    }

    #[test]
    fn same_id_revision_with_changed_semantics_has_distinct_snapshot() {
        let original = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            3,
            ProcessKind::Inspect,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap();
        let altered = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:test")),
            3,
            ProcessKind::Inspect,
            Vec::new(),
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap();

        assert_eq!(original.id, altered.id);
        assert_eq!(original.revision, altered.revision);
        assert_ne!(original.snapshot(), altered.snapshot());
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
        let spec = weld_spec();
        let execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &spec,
            &[&workpiece],
            &[welder()],
        )
        .unwrap();
        assert_eq!(execution.state, ProcessExecutionState::InProgress);
        assert_eq!(execution.spec_revision, 4);
        assert_eq!(execution.spec_snapshot, spec.snapshot());
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
    fn restored_workpiece_without_matter_is_rejected_before_execution() {
        let mut input = workpiece("workpiece:empty-matter", 1);
        input.matter_bindings.clear();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:empty-matter")),
            &weld_spec(),
            &[&input],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::WorkpieceMatterRequired { .. }
                ))
        ));
    }

    #[test]
    fn restored_invalid_input_matter_digest_is_rejected_before_execution() {
        let mut input = workpiece("workpiece:bad-digest", 1);
        input.matter_bindings[0].binding_digest.clear();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:bad-digest")),
            &weld_spec(),
            &[&input],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::InvalidWorkpieceMatterDigest { .. }
                ))
        ));
    }

    #[test]
    fn restored_duplicate_allocation_inside_one_workpiece_is_rejected() {
        let mut input = workpiece("workpiece:duplicate-matter", 1);
        input.matter_bindings.push(input.matter_bindings[0].clone());
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:duplicate-matter")),
            &weld_spec(),
            &[&input],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::DuplicateWorkpieceMatterAllocation { .. }
                ))
        ));
    }

    #[test]
    fn distinct_workpieces_cannot_alias_same_authority_scoped_allocation() {
        let first = workpiece("workpiece:alias-a", 1);
        let mut second = workpiece("workpiece:alias-b", 1);
        second.matter_bindings = first.matter_bindings.clone();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:cross-alias")),
            &weld_spec(),
            &[&first, &second],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::CrossWorkpieceMatterAllocationAlias {
                        first_workpiece_id,
                        second_workpiece_id,
                        ..
                    } if first_workpiece_id == &first.id && second_workpiece_id == &second.id
                ))
        ));
    }

    #[test]
    fn same_local_input_allocation_id_from_distinct_authorities_is_valid() {
        let mut first = workpiece("workpiece:authority-a", 1);
        first.matter_bindings = vec![
            MatterBinding::new(
                id("matter:authority-a"),
                id("allocation:shared-input"),
                1,
                "digest:a:shared-input:1",
            )
            .unwrap(),
        ];
        let mut second = workpiece("workpiece:authority-b", 1);
        second.matter_bindings = vec![
            MatterBinding::new(
                id("matter:authority-b"),
                id("allocation:shared-input"),
                1,
                "digest:b:shared-input:1",
            )
            .unwrap(),
        ];
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:multi-authority-input")),
            &weld_spec(),
            &[&first, &second],
            &[welder()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn distinct_multi_input_allocations_are_valid() {
        let first = workpiece("workpiece:multi-a", 1);
        let second = workpiece("workpiece:multi-b", 1);
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:distinct-multi")),
            &weld_spec(),
            &[&first, &second],
            &[welder()],
        );
        assert!(result.is_ok());
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
    fn completion_preserves_exact_before_after_and_process_semantics() {
        let workpiece = workpiece("workpiece:plate", 7);
        let spec = weld_spec();
        let mut execution = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:weld")),
            &spec,
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
        assert_eq!(evidence.spec_snapshot, spec.snapshot());
        assert_eq!(evidence.inputs[0].matter_bindings[0].revision, 7);
        assert_eq!(evidence.resulting_matter[0].revision, 8);
    }

    #[test]
    fn snapshot_round_trip_preserves_exact_semantics() {
        let snapshot = weld_spec().snapshot();
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        let restored: ProcessSpecSnapshot = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, snapshot);
        assert!(restored.admits_workpiece_state(WorkpieceLifecycle::Installed));
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
    fn separation_verbs_declare_explicit_serviceability_semantics() {
        assert_eq!(
            ProcessKind::ReleaseClamp.separation_semantics(),
            Some(SeparationSemantics::NonDestructive)
        );
        assert_eq!(
            ProcessKind::Unfasten.separation_semantics(),
            Some(SeparationSemantics::NonDestructive)
        );
        assert_eq!(
            ProcessKind::Disconnect.separation_semantics(),
            Some(SeparationSemantics::NonDestructive)
        );
        assert_eq!(
            ProcessKind::Decouple.separation_semantics(),
            Some(SeparationSemantics::NonDestructive)
        );
        assert_eq!(
            ProcessKind::Unseal.separation_semantics(),
            Some(SeparationSemantics::ConsumableInterface)
        );
        assert_eq!(
            ProcessKind::CutFree.separation_semantics(),
            Some(SeparationSemantics::Destructive)
        );
        assert_eq!(
            ProcessKind::Extract.separation_semantics(),
            Some(SeparationSemantics::NonDestructive)
        );
    }

    #[test]
    fn join_verbs_never_claim_a_universal_clean_inverse() {
        for kind in [
            ProcessKind::Clamp,
            ProcessKind::Fasten,
            ProcessKind::Weld,
            ProcessKind::Seal,
            ProcessKind::Splice,
            ProcessKind::Terminate,
            ProcessKind::Couple,
        ] {
            assert_eq!(kind.separation_semantics(), None);
        }
    }

    #[test]
    fn material_removing_separation_is_explicitly_destructive() {
        for kind in [
            ProcessKind::Cut,
            ProcessKind::Drill,
            ProcessKind::Grind,
            ProcessKind::CutFree,
        ] {
            assert_eq!(
                kind.separation_semantics(),
                Some(SeparationSemantics::Destructive)
            );
        }
    }

    #[test]
    fn coupling_and_decoupling_are_distinct_physical_operations() {
        assert_eq!(ProcessKind::Couple.family(), ProcessFamily::Connect);
        assert_eq!(ProcessKind::Couple.separation_semantics(), None);
        assert_eq!(ProcessKind::Decouple.family(), ProcessFamily::Separate);
        assert_eq!(
            ProcessKind::Decouple.separation_semantics(),
            Some(SeparationSemantics::NonDestructive)
        );
    }

    #[test]
    fn patch_conduit_workflow_composes_join_test_and_release_without_recipe_outputs() {
        let workflow = [
            ProcessKind::Clean,
            ProcessKind::Align,
            ProcessKind::Clamp,
            ProcessKind::Seal,
            ProcessKind::PressureTest,
            ProcessKind::ReleaseClamp,
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
                ProcessFamily::Separate,
            ]
        );
    }
}
