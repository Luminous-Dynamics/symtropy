//! End-to-end software composition for decision-aware evidence acquisition.
//!
//! Composes the decision-aware planner, consume-once attempt ledger and
//! time-of-validity gate. The result is only staging for a later independent
//! posterior audit. This module never updates hypotheses, protection levels,
//! health or authority. The attempt ledger remains an in-memory research kernel.

use std::fmt;

use crate::active_evidence_attempt_ledger::{
    AcquisitionAttemptOutcome, ActiveEvidenceAttemptError, ActiveEvidenceAttemptLedger,
    AdmittedAttemptReceipt, EvidenceAttemptRequest,
};
use crate::active_evidence_decision_planner::{
    ActiveEvidenceDecisionPlan, ActiveEvidenceDecisionReason, EvidenceActionDecisionAssessment,
};
use crate::active_evidence_reliability::{AdmittedAcquisitionReliability};
use crate::active_evidence_reliability_planner::{
    ReliableActiveEvidencePlan, ReliableEvidencePlanReason,
};
use crate::navigation_time_validity::{
    NavigationTimeDisposition, NavigationTimeValidityError, NavigationTimeValidityInput,
    NavigationTimeValidityPolicy, NavigationTimeValidityReport, evaluate_navigation_time_validity,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceTransactionDisposition {
    AcquisitionFailed,
    AcquiredEvidenceInvalid,
    ReconciliationRequired,
    TemporallyInapplicable(NavigationTimeDisposition),
    ReadyForIndependentPosteriorAudit,
    ReadyForIndependentForecastAudit,
    HistoricalReplayOnly,
    TimeAssessmentMissing,
    EvidenceBindingMissing,
    EvidenceBindingMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualifiedEvidenceBinding {
    pub action_id: u64,
    pub attempt_id: u64,
    pub result_id: u64,
    pub source_manifest_sha256: [u8; 32],
    pub payload_sha256: [u8; 32],
    pub qualification_receipt_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceTransactionReport {
    pub action_id: u64,
    pub attempt_id: u64,
    pub admitted_attempt: AdmittedAttemptReceipt,
    pub decision_reason: ActiveEvidenceDecisionReason,
    pub hypothesis_inventory_sha256: [u8; 32],
    pub decision_partition_sha256: [u8; 32],
    pub disposition: EvidenceTransactionDisposition,
    pub qualified_evidence_binding: Option<QualifiedEvidenceBinding>,
    /// Preserves the exact admitted acquisition-reliability interval and cohort/method lineage.
    pub selected_acquisition_reliability: Option<AdmittedAcquisitionReliability>,
    pub selected_reliability_domain_fingerprint_sha256: Option<[u8; 32]>,
    pub selected_reliability_acquisition_protocol_sha256: Option<[u8; 32]>,
    pub time_report: Option<NavigationTimeValidityReport>,
    pub eligible_for_independent_posterior_audit: bool,
    /// Always false. The posterior/fault-envelope owner must perform the update.
    pub posterior_updated: bool,
    /// Always false. Neither planning nor evidence acquisition grants authority.
    pub navigation_authorized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceTransactionError {
    PlanHasNoRecommendedAction,
    PlanMissingInventoryOrDecisionPartition,
    PlanMissingDecisionAssignments,
    AttemptDoesNotMatchRecommendedAction,
    EvidenceBindingMissing,
    EvidenceBindingMismatch,
    ReliablePlanHasNoRecommendedAction,
    SelectedReliabilityMissing(u64),
    ReliabilityDomainMismatch,
    ReliabilityProtocolMismatch,
    AttemptLedger(ActiveEvidenceAttemptError),
    TimeValidity(NavigationTimeValidityError),
}

impl fmt::Display for EvidenceTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanHasNoRecommendedAction => write!(f, "decision-aware plan has no recommended acquisition action"),
            Self::PlanMissingInventoryOrDecisionPartition => write!(f, "decision-aware plan is missing its inventory or decision-partition commitment"),
            Self::PlanMissingDecisionAssignments => write!(f, "decision-aware plan is missing frozen hypothesis-to-decision assignments"),
            Self::AttemptDoesNotMatchRecommendedAction => write!(f, "attempt action does not match the plan's recommendation"),
            Self::EvidenceBindingMissing => write!(f, "qualified acquisition is missing its payload/receipt/source binding"),
            Self::EvidenceBindingMismatch => write!(f, "payload, receipt, result, attempt, action or source-manifest bindings do not match"),
            Self::ReliablePlanHasNoRecommendedAction => write!(f, "reliability-aware plan has no recommended action"),
            Self::SelectedReliabilityMissing(id) => write!(f, "selected action {id} has no admitted reliability evidence in the plan frontier"),
            Self::ReliabilityDomainMismatch => write!(f, "attempt domain does not match the reliability interval's admitted domain"),
            Self::ReliabilityProtocolMismatch => write!(f, "attempt acquisition protocol does not match the reliability interval's admitted protocol"),
            Self::AttemptLedger(error) => write!(f, "attempt ledger rejected transition: {error}"),
            Self::TimeValidity(error) => write!(f, "time-validity assessment failed: {error}"),
        }
    }
}

impl std::error::Error for EvidenceTransactionError {}

/// Consume one executor outcome for the exact action selected by the plan.
///
/// Unknown outcomes remain globally blocking until explicitly reconciled.
/// Failed and invalid outcomes do not reach the time gate. A qualified payload
/// is eligible for independent posterior audit only when it is a time-valid
/// decision-epoch nowcast. A future prediction and historical estimate remain
/// separate result classes. Missing temporal evidence fails closed.
pub fn record_selected_evidence_outcome(
    plan: &ActiveEvidenceDecisionPlan,
    ledger: &mut ActiveEvidenceAttemptLedger,
    request: EvidenceAttemptRequest,
    outcome: AcquisitionAttemptOutcome,
    observed_navigation_state_sha256: [u8; 32],
    evidence_binding: Option<QualifiedEvidenceBinding>,
    time_assessment: Option<(NavigationTimeValidityPolicy, NavigationTimeValidityInput)>,
) -> Result<EvidenceTransactionReport, EvidenceTransactionError> {
    let selected_action_id = plan.recommended_action_id
        .ok_or(EvidenceTransactionError::PlanHasNoRecommendedAction)?;
    let inventory = plan.hypothesis_inventory_sha256
        .ok_or(EvidenceTransactionError::PlanMissingInventoryOrDecisionPartition)?;
    let partition = plan.decision_partition_sha256
        .ok_or(EvidenceTransactionError::PlanMissingInventoryOrDecisionPartition)?;
    if plan.decision_assignments.as_ref().is_none_or(|items| items.is_empty()) {
        return Err(EvidenceTransactionError::PlanMissingDecisionAssignments);
    }
    if request.action_id != selected_action_id {
        return Err(EvidenceTransactionError::AttemptDoesNotMatchRecommendedAction);
    }

    let admitted = ledger.begin_attempt(request)
        .map_err(EvidenceTransactionError::AttemptLedger)?;
    ledger.record_outcome(request.attempt_id, outcome, observed_navigation_state_sha256)
        .map_err(EvidenceTransactionError::AttemptLedger)?;

    let (disposition, qualified_evidence_binding, time_report, audit_eligible) = match outcome {
        AcquisitionAttemptOutcome::FailedOrTimedOut { .. } =>
            (EvidenceTransactionDisposition::AcquisitionFailed, None, None, false),
        AcquisitionAttemptOutcome::AcquiredButInvalid { .. } =>
            (EvidenceTransactionDisposition::AcquiredEvidenceInvalid, None, None, false),
        AcquisitionAttemptOutcome::UnknownOrUnreconciled { .. } =>
            (EvidenceTransactionDisposition::ReconciliationRequired, None, None, false),
        AcquisitionAttemptOutcome::QualifiedEvidenceAvailable {
            payload_sha256,
            qualification_receipt_sha256,
        } => {
            match evidence_binding {
                None => (
                    EvidenceTransactionDisposition::EvidenceBindingMissing,
                    None,
                    None,
                    false,
                ),
                Some(binding)
                    if binding.action_id != request.action_id
                        || binding.attempt_id != request.attempt_id
                        || binding.result_id == 0
                        || binding.payload_sha256 != payload_sha256
                        || binding.qualification_receipt_sha256 != qualification_receipt_sha256
                        || binding.source_manifest_sha256.iter().all(|byte| *byte == 0)
                        || binding.payload_sha256.iter().all(|byte| *byte == 0)
                        || binding.qualification_receipt_sha256.iter().all(|byte| *byte == 0) =>
                {
                    (
                        EvidenceTransactionDisposition::EvidenceBindingMismatch,
                        None,
                        None,
                        false,
                    )
                }
                Some(binding) => match time_assessment {
                    None => (
                        EvidenceTransactionDisposition::TimeAssessmentMissing,
                        Some(binding),
                        None,
                        false,
                    ),
                    Some((_, input))
                        if input.result_id != binding.result_id
                            || input.source_manifest_sha256 != binding.source_manifest_sha256 =>
                    {
                        (
                            EvidenceTransactionDisposition::EvidenceBindingMismatch,
                            Some(binding),
                            None,
                            false,
                        )
                    }
                    Some((policy, input)) => {
                        let report = evaluate_navigation_time_validity(policy, input)
                            .map_err(EvidenceTransactionError::TimeValidity)?;
                        let (stage, eligible) = match report.disposition {
                            NavigationTimeDisposition::DecisionEpochNowcastApplicable =>
                                (EvidenceTransactionDisposition::ReadyForIndependentPosteriorAudit, true),
                            NavigationTimeDisposition::FutureEpochPredictionApplicable =>
                                (EvidenceTransactionDisposition::ReadyForIndependentForecastAudit, false),
                            NavigationTimeDisposition::HistoricalEventTimeOnly =>
                                (EvidenceTransactionDisposition::HistoricalReplayOnly, false),
                            other => (EvidenceTransactionDisposition::TemporallyInapplicable(other), false),
                        };
                        (stage, Some(binding), Some(report), eligible)
                    }
                },
            }
        }
    };

    Ok(EvidenceTransactionReport {
        action_id: request.action_id,
        attempt_id: request.attempt_id,
        admitted_attempt: admitted,
        decision_reason: plan.reason,
        hypothesis_inventory_sha256: inventory,
        decision_partition_sha256: partition,
        disposition,
        qualified_evidence_binding,
        selected_acquisition_reliability: None,
        selected_reliability_domain_fingerprint_sha256: None,
        selected_reliability_acquisition_protocol_sha256: None,
        time_report,
        eligible_for_independent_posterior_audit: audit_eligible,
        posterior_updated: false,
        navigation_authorized: false,
    })
}

/// Convert the selected reliability-aware plan into the shared transaction
/// view while retaining its exact per-action admitted reliability contract.
///
/// The selected conditional assessment is cloned only as a compatibility
/// projection. The full reliability record is preserved in the transaction
/// report and is not reduced to a scalar confidence field.
fn reliable_plan_transaction_view(
    plan: &ReliableActiveEvidencePlan,
) -> Result<(ActiveEvidenceDecisionPlan, AdmittedAcquisitionReliability, [u8; 32], [u8; 32]), EvidenceTransactionError> {
    let action_id = plan
        .recommended_action_id
        .ok_or(EvidenceTransactionError::ReliablePlanHasNoRecommendedAction)?;
    let selected = plan
        .pareto_frontier
        .iter()
        .find(|assessment| assessment.conditional.action.action_id == action_id)
        .ok_or(EvidenceTransactionError::SelectedReliabilityMissing(action_id))?;
    let decision_reason = match plan.reason {
        ReliableEvidencePlanReason::TargetAlreadySatisfied =>
            ActiveEvidenceDecisionReason::TargetAlreadySatisfied,
        ReliableEvidencePlanReason::LowestLatencyDecisionResolvingTarget =>
            ActiveEvidenceDecisionReason::LowestLatencyDecisionResolvingAction,
        ReliableEvidencePlanReason::LowestLatencyTargetReachingByProtectionReduction =>
            ActiveEvidenceDecisionReason::LowestLatencyTargetReachingAction,
        ReliableEvidencePlanReason::BestExpectedDecisionResolution =>
            ActiveEvidenceDecisionReason::BestDecisionDiscriminationAvailableButTargetNotReached,
        ReliableEvidencePlanReason::BestExpectedProtectionReduction =>
            ActiveEvidenceDecisionReason::BestProtectionReductionAvailableButTargetNotReached,
        ReliableEvidencePlanReason::NoReliablyAdmittedAction
        | ReliableEvidencePlanReason::NoConditionalActionValue =>
            return Err(EvidenceTransactionError::ReliablePlanHasNoRecommendedAction),
    };
    let decision_plan = ActiveEvidenceDecisionPlan {
        reason: decision_reason,
        recommended_action_id: Some(action_id),
        pareto_frontier: vec![selected.conditional.clone()],
        conditional_target_reached: plan.conditional_target_reached,
        hypothesis_inventory_sha256: plan.hypothesis_inventory_sha256,
        decision_partition_sha256: plan.decision_partition_sha256,
        decision_assignments: plan.decision_assignments.clone(),
        authority_transition_not_authorized_by_plan: true,
    };
    Ok((
        decision_plan,
        selected.reliability,
        plan.expected_domain_fingerprint_sha256,
        plan.expected_acquisition_protocol_sha256,
    ))
}

/// Consume an outcome for an action selected by the reliability-aware planner.
///
/// This preserves the selected action's complete admitted reliability record
/// (interval, confidence error, independent campaign-block count, empirical
/// yield and source/cohort/method commitments) alongside the consume-once
/// attempt and exact payload/receipt/time binding.
pub fn record_reliably_selected_evidence_outcome(
    plan: &ReliableActiveEvidencePlan,
    ledger: &mut ActiveEvidenceAttemptLedger,
    request: EvidenceAttemptRequest,
    outcome: AcquisitionAttemptOutcome,
    observed_navigation_state_sha256: [u8; 32],
    evidence_binding: Option<QualifiedEvidenceBinding>,
    time_assessment: Option<(NavigationTimeValidityPolicy, NavigationTimeValidityInput)>,
) -> Result<EvidenceTransactionReport, EvidenceTransactionError> {
    let (decision_plan, reliability, expected_domain, expected_protocol) =
        reliable_plan_transaction_view(plan)?;
    if request.domain_fingerprint_sha256 != expected_domain {
        return Err(EvidenceTransactionError::ReliabilityDomainMismatch);
    }
    if request.acquisition_protocol_sha256 != expected_protocol {
        return Err(EvidenceTransactionError::ReliabilityProtocolMismatch);
    }
    let mut report = record_selected_evidence_outcome(
        &decision_plan,
        ledger,
        request,
        outcome,
        observed_navigation_state_sha256,
        evidence_binding,
        time_assessment,
    )?;
    report.selected_acquisition_reliability = Some(reliability);
    report.selected_reliability_domain_fingerprint_sha256 = Some(expected_domain);
    report.selected_reliability_acquisition_protocol_sha256 = Some(expected_protocol);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_acquisition::{ActiveEvidencePolicy, EvidenceActionCandidate, EvidenceActionKind};
    use crate::active_evidence_attempt_ledger::{ActiveEvidenceAttemptPolicy, ResolvedAcquisitionOutcome, AcquisitionAttemptDisposition};
    use crate::active_evidence_decision_planner::plan_active_evidence_with_decision_relevance;
    use crate::active_evidence_reliability::AdmittedAcquisitionReliability;
    use crate::active_evidence_reliability_planner::{ReliableActiveEvidencePlan, ReliableEvidenceActionAssessment, ReliableEvidencePlanReason};
    use crate::decision_relevant_discrimination::{HypothesisDecisionAssignment, project_decision_relevant_discrimination};
    use crate::navigation_time_validity::{NavigationTimeClaimKind, NavigationTimeInterval};
    use crate::uwb_hypothesis_discrimination::{UwbHypothesisDiscriminationReport, UwbHypothesisPairSeparation};

    const DOMAIN: [u8; 32] = [0x11; 32];
    const PROTOCOL: [u8; 32] = [0x22; 32];
    const PRE_STATE: [u8; 32] = [0x44; 32];

    fn nav_policy() -> ActiveEvidencePolicy {
        ActiveEvidencePolicy {
            current_protection_level_upper_m: 3.0,
            target_protection_level_upper_m: 1.0,
            maximum_action_latency_ns: 100,
            maximum_action_energy_uj: 1_000,
            require_new_dependency_group: false,
            already_observed_dependency_groups: vec![10],
        }
    }

    fn candidate(id: u64, pl: f64) -> EvidenceActionCandidate {
        EvidenceActionCandidate {
            action_id: id,
            kind: EvidenceActionKind::RequestIndependentAnchor,
            dependency_group_id: 20 + id,
            conditional_post_action_protection_level_upper_m: pl,
            worst_case_latency_ns: 30,
            worst_case_energy_uj: 200,
            available: true,
            freshness_and_calibration_preconditions_met: true,
        }
    }

    fn upstream(action_id: u64, gaps: [f64; 3]) -> UwbHypothesisDiscriminationReport {
        let pair_ids = [(1, 2), (1, 3), (2, 3)];
        let pair_separations: Vec<_> = pair_ids.into_iter().zip(gaps).map(
            |((lower_hypothesis_id, upper_hypothesis_id), normalized_outcome_gap)| {
                UwbHypothesisPairSeparation {
                    lower_hypothesis_id,
                    upper_hypothesis_id,
                    normalized_outcome_gap,
                    guaranteed_distinguishable: normalized_outcome_gap > 0.0,
                }
            }
        ).collect();
        let separated = pair_separations.iter().filter(|pair| pair.guaranteed_distinguishable).count();
        UwbHypothesisDiscriminationReport {
            action_id,
            hypothesis_inventory_sha256: [0x55; 32],
            outcome_model_sha256: [action_id as u8; 32],
            normalization_sha256: [0x66; 32],
            hypothesis_count: 3,
            hypothesis_pair_count: 3,
            distinguishable_pair_count: separated,
            ambiguous_pair_count: 3 - separated,
            minimum_positive_gap: pair_separations.iter().filter(|pair| pair.guaranteed_distinguishable).map(|pair| pair.normalized_outcome_gap).reduce(f64::min),
            maximum_pair_gap: pair_separations.iter().map(|pair| pair.normalized_outcome_gap).fold(0.0, f64::max),
            conditional_on_successful_valid_observation: true,
            pair_separations,
        }
    }

    fn projected(action_id: u64, gaps: [f64; 3]) -> crate::decision_relevant_discrimination::DecisionRelevantDiscriminationReport {
        project_decision_relevant_discrimination(
            &upstream(action_id, gaps),
            action_id,
            &[1, 2, 3],
            [0x55; 32],
            &[
                HypothesisDecisionAssignment { hypothesis_id: 1, decision_class_id: 10 },
                HypothesisDecisionAssignment { hypothesis_id: 2, decision_class_id: 10 },
                HypothesisDecisionAssignment { hypothesis_id: 3, decision_class_id: 20 },
            ],
            [0x77; 32],
        ).unwrap()
    }

    fn plan() -> ActiveEvidenceDecisionPlan {
        let candidates = [candidate(1, 3.0), candidate(2, 2.5)];
        let reports = [projected(1, [1.0, 0.0, 0.0]), projected(2, [0.0, 2.0, 2.0])];
        let result = plan_active_evidence_with_decision_relevance(&nav_policy(), &candidates, &reports).unwrap();
        assert_eq!(result.recommended_action_id, Some(2));
        result
    }

    fn reliable_plan() -> ReliableActiveEvidencePlan {
        let conditional_plan = plan();
        let conditional = conditional_plan.pareto_frontier
            .iter()
            .find(|assessment| assessment.action.action_id == 2)
            .unwrap()
            .clone();
        let reliability = AdmittedAcquisitionReliability {
            action_id: 2,
            probability_lower_bound: 0.8,
            probability_upper_bound: 0.95,
            confidence_error_probability: 0.05,
            independent_campaign_block_count: 10,
            valid_qualified_success_block_count: 9,
            failed_or_timed_out_block_count: 1,
            acquired_but_invalid_block_count: 0,
            non_success_block_count: 1,
            unresolved_block_count: 0,
            empirical_valid_success_fraction: 0.9,
            source_manifest_sha256: [0x81; 32],
            calibration_cohort_sha256: [0x91; 32],
            bound_method_sha256: [0xa1; 32],
        };
        let assessment = ReliableEvidenceActionAssessment {
            expected_decision_pair_yield_lower_bound:
                reliability.probability_lower_bound * conditional.distinguishable_decision_pair_count as f64,
            expected_decision_pair_yield_upper_bound:
                reliability.probability_upper_bound * conditional.distinguishable_decision_pair_count as f64,
            expected_protection_level_reduction_lower_bound_m:
                reliability.probability_lower_bound * conditional.conditional_protection_level_delta_m,
            expected_protection_level_reduction_upper_bound_m:
                reliability.probability_upper_bound * conditional.conditional_protection_level_delta_m,
            conditional,
            reliability,
        };
        ReliableActiveEvidencePlan {
            reason: ReliableEvidencePlanReason::BestExpectedDecisionResolution,
            recommended_action_id: Some(2),
            pareto_frontier: vec![assessment],
            reliability_rejections: Vec::new(),
            hypothesis_inventory_sha256: conditional_plan.hypothesis_inventory_sha256,
            decision_partition_sha256: conditional_plan.decision_partition_sha256,
            decision_assignments: conditional_plan.decision_assignments,
            conditional_target_reached: false,
            authority_transition_not_authorized_by_plan: true,
        }
    }

    fn ledger() -> ActiveEvidenceAttemptLedger {
        ActiveEvidenceAttemptLedger::new(ActiveEvidenceAttemptPolicy {
            expected_domain_fingerprint_sha256: DOMAIN,
            expected_acquisition_protocol_sha256: PROTOCOL,
            maximum_attempts_per_action: 3,
        }).unwrap()
    }

    fn request(action_id: u64, attempt_id: u64) -> EvidenceAttemptRequest {
        EvidenceAttemptRequest {
            action_id,
            attempt_id,
            action_contract_sha256: [0x33; 32],
            domain_fingerprint_sha256: DOMAIN,
            acquisition_protocol_sha256: PROTOCOL,
            pre_attempt_navigation_state_sha256: PRE_STATE,
        }
    }

    fn time_policy() -> NavigationTimeValidityPolicy {
        NavigationTimeValidityPolicy {
            policy_id: 8,
            policy_sha256: [0x88; 32],
            expected_time_basis_sha256: [0x99; 32],
            expected_estimator_or_predictor_revision_sha256: [0xaa; 32],
            expected_model_domain_support_sha256: [0xbb; 32],
            maximum_observation_age_ns: 100,
            maximum_acquisition_to_receipt_delay_ns: 30,
            maximum_receipt_to_processing_complete_ns: 20,
            maximum_prediction_horizon_ns: 50,
        }
    }

    fn time_input(kind: NavigationTimeClaimKind) -> NavigationTimeValidityInput {
        let interval = |lower_ns, upper_ns| NavigationTimeInterval {
            lower_ns,
            upper_ns,
            time_basis_sha256: [0x99; 32],
        };
        NavigationTimeValidityInput {
            result_id: 300,
            episode_id: 400,
            claim_kind: kind,
            measurement_event_time: interval(10, 10),
            payload_receipt_time: interval(20, 20),
            processing_start_time: interval(21, 21),
            processing_completion_time: interval(25, 25),
            decision_cutoff_time: interval(30, 30),
            target_valid_time: interval(30, 30),
            clock_correlation_sha256: [0xcc; 32],
            source_manifest_sha256: [0xdd; 32],
            estimator_or_predictor_revision_sha256: [0xaa; 32],
            model_domain_support_sha256: [0xbb; 32],
        }
    }

    fn binding(action_id: u64, attempt_id: u64) -> QualifiedEvidenceBinding {
        QualifiedEvidenceBinding {
            action_id,
            attempt_id,
            result_id: 300,
            source_manifest_sha256: [0xdd; 32],
            payload_sha256: [0xee; 32],
            qualification_receipt_sha256: [0xff; 32],
        }
    }

    fn qualified() -> AcquisitionAttemptOutcome {
        AcquisitionAttemptOutcome::QualifiedEvidenceAvailable {
            payload_sha256: [0xee; 32],
            qualification_receipt_sha256: [0xff; 32],
        }
    }

    #[test]
    fn reliability_interval_and_campaign_lineage_survive_into_consumed_attempt_report() {
        let selected_plan = reliable_plan();
        let selected = selected_plan.pareto_frontier[0].reliability;
        let mut ledger = ledger();
        let report = record_reliably_selected_evidence_outcome(
            &selected_plan,
            &mut ledger,
            request(2, 120),
            qualified(),
            PRE_STATE,
            Some(binding(2, 120)),
            Some((time_policy(), time_input(NavigationTimeClaimKind::DecisionEpochNowcast))),
        )
        .unwrap();

        assert_eq!(report.action_id, 2);
        assert_eq!(report.attempt_id, 120);
        assert_eq!(report.selected_acquisition_reliability, Some(selected));
        let provenance = report.selected_acquisition_reliability.unwrap();
        assert_eq!(provenance.probability_lower_bound, 0.8);
        assert_eq!(provenance.probability_upper_bound, 0.95);
        assert_eq!(provenance.independent_campaign_block_count, 10);
        assert_eq!(provenance.source_manifest_sha256, [0x81; 32]);
        assert_eq!(provenance.calibration_cohort_sha256, [0x91; 32]);
        assert_eq!(provenance.bound_method_sha256, [0xa1; 32]);
        assert_eq!(report.disposition, EvidenceTransactionDisposition::ReadyForIndependentPosteriorAudit);
        assert!(report.eligible_for_independent_posterior_audit);
        assert!(!report.posterior_updated);
        assert!(!report.navigation_authorized);
        assert_eq!(ledger.attempts_for_action(2), 1);
    }

    #[test]
    fn only_the_recommended_action_can_start_an_attempt() {
        let mut ledger = ledger();
        assert_eq!(
            record_selected_evidence_outcome(
                &plan(), &mut ledger, request(1, 100),
                AcquisitionAttemptOutcome::FailedOrTimedOut { failure_reason_sha256: [0x21; 32] },
                PRE_STATE, None, None,
            ),
            Err(EvidenceTransactionError::AttemptDoesNotMatchRecommendedAction)
        );
        assert!(ledger.events().is_empty());
    }

    #[test]
    fn unknown_outcome_blocks_every_action_until_reconciled() {
        let selected_plan = plan();
        let mut ledger = ledger();
        let report = record_selected_evidence_outcome(
            &selected_plan, &mut ledger, request(2, 100),
            AcquisitionAttemptOutcome::UnknownOrUnreconciled { unknown_reason_sha256: [0x31; 32] },
            PRE_STATE, None, None,
        ).unwrap();
        assert_eq!(report.disposition, EvidenceTransactionDisposition::ReconciliationRequired);
        assert!(!report.posterior_updated);
        assert!(!report.navigation_authorized);
        assert_eq!(ledger.begin_attempt(request(1, 101)), Err(ActiveEvidenceAttemptError::UnresolvedOutcomeBlocksNewAttempt(100)));

        ledger.reconcile_unknown(
            100,
            ResolvedAcquisitionOutcome::FailedOrTimedOut { failure_reason_sha256: [0x32; 32] },
            [0x33; 32],
            PRE_STATE,
        ).unwrap();
        assert_eq!(ledger.unresolved_attempt_id(), None);
        assert!(matches!(ledger.attempt(100).unwrap().disposition, AcquisitionAttemptDisposition::Reconciled { .. }));
    }

    #[test]
    fn valid_nowcast_only_reaches_independent_audit_staging() {
        let mut ledger = ledger();
        let report = record_selected_evidence_outcome(
            &plan(), &mut ledger, request(2, 102), qualified(), PRE_STATE,
            Some(binding(2, 102)), Some((time_policy(), time_input(NavigationTimeClaimKind::DecisionEpochNowcast))),
        ).unwrap();
        assert_eq!(report.disposition, EvidenceTransactionDisposition::ReadyForIndependentPosteriorAudit);
        assert!(report.eligible_for_independent_posterior_audit);
        assert_eq!(report.time_report.unwrap().disposition, NavigationTimeDisposition::DecisionEpochNowcastApplicable);
        assert!(!report.posterior_updated);
        assert!(!report.navigation_authorized);
    }

    #[test]
    fn qualified_but_stale_observation_is_not_admitted_to_current_audit() {
        let mut ledger = ledger();
        let mut stale = time_input(NavigationTimeClaimKind::DecisionEpochNowcast);
        stale.measurement_event_time.lower_ns = 0;
        stale.measurement_event_time.upper_ns = 0;
        stale.payload_receipt_time.lower_ns = 25;
        stale.payload_receipt_time.upper_ns = 25;
        stale.processing_start_time.lower_ns = 26;
        stale.processing_start_time.upper_ns = 26;
        stale.processing_completion_time.lower_ns = 27;
        stale.processing_completion_time.upper_ns = 27;
        stale.decision_cutoff_time.lower_ns = 150;
        stale.decision_cutoff_time.upper_ns = 150;
        stale.target_valid_time.lower_ns = 150;
        stale.target_valid_time.upper_ns = 150;
        let report = record_selected_evidence_outcome(
            &plan(), &mut ledger, request(2, 103), qualified(), PRE_STATE,
            Some(binding(2, 103)), Some((time_policy(), stale)),
        ).unwrap();
        assert_eq!(report.disposition, EvidenceTransactionDisposition::TemporallyInapplicable(NavigationTimeDisposition::StaleObservation));
        assert!(!report.eligible_for_independent_posterior_audit);
        assert_eq!(ledger.begin_attempt(request(2, 104)), Err(ActiveEvidenceAttemptError::ActionAlreadyQualified(2)));
    }

    #[test]
    fn future_prediction_and_historical_estimate_remain_distinct_from_nowcast() {
        let mut future_ledger = ledger();
        let mut future = time_input(NavigationTimeClaimKind::FutureEpochPrediction);
        future.target_valid_time.lower_ns = 40;
        future.target_valid_time.upper_ns = 45;
        let future_report = record_selected_evidence_outcome(
            &plan(), &mut future_ledger, request(2, 105), qualified(), PRE_STATE,
            Some(binding(2, 105)), Some((time_policy(), future)),
        ).unwrap();
        assert_eq!(future_report.disposition, EvidenceTransactionDisposition::ReadyForIndependentForecastAudit);
        assert!(!future_report.eligible_for_independent_posterior_audit);

        let mut historical_ledger = ledger();
        let mut historical = time_input(NavigationTimeClaimKind::EventTimeEstimate);
        historical.target_valid_time = historical.measurement_event_time;
        let historical_report = record_selected_evidence_outcome(
            &plan(), &mut historical_ledger, request(2, 106), qualified(), PRE_STATE,
            Some(binding(2, 106)), Some((time_policy(), historical)),
        ).unwrap();
        assert_eq!(historical_report.disposition, EvidenceTransactionDisposition::HistoricalReplayOnly);
        assert!(!historical_report.eligible_for_independent_posterior_audit);
    }

    #[test]
    fn acquired_payload_cannot_be_laundered_across_attempt_or_action_ids() {
        let mut ledger = ledger();
        let result = record_selected_evidence_outcome(
            &plan(),
            &mut ledger,
            request(2, 110),
            qualified(),
            PRE_STATE,
            Some(binding(2, 999)),
            Some((time_policy(), time_input(NavigationTimeClaimKind::DecisionEpochNowcast))),
        ).unwrap();
        assert_eq!(result.disposition, EvidenceTransactionDisposition::EvidenceBindingMismatch);
        assert!(!result.eligible_for_independent_posterior_audit);
        assert!(!result.posterior_updated);
        assert!(!result.navigation_authorized);
    }

    #[test]
    fn time_result_and_source_manifest_must_match_the_qualified_receipt_binding() {
        let mut ledger = ledger();
        let mut mismatched_time = time_input(NavigationTimeClaimKind::DecisionEpochNowcast);
        mismatched_time.result_id = 301;
        let result = record_selected_evidence_outcome(
            &plan(),
            &mut ledger,
            request(2, 111),
            qualified(),
            PRE_STATE,
            Some(binding(2, 111)),
            Some((time_policy(), mismatched_time)),
        ).unwrap();
        assert_eq!(result.disposition, EvidenceTransactionDisposition::EvidenceBindingMismatch);
        assert!(!result.eligible_for_independent_posterior_audit);
    }

    #[test]
    fn qualified_acquisition_without_payload_receipt_binding_is_not_time_gated() {
        let mut ledger = ledger();
        let result = record_selected_evidence_outcome(
            &plan(),
            &mut ledger,
            request(2, 112),
            qualified(),
            PRE_STATE,
            None,
            Some((time_policy(), time_input(NavigationTimeClaimKind::DecisionEpochNowcast))),
        ).unwrap();
        assert_eq!(result.disposition, EvidenceTransactionDisposition::EvidenceBindingMissing);
        assert!(result.time_report.is_none());
        assert!(!result.eligible_for_independent_posterior_audit);
    }

    #[test]
    fn failure_invalidity_and_missing_time_never_become_audit_eligible() {
        let mut failed_ledger = ledger();
        let failed = record_selected_evidence_outcome(
            &plan(), &mut failed_ledger, request(2, 107),
            AcquisitionAttemptOutcome::FailedOrTimedOut { failure_reason_sha256: [0x41; 32] },
            PRE_STATE, None, Some((time_policy(), time_input(NavigationTimeClaimKind::DecisionEpochNowcast))),
        ).unwrap();
        assert_eq!(failed.disposition, EvidenceTransactionDisposition::AcquisitionFailed);
        assert!(failed.time_report.is_none());
        assert!(!failed.eligible_for_independent_posterior_audit);

        let mut invalid_ledger = ledger();
        let invalid = record_selected_evidence_outcome(
            &plan(), &mut invalid_ledger, request(2, 108),
            AcquisitionAttemptOutcome::AcquiredButInvalid { payload_sha256: [0x42; 32], invalidity_receipt_sha256: [0x43; 32] },
            PRE_STATE, None, None,
        ).unwrap();
        assert_eq!(invalid.disposition, EvidenceTransactionDisposition::AcquiredEvidenceInvalid);
        assert!(invalid.time_report.is_none());
        assert!(!invalid.eligible_for_independent_posterior_audit);

        let mut no_time_ledger = ledger();
        let no_time = record_selected_evidence_outcome(
            &plan(), &mut no_time_ledger, request(2, 109), qualified(), PRE_STATE, Some(binding(2, 109)), None,
        ).unwrap();
        assert_eq!(no_time.disposition, EvidenceTransactionDisposition::TimeAssessmentMissing);
        assert!(!no_time.eligible_for_independent_posterior_audit);
        assert!(!no_time.posterior_updated);
    }
}
