//! Exact finite-horizon decision-aware stopping and abstention.
//!
//! This module is a deterministic finite-model oracle. It chooses among
//! stopping with a class decision, abstaining, or performing another
//! positive-cost binary experiment.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Number of available deterministic binary experiments.
pub const ACTIONS: usize = 8;

/// One finite hypothesis and its downstream decision class.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hypothesis {
    /// Eight-bit deterministic response signature; bit a is outcome of action a.
    pub signature: u8,
    /// Downstream class selected if the policy stops under this posterior mode.
    pub decision_class: u8,
    /// Positive prior mass. Weights are normalized internally.
    pub prior_weight: f64,
}

/// Costs and finite horizon for the decision problem.
///
/// Experiment costs and the zero-one task loss must be expressed in the same
/// scalar utility units. The solver does not choose that conversion.
#[derive(Clone, Copy, Debug)]
pub struct DecisionOptions {
    /// Positive finite cost for each of the eight binary experiments.
    pub action_costs: [f64; ACTIONS],
    /// Loss for abstaining / handing off for human review.
    pub abstain_loss: f64,
    /// Maximum number of additional experiments; at most eight.
    pub max_horizon: usize,
    /// Maximum number of memoized nonterminal belief states.
    pub max_states: usize,
}

/// Terminal or experiment node in the optimal decision policy.
#[derive(Clone, Debug, PartialEq)]
pub enum DecisionKind {
    /// Stop and choose the posterior-most-probable downstream class.
    Stop { decision_class: u8 },
    /// Abstain and incur the configured fallback loss.
    Abstain,
    /// Run an experiment and continue down the observed outcome branch.
    Experiment {
        action: u8,
        outcome_zero: Arc<DecisionNode>,
        outcome_one: Arc<DecisionNode>,
    },
}

/// Expected value decomposition at a policy-tree node.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionNode {
    /// Bellman objective: experiment cost + task error + abstention loss.
    pub expected_total_loss: f64,
    /// Expected sum of experiment costs from this node.
    pub expected_experiment_cost: f64,
    /// Expected zero-one downstream decision error.
    pub expected_decision_error: f64,
    /// Expected abstention / handoff penalty.
    pub expected_abstention_loss: f64,
    /// Probability that the policy eventually abstains.
    pub abstain_probability: f64,
    /// Number of experiments still available at this node.
    pub remaining_horizon: usize,
    /// Chosen stop, abstain, or experiment action.
    pub kind: DecisionKind,
}

/// Root value and decomposition of the exact finite-horizon solution.
#[derive(Clone, Debug)]
pub struct DecisionResult {
    pub expected_total_loss: f64,
    pub expected_experiment_cost: f64,
    pub expected_decision_error: f64,
    pub expected_abstention_loss: f64,
    pub abstain_probability: f64,
    /// Set only when the root policy stops immediately.
    pub selected_decision: Option<u8>,
    /// Set only when the root policy performs an experiment immediately.
    pub first_action: Option<u8>,
    pub memoized_states: usize,
    pub tree: Arc<DecisionNode>,
}

