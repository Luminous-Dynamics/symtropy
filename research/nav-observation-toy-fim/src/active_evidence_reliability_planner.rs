//! Reliability-aware decision-relevant active evidence planning.
//!
//! Composes the decision-aware planner with the domain/protocol-bound
//! valid-acquisition probability admission contract. The result keeps the
//! conditional outcome-set value, acquisition probability interval, expected
//! decision-pair yield, protection-level effect, latency and energy visible.
//!
//! The lower-bound expected pair yield is a planning metric under the admitted
//! external probability contract, not an observed yield or a physical guarantee.
//! This module does not estimate acquisition probabilities, authenticate the
//! interval source, execute actions, update posteriors or authorize navigation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::active_evidence_acquisition::{
    ActiveEvidencePolicy, EvidenceActionCandidate, plan_active_evidence_acquisition,
    ActiveEvidenceAcquisitionError, AcquisitionPlanReason,
};
use crate::active_evidence_decision_planner::{
    ActiveEvidenceDecisionError, ActiveEvidenceDecisionPlan,
    EvidenceActionDecisionAssessment, plan_active_evidence_with_decision_relevance,
};
use crate::active_evidence_reliability::{
    ActiveEvidenceReliabilityAdmission, ActiveEvidenceReliabilityError,
    ActiveEvidenceReliabilityPolicy, AdmittedAcquisitionReliability,
    RejectedAcquisitionReliability, ValidAcquisitionProbabilityContract,
    admit_active_evidence_reliability,
};
use crate::decision_relevant_discrimination::DecisionRelevantDiscriminationReport;

/// One admitted action scored with a conservative acquisition-probability interval.
#[derive(Clone, Debug, PartialEq)]
pub struct ReliableEvidenceActionAssessment {
    pub conditional: EvidenceActionDecisionAssessment,
    pub reliability: AdmittedAcquisitionReliability,
    /// probability_lower_bound * cross-decision pairs conditionally separated.
    /// This is a lower-bound expectation under the external admitted contract.
    pub expected_decision_pair_yield_lower_bound: f64,
    /// probability_upper_bound * cross-decision pairs conditionally separated.
    pub expected_decision_pair_yield_upper_bound: f64,
    /// probability_lower_bound * signed conditional PL reduction.
    /// Negative values preserve the expected penalty of a bound expansion.
    pub expected_protection_level_reduction_lower_bound_m: f64,
    /// probability_upper_bound * signed conditional PL reduction.
    pub expected_protection_level_reduction_upper_bound_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliableEvidencePlanReason {
    TargetAlreadySatisfied,
    LowestLatencyDecisionResolvingTarget,
    LowestLatencyTargetReachingByProtectionReduction,
    BestExpectedDecisionResolution,
    BestExpectedProtectionReduction,
    NoReliablyAdmittedAction,
    NoConditionalActionValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReliableActiveEvidencePlan {
    pub reason: ReliableEvidencePlanReason,
    pub recommended_action_id: Option<u64>,
    pub pareto_frontier: Vec<ReliableEvidenceActionAssessment>,
    pub reliability_rejections: Vec<RejectedAcquisitionReliability>,
    pub hypothesis_inventory_sha256: Option<[u8; 32]>,
    pub decision_partition_sha256: Option<[u8; 32]>,
    pub decision_assignments: Option<Vec<crate::decision_relevant_discrimination::HypothesisDecisionAssignment>>,
    pub conditional_target_reached: bool,
    /// Always true: this planner cannot authorize a navigation transition.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReliableEvidencePlannerError {
    Acquisition(ActiveEvidenceAcquisitionError),
    Decision(ActiveEvidenceDecisionError),
    Reliability(ActiveEvidenceReliabilityError),
    MissingConditionalAssessment(u64),
    ArithmeticOverflow(u64),
}

impl fmt::Display for ReliableEvidencePlannerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquisition(error) => write!(f, "active acquisition policy rejected candidates: {error}"),
            Self::Decision(error) => write!(f, "decision-aware discrimination validation failed: {error}"),
            Self::Reliability(error) => write!(f, "acquisition reliability admission failed: {error}"),
            Self::MissingConditionalAssessment(id) => write!(f, "admitted action {id} has no conditional decision assessment"),
            Self::ArithmeticOverflow(id) => write!(f, "reliability-adjusted value overflowed for action {id}"),
        }
    }
}

