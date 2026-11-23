//! Decision-equivalence projection over UWB hypothesis discrimination.
//!
//! The upstream discriminator scores every pair of surviving hypotheses.
//! This module projects that complete pair inventory onto pairs that imply
//! different navigation decisions. It avoids demanding identification of
//! nuisance parameters when those parameters do not change the decision.
//!
//! The projection is conditional on the upstream outcome-set enclosures,
//! complete hypothesis inventory, frozen decision partition and successful,
//! valid acquisition. It does not update hypotheses, protection levels or
//! navigation authority. Digest values are upstream-supplied commitments:
//! this module checks nonzero/equality conditions and preserves them, but does
//! not recompute or authenticate the digests.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::uwb_hypothesis_discrimination::{
    UwbHypothesisDiscriminationReport, UwbHypothesisPairSeparation,
};

/// Frozen mapping from one surviving physical/fault hypothesis to the
/// navigation decision class it implies under the selected profile.
///
/// Class IDs are profile-local stable identities. The commitment to the
/// complete canonical mapping is supplied separately by the profile owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HypothesisDecisionAssignment {
    pub hypothesis_id: u64,
    pub decision_class_id: u64,
}

/// One pair whose hypotheses imply different navigation decisions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionRelevantPairSeparation {
    pub lower_hypothesis_id: u64,
    pub upper_hypothesis_id: u64,
    pub lower_decision_class_id: u64,
    pub upper_decision_class_id: u64,
    pub normalized_outcome_gap: f64,
    pub guaranteed_distinguishable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionProjectionDisposition {
    /// Every surviving hypothesis implies the same decision. There is no
    /// decision-incompatible pair for this action to separate.
    NoDecisionIncompatiblePairs,
    /// At least one pair implying different decisions has overlapping
    /// conditional outcome enclosures.
    DecisionAmbiguityRemains,
    /// Every pair implying different decisions has disjoint conditional
    /// outcome enclosures under the supplied conservative model.
    AllDecisionIncompatiblePairsSeparated,
}

/// Canonical projection of an all-pairs discrimination report onto the
/// decision-incompatible hypothesis pairs.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionRelevantDiscriminationReport {
    pub action_id: u64,
    pub hypothesis_inventory_sha256: [u8; 32],
    pub outcome_model_sha256: [u8; 32],
    pub normalization_sha256: [u8; 32],
    pub decision_partition_sha256: [u8; 32],
    pub hypothesis_count: usize,
    pub decision_class_count: usize,
    pub decision_class_pair_count: usize,
    pub same_decision_hypothesis_pair_count: usize,
    pub decision_incompatible_hypothesis_pair_count: usize,
    pub distinguishable_decision_pair_count: usize,
    pub ambiguous_decision_pair_count: usize,
    pub disposition: DecisionProjectionDisposition,
    /// All outcome-set separations are conditional on successful, valid acquisition.
    pub conditional_on_successful_valid_observation: bool,
    /// Canonical order by hypothesis identity.
    pub decision_assignments: Vec<HypothesisDecisionAssignment>,
    /// Canonical order by (lower_hypothesis_id, upper_hypothesis_id).
    /// Same-decision pairs are intentionally absent from this projection.
    pub decision_pair_separations: Vec<DecisionRelevantPairSeparation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionProjectionError {
    ActionIdentityMismatch,
    InvalidUpstreamReport,
    MissingExpectedInventoryCommitment,
    InventoryCommitmentMismatch,
    MissingDecisionPartitionCommitment,
    EmptyExpectedHypothesisInventory,
    ExpectedHypothesisCountMismatch,
    EmptyDecisionAssignments,
    InvalidHypothesisIdentity,
    DuplicateExpectedHypothesisIdentity(u64),
    InvalidDecisionClassIdentity,
    DuplicateHypothesisAssignment(u64),
    MissingHypothesisAssignment(u64),
    UnexpectedHypothesisAssignment(u64),
    InvalidPairIdentity,
    DuplicateHypothesisPair,
    PairCountOverflow,
}

