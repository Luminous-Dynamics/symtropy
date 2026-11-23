//! Active evidence planning using decision-relevant fault discrimination.
//!
//! This wrapper consumes the existing decision-equivalence projection and
//! ranks actions only by separations between hypotheses that imply different
//! navigation decisions. Same-decision nuisance ambiguity remains visible in
//! the report but is not rewarded as decision progress.
//!
//! All outcome-set separation is conditional on successful, valid acquisition.
//! This planner does not execute an action, remove a hypothesis, update a
//! protection level, or authorize a navigation transition.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::active_evidence_acquisition::{
    AcquisitionPlanReason, ActiveEvidenceAcquisitionError, ActiveEvidencePolicy,
    EvidenceActionAssessment, EvidenceActionCandidate, plan_active_evidence_acquisition,
};
use crate::decision_relevant_discrimination::{
    DecisionProjectionDisposition, DecisionRelevantDiscriminationReport,
    HypothesisDecisionAssignment,
};

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceActionDecisionAssessment {
    pub action: EvidenceActionAssessment,
    /// Positive values narrow the caller-supplied conditional protection bound.
    /// Negative values mean it widens after successful acquisition and audit.
    pub conditional_protection_level_delta_m: f64,
    pub decision_class_count: usize,
    pub same_decision_hypothesis_pair_count: usize,
    pub decision_incompatible_hypothesis_pair_count: usize,
    pub distinguishable_decision_pair_count: usize,
    pub ambiguous_decision_pair_count: usize,
    pub disposition: DecisionProjectionDisposition,
    /// Diagnostic only; not comparable across different feature normalizations.
    pub minimum_positive_normalized_gap: Option<f64>,
    /// Exact action-specific model and feature-semantics commitments.
    pub outcome_model_sha256: [u8; 32],
    pub normalization_sha256: [u8; 32],
    /// Exact cross-decision pairs separated and left overlapping by this action.
    pub separated_decision_hypothesis_pairs: Vec<(u64, u64)>,
    pub unresolved_decision_hypothesis_pairs: Vec<(u64, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveEvidenceDecisionReason {
    TargetAlreadySatisfied,
    LowestLatencyDecisionResolvingAction,
    LowestLatencyTargetReachingAction,
    BestDecisionDiscriminationAvailableButTargetNotReached,
    BestProtectionReductionAvailableButTargetNotReached,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveEvidenceDecisionPlan {
    pub reason: ActiveEvidenceDecisionReason,
    pub recommended_action_id: Option<u64>,
    pub pareto_frontier: Vec<EvidenceActionDecisionAssessment>,
    pub conditional_target_reached: bool,
    pub hypothesis_inventory_sha256: Option<[u8; 32]>,
    pub decision_partition_sha256: Option<[u8; 32]>,
    /// Frozen hypothesis-to-decision mapping shared by every action report.
    pub decision_assignments: Option<Vec<HypothesisDecisionAssignment>>,
    /// Always true: this plan cannot authorize an operating-mode transition.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveEvidenceDecisionError {
    Acquisition(ActiveEvidenceAcquisitionError),
    MissingDecisionReport(u64),
    UnexpectedDecisionReport(u64),
    DuplicateDecisionReport(u64),
    InvalidDecisionReport(u64),
    InventoryMismatch,
    DecisionPartitionMismatch,
    HypothesisPairCountOverflow,
}

impl fmt::Display for ActiveEvidenceDecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquisition(error) => write!(f, "active acquisition validation failed: {error}"),
            Self::MissingDecisionReport(id) => {
                write!(f, "resource-feasible action {id} has no decision report")
            }
            Self::UnexpectedDecisionReport(id) => {
                write!(f, "decision report has no matching resource-feasible action: {id}")
            }
            Self::DuplicateDecisionReport(id) => {
                write!(f, "decision report is duplicated for action {id}")
            }
            Self::InvalidDecisionReport(id) => {
                write!(f, "decision report is incomplete or inconsistent for action {id}")
            }
            Self::InventoryMismatch => {
                write!(f, "candidate reports use different hypothesis inventories")
            }
            Self::DecisionPartitionMismatch => {
                write!(f, "candidate reports use different frozen decision partitions")
            }
            Self::HypothesisPairCountOverflow => {
                write!(f, "hypothesis or decision-class pair count overflowed")
            }
        }
    }
}

