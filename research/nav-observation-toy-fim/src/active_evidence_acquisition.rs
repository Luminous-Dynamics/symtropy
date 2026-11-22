//! Resource-constrained Pareto selection of the next navigation evidence action.
//!
//! Candidate post-action protection levels are conservative bounds conditional
//! on a successful, independently qualified acquisition. This scheduler only
//! recommends an action; it does not treat a requested measurement as already
//! observed, does not reduce the current protection level, and never grants
//! navigation authority. The controller must receive and qualify a fresh
//! evidence receipt, re-audit the posterior, and re-evaluate the mission
//! contract after the action completes.

use std::collections::HashSet;
use std::fmt;

/// Action class proposed by an independently qualified sensing/control model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceActionKind {
    ResynchronizeRadio,
    AcquireCir,
    RequestIndependentAnchor,
    RequestPassiveObserver,
    ChangeViewpoint,
    RequestVioRelocalization,
}

/// Conservative conditional outcome and maximum resource cost of one action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvidenceActionCandidate {
    pub action_id: u64,
    pub kind: EvidenceActionKind,
    /// Declared dependency/failure-domain group of the evidence this action
    /// would add. It must be backed by source and environment provenance.
    pub dependency_group_id: u64,
    /// Upper bound on the protection level only if the requested observation
    /// is acquired and independently qualified.
    pub conditional_post_action_protection_level_upper_m: f64,
    pub worst_case_latency_ns: u64,
    /// Conservative upper bound on energy, in microjoules.
    pub worst_case_energy_uj: u64,
    pub available: bool,
    pub freshness_and_calibration_preconditions_met: bool,
}

/// Pinned resource and mission requirements for one scheduling decision.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveEvidencePolicy {
    pub current_protection_level_upper_m: f64,
    pub target_protection_level_upper_m: f64,
    pub maximum_action_latency_ns: u64,
    pub maximum_action_energy_uj: u64,
    pub require_new_dependency_group: bool,
    pub already_observed_dependency_groups: Vec<u64>,
}

/// Candidate action after applying the current mission policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvidenceActionAssessment {
    pub action_id: u64,
    pub kind: EvidenceActionKind,
    pub dependency_group_id: u64,
    pub conditional_post_action_protection_level_upper_m: f64,
    pub guaranteed_protection_level_reduction_m: f64,
    pub worst_case_latency_ns: u64,
    pub worst_case_energy_uj: u64,
    pub reaches_target_if_acquisition_qualifies: bool,
}

/// Why the scheduler returned this recommendation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionPlanReason {
    TargetAlreadySatisfied,
    LowestLatencyTargetReachingAction,
    BestReductionAvailableButTargetNotReached,
}

/// Pareto frontier and a deterministic recommendation. This plan is not
/// evidence and cannot restore NORMAL navigation authority.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveEvidenceAcquisitionPlan {
    pub reason: AcquisitionPlanReason,
    pub recommended_action_id: Option<u64>,
    pub pareto_frontier: Vec<EvidenceActionAssessment>,
    pub conditional_target_reached: bool,
    /// Always true: this planner never authorizes an authority transition.
    /// The controller must separately qualify the acquired evidence and
    /// re-evaluate the posterior and mission contract.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveEvidenceAcquisitionError {
    InvalidProtectionLevel,
    InvalidDependencyGroup,
    DuplicateActionId(u64),
    InvalidActionIdentity,
    InvalidConditionalProtectionLevel(u64),
    NoFeasibleAction,
    ArithmeticOverflow,
}

impl fmt::Display for ActiveEvidenceAcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProtectionLevel => {
                write!(f, "current and target protection levels must be finite and nonnegative")
            }
            Self::InvalidDependencyGroup => {
                write!(f, "dependency group identities must be nonzero")
            }
            Self::DuplicateActionId(id) => write!(f, "duplicate evidence action ID: {id}"),
            Self::InvalidActionIdentity => {
                write!(f, "evidence action IDs and dependency groups must be nonzero")
            }
            Self::InvalidConditionalProtectionLevel(id) => {
                write!(f, "conditional post-action protection level is invalid: {id}")
            }
            Self::NoFeasibleAction => {
                write!(f, "no action satisfies freshness, resource and improvement constraints")
            }
            Self::ArithmeticOverflow => {
                write!(f, "protection-level reduction arithmetic overflowed")
            }
        }
    }
}

