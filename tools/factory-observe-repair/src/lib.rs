// SPDX-License-Identifier: AGPL-3.0-or-later
//! Narrow evaluator for the factory observe -> repair -> accepted-output benchmark.
//!
//! The crate consumes an admitted inspection receipt and an externally computed
//! impact closure. It does not implement Bayesian diagnosis, generic dependency
//! graphs, production scheduling, physical inspection or release authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const SCHEMA_VERSION: &str = "factory-observe-repair/0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentState {
    Completed,
    InFlight,
    Planned,
}

impl CommitmentState {
    fn is_frozen(self) -> bool {
        matches!(self, Self::Completed | Self::InFlight)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    Pass,
    Block,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCommitment {
    pub operation_id: u32,
    pub state: CommitmentState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactClosure {
    /// Exact owner-issued change-impact receipt; this crate does not construct it.
    pub receipt_id: String,
    /// Claim references affected by the admitted diagnosis/change-impact owner.
    pub claim_ids: Vec<String>,
    /// Operation IDs in the externally computed dependency closure.
    pub affected_operation_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionEvidence {
    /// Exact observation/inspection receipt reference from the diagnosis owner.
    pub receipt_id: String,
    pub test_id: String,
    pub outcome_id: String,
    /// Evidence lineage groups are supplied by the evidence/observation owner.
    pub measurement_source_group: String,
    pub baseline_source_group: String,
    pub calibration_status: GateStatus,
    pub applicability_status: GateStatus,
    pub outcome_status: GateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    pub episode_id: String,
    pub operations: Vec<OperationCommitment>,
    pub impact: ImpactClosure,
    pub inspection: InspectionEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairProposal {
    pub diagnosis_receipt_id: String,
    pub impact_receipt_id: String,
    /// Planned operations released for rescheduling/requalification.
    pub release_operation_ids: Vec<u32>,
    /// Completed/in-flight affected operations explicitly contained or held.
    pub containment_operation_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    CalibrationBlocked,
    CalibrationUnknown,
    ApplicabilityBlocked,
    ApplicabilityUnknown,
    OutcomeBlocked,
    OutcomeUnknown,
    CorrelatedObservationOrigin,
    RepairScopeIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationStatus {
    Eligible,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    EmptyIdentity(&'static str),
    DuplicateOperationId(u32),
    DuplicateClaimId(String),
    DuplicateAffectedOperation(u32),
    UnknownAffectedOperation(u32),
    DuplicateReleaseOperation(u32),
    DuplicateContainmentOperation(u32),
    OperationBothReleasedAndContained(u32),
    UnknownReleasedOperation(u32),
    UnknownContainedOperation(u32),
    FrozenOperationReleased(u32),
    PlannedOperationContained(u32),
    UnaffectedOperationContained(u32),
    InspectionReceiptMismatch,
    ImpactReceiptMismatch,
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EvaluationError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopeMetrics {
    pub total_operations: usize,
    pub affected_operations: usize,
    pub affected_frozen_operations: usize,
    pub affected_planned_operations: usize,
    pub released_operations: usize,
    pub contained_operations: usize,
    pub false_preserved_affected_operations: usize,
    pub unnecessary_released_operations: usize,
    pub unaffected_planned_operations: usize,
    pub unaffected_planned_operations_retained: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeEvaluation {
    pub status: EvaluationStatus,
    pub block_reason: Option<BlockReason>,
    pub metrics: ScopeMetrics,
}

fn require_nonempty(value: &str, field: &'static str) -> Result<(), EvaluationError> {
    if value.trim().is_empty() {
        return Err(EvaluationError::EmptyIdentity(field));
    }
    Ok(())
}

fn unique_ids(values: &[u32]) -> Result<BTreeSet<u32>, EvaluationError> {
    let mut result = BTreeSet::new();
    for value in values {
        if !result.insert(*value) {
            return Err(EvaluationError::DuplicateAffectedOperation(*value));
        }
    }
    Ok(result)
}

fn validate_episode(episode: &Episode) -> Result<BTreeMap<u32, CommitmentState>, EvaluationError> {
    require_nonempty(&episode.episode_id, "episode_id")?;
    require_nonempty(&episode.impact.receipt_id, "impact.receipt_id")?;
    require_nonempty(&episode.inspection.receipt_id, "inspection.receipt_id")?;
    require_nonempty(&episode.inspection.test_id, "inspection.test_id")?;
    require_nonempty(&episode.inspection.outcome_id, "inspection.outcome_id")?;
    require_nonempty(&episode.inspection.measurement_source_group, "measurement_source_group")?;
    require_nonempty(&episode.inspection.baseline_source_group, "baseline_source_group")?;

    if episode.impact.claim_ids.is_empty() {
        return Err(EvaluationError::EmptyIdentity("impact.claim_ids"));
    }
    let mut claims = BTreeSet::new();
    for claim_id in &episode.impact.claim_ids {
        require_nonempty(claim_id, "claim_id")?;
        if !claims.insert(claim_id) {
            return Err(EvaluationError::DuplicateClaimId(claim_id.clone()));
        }
    }

    let mut operations = BTreeMap::new();
    for operation in &episode.operations {
        if operations.insert(operation.operation_id, operation.state).is_some() {
            return Err(EvaluationError::DuplicateOperationId(operation.operation_id));
        }
    }

    let affected = unique_ids(&episode.impact.affected_operation_ids)?;
    for operation_id in affected {
        if !operations.contains_key(&operation_id) {
            return Err(EvaluationError::UnknownAffectedOperation(operation_id));
        }
    }
    Ok(operations)
}

fn evidence_block(inspection: &InspectionEvidence) -> Option<BlockReason> {
    if inspection.measurement_source_group == inspection.baseline_source_group {
        return Some(BlockReason::CorrelatedObservationOrigin);
    }
    match inspection.calibration_status {
        GateStatus::Block => return Some(BlockReason::CalibrationBlocked),
        GateStatus::Unknown => return Some(BlockReason::CalibrationUnknown),
        GateStatus::Pass => {}
    }
    match inspection.applicability_status {
        GateStatus::Block => return Some(BlockReason::ApplicabilityBlocked),
        GateStatus::Unknown => return Some(BlockReason::ApplicabilityUnknown),
        GateStatus::Pass => {}
    }
    match inspection.outcome_status {
        GateStatus::Block => Some(BlockReason::OutcomeBlocked),
        GateStatus::Unknown => Some(BlockReason::OutcomeUnknown),
        GateStatus::Pass => None,
    }
}

/// Evaluate the candidate's repair scope against an owner-supplied impact closure.
///
/// The evaluator reports unnecessary releases separately from false preservation.
/// A blocked/unknown inspection cannot be used to admit a repair scope. Completed
/// and in-flight work may be contained, but can never be released for rescheduling.
pub fn evaluate_scope(
    episode: &Episode,
    proposal: &RepairProposal,
) -> Result<ScopeEvaluation, EvaluationError> {
    let operations = validate_episode(episode)?;
    require_nonempty(&proposal.diagnosis_receipt_id, "diagnosis_receipt_id")?;
    if proposal.impact_receipt_id != episode.impact.receipt_id {
        return Err(EvaluationError::ImpactReceiptMismatch);
    }
    if proposal.diagnosis_receipt_id != episode.inspection.receipt_id {
        return Err(EvaluationError::InspectionReceiptMismatch);
    }

    let release_ids = unique_ids(&proposal.release_operation_ids)
        .map_err(|error| match error {
            EvaluationError::DuplicateAffectedOperation(id) => EvaluationError::DuplicateReleaseOperation(id),
            other => other,
        })?;
    let containment_ids = unique_ids(&proposal.containment_operation_ids)
        .map_err(|error| match error {
            EvaluationError::DuplicateAffectedOperation(id) => EvaluationError::DuplicateContainmentOperation(id),
            other => other,
        })?;
    for operation_id in release_ids.intersection(&containment_ids) {
        return Err(EvaluationError::OperationBothReleasedAndContained(*operation_id));
    }

    let affected: BTreeSet<u32> = episode.impact.affected_operation_ids.iter().copied().collect();
    let mut false_preserved = 0usize;
    let mut unnecessary_released = 0usize;
    let mut affected_frozen = 0usize;
    let mut affected_planned = 0usize;
    let mut unaffected_planned = 0usize;
    let mut unaffected_planned_retained = 0usize;

    for operation_id in &release_ids {
        match operations.get(operation_id) {
            None => return Err(EvaluationError::UnknownReleasedOperation(*operation_id)),
            Some(CommitmentState::Completed | CommitmentState::InFlight) => {
                return Err(EvaluationError::FrozenOperationReleased(*operation_id));
            }
            Some(CommitmentState::Planned) => {
                if !affected.contains(operation_id) {
                    unnecessary_released += 1;
                }
            }
        }
    }

    for operation_id in &containment_ids {
        match operations.get(operation_id) {
            None => return Err(EvaluationError::UnknownContainedOperation(*operation_id)),
            Some(CommitmentState::Planned) => {
                return Err(EvaluationError::PlannedOperationContained(*operation_id));
            }
            Some(CommitmentState::Completed | CommitmentState::InFlight) => {
                if !affected.contains(operation_id) {
                    return Err(EvaluationError::UnaffectedOperationContained(*operation_id));
                }
            }
        }
    }

    for (operation_id, state) in &operations {
        let is_affected = affected.contains(operation_id);
        if is_affected {
            match state {
                CommitmentState::Completed | CommitmentState::InFlight => {
                    affected_frozen += 1;
                    if !containment_ids.contains(operation_id) {
                        false_preserved += 1;
                    }
                }
                CommitmentState::Planned => {
                    affected_planned += 1;
                    if !release_ids.contains(operation_id) {
                        false_preserved += 1;
                    }
                }
            }
        } else if *state == CommitmentState::Planned {
            unaffected_planned += 1;
            if !release_ids.contains(operation_id) {
                unaffected_planned_retained += 1;
            }
        }
    }

    let status = if false_preserved > 0 {
        EvaluationStatus::Blocked
    } else if evidence_block(&episode.inspection).is_some() {
        EvaluationStatus::Blocked
    } else {
        EvaluationStatus::Eligible
    };
    let block_reason = evidence_block(&episode.inspection).or_else(|| {
        (false_preserved > 0).then_some(BlockReason::RepairScopeIncomplete)
    });

    Ok(ScopeEvaluation {
        status,
        block_reason,
        metrics: ScopeMetrics {
            total_operations: operations.len(),
            affected_operations: affected.len(),
            affected_frozen_operations: affected_frozen,
            affected_planned_operations: affected_planned,
            released_operations: release_ids.len(),
            contained_operations: containment_ids.len(),
            false_preserved_affected_operations: false_preserved,
            unnecessary_released_operations: unnecessary_released,
            unaffected_planned_operations: unaffected_planned,
            unaffected_planned_operations_retained: unaffected_planned_retained,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn episode() -> Episode {
        Episode {
            episode_id: "PUMP-VALVE-REPAIR-01".to_owned(),
            operations: vec![
                OperationCommitment { operation_id: 0, state: CommitmentState::Completed },
                OperationCommitment { operation_id: 1, state: CommitmentState::InFlight },
                OperationCommitment { operation_id: 2, state: CommitmentState::Planned },
                OperationCommitment { operation_id: 3, state: CommitmentState::Planned },
                OperationCommitment { operation_id: 4, state: CommitmentState::Planned },
                OperationCommitment { operation_id: 5, state: CommitmentState::Planned },
                OperationCommitment { operation_id: 6, state: CommitmentState::Planned },
            ],
            impact: ImpactClosure {
                receipt_id: "IMPACT-CLOSURE-01".to_owned(),
                claim_ids: vec!["DIMENSIONAL-CLAIM".to_owned(), "FLOW-CLAIM".to_owned()],
                affected_operation_ids: vec![0, 1, 2, 3],
            },
            inspection: InspectionEvidence {
                receipt_id: "INSPECTION-RECEIPT-01".to_owned(),
                test_id: "INDEPENDENT-PROCESS-TEST".to_owned(),
                outcome_id: "OUTCOME-POSITIVE".to_owned(),
                measurement_source_group: "INDEPENDENT-TEST-STAND".to_owned(),
                baseline_source_group: "MACHINE-DASHBOARD".to_owned(),
                calibration_status: GateStatus::Pass,
                applicability_status: GateStatus::Pass,
                outcome_status: GateStatus::Pass,
            },
        }
    }

    fn proposal() -> RepairProposal {
        RepairProposal {
            diagnosis_receipt_id: "INSPECTION-RECEIPT-01".to_owned(),
            impact_receipt_id: "IMPACT-CLOSURE-01".to_owned(),
            release_operation_ids: vec![2, 3],
            containment_operation_ids: vec![0, 1],
        }
    }

    #[test]
    fn dependency_closure_scope_is_eligible_and_preserves_unaffected_work() {
        let result = evaluate_scope(&episode(), &proposal()).unwrap();
        assert_eq!(result.status, EvaluationStatus::Eligible);
        assert_eq!(result.block_reason, None);
        assert_eq!(result.metrics.affected_operations, 4);
        assert_eq!(result.metrics.affected_frozen_operations, 2);
        assert_eq!(result.metrics.affected_planned_operations, 2);
        assert_eq!(result.metrics.released_operations, 2);
        assert_eq!(result.metrics.contained_operations, 2);
        assert_eq!(result.metrics.false_preserved_affected_operations, 0);
        assert_eq!(result.metrics.unnecessary_released_operations, 0);
        assert_eq!(result.metrics.unaffected_planned_operations_retained, 3);
    }

    #[test]
    fn full_reoptimization_is_eligible_but_unnecessary_release_is_visible() {
        let mut candidate = proposal();
        candidate.release_operation_ids = vec![2, 3, 4, 5, 6];
        let result = evaluate_scope(&episode(), &candidate).unwrap();
        assert_eq!(result.status, EvaluationStatus::Eligible);
        assert_eq!(result.metrics.unnecessary_released_operations, 3);
        assert_eq!(result.metrics.unaffected_planned_operations_retained, 0);
    }

    #[test]
    fn missing_affected_planned_operation_blocks_scope() {
        let mut candidate = proposal();
        candidate.release_operation_ids = vec![2];
        let result = evaluate_scope(&episode(), &candidate).unwrap();
        assert_eq!(result.status, EvaluationStatus::Blocked);
        assert_eq!(result.block_reason, Some(BlockReason::RepairScopeIncomplete));
        assert_eq!(result.metrics.false_preserved_affected_operations, 1);
    }

    #[test]
    fn frozen_operation_cannot_be_released() {
        let mut candidate = proposal();
        candidate.release_operation_ids = vec![0, 2, 3];
        assert_eq!(
            evaluate_scope(&episode(), &candidate),
            Err(EvaluationError::FrozenOperationReleased(0))
        );
    }

    #[test]
    fn impacted_completed_and_inflight_work_must_be_contained() {
        let mut candidate = proposal();
        candidate.containment_operation_ids = vec![0];
        let result = evaluate_scope(&episode(), &candidate).unwrap();
        assert_eq!(result.status, EvaluationStatus::Blocked);
        assert_eq!(result.metrics.false_preserved_affected_operations, 1);
    }

    #[test]
    fn stale_or_unknown_calibration_blocks_repair_admission() {
        for status in [GateStatus::Block, GateStatus::Unknown] {
            let mut source = episode();
            source.inspection.calibration_status = status;
            let result = evaluate_scope(&source, &proposal()).unwrap();
            assert_eq!(result.status, EvaluationStatus::Blocked);
            assert!(matches!(
                result.block_reason,
                Some(BlockReason::CalibrationBlocked | BlockReason::CalibrationUnknown)
            ));
        }
    }

    #[test]
    fn non_independent_observation_origin_blocks_evidence_use() {
        let mut source = episode();
        source.inspection.measurement_source_group = source.inspection.baseline_source_group.clone();
        let result = evaluate_scope(&source, &proposal()).unwrap();
        assert_eq!(result.status, EvaluationStatus::Blocked);
        assert_eq!(result.block_reason, Some(BlockReason::CorrelatedObservationOrigin));
    }

    #[test]
    fn duplicate_operation_identity_is_invalid_input() {
        let mut source = episode();
        source.operations.push(source.operations[0].clone());
        assert_eq!(
            evaluate_scope(&source, &proposal()),
            Err(EvaluationError::DuplicateOperationId(0))
        );
    }

    #[test]
    fn unknown_impact_operation_is_invalid_input() {
        let mut source = episode();
        source.impact.affected_operation_ids.push(99);
        assert_eq!(
            evaluate_scope(&source, &proposal()),
            Err(EvaluationError::UnknownAffectedOperation(99))
        );
    }

    #[test]
    fn proposal_must_bind_exact_diagnosis_and_impact_receipts() {
        let mut candidate = proposal();
        candidate.impact_receipt_id = "STALE-IMPACT-RECEIPT".to_owned();
        assert_eq!(
            evaluate_scope(&episode(), &candidate),
            Err(EvaluationError::ImpactReceiptMismatch)
        );
    }

    #[test]
    fn evaluation_is_deterministic_for_identical_receipts_and_scope() {
        let source = episode();
        let candidate = proposal();
        assert_eq!(
            evaluate_scope(&source, &candidate),
            evaluate_scope(&source, &candidate)
        );
    }
}
