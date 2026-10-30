//! Exact Bayes-optimal adaptive decision trees for finite deterministic experiments.
//!
//! The oracle is exact for the supplied finite hypothesis support, normalized prior,
//! deterministic binary observation semantics, and fixed positive action costs.
//! It is not a lower bound for noisy or continuous physical experiments.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub const ACTIONS: usize = 8;

/// One semantic hypothesis. Signature bit positions 0..7 hold deterministic outcomes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hypothesis {
    pub signature: u8,
    pub prior_weight: f64,
}

/// A node in the optimal adaptive decision tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionNode {
    pub expected_remaining_cost: f64,
    pub identified_hypothesis: Option<u8>,
    pub action: Option<u8>,
    pub outcome_zero: Option<Arc<DecisionNode>>,
    pub outcome_one: Option<Arc<DecisionNode>>,
}

impl DecisionNode {
    pub fn is_leaf(&self) -> bool {
        self.identified_hypothesis.is_some()
    }
}

/// Result of the exact dynamic program.
#[derive(Clone, Debug)]
pub struct OracleResult {
    pub expected_cost: f64,
    pub first_action: Option<u8>,
    pub memoized_states: usize,
    pub tree: Arc<DecisionNode>,
}

/// Solve the Bayes-optimal adaptive identification problem.
///
/// The Bellman value is the minimum expected remaining cost over all interventions
/// that split the current survivor set. The state is memoized by sorted signatures.
/// The solver receives no evaluator target.
///
/// max_states is a fail-closed complexity bound.
pub fn solve_exact(
    hypotheses: &[Hypothesis],
    action_costs: &[u32; ACTIONS],
    max_states: usize,
) -> Result<OracleResult, String> {
    if hypotheses.is_empty() {
        return Err("the hypothesis set must not be empty".into());
    }
    if action_costs.iter().any(|cost| *cost == 0) {
        return Err("all action costs must be positive integers".into());
    }
    let mut weights = BTreeMap::new();
    for hypothesis in hypotheses {
        if !hypothesis.prior_weight.is_finite() || hypothesis.prior_weight <= 0.0 {
            return Err(format!(
                "prior weight for signature {} must be finite and positive",
                hypothesis.signature
            ));
        }
        if weights
            .insert(hypothesis.signature, hypothesis.prior_weight)
            .is_some()
        {
            return Err(format!(
                "duplicate semantic signature {}",
                hypothesis.signature
            ));
        }
    }

    let signatures: Vec<u8> = weights.keys().copied().collect();
    let mut memo: HashMap<Vec<u8>, Arc<DecisionNode>> = HashMap::new();
    let tree = solve_state(&signatures, &weights, action_costs, max_states, &mut memo)?;
    Ok(OracleResult {
        expected_cost: tree.expected_remaining_cost,
        first_action: tree.action,
        memoized_states: memo.len(),
        tree,
    })
}

