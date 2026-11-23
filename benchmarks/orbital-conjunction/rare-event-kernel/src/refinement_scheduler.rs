// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic uncertainty-per-cost scheduling for unresolved risk cells.
//!
//! This module implements a transparent exact-rational baseline for the
//! value-of-information research: contribution-interval width divided by an
//! explicitly estimated refinement cost. The schedule is only a work-order
//! proposal. Selecting a candidate does not evaluate it, shrink its residual,
//! or relax any downstream qualification gate.

use std::collections::BTreeSet;

use crate::space_time_partition::{
    NonnegativeRationalInterval, Rational, SpaceTimePartitionError, SpaceTimeScope,
};

/// Stable identity and lineage for one unresolved refinement candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RefinementCandidateId {
    /// Stable identifier of this unresolved cell.
    pub cell_id: u64,
    /// Parent cell identifier, or absent when this is a root cell.
    pub parent_cell_id: Option<u64>,
    /// Refinement generation, with zero reserved for root cells.
    pub generation: u32,
}

/// One unresolved cell and its current contribution uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefinementCandidate {
    /// Event/model/target identity to which this candidate belongs.
    pub scope: SpaceTimeScope,
    /// Stable cell identity and parent lineage.
    pub id: RefinementCandidateId,
    /// Current contribution enclosure in the declared common contribution unit.
    pub contribution_bounds: NonnegativeRationalInterval,
    /// Estimated compute cost for one refinement attempt, in the caller's
    /// declared common cost unit. This is a scheduling estimate, not a
    /// measured or qualified cost.
    pub estimated_refinement_cost: Rational,
}

/// One candidate ranked by exact contribution-width-per-cost priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankedRefinementCandidate {
    /// Original unresolved candidate and its complete identity.
    pub candidate: RefinementCandidate,
    /// Exact current contribution interval width.
    pub contribution_width: Rational,
    /// Exact baseline priority: contribution width divided by estimated cost.
    pub priority: Rational,
    /// True when the candidate fits in the remaining greedy budget.
    ///
    /// A selected candidate is still unresolved until a scope-matched child
    /// partition and its independent verification receipt are received.
    pub selected_for_refinement: bool,
}

/// Deterministic scheduling proposal with a complete unresolved-candidate ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinementScheduleReport {
    /// Event/model/target identity shared by every scheduled candidate.
    pub scope: SpaceTimeScope,
    /// Total compute budget supplied by the caller.
    pub compute_budget: Rational,
    /// Sum of estimated costs for the selected work orders.
    pub selected_estimated_cost: Rational,
    /// Budget left after selecting affordable candidates.
    pub remaining_budget: Rational,
    /// All input candidates, sorted by descending priority and stable identity.
    pub ranked_candidates: Vec<RankedRefinementCandidate>,
    /// Selected work orders in deterministic priority order.
    pub selected_candidate_ids: Vec<RefinementCandidateId>,
    /// Every candidate remains unresolved, including selected work orders.
    ///
    /// This list is intentionally not reduced by scheduling. Completion
    /// requires a separate validated refinement result and parent/child
    /// partition-conservation check.
    pub unresolved_candidate_ids: Vec<RefinementCandidateId>,
}

/// Fail-closed errors from the deterministic refinement scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefinementScheduleError {
    /// A candidate belongs to a different event/model/target scope.
    ScopeMismatch,
    /// Two candidates reuse the same stable identity.
    DuplicateCandidateIdentity,
    /// A contribution interval is reversed.
    InvalidContributionInterval,
    /// A refinement cost is zero or cannot be used as a divisor.
    InvalidRefinementCost,
    /// Exact priority or budget arithmetic exceeds the rational representation.
    ArithmeticOverflow,
}

impl core::fmt::Display for RefinementScheduleError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::ScopeMismatch => "refinement candidate event/model/target scope mismatch",
            Self::DuplicateCandidateIdentity => "refinement candidate identity is duplicated",
            Self::InvalidContributionInterval => "refinement contribution interval is reversed",
            Self::InvalidRefinementCost => "estimated refinement cost must be positive",
            Self::ArithmeticOverflow => "exact refinement scheduling arithmetic overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RefinementScheduleError {}

