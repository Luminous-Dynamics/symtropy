// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact-identity reconciliation of pre- and post-action conjunction portfolios.
//!
//! This module compares caller-supplied event inventories and identifies new,
//! absent, definitely elevated, definitely reduced and interval-ambiguous
//! event risks. It does not propagate orbits, validate screening completeness,
//! authorize maneuvers or establish flight safety.

use std::collections::BTreeMap;

use crate::space_time_partition::{NonnegativeRationalInterval, Rational};

/// One event's collision-probability envelope, bound to a source assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConjunctionEventProbabilityEnvelope {
    /// Stable identity of the primary-secondary-TCA event.
    pub event_id: [u8; 32],
    /// Digest of the source assessment that produced this envelope.
    pub assessment_digest: [u8; 32],
    /// Inclusive probability interval; endpoints must lie in [0, 1].
    pub probability: NonnegativeRationalInterval,
}

/// A complete or explicitly incomplete conjunction inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjunctionPortfolioSnapshot {
    /// Digest identifying this immutable event inventory.
    pub snapshot_digest: [u8; 32],
    /// Digest of the catalog revision used for screening.
    pub catalog_revision_digest: [u8; 32],
    /// Digest of the ephemeris set screened for this inventory.
    pub ephemeris_digest: [u8; 32],
    /// Digest binding the definition, units and conditioning of event probability.
    pub risk_semantics_digest: [u8; 32],
    /// True only when the producer declares the inventory complete for its horizon.
    pub inventory_complete: bool,
    /// True only when the ephemeris was screened against the full declared catalog.
    pub full_catalog_screened: bool,
    /// Every event in the declared screening horizon; IDs must be unique.
    pub events: Vec<ConjunctionEventProbabilityEnvelope>,
}

/// Immutable evidence binding one proposed action to both screening snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionEffectReceipt {
    /// Digest of the proposed action and its maneuver assumptions.
    pub action_digest: [u8; 32],
    /// Snapshot digest of the pre-action portfolio.
    pub baseline_snapshot_digest: [u8; 32],
    /// Snapshot digest of the post-action portfolio.
    pub post_action_snapshot_digest: [u8; 32],
    /// Ephemeris digest expected for the post-action full-catalog screening.
    pub post_action_ephemeris_digest: [u8; 32],
    /// Whether the secondary object is also maneuverable.
    pub secondary_maneuverable: bool,
    /// Evidence digest for operator coordination when both objects can maneuver.
    pub coordination_digest: Option<[u8; 32]>,
}

/// Disposition of a post-action event inventory against an inclusive Pc threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPortfolioDisposition {
    /// Every post-action event's probability upper bound is strictly below threshold.
    PostActionBelowThreshold,
    /// At least one post-action event's lower bound is at or above threshold.
    ThresholdExceeded,
    /// At least one post-action probability interval overlaps the threshold.
    Indeterminate,
    /// Required full-catalog, inventory or operator-coordination evidence is absent.
    IncompleteEvidence,
}

/// Exact, replayable comparison of supplied pre- and post-action portfolios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPortfolioReport {
    /// Action identity bound by the receipt.
    pub action_digest: [u8; 32],
    /// Pre-action snapshot identity.
    pub baseline_snapshot_digest: [u8; 32],
    /// Post-action snapshot identity.
    pub post_action_snapshot_digest: [u8; 32],
    /// Pre-action catalog revision.
    pub baseline_catalog_revision_digest: [u8; 32],
    /// Post-action catalog revision.
    pub post_action_catalog_revision_digest: [u8; 32],
    /// Inclusive Pc threshold used for all post-action events.
    pub threshold: Rational,
    /// Count of events in the baseline inventory.
    pub baseline_event_count: usize,
    /// Count of events in the post-action inventory.
    pub post_action_event_count: usize,
    /// Baseline event IDs absent from the supplied post-action inventory.
    ///
    /// Absence is not proof of zero risk outside the declared screening horizon.
    pub absent_after_action_event_ids: Vec<[u8; 32]>,
    /// Post-action event IDs with no matching baseline identity.
    pub new_event_ids: Vec<[u8; 32]>,
    /// Retained events whose post-action lower Pc bound exceeds the baseline upper bound.
    pub definitely_elevated_event_ids: Vec<[u8; 32]>,
    /// Retained events whose post-action upper Pc bound is below the baseline lower bound.
    pub definitely_reduced_event_ids: Vec<[u8; 32]>,
    /// Retained events whose before/after intervals overlap without being identical.
    pub risk_change_indeterminate_event_ids: Vec<[u8; 32]>,
    /// Post-action events whose probability lower bound is at or above threshold.
    pub threshold_exceeded_event_ids: Vec<[u8; 32]>,
    /// Post-action events whose probability interval straddles or touches threshold.
    pub threshold_ambiguous_event_ids: Vec<[u8; 32]>,
    /// Threshold disposition, separate from event-delta classification.
    pub disposition: ActionPortfolioDisposition,
}