/// Solve a deterministic finite-horizon stop / abstain / experiment problem.
///
/// The API intentionally receives no evaluator target. Hypothesis signatures
/// are model support, not hidden ground truth. Posterior updates are exact
/// survivor filtering because this first slice assumes deterministic outputs.
pub fn solve_decision(
    hypotheses: &[Hypothesis],
    options: &DecisionOptions,
) -> Result<DecisionResult, String> {
    if hypotheses.is_empty() {
        return Err("the hypothesis support must not be empty".into());
    }
    if options.max_horizon > ACTIONS {
        return Err(format!("max_horizon must not exceed {ACTIONS}"));
    }
    if options.max_states == 0 {
        return Err("max_states must be positive".into());
    }
    if options
        .action_costs
        .iter()
        .any(|cost| !cost.is_finite() || *cost <= 0.0)
    {
        return Err("all experiment costs must be finite and positive".into());
    }
    if !options.abstain_loss.is_finite() || options.abstain_loss < 0.0 {
        return Err("abstain_loss must be finite and nonnegative".into());
    }

    let mut support = BTreeMap::new();
    for hypothesis in hypotheses {
        if !hypothesis.prior_weight.is_finite() || hypothesis.prior_weight <= 0.0 {
            return Err(format!(
                "prior weight for signature {} must be finite and positive",
                hypothesis.signature
            ));
        }
        if support
            .insert(
                hypothesis.signature,
                (hypothesis.decision_class, hypothesis.prior_weight),
            )
            .is_some()
        {
            return Err(format!(
                "duplicate semantic signature {}",
                hypothesis.signature
            ));
        }
    }

    let total_weight: f64 = support.values().map(|(_, weight)| *weight).sum();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return Err("total prior mass must be finite and positive".into());
    }

    let signatures: Vec<u8> = support.keys().copied().collect();
    let mut memo: HashMap<(Vec<u8>, usize), Arc<DecisionNode>> = HashMap::new();
    let tree = solve_state(
        &signatures,
        options.max_horizon,
        &support,
        options,
        &mut memo,
    )?;

    let selected_decision = match &tree.kind {
        DecisionKind::Stop { decision_class } => Some(*decision_class),
        DecisionKind::Abstain | DecisionKind::Experiment { .. } => None,
    };
    let first_action = match &tree.kind {
        DecisionKind::Experiment { action, .. } => Some(*action),
        DecisionKind::Stop { .. } | DecisionKind::Abstain => None,
    };

    Ok(DecisionResult {
        expected_total_loss: tree.expected_total_loss,
        expected_experiment_cost: tree.expected_experiment_cost,
        expected_decision_error: tree.expected_decision_error,
        expected_abstention_loss: tree.expected_abstention_loss,
        abstain_probability: tree.abstain_probability,
        selected_decision,
        first_action,
        memoized_states: memo.len(),
        tree,
    })
}