impl std::error::Error for ActiveEvidenceDecisionError {}

#[derive(Clone, Debug, PartialEq)]
struct ValidatedDecisionReport {
    inventory_sha256: [u8; 32],
    partition_sha256: [u8; 32],
    outcome_model_sha256: [u8; 32],
    normalization_sha256: [u8; 32],
    assignments: BTreeMap<u64, u64>,
    separated_pairs: Vec<(u64, u64)>,
    unresolved_pairs: Vec<(u64, u64)>,
    hypothesis_count: usize,
    decision_class_count: usize,
    same_decision_pair_count: usize,
    decision_incompatible_pair_count: usize,
    distinguishable_pair_count: usize,
    ambiguous_pair_count: usize,
    disposition: DecisionProjectionDisposition,
    minimum_positive_gap: Option<f64>,
}

fn checked_pair_count(count: usize) -> Result<usize, ActiveEvidenceDecisionError> {
    count
        .checked_mul(count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(ActiveEvidenceDecisionError::HypothesisPairCountOverflow)
}

/// Revalidate the public report before using it for action selection.
///
/// Reconstructs the expected cross-class pair set from the complete frozen
/// assignments, then checks every pair, class label, gap, summary and
/// disposition. Reports are not trusted merely because a projector normally
/// produces them.
fn validate_decision_report(
    report: &DecisionRelevantDiscriminationReport,
) -> Result<ValidatedDecisionReport, ActiveEvidenceDecisionError> {
    let invalid = || ActiveEvidenceDecisionError::InvalidDecisionReport(report.action_id);

    if report.action_id == 0
        || report.hypothesis_count == 0
        || report.hypothesis_inventory_sha256.iter().all(|byte| *byte == 0)
        || report.outcome_model_sha256.iter().all(|byte| *byte == 0)
        || report.normalization_sha256.iter().all(|byte| *byte == 0)
        || report.decision_partition_sha256.iter().all(|byte| *byte == 0)
        || !report.conditional_on_successful_valid_observation
        || report.decision_assignments.len() != report.hypothesis_count
    {
        return Err(invalid());
    }

    let total_hypothesis_pairs = checked_pair_count(report.hypothesis_count)?;
    let mut assignments = BTreeMap::new();
    let mut class_sizes = BTreeMap::<u64, usize>::new();

    for assignment in &report.decision_assignments {
        if assignment.hypothesis_id == 0 || assignment.decision_class_id == 0 {
            return Err(invalid());
        }
        if assignments
            .insert(assignment.hypothesis_id, assignment.decision_class_id)
            .is_some()
        {
            return Err(invalid());
        }
        let size = class_sizes.entry(assignment.decision_class_id).or_default();
        *size = size.checked_add(1).ok_or_else(invalid)?;
    }

    if assignments.len() != report.hypothesis_count {
        return Err(invalid());
    }

    let mut expected_same_decision_pairs = 0_usize;
    for size in class_sizes.values() {
        expected_same_decision_pairs = expected_same_decision_pairs
            .checked_add(checked_pair_count(*size)?)
            .ok_or_else(invalid)?;
    }

    let expected_cross_decision_pairs = total_hypothesis_pairs
        .checked_sub(expected_same_decision_pairs)
        .ok_or_else(invalid)?;
    let expected_class_pairs = checked_pair_count(class_sizes.len())?;

    if report.decision_class_count != class_sizes.len()
        || report.decision_class_pair_count != expected_class_pairs
        || report.same_decision_hypothesis_pair_count != expected_same_decision_pairs
        || report.decision_incompatible_hypothesis_pair_count != expected_cross_decision_pairs
        || report.decision_pair_separations.len() != expected_cross_decision_pairs
    {
        return Err(invalid());
    }

    let ids: Vec<(u64, u64)> = assignments
        .iter()
        .map(|(hypothesis_id, class_id)| (*hypothesis_id, *class_id))
        .collect();
    let mut expected_pairs = BTreeSet::new();
    for left_index in 0..ids.len() {
        for right_index in (left_index + 1)..ids.len() {
            if ids[left_index].1 != ids[right_index].1 {
                expected_pairs.insert((ids[left_index].0, ids[right_index].0));
            }
        }
    }

    let mut observed_pairs = BTreeSet::new();
    let mut distinguishable = 0_usize;
    let mut ambiguous = 0_usize;
    let mut minimum_positive_gap: Option<f64> = None;
    let mut separated_pairs = Vec::new();
    let mut unresolved_pairs = Vec::new();

    for pair in &report.decision_pair_separations {
        if pair.lower_hypothesis_id == 0
            || pair.upper_hypothesis_id <= pair.lower_hypothesis_id
            || !pair.normalized_outcome_gap.is_finite()
            || pair.normalized_outcome_gap < 0.0
            || pair.guaranteed_distinguishable != (pair.normalized_outcome_gap > 0.0)
        {
            return Err(invalid());
        }

        let lower_class = assignments
            .get(&pair.lower_hypothesis_id)
            .copied()
            .ok_or_else(invalid)?;
        let upper_class = assignments
            .get(&pair.upper_hypothesis_id)
            .copied()
            .ok_or_else(invalid)?;

        if lower_class == upper_class
            || lower_class != pair.lower_decision_class_id
            || upper_class != pair.upper_decision_class_id
            || !expected_pairs.contains(&(pair.lower_hypothesis_id, pair.upper_hypothesis_id))
            || !observed_pairs.insert((pair.lower_hypothesis_id, pair.upper_hypothesis_id))
        {
            return Err(invalid());
        }

        let pair_id = (pair.lower_hypothesis_id, pair.upper_hypothesis_id);
        if pair.guaranteed_distinguishable {
            distinguishable = distinguishable.checked_add(1).ok_or_else(invalid)?;
            minimum_positive_gap = Some(
                minimum_positive_gap.map_or(pair.normalized_outcome_gap, |current| {
                    current.min(pair.normalized_outcome_gap)
                }),
            );
            separated_pairs.push(pair_id);
        } else {
            ambiguous = ambiguous.checked_add(1).ok_or_else(invalid)?;
            unresolved_pairs.push(pair_id);
        }
    }

    if observed_pairs != expected_pairs
        || report.distinguishable_decision_pair_count != distinguishable
        || report.ambiguous_decision_pair_count != ambiguous
        || distinguishable.checked_add(ambiguous) != Some(expected_cross_decision_pairs)
    {
        return Err(invalid());
    }

    let expected_disposition = if expected_cross_decision_pairs == 0 {
        DecisionProjectionDisposition::NoDecisionIncompatiblePairs
    } else if ambiguous > 0 {
        DecisionProjectionDisposition::DecisionAmbiguityRemains
    } else {
        DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
    };

    if report.disposition != expected_disposition {
        return Err(invalid());
    }

    Ok(ValidatedDecisionReport {
        inventory_sha256: report.hypothesis_inventory_sha256,
        partition_sha256: report.decision_partition_sha256,
        outcome_model_sha256: report.outcome_model_sha256,
        normalization_sha256: report.normalization_sha256,
        assignments,
        separated_pairs,
        unresolved_pairs,
        hypothesis_count: report.hypothesis_count,
        decision_class_count: report.decision_class_count,
        same_decision_pair_count: report.same_decision_hypothesis_pair_count,
        decision_incompatible_pair_count: report.decision_incompatible_hypothesis_pair_count,
        distinguishable_pair_count: distinguishable,
        ambiguous_pair_count: ambiguous,
        disposition: expected_disposition,
        minimum_positive_gap,
    })
}

/// Plan using separate Pareto dimensions:
/// - decision-incompatible pairs guaranteed distinguishable;
/// - signed conditional protection-level reduction;
/// - worst-case latency;
/// - worst-case energy.
///
/// A same-decision nuisance pair cannot improve the discrimination objective.
/// A no-cross-decision-pair report is not treated as successful sensing.
/// All projections remain conditional on a later successful, valid acquisition.
pub fn plan_active_evidence_with_decision_relevance(
    policy: &ActiveEvidencePolicy,
    candidates: &[EvidenceActionCandidate],
    reports: &[DecisionRelevantDiscriminationReport],
) -> Result<ActiveEvidenceDecisionPlan, ActiveEvidenceDecisionError> {
    let baseline = match plan_active_evidence_acquisition(policy, candidates) {
        Ok(plan) => Some(plan),
        Err(ActiveEvidenceAcquisitionError::NoFeasibleAction) => None,
        Err(error) => return Err(ActiveEvidenceDecisionError::Acquisition(error)),
    };

    if baseline
        .as_ref()
        .is_some_and(|plan| plan.reason == AcquisitionPlanReason::TargetAlreadySatisfied)
    {
        return Ok(ActiveEvidenceDecisionPlan {
            reason: ActiveEvidenceDecisionReason::TargetAlreadySatisfied,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            conditional_target_reached: true,
            hypothesis_inventory_sha256: None,
            decision_partition_sha256: None,
            decision_assignments: None,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let known_groups: BTreeSet<u64> =
        policy.already_observed_dependency_groups.iter().copied().collect();

    // Match the original planner's availability, freshness, dependency and
    // hard-resource constraints, but allow a useful discriminator even when
    // the conservative PL only shrinks after the acquired result is audited.
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
        return Err(ActiveEvidenceDecisionError::Acquisition(
            ActiveEvidenceAcquisitionError::NoFeasibleAction,
        ));
    }

    let feasible_ids: BTreeSet<u64> = resource_feasible
        .iter()
        .map(|candidate| candidate.action_id)
        .collect();
    let mut reports_by_action = BTreeMap::new();
    let mut common_inventory: Option<([u8; 32], usize)> = None;
    let mut common_partition: Option<([u8; 32], BTreeMap<u64, u64>)> = None;

    for report in reports {
        if !feasible_ids.contains(&report.action_id) {
            return Err(ActiveEvidenceDecisionError::UnexpectedDecisionReport(report.action_id));
        }
        if reports_by_action.contains_key(&report.action_id) {
            return Err(ActiveEvidenceDecisionError::DuplicateDecisionReport(report.action_id));
        }

        let validated = validate_decision_report(report)?;
        match &common_inventory {
            None => {
                common_inventory = Some((
                    validated.inventory_sha256,
                    validated.hypothesis_count,
                ));
            }
            Some((digest, count))
                if *digest == validated.inventory_sha256
                    && *count == validated.hypothesis_count => {}
            Some(_) => return Err(ActiveEvidenceDecisionError::InventoryMismatch),
        }
        match &common_partition {
            None => {
                common_partition = Some((
                    validated.partition_sha256,
                    validated.assignments.clone(),
                ));
            }
            Some((digest, assignments))
                if *digest == validated.partition_sha256
                    && *assignments == validated.assignments => {}
            Some(_) => return Err(ActiveEvidenceDecisionError::DecisionPartitionMismatch),
        }
        reports_by_action.insert(report.action_id, validated);
    }

    let (inventory_digest, _) = common_inventory
        .ok_or(ActiveEvidenceDecisionError::MissingDecisionReport(
            resource_feasible[0].action_id,
        ))?;
    let (partition_digest, partition_assignment_map) = common_partition
        .ok_or(ActiveEvidenceDecisionError::MissingDecisionReport(
            resource_feasible[0].action_id,
        ))?;
    let decision_assignments: Vec<HypothesisDecisionAssignment> = partition_assignment_map
        .into_iter()
        .map(|(hypothesis_id, decision_class_id)| HypothesisDecisionAssignment {
            hypothesis_id,
            decision_class_id,
        })
        .collect();

    let mut assessments = Vec::with_capacity(resource_feasible.len());
    for candidate in &resource_feasible {
        let validated = reports_by_action
            .get(&candidate.action_id)
            .ok_or(ActiveEvidenceDecisionError::MissingDecisionReport(
                candidate.action_id,
            ))?;

        let signed_delta = policy.current_protection_level_upper_m
            - candidate.conditional_post_action_protection_level_upper_m;
        if !signed_delta.is_finite() {
            return Err(ActiveEvidenceDecisionError::Acquisition(
                ActiveEvidenceAcquisitionError::ArithmeticOverflow,
            ));
        }

        // A no-improvement action is retained only if it separates at least
        // one pair that implies a different navigation decision.
        if signed_delta <= 0.0 && validated.distinguishable_pair_count == 0 {
            continue;
        }

        assessments.push(EvidenceActionDecisionAssessment {
            action: EvidenceActionAssessment {
                action_id: candidate.action_id,
                kind: candidate.kind,
                dependency_group_id: candidate.dependency_group_id,
                conditional_post_action_protection_level_upper_m: candidate
                    .conditional_post_action_protection_level_upper_m,
                guaranteed_protection_level_reduction_m: signed_delta.max(0.0),
                worst_case_latency_ns: candidate.worst_case_latency_ns,
                worst_case_energy_uj: candidate.worst_case_energy_uj,
                reaches_target_if_acquisition_qualifies: candidate
                    .conditional_post_action_protection_level_upper_m
                    <= policy.target_protection_level_upper_m,
            },
            conditional_protection_level_delta_m: signed_delta,
            decision_class_count: validated.decision_class_count,
            same_decision_hypothesis_pair_count: validated.same_decision_pair_count,
            decision_incompatible_hypothesis_pair_count: validated.decision_incompatible_pair_count,
            distinguishable_decision_pair_count: validated.distinguishable_pair_count,
            ambiguous_decision_pair_count: validated.ambiguous_pair_count,
            disposition: validated.disposition,
            minimum_positive_normalized_gap: validated.minimum_positive_gap,
            outcome_model_sha256: validated.outcome_model_sha256,
            normalization_sha256: validated.normalization_sha256,
            separated_decision_hypothesis_pairs: validated.separated_pairs.clone(),
            unresolved_decision_hypothesis_pairs: validated.unresolved_pairs.clone(),
        });
    }

    if assessments.is_empty() {
        return Err(ActiveEvidenceDecisionError::Acquisition(
            ActiveEvidenceAcquisitionError::NoFeasibleAction,
        ));
    }

    let mut pareto_frontier = Vec::new();
    for candidate in &assessments {
        let dominated = assessments.iter().any(|other| {
            other.action.action_id != candidate.action.action_id
                && dominates_decision_assessment(other, candidate)
        });
        if !dominated {
            pareto_frontier.push(candidate.clone());
        }
    }

    pareto_frontier.sort_by(|left, right| {
        right
            .distinguishable_decision_pair_count
            .cmp(&left.distinguishable_decision_pair_count)
            .then_with(|| {
                right
                    .conditional_protection_level_delta_m
                    .total_cmp(&left.conditional_protection_level_delta_m)
            })
            .then_with(|| left.action.worst_case_latency_ns.cmp(&right.action.worst_case_latency_ns))
            .then_with(|| left.action.worst_case_energy_uj.cmp(&right.action.worst_case_energy_uj))
            .then_with(|| left.action.action_id.cmp(&right.action.action_id))
    });

    let target_reaching: Vec<_> = pareto_frontier
        .iter()
        .filter(|candidate| candidate.action.reaches_target_if_acquisition_qualifies)
        .collect();

    let (reason, selected) = if !target_reaching.is_empty() {
        let selected = target_reaching
            .into_iter()
            .min_by(|left, right| {
                right
                    .distinguishable_decision_pair_count
                    .cmp(&left.distinguishable_decision_pair_count)
                    .then_with(|| left.action.worst_case_latency_ns.cmp(&right.action.worst_case_latency_ns))
                    .then_with(|| left.action.worst_case_energy_uj.cmp(&right.action.worst_case_energy_uj))
                    .then_with(|| left.action.action_id.cmp(&right.action.action_id))
            })
            .cloned()
            .ok_or(ActiveEvidenceDecisionError::InvalidDecisionReport(0))?;
        let reason = if selected.distinguishable_decision_pair_count > 0 {
            ActiveEvidenceDecisionReason::LowestLatencyDecisionResolvingAction
        } else {
            ActiveEvidenceDecisionReason::LowestLatencyTargetReachingAction
        };
        (reason, selected)
    } else if assessments
        .iter()
        .any(|candidate| candidate.distinguishable_decision_pair_count > 0)
    {
        let selected = pareto_frontier
            .iter()
            .max_by(|left, right| {
                left.distinguishable_decision_pair_count
                    .cmp(&right.distinguishable_decision_pair_count)
                    .then_with(|| left.conditional_protection_level_delta_m.total_cmp(&right.conditional_protection_level_delta_m))
                    .then_with(|| right.action.worst_case_latency_ns.cmp(&left.action.worst_case_latency_ns))
                    .then_with(|| right.action.worst_case_energy_uj.cmp(&left.action.worst_case_energy_uj))
                    .then_with(|| right.action.action_id.cmp(&left.action.action_id))
            })
            .cloned()
            .ok_or(ActiveEvidenceDecisionError::InvalidDecisionReport(0))?;
        (ActiveEvidenceDecisionReason::BestDecisionDiscriminationAvailableButTargetNotReached, selected)
    } else {
        let selected = pareto_frontier
            .iter()
            .max_by(|left, right| {
                left.conditional_protection_level_delta_m
                    .total_cmp(&right.conditional_protection_level_delta_m)
                    .then_with(|| right.action.worst_case_latency_ns.cmp(&left.action.worst_case_latency_ns))
                    .then_with(|| right.action.worst_case_energy_uj.cmp(&left.action.worst_case_energy_uj))
                    .then_with(|| right.action.action_id.cmp(&left.action.action_id))
            })
            .cloned()
            .ok_or(ActiveEvidenceDecisionError::InvalidDecisionReport(0))?;
        (ActiveEvidenceDecisionReason::BestProtectionReductionAvailableButTargetNotReached, selected)
    };

    Ok(ActiveEvidenceDecisionPlan {
        reason,
        recommended_action_id: Some(selected.action.action_id),
        pareto_frontier,
        conditional_target_reached: selected.action.reaches_target_if_acquisition_qualifies,
        hypothesis_inventory_sha256: Some(inventory_digest),
        decision_partition_sha256: Some(partition_digest),
        decision_assignments: Some(decision_assignments),
        authority_transition_not_authorized_by_plan: true,
    })
}

fn dominates_decision_assessment(
    left: &EvidenceActionDecisionAssessment,
    right: &EvidenceActionDecisionAssessment,
) -> bool {
    let no_worse = left.distinguishable_decision_pair_count
        >= right.distinguishable_decision_pair_count
        && left.conditional_protection_level_delta_m >= right.conditional_protection_level_delta_m
        && left.action.worst_case_latency_ns <= right.action.worst_case_latency_ns
        && left.action.worst_case_energy_uj <= right.action.worst_case_energy_uj;
    let strictly_better = left.distinguishable_decision_pair_count
        > right.distinguishable_decision_pair_count
        || left.conditional_protection_level_delta_m > right.conditional_protection_level_delta_m
        || left.action.worst_case_latency_ns < right.action.worst_case_latency_ns
        || left.action.worst_case_energy_uj < right.action.worst_case_energy_uj;
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_acquisition::EvidenceActionKind;
    use crate::decision_relevant_discrimination::{
        project_decision_relevant_discrimination, HypothesisDecisionAssignment,
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

    fn upstream_report(action_id: u64, gaps: &[f64]) -> UwbHypothesisDiscriminationReport {
        assert_eq!(gaps.len(), 3);
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
        UwbHypothesisDiscriminationReport {
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
        }
    }

    fn projected_report(
        action_id: u64,
        gaps: &[f64],
        classes: &[(u64, u64)],
        partition_digest: [u8; 32],
    ) -> DecisionRelevantDiscriminationReport {
        let assignments: Vec<_> = classes
            .iter()
            .map(|(hypothesis_id, decision_class_id)| HypothesisDecisionAssignment {
                hypothesis_id: *hypothesis_id,
                decision_class_id: *decision_class_id,
            })
            .collect();
        project_decision_relevant_discrimination(
            &upstream_report(action_id, gaps),
            action_id,
            &[1, 2, 3],
            [0x11; 32],
            &assignments,
            partition_digest,
        )
        .unwrap()
    }

    fn default_classes() -> [(u64, u64); 3] {
        [(1, 10), (2, 10), (3, 20)]
    }

    #[test]
    fn prefers_cross_decision_separation_over_same_decision_parameter_separation() {
        let candidates = [candidate(1, 2.0, 20, 100), candidate(2, 2.5, 20, 100)];
        let classes = default_classes();
        let reports = [
            projected_report(1, &[1.0, 0.0, 0.0], &classes, [0x44; 32]),
            projected_report(2, &[0.0, 2.0, 2.0], &classes, [0x44; 32]),
        ];

        assert_eq!(reports[0].distinguishable_decision_pair_count, 0);
        assert_eq!(reports[1].same_decision_hypothesis_pair_count, 1);
        assert_eq!(reports[1].distinguishable_decision_pair_count, 2);

        let plan = plan_active_evidence_with_decision_relevance(&policy(), &candidates, &reports).unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        assert_eq!(
            plan.reason,
            ActiveEvidenceDecisionReason::BestDecisionDiscriminationAvailableButTargetNotReached
        );
        assert!(plan.authority_transition_not_authorized_by_plan);
        assert_eq!(
            plan.pareto_frontier[0].disposition,
            DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
        );
        assert_eq!(plan.pareto_frontier[0].outcome_model_sha256, [2; 32]);
        assert_eq!(plan.pareto_frontier[0].normalization_sha256, [0x33; 32]);
        assert_eq!(
            plan.pareto_frontier[0].separated_decision_hypothesis_pairs,
            vec![(1, 3), (2, 3)]
        );
        assert!(plan.pareto_frontier[0].unresolved_decision_hypothesis_pairs.is_empty());
        assert_eq!(
            plan.decision_assignments,
            Some(vec![
                HypothesisDecisionAssignment { hypothesis_id: 1, decision_class_id: 10 },
                HypothesisDecisionAssignment { hypothesis_id: 2, decision_class_id: 10 },
                HypothesisDecisionAssignment { hypothesis_id: 3, decision_class_id: 20 },
            ])
        );
    }

    #[test]
    fn no_cross_decision_pair_and_no_pl_reduction_is_not_successful_sensing() {
        let candidates = [candidate(1, 3.0, 20, 100)];
        let classes = [(1, 7), (2, 7), (3, 7)];
        let report = projected_report(1, &[1.0, 2.0, 3.0], &classes, [0x44; 32]);
        assert_eq!(
            report.disposition,
            DecisionProjectionDisposition::NoDecisionIncompatiblePairs
        );
        assert_eq!(
            plan_active_evidence_with_decision_relevance(&policy(), &candidates, &[report]),
            Err(ActiveEvidenceDecisionError::Acquisition(
                ActiveEvidenceAcquisitionError::NoFeasibleAction
            ))
        );
    }

    #[test]
    fn malformed_or_incomplete_decision_pair_report_rejects() {
        let candidates = [candidate(1, 2.0, 20, 100)];
        let classes = default_classes();
        let mut report = projected_report(1, &[0.0, 2.0, 2.0], &classes, [0x44; 32]);
        report.decision_pair_separations.pop();
        assert_eq!(
            plan_active_evidence_with_decision_relevance(&policy(), &candidates, &[report]),
            Err(ActiveEvidenceDecisionError::InvalidDecisionReport(1))
        );
    }

    #[test]
    fn mismatched_inventory_or_decision_partition_is_rejected() {
        let candidates = [candidate(1, 2.0, 20, 100), candidate(2, 2.0, 30, 200)];
        let classes = default_classes();
        let first = projected_report(1, &[0.0, 2.0, 2.0], &classes, [0x44; 32]);
        let mut different_inventory =
            projected_report(2, &[0.0, 2.0, 2.0], &classes, [0x44; 32]);
        different_inventory.hypothesis_inventory_sha256 = [0x55; 32];
        assert_eq!(
            plan_active_evidence_with_decision_relevance(
                &policy(),
                &candidates,
                &[first.clone(), different_inventory]
            ),
            Err(ActiveEvidenceDecisionError::InventoryMismatch)
        );

        let different_partition =
            projected_report(2, &[0.0, 2.0, 2.0], &[(1, 10), (2, 20), (3, 20)], [0x66; 32]);
        assert_eq!(
            plan_active_evidence_with_decision_relevance(
                &policy(),
                &candidates,
                &[first, different_partition]
            ),
            Err(ActiveEvidenceDecisionError::DecisionPartitionMismatch)
        );
    }

    #[test]
    fn every_resource_feasible_action_needs_exactly_one_report() {
        let candidates = [candidate(1, 2.0, 20, 100), candidate(2, 2.0, 30, 200)];
        let classes = default_classes();
        let first = projected_report(1, &[0.0, 2.0, 2.0], &classes, [0x44; 32]);
        assert_eq!(
            plan_active_evidence_with_decision_relevance(&policy(), &candidates, &[first.clone()]),
            Err(ActiveEvidenceDecisionError::MissingDecisionReport(2))
        );
        assert_eq!(
            plan_active_evidence_with_decision_relevance(
                &policy(),
                &candidates,
                &[first.clone(), first]
            ),
            Err(ActiveEvidenceDecisionError::DuplicateDecisionReport(1))
        );
        assert_eq!(
            plan_active_evidence_with_decision_relevance(
                &policy(),
                &candidates,
                &[projected_report(99, &[0.0, 2.0, 2.0], &classes, [0x44; 32])]
            ),
            Err(ActiveEvidenceDecisionError::UnexpectedDecisionReport(99))
        );
    }

    #[test]
    fn target_reaching_action_prefers_more_cross_class_separation() {
        let candidates = [candidate(1, 0.8, 80, 100), candidate(2, 0.8, 40, 200)];
        let classes = default_classes();
        let reports = [
            projected_report(1, &[0.0, 0.0, 2.0], &classes, [0x44; 32]),
            projected_report(2, &[0.0, 2.0, 2.0], &classes, [0x44; 32]),
        ];
        let plan = plan_active_evidence_with_decision_relevance(&policy(), &candidates, &reports).unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        assert_eq!(
            plan.reason,
            ActiveEvidenceDecisionReason::LowestLatencyDecisionResolvingAction
        );
    }

    #[test]
    fn target_reduction_without_cross_decision_separation_is_not_called_decision_resolving() {
        let candidates = [candidate(1, 0.8, 40, 200)];
        let classes = default_classes();
        let report = projected_report(1, &[1.0, 0.0, 0.0], &classes, [0x44; 32]);
        assert_eq!(report.distinguishable_decision_pair_count, 0);
        let plan = plan_active_evidence_with_decision_relevance(
            &policy(),
            &candidates,
            &[report],
        )
        .unwrap();
        assert_eq!(plan.recommended_action_id, Some(1));
        assert_eq!(
            plan.reason,
            ActiveEvidenceDecisionReason::LowestLatencyTargetReachingAction
        );
        assert!(plan.conditional_target_reached);
        assert!(plan.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn protection_reduction_fallback_remains_distinct_from_decision_discrimination() {
        let candidates = [candidate(1, 2.0, 80, 300), candidate(2, 1.5, 90, 400)];
        let classes = default_classes();
        let reports = [
            projected_report(1, &[0.0, 0.0, 0.0], &classes, [0x44; 32]),
            projected_report(2, &[0.0, 0.0, 0.0], &classes, [0x44; 32]),
        ];
        let plan = plan_active_evidence_with_decision_relevance(
            &policy(),
            &candidates,
            &reports,
        )
        .unwrap();
        assert_eq!(plan.recommended_action_id, Some(2));
        assert_eq!(
            plan.reason,
            ActiveEvidenceDecisionReason::BestProtectionReductionAvailableButTargetNotReached
        );
        assert!(!plan.conditional_target_reached);
    }

    #[test]
    fn decision_discrimination_can_justify_an_action_without_pl_reduction() {
        let candidates = [candidate(1, 3.0, 40, 200)];
        let reports = [projected_report(
            1,
            &[0.0, 2.0, 2.0],
            &default_classes(),
            [0x44; 32],
        )];
        let plan = plan_active_evidence_with_decision_relevance(&policy(), &candidates, &reports).unwrap();
        assert_eq!(plan.recommended_action_id, Some(1));
        assert_eq!(
            plan.reason,
            ActiveEvidenceDecisionReason::BestDecisionDiscriminationAvailableButTargetNotReached
        );
        assert!(!plan.conditional_target_reached);
        assert!(plan.authority_transition_not_authorized_by_plan);
        assert_eq!(plan.pareto_frontier[0].outcome_model_sha256, [1; 32]);
        assert_eq!(
            plan.pareto_frontier[0].separated_decision_hypothesis_pairs,
            vec![(1, 3), (2, 3)]
        );
        assert!(plan.pareto_frontier[0].unresolved_decision_hypothesis_pairs.is_empty());
    }
}
