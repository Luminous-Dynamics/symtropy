// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision-focused selection of observations that may distinguish plausible
//! ecological model structures.
//!
//! The value is expected reduction in decision loss, not entropy reduction.
//! Results are conditional on supplied structures, weights, likelihoods, and
//! loss units. They do not establish that the model set is adequate or biological.

use std::collections::{BTreeMap, BTreeSet};

use crate::decision::{ModelOutcome, score_model_ensemble};

const PROBABILITY_TOLERANCE: f64 = 1e-9;

/// One prospective observation design with an explicit likelihood for each
/// possible outcome under each registered model structure.
///
/// Cost must be declared in the same units as the action losses. Each model's
/// outcome likelihoods must sum to one across this design's possible outcomes.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationDesign {
    pub observation_id: String,
    pub loss_equivalent_cost: f64,
    /// outcome -> model ID -> P(outcome | model, observation design)
    pub outcome_likelihoods: BTreeMap<String, BTreeMap<String, f64>>,
}

/// Decision consequences conditional on one possible observation outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeDecision {
    pub outcome: String,
    pub probability: f64,
    pub selected_action: String,
    pub expected_loss_after_observation: f64,
    /// Posterior structure weights after observing this outcome.
    pub posterior_model_weights: BTreeMap<String, f64>,
}

/// Expected value of an observation design under the declared model prior.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationValue {
    pub observation_id: String,
    pub baseline_action: String,
    pub baseline_expected_loss: f64,
    pub expected_post_observation_loss: f64,
    pub gross_value_of_information: f64,
    pub loss_equivalent_cost: f64,
    pub net_decision_value: f64,
    pub outcomes: Vec<OutcomeDecision>,
}