impl std::error::Error for ReliableEvidencePlannerError {}

/// Plan across resource-feasible actions after admitting each action's
/// externally computed probability interval for a fresh, valid, independently
/// qualified observation.
///
/// The probability contract is bound to the exact domain and acquisition
/// protocol through the existing #7082 admission gate. The decision projection
/// is validated by the existing #7145 planner. Missing/weak reliability
/// contracts are explicit rejections and cannot be treated as p=1.
///
/// The wrapper recomputes single-candidate conditional assessments after the
/// multi-action validation pass so actions dominated on conditional value but
/// strong on admitted acquisition reliability are not discarded prematurely.
pub fn plan_active_evidence_with_reliability(
    policy: &ActiveEvidencePolicy,
    reliability_policy: &ActiveEvidenceReliabilityPolicy,
    candidates: &[EvidenceActionCandidate],
    decision_reports: &[DecisionRelevantDiscriminationReport],
    reliability_contracts: &[ValidAcquisitionProbabilityContract],
) -> Result<ReliableActiveEvidencePlan, ReliableEvidencePlannerError> {
    let baseline = match plan_active_evidence_acquisition(policy, candidates) {
        Ok(plan) => Some(plan),
        Err(ActiveEvidenceAcquisitionError::NoFeasibleAction) => None,
        Err(error) => return Err(ReliableEvidencePlannerError::Acquisition(error)),
    };
    if baseline.as_ref().is_some_and(|plan| plan.reason == AcquisitionPlanReason::TargetAlreadySatisfied) {
        return Ok(ReliableActiveEvidencePlan {
            reason: ReliableEvidencePlanReason::TargetAlreadySatisfied,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            reliability_rejections: Vec::new(),
            hypothesis_inventory_sha256: None,
            decision_partition_sha256: None,
            decision_assignments: None,
            conditional_target_reached: true,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let known_groups: BTreeSet<u64> =
        policy.already_observed_dependency_groups.iter().copied().collect();
    let resource_feasible: Vec<&EvidenceActionCandidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.available
                && candidate.freshness_and_calibration_preconditions_met
                && candidate.worst_case_latency_ns <= policy.maximum_action_latency_ns
                && candidate.worst_case_energy_uj <= policy.maximum_action_energy_uj
                && (!policy.require_new_dependency_group
                    || !known_groups.contains(&candidate.dependency_group_id))
        })
        .collect();

    if resource_feasible.is_empty() {
        return Ok(ReliableActiveEvidencePlan {
            reason: ReliableEvidencePlanReason::NoReliablyAdmittedAction,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            reliability_rejections: Vec::new(),
            hypothesis_inventory_sha256: None,
            decision_partition_sha256: None,
            decision_assignments: None,
            conditional_target_reached: false,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let action_ids: Vec<u64> = resource_feasible.iter().map(|candidate| candidate.action_id).collect();
    let admission: ActiveEvidenceReliabilityAdmission = admit_active_evidence_reliability(
        reliability_policy,
        &action_ids,
        reliability_contracts,
    )
    .map_err(ReliableEvidencePlannerError::Reliability)?;

    let admitted_by_action: BTreeMap<u64, AdmittedAcquisitionReliability> = admission
        .admitted_actions
        .iter()
        .copied()
        .map(|item| (item.action_id, item))
        .collect();

    let admitted_candidates: Vec<EvidenceActionCandidate> = resource_feasible
        .iter()
        .filter(|candidate| admitted_by_action.contains_key(&candidate.action_id))
        .map(|candidate| (**candidate).clone())
        .collect();

    if admitted_candidates.is_empty() {
        return Ok(ReliableActiveEvidencePlan {
            reason: ReliableEvidencePlanReason::NoReliablyAdmittedAction,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            reliability_rejections: admission.rejected_actions,
            hypothesis_inventory_sha256: None,
            decision_partition_sha256: None,
            decision_assignments: None,
            conditional_target_reached: false,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let admitted_ids: BTreeSet<u64> = admitted_candidates.iter().map(|candidate| candidate.action_id).collect();
    let admitted_reports: Vec<DecisionRelevantDiscriminationReport> = decision_reports
        .iter()
        .filter(|report| admitted_ids.contains(&report.action_id))
        .cloned()
        .collect();

    // Validate report completeness, common inventory and common partition
    // across the entire reliability-admitted candidate set.
    let combined_plan = match plan_active_evidence_with_decision_relevance(
        policy,
        &admitted_candidates,
        &admitted_reports,
    ) {
        Ok(plan) => plan,
        Err(ActiveEvidenceDecisionError::Acquisition(ActiveEvidenceAcquisitionError::NoFeasibleAction)) => {
            return Ok(ReliableActiveEvidencePlan {
                reason: ReliableEvidencePlanReason::NoConditionalActionValue,
                recommended_action_id: None,
                pareto_frontier: Vec::new(),
                reliability_rejections: admission.rejected_actions,
                hypothesis_inventory_sha256: None,
                decision_partition_sha256: None,
                decision_assignments: None,
                conditional_target_reached: false,
                authority_transition_not_authorized_by_plan: true,
            });
        }
        Err(error) => return Err(ReliableEvidencePlannerError::Decision(error)),
    };

    if combined_plan.recommended_action_id.is_none() {
        return Ok(ReliableActiveEvidencePlan {
            reason: ReliableEvidencePlanReason::NoConditionalActionValue,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            reliability_rejections: admission.rejected_actions,
            hypothesis_inventory_sha256: combined_plan.hypothesis_inventory_sha256,
            decision_partition_sha256: combined_plan.decision_partition_sha256,
            decision_assignments: combined_plan.decision_assignments,
            conditional_target_reached: false,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let report_by_action: BTreeMap<u64, &DecisionRelevantDiscriminationReport> = admitted_reports
        .iter()
        .map(|report| (report.action_id, report))
        .collect();

    let mut assessments = Vec::new();
    for candidate in &admitted_candidates {
        let report = report_by_action
            .get(&candidate.action_id)
            .copied()
            .ok_or(ReliableEvidencePlannerError::MissingConditionalAssessment(candidate.action_id))?;

        // The one-candidate planner call reuses its full report validator to
        // materialize this candidate's assessment even if the multi-action
        // conditional-only Pareto pass had dominated it.
        let single = plan_active_evidence_with_decision_relevance(
            policy,
            std::slice::from_ref(candidate),
            std::slice::from_ref(report),
        )
        .map_err(ReliableEvidencePlannerError::Decision)?;
        let conditional = single
            .pareto_frontier
            .first()
            .cloned()
            .ok_or(ReliableEvidencePlannerError::MissingConditionalAssessment(candidate.action_id))?;
        let reliability = admitted_by_action[&candidate.action_id];

        let pair_count = conditional.distinguishable_decision_pair_count as f64;
        let expected_pair_lower = reliability.probability_lower_bound * pair_count;
        let expected_pair_upper = reliability.probability_upper_bound * pair_count;
        let expected_pl_at_lower_probability =
            reliability.probability_lower_bound * conditional.conditional_protection_level_delta_m;
        let expected_pl_at_upper_probability =
            reliability.probability_upper_bound * conditional.conditional_protection_level_delta_m;
        if !expected_pair_lower.is_finite()
            || !expected_pair_upper.is_finite()
            || !expected_pl_at_lower_probability.is_finite()
            || !expected_pl_at_upper_probability.is_finite()
        {
            return Err(ReliableEvidencePlannerError::ArithmeticOverflow(candidate.action_id));
        }
        // Multiplication by a negative signed PL change reverses interval order.
        let expected_pl_lower = expected_pl_at_lower_probability.min(expected_pl_at_upper_probability);
        let expected_pl_upper = expected_pl_at_lower_probability.max(expected_pl_at_upper_probability);

        assessments.push(ReliableEvidenceActionAssessment {
            conditional,
            reliability,
            expected_decision_pair_yield_lower_bound: expected_pair_lower,
            expected_decision_pair_yield_upper_bound: expected_pair_upper,
            expected_protection_level_reduction_lower_bound_m: expected_pl_lower,
            expected_protection_level_reduction_upper_bound_m: expected_pl_upper,
        });
    }

    let mut frontier = Vec::new();
    for candidate in &assessments {
        let dominated = assessments.iter().any(|other| {
            other.conditional.action.action_id != candidate.conditional.action.action_id
                && dominates(other, candidate)
        });
        if !dominated {
            frontier.push(candidate.clone());
        }
    }

    frontier.sort_by(|left, right| {
        right.expected_decision_pair_yield_lower_bound
            .total_cmp(&left.expected_decision_pair_yield_lower_bound)
            .then_with(|| right.expected_protection_level_reduction_lower_bound_m
                .total_cmp(&left.expected_protection_level_reduction_lower_bound_m))
            .then_with(|| left.conditional.action.worst_case_latency_ns.cmp(&right.conditional.action.worst_case_latency_ns))
            .then_with(|| left.conditional.action.worst_case_energy_uj.cmp(&right.conditional.action.worst_case_energy_uj))
            .then_with(|| left.conditional.action.action_id.cmp(&right.conditional.action.action_id))
    });

    let target_reaching: Vec<&ReliableEvidenceActionAssessment> = frontier
        .iter()
        .filter(|assessment| assessment.conditional.action.reaches_target_if_acquisition_qualifies)
        .collect();

    let (reason, selected) = if !target_reaching.is_empty() {
        let selected = *target_reaching.iter().max_by(|left, right| compare_value(left, right))
            .ok_or(ReliableEvidencePlannerError::MissingConditionalAssessment(0))?;
        let reason = if selected.expected_decision_pair_yield_lower_bound > 0.0 {
            ReliableEvidencePlanReason::LowestLatencyDecisionResolvingTarget
        } else {
            ReliableEvidencePlanReason::LowestLatencyTargetReachingByProtectionReduction
        };
        (reason, selected)
    } else if assessments.iter().any(|assessment| assessment.expected_decision_pair_yield_lower_bound > 0.0) {
        let selected = frontier.iter().max_by(|left, right| compare_value(left, right))
            .ok_or(ReliableEvidencePlannerError::MissingConditionalAssessment(0))?;
        (ReliableEvidencePlanReason::BestExpectedDecisionResolution, selected)
    } else {
        let selected = frontier.iter().max_by(|left, right| compare_pl_value(left, right))
            .ok_or(ReliableEvidencePlannerError::MissingConditionalAssessment(0))?;
        (ReliableEvidencePlanReason::BestExpectedProtectionReduction, selected)
    };

    let recommended_action_id = selected.conditional.action.action_id;
    let conditional_target_reached = selected.conditional.action.reaches_target_if_acquisition_qualifies;
    Ok(ReliableActiveEvidencePlan {
        reason,
        recommended_action_id: Some(recommended_action_id),
        pareto_frontier: frontier,
        reliability_rejections: admission.rejected_actions,
        hypothesis_inventory_sha256: combined_plan.hypothesis_inventory_sha256,
        decision_partition_sha256: combined_plan.decision_partition_sha256,
        decision_assignments: combined_plan.decision_assignments,
        conditional_target_reached,
        authority_transition_not_authorized_by_plan: true,
    })
}

fn compare_value(
    left: &ReliableEvidenceActionAssessment,
    right: &ReliableEvidenceActionAssessment,
) -> std::cmp::Ordering {
    left.expected_decision_pair_yield_lower_bound
        .total_cmp(&right.expected_decision_pair_yield_lower_bound)
        .then_with(|| left.expected_protection_level_reduction_lower_bound_m
            .total_cmp(&right.expected_protection_level_reduction_lower_bound_m))
        .then_with(|| right.conditional.action.worst_case_latency_ns.cmp(&left.conditional.action.worst_case_latency_ns))
        .then_with(|| right.conditional.action.worst_case_energy_uj.cmp(&left.conditional.action.worst_case_energy_uj))
        .then_with(|| right.conditional.action.action_id.cmp(&left.conditional.action.action_id))
}

fn compare_pl_value(
    left: &ReliableEvidenceActionAssessment,
    right: &ReliableEvidenceActionAssessment,
) -> std::cmp::Ordering {
    left.expected_protection_level_reduction_lower_bound_m
        .total_cmp(&right.expected_protection_level_reduction_lower_bound_m)
        .then_with(|| right.conditional.action.worst_case_latency_ns.cmp(&left.conditional.action.worst_case_latency_ns))
        .then_with(|| right.conditional.action.worst_case_energy_uj.cmp(&left.conditional.action.worst_case_energy_uj))
        .then_with(|| right.conditional.action.action_id.cmp(&left.conditional.action.action_id))
}

fn dominates(
    left: &ReliableEvidenceActionAssessment,
    right: &ReliableEvidenceActionAssessment,
) -> bool {
    let no_worse = left.expected_decision_pair_yield_lower_bound
        >= right.expected_decision_pair_yield_lower_bound
        && left.expected_protection_level_reduction_lower_bound_m
            >= right.expected_protection_level_reduction_lower_bound_m
        && left.conditional.action.worst_case_latency_ns
            <= right.conditional.action.worst_case_latency_ns
        && left.conditional.action.worst_case_energy_uj
            <= right.conditional.action.worst_case_energy_uj;
    let strictly_better = left.expected_decision_pair_yield_lower_bound
        > right.expected_decision_pair_yield_lower_bound
        || left.expected_protection_level_reduction_lower_bound_m
            > right.expected_protection_level_reduction_lower_bound_m
        || left.conditional.action.worst_case_latency_ns
            < right.conditional.action.worst_case_latency_ns
        || left.conditional.action.worst_case_energy_uj
            < right.conditional.action.worst_case_energy_uj;
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_acquisition::EvidenceActionKind;
    use crate::active_evidence_reliability::{AcquisitionCampaignBlock, AcquisitionCampaignOutcome};
    use crate::decision_relevant_discrimination::{project_decision_relevant_discrimination, HypothesisDecisionAssignment};
    use crate::uwb_hypothesis_discrimination::{UwbHypothesisDiscriminationReport, UwbHypothesisPairSeparation};

    const DOMAIN: [u8; 32] = [0x11; 32];
    const PROTOCOL: [u8; 32] = [0x22; 32];

    fn policy() -> ActiveEvidencePolicy {
        ActiveEvidencePolicy {
            current_protection_level_upper_m: 3.0,
            target_protection_level_upper_m: 1.0,
            maximum_action_latency_ns: 100,
            maximum_action_energy_uj: 1_000,
            require_new_dependency_group: false,
            already_observed_dependency_groups: vec![10],
        }
    }

    fn reliability_policy() -> ActiveEvidenceReliabilityPolicy {
        ActiveEvidenceReliabilityPolicy {
            expected_domain_fingerprint_sha256: DOMAIN,
            expected_acquisition_protocol_sha256: PROTOCOL,
            minimum_probability_lower_bound: 0.1,
            maximum_confidence_error_probability: 0.05,
            minimum_independent_campaign_blocks: 3,
        }
    }

    fn candidate(id: u64, pl: f64, latency: u64, energy: u64) -> EvidenceActionCandidate {
        EvidenceActionCandidate {
            action_id: id,
            kind: EvidenceActionKind::RequestIndependentAnchor,
            dependency_group_id: 20 + id,
            conditional_post_action_protection_level_upper_m: pl,
            worst_case_latency_ns: latency,
            worst_case_energy_uj: energy,
            available: true,
            freshness_and_calibration_preconditions_met: true,
        }
    }

    fn report(action_id: u64, gaps: [f64; 3]) -> DecisionRelevantDiscriminationReport {
        let ids = [(1, 2), (1, 3), (2, 3)];
        let pairs: Vec<_> = ids.into_iter().zip(gaps).map(|((lower, upper), gap)| UwbHypothesisPairSeparation {
            lower_hypothesis_id: lower,
            upper_hypothesis_id: upper,
            normalized_outcome_gap: gap,
            guaranteed_distinguishable: gap > 0.0,
        }).collect();
        let count = pairs.iter().filter(|pair| pair.guaranteed_distinguishable).count();
        let upstream = UwbHypothesisDiscriminationReport {
            action_id,
            hypothesis_inventory_sha256: [0x55; 32],
            outcome_model_sha256: [action_id as u8; 32],
            normalization_sha256: [0x66; 32],
            hypothesis_count: 3,
            hypothesis_pair_count: 3,
            distinguishable_pair_count: count,
            ambiguous_pair_count: 3 - count,
            minimum_positive_gap: pairs.iter().filter(|pair| pair.guaranteed_distinguishable).map(|pair| pair.normalized_outcome_gap).reduce(f64::min),
            maximum_pair_gap: pairs.iter().map(|pair| pair.normalized_outcome_gap).fold(0.0, f64::max),
            conditional_on_successful_valid_observation: true,
            pair_separations: pairs,
        };
        project_decision_relevant_discrimination(
            &upstream,
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

    fn contract(action_id: u64, lower: f64, upper: f64, successes: usize) -> ValidAcquisitionProbabilityContract {
        let campaign_blocks = (0..10).map(|index| AcquisitionCampaignBlock {
            block_id: 1 + index as u64,
            independence_group_id: 101 + index as u64,
            outcome: if index < successes {
                AcquisitionCampaignOutcome::ValidQualifiedSuccess
            } else {
                AcquisitionCampaignOutcome::FailedOrTimedOut
            },
        }).collect();
        ValidAcquisitionProbabilityContract {
            action_id,
            domain_fingerprint_sha256: DOMAIN,
            acquisition_protocol_sha256: PROTOCOL,
            source_manifest_sha256: [0x80 + action_id as u8; 32],
            calibration_cohort_sha256: [0x90 + action_id as u8; 32],
            bound_method_sha256: [0xa0 + action_id as u8; 32],
            confidence_error_probability: 0.05,
            probability_lower_bound: lower,
            probability_upper_bound: upper,
            campaign_blocks,
        }
    }

    #[test]
    fn high_conditional_value_but_low_success_probability_loses_on_expected_useful_yield() {
        let candidates = [candidate(1, 2.5, 40, 200), candidate(2, 2.6, 40, 200)];
        // Action 1 separates both cross-decision pairs, action 2 separates one.
        let reports = [report(1, [0.0, 2.0, 2.0]), report(2, [0.0, 1.0, 0.0])];
        let contracts = [contract(1, 0.2, 0.6, 4), contract(2, 0.8, 0.95, 9)];
        let plan = plan_active_evidence_with_reliability(
            &policy(), &reliability_policy(), &candidates, &reports, &contracts,
        ).unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        let a = plan.pareto_frontier.iter().find(|item| item.conditional.action.action_id == 1).unwrap();
        let b = plan.pareto_frontier.iter().find(|item| item.conditional.action.action_id == 2).unwrap();
        assert_eq!(a.conditional.distinguishable_decision_pair_count, 2);
        assert_eq!(b.conditional.distinguishable_decision_pair_count, 1);
        assert!((a.expected_decision_pair_yield_lower_bound - 0.4).abs() < 1e-12);
        assert!((b.expected_decision_pair_yield_lower_bound - 0.8).abs() < 1e-12);
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn action_below_the_probability_lower_bound_threshold_is_rejected_not_assumed_successful() {
        let candidates = [candidate(1, 2.2, 40, 200), candidate(2, 2.5, 40, 200)];
        let reports = [report(1, [0.0, 2.0, 2.0]), report(2, [0.0, 1.0, 0.0])];
        let contracts = [contract(1, 0.05, 0.7, 4), contract(2, 0.8, 0.95, 9)];
        let plan = plan_active_evidence_with_reliability(
            &policy(), &reliability_policy(), &candidates, &reports, &contracts,
        ).unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        assert_eq!(plan.reliability_rejections.len(), 1);
        assert_eq!(plan.reliability_rejections[0].action_id, 1);
        assert_eq!(plan.reliability_rejections[0].reason, crate::active_evidence_reliability::ReliabilityRejectionReason::ProbabilityLowerBoundTooSmall);
    }

    #[test]
    fn negative_expected_pl_change_keeps_the_probability_interval_ordered() {
        let candidates = [candidate(1, 3.5, 40, 200)];
        let reports = [report(1, [0.0, 2.0, 2.0])];
        let contracts = [contract(1, 0.8, 0.95, 9)];
        let plan = plan_active_evidence_with_reliability(
            &policy(), &reliability_policy(), &candidates, &reports, &contracts,
        ).unwrap();
        let item = &plan.pareto_frontier[0];
        assert!((item.expected_protection_level_reduction_lower_bound_m - (-0.475)).abs() < 1e-12);
        assert!((item.expected_protection_level_reduction_upper_bound_m - (-0.4)).abs() < 1e-12);
        assert!(item.expected_protection_level_reduction_lower_bound_m
            <= item.expected_protection_level_reduction_upper_bound_m);
    }

    #[test]
    fn fully_admitted_but_nonvaluable_actions_do_not_get_recommended() {
        let candidates = [candidate(1, 3.0, 40, 200)];
        let reports = [report(1, [0.0, 0.0, 0.0])];
        let contracts = [contract(1, 0.8, 0.95, 9)];
        let plan = plan_active_evidence_with_reliability(
            &policy(), &reliability_policy(), &candidates, &reports, &contracts,
        ).unwrap();
        assert_eq!(plan.reason, ReliableEvidencePlanReason::NoConditionalActionValue);
        assert_eq!(plan.recommended_action_id, None);
    }

    #[test]
    fn missing_campaign_contract_never_defaults_to_probability_one() {
        let candidates = [candidate(1, 2.2, 40, 200)];
        let reports = [report(1, [0.0, 2.0, 2.0])];
        let plan = plan_active_evidence_with_reliability(
            &policy(), &reliability_policy(), &candidates, &reports, &[],
        ).unwrap();
        assert_eq!(plan.recommended_action_id, None);
        assert_eq!(plan.reason, ReliableEvidencePlanReason::NoReliablyAdmittedAction);
        assert_eq!(plan.reliability_rejections[0].reason, crate::active_evidence_reliability::ReliabilityRejectionReason::MissingContract);
    }

    #[test]
    fn mismatched_acquisition_protocol_is_excluded() {
        let candidates = [candidate(1, 2.2, 40, 200)];
        let reports = [report(1, [0.0, 2.0, 2.0])];
        let mut bad = contract(1, 0.8, 0.95, 9);
        bad.acquisition_protocol_sha256 = [0x23; 32];
        let plan = plan_active_evidence_with_reliability(
            &policy(), &reliability_policy(), &candidates, &reports, &[bad],
        ).unwrap();
        assert_eq!(plan.recommended_action_id, None);
        assert_eq!(plan.reliability_rejections[0].reason, crate::active_evidence_reliability::ReliabilityRejectionReason::AcquisitionProtocolMismatch);
    }

    #[test]
    fn target_already_satisfied_does_not_require_a_sensing_reliability_contract() {
        let mut cfg = policy();
        cfg.current_protection_level_upper_m = 0.5;
        let plan = plan_active_evidence_with_reliability(
            &cfg, &reliability_policy(), &[], &[], &[],
        ).unwrap();
        assert_eq!(plan.reason, ReliableEvidencePlanReason::TargetAlreadySatisfied);
        assert_eq!(plan.recommended_action_id, None);
    }

    #[test]
    fn invalid_campaign_contract_is_not_silently_ignored() {
        let candidates = [candidate(1, 2.2, 40, 200)];
        let reports = [report(1, [0.0, 2.0, 2.0])];
        let mut bad = contract(1, 0.8, 0.95, 2);
        bad.campaign_blocks[0].independence_group_id = bad.campaign_blocks[1].independence_group_id;
        assert_eq!(
            plan_active_evidence_with_reliability(
                &policy(), &reliability_policy(), &candidates, &reports, &[bad],
            ),
            Err(ReliableEvidencePlannerError::Reliability(
                ActiveEvidenceReliabilityError::DuplicateCampaignIndependenceGroup(102)
            ))
        );
    }
}