/// Rank unresolved candidates by exact contribution-width-per-cost priority.
///
/// Candidates are considered in descending priority order, with stable cell
/// identity as the deterministic tie-break. A candidate whose cost exceeds the
/// remaining budget is skipped rather than blocking later, cheaper candidates.
/// Zero-width candidates are retained in the ledger but are not selected.
///
/// The function does not mutate candidates, mark selected work as completed,
/// remove unresolved residuals, or certify the correctness of the supplied
/// enclosures or cost estimates.
pub fn schedule_refinements(
    scope: SpaceTimeScope,
    compute_budget: Rational,
    candidates: &[RefinementCandidate],
) -> Result<RefinementScheduleReport, RefinementScheduleError> {
    let mut identities = BTreeSet::new();
    let mut ranked = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        if candidate.scope != scope {
            return Err(RefinementScheduleError::ScopeMismatch);
        }
        if !identities.insert(candidate.id) {
            return Err(RefinementScheduleError::DuplicateCandidateIdentity);
        }
        if candidate.contribution_bounds.lower > candidate.contribution_bounds.upper {
            return Err(RefinementScheduleError::InvalidContributionInterval);
        }
        if candidate.estimated_refinement_cost.numerator() == 0 {
            return Err(RefinementScheduleError::InvalidRefinementCost);
        }

        let contribution_width = candidate
            .contribution_bounds
            .upper
            .checked_sub(candidate.contribution_bounds.lower)
            .map_err(|_| RefinementScheduleError::InvalidContributionInterval)?;
        let priority = contribution_width
            .checked_div(candidate.estimated_refinement_cost)
            .map_err(map_rational_error)?;

        ranked.push(RankedRefinementCandidate {
            candidate: *candidate,
            contribution_width,
            priority,
            selected_for_refinement: false,
        });
    }

    ranked.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.candidate.id.cmp(&right.candidate.id))
    });

    let mut remaining_budget = compute_budget;
    let mut selected_estimated_cost = Rational::from_integer(0);
    let mut selected_candidate_ids = Vec::new();

    for ranked_candidate in &mut ranked {
        let has_uncertainty = ranked_candidate.contribution_width.numerator() != 0;
        let cost = ranked_candidate.candidate.estimated_refinement_cost;
        if has_uncertainty && cost <= remaining_budget {
            remaining_budget = remaining_budget
                .checked_sub(cost)
                .map_err(|_| RefinementScheduleError::ArithmeticOverflow)?;
            selected_estimated_cost = selected_estimated_cost
                .checked_add(cost)
                .map_err(|_| RefinementScheduleError::ArithmeticOverflow)?;
            ranked_candidate.selected_for_refinement = true;
            selected_candidate_ids.push(ranked_candidate.candidate.id);
        }
    }

    // Scheduling is not evaluation: all candidate identities remain in the
    // unresolved ledger, including those selected for a refinement attempt.
    let unresolved_candidate_ids = ranked
        .iter()
        .map(|candidate| candidate.candidate.id)
        .collect();

    Ok(RefinementScheduleReport {
        scope,
        compute_budget,
        selected_estimated_cost,
        remaining_budget,
        ranked_candidates: ranked,
        selected_candidate_ids,
        unresolved_candidate_ids,
    })
}