/// Input, identity or event-inventory validation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPortfolioError {
    /// A required digest is all zero.
    InvalidDigest,
    /// Receipt snapshot or ephemeris identity does not match the supplied snapshot.
    ReceiptIdentityMismatch,
    /// Before and after snapshots use different probability semantics.
    RiskSemanticsMismatch,
    /// An event identity is duplicated within one snapshot.
    DuplicateEventIdentity,
    /// An event identity is all zero.
    InvalidEventIdentity,
    /// An event assessment digest is all zero.
    InvalidAssessmentDigest,
    /// A probability interval is reversed or its upper endpoint exceeds one.
    InvalidProbabilityInterval,
    /// The inclusive probability threshold exceeds one.
    InvalidThreshold,
    /// A supplied coordination digest is all zero.
    InvalidCoordinationDigest,
}

impl core::fmt::Display for ActionPortfolioError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidDigest => "a required action or snapshot digest is invalid",
            Self::ReceiptIdentityMismatch => "action receipt does not match supplied snapshots",
            Self::RiskSemanticsMismatch => "pre- and post-action risk semantics differ",
            Self::DuplicateEventIdentity => "event identity is duplicated in a portfolio",
            Self::InvalidEventIdentity => "event identity is invalid",
            Self::InvalidAssessmentDigest => "event assessment digest is invalid",
            Self::InvalidProbabilityInterval => "event probability interval is invalid",
            Self::InvalidThreshold => "probability threshold exceeds one",
            Self::InvalidCoordinationDigest => "operator coordination digest is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ActionPortfolioError {}

/// Reconcile pre- and post-action event identities and probability envelopes.
///
/// The threshold rule is inclusive: an event is definitely above threshold
/// when its lower bound is at or above the threshold, definitely below when
/// its upper bound is strictly below, and indeterminate otherwise. Missing
/// events are reported as absent from the supplied post-action inventory and
/// are never silently assigned zero probability.
///
/// Incomplete screening and missing required coordination produce an explicit
/// IncompleteEvidence disposition. Identity mismatches, duplicate IDs and
/// malformed probability intervals are errors. A below-threshold disposition
/// applies only to the declared horizon and catalog. The function does not
/// validate the producer's completeness claims or authorize external action.
pub fn reconcile_action_effects(
    before: &ConjunctionPortfolioSnapshot,
    after: &ConjunctionPortfolioSnapshot,
    receipt: ActionEffectReceipt,
    threshold: Rational,
) -> Result<ActionPortfolioReport, ActionPortfolioError> {
    let one = Rational::from_integer(1);
    if is_zero_digest(&receipt.action_digest)
        || is_zero_digest(&before.snapshot_digest)
        || is_zero_digest(&after.snapshot_digest)
        || is_zero_digest(&before.catalog_revision_digest)
        || is_zero_digest(&after.catalog_revision_digest)
        || is_zero_digest(&before.ephemeris_digest)
        || is_zero_digest(&after.ephemeris_digest)
        || is_zero_digest(&before.risk_semantics_digest)
        || is_zero_digest(&after.risk_semantics_digest)
    {
        return Err(ActionPortfolioError::InvalidDigest);
    }
    if threshold > one {
        return Err(ActionPortfolioError::InvalidThreshold);
    }
    if receipt.baseline_snapshot_digest != before.snapshot_digest
        || receipt.post_action_snapshot_digest != after.snapshot_digest
        || receipt.post_action_ephemeris_digest != after.ephemeris_digest
    {
        return Err(ActionPortfolioError::ReceiptIdentityMismatch);
    }
    if before.risk_semantics_digest != after.risk_semantics_digest {
        return Err(ActionPortfolioError::RiskSemanticsMismatch);
    }
    if let Some(coordination_digest) = receipt.coordination_digest {
        if is_zero_digest(&coordination_digest) {
            return Err(ActionPortfolioError::InvalidCoordinationDigest);
        }
    }

    let baseline = index_events(&before.events)?;
    let post_action = index_events(&after.events)?;

    let mut absent_after_action_event_ids = Vec::new();
    let mut new_event_ids = Vec::new();
    let mut definitely_elevated_event_ids = Vec::new();
    let mut definitely_reduced_event_ids = Vec::new();
    let mut risk_change_indeterminate_event_ids = Vec::new();
    let mut threshold_exceeded_event_ids = Vec::new();
    let mut threshold_ambiguous_event_ids = Vec::new();

    for event_id in baseline.keys() {
        if !post_action.contains_key(event_id) {
            absent_after_action_event_ids.push(*event_id);
        }
    }

    for (event_id, post_event) in &post_action {
        match baseline.get(event_id) {
            None => new_event_ids.push(*event_id),
            Some(pre_event) => {
                let pre = pre_event.probability;
                let post = post_event.probability;
                if post.lower > pre.upper {
                    definitely_elevated_event_ids.push(*event_id);
                } else if post.upper < pre.lower {
                    definitely_reduced_event_ids.push(*event_id);
                } else if post != pre {
                    risk_change_indeterminate_event_ids.push(*event_id);
                }
            }
        }

        if post_event.probability.lower >= threshold {
            threshold_exceeded_event_ids.push(*event_id);
        } else if post_event.probability.upper >= threshold {
            threshold_ambiguous_event_ids.push(*event_id);
        }
    }

    let evidence_complete = before.inventory_complete
        && after.inventory_complete
        && before.full_catalog_screened
        && after.full_catalog_screened
        && (!receipt.secondary_maneuverable || receipt.coordination_digest.is_some());

    let disposition = if !evidence_complete {
        ActionPortfolioDisposition::IncompleteEvidence
    } else if !threshold_exceeded_event_ids.is_empty() {
        ActionPortfolioDisposition::ThresholdExceeded
    } else if !threshold_ambiguous_event_ids.is_empty() {
        ActionPortfolioDisposition::Indeterminate
    } else {
        ActionPortfolioDisposition::PostActionBelowThreshold
    };

    Ok(ActionPortfolioReport {
        action_digest: receipt.action_digest,
        baseline_snapshot_digest: before.snapshot_digest,
        post_action_snapshot_digest: after.snapshot_digest,
        baseline_catalog_revision_digest: before.catalog_revision_digest,
        post_action_catalog_revision_digest: after.catalog_revision_digest,
        threshold,
        baseline_event_count: baseline.len(),
        post_action_event_count: post_action.len(),
        absent_after_action_event_ids,
        new_event_ids,
        definitely_elevated_event_ids,
        definitely_reduced_event_ids,
        risk_change_indeterminate_event_ids,
        threshold_exceeded_event_ids,
        threshold_ambiguous_event_ids,
        disposition,
    })
}