impl fmt::Display for DecisionProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActionIdentityMismatch => write!(f, "upstream report action identity does not match the requested action"),
            Self::InvalidUpstreamReport => write!(
                f,
                "upstream hypothesis-discrimination report is incomplete or internally inconsistent"
            ),
            Self::MissingExpectedInventoryCommitment => {
                write!(f, "expected hypothesis-inventory SHA-256 commitment must be nonzero")
            }
            Self::InventoryCommitmentMismatch => {
                write!(f, "upstream discrimination report is bound to a different hypothesis inventory")
            }
            Self::MissingDecisionPartitionCommitment => {
                write!(f, "decision partition SHA-256 commitment must be nonzero")
            }
            Self::EmptyExpectedHypothesisInventory => {
                write!(f, "expected hypothesis inventory must not be empty")
            }
            Self::ExpectedHypothesisCountMismatch => {
                write!(f, "expected hypothesis inventory count differs from the upstream report")
            }
            Self::EmptyDecisionAssignments => {
                write!(f, "at least one hypothesis decision assignment is required")
            }
            Self::InvalidHypothesisIdentity => {
                write!(f, "hypothesis identity in the decision partition must be nonzero")
            }
            Self::DuplicateExpectedHypothesisIdentity(id) => {
                write!(f, "expected hypothesis identity is duplicated: {id}")
            }
            Self::InvalidDecisionClassIdentity => {
                write!(f, "decision-class identity must be nonzero")
            }
            Self::DuplicateHypothesisAssignment(id) => {
                write!(f, "hypothesis has more than one decision assignment: {id}")
            }
            Self::MissingHypothesisAssignment(id) => {
                write!(f, "decision partition omits surviving hypothesis: {id}")
            }
            Self::UnexpectedHypothesisAssignment(id) => {
                write!(f, "decision partition contains an unknown hypothesis: {id}")
            }
            Self::InvalidPairIdentity => {
                write!(f, "upstream pair identity is zero, reversed, or outside the inventory")
            }
            Self::DuplicateHypothesisPair => {
                write!(f, "upstream report contains a duplicate hypothesis pair")
            }
            Self::PairCountOverflow => write!(f, "hypothesis or decision-class pair count overflowed"),
        }
    }
}

impl std::error::Error for DecisionProjectionError {}

