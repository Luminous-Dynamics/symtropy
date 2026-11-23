//! Risk-aware evaluation of future active-navigation observation outcomes.
//!
//! This module separates three things that must not be conflated:
//! - the decision impact model (what residual ambiguity matters);
//! - the admitted valid-acquisition probability interval (whether an observation
//!   can arrive fresh, valid and qualified under one frozen action protocol);
//! - the external scenario law over possible qualified, failed and invalid
//!   observation outcomes.
//!
//! It computes mean residual impact, an upper quantile and upper-tail AVaR
//! (CVaR) from an explicitly supplied finite scenario law. The scenario
//! probabilities are not estimated here. A matching #7165 reliability plan
//! constrains the total probability mass assigned to qualified acquisition.
//!
//! Failed or invalid acquisition is required to retain the pre-action residual
//! impact. A qualified observation may improve or worsen residual impact.
//! Unknown/unreconciled execution is deliberately not a probabilistic outcome:
//! the attempt ledger must block new mutation until reconciliation.
//!
//! This is a deterministic finite-distribution risk reducer, not a posterior,
//! calibrated scenario model, physical safety guarantee, or authority transition.

use std::collections::BTreeSet;
use std::fmt;

use crate::active_evidence_reliability::AdmittedAcquisitionReliability;
use crate::active_evidence_reliability_planner::ReliableActiveEvidencePlan;
use crate::decision_impact_interval_robustness::{RobustDecisionClassPairAssessment, RobustDecisionImpactAssessment};

const PROBABILITY_SCALE: u64 = 1_000_000_000;
const RESIDUAL_SCALE: u32 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationObservationOutcomeKind {
    QualifiedObservation,
    FailedOrTimedOut,
    AcquiredButInvalid,
    UnknownOrUnreconciled,
}

/// One mutually exclusive branch in an externally committed action-outcome
/// scenario law. The residual-impact value must be an upper bound under the
/// exact impact uncertainty model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationObservationOutcomeScenario {
    pub scenario_id: u64,
    /// Exact probability mass in parts per billion. All scenarios must sum to 1e9.
    pub probability_mass_parts_per_billion: u64,
    pub outcome_kind: NavigationObservationOutcomeKind,
    pub residual_impact_upper_parts_per_billion: u32,
}

/// Frozen finite scenario law for one action and one navigation profile.
///
/// Digests are external commitments. This module checks presence/equality but
/// does not authenticate source bytes, estimate scenario probabilities or
/// validate that the scenario law covers the physical outcome space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationOutcomeRiskContract {
    pub action_id: u64,
    pub decision_partition_sha256: [u8; 32],
    pub impact_model_sha256: [u8; 32],
    pub impact_uncertainty_model_sha256: [u8; 32],
    pub outcome_model_sha256: [u8; 32],
    pub scenario_law_sha256: [u8; 32],
    /// Residual impact before this action. Failed/invalid branches must retain it.
    pub baseline_residual_impact_upper_parts_per_billion: u32,
    /// AVaR confidence level in parts per billion; e.g. 950_000_000 = 95%.
    pub avar_confidence_level_parts_per_billion: u64,
    pub scenarios: Vec<NavigationObservationOutcomeScenario>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavigationOutcomeRiskAssessment {
    pub action_id: u64,
    pub domain_fingerprint_sha256: [u8; 32],
    pub acquisition_protocol_sha256: [u8; 32],
    pub decision_partition_sha256: [u8; 32],
    pub impact_model_sha256: [u8; 32],
    pub impact_uncertainty_model_sha256: [u8; 32],
    pub outcome_model_sha256: [u8; 32],
    pub scenario_law_sha256: [u8; 32],
    pub avar_confidence_level_parts_per_billion: u64,
    pub avar_tail_probability_mass_parts_per_billion: u64,
    pub scenario_count: usize,
    pub qualified_observation_probability_mass_parts_per_billion: u64,
    pub failed_or_timed_out_probability_mass_parts_per_billion: u64,
    pub acquired_but_invalid_probability_mass_parts_per_billion: u64,
    pub unknown_or_unreconciled_probability_mass_parts_per_billion: u64,
    /// Ceiling of the scenario-law expected residual impact.
    pub mean_residual_impact_upper_parts_per_billion: u32,
    /// Upper alpha-quantile of residual impact.
    pub value_at_risk_upper_parts_per_billion: u32,
    /// Upper-tail average residual impact at the requested alpha.
    pub avar_residual_impact_upper_parts_per_billion: u32,
    /// Maximum residual impact among the supplied finite scenarios.
    pub worst_scenario_residual_impact_upper_parts_per_billion: u32,
    /// Source reliability lineage is copied from the exact admitted plan.
    pub independent_campaign_block_count: usize,
    pub reliability_confidence_error_probability: f64,
    pub reliability_probability_lower_bound: f64,
    pub reliability_probability_upper_bound: f64,
    pub reliability_source_manifest_sha256: [u8; 32],
    pub reliability_calibration_cohort_sha256: [u8; 32],
    pub reliability_bound_method_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationOutcomeRiskError {
    InvalidIdentityOrCommitment,
    MissingActionInReliablePlan(u64),
    DuplicateActionInReliablePlan(u64),
    InvalidReliabilityRecord,
    ImpactCommitmentMismatch,
    InvalidRiskPolicy,
    EmptyScenarioSet,
    InvalidScenarioIdentity(u64),
    DuplicateScenarioIdentity(u64),
    InvalidProbabilityMass(u64),
    ProbabilityMassDoesNotSumToOne,
    QualifiedProbabilityOutsideAdmittedInterval,
    InvalidResidualImpact(u64),
    NonQualifiedBranchImprovesBaseline(u64),
    InvalidRobustImpactAssessment,
    ArithmeticOverflow,
}

impl fmt::Display for NavigationOutcomeRiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentityOrCommitment => write!(f, "outcome-risk action or model commitment is missing"),
            Self::MissingActionInReliablePlan(id) => write!(f, "selected action {id} is absent from the reliability-aware Pareto frontier"),
            Self::DuplicateActionInReliablePlan(id) => write!(f, "action {id} occurs more than once in the reliability-aware Pareto frontier"),
            Self::InvalidReliabilityRecord => write!(f, "admitted acquisition-reliability record is malformed"),
            Self::ImpactCommitmentMismatch => write!(f, "scenario outcome law uses a different decision/impact model"),
            Self::InvalidRiskPolicy => write!(f, "AVaR confidence level or baseline residual impact is invalid"),
            Self::EmptyScenarioSet => write!(f, "outcome-risk scenario law must not be empty"),
            Self::InvalidScenarioIdentity(id) => write!(f, "scenario identity must be nonzero: {id}"),
            Self::DuplicateScenarioIdentity(id) => write!(f, "scenario identity is duplicated: {id}"),
            Self::InvalidProbabilityMass(id) => write!(f, "scenario {id} has zero or invalid probability mass"),
            Self::ProbabilityMassDoesNotSumToOne => write!(f, "scenario probability masses must sum exactly to 1e9 parts per billion"),
            Self::QualifiedProbabilityOutsideAdmittedInterval => write!(f, "qualified scenario mass falls outside the admitted acquisition probability interval"),
            Self::InvalidResidualImpact(id) => write!(f, "scenario {id} residual impact must be between zero and one"),
            Self::NonQualifiedBranchImprovesBaseline(id) => write!(f, "non-qualified scenario {id} cannot claim residual-impact improvement"),
            Self::InvalidRobustImpactAssessment => write!(f, "robust decision-impact assessment is incomplete or internally inconsistent"),
            Self::ArithmeticOverflow => write!(f, "scenario probability or residual-impact arithmetic overflowed"),
        }
    }
}

