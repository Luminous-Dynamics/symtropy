//! Pareto composition of active evidence acquisition and fault-hypothesis
//! discrimination.
//!
//! This wrapper preserves distinct objectives instead of converting metres,
//! latency, energy and hypothesis discrimination into one arbitrary score.
//! It composes the existing active-evidence planner with conditional
//! observation-outcome separation reports.
//!
//! A discrimination report is conditional on a successful, valid acquisition.
//! It cannot itself remove hypotheses, update a protection level, or authorize
//! a navigation authority transition.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::active_evidence_acquisition::{
    AcquisitionPlanReason, ActiveEvidenceAcquisitionError, ActiveEvidencePolicy,
    EvidenceActionAssessment, EvidenceActionCandidate, plan_active_evidence_acquisition,
};
use crate::uwb_hypothesis_discrimination::UwbHypothesisDiscriminationReport;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvidenceActionDiscriminationAssessment {
    pub action: EvidenceActionAssessment,
    pub distinguishable_hypothesis_pair_count: usize,
    pub ambiguous_hypothesis_pair_count: usize,
    /// Diagnostic only. Outcome-space normalization may differ between action
    /// modalities, so this value is not compared across actions.
    pub minimum_positive_normalized_gap: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveEvidenceDiscriminationPlan {
    pub reason: AcquisitionPlanReason,
    pub recommended_action_id: Option<u64>,
    pub pareto_frontier: Vec<EvidenceActionDiscriminationAssessment>,
    pub conditional_target_reached: bool,
    /// Always true: this combined planner never authorizes a state transition.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveEvidenceDiscriminationError {
    Acquisition(ActiveEvidenceAcquisitionError),
    MissingDiscriminationReport(u64),
    UnexpectedDiscriminationReport(u64),
    DuplicateDiscriminationReport(u64),
    InvalidDiscriminationReport(u64),
    InventoryMismatch,
    HypothesisPairCountOverflow,
}

impl fmt::Display for ActiveEvidenceDiscriminationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquisition(error) => write!(f, "active acquisition validation failed: {error}"),
            Self::MissingDiscriminationReport(id) => {
                write!(f, "feasible action {id} has no hypothesis-discrimination report")
            }
            Self::UnexpectedDiscriminationReport(id) => {
                write!(f, "discrimination report has no matching feasible action: {id}")
            }
            Self::DuplicateDiscriminationReport(id) => {
                write!(f, "duplicate hypothesis-discrimination report for action {id}")
            }
            Self::InvalidDiscriminationReport(id) => {
                write!(f, "hypothesis-discrimination report is incomplete or invalid for action {id}")
            }
            Self::InventoryMismatch => {
                write!(f, "candidate actions are not bound to the same hypothesis inventory")
            }
            Self::HypothesisPairCountOverflow => {
                write!(f, "hypothesis-pair count overflowed")
            }
        }
    }
}

impl std::error::Error for ActiveEvidenceDiscriminationError {}

