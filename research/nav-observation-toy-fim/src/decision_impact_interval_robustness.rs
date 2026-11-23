//! Robust consequence-weighted discrimination under uncertain impact weights.
//!
//! Takes the per-decision-class-pair counts from the validated point scorer
//! and computes a worst/best weighted-resolution interval over an independent
//! box of positive impact-weight intervals. The weight intervals are relative
//! importance inputs, not probabilities or calibrated physical loss.
//!
//! The lower score is a conservative lower bound over all admissible impact
//! weights and lower-rounded class-pair resolutions. The upper score is a
//! conservative upper bound over all admissible weights and upper-rounded
//! class-pair resolutions. Endpoints are rounded outward to parts per billion.
//!
//! This is robust weighted discrimination, not Bayes risk or minimax regret.
//! If the real joint impact set has additional constraints, its enclosing box
//! remains conservative but may be loose. Correlation constraints are not
//! inferred. Digests are caller-supplied commitments and are not authenticated.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::decision_impact_weighted_discrimination::{
    DecisionImpactWeightedAssessment,
};

const SCALE: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionClassPairImpactWeightInterval {
    pub lower_decision_class_id: u64,
    pub upper_decision_class_id: u64,
    pub weight_lower_bound: u64,
    pub weight_upper_bound: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionImpactIntervalPolicy {
    pub expected_decision_partition_sha256: [u8; 32],
    pub expected_impact_model_sha256: [u8; 32],
    pub impact_uncertainty_model_sha256: [u8; 32],
    /// Independent interval box. Every unordered decision-class pair is required.
    pub weight_intervals: Vec<DecisionClassPairImpactWeightInterval>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RobustDecisionClassPairAssessment {
    pub lower_decision_class_id: u64,
    pub upper_decision_class_id: u64,
    pub weight_lower_bound: u64,
    pub weight_upper_bound: u64,
    pub hypothesis_pair_count: usize,
    pub guaranteed_separated_pair_count: usize,
    pub unresolved_pair_count: usize,
    /// Floor of the exact class-pair fraction, scaled by 1e9.
    pub resolution_lower_parts_per_billion: u32,
    /// Ceiling of the exact class-pair fraction, scaled by 1e9.
    pub resolution_upper_parts_per_billion: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustDecisionImpactAssessment {
    pub action_id: u64,
    pub decision_partition_sha256: [u8; 32],
    pub impact_model_sha256: [u8; 32],
    pub impact_uncertainty_model_sha256: [u8; 32],
    /// Floor of the minimum possible weighted resolution over the interval box.
    pub worst_case_resolved_fraction_parts_per_billion: u32,
    /// Ceiling of the maximum possible weighted resolution over the interval box.
    pub best_case_resolved_fraction_parts_per_billion: u32,
    /// Complement of the conservative lower score.
    pub worst_case_unresolved_fraction_parts_per_billion: u32,
    /// Complement of the conservative upper score.
    pub best_case_unresolved_fraction_parts_per_billion: u32,
    /// Lowest class-pair lower resolution; never hidden by the weighted mean.
    pub minimum_class_pair_resolution_lower_parts_per_billion: u32,
    pub minimum_class_pair_resolution_pair: (u64, u64),
    pub pair_assessments: Vec<RobustDecisionClassPairAssessment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobustDecisionImpactError {
    InvalidAssessment,
    MissingPartitionCommitment,
    PartitionMismatch,
    MissingImpactModelCommitment,
    ImpactModelMismatch,
    MissingUncertaintyModelCommitment,
    InvalidDecisionClassPair(u64, u64),
    DuplicateDecisionClassPair(u64, u64),
    MissingWeightInterval(u64, u64),
    UnexpectedWeightInterval(u64, u64),
    DuplicateWeightInterval(u64, u64),
    InvalidWeightInterval(u64, u64),
    ArithmeticOverflow,
}

impl fmt::Display for RobustDecisionImpactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAssessment => write!(f, "point assessment is incomplete or internally inconsistent"),
            Self::MissingPartitionCommitment => write!(f, "decision-partition commitment must be nonzero"),
            Self::PartitionMismatch => write!(f, "interval policy and point assessment use different decision partitions"),
            Self::MissingImpactModelCommitment => write!(f, "point impact-model commitment must be nonzero"),
            Self::ImpactModelMismatch => write!(f, "interval policy and point assessment use different impact models"),
            Self::MissingUncertaintyModelCommitment => write!(f, "impact-uncertainty model commitment must be nonzero"),
            Self::InvalidDecisionClassPair(a, b) => write!(f, "invalid decision-class pair ({a}, {b}) or inconsistent pair counts"),
            Self::DuplicateDecisionClassPair(a, b) => write!(f, "decision-class pair ({a}, {b}) is duplicated in point assessment"),
            Self::MissingWeightInterval(a, b) => write!(f, "impact weight interval for class pair ({a}, {b}) is missing"),
            Self::UnexpectedWeightInterval(a, b) => write!(f, "impact weight interval for unknown class pair ({a}, {b})"),
            Self::DuplicateWeightInterval(a, b) => write!(f, "impact weight interval for class pair ({a}, {b}) is duplicated"),
            Self::InvalidWeightInterval(a, b) => write!(f, "impact weight interval for class pair ({a}, {b}) must be finite, positive and ordered"),
            Self::ArithmeticOverflow => write!(f, "robust weighted-ratio arithmetic overflowed"),
        }
    }
}

impl std::error::Error for RobustDecisionImpactError {}

#[derive(Clone, Copy, Debug)]
struct PairResolution {
    key: (u64, u64),
    weight_lower: u64,
    weight_upper: u64,
    resolution_lower_ppb: u32,
    resolution_upper_ppb: u32,
}

/// Compute sharp robust weighted-ratio bounds over the declared independent
/// interval box, up to conservative integer rounding at 1e-9 resolution.
///
/// For class pairs i with resolution r_i and weight w_i in [l_i,u_i], the
/// score is sum(w_i*r_i)/sum(w_i). The lower endpoint solves the monotone
/// piecewise-linear condition:
///
///   min_{w_i in [l_i,u_i]} sum_i w_i (r_i - t) >= 0.
///
/// The upper endpoint solves the corresponding max expression <= 0.
/// Each term's extremum is attained at an interval endpoint, so binary search
/// over integer parts-per-billion values yields deterministic outward bounds
/// without floating-point comparison or an assumed probability distribution.
pub fn assess_robust_decision_impact(
    point: &DecisionImpactWeightedAssessment,
    policy: &DecisionImpactIntervalPolicy,
) -> Result<RobustDecisionImpactAssessment, RobustDecisionImpactError> {
    let pairs = validate_and_pair(point, policy)?;
    let lower = lower_robust_ratio(&pairs)?;
    let upper = upper_robust_ratio(&pairs)?;
    if lower > upper {
        return Err(RobustDecisionImpactError::InvalidAssessment);
    }

    let minimum_pair = pairs
        .iter()
        .min_by_key(|pair| (pair.resolution_lower_ppb, pair.key))
        .ok_or(RobustDecisionImpactError::InvalidAssessment)?;

    let total_scale = u32::try_from(SCALE)
        .map_err(|_| RobustDecisionImpactError::ArithmeticOverflow)?;

    let pair_assessments = pairs
        .iter()
        .map(|pair| {
            let source = point
                .class_pair_assessments
                .iter()
                .find(|item| (item.lower_decision_class_id, item.upper_decision_class_id) == pair.key)
                .ok_or(RobustDecisionImpactError::InvalidAssessment)?;
            Ok(RobustDecisionClassPairAssessment {
                lower_decision_class_id: pair.key.0,
                upper_decision_class_id: pair.key.1,
                weight_lower_bound: pair.weight_lower,
                weight_upper_bound: pair.weight_upper,
                hypothesis_pair_count: source.hypothesis_pair_count,
                guaranteed_separated_pair_count: source.guaranteed_separated_pair_count,
                unresolved_pair_count: source.unresolved_pair_count,
                resolution_lower_parts_per_billion: pair.resolution_lower_ppb,
                resolution_upper_parts_per_billion: pair.resolution_upper_ppb,
            })
        })
        .collect::<Result<Vec<_>, RobustDecisionImpactError>>()?;

    Ok(RobustDecisionImpactAssessment {
        action_id: point.action_id,
        decision_partition_sha256: point.decision_partition_sha256,
        impact_model_sha256: point.impact_model_sha256,
        impact_uncertainty_model_sha256: policy.impact_uncertainty_model_sha256,
        worst_case_resolved_fraction_parts_per_billion: lower,
        best_case_resolved_fraction_parts_per_billion: upper,
        worst_case_unresolved_fraction_parts_per_billion: total_scale - lower,
        best_case_unresolved_fraction_parts_per_billion: total_scale - upper,
        minimum_class_pair_resolution_lower_parts_per_billion: minimum_pair.resolution_lower_ppb,
        minimum_class_pair_resolution_pair: minimum_pair.key,
        pair_assessments,
    })
}

fn validate_and_pair(
    point: &DecisionImpactWeightedAssessment,
    policy: &DecisionImpactIntervalPolicy,
) -> Result<Vec<PairResolution>, RobustDecisionImpactError> {
    if point.action_id == 0
        || point.decision_class_pair_count == 0
        || point.class_pair_assessments.len() != point.decision_class_pair_count
        || point.total_relative_impact_weight == 0
    {
        return Err(RobustDecisionImpactError::InvalidAssessment);
    }
    if point.decision_partition_sha256.iter().all(|byte| *byte == 0)
        || policy.expected_decision_partition_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(RobustDecisionImpactError::MissingPartitionCommitment);
    }
    if point.decision_partition_sha256 != policy.expected_decision_partition_sha256 {
        return Err(RobustDecisionImpactError::PartitionMismatch);
    }
    if point.impact_model_sha256.iter().all(|byte| *byte == 0)
        || policy.expected_impact_model_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(RobustDecisionImpactError::MissingImpactModelCommitment);
    }
    if point.impact_model_sha256 != policy.expected_impact_model_sha256 {
        return Err(RobustDecisionImpactError::ImpactModelMismatch);
    }
    if policy.impact_uncertainty_model_sha256.iter().all(|byte| *byte == 0) {
        return Err(RobustDecisionImpactError::MissingUncertaintyModelCommitment);
    }

    let mut point_pairs = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut summed_pairs = 0_usize;
    let mut total_weight = 0_u128;
    let mut weighted_resolution = 0_u128;

    for item in &point.class_pair_assessments {
        let key = (item.lower_decision_class_id, item.upper_decision_class_id);
        if key.0 == 0
            || key.1 <= key.0
            || item.hypothesis_pair_count == 0
            || item.guaranteed_separated_pair_count > item.hypothesis_pair_count
            || item.unresolved_pair_count
                != item.hypothesis_pair_count - item.guaranteed_separated_pair_count
            || item.relative_impact_weight == 0
        {
            return Err(RobustDecisionImpactError::InvalidDecisionClassPair(key.0, key.1));
        }
        if !seen.insert(key) {
            return Err(RobustDecisionImpactError::DuplicateDecisionClassPair(key.0, key.1));
        }

        let numerator = (item.guaranteed_separated_pair_count as u128)
            .checked_mul(u128::from(SCALE))
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;
        let denominator = item.hypothesis_pair_count as u128;
        let expected_floor = numerator / denominator;
        let expected_ceil = numerator
            .checked_add(denominator - 1)
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?
            / denominator;
        if u32::try_from(expected_floor).ok() != Some(item.resolved_fraction_parts_per_billion) {
            return Err(RobustDecisionImpactError::InvalidDecisionClassPair(key.0, key.1));
        }

        summed_pairs = summed_pairs
            .checked_add(item.hypothesis_pair_count)
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;
        total_weight = total_weight
            .checked_add(u128::from(item.relative_impact_weight))
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;
        weighted_resolution = weighted_resolution
            .checked_add(
                u128::from(item.relative_impact_weight)
                    .checked_mul(expected_floor)
                    .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?,
            )
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;

        point_pairs.insert(key, (item, u32::try_from(expected_ceil)
            .map_err(|_| RobustDecisionImpactError::ArithmeticOverflow)?));
    }

    let class_ids: BTreeSet<u64> = point_pairs
        .keys()
        .flat_map(|pair| [pair.0, pair.1])
        .collect();
    let expected_class_pair_count = class_ids
        .len()
        .checked_mul(class_ids.len().saturating_sub(1))
        .and_then(|count| count.checked_div(2))
        .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;
    if expected_class_pair_count != point.decision_class_pair_count {
        return Err(RobustDecisionImpactError::InvalidAssessment);
    }
    let ordered_class_ids: Vec<u64> = class_ids.into_iter().collect();
    for index in 0..ordered_class_ids.len() {
        for other_index in (index + 1)..ordered_class_ids.len() {
            let key = (ordered_class_ids[index], ordered_class_ids[other_index]);
            if !point_pairs.contains_key(&key) {
                return Err(RobustDecisionImpactError::InvalidAssessment);
            }
        }
    }

    if summed_pairs != point.total_cross_decision_hypothesis_pair_count
        || total_weight != point.total_relative_impact_weight
        || u32::try_from(weighted_resolution / total_weight).ok()
            != Some(point.resolved_impact_fraction_parts_per_billion)
        || point.unresolved_impact_fraction_parts_per_billion
            != u32::try_from(SCALE).map_err(|_| RobustDecisionImpactError::ArithmeticOverflow)?
                .checked_sub(point.resolved_impact_fraction_parts_per_billion)
                .ok_or(RobustDecisionImpactError::InvalidAssessment)?
    {
        return Err(RobustDecisionImpactError::InvalidAssessment);
    }

    let class_keys: Vec<(u64, u64)> = point_pairs.keys().copied().collect();
    let mut interval_map = BTreeMap::new();
    for interval in &policy.weight_intervals {
        let key = (
            interval.lower_decision_class_id,
            interval.upper_decision_class_id,
        );
        if key.0 == 0 || key.1 <= key.0 {
            return Err(RobustDecisionImpactError::InvalidWeightInterval(key.0, key.1));
        }
        if interval.weight_lower_bound == 0
            || interval.weight_upper_bound < interval.weight_lower_bound
        {
            return Err(RobustDecisionImpactError::InvalidWeightInterval(key.0, key.1));
        }
        if interval_map.insert(key, (interval.weight_lower_bound, interval.weight_upper_bound)).is_some() {
            return Err(RobustDecisionImpactError::DuplicateWeightInterval(key.0, key.1));
        }
    }

    for key in &class_keys {
        if !interval_map.contains_key(key) {
            return Err(RobustDecisionImpactError::MissingWeightInterval(key.0, key.1));
        }
    }
    for key in interval_map.keys() {
        if !point_pairs.contains_key(key) {
            return Err(RobustDecisionImpactError::UnexpectedWeightInterval(key.0, key.1));
        }
    }

    let mut out = Vec::with_capacity(class_keys.len());
    for key in class_keys {
        let (source, resolution_upper_ppb) = point_pairs[&key];
        let (weight_lower, weight_upper) = interval_map[&key];
        out.push(PairResolution {
            key,
            weight_lower,
            weight_upper,
            resolution_lower_ppb: source.resolved_fraction_parts_per_billion,
            resolution_upper_ppb,
        });
    }
    Ok(out)
}

