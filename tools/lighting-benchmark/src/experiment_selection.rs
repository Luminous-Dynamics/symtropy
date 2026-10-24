//! Deterministic selection of an already-evaluated experiment proposal.
//!
//! This module does not estimate expected value of information. Symthaea or a
//! separate calibrated model must supply estimates, units and provenance. The
//! selector only validates those estimates, applies declared safety/budget
//! gates, and picks the eligible proposal with highest positive net value.

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentCandidate {
    pub id: String,
    /// Expected decision-loss reduction, in the declared common value unit.
    pub expected_decision_loss_reduction: f64,
    /// Cost converted into the same decision-value unit.
    pub cost_penalty_loss_equivalent: f64,
    /// Resource budget consumption, for example high-fidelity solver calls.
    pub budget_cost: f64,
    /// Must be set by an external policy/lab authorization check.
    pub safety_cleared: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentSelection {
    pub candidate_id: String,
    pub net_expected_value: f64,
    pub budget_cost: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    InvalidBudget,
    EmptyCandidateId,
    DuplicateCandidateId,
    NonFiniteEstimate,
    NegativeEstimate,
    NonPositiveBudgetCost,
    NonFiniteNetValue,
}

/// Select the experiment with greatest strictly positive net expected value.
///
/// All candidates are validated before any choice is returned. A malformed
/// option aborts selection instead of being silently skipped. Unsafe or
/// over-budget candidates remain ineligible. Ties resolve deterministically by
/// smaller budget cost, then lexicographically smaller candidate ID.
///
/// The expected-value model, units, calibration and evidence are not validated
/// here and must be included in the calling run's provenance receipt.
pub fn select_next_experiment(
    candidates: &[ExperimentCandidate],
    remaining_budget: f64,
) -> Result<Option<ExperimentSelection>, SelectionError> {
    if !remaining_budget.is_finite() || remaining_budget < 0.0 {
        return Err(SelectionError::InvalidBudget);
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut best: Option<ExperimentSelection> = None;

    for candidate in candidates {
        if candidate.id.trim().is_empty() {
            return Err(SelectionError::EmptyCandidateId);
        }
        if !seen.insert(candidate.id.as_str()) {
            return Err(SelectionError::DuplicateCandidateId);
        }

        let estimates = [
            candidate.expected_decision_loss_reduction,
            candidate.cost_penalty_loss_equivalent,
            candidate.budget_cost,
        ];
        if estimates.iter().any(|value| !value.is_finite()) {
            return Err(SelectionError::NonFiniteEstimate);
        }
        if candidate.expected_decision_loss_reduction < 0.0
            || candidate.cost_penalty_loss_equivalent < 0.0
        {
            return Err(SelectionError::NegativeEstimate);
        }
        if candidate.budget_cost <= 0.0 {
            return Err(SelectionError::NonPositiveBudgetCost);
        }
        let net_value =
            candidate.expected_decision_loss_reduction - candidate.cost_penalty_loss_equivalent;
        if !net_value.is_finite() {
            return Err(SelectionError::NonFiniteNetValue);
        }

        if !candidate.safety_cleared
            || candidate.budget_cost > remaining_budget
            || net_value <= 0.0
        {
            continue;
        }

        let proposed = ExperimentSelection {
            candidate_id: candidate.id.clone(),
            net_expected_value: net_value,
            budget_cost: candidate.budget_cost,
        };

        let replace_best = match &best {
            None => true,
            Some(current) => {
                proposed.net_expected_value > current.net_expected_value
                    || (proposed.net_expected_value == current.net_expected_value
                        && (proposed.budget_cost < current.budget_cost
                            || (proposed.budget_cost == current.budget_cost
                                && proposed.candidate_id < current.candidate_id)))
            }
        };
        if replace_best {
            best = Some(proposed);
        }
    }

    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        id: &str,
        benefit: f64,
        penalty: f64,
        budget_cost: f64,
        safety_cleared: bool,
    ) -> ExperimentCandidate {
        ExperimentCandidate {
            id: id.to_owned(),
            expected_decision_loss_reduction: benefit,
            cost_penalty_loss_equivalent: penalty,
            budget_cost,
            safety_cleared,
        }
    }

    #[test]
    fn selects_largest_positive_net_value_within_budget() {
        let options = vec![
            candidate("a", 10.0, 2.0, 1.0, true),
            candidate("b", 15.0, 3.0, 2.0, true),
            candidate("c", 100.0, 1.0, 9.0, true),
        ];
        let selected = select_next_experiment(&options, 3.0).unwrap().unwrap();
        assert_eq!(selected.candidate_id, "b");
        assert_eq!(selected.net_expected_value, 12.0);
        assert_eq!(selected.budget_cost, 2.0);
    }

    #[test]
    fn unsafe_or_non_positive_value_options_are_not_selected() {
        let options = vec![
            candidate("unsafe", 100.0, 0.0, 1.0, false),
            candidate("no-benefit", 2.0, 2.0, 1.0, true),
            candidate("negative-benefit", 1.0, 2.0, 1.0, true),
        ];
        assert_eq!(select_next_experiment(&options, 10.0).unwrap(), None);
    }

    #[test]
    fn tie_breaking_is_deterministic() {
        let options = vec![
            candidate("z-costly", 10.0, 2.0, 3.0, true),
            candidate("b-cheap", 10.0, 2.0, 1.0, true),
            candidate("a-cheap", 10.0, 2.0, 1.0, true),
        ];
        let selected = select_next_experiment(&options, 10.0).unwrap().unwrap();
        assert_eq!(selected.candidate_id, "a-cheap");
    }

    #[test]
    fn invalid_budget_and_invalid_estimates_fail_closed() {
        assert_eq!(
            select_next_experiment(&[], f64::NAN),
            Err(SelectionError::InvalidBudget)
        );
        assert_eq!(
            select_next_experiment(&[candidate("bad", f64::NAN, 0.0, 1.0, true)], 3.0),
            Err(SelectionError::NonFiniteEstimate)
        );
        assert_eq!(
            select_next_experiment(&[candidate("bad", -1.0, 0.0, 1.0, true)], 3.0),
            Err(SelectionError::NegativeEstimate)
        );
    }

    #[test]
    fn duplicate_ids_abort_selection_instead_of_colliding() {
        let options = vec![
            candidate("same", 10.0, 2.0, 1.0, true),
            candidate("same", 11.0, 2.0, 1.0, true),
        ];
        assert_eq!(
            select_next_experiment(&options, 10.0),
            Err(SelectionError::DuplicateCandidateId)
        );
    }

    #[test]
    fn malformed_candidate_aborts_even_if_other_candidate_is_better() {
        let options = vec![
            candidate("good", 10.0, 2.0, 1.0, true),
            candidate("missing-cost", 8.0, 2.0, 0.0, true),
        ];
        assert_eq!(
            select_next_experiment(&options, 10.0),
            Err(SelectionError::NonPositiveBudgetCost)
        );
    }
}