/// Plan using four separate Pareto objectives:
/// - greater conditional protection-level reduction;
/// - lower worst-case latency;
/// - lower worst-case energy;
/// - greater count of guaranteed distinguishable hypothesis pairs.
///
/// Reports are required only for actions that satisfy the existing planner's
/// availability, freshness, new-dependency and resource constraints. A
/// candidate whose outcome model is incomplete cannot be considered feasible
/// for this discrimination-aware policy.
pub fn plan_active_evidence_with_hypothesis_discrimination(
    policy: &ActiveEvidencePolicy,
    candidates: &[EvidenceActionCandidate],
    reports: &[UwbHypothesisDiscriminationReport],
) -> Result<ActiveEvidenceDiscriminationPlan, ActiveEvidenceDiscriminationError> {
    let baseline = plan_active_evidence_acquisition(policy, candidates)
        .map_err(ActiveEvidenceDiscriminationError::Acquisition)?;

    if baseline.reason == AcquisitionPlanReason::TargetAlreadySatisfied {
        return Ok(ActiveEvidenceDiscriminationPlan {
            reason: AcquisitionPlanReason::TargetAlreadySatisfied,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            conditional_target_reached: true,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let known_groups: BTreeSet<u64> =
        policy.already_observed_dependency_groups.iter().copied().collect();

    let feasible: Vec<&EvidenceActionCandidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.available
                && candidate.freshness_and_calibration_preconditions_met
                && candidate.worst_case_latency_ns <= policy.maximum_action_latency_ns
                && candidate.worst_case_energy_uj <= policy.maximum_action_energy_uj
                && (!policy.require_new_dependency_group
                    || !known_groups.contains(&candidate.dependency_group_id))
                && candidate.conditional_post_action_protection_level_upper_m
                    < policy.current_protection_level_upper_m
        })
        .collect();

    let feasible_ids: BTreeSet<u64> = feasible.iter().map(|candidate| candidate.action_id).collect();
    let mut reports_by_action = BTreeMap::new();
    let mut inventory_commitment = None;
    let mut expected_hypothesis_count = None;
    let mut expected_pair_count = None;

    for report in reports {
        if !feasible_ids.contains(&report.action_id) {
            return Err(ActiveEvidenceDiscriminationError::UnexpectedDiscriminationReport(
                report.action_id,
            ));
        }
        if reports_by_action.insert(report.action_id, report).is_some() {
            return Err(ActiveEvidenceDiscriminationError::DuplicateDiscriminationReport(
                report.action_id,
            ));
        }
        if !report.conditional_on_successful_valid_observation {
            return Err(ActiveEvidenceDiscriminationError::InvalidDiscriminationReport(
                report.action_id,
            ));
        }
        if report.hypothesis_count == 0
            || report.hypothesis_inventory_sha256.iter().all(|byte| *byte == 0)
            || report.outcome_model_sha256.iter().all(|byte| *byte == 0)
            || report.normalization_sha256.iter().all(|byte| *byte == 0)
        {
            return Err(ActiveEvidenceDiscriminationError::InvalidDiscriminationReport(
                report.action_id,
            ));
        }
        let pair_count = report
            .hypothesis_count
            .checked_mul(report.hypothesis_count.saturating_sub(1))
            .and_then(|value| value.checked_div(2))
            .ok_or(ActiveEvidenceDiscriminationError::HypothesisPairCountOverflow)?;

        let mut pair_ids = BTreeSet::new();
        let mut observed_hypothesis_ids = BTreeSet::new();
        let mut distinguishable_count = 0_usize;
        let mut minimum_positive_gap = None;
        let mut maximum_pair_gap = 0.0_f64;
        let mut all_pair_values_valid = true;

        for pair in &report.pair_separations {
            let valid = pair.lower_hypothesis_id != 0
                && pair.upper_hypothesis_id > pair.lower_hypothesis_id
                && pair.normalized_outcome_gap.is_finite()
                && pair.normalized_outcome_gap >= 0.0
                && pair.guaranteed_distinguishable == (pair.normalized_outcome_gap > 0.0)
                && pair_ids.insert((pair.lower_hypothesis_id, pair.upper_hypothesis_id));
            if !valid {
                all_pair_values_valid = false;
                break;
            }
            observed_hypothesis_ids.insert(pair.lower_hypothesis_id);
            observed_hypothesis_ids.insert(pair.upper_hypothesis_id);
            maximum_pair_gap = maximum_pair_gap.max(pair.normalized_outcome_gap);
            if pair.guaranteed_distinguishable {
                distinguishable_count += 1;
                minimum_positive_gap = Some(
                    minimum_positive_gap.map_or(pair.normalized_outcome_gap, |current: f64| {
                        current.min(pair.normalized_outcome_gap)
                    }),
                );
            }
        }

        if report.hypothesis_pair_count != pair_count
            || report.pair_separations.len() != pair_count
            || pair_ids.len() != pair_count
            || (report.hypothesis_count > 1
                && observed_hypothesis_ids.len() != report.hypothesis_count)
            || report.distinguishable_pair_count != distinguishable_count
            || report.ambiguous_pair_count != pair_count - distinguishable_count
            || !all_pair_values_valid
            || report.maximum_pair_gap != maximum_pair_gap
            || report.minimum_positive_gap != minimum_positive_gap
        {
            return Err(ActiveEvidenceDiscriminationError::InvalidDiscriminationReport(
                report.action_id,
            ));
        }
        match (inventory_commitment, expected_hypothesis_count, expected_pair_count) {
            (None, None, None) => {
                inventory_commitment = Some(report.hypothesis_inventory_sha256);
                expected_hypothesis_count = Some(report.hypothesis_count);
                expected_pair_count = Some(report.hypothesis_pair_count);
            }
            (Some(digest), Some(count), Some(pairs))
                if digest == report.hypothesis_inventory_sha256
                    && count == report.hypothesis_count
                    && pairs == report.hypothesis_pair_count => {}
            _ => return Err(ActiveEvidenceDiscriminationError::InventoryMismatch),
        }
    }

    let mut assessments = Vec::with_capacity(feasible.len());
    for candidate in &feasible {
        let report = reports_by_action
            .get(&candidate.action_id)
            .ok_or(ActiveEvidenceDiscriminationError::MissingDiscriminationReport(
                candidate.action_id,
            ))?;

        let reduction = policy.current_protection_level_upper_m
            - candidate.conditional_post_action_protection_level_upper_m;
        if !reduction.is_finite() || reduction <= 0.0 {
            return Err(ActiveEvidenceDiscriminationError::Acquisition(
                ActiveEvidenceAcquisitionError::ArithmeticOverflow,
            ));
        }

        assessments.push(EvidenceActionDiscriminationAssessment {
            action: EvidenceActionAssessment {
                action_id: candidate.action_id,
                kind: candidate.kind,
                dependency_group_id: candidate.dependency_group_id,
                conditional_post_action_protection_level_upper_m: candidate
                    .conditional_post_action_protection_level_upper_m,
                guaranteed_protection_level_reduction_m: reduction,
                worst_case_latency_ns: candidate.worst_case_latency_ns,
                worst_case_energy_uj: candidate.worst_case_energy_uj,
                reaches_target_if_acquisition_qualifies: candidate
                    .conditional_post_action_protection_level_upper_m
                    <= policy.target_protection_level_upper_m,
            },
            distinguishable_hypothesis_pair_count: report.distinguishable_pair_count,
            ambiguous_hypothesis_pair_count: report.ambiguous_pair_count,
            minimum_positive_normalized_gap: report.minimum_positive_gap,
        });
    }

    let mut pareto_frontier = Vec::new();
    for candidate in &assessments {
        let dominated = assessments.iter().any(|other| {
            other.action.action_id != candidate.action.action_id
                && dominates_discrimination_assessment(other, candidate)
        });
        if !dominated {
            pareto_frontier.push(*candidate);
        }
    }

    pareto_frontier.sort_by(|left, right| {
        right
            .distinguishable_hypothesis_pair_count
            .cmp(&left.distinguishable_hypothesis_pair_count)
            .then_with(|| {
                right
                    .action
                    .guaranteed_protection_level_reduction_m
                    .total_cmp(&left.action.guaranteed_protection_level_reduction_m)
            })
            .then_with(|| {
                left.action
                    .worst_case_latency_ns
                    .cmp(&right.action.worst_case_latency_ns)
            })
            .then_with(|| {
                left.action
                    .worst_case_energy_uj
                    .cmp(&right.action.worst_case_energy_uj)
            })
            .then_with(|| left.action.action_id.cmp(&right.action.action_id))
    });

    let target_reaching: Vec<_> = pareto_frontier
        .iter()
        .filter(|candidate| candidate.action.reaches_target_if_acquisition_qualifies)
        .collect();

    let (reason, selected) = if !target_reaching.is_empty() {
        let selected = *target_reaching
            .into_iter()
            .min_by(|left, right| {
                right
                    .distinguishable_hypothesis_pair_count
                    .cmp(&left.distinguishable_hypothesis_pair_count)
                    .then_with(|| {
                        left.action
                            .worst_case_latency_ns
                            .cmp(&right.action.worst_case_latency_ns)
                    })
                    .then_with(|| {
                        left.action
                            .worst_case_energy_uj
                            .cmp(&right.action.worst_case_energy_uj)
                    })
                    .then_with(|| left.action.action_id.cmp(&right.action.action_id))
            })
            .ok_or(ActiveEvidenceDiscriminationError::InvalidDiscriminationReport(0))?;
        (
            AcquisitionPlanReason::LowestLatencyTargetReachingAction,
            selected,
        )
    } else {
        let selected = *pareto_frontier
            .iter()
            .max_by(|left, right| {
                left.distinguishable_hypothesis_pair_count
                    .cmp(&right.distinguishable_hypothesis_pair_count)
                    .then_with(|| {
                        left.action
                            .guaranteed_protection_level_reduction_m
                            .total_cmp(&right.action.guaranteed_protection_level_reduction_m)
                    })
                    .then_with(|| {
                        right
                            .action
                            .worst_case_latency_ns
                            .cmp(&left.action.worst_case_latency_ns)
                    })
                    .then_with(|| {
                        right
                            .action
                            .worst_case_energy_uj
                            .cmp(&left.action.worst_case_energy_uj)
                    })
                    .then_with(|| right.action.action_id.cmp(&left.action.action_id))
            })
            .ok_or(ActiveEvidenceDiscriminationError::InvalidDiscriminationReport(0))?;
        (
            AcquisitionPlanReason::BestReductionAvailableButTargetNotReached,
            selected,
        )
    };

    Ok(ActiveEvidenceDiscriminationPlan {
        reason,
        recommended_action_id: Some(selected.action.action_id),
        pareto_frontier,
        conditional_target_reached: selected.action.reaches_target_if_acquisition_qualifies,
        authority_transition_not_authorized_by_plan: true,
    })
}

fn dominates_discrimination_assessment(
    left: &EvidenceActionDiscriminationAssessment,
    right: &EvidenceActionDiscriminationAssessment,
) -> bool {
    let no_worse = left
        .distinguishable_hypothesis_pair_count
        >= right.distinguishable_hypothesis_pair_count
        && left
            .action
            .guaranteed_protection_level_reduction_m
            >= right
                .action
                .guaranteed_protection_level_reduction_m
        && left.action.worst_case_latency_ns <= right.action.worst_case_latency_ns
        && left.action.worst_case_energy_uj <= right.action.worst_case_energy_uj;
    let strictly_better = left
        .distinguishable_hypothesis_pair_count
        > right.distinguishable_hypothesis_pair_count
        || left
            .action
            .guaranteed_protection_level_reduction_m
            > right
                .action
                .guaranteed_protection_level_reduction_m
        || left.action.worst_case_latency_ns < right.action.worst_case_latency_ns
        || left.action.worst_case_energy_uj < right.action.worst_case_energy_uj;
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_acquisition::EvidenceActionKind;
    use crate::uwb_hypothesis_discrimination::UwbHypothesisPairSeparation;

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

    fn report(
        action_id: u64,
        distinguishable_pairs: &[(u64, u64, f64)],
    ) -> UwbHypothesisDiscriminationReport {
        let pair_separations: Vec<_> = distinguishable_pairs
            .iter()
            .map(|(left, right, gap)| UwbHypothesisPairSeparation {
                lower_hypothesis_id: *left,
                upper_hypothesis_id: *right,
                normalized_outcome_gap: *gap,
                guaranteed_distinguishable: *gap > 0.0,
            })
            .collect();
        let distinguishable_count = pair_separations
            .iter()
            .filter(|pair| pair.guaranteed_distinguishable)
            .count();
        let minimum_positive_gap = pair_separations
            .iter()
            .filter(|pair| pair.guaranteed_distinguishable)
            .map(|pair| pair.normalized_outcome_gap)
            .reduce(f64::min);
        UwbHypothesisDiscriminationReport {
            action_id,
            hypothesis_inventory_sha256: [0x11; 32],
            outcome_model_sha256: [action_id as u8; 32],
            normalization_sha256: [0x33; 32],
            hypothesis_count: 3,
            hypothesis_pair_count: 3,
            distinguishable_pair_count: distinguishable_count,
            ambiguous_pair_count: 3 - distinguishable_count,
            minimum_positive_gap,
            maximum_pair_gap: pair_separations
                .iter()
                .map(|pair| pair.normalized_outcome_gap)
                .fold(0.0, f64::max),
            conditional_on_successful_valid_observation: true,
            pair_separations,
        }
    }

    fn full_report(action_id: u64, distinguishable_count: usize) -> UwbHypothesisDiscriminationReport {
        match distinguishable_count {
            0 => report(action_id, &[(1, 2, 0.0), (1, 3, 0.0), (2, 3, 0.0)]),
            1 => report(action_id, &[(1, 2, 1.0), (1, 3, 0.0), (2, 3, 0.0)]),
            2 => report(action_id, &[(1, 2, 1.0), (1, 3, 0.0), (2, 3, 2.0)]),
            _ => report(action_id, &[(1, 2, 1.0), (1, 3, 2.0), (2, 3, 3.0)]),
        }
    }

    #[test]
    fn prefers_hypothesis_discrimination_when_pl_and_cost_are_equal() {
        let candidates = [candidate(1, 0.8, 40, 200), candidate(2, 0.8, 40, 200)];
        let reports = [full_report(1, 0), full_report(2, 3)];
        let plan =
            plan_active_evidence_with_hypothesis_discrimination(&policy(), &candidates, &reports)
                .unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        assert_eq!(plan.pareto_frontier.len(), 1);
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn preserves_tradeoffs_between_pl_reduction_and_hypothesis_discrimination() {
        let candidates = [candidate(1, 0.5, 50, 300), candidate(2, 0.8, 40, 200)];
        let reports = [full_report(1, 0), full_report(2, 3)];
        let plan =
            plan_active_evidence_with_hypothesis_discrimination(&policy(), &candidates, &reports)
                .unwrap();
        assert_eq!(plan.pareto_frontier.len(), 2);
        assert_eq!(plan.recommended_action_id, Some(2));
    }

    #[test]
    fn target_reaching_action_is_selected_by_discrimination_before_cost() {
        let candidates = [candidate(1, 0.8, 90, 100), candidate(2, 0.8, 40, 200)];
        let reports = [full_report(1, 3), full_report(2, 0)];
        let plan =
            plan_active_evidence_with_hypothesis_discrimination(&policy(), &candidates, &reports)
                .unwrap();
        assert_eq!(plan.recommended_action_id, Some(1));
    }

    #[test]
    fn report_must_cover_every_feasible_action() {
        let candidates = [candidate(1, 0.8, 40, 200), candidate(2, 0.8, 40, 200)];
        assert_eq!(
            plan_active_evidence_with_hypothesis_discrimination(
                &policy(),
                &candidates,
                &[full_report(1, 2)]
            ),
            Err(ActiveEvidenceDiscriminationError::MissingDiscriminationReport(2))
        );
    }

    #[test]
    fn mismatched_hypothesis_inventory_is_rejected() {
        let candidates = [candidate(1, 0.8, 40, 200), candidate(2, 0.8, 40, 200)];
        let mut mismatched = full_report(2, 2);
        mismatched.hypothesis_inventory_sha256 = [0x44; 32];
        assert_eq!(
            plan_active_evidence_with_hypothesis_discrimination(
                &policy(),
                &candidates,
                &[full_report(1, 2), mismatched]
            ),
            Err(ActiveEvidenceDiscriminationError::InventoryMismatch)
        );
    }

    #[test]
    fn target_already_satisfied_needs_no_measurement_model() {
        let mut satisfied = policy();
        satisfied.current_protection_level_upper_m = 0.5;
        let plan =
            plan_active_evidence_with_hypothesis_discrimination(&satisfied, &[], &[]).unwrap();
        assert_eq!(plan.recommended_action_id, None);
        assert!(plan.conditional_target_reached);
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn no_discriminating_action_keeps_the_best_pl_reduction() {
        let candidates = [candidate(1, 2.0, 80, 300), candidate(2, 1.5, 90, 400)];
        let reports = [full_report(1, 0), full_report(2, 0)];
        let plan =
            plan_active_evidence_with_hypothesis_discrimination(&policy(), &candidates, &reports)
                .unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        assert!(!plan.conditional_target_reached);
        assert_eq!(
            plan.reason,
            AcquisitionPlanReason::BestReductionAvailableButTargetNotReached
        );
    }

    #[test]
    fn rejects_duplicate_or_invalid_discrimination_reports() {
        let candidates = [candidate(1, 0.8, 40, 200)];
        assert_eq!(
            plan_active_evidence_with_hypothesis_discrimination(
                &policy(),
                &candidates,
                &[full_report(1, 2), full_report(1, 2)]
            ),
            Err(ActiveEvidenceDiscriminationError::DuplicateDiscriminationReport(1))
        );

        let mut invalid = full_report(1, 2);
        invalid.pair_separations[0].normalized_outcome_gap = f64::NAN;
        assert_eq!(
            plan_active_evidence_with_hypothesis_discrimination(
                &policy(),
                &candidates,
                &[invalid]
            ),
            Err(ActiveEvidenceDiscriminationError::InvalidDiscriminationReport(1))
        );
    }
}
