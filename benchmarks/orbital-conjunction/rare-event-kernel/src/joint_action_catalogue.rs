// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite catalogue of joint-action outcomes.
//!
//! A coalition's outcome is supplied as one independently assessed post-action
//! portfolio. This module never derives a multi-action result by adding
//! single-action deltas. It verifies closed finite catalogue coverage and
//! selects only among fully screened, below-threshold, consequence-qualified
//! coalition outcomes.

use std::collections::{BTreeMap, BTreeSet};

use crate::action_effect_portfolio::{
    ActionEffectReceipt, ActionPortfolioDisposition, ActionPortfolioError,
    ConjunctionPortfolioSnapshot, reconcile_action_effects,
};
use crate::space_time_partition::Rational;

/// Maximum action count for the exact finite coalition catalogue.
pub const MAX_JOINT_ACTIONS: usize = 16;

/// One candidate maneuver/action in the frozen planning universe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JointActionCandidate {
    /// Stable action identity.
    pub action_id: [u8; 32],
    /// Resource units consumed when this action is selected.
    pub resource_cost: u64,
    /// Stable identity of the operator responsible for this action.
    pub operator_digest: [u8; 32],
    /// Whether actionability evidence permits this action to be considered.
    pub actionable: bool,
    /// Whether this action requires an explicit operator coordination receipt.
    pub coordination_required: bool,
}

/// Outcome status for one exact canonical action set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointActionOutcomeState {
    /// Independently produced, complete post-action portfolio and consequence result.
    Qualified {
        /// Complete post-action event inventory screened against the declared catalog.
        post_action_snapshot: ConjunctionPortfolioSnapshot,
        /// Identity of the independently qualified consequence model.
        consequence_model_digest: [u8; 32],
        /// Caller-supplied robust lower bound on consequence avoided, in fixed integer ticks.
        robust_consequence_avoided_ticks: u64,
        /// Evidence digest for required operator coordination.
        coordination_digest: Option<[u8; 32]>,
    },
    /// The exact joint action set cannot be executed under the pinned constraints.
    Infeasible {
        /// Evidence for the infeasibility determination.
        reason_digest: [u8; 32],
    },
    /// The exact joint action set has incomplete or unresolved physical evidence.
    Unresolved {
        /// Evidence identifying the unresolved condition.
        reason_digest: [u8; 32],
    },
}

/// One outcome row for one canonical action set.
///
/// Action IDs must be strictly increasing. The action-set digest is an opaque
/// caller-supplied identity and is not recomputed by this policy verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointActionOutcomeRecord {
    /// Canonically sorted action identities; the empty set is the no-action baseline.
    pub action_ids: Vec<[u8; 32]>,
    /// Stable digest binding the exact action set and its maneuver assumptions.
    pub action_set_digest: [u8; 32],
    /// Explicit outcome of this exact action set.
    pub state: JointActionOutcomeState,
}

/// Inputs for an exact finite joint-action outcome catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointActionCatalogueConfig {
    /// Frozen baseline event portfolio.
    pub baseline_snapshot: ConjunctionPortfolioSnapshot,
    /// Complete candidate action universe, at most MAX_JOINT_ACTIONS.
    pub candidates: Vec<JointActionCandidate>,
    /// Exactly one outcome row is required for every statically selectable coalition.
    pub outcomes: Vec<JointActionOutcomeRecord>,
    /// One consequence-model identity shared by every qualified coalition.
    pub consequence_model_digest: [u8; 32],
    /// Inclusive collision-probability threshold applied to each post-action event.
    pub probability_threshold: Rational,
    /// Maximum aggregate resource cost.
    pub resource_capacity: u64,
}

/// Final policy-level disposition of the complete coalition catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointActionSelectionDisposition {
    /// Every selectable coalition is resolved and the best safe coalition is known.
    QualifiedOptimum,
    /// Every selectable coalition is resolved, but none passes the hard safety gate.
    NoSafePortfolio,
    /// At least one selectable coalition remains unresolved or statistically/structurally ambiguous.
    AbstainIncompleteEvidence,
}

