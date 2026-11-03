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
            Self::InvalidLength => write!(f, "SHA-256 digest must contain 64 hexadecimal characters"),
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
            let high = decode_hex(pair[0]).ok_or(
                DigestError::InvalidHexCharacter { index: index * 2 },
            )?;
            let low = decode_hex(pair[1]).ok_or(
                DigestError::InvalidHexCharacter {
                    index: index * 2 + 1,
                },
            )?;
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
            Self::ExogenousScheduleMismatch => write!(f, "paired arms use different exogenous schedules"),
            Self::InitialStateMismatch => write!(f, "paired arms use different initial states"),
            Self::TopologyMismatch => write!(f, "paired arms use different topologies"),
            Self::ModelFamilyMismatch => write!(f, "paired arms use different model families"),
            Self::TimeGridMismatch => write!(f, "paired arms use different time grids"),
            Self::HorizonMismatch => write!(f, "paired arms use different horizons"),
            Self::FactorMismatch => write!(f, "paired arms vary different intervention factors"),
            Self::SameFactorLevel => write!(f, "control and treatment use the same factor level"),
            Self::UndeclaredResourceDifference(kind) => {
                write!(f, "resource {kind:?} differs without a declared treatment exception")
            }
            Self::ZeroScenarioDigest => write!(f, "required scenario digest is an all-zero unset sentinel"),
            Self::ZeroEvaluatorTruthDigest => write!(f, "evaluator-only truth digest is an all-zero unset sentinel"),
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
            candidate_view: CandidateScenarioView {
                control,
                treatment,
            },
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
            Self::ExceedsAvailable { kind, available, committed, requested } => write!(
                f,
                "resource {kind:?} exceeds available envelope: available={available}, committed={committed}, requested={requested}"
            ),
            Self::ActualExceedsReservation { kind, reserved, actual } => write!(
                f,
                "actual resource {kind:?} exceeds reservation: reserved={reserved}, actual={actual}"
            ),
            Self::ArithmeticOverflow(kind) => write!(f, "resource arithmetic overflow for {kind:?}"),
            Self::UnknownCommitment(id) => write!(f, "unknown resource commitment: {id}"),
            Self::InvalidTransition { id, state } => {
                write!(f, "resource commitment {id} cannot transition from {state:?}")
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
    pub const fn available(&self) -> ResourceEnvelope { self.available }

    /// Return resources reserved for planned but unsettled work.
    pub const fn reserved(&self) -> ResourceEnvelope { self.reserved }

    /// Return actual resource consumption recorded by settled commitments.
    pub const fn consumed(&self) -> ResourceEnvelope { self.consumed }

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
    pub fn commitment_count(&self) -> usize { self.commitments.len() }

    /// Return one commitment's lifecycle state, if it exists.
    pub fn commitment_state(&self, commitment_id: &str) -> Option<ResourceCommitmentState> {
        self.commitments.get(commitment_id.trim()).map(|record| record.state)
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
        if id.is_empty() { return Err(LedgerError::EmptyCommitmentId); }
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
            let next = reserved.checked_add(requested)
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
        self.commitments.insert(id, ResourceCommitmentRecord {
            reserved: request,
            settled: None,
            state: ResourceCommitmentState::Reserved,
        });
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
        let record = self.commitments.get(id).copied()
            .ok_or_else(|| LedgerError::UnknownCommitment(id.to_owned()))?;
        if record.state != ResourceCommitmentState::Reserved {
            return Err(LedgerError::InvalidTransition { id: id.to_owned(), state: record.state });
        }

        let mut next_reserved = *self.reserved.values();
        let mut next_consumed = *self.consumed.values();
        for kind in ResourceKind::ALL {
            let index = kind.index();
            let reserved = record.reserved.get(kind);
            let actual_value = actual.get(kind);
            if actual_value > reserved {
                return Err(LedgerError::ActualExceedsReservation {
                    kind, reserved, actual: actual_value,
                });
            }
            next_reserved[index] = self.reserved.get(kind) - reserved;
            next_consumed[index] = self.consumed.get(kind).checked_add(actual_value)
                .ok_or(LedgerError::ArithmeticOverflow(kind))?;
        }

        self.reserved = ResourceEnvelope::from_values(next_reserved);
        self.consumed = ResourceEnvelope::from_values(next_consumed);
        self.commitments.insert(id.to_owned(), ResourceCommitmentRecord {
            reserved: record.reserved,
            settled: Some(actual),
            state: ResourceCommitmentState::Settled,
        });
        Ok(())
    }

    /// Release a reservation for work that was cancelled or not undertaken.
    pub fn release(&mut self, commitment_id: &str) -> Result<(), LedgerError> {
        let id = commitment_id.trim();
        let record = self.commitments.get(id).copied()
            .ok_or_else(|| LedgerError::UnknownCommitment(id.to_owned()))?;
        if record.state != ResourceCommitmentState::Reserved {
            return Err(LedgerError::InvalidTransition { id: id.to_owned(), state: record.state });
        }

        let mut next_reserved = *self.reserved.values();
        for kind in ResourceKind::ALL {
            next_reserved[kind.index()] =
                self.reserved.get(kind) - record.reserved.get(kind);
        }
        self.reserved = ResourceEnvelope::from_values(next_reserved);
        self.commitments.insert(id.to_owned(), ResourceCommitmentRecord {
            reserved: record.reserved,
            settled: None,
            state: ResourceCommitmentState::Released,
        });
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
            let next = consumed.checked_add(requested)
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
        self.commitments.insert(id, ResourceCommitmentRecord {
            reserved: ResourceEnvelope::zero(),
            settled: Some(actual),
            state: ResourceCommitmentState::Settled,
        });
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
            (Some(_), Some(_)) if control.status != self.status => {
                MetricDelta {
                    value: None,
                    status: MetricDeltaStatus::EvidenceStatusMismatch {
                        control: control.status,
                        treatment: self.status,
                    },
                }
            }
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
                write!(f, "outcome field {field} has an invalid status/value combination")
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
        self.essential_shortfall_l.validate("essential_shortfall_l")?;
        self.interruption_service_seconds
            .validate("interruption_service_seconds")?;
        self.worst_group_service_ppm
            .validate("worst_group_service_ppm")?;
        self.physical_loss_l.validate("physical_loss_l")?;
        self.commercial_loss_l.validate("commercial_loss_l")?;
        self.unbilled_authorised_l.validate("unbilled_authorised_l")?;
        self.asset_failures.validate("asset_failures")?;
        self.verified_repairs.validate("verified_repairs")?;
        self.whole_life_cost_minor.validate("whole_life_cost_minor")?;
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
            asset_failures: treatment.asset_failures.paired_delta(control.asset_failures),
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
    pub fn new(
        control: OutcomeVector,
        treatment: OutcomeVector,
    ) -> Result<Self, OutcomeError> {
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
            Self::InvalidCodeCommit => write!(f, "code commit must be a 40- or 64-character hexadecimal object ID"),
            Self::EmptySolverVersion => write!(f, "solver/runtime version is required"),
            Self::ZeroDigest => write!(f, "required provenance digest is an all-zero unset sentinel"),
            Self::SourceSnapshotNotFrozen => write!(f, "source bytes must be snapshotted before a confirmatory receipt can be created"),
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
            || !execution.code_commit.bytes().all(|byte| decode_hex(byte).is_some())
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
            Self::TimestampRegression { previous_ms, current_ms } => write!(
                f,
                "event time regressed from {previous_ms} ms to {current_ms} ms"
            ),
            Self::RootHasPredecessor => write!(f, "root observation must not have a predecessor"),
            Self::BrokenLineage(id) => write!(f, "event {id} does not link to its immediate predecessor"),
            Self::IncompleteChain => write!(f, "decision-to-service evidence chain is incomplete"),
            Self::InvalidStageOrder { index } => write!(f, "invalid evidence stage at chain position {index}"),
            Self::MissingHumanDisposition => write!(f, "human decision event lacks an explicit disposition"),
            Self::UnexpectedHumanDisposition(id) => {
                write!(f, "non-decision event {id} carries a human decision disposition")
            }
            Self::ActionAfterNonAuthorization => {
                write!(f, "rejected or deferred recommendation cannot be followed by an operational action")
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
                return Err(DecisionChainError::ZeroEvidenceDigest(event.event_id.clone()));
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
        let control = arm("reactive_only", envelope([1000, 100, 100, 600, 30, 300, 500, 2]));
        let treatment = arm("risk_prioritised", envelope([1000, 100, 100, 480, 30, 300, 500, 2]));
        let block = PairedScenarioBlock::new(
            control,
            treatment,
            &[ResourceKind::CrewMinutes],
            digest(99),
        )
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
        let control = arm("reactive_only", envelope([1000, 100, 100, 600, 30, 300, 500, 2]));
        let mut treatment = arm("risk_prioritised", control.resources);
        treatment.exogenous_digest = digest(8);

        assert_eq!(
            PairedScenarioBlock::new(control, treatment, &[], digest(99)),
            Err(PairError::ExogenousScheduleMismatch)
        );
    }

    #[test]
    fn paired_block_rejects_undeclared_resource_envelope_difference() {
        let control = arm("reactive_only", envelope([1000, 100, 100, 600, 30, 300, 500, 2]));
        let treatment = arm("risk_prioritised", envelope([1000, 100, 90, 600, 30, 300, 500, 2]));

        assert_eq!(
            PairedScenarioBlock::new(control, treatment, &[], digest(99)),
            Err(PairError::UndeclaredResourceDifference(ResourceKind::OperatingMinor))
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
        assert_eq!(ledger.commitment_state("work-1"), Some(ResourceCommitmentState::Reserved));

        ledger.settle("work-1", actual).unwrap();
        assert_eq!(ledger.reserved(), ResourceEnvelope::zero());
        assert_eq!(ledger.consumed(), actual);
        assert_eq!(ledger.remaining().get(ResourceKind::CrewMinutes), 510);
        assert_eq!(ledger.commitment_state("work-1"), Some(ResourceCommitmentState::Settled));
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
        assert_eq!(ledger.commitment_state("cancelled-work"), Some(ResourceCommitmentState::Released));
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
        let ids = ["obs", "estimate", "recommendation", "decision", "work", "verify", "outcome", "review"];
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
        assert_eq!(chain.status(), DecisionServiceStatus::VerifiedServiceOutcome);
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
            Err(DecisionChainError::ZeroEvidenceDigest("recommendation".to_owned()))
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
        assert!(ProvenanceReceipt::new(
            digests,
            identity,
            SourceSnapshotStatus::Frozen
        ).is_ok());

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
            ProvenanceReceipt::new(digests, identity.clone(), SourceSnapshotStatus::NotSnapshotted),
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
