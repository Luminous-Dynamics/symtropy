//! Compose valid-acquisition reliability admission with decision-aware planning.
//!
//! Reliability remains an explicit eligibility contract, not a multiplier on
//! information gain or protection-level reduction. Only actions admitted by
//! the existing domain/protocol/campaign reliability kernel are passed to
//! the decision-aware planner. Rejected candidates remain visible with their
//! rejection reasons and are not silently assigned a success probability.
//!
//! This wrapper does not estimate reliability, authenticate source manifests,
//! prove campaign independence, execute actions or authorize navigation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::active_evidence_acquisition::{
    ActiveEvidencePolicy, EvidenceActionCandidate,
};
use crate::active_evidence_decision_planner::{
    ActiveEvidenceDecisionError, ActiveEvidenceDecisionPlan,
    plan_active_evidence_with_decision_relevance,
};
use crate::active_evidence_reliability::{
    ActiveEvidenceReliabilityError,
    ActiveEvidenceReliabilityPolicy, AdmittedAcquisitionReliability,
    RejectedAcquisitionReliability, ValidAcquisitionProbabilityContract,
    admit_active_evidence_reliability,
};
use crate::decision_relevant_discrimination::DecisionRelevantDiscriminationReport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliabilityGatedDecisionDisposition {
    PlanProduced,
    NoReliabilityAdmittedActions,
    NoResourceFeasibleActions,
    NoActionImprovesProtectionOrDecisionDiscrimination,
}

/// Result of applying reliability as an eligibility gate before
/// decision-relevant active evidence planning.
#[derive(Clone, Debug, PartialEq)]
pub struct ReliabilityGatedDecisionPlan {
    pub disposition: ReliabilityGatedDecisionDisposition,
    pub decision_plan: Option<ActiveEvidenceDecisionPlan>,
    /// Exact probability intervals and their campaign/source summaries.
    /// They are reported separately and are not multiplied into action value.
    pub admitted_acquisition_reliability: Vec<AdmittedAcquisitionReliability>,
    /// Candidates that are not ranked because their reliability contract was
    /// missing, unsupported, sparse, too weak or otherwise rejected upstream.
    pub rejected_acquisition_reliability: Vec<RejectedAcquisitionReliability>,
    /// Reliability-admitted candidates excluded only by resource/freshness/
    /// availability/dependency feasibility at this planning step.
    pub resource_excluded_action_ids: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReliabilityGatedDecisionError {
    InvalidCandidateIdentity,
    DuplicateCandidateId(u64),
    InvalidCandidateProtectionLevel(u64),
    ReliabilityAdmission(ActiveEvidenceReliabilityError),
    UnexpectedDecisionReport(u64),
    DuplicateDecisionReport(u64),
    DecisionPlanner(ActiveEvidenceDecisionError),
}

impl fmt::Display for ReliabilityGatedDecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCandidateIdentity => {
                write!(f, "candidate action and dependency-group IDs must be nonzero")
            }
            Self::DuplicateCandidateId(id) => write!(f, "candidate action ID is duplicated: {id}"),
            Self::InvalidCandidateProtectionLevel(id) => {
                write!(f, "candidate action {id} has an invalid conditional protection level")
            }
            Self::ReliabilityAdmission(error) => write!(f, "reliability admission failed: {error}"),
            Self::UnexpectedDecisionReport(id) => {
                write!(f, "decision report references an unknown action: {id}")
            }
            Self::DuplicateDecisionReport(id) => {
                write!(f, "decision report is duplicated for action: {id}")
            }
            Self::DecisionPlanner(error) => write!(f, "decision-aware planner failed: {error}"),
        }
    }
}

impl std::error::Error for ReliabilityGatedDecisionError {}