fn solve_state(
    signatures: &[u8],
    horizon: usize,
    support: &BTreeMap<u8, (u8, f64)>,
    options: &DecisionOptions,
    memo: &mut HashMap<(Vec<u8>, usize), Arc<DecisionNode>>,
) -> Result<Arc<DecisionNode>, String> {
    if signatures.is_empty() {
        return Err("an empty posterior support is invalid".into());
    }
    if signatures.len() == 1 {
        let (decision_class, _) = support
            .get(&signatures[0])
            .ok_or_else(|| format!("posterior references unknown signature {}", signatures[0]))?;
        return Ok(Arc::new(DecisionNode {
            expected_total_loss: 0.0,
            expected_experiment_cost: 0.0,
            expected_decision_error: 0.0,
            expected_abstention_loss: 0.0,
            abstain_probability: 0.0,
            remaining_horizon: horizon,
            kind: DecisionKind::Stop {
                decision_class: *decision_class,
            },
        }));
    }

    let key = (signatures.to_vec(), horizon);
    if let Some(node) = memo.get(&key) {
        return Ok(Arc::clone(node));
    }
    if memo.len() >= options.max_states {
        return Err(format!(
            "decision oracle state budget exceeded: limit {}, already solved {} states",
            options.max_states,
            memo.len()
        ));
    }

    let mut total_weight = 0.0;
    let mut class_weights = BTreeMap::<u8, f64>::new();
    for signature in signatures {
        let (decision_class, weight) = support
            .get(signature)
            .ok_or_else(|| format!("posterior references unknown signature {signature}"))?;
        total_weight += *weight;
        *class_weights.entry(*decision_class).or_insert(0.0) += *weight;
    }
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return Err("posterior mass must remain finite and positive".into());
    }

    let mut selected_class = None;
    let mut selected_class_weight = -1.0_f64;
    for (decision_class, weight) in &class_weights {
        if *weight > selected_class_weight {
            selected_class = Some(*decision_class);
            selected_class_weight = *weight;
        }
    }
    let decision_class = selected_class.ok_or("posterior has no decision classes")?;
    let stop_risk = ((total_weight - selected_class_weight) / total_weight).clamp(0.0, 1.0);

    let mut best = Arc::new(DecisionNode {
        expected_total_loss: stop_risk,
        expected_experiment_cost: 0.0,
        expected_decision_error: stop_risk,
        expected_abstention_loss: 0.0,
        abstain_probability: 0.0,
        remaining_horizon: horizon,
        kind: DecisionKind::Stop { decision_class },
    });

    let abstain = Arc::new(DecisionNode {
        expected_total_loss: options.abstain_loss,
        expected_experiment_cost: 0.0,
        expected_decision_error: 0.0,
        expected_abstention_loss: options.abstain_loss,
        abstain_probability: 1.0,
        remaining_horizon: horizon,
        kind: DecisionKind::Abstain,
    });
    if abstain
        .expected_total_loss
        .total_cmp(&best.expected_total_loss)
        .is_lt()
    {
        best = abstain;
    }

    if horizon > 0 {
        for action in 0..ACTIONS as u8 {
            let mut zero = Vec::new();
            let mut one = Vec::new();
            let mut zero_weight = 0.0;
            let mut one_weight = 0.0;
            for signature in signatures {
                let (_, weight) = support
                    .get(signature)
                    .ok_or_else(|| format!("posterior references unknown signature {signature}"))?;
                if (*signature >> action) & 1 == 1 {
                    one.push(*signature);
                    one_weight += *weight;
                } else {
                    zero.push(*signature);
                    zero_weight += *weight;
                }
            }
            if zero.is_empty() || one.is_empty() {
                continue;
            }

            let zero_node = solve_state(&zero, horizon - 1, support, options, memo)?;
            let one_node = solve_state(&one, horizon - 1, support, options, memo)?;
            let p_zero = zero_weight / total_weight;
            let p_one = one_weight / total_weight;
            let action_cost = options.action_costs[action as usize];
            let expected_experiment_cost = action_cost
                + p_zero * zero_node.expected_experiment_cost
                + p_one * one_node.expected_experiment_cost;
            let expected_decision_error = p_zero * zero_node.expected_decision_error
                + p_one * one_node.expected_decision_error;
            let expected_abstention_loss = p_zero * zero_node.expected_abstention_loss
                + p_one * one_node.expected_abstention_loss;
            let abstain_probability =
                p_zero * zero_node.abstain_probability + p_one * one_node.abstain_probability;
            let expected_total_loss =
                expected_experiment_cost + expected_decision_error + expected_abstention_loss;
            if !expected_total_loss.is_finite()
                || !expected_experiment_cost.is_finite()
                || !expected_decision_error.is_finite()
                || !expected_abstention_loss.is_finite()
                || !abstain_probability.is_finite()
            {
                return Err("decision objective became non-finite".into());
            }

            let candidate = Arc::new(DecisionNode {
                expected_total_loss,
                expected_experiment_cost,
                expected_decision_error,
                expected_abstention_loss,
                abstain_probability,
                remaining_horizon: horizon,
                kind: DecisionKind::Experiment {
                    action,
                    outcome_zero: zero_node,
                    outcome_one: one_node,
                },
            });
            if candidate
                .expected_total_loss
                .total_cmp(&best.expected_total_loss)
                .is_lt()
            {
                best = candidate;
            }
        }
    }

    memo.insert(key, Arc::clone(&best));
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(action_cost: f64, abstain_loss: f64, horizon: usize) -> DecisionOptions {
        DecisionOptions {
            action_costs: [action_cost; ACTIONS],
            abstain_loss,
            max_horizon: horizon,
            max_states: 10_000,
        }
    }

    fn binary_fixture(weight_zero: f64, weight_one: f64) -> Vec<Hypothesis> {
        vec![
            Hypothesis {
                signature: 0,
                decision_class: 0,
                prior_weight: weight_zero,
            },
            Hypothesis {
                signature: 1,
                decision_class: 1,
                prior_weight: weight_one,
            },
        ]
    }

    #[test]
    fn queries_when_expected_decision_risk_reduction_exceeds_cost() {
        let result = solve_decision(&binary_fixture(1.0, 1.0), &options(0.1, 0.2, 1)).unwrap();
        assert_eq!(result.first_action, Some(0));
        assert!((result.expected_total_loss - 0.1).abs() < 1e-12);
        assert!((result.expected_experiment_cost - 0.1).abs() < 1e-12);
        assert!(result.expected_decision_error.abs() < 1e-12);
        assert!((result.abstain_probability).abs() < 1e-12);
    }

    #[test]
    fn abstains_when_a_query_costs_more_than_fallback() {
        let result = solve_decision(&binary_fixture(1.0, 1.0), &options(0.8, 0.2, 1)).unwrap();
        assert!(matches!(result.tree.kind, DecisionKind::Abstain));
        assert_eq!(result.first_action, None);
        assert!((result.expected_total_loss - 0.2).abs() < 1e-12);
        assert!((result.abstain_probability - 1.0).abs() < 1e-12);
    }

    #[test]
    fn stops_when_posterior_decision_risk_is_already_low() {
        let result = solve_decision(&binary_fixture(19.0, 1.0), &options(0.1, 0.2, 3)).unwrap();
        assert_eq!(result.selected_decision, Some(0));
        assert_eq!(result.first_action, None);
        assert!((result.expected_decision_error - 0.05).abs() < 1e-12);
        assert!((result.expected_total_loss - 0.05).abs() < 1e-12);
    }

    #[test]
    fn zero_horizon_forces_stop_or_abstain_not_an_experiment() {
        let result = solve_decision(&binary_fixture(1.0, 1.0), &options(0.1, 0.2, 0)).unwrap();
        assert!(matches!(result.tree.kind, DecisionKind::Abstain));
        assert_eq!(result.first_action, None);
    }

    #[test]
    fn hypotheses_in_the_same_decision_class_need_not_be_identified() {
        let hypotheses = [
            Hypothesis {
                signature: 0,
                decision_class: 7,
                prior_weight: 2.0,
            },
            Hypothesis {
                signature: 1,
                decision_class: 7,
                prior_weight: 3.0,
            },
        ];
        let result = solve_decision(&hypotheses, &options(0.01, 0.5, 4)).unwrap();
        assert_eq!(result.selected_decision, Some(7));
        assert_eq!(result.first_action, None);
        assert!(result.expected_total_loss.abs() < 1e-12);
    }

    #[test]
    fn class_ties_choose_the_lowest_class_id_and_keep_stop_on_value_ties() {
        let result = solve_decision(&binary_fixture(1.0, 1.0), &options(10.0, 0.5, 8)).unwrap();
        assert_eq!(result.selected_decision, Some(0));
        assert_eq!(result.first_action, None);
        assert!((result.expected_total_loss - 0.5).abs() < 1e-12);
    }

    #[test]
    fn common_prior_scaling_does_not_change_the_policy_or_value() {
        let first = binary_fixture(1.0, 3.0);
        let second = binary_fixture(10.0, 30.0);
        let opts = options(0.1, 0.2, 1);
        let a = solve_decision(&first, &opts).unwrap();
        let b = solve_decision(&second, &opts).unwrap();
        assert_eq!(a.first_action, b.first_action);
        assert_eq!(a.selected_decision, b.selected_decision);
        assert!((a.expected_total_loss - b.expected_total_loss).abs() < 1e-12);
    }

    #[test]
    fn input_order_does_not_change_the_optimal_decision() {
        let forward = binary_fixture(2.0, 3.0);
        let reverse = binary_fixture(2.0, 3.0)
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let opts = options(0.1, 0.2, 1);
        let a = solve_decision(&forward, &opts).unwrap();
        let b = solve_decision(&reverse, &opts).unwrap();
        assert_eq!(a.first_action, b.first_action);
        assert_eq!(a.selected_decision, b.selected_decision);
        assert!((a.expected_total_loss - b.expected_total_loss).abs() < 1e-12);
    }

    fn brute_value(
        signatures: &[u8],
        support: &BTreeMap<u8, (u8, f64)>,
        options: &DecisionOptions,
        horizon: usize,
    ) -> f64 {
        let total_weight: f64 = signatures
            .iter()
            .map(|signature| support[signature].1)
            .sum();
        let mut class_weights = BTreeMap::<u8, f64>::new();
        for signature in signatures {
            let (decision_class, weight) = support[signature];
            *class_weights.entry(decision_class).or_insert(0.0) += weight;
        }
        let largest_class = class_weights.values().copied().fold(0.0_f64, f64::max);
        let stop = 1.0 - largest_class / total_weight;
        let mut best = stop.min(options.abstain_loss);

        if horizon == 0 {
            return best;
        }
        for action in 0..ACTIONS as u8 {
            let mut zero = Vec::new();
            let mut one = Vec::new();
            let mut zero_weight = 0.0;
            let mut one_weight = 0.0;
            for signature in signatures {
                let weight = support[signature].1;
                if (*signature >> action) & 1 == 1 {
                    one.push(*signature);
                    one_weight += weight;
                } else {
                    zero.push(*signature);
                    zero_weight += weight;
                }
            }
            if zero.is_empty() || one.is_empty() {
                continue;
            }
            let candidate = options.action_costs[action as usize]
                + (zero_weight / total_weight) * brute_value(&zero, support, options, horizon - 1)
                + (one_weight / total_weight) * brute_value(&one, support, options, horizon - 1);
            best = best.min(candidate);
        }
        best
    }

    #[test]
    fn memoized_dp_matches_independent_brute_force_for_all_247_subsets() {
        let costs = [0.07, 0.11, 0.13, 0.17, 0.19, 0.23, 0.29, 0.31];
        let opts = DecisionOptions {
            action_costs: costs,
            abstain_loss: 0.24,
            max_horizon: 3,
            max_states: 10_000,
        };
        for mask in 1u16..=255 {
            if mask.count_ones() < 2 {
                continue;
            }
            let hypotheses: Vec<Hypothesis> = (0..8u8)
                .filter(|signature| mask & (1 << *signature) != 0)
                .map(|signature| Hypothesis {
                    signature,
                    decision_class: signature.count_ones() as u8 % 3,
                    prior_weight: 1.0 + f64::from((signature * 7) % 5),
                })
                .collect();
            let support: BTreeMap<u8, (u8, f64)> = hypotheses
                .iter()
                .map(|hypothesis| {
                    (
                        hypothesis.signature,
                        (hypothesis.decision_class, hypothesis.prior_weight),
                    )
                })
                .collect();
            let signatures: Vec<u8> = support.keys().copied().collect();
            let actual = solve_decision(&hypotheses, &opts)
                .unwrap()
                .expected_total_loss;
            let expected = brute_value(&signatures, &support, &opts, opts.max_horizon);
            assert!(
                (actual - expected).abs() < 1e-12,
                "subset mask {mask:#05x}: actual {actual}, expected {expected}"
            );
        }
    }

    #[test]
    fn total_objective_equals_expected_cost_plus_task_and_abstention_losses() {
        let result = solve_decision(&binary_fixture(1.0, 1.0), &options(0.1, 0.2, 1)).unwrap();
        let decomposed = result.expected_experiment_cost
            + result.expected_decision_error
            + result.expected_abstention_loss;
        assert!((result.expected_total_loss - decomposed).abs() < 1e-12);
    }

    #[test]
    fn rejects_empty_duplicate_invalid_mass_cost_horizon_and_state_limit() {
        let opts = options(0.1, 0.2, 1);
        assert!(solve_decision(&[], &opts).is_err());
        let duplicate = [binary_fixture(1.0, 1.0)[0], binary_fixture(1.0, 1.0)[0]];
        assert!(solve_decision(&duplicate, &opts).is_err());
        let invalid_mass = [Hypothesis {
            signature: 0,
            decision_class: 0,
            prior_weight: f64::NAN,
        }];
        assert!(solve_decision(&invalid_mass, &opts).is_err());
        assert!(
            solve_decision(
                &binary_fixture(1.0, 1.0),
                &DecisionOptions {
                    action_costs: [0.0; ACTIONS],
                    ..opts
                }
            )
            .is_err()
        );
        assert!(
            solve_decision(
                &binary_fixture(1.0, 1.0),
                &DecisionOptions {
                    max_horizon: ACTIONS + 1,
                    ..opts
                }
            )
            .is_err()
        );
        assert!(
            solve_decision(
                &binary_fixture(1.0, 1.0),
                &DecisionOptions {
                    max_states: 0,
                    ..opts
                }
            )
            .is_err()
        );
    }

    #[test]
    fn state_budget_exhaustion_fails_closed() {
        let hypotheses = [
            Hypothesis {
                signature: 0,
                decision_class: 0,
                prior_weight: 1.0,
            },
            Hypothesis {
                signature: 1,
                decision_class: 1,
                prior_weight: 1.0,
            },
            Hypothesis {
                signature: 2,
                decision_class: 0,
                prior_weight: 1.0,
            },
            Hypothesis {
                signature: 3,
                decision_class: 1,
                prior_weight: 1.0,
            },
        ];
        let opts = DecisionOptions {
            max_states: 1,
            ..options(0.01, 0.2, 3)
        };
        assert!(solve_decision(&hypotheses, &opts).is_err());
    }
}
