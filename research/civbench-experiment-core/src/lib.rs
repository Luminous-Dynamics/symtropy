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
use std::collections::BTreeSet;

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
        /// Already committed quantity.
        committed: u64,
        /// New requested quantity.
        requested: u64,
    },
    /// Adding resource quantities overflowed the integer representation.
    ArithmeticOverflow(ResourceKind),
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
            Self::ArithmeticOverflow(kind) => {
                write!(f, "resource arithmetic overflow for {kind:?}")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

/// A deterministic resource ledger for one policy arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLedger {
    available: ResourceEnvelope,
    committed: ResourceEnvelope,
    commitment_ids: BTreeSet<String>,
}

impl ResourceLedger {
    /// Create a ledger with a finite available-resource envelope.
    pub fn new(available: ResourceEnvelope) -> Self {
        Self {
            available,
            committed: ResourceEnvelope::zero(),
            commitment_ids: BTreeSet::new(),
        }
    }

    /// Return the available resource envelope.
    pub const fn available(&self) -> ResourceEnvelope {
        self.available
    }

    /// Return the currently committed quantities.
    pub const fn committed(&self) -> ResourceEnvelope {
        self.committed
    }

    /// Return the number of unique commitments.
    pub fn commitment_count(&self) -> usize {
        self.commitment_ids.len()
    }

    /// Commit a resource bundle atomically.
    ///
    /// Duplicate identifiers, arithmetic overflow and over-budget requests
    /// leave the ledger unchanged.
    pub fn commit(
        &mut self,
        commitment_id: impl Into<String>,
        request: ResourceEnvelope,
    ) -> Result<(), LedgerError> {
        let id = commitment_id.into();
        let id = id.trim().to_owned();
        if id.is_empty() {
            return Err(LedgerError::EmptyCommitmentId);
        }
        if self.commitment_ids.contains(&id) {
            return Err(LedgerError::DuplicateCommitment(id));
        }

        let mut next_values = *self.committed.values();
        for kind in ResourceKind::ALL {
            let index = kind.index();
            let committed = self.committed.get(kind);
            let requested = request.get(kind);
            let next = committed
                .checked_add(requested)
                .ok_or(LedgerError::ArithmeticOverflow(kind))?;
            let available = self.available.get(kind);
            if next > available {
                return Err(LedgerError::ExceedsAvailable {
                    kind,
                    available,
                    committed,
                    requested,
                });
            }
            next_values[index] = next;
        }

        // Commit only after every dimension has been checked.
        self.committed = ResourceEnvelope::from_values(next_values);
        self.commitment_ids.insert(id);
        Ok(())
    }
}

/// Typed, non-composite outcome vector from one evaluated scenario arm.
///
/// Values use explicit integer units to avoid introducing floating-point
/// rounding into the paired-difference receipt. The caller is responsible for
/// supplying values under the protocol's declared metric definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutcomeVector {
    /// Unmet essential water demand, in litres.
    pub essential_shortfall_l: u64,
    /// Service interruption exposure, in service-unit seconds.
    pub interruption_service_seconds: u64,
    /// Worst-group service performance, in parts per million (0..=1,000,000).
    pub worst_group_service_ppm: u64,
    /// Physical water loss, in litres.
    pub physical_loss_l: u64,
    /// Commercial/apparent loss, in litres.
    pub commercial_loss_l: u64,
    /// Unbilled authorised consumption, in litres.
    pub unbilled_authorised_l: u64,
    /// Asset failures in the declared observation window.
    pub asset_failures: u64,
    /// Independently verified completed repairs.
    pub verified_repairs: u64,
    /// Whole-life cost, in the declared smallest currency unit and price year.
    pub whole_life_cost_minor: u64,
    /// Time to qualified recovery, in seconds.
    pub recovery_seconds: u64,
    /// Qualified fallback water delivered, in litres.
    pub safe_fallback_delivery_l: u64,
    /// False dispatches in the declared observation window.
    pub false_dispatches: u64,
}

/// Signed outcome difference, defined as treatment minus control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutcomeDelta {
    /// Change in unmet essential water demand, in litres.
    pub essential_shortfall_l: i128,
    /// Change in service interruption exposure, in service-unit seconds.
    pub interruption_service_seconds: i128,
    /// Change in worst-group service performance, in parts per million.
    pub worst_group_service_ppm: i128,
    /// Change in physical water loss, in litres.
    pub physical_loss_l: i128,
    /// Change in commercial/apparent loss, in litres.
    pub commercial_loss_l: i128,
    /// Change in unbilled authorised consumption, in litres.
    pub unbilled_authorised_l: i128,
    /// Change in asset failures.
    pub asset_failures: i128,
    /// Change in independently verified completed repairs.
    pub verified_repairs: i128,
    /// Change in whole-life cost, in the declared smallest currency unit.
    pub whole_life_cost_minor: i128,
    /// Change in time to qualified recovery, in seconds.
    pub recovery_seconds: i128,
    /// Change in qualified fallback water delivered, in litres.
    pub safe_fallback_delivery_l: i128,
    /// Change in false dispatches.
    pub false_dispatches: i128,
}

