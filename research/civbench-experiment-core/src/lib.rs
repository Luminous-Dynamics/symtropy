// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Small, dependency-free qualification primitives for matched CivBench experiments.
//!
//! This crate does not run a hydraulic model, choose a policy, parse the public
//! experiment manifest, or authorize infrastructure operations. It validates
//! paired scenario contracts, constrains finite resource commitments, computes
//! typed paired-outcome deltas, and requires complete provenance digests.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

/// SHA-256 digest represented as 32 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

/// Error returned when parsing a hexadecimal SHA-256 digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestError {
    /// A SHA-256 digest must contain exactly 64 hexadecimal characters.
    InvalidLength,
    /// A character was not a hexadecimal digit.
    InvalidHexCharacter { index: usize },
}

impl fmt::Display for DigestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => {
                write!(f, "SHA-256 digest must contain 64 hexadecimal characters")
            }
            Self::InvalidHexCharacter { index } => {
                write!(f, "invalid hexadecimal character at byte index {index}")
            }
        }
    }
}

impl std::error::Error for DigestError {}

impl Digest {
    /// Construct a digest from its raw 32-byte representation.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Parse exactly 64 hexadecimal characters into a SHA-256 digest.
    pub fn from_hex(value: &str) -> Result<Self, DigestError> {
        let bytes = value.as_bytes();
        if bytes.len() != 64 {
            return Err(DigestError::InvalidLength);
        }

        let mut digest = [0_u8; 32];
        for (index, pair) in bytes.chunks_exact(2).enumerate() {
            let high =
                decode_hex(pair[0]).ok_or(DigestError::InvalidHexCharacter { index: index * 2 })?;
            let low = decode_hex(pair[1]).ok_or(DigestError::InvalidHexCharacter {
                index: index * 2 + 1,
            })?;
            digest[index] = (high << 4) | low;
        }
        Ok(Self(digest))
    }

    /// Return the lowercase hexadecimal representation.
    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }

    /// Return the raw 32-byte representation.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Whether this digest is all zero bytes, a common unset placeholder.
    pub const fn is_zero(&self) -> bool {
        let mut index = 0;
        while index < self.0.len() {
            if self.0[index] != 0 {
                return false;
            }
            index += 1;
        }
        true
    }
}

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

/// An intervention factor. Physical conditions are blocking strata, not interventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionFactor {
    /// Allocation and governance conditions.
    Governance,
    /// Maintenance, inspection and renewal conditions.
    Maintenance,
    /// Information quality and observability conditions.
    Information,
}

/// One assigned level of an intervention factor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorAssignment {
    /// Factor being varied.
    pub factor: InterventionFactor,
    /// Stable, versioned level identifier.
    pub level: String,
}

impl FactorAssignment {
    /// Construct a factor assignment.
    pub fn new(factor: InterventionFactor, level: impl Into<String>) -> Self {
        Self {
            factor,
            level: level.into(),
        }
    }
}

/// Resource dimensions whose available quantity is finite and ledgered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum ResourceKind {
    /// Raw water allocated to the scenario, in litres.
    WaterLitres = 0,
    /// Capital budget, in the declared smallest currency unit.
    CapitalMinor = 1,
    /// Operating budget, in the declared smallest currency unit.
    OperatingMinor = 2,
    /// Qualified crew time, in minutes.
    CrewMinutes = 3,
    /// Spare-parts inventory, in declared count units.
    PartsUnits = 4,
    /// Contractor capacity, in minutes.
    ContractorMinutes = 5,
    /// Energy, in watt-hours.
    EnergyWh = 6,
    /// Sensors available for deployment, in whole units.
    SensorUnits = 7,
}

impl ResourceKind {
    /// All resource dimensions in stable index order.
    pub const ALL: [Self; 8] = [
        Self::WaterLitres,
        Self::CapitalMinor,
        Self::OperatingMinor,
        Self::CrewMinutes,
        Self::PartsUnits,
        Self::ContractorMinutes,
        Self::EnergyWh,
        Self::SensorUnits,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// A finite vector of resources available to one scenario arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceEnvelope {
    values: [u64; 8],
}

impl ResourceEnvelope {
    /// Construct an envelope in ResourceKind::ALL order.
    pub const fn from_values(values: [u64; 8]) -> Self {
        Self { values }
    }

    /// Construct an envelope with every resource quantity set to zero.
    pub const fn zero() -> Self {
        Self { values: [0; 8] }
    }

    /// Read one resource quantity.
    pub const fn get(&self, kind: ResourceKind) -> u64 {
        self.values[kind.index()]
    }

    /// Return all quantities in ResourceKind::ALL order.
    pub const fn values(&self) -> &[u64; 8] {
        &self.values
    }
}

/// Immutable, evaluator-qualified scenario inputs shared within a paired block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioArm {
    /// Stable pair/block identifier.
    pub block_id: String,
    /// Digest of the common exogenous event schedule.
    pub exogenous_digest: Digest,
    /// Digest of the common initial physical/institutional state.
    pub initial_state_digest: Digest,
    /// Digest of the topology assigned to this block.
    pub topology_digest: Digest,
    /// Digest of the declared structural model family.
    pub model_family_digest: Digest,
    /// Digest of the common simulation time grid.
    pub time_grid_digest: Digest,
    /// Simulation horizon in ticks.
    pub horizon_ticks: u64,
    /// Intervention assignment for this arm.
    pub assignment: FactorAssignment,
    /// Finite resource envelope available to this arm.
    pub resources: ResourceEnvelope,
}

impl ScenarioArm {
    /// Construct one immutable arm description.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        block_id: impl Into<String>,
        exogenous_digest: Digest,
        initial_state_digest: Digest,
        topology_digest: Digest,
        model_family_digest: Digest,
        time_grid_digest: Digest,
        horizon_ticks: u64,
        assignment: FactorAssignment,
        resources: ResourceEnvelope,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            exogenous_digest,
            initial_state_digest,
            topology_digest,
            model_family_digest,
            time_grid_digest,
            horizon_ticks,
            assignment,
            resources,
        }
    }
}

/// Errors returned when two scenario arms violate the paired-comparison contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairError {
    /// The block identifier is empty.
    EmptyBlockId,
    /// The intervention level identifier is empty.
    EmptyFactorLevel,
    /// Arms do not belong to the same scenario block.
    BlockIdMismatch,
    /// Arms use different exogenous event schedules.
    ExogenousScheduleMismatch,
    /// Arms do not start from the same initial state.
    InitialStateMismatch,
    /// Arms use different topologies.
    TopologyMismatch,
    /// Arms use different structural model families.
    ModelFamilyMismatch,
    /// Arms use different simulation time grids.
    TimeGridMismatch,
    /// Arms use different evaluation horizons.
    HorizonMismatch,
    /// Arms vary different intervention factors.
    FactorMismatch,
    /// Control and treatment use the same factor level.
    SameFactorLevel,
    /// A resource differs without being declared as an intended treatment change.
    UndeclaredResourceDifference(ResourceKind),
    /// A required scenario digest is the all-zero unset sentinel.
    ZeroScenarioDigest,
    /// The evaluator-only truth digest is the all-zero unset sentinel.
    ZeroEvaluatorTruthDigest,
    /// The evaluation horizon must contain at least one tick.
    ZeroHorizon,
}

impl fmt::Display for PairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBlockId => write!(f, "scenario block identifier is empty"),
            Self::EmptyFactorLevel => write!(f, "intervention factor level is empty"),
            Self::BlockIdMismatch => write!(f, "paired arms use different block identifiers"),
            Self::ExogenousScheduleMismatch => {
                write!(f, "paired arms use different exogenous schedules")
            }
            Self::InitialStateMismatch => write!(f, "paired arms use different initial states"),
            Self::TopologyMismatch => write!(f, "paired arms use different topologies"),
            Self::ModelFamilyMismatch => write!(f, "paired arms use different model families"),
            Self::TimeGridMismatch => write!(f, "paired arms use different time grids"),
            Self::HorizonMismatch => write!(f, "paired arms use different horizons"),
            Self::FactorMismatch => write!(f, "paired arms vary different intervention factors"),
            Self::SameFactorLevel => write!(f, "control and treatment use the same factor level"),
            Self::UndeclaredResourceDifference(kind) => {
                write!(
                    f,
                    "resource {kind:?} differs without a declared treatment exception"
                )
            }
            Self::ZeroScenarioDigest => {
                write!(f, "required scenario digest is an all-zero unset sentinel")
            }
            Self::ZeroEvaluatorTruthDigest => write!(
                f,
                "evaluator-only truth digest is an all-zero unset sentinel"
            ),
            Self::ZeroHorizon => write!(f, "scenario evaluation horizon must be greater than zero"),
        }
    }
}

impl std::error::Error for PairError {}

/// The candidate-visible projection of a paired block.
///
/// Evaluator-only truth is deliberately not included in this type. Candidate
/// code should receive this projection, not the complete evaluator-side block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateScenarioView {
    /// Control arm.
    pub control: ScenarioArm,
    /// Treatment arm.
    pub treatment: ScenarioArm,
}

/// A validated pair plus evaluator-only truth kept outside the candidate view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedScenarioBlock {
    candidate_view: CandidateScenarioView,
    evaluator_truth_digest: Digest,
}

impl PairedScenarioBlock {
    /// Validate and construct a paired scenario block.
    ///
    /// Resource dimensions may differ only when explicitly listed in
    /// treatment_resource_changes. This exception must correspond to the
    /// registered treatment, not an accidental budget mismatch.
    pub fn new(
        control: ScenarioArm,
        treatment: ScenarioArm,
        treatment_resource_changes: &[ResourceKind],
        evaluator_truth_digest: Digest,
    ) -> Result<Self, PairError> {
        if control.block_id.trim().is_empty() || treatment.block_id.trim().is_empty() {
            return Err(PairError::EmptyBlockId);
        }
        if control.assignment.level.trim().is_empty()
            || treatment.assignment.level.trim().is_empty()
        {
            return Err(PairError::EmptyFactorLevel);
        }
        if control.horizon_ticks == 0 || treatment.horizon_ticks == 0 {
            return Err(PairError::ZeroHorizon);
        }
        let scenario_digests = [
            control.exogenous_digest,
            control.initial_state_digest,
            control.topology_digest,
            control.model_family_digest,
            control.time_grid_digest,
            treatment.exogenous_digest,
            treatment.initial_state_digest,
            treatment.topology_digest,
            treatment.model_family_digest,
            treatment.time_grid_digest,
        ];
        if scenario_digests.iter().any(Digest::is_zero) {
            return Err(PairError::ZeroScenarioDigest);
        }
        if evaluator_truth_digest.is_zero() {
            return Err(PairError::ZeroEvaluatorTruthDigest);
        }
        if control.block_id != treatment.block_id {
            return Err(PairError::BlockIdMismatch);
        }
        if control.exogenous_digest != treatment.exogenous_digest {
            return Err(PairError::ExogenousScheduleMismatch);
        }
        if control.initial_state_digest != treatment.initial_state_digest {
            return Err(PairError::InitialStateMismatch);
        }
        if control.topology_digest != treatment.topology_digest {
            return Err(PairError::TopologyMismatch);
        }
        if control.model_family_digest != treatment.model_family_digest {
            return Err(PairError::ModelFamilyMismatch);
        }
        if control.time_grid_digest != treatment.time_grid_digest {
            return Err(PairError::TimeGridMismatch);
        }
        if control.horizon_ticks != treatment.horizon_ticks {
            return Err(PairError::HorizonMismatch);
        }
        if control.assignment.factor != treatment.assignment.factor {
            return Err(PairError::FactorMismatch);
        }
        if control.assignment.level == treatment.assignment.level {
            return Err(PairError::SameFactorLevel);
        }

        for kind in ResourceKind::ALL {
            let declared_change = treatment_resource_changes.contains(&kind);
            if !declared_change && control.resources.get(kind) != treatment.resources.get(kind) {
                return Err(PairError::UndeclaredResourceDifference(kind));
            }
        }

        Ok(Self {
            candidate_view: CandidateScenarioView { control, treatment },
            evaluator_truth_digest,
        })
    }

    /// Return the candidate-visible scenario projection, without evaluator truth.
    pub const fn candidate_view(&self) -> &CandidateScenarioView {
        &self.candidate_view
    }

    /// Return the evaluator-only truth digest for the independent evaluator.
    ///
    /// Do not pass this value or the complete PairedScenarioBlock to candidate
    /// policy code.
    pub const fn evaluator_truth_digest(&self) -> Digest {
        self.evaluator_truth_digest
    }
}

/// Lifecycle state of one resource commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceCommitmentState {
    /// Capacity has been reserved for planned work but is not yet realized.
    Reserved,
    /// Work has been settled and actual resource consumption recorded.
    Settled,
    /// Planned work was cancelled and its reservation released.
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceCommitmentRecord {
    reserved: ResourceEnvelope,
    settled: Option<ResourceEnvelope>,
    state: ResourceCommitmentState,
}

/// Error returned when a resource commitment is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// Commitment identifiers must be non-empty.
    EmptyCommitmentId,
    /// A commitment identifier was already recorded.
    DuplicateCommitment(String),
    /// Adding the commitment would exceed a resource envelope.
    ExceedsAvailable {
        /// Resource dimension that would be exceeded.
        kind: ResourceKind,
        /// Available quantity.
        available: u64,
        /// Already reserved or consumed quantity.
        committed: u64,
        /// New requested quantity.
        requested: u64,
    },
    /// Actual settlement exceeds the reservation in a resource dimension.
    ActualExceedsReservation {
        /// Resource dimension that would exceed its reservation.
        kind: ResourceKind,
        /// Reserved quantity.
        reserved: u64,
        /// Actual quantity requested for settlement.
        actual: u64,
    },
    /// Adding resource quantities overflowed the integer representation.
    ArithmeticOverflow(ResourceKind),
    /// The requested commitment identifier does not exist.
    UnknownCommitment(String),
    /// A commitment cannot be transitioned from its current state.
    InvalidTransition {
        /// Commitment identifier.
        id: String,
        /// Existing lifecycle state.
        state: ResourceCommitmentState,
    },
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommitmentId => write!(f, "resource commitment identifier is empty"),
            Self::DuplicateCommitment(id) => write!(f, "duplicate resource commitment: {id}"),
            Self::ExceedsAvailable {
                kind,
                available,
                committed,
                requested,
            } => write!(
                f,
                "resource {kind:?} exceeds available envelope: available={available}, committed={committed}, requested={requested}"
            ),
            Self::ActualExceedsReservation {
                kind,
                reserved,
                actual,
            } => write!(
                f,
                "actual resource {kind:?} exceeds reservation: reserved={reserved}, actual={actual}"
            ),
            Self::ArithmeticOverflow(kind) => {
                write!(f, "resource arithmetic overflow for {kind:?}")
            }
            Self::UnknownCommitment(id) => write!(f, "unknown resource commitment: {id}"),
            Self::InvalidTransition { id, state } => {
                write!(
                    f,
                    "resource commitment {id} cannot transition from {state:?}"
                )
            }
        }
    }
}

impl std::error::Error for LedgerError {}

/// A deterministic resource ledger for one policy arm.
///
/// Available capacity is split into reserved, consumed and remaining buckets.
/// A reservation can be settled once or released once. Settling records actual
/// consumption, which may be lower than reserved but never higher. IDs remain
/// recorded after settlement/release to prevent accidental reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLedger {
    available: ResourceEnvelope,
    reserved: ResourceEnvelope,
    consumed: ResourceEnvelope,
    commitments: BTreeMap<String, ResourceCommitmentRecord>,
}