impl std::error::Error for NavigationOutcomeRiskError {}

/// Assess a finite action-outcome law using mean, VaR and upper-tail AVaR.
///
/// The action must occur exactly once in the reliability-admitted Pareto
/// frontier. The total mass of QualifiedObservation branches must fall inside
/// that exact action's admitted fresh-valid-qualified acquisition interval. Failed and
/// invalid branches retain the baseline residual impact. The result remains a
/// planning assessment; it does not update beliefs or navigation authority.
pub fn assess_navigation_outcome_risk(
    reliable_plan: &ReliableActiveEvidencePlan,
    robust_impact: &RobustDecisionImpactAssessment,
    contract: &NavigationOutcomeRiskContract,
) -> Result<NavigationOutcomeRiskAssessment, NavigationOutcomeRiskError> {
    if contract.action_id == 0
        || reliable_plan.expected_domain_fingerprint_sha256.iter().all(|byte| *byte == 0)
        || reliable_plan.expected_acquisition_protocol_sha256.iter().all(|byte| *byte == 0)
        || contract.decision_partition_sha256.iter().all(|byte| *byte == 0)
        || contract.impact_model_sha256.iter().all(|byte| *byte == 0)
        || contract.impact_uncertainty_model_sha256.iter().all(|byte| *byte == 0)
        || contract.outcome_model_sha256.iter().all(|byte| *byte == 0)
        || contract.scenario_law_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(NavigationOutcomeRiskError::InvalidIdentityOrCommitment);
    }
    if robust_impact.action_id != contract.action_id
        || robust_impact.decision_partition_sha256 != contract.decision_partition_sha256
        || robust_impact.impact_model_sha256 != contract.impact_model_sha256
        || robust_impact.impact_uncertainty_model_sha256
            != contract.impact_uncertainty_model_sha256
    {
        return Err(NavigationOutcomeRiskError::ImpactCommitmentMismatch);
    }
    validate_robust_impact(robust_impact)?;
    if contract.baseline_residual_impact_upper_parts_per_billion > RESIDUAL_SCALE
        || contract.avar_confidence_level_parts_per_billion == 0
        || contract.avar_confidence_level_parts_per_billion >= PROBABILITY_SCALE
    {
        return Err(NavigationOutcomeRiskError::InvalidRiskPolicy);
    }
    if contract.scenarios.is_empty() {
        return Err(NavigationOutcomeRiskError::EmptyScenarioSet);
    }

    let mut matching = reliable_plan
        .pareto_frontier
        .iter()
        .filter(|item| item.conditional.action.action_id == contract.action_id);
    let selected = matching
        .next()
        .ok_or(NavigationOutcomeRiskError::MissingActionInReliablePlan(
            contract.action_id,
        ))?;
    if matching.next().is_some() {
        return Err(NavigationOutcomeRiskError::DuplicateActionInReliablePlan(
            contract.action_id,
        ));
    }

    if selected.conditional.outcome_model_sha256 != contract.outcome_model_sha256 {
        return Err(NavigationOutcomeRiskError::ImpactCommitmentMismatch);
    }
    let reliability = &selected.reliability;
    if reliability.action_id != contract.action_id {
        return Err(NavigationOutcomeRiskError::InvalidReliabilityRecord);
    }
    validate_reliability(reliability)?;
    let mut seen_ids = BTreeSet::new();
    let mut total_probability = 0_u64;
    let mut qualified_probability = 0_u64;
    let mut failed_probability = 0_u64;
    let mut invalid_probability = 0_u64;
    let mut unknown_probability = 0_u64;
    let mut weighted_residual_sum = 0_u128;
    let mut worst_residual = 0_u32;

    for scenario in &contract.scenarios {
        if scenario.scenario_id == 0 {
            return Err(NavigationOutcomeRiskError::InvalidScenarioIdentity(0));
        }
        if !seen_ids.insert(scenario.scenario_id) {
            return Err(NavigationOutcomeRiskError::DuplicateScenarioIdentity(
                scenario.scenario_id,
            ));
        }
        if scenario.probability_mass_parts_per_billion == 0 {
            return Err(NavigationOutcomeRiskError::InvalidProbabilityMass(
                scenario.scenario_id,
            ));
        }
        if scenario.residual_impact_upper_parts_per_billion > RESIDUAL_SCALE {
            return Err(NavigationOutcomeRiskError::InvalidResidualImpact(
                scenario.scenario_id,
            ));
        }
        match scenario.outcome_kind {
            NavigationObservationOutcomeKind::QualifiedObservation => {
                qualified_probability = qualified_probability
                    .checked_add(scenario.probability_mass_parts_per_billion)
                    .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
            }
            NavigationObservationOutcomeKind::FailedOrTimedOut => {
                if scenario.residual_impact_upper_parts_per_billion
                    != contract.baseline_residual_impact_upper_parts_per_billion
                {
                    return Err(
                        NavigationOutcomeRiskError::NonQualifiedBranchImprovesBaseline(
                            scenario.scenario_id,
                        ),
                    );
                }
                failed_probability = failed_probability
                    .checked_add(scenario.probability_mass_parts_per_billion)
                    .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
            }
            NavigationObservationOutcomeKind::AcquiredButInvalid
            | NavigationObservationOutcomeKind::UnknownOrUnreconciled => {
                if scenario.residual_impact_upper_parts_per_billion
                    != contract.baseline_residual_impact_upper_parts_per_billion
                {
                    return Err(
                        NavigationOutcomeRiskError::NonQualifiedBranchImprovesBaseline(
                            scenario.scenario_id,
                        ),
                    );
                }
                if scenario.outcome_kind == NavigationObservationOutcomeKind::AcquiredButInvalid {
                    invalid_probability = invalid_probability
                        .checked_add(scenario.probability_mass_parts_per_billion)
                        .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
                } else {
                    unknown_probability = unknown_probability
                        .checked_add(scenario.probability_mass_parts_per_billion)
                        .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
                }
            }
        }
        total_probability = total_probability
            .checked_add(scenario.probability_mass_parts_per_billion)
            .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
        weighted_residual_sum = weighted_residual_sum
            .checked_add(
                u128::from(scenario.probability_mass_parts_per_billion)
                    .checked_mul(u128::from(
                        scenario.residual_impact_upper_parts_per_billion,
                    ))
                    .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?,
            )
            .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
        worst_residual = worst_residual.max(scenario.residual_impact_upper_parts_per_billion);
    }

    if total_probability != PROBABILITY_SCALE {
        return Err(NavigationOutcomeRiskError::ProbabilityMassDoesNotSumToOne);
    }
    let qualified_fraction = qualified_probability as f64 / PROBABILITY_SCALE as f64;
    if qualified_fraction < reliability.probability_lower_bound
        || qualified_fraction > reliability.probability_upper_bound
    {
        return Err(NavigationOutcomeRiskError::QualifiedProbabilityOutsideAdmittedInterval);
    }

    let mean_residual = ceil_div(
        weighted_residual_sum,
        u128::from(PROBABILITY_SCALE),
    )?;
    let tail_probability = PROBABILITY_SCALE
        .checked_sub(contract.avar_confidence_level_parts_per_billion)
        .ok_or(NavigationOutcomeRiskError::InvalidRiskPolicy)?;
    let avar_residual = upper_tail_avar(&contract.scenarios, tail_probability)?;
    let var_residual = upper_quantile_var(
        &contract.scenarios,
        contract.avar_confidence_level_parts_per_billion,
    )?;

    Ok(NavigationOutcomeRiskAssessment {
        action_id: contract.action_id,
        domain_fingerprint_sha256: reliable_plan.expected_domain_fingerprint_sha256,
        acquisition_protocol_sha256: reliable_plan.expected_acquisition_protocol_sha256,
        decision_partition_sha256: contract.decision_partition_sha256,
        impact_model_sha256: contract.impact_model_sha256,
        impact_uncertainty_model_sha256: contract.impact_uncertainty_model_sha256,
        outcome_model_sha256: contract.outcome_model_sha256,
        scenario_law_sha256: contract.scenario_law_sha256,
        avar_confidence_level_parts_per_billion: contract.avar_confidence_level_parts_per_billion,
        avar_tail_probability_mass_parts_per_billion: tail_probability,
        scenario_count: contract.scenarios.len(),
        qualified_observation_probability_mass_parts_per_billion: qualified_probability,
        failed_or_timed_out_probability_mass_parts_per_billion: failed_probability,
        acquired_but_invalid_probability_mass_parts_per_billion: invalid_probability,
        unknown_or_unreconciled_probability_mass_parts_per_billion: unknown_probability,
        mean_residual_impact_upper_parts_per_billion: mean_residual,
        value_at_risk_upper_parts_per_billion: var_residual,
        avar_residual_impact_upper_parts_per_billion: avar_residual,
        worst_scenario_residual_impact_upper_parts_per_billion: worst_residual,
        independent_campaign_block_count: reliability.independent_campaign_block_count,
        reliability_confidence_error_probability: reliability.confidence_error_probability,
        reliability_probability_lower_bound: reliability.probability_lower_bound,
        reliability_probability_upper_bound: reliability.probability_upper_bound,
        reliability_source_manifest_sha256: reliability.source_manifest_sha256,
        reliability_calibration_cohort_sha256: reliability.calibration_cohort_sha256,
        reliability_bound_method_sha256: reliability.bound_method_sha256,
    })
}

