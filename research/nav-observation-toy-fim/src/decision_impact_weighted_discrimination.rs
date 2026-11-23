//! Consequence-weighted projection of decision-relevant UWB discrimination.
//!
//! This module addresses a limitation of unweighted pair counting: not every
//! pair of distinct navigation decisions has the same consequence. A frozen,
//! externally owned impact model assigns a positive relative weight to every
//! unordered decision-class pair. The evaluator then computes the fraction of
//! that impact weight resolved by an action, normalizing within each class
//! pair so a class pair with many nuisance hypotheses does not dominate merely
//! because it has more pair instances.
//!
//! The output is a consequence-aware discrimination proxy, not Bayes risk,
//! expected loss, minimax regret, a posterior probability, or a safety case.
//! The weights are symmetric, dimensionless, externally supplied and bound to
//! the exact decision partition. Directed/asymmetric loss requires a richer
//! state-to-action loss model and is intentionally not inferred here.
//!
//! Acquisition reliability is composed separately from the existing #7082
//! admission result. The resulting expectation assumes no incremental
//! discrimination benefit when a valid, qualified observation is not obtained.
//! Neither function updates a posterior, a protection level or authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::active_evidence_reliability::AdmittedAcquisitionReliability;
use crate::active_evidence_reliability_planner::ReliableActiveEvidencePlan;
use crate::decision_relevant_discrimination::{
    DecisionProjectionDisposition, DecisionRelevantDiscriminationReport,
};

const SHA256_BYTES: usize = 32;
const RESOLUTION_SCALE: u128 = 1_000_000_000;

/// Positive, dimensionless relative impact weight for one unordered pair of
/// distinct navigation decision classes.
///
/// Class IDs are profile-local. The pair must be canonical
/// (lower_decision_class_id < upper_decision_class_id). The weight is not a
/// probability, a calibrated monetary value, or a claim about physical harm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionClassPairImpactWeight {
    pub lower_decision_class_id: u64,
    pub upper_decision_class_id: u64,
    pub relative_impact_weight: u64,
}

/// Frozen, complete symmetric impact-weight matrix for one decision partition.
///
/// Both digests are caller-supplied commitments. This module checks presence
/// and equality but does not hash, authenticate, or validate the authority of
/// the source model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionImpactWeightPolicy {
    pub expected_decision_partition_sha256: [u8; SHA256_BYTES],
    pub impact_model_sha256: [u8; SHA256_BYTES],
    pub class_pair_weights: Vec<DecisionClassPairImpactWeight>,
}

/// Per-class-pair accounting after normalizing within the class pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionClassPairImpactAssessment {
    pub lower_decision_class_id: u64,
    pub upper_decision_class_id: u64,
    pub relative_impact_weight: u64,
    pub hypothesis_pair_count: usize,
    pub guaranteed_separated_pair_count: usize,
    pub unresolved_pair_count: usize,
    /// Floor of separated / total, scaled by 1e9. Flooring is conservative.
    pub resolved_fraction_parts_per_billion: u32,
}

/// Consequence-weighted, conditional discrimination score for one action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionImpactWeightedAssessment {
    pub action_id: u64,
    pub decision_partition_sha256: [u8; SHA256_BYTES],
    pub impact_model_sha256: [u8; SHA256_BYTES],
    pub decision_class_pair_count: usize,
    pub total_cross_decision_hypothesis_pair_count: usize,
    pub total_relative_impact_weight: u128,
    /// Weighted mean resolution fraction, in parts per billion.
    pub resolved_impact_fraction_parts_per_billion: u32,
    /// Conservative complement of the weighted resolution score.
    pub unresolved_impact_fraction_parts_per_billion: u32,
    pub class_pair_assessments: Vec<DecisionClassPairImpactAssessment>,
}