/// For fixed lower-rounded per-pair resolutions, find floor(min weighted mean).
fn lower_robust_ratio(pairs: &[PairResolution]) -> Result<u32, RobustDecisionImpactError> {
    let mut low = 0_u64;
    let mut high = SCALE;
    while low < high {
        let middle = low + (high - low + 1) / 2;
        if extremal_residual_sum(pairs, middle, true)? >= 0 {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    u32::try_from(low).map_err(|_| RobustDecisionImpactError::ArithmeticOverflow)
}

/// For upper-rounded per-pair resolutions, find ceil(max weighted mean).
fn upper_robust_ratio(pairs: &[PairResolution]) -> Result<u32, RobustDecisionImpactError> {
    let mut low = 0_u64;
    let mut high = SCALE;
    while low < high {
        let middle = low + (high - low) / 2;
        if extremal_residual_sum(pairs, middle, false)? <= 0 {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    u32::try_from(low).map_err(|_| RobustDecisionImpactError::ArithmeticOverflow)
}

/// If minimize=true, choose each weight endpoint that minimizes w*(r-t).
/// Otherwise choose the endpoint that maximizes it.
fn extremal_residual_sum(
    pairs: &[PairResolution],
    threshold_ppb: u64,
    minimize: bool,
) -> Result<i128, RobustDecisionImpactError> {
    let mut sum = 0_i128;
    for pair in pairs {
        let resolution = if minimize {
            i128::from(pair.resolution_lower_ppb)
        } else {
            i128::from(pair.resolution_upper_ppb)
        };
        let delta = resolution - i128::from(threshold_ppb);
        let use_lower_weight = if minimize {
            delta >= 0
        } else {
            delta < 0
        };
        let weight = if use_lower_weight {
            pair.weight_lower
        } else {
            pair.weight_upper
        };
        let term = i128::from(weight)
            .checked_mul(delta)
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;
        sum = sum
            .checked_add(term)
            .ok_or(RobustDecisionImpactError::ArithmeticOverflow)?;
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_impact_weighted_discrimination::{
        DecisionClassPairImpactAssessment,
    };

    fn point_assessment(values: &[(u64, u64, usize, usize, u64)]) -> DecisionImpactWeightedAssessment {
        let mut total_pairs = 0_usize;
        let mut total_weight = 0_u128;
        let mut weighted = 0_u128;
        let mut class_pair_assessments = Vec::new();

        for (a, b, total, separated, weight) in values {
            let numerator = (*separated as u128) * u128::from(SCALE);
            let floor = numerator / *total as u128;
            let unresolved = *total - *separated;
            total_pairs += *total;
            total_weight += u128::from(*weight);
            weighted += u128::from(*weight) * floor;
            class_pair_assessments.push(DecisionClassPairImpactAssessment {
                lower_decision_class_id: *a,
                upper_decision_class_id: *b,
                relative_impact_weight: *weight,
                hypothesis_pair_count: *total,
                guaranteed_separated_pair_count: *separated,
                unresolved_pair_count: unresolved,
                resolved_fraction_parts_per_billion: floor as u32,
            });
        }

        DecisionImpactWeightedAssessment {
            action_id: 7,
            decision_partition_sha256: [0x11; 32],
            impact_model_sha256: [0x22; 32],
            decision_class_pair_count: values.len(),
            total_cross_decision_hypothesis_pair_count: total_pairs,
            total_relative_impact_weight: total_weight,
            resolved_impact_fraction_parts_per_billion: (weighted / total_weight) as u32,
            unresolved_impact_fraction_parts_per_billion: SCALE as u32 - (weighted / total_weight) as u32,
            class_pair_assessments,
        }
    }

    fn policy(intervals: &[(u64, u64, u64, u64)]) -> DecisionImpactIntervalPolicy {
        DecisionImpactIntervalPolicy {
            expected_decision_partition_sha256: [0x11; 32],
            expected_impact_model_sha256: [0x22; 32],
            impact_uncertainty_model_sha256: [0x33; 32],
            weight_intervals: intervals
                .iter()
                .map(|(a, b, lower, upper)| DecisionClassPairImpactWeightInterval {
                    lower_decision_class_id: *a,
                    upper_decision_class_id: *b,
                    weight_lower_bound: *lower,
                    weight_upper_bound: *upper,
                })
                .collect(),
        }
    }

    #[test]
    fn fixed_weights_produce_tight_outward_bounds() {
        let point = point_assessment(&[(1, 2, 3, 1, 2), (1, 3, 2, 2, 2)]);
        let result = assess_robust_decision_impact(
            &point,
            &policy(&[(1, 2, 2, 2), (1, 3, 2, 2)]),
        )
        .unwrap();
        // (2 * 1/3 + 2 * 1) / 4 = 2/3.
        assert_eq!(result.worst_case_resolved_fraction_parts_per_billion, 666_666_666);
        assert_eq!(result.best_case_resolved_fraction_parts_per_billion, 666_666_667);
        assert_eq!(result.minimum_class_pair_resolution_lower_parts_per_billion, 333_333_333);
        assert_eq!(result.minimum_class_pair_resolution_pair, (1, 2));
    }

    #[test]
    fn interval_box_reports_sharp_adversarial_weight_extremes() {
        let point = point_assessment(&[(1, 2, 1, 0, 1), (1, 3, 1, 1, 1)]);
        let result = assess_robust_decision_impact(
            &point,
            &policy(&[(1, 2, 1, 100), (1, 3, 1, 100)]),
        )
        .unwrap();
        assert_eq!(result.worst_case_resolved_fraction_parts_per_billion, 9_900_990);
        assert_eq!(result.best_case_resolved_fraction_parts_per_billion, 990_099_010);
        assert_eq!(result.worst_case_unresolved_fraction_parts_per_billion, 990_099_010);
        assert_eq!(result.best_case_unresolved_fraction_parts_per_billion, 9_900_990);
    }

    #[test]
    fn exact_endpoint_enumeration_is_enclosed_for_a_three_pair_weight_box() {
        let point = point_assessment(&[
            (1, 2, 2, 0, 2),
            (1, 3, 2, 1, 3),
            (2, 3, 2, 2, 2),
        ]);
        let intervals = [(1, 2, 1, 3), (1, 3, 2, 4), (2, 3, 1, 2)];
        let result = assess_robust_decision_impact(&point, &policy(&intervals)).unwrap();

        for mask in 0..8 {
            let weights: [u64; 3] = [
                if mask & 1 == 0 { 1 } else { 3 },
                if mask & 2 == 0 { 2 } else { 4 },
                if mask & 4 == 0 { 1 } else { 2 },
            ];
            let resolutions = [0_u64, 500_000_000, 1_000_000_000];
            let numerator = weights
                .iter()
                .zip(resolutions.iter())
                .map(|(weight, resolution)| u128::from(*weight) * u128::from(*resolution))
                .sum::<u128>();
            let denominator = weights.iter().map(|value| u128::from(*value)).sum::<u128>();
            assert!(
                u128::from(result.worst_case_resolved_fraction_parts_per_billion) * denominator
                    <= numerator
            );
            assert!(
                u128::from(result.best_case_resolved_fraction_parts_per_billion) * denominator
                    >= numerator
            );
        }
    }

    #[test]
    fn widening_impact_intervals_cannot_improve_robust_assurance() {
        let point = point_assessment(&[
            (1, 2, 1, 0, 2),
            (1, 3, 1, 1, 2),
            (2, 3, 1, 0, 2),
        ]);
        let narrow = assess_robust_decision_impact(
            &point,
            &policy(&[(1, 2, 2, 3), (1, 3, 2, 3), (2, 3, 2, 3)]),
        )
        .unwrap();
        let wide = assess_robust_decision_impact(
            &point,
            &policy(&[(1, 2, 1, 5), (1, 3, 1, 5), (2, 3, 1, 5)]),
        )
        .unwrap();
        assert!(wide.worst_case_resolved_fraction_parts_per_billion <= narrow.worst_case_resolved_fraction_parts_per_billion);
        assert!(wide.best_case_resolved_fraction_parts_per_billion >= narrow.best_case_resolved_fraction_parts_per_billion);
    }

    #[test]
    fn missing_duplicate_extra_and_invalid_weight_intervals_reject() {
        let point = point_assessment(&[(1, 2, 2, 1, 3), (1, 3, 2, 1, 3)]);
        assert_eq!(
            assess_robust_decision_impact(&point, &policy(&[(1, 2, 1, 2)])),
            Err(RobustDecisionImpactError::MissingWeightInterval(1, 3))
        );
        assert_eq!(
            assess_robust_decision_impact(
                &point,
                &policy(&[(1, 2, 1, 2), (1, 2, 2, 3), (1, 3, 1, 2)]),
            ),
            Err(RobustDecisionImpactError::DuplicateWeightInterval(1, 2))
        );
        assert_eq!(
            assess_robust_decision_impact(
                &point,
                &policy(&[(1, 2, 1, 2), (1, 3, 1, 2), (2, 3, 1, 2)]),
            ),
            Err(RobustDecisionImpactError::UnexpectedWeightInterval(2, 3))
        );
        assert_eq!(
            assess_robust_decision_impact(
                &point,
                &policy(&[(1, 2, 0, 2), (1, 3, 1, 2)]),
            ),
            Err(RobustDecisionImpactError::InvalidWeightInterval(1, 2))
        );
        assert_eq!(
            assess_robust_decision_impact(
                &point,
                &policy(&[(1, 2, 3, 2), (1, 3, 1, 2)]),
            ),
            Err(RobustDecisionImpactError::InvalidWeightInterval(1, 2))
        );
    }

    #[test]
    fn mismatched_or_missing_commitments_reject() {
        let point = point_assessment(&[(1, 2, 1, 1, 1)]);
        let mut candidate = policy(&[(1, 2, 1, 2)]);
        candidate.expected_decision_partition_sha256 = [0x44; 32];
        assert_eq!(
            assess_robust_decision_impact(&point, &candidate),
            Err(RobustDecisionImpactError::PartitionMismatch)
        );
        candidate = policy(&[(1, 2, 1, 2)]);
        candidate.expected_impact_model_sha256 = [0x44; 32];
        assert_eq!(
            assess_robust_decision_impact(&point, &candidate),
            Err(RobustDecisionImpactError::ImpactModelMismatch)
        );
        candidate = policy(&[(1, 2, 1, 2)]);
        candidate.impact_uncertainty_model_sha256 = [0; 32];
        assert_eq!(
            assess_robust_decision_impact(&point, &candidate),
            Err(RobustDecisionImpactError::MissingUncertaintyModelCommitment)
        );
    }

    #[test]
    fn permutation_of_pair_and_interval_records_does_not_change_result() {
        let point = point_assessment(&[
            (1, 2, 1, 0, 2),
            (1, 3, 1, 1, 3),
            (2, 3, 1, 0, 4),
        ]);
        let first = assess_robust_decision_impact(
            &point,
            &policy(&[(1, 2, 1, 3), (1, 3, 2, 4), (2, 3, 1, 2)]),
        )
        .unwrap();
        let mut reordered = point.clone();
        reordered.class_pair_assessments.reverse();
        let second = assess_robust_decision_impact(
            &reordered,
            &policy(&[(2, 3, 1, 2), (1, 3, 2, 4), (1, 2, 1, 3)]),
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn incomplete_decision_class_pair_graph_rejects() {
        let point = point_assessment(&[
            (1, 2, 1, 1, 1),
            (1, 3, 1, 0, 1),
            (2, 4, 1, 0, 1),
        ]);
        assert_eq!(
            assess_robust_decision_impact(
                &point,
                &policy(&[(1, 2, 1, 2), (1, 3, 1, 2), (2, 4, 1, 2)]),
            ),
            Err(RobustDecisionImpactError::InvalidAssessment)
        );
    }

    #[test]
    fn tampered_point_counters_reject() {
        let mut point = point_assessment(&[(1, 2, 2, 1, 3)]);
        point.class_pair_assessments[0].unresolved_pair_count = 0;
        assert_eq!(
            assess_robust_decision_impact(&point, &policy(&[(1, 2, 1, 5)])),
            Err(RobustDecisionImpactError::InvalidDecisionClassPair(1, 2))
        );
        let mut point = point_assessment(&[(1, 2, 2, 1, 3)]);
        point.total_cross_decision_hypothesis_pair_count = 3;
        assert_eq!(
            assess_robust_decision_impact(&point, &policy(&[(1, 2, 1, 5)])),
            Err(RobustDecisionImpactError::InvalidAssessment)
        );
    }
}