impl ResourceLedger {
    /// Create a ledger with a finite available-resource envelope.
    pub fn new(available: ResourceEnvelope) -> Self {
        Self {
            available,
            reserved: ResourceEnvelope::zero(),
            consumed: ResourceEnvelope::zero(),
            commitments: BTreeMap::new(),
        }
    }

    /// Return the original available resource envelope.
    pub const fn available(&self) -> ResourceEnvelope {
        self.available
    }

    /// Return resources reserved for planned but unsettled work.
    pub const fn reserved(&self) -> ResourceEnvelope {
        self.reserved
    }

    /// Return actual resource consumption recorded by settled commitments.
    pub const fn consumed(&self) -> ResourceEnvelope {
        self.consumed
    }

    /// Return reserved plus consumed resources.
    pub fn committed(&self) -> ResourceEnvelope {
        let mut values = [0_u64; 8];
        for kind in ResourceKind::ALL {
            values[kind.index()] = self.reserved.get(kind) + self.consumed.get(kind);
        }
        ResourceEnvelope::from_values(values)
    }

    /// Return unreserved and unconsumed resource capacity.
    pub fn remaining(&self) -> ResourceEnvelope {
        let committed = self.committed();
        let mut values = [0_u64; 8];
        for kind in ResourceKind::ALL {
            values[kind.index()] = self.available.get(kind) - committed.get(kind);
        }
        ResourceEnvelope::from_values(values)
    }

    /// Return the number of unique commitment identifiers ever recorded.
    pub fn commitment_count(&self) -> usize {
        self.commitments.len()
    }

    /// Return one commitment's lifecycle state, if it exists.
    pub fn commitment_state(&self, commitment_id: &str) -> Option<ResourceCommitmentState> {
        self.commitments
            .get(commitment_id.trim())
            .map(|record| record.state)
    }

    /// Return the original reservation for a recorded commitment.
    pub fn commitment_reservation(&self, commitment_id: &str) -> Option<ResourceEnvelope> {
        self.commitments
            .get(commitment_id.trim())
            .map(|record| record.reserved)
    }

    /// Return actual settled resource consumption for a completed commitment.
    ///
    /// Returns None when the commitment is unknown, still reserved, or released.
    pub fn commitment_settlement(&self, commitment_id: &str) -> Option<ResourceEnvelope> {
        self.commitments
            .get(commitment_id.trim())
            .and_then(|record| record.settled)
    }

    fn new_commitment_id(&self, commitment_id: impl Into<String>) -> Result<String, LedgerError> {
        let id = commitment_id.into().trim().to_owned();
        if id.is_empty() {
            return Err(LedgerError::EmptyCommitmentId);
        }
        if self.commitments.contains_key(&id) {
            return Err(LedgerError::DuplicateCommitment(id));
        }
        Ok(id)
    }

    /// Reserve a resource bundle atomically for planned work.
    ///
    /// Reserved resources reduce remaining capacity but are not reported as consumed.
    pub fn reserve(
        &mut self,
        commitment_id: impl Into<String>,
        request: ResourceEnvelope,
    ) -> Result<(), LedgerError> {
        let id = self.new_commitment_id(commitment_id)?;
        let remaining = self.remaining();
        let mut next_values = *self.reserved.values();

        for kind in ResourceKind::ALL {
            let index = kind.index();
            let reserved = self.reserved.get(kind);
            let requested = request.get(kind);
            let next = reserved
                .checked_add(requested)
                .ok_or(LedgerError::ArithmeticOverflow(kind))?;
            if requested > remaining.get(kind) {
                return Err(LedgerError::ExceedsAvailable {
                    kind,
                    available: self.available.get(kind),
                    committed: self.committed().get(kind),
                    requested,
                });
            }
            next_values[index] = next;
        }

        self.reserved = ResourceEnvelope::from_values(next_values);
        self.commitments.insert(
            id,
            ResourceCommitmentRecord {
                reserved: request,
                settled: None,
                state: ResourceCommitmentState::Reserved,
            },
        );
        Ok(())
    }

    /// Settle a prior reservation using actual resource consumption.
    ///
    /// Actual consumption may be lower than the reservation, returning unused
    /// capacity to remaining. Overruns and invalid transitions leave state unchanged.
    pub fn settle(
        &mut self,
        commitment_id: &str,
        actual: ResourceEnvelope,
    ) -> Result<(), LedgerError> {
        let id = commitment_id.trim();
        let record = self
            .commitments
            .get(id)
            .copied()
            .ok_or_else(|| LedgerError::UnknownCommitment(id.to_owned()))?;
        if record.state != ResourceCommitmentState::Reserved {
            return Err(LedgerError::InvalidTransition {
                id: id.to_owned(),
                state: record.state,
            });
        }

        let mut next_reserved = *self.reserved.values();
        let mut next_consumed = *self.consumed.values();
        for kind in ResourceKind::ALL {
            let index = kind.index();
            let reserved = record.reserved.get(kind);
            let actual_value = actual.get(kind);
            if actual_value > reserved {
                return Err(LedgerError::ActualExceedsReservation {
                    kind,
                    reserved,
                    actual: actual_value,
                });
            }
            next_reserved[index] = self.reserved.get(kind) - reserved;
            next_consumed[index] = self
                .consumed
                .get(kind)
                .checked_add(actual_value)
                .ok_or(LedgerError::ArithmeticOverflow(kind))?;
        }

        self.reserved = ResourceEnvelope::from_values(next_reserved);
        self.consumed = ResourceEnvelope::from_values(next_consumed);
        self.commitments.insert(
            id.to_owned(),
            ResourceCommitmentRecord {
                reserved: record.reserved,
                settled: Some(actual),
                state: ResourceCommitmentState::Settled,
            },
        );
        Ok(())
    }

    /// Release a reservation for work that was cancelled or not undertaken.
    pub fn release(&mut self, commitment_id: &str) -> Result<(), LedgerError> {
        let id = commitment_id.trim();
        let record = self
            .commitments
            .get(id)
            .copied()
            .ok_or_else(|| LedgerError::UnknownCommitment(id.to_owned()))?;
        if record.state != ResourceCommitmentState::Reserved {
            return Err(LedgerError::InvalidTransition {
                id: id.to_owned(),
                state: record.state,
            });
        }

        let mut next_reserved = *self.reserved.values();
        for kind in ResourceKind::ALL {
            next_reserved[kind.index()] = self.reserved.get(kind) - record.reserved.get(kind);
        }
        self.reserved = ResourceEnvelope::from_values(next_reserved);
        self.commitments.insert(
            id.to_owned(),
            ResourceCommitmentRecord {
                reserved: record.reserved,
                settled: None,
                state: ResourceCommitmentState::Released,
            },
        );
        Ok(())
    }

    /// Directly record a resource bundle as already consumed.
    ///
    /// This is an immediate settlement for known realized costs without a reservation.
    pub fn commit(
        &mut self,
        commitment_id: impl Into<String>,
        actual: ResourceEnvelope,
    ) -> Result<(), LedgerError> {
        let id = self.new_commitment_id(commitment_id)?;
        let remaining = self.remaining();
        let mut next_values = *self.consumed.values();

        for kind in ResourceKind::ALL {
            let index = kind.index();
            let consumed = self.consumed.get(kind);
            let requested = actual.get(kind);
            let next = consumed
                .checked_add(requested)
                .ok_or(LedgerError::ArithmeticOverflow(kind))?;
            if requested > remaining.get(kind) {
                return Err(LedgerError::ExceedsAvailable {
                    kind,
                    available: self.available.get(kind),
                    committed: self.committed().get(kind),
                    requested,
                });
            }
            next_values[index] = next;
        }

        self.consumed = ResourceEnvelope::from_values(next_values);
        self.commitments.insert(
            id,
            ResourceCommitmentRecord {
                reserved: ResourceEnvelope::zero(),
                settled: Some(actual),
                state: ResourceCommitmentState::Settled,
            },
        );
        Ok(())
    }
}

/// Evidence status for an outcome metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricEvidenceStatus {
    /// Measured directly from a declared observation source.
    Observed,
    /// Calculated by reproducible arithmetic from source-qualified values.
    Derived,
    /// Produced by a declared simulation or model.
    Modelled,
    /// No measurement is available for this case.
    NotMeasured,
    /// The metric is outside the qualified capability of this model/evaluator.
    Unsupported,
    /// The value exists but is withheld under a data-governance constraint.
    Withheld,
    /// The metric does not apply to this scenario.
    NotApplicable,
}

/// A numeric metric paired with an explicit evidence status.
///
/// Unknown, unsupported, withheld and not-applicable values are represented
/// with None; they are never silently converted to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualifiedMetric {
    /// Numeric value in the unit declared by the parent outcome field.
    pub value: Option<u64>,
    /// Evidence state explaining whether the value is available.
    pub status: MetricEvidenceStatus,
}

impl QualifiedMetric {
    /// Construct a directly observed metric.
    pub const fn observed(value: u64) -> Self {
        Self {
            value: Some(value),
            status: MetricEvidenceStatus::Observed,
        }
    }

    /// Construct a derived metric.
    pub const fn derived(value: u64) -> Self {
        Self {
            value: Some(value),
            status: MetricEvidenceStatus::Derived,
        }
    }

    /// Construct a modelled metric.
    pub const fn modelled(value: u64) -> Self {
        Self {
            value: Some(value),
            status: MetricEvidenceStatus::Modelled,
        }
    }

    /// Construct a metric that has not been measured.
    pub const fn not_measured() -> Self {
        Self {
            value: None,
            status: MetricEvidenceStatus::NotMeasured,
        }
    }

    /// Construct a metric unsupported by this model/evaluator.
    pub const fn unsupported() -> Self {
        Self {
            value: None,
            status: MetricEvidenceStatus::Unsupported,
        }
    }

    /// Construct a metric withheld by a data-governance constraint.
    pub const fn withheld() -> Self {
        Self {
            value: None,
            status: MetricEvidenceStatus::Withheld,
        }
    }

    /// Construct a metric that does not apply to this scenario.
    pub const fn not_applicable() -> Self {
        Self {
            value: None,
            status: MetricEvidenceStatus::NotApplicable,
        }
    }

    fn validate(self, field: &'static str) -> Result<(), OutcomeError> {
        let requires_value = matches!(
            self.status,
            MetricEvidenceStatus::Observed
                | MetricEvidenceStatus::Derived
                | MetricEvidenceStatus::Modelled
        );
        if self.value.is_some() != requires_value {
            return Err(OutcomeError::StatusValueMismatch { field });
        }
        Ok(())
    }

    fn paired_delta(self, control: Self) -> MetricDelta {
        match (control.value, self.value) {
            (None, None) => MetricDelta {
                value: None,
                status: MetricDeltaStatus::BothMissing {
                    control: control.status,
                    treatment: self.status,
                },
            },
            (None, Some(_)) => MetricDelta {
                value: None,
                status: MetricDeltaStatus::MissingControl {
                    status: control.status,
                },
            },
            (Some(_), None) => MetricDelta {
                value: None,
                status: MetricDeltaStatus::MissingTreatment {
                    status: self.status,
                },
            },
            (Some(_), Some(_)) if control.status != self.status => MetricDelta {
                value: None,
                status: MetricDeltaStatus::EvidenceStatusMismatch {
                    control: control.status,
                    treatment: self.status,
                },
            },
            (Some(control_value), Some(treatment_value)) => MetricDelta {
                value: Some(i128::from(treatment_value) - i128::from(control_value)),
                status: MetricDeltaStatus::Comparable {
                    evidence_status: self.status,
                },
            },
        }
    }
}

impl Default for QualifiedMetric {
    fn default() -> Self {
        Self::not_measured()
    }
}

/// Qualification state of a treatment-minus-control metric comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricDeltaStatus {
    /// Both numeric values exist and carry the same evidence status.
    Comparable {
        /// Shared evidence status of the two values.
        evidence_status: MetricEvidenceStatus,
    },
    /// The control value is unavailable.
    MissingControl {
        /// Why the control value is missing.
        status: MetricEvidenceStatus,
    },
    /// The treatment value is unavailable.
    MissingTreatment {
        /// Why the treatment value is missing.
        status: MetricEvidenceStatus,
    },
    /// Neither arm has a numeric value.
    BothMissing {
        /// Control-side missingness reason.
        control: MetricEvidenceStatus,
        /// Treatment-side missingness reason.
        treatment: MetricEvidenceStatus,
    },
    /// Both values exist but their evidence classes are not comparable in v0.1.
    EvidenceStatusMismatch {
        /// Control-side evidence class.
        control: MetricEvidenceStatus,
        /// Treatment-side evidence class.
        treatment: MetricEvidenceStatus,
    },
}

/// One evidence-qualified metric delta.
///
/// The numeric delta is present only when both arms have values with exactly
/// the same evidence status. Cross-status conversions require a separately
/// preregistered and versioned comparison bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricDelta {
    /// Signed treatment-minus-control difference, when comparable.
    pub value: Option<i128>,
    /// Explicit reason the delta is or is not comparable.
    pub status: MetricDeltaStatus,
}

/// Typed, non-composite outcome vector from one evaluated scenario arm.
///
/// Numeric units are carried by field names and the protocol. Missing values
/// preserve an explicit status and cannot be mistaken for measured zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutcomeVector {
    /// Unmet essential water demand, in litres.
    pub essential_shortfall_l: QualifiedMetric,
    /// Service interruption exposure, in service-unit seconds.
    pub interruption_service_seconds: QualifiedMetric,
    /// Worst-group service performance, in parts per million (0..=1,000,000).
    pub worst_group_service_ppm: QualifiedMetric,
    /// Physical water loss, in litres.
    pub physical_loss_l: QualifiedMetric,
    /// Commercial/apparent loss, in litres.
    pub commercial_loss_l: QualifiedMetric,
    /// Unbilled authorised consumption, in litres.
    pub unbilled_authorised_l: QualifiedMetric,
    /// Asset failures in the declared observation window.
    pub asset_failures: QualifiedMetric,
    /// Independently verified completed repairs.
    pub verified_repairs: QualifiedMetric,
    /// Whole-life cost, in the declared smallest currency unit and price year.
    pub whole_life_cost_minor: QualifiedMetric,
    /// Time to qualified recovery, in seconds.
    pub recovery_seconds: QualifiedMetric,
    /// Qualified fallback water delivered, in litres.
    pub safe_fallback_delivery_l: QualifiedMetric,
    /// False dispatches in the declared observation window.
    pub false_dispatches: QualifiedMetric,
}

/// Error returned when an outcome vector violates a declared value domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeError {
    /// A metric's evidence status and numeric value presence do not agree.
    StatusValueMismatch {
        /// Outcome field whose status/value combination is invalid.
        field: &'static str,
    },
    /// Worst-group service performance must be between zero and one million ppm.
    WorstGroupServiceOutOfRange {
        /// Invalid value.
        value: u64,
    },
}

impl fmt::Display for OutcomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatusValueMismatch { field } => {
                write!(
                    f,
                    "outcome field {field} has an invalid status/value combination"
                )
            }
            Self::WorstGroupServiceOutOfRange { value } => write!(
                f,
                "worst-group service value {value} ppm exceeds the 0..=1000000 domain"
            ),
        }
    }
}

impl std::error::Error for OutcomeError {}