/// Reliability-adjusted planning proxy. The interval is conditional on the
/// externally admitted valid-acquisition probability and the explicit
/// zero-benefit-on-invalid-or-failed-acquisition assumption.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReliableDecisionImpactAssessment {
    pub action_id: u64,
    pub domain_fingerprint_sha256: [u8; SHA256_BYTES],
    pub acquisition_protocol_sha256: [u8; SHA256_BYTES],
    pub decision_partition_sha256: [u8; SHA256_BYTES],
    pub impact_model_sha256: [u8; SHA256_BYTES],
    pub admitted_acquisition_probability_lower_bound: f64,
    pub admitted_acquisition_probability_upper_bound: f64,
    /// Conditional score multiplied by the lower acquisition-probability bound.
    pub expected_resolved_impact_fraction_lower_bound: f64,
    /// Conditional score multiplied by the upper acquisition-probability bound.
    pub expected_resolved_impact_fraction_upper_bound: f64,
    /// 1 - upper expected resolution; lower endpoint of residual ambiguity.
    pub expected_residual_impact_fraction_lower_bound: f64,
    /// 1 - lower expected resolution; upper endpoint of residual ambiguity.
    pub expected_residual_impact_fraction_upper_bound: f64,
    pub independent_campaign_block_count: usize,
    pub confidence_error_probability: f64,
    pub source_manifest_sha256: [u8; SHA256_BYTES],
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub bound_method_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionImpactError {
    InvalidReport,
    MissingDecisionPartitionCommitment,
    DecisionPartitionMismatch,
    MissingImpactModelCommitment,
    InvalidDecisionAssignment,
    DuplicateDecisionAssignment(u64),
    InvalidDecisionPair,
    DuplicateDecisionPair,
    MissingDecisionClassPairWeight(u64, u64),
    UnexpectedDecisionClassPairWeight(u64, u64),
    DuplicateDecisionClassPairWeight(u64, u64),
    ZeroDecisionClassPairWeight(u64, u64),
    NonCanonicalDecisionClassPair(u64, u64),
    NoDecisionPairUniverse,
    ArithmeticOverflow,
    ActionIdentityMismatch,
    InvalidReliabilityContract,
}

impl fmt::Display for DecisionImpactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReport => write!(f, "decision-relevant discrimination report is incomplete or inconsistent"),
            Self::MissingDecisionPartitionCommitment => write!(f, "decision partition commitment must be nonzero"),
            Self::DecisionPartitionMismatch => write!(f, "impact policy and discrimination report use different decision partitions"),
            Self::MissingImpactModelCommitment => write!(f, "impact model commitment must be nonzero"),
            Self::InvalidDecisionAssignment => write!(f, "decision assignment contains a zero identity"),
            Self::DuplicateDecisionAssignment(id) => write!(f, "hypothesis {id} has duplicate decision assignments"),
            Self::InvalidDecisionPair => write!(f, "decision-relevant hypothesis pair is malformed or inconsistent with assignments"),
            Self::DuplicateDecisionPair => write!(f, "decision-relevant hypothesis pair is duplicated"),
            Self::MissingDecisionClassPairWeight(a, b) => write!(f, "impact weight is missing for decision-class pair ({a}, {b})"),
            Self::UnexpectedDecisionClassPairWeight(a, b) => write!(f, "impact model contains an unreferenced decision-class pair ({a}, {b})"),
            Self::DuplicateDecisionClassPairWeight(a, b) => write!(f, "impact model duplicates decision-class pair ({a}, {b})"),
            Self::ZeroDecisionClassPairWeight(a, b) => write!(f, "impact weight for decision-class pair ({a}, {b}) must be positive"),
            Self::NonCanonicalDecisionClassPair(a, b) => write!(f, "decision-class pair ({a}, {b}) is not in canonical ascending order"),
            Self::NoDecisionPairUniverse => write!(f, "at least two decision classes are required for impact-weighted discrimination"),
            Self::ArithmeticOverflow => write!(f, "decision-impact pair accounting or fixed-point arithmetic overflowed"),
            Self::ActionIdentityMismatch => write!(f, "reliability record action ID differs from the assessed action"),
            Self::InvalidReliabilityContract => write!(f, "admitted acquisition reliability record is malformed"),
        }
    }
}

impl std::error::Error for DecisionImpactError {}

