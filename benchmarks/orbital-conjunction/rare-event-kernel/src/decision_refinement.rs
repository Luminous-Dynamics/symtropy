// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Decision-gated orchestration of threshold assessment and refinement proposals.
//!
//! The controller evaluates a caller-supplied risk envelope first. It proposes
//! refinement work only while that envelope overlaps the declared action
//! threshold. A determinate result produces no further work order; unresolved
//! cases preserve the scheduler's complete candidate ledger and explicitly
//! abstain when there is no budget or no selectable work.
//!
//! This is a planning state machine, not a refinement executor. A proposed
//! work order does not shrink a risk interval, complete a candidate, qualify
//! physical integration, or authorize a maneuver.

use crate::decision_gate::{
    DecisionGateError, DecisionSufficiency, DecisionSufficiencyReport, RiskThresholdRule,
    assess_risk_threshold,
};
use crate::refinement_scheduler::{
    RefinementCandidate, RefinementScheduleError, RefinementScheduleReport, schedule_refinements,
};
use crate::space_time_partition::{Rational, SpaceTimeScope, NonnegativeRationalInterval};

/// Controller outcome after threshold classification and possible scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionRefinementDisposition {
    /// The complete supplied interval satisfies the action-threshold rule.
    ThresholdMet,
    /// The complete supplied interval does not satisfy the action-threshold rule.
    ThresholdNotMet,
    /// One or more work orders were proposed; every candidate remains unresolved.
    RefinementWorkProposed,
    /// The unresolved assessment has no compute budget available.
    AbstainBudgetExhausted,
    /// The unresolved assessment has no candidate cells to refine.
    AbstainNoCandidates,
    /// Candidates remain unresolved, but the scheduler selected no work.
    AbstainNoWorkSelected,
}

/// Replayable result of one decision-gated refinement planning step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRefinementReport {
    /// Exact threshold evaluation performed before any scheduling.
    pub decision: DecisionSufficiencyReport,
    /// Controller outcome. Abstention is distinct from a threshold decision.
    pub disposition: DecisionRefinementDisposition,
    /// Present only for indeterminate decisions; it retains all unresolved IDs.
    pub schedule: Option<RefinementScheduleReport>,
}

/// Errors from decision-gated refinement planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionRefinementError {
    /// The caller-supplied risk interval could not be classified.
    DecisionGate(DecisionGateError),
    /// The refinement candidate set failed scheduler validation.
    Scheduler(RefinementScheduleError),
}

impl core::fmt::Display for DecisionRefinementError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DecisionGate(error) => write!(formatter, "decision gate: {error}"),
            Self::Scheduler(error) => write!(formatter, "refinement scheduler: {error}"),
        }
    }
}

impl std::error::Error for DecisionRefinementError {}