impl OutcomeVector {
    /// Validate status/value consistency and bounded service performance.
    pub fn validate(&self) -> Result<(), OutcomeError> {
        self.essential_shortfall_l
            .validate("essential_shortfall_l")?;
        self.interruption_service_seconds
            .validate("interruption_service_seconds")?;
        self.worst_group_service_ppm
            .validate("worst_group_service_ppm")?;
        self.physical_loss_l.validate("physical_loss_l")?;
        self.commercial_loss_l.validate("commercial_loss_l")?;
        self.unbilled_authorised_l
            .validate("unbilled_authorised_l")?;
        self.asset_failures.validate("asset_failures")?;
        self.verified_repairs.validate("verified_repairs")?;
        self.whole_life_cost_minor
            .validate("whole_life_cost_minor")?;
        self.recovery_seconds.validate("recovery_seconds")?;
        self.safe_fallback_delivery_l
            .validate("safe_fallback_delivery_l")?;
        self.false_dispatches.validate("false_dispatches")?;

        if let Some(value) = self.worst_group_service_ppm.value {
            if value > 1_000_000 {
                return Err(OutcomeError::WorstGroupServiceOutOfRange { value });
            }
        }
        Ok(())
    }
}

/// Signed, optional treatment-minus-control outcome differences.
///
/// A field is None when either arm lacks a valid numeric value. This prevents
/// missingness from being silently interpreted as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutcomeDelta {
    /// Change in unmet essential water demand, in litres.
    pub essential_shortfall_l: MetricDelta,
    /// Change in service interruption exposure, in service-unit seconds.
    pub interruption_service_seconds: MetricDelta,
    /// Change in worst-group service performance, in parts per million.
    pub worst_group_service_ppm: MetricDelta,
    /// Change in physical water loss, in litres.
    pub physical_loss_l: MetricDelta,
    /// Change in commercial/apparent loss, in litres.
    pub commercial_loss_l: MetricDelta,
    /// Change in unbilled authorised consumption, in litres.
    pub unbilled_authorised_l: MetricDelta,
    /// Change in asset failures.
    pub asset_failures: MetricDelta,
    /// Change in independently verified completed repairs.
    pub verified_repairs: MetricDelta,
    /// Change in whole-life cost, in the declared smallest currency unit.
    pub whole_life_cost_minor: MetricDelta,
    /// Change in time to qualified recovery, in seconds.
    pub recovery_seconds: MetricDelta,
    /// Change in qualified fallback water delivered, in litres.
    pub safe_fallback_delivery_l: MetricDelta,
    /// Change in false dispatches.
    pub false_dispatches: MetricDelta,
}

impl OutcomeDelta {
    /// Compute optional treatment-minus-control differences for every outcome.
    fn between(control: OutcomeVector, treatment: OutcomeVector) -> Self {
        Self {
            essential_shortfall_l: treatment
                .essential_shortfall_l
                .paired_delta(control.essential_shortfall_l),
            interruption_service_seconds: treatment
                .interruption_service_seconds
                .paired_delta(control.interruption_service_seconds),
            worst_group_service_ppm: treatment
                .worst_group_service_ppm
                .paired_delta(control.worst_group_service_ppm),
            physical_loss_l: treatment
                .physical_loss_l
                .paired_delta(control.physical_loss_l),
            commercial_loss_l: treatment
                .commercial_loss_l
                .paired_delta(control.commercial_loss_l),
            unbilled_authorised_l: treatment
                .unbilled_authorised_l
                .paired_delta(control.unbilled_authorised_l),
            asset_failures: treatment
                .asset_failures
                .paired_delta(control.asset_failures),
            verified_repairs: treatment
                .verified_repairs
                .paired_delta(control.verified_repairs),
            whole_life_cost_minor: treatment
                .whole_life_cost_minor
                .paired_delta(control.whole_life_cost_minor),
            recovery_seconds: treatment
                .recovery_seconds
                .paired_delta(control.recovery_seconds),
            safe_fallback_delivery_l: treatment
                .safe_fallback_delivery_l
                .paired_delta(control.safe_fallback_delivery_l),
            false_dispatches: treatment
                .false_dispatches
                .paired_delta(control.false_dispatches),
        }
    }
}

/// A paired, component-wise outcome receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedOutcomeReceipt {
    /// Control outcome vector.
    pub control: OutcomeVector,
    /// Treatment outcome vector.
    pub treatment: OutcomeVector,
    /// Optional signed treatment-minus-control differences.
    pub delta: OutcomeDelta,
}

impl PairedOutcomeReceipt {
    /// Construct a receipt after validating both outcome vectors.
    pub fn new(control: OutcomeVector, treatment: OutcomeVector) -> Result<Self, OutcomeError> {
        control.validate()?;
        treatment.validate()?;
        Ok(Self {
            control,
            treatment,
            delta: OutcomeDelta::between(control, treatment),
        })
    }
}

/// Required content digests for an independently reproducible experiment receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvenanceDigests {
    /// Protocol specification digest.
    pub protocol: Digest,
    /// Machine-readable protocol schema digest.
    pub schema: Digest,
    /// Source inventory digest.
    pub source_inventory: Digest,
    /// Frozen source snapshot collection digest.
    pub source_snapshots: Digest,
    /// Model implementation/configuration digest.
    pub model: Digest,
    /// Topology digest.
    pub topology: Digest,
    /// Scenario generator digest.
    pub scenario_generator: Digest,
    /// Exact scenario input digest.
    pub scenario: Digest,
    /// Policy implementation/configuration digest.
    pub policy: Digest,
    /// Independent evaluator implementation/configuration digest.
    pub evaluator: Digest,
    /// Output artifact digest.
    pub output: Digest,
}

/// Execution identity required to bind a provenance receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionIdentity {
    /// Full source commit identifier.
    pub code_commit: String,
    /// Solver/runtime version string.
    pub solver_version: String,
    /// Runtime environment digest.
    pub runtime_environment: Digest,
}

/// Whether source bytes have been frozen and content-hash verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSnapshotStatus {
    /// Source bytes were captured and their digest recorded.
    Frozen,
    /// One or more required source bytes have not been snapshotted.
    NotSnapshotted,
}

/// A fail-closed provenance receipt.
///
/// Digest is a fixed-size type, so a missing or malformed digest cannot be
/// represented. Source snapshot digest is mandatory; a source marked
/// not_snapshotted cannot produce a qualified confirmatory receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceReceipt {
    /// Required artifact/input digests.
    pub digests: ProvenanceDigests,
    /// Execution identity.
    pub execution: ExecutionIdentity,
}

/// Errors returned when execution identity cannot qualify a receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptError {
    /// The exact source commit identifier is empty.
    EmptyCodeCommit,
    /// The code commit must be a 40- or 64-character hexadecimal object ID.
    InvalidCodeCommit,
    /// The solver/runtime version is empty.
    EmptySolverVersion,
    /// A required digest is an all-zero unset sentinel.
    ZeroDigest,
    /// Confirmatory receipts require frozen source bytes.
    SourceSnapshotNotFrozen,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCodeCommit => write!(f, "exact code commit identifier is required"),
            Self::InvalidCodeCommit => write!(
                f,
                "code commit must be a 40- or 64-character hexadecimal object ID"
            ),
            Self::EmptySolverVersion => write!(f, "solver/runtime version is required"),
            Self::ZeroDigest => write!(
                f,
                "required provenance digest is an all-zero unset sentinel"
            ),
            Self::SourceSnapshotNotFrozen => write!(
                f,
                "source bytes must be snapshotted before a confirmatory receipt can be created"
            ),
        }
    }
}

impl std::error::Error for ReceiptError {}

impl ProvenanceReceipt {
    /// Construct a receipt only when its execution identity is complete.
    pub fn new(
        digests: ProvenanceDigests,
        execution: ExecutionIdentity,
        source_snapshot_status: SourceSnapshotStatus,
    ) -> Result<Self, ReceiptError> {
        if source_snapshot_status != SourceSnapshotStatus::Frozen {
            return Err(ReceiptError::SourceSnapshotNotFrozen);
        }
        let required_digests = [
            digests.protocol,
            digests.schema,
            digests.source_inventory,
            digests.source_snapshots,
            digests.model,
            digests.topology,
            digests.scenario_generator,
            digests.scenario,
            digests.policy,
            digests.evaluator,
            digests.output,
            execution.runtime_environment,
        ];
        if required_digests.iter().any(Digest::is_zero) {
            return Err(ReceiptError::ZeroDigest);
        }
        if execution.code_commit.trim().is_empty() {
            return Err(ReceiptError::EmptyCodeCommit);
        }
        if !matches!(execution.code_commit.len(), 40 | 64)
            || !execution
                .code_commit
                .bytes()
                .all(|byte| decode_hex(byte).is_some())
        {
            return Err(ReceiptError::InvalidCodeCommit);
        }
        if execution.solver_version.trim().is_empty() {
            return Err(ReceiptError::EmptySolverVersion);
        }
        Ok(Self { digests, execution })
    }
}

/// Ordered stages in a decision-to-service evidence chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStage {
    /// Dated, source-qualified observation or sensor sample.
    Observation,
    /// State estimate derived from observations and a pinned model.
    StateEstimate,
    /// Recommendation with declared scope and uncertainty.
    Recommendation,
    /// Human decision to authorize, reject or defer the recommendation.
    HumanDecision,
    /// Issued operational work order or authorized action record.
    WorkOrder,
    /// Independent verification of completed field work.
    IndependentVerification,
    /// Measured service outcome linked to the verified action.
    ServiceOutcome,
    /// Post-event model discrepancy and residual review.
    ModelDiscrepancyReview,
}

/// Human disposition recorded before any operational action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanDecisionDisposition {
    /// A named human authority authorized the proposed action.
    Authorized,
    /// The recommendation was rejected.
    Rejected,
    /// The recommendation was deferred.
    Deferred,
}

/// Terminal status of a validated decision-to-service chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionServiceStatus {
    /// The human decision rejected or deferred the recommendation; no action follows.
    NotAuthorized,
    /// Authorized action, independent verification, measured outcome and model review exist.
    VerifiedServiceOutcome,
}

/// One immutable event in a decision-to-service evidence chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceEvent {
    /// Unique event identifier.
    pub event_id: String,
    /// Event type in the required chain.
    pub stage: EvidenceStage,
    /// Event time as Unix epoch milliseconds.
    pub occurred_at_unix_ms: u64,
    /// Immediate predecessor event identifier; absent only for the root observation.
    pub predecessor_event_id: Option<String>,
    /// Digest of the event evidence payload.
    pub evidence_digest: Digest,
    /// Present only on a human-decision event.
    pub disposition: Option<HumanDecisionDisposition>,
}

impl EvidenceEvent {
    /// Construct one event in the evidence chain.
    pub fn new(
        event_id: impl Into<String>,
        stage: EvidenceStage,
        occurred_at_unix_ms: u64,
        predecessor_event_id: Option<String>,
        evidence_digest: Digest,
        disposition: Option<HumanDecisionDisposition>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            stage,
            occurred_at_unix_ms,
            predecessor_event_id,
            evidence_digest,
            disposition,
        }
    }
}

/// Errors returned when a decision-to-service chain is incomplete or incoherent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionChainError {
    /// An event identifier is empty.
    EmptyEventId,
    /// An event identifier is duplicated.
    DuplicateEventId(String),
    /// An event evidence digest is an all-zero unset sentinel.
    ZeroEvidenceDigest(String),
    /// Event timestamps move backwards.
    TimestampRegression {
        /// Earlier event timestamp.
        previous_ms: u64,
        /// Later list entry timestamp.
        current_ms: u64,
    },
    /// Root observation unexpectedly declares a predecessor.
    RootHasPredecessor,
    /// An event does not link to its immediate predecessor.
    BrokenLineage(String),
    /// The required decision or closure chain is incomplete.
    IncompleteChain,
    /// Event stage is not the expected stage at this position.
    InvalidStageOrder {
        /// Zero-based event position.
        index: usize,
    },
    /// Human decision event lacks a disposition.
    MissingHumanDisposition,
    /// A non-decision event incorrectly carries a human disposition.
    UnexpectedHumanDisposition(String),
    /// A rejected/deferred recommendation is followed by an action or claimed outcome.
    ActionAfterNonAuthorization,
}

impl fmt::Display for DecisionChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEventId => write!(f, "evidence event identifier is empty"),
            Self::DuplicateEventId(id) => write!(f, "duplicate evidence event identifier: {id}"),
            Self::ZeroEvidenceDigest(id) => write!(f, "event {id} has an all-zero evidence digest"),
            Self::TimestampRegression {
                previous_ms,
                current_ms,
            } => write!(
                f,
                "event time regressed from {previous_ms} ms to {current_ms} ms"
            ),
            Self::RootHasPredecessor => write!(f, "root observation must not have a predecessor"),
            Self::BrokenLineage(id) => {
                write!(f, "event {id} does not link to its immediate predecessor")
            }
            Self::IncompleteChain => write!(f, "decision-to-service evidence chain is incomplete"),
            Self::InvalidStageOrder { index } => {
                write!(f, "invalid evidence stage at chain position {index}")
            }
            Self::MissingHumanDisposition => {
                write!(f, "human decision event lacks an explicit disposition")
            }
            Self::UnexpectedHumanDisposition(id) => {
                write!(
                    f,
                    "non-decision event {id} carries a human decision disposition"
                )
            }
            Self::ActionAfterNonAuthorization => {
                write!(
                    f,
                    "rejected or deferred recommendation cannot be followed by an operational action"
                )
            }
        }
    }
}

impl std::error::Error for DecisionChainError {}

/// A validated decision-to-service evidence chain.
///
/// A rejected or deferred recommendation is a valid terminal decision, but
/// it is not a verified service intervention. An M4 closure requires all
/// eight stages through post-event model discrepancy review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionServiceChain {
    events: Vec<EvidenceEvent>,
    status: DecisionServiceStatus,
}