fn map_rational_error(error: SpaceTimePartitionError) -> RefinementScheduleError {
    match error {
        SpaceTimePartitionError::ArithmeticOverflow => RefinementScheduleError::ArithmeticOverflow,
        SpaceTimePartitionError::InvalidRationalDivision => {
            RefinementScheduleError::InvalidRefinementCost
        }
        _ => RefinementScheduleError::InvalidContributionInterval,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn candidate(cell_id: u64, width: u128, cost: u128) -> RefinementCandidate {
        let (parent_cell_id, generation) = if cell_id > 100 {
            (Some(cell_id - 100), 1)
        } else {
            (None, 0)
        };
        RefinementCandidate {
            scope: scope(),
            id: RefinementCandidateId {
                cell_id,
                parent_cell_id,
                generation,
            },
            contribution_bounds: NonnegativeRationalInterval {
                lower: Rational::from_integer(0),
                upper: Rational::from_integer(width),
            },
            estimated_refinement_cost: Rational::from_integer(cost),
        }
    }

    fn exhaustive_best_subset(
        candidates: &[RefinementCandidate],
        budget: Rational,
    ) -> (Rational, Rational, Vec<RefinementCandidateId>) {
        assert!(candidates.len() <= 20);
        let mut best_width = Rational::from_integer(0);
        let mut best_cost = Rational::from_integer(0);
        let mut best_ids = Vec::new();
        let subset_count = 1_u64 << candidates.len();

        for mask in 0..subset_count {
            let mut width = Rational::from_integer(0);
            let mut cost = Rational::from_integer(0);
            let mut ids = Vec::new();
            let mut feasible = true;

            for (index, candidate) in candidates.iter().enumerate() {
                if mask & (1_u64 << index) != 0 {
                    width = width
                        .checked_add(
                            candidate
                                .contribution_bounds
                                .upper
                                .checked_sub(candidate.contribution_bounds.lower)
                                .unwrap(),
                        )
                        .unwrap();
                    cost = cost
                        .checked_add(candidate.estimated_refinement_cost)
                        .unwrap();
                    ids.push(candidate.id);
                    if cost > budget {
                        feasible = false;
                        break;
                    }
                }
            }

            ids.sort();
            if feasible
                && (width > best_width
                    || (width == best_width
                        && (cost < best_cost || (cost == best_cost && ids < best_ids))))
            {
                best_width = width;
                best_cost = cost;
                best_ids = ids;
            }
        }

        (best_width, best_cost, best_ids)
    }

    fn selected_width(
        candidates: &[RefinementCandidate],
        selected: &[RefinementCandidateId],
    ) -> Rational {
        candidates
            .iter()
            .filter(|candidate| selected.contains(&candidate.id))
            .map(|candidate| {
                candidate
                    .contribution_bounds
                    .upper
                    .checked_sub(candidate.contribution_bounds.lower)
                    .unwrap()
            })
            .try_fold(Rational::from_integer(0), |total, width| {
                total.checked_add(width)
            })
            .unwrap()
    }

    #[test]
    fn priority_is_exact_contribution_width_divided_by_estimated_cost() {
        let candidates = [
            candidate(10, 10, 5),
            candidate(20, 5, 1),
            candidate(30, 7, 2),
        ];
        let report = schedule_refinements(scope(), Rational::from_integer(3), &candidates).unwrap();

        assert_eq!(
            report
                .ranked_candidates
                .iter()
                .map(|item| item.priority)
                .collect::<Vec<_>>(),
            [rational(5, 1), rational(7, 2), rational(2, 1)]
        );
        assert_eq!(
            report.selected_candidate_ids,
            [candidates[1].id, candidates[2].id]
        );
        assert_eq!(report.selected_estimated_cost, Rational::from_integer(3));
        assert_eq!(report.remaining_budget, Rational::from_integer(0));
    }

    #[test]
    fn unaffordable_high_priority_candidate_does_not_block_a_cheaper_one() {
        let expensive = candidate(1, 100, 10);
        let affordable = candidate(2, 5, 1);
        let report =
            schedule_refinements(scope(), Rational::from_integer(1), &[expensive, affordable])
                .unwrap();

        assert_eq!(report.selected_candidate_ids, [affordable.id]);
    }

    #[test]
    fn equal_priorities_use_stable_candidate_identity_as_tiebreak() {
        let larger_id = candidate(20, 4, 2);
        let smaller_id = candidate(10, 2, 1);
        let report =
            schedule_refinements(scope(), Rational::from_integer(3), &[larger_id, smaller_id])
                .unwrap();

        assert_eq!(
            report
                .ranked_candidates
                .iter()
                .map(|item| item.candidate.id)
                .collect::<Vec<_>>(),
            [smaller_id.id, larger_id.id]
        );
    }

    #[test]
    fn serialization_permutation_replays_the_same_schedule() {
        let first = candidate(10, 2, 1);
        let second = candidate(20, 4, 2);
        let third = candidate(30, 9, 3);
        let forward =
            schedule_refinements(scope(), Rational::from_integer(3), &[first, second, third])
                .unwrap();
        let reverse =
            schedule_refinements(scope(), Rational::from_integer(3), &[third, second, first])
                .unwrap();

        assert_eq!(forward, reverse);
    }

    #[test]
    fn selected_candidates_remain_in_the_unresolved_ledger() {
        let first = candidate(1, 10, 1);
        let second = candidate(2, 5, 1);
        let report =
            schedule_refinements(scope(), Rational::from_integer(1), &[first, second]).unwrap();

        assert_eq!(report.selected_candidate_ids, [first.id]);
        assert_eq!(report.unresolved_candidate_ids.len(), 2);
        assert!(report.unresolved_candidate_ids.contains(&first.id));
        assert!(report.unresolved_candidate_ids.contains(&second.id));
    }

    #[test]
    fn zero_width_candidate_is_retained_but_not_selected() {
        let zero_width = candidate(1, 0, 1);
        let report =
            schedule_refinements(scope(), Rational::from_integer(10), &[zero_width]).unwrap();

        assert!(report.selected_candidate_ids.is_empty());
        assert_eq!(report.unresolved_candidate_ids, [zero_width.id]);
    }

    #[test]
    fn zero_refinement_cost_fails_closed() {
        assert_eq!(
            schedule_refinements(scope(), Rational::from_integer(1), &[candidate(1, 1, 0)],),
            Err(RefinementScheduleError::InvalidRefinementCost)
        );
    }

    #[test]
    fn reversed_contribution_interval_fails_closed() {
        let mut invalid = candidate(1, 1, 1);
        invalid.contribution_bounds = NonnegativeRationalInterval {
            lower: Rational::from_integer(2),
            upper: Rational::from_integer(1),
        };
        assert_eq!(
            schedule_refinements(scope(), Rational::from_integer(1), &[invalid]),
            Err(RefinementScheduleError::InvalidContributionInterval)
        );
    }

    #[test]
    fn cross_scope_candidate_fails_closed() {
        let mut stale = candidate(1, 1, 1);
        stale.scope.model_digest = [9; 32];
        assert_eq!(
            schedule_refinements(scope(), Rational::from_integer(1), &[stale]),
            Err(RefinementScheduleError::ScopeMismatch)
        );
    }

    #[test]
    fn duplicate_candidate_identity_fails_closed() {
        let duplicate = candidate(1, 2, 1);
        assert_eq!(
            schedule_refinements(scope(), Rational::from_integer(2), &[duplicate, duplicate],),
            Err(RefinementScheduleError::DuplicateCandidateIdentity)
        );
    }

    #[test]
    fn zero_budget_selects_nothing_and_preserves_every_candidate() {
        let candidates = [candidate(1, 10, 1), candidate(2, 5, 2)];
        let report = schedule_refinements(scope(), Rational::from_integer(0), &candidates).unwrap();

        assert!(report.selected_candidate_ids.is_empty());
        assert_eq!(report.remaining_budget, Rational::from_integer(0));
        assert_eq!(report.unresolved_candidate_ids.len(), 2);
    }

    #[test]
    fn unrepresentable_priority_fails_closed_instead_of_wrapping() {
        let unrepresentable = RefinementCandidate {
            scope: scope(),
            id: RefinementCandidateId {
                cell_id: 1,
                parent_cell_id: None,
                generation: 0,
            },
            contribution_bounds: NonnegativeRationalInterval {
                lower: Rational::from_integer(0),
                upper: Rational::from_integer(2),
            },
            estimated_refinement_cost: rational(1, u128::MAX),
        };

        assert_eq!(
            schedule_refinements(scope(), Rational::from_integer(1), &[unrepresentable],),
            Err(RefinementScheduleError::ArithmeticOverflow)
        );
    }

    #[test]
    fn exhaustive_knapsack_oracle_quantifies_greedy_scheduler_regret() {
        // The highest width/cost item (6/4) is locally preferred, but the two
        // lower-ratio items (10/7 each) jointly produce a better budget fill.
        let candidates = [candidate(1, 6, 4), candidate(2, 10, 7), candidate(3, 10, 7)];
        let budget = Rational::from_integer(14);
        let greedy = schedule_refinements(scope(), budget, &candidates).unwrap();
        let greedy_width = selected_width(&candidates, &greedy.selected_candidate_ids);
        let (optimal_width, optimal_cost, optimal_ids) =
            exhaustive_best_subset(&candidates, budget);

        assert_eq!(greedy_width, Rational::from_integer(16));
        assert_eq!(greedy.selected_estimated_cost, Rational::from_integer(11));
        assert_eq!(optimal_width, Rational::from_integer(20));
        assert_eq!(optimal_cost, Rational::from_integer(14));
        assert_eq!(optimal_ids, [candidates[1].id, candidates[2].id]);
        assert_eq!(
            optimal_width.checked_sub(greedy_width).unwrap(),
            Rational::from_integer(4)
        );
        assert_eq!(
            optimal_width
                .checked_sub(greedy_width)
                .unwrap()
                .checked_div(optimal_width)
                .unwrap(),
            rational(1, 5)
        );
    }

    #[test]
    fn exact_oracle_never_selects_a_subset_over_budget() {
        let candidates = [
            candidate(1, 13, 8),
            candidate(2, 9, 6),
            candidate(3, 7, 5),
            candidate(4, 3, 2),
        ];
        let budget = Rational::from_integer(10);
        let (optimal_width, optimal_cost, ids) = exhaustive_best_subset(&candidates, budget);

        assert!(optimal_cost <= budget);
        assert_eq!(selected_width(&candidates, &ids), optimal_width);
    }

    #[test]
    fn empty_candidate_set_returns_an_empty_replayable_report() {
        let report = schedule_refinements(scope(), Rational::from_integer(5), &[]).unwrap();

        assert!(report.ranked_candidates.is_empty());
        assert!(report.selected_candidate_ids.is_empty());
        assert!(report.unresolved_candidate_ids.is_empty());
        assert_eq!(report.remaining_budget, Rational::from_integer(5));
    }
}