/// Evaluate a risk envelope and propose refinement only when the decision is unresolved.
///
/// If the supplied interval is wholly on one side of the threshold, the
/// controller returns that result with no schedule. If it straddles the
/// threshold, the scheduler is invoked and its complete unresolved ledger is
/// retained. Zero budget, an empty candidate list, or no selected candidate
/// yields a distinct abstention disposition rather than a binary decision.
///
/// All candidate enclosures and costs remain caller-supplied assumptions. The
/// controller does not verify physical completeness, estimate refinement
/// success probabilities, execute work, or update the risk interval.
pub fn plan_decision_refinement(
    scope: SpaceTimeScope,
    risk_interval: NonnegativeRationalInterval,
    threshold: Rational,
    rule: RiskThresholdRule,
    compute_budget: Rational,
    candidates: &[RefinementCandidate],
) -> Result<DecisionRefinementReport, DecisionRefinementError> {
    let decision = assess_risk_threshold(risk_interval, threshold, rule)
        .map_err(DecisionRefinementError::DecisionGate)?;

    match decision.disposition {
        DecisionSufficiency::ThresholdMet => Ok(DecisionRefinementReport {
            decision,
            disposition: DecisionRefinementDisposition::ThresholdMet,
            schedule: None,
        }),
        DecisionSufficiency::ThresholdNotMet => Ok(DecisionRefinementReport {
            decision,
            disposition: DecisionRefinementDisposition::ThresholdNotMet,
            schedule: None,
        }),
        DecisionSufficiency::Indeterminate => {
            let schedule = schedule_refinements(scope, compute_budget, candidates)
                .map_err(DecisionRefinementError::Scheduler)?;

            let disposition = if candidates.is_empty() {
                DecisionRefinementDisposition::AbstainNoCandidates
            } else if compute_budget.numerator() == 0 {
                DecisionRefinementDisposition::AbstainBudgetExhausted
            } else if schedule.selected_candidate_ids.is_empty() {
                DecisionRefinementDisposition::AbstainNoWorkSelected
            } else {
                DecisionRefinementDisposition::RefinementWorkProposed
            };

            Ok(DecisionRefinementReport {
                decision,
                disposition,
                schedule: Some(schedule),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refinement_scheduler::{RefinementCandidateId};
    use crate::space_time_partition::NonnegativeRationalInterval;

    fn scope() -> SpaceTimeScope {
        SpaceTimeScope {
            event_digest: [1; 32],
            model_digest: [2; 32],
            target_distribution_digest: [3; 32],
        }
    }

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn interval(lower: Rational, upper: Rational) -> NonnegativeRationalInterval {
        NonnegativeRationalInterval { lower, upper }
    }

    fn candidate(
        cell_id: u64,
        width: u128,
        cost: u128,
        candidate_scope: SpaceTimeScope,
    ) -> RefinementCandidate {
        RefinementCandidate {
            scope: candidate_scope,
            id: RefinementCandidateId {
                cell_id,
                parent_cell_id: None,
                generation: 0,
            },
            contribution_bounds: interval(Rational::from_integer(0), Rational::from_integer(width)),
            estimated_refinement_cost: Rational::from_integer(cost),
        }
    }

    #[test]
    fn determinate_threshold_met_result_does_not_schedule_more_work() {
        let candidate = candidate(1, 10, 1, scope());
        let report = plan_decision_refinement(
            scope(),
            interval(rational(1, 10), rational(1, 5)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
            Rational::from_integer(1),
            &[candidate],
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionRefinementDisposition::ThresholdMet);
        assert!(report.schedule.is_none());
    }

    #[test]
    fn determinate_threshold_not_met_result_does_not_schedule_more_work() {
        let candidate = candidate(1, 10, 1, scope());
        let report = plan_decision_refinement(
            scope(),
            interval(rational(1, 20), rational(9, 100)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
            Rational::from_integer(1),
            &[candidate],
        )
        .unwrap();

        assert_eq!(
            report.disposition,
            DecisionRefinementDisposition::ThresholdNotMet
        );
        assert!(report.schedule.is_none());
    }

    #[test]
    fn indeterminate_result_proposes_work_without_clearing_unresolved_cells() {
        let candidate = candidate(1, 2, 1, scope());
        let report = plan_decision_refinement(
            scope(),
            interval(rational(9, 100), rational(11, 100)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
            Rational::from_integer(1),
            &[candidate],
        )
        .unwrap();

        assert_eq!(
            report.disposition,
            DecisionRefinementDisposition::RefinementWorkProposed
        );
        let schedule = report.schedule.unwrap();
        assert_eq!(schedule.selected_candidate_ids, [candidate.id]);
        assert_eq!(schedule.unresolved_candidate_ids, [candidate.id]);
    }

    #[test]
    fn zero_budget_abstains_and_preserves_the_complete_unresolved_ledger() {
        let candidate = candidate(1, 2, 1, scope());
        let report = plan_decision_refinement(
            scope(),
            interval(rational(9, 100), rational(11, 100)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
            Rational::from_integer(0),
            &[candidate],
        )
        .unwrap();

        assert_eq!(
            report.disposition,
            DecisionRefinementDisposition::AbstainBudgetExhausted
        );
        let schedule = report.schedule.unwrap();
        assert!(schedule.selected_candidate_ids.is_empty());
        assert_eq!(schedule.unresolved_candidate_ids, [candidate.id]);
    }

    #[test]
    fn empty_candidate_set_abstains_instead_of_claiming_decision_sufficiency() {
        let report = plan_decision_refinement(
            scope(),
            interval(rational(9, 100), rational(11, 100)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
            Rational::from_integer(1),
            &[],
        )
        .unwrap();

        assert_eq!(
            report.disposition,
            DecisionRefinementDisposition::AbstainNoCandidates
        );
        assert!(report.schedule.unwrap().unresolved_candidate_ids.is_empty());
    }

    #[test]
    fn no_affordable_or_uncertain_work_abstains_without_dropping_candidates() {
        let candidate = candidate(1, 2, 2, scope());
        let report = plan_decision_refinement(
            scope(),
            interval(rational(9, 100), rational(11, 100)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
            Rational::from_integer(1),
            &[candidate],
        )
        .unwrap();

        assert_eq!(
            report.disposition,
            DecisionRefinementDisposition::AbstainNoWorkSelected
        );
        let schedule = report.schedule.unwrap();
        assert_eq!(schedule.unresolved_candidate_ids, [candidate.id]);
        assert!(schedule.selected_candidate_ids.is_empty());
    }

    #[test]
    fn stale_candidate_scope_fails_closed_for_an_indeterminate_assessment() {
        let mut stale_scope = scope();
        stale_scope.model_digest = [9; 32];
        let candidate = candidate(1, 2, 1, stale_scope);

        assert_eq!(
            plan_decision_refinement(
                scope(),
                interval(rational(9, 100), rational(11, 100)),
                rational(1, 10),
                RiskThresholdRule::AtOrAbove,
                Rational::from_integer(1),
                &[candidate],
            ),
            Err(DecisionRefinementError::Scheduler(
                RefinementScheduleError::ScopeMismatch
            ))
        );
    }
}