/// Summary of exact joint-action catalogue enumeration and selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointActionCatalogueReport {
    /// Final policy disposition.
    pub disposition: JointActionSelectionDisposition,
    /// Selected action identities, present only when the global optimum is qualified.
    pub selected_action_ids: Option<Vec<[u8; 32]>>,
    /// Opaque action-set digest of the selected portfolio.
    pub selected_action_set_digest: Option<[u8; 32]>,
    /// Resource units consumed by the selected portfolio.
    pub selected_resource_cost: Option<u64>,
    /// Robust consequence-avoided lower bound for the selected portfolio.
    pub selected_robust_consequence_avoided_ticks: Option<u64>,
    /// Number of statically selectable coalitions whose outcome was required.
    pub expected_coalition_count: usize,
    /// Number of fully screened, below-threshold, consequence-qualified coalitions.
    pub qualified_safe_count: usize,
    /// Number of coalitions with a confirmed post-action threshold exceedance.
    pub threshold_exceeded_count: usize,
    /// Number of coalitions with an interval overlapping the action threshold.
    pub threshold_ambiguous_count: usize,
    /// Number of coalitions with incomplete screening, catalog or coordination evidence.
    pub incomplete_evidence_count: usize,
    /// Number of coalitions explicitly marked physically infeasible.
    pub infeasible_count: usize,
    /// Number of coalitions explicitly marked unresolved.
    pub unresolved_count: usize,
    /// Number of coalitions excluded by static actionability or capacity constraints.
    pub statically_excluded_count: usize,
}

/// Validation failures for a joint-action outcome catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointActionCatalogueError {
    /// Candidate count exceeds the bounded exact-oracle limit.
    TooManyActions,
    /// Required identity is all zero.
    InvalidIdentity,
    /// Action identity is duplicated or all zero.
    InvalidActionIdentity,
    /// Candidate operator identity is all zero.
    InvalidOperatorIdentity,
    /// Outcome action IDs are not strictly increasing.
    NonCanonicalActionSet,
    /// Outcome references an action outside the frozen candidate universe.
    UnknownAction,
    /// Two outcome rows refer to the same canonical action set.
    DuplicateCoalitionOutcome,
    /// Outcome row is present for a statically unselectable action set.
    UnexpectedCoalitionOutcome,
    /// A required statically selectable coalition has no outcome row.
    MissingCoalitionOutcome,
    /// Distinct coalitions reuse the same action-set digest.
    DuplicateActionSetDigest,
    /// An infeasible or unresolved row lacks a reason digest.
    InvalidReasonDigest,
    /// A qualified row uses a different consequence model.
    ConsequenceModelMismatch,
    /// A selected empty action set does not exactly reproduce the baseline with zero avoided consequence.
    InvalidNoActionControl,
    /// Aggregate action cost arithmetic overflowed.
    ResourceArithmeticOverflow,
    /// Underlying before/after event portfolio reconciliation rejected the evidence.
    InvalidPortfolioEvidence,
}

impl core::fmt::Display for JointActionCatalogueError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::TooManyActions => "joint-action catalogue exceeds the exact action limit",
            Self::InvalidIdentity => "a required joint-action identity is invalid",
            Self::InvalidActionIdentity => "action identity is duplicated or invalid",
            Self::InvalidOperatorIdentity => "candidate operator identity is invalid",
            Self::NonCanonicalActionSet => "coalition action identities are not strictly sorted",
            Self::UnknownAction => "coalition references an action outside the frozen universe",
            Self::DuplicateCoalitionOutcome => "coalition has more than one outcome row",
            Self::UnexpectedCoalitionOutcome => "outcome row exists for a statically unselectable coalition",
            Self::MissingCoalitionOutcome => "a statically selectable coalition lacks an outcome row",
            Self::DuplicateActionSetDigest => "different coalition rows reuse an action-set digest",
            Self::InvalidReasonDigest => "infeasible or unresolved outcome lacks a reason digest",
            Self::ConsequenceModelMismatch => "qualified coalition uses a different consequence model",
            Self::InvalidNoActionControl => "no-action control does not reproduce the baseline",
            Self::ResourceArithmeticOverflow => "joint action resource cost overflowed",
            Self::InvalidPortfolioEvidence => "joint action before/after portfolio evidence is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for JointActionCatalogueError {}

impl From<ActionPortfolioError> for JointActionCatalogueError {
    fn from(_: ActionPortfolioError) -> Self {
        Self::InvalidPortfolioEvidence
    }
}