/// Error returned when an outcome vector violates a declared value domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeError {
    /// Worst-group service performance must be between zero and one million ppm.
    WorstGroupServiceOutOfRange { value: u64 },
}

impl fmt::Display for OutcomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorstGroupServiceOutOfRange { value } => write!(
                f,
                "worst-group service value {value} ppm exceeds the 0..=1000000 domain"
            ),
        }
    }
}

impl std::error::Error for OutcomeError {}

impl OutcomeVector {
    /// Validate the bounded fields in the outcome vector.
    pub fn validate(&self) -> Result<(), OutcomeError> {
        if self.worst_group_service_ppm > 1_000_000 {
            return Err(OutcomeError::WorstGroupServiceOutOfRange {
                value: self.worst_group_service_ppm,
            });
        }
        Ok(())
    }
}

impl OutcomeDelta {
    /// Compute treatment minus control for every outcome, preserving sign.
    pub fn between(control: OutcomeVector, treatment: OutcomeVector) -> Self {
        Self {
            essential_shortfall_l: delta(treatment.essential_shortfall_l, control.essential_shortfall_l),
            interruption_service_seconds: delta(
                treatment.interruption_service_seconds,
                control.interruption_service_seconds,
            ),
            worst_group_service_ppm: delta(
                treatment.worst_group_service_ppm,
                control.worst_group_service_ppm,
            ),
            physical_loss_l: delta(treatment.physical_loss_l, control.physical_loss_l),
            commercial_loss_l: delta(treatment.commercial_loss_l, control.commercial_loss_l),
            unbilled_authorised_l: delta(
                treatment.unbilled_authorised_l,
                control.unbilled_authorised_l,
            ),
            asset_failures: delta(treatment.asset_failures, control.asset_failures),
            verified_repairs: delta(treatment.verified_repairs, control.verified_repairs),
            whole_life_cost_minor: delta(
                treatment.whole_life_cost_minor,
                control.whole_life_cost_minor,
            ),
            recovery_seconds: delta(treatment.recovery_seconds, control.recovery_seconds),
            safe_fallback_delivery_l: delta(
                treatment.safe_fallback_delivery_l,
                control.safe_fallback_delivery_l,
            ),
            false_dispatches: delta(treatment.false_dispatches, control.false_dispatches),
        }
    }
}

fn delta(treatment: u64, control: u64) -> i128 {
    i128::from(treatment) - i128::from(control)
}

/// A paired, component-wise outcome receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedOutcomeReceipt {
    /// Control outcome vector.
    pub control: OutcomeVector,
    /// Treatment outcome vector.
    pub treatment: OutcomeVector,
    /// Signed treatment-minus-control difference.
    pub delta: OutcomeDelta,
}

impl PairedOutcomeReceipt {
    /// Construct a receipt with its signed difference calculated internally.
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
        let control = arm("reactive_only", resources);
        let treatment = arm("risk_prioritised", resources);
        assert_eq!(
            PairedScenarioBlock::new(control.clone(), treatment.clone(), &[], digest(99)),
            Err(PairError::ZeroScenarioDigest)
        );

        let mut valid_control = control.clone();
        valid_control.exogenous_digest = digest(1);
        let mut valid_treatment = treatment.clone();
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
            ledger.commit("same-id", zero),
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
    fn outcome_receipt_preserves_signed_component_deltas() {
        let control = OutcomeVector {
            essential_shortfall_l: 1000,
            worst_group_service_ppm: 800_000,
            false_dispatches: 10,
            ..OutcomeVector::default()
        };
        let treatment = OutcomeVector {
            essential_shortfall_l: 700,
            worst_group_service_ppm: 850_000,
            false_dispatches: 14,
            ..OutcomeVector::default()
        };
        let receipt = PairedOutcomeReceipt::new(control, treatment).unwrap();

        assert_eq!(receipt.delta.essential_shortfall_l, -300);
        assert_eq!(receipt.delta.worst_group_service_ppm, 50_000);
        assert_eq!(receipt.delta.false_dispatches, 4);
    }

    #[test]
    fn outcome_receipt_rejects_out_of_range_service_value() {
        let invalid = OutcomeVector {
            worst_group_service_ppm: 1_000_001,
            ..OutcomeVector::default()
        };
        assert_eq!(
            PairedOutcomeReceipt::new(OutcomeVector::default(), invalid),
            Err(OutcomeError::WorstGroupServiceOutOfRange { value: 1_000_001 })
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