fn validate_robust_impact(
    assessment: &RobustDecisionImpactAssessment,
) -> Result<(), NavigationOutcomeRiskError> {
    if assessment.action_id == 0
        || assessment.pair_assessments.is_empty()
        || assessment.worst_case_resolved_fraction_parts_per_billion
            > assessment.best_case_resolved_fraction_parts_per_billion
        || assessment.best_case_resolved_fraction_parts_per_billion > RESIDUAL_SCALE
        || assessment.worst_case_unresolved_fraction_parts_per_billion
            != RESIDUAL_SCALE - assessment.worst_case_resolved_fraction_parts_per_billion
        || assessment.best_case_unresolved_fraction_parts_per_billion
            != RESIDUAL_SCALE - assessment.best_case_resolved_fraction_parts_per_billion
        || assessment.worst_case_resolved_fraction_parts_per_billion
            < assessment.minimum_class_pair_resolution_lower_parts_per_billion
    {
        return Err(NavigationOutcomeRiskError::InvalidRobustImpactAssessment);
    }

    let mut seen = BTreeSet::new();
    let mut classes = BTreeSet::new();
    let mut minimum_pair: Option<(u32, (u64, u64))> = None;
    for pair in &assessment.pair_assessments {
        let key = (pair.lower_decision_class_id, pair.upper_decision_class_id);
        if key.0 == 0
            || key.1 <= key.0
            || pair.weight_lower_bound == 0
            || pair.weight_upper_bound < pair.weight_lower_bound
            || pair.hypothesis_pair_count == 0
            || pair.guaranteed_separated_pair_count > pair.hypothesis_pair_count
            || pair.unresolved_pair_count
                != pair.hypothesis_pair_count - pair.guaranteed_separated_pair_count
            || pair.resolution_lower_parts_per_billion
                > pair.resolution_upper_parts_per_billion
            || pair.resolution_upper_parts_per_billion > RESIDUAL_SCALE
        {
            return Err(NavigationOutcomeRiskError::InvalidRobustImpactAssessment);
        }
        let numerator = (pair.guaranteed_separated_pair_count as u128)
            .checked_mul(u128::from(PROBABILITY_SCALE))
            .ok_or(NavigationOutcomeRiskError::InvalidRobustImpactAssessment)?;
        let denominator = pair.hypothesis_pair_count as u128;
        let expected_lower = numerator / denominator;
        let expected_upper = numerator
            .checked_add(denominator - 1)
            .ok_or(NavigationOutcomeRiskError::InvalidRobustImpactAssessment)?
            / denominator;
        if u32::try_from(expected_lower).ok() != Some(pair.resolution_lower_parts_per_billion)
            || u32::try_from(expected_upper).ok() != Some(pair.resolution_upper_parts_per_billion)
        {
            return Err(NavigationOutcomeRiskError::InvalidRobustImpactAssessment);
        }
        if !seen.insert(key) {
            return Err(NavigationOutcomeRiskError::InvalidRobustImpactAssessment);
        }
        classes.insert(key.0);
        classes.insert(key.1);
        let candidate = (pair.resolution_lower_parts_per_billion, key);
        minimum_pair = Some(minimum_pair.map_or(candidate, |current| current.min(candidate)));
    }

    let class_count = classes.len();
    let expected_pairs = class_count
        .checked_mul(class_count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(NavigationOutcomeRiskError::InvalidRobustImpactAssessment)?;
    if expected_pairs != assessment.pair_assessments.len()
        || minimum_pair != Some((
            assessment.minimum_class_pair_resolution_lower_parts_per_billion,
            assessment.minimum_class_pair_resolution_pair,
        ))
    {
        return Err(NavigationOutcomeRiskError::InvalidRobustImpactAssessment);
    }
    let ordered: Vec<u64> = classes.into_iter().collect();
    for index in 0..ordered.len() {
        for other in (index + 1)..ordered.len() {
            if !seen.contains(&(ordered[index], ordered[other])) {
                return Err(NavigationOutcomeRiskError::InvalidRobustImpactAssessment);
            }
        }
    }
    Ok(())
}

fn validate_reliability(
    reliability: &AdmittedAcquisitionReliability,
) -> Result<(), NavigationOutcomeRiskError> {
    let total = reliability
        .valid_qualified_success_block_count
        .checked_add(reliability.non_success_block_count)
        .ok_or(NavigationOutcomeRiskError::InvalidReliabilityRecord)?;
    let non_success = reliability
        .failed_or_timed_out_block_count
        .checked_add(reliability.acquired_but_invalid_block_count)
        .and_then(|value| value.checked_add(reliability.unresolved_block_count))
        .ok_or(NavigationOutcomeRiskError::InvalidReliabilityRecord)?;
    let empirical = if reliability.independent_campaign_block_count == 0 {
        f64::NAN
    } else {
        reliability.valid_qualified_success_block_count as f64
            / reliability.independent_campaign_block_count as f64
    };
    if reliability.action_id == 0
        || reliability.independent_campaign_block_count == 0
        || total != reliability.independent_campaign_block_count
        || non_success != reliability.non_success_block_count
        || !reliability.probability_lower_bound.is_finite()
        || !reliability.probability_upper_bound.is_finite()
        || reliability.probability_lower_bound < 0.0
        || reliability.probability_upper_bound > 1.0
        || reliability.probability_lower_bound > reliability.probability_upper_bound
        || !reliability.confidence_error_probability.is_finite()
        || reliability.confidence_error_probability <= 0.0
        || reliability.confidence_error_probability >= 1.0
        || !reliability.empirical_valid_success_fraction.is_finite()
        || (reliability.empirical_valid_success_fraction - empirical).abs() > f64::EPSILON
        || reliability.probability_lower_bound > empirical
        || reliability.probability_upper_bound < empirical
        || reliability.source_manifest_sha256.iter().all(|byte| *byte == 0)
        || reliability.calibration_cohort_sha256.iter().all(|byte| *byte == 0)
        || reliability.bound_method_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(NavigationOutcomeRiskError::InvalidReliabilityRecord);
    }
    Ok(())
}