/// Validate and score one complete decision-relevant discrimination report.
///
/// Every unordered decision-class pair must have exactly one positive impact
/// weight. The fraction of separable hypothesis pairs is calculated within
/// each class pair, then averaged by the externally supplied impact weights.
/// This avoids allowing a class pair to dominate solely because it contains
/// more hypotheses. Pair separations and the decision partition are treated as
/// conditional model inputs; neither is independently regenerated here.
pub fn assess_decision_impact_weighted_discrimination(
    report: &DecisionRelevantDiscriminationReport,
    policy: &DecisionImpactWeightPolicy,
) -> Result<DecisionImpactWeightedAssessment, DecisionImpactError> {
    if report.action_id == 0
        || report.hypothesis_count == 0
        || report.hypothesis_inventory_sha256.iter().all(|byte| *byte == 0)
        || report.outcome_model_sha256.iter().all(|byte| *byte == 0)
        || report.normalization_sha256.iter().all(|byte| *byte == 0)
        || !report.conditional_on_successful_valid_observation
    {
        return Err(DecisionImpactError::InvalidReport);
    }
    if report.decision_partition_sha256.iter().all(|byte| *byte == 0)
        || policy.expected_decision_partition_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(DecisionImpactError::MissingDecisionPartitionCommitment);
    }
    if report.decision_partition_sha256 != policy.expected_decision_partition_sha256 {
        return Err(DecisionImpactError::DecisionPartitionMismatch);
    }
    if policy.impact_model_sha256.iter().all(|byte| *byte == 0) {
        return Err(DecisionImpactError::MissingImpactModelCommitment);
    }

    let total_hypothesis_pairs = report
        .hypothesis_count
        .checked_mul(report.hypothesis_count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(DecisionImpactError::ArithmeticOverflow)?;

    if report.decision_assignments.len() != report.hypothesis_count {
        return Err(DecisionImpactError::InvalidReport);
    }

    let mut decision_by_hypothesis = BTreeMap::new();
    let mut class_sizes: BTreeMap<u64, usize> = BTreeMap::new();
    for assignment in &report.decision_assignments {
        if assignment.hypothesis_id == 0 || assignment.decision_class_id == 0 {
            return Err(DecisionImpactError::InvalidDecisionAssignment);
        }
        if decision_by_hypothesis
            .insert(assignment.hypothesis_id, assignment.decision_class_id)
            .is_some()
        {
            return Err(DecisionImpactError::DuplicateDecisionAssignment(
                assignment.hypothesis_id,
            ));
        }
        let size = class_sizes.entry(assignment.decision_class_id).or_default();
        *size = size.checked_add(1).ok_or(DecisionImpactError::ArithmeticOverflow)?;
    }

    let class_ids: Vec<u64> = class_sizes.keys().copied().collect();
    if class_ids.len() < 2 {
        return Err(DecisionImpactError::NoDecisionPairUniverse);
    }
    let expected_class_pair_count = class_ids
        .len()
        .checked_mul(class_ids.len().saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(DecisionImpactError::ArithmeticOverflow)?;
    if report.decision_class_count != class_ids.len()
        || report.decision_class_pair_count != expected_class_pair_count
    {
        return Err(DecisionImpactError::InvalidReport);
    }

    let mut same_class_pair_count = 0_usize;
    for count in class_sizes.values() {
        let within = count
            .checked_mul(count.saturating_sub(1))
            .and_then(|value| value.checked_div(2))
            .ok_or(DecisionImpactError::ArithmeticOverflow)?;
        same_class_pair_count = same_class_pair_count
            .checked_add(within)
            .ok_or(DecisionImpactError::ArithmeticOverflow)?;
    }
    let expected_cross_class_pair_count = total_hypothesis_pairs
        .checked_sub(same_class_pair_count)
        .ok_or(DecisionImpactError::InvalidReport)?;

    if report.same_decision_hypothesis_pair_count != same_class_pair_count
        || report.decision_incompatible_hypothesis_pair_count != expected_cross_class_pair_count
        || report.decision_pair_separations.len() != expected_cross_class_pair_count
    {
        return Err(DecisionImpactError::InvalidReport);
    }

    let mut pair_counts: BTreeMap<(u64, u64), (usize, usize)> = BTreeMap::new();
    let mut seen_pairs = BTreeSet::new();
    let mut separated_count = 0_usize;
    let mut ambiguous_count = 0_usize;

    for pair in &report.decision_pair_separations {
        if pair.lower_hypothesis_id == 0
            || pair.upper_hypothesis_id <= pair.lower_hypothesis_id
            || !pair.normalized_outcome_gap.is_finite()
            || pair.normalized_outcome_gap < 0.0
            || pair.guaranteed_distinguishable != (pair.normalized_outcome_gap > 0.0)
        {
            return Err(DecisionImpactError::InvalidDecisionPair);
        }
        if !seen_pairs.insert((pair.lower_hypothesis_id, pair.upper_hypothesis_id)) {
            return Err(DecisionImpactError::DuplicateDecisionPair);
        }

        let lower_class = *decision_by_hypothesis
            .get(&pair.lower_hypothesis_id)
            .ok_or(DecisionImpactError::InvalidDecisionPair)?;
        let upper_class = *decision_by_hypothesis
            .get(&pair.upper_hypothesis_id)
            .ok_or(DecisionImpactError::InvalidDecisionPair)?;
        if lower_class == upper_class
            || lower_class != pair.lower_decision_class_id
            || upper_class != pair.upper_decision_class_id
        {
            return Err(DecisionImpactError::InvalidDecisionPair);
        }

        let class_pair = canonical_class_pair(lower_class, upper_class);
        let counts = pair_counts.entry(class_pair).or_default();
        counts.0 = counts.0.checked_add(1).ok_or(DecisionImpactError::ArithmeticOverflow)?;
        if pair.guaranteed_distinguishable {
            counts.1 = counts.1.checked_add(1).ok_or(DecisionImpactError::ArithmeticOverflow)?;
            separated_count = separated_count.checked_add(1).ok_or(DecisionImpactError::ArithmeticOverflow)?;
        } else {
            ambiguous_count = ambiguous_count.checked_add(1).ok_or(DecisionImpactError::ArithmeticOverflow)?;
        }
    }

    if separated_count != report.distinguishable_decision_pair_count
        || ambiguous_count != report.ambiguous_decision_pair_count
        || separated_count.checked_add(ambiguous_count) != Some(expected_cross_class_pair_count)
    {
        return Err(DecisionImpactError::InvalidReport);
    }

    let expected_disposition = if expected_cross_class_pair_count == 0 {
        DecisionProjectionDisposition::NoDecisionIncompatiblePairs
    } else if ambiguous_count > 0 {
        DecisionProjectionDisposition::DecisionAmbiguityRemains
    } else {
        DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
    };
    if report.disposition != expected_disposition {
        return Err(DecisionImpactError::InvalidReport);
    }

    let mut supplied_weights = BTreeMap::new();
    for item in &policy.class_pair_weights {
        let (a, b) = (item.lower_decision_class_id, item.upper_decision_class_id);
        if a == 0 || b == 0 || a == b {
            return Err(DecisionImpactError::InvalidDecisionAssignment);
        }
        if a > b {
            return Err(DecisionImpactError::NonCanonicalDecisionClassPair(a, b));
        }
        if item.relative_impact_weight == 0 {
            return Err(DecisionImpactError::ZeroDecisionClassPairWeight(a, b));
        }
        if supplied_weights
            .insert((a, b), item.relative_impact_weight)
            .is_some()
        {
            return Err(DecisionImpactError::DuplicateDecisionClassPairWeight(a, b));
        }
    }

    for index in 0..class_ids.len() {
        for other_index in (index + 1)..class_ids.len() {
            let key = (class_ids[index], class_ids[other_index]);
            if !pair_counts.contains_key(&key) {
                return Err(DecisionImpactError::InvalidReport);
            }
            if !supplied_weights.contains_key(&key) {
                return Err(DecisionImpactError::MissingDecisionClassPairWeight(key.0, key.1));
            }
        }
    }

    let expected_weight_count = expected_class_pair_count;
    if supplied_weights.len() < expected_weight_count {
        return Err(DecisionImpactError::InvalidReport);
    }
    for key in supplied_weights.keys() {
        if !pair_counts.contains_key(key) {
            return Err(DecisionImpactError::UnexpectedDecisionClassPairWeight(key.0, key.1));
        }
    }

    let mut total_weight = 0_u128;
    for weight in supplied_weights.values() {
        total_weight = total_weight
            .checked_add(u128::from(*weight))
            .ok_or(DecisionImpactError::ArithmeticOverflow)?;
    }
    if total_weight == 0 {
        return Err(DecisionImpactError::InvalidReport);
    }

    let mut weighted_resolution_numerator = 0_u128;
    let mut class_pair_assessments = Vec::with_capacity(expected_class_pair_count);
    for key in pair_counts.keys() {
        let (total, separated) = pair_counts[key];
        let weight = supplied_weights[key];
        let resolved_ppb = (separated as u128)
            .checked_mul(RESOLUTION_SCALE)
            .ok_or(DecisionImpactError::ArithmeticOverflow)?
            .checked_div(total as u128)
            .ok_or(DecisionImpactError::ArithmeticOverflow)?;
        let weighted = u128::from(weight)
            .checked_mul(resolved_ppb)
            .ok_or(DecisionImpactError::ArithmeticOverflow)?;
        weighted_resolution_numerator = weighted_resolution_numerator
            .checked_add(weighted)
            .ok_or(DecisionImpactError::ArithmeticOverflow)?;

        class_pair_assessments.push(DecisionClassPairImpactAssessment {
            lower_decision_class_id: key.0,
            upper_decision_class_id: key.1,
            relative_impact_weight: weight,
            hypothesis_pair_count: total,
            guaranteed_separated_pair_count: separated,
            unresolved_pair_count: total.checked_sub(separated).ok_or(DecisionImpactError::InvalidReport)?,
            resolved_fraction_parts_per_billion: u32::try_from(resolved_ppb)
                .map_err(|_| DecisionImpactError::ArithmeticOverflow)?,
        });
    }

    if class_pair_assessments.len() != expected_weight_count {
        return Err(DecisionImpactError::InvalidReport);
    }
    let weighted_resolution_ppb = weighted_resolution_numerator
        .checked_div(total_weight)
        .ok_or(DecisionImpactError::ArithmeticOverflow)?;
    let resolved_ppb = u32::try_from(weighted_resolution_ppb)
        .map_err(|_| DecisionImpactError::ArithmeticOverflow)?;
    let unresolved_ppb = u32::try_from(RESOLUTION_SCALE)
        .map_err(|_| DecisionImpactError::ArithmeticOverflow)?
        .checked_sub(resolved_ppb)
        .ok_or(DecisionImpactError::InvalidReport)?;

    Ok(DecisionImpactWeightedAssessment {
        action_id: report.action_id,
        decision_partition_sha256: report.decision_partition_sha256,
        impact_model_sha256: policy.impact_model_sha256,
        decision_class_pair_count: expected_class_pair_count,
        total_cross_decision_hypothesis_pair_count: expected_cross_class_pair_count,
        total_relative_impact_weight: total_weight,
        resolved_impact_fraction_parts_per_billion: resolved_ppb,
        unresolved_impact_fraction_parts_per_billion: unresolved_ppb,
        class_pair_assessments,
    })
}

/// Compose a consequence-weighted conditional score with the matching
/// action-specific record in the #7165 reliability-aware plan. This keeps the
/// plan's domain/protocol commitments and the admitted record's source/cohort/
/// method commitments together in the resulting assessment. The probability
/// method and campaign independence remain externally qualified inputs.
pub fn apply_acquisition_reliability_to_decision_impact(
    assessment: &DecisionImpactWeightedAssessment,
    reliability_plan: &ReliableActiveEvidencePlan,
) -> Result<ReliableDecisionImpactAssessment, DecisionImpactError> {
    if assessment.action_id == 0
        || reliability_plan.expected_domain_fingerprint_sha256.iter().all(|byte| *byte == 0)
        || reliability_plan.expected_acquisition_protocol_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(DecisionImpactError::InvalidReport);
    }

    let mut matching = reliability_plan
        .pareto_frontier
        .iter()
        .filter(|item| item.conditional.action.action_id == assessment.action_id);
    let selected = matching
        .next()
        .ok_or(DecisionImpactError::ActionIdentityMismatch)?;
    if matching.next().is_some() {
        return Err(DecisionImpactError::InvalidReliabilityContract);
    }
    let reliability = &selected.reliability;
    if reliability.action_id != assessment.action_id {
        return Err(DecisionImpactError::ActionIdentityMismatch);
    }
    validate_admitted_reliability(reliability)?;

    let conditional_resolution =
        f64::from(assessment.resolved_impact_fraction_parts_per_billion)
            / RESOLUTION_SCALE as f64;
    let lower = conditional_resolution * reliability.probability_lower_bound;
    let upper = conditional_resolution * reliability.probability_upper_bound;
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(DecisionImpactError::InvalidReliabilityContract);
    }

    Ok(ReliableDecisionImpactAssessment {
        action_id: assessment.action_id,
        domain_fingerprint_sha256: reliability_plan.expected_domain_fingerprint_sha256,
        acquisition_protocol_sha256: reliability_plan.expected_acquisition_protocol_sha256,
        decision_partition_sha256: assessment.decision_partition_sha256,
        impact_model_sha256: assessment.impact_model_sha256,
        admitted_acquisition_probability_lower_bound: reliability.probability_lower_bound,
        admitted_acquisition_probability_upper_bound: reliability.probability_upper_bound,
        expected_resolved_impact_fraction_lower_bound: lower,
        expected_resolved_impact_fraction_upper_bound: upper,
        expected_residual_impact_fraction_lower_bound: 1.0 - upper,
        expected_residual_impact_fraction_upper_bound: 1.0 - lower,
        independent_campaign_block_count: reliability.independent_campaign_block_count,
        confidence_error_probability: reliability.confidence_error_probability,
        source_manifest_sha256: reliability.source_manifest_sha256,
        calibration_cohort_sha256: reliability.calibration_cohort_sha256,
        bound_method_sha256: reliability.bound_method_sha256,
    })
}

