//! Conditional observation discrimination between competing UWB fault hypotheses.
//!
//! This module evaluates externally supplied, normalized outcome-set models
//! for a candidate observation action. For each hypothesis h, the caller
//! supplies a Euclidean ball containing all normalized feature outcomes that
//! the action may produce under h, conditional on a successful and valid
//! acquisition.
//!
//! Two hypotheses are guaranteed distinguishable by this action only when
//! their supplied outcome balls are strictly disjoint:
//!
//!     gap(i,j) = max(0, ||mu_i - mu_j||_2 - r_i - r_j)
//!
//! A positive gap is a separation certificate conditional on the supplied
//! outcome enclosures and normalization commitment. Zero means the action
//! does not guarantee discrimination for that pair.
//!
//! The module does not predict whether the action succeeds, estimate outcome
//! distributions, prove calibration, execute sensing, update a position, or
//! authorize NORMAL. Its output is a decision-support diagnostic for the
//! existing resource-bounded active-acquisition planner.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct UwbHypothesisOutcomeBall {
    pub hypothesis_id: u64,
    /// Dimensionless, normalized observation-feature vector. The normalization
    /// contract is committed by the action model below.
    pub normalized_center: Vec<f64>,
    /// Conservative Euclidean error radius in the same normalized space.
    pub normalized_error_radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbObservationActionModel {
    pub action_id: u64,
    /// Must equal the digest of the hypothesis inventory being discriminated.
    pub hypothesis_inventory_sha256: [u8; 32],
    /// Commitment to the predicted outcome-set model and its provenance.
    pub outcome_model_sha256: [u8; 32],
    /// Commitment to feature ordering, scaling and normalization semantics.
    pub normalization_sha256: [u8; 32],
    /// One conditional outcome set for every currently surviving hypothesis.
    pub outcomes: Vec<UwbHypothesisOutcomeBall>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbHypothesisPairSeparation {
    pub lower_hypothesis_id: u64,
    pub upper_hypothesis_id: u64,
    /// Zero means no guaranteed separation. Positive values are lower bounds
    /// on the Euclidean gap between the two conditional outcome sets.
    pub normalized_outcome_gap: f64,
    pub guaranteed_distinguishable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbHypothesisDiscriminationReport {
    pub action_id: u64,
    pub hypothesis_inventory_sha256: [u8; 32],
    pub outcome_model_sha256: [u8; 32],
    pub normalization_sha256: [u8; 32],
    pub hypothesis_count: usize,
    pub hypothesis_pair_count: usize,
    pub distinguishable_pair_count: usize,
    pub ambiguous_pair_count: usize,
    pub minimum_positive_gap: Option<f64>,
    pub maximum_pair_gap: f64,
    /// All outcome sets are conditional on successful, valid acquisition.
    /// This report is not an acquisition-success prediction.
    pub conditional_on_successful_valid_observation: bool,
    /// Canonically sorted by the pair of hypothesis IDs.
    pub pair_separations: Vec<UwbHypothesisPairSeparation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UwbHypothesisDiscriminationError {
    InvalidActionIdentity,
    MissingInventoryCommitment,
    InventoryCommitmentMismatch,
    MissingOutcomeModelCommitment,
    MissingNormalizationCommitment,
    EmptyHypothesisSet,
    InvalidExpectedHypothesisIdentity,
    DuplicateExpectedHypothesisIdentity(u64),
    EmptyOutcomeModel,
    OutcomeHypothesisCountMismatch,
    InvalidOutcomeHypothesisIdentity,
    DuplicateOutcomeHypothesisIdentity(u64),
    UnexpectedOutcomeHypothesisIdentity(u64),
    MissingOutcomeHypothesisIdentity(u64),
    FeatureDimensionMismatch,
    InvalidOutcomeCenter(u64),
    InvalidOutcomeRadius(u64),
    ArithmeticOverflow,
    PairCountOverflow,
}

impl fmt::Display for UwbHypothesisDiscriminationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidActionIdentity => write!(f, "action identity must be nonzero"),
            Self::MissingInventoryCommitment => write!(f, "hypothesis inventory commitment must be nonzero"),
            Self::InventoryCommitmentMismatch => write!(f, "action model is bound to a different hypothesis inventory"),
            Self::MissingOutcomeModelCommitment => write!(f, "outcome model commitment must be nonzero"),
            Self::MissingNormalizationCommitment => write!(f, "feature normalization commitment must be nonzero"),
            Self::EmptyHypothesisSet => write!(f, "at least one surviving hypothesis is required"),
            Self::InvalidExpectedHypothesisIdentity => write!(f, "expected hypothesis identity must be nonzero"),
            Self::DuplicateExpectedHypothesisIdentity(id) => write!(f, "duplicate expected hypothesis identity: {id}"),
            Self::EmptyOutcomeModel => write!(f, "at least one conditional outcome set is required"),
            Self::OutcomeHypothesisCountMismatch => write!(f, "the outcome model must contain exactly one set per surviving hypothesis"),
            Self::InvalidOutcomeHypothesisIdentity => write!(f, "outcome hypothesis identity must be nonzero"),
            Self::DuplicateOutcomeHypothesisIdentity(id) => write!(f, "duplicate outcome hypothesis identity: {id}"),
            Self::UnexpectedOutcomeHypothesisIdentity(id) => write!(f, "outcome model contains an unexpected hypothesis: {id}"),
            Self::MissingOutcomeHypothesisIdentity(id) => write!(f, "outcome model omits surviving hypothesis: {id}"),
            Self::FeatureDimensionMismatch => write!(f, "all conditional outcome centers must have the same nonzero feature dimension"),
            Self::InvalidOutcomeCenter(id) => write!(f, "normalized outcome center is invalid for hypothesis {id}"),
            Self::InvalidOutcomeRadius(id) => write!(f, "normalized outcome radius is invalid for hypothesis {id}"),
            Self::ArithmeticOverflow => write!(f, "outcome-set separation arithmetic overflowed"),
            Self::PairCountOverflow => write!(f, "hypothesis-pair count overflowed"),
        }
    }
}