fn ceil_div(
    numerator: u128,
    denominator: u128,
) -> Result<u32, NavigationOutcomeRiskError> {
    if denominator == 0 {
        return Err(NavigationOutcomeRiskError::ArithmeticOverflow);
    }
    let rounded = numerator
        .checked_add(denominator - 1)
        .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?
        / denominator;
    u32::try_from(rounded).map_err(|_| NavigationOutcomeRiskError::ArithmeticOverflow)
}

fn upper_tail_avar(
    scenarios: &[NavigationObservationOutcomeScenario],
    tail_probability_parts_per_billion: u64,
) -> Result<u32, NavigationOutcomeRiskError> {
    if tail_probability_parts_per_billion == 0
        || tail_probability_parts_per_billion > PROBABILITY_SCALE
    {
        return Err(NavigationOutcomeRiskError::InvalidRiskPolicy);
    }
    let mut ordered: Vec<&NavigationObservationOutcomeScenario> = scenarios.iter().collect();
    ordered.sort_by(|left, right| {
        right
            .residual_impact_upper_parts_per_billion
            .cmp(&left.residual_impact_upper_parts_per_billion)
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });

    let mut remaining = tail_probability_parts_per_billion;
    let mut weighted_tail = 0_u128;
    for scenario in ordered {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(scenario.probability_mass_parts_per_billion);
        weighted_tail = weighted_tail
            .checked_add(
                u128::from(take)
                    .checked_mul(u128::from(
                        scenario.residual_impact_upper_parts_per_billion,
                    ))
                    .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?,
            )
            .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
        remaining = remaining
            .checked_sub(take)
            .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
    }
    if remaining != 0 {
        return Err(NavigationOutcomeRiskError::ProbabilityMassDoesNotSumToOne);
    }
    ceil_div(weighted_tail, u128::from(tail_probability_parts_per_billion))
}