/// Rank observation designs by expected decision-loss reduction minus cost.
///
/// Only actions admissible under every registered model are considered. If the
/// model set has no shared admissible action, selection fails closed.
///
/// This is a Bayesian decision calculation conditional on explicit model
/// weights. It is not robust to omitted structures by itself; a separate
/// model-set adequacy assessment is required before consequential qualification.
pub fn score_observation_designs(
    models: &[ModelOutcome],
    designs: &[ObservationDesign],
) -> Result<Vec<ObservationValue>, Vec<String>> {
    if designs.is_empty() {
        return Err(vec!["observation design set must not be empty".to_owned()]);
    }

    // Reuse the canonical action, model-ID, loss, and model-weight acceptance rule.
    score_model_ensemble(models, 0.5)?;

    let model_ids: BTreeSet<&str> = models.iter().map(|model| model.model_id.as_str()).collect();
    let shared_actions: BTreeSet<String> = models[0]
        .admissible_actions
        .iter()
        .filter(|action| {
            models
                .iter()
                .all(|model| model.admissible_actions.contains(*action))
        })
        .cloned()
        .collect();
    if shared_actions.is_empty() {
        return Err(vec![
            "model set has no action admissible under every registered structure".to_owned(),
        ]);
    }

    let prior_weights: Vec<f64> = models.iter().map(|model| model.weight).collect();
    let (baseline_action, baseline_expected_loss) =
        best_action(&shared_actions, models, &prior_weights)?;

    let mut design_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(designs.len());
    let mut errors = Vec::new();

    for (index, design) in designs.iter().enumerate() {
        let prefix = format!("designs[{index}]");
        if design.observation_id.trim().is_empty() {
            errors.push(format!("{prefix}.observation_id must not be empty"));
            continue;
        }
        if !design_ids.insert(design.observation_id.as_str()) {
            errors.push(format!(
                "duplicate observation design ID '{}'",
                design.observation_id
            ));
            continue;
        }
        if !design.loss_equivalent_cost.is_finite() || design.loss_equivalent_cost < 0.0 {
            errors.push(format!(
                "{prefix}.loss_equivalent_cost must be finite and non-negative"
            ));
            continue;
        }
        if design.outcome_likelihoods.is_empty() {
            errors.push(format!("{prefix}.outcome_likelihoods must not be empty"));
            continue;
        }

        let mut valid_design = true;
        let mut likelihood_sums: BTreeMap<&str, f64> = models
            .iter()
            .map(|model| (model.model_id.as_str(), 0.0))
            .collect();

        for (outcome, likelihoods_by_model) in &design.outcome_likelihoods {
            if outcome.trim().is_empty() {
                errors.push(format!("{prefix} contains an empty outcome name"));
                valid_design = false;
            }
            for model_id in &model_ids {
                match likelihoods_by_model.get(*model_id) {
                    Some(probability)
                        if probability.is_finite() && (0.0..=1.0).contains(probability) =>
                    {
                        *likelihood_sums
                            .get_mut(model_id)
                            .expect("model IDs were initialized") += probability;
                    }
                    Some(_) => {
                        errors.push(format!(
                            "{prefix}.outcome_likelihoods.{outcome}.{model_id} must be a finite probability in [0, 1]"
                        ));
                        valid_design = false;
                    }
                    None => {
                        errors.push(format!(
                            "{prefix}.outcome_likelihoods.{outcome} is missing model '{model_id}'"
                        ));
                        valid_design = false;
                    }
                }
            }
            if let Some(extra) = likelihoods_by_model
                .keys()
                .find(|model_id| !model_ids.contains(model_id.as_str()))
            {
                errors.push(format!(
                    "{prefix}.outcome_likelihoods.{outcome} contains unknown model '{extra}'"
                ));
                valid_design = false;
            }
        }

        for (model_id, total) in &likelihood_sums {
            if !total.is_finite() || (total - 1.0).abs() > PROBABILITY_TOLERANCE {
                errors.push(format!(
                    "{prefix} likelihoods for model '{model_id}' sum to {total}, not one"
                ));
                valid_design = false;
            }
        }
        if !valid_design {
            continue;
        }

        let mut expected_post_observation_loss = 0.0;
        let mut outcome_decisions = Vec::with_capacity(design.outcome_likelihoods.len());
        let mut probability_sum = 0.0;

        for (outcome, likelihoods_by_model) in &design.outcome_likelihoods {
            let outcome_probability: f64 = models
                .iter()
                .map(|model| {
                    model.weight
                        * likelihoods_by_model
                            .get(&model.model_id)
                            .expect("likelihood key set was validated")
                })
                .sum();
            if !outcome_probability.is_finite() || outcome_probability <= 0.0 {
                errors.push(format!(
                    "{prefix} outcome '{outcome}' has zero or invalid probability under the model prior"
                ));
                valid_design = false;
                continue;
            }

            let posterior_weights: Vec<f64> = models
                .iter()
                .map(|model| {
                    model.weight
                        * likelihoods_by_model
                            .get(&model.model_id)
                            .expect("likelihood key set was validated")
                        / outcome_probability
                })
                .collect();
            let (selected_action, loss_after_observation) =
                match best_action(&shared_actions, models, &posterior_weights) {
                    Ok(best) => best,
                    Err(mut action_errors) => {
                        errors.append(&mut action_errors);
                        valid_design = false;
                        continue;
                    }
                };

            expected_post_observation_loss += outcome_probability * loss_after_observation;
            probability_sum += outcome_probability;
            let posterior_model_weights = models
                .iter()
                .zip(&posterior_weights)
                .map(|(model, weight)| (model.model_id.clone(), *weight))
                .collect();
            outcome_decisions.push(OutcomeDecision {
                outcome: outcome.clone(),
                probability: outcome_probability,
                selected_action,
                expected_loss_after_observation: loss_after_observation,
                posterior_model_weights,
            });
        }

        if !valid_design {
            continue;
        }
        if (probability_sum - 1.0).abs() > PROBABILITY_TOLERANCE {
            errors.push(format!(
                "{prefix} outcome probabilities sum to {probability_sum}, not one"
            ));
            continue;
        }

        let raw_gross_value = baseline_expected_loss - expected_post_observation_loss;
        if !raw_gross_value.is_finite() || raw_gross_value < -PROBABILITY_TOLERANCE {
            errors.push(format!(
                "{prefix} produced invalid negative expected information value ({raw_gross_value})"
            ));
            continue;
        }
        let gross_value_of_information = raw_gross_value.max(0.0);
        let net_decision_value = gross_value_of_information - design.loss_equivalent_cost;
        results.push(ObservationValue {
            observation_id: design.observation_id.clone(),
            baseline_action: baseline_action.clone(),
            baseline_expected_loss,
            expected_post_observation_loss,
            gross_value_of_information,
            loss_equivalent_cost: design.loss_equivalent_cost,
            net_decision_value,
            outcomes: outcome_decisions,
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    results.sort_by(|left, right| {
        right
            .net_decision_value
            .total_cmp(&left.net_decision_value)
            .then_with(|| left.observation_id.cmp(&right.observation_id))
    });
    Ok(results)
}

fn best_action(
    actions: &BTreeSet<String>,
    models: &[ModelOutcome],
    weights: &[f64],
) -> Result<(String, f64), Vec<String>> {
    if actions.is_empty() || weights.len() != models.len() {
        return Err(vec![
            "best-action evaluation requires non-empty actions and one weight per model".to_owned(),
        ]);
    }

    actions
        .iter()
        .map(|action| {
            let loss = models
                .iter()
                .zip(weights)
                .map(|(model, weight)| {
                    weight
                        * model
                            .action_losses
                            .get(action)
                            .expect("shared action was validated by the model scorer")
                })
                .sum::<f64>();
            (action.clone(), loss)
        })
        .min_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        })
        .ok_or_else(|| vec!["no admissible action can be evaluated".to_owned()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_disagreeing_models() -> Vec<ModelOutcome> {
        vec![
            ModelOutcome {
                model_id: "structure_a".to_owned(),
                weight: 0.5,
                action_losses: BTreeMap::from([
                    ("act_a".to_owned(), 0.0),
                    ("act_b".to_owned(), 10.0),
                ]),
                admissible_actions: BTreeSet::from(["act_a".to_owned(), "act_b".to_owned()]),
            },
            ModelOutcome {
                model_id: "structure_b".to_owned(),
                weight: 0.5,
                action_losses: BTreeMap::from([
                    ("act_a".to_owned(), 10.0),
                    ("act_b".to_owned(), 0.0),
                ]),
                admissible_actions: BTreeSet::from(["act_a".to_owned(), "act_b".to_owned()]),
            },
        ]
    }

    fn informative_design(cost: f64) -> ObservationDesign {
        ObservationDesign {
            observation_id: "plume_sample".to_owned(),
            loss_equivalent_cost: cost,
            outcome_likelihoods: BTreeMap::from([
                (
                    "cue_a".to_owned(),
                    BTreeMap::from([
                        ("structure_a".to_owned(), 0.9),
                        ("structure_b".to_owned(), 0.1),
                    ]),
                ),
                (
                    "cue_b".to_owned(),
                    BTreeMap::from([
                        ("structure_a".to_owned(), 0.1),
                        ("structure_b".to_owned(), 0.9),
                    ]),
                ),
            ]),
        }
    }

    #[test]
    fn values_observations_by_expected_decision_loss_reduction() {
        let result =
            score_observation_designs(&two_disagreeing_models(), &[informative_design(1.0)])
                .expect("valid signal");
        let value = &result[0];
        assert!((value.baseline_expected_loss - 5.0).abs() < 1e-9);
        assert!((value.expected_post_observation_loss - 1.0).abs() < 1e-9);
        assert!((value.gross_value_of_information - 4.0).abs() < 1e-9);
        assert!((value.net_decision_value - 3.0).abs() < 1e-9);
        assert_eq!(value.outcomes.len(), 2);
        assert_eq!(value.outcomes[0].selected_action, "act_a");
        assert_eq!(value.outcomes[1].selected_action, "act_b");
        assert!((value.outcomes[0].posterior_model_weights["structure_a"] - 0.9).abs() < 1e-9);
        assert!((value.outcomes[0].posterior_model_weights["structure_b"] - 0.1).abs() < 1e-9);
        assert!((value.outcomes[1].posterior_model_weights["structure_a"] - 0.1).abs() < 1e-9);
        assert!((value.outcomes[1].posterior_model_weights["structure_b"] - 0.9).abs() < 1e-9);
    }

    #[test]
    fn entropy_relevant_but_decision_irrelevant_observation_has_zero_value() {
        let mut models = two_disagreeing_models();
        for model in &mut models {
            model.action_losses =
                BTreeMap::from([("act_a".to_owned(), 2.0), ("act_b".to_owned(), 6.0)]);
        }
        let result =
            score_observation_designs(&models, &[informative_design(0.0)]).expect("valid signal");
        assert!(result[0].gross_value_of_information.abs() < 1e-9);
        assert!(result[0].net_decision_value.abs() < 1e-9);
    }

    #[test]
    fn costs_are_subtracted_in_declared_loss_equivalent_units() {
        let result =
            score_observation_designs(&two_disagreeing_models(), &[informative_design(5.0)])
                .expect("valid expensive signal");
        assert!((result[0].net_decision_value + 1.0).abs() < 1e-9);
    }

    #[test]
    fn candidate_order_is_deterministic_and_cost_sensitive() {
        let cheaper = informative_design(1.0);
        let mut expensive = informative_design(2.0);
        expensive.observation_id = "second_sample".to_owned();
        expensive.outcome_likelihoods = cheaper.outcome_likelihoods.clone();
        let result = score_observation_designs(&two_disagreeing_models(), &[expensive, cheaper])
            .expect("valid designs");
        assert_eq!(result[0].observation_id, "plume_sample");
        assert!(result[0].net_decision_value > result[1].net_decision_value);
    }

    #[test]
    fn rejects_unnormalized_likelihoods() {
        let mut design = informative_design(0.0);
        design
            .outcome_likelihoods
            .get_mut("cue_b")
            .expect("cue_b")
            .insert("structure_a".to_owned(), 0.2);
        assert!(score_observation_designs(&two_disagreeing_models(), &[design]).is_err());
    }

    #[test]
    fn rejects_missing_model_likelihood() {
        let mut design = informative_design(0.0);
        design
            .outcome_likelihoods
            .get_mut("cue_a")
            .expect("cue_a")
            .remove("structure_b");
        assert!(score_observation_designs(&two_disagreeing_models(), &[design]).is_err());
    }

    #[test]
    fn rejects_unknown_model_likelihood() {
        let mut design = informative_design(0.0);
        design
            .outcome_likelihoods
            .get_mut("cue_a")
            .expect("cue_a")
            .insert("unregistered_structure".to_owned(), 0.0);
        assert!(score_observation_designs(&two_disagreeing_models(), &[design]).is_err());
    }

    #[test]
    fn rejects_no_common_admissible_action() {
        let mut models = two_disagreeing_models();
        models[0].admissible_actions = BTreeSet::from(["act_a".to_owned()]);
        models[1].admissible_actions = BTreeSet::from(["act_b".to_owned()]);
        assert!(score_observation_designs(&models, &[informative_design(0.0)]).is_err());
    }

    #[test]
    fn rejects_zero_probability_declared_outcome() {
        let mut design = informative_design(0.0);
        design.outcome_likelihoods.insert(
            "impossible_cue".to_owned(),
            BTreeMap::from([
                ("structure_a".to_owned(), 0.0),
                ("structure_b".to_owned(), 0.0),
            ]),
        );
        assert!(score_observation_designs(&two_disagreeing_models(), &[design]).is_err());
    }

    #[test]
    fn rejects_duplicate_observation_ids() {
        let first = informative_design(0.0);
        let mut second = informative_design(0.1);
        second.outcome_likelihoods = first.outcome_likelihoods.clone();
        assert!(score_observation_designs(&two_disagreeing_models(), &[first, second]).is_err());
    }

    #[test]
    fn rejects_negative_or_non_finite_cost() {
        let mut negative = informative_design(-1.0);
        negative.observation_id = "negative_cost".to_owned();
        let mut infinite = informative_design(f64::INFINITY);
        infinite.observation_id = "infinite_cost".to_owned();
        assert!(score_observation_designs(&two_disagreeing_models(), &[negative]).is_err());
        assert!(score_observation_designs(&two_disagreeing_models(), &[infinite]).is_err());
    }
}