fn solve_state(
    signatures: &[u8],
    weights: &BTreeMap<u8, f64>,
    costs: &[u32; ACTIONS],
    max_states: usize,
    memo: &mut HashMap<Vec<u8>, Arc<DecisionNode>>,
) -> Result<Arc<DecisionNode>, String> {
    if signatures.len() == 1 {
        return Ok(Arc::new(DecisionNode {
            expected_remaining_cost: 0.0,
            identified_hypothesis: Some(signatures[0]),
            action: None,
            outcome_zero: None,
            outcome_one: None,
        }));
    }
    if signatures.is_empty() {
        return Err("an empty survivor state is not a valid decision-tree node".into());
    }
    if let Some(node) = memo.get(signatures) {
        return Ok(Arc::clone(node));
    }
    if memo.len() >= max_states {
        return Err(format!(
            "oracle state budget exceeded: limit {max_states}, already solved {} states",
            memo.len()
        ));
    }

    let total_weight: f64 = signatures.iter().map(|signature| weights[signature]).sum();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return Err("survivor prior mass is non-finite or non-positive".into());
    }

    let mut best: Option<(f64, u8, Arc<DecisionNode>, Arc<DecisionNode>)> = None;
    for action in 0..ACTIONS as u8 {
        let mut zero = Vec::new();
        let mut one = Vec::new();
        let mut zero_weight = 0.0;
        let mut one_weight = 0.0;
        for signature in signatures {
            if (*signature >> action) & 1 == 1 {
                one.push(*signature);
                one_weight += weights[signature];
            } else {
                zero.push(*signature);
                zero_weight += weights[signature];
            }
        }
        if zero.is_empty() || one.is_empty() {
            continue;
        }
        let zero_node = solve_state(&zero, weights, costs, max_states, memo)?;
        let one_node = solve_state(&one, weights, costs, max_states, memo)?;
        let expected = f64::from(costs[action as usize])
            + (zero_weight / total_weight) * zero_node.expected_remaining_cost
            + (one_weight / total_weight) * one_node.expected_remaining_cost;
        if !expected.is_finite() {
            return Err("oracle objective became non-finite".into());
        }
        let replace = best.as_ref().is_none_or(|(best_cost, best_action, _, _)| {
            expected.total_cmp(best_cost).is_lt()
                || (expected.total_cmp(best_cost).is_eq() && action < *best_action)
        });
        if replace {
            best = Some((expected, action, zero_node, one_node));
        }
    }

    let Some((expected_remaining_cost, action, outcome_zero, outcome_one)) = best else {
        return Err(format!(
            "hypotheses cannot be separated by the {ACTIONS} available binary actions"
        ));
    };
    let node = Arc::new(DecisionNode {
        expected_remaining_cost,
        identified_hypothesis: None,
        action: Some(action),
        outcome_zero: Some(outcome_zero),
        outcome_one: Some(outcome_one),
    });
    memo.insert(signatures.to_vec(), Arc::clone(&node));
    Ok(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hypotheses(signatures: &[u8], weights: &[f64]) -> Vec<Hypothesis> {
        signatures
            .iter()
            .copied()
            .zip(weights.iter().copied())
            .map(|(signature, prior_weight)| Hypothesis {
                signature,
                prior_weight,
            })
            .collect()
    }

    /// Independent exhaustive recursion without memoization, used on tiny domains.
    fn brute_force(signatures: &[u8], weights: &BTreeMap<u8, f64>, costs: &[u32; ACTIONS]) -> f64 {
        if signatures.len() <= 1 {
            return 0.0;
        }
        let total: f64 = signatures.iter().map(|sig| weights[sig]).sum();
        let mut best = f64::INFINITY;
        for action in 0..ACTIONS as u8 {
            let zero: Vec<u8> = signatures
                .iter()
                .copied()
                .filter(|sig| (*sig >> action) & 1 == 0)
                .collect();
            let one: Vec<u8> = signatures
                .iter()
                .copied()
                .filter(|sig| (*sig >> action) & 1 == 1)
                .collect();
            if zero.is_empty() || one.is_empty() {
                continue;
            }
            let p_zero: f64 = zero.iter().map(|sig| weights[sig]).sum::<f64>() / total;
            let p_one: f64 = one.iter().map(|sig| weights[sig]).sum::<f64>() / total;
            let value = f64::from(costs[action as usize])
                + p_zero * brute_force(&zero, weights, costs)
                + p_one * brute_force(&one, weights, costs);
            best = best.min(value);
        }
        best
    }

    #[test]
    fn dp_matches_brute_force_for_every_nontrivial_subset_of_three_bit_functions() {
        let costs = [3, 1, 2, 4, 5, 2, 3, 1];
        for subset in 1u16..=u8::MAX as u16 {
            let signatures: Vec<u8> = (0..8u8)
                .filter(|signature| (subset >> *signature) & 1 == 1)
                .collect();
            if signatures.len() <= 1 {
                continue;
            }
            let raw_weights: Vec<f64> = signatures
                .iter()
                .map(|signature| 1.0 + f64::from((signature * 7) % 5))
                .collect();
            let input = hypotheses(&signatures, &raw_weights);
            let result = solve_exact(&input, &costs, 10_000).unwrap();
            let weights: BTreeMap<u8, f64> = input
                .iter()
                .map(|h| (h.signature, h.prior_weight))
                .collect();
            let expected = brute_force(&signatures, &weights, &costs);
            assert!(
                (result.expected_cost - expected).abs() < 1e-10,
                "subset {subset:#x}: DP {}, brute force {expected}",
                result.expected_cost
            );
        }
    }

    #[test]
    fn equal_cost_full_three_bit_domain_has_exactly_three_units_expected_cost() {
        let input = hypotheses(&(0..8).collect::<Vec<_>>(), &[1.0; 8]);
        let result = solve_exact(&input, &[1; ACTIONS], 10_000).unwrap();
        assert!((result.expected_cost - 3.0).abs() < 1e-12);
        assert!(result.first_action.is_some());
    }

    #[test]
    fn full_256_function_universe_visits_all_partial_observation_states() {
        let input = (0u16..=255)
            .map(|signature| Hypothesis {
                signature: signature as u8,
                prior_weight: 1.0,
            })
            .collect::<Vec<_>>();
        let costs = [8, 7, 6, 5, 4, 3, 2, 1];
        let result = solve_exact(&input, &costs, 10_000).unwrap();
        assert_eq!(result.memoized_states, 3usize.pow(8) - 2usize.pow(8));
        assert_eq!(
            result.expected_cost,
            costs.iter().map(|cost| f64::from(*cost)).sum::<f64>()
        );
        assert_eq!(result.first_action, Some(0));
    }

    #[test]
    fn singleton_is_a_zero_cost_leaf() {
        let input = hypotheses(&[173], &[0.25]);
        let result = solve_exact(&input, &[1; ACTIONS], 10_000).unwrap();
        assert_eq!(result.expected_cost, 0.0);
        assert_eq!(result.first_action, None);
        assert_eq!(result.tree.identified_hypothesis, Some(173));
        assert_eq!(result.memoized_states, 0);
    }

    #[test]
    fn prior_scaling_does_not_change_expected_cost_or_tree() {
        let costs = [2, 4, 1, 5, 3, 2, 1, 4];
        let a = solve_exact(
            &hypotheses(&[0, 1, 2, 3], &[1.0, 2.0, 3.0, 4.0]),
            &costs,
            10_000,
        )
        .unwrap();
        let b = solve_exact(
            &hypotheses(&[0, 1, 2, 3], &[10.0, 20.0, 30.0, 40.0]),
            &costs,
            10_000,
        )
        .unwrap();
        assert!((a.expected_cost - b.expected_cost).abs() < 1e-12);
        assert_eq!(a.first_action, b.first_action);
    }

    #[test]
    fn results_are_independent_of_input_hypothesis_order() {
        let costs = [3, 1, 2, 4, 5, 2, 3, 1];
        let a = solve_exact(
            &hypotheses(&[0, 1, 2, 3], &[1.0, 2.0, 3.0, 4.0]),
            &costs,
            10_000,
        )
        .unwrap();
        let b = solve_exact(
            &hypotheses(&[3, 2, 1, 0], &[4.0, 3.0, 2.0, 1.0]),
            &costs,
            10_000,
        )
        .unwrap();
        assert_eq!(a.first_action, b.first_action);
        assert!((a.expected_cost - b.expected_cost).abs() < 1e-12);
    }

    #[test]
    fn exact_first_action_is_no_worse_than_every_admissible_first_action() {
        let input = hypotheses(&[0, 1, 2, 3, 4, 5], &[1.0, 2.0, 1.5, 4.0, 3.0, 2.5]);
        let costs = [4, 2, 1, 5, 3, 2, 4, 1];
        let result = solve_exact(&input, &costs, 10_000).unwrap();
        let weights: BTreeMap<u8, f64> = input
            .iter()
            .map(|h| (h.signature, h.prior_weight))
            .collect();
        let signatures: Vec<u8> = weights.keys().copied().collect();
        let total: f64 = signatures.iter().map(|sig| weights[sig]).sum();
        for action in 0..ACTIONS as u8 {
            let zero: Vec<u8> = signatures
                .iter()
                .copied()
                .filter(|sig| (*sig >> action) & 1 == 0)
                .collect();
            let one: Vec<u8> = signatures
                .iter()
                .copied()
                .filter(|sig| (*sig >> action) & 1 == 1)
                .collect();
            if zero.is_empty() || one.is_empty() {
                continue;
            }
            let p_zero: f64 = zero.iter().map(|sig| weights[sig]).sum::<f64>() / total;
            let p_one: f64 = one.iter().map(|sig| weights[sig]).sum::<f64>() / total;
            let mut memo_zero = HashMap::new();
            let v_zero = solve_state(&zero, &weights, &costs, 10_000, &mut memo_zero)
                .unwrap()
                .expected_remaining_cost;
            let mut memo_one = HashMap::new();
            let v_one = solve_state(&one, &weights, &costs, 10_000, &mut memo_one)
                .unwrap()
                .expected_remaining_cost;
            let value = f64::from(costs[action as usize]) + p_zero * v_zero + p_one * v_one;
            assert!(result.expected_cost <= value + 1e-12);
        }
    }

    #[test]
    fn rejects_duplicate_signatures_nonpositive_weights_and_zero_costs() {
        assert!(solve_exact(&hypotheses(&[1, 1], &[1.0, 1.0]), &[1; ACTIONS], 10).is_err());
        assert!(solve_exact(&hypotheses(&[1], &[0.0]), &[1; ACTIONS], 10).is_err());
        assert!(solve_exact(&hypotheses(&[1], &[f64::NAN]), &[1; ACTIONS], 10).is_err());
        assert!(solve_exact(&hypotheses(&[1], &[1.0]), &[0; ACTIONS], 10).is_err());
    }

    #[test]
    fn state_budget_exhaustion_fails_closed() {
        let input = hypotheses(&(0..8).collect::<Vec<_>>(), &[1.0; 8]);
        assert!(solve_exact(&input, &[1; ACTIONS], 1).is_err());
    }

    #[test]
    fn no_hidden_target_is_accepted_by_the_solver_api() {
        let input = hypotheses(&[0, 1, 2, 3], &[1.0; 4]);
        let result = solve_exact(&input, &[1; ACTIONS], 10_000).unwrap();
        assert_eq!(result.expected_cost, 2.0);
    }
}