/// Project an already-produced all-pairs discrimination report onto pairs
/// whose hypotheses imply different decisions.
///
/// The caller supplies the expected, canonical hypothesis IDs and their
/// inventory commitment from the current fault-inventory owner. This prevents
/// the pair report and decision mapping from defining their own incomplete
/// universe. The function validates exactly n*(n-1)/2 unique unordered pairs,
/// every assignment, all pair values and the upstream summary counters.
///
/// Same-decision pairs remain in same_decision_hypothesis_pair_count but do
/// not penalize decision identifiability. The decision partition is a
/// predeclared profile input; this function does not infer it from outcomes.
pub fn project_decision_relevant_discrimination(
    upstream: &UwbHypothesisDiscriminationReport,
    expected_action_id: u64,
    expected_hypothesis_ids: &[u64],
    expected_inventory_sha256: [u8; 32],
    assignments: &[HypothesisDecisionAssignment],
    decision_partition_sha256: [u8; 32],
) -> Result<DecisionRelevantDiscriminationReport, DecisionProjectionError> {
    if expected_action_id == 0 || upstream.action_id != expected_action_id {
        return Err(DecisionProjectionError::ActionIdentityMismatch);
    }
    if upstream.hypothesis_inventory_sha256.iter().all(|byte| *byte == 0)
        || upstream.outcome_model_sha256.iter().all(|byte| *byte == 0)
        || upstream.normalization_sha256.iter().all(|byte| *byte == 0)
        || !upstream.conditional_on_successful_valid_observation
        || upstream.hypothesis_count == 0
    {
        return Err(DecisionProjectionError::InvalidUpstreamReport);
    }
    if expected_inventory_sha256.iter().all(|byte| *byte == 0) {
        return Err(DecisionProjectionError::MissingExpectedInventoryCommitment);
    }
    if upstream.hypothesis_inventory_sha256 != expected_inventory_sha256 {
        return Err(DecisionProjectionError::InventoryCommitmentMismatch);
    }
    if decision_partition_sha256.iter().all(|byte| *byte == 0) {
        return Err(DecisionProjectionError::MissingDecisionPartitionCommitment);
    }
    if expected_hypothesis_ids.is_empty() {
        return Err(DecisionProjectionError::EmptyExpectedHypothesisInventory);
    }
    if expected_hypothesis_ids.len() != upstream.hypothesis_count {
        return Err(DecisionProjectionError::ExpectedHypothesisCountMismatch);
    }
    if assignments.is_empty() {
        return Err(DecisionProjectionError::EmptyDecisionAssignments);
    }

    let expected_pair_count = upstream
        .hypothesis_count
        .checked_mul(upstream.hypothesis_count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(DecisionProjectionError::PairCountOverflow)?;

    if upstream.hypothesis_pair_count != expected_pair_count
        || upstream.pair_separations.len() != expected_pair_count
    {
        return Err(DecisionProjectionError::InvalidUpstreamReport);
    }

    let mut expected_hypotheses = BTreeSet::new();
    for hypothesis_id in expected_hypothesis_ids {
        if *hypothesis_id == 0 {
            return Err(DecisionProjectionError::InvalidHypothesisIdentity);
        }
        if !expected_hypotheses.insert(*hypothesis_id) {
            return Err(DecisionProjectionError::DuplicateExpectedHypothesisIdentity(
                *hypothesis_id,
            ));
        }
    }

    let mut decision_by_hypothesis = BTreeMap::new();
    for assignment in assignments {
        if assignment.hypothesis_id == 0 {
            return Err(DecisionProjectionError::InvalidHypothesisIdentity);
        }
        if assignment.decision_class_id == 0 {
            return Err(DecisionProjectionError::InvalidDecisionClassIdentity);
        }
        if decision_by_hypothesis
            .insert(assignment.hypothesis_id, assignment.decision_class_id)
            .is_some()
        {
            return Err(DecisionProjectionError::DuplicateHypothesisAssignment(
                assignment.hypothesis_id,
            ));
        }
    }

    for hypothesis_id in &expected_hypotheses {
        if !decision_by_hypothesis.contains_key(hypothesis_id) {
            return Err(DecisionProjectionError::MissingHypothesisAssignment(
                *hypothesis_id,
            ));
        }
    }
    if let Some(unexpected) = decision_by_hypothesis
        .keys()
        .find(|hypothesis_id| !expected_hypotheses.contains(hypothesis_id))
    {
        return Err(DecisionProjectionError::UnexpectedHypothesisAssignment(
            *unexpected,
        ));
    }

    let mut seen_pairs = BTreeSet::new();
    let mut observed_hypotheses = BTreeSet::new();
    let mut upstream_distinguishable = 0_usize;
    let mut upstream_minimum_positive_gap: Option<f64> = None;
    let mut upstream_maximum_gap = 0.0_f64;
    let mut projected_pairs = Vec::new();
    let mut same_decision_pair_count = 0_usize;
    let mut distinguishable_decision_pair_count = 0_usize;
    let mut ambiguous_decision_pair_count = 0_usize;

    let mut canonical_upstream_pairs: Vec<UwbHypothesisPairSeparation> =
        upstream.pair_separations.clone();
    canonical_upstream_pairs.sort_by_key(|pair| {
        (pair.lower_hypothesis_id, pair.upper_hypothesis_id)
    });

    for pair in &canonical_upstream_pairs {
        if pair.lower_hypothesis_id == 0
            || pair.upper_hypothesis_id <= pair.lower_hypothesis_id
            || !expected_hypotheses.contains(&pair.lower_hypothesis_id)
            || !expected_hypotheses.contains(&pair.upper_hypothesis_id)
        {
            return Err(DecisionProjectionError::InvalidPairIdentity);
        }
        if !seen_pairs.insert((pair.lower_hypothesis_id, pair.upper_hypothesis_id)) {
            return Err(DecisionProjectionError::DuplicateHypothesisPair);
        }
        if !pair.normalized_outcome_gap.is_finite()
            || pair.normalized_outcome_gap < 0.0
            || pair.guaranteed_distinguishable != (pair.normalized_outcome_gap > 0.0)
        {
            return Err(DecisionProjectionError::InvalidUpstreamReport);
        }

        observed_hypotheses.insert(pair.lower_hypothesis_id);
        observed_hypotheses.insert(pair.upper_hypothesis_id);
        upstream_maximum_gap = upstream_maximum_gap.max(pair.normalized_outcome_gap);
        if pair.guaranteed_distinguishable {
            upstream_distinguishable += 1;
            upstream_minimum_positive_gap = Some(
                upstream_minimum_positive_gap.map_or(pair.normalized_outcome_gap, |current| {
                    current.min(pair.normalized_outcome_gap)
                }),
            );
        }

        let lower_class = *decision_by_hypothesis
            .get(&pair.lower_hypothesis_id)
            .ok_or(DecisionProjectionError::MissingHypothesisAssignment(
                pair.lower_hypothesis_id,
            ))?;
        let upper_class = *decision_by_hypothesis
            .get(&pair.upper_hypothesis_id)
            .ok_or(DecisionProjectionError::MissingHypothesisAssignment(
                pair.upper_hypothesis_id,
            ))?;

        if lower_class == upper_class {
            same_decision_pair_count += 1;
        } else {
            if pair.guaranteed_distinguishable {
                distinguishable_decision_pair_count += 1;
            } else {
                ambiguous_decision_pair_count += 1;
            }
            projected_pairs.push(DecisionRelevantPairSeparation {
                lower_hypothesis_id: pair.lower_hypothesis_id,
                upper_hypothesis_id: pair.upper_hypothesis_id,
                lower_decision_class_id: lower_class,
                upper_decision_class_id: upper_class,
                normalized_outcome_gap: pair.normalized_outcome_gap,
                guaranteed_distinguishable: pair.guaranteed_distinguishable,
            });
        }
    }

    if upstream.hypothesis_count > 1 && observed_hypotheses != expected_hypotheses {
        return Err(DecisionProjectionError::InvalidUpstreamReport);
    }

    let upstream_ambiguous = expected_pair_count
        .checked_sub(upstream_distinguishable)
        .ok_or(DecisionProjectionError::InvalidUpstreamReport)?;
    if upstream.distinguishable_pair_count != upstream_distinguishable
        || upstream.ambiguous_pair_count != upstream_ambiguous
        || upstream.maximum_pair_gap != upstream_maximum_gap
        || upstream.minimum_positive_gap != upstream_minimum_positive_gap
    {
        return Err(DecisionProjectionError::InvalidUpstreamReport);
    }

    let decision_classes: BTreeSet<u64> =
        decision_by_hypothesis.values().copied().collect();
    let decision_class_pair_count = decision_classes
        .len()
        .checked_mul(decision_classes.len().saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(DecisionProjectionError::PairCountOverflow)?;

    let decision_incompatible_pair_count = projected_pairs.len();
    let disposition = if decision_incompatible_pair_count == 0 {
        DecisionProjectionDisposition::NoDecisionIncompatiblePairs
    } else if ambiguous_decision_pair_count > 0 {
        DecisionProjectionDisposition::DecisionAmbiguityRemains
    } else {
        DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
    };

    let decision_assignments = decision_by_hypothesis
        .into_iter()
        .map(|(hypothesis_id, decision_class_id)| HypothesisDecisionAssignment {
            hypothesis_id,
            decision_class_id,
        })
        .collect();

    Ok(DecisionRelevantDiscriminationReport {
        action_id: upstream.action_id,
        hypothesis_inventory_sha256: upstream.hypothesis_inventory_sha256,
        outcome_model_sha256: upstream.outcome_model_sha256,
        normalization_sha256: upstream.normalization_sha256,
        decision_partition_sha256,
        hypothesis_count: upstream.hypothesis_count,
        decision_class_count: decision_classes.len(),
        decision_class_pair_count,
        same_decision_hypothesis_pair_count: same_decision_pair_count,
        decision_incompatible_hypothesis_pair_count: decision_incompatible_pair_count,
        distinguishable_decision_pair_count,
        ambiguous_decision_pair_count,
        disposition,
        conditional_on_successful_valid_observation: true,
        decision_assignments,
        decision_pair_separations: projected_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(lower: u64, upper: u64, gap: f64) -> UwbHypothesisPairSeparation {
        UwbHypothesisPairSeparation {
            lower_hypothesis_id: lower,
            upper_hypothesis_id: upper,
            normalized_outcome_gap: gap,
            guaranteed_distinguishable: gap > 0.0,
        }
    }

    fn upstream(pairs: Vec<UwbHypothesisPairSeparation>, hypothesis_count: usize) -> UwbHypothesisDiscriminationReport {
        let distinguishable_pair_count = pairs
            .iter()
            .filter(|pair| pair.guaranteed_distinguishable)
            .count();
        let hypothesis_pair_count = pairs.len();
        UwbHypothesisDiscriminationReport {
            action_id: 77,
            hypothesis_inventory_sha256: [0x11; 32],
            outcome_model_sha256: [0x22; 32],
            normalization_sha256: [0x33; 32],
            hypothesis_count,
            hypothesis_pair_count,
            distinguishable_pair_count,
            ambiguous_pair_count: hypothesis_pair_count - distinguishable_pair_count,
            minimum_positive_gap: pairs
                .iter()
                .filter(|pair| pair.guaranteed_distinguishable)
                .map(|pair| pair.normalized_outcome_gap)
                .reduce(f64::min),
            maximum_pair_gap: pairs
                .iter()
                .map(|pair| pair.normalized_outcome_gap)
                .fold(0.0, f64::max),
            conditional_on_successful_valid_observation: true,
            pair_separations: pairs,
        }
    }

    fn assignments(classes: &[(u64, u64)]) -> Vec<HypothesisDecisionAssignment> {
        classes
            .iter()
            .map(|(hypothesis_id, decision_class_id)| HypothesisDecisionAssignment {
                hypothesis_id: *hypothesis_id,
                decision_class_id: *decision_class_id,
            })
            .collect()
    }

    fn project(
        report: &UwbHypothesisDiscriminationReport,
        hypothesis_ids: &[u64],
        decision_assignments: &[HypothesisDecisionAssignment],
    ) -> Result<DecisionRelevantDiscriminationReport, DecisionProjectionError> {
        project_decision_relevant_discrimination(
            report,
            77,
            hypothesis_ids,
            [0x11; 32],
            decision_assignments,
            [0x44; 32],
        )
    }

    #[test]
    fn overlapping_same_decision_pair_does_not_block_decision_separation() {
        let report = upstream(
            vec![pair(1, 2, 0.0), pair(1, 3, 1.0), pair(2, 3, 2.0)],
            3,
        );
        let result = project(
            &report,
            &[1, 2, 3],
            &assignments(&[(1, 10), (2, 10), (3, 20)]),
        )
        .unwrap();

        assert_eq!(result.same_decision_hypothesis_pair_count, 1);
        assert_eq!(result.decision_incompatible_hypothesis_pair_count, 2);
        assert_eq!(result.distinguishable_decision_pair_count, 2);
        assert_eq!(result.ambiguous_decision_pair_count, 0);
        assert_eq!(
            result.disposition,
            DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
        );
        assert_eq!(result.decision_pair_separations.len(), 2);
        assert!(result.conditional_on_successful_valid_observation);
    }

    #[test]
    fn overlapping_cross_decision_pair_keeps_decision_ambiguity() {
        let report = upstream(
            vec![pair(1, 2, 0.0), pair(1, 3, 2.0), pair(2, 3, 1.0)],
            3,
        );
        let result = project(
            &report,
            &[1, 2, 3],
            &assignments(&[(1, 10), (2, 20), (3, 20)]),
        )
        .unwrap();

        assert_eq!(result.same_decision_hypothesis_pair_count, 1);
        assert_eq!(result.decision_incompatible_hypothesis_pair_count, 2);
        assert_eq!(result.ambiguous_decision_pair_count, 1);
        assert_eq!(
            result.disposition,
            DecisionProjectionDisposition::DecisionAmbiguityRemains
        );
    }

    #[test]
    fn a_single_class_is_not_misreported_as_a_successful_sensing_action() {
        let report = upstream(
            vec![pair(1, 2, 0.0), pair(1, 3, 3.0), pair(2, 3, 0.0)],
            3,
        );
        let result = project(
            &report,
            &[1, 2, 3],
            &assignments(&[(1, 7), (2, 7), (3, 7)]),
        )
        .unwrap();

        assert_eq!(result.decision_class_count, 1);
        assert_eq!(result.decision_class_pair_count, 0);
        assert_eq!(result.decision_incompatible_hypothesis_pair_count, 0);
        assert_eq!(
            result.disposition,
            DecisionProjectionDisposition::NoDecisionIncompatiblePairs
        );
    }

    #[test]
    fn adding_a_hypothesis_in_a_new_decision_class_cannot_improve_separation() {
        let two = upstream(vec![pair(1, 2, 1.0)], 2);
        let two_result = project(&two, &[1, 2], &assignments(&[(1, 10), (2, 20)])).unwrap();
        assert_eq!(
            two_result.disposition,
            DecisionProjectionDisposition::AllDecisionIncompatiblePairsSeparated
        );

        let three = upstream(
            vec![pair(1, 2, 1.0), pair(1, 3, 0.0), pair(2, 3, 2.0)],
            3,
        );
        let three_result = project(
            &three,
            &[1, 2, 3],
            &assignments(&[(1, 10), (2, 20), (3, 30)]),
        )
        .unwrap();
        assert_eq!(
            three_result.disposition,
            DecisionProjectionDisposition::DecisionAmbiguityRemains
        );
        assert_eq!(three_result.ambiguous_decision_pair_count, 1);
    }

    #[test]
    fn adding_same_class_hypothesis_cannot_hide_an_existing_unresolved_pair() {
        let two = upstream(vec![pair(1, 2, 0.0)], 2);
        let two_result = project(&two, &[1, 2], &assignments(&[(1, 10), (2, 20)])).unwrap();
        assert_eq!(
            two_result.disposition,
            DecisionProjectionDisposition::DecisionAmbiguityRemains
        );

        let three = upstream(
            vec![pair(1, 2, 0.0), pair(1, 3, 4.0), pair(2, 3, 1.0)],
            3,
        );
        let three_result = project(
            &three,
            &[1, 2, 3],
            &assignments(&[(1, 10), (2, 20), (3, 10)]),
        )
        .unwrap();
        assert_eq!(
            three_result.disposition,
            DecisionProjectionDisposition::DecisionAmbiguityRemains
        );
        assert_eq!(three_result.ambiguous_decision_pair_count, 1);
    }

    #[test]
    fn assignment_and_pair_order_are_canonical() {
        let first = upstream(
            vec![pair(1, 2, 0.0), pair(1, 3, 1.0), pair(2, 3, 2.0)],
            3,
        );
        let permuted = upstream(
            vec![pair(2, 3, 2.0), pair(1, 3, 1.0), pair(1, 2, 0.0)],
            3,
        );
        let a = project(
            &first,
            &[1, 2, 3],
            &assignments(&[(1, 10), (2, 10), (3, 20)]),
        )
        .unwrap();
        let b = project(
            &permuted,
            &[3, 2, 1],
            &assignments(&[(3, 20), (1, 10), (2, 10)]),
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn missing_duplicate_and_extra_assignments_are_rejected() {
        let report = upstream(vec![pair(1, 2, 1.0)], 2);

        assert_eq!(
            project(&report, &[1, 2], &assignments(&[(1, 10)])),
            Err(DecisionProjectionError::MissingHypothesisAssignment(2))
        );

        assert_eq!(
            project(
                &report,
                &[1, 2],
                &assignments(&[(1, 10), (1, 20)]),
            ),
            Err(DecisionProjectionError::DuplicateHypothesisAssignment(1))
        );

        assert_eq!(
            project(
                &report,
                &[1, 2],
                &assignments(&[(1, 10), (3, 20)]),
            ),
            Err(DecisionProjectionError::MissingHypothesisAssignment(2))
        );
        assert_eq!(
            project(
                &report,
                &[1, 2],
                &assignments(&[(1, 10), (2, 20), (3, 30)]),
            ),
            Err(DecisionProjectionError::UnexpectedHypothesisAssignment(3))
        );
    }

    #[test]
    fn malformed_pair_report_and_commitment_mismatches_fail_closed() {
        let mut malformed = upstream(vec![pair(1, 2, 1.0)], 2);
        malformed.pair_separations[0].upper_hypothesis_id = 1;
        assert_eq!(
            project(&malformed, &[1, 2], &assignments(&[(1, 10), (2, 20)])),
            Err(DecisionProjectionError::InvalidPairIdentity)
        );

        let valid = upstream(vec![pair(1, 2, 1.0)], 2);
        assert_eq!(
            project_decision_relevant_discrimination(
                &valid,
                77,
                &[1, 2],
                [0x55; 32],
                &assignments(&[(1, 10), (2, 20)]),
                [0x44; 32],
            ),
            Err(DecisionProjectionError::InventoryCommitmentMismatch)
        );
        assert_eq!(
            project_decision_relevant_discrimination(
                &valid,
                78,
                &[1, 2],
                [0x11; 32],
                &assignments(&[(1, 10), (2, 20)]),
                [0x44; 32],
            ),
            Err(DecisionProjectionError::ActionIdentityMismatch)
        );
        assert_eq!(
            project_decision_relevant_discrimination(
                &valid,
                77,
                &[1, 2],
                [0x11; 32],
                &assignments(&[(1, 10), (2, 20)]),
                [0; 32],
            ),
            Err(DecisionProjectionError::MissingDecisionPartitionCommitment)
        );
    }

    #[test]
    fn single_hypothesis_inventory_has_no_decision_incompatible_pair() {
        let report = upstream(Vec::new(), 1);
        let result = project(&report, &[9], &assignments(&[(9, 3)])).unwrap();
        assert_eq!(result.hypothesis_count, 1);
        assert_eq!(
            result.disposition,
            DecisionProjectionDisposition::NoDecisionIncompatiblePairs
        );
    }
}