fn upper_quantile_var(
    scenarios: &[NavigationObservationOutcomeScenario],
    confidence_level_parts_per_billion: u64,
) -> Result<u32, NavigationOutcomeRiskError> {
    if confidence_level_parts_per_billion == 0
        || confidence_level_parts_per_billion >= PROBABILITY_SCALE
    {
        return Err(NavigationOutcomeRiskError::InvalidRiskPolicy);
    }
    let mut ordered: Vec<&NavigationObservationOutcomeScenario> = scenarios.iter().collect();
    ordered.sort_by(|left, right| {
        left.residual_impact_upper_parts_per_billion
            .cmp(&right.residual_impact_upper_parts_per_billion)
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });
    let mut cumulative = 0_u64;
    for scenario in ordered {
        cumulative = cumulative
            .checked_add(scenario.probability_mass_parts_per_billion)
            .ok_or(NavigationOutcomeRiskError::ArithmeticOverflow)?;
        if cumulative >= confidence_level_parts_per_billion {
            return Ok(scenario.residual_impact_upper_parts_per_billion);
        }
    }
    Err(NavigationOutcomeRiskError::ProbabilityMassDoesNotSumToOne)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_acquisition::{
        EvidenceActionAssessment, EvidenceActionKind,
    };
    use crate::active_evidence_decision_planner::EvidenceActionDecisionAssessment;
    use crate::active_evidence_reliability_planner::{
        ReliableEvidenceActionAssessment, ReliableEvidencePlanReason,
    };

    fn reliable_plan(
        action_id: u64,
        lower: f64,
        upper: f64,
    ) -> ReliableActiveEvidencePlan {
        ReliableActiveEvidencePlan {
            reason: ReliableEvidencePlanReason::BestExpectedDecisionResolution,
            expected_domain_fingerprint_sha256: [0x11; 32],
            expected_acquisition_protocol_sha256: [0x22; 32],
            recommended_action_id: Some(action_id),
            pareto_frontier: vec![ReliableEvidenceActionAssessment {
                conditional: EvidenceActionDecisionAssessment {
                    action: EvidenceActionAssessment {
                        action_id,
                        kind: EvidenceActionKind::AcquireCir,
                        dependency_group_id: 9,
                        conditional_post_action_protection_level_upper_m: 0.5,
                        guaranteed_protection_level_reduction_m: 0.5,
                        worst_case_latency_ns: 100,
                        worst_case_energy_uj: 200,
                        reaches_target_if_acquisition_qualifies: false,
                    },
                    conditional_protection_level_delta_m: 0.5,
                    decision_class_count: 2,
                    same_decision_hypothesis_pair_count: 0,
                    decision_incompatible_hypothesis_pair_count: 2,
                    distinguishable_decision_pair_count: 1,
                    ambiguous_decision_pair_count: 1,
                    disposition: crate::decision_relevant_discrimination::DecisionProjectionDisposition::DecisionAmbiguityRemains,
                    minimum_positive_normalized_gap: Some(0.25),
                    outcome_model_sha256: [0x21; 32],
                    normalization_sha256: [0x31; 32],
                    separated_decision_hypothesis_pairs: vec![(1, 2)],
                    unresolved_decision_hypothesis_pairs: vec![(1, 3)],
                },
                reliability: AdmittedAcquisitionReliability {
                    action_id,
                    probability_lower_bound: lower,
                    probability_upper_bound: upper,
                    confidence_error_probability: 0.05,
                    independent_campaign_block_count: 10,
                    valid_qualified_success_block_count: 10,
                    failed_or_timed_out_block_count: 0,
                    acquired_but_invalid_block_count: 0,
                    non_success_block_count: 0,
                    unresolved_block_count: 0,
                    empirical_valid_success_fraction: 1.0,
                    source_manifest_sha256: [0x61; 32],
                    calibration_cohort_sha256: [0x62; 32],
                    bound_method_sha256: [0x63; 32],
                },
                expected_decision_pair_yield_lower_bound: lower * 2.0,
                expected_decision_pair_yield_upper_bound: upper * 2.0,
                expected_protection_level_reduction_lower_bound_m: lower * 0.5,
                expected_protection_level_reduction_upper_bound_m: upper * 0.5,
            }],
            reliability_rejections: Vec::new(),
            hypothesis_inventory_sha256: Some([0x41; 32]),
            decision_partition_sha256: Some([0x44; 32]),
            decision_assignments: None,
            conditional_target_reached: false,
            authority_transition_not_authorized_by_plan: true,
        }
    }

    fn robust_impact(action_id: u64) -> RobustDecisionImpactAssessment {
        RobustDecisionImpactAssessment {
            action_id,
            decision_partition_sha256: [0x44; 32],
            impact_model_sha256: [0x55; 32],
            impact_uncertainty_model_sha256: [0x66; 32],
            worst_case_resolved_fraction_parts_per_billion: 250_000_000,
            best_case_resolved_fraction_parts_per_billion: 250_000_000,
            worst_case_unresolved_fraction_parts_per_billion: 750_000_000,
            best_case_unresolved_fraction_parts_per_billion: 750_000_000,
            minimum_class_pair_resolution_lower_parts_per_billion: 250_000_000,
            minimum_class_pair_resolution_pair: (1, 2),
            pair_assessments: vec![RobustDecisionClassPairAssessment {
                lower_decision_class_id: 1,
                upper_decision_class_id: 2,
                weight_lower_bound: 1,
                weight_upper_bound: 2,
                hypothesis_pair_count: 4,
                guaranteed_separated_pair_count: 1,
                unresolved_pair_count: 3,
                resolution_lower_parts_per_billion: 250_000_000,
                resolution_upper_parts_per_billion: 250_000_000,
            }],
        }
    }

    fn scenario(
        id: u64,
        mass: u64,
        kind: NavigationObservationOutcomeKind,
        residual: u32,
    ) -> NavigationObservationOutcomeScenario {
        NavigationObservationOutcomeScenario {
            scenario_id: id,
            probability_mass_parts_per_billion: mass,
            outcome_kind: kind,
            residual_impact_upper_parts_per_billion: residual,
        }
    }

    fn contract(
        alpha: u64,
        baseline: u32,
        scenarios: Vec<NavigationObservationOutcomeScenario>,
    ) -> NavigationOutcomeRiskContract {
        NavigationOutcomeRiskContract {
            action_id: 7,
            decision_partition_sha256: [0x44; 32],
            impact_model_sha256: [0x55; 32],
            impact_uncertainty_model_sha256: [0x66; 32],
            outcome_model_sha256: [0x77; 32],
            scenario_law_sha256: [0x88; 32],
            baseline_residual_impact_upper_parts_per_billion: baseline,
            avar_confidence_level_parts_per_billion: alpha,
            scenarios,
        }
    }

    #[test]
    fn avar_exposes_rare_high_residual_hidden_by_a_low_mean() {
        let scenarios = vec![
            scenario(1, 990_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
            scenario(2, 10_000_000, NavigationObservationOutcomeKind::FailedOrTimedOut, RESIDUAL_SCALE),
        ];
        let result = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(990_000_000, RESIDUAL_SCALE, scenarios),
        )
        .unwrap();
        assert_eq!(result.mean_residual_impact_upper_parts_per_billion, 10_000_000);
        assert_eq!(result.value_at_risk_upper_parts_per_billion, 0);
        assert_eq!(result.avar_residual_impact_upper_parts_per_billion, RESIDUAL_SCALE);
        assert_eq!(result.worst_scenario_residual_impact_upper_parts_per_billion, RESIDUAL_SCALE);
        assert_eq!(result.failed_or_timed_out_probability_mass_parts_per_billion, 10_000_000);
    }

    #[test]
    fn avar_uses_fraction_of_boundary_scenario_exactly() {
        let scenarios = vec![
            scenario(1, 900_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
            scenario(2, 80_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 500_000_000),
            scenario(3, 20_000_000, NavigationObservationOutcomeKind::QualifiedObservation, RESIDUAL_SCALE),
        ];
        let result = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(950_000_000, 0, scenarios),
        )
        .unwrap();
        assert_eq!(result.mean_residual_impact_upper_parts_per_billion, 60_000_000);
        assert_eq!(result.value_at_risk_upper_parts_per_billion, 500_000_000);
        assert_eq!(result.avar_residual_impact_upper_parts_per_billion, 700_000_000);
    }

    #[test]
    fn avar_is_monotone_in_confidence_level() {
        let scenarios = vec![
            scenario(1, 700_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 100_000_000),
            scenario(2, 200_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 400_000_000),
            scenario(3, 100_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 900_000_000),
        ];
        let low = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(800_000_000, 0, scenarios.clone()),
        )
        .unwrap();
        let high = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(990_000_000, 0, scenarios),
        )
        .unwrap();
        assert!(low.avar_residual_impact_upper_parts_per_billion <= high.avar_residual_impact_upper_parts_per_billion);
    }

    #[test]
    fn failed_and_invalid_outcomes_cannot_claim_improvement() {
        let scenarios = vec![
            scenario(1, 900_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
            scenario(2, 100_000_000, NavigationObservationOutcomeKind::AcquiredButInvalid, 1),
        ];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.9, 1.0),
                &robust_impact(7),
                &contract(950_000_000, RESIDUAL_SCALE, scenarios),
            ),
            Err(NavigationOutcomeRiskError::NonQualifiedBranchImprovesBaseline(2))
        );
    }

    #[test]
    fn unknown_outcome_retains_baseline_and_remains_visible_as_a_blocking_branch() {
        let scenarios = vec![
            scenario(1, 990_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 200_000_000),
            scenario(2, 10_000_000, NavigationObservationOutcomeKind::UnknownOrUnreconciled, RESIDUAL_SCALE),
        ];
        let result = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(990_000_000, RESIDUAL_SCALE, scenarios),
        )
        .unwrap();
        assert_eq!(result.unknown_or_unreconciled_probability_mass_parts_per_billion, 10_000_000);
        assert_eq!(result.mean_residual_impact_upper_parts_per_billion, 208_000_000);
        assert_eq!(result.avar_residual_impact_upper_parts_per_billion, RESIDUAL_SCALE);
    }

    #[test]
    fn scenario_mass_and_reliability_interval_are_both_enforced() {
        let not_normalized = vec![
            scenario(1, 800_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
        ];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.8, 1.0),
                &robust_impact(7),
                &contract(950_000_000, 0, not_normalized),
            ),
            Err(NavigationOutcomeRiskError::ProbabilityMassDoesNotSumToOne)
        );

        let outside_reliability = vec![
            scenario(1, 800_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
            scenario(2, 200_000_000, NavigationObservationOutcomeKind::FailedOrTimedOut, 0),
        ];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.9, 1.0),
                &robust_impact(7),
                &contract(950_000_000, 0, outside_reliability),
            ),
            Err(NavigationOutcomeRiskError::QualifiedProbabilityOutsideAdmittedInterval)
        );
    }

    #[test]
    fn action_and_impact_commitments_must_match() {
        let scenarios = vec![scenario(
            1,
            PROBABILITY_SCALE,
            NavigationObservationOutcomeKind::QualifiedObservation,
            250_000_000,
        )];
        let mut law = contract(950_000_000, 0, scenarios);
        law.impact_model_sha256 = [0x99; 32];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.99, 1.0),
                &robust_impact(7),
                &law,
            ),
            Err(NavigationOutcomeRiskError::ImpactCommitmentMismatch)
        );

        let law = contract(
            950_000_000,
            0,
            vec![scenario(
                1,
                PROBABILITY_SCALE,
                NavigationObservationOutcomeKind::QualifiedObservation,
                250_000_000,
            )],
        );
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(8, 1.0, 1.0),
                &robust_impact(7),
                &law,
            ),
            Err(NavigationOutcomeRiskError::MissingActionInReliablePlan(7))
        );
    }

    #[test]
    fn non_recommended_admitted_frontier_action_can_be_risk_assessed() {
        let scenarios = vec![scenario(
            1,
            PROBABILITY_SCALE,
            NavigationObservationOutcomeKind::QualifiedObservation,
            250_000_000,
        )];
        let law = contract(950_000_000, 0, scenarios);
        let mut plan = reliable_plan(8, 0.99, 1.0);
        plan.pareto_frontier.push(
            reliable_plan(7, 0.99, 1.0).pareto_frontier.remove(0)
        );
        let result = assess_navigation_outcome_risk(
            &plan,
            &robust_impact(7),
            &law,
        )
        .unwrap();
        assert_eq!(result.action_id, 7);
        assert_eq!(plan.recommended_action_id, Some(8));
    }

    #[test]
    fn outcome_model_and_reliability_action_are_bound_to_selected_plan_entry() {
        let scenarios = vec![scenario(
            1,
            PROBABILITY_SCALE,
            NavigationObservationOutcomeKind::QualifiedObservation,
            250_000_000,
        )];
        let law = contract(950_000_000, 0, scenarios);
        let mut plan = reliable_plan(7, 0.99, 1.0);
        plan.pareto_frontier[0].conditional.outcome_model_sha256 = [0x99; 32];
        assert_eq!(
            assess_navigation_outcome_risk(&plan, &robust_impact(7), &law),
            Err(NavigationOutcomeRiskError::ImpactCommitmentMismatch)
        );

        let mut plan = reliable_plan(7, 0.99, 1.0);
        plan.pareto_frontier[0].reliability.action_id = 8;
        assert_eq!(
            assess_navigation_outcome_risk(&plan, &robust_impact(7), &law),
            Err(NavigationOutcomeRiskError::InvalidReliabilityRecord)
        );
    }

    #[test]
    fn scenario_permutation_does_not_change_risk_assessment() {
        let scenarios = vec![
            scenario(3, 100_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 900_000_000),
            scenario(1, 700_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 100_000_000),
            scenario(2, 200_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 400_000_000),
        ];
        let first = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(950_000_000, 0, scenarios.clone()),
        )
        .unwrap();
        let mut reversed = scenarios;
        reversed.reverse();
        let second = assess_navigation_outcome_risk(
            &reliable_plan(7, 0.99, 1.0),
            &robust_impact(7),
            &contract(950_000_000, 0, reversed),
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_ids_zero_mass_and_missing_commitments_reject() {
        let repeated = vec![
            scenario(1, 500_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
            scenario(1, 500_000_000, NavigationObservationOutcomeKind::QualifiedObservation, 0),
        ];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.99, 1.0),
                &robust_impact(7),
                &contract(950_000_000, 0, repeated),
            ),
            Err(NavigationOutcomeRiskError::DuplicateScenarioIdentity(1))
        );

        let zero_mass = vec![scenario(
            1,
            0,
            NavigationObservationOutcomeKind::QualifiedObservation,
            0,
        )];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.99, 1.0),
                &robust_impact(7),
                &contract(950_000_000, 0, zero_mass),
            ),
            Err(NavigationOutcomeRiskError::InvalidProbabilityMass(1))
        );

        let mut missing = contract(
            950_000_000,
            0,
            vec![scenario(
                1,
                PROBABILITY_SCALE,
                NavigationObservationOutcomeKind::QualifiedObservation,
                0,
            )],
        );
        missing.scenario_law_sha256 = [0; 32];
        assert_eq!(
            assess_navigation_outcome_risk(
                &reliable_plan(7, 0.99, 1.0),
                &robust_impact(7),
                &missing,
            ),
            Err(NavigationOutcomeRiskError::InvalidIdentityOrCommitment)
        );
    }
}
