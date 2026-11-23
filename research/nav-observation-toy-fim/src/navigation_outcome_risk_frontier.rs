//! Multi-objective selection over reliability-admitted navigation outcome-risk assessments.
//!
//! This selector is downstream of the #7165 reliability-aware evidence plan,
//! #7211 interval-robust impact assessment and the finite outcome-risk evaluator.
//! It keeps mean residual, VaR, AVaR, worst-case residual, latency and energy as
//! separate Pareto dimensions. Hard profile ceilings are applied before
//! Pareto construction. A deterministic recommendation is then chosen only
//! using the explicit lexicographic preference declared by the profile owner.
//!
//! This module does not acquire observations, mutate a posterior, update a
//! protection level, retry an unknown outcome or authorize navigation.

use std::collections::BTreeSet;
use std::fmt;

use crate::active_evidence_acquisition::EvidenceActionKind;
use crate::navigation_outcome_risk::NavigationOutcomeRiskAssessment;

const PROBABILITY_SCALE: u64 = 1_000_000_000;
const RISK_SCALE: u32 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationRiskSelectionPreference {
    AvarThenWorstThenLatency,
    WorstThenAvarThenLatency,
    VarThenAvarThenWorst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationActionProtocolCommitment {
    pub action_id: u64,
    pub acquisition_protocol_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavigationRiskSelectionPolicy {
    pub expected_domain_fingerprint_sha256: [u8; 32],
    /// Predeclared per-action protocol allowlist. Different sensing modalities
    /// may use different protocols, but every protocol is bound to one action.
    pub action_protocol_commitments: Vec<NavigationActionProtocolCommitment>,
    pub expected_decision_partition_sha256: [u8; 32],
    pub expected_impact_model_sha256: [u8; 32],
    pub expected_impact_uncertainty_model_sha256: [u8; 32],
    pub maximum_action_latency_ns: u64,
    pub maximum_action_energy_uj: u64,
    /// All candidates must use the same AVaR confidence level.
    pub expected_avar_confidence_level_parts_per_billion: u64,
    pub minimum_acquisition_probability_lower_bound: f64,
    pub minimum_independent_campaign_blocks: usize,
    pub maximum_reliability_confidence_error_probability: f64,
    pub maximum_mean_residual_impact_parts_per_billion: u32,
    pub maximum_var_residual_impact_parts_per_billion: u32,
    pub maximum_avar_residual_impact_parts_per_billion: u32,
    pub maximum_worst_scenario_residual_impact_parts_per_billion: u32,
    pub preference: NavigationRiskSelectionPreference,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavigationRiskActionCandidate {
    pub risk: NavigationOutcomeRiskAssessment,
    pub action_kind: EvidenceActionKind,
    pub dependency_group_id: u64,
    pub worst_case_latency_ns: u64,
    pub worst_case_energy_uj: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationRiskActionRejectionReason {
    ResourceBudgetExceeded,
    MeanResidualCeilingExceeded,
    VarResidualCeilingExceeded,
    AvarResidualCeilingExceeded,
    WorstResidualCeilingExceeded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavigationRiskActionAssessment {
    pub candidate: NavigationRiskActionCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationRiskPlanReason {
    SelectedByDeclaredLexicographicPreference,
    NoRiskAndResourceAdmissibleAction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavigationOutcomeRiskParetoPlan {
    pub reason: NavigationRiskPlanReason,
    pub expected_domain_fingerprint_sha256: [u8; 32],
    pub action_protocol_commitments: Vec<NavigationActionProtocolCommitment>,
    pub recommended_action_id: Option<u64>,
    pub pareto_frontier: Vec<NavigationRiskActionAssessment>,
    pub rejected_actions: Vec<(u64, NavigationRiskActionRejectionReason)>,
    /// Always true: this selector cannot authorize a navigation transition.
    pub authority_transition_not_authorized_by_plan: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationRiskSelectionError {
    InvalidPolicy,
    InvalidActionIdentity,
    DuplicateActionId(u64),
    InvalidDependencyGroup(u64),
    InvalidRiskAssessment(u64),
    DomainMismatch(u64),
    AcquisitionProtocolMismatch(u64),
    DecisionPartitionMismatch(u64),
    ImpactModelMismatch(u64),
    ImpactUncertaintyModelMismatch(u64),
    ArithmeticOverflow,
}

impl fmt::Display for NavigationRiskSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(f, "risk-selection policy has invalid commitments, budgets or residual ceilings"),
            Self::InvalidActionIdentity => write!(f, "candidate action ID must be nonzero"),
            Self::DuplicateActionId(id) => write!(f, "candidate action ID {id} is duplicated"),
            Self::InvalidDependencyGroup(id) => write!(f, "candidate action {id} has an invalid dependency group"),
            Self::InvalidRiskAssessment(id) => write!(f, "candidate action {id} has an incomplete or internally inconsistent risk assessment"),
            Self::DomainMismatch(id) => write!(f, "candidate action {id} has a different domain commitment"),
            Self::AcquisitionProtocolMismatch(id) => write!(f, "candidate action {id} has a different acquisition protocol"),
            Self::DecisionPartitionMismatch(id) => write!(f, "candidate action {id} has a different decision partition"),
            Self::ImpactModelMismatch(id) => write!(f, "candidate action {id} has a different impact model"),
            Self::ImpactUncertaintyModelMismatch(id) => write!(f, "candidate action {id} has a different impact-uncertainty model"),
            Self::ArithmeticOverflow => write!(f, "risk-selection arithmetic overflowed"),
        }
    }
}

impl std::error::Error for NavigationRiskSelectionError {}

/// Construct a Pareto plan over multiple independently assessed candidate actions.
///
/// Every candidate must be tied to the exact same domain, acquisition protocol,
/// decision partition, impact model and impact-uncertainty model. Outcome-model
/// and scenario-law digests remain action-specific and are retained in the
/// returned risk assessments.
///
/// Hard ceilings are conjunctive. A candidate that violates any hard resource
/// or residual-risk ceiling is rejected before Pareto comparison. The Pareto
/// objective minimizes mean residual, VaR, AVaR, worst residual, latency and
/// energy. The declared lexicographic preference is used only to choose one
/// recommendation from the resulting frontier; it never replaces the
/// multidimensional frontier with a hidden scalar score.
pub fn plan_navigation_outcome_risk_frontier(
    policy: &NavigationRiskSelectionPolicy,
    candidates: &[NavigationRiskActionCandidate],
) -> Result<NavigationOutcomeRiskParetoPlan, NavigationRiskSelectionError> {
    validate_policy(policy)?;

    let mut seen_ids = BTreeSet::new();
    let mut eligible = Vec::new();
    let mut rejected = Vec::new();

    for candidate in candidates {
        let id = candidate.risk.action_id;
        if id == 0 {
            return Err(NavigationRiskSelectionError::InvalidActionIdentity);
        }
        if !seen_ids.insert(id) {
            return Err(NavigationRiskSelectionError::DuplicateActionId(id));
        }
        if candidate.dependency_group_id == 0 {
            return Err(NavigationRiskSelectionError::InvalidDependencyGroup(id));
        }
        validate_risk(policy, &candidate.risk, id)?;
        validate_commitments(policy, &candidate.risk, id)?;

        let reason = if candidate.worst_case_latency_ns > policy.maximum_action_latency_ns
            || candidate.worst_case_energy_uj > policy.maximum_action_energy_uj
        {
            Some(NavigationRiskActionRejectionReason::ResourceBudgetExceeded)
        } else if candidate.risk.mean_residual_impact_upper_parts_per_billion
            > policy.maximum_mean_residual_impact_parts_per_billion
        {
            Some(NavigationRiskActionRejectionReason::MeanResidualCeilingExceeded)
        } else if candidate.risk.value_at_risk_upper_parts_per_billion
            > policy.maximum_var_residual_impact_parts_per_billion
        {
            Some(NavigationRiskActionRejectionReason::VarResidualCeilingExceeded)
        } else if candidate.risk.avar_residual_impact_upper_parts_per_billion
            > policy.maximum_avar_residual_impact_parts_per_billion
        {
            Some(NavigationRiskActionRejectionReason::AvarResidualCeilingExceeded)
        } else if candidate.risk.worst_scenario_residual_impact_upper_parts_per_billion
            > policy.maximum_worst_scenario_residual_impact_parts_per_billion
        {
            Some(NavigationRiskActionRejectionReason::WorstResidualCeilingExceeded)
        } else {
            None
        };

        if let Some(reason) = reason {
            rejected.push((id, reason));
        } else {
            eligible.push(NavigationRiskActionAssessment {
                candidate: candidate.clone(),
            });
        }
    }

    let mut frontier = Vec::new();
    for candidate in &eligible {
        let dominated = eligible.iter().any(|other| {
            other.candidate.risk.action_id != candidate.candidate.risk.action_id
                && dominates(other, candidate)
        });
        if !dominated {
            frontier.push(candidate.clone());
        }
    }

    frontier.sort_by(|left, right| {
        compare_preference(policy.preference, left, right)
            .then_with(|| left.candidate.risk.action_id.cmp(&right.candidate.risk.action_id))
    });

    let recommended_action_id = frontier.first().map(|item| item.candidate.risk.action_id);
    let reason = if recommended_action_id.is_some() {
        NavigationRiskPlanReason::SelectedByDeclaredLexicographicPreference
    } else {
        NavigationRiskPlanReason::NoRiskAndResourceAdmissibleAction
    };

    Ok(NavigationOutcomeRiskParetoPlan {
        reason,
        expected_domain_fingerprint_sha256: policy.expected_domain_fingerprint_sha256,
        action_protocol_commitments: policy.action_protocol_commitments.clone(),
        recommended_action_id,
        pareto_frontier: frontier,
        rejected_actions: rejected,
        authority_transition_not_authorized_by_plan: true,
    })
}

fn validate_policy(
    policy: &NavigationRiskSelectionPolicy,
) -> Result<(), NavigationRiskSelectionError> {
    if policy.expected_domain_fingerprint_sha256.iter().all(|byte| *byte == 0)
        || policy.action_protocol_commitments.is_empty()
        || policy.expected_decision_partition_sha256.iter().all(|byte| *byte == 0)
        || policy.expected_impact_model_sha256.iter().all(|byte| *byte == 0)
        || policy.expected_impact_uncertainty_model_sha256.iter().all(|byte| *byte == 0)
        || policy.maximum_action_latency_ns == 0
        || policy.maximum_action_energy_uj == 0
        || policy.expected_avar_confidence_level_parts_per_billion == 0
        || policy.expected_avar_confidence_level_parts_per_billion >= PROBABILITY_SCALE
        || !policy.minimum_acquisition_probability_lower_bound.is_finite()
        || policy.minimum_acquisition_probability_lower_bound <= 0.0
        || policy.minimum_acquisition_probability_lower_bound > 1.0
        || policy.minimum_independent_campaign_blocks == 0
        || !policy.maximum_reliability_confidence_error_probability.is_finite()
        || policy.maximum_reliability_confidence_error_probability <= 0.0
        || policy.maximum_reliability_confidence_error_probability >= 1.0
        || policy.maximum_mean_residual_impact_parts_per_billion > RISK_SCALE
        || policy.maximum_var_residual_impact_parts_per_billion > RISK_SCALE
        || policy.maximum_avar_residual_impact_parts_per_billion > RISK_SCALE
        || policy.maximum_worst_scenario_residual_impact_parts_per_billion > RISK_SCALE
    {
        return Err(NavigationRiskSelectionError::InvalidPolicy);
    }
    let mut seen_actions = BTreeSet::new();
    for binding in &policy.action_protocol_commitments {
        if binding.action_id == 0
            || binding.acquisition_protocol_sha256.iter().all(|byte| *byte == 0)
            || !seen_actions.insert(binding.action_id)
        {
            return Err(NavigationRiskSelectionError::InvalidPolicy);
        }
    }
    Ok(())
}

fn validate_commitments(
    policy: &NavigationRiskSelectionPolicy,
    risk: &NavigationOutcomeRiskAssessment,
    action_id: u64,
) -> Result<(), NavigationRiskSelectionError> {
    if risk.domain_fingerprint_sha256 != policy.expected_domain_fingerprint_sha256 {
        return Err(NavigationRiskSelectionError::DomainMismatch(action_id));
    }
    let expected_protocol = policy
        .action_protocol_commitments
        .iter()
        .find(|binding| binding.action_id == action_id)
        .map(|binding| binding.acquisition_protocol_sha256)
        .ok_or(NavigationRiskSelectionError::AcquisitionProtocolMismatch(action_id))?;
    if risk.acquisition_protocol_sha256 != expected_protocol {
        return Err(NavigationRiskSelectionError::AcquisitionProtocolMismatch(action_id));
    }
    if risk.decision_partition_sha256 != policy.expected_decision_partition_sha256 {
        return Err(NavigationRiskSelectionError::DecisionPartitionMismatch(action_id));
    }
    if risk.impact_model_sha256 != policy.expected_impact_model_sha256 {
        return Err(NavigationRiskSelectionError::ImpactModelMismatch(action_id));
    }
    if risk.impact_uncertainty_model_sha256
        != policy.expected_impact_uncertainty_model_sha256
    {
        return Err(NavigationRiskSelectionError::ImpactUncertaintyModelMismatch(action_id));
    }
    Ok(())
}

fn validate_risk(
    policy: &NavigationRiskSelectionPolicy,
    risk: &NavigationOutcomeRiskAssessment,
    expected_action_id: u64,
) -> Result<(), NavigationRiskSelectionError> {
    let total_probability = risk
        .qualified_observation_probability_mass_parts_per_billion
        .checked_add(risk.failed_or_timed_out_probability_mass_parts_per_billion)
        .and_then(|value| value.checked_add(risk.acquired_but_invalid_probability_mass_parts_per_billion))
        .and_then(|value| value.checked_add(risk.unknown_or_unreconciled_probability_mass_parts_per_billion))
        .ok_or(NavigationRiskSelectionError::ArithmeticOverflow)?;

    if risk.action_id != expected_action_id
        || risk.scenario_count == 0
        || total_probability != PROBABILITY_SCALE
        || risk.avar_confidence_level_parts_per_billion
            != policy.expected_avar_confidence_level_parts_per_billion
        || risk.avar_tail_probability_mass_parts_per_billion
            != PROBABILITY_SCALE - policy.expected_avar_confidence_level_parts_per_billion
        || risk.mean_residual_impact_upper_parts_per_billion > RISK_SCALE
        || risk.value_at_risk_upper_parts_per_billion > RISK_SCALE
        || risk.avar_residual_impact_upper_parts_per_billion > RISK_SCALE
        || risk.worst_scenario_residual_impact_upper_parts_per_billion > RISK_SCALE
        || risk.mean_residual_impact_upper_parts_per_billion
            > risk.avar_residual_impact_upper_parts_per_billion
        || risk.value_at_risk_upper_parts_per_billion
            > risk.avar_residual_impact_upper_parts_per_billion
        || risk.avar_residual_impact_upper_parts_per_billion
            > risk.worst_scenario_residual_impact_upper_parts_per_billion
        || risk.domain_fingerprint_sha256.iter().all(|byte| *byte == 0)
        || risk.acquisition_protocol_sha256.iter().all(|byte| *byte == 0)
        || risk.decision_partition_sha256.iter().all(|byte| *byte == 0)
        || risk.impact_model_sha256.iter().all(|byte| *byte == 0)
        || risk.impact_uncertainty_model_sha256.iter().all(|byte| *byte == 0)
        || risk.outcome_model_sha256.iter().all(|byte| *byte == 0)
        || risk.scenario_law_sha256.iter().all(|byte| *byte == 0)
        || risk.reliability_source_manifest_sha256.iter().all(|byte| *byte == 0)
        || risk.reliability_calibration_cohort_sha256.iter().all(|byte| *byte == 0)
        || risk.reliability_bound_method_sha256.iter().all(|byte| *byte == 0)
        || risk.independent_campaign_block_count < policy.minimum_independent_campaign_blocks
        || !risk.reliability_probability_lower_bound.is_finite()
        || !risk.reliability_probability_upper_bound.is_finite()
        || risk.reliability_probability_lower_bound < 0.0
        || risk.reliability_probability_upper_bound > 1.0
        || risk.reliability_probability_lower_bound > risk.reliability_probability_upper_bound
        || risk.reliability_probability_lower_bound
            < policy.minimum_acquisition_probability_lower_bound
        || !risk.reliability_confidence_error_probability.is_finite()
        || risk.reliability_confidence_error_probability <= 0.0
        || risk.reliability_confidence_error_probability
            > policy.maximum_reliability_confidence_error_probability
        || risk.reliability_confidence_error_probability >= 1.0
        || (risk.qualified_observation_probability_mass_parts_per_billion as f64
            / PROBABILITY_SCALE as f64) < risk.reliability_probability_lower_bound
        || (risk.qualified_observation_probability_mass_parts_per_billion as f64
            / PROBABILITY_SCALE as f64) > risk.reliability_probability_upper_bound
    {
        return Err(NavigationRiskSelectionError::InvalidRiskAssessment(
            expected_action_id,
        ));
    }
    Ok(())
}

fn dominates(
    left: &NavigationRiskActionAssessment,
    right: &NavigationRiskActionAssessment,
) -> bool {
    let l = &left.candidate;
    let r = &right.candidate;
    let left_values = (
        l.risk.mean_residual_impact_upper_parts_per_billion,
        l.risk.value_at_risk_upper_parts_per_billion,
        l.risk.avar_residual_impact_upper_parts_per_billion,
        l.risk.worst_scenario_residual_impact_upper_parts_per_billion,
        l.worst_case_latency_ns,
        l.worst_case_energy_uj,
    );
    let right_values = (
        r.risk.mean_residual_impact_upper_parts_per_billion,
        r.risk.value_at_risk_upper_parts_per_billion,
        r.risk.avar_residual_impact_upper_parts_per_billion,
        r.risk.worst_scenario_residual_impact_upper_parts_per_billion,
        r.worst_case_latency_ns,
        r.worst_case_energy_uj,
    );
    let no_worse = left_values.0 <= right_values.0
        && left_values.1 <= right_values.1
        && left_values.2 <= right_values.2
        && left_values.3 <= right_values.3
        && left_values.4 <= right_values.4
        && left_values.5 <= right_values.5;
    let strictly_better = left_values.0 < right_values.0
        || left_values.1 < right_values.1
        || left_values.2 < right_values.2
        || left_values.3 < right_values.3
        || left_values.4 < right_values.4
        || left_values.5 < right_values.5;
    no_worse && strictly_better
}

fn compare_preference(
    preference: NavigationRiskSelectionPreference,
    left: &NavigationRiskActionAssessment,
    right: &NavigationRiskActionAssessment,
) -> std::cmp::Ordering {
    let l = &left.candidate;
    let r = &right.candidate;
    match preference {
        NavigationRiskSelectionPreference::AvarThenWorstThenLatency => l
            .risk
            .avar_residual_impact_upper_parts_per_billion
            .cmp(&r.risk.avar_residual_impact_upper_parts_per_billion)
            .then_with(|| l.risk.worst_scenario_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.worst_scenario_residual_impact_upper_parts_per_billion
            ))
            .then_with(|| l.worst_case_latency_ns.cmp(&r.worst_case_latency_ns))
            .then_with(|| l.worst_case_energy_uj.cmp(&r.worst_case_energy_uj))
            .then_with(|| l.risk.value_at_risk_upper_parts_per_billion.cmp(
                &r.risk.value_at_risk_upper_parts_per_billion
            ))
            .then_with(|| l.risk.mean_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.mean_residual_impact_upper_parts_per_billion
            )),
        NavigationRiskSelectionPreference::WorstThenAvarThenLatency => l
            .risk
            .worst_scenario_residual_impact_upper_parts_per_billion
            .cmp(&r.risk.worst_scenario_residual_impact_upper_parts_per_billion)
            .then_with(|| l.risk.avar_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.avar_residual_impact_upper_parts_per_billion
            ))
            .then_with(|| l.worst_case_latency_ns.cmp(&r.worst_case_latency_ns))
            .then_with(|| l.worst_case_energy_uj.cmp(&r.worst_case_energy_uj))
            .then_with(|| l.risk.value_at_risk_upper_parts_per_billion.cmp(
                &r.risk.value_at_risk_upper_parts_per_billion
            ))
            .then_with(|| l.risk.mean_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.mean_residual_impact_upper_parts_per_billion
            )),
        NavigationRiskSelectionPreference::VarThenAvarThenWorst => l
            .risk
            .value_at_risk_upper_parts_per_billion
            .cmp(&r.risk.value_at_risk_upper_parts_per_billion)
            .then_with(|| l.risk.avar_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.avar_residual_impact_upper_parts_per_billion
            ))
            .then_with(|| l.risk.worst_scenario_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.worst_scenario_residual_impact_upper_parts_per_billion
            ))
            .then_with(|| l.worst_case_latency_ns.cmp(&r.worst_case_latency_ns))
            .then_with(|| l.worst_case_energy_uj.cmp(&r.worst_case_energy_uj))
            .then_with(|| l.risk.mean_residual_impact_upper_parts_per_billion.cmp(
                &r.risk.mean_residual_impact_upper_parts_per_billion
            )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation_outcome_risk::NavigationOutcomeRiskAssessment;

    fn policy() -> NavigationRiskSelectionPolicy {
        NavigationRiskSelectionPolicy {
            expected_domain_fingerprint_sha256: [0x11; 32],
            action_protocol_commitments: (1..=20)
                .map(|action_id| NavigationActionProtocolCommitment {
                    action_id,
                    acquisition_protocol_sha256: [0x22; 32],
                })
                .collect(),
            expected_decision_partition_sha256: [0x33; 32],
            expected_impact_model_sha256: [0x44; 32],
            expected_impact_uncertainty_model_sha256: [0x55; 32],
            maximum_action_latency_ns: 1_000,
            maximum_action_energy_uj: 1_000,
            expected_avar_confidence_level_parts_per_billion: 950_000_000,
            minimum_acquisition_probability_lower_bound: 0.8,
            minimum_independent_campaign_blocks: 10,
            maximum_reliability_confidence_error_probability: 0.05,
            maximum_mean_residual_impact_parts_per_billion: 900_000_000,
            maximum_var_residual_impact_parts_per_billion: 900_000_000,
            maximum_avar_residual_impact_parts_per_billion: 900_000_000,
            maximum_worst_scenario_residual_impact_parts_per_billion: 1_000_000_000,
            preference: NavigationRiskSelectionPreference::AvarThenWorstThenLatency,
        }
    }

    fn candidate(
        action_id: u64,
        mean: u32,
        var: u32,
        avar: u32,
        worst: u32,
        latency: u64,
        energy: u64,
    ) -> NavigationRiskActionCandidate {
        NavigationRiskActionCandidate {
            risk: NavigationOutcomeRiskAssessment {
                action_id,
                domain_fingerprint_sha256: [0x11; 32],
                acquisition_protocol_sha256: [0x22; 32],
                decision_partition_sha256: [0x33; 32],
                impact_model_sha256: [0x44; 32],
                impact_uncertainty_model_sha256: [0x55; 32],
                outcome_model_sha256: [action_id as u8; 32],
                scenario_law_sha256: [action_id.wrapping_add(1) as u8; 32],
                avar_confidence_level_parts_per_billion: 950_000_000,
                avar_tail_probability_mass_parts_per_billion: 50_000_000,
                scenario_count: 2,
                qualified_observation_probability_mass_parts_per_billion: 900_000_000,
                failed_or_timed_out_probability_mass_parts_per_billion: 100_000_000,
                acquired_but_invalid_probability_mass_parts_per_billion: 0,
                unknown_or_unreconciled_probability_mass_parts_per_billion: 0,
                mean_residual_impact_upper_parts_per_billion: mean,
                value_at_risk_upper_parts_per_billion: var,
                avar_residual_impact_upper_parts_per_billion: avar,
                worst_scenario_residual_impact_upper_parts_per_billion: worst,
                independent_campaign_block_count: 100,
                reliability_confidence_error_probability: 0.05,
                reliability_probability_lower_bound: 0.9,
                reliability_probability_upper_bound: 1.0,
                reliability_source_manifest_sha256: [0x61; 32],
                reliability_calibration_cohort_sha256: [0x62; 32],
                reliability_bound_method_sha256: [0x63; 32],
            },
            action_kind: EvidenceActionKind::AcquireCir,
            dependency_group_id: action_id + 100,
            worst_case_latency_ns: latency,
            worst_case_energy_uj: energy,
        }
    }

    #[test]
    fn pareto_frontier_preserves_tradeoffs_between_tail_risk_and_latency() {
        let result = plan_navigation_outcome_risk_frontier(
            &policy(),
            &[
                candidate(1, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800),
                candidate(2, 350_000_000, 400_000_000, 450_000_000, 600_000_000, 400, 400),
            ],
        )
        .unwrap();
        assert_eq!(result.pareto_frontier.len(), 2);
        assert_eq!(
            result.recommended_action_id,
            Some(2),
            "declared AVaR-first preference chooses the lower tail risk"
        );
    }

    #[test]
    fn dominated_actions_are_removed_without_collapsing_the_other_axes() {
        let result = plan_navigation_outcome_risk_frontier(
            &policy(),
            &[
                candidate(1, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800),
                candidate(2, 350_000_000, 400_000_000, 450_000_000, 600_000_000, 400, 400),
                candidate(3, 400_000_000, 500_000_000, 600_000_000, 800_000_000, 900, 900),
            ],
        )
        .unwrap();
        assert_eq!(result.pareto_frontier.len(), 2);
        assert!(result.pareto_frontier.iter().all(|item| item.candidate.risk.action_id != 3));
    }

    #[test]
    fn hard_resource_and_risk_ceilings_reject_before_pareto_ranking() {
        let result = plan_navigation_outcome_risk_frontier(
            &policy(),
            &[
                candidate(1, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800),
                candidate(2, 300_000_000, 700_000_000, 950_000_000, 1_000_000_000, 400, 400),
                candidate(3, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 1_200, 400),
            ],
        )
        .unwrap();
        assert_eq!(result.recommended_action_id, Some(1));
        assert_eq!(result.rejected_actions.len(), 2);
        assert_eq!(result.rejected_actions[0].1, NavigationRiskActionRejectionReason::AvarResidualCeilingExceeded);
        assert_eq!(result.rejected_actions[1].1, NavigationRiskActionRejectionReason::ResourceBudgetExceeded);
    }

    #[test]
    fn no_candidate_meets_hard_contract_so_selector_abstains() {
        let result = plan_navigation_outcome_risk_frontier(
            &policy(),
            &[candidate(1, 300_000_000, 900_000_000, 950_000_000, 1_000_000_000, 800, 800)],
        )
        .unwrap();
        assert_eq!(result.recommended_action_id, None);
        assert_eq!(result.reason, NavigationRiskPlanReason::NoRiskAndResourceAdmissibleAction);
        assert!(result.authority_transition_not_authorized_by_plan);
    }

    #[test]
    fn action_ids_and_commitments_must_be_complete_and_consistent() {
        let mut invalid = policy();
        invalid.expected_domain_fingerprint_sha256 = [0; 32];
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&invalid, &[]),
            Err(NavigationRiskSelectionError::InvalidPolicy)
        );

        let duplicate = vec![
            candidate(1, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800),
            candidate(1, 350_000_000, 400_000_000, 450_000_000, 600_000_000, 400, 400),
        ];
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &duplicate),
            Err(NavigationRiskSelectionError::DuplicateActionId(1))
        );

        let mut mismatch = candidate(4, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800);
        mismatch.risk.impact_model_sha256 = [0x99; 32];
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[mismatch]),
            Err(NavigationRiskSelectionError::ImpactModelMismatch(4))
        );
    }

    #[test]
    fn risk_metrics_must_be_monotone_and_probability_mass_must_sum_to_one() {
        let mut bad = candidate(1, 300_000_000, 700_000_000, 500_000_000, 700_000_000, 800, 800);
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[bad.clone()]),
            Err(NavigationRiskSelectionError::InvalidRiskAssessment(1))
        );
        bad = candidate(1, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800);
        bad.risk.unknown_or_unreconciled_probability_mass_parts_per_billion = 1;
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[bad]),
            Err(NavigationRiskSelectionError::InvalidRiskAssessment(1))
        );
    }

    #[test]
    fn action_specific_protocol_allowlist_supports_multimodal_comparison() {
        let mut first = candidate(1, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800);
        let mut second = candidate(2, 350_000_000, 400_000_000, 450_000_000, 600_000_000, 400, 400);
        second.risk.acquisition_protocol_sha256 = [0x77; 32];

        let mut cfg = policy();
        cfg.action_protocol_commitments
            .iter_mut()
            .find(|item| item.action_id == 2)
            .unwrap()
            .acquisition_protocol_sha256 = [0x77; 32];

        let plan = plan_navigation_outcome_risk_frontier(&cfg, &[first, second]).unwrap();
        assert_eq!(plan.pareto_frontier.len(), 2);
        assert!(plan.action_protocol_commitments.iter().any(|item|
            item.action_id == 1 && item.acquisition_protocol_sha256 == [0x22; 32]
        ));
        assert!(plan.action_protocol_commitments.iter().any(|item|
            item.action_id == 2 && item.acquisition_protocol_sha256 == [0x77; 32]
        ));
    }

    #[test]
    fn undeclared_or_mismatched_action_protocol_rejects() {
        let mut undeclared = candidate(21, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800);
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[undeclared]),
            Err(NavigationRiskSelectionError::AcquisitionProtocolMismatch(21))
        );

        let mut mismatch = candidate(2, 300_000_000, 400_000_000, 500_000_000, 700_000_000, 800, 800);
        mismatch.risk.acquisition_protocol_sha256 = [0x99; 32];
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[mismatch]),
            Err(NavigationRiskSelectionError::AcquisitionProtocolMismatch(2))
        );
    }

    #[test]
    fn confidence_level_and_reliability_strength_must_be_comparable() {
        let mut wrong_alpha = candidate(
            1,
            300_000_000,
            400_000_000,
            500_000_000,
            700_000_000,
            800,
            800,
        );
        wrong_alpha.risk.avar_confidence_level_parts_per_billion = 900_000_000;
        wrong_alpha.risk.avar_tail_probability_mass_parts_per_billion = 100_000_000;
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[wrong_alpha]),
            Err(NavigationRiskSelectionError::InvalidRiskAssessment(1))
        );

        let mut weak_reliability = candidate(
            2,
            300_000_000,
            400_000_000,
            500_000_000,
            700_000_000,
            800,
            800,
        );
        weak_reliability.risk.reliability_probability_lower_bound = 0.7;
        assert_eq!(
            plan_navigation_outcome_risk_frontier(&policy(), &[weak_reliability]),
            Err(NavigationRiskSelectionError::InvalidRiskAssessment(2))
        );
    }

    #[test]
    fn preference_order_changes_only_the_tie_break_policy() {
        let candidates = [
            candidate(1, 200_000_000, 400_000_000, 600_000_000, 700_000_000, 300, 300),
            candidate(2, 200_000_000, 500_000_000, 500_000_000, 800_000_000, 300, 300),
        ];
        let mut avar_policy = policy();
        avar_policy.preference = NavigationRiskSelectionPreference::AvarThenWorstThenLatency;
        let first = plan_navigation_outcome_risk_frontier(&avar_policy, &candidates).unwrap();
        assert_eq!(first.recommended_action_id, Some(2));

        let mut worst_policy = policy();
        worst_policy.preference = NavigationRiskSelectionPreference::WorstThenAvarThenLatency;
        let second = plan_navigation_outcome_risk_frontier(&worst_policy, &candidates).unwrap();
        assert_eq!(second.recommended_action_id, Some(2));
    }
}
