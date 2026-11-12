//! Evidence-gated, offline inter-pass recommendation contract.
//!
//! V0 accepts synthetic contract observations only. It does not parse BAM raw
//! files, qualify a welding procedure, predict physical weld quality, or
//! authorize robot execution. Every numeric objective is a symbolic unit.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredGeometry,
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcedureScope {
    SyntheticEnvelopeOnly,
    UnverifiedPhysicalProcedure,
    QualifiedPhysicalProcedure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanQuality {
    pub registered: bool,
    pub coverage_permille: u16,
    pub minimum_coverage_permille: u16,
    pub occlusion_permille: u16,
    pub maximum_occlusion_permille: u16,
    pub registration_error_units: u64,
    pub maximum_registration_error_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedState {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub after_pass_id: String,
    pub scan_id: String,
    pub scan_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub coordinate_frame: String,
    pub residual_fill_area_units: u64,
    pub uncertainty_bound_units: u64,
    pub observed_at_tick: u64,
    pub quality: ScanQuality,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcedureEnvelope {
    pub procedure_id: String,
    pub revision: String,
    pub scope: ProcedureScope,
    pub minimum_deposition_area_units: u64,
    pub maximum_deposition_area_units: u64,
    pub live_execution_authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassCandidate {
    pub candidate_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub source_scan_id: String,
    pub source_scan_sha256: [u8; 32],
    pub source_registration_receipt_sha256: [u8; 32],
    pub procedure_id: String,
    pub procedure_revision: String,
    pub deposition_area_units: u64,
    pub duration_units: u64,
    pub material_units: u64,
    pub energy_units: u64,
    pub robustness_risk_units: u64,
    pub within_declared_procedure_envelope: bool,
    pub resource_feasible: bool,
    pub path_reachable: bool,
    pub collision_free: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerInput {
    pub observed_state: ObservedState,
    pub active_workpiece_id: String,
    pub active_joint_revision: String,
    pub active_layer_id: String,
    pub expected_predecessor_pass_id: String,
    pub decision_at_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub maximum_accepted_uncertainty_units: u64,
    pub future_information_used: bool,
    pub model_domain_supported: bool,
    pub live_execution_requested: bool,
    pub procedure: ProcedureEnvelope,
    pub candidates: Vec<PassCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    PhysicalOrSimulationEvidenceNotAdmitted,
    ScanNotRegistered,
    ScanDigestMissing,
    RegistrationReceiptMissing,
    ScanCoverageBelowMinimum,
    ScanOcclusionAboveMaximum,
    ScanRegistrationErrorTooLarge,
    UncertaintyNotBounded,
    UncertaintyExceedsDecisionLimit,
    StalePredecessorState,
    ObservationFromFuture,
    ObservationTooOld,
    FutureInformationLeakage,
    ModelOutsideDeclaredDomain,
    ProcedureScopeNotAdmitted,
    LiveExecutionForbidden,
    InvalidProcedureEnvelope,
    DuplicateCandidateIdentity,
    NoFeasibleCandidate,
    NumericOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateExclusion {
    IdentityMismatch,
    WrongSourceScan,
    WrongProcedureRevision,
    OutsideDeclaredProcedureEnvelope,
    ResourceInfeasible,
    PathUnreachable,
    Collision,
    ZeroDuration,
    ZeroMaterial,
    DepositionOutsideProcedureBounds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveVector {
    pub robust_geometry_error_units: u64,
    pub duration_units: u64,
    pub material_units: u64,
    pub energy_units: u64,
    pub robustness_risk_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recommendation {
    pub candidate_id: String,
    pub objectives: ObjectiveVector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedCandidate {
    pub candidate_id: String,
    pub reason: CandidateExclusion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Abstain {
        reason: AbstainReason,
        excluded: Vec<ExcludedCandidate>,
    },
    ParetoRecommendations {
        recommendations: Vec<Recommendation>,
        feasible_candidate_count: usize,
        excluded: Vec<ExcludedCandidate>,
        automatic_selection: bool,
        live_execution_authorized: bool,
        claim_ceiling: &'static str,
    },
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn validate_observation(input: &PlannerInput) -> Result<(), AbstainReason> {
    let state = &input.observed_state;
    let identities = [
        state.workpiece_id.as_str(),
        state.joint_revision.as_str(),
        state.layer_id.as_str(),
        state.after_pass_id.as_str(),
        state.scan_id.as_str(),
        state.coordinate_frame.as_str(),
        input.active_workpiece_id.as_str(),
        input.active_joint_revision.as_str(),
        input.active_layer_id.as_str(),
        input.expected_predecessor_pass_id.as_str(),
    ];
    if identities.into_iter().any(is_empty) {
        return Err(AbstainReason::EmptyIdentity);
    }

    if state.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Err(AbstainReason::PhysicalOrSimulationEvidenceNotAdmitted);
    }
    if input.procedure.scope != ProcedureScope::SyntheticEnvelopeOnly {
        return Err(AbstainReason::ProcedureScopeNotAdmitted);
    }
    if input.procedure.live_execution_authorized || input.live_execution_requested {
        return Err(AbstainReason::LiveExecutionForbidden);
    }
    if !input.model_domain_supported {
        return Err(AbstainReason::ModelOutsideDeclaredDomain);
    }
    if input.future_information_used {
        return Err(AbstainReason::FutureInformationLeakage);
    }

    if state.workpiece_id != input.active_workpiece_id
        || state.joint_revision != input.active_joint_revision
        || state.layer_id != input.active_layer_id
        || state.after_pass_id != input.expected_predecessor_pass_id
    {
        return Err(AbstainReason::StalePredecessorState);
    }
    if state.observed_at_tick > input.decision_at_tick {
        return Err(AbstainReason::ObservationFromFuture);
    }
    if input
        .decision_at_tick
        .saturating_sub(state.observed_at_tick)
        > input.maximum_observation_age_ticks
    {
        return Err(AbstainReason::ObservationTooOld);
    }

    if !state.quality.registered {
        return Err(AbstainReason::ScanNotRegistered);
    }
    if state.scan_sha256.iter().all(|byte| *byte == 0) {
        return Err(AbstainReason::ScanDigestMissing);
    }
    if state
        .registration_receipt_sha256
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(AbstainReason::RegistrationReceiptMissing);
    }
    if state.quality.minimum_coverage_permille > 1_000
        || state.quality.coverage_permille > 1_000
        || state.quality.coverage_permille < state.quality.minimum_coverage_permille
    {
        return Err(AbstainReason::ScanCoverageBelowMinimum);
    }
    if state.quality.maximum_occlusion_permille > 1_000
        || state.quality.occlusion_permille > 1_000
        || state.quality.occlusion_permille > state.quality.maximum_occlusion_permille
    {
        return Err(AbstainReason::ScanOcclusionAboveMaximum);
    }
    if state.quality.registration_error_units > state.quality.maximum_registration_error_units {
        return Err(AbstainReason::ScanRegistrationErrorTooLarge);
    }
    if input.procedure.procedure_id.trim().is_empty()
        || input.procedure.revision.trim().is_empty()
        || input.procedure.minimum_deposition_area_units
            > input.procedure.maximum_deposition_area_units
    {
        return Err(AbstainReason::InvalidProcedureEnvelope);
    }

    if state.uncertainty_bound_units == 0 {
        return Err(AbstainReason::UncertaintyNotBounded);
    }
    if state.uncertainty_bound_units > input.maximum_accepted_uncertainty_units {
        return Err(AbstainReason::UncertaintyExceedsDecisionLimit);
    }
    state
        .residual_fill_area_units
        .checked_add(state.uncertainty_bound_units)
        .ok_or(AbstainReason::NumericOverflow)?;
    Ok(())
}

fn objectives_for(
    state: &ObservedState,
    candidate: &PassCandidate,
) -> Result<ObjectiveVector, AbstainReason> {
    let low = state
        .residual_fill_area_units
        .saturating_sub(state.uncertainty_bound_units);
    let high = state
        .residual_fill_area_units
        .checked_add(state.uncertainty_bound_units)
        .ok_or(AbstainReason::NumericOverflow)?;
    let underfill = high.abs_diff(candidate.deposition_area_units);
    let overfill = low.abs_diff(candidate.deposition_area_units);
    Ok(ObjectiveVector {
        robust_geometry_error_units: underfill.max(overfill),
        duration_units: candidate.duration_units,
        material_units: candidate.material_units,
        energy_units: candidate.energy_units,
        robustness_risk_units: candidate.robustness_risk_units,
    })
}

fn exclusion_for(input: &PlannerInput, candidate: &PassCandidate) -> Option<CandidateExclusion> {
    let state = &input.observed_state;
    if is_empty(&candidate.candidate_id)
        || candidate.workpiece_id != state.workpiece_id
        || candidate.joint_revision != state.joint_revision
        || candidate.layer_id != state.layer_id
        || candidate.predecessor_pass_id != state.after_pass_id
    {
        return Some(CandidateExclusion::IdentityMismatch);
    }
    if candidate.source_scan_id != state.scan_id
        || candidate.source_scan_sha256 != state.scan_sha256
        || candidate.source_registration_receipt_sha256 != state.registration_receipt_sha256
    {
        return Some(CandidateExclusion::WrongSourceScan);
    }
    if candidate.procedure_id != input.procedure.procedure_id
        || candidate.procedure_revision != input.procedure.revision
    {
        return Some(CandidateExclusion::WrongProcedureRevision);
    }
    if !candidate.within_declared_procedure_envelope {
        return Some(CandidateExclusion::OutsideDeclaredProcedureEnvelope);
    }
    if !candidate.resource_feasible {
        return Some(CandidateExclusion::ResourceInfeasible);
    }
    if !candidate.path_reachable {
        return Some(CandidateExclusion::PathUnreachable);
    }
    if !candidate.collision_free {
        return Some(CandidateExclusion::Collision);
    }
    if candidate.duration_units == 0 {
        return Some(CandidateExclusion::ZeroDuration);
    }
    if candidate.material_units == 0 {
        return Some(CandidateExclusion::ZeroMaterial);
    }
    if candidate.deposition_area_units < input.procedure.minimum_deposition_area_units
        || candidate.deposition_area_units > input.procedure.maximum_deposition_area_units
    {
        return Some(CandidateExclusion::DepositionOutsideProcedureBounds);
    }
    None
}

fn dominates(left: &ObjectiveVector, right: &ObjectiveVector) -> bool {
    let left_values = [
        left.robust_geometry_error_units,
        left.duration_units,
        left.material_units,
        left.energy_units,
        left.robustness_risk_units,
    ];
    let right_values = [
        right.robust_geometry_error_units,
        right.duration_units,
        right.material_units,
        right.energy_units,
        right.robustness_risk_units,
    ];
    let mut strictly_better = false;
    for (left_value, right_value) in left_values.into_iter().zip(right_values) {
        if left_value > right_value {
            return false;
        }
        if left_value < right_value {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Produce a recommendation frontier from the latest admissible synthetic scan.
///
/// The function never picks a single pass automatically and never authorizes
/// live execution. All outputs use symbolic units and are not physical weld
/// predictions, WPS qualification, NDT decisions or production authority.
pub fn recommend_next_pass(input: &PlannerInput) -> Decision {
    if let Err(reason) = validate_observation(input) {
        return Decision::Abstain {
            reason,
            excluded: Vec::new(),
        };
    }

    let mut ids = HashSet::new();
    for candidate in &input.candidates {
        if !ids.insert(candidate.candidate_id.as_str()) {
            return Decision::Abstain {
                reason: AbstainReason::DuplicateCandidateIdentity,
                excluded: Vec::new(),
            };
        }
    }

    let mut feasible = Vec::new();
    let mut excluded = Vec::new();
    for candidate in &input.candidates {
        if let Some(reason) = exclusion_for(input, candidate) {
            excluded.push(ExcludedCandidate {
                candidate_id: candidate.candidate_id.clone(),
                reason,
            });
            continue;
        }
        match objectives_for(&input.observed_state, candidate) {
            Ok(objectives) => feasible.push(Recommendation {
                candidate_id: candidate.candidate_id.clone(),
                objectives,
            }),
            Err(reason) => {
                return Decision::Abstain { reason, excluded };
            }
        }
    }

    if feasible.is_empty() {
        return Decision::Abstain {
            reason: AbstainReason::NoFeasibleCandidate,
            excluded,
        };
    }

    let frontier: Vec<Recommendation> = feasible
        .iter()
        .filter(|candidate| {
            !feasible.iter().any(|other| {
                other.candidate_id != candidate.candidate_id
                    && dominates(&other.objectives, &candidate.objectives)
            })
        })
        .cloned()
        .collect();

    let mut recommendations = frontier;
    recommendations.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    Decision::ParetoRecommendations {
        recommendations,
        feasible_candidate_count: feasible.len(),
        excluded,
        automatic_selection: false,
        live_execution_authorized: false,
        claim_ceiling: "SYNTHETIC_INTERPASS_PLAN_CONTRACT_ONLY",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TemporalEvidenceKind {
    ProcessObservation,
    GeometryScan,
    RegistrationReceipt,
    NdtOutcome,
    RepairDisposition,
    ModelCalibration,
    ModelVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TemporalEvidenceRole {
    Feature,
    OutcomeLabel,
    ModelArtifact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TemporalDecisionPurpose {
    FastProcess,
    MediumNextPass,
    SlowQualityRequalification,
}

impl TemporalDecisionPurpose {
    fn permission_index(self) -> usize {
        match self {
            Self::FastProcess => 0,
            Self::MediumNextPass => 1,
            Self::SlowQualityRequalification => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalEvidenceEvent {
    pub event_id: String,
    pub kind: TemporalEvidenceKind,
    pub role: TemporalEvidenceRole,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: Option<String>,
    pub pass_id: Option<String>,
    pub clock_id: String,
    pub measurement_tick: u64,
    pub first_received_tick: u64,
    pub valid_from_tick: u64,
    pub valid_until_tick: Option<u64>,
    pub timestamp_precision_ticks: u64,
    pub purpose_authorized: [bool; 3],
    pub source_sha256: [u8; 32],
    pub transform_receipt_sha256: [u8; 32],
    pub supersedes_event_id: Option<String>,
    pub revokes_event_id: Option<String>,
    pub model_training_cutoff_tick: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalDecisionContext {
    pub purpose: TemporalDecisionPurpose,
    pub decision_tick: u64,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: Option<String>,
    pub predecessor_pass_id: Option<String>,
    pub clock_id: String,
    pub maximum_timestamp_precision_ticks: u64,
    pub policy_revision: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalExclusionReason {
    FutureMeasurement,
    LateArrival,
    ClockDomainMismatch,
    WorkpieceMismatch,
    JointRevisionMismatch,
    LayerMismatch,
    PassMismatch,
    NotYetValid,
    Expired,
    TimestampPrecisionTooCoarse,
    PurposeNotAuthorized,
    OutcomeLabelLeakage,
    MissingSourceDigest,
    MissingTransformReceipt,
    ModelTrainedAfterDecisionCutoff,
    Superseded,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedTemporalEvent {
    pub event_id: String,
    pub reason: TemporalExclusionReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalSnapshotError {
    EmptyDecisionIdentity,
    EmptyEventIdentity,
    DuplicateEventId(String),
    ReceiptBeforeMeasurement(String),
    InvalidValidityInterval(String),
    MissingSupersessionTarget { event_id: String, target_id: String },
    CrossIdentitySupersession { event_id: String, target_id: String },
    SelfReferentialSupersession(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointInTimeEvidenceSnapshot {
    pub purpose: TemporalDecisionPurpose,
    pub decision_tick: u64,
    pub policy_revision: String,
    pub included: Vec<TemporalEvidenceEvent>,
    pub excluded: Vec<ExcludedTemporalEvent>,
    pub snapshot_sha256: [u8; 32],
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

fn temporal_exclusion(
    event: &TemporalEvidenceEvent,
    decision: &TemporalDecisionContext,
) -> Option<TemporalExclusionReason> {
    if event.measurement_tick > decision.decision_tick {
        return Some(TemporalExclusionReason::FutureMeasurement);
    }
    if event.first_received_tick > decision.decision_tick {
        return Some(TemporalExclusionReason::LateArrival);
    }
    if event.clock_id != decision.clock_id {
        return Some(TemporalExclusionReason::ClockDomainMismatch);
    }
    if event.workpiece_id != decision.workpiece_id {
        return Some(TemporalExclusionReason::WorkpieceMismatch);
    }
    if event.joint_revision != decision.joint_revision {
        return Some(TemporalExclusionReason::JointRevisionMismatch);
    }
    if decision
        .layer_id
        .as_ref()
        .is_some_and(|layer| event.layer_id.as_ref() != Some(layer))
    {
        return Some(TemporalExclusionReason::LayerMismatch);
    }
    if decision
        .predecessor_pass_id
        .as_ref()
        .is_some_and(|pass| event.pass_id.as_ref() != Some(pass))
    {
        return Some(TemporalExclusionReason::PassMismatch);
    }
    if event.valid_from_tick > decision.decision_tick {
        return Some(TemporalExclusionReason::NotYetValid);
    }
    if event
        .valid_until_tick
        .is_some_and(|until| until < decision.decision_tick)
    {
        return Some(TemporalExclusionReason::Expired);
    }
    if event.timestamp_precision_ticks == 0
        || event.timestamp_precision_ticks > decision.maximum_timestamp_precision_ticks
    {
        return Some(TemporalExclusionReason::TimestampPrecisionTooCoarse);
    }
    if !event.purpose_authorized[decision.purpose.permission_index()] {
        return Some(TemporalExclusionReason::PurposeNotAuthorized);
    }
    if event.role == TemporalEvidenceRole::OutcomeLabel
        && decision.purpose != TemporalDecisionPurpose::SlowQualityRequalification
    {
        return Some(TemporalExclusionReason::OutcomeLabelLeakage);
    }
    if is_zero_digest(&event.source_sha256) {
        return Some(TemporalExclusionReason::MissingSourceDigest);
    }
    if is_zero_digest(&event.transform_receipt_sha256) {
        return Some(TemporalExclusionReason::MissingTransformReceipt);
    }
    if event
        .model_training_cutoff_tick
        .is_some_and(|cutoff| cutoff > decision.decision_tick)
    {
        return Some(TemporalExclusionReason::ModelTrainedAfterDecisionCutoff);
    }
    None
}

fn append_canonical_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u64).to_le_bytes());
    output.extend_from_slice(value.as_bytes());
}

fn append_canonical_event(output: &mut Vec<u8>, event: &TemporalEvidenceEvent) {
    append_canonical_string(output, &event.event_id);
    output.push(event.kind as u8);
    output.push(event.role as u8);
    append_canonical_string(output, &event.workpiece_id);
    append_canonical_string(output, &event.joint_revision);
    append_canonical_string(output, event.layer_id.as_deref().unwrap_or(""));
    append_canonical_string(output, event.pass_id.as_deref().unwrap_or(""));
    append_canonical_string(output, &event.clock_id);
    output.extend_from_slice(&event.measurement_tick.to_le_bytes());
    output.extend_from_slice(&event.first_received_tick.to_le_bytes());
    output.extend_from_slice(&event.valid_from_tick.to_le_bytes());
    output.extend_from_slice(&event.valid_until_tick.unwrap_or(u64::MAX).to_le_bytes());
    output.extend_from_slice(&event.timestamp_precision_ticks.to_le_bytes());
    for allowed in event.purpose_authorized {
        output.push(u8::from(allowed));
    }
    output.extend_from_slice(&event.source_sha256);
    output.extend_from_slice(&event.transform_receipt_sha256);
    append_canonical_string(output, event.supersedes_event_id.as_deref().unwrap_or(""));
    append_canonical_string(output, event.revokes_event_id.as_deref().unwrap_or(""));
    output.extend_from_slice(
        &event
            .model_training_cutoff_tick
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut message = input.to_vec();
    let bit_length = (message.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = INITIAL;
    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut digest = [0u8; 32];
    for (index, word) in state.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// Build a point-in-time evidence snapshot from a bitemporal event ledger.
///
/// Both measurement time and first-receipt time must be <= the decision
/// cutoff. Late-arriving observations cannot rewrite an earlier decision.
/// Supersession/revocation only takes effect when that event was itself
/// eligible and known by the cutoff. Output ordering and SHA-256 are stable
/// under input permutation. This function uses synthetic clock ticks in v0.
pub fn build_point_in_time_snapshot(
    events: &[TemporalEvidenceEvent],
    decision: &TemporalDecisionContext,
) -> Result<PointInTimeEvidenceSnapshot, TemporalSnapshotError> {
    if decision.workpiece_id.trim().is_empty()
        || decision.joint_revision.trim().is_empty()
        || decision.clock_id.trim().is_empty()
        || decision.policy_revision.trim().is_empty()
        || decision.maximum_timestamp_precision_ticks == 0
    {
        return Err(TemporalSnapshotError::EmptyDecisionIdentity);
    }

    let mut by_id = std::collections::HashMap::new();
    for event in events {
        if event.event_id.trim().is_empty()
            || event.workpiece_id.trim().is_empty()
            || event.joint_revision.trim().is_empty()
            || event.clock_id.trim().is_empty()
        {
            return Err(TemporalSnapshotError::EmptyEventIdentity);
        }
        if event.first_received_tick < event.measurement_tick {
            return Err(TemporalSnapshotError::ReceiptBeforeMeasurement(
                event.event_id.clone(),
            ));
        }
        if event
            .valid_until_tick
            .is_some_and(|until| until < event.valid_from_tick)
        {
            return Err(TemporalSnapshotError::InvalidValidityInterval(
                event.event_id.clone(),
            ));
        }
        if by_id.insert(event.event_id.as_str(), event).is_some() {
            return Err(TemporalSnapshotError::DuplicateEventId(
                event.event_id.clone(),
            ));
        }
    }

    for event in events {
        for target_id in [
            event.supersedes_event_id.as_deref(),
            event.revokes_event_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if target_id == event.event_id {
                return Err(TemporalSnapshotError::SelfReferentialSupersession(
                    event.event_id.clone(),
                ));
            }
            let Some(target) = by_id.get(target_id) else {
                return Err(TemporalSnapshotError::MissingSupersessionTarget {
                    event_id: event.event_id.clone(),
                    target_id: target_id.to_owned(),
                });
            };
            if target.workpiece_id != event.workpiece_id
                || target.joint_revision != event.joint_revision
            {
                return Err(TemporalSnapshotError::CrossIdentitySupersession {
                    event_id: event.event_id.clone(),
                    target_id: target_id.to_owned(),
                });
            }
            if target.measurement_tick > event.measurement_tick
                || target.first_received_tick > event.first_received_tick
            {
                return Err(TemporalSnapshotError::SelfReferentialSupersession(
                    event.event_id.clone(),
                ));
            }
        }
    }

    let mut eligible = Vec::new();
    let mut excluded = Vec::new();
    for event in events {
        if let Some(reason) = temporal_exclusion(event, decision) {
            excluded.push(ExcludedTemporalEvent {
                event_id: event.event_id.clone(),
                reason,
            });
        } else {
            eligible.push(event.clone());
        }
    }

    let mut revoked = HashSet::new();
    for event in &eligible {
        if let Some(target) = &event.revokes_event_id {
            revoked.insert(target.clone());
        }
    }
    let mut superseded = HashSet::new();
    for event in &eligible {
        if !revoked.contains(&event.event_id) {
            if let Some(target) = &event.supersedes_event_id {
                superseded.insert(target.clone());
            }
        }
    }

    let mut included = Vec::new();
    for event in eligible {
        if revoked.contains(&event.event_id) {
            excluded.push(ExcludedTemporalEvent {
                event_id: event.event_id,
                reason: TemporalExclusionReason::Revoked,
            });
        } else if superseded.contains(&event.event_id) {
            excluded.push(ExcludedTemporalEvent {
                event_id: event.event_id,
                reason: TemporalExclusionReason::Superseded,
            });
        } else {
            included.push(event);
        }
    }

    included.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    excluded.sort_by(|left, right| left.event_id.cmp(&right.event_id));

    let mut canonical = Vec::new();
    canonical.extend_from_slice(b"WELD-POINT-IN-TIME-SNAPSHOT-V1\0");
    canonical.push(decision.purpose as u8);
    canonical.extend_from_slice(&decision.decision_tick.to_le_bytes());
    append_canonical_string(&mut canonical, &decision.workpiece_id);
    append_canonical_string(&mut canonical, &decision.joint_revision);
    append_canonical_string(&mut canonical, decision.layer_id.as_deref().unwrap_or(""));
    append_canonical_string(
        &mut canonical,
        decision.predecessor_pass_id.as_deref().unwrap_or(""),
    );
    append_canonical_string(&mut canonical, &decision.clock_id);
    append_canonical_string(&mut canonical, &decision.policy_revision);
    canonical.extend_from_slice(&decision.maximum_timestamp_precision_ticks.to_le_bytes());
    for event in &included {
        append_canonical_event(&mut canonical, event);
    }
    let snapshot_sha256 = sha256(&canonical);

    Ok(PointInTimeEvidenceSnapshot {
        purpose: decision.purpose,
        decision_tick: decision.decision_tick,
        policy_revision: decision.policy_revision.clone(),
        included,
        excluded,
        snapshot_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> PlannerInput {
        let state = ObservedState {
            evidence_class: EvidenceClass::SyntheticContractOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-03".to_owned(),
            after_pass_id: "PASS-02".to_owned(),
            scan_id: "SCAN-AFTER-PASS-02".to_owned(),
            scan_sha256: [0x11; 32],
            registration_receipt_sha256: [0x22; 32],
            coordinate_frame: "SYNTHETIC-JOINT-FRAME".to_owned(),
            residual_fill_area_units: 100,
            uncertainty_bound_units: 10,
            observed_at_tick: 100,
            quality: ScanQuality {
                registered: true,
                coverage_permille: 995,
                minimum_coverage_permille: 980,
                occlusion_permille: 5,
                maximum_occlusion_permille: 50,
                registration_error_units: 1,
                maximum_registration_error_units: 2,
            },
        };
        let procedure = ProcedureEnvelope {
            procedure_id: "SYNTHETIC-PROC-01".to_owned(),
            revision: "R1".to_owned(),
            scope: ProcedureScope::SyntheticEnvelopeOnly,
            minimum_deposition_area_units: 60,
            maximum_deposition_area_units: 140,
            live_execution_authorized: false,
        };
        let make_candidate = |id: &str, deposition, time, material, energy, risk| PassCandidate {
                candidate_id: id.to_owned(),
                workpiece_id: "WID-SYNTH-01".to_owned(),
                joint_revision: "JOINT-R1".to_owned(),
                layer_id: "LAYER-03".to_owned(),
                predecessor_pass_id: "PASS-02".to_owned(),
                source_scan_id: "SCAN-AFTER-PASS-02".to_owned(),
                source_scan_sha256: [0x11; 32],
                source_registration_receipt_sha256: [0x22; 32],
                procedure_id: "SYNTHETIC-PROC-01".to_owned(),
                procedure_revision: "R1".to_owned(),
                deposition_area_units: deposition,
                duration_units: time,
                material_units: material,
                energy_units: energy,
                robustness_risk_units: risk,
                within_declared_procedure_envelope: true,
                resource_feasible: true,
                path_reachable: true,
                collision_free: true,
            };
        let candidates = vec![
            make_candidate("FILL-UNDER", 82, 12, 82, 70, 1),
            make_candidate("FILL-MID", 100, 15, 100, 80, 2),
            make_candidate("FILL-OVER", 130, 10, 130, 60, 1),
        ];
        PlannerInput {
            observed_state: state,
            active_workpiece_id: "WID-SYNTH-01".to_owned(),
            active_joint_revision: "JOINT-R1".to_owned(),
            active_layer_id: "LAYER-03".to_owned(),
            expected_predecessor_pass_id: "PASS-02".to_owned(),
            decision_at_tick: 100,
            maximum_observation_age_ticks: 10,
            maximum_accepted_uncertainty_units: 30,
            future_information_used: false,
            model_domain_supported: true,
            live_execution_requested: false,
            procedure,
            candidates,
        }
    }

    fn temporal_test_event(
        event_id: &str,
        kind: TemporalEvidenceKind,
        measurement_tick: u64,
        first_received_tick: u64,
    ) -> TemporalEvidenceEvent {
        TemporalEvidenceEvent {
            event_id: event_id.to_owned(),
            kind,
            role: TemporalEvidenceRole::Feature,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: Some("LAYER-03".to_owned()),
            pass_id: Some("PASS-02".to_owned()),
            clock_id: "UTC-MASTER".to_owned(),
            measurement_tick,
            first_received_tick,
            valid_from_tick: 0,
            valid_until_tick: None,
            timestamp_precision_ticks: 1,
            purpose_authorized: [true, true, true],
            source_sha256: [0x11; 32],
            transform_receipt_sha256: [0x22; 32],
            supersedes_event_id: None,
            revokes_event_id: None,
            model_training_cutoff_tick: None,
        }
    }

    fn temporal_test_decision(
        purpose: TemporalDecisionPurpose,
        decision_tick: u64,
    ) -> TemporalDecisionContext {
        TemporalDecisionContext {
            purpose,
            decision_tick,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: Some("LAYER-03".to_owned()),
            predecessor_pass_id: Some("PASS-02".to_owned()),
            clock_id: "UTC-MASTER".to_owned(),
            maximum_timestamp_precision_ticks: 5,
            policy_revision: "TEMPORAL-POLICY-R1".to_owned(),
        }
    }

    #[test]
    fn clock_domain_mismatch_is_excluded() {
        let mut event = temporal_test_event(
            "EV-WRONG-CLOCK",
            TemporalEvidenceKind::ProcessObservation,
            80,
            81,
        );
        event.clock_id = "UNMAPPED-DEVICE-CLOCK".to_owned();
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::FastProcess, 100),
        )
        .expect("unmapped clock is excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::ClockDomainMismatch
        );
    }

    #[test]
    fn expired_evidence_is_excluded() {
        let mut event =
            temporal_test_event("EV-EXPIRED", TemporalEvidenceKind::ModelCalibration, 80, 81);
        event.valid_until_tick = Some(99);
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("expired evidence is excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::Expired
        );
    }

    #[test]
    fn not_yet_valid_evidence_is_excluded() {
        let mut event =
            temporal_test_event("EV-NOT-YET", TemporalEvidenceKind::ModelCalibration, 80, 81);
        event.valid_from_tick = 101;
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("not-yet-valid evidence is excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::NotYetValid
        );
    }

    #[test]
    fn purpose_authorization_is_enforced() {
        let mut event = temporal_test_event(
            "EV-NO-PERMISSION",
            TemporalEvidenceKind::ProcessObservation,
            80,
            81,
        );
        event.purpose_authorized[0] = false;
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::FastProcess, 100),
        )
        .expect("unauthorized purpose is excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::PurposeNotAuthorized
        );
    }

    #[test]
    fn joint_revision_mismatch_is_excluded() {
        let mut event =
            temporal_test_event("EV-OTHER-REV", TemporalEvidenceKind::GeometryScan, 80, 81);
        event.joint_revision = "JOINT-R2".to_owned();
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("wrong joint revision excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::JointRevisionMismatch
        );
    }

    #[test]
    fn layer_mismatch_is_excluded() {
        let mut event =
            temporal_test_event("EV-OTHER-LAYER", TemporalEvidenceKind::GeometryScan, 80, 81);
        event.layer_id = Some("LAYER-02".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("wrong layer excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::LayerMismatch
        );
    }

    #[test]
    fn predecessor_pass_mismatch_is_excluded() {
        let mut event = temporal_test_event(
            "EV-OTHER-PASS",
            TemporalEvidenceKind::ProcessObservation,
            80,
            81,
        );
        event.pass_id = Some("PASS-01".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("wrong predecessor pass excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::PassMismatch
        );
    }

    #[test]
    fn missing_registration_receipt_digest_is_excluded() {
        let mut event = temporal_test_event(
            "EV-NO-TRANSFORM",
            TemporalEvidenceKind::GeometryScan,
            80,
            81,
        );
        event.transform_receipt_sha256 = [0; 32];
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("missing transform receipt is quarantined");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::MissingTransformReceipt
        );
    }

    #[test]
    fn revoked_superseding_scan_does_not_hide_the_prior_scan() {
        let old = temporal_test_event("SCAN-OLD", TemporalEvidenceKind::GeometryScan, 80, 81);
        let mut replacement =
            temporal_test_event("SCAN-NEW", TemporalEvidenceKind::GeometryScan, 90, 91);
        replacement.supersedes_event_id = Some("SCAN-OLD".to_owned());
        let mut revocation =
            temporal_test_event("REV-NEW", TemporalEvidenceKind::RegistrationReceipt, 95, 96);
        revocation.revokes_event_id = Some("SCAN-NEW".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[old, replacement, revocation],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("revoked replacement no longer supersedes the prior scan");
        assert!(
            snapshot
                .included
                .iter()
                .any(|event| event.event_id == "SCAN-OLD")
        );
        assert!(
            !snapshot
                .included
                .iter()
                .any(|event| event.event_id == "SCAN-NEW")
        );
        assert!(snapshot.excluded.iter().any(|event| {
            event.event_id == "SCAN-NEW" && event.reason == TemporalExclusionReason::Revoked
        }));
    }

    #[test]
    fn point_in_time_sha256_matches_the_standard_abc_known_answer() {
        assert_eq!(
            sha256(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

    #[test]
    fn evidence_measured_and_received_at_cutoff_is_eligible() {
        let event = temporal_test_event(
            "EV-PROCESS-01",
            TemporalEvidenceKind::ProcessObservation,
            100,
            100,
        );
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("valid point-in-time snapshot");
        assert_eq!(snapshot.included.len(), 1);
        assert!(snapshot.excluded.is_empty());
    }

    #[test]
    fn late_arriving_measurement_is_not_backfilled_into_history() {
        let event = temporal_test_event(
            "EV-LATE-01",
            TemporalEvidenceKind::ProcessObservation,
            90,
            101,
        );
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("late event is excluded, not fatal");
        assert!(snapshot.included.is_empty());
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::LateArrival
        );
    }

    #[test]
    fn future_measurement_is_excluded_even_when_received() {
        let event =
            temporal_test_event("EV-FUTURE-01", TemporalEvidenceKind::GeometryScan, 101, 102);
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("future measurement is excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::FutureMeasurement
        );
    }

    #[test]
    fn ndt_label_is_not_a_medium_loop_feature() {
        let mut event = temporal_test_event("EV-NDT-01", TemporalEvidenceKind::NdtOutcome, 90, 91);
        event.role = TemporalEvidenceRole::OutcomeLabel;
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("NDT label is quarantined from medium loop");
        assert!(snapshot.included.is_empty());
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::OutcomeLabelLeakage
        );
    }

    #[test]
    fn later_ndt_outcome_is_available_to_later_quality_requalification() {
        let mut event =
            temporal_test_event("EV-NDT-02", TemporalEvidenceKind::NdtOutcome, 101, 102);
        event.role = TemporalEvidenceRole::OutcomeLabel;
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::SlowQualityRequalification, 110),
        )
        .expect("later quality view may use the recorded NDT outcome");
        assert_eq!(snapshot.included.len(), 1);
    }

    #[test]
    fn superseding_scan_known_by_cutoff_replaces_prior_scan() {
        let old = temporal_test_event("SCAN-OLD", TemporalEvidenceKind::GeometryScan, 80, 81);
        let mut new = temporal_test_event("SCAN-NEW", TemporalEvidenceKind::GeometryScan, 90, 91);
        new.supersedes_event_id = Some("SCAN-OLD".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[old, new],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("valid supersession");
        assert_eq!(
            snapshot
                .included
                .iter()
                .map(|e| e.event_id.as_str())
                .collect::<Vec<_>>(),
            vec!["SCAN-NEW"]
        );
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::Superseded
        );
    }

    #[test]
    fn late_superseding_scan_does_not_rewrite_earlier_snapshot() {
        let old = temporal_test_event("SCAN-OLD", TemporalEvidenceKind::GeometryScan, 80, 81);
        let mut new = temporal_test_event("SCAN-NEW", TemporalEvidenceKind::GeometryScan, 90, 101);
        new.supersedes_event_id = Some("SCAN-OLD".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[new, old],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("late supersession does not alter prior view");
        assert!(
            snapshot
                .included
                .iter()
                .any(|event| event.event_id == "SCAN-OLD")
        );
        assert!(snapshot.excluded.iter().any(|event| {
            event.event_id == "SCAN-NEW" && event.reason == TemporalExclusionReason::LateArrival
        }));
    }

    #[test]
    fn revocation_known_by_cutoff_removes_evidence() {
        let old = temporal_test_event("CAL-OLD", TemporalEvidenceKind::ModelCalibration, 80, 81);
        let mut revocation =
            temporal_test_event("REV-01", TemporalEvidenceKind::RegistrationReceipt, 90, 91);
        revocation.revokes_event_id = Some("CAL-OLD".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[old, revocation],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("valid revocation");
        assert!(
            !snapshot
                .included
                .iter()
                .any(|event| event.event_id == "CAL-OLD")
        );
        assert!(snapshot.excluded.iter().any(|event| {
            event.event_id == "CAL-OLD" && event.reason == TemporalExclusionReason::Revoked
        }));
    }

    #[test]
    fn late_revocation_does_not_rewrite_historical_snapshot() {
        let old = temporal_test_event("CAL-OLD", TemporalEvidenceKind::ModelCalibration, 80, 81);
        let mut revocation =
            temporal_test_event("REV-02", TemporalEvidenceKind::RegistrationReceipt, 90, 101);
        revocation.revokes_event_id = Some("CAL-OLD".to_owned());
        let snapshot = build_point_in_time_snapshot(
            &[revocation, old],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("late revocation does not rewrite history");
        assert!(
            snapshot
                .included
                .iter()
                .any(|event| event.event_id == "CAL-OLD")
        );
    }

    #[test]
    fn duplicate_temporal_event_identity_rejects_the_ledger() {
        let event = temporal_test_event("EV-DUP", TemporalEvidenceKind::GeometryScan, 80, 81);
        assert_eq!(
            build_point_in_time_snapshot(
                &[event.clone(), event],
                &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
            ),
            Err(TemporalSnapshotError::DuplicateEventId("EV-DUP".to_owned()))
        );
    }

    #[test]
    fn receipt_before_measurement_rejects_the_ledger() {
        let event =
            temporal_test_event("EV-CLOCK", TemporalEvidenceKind::ProcessObservation, 90, 80);
        assert_eq!(
            build_point_in_time_snapshot(
                &[event],
                &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
            ),
            Err(TemporalSnapshotError::ReceiptBeforeMeasurement(
                "EV-CLOCK".to_owned()
            ))
        );
    }

    #[test]
    fn missing_source_digest_is_excluded() {
        let mut event = temporal_test_event(
            "EV-NO-SOURCE-HASH",
            TemporalEvidenceKind::GeometryScan,
            80,
            81,
        );
        event.source_sha256 = [0; 32];
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("missing digest is quarantined");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::MissingSourceDigest
        );
    }

    #[test]
    fn timestamp_precision_too_coarse_is_excluded() {
        let mut event = temporal_test_event(
            "EV-COARSE",
            TemporalEvidenceKind::ProcessObservation,
            80,
            81,
        );
        event.timestamp_precision_ticks = 10;
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::FastProcess, 100),
        )
        .expect("coarse timestamp is excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::TimestampPrecisionTooCoarse
        );
    }

    #[test]
    fn snapshot_digest_is_invariant_to_event_input_permutation() {
        let first = temporal_test_event("EV-A", TemporalEvidenceKind::ProcessObservation, 80, 81);
        let second = temporal_test_event("EV-B", TemporalEvidenceKind::GeometryScan, 90, 91);
        let decision = temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100);
        let a = build_point_in_time_snapshot(&[first.clone(), second.clone()], &decision)
            .expect("first snapshot");
        let b =
            build_point_in_time_snapshot(&[second, first], &decision).expect("permuted snapshot");
        assert_eq!(a.snapshot_sha256, b.snapshot_sha256);
        assert_eq!(a.included, b.included);
    }

    #[test]
    fn late_event_does_not_change_historical_snapshot_digest() {
        let event = temporal_test_event("EV-BASE", TemporalEvidenceKind::GeometryScan, 80, 81);
        let late =
            temporal_test_event("EV-LATE", TemporalEvidenceKind::ProcessObservation, 90, 101);
        let decision = temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100);
        let a = build_point_in_time_snapshot(std::slice::from_ref(&event), &decision)
            .expect("base snapshot");
        let b = build_point_in_time_snapshot(&[event, late], &decision)
            .expect("snapshot with late arrival");
        assert_eq!(a.snapshot_sha256, b.snapshot_sha256);
    }

    #[test]
    fn model_trained_after_cutoff_is_excluded_from_historical_replay() {
        let mut event =
            temporal_test_event("MODEL-NEW", TemporalEvidenceKind::ModelVersion, 90, 91);
        event.role = TemporalEvidenceRole::ModelArtifact;
        event.model_training_cutoff_tick = Some(101);
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("future-trained model excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::ModelTrainedAfterDecisionCutoff
        );
    }

    #[test]
    fn mismatched_workpiece_is_excluded_from_snapshot() {
        let mut event =
            temporal_test_event("EV-OTHER-WID", TemporalEvidenceKind::GeometryScan, 80, 81);
        event.workpiece_id = "WID-OTHER".to_owned();
        let snapshot = build_point_in_time_snapshot(
            &[event],
            &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100),
        )
        .expect("wrong WID excluded");
        assert_eq!(
            snapshot.excluded[0].reason,
            TemporalExclusionReason::WorkpieceMismatch
        );
    }

    #[test]
    fn missing_supersession_target_rejects_the_ledger() {
        let mut event =
            temporal_test_event("EV-ORPHAN", TemporalEvidenceKind::GeometryScan, 90, 91);
        event.supersedes_event_id = Some("DOES-NOT-EXIST".to_owned());
        assert!(matches!(
            build_point_in_time_snapshot(
                &[event],
                &temporal_test_decision(TemporalDecisionPurpose::MediumNextPass, 100)
            ),
            Err(TemporalSnapshotError::MissingSupersessionTarget { .. })
        ));
    }

    #[test]
    fn returns_a_pareto_set_without_automatically_picking_a_pass() {
        let result = recommend_next_pass(&base_input());
        let Decision::ParetoRecommendations {
            recommendations,
            feasible_candidate_count,
            automatic_selection,
            live_execution_authorized,
            claim_ceiling,
            ..
        } = result
        else {
            panic!("valid synthetic observation should produce a recommendation frontier");
        };
        assert_eq!(feasible_candidate_count, 3);
        assert_eq!(
            recommendations
                .iter()
                .map(|item| item.candidate_id.as_str())
                .collect::<Vec<_>>(),
            vec!["FILL-MID", "FILL-OVER", "FILL-UNDER"]
        );
        assert_eq!(
            recommendations[0].objectives.robust_geometry_error_units,
            10
        );
        assert_eq!(
            recommendations[1].objectives.robust_geometry_error_units,
            40
        );
        assert_eq!(
            recommendations[2].objectives.robust_geometry_error_units,
            28
        );
        assert!(!automatic_selection);
        assert!(!live_execution_authorized);
        assert_eq!(claim_ceiling, "SYNTHETIC_INTERPASS_PLAN_CONTRACT_ONLY");
    }

    #[test]
    fn geometry_only_and_time_only_baselines_choose_different_passes() {
        let Decision::ParetoRecommendations {
            recommendations, ..
        } = recommend_next_pass(&base_input())
        else {
            panic!("valid synthetic observation should produce recommendations");
        };
        let geometry_only = recommendations
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .robust_geometry_error_units
                    .cmp(&right.objectives.robust_geometry_error_units)
                    .then_with(|| left.candidate_id.cmp(&right.candidate_id))
            })
            .expect("geometry-only baseline candidate");
        let time_only = recommendations
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .duration_units
                    .cmp(&right.objectives.duration_units)
                    .then_with(|| left.candidate_id.cmp(&right.candidate_id))
            })
            .expect("time-only baseline candidate");
        assert_eq!(geometry_only.candidate_id, "FILL-MID");
        assert_eq!(time_only.candidate_id, "FILL-OVER");
        assert_eq!(recommendations.len(), 3);
    }

    #[test]
    fn receding_horizon_replans_from_the_new_after_pass_scan() {
        let first = recommend_next_pass(&base_input());
        let Decision::ParetoRecommendations {
            recommendations: first_frontier,
            ..
        } = first
        else {
            panic!("first observation should produce recommendations");
        };
        let first_geometry_best = first_frontier
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .robust_geometry_error_units
                    .cmp(&right.objectives.robust_geometry_error_units)
            })
            .expect("first geometry baseline");
        assert_eq!(first_geometry_best.candidate_id, "FILL-MID");

        let mut second_input = base_input();
        second_input.observed_state.after_pass_id = "PASS-03".to_owned();
        second_input.observed_state.scan_id = "SCAN-AFTER-PASS-03".to_owned();
        second_input.observed_state.scan_sha256 = [0x33; 32];
        second_input.observed_state.registration_receipt_sha256 = [0x44; 32];
        second_input.observed_state.residual_fill_area_units = 55;
        second_input.observed_state.uncertainty_bound_units = 5;
        second_input.observed_state.observed_at_tick = 110;
        second_input.expected_predecessor_pass_id = "PASS-03".to_owned();
        second_input.decision_at_tick = 110;

        let next_actions = [("FILL-45", 45), ("FILL-55", 55), ("FILL-70", 70)];
        for (candidate, (candidate_id, deposition)) in
            second_input.candidates.iter_mut().zip(next_actions)
        {
            candidate.candidate_id = candidate_id.to_owned();
            candidate.predecessor_pass_id = "PASS-03".to_owned();
            candidate.source_scan_id = "SCAN-AFTER-PASS-03".to_owned();
            candidate.source_scan_sha256 = [0x33; 32];
            candidate.source_registration_receipt_sha256 = [0x44; 32];
            candidate.deposition_area_units = deposition;
        }

        let second = recommend_next_pass(&second_input);
        let Decision::ParetoRecommendations {
            recommendations: second_frontier,
            ..
        } = second
        else {
            panic!("new observation should produce a new recommendation frontier");
        };
        let second_geometry_best = second_frontier
            .iter()
            .min_by(|left, right| {
                left.objectives
                    .robust_geometry_error_units
                    .cmp(&right.objectives.robust_geometry_error_units)
            })
            .expect("second geometry baseline");
        assert_eq!(second_geometry_best.candidate_id, "FILL-55");
        assert_ne!(
            first_geometry_best.candidate_id,
            second_geometry_best.candidate_id
        );
    }

    #[test]
    fn stale_predecessor_scan_causes_abstention() {
        let mut input = base_input();
        input.observed_state.after_pass_id = "PASS-01".to_owned();
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::StalePredecessorState,
                ..
            }
        ));
    }

    #[test]
    fn unregistered_scan_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.registered = false;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanNotRegistered,
                ..
            }
        ));
    }

    #[test]
    fn low_coverage_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.coverage_permille = 900;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanCoverageBelowMinimum,
                ..
            }
        ));
    }

    #[test]
    fn excessive_occlusion_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.occlusion_permille = 80;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanOcclusionAboveMaximum,
                ..
            }
        ));
    }

    #[test]
    fn excessive_registration_error_causes_abstention() {
        let mut input = base_input();
        input.observed_state.quality.registration_error_units = 3;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanRegistrationErrorTooLarge,
                ..
            }
        ));
    }

    #[test]
    fn future_observation_causes_abstention() {
        let mut input = base_input();
        input.observed_state.observed_at_tick = 101;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ObservationFromFuture,
                ..
            }
        ));
    }

    #[test]
    fn old_observation_causes_abstention() {
        let mut input = base_input();
        input.observed_state.observed_at_tick = 50;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ObservationTooOld,
                ..
            }
        ));
    }

    #[test]
    fn future_information_leakage_causes_abstention() {
        let mut input = base_input();
        input.future_information_used = true;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::FutureInformationLeakage,
                ..
            }
        ));
    }

    #[test]
    fn out_of_domain_model_causes_abstention() {
        let mut input = base_input();
        input.model_domain_supported = false;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ModelOutsideDeclaredDomain,
                ..
            }
        ));
    }

    #[test]
    fn measured_or_simulated_data_are_not_yet_admitted_by_v0() {
        let mut input = base_input();
        input.observed_state.evidence_class = EvidenceClass::MeasuredGeometry;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::PhysicalOrSimulationEvidenceNotAdmitted,
                ..
            }
        ));
    }

    #[test]
    fn live_execution_is_never_authorized() {
        let mut input = base_input();
        input.live_execution_requested = true;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::LiveExecutionForbidden,
                ..
            }
        ));
    }

    #[test]
    fn candidate_bound_to_another_scan_is_excluded() {
        let mut input = base_input();
        input.candidates[0].source_scan_id = "STALE-SCAN".to_owned();
        let Decision::ParetoRecommendations { excluded, feasible_candidate_count, .. } =
            recommend_next_pass(&input) else {
                panic!("remaining candidates should be feasible");
            };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::WrongSourceScan);
    }

    #[test]
    fn procedure_revision_mismatch_is_excluded() {
        let mut input = base_input();
        input.candidates[0].procedure_revision = "R0".to_owned();
        let Decision::ParetoRecommendations { excluded, feasible_candidate_count, .. } =
            recommend_next_pass(&input) else {
                panic!("remaining candidates should be feasible");
            };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(
            excluded[0].reason,
            CandidateExclusion::WrongProcedureRevision
        );
    }

    #[test]
    fn infeasible_candidates_are_not_pareto_options() {
        let mut input = base_input();
        input.candidates[0].collision_free = false;
        input.candidates[1].resource_feasible = false;
        let Decision::ParetoRecommendations {
            recommendations,
            feasible_candidate_count,
            excluded,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("one feasible candidate remains");
        };
        assert_eq!(feasible_candidate_count, 1);
        assert_eq!(recommendations.len(), 1);
        assert_eq!(excluded.len(), 2);
    }

    #[test]
    fn candidate_with_wrong_scan_digest_is_excluded() {
        let mut input = base_input();
        input.candidates[0].source_scan_sha256 = [0x33; 32];
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should remain feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::WrongSourceScan);
    }

    #[test]
    fn candidate_with_wrong_registration_receipt_is_excluded() {
        let mut input = base_input();
        input.candidates[0].source_registration_receipt_sha256 = [0x44; 32];
        let Decision::ParetoRecommendations {
            excluded,
            feasible_candidate_count,
            ..
        } = recommend_next_pass(&input)
        else {
            panic!("remaining candidates should remain feasible");
        };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::WrongSourceScan);
    }

    #[test]
    fn missing_scan_digest_causes_abstention() {
        let mut input = base_input();
        input.observed_state.scan_sha256 = [0; 32];
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ScanDigestMissing,
                ..
            }
        ));
    }

    #[test]
    fn missing_registration_receipt_causes_abstention() {
        let mut input = base_input();
        input.observed_state.registration_receipt_sha256 = [0; 32];
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::RegistrationReceiptMissing,
                ..
            }
        ));
    }

    #[test]
    fn no_feasible_candidates_causes_abstention() {
        let mut input = base_input();
        for candidate in &mut input.candidates {
            candidate.resource_feasible = false;
        }
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::NoFeasibleCandidate,
                ..
            }
        ));
    }

    #[test]
    fn duplicate_candidate_identity_causes_abstention() {
        let mut input = base_input();
        input.candidates[1].candidate_id = input.candidates[0].candidate_id.clone();
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::DuplicateCandidateIdentity,
                ..
            }
        ));
    }

    #[test]
    fn robust_geometry_error_accounts_for_measurement_uncertainty() {
        let mut input = base_input();
        input.observed_state.uncertainty_bound_units = 25;
        let Decision::ParetoRecommendations {
            recommendations, ..
        } = recommend_next_pass(&input)
        else {
            panic!("valid synthetic input should be admitted");
        };
        let middle = recommendations
            .iter()
            .find(|r| r.candidate_id == "FILL-MID")
            .unwrap();
        assert_eq!(middle.objectives.robust_geometry_error_units, 25);
    }

    #[test]
    fn physical_procedure_scope_is_not_implied_by_synthetic_contract() {
        let mut input = base_input();
        input.procedure.scope = ProcedureScope::UnverifiedPhysicalProcedure;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::ProcedureScopeNotAdmitted,
                ..
            }
        ));
    }

    #[test]
    fn zero_uncertainty_is_not_admitted_as_a_measured_bound() {
        let mut input = base_input();
        input.observed_state.uncertainty_bound_units = 0;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::UncertaintyNotBounded,
                ..
            }
        ));
    }

    #[test]
    fn empty_candidate_identity_is_excluded() {
        let mut input = base_input();
        input.candidates[0].candidate_id.clear();
        let Decision::ParetoRecommendations { excluded, feasible_candidate_count, .. } =
            recommend_next_pass(&input) else {
                panic!("remaining candidates should remain feasible");
            };
        assert_eq!(feasible_candidate_count, 2);
        assert_eq!(excluded[0].reason, CandidateExclusion::IdentityMismatch);
    }

    #[test]
    fn uncertainty_above_decision_limit_causes_abstention() {
        let mut input = base_input();
        input.observed_state.uncertainty_bound_units = 31;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::UncertaintyExceedsDecisionLimit,
                ..
            }
        ));
    }

    #[test]
    fn numeric_overflow_abstains_instead_of_wrapping() {
        let mut input = base_input();
        input.observed_state.residual_fill_area_units = u64::MAX;
        input.observed_state.uncertainty_bound_units = 1;
        assert!(matches!(
            recommend_next_pass(&input),
            Decision::Abstain {
                reason: AbstainReason::NumericOverflow,
                ..
            }
        ));
    }
}