/// Verify the closed coalition catalogue and choose the best qualified safe portfolio.
///
/// The function enumerates all subsets of the frozen candidate universe. Subsets
/// containing a non-actionable action or exceeding resource capacity are
/// statically excluded and require no physical outcome row. Every remaining
/// subset, including the empty no-action control, must have exactly one row.
///
/// A qualified coalition is eligible only when its independently supplied
/// post-action inventory is complete, full-catalog screened, uses the same
/// frozen catalog revision and risk semantics as the baseline, has no event
/// at or ambiguously overlapping the inclusive threshold, and is bound to the
/// configured consequence-model identity. A known threshold exceedance is
/// excluded; an incomplete, ambiguous or unresolved coalition forces global
/// abstention because an unmeasured combination could dominate the observed
/// safe candidates.
///
/// Ties are deterministic: higher robust consequence avoided, then lower
/// resource cost, then lexicographically smaller canonical action IDs.
pub fn evaluate_joint_action_catalogue(
    config: JointActionCatalogueConfig,
) -> Result<JointActionCatalogueReport, JointActionCatalogueError> {
    if config.candidates.len() > MAX_JOINT_ACTIONS {
        return Err(JointActionCatalogueError::TooManyActions);
    }
    if is_zero_digest(&config.baseline_snapshot.snapshot_digest)
        || is_zero_digest(&config.consequence_model_digest)
    {
        return Err(JointActionCatalogueError::InvalidIdentity);
    }
    let mut candidates = BTreeMap::new();
    for candidate in &config.candidates {
        if is_zero_digest(&candidate.action_id) || candidates.contains_key(&candidate.action_id) {
            return Err(JointActionCatalogueError::InvalidActionIdentity);
        }
        if is_zero_digest(&candidate.operator_digest) {
            return Err(JointActionCatalogueError::InvalidOperatorIdentity);
        }
        candidates.insert(candidate.action_id, candidate);
    }

    let mut expected = BTreeMap::<Vec<[u8; 32]>, u64>::new();
    let subset_count = 1_usize
        .checked_shl(config.candidates.len() as u32)
        .ok_or(JointActionCatalogueError::TooManyActions)?;
    for mask in 0..subset_count {
        let mut action_ids = Vec::new();
        let mut resource_cost = 0_u64;
        let mut statically_selectable = true;
        for (index, candidate) in config.candidates.iter().enumerate() {
            if (mask & (1 << index)) == 0 {
                continue;
            }
            if !candidate.actionable {
                statically_selectable = false;
                break;
            }
            resource_cost = resource_cost
                .checked_add(candidate.resource_cost)
                .ok_or(JointActionCatalogueError::ResourceArithmeticOverflow)?;
            action_ids.push(candidate.action_id);
        }
        if !statically_selectable || resource_cost > config.resource_capacity {
            continue;
        }
        action_ids.sort_unstable();
        expected.insert(action_ids, resource_cost);
    }

    let mut outcome_by_set = BTreeMap::new();
    let mut action_set_digests = BTreeSet::new();
    for outcome in &config.outcomes {
        if is_zero_digest(&outcome.action_set_digest) {
            return Err(JointActionCatalogueError::InvalidIdentity);
        }
        if !action_set_digests.insert(outcome.action_set_digest) {
            return Err(JointActionCatalogueError::DuplicateActionSetDigest);
        }
        if outcome.action_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(JointActionCatalogueError::NonCanonicalActionSet);
        }
        if outcome.action_ids.iter().any(|id| !candidates.contains_key(id)) {
            return Err(JointActionCatalogueError::UnknownAction);
        }
        if !expected.contains_key(&outcome.action_ids) {
            return Err(JointActionCatalogueError::UnexpectedCoalitionOutcome);
        }
        if outcome_by_set.insert(outcome.action_ids.clone(), outcome).is_some() {
            return Err(JointActionCatalogueError::DuplicateCoalitionOutcome);
        }
    }
    if outcome_by_set.len() != expected.len()
        || expected.keys().any(|action_ids| !outcome_by_set.contains_key(action_ids))
    {
        return Err(JointActionCatalogueError::MissingCoalitionOutcome);
    }

    let mut report = JointActionCatalogueReport {
        disposition: JointActionSelectionDisposition::NoSafePortfolio,
        selected_action_ids: None,
        selected_action_set_digest: None,
        selected_resource_cost: None,
        selected_robust_consequence_avoided_ticks: None,
        expected_coalition_count: expected.len(),
        qualified_safe_count: 0,
        threshold_exceeded_count: 0,
        threshold_ambiguous_count: 0,
        incomplete_evidence_count: 0,
        infeasible_count: 0,
        unresolved_count: 0,
        statically_excluded_count: subset_count - expected.len(),
    };
    let mut best: Option<(Vec<[u8; 32]>, [u8; 32], u64, u64)> = None;
    let mut must_abstain = false;

    for (action_ids, resource_cost) in &expected {
        let outcome = outcome_by_set
            .get(action_ids)
            .ok_or(JointActionCatalogueError::MissingCoalitionOutcome)?;
        match &outcome.state {
            JointActionOutcomeState::Infeasible { reason_digest } => {
                if is_zero_digest(reason_digest) {
                    return Err(JointActionCatalogueError::InvalidReasonDigest);
                }
                report.infeasible_count += 1;
            }
            JointActionOutcomeState::Unresolved { reason_digest } => {
                if is_zero_digest(reason_digest) {
                    return Err(JointActionCatalogueError::InvalidReasonDigest);
                }
                report.unresolved_count += 1;
                must_abstain = true;
            }
            JointActionOutcomeState::Qualified {
                post_action_snapshot,
                consequence_model_digest,
                robust_consequence_avoided_ticks,
                coordination_digest,
            } => {
                if *consequence_model_digest != config.consequence_model_digest {
                    return Err(JointActionCatalogueError::ConsequenceModelMismatch);
                }
                let mut operators = BTreeSet::new();
                let mut coordination_required = false;
                for action_id in action_ids {
                    let candidate = candidates
                        .get(action_id)
                        .ok_or(JointActionCatalogueError::UnknownAction)?;
                    operators.insert(candidate.operator_digest);
                    coordination_required |= candidate.coordination_required;
                }
                coordination_required |= operators.len() > 1;
                if action_ids.is_empty()
                    && (post_action_snapshot != &config.baseline_snapshot
                        || *robust_consequence_avoided_ticks != 0)
                {
                    return Err(JointActionCatalogueError::InvalidNoActionControl);
                }

                let receipt = ActionEffectReceipt {
                    action_digest: outcome.action_set_digest,
                    baseline_snapshot_digest: config.baseline_snapshot.snapshot_digest,
                    post_action_snapshot_digest: post_action_snapshot.snapshot_digest,
                    post_action_ephemeris_digest: post_action_snapshot.ephemeris_digest,
                    secondary_maneuverable: coordination_required,
                    coordination_digest: *coordination_digest,
                };
                let reconciliation = reconcile_action_effects(
                    &config.baseline_snapshot,
                    post_action_snapshot,
                    receipt,
                    config.probability_threshold,
                )?;

                // A catalog revision change makes consequence deltas incomparable,
                // even when the post-action event inventory is itself complete.
                if reconciliation.catalog_revision_changed {
                    report.incomplete_evidence_count += 1;
                    must_abstain = true;
                    continue;
                }

                match reconciliation.disposition {
                    ActionPortfolioDisposition::PostActionBelowThreshold => {
                        report.qualified_safe_count += 1;
                        let candidate = (
                            action_ids.clone(),
                            outcome.action_set_digest,
                            *resource_cost,
                            *robust_consequence_avoided_ticks,
                        );
                        if is_better(&candidate, best.as_ref()) {
                            best = Some(candidate);
                        }
                    }
                    ActionPortfolioDisposition::ThresholdExceeded => {
                        report.threshold_exceeded_count += 1;
                    }
                    ActionPortfolioDisposition::Indeterminate => {
                        report.threshold_ambiguous_count += 1;
                        must_abstain = true;
                    }
                    ActionPortfolioDisposition::IncompleteEvidence => {
                        report.incomplete_evidence_count += 1;
                        must_abstain = true;
                    }
                }
            }
        }
    }

    if must_abstain {
        report.disposition = JointActionSelectionDisposition::AbstainIncompleteEvidence;
        return Ok(report);
    }
    if let Some((action_ids, digest, resource_cost, avoided)) = best {
        report.disposition = JointActionSelectionDisposition::QualifiedOptimum;
        report.selected_action_ids = Some(action_ids);
        report.selected_action_set_digest = Some(digest);
        report.selected_resource_cost = Some(resource_cost);
        report.selected_robust_consequence_avoided_ticks = Some(avoided);
    }
    Ok(report)
}