impl DecisionServiceChain {
    /// Validate a linear event chain and classify its terminal status.
    ///
    /// This v0.1 validator supports one event per stage. A future branching or
    /// multi-observation event graph needs a separately versioned schema.
    pub fn new(mut events: Vec<EvidenceEvent>) -> Result<Self, DecisionChainError> {
        if events.len() < 4 {
            return Err(DecisionChainError::IncompleteChain);
        }

        let mut identifiers = BTreeSet::new();
        for event in &mut events {
            event.event_id = event.event_id.trim().to_owned();
            if event.event_id.is_empty() {
                return Err(DecisionChainError::EmptyEventId);
            }
            if !identifiers.insert(event.event_id.clone()) {
                return Err(DecisionChainError::DuplicateEventId(event.event_id.clone()));
            }
            if event.evidence_digest.is_zero() {
                return Err(DecisionChainError::ZeroEvidenceDigest(
                    event.event_id.clone(),
                ));
            }
        }

        if events[0].stage != EvidenceStage::Observation {
            return Err(DecisionChainError::InvalidStageOrder { index: 0 });
        }
        if events[0].predecessor_event_id.is_some() {
            return Err(DecisionChainError::RootHasPredecessor);
        }
        if events[0].disposition.is_some() {
            return Err(DecisionChainError::UnexpectedHumanDisposition(
                events[0].event_id.clone(),
            ));
        }

        for pair in events.windows(2) {
            let previous = &pair[0];
            let current = &pair[1];
            if current.occurred_at_unix_ms < previous.occurred_at_unix_ms {
                return Err(DecisionChainError::TimestampRegression {
                    previous_ms: previous.occurred_at_unix_ms,
                    current_ms: current.occurred_at_unix_ms,
                });
            }
            if current.predecessor_event_id.as_deref() != Some(previous.event_id.as_str()) {
                return Err(DecisionChainError::BrokenLineage(current.event_id.clone()));
            }
            if current.stage == EvidenceStage::HumanDecision {
                if current.disposition.is_none() {
                    return Err(DecisionChainError::MissingHumanDisposition);
                }
            } else if current.disposition.is_some() {
                return Err(DecisionChainError::UnexpectedHumanDisposition(
                    current.event_id.clone(),
                ));
            }
        }

        let base_stages = [
            EvidenceStage::Observation,
            EvidenceStage::StateEstimate,
            EvidenceStage::Recommendation,
            EvidenceStage::HumanDecision,
        ];
        for (index, expected) in base_stages.iter().enumerate() {
            if events[index].stage != *expected {
                return Err(DecisionChainError::InvalidStageOrder { index });
            }
        }

        let disposition = events[3]
            .disposition
            .ok_or(DecisionChainError::MissingHumanDisposition)?;
        if disposition != HumanDecisionDisposition::Authorized {
            if events.len() != 4 {
                return Err(DecisionChainError::ActionAfterNonAuthorization);
            }
            return Ok(Self {
                events,
                status: DecisionServiceStatus::NotAuthorized,
            });
        }

        let complete_stages = [
            EvidenceStage::Observation,
            EvidenceStage::StateEstimate,
            EvidenceStage::Recommendation,
            EvidenceStage::HumanDecision,
            EvidenceStage::WorkOrder,
            EvidenceStage::IndependentVerification,
            EvidenceStage::ServiceOutcome,
            EvidenceStage::ModelDiscrepancyReview,
        ];
        if events.len() != complete_stages.len() {
            return Err(DecisionChainError::IncompleteChain);
        }
        for (index, expected) in complete_stages.iter().enumerate() {
            if events[index].stage != *expected {
                return Err(DecisionChainError::InvalidStageOrder { index });
            }
        }

        Ok(Self {
            events,
            status: DecisionServiceStatus::VerifiedServiceOutcome,
        })
    }

    /// Return the validated event list.
    pub fn events(&self) -> &[EvidenceEvent] {
        &self.events
    }

    /// Return the validated terminal status.
    pub const fn status(&self) -> DecisionServiceStatus {
        self.status
    }
}

/// One dataset registered in the machine-readable leak benchmark contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeakDatasetDescriptor {
    /// Stable dataset identifier.
    pub dataset_id: String,
    /// Network topology represented by the dataset.
    pub topology: String,
    /// Number of complete scenarios.
    pub scenario_count: usize,
}

/// One external evaluation lane whose evidence semantics differ from LeakDB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEvaluationLaneDescriptor {
    /// Stable evaluation lane identifier.
    pub lane_id: String,
    /// Stable dataset identifier.
    pub dataset_id: String,
    /// Sampling interval in minutes.
    pub time_step_minutes: u64,
    /// Record count when the public dataset has a fixed row count.
    pub record_count: Option<usize>,
    /// Number of public derived features, when applicable.
    pub feature_count: Option<usize>,
    /// Whether the dataset supports pipe-level localization.
    pub pipe_localization_supported: bool,
    /// Whether the evaluation data are a secret, blind holdout.
    pub blind_holdout: bool,
    /// Source inventory identifiers backing the lane.
    pub source_ids: Vec<String>,
}

/// Validated summary of the distinct cyber-physical resilience benchmark track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyberPhysicalResilienceDescriptor {
    /// Registered benchmark resource identifiers.
    pub resource_ids: Vec<String>,
    /// C1-C3 evaluation lane identifiers.
    pub evaluation_lane_ids: Vec<String>,
    /// Disjoint event classes retained by the protocol.
    pub event_classes: Vec<String>,
    /// Non-composite benchmark metrics.
    pub metrics: Vec<String>,
}

/// Validated projection of CivBench experiment protocol v0.1.
///
/// This adapter validates the decision-critical manifest contract used by
/// the paired experiment runner. Callers should validate JSON Schema first,
/// then use this semantic adapter before constructing a scenario run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentProtocolV0_1 {
    /// Protocol identifier.
    pub protocol_id: String,
    /// Protocol version.
    pub version: String,
    /// Manifest lifecycle status.
    pub status: String,
    /// True only when receipt fields have a valid syntactic shape.
    /// This does not authenticate the signature or verify the external registry.
    pub pre_registration_receipt_shape_valid: bool,
    /// Number of declared source records.
    pub source_count: usize,
    /// Stable source identifiers.
    pub source_ids: Vec<String>,
    /// Number of declared intervention levels for G, M, I, H in that order.
    pub factor_level_counts: [usize; 4],
    /// Stage identifiers in manifest order.
    pub stage_ids: Vec<String>,
    /// Estimand identifiers in manifest order.
    pub estimand_ids: Vec<String>,
    /// Declared benchmark lane names.
    pub benchmark_lanes: Vec<String>,
    /// LeakDB dataset inventory.
    pub leak_datasets: Vec<LeakDatasetDescriptor>,
    /// External SCADA and weak-label evaluation lanes.
    pub external_evaluation_lanes: Vec<ExternalEvaluationLaneDescriptor>,
    /// Separate cyber-physical resilience benchmark contract.
    pub cyberphysical_resilience: CyberPhysicalResilienceDescriptor,
}

/// Error returned when a JSON protocol manifest fails semantic validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestAdapterError {
    /// JSON path or top-level location of the invalid contract.
    pub path: String,
    /// Expected value or reason for rejection.
    pub reason: String,
}

impl ManifestAdapterError {
    fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ManifestAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid experiment manifest at {}: {}",
            self.path, self.reason
        )
    }
}

impl std::error::Error for ManifestAdapterError {}

fn manifest_field<'a>(
    value: &'a serde_json::Value,
    key: &str,
    path: &str,
) -> Result<&'a serde_json::Value, ManifestAdapterError> {
    value.get(key).ok_or_else(|| {
        ManifestAdapterError::new(
            format!("{path}.{key}"),
            "required field is missing",
        )
    })
}

fn manifest_string(
    value: &serde_json::Value,
    path: &str,
) -> Result<String, ManifestAdapterError> {
    value
        .as_str()
        .map(str::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ManifestAdapterError::new(path, "expected a non-empty string")
        })
}

fn manifest_u64(
    value: &serde_json::Value,
    path: &str,
) -> Result<u64, ManifestAdapterError> {
    value.as_u64().ok_or_else(|| {
        ManifestAdapterError::new(path, "expected a non-negative integer")
    })
}

fn manifest_bool(
    value: &serde_json::Value,
    path: &str,
) -> Result<bool, ManifestAdapterError> {
    value
        .as_bool()
        .ok_or_else(|| ManifestAdapterError::new(path, "expected a boolean"))
}

fn manifest_array<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a Vec<serde_json::Value>, ManifestAdapterError> {
    value
        .as_array()
        .ok_or_else(|| ManifestAdapterError::new(path, "expected an array"))
}

fn manifest_string_array(
    value: &serde_json::Value,
    path: &str,
) -> Result<Vec<String>, ManifestAdapterError> {
    let values = manifest_array(value, path)?;
    let mut output = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();

    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let item = manifest_string(value, &item_path)?;
        if !seen.insert(item.clone()) {
            return Err(ManifestAdapterError::new(
                item_path,
                "duplicate identifier",
            ));
        }
        output.push(item);
    }
    Ok(output)
}