fn index_events(
    events: &[ConjunctionEventProbabilityEnvelope],
) -> Result<BTreeMap<[u8; 32], &ConjunctionEventProbabilityEnvelope>, ActionPortfolioError> {
    let one = Rational::from_integer(1);
    let mut indexed = BTreeMap::new();
    for event in events {
        if is_zero_digest(&event.event_id) {
            return Err(ActionPortfolioError::InvalidEventIdentity);
        }
        if is_zero_digest(&event.assessment_digest) {
            return Err(ActionPortfolioError::InvalidAssessmentDigest);
        }
        if event.probability.lower > event.probability.upper || event.probability.upper > one {
            return Err(ActionPortfolioError::InvalidProbabilityInterval);
        }
        if indexed.insert(event.event_id, event).is_some() {
            return Err(ActionPortfolioError::DuplicateEventIdentity);
        }
    }
    Ok(indexed)
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTION: [u8; 32] = [1; 32];
    const BEFORE: [u8; 32] = [2; 32];
    const AFTER: [u8; 32] = [3; 32];
    const BEFORE_CATALOG: [u8; 32] = [4; 32];
    const AFTER_CATALOG: [u8; 32] = [5; 32];
    const BEFORE_EPHEMERIS: [u8; 32] = [6; 32];
    const AFTER_EPHEMERIS: [u8; 32] = [7; 32];
    const SEMANTICS: [u8; 32] = [8; 32];
    const COORDINATION: [u8; 32] = [9; 32];

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn probability(lower: (u128, u128), upper: (u128, u128)) -> NonnegativeRationalInterval {
        NonnegativeRationalInterval {
            lower: rational(lower.0, lower.1),
            upper: rational(upper.0, upper.1),
        }
    }

    fn event(
        id: u8,
        assessment: u8,
        lower: (u128, u128),
        upper: (u128, u128),
    ) -> ConjunctionEventProbabilityEnvelope {
        ConjunctionEventProbabilityEnvelope {
            event_id: [id; 32],
            assessment_digest: [assessment; 32],
            probability: probability(lower, upper),
        }
    }

    fn snapshot(
        digest: [u8; 32],
        catalog: [u8; 32],
        ephemeris: [u8; 32],
        events: Vec<ConjunctionEventProbabilityEnvelope>,
    ) -> ConjunctionPortfolioSnapshot {
        ConjunctionPortfolioSnapshot {
            snapshot_digest: digest,
            catalog_revision_digest: catalog,
            ephemeris_digest: ephemeris,
            risk_semantics_digest: SEMANTICS,
            inventory_complete: true,
            full_catalog_screened: true,
            events,
        }
    }

    fn receipt() -> ActionEffectReceipt {
        ActionEffectReceipt {
            action_digest: ACTION,
            baseline_snapshot_digest: BEFORE,
            post_action_snapshot_digest: AFTER,
            post_action_ephemeris_digest: AFTER_EPHEMERIS,
            secondary_maneuverable: false,
            coordination_digest: None,
        }
    }

    fn reconcile(
        before_events: Vec<ConjunctionEventProbabilityEnvelope>,
        after_events: Vec<ConjunctionEventProbabilityEnvelope>,
    ) -> ActionPortfolioReport {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, before_events);
        let after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, after_events);
        reconcile_action_effects(&before, &after, receipt(), rational(1, 10_000)).unwrap()
    }

    #[test]
    fn complete_portfolio_reports_new_absent_reduced_and_elevated_events() {
        let report = reconcile(
            vec![
                event(10, 20, (1, 100_000), (2, 100_000)),
                event(11, 21, (1, 1_000_000), (2, 1_000_000)),
                event(12, 22, (1, 1_000_000), (2, 1_000_000)),
            ],
            vec![
                event(10, 30, (1, 1_000_000), (2, 1_000_000)),
                event(11, 31, (3, 1_000_000), (4, 1_000_000)),
                event(13, 32, (1, 2_000_000), (2, 2_000_000)),
            ],
        );
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::PostActionBelowThreshold
        );
        assert_eq!(report.absent_after_action_event_ids, vec![[12; 32]]);
        assert_eq!(report.definitely_reduced_event_ids, vec![[10; 32]]);
        assert_eq!(report.definitely_elevated_event_ids, vec![[11; 32]]);
        assert_eq!(report.new_event_ids, vec![[13; 32]]);
        assert!(report.threshold_exceeded_event_ids.is_empty());
    }

    #[test]
    fn new_event_above_threshold_is_not_hidden_by_improvement_to_target_event() {
        let report = reconcile(
            vec![event(10, 20, (2, 10_000), (3, 10_000))],
            vec![
                event(10, 30, (1, 100_000), (2, 100_000)),
                event(11, 31, (2, 10_000), (3, 10_000)),
            ],
        );
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::ThresholdExceeded
        );
        assert_eq!(report.new_event_ids, vec![[11; 32]]);
        assert_eq!(report.threshold_exceeded_event_ids, vec![[11; 32]]);
    }

    #[test]
    fn probability_interval_touching_threshold_is_indeterminate() {
        let report = reconcile(vec![], vec![event(10, 20, (9, 100_000), (1, 10_000))]);
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::Indeterminate
        );
        assert_eq!(report.threshold_ambiguous_event_ids, vec![[10; 32]]);
    }

    #[test]
    fn post_event_wholly_above_threshold_is_reported_as_exceeded() {
        let report = reconcile(vec![], vec![event(10, 20, (2, 10_000), (3, 10_000))]);
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::ThresholdExceeded
        );
    }

    #[test]
    fn fully_screened_empty_post_action_inventory_is_below_threshold() {
        let report = reconcile(vec![event(10, 20, (1, 10_000), (2, 10_000))], vec![]);
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::PostActionBelowThreshold
        );
        assert_eq!(report.absent_after_action_event_ids, vec![[10; 32]]);
    }

    #[test]
    fn incomplete_or_non_full_catalog_screening_abstains() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let mut after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        after.full_catalog_screened = false;
        let report =
            reconcile_action_effects(&before, &after, receipt(), rational(1, 10_000)).unwrap();
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::IncompleteEvidence
        );

        let mut incomplete_before = before.clone();
        incomplete_before.inventory_complete = false;
        let complete_after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        let report = reconcile_action_effects(
            &incomplete_before,
            &complete_after,
            receipt(),
            rational(1, 10_000),
        )
        .unwrap();
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::IncompleteEvidence
        );
    }

    #[test]
    fn missing_secondary_operator_coordination_abstains() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        let mut action_receipt = receipt();
        action_receipt.secondary_maneuverable = true;
        let report =
            reconcile_action_effects(&before, &after, action_receipt, rational(1, 10_000)).unwrap();
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::IncompleteEvidence
        );
    }

    #[test]
    fn supplied_operator_coordination_digest_satisfies_receipt_presence_check() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        let mut action_receipt = receipt();
        action_receipt.secondary_maneuverable = true;
        action_receipt.coordination_digest = Some(COORDINATION);
        let report =
            reconcile_action_effects(&before, &after, action_receipt, rational(1, 10_000)).unwrap();
        assert_eq!(
            report.disposition,
            ActionPortfolioDisposition::PostActionBelowThreshold
        );
    }

    #[test]
    fn mismatched_snapshot_or_action_ephemeris_digest_fails_closed() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        let mut action_receipt = receipt();
        action_receipt.post_action_ephemeris_digest = BEFORE_EPHEMERIS;
        assert_eq!(
            reconcile_action_effects(&before, &after, action_receipt, rational(1, 10_000)),
            Err(ActionPortfolioError::ReceiptIdentityMismatch)
        );
    }

    #[test]
    fn risk_semantics_change_between_snapshots_is_rejected() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let mut after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        after.risk_semantics_digest = [99; 32];
        assert_eq!(
            reconcile_action_effects(&before, &after, receipt(), rational(1, 10_000)),
            Err(ActionPortfolioError::RiskSemanticsMismatch)
        );
    }

    #[test]
    fn duplicate_and_zero_event_ids_are_rejected() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let duplicate = snapshot(
            AFTER,
            AFTER_CATALOG,
            AFTER_EPHEMERIS,
            vec![
                event(10, 20, (1, 100_000), (2, 100_000)),
                event(10, 21, (1, 100_000), (2, 100_000)),
            ],
        );
        assert_eq!(
            reconcile_action_effects(&before, &duplicate, receipt(), rational(1, 10_000)),
            Err(ActionPortfolioError::DuplicateEventIdentity)
        );

        let zero_id = snapshot(
            AFTER,
            AFTER_CATALOG,
            AFTER_EPHEMERIS,
            vec![ConjunctionEventProbabilityEnvelope {
                event_id: [0; 32],
                assessment_digest: [20; 32],
                probability: probability((0, 1), (1, 10)),
            }],
        );
        assert_eq!(
            reconcile_action_effects(&before, &zero_id, receipt(), rational(1, 10_000)),
            Err(ActionPortfolioError::InvalidEventIdentity)
        );
    }

    #[test]
    fn invalid_probability_intervals_and_assessment_digests_are_rejected() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let reversed = snapshot(
            AFTER,
            AFTER_CATALOG,
            AFTER_EPHEMERIS,
            vec![ConjunctionEventProbabilityEnvelope {
                event_id: [10; 32],
                assessment_digest: [20; 32],
                probability: probability((2, 10), (1, 10)),
            }],
        );
        assert_eq!(
            reconcile_action_effects(&before, &reversed, receipt(), rational(1, 10)),
            Err(ActionPortfolioError::InvalidProbabilityInterval)
        );

        let above_one = snapshot(
            AFTER,
            AFTER_CATALOG,
            AFTER_EPHEMERIS,
            vec![event(10, 20, (1, 1), (2, 1))],
        );
        assert_eq!(
            reconcile_action_effects(&before, &above_one, receipt(), rational(1, 10)),
            Err(ActionPortfolioError::InvalidProbabilityInterval)
        );

        let no_assessment_digest = snapshot(
            AFTER,
            AFTER_CATALOG,
            AFTER_EPHEMERIS,
            vec![event(10, 0, (0, 1), (1, 10))],
        );
        assert_eq!(
            reconcile_action_effects(&before, &no_assessment_digest, receipt(), rational(1, 10)),
            Err(ActionPortfolioError::InvalidAssessmentDigest)
        );
    }

    #[test]
    fn zero_coordination_digest_is_rejected() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        let mut action_receipt = receipt();
        action_receipt.coordination_digest = Some([0; 32]);
        assert_eq!(
            reconcile_action_effects(&before, &after, action_receipt, rational(1, 10_000)),
            Err(ActionPortfolioError::InvalidCoordinationDigest)
        );
    }

    #[test]
    fn threshold_above_probability_domain_is_rejected() {
        let before = snapshot(BEFORE, BEFORE_CATALOG, BEFORE_EPHEMERIS, vec![]);
        let after = snapshot(AFTER, AFTER_CATALOG, AFTER_EPHEMERIS, vec![]);
        assert_eq!(
            reconcile_action_effects(&before, &after, receipt(), rational(2, 1)),
            Err(ActionPortfolioError::InvalidThreshold)
        );
    }
}