fn is_better(
    candidate: &(Vec<[u8; 32]>, [u8; 32], u64, u64),
    current: Option<&(Vec<[u8; 32]>, [u8; 32], u64, u64)>,
) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate.3 > current.3
        || (candidate.3 == current.3
            && (candidate.2 < current.2
                || (candidate.2 == current.2 && candidate.0 < current.0)))
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_effect_portfolio::ConjunctionEventProbabilityEnvelope;
    use crate::space_time_partition::NonnegativeRationalInterval;

    const BASELINE: [u8; 32] = [1; 32];
    const CATALOG: [u8; 32] = [2; 32];
    const EPHEMERIS: [u8; 32] = [3; 32];
    const SCOPE: [u8; 32] = [4; 32];
    const RISK: [u8; 32] = [5; 32];
    const CONSEQUENCE: [u8; 32] = [6; 32];
    const OPERATOR_A: [u8; 32] = [7; 32];
    const OPERATOR_B: [u8; 32] = [8; 32];

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn probability(
        lower: (u128, u128),
        upper: (u128, u128),
    ) -> NonnegativeRationalInterval {
        NonnegativeRationalInterval {
            lower: rational(lower.0, lower.1),
            upper: rational(upper.0, upper.1),
        }
    }

    fn event(
        id: u8,
        lower: (u128, u128),
        upper: (u128, u128),
    ) -> ConjunctionEventProbabilityEnvelope {
        ConjunctionEventProbabilityEnvelope {
            event_id: [id; 32],
            assessment_digest: [id + 20; 32],
            probability: probability(lower, upper),
        }
    }

    fn snapshot(
        snapshot_id: [u8; 32],
        ephemeris: [u8; 32],
        events: Vec<ConjunctionEventProbabilityEnvelope>,
    ) -> ConjunctionPortfolioSnapshot {
        ConjunctionPortfolioSnapshot {
            snapshot_digest: snapshot_id,
            catalog_revision_digest: CATALOG,
            ephemeris_digest: ephemeris,
            screening_scope_digest: SCOPE,
            risk_semantics_digest: RISK,
            inventory_complete: true,
            full_catalog_screened: true,
            events,
        }
    }

    fn baseline() -> ConjunctionPortfolioSnapshot {
        snapshot(
            BASELINE,
            EPHEMERIS,
            vec![event(10, (1, 1_000_000), (2, 1_000_000))],
        )
    }

    fn candidate(id: u8, cost: u64, operator: [u8; 32]) -> JointActionCandidate {
        JointActionCandidate {
            action_id: [id; 32],
            resource_cost: cost,
            operator_digest: operator,
            actionable: true,
            coordination_required: false,
        }
    }

    fn qualified(
        action_ids: Vec<[u8; 32]>,
        id: u8,
        post_events: Vec<ConjunctionEventProbabilityEnvelope>,
        avoided: u64,
    ) -> JointActionOutcomeRecord {
        let post = if action_ids.is_empty() {
            baseline()
        } else {
            snapshot([id + 40; 32], [id + 50; 32], post_events)
        };
        JointActionOutcomeRecord {
            action_ids,
            action_set_digest: [id + 60; 32],
            state: JointActionOutcomeState::Qualified {
                post_action_snapshot: post,
                consequence_model_digest: CONSEQUENCE,
                robust_consequence_avoided_ticks: avoided,
                coordination_digest: None,
            },
        }
    }

    fn config(
        candidates: Vec<JointActionCandidate>,
        outcomes: Vec<JointActionOutcomeRecord>,
        capacity: u64,
    ) -> JointActionCatalogueConfig {
        JointActionCatalogueConfig {
            baseline_snapshot: baseline(),
            candidates,
            outcomes,
            consequence_model_digest: CONSEQUENCE,
            probability_threshold: rational(1, 10_000),
            resource_capacity: capacity,
        }
    }

    #[test]
    fn exact_catalogue_selects_the_highest_robust_safe_joint_outcome() {
        let a = [10; 32];
        let b = [11; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(vec![a], 2, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 4),
            qualified(vec![b], 3, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 7),
            qualified(
                vec![a, b],
                4,
                vec![event(10, (1, 3_000_000), (2, 3_000_000))],
                12,
            ),
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A), candidate(11, 1, OPERATOR_A)],
            outcomes,
            2,
        ))
        .unwrap();
        assert_eq!(report.disposition, JointActionSelectionDisposition::QualifiedOptimum);
        assert_eq!(report.selected_action_ids, Some(vec![a, b]));
        assert_eq!(report.selected_robust_consequence_avoided_ticks, Some(12));
        assert_eq!(report.expected_coalition_count, 4);
        assert_eq!(report.qualified_safe_count, 4);
    }

    #[test]
    fn individually_safe_actions_do_not_hide_a_jointly_created_threshold_event() {
        let a = [10; 32];
        let b = [11; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(vec![a], 2, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 8),
            qualified(vec![b], 3, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 7),
            qualified(
                vec![a, b],
                4,
                vec![
                    event(10, (1, 2_000_000), (2, 2_000_000)),
                    event(12, (2, 10_000), (3, 10_000)),
                ],
                100,
            ),
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A), candidate(11, 1, OPERATOR_A)],
            outcomes,
            2,
        ))
        .unwrap();
        assert_eq!(report.disposition, JointActionSelectionDisposition::QualifiedOptimum);
        assert_eq!(report.selected_action_ids, Some(vec![a]));
        assert_eq!(report.selected_robust_consequence_avoided_ticks, Some(8));
        assert_eq!(report.threshold_exceeded_count, 1);
    }

    #[test]
    fn duplicate_rows_for_the_same_coalition_are_rejected() {
        let a = [10; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(vec![a], 2, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 50),
            JointActionOutcomeRecord {
                action_ids: vec![a],
                action_set_digest: [90; 32],
                state: JointActionOutcomeState::Unresolved { reason_digest: [91; 32] },
            },
        ];
        let result = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A)],
            outcomes,
            1,
        ));
        assert_eq!(result, Err(JointActionCatalogueError::DuplicateCoalitionOutcome));
    }

    #[test]
    fn missing_selectable_coalition_row_prevents_an_optimality_claim() {
        let a = [10; 32];
        let outcomes = vec![qualified(vec![], 1, vec![], 0)];
        assert_eq!(
            evaluate_joint_action_catalogue(config(
                vec![candidate(10, 1, OPERATOR_A)],
                outcomes,
                1,
            )),
            Err(JointActionCatalogueError::MissingCoalitionOutcome)
        );
    }

    #[test]
    fn unresolved_outcome_is_explicit_and_prevents_selection() {
        let a = [10; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            JointActionOutcomeRecord {
                action_ids: vec![a],
                action_set_digest: [90; 32],
                state: JointActionOutcomeState::Unresolved { reason_digest: [91; 32] },
            },
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A)],
            outcomes,
            1,
        ))
        .unwrap();
        assert_eq!(
            report.disposition,
            JointActionSelectionDisposition::AbstainIncompleteEvidence
        );
        assert_eq!(report.selected_action_ids, None);
        assert_eq!(report.unresolved_count, 1);
    }

    #[test]
    fn statically_unactionable_and_over_capacity_subsets_need_no_outcome_row() {
        let mut unavailable = candidate(11, 1, OPERATOR_A);
        unavailable.actionable = false;
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(
                vec![[10; 32]],
                2,
                vec![event(10, (1, 2_000_000), (2, 2_000_000))],
                5,
            ),
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A), unavailable, candidate(12, 3, OPERATOR_A)],
            outcomes,
            1,
        ))
        .unwrap();
        assert_eq!(report.expected_coalition_count, 2);
        assert_eq!(report.statically_excluded_count, 6);
        assert_eq!(report.selected_action_ids, Some(vec![[10; 32]]));
    }

    #[test]
    fn cross_operator_joint_actions_require_coordination_evidence() {
        let a = [10; 32];
        let b = [11; 32];
        let mut joint = qualified(
            vec![a, b],
            4,
            vec![event(10, (1, 3_000_000), (2, 3_000_000))],
            12,
        );
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(vec![a], 2, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 4),
            qualified(vec![b], 3, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 7),
            joint.clone(),
        ];
        let result = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A), candidate(11, 1, OPERATOR_B)],
            outcomes.clone(),
            2,
        ));
        let incomplete = result.unwrap();
        assert_eq!(
            incomplete.disposition,
            JointActionSelectionDisposition::AbstainIncompleteEvidence
        );
        assert_eq!(incomplete.selected_action_ids, None);
        assert_eq!(incomplete.incomplete_evidence_count, 1);

        if let JointActionOutcomeState::Qualified { coordination_digest, .. } = &mut joint.state {
            *coordination_digest = Some([92; 32]);
        }
        let mut coordinated = outcomes;
        coordinated[3] = joint;
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A), candidate(11, 1, OPERATOR_B)],
            coordinated,
            2,
        ))
        .unwrap();
        assert_eq!(report.selected_action_ids, Some(vec![a, b]));
    }

    #[test]
    fn consequence_model_mismatch_is_rejected() {
        let a = [10; 32];
        let mut row = qualified(
            vec![a],
            2,
            vec![event(10, (1, 2_000_000), (2, 2_000_000))],
            5,
        );
        if let JointActionOutcomeState::Qualified { consequence_model_digest, .. } = &mut row.state {
            *consequence_model_digest = [99; 32];
        }
        assert_eq!(
            evaluate_joint_action_catalogue(config(
                vec![candidate(10, 1, OPERATOR_A)],
                vec![qualified(vec![], 1, vec![], 0), row],
                1,
            )),
            Err(JointActionCatalogueError::ConsequenceModelMismatch)
        );
    }

    #[test]
    fn tied_objectives_use_lower_cost_then_canonical_action_order() {
        let a = [10; 32];
        let b = [11; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(vec![a], 2, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 5),
            qualified(vec![b], 3, vec![event(10, (1, 2_000_000), (2, 2_000_000))], 5),
            qualified(
                vec![a, b],
                4,
                vec![event(10, (1, 3_000_000), (2, 3_000_000))],
                5,
            ),
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(11, 1, OPERATOR_A), candidate(10, 1, OPERATOR_A)],
            outcomes,
            2,
        ))
        .unwrap();
        assert_eq!(report.selected_action_ids, Some(vec![a]));
    }

    #[test]
    fn threshold_overlap_for_one_joint_outcome_forces_global_abstention() {
        let a = [10; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            qualified(
                vec![a],
                2,
                vec![event(10, (0, 1), (1, 10_000))],
                50,
            ),
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A)],
            outcomes,
            1,
        ))
        .unwrap();
        assert_eq!(
            report.disposition,
            JointActionSelectionDisposition::AbstainIncompleteEvidence
        );
        assert_eq!(report.threshold_ambiguous_count, 1);
        assert_eq!(report.selected_action_ids, None);
    }

    #[test]
    fn catalog_revision_drift_prevents_a_global_optimum_claim() {
        let a = [10; 32];
        let mut row = qualified(
            vec![a],
            2,
            vec![event(10, (1, 2_000_000), (2, 2_000_000))],
            50,
        );
        if let JointActionOutcomeState::Qualified { post_action_snapshot, .. } = &mut row.state {
            post_action_snapshot.catalog_revision_digest = [99; 32];
        }
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A)],
            vec![qualified(vec![], 1, vec![], 0), row],
            1,
        ))
        .unwrap();
        assert_eq!(
            report.disposition,
            JointActionSelectionDisposition::AbstainIncompleteEvidence
        );
        assert_eq!(report.incomplete_evidence_count, 1);
        assert_eq!(report.selected_action_ids, None);
    }

    #[test]
    fn a_fully_assessed_infeasible_coalition_does_not_create_false_uncertainty() {
        let a = [10; 32];
        let outcomes = vec![
            qualified(vec![], 1, vec![], 0),
            JointActionOutcomeRecord {
                action_ids: vec![a],
                action_set_digest: [90; 32],
                state: JointActionOutcomeState::Infeasible { reason_digest: [91; 32] },
            },
        ];
        let report = evaluate_joint_action_catalogue(config(
            vec![candidate(10, 1, OPERATOR_A)],
            outcomes,
            1,
        ))
        .unwrap();
        assert_eq!(report.disposition, JointActionSelectionDisposition::QualifiedOptimum);
        assert_eq!(report.selected_action_ids, Some(vec![]));
        assert_eq!(report.infeasible_count, 1);
    }

}