fn manifest_object_ids(
    value: &serde_json::Value,
    path: &str,
    key: &str,
) -> Result<Vec<String>, ManifestAdapterError> {
    let entries = manifest_array(value, path)?;
    let mut ids = Vec::with_capacity(entries.len());
    let mut seen = BTreeSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        let id_path = format!("{entry_path}.{key}");
        let id = manifest_string(
            manifest_field(entry, key, &entry_path)?,
            &id_path,
        )?;
        if !seen.insert(id.clone()) {
            return Err(ManifestAdapterError::new(
                id_path,
                "duplicate identifier",
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn manifest_require_ids(
    actual: &[String],
    required: &[&str],
    path: &str,
) -> Result<(), ManifestAdapterError> {
    for required_id in required {
        if !actual.iter().any(|value| value == required_id) {
            return Err(ManifestAdapterError::new(
                path,
                format!("required identifier {required_id} is missing"),
            ));
        }
    }
    Ok(())
}

fn is_utc_rfc3339_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20 || !bytes.is_ascii() || !value.ends_with('Z') {
        return false;
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    if !(0..4).all(|index| bytes[index].is_ascii_digit())
        || !(5..7).all(|index| bytes[index].is_ascii_digit())
        || !(8..10).all(|index| bytes[index].is_ascii_digit())
        || !(11..13).all(|index| bytes[index].is_ascii_digit())
        || !(14..16).all(|index| bytes[index].is_ascii_digit())
        || !(17..19).all(|index| bytes[index].is_ascii_digit())
    {
        return false;
    }

    let number = |start: usize, end: usize| -> Option<u32> {
        value.get(start..end)?.parse().ok()
    };
    let year = match number(0, 4) {
        Some(value) if value > 0 => value,
        _ => return false,
    };
    let month = match number(5, 7) {
        Some(value @ 1..=12) => value,
        _ => return false,
    };
    let day = match number(8, 10) {
        Some(value) => value,
        None => return false,
    };
    let hour = match number(11, 13) {
        Some(value @ 0..=23) => value,
        _ => return false,
    };
    let minute = match number(14, 16) {
        Some(value @ 0..=59) => value,
        _ => return false,
    };
    let second = match number(17, 19) {
        Some(value @ 0..=60) => value,
        _ => return false,
    };
    let _ = (hour, minute, second);

    let fractional = &value[19..value.len() - 1];
    if !fractional.is_empty()
        && !(fractional.starts_with('.')
            && fractional.len() > 1
            && fractional[1..].bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

impl ExperimentProtocolV0_1 {
    /// Parse and semantically validate the experiment manifest JSON.
    ///
    /// The current design-only manifest is not eligible for confirmatory runs.
    /// A future locked manifest must include an external registry URI, UTC lock
    /// timestamp, valid SHA-256 lock digest and signature before admission.
    pub fn parse_json(input: &str) -> Result<Self, ManifestAdapterError> {
        let root: serde_json::Value = serde_json::from_str(input).map_err(|error| {
            ManifestAdapterError::new("$", format!("invalid JSON: {error}"))
        })?;

        let protocol_id = manifest_string(
            manifest_field(&root, "protocol_id", "$")?,
            "$.protocol_id",
        )?;
        if protocol_id != "CIVBENCH-JHB-WATER-EXPERIMENT-001" {
            return Err(ManifestAdapterError::new(
                "$.protocol_id",
                "unsupported protocol identifier",
            ));
        }

        let version = manifest_string(
            manifest_field(&root, "version", "$")?,
            "$.version",
        )?;
        if version != "0.1.0" {
            return Err(ManifestAdapterError::new(
                "$.version",
                "this adapter supports exactly version 0.1.0",
            ));
        }

        let status = manifest_string(
            manifest_field(&root, "status", "$")?,
            "$.status",
        )?;
        if !matches!(
            status.as_str(),
            "design_only_pre_registration_pending"
                | "locked_pre_registration"
                | "retired"
        ) {
            return Err(ManifestAdapterError::new(
                "$.status",
                "unknown manifest lifecycle status",
            ));
        }

        let scope = manifest_field(&root, "scope", "$")?;
        let source_values =
            manifest_field(scope, "reference_sources", "$.scope")?;
        let source_ids = manifest_object_ids(
            source_values,
            "$.scope.reference_sources",
            "source_id",
        )?;
        if source_ids.is_empty() {
            return Err(ManifestAdapterError::new(
                "$.scope.reference_sources",
                "at least one source is required",
            ));
        }

        let factors = manifest_field(&root, "factors", "$")?;
        let factor_names = ["G", "M", "I", "H"];
        let expected_names = [
            "allocation_governance",
            "maintenance_renewal",
            "information_quality_observability",
            "physical_network_conditions",
        ];
        let expected_kinds = [
            "intervention",
            "intervention",
            "intervention",
            "blocking_strata_not_policy_benefit",
        ];
        let expected_counts = [4_usize, 5, 4, 4];
        let mut factor_level_counts = [0_usize; 4];
        for (index, factor_name) in factor_names.iter().enumerate() {
            let factor_path = format!("$.factors.{factor_name}");
            let factor = manifest_field(factors, factor_name, "$.factors")?;
            let name = manifest_string(
                manifest_field(factor, "name", &factor_path)?,
                &format!("{factor_path}.name"),
            )?;
            if name != expected_names[index] {
                return Err(ManifestAdapterError::new(
                    format!("{factor_path}.name"),
                    format!("expected {}", expected_names[index]),
                ));
            }
            let kind = manifest_string(
                manifest_field(factor, "kind", &factor_path)?,
                &format!("{factor_path}.kind"),
            )?;
            if kind != expected_kinds[index] {
                return Err(ManifestAdapterError::new(
                    format!("{factor_path}.kind"),
                    format!("expected {}", expected_kinds[index]),
                ));
            }
            let levels = manifest_string_array(
                manifest_field(factor, "levels", &factor_path)?,
                &format!("{factor_path}.levels"),
            )?;
            if levels.len() != expected_counts[index] {
                return Err(ManifestAdapterError::new(
                    format!("{factor_path}.levels"),
                    format!(
                        "expected {} unique levels, found {}",
                        expected_counts[index],
                        levels.len()
                    ),
                ));
            }
            factor_level_counts[index] = levels.len();
        }

        let stage_ids = manifest_object_ids(
            manifest_field(&root, "stages", "$")?,
            "$.stages",
            "id",
        )?;
        if stage_ids.len() != 6 {
            return Err(ManifestAdapterError::new(
                "$.stages",
                "version 0.1.0 requires six stages",
            ));
        }

        let estimand_ids = manifest_object_ids(
            manifest_field(&root, "estimands", "$")?,
            "$.estimands",
            "id",
        )?;
        if estimand_ids.len() != 12 {
            return Err(ManifestAdapterError::new(
                "$.estimands",
                "version 0.1.0 requires twelve estimands",
            ));
        }

        let targets =
            manifest_field(&root, "benchmark_targets", "$")?;
        let target_object = targets.as_object().ok_or_else(|| {
            ManifestAdapterError::new(
                "$.benchmark_targets",
                "expected an object",
            )
        })?;
        let benchmark_lanes: Vec<String> =
            target_object.keys().cloned().collect();
        if benchmark_lanes.len() != 11 {
            return Err(ManifestAdapterError::new(
                "$.benchmark_targets",
                "version 0.1.0 requires eleven benchmark lanes",
            ));
        }
        manifest_require_ids(
            &benchmark_lanes,
            &[
                "accounting_correctness",
                "hydraulic_fidelity",
                "leak_detection",
                "leak_localization",
                "competitive_improvement",
                "sensor_placement",
                "robustness",
                "resilience_performance_curves",
                "hierarchical_hydroinformatics",
                "digital_twin_maturity",
                "leak_validation_integrity",
            ],
            "$.benchmark_targets",
        )?;

        let outcome =
            manifest_field(&root, "outcome_contract", "$")?;
        let status_values = manifest_string_array(
            manifest_field(
                outcome,
                "metric_evidence_statuses",
                "$.outcome_contract",
            )?,
            "$.outcome_contract.metric_evidence_statuses",
        )?;
        let expected_statuses = [
            "observed",
            "derived",
            "modelled",
            "not_measured",
            "unsupported",
            "withheld",
            "not_applicable",
        ];
        if status_values.len() != expected_statuses.len()
            || !expected_statuses.iter().all(|expected| {
                status_values.iter().any(|value| value == expected)
            })
        {
            return Err(ManifestAdapterError::new(
                "$.outcome_contract.metric_evidence_statuses",
                "all seven v0.1 evidence statuses must be present exactly once",
            ));
        }

        let missing_behavior = manifest_string(
            manifest_field(
                outcome,
                "missing_value_behavior",
                "$.outcome_contract",
            )?,
            "$.outcome_contract.missing_value_behavior",
        )?;
        if missing_behavior != "no_zero_fill" {
            return Err(ManifestAdapterError::new(
                "$.outcome_contract.missing_value_behavior",
                "missing metrics must never be zero-filled",
            ));
        }

        let paired_rule = manifest_string(
            manifest_field(outcome, "paired_difference", "$.outcome_contract")?,
            "$.outcome_contract.paired_difference",
        )?;
        if !paired_rule.contains("exactly_the_same_evidence_status") {
            return Err(ManifestAdapterError::new(
                "$.outcome_contract.paired_difference",
                "paired deltas require evidence-class parity",
            ));
        }

        let leak =
            manifest_field(&root, "leak_benchmark_contract", "$")?;
        let dataset_values = manifest_field(
            leak,
            "dataset_inventory",
            "$.leak_benchmark_contract",
        )?;
        let dataset_entries = manifest_array(
            dataset_values,
            "$.leak_benchmark_contract.dataset_inventory",
        )?;
        let mut leak_datasets =
            Vec::with_capacity(dataset_entries.len());
        let mut dataset_ids = BTreeSet::new();

        for (index, entry) in dataset_entries.iter().enumerate() {
            let entry_path =
                format!("$.leak_benchmark_contract.dataset_inventory[{index}]");
            let dataset_id = manifest_string(
                manifest_field(entry, "dataset_id", &entry_path)?,
                &format!("{entry_path}.dataset_id"),
            )?;
            if !dataset_ids.insert(dataset_id.clone()) {
                return Err(ManifestAdapterError::new(
                    format!("{entry_path}.dataset_id"),
                    "duplicate dataset identifier",
                ));
            }

            let topology = manifest_string(
                manifest_field(entry, "topology", &entry_path)?,
                &format!("{entry_path}.topology"),
            )?;
            let scenario_count_u64 = manifest_field(
                entry,
                "scenario_count",
                &entry_path,
            )?
            .as_u64()
            .ok_or_else(|| {
                ManifestAdapterError::new(
                    format!("{entry_path}.scenario_count"),
                    "expected a positive integer",
                )
            })?;
            let scenario_count =
                usize::try_from(scenario_count_u64).map_err(|_| {
                    ManifestAdapterError::new(
                        format!("{entry_path}.scenario_count"),
                        "count exceeds platform range",
                    )
                })?;
            if scenario_count == 0 {
                return Err(ManifestAdapterError::new(
                    format!("{entry_path}.scenario_count"),
                    "scenario count must be positive",
                ));
            }

            let source_refs = manifest_string_array(
                manifest_field(entry, "source_refs", &entry_path)?,
                &format!("{entry_path}.source_refs"),
            )?;
            for source_ref in source_refs {
                if !source_ids.iter().any(|id| id == &source_ref) {
                    return Err(ManifestAdapterError::new(
                        format!("{entry_path}.source_refs"),
                        format!("unknown source identifier {source_ref}"),
                    ));
                }
            }

            leak_datasets.push(LeakDatasetDescriptor {
                dataset_id,
                topology,
                scenario_count,
            });
        }

        for (dataset_id, topology, expected_count) in [
            ("LEAKDB_PAPER_HANOI_500", "Hanoi", 500_usize),
            ("LEAKDB_HANOI_1000", "Hanoi", 1000_usize),
            ("LEAKDB_NET1_1000", "Net1", 1000_usize),
        ] {
            let dataset = leak_datasets
                .iter()
                .find(|dataset| dataset.dataset_id == dataset_id)
                .ok_or_else(|| {
                    ManifestAdapterError::new(
                        "$.leak_benchmark_contract.dataset_inventory",
                        format!("required dataset {dataset_id} is missing"),
                    )
                })?;
            if dataset.topology != topology
                || dataset.scenario_count != expected_count
            {
                return Err(ManifestAdapterError::new(
                    "$.leak_benchmark_contract.dataset_inventory",
                    format!(
                        "dataset {dataset_id} has unexpected topology or scenario count"
                    ),
                ));
            }
        }

        let lanes = manifest_field(
            leak,
            "evaluation_lanes",
            "$.leak_benchmark_contract",
        )?;
        for lane in [
            "L0_paper_reproduction",
            "L1_full_corpus_scenario_holdout",
            "L2_cross_topology_transfer",
        ] {
            manifest_field(
                lanes,
                lane,
                "$.leak_benchmark_contract.evaluation_lanes",
            )?;
        }

        let external_lanes = manifest_field(
            leak,
            "external_evaluation_lanes",
            "$.leak_benchmark_contract",
        )?;

        let battledim_path =
            "$.leak_benchmark_contract.external_evaluation_lanes.L3_BATTLEDIM_L_TOWN_EXTERNAL_SCADA";
        let battledim = manifest_field(
            external_lanes,
            "L3_BATTLEDIM_L_TOWN_EXTERNAL_SCADA",
            "$.leak_benchmark_contract.external_evaluation_lanes",
        )?;
        let battledim_id = manifest_string(
            manifest_field(battledim, "dataset_id", battledim_path)?,
            &format!("{battledim_path}.dataset_id"),
        )?;
        if battledim_id != "BATTLEDIM_L_TOWN_2018_2019" {
            return Err(ManifestAdapterError::new(
                format!("{battledim_path}.dataset_id"),
                "unexpected BattLeDIM dataset identity",
            ));
        }

        let battledim_network = manifest_string(
            manifest_field(battledim, "network", battledim_path)?,
            &format!("{battledim_path}.network"),
        )?;
        if battledim_network
            != "L-Town hypothetical network derived from a real Cyprus water-distribution network"
        {
            return Err(ManifestAdapterError::new(
                format!("{battledim_path}.network"),
                "preserve the L-Town/Cyprus network identity",
            ));
        }
        let training_period = manifest_string(
            manifest_field(battledim, "training_period", battledim_path)?,
            &format!("{battledim_path}.training_period"),
        )?;
        let evaluation_period = manifest_string(
            manifest_field(battledim, "evaluation_period", battledim_path)?,
            &format!("{battledim_path}.evaluation_period"),
        )?;
        if training_period != "2018-01-01T00:00/2018-12-31T23:55"
            || evaluation_period != "2019-01-01T00:00/2019-12-31T23:55"
        {
            return Err(ManifestAdapterError::new(
                battledim_path,
                "BattLeDIM must preserve the 2018 training and public 2019 evaluation periods",
            ));
        }
        let battledim_step = manifest_u64(
            manifest_field(battledim, "time_step_minutes", battledim_path)?,
            &format!("{battledim_path}.time_step_minutes"),
        )?;
        if battledim_step != 5 {
            return Err(ManifestAdapterError::new(
                format!("{battledim_path}.time_step_minutes"),
                "BattLeDIM SCADA sampling interval must be five minutes",
            ));
        }
        let battledim_sensor_path = format!("{battledim_path}.sensor_inventory");
        let battledim_sensors =
            manifest_field(battledim, "sensor_inventory", battledim_path)?;
        for (key, expected) in [
            ("tank_level", 1_u64),
            ("flow", 3_u64),
            ("pressure", 33_u64),
            ("area_c_amr", 82_u64),
        ] {
            let actual = manifest_u64(
                manifest_field(battledim_sensors, key, &battledim_sensor_path)?,
                &format!("{battledim_sensor_path}.{key}"),
            )?;
            if actual != expected {
                return Err(ManifestAdapterError::new(
                    format!("{battledim_sensor_path}.{key}"),
                    format!("expected {expected}"),
                ));
            }
        }
        let battledim_task = manifest_string(
            manifest_field(battledim, "task", battledim_path)?,
            &format!("{battledim_path}.task"),
        )?;
        if battledim_task != "event_detection_and_leak_localization_with_operational_costs" {
            return Err(ManifestAdapterError::new(
                format!("{battledim_path}.task"),
                "unexpected BattLeDIM evaluation task",
            ));
        }
        let battledim_holdout = manifest_string(
            manifest_field(battledim, "holdout_status", battledim_path)?,
            &format!("{battledim_path}.holdout_status"),
        )?;
        if battledim_holdout != "public_historical_external_evaluation_not_secret_or_blind" {
            return Err(ManifestAdapterError::new(
                format!("{battledim_path}.holdout_status"),
                "public historical data must not be described as a blind holdout",
            ));
        }
        let battledim_source_ids = manifest_string_array(
            manifest_field(battledim, "source_refs", battledim_path)?,
            &format!("{battledim_path}.source_refs"),
        )?;
        if battledim_source_ids.len() < 2
            || battledim_source_ids.iter().any(|id| !source_ids.contains(id))
        {
            return Err(ManifestAdapterError::new(
                format!("{battledim_path}.source_refs"),
                "external lane must cite at least two known source records",
            ));
        }

        let slovak_path =
            "$.leak_benchmark_contract.external_evaluation_lanes.L4_SLOVAK_WEAK_LABEL_TEMPORAL_ANOMALY";
        let slovak = manifest_field(
            external_lanes,
            "L4_SLOVAK_WEAK_LABEL_TEMPORAL_ANOMALY",
            "$.leak_benchmark_contract.external_evaluation_lanes",
        )?;
        let slovak_id = manifest_string(
            manifest_field(slovak, "dataset_id", slovak_path)?,
            &format!("{slovak_path}.dataset_id"),
        )?;
        if slovak_id != "SLOVAK_2022_ONE_ZONE_HOURLY_ANOMALY_SCORES" {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.dataset_id"),
                "unexpected Slovak weak-label dataset identity",
            ));
        }

        let slovak_network = manifest_string(
            manifest_field(slovak, "network", slovak_path)?,
            &format!("{slovak_path}.network"),
        )?;
        if slovak_network != "one anonymized Slovak utility distribution zone" {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.network"),
                "preserve the single-zone utility scope",
            ));
        }
        let slovak_timezone = manifest_string(
            manifest_field(slovak, "timezone", slovak_path)?,
            &format!("{slovak_path}.timezone"),
        )?;
        if slovak_timezone != "Europe/Bratislava" {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.timezone"),
                "preserve the source timezone",
            ));
        }
        let slovak_coverage = manifest_string(
            manifest_field(slovak, "data_coverage", slovak_path)?,
            &format!("{slovak_path}.data_coverage"),
        )?;
        if slovak_coverage != "2021-12-31T23:00/2022-12-30T23:00" {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.data_coverage"),
                "unexpected public-record time coverage",
            ));
        }
        let feature_representation = manifest_string(
            manifest_field(slovak, "feature_representation", slovak_path)?,
            &format!("{slovak_path}.feature_representation"),
        )?;
        if !feature_representation.contains("precomputed standardized anomaly scores")
            || !feature_representation.contains("raw telemetry")
        {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.feature_representation"),
                "public release limitations must state that features are precomputed and raw telemetry is unavailable",
            ));
        }
        let label_definition = manifest_string(
            manifest_field(slovak, "label", slovak_path)?,
            &format!("{slovak_path}.label"),
        )?;
        if !label_definition.starts_with("fault_d7:")
            || !label_definition.contains("plus-or-minus seven-day window")
        {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.label"),
                "fault_d7 must remain a plus-or-minus seven-day operational label, not exact onset",
            ));
        }
        let leakage_risk = manifest_string(
            manifest_field(slovak, "leakage_risk", slovak_path)?,
            &format!("{slovak_path}.leakage_risk"),
        )?;
        if !leakage_risk.contains("feature selection conditioned on the fault label") {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.leakage_risk"),
                "label-conditioned feature selection limitation must be explicit",
            ));
        }
        let slovak_step = manifest_u64(
            manifest_field(slovak, "time_step_minutes", slovak_path)?,
            &format!("{slovak_path}.time_step_minutes"),
        )?;
        let slovak_records = manifest_u64(
            manifest_field(slovak, "record_count", slovak_path)?,
            &format!("{slovak_path}.record_count"),
        )?;
        let slovak_features = manifest_u64(
            manifest_field(slovak, "feature_count", slovak_path)?,
            &format!("{slovak_path}.feature_count"),
        )?;
        let slovak_label_window = manifest_u64(
            manifest_field(slovak, "label_window_days", slovak_path)?,
            &format!("{slovak_path}.label_window_days"),
        )?;
        if slovak_step != 60
            || slovak_records != 8_737
            || slovak_features != 18
            || slovak_label_window != 7
        {
            return Err(ManifestAdapterError::new(
                slovak_path,
                "Slovak lane must preserve its 8,737 hourly, 18-feature, +/-7-day identity",
            ));
        }
        let localization_supported = manifest_bool(
            manifest_field(slovak, "pipe_localization_supported", slovak_path)?,
            &format!("{slovak_path}.pipe_localization_supported"),
        )?;
        if localization_supported {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.pipe_localization_supported"),
                "weak-label zone data do not support pipe-level localization",
            ));
        }
        let slovak_task = manifest_string(
            manifest_field(slovak, "task", slovak_path)?,
            &format!("{slovak_path}.task"),
        )?;
        if slovak_task != "weak_label_temporal_anomaly_and_lead_time_only" {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.task"),
                "unexpected Slovak evaluation task",
            ));
        }
        let slovak_holdout = manifest_string(
            manifest_field(slovak, "holdout_status", slovak_path)?,
            &format!("{slovak_path}.holdout_status"),
        )?;
        if slovak_holdout != "public_single_zone_processed_dataset_not_blind_holdout" {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.holdout_status"),
                "public processed data must not be described as a blind holdout",
            ));
        }
        let negative_semantics = manifest_string(
            manifest_field(slovak, "negative_semantics", slovak_path)?,
            &format!("{slovak_path}.negative_semantics"),
        )?;
        if !negative_semantics.contains("not_proof_of_leak_free_state") {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.negative_semantics"),
                "unconfirmed windows must not be treated as verified negatives",
            ));
        }
        let slovak_source_ids = manifest_string_array(
            manifest_field(slovak, "source_refs", slovak_path)?,
            &format!("{slovak_path}.source_refs"),
        )?;
        if slovak_source_ids.is_empty()
            || slovak_source_ids.iter().any(|id| !source_ids.contains(id))
        {
            return Err(ManifestAdapterError::new(
                format!("{slovak_path}.source_refs"),
                "external lane must cite known source records",
            ));
        }

        let external_evaluation_lanes = vec![
            ExternalEvaluationLaneDescriptor {
                lane_id: "L3_BATTLEDIM_L_TOWN_EXTERNAL_SCADA".to_owned(),
                dataset_id: battledim_id,
                time_step_minutes: battledim_step,
                record_count: None,
                feature_count: None,
                pipe_localization_supported: true,
                blind_holdout: false,
                source_ids: battledim_source_ids,
            },
            ExternalEvaluationLaneDescriptor {
                lane_id: "L4_SLOVAK_WEAK_LABEL_TEMPORAL_ANOMALY".to_owned(),
                dataset_id: slovak_id,
                time_step_minutes: slovak_step,
                record_count: Some(usize::try_from(slovak_records).map_err(|_| {
                    ManifestAdapterError::new(
                        format!("{slovak_path}.record_count"),
                        "record count exceeds platform range",
                    )
                })?),
                feature_count: Some(usize::try_from(slovak_features).map_err(|_| {
                    ManifestAdapterError::new(
                        format!("{slovak_path}.feature_count"),
                        "feature count exceeds platform range",
                    )
                })?),
                pipe_localization_supported: false,
                blind_holdout: false,
                source_ids: slovak_source_ids,
            },
        ];

        let cyber_path = "$.cyberphysical_resilience_contract";
        let cyber = manifest_field(&root, "cyberphysical_resilience_contract", "$")?;
        let cyber_version = manifest_string(
            manifest_field(cyber, "version", cyber_path)?,
            "$.cyberphysical_resilience_contract.version",
        )?;
        if cyber_version != "0.1.0" {
            return Err(ManifestAdapterError::new(
                "$.cyberphysical_resilience_contract.version",
                "unsupported cyber-physical contract version",
            ));
        }

        let resource_values = manifest_field(
            cyber,
            "benchmark_resources",
            cyber_path,
        )?;
        let resource_entries = manifest_array(
            resource_values,
            "$.cyberphysical_resilience_contract.benchmark_resources",
        )?;
        if resource_entries.len() != 3 {
            return Err(ManifestAdapterError::new(
                "$.cyberphysical_resilience_contract.benchmark_resources",
                "exactly three distinct research resources are registered in v0.1",
            ));
        }
        let resource_ids = manifest_object_ids(
            resource_values,
            "$.cyberphysical_resilience_contract.benchmark_resources",
            "resource_id",
        )?;
        let expected_resources = [
            ("WDSEventDB", "WDSEVENTDB_REALTIME_TESTBED_DATASET_2025"),
            ("ACG_Water_Testbed_2026", "ACG_WATER_TESTBED_2026"),
            ("WDT_Chronological_Benchmark_2026", "PEREZ_PEREZ_WDT_CYBER_RESILIENCE_2026"),
        ];
        for (resource_id, expected_source) in expected_resources {
            let entry = resource_entries
                .iter()
                .find(|entry| {
                    entry.get("resource_id").and_then(serde_json::Value::as_str)
                        == Some(resource_id)
                })
                .ok_or_else(|| {
                    ManifestAdapterError::new(
                        "$.cyberphysical_resilience_contract.benchmark_resources",
                        format!("required resource {resource_id} is missing"),
                    )
                })?;
            let source_id = manifest_string(
                manifest_field(
                    entry,
                    "source_id",
                    "$.cyberphysical_resilience_contract.benchmark_resources",
                )?,
                "$.cyberphysical_resilience_contract.benchmark_resources.source_id",
            )?;
            if source_id != expected_source || !source_ids.contains(&source_id) {
                return Err(ManifestAdapterError::new(
                    "$.cyberphysical_resilience_contract.benchmark_resources.source_id",
                    format!("resource {resource_id} must cite known source {expected_source}"),
                ));
            }
            manifest_string(
                manifest_field(entry, "claim_limit", "$.cyberphysical_resilience_contract.benchmark_resources")?,
                "$.cyberphysical_resilience_contract.benchmark_resources.claim_limit",
            )?;
            let event_labels = manifest_string_array(
                manifest_field(entry, "event_classes", "$.cyberphysical_resilience_contract.benchmark_resources")?,
                "$.cyberphysical_resilience_contract.benchmark_resources.event_classes",
            )?;
            if event_labels.is_empty() {
                return Err(ManifestAdapterError::new(
                    "$.cyberphysical_resilience_contract.benchmark_resources.event_classes",
                    "each resource must declare its supported event classes",
                ));
            }
        }

        let event_classes = manifest_string_array(
            manifest_field(cyber, "event_classes", cyber_path)?,
            "$.cyberphysical_resilience_contract.event_classes",
        )?;
        let required_event_classes = [
            "leak",
            "sensor_failure",
            "communication_anomaly",
            "cyberattack",
            "process_drift",
            "unknown",
        ];
        if event_classes.len() != required_event_classes.len()
            || required_event_classes.iter().any(|expected| {
                !event_classes.iter().any(|value| value == expected)
            })
        {
            return Err(ManifestAdapterError::new(
                "$.cyberphysical_resilience_contract.event_classes",
                "leak, sensor failure, communication anomaly, cyberattack, process drift and unknown must remain distinct",
            ));
        }

        let evaluation_lanes = manifest_field(
            cyber,
            "evaluation_lanes",
            cyber_path,
        )?;
        let lane_ids = vec![
            "C1_event_discrimination".to_owned(),
            "C2_cyber_to_physical_propagation".to_owned(),
            "C3_control_and_resilience_comparison".to_owned(),
        ];
        let expected_lane_resources = [
            ("C1_event_discrimination", vec!["WDSEventDB", "WDT_Chronological_Benchmark_2026"]),
            ("C2_cyber_to_physical_propagation", vec!["ACG_Water_Testbed_2026"]),
            ("C3_control_and_resilience_comparison", vec!["WDSEventDB", "ACG_Water_Testbed_2026", "WDT_Chronological_Benchmark_2026"]),
        ];
        for (lane_id, expected_ids) in expected_lane_resources {
            let lane = manifest_field(evaluation_lanes, lane_id, "$.cyberphysical_resilience_contract.evaluation_lanes")?;
            let lane_path = format!("$.cyberphysical_resilience_contract.evaluation_lanes.{lane_id}");
            let task = manifest_string(
                manifest_field(lane, "task", &lane_path)?,
                &format!("{lane_path}.task"),
            )?;
            let _ = task;
            let ids = manifest_string_array(
                manifest_field(lane, "resource_ids", &lane_path)?,
                &format!("{lane_path}.resource_ids"),
            )?;
            if ids.len() != expected_ids.len()
                || expected_ids.iter().any(|expected| !ids.iter().any(|value| value == expected))
                || ids.iter().any(|id| !resource_ids.contains(id))
            {
                return Err(ManifestAdapterError::new(
                    format!("{lane_path}.resource_ids"),
                    "lane must use exactly its registered benchmark resources",
                ));
            }
        }

        let metrics = manifest_string_array(
            manifest_field(cyber, "metrics", cyber_path)?,
            "$.cyberphysical_resilience_contract.metrics",
        )?;
        let required_metrics = [
            "event_precision_recall",
            "event_onset_delay",
            "false_alarm_episodes_per_system_day",
            "calibration_and_abstention",
            "physical_consequence_exposure",
            "time_to_safe_state",
            "recovery_time",
            "service_deficit",
        ];
        if metrics.len() != required_metrics.len()
            || required_metrics.iter().any(|expected| {
                !metrics.iter().any(|value| value == expected)
            })
        {
            return Err(ManifestAdapterError::new(
                "$.cyberphysical_resilience_contract.metrics",
                "all eight non-composite event and physical-outcome metrics are required",
            ));
        }

        let hard_gates = manifest_field(cyber, "hard_gates", cyber_path)?;
        for gate in [
            "no_live_plc_or_production_scada",
            "human_authorization_required_for_operational_actions",
            "keep_leak_sensor_failure_and_attack_labels_distinct",
            "do_not_infer_malicious_intent_from_anomaly_alone",
            "benchmark_score_never_grants_actuation_authority",
            "independent_safety_interlocks_and_rollback_required",
        ] {
            let path = format!("{cyber_path}.hard_gates.{gate}");
            let enabled = manifest_bool(manifest_field(hard_gates, gate, &format!("{cyber_path}.hard_gates"))?, &path)?;
            if !enabled {
                return Err(ManifestAdapterError::new(
                    path,
                    "cyber-physical evaluation hard gates cannot be disabled",
                ));
            }
        }

        let cyberphysical_resilience = CyberPhysicalResilienceDescriptor {
            resource_ids,
            evaluation_lane_ids: lane_ids,
            event_classes,
            metrics,
        };

        let confidence = manifest_field(
            leak,
            "confidence_contract",
            "$.leak_benchmark_contract",
        )?;
        let independent_negative_count = manifest_u64(
            manifest_field(
                confidence,
                "independent_negative_scenarios_for_one_sided_95_upper_bound_below_1_percent",
                "$.leak_benchmark_contract.confidence_contract",
            )?,
            "$.leak_benchmark_contract.confidence_contract.independent_negative_scenarios_for_one_sided_95_upper_bound_below_1_percent",
        )?;
        if independent_negative_count != 299 {
            return Err(ManifestAdapterError::new(
                "$.leak_benchmark_contract.confidence_contract.independent_negative_scenarios_for_one_sided_95_upper_bound_below_1_percent",
                "at least 299 independent negatives are required for a one-sided 95% exact upper FPR bound below 1% when zero false positives are observed",
            ));
        }
        let exact_upper_bound = manifest_field(
            confidence,
            "example_0_of_299_one_sided_95_percent_exact_upper_bound_percent",
            "$.leak_benchmark_contract.confidence_contract",
        )?
        .as_f64()
        .ok_or_else(|| {
            ManifestAdapterError::new(
                "$.leak_benchmark_contract.confidence_contract.example_0_of_299_one_sided_95_percent_exact_upper_bound_percent",
                "expected a finite numeric percentage",
            )
        })?;
        if (exact_upper_bound - 0.996_914_679_3).abs() > 1.0e-9 {
            return Err(ManifestAdapterError::new(
                "$.leak_benchmark_contract.confidence_contract.example_0_of_299_one_sided_95_percent_exact_upper_bound_percent",
                "unexpected one-sided exact binomial bound for 0/299",
            ));
        }

        let split = manifest_field(
            leak,
            "split_contract",
            "$.leak_benchmark_contract",
        )?;
        let split_unit = manifest_string(
            manifest_field(
                split,
                "unit_of_split",
                "$.leak_benchmark_contract.split_contract",
            )?,
            "$.leak_benchmark_contract.split_contract.unit_of_split",
        )?;
        if split_unit != "complete_scenario" {
            return Err(ManifestAdapterError::new(
                "$.leak_benchmark_contract.split_contract.unit_of_split",
                "time windows from one scenario must never cross partitions",
            ));
        }

        let partitions = manifest_string_array(
            manifest_field(
                split,
                "partitions",
                "$.leak_benchmark_contract.split_contract",
            )?,
            "$.leak_benchmark_contract.split_contract.partitions",
        )?;
        manifest_require_ids(
            &partitions,
            &["train", "validation", "calibration", "sealed_test"],
            "$.leak_benchmark_contract.split_contract.partitions",
        )?;

        let fit_rules = manifest_field(
            split,
            "fit_rules",
            "$.leak_benchmark_contract.split_contract",
        )?;
        let required_fit_rules = [
            ("transforms", "fit_on_train_only"),
            ("hyperparameters", "nested_selection_within_train_only"),
            ("probability_calibration", "calibration_partition_only"),
            ("operating_threshold", "calibration_partition_only"),
            ("sensor_placement", "train_partition_only"),
            ("sealed_test", "single_frozen_evaluation_after_all_choices_locked"),
        ];
        for (field, expected) in required_fit_rules {
            let actual = manifest_string(
                manifest_field(
                    fit_rules,
                    field,
                    "$.leak_benchmark_contract.split_contract.fit_rules",
                )?,
                &format!(
                    "$.leak_benchmark_contract.split_contract.fit_rules.{field}"
                ),
            )?;
            if actual != expected {
                return Err(ManifestAdapterError::new(
                    format!(
                        "$.leak_benchmark_contract.split_contract.fit_rules.{field}"
                    ),
                    format!("expected {expected}"),
                ));
            }
        }

        let test_label_access = manifest_string(
            manifest_field(
                split,
                "test_label_access",
                "$.leak_benchmark_contract.split_contract",
            )?,
            "$.leak_benchmark_contract.split_contract.test_label_access",
        )?;
        if test_label_access != "independent_evaluator_only" {
            return Err(ManifestAdapterError::new(
                "$.leak_benchmark_contract.split_contract.test_label_access",
                "sealed test labels must be evaluator-only",
            ));
        }

        for flag in [
            "group_related_scenario_families",
            "frozen_split_manifest_required",
            "source_archive_digest_required",
        ] {
            if manifest_field(
                split,
                flag,
                "$.leak_benchmark_contract.split_contract",
            )?
            .as_bool()
                != Some(true)
            {
                return Err(ManifestAdapterError::new(
                    format!("$.leak_benchmark_contract.split_contract.{flag}"),
                    "must be true",
                ));
            }
        }

        let provenance =
            manifest_field(&root, "provenance_receipt", "$")?;
        let required_digests = manifest_string_array(
            manifest_field(provenance, "required_digests", "$.provenance_receipt")?,
            "$.provenance_receipt.required_digests",
        )?;
        manifest_require_ids(
            &required_digests,
            &[
                "protocol",
                "schema",
                "source_inventory",
                "source_snapshots_if_available",
            ],
            "$.provenance_receipt.required_digests",
        )?;
        let missing_digest_behavior = manifest_string(
            manifest_field(
                provenance,
                "missing_digest_behavior",
                "$.provenance_receipt",
            )?,
            "$.provenance_receipt.missing_digest_behavior",
        )?;
        if missing_digest_behavior != "fail_closed" {
            return Err(ManifestAdapterError::new(
                "$.provenance_receipt.missing_digest_behavior",
                "missing provenance must fail closed",
            ));
        }

        let pre_registration_receipt_shape_valid = if status == "locked_pre_registration" {
            let receipt = manifest_field(&root, "pre_registration_receipt", "$")?;
            let registry_uri = manifest_string(
                manifest_field(receipt, "registry_uri", "$.pre_registration_receipt")?,
                "$.pre_registration_receipt.registry_uri",
            )?;
            if !registry_uri.starts_with("https://") || registry_uri.len() <= 8 {
                return Err(ManifestAdapterError::new(
                    "$.pre_registration_receipt.registry_uri",
                    "registry URI must use HTTPS",
                ));
            }

            let locked_at_utc = manifest_string(
                manifest_field(receipt, "locked_at_utc", "$.pre_registration_receipt")?,
                "$.pre_registration_receipt.locked_at_utc",
            )?;
            if !is_utc_rfc3339_timestamp(&locked_at_utc) {
                return Err(ManifestAdapterError::new(
                    "$.pre_registration_receipt.locked_at_utc",
                    "expected an RFC 3339 UTC timestamp ending in Z",
                ));
            }

            // Presence and syntax are not cryptographic verification. A registry
            // verifier must authenticate the signature and immutable lock record.
            manifest_string(
                manifest_field(receipt, "signature", "$.pre_registration_receipt")?,
                "$.pre_registration_receipt.signature",
            )?;
            let digest_hex = manifest_string(
                manifest_field(receipt, "lock_digest", "$.pre_registration_receipt")?,
                "$.pre_registration_receipt.lock_digest",
            )?;
            let lock_digest = Digest::from_hex(&digest_hex).map_err(|error| {
                ManifestAdapterError::new(
                    "$.pre_registration_receipt.lock_digest",
                    error.to_string(),
                )
            })?;
            if lock_digest.is_zero() {
                return Err(ManifestAdapterError::new(
                    "$.pre_registration_receipt.lock_digest",
                    "lock digest cannot be the all-zero sentinel",
                ));
            }
            true
        } else {
            false
        };

        Ok(Self {
            protocol_id,
            version,
            status,
            pre_registration_receipt_shape_valid,
            source_count: source_ids.len(),
            source_ids,
            factor_level_counts,
            stage_ids,
            estimand_ids,
            benchmark_lanes,
            leak_datasets,
            external_evaluation_lanes,
            cyberphysical_resilience,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest::from_bytes([byte; 32])
    }

    fn envelope(values: [u64; 8]) -> ResourceEnvelope {
        ResourceEnvelope::from_values(values)
    }

    fn arm(level: &str, resources: ResourceEnvelope) -> ScenarioArm {
        ScenarioArm::new(
            "block-001",
            digest(1),
            digest(2),
            digest(3),
            digest(4),
            digest(5),
            24,
            FactorAssignment::new(InterventionFactor::Maintenance, level),
            resources,
        )
    }

    #[test]
    fn manifest_adapter_accepts_the_pinned_protocol_fixture() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let manifest = ExperimentProtocolV0_1::parse_json(input).unwrap();

        assert_eq!(manifest.protocol_id, "CIVBENCH-JHB-WATER-EXPERIMENT-001");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.status, "design_only_pre_registration_pending");
        assert!(!manifest.pre_registration_receipt_shape_valid);
        assert_eq!(manifest.source_count, 22);
        assert_eq!(manifest.factor_level_counts, [4, 5, 4, 4]);
        assert_eq!(manifest.stage_ids.len(), 6);
        assert_eq!(manifest.estimand_ids.len(), 12);
        assert_eq!(manifest.benchmark_lanes.len(), 11);
        assert_eq!(manifest.leak_datasets.len(), 3);
        assert_eq!(manifest.external_evaluation_lanes.len(), 2);
        assert_eq!(manifest.cyberphysical_resilience.resource_ids.len(), 3);
        assert_eq!(manifest.cyberphysical_resilience.evaluation_lane_ids.len(), 3);
        assert_eq!(manifest.cyberphysical_resilience.event_classes.len(), 6);
        assert_eq!(manifest.cyberphysical_resilience.metrics.len(), 8);
        assert!(manifest.external_evaluation_lanes.iter().any(|lane| {
            lane.dataset_id == "BATTLEDIM_L_TOWN_2018_2019"
                && lane.time_step_minutes == 5
                && lane.pipe_localization_supported
                && !lane.blind_holdout
        }));
        assert!(manifest.external_evaluation_lanes.iter().any(|lane| {
            lane.dataset_id == "SLOVAK_2022_ONE_ZONE_HOURLY_ANOMALY_SCORES"
                && lane.record_count == Some(8_737)
                && lane.feature_count == Some(18)
                && !lane.pipe_localization_supported
                && !lane.blind_holdout
        }));
        assert!(manifest.leak_datasets.iter().any(|dataset| {
            dataset.dataset_id == "LEAKDB_HANOI_1000" && dataset.scenario_count == 1000
        }));
    }

    #[test]
    fn manifest_adapter_rejects_unsupported_protocol_version() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["version"] = serde_json::Value::String("0.2.0".to_owned());

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path == "$.version"
        ));
    }

    #[test]
    fn manifest_adapter_rejects_factor_cardinality_drift() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["factors"]["M"]["levels"].as_array_mut().unwrap().pop();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path == "$.factors.M.levels"
        ));
    }

    #[test]
    fn manifest_adapter_rejects_unsealed_threshold_fit() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["split_contract"]["fit_rules"]["operating_threshold"] =
            serde_json::Value::String("sealed_test_partition".to_owned());

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path == "$.leak_benchmark_contract.split_contract.fit_rules.operating_threshold"
        ));
    }

    #[test]
    fn manifest_adapter_rejects_unknown_dataset_source_reference() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["dataset_inventory"][0]["source_refs"][0] =
            serde_json::Value::String("UNKNOWN_SOURCE".to_owned());

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path.ends_with("source_refs")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_locked_status_without_external_receipt() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["status"] = serde_json::Value::String("locked_pre_registration".to_owned());

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path == "$.pre_registration_receipt"
        ));
    }

    #[test]
    fn manifest_adapter_records_receipt_shape_without_authenticating_signature() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["status"] = serde_json::Value::String("locked_pre_registration".to_owned());
        value["pre_registration_receipt"] = serde_json::json!({
            "registry_uri": "https://registry.example.test/records/civbench-001",
            "locked_at_utc": "2026-11-03T05:00:00Z",
            "lock_digest": "ab".repeat(32),
            "signature": "present-but-not-cryptographically-verified"
        });

        let manifest =
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert!(manifest.pre_registration_receipt_shape_valid);
        assert_eq!(manifest.status, "locked_pre_registration");
    }

    #[test]
    fn manifest_adapter_rejects_non_utc_registration_timestamp() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["status"] = serde_json::Value::String("locked_pre_registration".to_owned());
        value["pre_registration_receipt"] = serde_json::json!({
            "registry_uri": "https://registry.example.test/records/civbench-001",
            "locked_at_utc": "2026-11-03T07:00:00+02:00",
            "lock_digest": "ab".repeat(32),
            "signature": "not-verified"
        });

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path == "$.pre_registration_receipt.locked_at_utc"
        ));
    }

    #[test]
    fn manifest_adapter_requires_h_to_remain_a_blocking_stratum() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["factors"]["H"]["kind"] = serde_json::Value::String("intervention".to_owned());

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path == "$.factors.H.kind"
        ));
    }

    #[test]
    fn manifest_adapter_rejects_battledim_sensor_inventory_drift() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["external_evaluation_lanes"]
            ["L3_BATTLEDIM_L_TOWN_EXTERNAL_SCADA"]["sensor_inventory"]["pressure"] = 32.into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path.ends_with("sensor_inventory.pressure")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_slovak_pipe_localization_claim() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["external_evaluation_lanes"]
            ["L4_SLOVAK_WEAK_LABEL_TEMPORAL_ANOMALY"]["pipe_localization_supported"] = true.into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path.ends_with("pipe_localization_supported")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_unknown_external_lane_source() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["external_evaluation_lanes"]
            ["L4_SLOVAK_WEAK_LABEL_TEMPORAL_ANOMALY"]["source_refs"][0] =
            "UNKNOWN_SOURCE".into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path.ends_with("source_refs")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_underpowered_sub_one_percent_fpr_claim() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["confidence_contract"]
            ["independent_negative_scenarios_for_one_sided_95_upper_bound_below_1_percent"] =
            298.into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path.ends_with("independent_negative_scenarios_for_one_sided_95_upper_bound_below_1_percent")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_battledim_period_relabeling() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["external_evaluation_lanes"]
            ["L3_BATTLEDIM_L_TOWN_EXTERNAL_SCADA"]["evaluation_period"] =
            "2018-01-01T00:00/2018-12-31T23:55".into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path.ends_with("external_evaluation_lanes.L3_BATTLEDIM_L_TOWN_EXTERNAL_SCADA")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_exact_onset_relabeling_of_slovak_fault_d7() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["leak_benchmark_contract"]["external_evaluation_lanes"]
            ["L4_SLOVAK_WEAK_LABEL_TEMPORAL_ANOMALY"]["label"] =
            "fault_d7: exact leak onset".into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path.ends_with(".label")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_live_plc_cyber_benchmark_scope() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["cyberphysical_resilience_contract"]["hard_gates"]["no_live_plc_or_production_scada"] = false.into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path.ends_with("no_live_plc_or_production_scada")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_unknown_cyber_benchmark_source() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["cyberphysical_resilience_contract"]["benchmark_resources"][0]["source_id"] =
            "UNKNOWN_SOURCE".into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path.ends_with("benchmark_resources.source_id")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_cyber_lane_resource_substitution() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["cyberphysical_resilience_contract"]["evaluation_lanes"]
            ["C2_cyber_to_physical_propagation"]["resource_ids"][0] = "WDSEventDB".into();

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. })
                if path.ends_with("C2_cyber_to_physical_propagation.resource_ids")
        ));
    }

    #[test]
    fn manifest_adapter_rejects_merged_cyber_and_leak_event_classes() {
        let input = include_str!("../tests/fixtures/experiment.v0.1.json");
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["cyberphysical_resilience_contract"]["event_classes"]
            .as_array_mut().unwrap().retain(|value| value != "cyberattack");

        assert!(matches!(
            ExperimentProtocolV0_1::parse_json(&serde_json::to_string(&value).unwrap()),
            Err(ManifestAdapterError { path, .. }) if path.ends_with("event_classes")
        ));
    }

    #[test]
    fn digest_hex_round_trip_is_stable() {
        let expected = digest(0xab);
        let encoded = expected.to_hex();
        assert_eq!(encoded.len(), 64);
        assert_eq!(Digest::from_hex(&encoded).unwrap(), expected);
        assert_eq!(Digest::from_hex(&encoded.to_uppercase()).unwrap(), expected);
    }

    #[test]
    fn digest_parser_rejects_bad_length_and_non_hex() {
        assert_eq!(Digest::from_hex("abc"), Err(DigestError::InvalidLength));
        let mut invalid = "0".repeat(64);
        invalid.replace_range(17..18, "z");
        assert_eq!(
            Digest::from_hex(&invalid),
            Err(DigestError::InvalidHexCharacter { index: 17 })
        );
    }

    #[test]
    fn paired_block_accepts_only_declared_resource_treatment_changes() {
        let control = arm(
            "reactive_only",
            envelope([1000, 100, 100, 600, 30, 300, 500, 2]),
        );
        let treatment = arm(
            "risk_prioritised",
            envelope([1000, 100, 100, 480, 30, 300, 500, 2]),
        );
        let block =
            PairedScenarioBlock::new(control, treatment, &[ResourceKind::CrewMinutes], digest(99))
                .unwrap();

        let view = block.candidate_view();
        assert_eq!(view.control.assignment.level, "reactive_only");
        assert_eq!(view.treatment.assignment.level, "risk_prioritised");
        assert_eq!(view.control.resources.get(ResourceKind::CrewMinutes), 600);
        assert_eq!(view.treatment.resources.get(ResourceKind::CrewMinutes), 480);
        assert_eq!(block.evaluator_truth_digest(), digest(99));
    }

    #[test]
    fn paired_block_rejects_zero_truth_and_zero_horizon() {
        let resources = envelope([1000, 100, 100, 600, 30, 300, 500, 2]);
        let mut control = arm("reactive_only", resources);
        let mut treatment = arm("risk_prioritised", resources);
        control.exogenous_digest = digest(0);
        treatment.exogenous_digest = digest(0);
        assert_eq!(
            PairedScenarioBlock::new(control.clone(), treatment.clone(), &[], digest(99)),
            Err(PairError::ZeroScenarioDigest)
        );

        let mut valid_control = arm("reactive_only", resources);
        let mut valid_treatment = arm("risk_prioritised", resources);
        valid_control.exogenous_digest = digest(1);
        valid_treatment.exogenous_digest = digest(1);

        assert_eq!(
            PairedScenarioBlock::new(
                valid_control.clone(),
                valid_treatment.clone(),
                &[],
                digest(0)
            ),
            Err(PairError::ZeroEvaluatorTruthDigest)
        );
        valid_treatment.horizon_ticks = 0;
        assert_eq!(
            PairedScenarioBlock::new(valid_control, valid_treatment, &[], digest(99)),
            Err(PairError::ZeroHorizon)
        );
    }

    #[test]
    fn paired_block_rejects_exogenous_schedule_mismatch() {
        let control = arm(
            "reactive_only",
            envelope([1000, 100, 100, 600, 30, 300, 500, 2]),
        );
        let mut treatment = arm("risk_prioritised", control.resources);
        treatment.exogenous_digest = digest(8);

        assert_eq!(
            PairedScenarioBlock::new(control, treatment, &[], digest(99)),
            Err(PairError::ExogenousScheduleMismatch)
        );
    }

    #[test]
    fn paired_block_rejects_undeclared_resource_envelope_difference() {
        let control = arm(
            "reactive_only",
            envelope([1000, 100, 100, 600, 30, 300, 500, 2]),
        );
        let treatment = arm(
            "risk_prioritised",
            envelope([1000, 100, 90, 600, 30, 300, 500, 2]),
        );

        assert_eq!(
            PairedScenarioBlock::new(control, treatment, &[], digest(99)),
            Err(PairError::UndeclaredResourceDifference(
                ResourceKind::OperatingMinor
            ))
        );
    }

    #[test]
    fn resource_ledger_commits_within_budget() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 600, 30, 300, 500, 2]));
        ledger
            .commit("repair-a", envelope([100, 10, 20, 120, 2, 30, 50, 0]))
            .unwrap();
        ledger
            .commit("repair-b", envelope([200, 20, 30, 180, 3, 40, 80, 1]))
            .unwrap();

        assert_eq!(ledger.commitment_count(), 2);
        assert_eq!(ledger.committed().get(ResourceKind::WaterLitres), 300);
        assert_eq!(ledger.committed().get(ResourceKind::CrewMinutes), 300);
    }

    #[test]
    fn resource_ledger_rejects_double_spend_and_preserves_state() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 100, 30, 300, 500, 2]));
        ledger
            .commit("repair-a", envelope([0, 0, 0, 70, 0, 0, 0, 0]))
            .unwrap();
        let before = ledger.clone();

        assert_eq!(
            ledger.commit("repair-b", envelope([0, 0, 0, 40, 0, 0, 0, 0])),
            Err(LedgerError::ExceedsAvailable {
                kind: ResourceKind::CrewMinutes,
                available: 100,
                committed: 70,
                requested: 40,
            })
        );
        assert_eq!(ledger, before);
        assert_eq!(ledger.commitment_count(), 1);
    }

    #[test]
    fn resource_ledger_rejects_duplicate_commitment_ids() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 100, 30, 300, 500, 2]));
        let zero = ResourceEnvelope::zero();
        ledger.commit("same-id", zero).unwrap();
        assert_eq!(
            ledger.commit(" same-id ", zero),
            Err(LedgerError::DuplicateCommitment("same-id".to_owned()))
        );
        assert_eq!(ledger.commitment_count(), 1);
    }

    #[test]
    fn resource_ledger_is_atomic_across_multiple_resource_dimensions() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 100, 30, 300, 500, 2]));
        let request = envelope([100, 10, 20, 10, 2, 30, 600, 1]);
        let before = ledger.clone();

        assert!(matches!(
            ledger.commit("too-large", request),
            Err(LedgerError::ExceedsAvailable {
                kind: ResourceKind::EnergyWh,
                ..
            })
        ));
        assert_eq!(ledger, before);
    }

    #[test]
    fn resource_reservation_settlement_separates_buckets() {
        let available = envelope([1000, 100, 200, 600, 30, 300, 500, 2]);
        let mut ledger = ResourceLedger::new(available);
        let reservation = envelope([100, 10, 20, 120, 2, 30, 50, 1]);
        let actual = envelope([80, 8, 15, 90, 1, 25, 40, 1]);

        ledger.reserve("work-1", reservation).unwrap();
        assert_eq!(ledger.reserved(), reservation);
        assert_eq!(ledger.consumed(), ResourceEnvelope::zero());
        assert_eq!(ledger.remaining().get(ResourceKind::CrewMinutes), 480);
        assert_eq!(
            ledger.commitment_state("work-1"),
            Some(ResourceCommitmentState::Reserved)
        );

        ledger.settle("work-1", actual).unwrap();
        assert_eq!(ledger.reserved(), ResourceEnvelope::zero());
        assert_eq!(ledger.consumed(), actual);
        assert_eq!(ledger.remaining().get(ResourceKind::CrewMinutes), 510);
        assert_eq!(
            ledger.commitment_state("work-1"),
            Some(ResourceCommitmentState::Settled)
        );
    }

    #[test]
    fn resource_ledger_exposes_reserved_and_settled_amounts_per_commitment() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 600, 30, 300, 500, 2]));
        let reservation = envelope([0, 0, 0, 100, 0, 0, 0, 0]);
        let actual = envelope([0, 0, 0, 75, 0, 0, 0, 0]);

        ledger.reserve("job-17", reservation).unwrap();
        assert_eq!(ledger.commitment_reservation("job-17"), Some(reservation));
        assert_eq!(ledger.commitment_settlement("job-17"), None);

        ledger.settle("job-17", actual).unwrap();
        assert_eq!(ledger.commitment_reservation("job-17"), Some(reservation));
        assert_eq!(ledger.commitment_settlement("job-17"), Some(actual));
    }

    #[test]
    fn resource_reservation_release_returns_capacity_without_fake_consumption() {
        let available = envelope([1000, 100, 200, 600, 30, 300, 500, 2]);
        let mut ledger = ResourceLedger::new(available);
        let reservation = envelope([0, 0, 0, 240, 0, 0, 0, 0]);

        ledger.reserve("cancelled-work", reservation).unwrap();
        assert_eq!(ledger.remaining().get(ResourceKind::CrewMinutes), 360);
        ledger.release("cancelled-work").unwrap();
        assert_eq!(ledger.reserved(), ResourceEnvelope::zero());
        assert_eq!(ledger.consumed(), ResourceEnvelope::zero());
        assert_eq!(ledger.remaining().get(ResourceKind::CrewMinutes), 600);
        assert_eq!(
            ledger.commitment_state("cancelled-work"),
            Some(ResourceCommitmentState::Released)
        );
    }

    #[test]
    fn resource_settlement_overrun_is_rejected_atomically() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 100, 30, 300, 500, 2]));
        let reservation = envelope([0, 0, 0, 70, 0, 0, 0, 0]);
        ledger.reserve("work-1", reservation).unwrap();
        let before = ledger.clone();
        let overrun = envelope([0, 0, 0, 71, 0, 0, 0, 0]);

        assert_eq!(
            ledger.settle("work-1", overrun),
            Err(LedgerError::ActualExceedsReservation {
                kind: ResourceKind::CrewMinutes,
                reserved: 70,
                actual: 71
            })
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn resource_ledger_rejects_invalid_lifecycle_transitions_and_id_reuse() {
        let mut ledger = ResourceLedger::new(envelope([1000, 100, 200, 100, 30, 300, 500, 2]));
        let reservation = envelope([0, 0, 0, 10, 0, 0, 0, 0]);
        ledger.reserve("work-1", reservation).unwrap();
        ledger.release("work-1").unwrap();

        assert_eq!(
            ledger.settle("work-1", ResourceEnvelope::zero()),
            Err(LedgerError::InvalidTransition {
                id: "work-1".to_owned(),
                state: ResourceCommitmentState::Released
            })
        );
        assert_eq!(
            ledger.release("work-1"),
            Err(LedgerError::InvalidTransition {
                id: "work-1".to_owned(),
                state: ResourceCommitmentState::Released
            })
        );
        assert_eq!(
            ledger.reserve("work-1", reservation),
            Err(LedgerError::DuplicateCommitment("work-1".to_owned()))
        );
    }

    #[test]
    fn resource_ledger_rejects_unknown_settlement_and_release() {
        let mut ledger = ResourceLedger::new(envelope([100, 10, 20, 30, 4, 5, 6, 7]));
        assert_eq!(
            ledger.settle("missing", ResourceEnvelope::zero()),
            Err(LedgerError::UnknownCommitment("missing".to_owned()))
        );
        assert_eq!(
            ledger.release("missing"),
            Err(LedgerError::UnknownCommitment("missing".to_owned()))
        );
    }

    #[test]
    fn outcome_receipt_preserves_signed_component_deltas() {
        let control = OutcomeVector {
            essential_shortfall_l: QualifiedMetric::observed(1000),
            worst_group_service_ppm: QualifiedMetric::observed(800_000),
            false_dispatches: QualifiedMetric::observed(10),
            ..OutcomeVector::default()
        };
        let treatment = OutcomeVector {
            essential_shortfall_l: QualifiedMetric::observed(700),
            worst_group_service_ppm: QualifiedMetric::observed(850_000),
            false_dispatches: QualifiedMetric::observed(14),
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control, treatment).unwrap();

        assert_eq!(receipt.delta.essential_shortfall_l.value, Some(-300));
        assert_eq!(
            receipt.delta.essential_shortfall_l.status,
            MetricDeltaStatus::Comparable {
                evidence_status: MetricEvidenceStatus::Observed
            }
        );
        assert_eq!(receipt.delta.worst_group_service_ppm.value, Some(50_000));
        assert_eq!(receipt.delta.false_dispatches.value, Some(4));
    }

    #[test]
    fn outcome_receipt_preserves_missingness_instead_of_zero_filling() {
        let control = OutcomeVector {
            physical_loss_l: QualifiedMetric::observed(0),
            ..OutcomeVector::default()
        };
        let treatment = OutcomeVector {
            physical_loss_l: QualifiedMetric::unsupported(),
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control, treatment).unwrap();
        assert_eq!(receipt.delta.physical_loss_l.value, None);
        assert_eq!(
            receipt.delta.physical_loss_l.status,
            MetricDeltaStatus::EvidenceStatusMismatch {
                control: MetricEvidenceStatus::Observed,
                treatment: MetricEvidenceStatus::Unsupported
            }
        );
        assert_eq!(receipt.control.physical_loss_l.value, Some(0));
        assert_eq!(receipt.treatment.physical_loss_l.value, None);
        assert_eq!(
            receipt.treatment.physical_loss_l.status,
            MetricEvidenceStatus::Unsupported
        );
    }

    #[test]
    fn outcome_receipt_rejects_mixed_numeric_evidence_classes() {
        let control = OutcomeVector {
            physical_loss_l: QualifiedMetric::observed(100),
            ..OutcomeVector::default()
        };
        let treatment = OutcomeVector {
            physical_loss_l: QualifiedMetric::modelled(70),
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control, treatment).unwrap();

        assert_eq!(receipt.delta.physical_loss_l.value, None);
        assert_eq!(
            receipt.delta.physical_loss_l.status,
            MetricDeltaStatus::EvidenceStatusMismatch {
                control: MetricEvidenceStatus::Observed,
                treatment: MetricEvidenceStatus::Modelled
            }
        );
    }

    #[test]
    fn outcome_receipt_distinguishes_missing_control_from_missing_treatment() {
        let control_missing = OutcomeVector {
            physical_loss_l: QualifiedMetric::not_measured(),
            ..OutcomeVector::default()
        };
        let treatment_observed = OutcomeVector {
            physical_loss_l: QualifiedMetric::observed(10),
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control_missing, treatment_observed).unwrap();
        assert_eq!(receipt.delta.physical_loss_l.value, None);
        assert_eq!(
            receipt.delta.physical_loss_l.status,
            MetricDeltaStatus::MissingControl {
                status: MetricEvidenceStatus::NotMeasured
            }
        );

        let control_observed = OutcomeVector {
            physical_loss_l: QualifiedMetric::observed(10),
            ..OutcomeVector::default()
        };
        let treatment_missing = OutcomeVector {
            physical_loss_l: QualifiedMetric::withheld(),
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control_observed, treatment_missing).unwrap();
        assert_eq!(receipt.delta.physical_loss_l.value, None);
        assert_eq!(
            receipt.delta.physical_loss_l.status,
            MetricDeltaStatus::MissingTreatment {
                status: MetricEvidenceStatus::Withheld
            }
        );
    }

    #[test]
    fn outcome_receipt_reports_both_missing_reasons() {
        let control = OutcomeVector {
            physical_loss_l: QualifiedMetric::not_measured(),
            ..OutcomeVector::default()
        };
        let treatment = OutcomeVector {
            physical_loss_l: QualifiedMetric::unsupported(),
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control, treatment).unwrap();

        assert_eq!(receipt.delta.physical_loss_l.value, None);
        assert_eq!(
            receipt.delta.physical_loss_l.status,
            MetricDeltaStatus::BothMissing {
                control: MetricEvidenceStatus::NotMeasured,
                treatment: MetricEvidenceStatus::Unsupported
            }
        );
    }

    #[test]
    fn outcome_receipt_rejects_status_value_mismatch() {
        let invalid = OutcomeVector {
            physical_loss_l: QualifiedMetric {
                value: None,
                status: MetricEvidenceStatus::Observed,
            },
            ..OutcomeVector::default()
        };
        assert_eq!(
            PairedOutcomeReceipt::new(OutcomeVector::default(), invalid),
            Err(OutcomeError::StatusValueMismatch {
                field: "physical_loss_l"
            })
        );
    }

    #[test]
    fn outcome_receipt_rejects_out_of_range_service_value() {
        let invalid = OutcomeVector {
            worst_group_service_ppm: QualifiedMetric::observed(1_000_001),
            ..OutcomeVector::default()
        };
        assert_eq!(
            PairedOutcomeReceipt::new(OutcomeVector::default(), invalid),
            Err(OutcomeError::WorstGroupServiceOutOfRange { value: 1_000_001 })
        );
    }

    fn decision_chain_events() -> Vec<EvidenceEvent> {
        let stages = [
            EvidenceStage::Observation,
            EvidenceStage::StateEstimate,
            EvidenceStage::Recommendation,
            EvidenceStage::HumanDecision,
            EvidenceStage::WorkOrder,
            EvidenceStage::IndependentVerification,
            EvidenceStage::ServiceOutcome,
            EvidenceStage::ModelDiscrepancyReview,
        ];
        let ids = [
            "obs",
            "estimate",
            "recommendation",
            "decision",
            "work",
            "verify",
            "outcome",
            "review",
        ];
        stages
            .iter()
            .enumerate()
            .map(|(index, stage)| {
                let sequence = u64::try_from(index).unwrap() + 1;
                let digest_byte = u8::try_from(index).unwrap() + 1;
                EvidenceEvent::new(
                    ids[index],
                    *stage,
                    sequence * 1000,
                    if index == 0 {
                        None
                    } else {
                        Some(ids[index - 1].to_owned())
                    },
                    digest(digest_byte),
                    if *stage == EvidenceStage::HumanDecision {
                        Some(HumanDecisionDisposition::Authorized)
                    } else {
                        None
                    },
                )
            })
            .collect()
    }

    #[test]
    fn decision_service_chain_requires_complete_verified_closure() {
        let chain = DecisionServiceChain::new(decision_chain_events()).unwrap();
        assert_eq!(
            chain.status(),
            DecisionServiceStatus::VerifiedServiceOutcome
        );
        assert_eq!(chain.events().len(), 8);
        assert_eq!(
            chain.events().last().unwrap().stage,
            EvidenceStage::ModelDiscrepancyReview
        );
    }

    #[test]
    fn rejected_or_deferred_decision_is_terminal_without_claimed_action() {
        let mut events = decision_chain_events();
        events[3].disposition = Some(HumanDecisionDisposition::Rejected);
        events.truncate(4);
        let chain = DecisionServiceChain::new(events).unwrap();
        assert_eq!(chain.status(), DecisionServiceStatus::NotAuthorized);

        let mut deferred = decision_chain_events();
        deferred[3].disposition = Some(HumanDecisionDisposition::Deferred);
        deferred.truncate(4);
        assert_eq!(
            DecisionServiceChain::new(deferred).unwrap().status(),
            DecisionServiceStatus::NotAuthorized
        );
    }

    #[test]
    fn decision_chain_rejects_action_after_rejection() {
        let mut events = decision_chain_events();
        events[3].disposition = Some(HumanDecisionDisposition::Rejected);
        assert_eq!(
            DecisionServiceChain::new(events),
            Err(DecisionChainError::ActionAfterNonAuthorization)
        );
    }

    #[test]
    fn decision_chain_rejects_broken_lineage_and_time_regression() {
        let mut broken_parent = decision_chain_events();
        broken_parent[1].predecessor_event_id = None;
        assert_eq!(
            DecisionServiceChain::new(broken_parent),
            Err(DecisionChainError::BrokenLineage("estimate".to_owned()))
        );

        let mut time_regression = decision_chain_events();
        time_regression[5].occurred_at_unix_ms = 4_500;
        assert_eq!(
            DecisionServiceChain::new(time_regression),
            Err(DecisionChainError::TimestampRegression {
                previous_ms: 5_000,
                current_ms: 4_500
            })
        );
    }

    #[test]
    fn decision_chain_rejects_duplicate_ids_zero_digests_and_wrong_stage_order() {
        let mut duplicate = decision_chain_events();
        duplicate[6].event_id = "work".to_owned();
        assert_eq!(
            DecisionServiceChain::new(duplicate),
            Err(DecisionChainError::DuplicateEventId("work".to_owned()))
        );

        let mut zero_digest = decision_chain_events();
        zero_digest[2].evidence_digest = digest(0);
        assert_eq!(
            DecisionServiceChain::new(zero_digest),
            Err(DecisionChainError::ZeroEvidenceDigest(
                "recommendation".to_owned()
            ))
        );

        let mut wrong_stage = decision_chain_events();
        wrong_stage[5].stage = EvidenceStage::ServiceOutcome;
        assert_eq!(
            DecisionServiceChain::new(wrong_stage),
            Err(DecisionChainError::InvalidStageOrder { index: 5 })
        );
    }

    #[test]
    fn provenance_receipt_requires_exact_execution_identity() {
        let digests = ProvenanceDigests {
            protocol: digest(1),
            schema: digest(2),
            source_inventory: digest(3),
            source_snapshots: digest(4),
            model: digest(5),
            topology: digest(6),
            scenario_generator: digest(7),
            scenario: digest(8),
            policy: digest(9),
            evaluator: digest(10),
            output: digest(11),
        };
        let identity = ExecutionIdentity {
            code_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            solver_version: "civbench-experiment-core/0.1.0".to_owned(),
            runtime_environment: digest(12),
        };
        assert!(ProvenanceReceipt::new(digests, identity, SourceSnapshotStatus::Frozen).is_ok());

        let missing_commit = ExecutionIdentity {
            code_commit: String::new(),
            solver_version: "civbench-experiment-core/0.1.0".to_owned(),
            runtime_environment: digest(12),
        };
        assert_eq!(
            ProvenanceReceipt::new(digests, missing_commit, SourceSnapshotStatus::Frozen),
            Err(ReceiptError::EmptyCodeCommit)
        );

        let invalid_commit = ExecutionIdentity {
            code_commit: "not-a-commit".to_owned(),
            solver_version: "civbench-experiment-core/0.1.0".to_owned(),
            runtime_environment: digest(12),
        };
        assert_eq!(
            ProvenanceReceipt::new(digests, invalid_commit, SourceSnapshotStatus::Frozen),
            Err(ReceiptError::InvalidCodeCommit)
        );

        let identity = ExecutionIdentity {
            code_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            solver_version: "civbench-experiment-core/0.1.0".to_owned(),
            runtime_environment: digest(12),
        };
        assert_eq!(
            ProvenanceReceipt::new(
                digests,
                identity.clone(),
                SourceSnapshotStatus::NotSnapshotted
            ),
            Err(ReceiptError::SourceSnapshotNotFrozen)
        );

        let zero_digest_set = ProvenanceDigests {
            source_snapshots: digest(0),
            ..digests
        };
        assert_eq!(
            ProvenanceReceipt::new(zero_digest_set, identity, SourceSnapshotStatus::Frozen),
            Err(ReceiptError::ZeroDigest)
        );
    }
}