impl std::error::Error for ActiveEvidenceAcquisitionError {}

/// Select a feasible evidence action without inventing a scalar conversion
/// between metres, energy and time.
///
/// Selection:
/// 1. Reject unavailable/stale/uncalibrated actions and actions exceeding the
///    predeclared resource budget.
/// 2. When required, reject actions whose evidence comes from an already used
///    dependency group.
/// 3. Compute the Pareto frontier over protection-level reduction, latency and
///    energy. A dominated action is removed without arbitrary weights.
/// 4. If any frontier action would reach the target *conditional on successful
///    qualification*, select the lowest-latency target-reaching action, then
///    lowest energy, then action ID.
/// 5. Otherwise select maximum conservative reduction, then latency, energy
///    and action ID. The plan explicitly says the target remains unmet.
pub fn plan_active_evidence_acquisition(
    policy: &ActiveEvidencePolicy,
    candidates: &[EvidenceActionCandidate],
) -> Result<ActiveEvidenceAcquisitionPlan, ActiveEvidenceAcquisitionError> {
    validate_policy(policy)?;

    if policy.current_protection_level_upper_m <= policy.target_protection_level_upper_m {
        return Ok(ActiveEvidenceAcquisitionPlan {
            reason: AcquisitionPlanReason::TargetAlreadySatisfied,
            recommended_action_id: None,
            pareto_frontier: Vec::new(),
            conditional_target_reached: true,
            authority_transition_not_authorized_by_plan: true,
        });
    }

    let known_groups: HashSet<u64> =
        policy.already_observed_dependency_groups.iter().copied().collect();
    if known_groups.contains(&0) {
        return Err(ActiveEvidenceAcquisitionError::InvalidDependencyGroup);
    }

    let mut action_ids = HashSet::with_capacity(candidates.len());
    let mut feasible = Vec::new();

    for candidate in candidates {
        if candidate.action_id == 0 || candidate.dependency_group_id == 0 {
            return Err(ActiveEvidenceAcquisitionError::InvalidActionIdentity);
        }
        if !action_ids.insert(candidate.action_id) {
            return Err(ActiveEvidenceAcquisitionError::DuplicateActionId(
                candidate.action_id,
            ));
        }
        if !candidate
            .conditional_post_action_protection_level_upper_m
            .is_finite()
            || candidate.conditional_post_action_protection_level_upper_m < 0.0
        {
            return Err(
                ActiveEvidenceAcquisitionError::InvalidConditionalProtectionLevel(
                    candidate.action_id,
                ),
            );
        }
        if !candidate.available
            || !candidate.freshness_and_calibration_preconditions_met
            || candidate.worst_case_latency_ns > policy.maximum_action_latency_ns
            || candidate.worst_case_energy_uj > policy.maximum_action_energy_uj
            || (policy.require_new_dependency_group
                && known_groups.contains(&candidate.dependency_group_id))
        {
            continue;
        }
        if candidate.conditional_post_action_protection_level_upper_m
            >= policy.current_protection_level_upper_m
        {
            continue;
        }

        let reduction =
            policy.current_protection_level_upper_m
                - candidate.conditional_post_action_protection_level_upper_m;
        if !reduction.is_finite() || reduction <= 0.0 {
            return Err(ActiveEvidenceAcquisitionError::ArithmeticOverflow);
        }
        feasible.push(EvidenceActionAssessment {
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
        });
    }

    if feasible.is_empty() {
        return Err(ActiveEvidenceAcquisitionError::NoFeasibleAction);
    }

    let mut pareto_frontier = Vec::new();
    for candidate in &feasible {
        let dominated = feasible.iter().any(|other| {
            other.action_id != candidate.action_id
                && dominates(other, candidate)
        });
        if !dominated {
            pareto_frontier.push(*candidate);
        }
    }

    pareto_frontier.sort_by(|left, right| {
        right
            .guaranteed_protection_level_reduction_m
            .total_cmp(&left.guaranteed_protection_level_reduction_m)
            .then_with(|| left.worst_case_latency_ns.cmp(&right.worst_case_latency_ns))
            .then_with(|| left.worst_case_energy_uj.cmp(&right.worst_case_energy_uj))
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    let target_reaching: Vec<_> = pareto_frontier
        .iter()
        .filter(|candidate| candidate.reaches_target_if_acquisition_qualifies)
        .collect();

    let (reason, selected) = if !target_reaching.is_empty() {
        let selected = target_reaching
            .into_iter()
            .min_by(|left, right| {
                left.worst_case_latency_ns
                    .cmp(&right.worst_case_latency_ns)
                    .then_with(|| left.worst_case_energy_uj.cmp(&right.worst_case_energy_uj))
                    .then_with(|| left.action_id.cmp(&right.action_id))
            })
            .expect("nonempty target-reaching frontier");
        (
            AcquisitionPlanReason::LowestLatencyTargetReachingAction,
            *selected,
        )
    } else {
        let selected = pareto_frontier[0];
        (
            AcquisitionPlanReason::BestReductionAvailableButTargetNotReached,
            selected,
        )
    };

    Ok(ActiveEvidenceAcquisitionPlan {
        reason,
        recommended_action_id: Some(selected.action_id),
        pareto_frontier,
        conditional_target_reached: selected.reaches_target_if_acquisition_qualifies,
        authority_transition_not_authorized_by_plan: true,
    })
}

fn validate_policy(
    policy: &ActiveEvidencePolicy,
) -> Result<(), ActiveEvidenceAcquisitionError> {
    if !policy.current_protection_level_upper_m.is_finite()
        || policy.current_protection_level_upper_m < 0.0
        || !policy.target_protection_level_upper_m.is_finite()
        || policy.target_protection_level_upper_m < 0.0
    {
        return Err(ActiveEvidenceAcquisitionError::InvalidProtectionLevel);
    }
    if policy
        .already_observed_dependency_groups
        .iter()
        .any(|group| *group == 0)
    {
        return Err(ActiveEvidenceAcquisitionError::InvalidDependencyGroup);
    }
    Ok(())
}

fn dominates(
    left: &EvidenceActionAssessment,
    right: &EvidenceActionAssessment,
) -> bool {
    let no_worse = left.guaranteed_protection_level_reduction_m
        >= right.guaranteed_protection_level_reduction_m
        && left.worst_case_latency_ns <= right.worst_case_latency_ns
        && left.worst_case_energy_uj <= right.worst_case_energy_uj;
    let strictly_better = left.guaranteed_protection_level_reduction_m
        > right.guaranteed_protection_level_reduction_m
        || left.worst_case_latency_ns < right.worst_case_latency_ns
        || left.worst_case_energy_uj < right.worst_case_energy_uj;
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ActiveEvidencePolicy {
        ActiveEvidencePolicy {
            current_protection_level_upper_m: 3.0,
            target_protection_level_upper_m: 1.0,
            maximum_action_latency_ns: 100,
            maximum_action_energy_uj: 1_000,
            require_new_dependency_group: false,
            already_observed_dependency_groups: vec![10, 20],
        }
    }

    fn candidate(
        id: u64,
        group: u64,
        post_pl: f64,
        latency: u64,
        energy: u64,
    ) -> EvidenceActionCandidate {
        EvidenceActionCandidate {
            action_id: id,
            kind: EvidenceActionKind::AcquireCir,
            dependency_group_id: group,
            conditional_post_action_protection_level_upper_m: post_pl,
            worst_case_latency_ns: latency,
            worst_case_energy_uj: energy,
            available: true,
            freshness_and_calibration_preconditions_met: true,
        }
    }

    #[test]
    fn selects_lowest_latency_target_reaching_pareto_action() {
        let result = plan_active_evidence_acquisition(
            &policy(),
            &[
                candidate(1, 30, 0.8, 100, 500),
                candidate(2, 40, 0.9, 50, 300),
                candidate(3, 50, 0.5, 80, 1_000),
            ],
        )
        .unwrap();

        assert_eq!(
            result.reason,
            AcquisitionPlanReason::LowestLatencyTargetReachingAction
        );
        assert_eq!(result.recommended_action_id, Some(2));
        assert!(result.conditional_target_reached);
        assert!(result.authority_transition_not_authorized_by_plan);
        assert_eq!(result.pareto_frontier.len(), 3);
    }

    #[test]
    fn enforces_latency_and_energy_budgets_before_frontier() {
        let result = plan_active_evidence_acquisition(
            &policy(),
            &[
                candidate(1, 30, 0.5, 101, 500),
                candidate(2, 40, 0.6, 50, 1_001),
                candidate(3, 50, 0.8, 90, 900),
            ],
        )
        .unwrap();

        assert_eq!(result.recommended_action_id, Some(3));
        assert_eq!(result.pareto_frontier.len(), 1);
    }

    #[test]
    fn requires_a_new_dependency_group_when_policy_requests_it() {
        let mut required = policy();
        required.require_new_dependency_group = true;
        let result = plan_active_evidence_acquisition(
            &required,
            &[
                candidate(1, 10, 0.5, 10, 10),
                candidate(2, 20, 0.6, 20, 20),
                candidate(3, 30, 0.8, 30, 30),
            ],
        )
        .unwrap();
        assert_eq!(result.recommended_action_id, Some(3));
    }

    #[test]
    fn remains_below_target_when_no_available_action_can_reach_it() {
        let result = plan_active_evidence_acquisition(
            &policy(),
            &[
                candidate(1, 30, 2.0, 80, 700),
                candidate(2, 40, 1.5, 90, 800),
            ],
        )
        .unwrap();
        assert_eq!(
            result.reason,
            AcquisitionPlanReason::BestReductionAvailableButTargetNotReached
        );
        assert_eq!(result.recommended_action_id, Some(2));
        assert!(!result.conditional_target_reached);
        assert!(result.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn removes_pareto_dominated_actions() {
        let result = plan_active_evidence_acquisition(
            &policy(),
            &[
                candidate(1, 30, 0.8, 50, 300),
                candidate(2, 40, 0.9, 60, 400),
                candidate(3, 50, 0.7, 70, 500),
            ],
        )
        .unwrap();
        assert_eq!(
            result
                .pareto_frontier
                .iter()
                .map(|candidate| candidate.action_id)
                .collect::<Vec<_>>(),
            vec![3, 1]
        );
        assert!(!result
            .pareto_frontier
            .iter()
            .any(|candidate| candidate.action_id == 2));
    }

    #[test]
    fn target_already_satisfied_produces_no_request() {
        let mut satisfied = policy();
        satisfied.current_protection_level_upper_m = 0.5;
        let result = plan_active_evidence_acquisition(&satisfied, &[]).unwrap();
        assert_eq!(result.reason, AcquisitionPlanReason::TargetAlreadySatisfied);
        assert_eq!(result.recommended_action_id, None);
        assert!(result.must_requalify_after_acquisition);
    }

    #[test]
    fn stale_unavailable_and_nonreducing_actions_are_not_selected() {
        let mut stale = candidate(1, 30, 0.5, 10, 10);
        stale.freshness_and_calibration_preconditions_met = false;
        let mut unavailable = candidate(2, 40, 0.6, 10, 10);
        unavailable.available = false;
        let nonreducing = candidate(3, 50, 3.0, 10, 10);
        assert_eq!(
            plan_active_evidence_acquisition(
                &policy(),
                &[stale, unavailable, nonreducing]
            ),
            Err(ActiveEvidenceAcquisitionError::NoFeasibleAction)
        );
    }

    #[test]
    fn invalid_and_duplicate_action_identities_fail_closed() {
        assert_eq!(
            plan_active_evidence_acquisition(
                &policy(),
                &[candidate(1, 0, 0.5, 10, 10)]
            ),
            Err(ActiveEvidenceAcquisitionError::InvalidActionIdentity)
        );
        assert_eq!(
            plan_active_evidence_acquisition(
                &policy(),
                &[
                    candidate(1, 30, 0.5, 10, 10),
                    candidate(1, 40, 0.6, 20, 20),
                ]
            ),
            Err(ActiveEvidenceAcquisitionError::DuplicateActionId(1))
        );
    }
}