/// Compute the existing domain/protocol/campaign reliability admission, then
/// apply it as an eligibility gate before decision-aware action ranking.
///
/// Every candidate is passed through the existing #7082 admission kernel.
/// Rejected candidates are returned with their reasons and excluded from the
/// decision planner. For admitted candidates, the reliability interval is
/// preserved in the output for audit, but is not converted into a weighted
/// utility or multiplied by conditional information/PL reduction.
pub fn plan_active_evidence_with_reliability_and_decision_relevance(
    policy: &ActiveEvidencePolicy,
    candidates: &[EvidenceActionCandidate],
    decision_reports: &[DecisionRelevantDiscriminationReport],
    reliability_policy: &ActiveEvidenceReliabilityPolicy,
    reliability_contracts: &[ValidAcquisitionProbabilityContract],
) -> Result<ReliabilityGatedDecisionPlan, ReliabilityGatedDecisionError> {
    let mut candidate_ids = BTreeSet::new();
    for candidate in candidates {
        if candidate.action_id == 0 || candidate.dependency_group_id == 0 {
            return Err(ReliabilityGatedDecisionError::InvalidCandidateIdentity);
        }
        if !candidate_ids.insert(candidate.action_id) {
            return Err(ReliabilityGatedDecisionError::DuplicateCandidateId(
                candidate.action_id,
            ));
        }
        if !candidate
            .conditional_post_action_protection_level_upper_m
            .is_finite()
            || candidate.conditional_post_action_protection_level_upper_m < 0.0
        {
            return Err(
                ReliabilityGatedDecisionError::InvalidCandidateProtectionLevel(
                    candidate.action_id,
                ),
            );
        }
    }

    let candidate_action_ids: Vec<u64> = candidates
        .iter()
        .map(|candidate| candidate.action_id)
        .collect();
    let mut observed_report_ids = BTreeSet::new();
    for report in decision_reports {
        if !candidate_ids.contains(&report.action_id) {
            return Err(ReliabilityGatedDecisionError::UnexpectedDecisionReport(
                report.action_id,
            ));
        }
        if !observed_report_ids.insert(report.action_id) {
            return Err(ReliabilityGatedDecisionError::DuplicateDecisionReport(
                report.action_id,
            ));
        }
    }
    let reliability = admit_active_evidence_reliability(
        reliability_policy,
        &candidate_action_ids,
        reliability_contracts,
    )
    .map_err(ReliabilityGatedDecisionError::ReliabilityAdmission)?;

    let admitted: BTreeMap<u64, AdmittedAcquisitionReliability> = reliability
        .admitted_actions
        .iter()
        .map(|record| (record.action_id, *record))
        .collect();
    let admitted_acquisition_reliability = reliability.admitted_actions.clone();
    let rejected_acquisition_reliability = reliability.rejected_actions.clone();

    let already_satisfied = match plan_active_evidence_with_decision_relevance(policy, &[], &[]) {
        Ok(plan) => Some(plan),
        Err(ActiveEvidenceDecisionError::Acquisition(
            crate::active_evidence_acquisition::ActiveEvidenceAcquisitionError::NoFeasibleAction,
        )) => None,
        Err(error) => return Err(ReliabilityGatedDecisionError::DecisionPlanner(error)),
    };
    if let Some(plan) = already_satisfied {
        return Ok(ReliabilityGatedDecisionPlan {
            disposition: ReliabilityGatedDecisionDisposition::PlanProduced,
            decision_plan: Some(plan),
            admitted_acquisition_reliability,
            rejected_acquisition_reliability,
            resource_excluded_action_ids: Vec::new(),
        });
    }

    let admitted_candidates: Vec<_> = candidates
        .iter()
        .filter(|candidate| admitted.contains_key(&candidate.action_id))
        .copied()
        .collect();

    if admitted_candidates.is_empty() {
        return Ok(ReliabilityGatedDecisionPlan {
            disposition: ReliabilityGatedDecisionDisposition::NoReliabilityAdmittedActions,
            decision_plan: None,
            admitted_acquisition_reliability,
            rejected_acquisition_reliability,
            resource_excluded_action_ids: Vec::new(),
        });
    }

    // Mirror the decision planner's hard feasibility constraints so that only
    // reports for actions which can actually be considered are forwarded.
    let known_groups: BTreeSet<u64> =
        policy.already_observed_dependency_groups.iter().copied().collect();
    let resource_feasible_ids: BTreeSet<u64> = admitted_candidates
        .iter()
        .filter(|candidate| {
            candidate.available
                && candidate.freshness_and_calibration_preconditions_met
                && candidate.worst_case_latency_ns <= policy.maximum_action_latency_ns
                && candidate.worst_case_energy_uj <= policy.maximum_action_energy_uj
                && (!policy.require_new_dependency_group
                    || !known_groups.contains(&candidate.dependency_group_id))
        })
        .map(|candidate| candidate.action_id)
        .collect();

    let resource_excluded_action_ids: Vec<u64> = admitted
        .keys()
        .filter(|action_id| !resource_feasible_ids.contains(action_id))
        .copied()
        .collect();

    if resource_feasible_ids.is_empty() {
        // Ask the underlying policy to validate its scalar/domain parameters;
        // NoFeasibleAction is the expected result for the empty eligible set.
        match plan_active_evidence_with_decision_relevance(policy, &[], &[]) {
            Err(ActiveEvidenceDecisionError::Acquisition(
                crate::active_evidence_acquisition::ActiveEvidenceAcquisitionError::NoFeasibleAction,
            )) => {}
            Ok(plan) if plan.reason == crate::active_evidence_decision_planner::ActiveEvidenceDecisionReason::TargetAlreadySatisfied => {
                return Ok(ReliabilityGatedDecisionPlan {
                    disposition: ReliabilityGatedDecisionDisposition::PlanProduced,
                    decision_plan: Some(plan),
                    admitted_acquisition_reliability,
                    rejected_acquisition_reliability,
                    resource_excluded_action_ids,
                });
            }
            Err(error) => return Err(ReliabilityGatedDecisionError::DecisionPlanner(error)),
            Ok(_) => {}
        }
        return Ok(ReliabilityGatedDecisionPlan {
            disposition: ReliabilityGatedDecisionDisposition::NoResourceFeasibleActions,
            decision_plan: None,
            admitted_acquisition_reliability,
            rejected_acquisition_reliability,
            resource_excluded_action_ids,
        });
    }

    let filtered_candidates: Vec<_> = admitted_candidates
        .into_iter()
        .filter(|candidate| resource_feasible_ids.contains(&candidate.action_id))
        .collect();
    let filtered_reports: Vec<_> = decision_reports
        .iter()
        .filter(|report| resource_feasible_ids.contains(&report.action_id))
        .cloned()
        .collect();

    let decision_plan = match plan_active_evidence_with_decision_relevance(
        policy,
        &filtered_candidates,
        &filtered_reports,
    ) {
        Ok(plan) => Some(plan),
        Err(ActiveEvidenceDecisionError::Acquisition(
            crate::active_evidence_acquisition::ActiveEvidenceAcquisitionError::NoFeasibleAction,
        )) => None,
        Err(error) => return Err(ReliabilityGatedDecisionError::DecisionPlanner(error)),
    };

    let disposition = if decision_plan.is_some() {
        ReliabilityGatedDecisionDisposition::PlanProduced
    } else {
        ReliabilityGatedDecisionDisposition::NoActionImprovesProtectionOrDecisionDiscrimination
    };

    Ok(ReliabilityGatedDecisionPlan {
        disposition,
        decision_plan,
        admitted_acquisition_reliability,
        rejected_acquisition_reliability,
        resource_excluded_action_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_acquisition::EvidenceActionKind;
    use crate::active_evidence_decision_planner::ActiveEvidenceDecisionReason;
    use crate::active_evidence_reliability::{
        AcquisitionCampaignBlock, AcquisitionCampaignOutcome, ReliabilityRejectionReason,
    };
    use crate::decision_relevant_discrimination::{
        HypothesisDecisionAssignment, project_decision_relevant_discrimination,
    };
    use crate::uwb_hypothesis_discrimination::{
        UwbHypothesisDiscriminationReport, UwbHypothesisPairSeparation,
    };

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

    fn reliability_policy(minimum_probability_lower_bound: f64) -> ActiveEvidenceReliabilityPolicy {
        ActiveEvidenceReliabilityPolicy {
            expected_domain_fingerprint_sha256: [0x31; 32],
            expected_acquisition_protocol_sha256: [0x32; 32],
            minimum_probability_lower_bound,
            maximum_confidence_error_probability: 0.05,
            minimum_independent_campaign_blocks: 4,
        }
    }

    fn candidate(
        id: u64,
        pl: f64,
        latency: u64,
        energy: u64,
        available: bool,
    ) -> EvidenceActionCandidate {
        EvidenceActionCandidate {
            action_id: id,
            kind: EvidenceActionKind::RequestIndependentAnchor,
            dependency_group_id: 20 + id,
            conditional_post_action_protection_level_upper_m: pl,
            worst_case_latency_ns: latency,
            worst_case_energy_uj: energy,
            available,
            freshness_and_calibration_preconditions_met: true,
        }
    }

    fn reliability_contract(
        action_id: u64,
        valid_successes: usize,
        probability_lower_bound: f64,
    ) -> ValidAcquisitionProbabilityContract {
        let campaign_blocks = (0..4)
            .map(|index| AcquisitionCampaignBlock {
                block_id: index as u64 + 1,
                independence_group_id: index as u64 + 101,
                outcome: if (index as usize) < valid_successes {
                    AcquisitionCampaignOutcome::ValidQualifiedSuccess
                } else {
                    AcquisitionCampaignOutcome::FailedOrTimedOut
                },
            })
            .collect();
        ValidAcquisitionProbabilityContract {
            action_id,
            domain_fingerprint_sha256: [0x31; 32],
            acquisition_protocol_sha256: [0x32; 32],
            source_manifest_sha256: [0x21; 32],
            calibration_cohort_sha256: [0x22; 32],
            bound_method_sha256: [0x23; 32],
            confidence_error_probability: 0.05,
            probability_lower_bound,
            probability_upper_bound: 1.0,
            campaign_blocks,
        }
    }

    fn decision_report(
        action_id: u64,
        gaps: &[f64],
    ) -> DecisionRelevantDiscriminationReport {
        let pairs = [(1, 2), (1, 3), (2, 3)];
        let pair_separations = pairs
            .into_iter()
            .zip(gaps.iter().copied())
            .map(|((lower_hypothesis_id, upper_hypothesis_id), normalized_outcome_gap)| {
                UwbHypothesisPairSeparation {
                    lower_hypothesis_id,
                    upper_hypothesis_id,
                    normalized_outcome_gap,
                    guaranteed_distinguishable: normalized_outcome_gap > 0.0,
                }
            })
            .collect::<Vec<_>>();
        let distinguishable = pair_separations
            .iter()
            .filter(|pair| pair.guaranteed_distinguishable)
            .count();
        let upstream = UwbHypothesisDiscriminationReport {
            action_id,
            hypothesis_inventory_sha256: [0x11; 32],
            outcome_model_sha256: [action_id as u8; 32],
            normalization_sha256: [0x33; 32],
            hypothesis_count: 3,
            hypothesis_pair_count: 3,
            distinguishable_pair_count: distinguishable,
            ambiguous_pair_count: 3 - distinguishable,
            minimum_positive_gap: pair_separations
                .iter()
                .filter(|pair| pair.guaranteed_distinguishable)
                .map(|pair| pair.normalized_outcome_gap)
                .reduce(f64::min),
            maximum_pair_gap: pair_separations
                .iter()
                .map(|pair| pair.normalized_outcome_gap)
                .fold(0.0, f64::max),
            conditional_on_successful_valid_observation: true,
            pair_separations,
        };
        project_decision_relevant_discrimination(
            &upstream,
            action_id,
            &[1, 2, 3],
            [0x11; 32],
            &[
                HypothesisDecisionAssignment {
                    hypothesis_id: 1,
                    decision_class_id: 10,
                },
                HypothesisDecisionAssignment {
                    hypothesis_id: 2,
                    decision_class_id: 10,
                },
                HypothesisDecisionAssignment {
                    hypothesis_id: 3,
                    decision_class_id: 20,
                },
            ],
            [0x44; 32],
        )
        .unwrap()
    }

    #[test]
    fn reliability_is_an_eligibility_gate_not_a_utility_multiplier() {
        let candidates = [
            candidate(1, 1.2, 20, 100, true),
            candidate(2, 2.5, 20, 100, true),
        ];
        let contracts = [
            // Empirical success is 0.75, but the lower bound is below policy.
            reliability_contract(1, 3, 0.60),
            // Action 2 satisfies the domain, protocol, cohort and lower-bound policy.
            reliability_contract(2, 4, 0.70),
        ];
        let reports = [
            decision_report(1, &[1.0, 0.0, 0.0]),
            decision_report(2, &[0.0, 2.0, 2.0]),
        ];
        let result = plan_active_evidence_with_reliability_and_decision_relevance(
            &policy(),
            &candidates,
            &reports,
            &reliability_policy(0.70),
            &contracts,
        )
        .unwrap();

        assert_eq!(
            result.disposition,
            ReliabilityGatedDecisionDisposition::PlanProduced
        );
        assert_eq!(
            result.decision_plan.as_ref().unwrap().recommended_action_id,
            Some(2)
        );
        assert_eq!(result.rejected_acquisition_reliability.len(), 1);
        assert_eq!(result.rejected_acquisition_reliability[0].action_id, 1);
        assert_eq!(
            result.rejected_acquisition_reliability[0].reason,
            ReliabilityRejectionReason::ProbabilityLowerBoundTooSmall
        );
        assert_eq!(
            result.admitted_acquisition_reliability[0].probability_lower_bound,
            0.70
        );
        assert_eq!(
            result.decision_plan.as_ref().unwrap().reason,
            ActiveEvidenceDecisionReason::BestDecisionDiscriminationAvailableButTargetNotReached
        );
    }

    #[test]
    fn admitted_probability_bounds_do_not_override_decision_value() {
        let candidates = [
            candidate(1, 2.0, 20, 100, true),
            candidate(2, 2.5, 20, 100, true),
        ];
        let contracts = [
            reliability_contract(1, 4, 0.90),
            reliability_contract(2, 4, 0.70),
        ];
        let reports = [
            decision_report(1, &[1.0, 0.0, 0.0]),
            decision_report(2, &[0.0, 2.0, 2.0]),
        ];
        let result = plan_active_evidence_with_reliability_and_decision_relevance(
            &policy(),
            &candidates,
            &reports,
            &reliability_policy(0.60),
            &contracts,
        )
        .unwrap();

        assert_eq!(
            result.decision_plan.as_ref().unwrap().recommended_action_id,
            Some(2)
        );
        assert_eq!(result.admitted_acquisition_reliability.len(), 2);
        assert_eq!(
            result.admitted_acquisition_reliability[0].probability_lower_bound,
            0.90
        );
        assert_eq!(
            result.admitted_acquisition_reliability[1].probability_lower_bound,
            0.70
        );
    }

    #[test]
    fn missing_and_weak_reliability_contracts_remain_explicit_rejections() {
        let candidates = [
            candidate(1, 1.2, 20, 100, true),
            candidate(2, 2.5, 20, 100, true),
        ];
        let contracts = [reliability_contract(1, 3, 0.60)];
        let result = plan_active_evidence_with_reliability_and_decision_relevance(
            &policy(),
            &candidates,
            &[],
            &reliability_policy(0.70),
            &contracts,
        )
        .unwrap();

        assert_eq!(
            result.disposition,
            ReliabilityGatedDecisionDisposition::NoReliabilityAdmittedActions
        );
        assert_eq!(result.decision_plan, None);
        assert_eq!(result.rejected_acquisition_reliability.len(), 2);
        assert_eq!(
            result.rejected_acquisition_reliability[0].reason,
            ReliabilityRejectionReason::ProbabilityLowerBoundTooSmall
        );
        assert_eq!(
            result.rejected_acquisition_reliability[1].reason,
            ReliabilityRejectionReason::MissingContract
        );
    }

    #[test]
    fn duplicate_and_malformed_reliability_contracts_fail_closed() {
        let candidates = [candidate(1, 2.0, 20, 100, true)];
        let duplicate = [
            reliability_contract(1, 4, 0.70),
            reliability_contract(1, 4, 0.70),
        ];
        assert_eq!(
            plan_active_evidence_with_reliability_and_decision_relevance(
                &policy(),
                &candidates,
                &[],
                &reliability_policy(0.70),
                &duplicate,
            ),
            Err(ReliabilityGatedDecisionError::ReliabilityAdmission(
                ActiveEvidenceReliabilityError::DuplicateReliabilityContract(1)
            ))
        );

        let malformed = [reliability_contract(1, 3, 0.90)];
        assert_eq!(
            plan_active_evidence_with_reliability_and_decision_relevance(
                &policy(),
                &candidates,
                &[],
                &reliability_policy(0.70),
                &malformed,
            ),
            Err(ReliabilityGatedDecisionError::ReliabilityAdmission(
                ActiveEvidenceReliabilityError::InvalidProbabilityInterval(1)
            ))
        );
    }

    #[test]
    fn resource_infeasible_admitted_action_is_recorded_but_not_planned() {
        let candidates = [
            candidate(1, 2.0, 20, 100, true),
            candidate(2, 2.0, 20, 100, false),
        ];
        let contracts = [
            reliability_contract(1, 4, 0.70),
            reliability_contract(2, 4, 0.70),
        ];
        let reports = [decision_report(1, &[0.0, 2.0, 2.0])];
        let result = plan_active_evidence_with_reliability_and_decision_relevance(
            &policy(),
            &candidates,
            &reports,
            &reliability_policy(0.70),
            &contracts,
        )
        .unwrap();

        assert_eq!(
            result.decision_plan.unwrap().recommended_action_id,
            Some(1)
        );
        assert_eq!(result.resource_excluded_action_ids, vec![2]);
        assert_eq!(result.admitted_acquisition_reliability.len(), 2);
    }

    #[test]
    fn unknown_and_duplicate_decision_reports_reject_even_if_action_would_be_gated_out() {
        let candidates = [candidate(1, 2.0, 20, 100, true)];
        let contracts = [reliability_contract(1, 4, 0.70)];
        let admitted_policy = reliability_policy(0.70);
        assert_eq!(
            plan_active_evidence_with_reliability_and_decision_relevance(
                &policy(),
                &candidates,
                &[decision_report(99, &[0.0, 2.0, 2.0])],
                &admitted_policy,
                &contracts,
            ),
            Err(ReliabilityGatedDecisionError::UnexpectedDecisionReport(99))
        );
        let report = decision_report(1, &[0.0, 2.0, 2.0]);
        assert_eq!(
            plan_active_evidence_with_reliability_and_decision_relevance(
                &policy(),
                &candidates,
                &[report.clone(), report],
                &admitted_policy,
                &contracts,
            ),
            Err(ReliabilityGatedDecisionError::DuplicateDecisionReport(1))
        );
    }

    #[test]
    fn target_already_satisfied_does_not_require_sensing_or_reliability() {
        let mut satisfied = policy();
        satisfied.current_protection_level_upper_m = 0.5;
        let result = plan_active_evidence_with_reliability_and_decision_relevance(
            &satisfied,
            &[],
            &[],
            &reliability_policy(0.70),
            &[],
        )
        .unwrap();
        assert_eq!(
            result.disposition,
            ReliabilityGatedDecisionDisposition::PlanProduced
        );
        assert_eq!(
            result.decision_plan.as_ref().unwrap().reason,
            ActiveEvidenceDecisionReason::TargetAlreadySatisfied
        );
    }
}
