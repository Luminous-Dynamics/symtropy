// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-fixture oracle for capacity-constrained mitigation portfolios.
//!
//! This test-oriented research kernel selects among caller-supplied mitigation
//! actions by an additive, common-unit expected-consequence-avoided objective,
//! subject to a finite resource budget, actionability gates and pairwise action
//! incompatibilities. It is an exact reference for small synthetic fixtures,
//! not a production maneuver planner.
//!
//! Collision probability is intentionally not used as the objective. Pc is a
//! likelihood metric; the consequence values are caller-supplied outputs from
//! a separate consequence model. The oracle does not validate that model or
//! verify post-maneuver safety. Incompatibility constraints must be supplied
//! from independently assessed feasibility/coordination evidence.

use std::collections::BTreeSet;

use crate::space_time_partition::Rational;

/// One candidate mitigation action in a frozen portfolio fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioAction {
    /// Stable action identity.
    pub action_id: u64,
    /// Caller-supplied expected consequence avoided in a shared unit.
    ///
    /// This is not Pc and is not calculated by this module. The caller must
    /// ensure all candidate values use the same consequence model and units.
    pub expected_consequence_avoided: Rational,
    /// Positive integer resource consumption for this action.
    pub resource_units: u64,
    /// Whether a separate input-quality/actionability gate admits the action.
    pub actionable: bool,
}

/// Pair of actions that cannot both be selected in one portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatibleActionPair {
    /// First action identity; endpoints are canonicalized in reports.
    pub first_action_id: u64,
    /// Second action identity; endpoints are canonicalized in reports.
    pub second_action_id: u64,
}

/// Exact optimal selection for a small finite mitigation portfolio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioOracleReport {
    /// Total resource capacity supplied by the caller.
    pub resource_capacity: u64,
    /// Resource units consumed by the selected actions.
    pub selected_resource_units: u64,
    /// Exact additive expected consequence avoided by the selected actions.
    pub total_expected_consequence_avoided: Rational,
    /// Selected action identities, sorted ascending for stable replay.
    pub selected_action_ids: Vec<u64>,
    /// Action identities excluded by the caller-supplied actionability gate.
    pub rejected_non_actionable_ids: Vec<u64>,
    /// Action identities not selected by the exact capacity/conflict optimum.
    /// This does not label the individual reason for non-selection.
    pub unselected_action_ids: Vec<u64>,
    /// Canonicalized pairwise incompatibilities included in the oracle input.
    pub incompatible_pairs: Vec<IncompatibleActionPair>,
}

/// Fail-closed validation or arithmetic errors from the portfolio oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioOracleError {
    /// More than 20 actions were supplied; exhaustive enumeration is bounded.
    TooManyActions,
    /// Two actions share an identity.
    DuplicateActionIdentity,
    /// An action consumes zero resource units.
    ZeroResourceCost,
    /// A conflict pair refers to the same action twice.
    SelfConflict,
    /// A conflict pair refers to an unknown action identity.
    UnknownConflictAction,
    /// A conflict pair is repeated, including with reversed endpoints.
    DuplicateConflictPair,
    /// Exact consequence addition or resource addition overflowed.
    ArithmeticOverflow,
}

impl core::fmt::Display for PortfolioOracleError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::TooManyActions => "exact portfolio oracle supports at most 20 actions",
            Self::DuplicateActionIdentity => "portfolio action identity is duplicated",
            Self::ZeroResourceCost => "portfolio action resource cost must be positive",
            Self::SelfConflict => "an action cannot be declared incompatible with itself",
            Self::UnknownConflictAction => "portfolio conflict references an unknown action",
            Self::DuplicateConflictPair => "portfolio incompatibility pair is duplicated",
            Self::ArithmeticOverflow => "exact portfolio objective or resource arithmetic overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PortfolioOracleError {}