fn validate_admitted_reliability(
    reliability: &AdmittedAcquisitionReliability,
) -> Result<(), DecisionImpactError> {
    let total_outcomes = reliability
        .valid_qualified_success_block_count
        .checked_add(reliability.non_success_block_count)
        .ok_or(DecisionImpactError::InvalidReliabilityContract)?;
    let non_success_outcomes = reliability
        .failed_or_timed_out_block_count
        .checked_add(reliability.acquired_but_invalid_block_count)
        .and_then(|count| count.checked_add(reliability.unresolved_block_count))
        .ok_or(DecisionImpactError::InvalidReliabilityContract)?;

    let empirical = if reliability.independent_campaign_block_count == 0 {
        f64::NAN
    } else {
        reliability.valid_qualified_success_block_count as f64
            / reliability.independent_campaign_block_count as f64
    };

    if total_outcomes != reliability.independent_campaign_block_count
        || non_success_outcomes != reliability.non_success_block_count
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
        return Err(DecisionImpactError::InvalidReliabilityContract);
    }
    Ok(())
}

fn canonical_class_pair(a: u64, b: u64) -> (u64, u64) {
    if a < b { (a, b) } else { (b, a) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_evidence_reliability::AdmittedAcquisitionReliability;
    use crate::active_evidence_acquisition::{EvidenceActionAssessment, EvidenceActionKind};
    use crate::active_evidence_decision_planner::EvidenceActionDecisionAssessment;
    use crate::active_evidence_reliability_planner::{
        ReliableActiveEvidencePlan, ReliableEvidenceActionAssessment, ReliableEvidencePlanReason,
    };
    use crate::decision_relevant_discrimination::{
        DecisionProjectionDisposition, DecisionRelevantPairSeparation, HypothesisDecisionAssignment,
    };

    fn report_for_class_sizes(
        action_id: u64,
        class_sizes: &[(u64, usize)],
        separated_class_pairs: &[(u64, u64)],
        partition: [u8; 32],
    ) -> DecisionRelevantDiscriminationReport {
        let mut assignments = Vec::new();
        let mut class_ids = BTreeSet::new();
        let mut next_hypothesis = 1_u64;
        for (class_id, count) in class_sizes {
            class_ids.insert(*class_id);
            for _ in 0..*count {
                assignments.push(HypothesisDecisionAssignment {
                    hypothesis_id: next_hypothesis,
                    decision_class_id: *class_id,
                });
                next_hypothesis += 1;
            }
        }

        let mut pairs = Vec::new();
        for left in 0..assignments.len() {
            for right in (left + 1)..assignments.len() {
                let first = assignments[left];
                let second = assignments[right];
                if first.decision_class_id == second.decision_class_id {
                    continue;
                }
                let class_pair = canonical_class_pair(
                    first.decision_class_id,
                    second.decision_class_id,
                );
                let separated = separated_class_pairs.contains(&class_pair);
                pairs.push(DecisionRelevantPairSeparation {
                    lower_hypothesis_id: first.hypothesis_id,
                    upper_hypothesis_id: second.hypothesis_id,
                    lower_decision_class_id: first.decision_class_id,
                    upper_decision_class_id: second.decision_class_id,
                    normalized_outcome_gap: if separated { 0.25 } else { 0.0 },
                    guaranteed_distinguishable: separated,
                });
            }
        }
        let same_pairs = class_sizes
            .iter()
            .map(|(_, count)| *count * count.saturating_sub(1) / 2)
            .sum::<usize>();
        let separated_count = pairs.iter().filter(|pair| pair.guaranteed_distinguishable).count();
        let ambiguous_count = pairs.len() - separated_count;

        DecisionRelevantDiscriminationReport {
            action_id,
            hypothesis_inventory_sha256: [0x11; 32],
            outcome_model_sha256: [0x22; 32],
            normalization_sha256: [0x33; 32],
            decision_partition_sha256: partition,
            hypothesis_count: assignments.len(),
            decision_class_count: class_ids.len(),
            decision_class_pair_count: class_ids.len() * class_ids.len().saturating_sub(1) / 2,
            same_decision_hypothesis_pair_count: same_pairs,
            decision_incompatible_hypothesis_pair_count: pairs.len(),
            distinguishable_decision_pair_count: separated_count,
            ambiguous_decision_pair_count: ambiguous_count,
            disposition: if pairs.is_empty() {
                DecisionProjectionDisposition::NoDecisionIncompatiblePairs
            } else if ambiguous_count > 0 {
                DecisionProjectionDisposition::DecisionAmbiguityRemains
            } else {
                DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
            },
            conditional_on_successful_valid_observation: true,
            decision_assignments: assignments,
            decision_pair_separations: pairs,
        }
    }

    fn policy(weights: &[(u64, u64, u64)]) -> DecisionImpactWeightPolicy {
        DecisionImpactWeightPolicy {
            expected_decision_partition_sha256: [0x44; 32],
            impact_model_sha256: [0x55; 32],
            class_pair_weights: weights
                .iter()
                .map(|(a, b, weight)| DecisionClassPairImpactWeight {
                    lower_decision_class_id: *a,
                    upper_decision_class_id: *b,
                    relative_impact_weight: *weight,
                })
                .collect(),
        }
    }

    fn reliability(action_id: u64, lower: f64, upper: f64) -> AdmittedAcquisitionReliability {
        AdmittedAcquisitionReliability {
            action_id,
            probability_lower_bound: lower,
            probability_upper_bound: upper,
            confidence_error_probability: 0.05,
            independent_campaign_block_count: 10,
            valid_qualified_success_block_count: 8,
            failed_or_timed_out_block_count: 1,
            acquired_but_invalid_block_count: 1,
            non_success_block_count: 2,
            unresolved_block_count: 0,
            empirical_valid_success_fraction: 0.8,
            source_manifest_sha256: [0x61; 32],
            calibration_cohort_sha256: [0x62; 32],
            bound_method_sha256: [0x63; 32],
        }
    }

    fn reliable_plan(action_id: u64, lower: f64, upper: f64) -> ReliableActiveEvidencePlan {
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
                    disposition: DecisionProjectionDisposition::DecisionAmbiguityRemains,
                    minimum_positive_normalized_gap: Some(0.25),
                    outcome_model_sha256: [0x21; 32],
                    normalization_sha256: [0x31; 32],
                    separated_decision_hypothesis_pairs: vec![(1, 2)],
                    unresolved_decision_hypothesis_pairs: vec![(1, 3)],
                },
                reliability: reliability(action_id, lower, upper),
                expected_decision_pair_yield_lower_bound: lower,
                expected_decision_pair_yield_upper_bound: upper,
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

    #[test]
    fn consequence_weighting_prevents_many_low_impact_pairs_from_masking_one_severe_pair() {
        let report = report_for_class_sizes(
            7,
            &[(1, 1), (2, 1), (3, 10)],
            &[(1, 3), (2, 3)],
            [0x44; 32],
        );
        // 20 of 21 hypothesis pairs are separated. An unweighted hypothesis-
        // pair count would report 95.2% separation. The one unresolved (1,2)
        // class pair carries 1000x the impact weight of either resolved pair.
        let result = assess_decision_impact_weighted_discrimination(
            &report,
            &policy(&[(1, 2, 1000), (1, 3, 1), (2, 3, 1)]),
        )
        .unwrap();

        assert_eq!(result.total_cross_decision_hypothesis_pair_count, 21);
        assert_eq!(result.decision_class_pair_count, 3);
        assert_eq!(result.resolved_impact_fraction_parts_per_billion, 1_996_007);
        assert_eq!(result.unresolved_impact_fraction_parts_per_billion, 998_003_993);
        assert_eq!(result.class_pair_assessments[0].unresolved_pair_count, 1);
        assert_eq!(result.class_pair_assessments[0].relative_impact_weight, 1000);
    }

    #[test]
    fn within_class_pair_normalization_removes_hypothesis_count_advantage() {
        let many = report_for_class_sizes(7, &[(1, 1), (2, 20)], &[(1, 2)], [0x44; 32]);
        let one = report_for_class_sizes(8, &[(1, 1), (2, 1)], &[(1, 2)], [0x44; 32]);
        let matrix = policy(&[(1, 2, 10)]);
        let many_result = assess_decision_impact_weighted_discrimination(&many, &matrix).unwrap();
        let one_result = assess_decision_impact_weighted_discrimination(&one, &matrix).unwrap();
        assert_eq!(
            many_result.resolved_impact_fraction_parts_per_billion,
            one_result.resolved_impact_fraction_parts_per_billion
        );
    }

    #[test]
    fn every_decision_class_pair_requires_exactly_one_weight() {
        let report = report_for_class_sizes(7, &[(1, 1), (2, 1), (3, 1)], &[], [0x44; 32]);
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &policy(&[(1, 2, 1), (1, 3, 1)])),
            Err(DecisionImpactError::MissingDecisionClassPairWeight(2, 3))
        );
        assert_eq!(
            assess_decision_impact_weighted_discrimination(
                &report,
                &policy(&[(1, 2, 1), (1, 3, 1), (2, 3, 1), (1, 4, 1)]),
            ),
            Err(DecisionImpactError::UnexpectedDecisionClassPairWeight(1, 4))
        );
    }

    #[test]
    fn partition_mismatch_and_missing_model_commitment_reject() {
        let report = report_for_class_sizes(7, &[(1, 1), (2, 1)], &[], [0x44; 32]);
        let mut candidate = policy(&[(1, 2, 1)]);
        candidate.expected_decision_partition_sha256 = [0x45; 32];
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &candidate),
            Err(DecisionImpactError::DecisionPartitionMismatch)
        );

        candidate = policy(&[(1, 2, 1)]);
        candidate.impact_model_sha256 = [0; 32];
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &candidate),
            Err(DecisionImpactError::MissingImpactModelCommitment)
        );
    }

    #[test]
    fn malformed_or_duplicate_weights_reject() {
        let report = report_for_class_sizes(7, &[(1, 1), (2, 1)], &[], [0x44; 32]);
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &policy(&[(2, 1, 1)])),
            Err(DecisionImpactError::NonCanonicalDecisionClassPair(2, 1))
        );
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &policy(&[(1, 2, 0)])),
            Err(DecisionImpactError::ZeroDecisionClassPairWeight(1, 2))
        );
        assert_eq!(
            assess_decision_impact_weighted_discrimination(
                &report,
                &policy(&[(1, 2, 1), (1, 2, 2)]),
            ),
            Err(DecisionImpactError::DuplicateDecisionClassPairWeight(1, 2))
        );
    }

    #[test]
    fn pair_class_assignment_and_projection_counters_are_revalidated() {
        let mut report = report_for_class_sizes(7, &[(1, 1), (2, 1)], &[(1, 2)], [0x44; 32]);
        report.decision_pair_separations[0].lower_decision_class_id = 2;
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &policy(&[(1, 2, 1)])),
            Err(DecisionImpactError::InvalidDecisionPair)
        );

        let mut report = report_for_class_sizes(7, &[(1, 1), (2, 1)], &[(1, 2)], [0x44; 32]);
        report.ambiguous_decision_pair_count = 1;
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &policy(&[(1, 2, 1)])),
            Err(DecisionImpactError::InvalidReport)
        );
    }

    #[test]
    fn canonical_score_is_invariant_to_input_serialization_order() {
        let mut report = report_for_class_sizes(
            7,
            &[(1, 2), (2, 2), (3, 1)],
            &[(1, 2), (2, 3)],
            [0x44; 32],
        );
        let first = assess_decision_impact_weighted_discrimination(
            &report,
            &policy(&[(1, 2, 3), (1, 3, 5), (2, 3, 7)]),
        )
        .unwrap();
        report.decision_pair_separations.reverse();
        report.decision_assignments.reverse();
        let second = assess_decision_impact_weighted_discrimination(
            &report,
            &policy(&[(2, 3, 7), (1, 3, 5), (1, 2, 3)]),
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn reliability_adjustment_scales_conditional_value_without_claiming_a_measurement() {
        let mut report = report_for_class_sizes(7, &[(1, 1), (2, 2)], &[], [0x44; 32]);
        report.decision_pair_separations[0].normalized_outcome_gap = 0.25;
        report.decision_pair_separations[0].guaranteed_distinguishable = true;
        report.distinguishable_decision_pair_count = 1;
        report.ambiguous_decision_pair_count = 1;
        let conditional = assess_decision_impact_weighted_discrimination(
            &report,
            &policy(&[(1, 2, 5)]),
        )
        .unwrap();
        // One of two hypothesis pairs is distinguishable: conditional score 0.5.
        let result = apply_acquisition_reliability_to_decision_impact(
            &conditional,
            &reliable_plan(7, 0.5, 0.8),
        )
        .unwrap();
        assert_eq!(result.expected_resolved_impact_fraction_lower_bound, 0.25);
        assert_eq!(result.expected_resolved_impact_fraction_upper_bound, 0.4);
        assert_eq!(result.expected_residual_impact_fraction_lower_bound, 0.6);
        assert_eq!(result.expected_residual_impact_fraction_upper_bound, 0.75);
        assert_eq!(result.independent_campaign_block_count, 10);
        assert_eq!(result.domain_fingerprint_sha256, [0x11; 32]);
        assert_eq!(result.acquisition_protocol_sha256, [0x22; 32]);
        assert_eq!(result.source_manifest_sha256, [0x61; 32]);
        assert_eq!(result.calibration_cohort_sha256, [0x62; 32]);
        assert_eq!(result.bound_method_sha256, [0x63; 32]);
    }

    #[test]
    fn missing_plan_domain_or_protocol_commitment_rejects() {
        let report = report_for_class_sizes(7, &[(1, 1), (2, 1)], &[(1, 2)], [0x44; 32]);
        let conditional = assess_decision_impact_weighted_discrimination(
            &report,
            &policy(&[(1, 2, 5)]),
        )
        .unwrap();

        let mut plan = reliable_plan(7, 0.5, 0.8);
        plan.expected_domain_fingerprint_sha256 = [0; 32];
        assert_eq!(
            apply_acquisition_reliability_to_decision_impact(&conditional, &plan),
            Err(DecisionImpactError::InvalidReport)
        );

        let mut plan = reliable_plan(7, 0.5, 0.8);
        plan.expected_acquisition_protocol_sha256 = [0; 32];
        assert_eq!(
            apply_acquisition_reliability_to_decision_impact(&conditional, &plan),
            Err(DecisionImpactError::InvalidReport)
        );
    }

    #[test]
    fn action_mismatch_and_malformed_reliability_reject() {
        let report = report_for_class_sizes(7, &[(1, 1), (2, 1)], &[(1, 2)], [0x44; 32]);
        let conditional = assess_decision_impact_weighted_discrimination(
            &report,
            &policy(&[(1, 2, 5)]),
        )
        .unwrap();
        assert_eq!(
            apply_acquisition_reliability_to_decision_impact(&conditional, &reliable_plan(8, 0.5, 0.8)),
            Err(DecisionImpactError::ActionIdentityMismatch)
        );
        assert_eq!(
            apply_acquisition_reliability_to_decision_impact(&conditional, &reliable_plan(7, 0.9, 0.8)),
            Err(DecisionImpactError::InvalidReliabilityContract)
        );
    }

    #[test]
    fn one_decision_class_has_no_weighted_pair_score() {
        let report = report_for_class_sizes(7, &[(1, 4)], &[], [0x44; 32]);
        assert_eq!(
            assess_decision_impact_weighted_discrimination(&report, &policy(&[])),
            Err(DecisionImpactError::NoDecisionPairUniverse)
        );
    }
}