impl std::error::Error for UwbHypothesisDiscriminationError {}

/// Compute guaranteed pairwise outcome-set separation for one candidate
/// observation action.
///
/// The expected hypothesis IDs should come directly from the current
/// hypothesis inventory. An action that cannot predict a qualified outcome
/// enclosure for every surviving hypothesis is rejected rather than scored
/// using a favourable subset.
pub fn evaluate_uwb_hypothesis_discrimination(
    expected_hypothesis_ids: &[u64],
    expected_inventory_sha256: [u8; 32],
    action: &UwbObservationActionModel,
) -> Result<UwbHypothesisDiscriminationReport, UwbHypothesisDiscriminationError> {
    if action.action_id == 0 {
        return Err(UwbHypothesisDiscriminationError::InvalidActionIdentity);
    }
    if expected_inventory_sha256.iter().all(|byte| *byte == 0) {
        return Err(UwbHypothesisDiscriminationError::MissingInventoryCommitment);
    }
    if action.hypothesis_inventory_sha256.iter().all(|byte| *byte == 0) {
        return Err(UwbHypothesisDiscriminationError::MissingInventoryCommitment);
    }
    if action.hypothesis_inventory_sha256 != expected_inventory_sha256 {
        return Err(UwbHypothesisDiscriminationError::InventoryCommitmentMismatch);
    }
    if action.outcome_model_sha256.iter().all(|byte| *byte == 0) {
        return Err(UwbHypothesisDiscriminationError::MissingOutcomeModelCommitment);
    }
    if action.normalization_sha256.iter().all(|byte| *byte == 0) {
        return Err(UwbHypothesisDiscriminationError::MissingNormalizationCommitment);
    }
    if expected_hypothesis_ids.is_empty() {
        return Err(UwbHypothesisDiscriminationError::EmptyHypothesisSet);
    }
    if action.outcomes.is_empty() {
        return Err(UwbHypothesisDiscriminationError::EmptyOutcomeModel);
    }
    if action.outcomes.len() != expected_hypothesis_ids.len() {
        return Err(UwbHypothesisDiscriminationError::OutcomeHypothesisCountMismatch);
    }

    let mut expected = BTreeSet::new();
    for id in expected_hypothesis_ids {
        if *id == 0 {
            return Err(UwbHypothesisDiscriminationError::InvalidExpectedHypothesisIdentity);
        }
        if !expected.insert(*id) {
            return Err(UwbHypothesisDiscriminationError::DuplicateExpectedHypothesisIdentity(*id));
        }
    }

    let feature_dimension = action.outcomes[0].normalized_center.len();
    if feature_dimension == 0 {
        return Err(UwbHypothesisDiscriminationError::FeatureDimensionMismatch);
    }

    let mut outcomes = action.outcomes.clone();
    outcomes.sort_by_key(|outcome| outcome.hypothesis_id);
    let mut observed = BTreeSet::new();

    for outcome in &outcomes {
        if outcome.hypothesis_id == 0 {
            return Err(UwbHypothesisDiscriminationError::InvalidOutcomeHypothesisIdentity);
        }
        if !observed.insert(outcome.hypothesis_id) {
            return Err(
                UwbHypothesisDiscriminationError::DuplicateOutcomeHypothesisIdentity(
                    outcome.hypothesis_id,
                ),
            );
        }
        if !expected.contains(&outcome.hypothesis_id) {
            return Err(
                UwbHypothesisDiscriminationError::UnexpectedOutcomeHypothesisIdentity(
                    outcome.hypothesis_id,
                ),
            );
        }
        if outcome.normalized_center.len() != feature_dimension {
            return Err(UwbHypothesisDiscriminationError::FeatureDimensionMismatch);
        }
        if !outcome.normalized_center.iter().all(|value| value.is_finite()) {
            return Err(UwbHypothesisDiscriminationError::InvalidOutcomeCenter(
                outcome.hypothesis_id,
            ));
        }
        if !outcome.normalized_error_radius.is_finite()
            || outcome.normalized_error_radius < 0.0
        {
            return Err(UwbHypothesisDiscriminationError::InvalidOutcomeRadius(
                outcome.hypothesis_id,
            ));
        }
    }

    for expected_id in &expected {
        if !observed.contains(expected_id) {
            return Err(
                UwbHypothesisDiscriminationError::MissingOutcomeHypothesisIdentity(*expected_id),
            );
        }
    }

    let pair_count = outcomes
        .len()
        .checked_mul(outcomes.len().saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(UwbHypothesisDiscriminationError::PairCountOverflow)?;

    let mut pair_separations = Vec::with_capacity(pair_count);
    let mut distinguishable_pair_count = 0_usize;
    let mut minimum_positive_gap = None;
    let mut maximum_pair_gap = 0.0_f64;

    for left_index in 0..outcomes.len() {
        for right_index in (left_index + 1)..outcomes.len() {
            let left = &outcomes[left_index];
            let right = &outcomes[right_index];
            let distance = normalized_distance(
                &left.normalized_center,
                &right.normalized_center,
            )
            .ok_or(UwbHypothesisDiscriminationError::ArithmeticOverflow)?;
            let uncertainty_sum =
                left.normalized_error_radius + right.normalized_error_radius;
            if !uncertainty_sum.is_finite() {
                return Err(UwbHypothesisDiscriminationError::ArithmeticOverflow);
            }
            let gap = (distance - uncertainty_sum).max(0.0);
            if !gap.is_finite() {
                return Err(UwbHypothesisDiscriminationError::ArithmeticOverflow);
            }
            let distinguishable = gap > 0.0;
            if distinguishable {
                distinguishable_pair_count += 1;
                minimum_positive_gap = Some(
                    minimum_positive_gap.map_or(gap, |current: f64| current.min(gap)),
                );
            }
            maximum_pair_gap = maximum_pair_gap.max(gap);
            pair_separations.push(UwbHypothesisPairSeparation {
                lower_hypothesis_id: left.hypothesis_id,
                upper_hypothesis_id: right.hypothesis_id,
                normalized_outcome_gap: gap,
                guaranteed_distinguishable: distinguishable,
            });
        }
    }

    let hypothesis_pair_count = pair_separations.len();
    Ok(UwbHypothesisDiscriminationReport {
        action_id: action.action_id,
        hypothesis_inventory_sha256: action.hypothesis_inventory_sha256,
        outcome_model_sha256: action.outcome_model_sha256,
        normalization_sha256: action.normalization_sha256,
        hypothesis_count: outcomes.len(),
        hypothesis_pair_count,
        distinguishable_pair_count,
        ambiguous_pair_count: hypothesis_pair_count - distinguishable_pair_count,
        minimum_positive_gap,
        maximum_pair_gap,
        conditional_on_successful_valid_observation: true,
        pair_separations,
    })
}

fn normalized_distance(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let mut distance = 0.0_f64;
    for (left_value, right_value) in left.iter().zip(right) {
        let delta = left_value - right_value;
        if !delta.is_finite() {
            return None;
        }
        distance = distance.hypot(delta);
    }
    distance.is_finite().then_some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(outcomes: Vec<UwbHypothesisOutcomeBall>) -> UwbObservationActionModel {
        UwbObservationActionModel {
            action_id: 77,
            hypothesis_inventory_sha256: [0x11; 32],
            outcome_model_sha256: [0x22; 32],
            normalization_sha256: [0x33; 32],
            outcomes,
        }
    }

    fn outcome(id: u64, center: &[f64], radius: f64) -> UwbHypothesisOutcomeBall {
        UwbHypothesisOutcomeBall {
            hypothesis_id: id,
            normalized_center: center.to_vec(),
            normalized_error_radius: radius,
        }
    }

    #[test]
    fn independent_observation_separates_competing_fault_hypotheses() {
        let model = action(vec![
            outcome(1, &[0.0, 0.0], 0.5),
            outcome(2, &[4.0, 0.0], 0.5),
            outcome(3, &[1.5, 0.0], 1.0),
        ]);
        let report =
            evaluate_uwb_hypothesis_discrimination(&[1, 2, 3], [0x11; 32], &model)
                .unwrap();
        assert_eq!(report.hypothesis_pair_count, 3);
        assert_eq!(report.distinguishable_pair_count, 2);
        assert_eq!(report.ambiguous_pair_count, 1);
        assert_eq!(report.minimum_positive_gap, Some(1.0));
        assert!(report.conditional_on_successful_valid_observation);
        assert_eq!(report.pair_separations[0].normalized_outcome_gap, 3.0);
        assert!(report.pair_separations[0].guaranteed_distinguishable);
    }

    #[test]
    fn shared_clock_action_with_overlapping_outcomes_does_not_discriminate() {
        let model = action(vec![
            outcome(1, &[2.0, 1.0], 0.4),
            outcome(2, &[2.1, 1.0], 0.4),
        ]);
        let report =
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &model)
                .unwrap();
        assert_eq!(report.distinguishable_pair_count, 0);
        assert_eq!(report.ambiguous_pair_count, 1);
        assert_eq!(report.maximum_pair_gap, 0.0);
        assert_eq!(report.minimum_positive_gap, None);
    }

    #[test]
    fn increasing_outcome_uncertainty_cannot_improve_discrimination() {
        let narrow = action(vec![outcome(1, &[0.0], 0.1), outcome(2, &[3.0], 0.1)]);
        let wide = action(vec![outcome(1, &[0.0], 1.5), outcome(2, &[3.0], 1.5)]);
        let narrow_report =
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &narrow).unwrap();
        let wide_report =
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &wide).unwrap();
        assert!(wide_report.maximum_pair_gap <= narrow_report.maximum_pair_gap);
        assert!(wide_report.distinguishable_pair_count <= narrow_report.distinguishable_pair_count);
    }

    #[test]
    fn action_and_feature_contracts_are_bound_to_exact_inventory() {
        let model = action(vec![outcome(1, &[0.0], 0.1), outcome(2, &[3.0], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x44; 32], &model),
            Err(UwbHypothesisDiscriminationError::InventoryCommitmentMismatch)
        );
        let mut missing_model_digest = model.clone();
        missing_model_digest.outcome_model_sha256 = [0; 32];
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &missing_model_digest),
            Err(UwbHypothesisDiscriminationError::MissingOutcomeModelCommitment)
        );
        let mut missing_normalization_digest = model.clone();
        missing_normalization_digest.normalization_sha256 = [0; 32];
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &missing_normalization_digest),
            Err(UwbHypothesisDiscriminationError::MissingNormalizationCommitment)
        );
    }

    #[test]
    fn every_surviving_hypothesis_must_have_exactly_one_outcome_set() {
        let missing = action(vec![outcome(1, &[0.0], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &missing),
            Err(UwbHypothesisDiscriminationError::OutcomeHypothesisCountMismatch)
        );

        let duplicate = action(vec![outcome(1, &[0.0], 0.1), outcome(1, &[3.0], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &duplicate),
            Err(UwbHypothesisDiscriminationError::DuplicateOutcomeHypothesisIdentity(1))
        );

        let unexpected = action(vec![outcome(1, &[0.0], 0.1), outcome(3, &[3.0], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &unexpected),
            Err(UwbHypothesisDiscriminationError::UnexpectedOutcomeHypothesisIdentity(3))
        );
    }

    #[test]
    fn canonical_pair_order_is_independent_of_input_order() {
        let first = action(vec![outcome(3, &[5.0], 0.1), outcome(1, &[0.0], 0.1), outcome(2, &[2.0], 0.1)]);
        let second = action(vec![outcome(2, &[2.0], 0.1), outcome(3, &[5.0], 0.1), outcome(1, &[0.0], 0.1)]);
        let a = evaluate_uwb_hypothesis_discrimination(&[1, 2, 3], [0x11; 32], &first).unwrap();
        let b = evaluate_uwb_hypothesis_discrimination(&[3, 2, 1], [0x11; 32], &second).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_invalid_radii_centers_and_feature_dimensions() {
        let invalid_radius = action(vec![outcome(1, &[0.0], -1.0)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1], [0x11; 32], &invalid_radius),
            Err(UwbHypothesisDiscriminationError::InvalidOutcomeRadius(1))
        );
        let invalid_center = action(vec![outcome(1, &[f64::NAN], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1], [0x11; 32], &invalid_center),
            Err(UwbHypothesisDiscriminationError::InvalidOutcomeCenter(1))
        );
        let dimension_mismatch = action(vec![outcome(1, &[0.0], 0.1), outcome(2, &[1.0, 2.0], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &dimension_mismatch),
            Err(UwbHypothesisDiscriminationError::FeatureDimensionMismatch)
        );
    }

    #[test]
    fn rejects_duplicate_or_invalid_expected_hypothesis_ids() {
        let model = action(vec![outcome(1, &[0.0], 0.1), outcome(2, &[3.0], 0.1)]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 1], [0x11; 32], &model),
            Err(UwbHypothesisDiscriminationError::DuplicateExpectedHypothesisIdentity(1))
        );
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[0, 2], [0x11; 32], &model),
            Err(UwbHypothesisDiscriminationError::InvalidExpectedHypothesisIdentity)
        );
    }

    #[test]
    fn distance_gap_is_conservative_and_overflow_fails_closed() {
        let model = action(vec![outcome(1, &[0.0, 0.0], 1.0), outcome(2, &[3.0, 4.0], 1.0)]);
        let report =
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &model).unwrap();
        assert_eq!(report.maximum_pair_gap, 3.0);

        let overflow = action(vec![
            outcome(1, &[-1.0e308], 0.0),
            outcome(2, &[1.0e308], 0.0),
        ]);
        assert_eq!(
            evaluate_uwb_hypothesis_discrimination(&[1, 2], [0x11; 32], &overflow),
            Err(UwbHypothesisDiscriminationError::ArithmeticOverflow)
        );
    }
}