/// Enumerate every feasible action subset and return the exact optimum.
///
/// The objective is maximum total caller-supplied expected consequence avoided.
/// Ties prefer lower resource consumption and then lexicographically smaller
/// sorted action IDs. Non-actionable actions are never selected. Any subset
/// containing both endpoints of an incompatibility pair is infeasible.
///
/// The oracle is intentionally limited to 20 actions (at most 1,048,576
/// subsets) and is intended as a reference for synthetic benchmark fixtures.
/// It does not optimize Pc, infer actionability, create conflict constraints,
/// model non-additive effects, or authorize / execute a maneuver.
pub fn exact_portfolio_oracle(
    resource_capacity: u64,
    actions: &[PortfolioAction],
    incompatible_pairs: &[IncompatibleActionPair],
) -> Result<PortfolioOracleReport, PortfolioOracleError> {
    if actions.len() > 20 {
        return Err(PortfolioOracleError::TooManyActions);
    }

    let mut action_ids = BTreeSet::new();
    let mut rejected_non_actionable_ids = Vec::new();
    for action in actions {
        if !action_ids.insert(action.action_id) {
            return Err(PortfolioOracleError::DuplicateActionIdentity);
        }
        if action.resource_units == 0 {
            return Err(PortfolioOracleError::ZeroResourceCost);
        }
        if !action.actionable {
            rejected_non_actionable_ids.push(action.action_id);
        }
    }

    rejected_non_actionable_ids.sort_unstable();

    let mut canonical_pairs = BTreeSet::new();
    for pair in incompatible_pairs {
        if pair.first_action_id == pair.second_action_id {
            return Err(PortfolioOracleError::SelfConflict);
        }
        if !action_ids.contains(&pair.first_action_id)
            || !action_ids.contains(&pair.second_action_id)
        {
            return Err(PortfolioOracleError::UnknownConflictAction);
        }
        let canonical = IncompatibleActionPair {
            first_action_id: pair.first_action_id.min(pair.second_action_id),
            second_action_id: pair.first_action_id.max(pair.second_action_id),
        };
        if !canonical_pairs.insert(canonical) {
            return Err(PortfolioOracleError::DuplicateConflictPair);
        }
    }

    let conflict_pairs: Vec<(u64, u64)> = canonical_pairs
        .iter()
        .map(|pair| (pair.first_action_id, pair.second_action_id))
        .collect();

    let mut best_benefit = Rational::from_integer(0);
    let mut best_resource_units = 0_u64;
    let mut best_action_ids = Vec::new();
    let subset_count = 1_u64 << actions.len();

    for mask in 0..subset_count {
        let mut selected_ids = Vec::new();
        let mut resource_units = 0_u64;
        let mut benefit = Rational::from_integer(0);
        let mut feasible = true;

        for (index, action) in actions.iter().enumerate() {
            if mask & (1_u64 << index) == 0 {
                continue;
            }
            if !action.actionable {
                feasible = false;
                break;
            }
            resource_units = resource_units
                .checked_add(action.resource_units)
                .ok_or(PortfolioOracleError::ArithmeticOverflow)?;
            if resource_units > resource_capacity {
                feasible = false;
                break;
            }
            benefit = benefit
                .checked_add(action.expected_consequence_avoided)
                .map_err(|_| PortfolioOracleError::ArithmeticOverflow)?;
            selected_ids.push(action.action_id);
        }

        if !feasible {
            continue;
        }
        selected_ids.sort_unstable();

        if conflict_pairs.iter().any(|(left, right)| {
            selected_ids.binary_search(left).is_ok() && selected_ids.binary_search(right).is_ok()
        }) {
            continue;
        }

        if benefit > best_benefit
            || (benefit == best_benefit
                && (resource_units < best_resource_units
                    || (resource_units == best_resource_units
                        && selected_ids < best_action_ids)))
        {
            best_benefit = benefit;
            best_resource_units = resource_units;
            best_action_ids = selected_ids;
        }
    }

    let unselected_action_ids = action_ids
        .iter()
        .filter(|action_id| !best_action_ids.contains(action_id))
        .copied()
        .collect();

    Ok(PortfolioOracleReport {
        resource_capacity,
        selected_resource_units: best_resource_units,
        total_expected_consequence_avoided: best_benefit,
        selected_action_ids: best_action_ids,
        rejected_non_actionable_ids,
        unselected_action_ids,
        incompatible_pairs: canonical_pairs.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(action_id: u64, benefit: u128, resource_units: u64) -> PortfolioAction {
        PortfolioAction {
            action_id,
            expected_consequence_avoided: Rational::from_integer(benefit),
            resource_units,
            actionable: true,
        }
    }

    #[test]
    fn consequence_objective_can_prefer_lower_pc_event_over_higher_pc_event() {
        // The labels encode the benchmark scenario, not a Pc calculation:
        // a high-Pc / low-consequence event competes with a lower-Pc event
        // whose independently modelled avoided consequence is much larger.
        let high_pc_low_consequence = action(1, 2, 1);
        let lower_pc_high_consequence = action(2, 20, 1);

        let report = exact_portfolio_oracle(
            1,
            &[high_pc_low_consequence, lower_pc_high_consequence],
            &[],
        )
        .unwrap();

        assert_eq!(report.selected_action_ids, [2]);
        assert_eq!(
            report.total_expected_consequence_avoided,
            Rational::from_integer(20)
        );
    }

    #[test]
    fn exact_oracle_beats_greedy_benefit_per_resource_on_a_finite_fixture() {
        let actions = [action(1, 6, 4), action(2, 10, 7), action(3, 10, 7)];
        let report = exact_portfolio_oracle(14, &actions, &[]).unwrap();

        assert_eq!(report.selected_action_ids, [2, 3]);
        assert_eq!(report.selected_resource_units, 14);
        assert_eq!(
            report.total_expected_consequence_avoided,
            Rational::from_integer(20)
        );
    }

    #[test]
    fn mutually_incompatible_maneuvers_are_never_selected_together() {
        let actions = [action(1, 15, 1), action(2, 14, 1), action(3, 10, 1)];
        let conflicts = [IncompatibleActionPair {
            first_action_id: 1,
            second_action_id: 2,
        }];
        let report = exact_portfolio_oracle(2, &actions, &conflicts).unwrap();

        assert_eq!(report.selected_action_ids, [1, 3]);
        assert_eq!(
            report.total_expected_consequence_avoided,
            Rational::from_integer(25)
        );
        assert_eq!(report.incompatible_pairs, conflicts);
    }

    #[test]
    fn conflict_pairs_are_canonicalized_independent_of_endpoint_order() {
        let actions = [action(1, 10, 1), action(2, 9, 1)];
        let forward = [IncompatibleActionPair {
            first_action_id: 1,
            second_action_id: 2,
        }];
        let reverse = [IncompatibleActionPair {
            first_action_id: 2,
            second_action_id: 1,
        }];

        assert_eq!(
            exact_portfolio_oracle(1, &actions, &forward).unwrap(),
            exact_portfolio_oracle(1, &actions, &reverse).unwrap()
        );
    }

    #[test]
    fn non_actionable_events_are_excluded_and_reported() {
        let mut non_actionable = action(3, 100, 1);
        non_actionable.actionable = false;
        let actions = [action(1, 5, 1), non_actionable];
        let report = exact_portfolio_oracle(2, &actions, &[]).unwrap();

        assert_eq!(report.selected_action_ids, [1]);
        assert_eq!(report.rejected_non_actionable_ids, [3]);
        assert_eq!(report.unselected_action_ids, [3]);
    }

    #[test]
    fn zero_capacity_returns_empty_selection_and_preserves_all_unselected_actions() {
        let actions = [action(1, 5, 1), action(2, 3, 2)];
        let report = exact_portfolio_oracle(0, &actions, &[]).unwrap();

        assert!(report.selected_action_ids.is_empty());
        assert_eq!(report.selected_resource_units, 0);
        assert_eq!(report.unselected_action_ids, [1, 2]);
    }

    #[test]
    fn exact_ties_prefer_lower_resource_then_stable_action_identity() {
        let actions = [action(2, 10, 2), action(1, 10, 1)];
        let report = exact_portfolio_oracle(2, &actions, &[]).unwrap();

        assert_eq!(report.selected_action_ids, [1]);
        assert_eq!(report.selected_resource_units, 1);
    }

    #[test]
    fn input_permutation_produces_an_identical_portfolio_report() {
        let actions = [action(3, 9, 3), action(1, 6, 2), action(2, 8, 2)];
        let forward = exact_portfolio_oracle(4, &actions, &[]).unwrap();
        let reverse = exact_portfolio_oracle(4, &[actions[2], actions[1], actions[0]], &[]).unwrap();

        assert_eq!(forward, reverse);
    }

    #[test]
    fn duplicate_action_identity_fails_closed() {
        let duplicate = action(1, 2, 1);
        assert_eq!(
            exact_portfolio_oracle(2, &[duplicate, duplicate], &[]),
            Err(PortfolioOracleError::DuplicateActionIdentity)
        );
    }

    #[test]
    fn unknown_conflict_endpoint_fails_closed() {
        let conflict = [IncompatibleActionPair {
            first_action_id: 1,
            second_action_id: 99,
        }];
        assert_eq!(
            exact_portfolio_oracle(2, &[action(1, 5, 1)], &conflict),
            Err(PortfolioOracleError::UnknownConflictAction)
        );
    }

    #[test]
    fn self_conflict_fails_closed() {
        let conflict = [IncompatibleActionPair {
            first_action_id: 1,
            second_action_id: 1,
        }];
        assert_eq!(
            exact_portfolio_oracle(2, &[action(1, 5, 1)], &conflict),
            Err(PortfolioOracleError::SelfConflict)
        );
    }

    #[test]
    fn duplicate_conflict_pair_fails_closed_even_when_reversed() {
        let conflicts = [
            IncompatibleActionPair {
                first_action_id: 1,
                second_action_id: 2,
            },
            IncompatibleActionPair {
                first_action_id: 2,
                second_action_id: 1,
            },
        ];
        assert_eq!(
            exact_portfolio_oracle(2, &[action(1, 5, 1), action(2, 4, 1)], &conflicts),
            Err(PortfolioOracleError::DuplicateConflictPair)
        );
    }

    #[test]
    fn zero_resource_cost_fails_closed() {
        let invalid = action(1, 5, 0);
        assert_eq!(
            exact_portfolio_oracle(2, &[invalid], &[]),
            Err(PortfolioOracleError::ZeroResourceCost)
        );
    }


    #[test]
    fn non_actionable_rejection_report_is_stable_under_input_permutation() {
        let mut rejected_first = action(9, 100, 1);
        rejected_first.actionable = false;
        let mut rejected_second = action(3, 100, 1);
        rejected_second.actionable = false;
        let actions = [rejected_first, action(1, 5, 1), rejected_second];

        let forward = exact_portfolio_oracle(1, &actions, &[]).unwrap();
        let reverse = exact_portfolio_oracle(1, &[actions[2], actions[1], actions[0]], &[]).unwrap();

        assert_eq!(forward, reverse);
        assert_eq!(forward.rejected_non_actionable_ids, [3, 9]);
    }

    #[test]
    fn rational_consequence_overflow_fails_closed() {
        let actions = [
            PortfolioAction {
                action_id: 1,
                expected_consequence_avoided: Rational::from_integer(u128::MAX),
                resource_units: 1,
                actionable: true,
            },
            action(2, 1, 1),
        ];

        assert_eq!(
            exact_portfolio_oracle(2, &actions, &[]),
            Err(PortfolioOracleError::ArithmeticOverflow)
        );
    }

    #[test]
    fn exact_oracle_rejects_more_than_twenty_actions() {
        let actions: Vec<_> = (0..21).map(|id| action(id, 1, 1)).collect();
        assert_eq!(
            exact_portfolio_oracle(21, &actions, &[]),
            Err(PortfolioOracleError::TooManyActions)
        );
    }
}
