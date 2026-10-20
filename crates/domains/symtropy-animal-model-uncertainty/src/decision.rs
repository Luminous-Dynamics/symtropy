// SPDX-License-Identifier: AGPL-3.0-or-later

//! Model-set decision-risk metrics for ecological policies.
//!
//! This module scores actions across explicit model alternatives. It does not
//! choose a normative objective: callers must define losses and hard constraints.

use std::collections::{BTreeMap, BTreeSet};

const WEIGHT_SUM_TOLERANCE: f64 = 1e-9;

/// One registered model's losses and admissible actions for a frozen scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelOutcome {
    pub model_id: String,
    pub weight: f64,
    pub action_losses: BTreeMap<String, f64>,
    pub admissible_actions: BTreeSet<String>,
}

/// Risk summary for a single action, conditioned on the complete declared model set.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionRisk {
    pub action: String,
    pub expected_loss: f64,
    pub worst_case_loss: f64,
    /// Mean loss in the upper tail beyond `cvar_alpha`, weighted by model probability.
    pub cvar_loss: f64,
    /// `None` means the action violates that model's admissibility constraints.
    pub per_model_regret: BTreeMap<String, Option<f64>>,
    /// Only defined if the action is admissible under every declared model.
    pub max_regret: Option<f64>,
    pub admissible_across_all_models: bool,
}

/// Score every shared action across a complete set of model outcomes.
///
/// The model weights must be finite, positive and sum to one. All models must
/// report the same actions and finite non-negative losses. Missing model/action
/// values fail closed instead of being dropped or replaced with zero.
/// `cvar_alpha` must be strictly between zero and one (for example, 0.9).
pub fn score_model_ensemble(
    models: &[ModelOutcome],
    cvar_alpha: f64,
) -> Result<Vec<ActionRisk>, Vec<String>> {
    let mut errors = Vec::new();
    if models.is_empty() {
        return Err(vec!["model set must not be empty".to_owned()]);
    }
    if !cvar_alpha.is_finite() || cvar_alpha == 0.0 || !(0.0..1.0).contains(&cvar_alpha) {
        errors.push("cvar_alpha must be finite and strictly between zero and one".to_owned());
    }

    let mut model_ids = BTreeSet::new();
    let mut weight_sum = 0.0;
    let first_actions: BTreeSet<String> = models[0].action_losses.keys().cloned().collect();
    if first_actions.is_empty() {
        errors.push("each model must declare at least one action".to_owned());
    }

    for (index, model) in models.iter().enumerate() {
        if model.model_id.trim().is_empty() {
            errors.push(format!("models[{index}].model_id must not be empty"));
        } else if !model_ids.insert(model.model_id.as_str()) {
            errors.push(format!("duplicate model id '{}'", model.model_id));
        }

        if !model.weight.is_finite() || model.weight <= 0.0 {
            errors.push(format!(
                "model '{}' weight must be finite and positive",
                model.model_id
            ));
        } else {
            weight_sum += model.weight;
        }

        let actions: BTreeSet<String> = model.action_losses.keys().cloned().collect();
        if actions != first_actions {
            errors.push(format!(
                "model '{}' action set differs from the complete model set",
                model.model_id
            ));
        }

        if model.admissible_actions.is_empty() {
            errors.push(format!(
                "model '{}' must declare at least one admissible action",
                model.model_id
            ));
        }
        for action in &model.admissible_actions {
            if !model.action_losses.contains_key(action) {
                errors.push(format!(
                    "model '{}' marks undeclared action '{action}' admissible",
                    model.model_id
                ));
            }
        }
        for (action, loss) in &model.action_losses {
            if action.trim().is_empty() {
                errors.push(format!(
                    "model '{}' contains an empty action name",
                    model.model_id
                ));
            }
            if !loss.is_finite() || *loss < 0.0 {
                errors.push(format!(
                    "model '{}' action '{action}' loss must be finite and non-negative",
                    model.model_id
                ));
            }
        }
    }

    if !weight_sum.is_finite() || (weight_sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
        errors.push(format!(
            "model weights must sum to one within {WEIGHT_SUM_TOLERANCE}; got {weight_sum}"
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut scored = Vec::with_capacity(first_actions.len());
    for action in first_actions {
        let mut expected_loss = 0.0;
        let mut worst_case_loss: f64 = 0.0;
        let mut conditional_losses = Vec::with_capacity(models.len());
        let mut per_model_regret = BTreeMap::new();
        let mut admissible_across_all_models = true;

        for model in models {
            let loss = model.action_losses[&action];
            expected_loss += model.weight * loss;
            worst_case_loss = worst_case_loss.max(loss);
            conditional_losses.push((loss, model.weight));

            if model.admissible_actions.contains(&action) {
                let best_admissible_loss = model
                    .admissible_actions
                    .iter()
                    .map(|candidate| model.action_losses[candidate])
                    .fold(f64::INFINITY, f64::min);
                per_model_regret.insert(
                    model.model_id.clone(),
                    Some((loss - best_admissible_loss).max(0.0)),
                );
            } else {
                admissible_across_all_models = false;
                per_model_regret.insert(model.model_id.clone(), None);
            }
        }

        conditional_losses.sort_by(|left, right| right.0.total_cmp(&left.0));
        let tail_mass = 1.0 - cvar_alpha;
        let mut remaining_mass = tail_mass;
        let mut weighted_tail_loss = 0.0;
        for (loss, weight) in conditional_losses {
            if remaining_mass <= 0.0 {
                break;
            }
            let included_mass = weight.min(remaining_mass);
            weighted_tail_loss += loss * included_mass;
            remaining_mass -= included_mass;
        }

        if remaining_mass > WEIGHT_SUM_TOLERANCE {
            errors.push(format!(
                "action '{action}' could not fill the requested CVaR tail"
            ));
            continue;
        }
        let cvar_loss = weighted_tail_loss / tail_mass;
        let max_regret = if admissible_across_all_models {
            per_model_regret
                .values()
                .filter_map(|regret| *regret)
                .reduce(f64::max)
        } else {
            None
        };

        if !expected_loss.is_finite() || !worst_case_loss.is_finite() || !cvar_loss.is_finite() {
            errors.push(format!(
                "action '{action}' produced a non-finite risk summary"
            ));
            continue;
        }

        scored.push(ActionRisk {
            action,
            expected_loss,
            worst_case_loss,
            cvar_loss,
            per_model_regret,
            max_regret,
            admissible_across_all_models,
        });
    }

    if errors.is_empty() {
        Ok(scored)
    } else {
        Err(errors)
    }
}

/// A defensible interval for the probability/weight assigned to one model structure.
///
/// The interval bounds represent uncertainty about model weights, not parameter
/// uncertainty or within-model stochastic noise. The intervals are coupled by
/// the requirement that all selected weights sum to one.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelWeightInterval {
    pub model_id: String,
    pub lower: f64,
    pub upper: f64,
}

/// Extremal expected-loss values over a box-constrained model-weight simplex.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightSensitivity {
    pub action: String,
    pub minimum_expected_loss: f64,
    pub maximum_expected_loss: f64,
    /// Feasible weights attaining the minimum within floating-point tolerance.
    pub minimizing_weights: BTreeMap<String, f64>,
    /// Feasible weights attaining the maximum within floating-point tolerance.
    pub maximizing_weights: BTreeMap<String, f64>,
}

/// Qualitative result of comparing two actions under the same admissible weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferenceStability {
    FirstStrictlyBetter,
    FirstNonInferior,
    SecondStrictlyBetter,
    SecondNonInferior,
    Equivalent,
    CanFlip,
}

impl PreferenceStability {
    /// Stable machine-readable label for reports and receipts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstStrictlyBetter => "first_strictly_better",
            Self::FirstNonInferior => "first_noninferior_within_margin",
            Self::SecondStrictlyBetter => "second_strictly_better",
            Self::SecondNonInferior => "second_noninferior_within_margin",
            Self::Equivalent => "equivalent",
            Self::CanFlip => "ranking_can_flip",
        }
    }
}

/// Worst/best paired loss difference for two actions under a shared weight vector.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionPreferenceRange {
    pub first_action: String,
    pub second_action: String,
    /// Hard-constraint availability, distinct from the loss comparison.
    pub first_admissible_across_all_models: bool,
    pub second_admissible_across_all_models: bool,
    /// Extremes of loss(first_action) - loss(second_action).
    /// Negative values favor the first action; positive values favor the second.
    pub minimum_loss_difference: f64,
    pub maximum_loss_difference: f64,
    /// Feasible weights attaining the minimum difference within tolerance.
    pub minimizing_weights: BTreeMap<String, f64>,
    /// Feasible weights attaining the maximum difference within tolerance.
    pub maximizing_weights: BTreeMap<String, f64>,
    pub stability: PreferenceStability,
}

/// Bound an action's expected loss when model-structure weights are uncertain.
///
/// This solves the linear optimization problem over all weights satisfying the
/// declared per-model intervals and summing to one. It does not assume that
/// the point weights are scientifically calibrated, nor infer model weights
/// from fit. The point weights in `models` must be normalized and lie inside
/// their respective intervals. Missing models, losses or infeasible bounds fail
/// closed. Results are epistemic sensitivity bounds, not probabilistic
/// confidence intervals.
pub fn score_weight_sensitivity(
    models: &[ModelOutcome],
    action: &str,
    intervals: &[ModelWeightInterval],
) -> Result<WeightSensitivity, Vec<String>> {
    let mut errors = Vec::new();
    if models.is_empty() {
        errors.push("model set must not be empty".to_owned());
    }
    if action.trim().is_empty() {
        errors.push("action must not be empty".to_owned());
    }
    if intervals.is_empty() {
        errors.push("weight intervals must not be empty".to_owned());
    }

    let mut model_ids = BTreeSet::new();
    let mut nominal_weight_sum = 0.0;
    let mut losses = BTreeMap::new();
    for (index, model) in models.iter().enumerate() {
        if model.model_id.trim().is_empty() {
            errors.push(format!("models[{index}].model_id must not be empty"));
        } else if !model_ids.insert(model.model_id.clone()) {
            errors.push(format!("duplicate model id '{}'", model.model_id));
        }
        if !model.weight.is_finite() || model.weight <= 0.0 {
            errors.push(format!(
                "model '{}' weight must be finite and positive",
                model.model_id
            ));
        } else {
            nominal_weight_sum += model.weight;
        }
        match model.action_losses.get(action) {
            Some(loss) if loss.is_finite() && *loss >= 0.0 => {
                losses.insert(model.model_id.clone(), *loss);
            }
            Some(_) => errors.push(format!(
                "model '{}' action '{action}' loss must be finite and non-negative",
                model.model_id
            )),
            None => errors.push(format!(
                "model '{}' is missing loss for action '{action}'",
                model.model_id
            )),
        }
    }
    if !nominal_weight_sum.is_finite() || (nominal_weight_sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
        errors.push(format!(
            "nominal model weights must sum to one within {WEIGHT_SUM_TOLERANCE}; got {nominal_weight_sum}"
        ));
    }

    let mut bounds = BTreeMap::new();
    let mut lower_sum = 0.0;
    let mut upper_sum = 0.0;
    for (index, interval) in intervals.iter().enumerate() {
        if interval.model_id.trim().is_empty() {
            errors.push(format!("intervals[{index}].model_id must not be empty"));
        } else if bounds
            .insert(interval.model_id.clone(), (interval.lower, interval.upper))
            .is_some()
        {
            errors.push(format!(
                "duplicate weight interval for model '{}'",
                interval.model_id
            ));
        }
        if !interval.lower.is_finite()
            || !interval.upper.is_finite()
            || interval.lower < 0.0
            || interval.upper > 1.0
            || interval.lower > interval.upper
        {
            errors.push(format!(
                "weight interval for '{}' must satisfy 0 <= lower <= upper <= 1",
                interval.model_id
            ));
        } else {
            lower_sum += interval.lower;
            upper_sum += interval.upper;
        }
    }
    let interval_ids: BTreeSet<String> = bounds.keys().cloned().collect();
    if interval_ids != model_ids {
        errors
            .push("weight interval model IDs must exactly match the declared model set".to_owned());
    }
    if lower_sum > 1.0 {
        errors.push(format!(
            "weight interval lower bounds sum to {lower_sum}, exceeding one"
        ));
    }
    if upper_sum < 1.0 {
        errors.push(format!(
            "weight interval upper bounds sum to {upper_sum}, below one"
        ));
    }
    for model in models {
        if let Some((lower, upper)) = bounds.get(&model.model_id)
            && model.weight.is_finite()
            && (model.weight < *lower - WEIGHT_SUM_TOLERANCE
                || model.weight > *upper + WEIGHT_SUM_TOLERANCE)
        {
            errors.push(format!(
                "nominal weight for '{}' falls outside its declared interval",
                model.model_id
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let minimizing_weights = extremal_weight_assignment(models, &losses, &bounds, false)?;
    let maximizing_weights = extremal_weight_assignment(models, &losses, &bounds, true)?;
    let minimum_expected_loss = weighted_loss(&losses, &minimizing_weights)?;
    let maximum_expected_loss = weighted_loss(&losses, &maximizing_weights)?;
    if minimum_expected_loss > maximum_expected_loss + WEIGHT_SUM_TOLERANCE {
        return Err(vec![
            "computed minimum expected loss exceeds maximum expected loss".to_owned(),
        ]);
    }

    Ok(WeightSensitivity {
        action: action.to_owned(),
        minimum_expected_loss,
        maximum_expected_loss,
        minimizing_weights,
        maximizing_weights,
    })
}

/// Compare two actions using the same feasible model weights, avoiding the
/// false inference that independent loss envelopes imply a ranking reversal.
/// Positive differences favor the second action; negative differences favor
/// the first. All model outcomes and intervals are validated before scoring.
pub fn compare_actions_under_weight_uncertainty(
    models: &[ModelOutcome],
    first_action: &str,
    second_action: &str,
    intervals: &[ModelWeightInterval],
    practical_loss_difference_margin: f64,
) -> Result<ActionPreferenceRange, Vec<String>> {
    if first_action == second_action {
        return Err(vec!["actions to compare must be distinct".to_owned()]);
    }
    if !practical_loss_difference_margin.is_finite() || practical_loss_difference_margin < 0.0 {
        return Err(vec![
            "practical loss-difference margin must be finite and non-negative".to_owned(),
        ]);
    }
    score_weight_sensitivity(models, first_action, intervals)?;
    score_weight_sensitivity(models, second_action, intervals)?;

    let differences: BTreeMap<String, f64> = models
        .iter()
        .map(|model| {
            (
                model.model_id.clone(),
                model.action_losses[first_action] - model.action_losses[second_action],
            )
        })
        .collect();
    if differences.values().any(|value| !value.is_finite()) {
        return Err(vec![
            "action comparison produced a non-finite loss difference".to_owned(),
        ]);
    }
    let bounds: BTreeMap<String, (f64, f64)> = intervals
        .iter()
        .map(|interval| (interval.model_id.clone(), (interval.lower, interval.upper)))
        .collect();
    let minimizing_weights = extremal_weight_assignment(models, &differences, &bounds, false)?;
    let maximizing_weights = extremal_weight_assignment(models, &differences, &bounds, true)?;
    let minimum_loss_difference = weighted_loss(&differences, &minimizing_weights)?;
    let maximum_loss_difference = weighted_loss(&differences, &maximizing_weights)?;
    let stability = if maximum_loss_difference < -practical_loss_difference_margin {
        PreferenceStability::FirstStrictlyBetter
    } else if minimum_loss_difference > practical_loss_difference_margin {
        PreferenceStability::SecondStrictlyBetter
    } else if minimum_loss_difference >= -practical_loss_difference_margin
        && maximum_loss_difference <= practical_loss_difference_margin
    {
        PreferenceStability::Equivalent
    } else if maximum_loss_difference <= practical_loss_difference_margin {
        PreferenceStability::FirstNonInferior
    } else if minimum_loss_difference >= -practical_loss_difference_margin {
        PreferenceStability::SecondNonInferior
    } else {
        PreferenceStability::CanFlip
    };

    Ok(ActionPreferenceRange {
        first_action: first_action.to_owned(),
        second_action: second_action.to_owned(),
        first_admissible_across_all_models: models
            .iter()
            .all(|model| model.admissible_actions.contains(first_action)),
        second_admissible_across_all_models: models
            .iter()
            .all(|model| model.admissible_actions.contains(second_action)),
        minimum_loss_difference,
        maximum_loss_difference,
        minimizing_weights,
        maximizing_weights,
        stability,
    })
}

fn extremal_weight_assignment(
    models: &[ModelOutcome],
    values: &BTreeMap<String, f64>,
    bounds: &BTreeMap<String, (f64, f64)>,
    maximize: bool,
) -> Result<BTreeMap<String, f64>, Vec<String>> {
    let mut weights: BTreeMap<String, f64> = bounds
        .iter()
        .map(|(model_id, (lower, _))| (model_id.clone(), *lower))
        .collect();
    let mut remaining = (1.0 - weights.values().sum::<f64>()).max(0.0);
    let mut order: Vec<&ModelOutcome> = models.iter().collect();
    order.sort_by(|left, right| {
        let left_loss = values[&left.model_id];
        let right_loss = values[&right.model_id];
        let ordering = if maximize {
            right_loss.total_cmp(&left_loss)
        } else {
            left_loss.total_cmp(&right_loss)
        };
        ordering.then_with(|| left.model_id.cmp(&right.model_id))
    });
    for model in order {
        if remaining <= 0.0 {
            break;
        }
        let Some((_, upper)) = bounds.get(&model.model_id) else {
            return Err(vec![format!(
                "missing weight interval for model '{}'",
                model.model_id
            )]);
        };
        let Some(current) = weights.get(&model.model_id).copied() else {
            return Err(vec![format!(
                "missing weight allocation for model '{}'",
                model.model_id
            )]);
        };
        let addition = (*upper - current).min(remaining);
        let Some(weight) = weights.get_mut(&model.model_id) else {
            return Err(vec![format!(
                "missing mutable weight allocation for model '{}'",
                model.model_id
            )]);
        };
        *weight += addition;
        remaining = (remaining - addition).max(0.0);
    }
    if remaining > 0.0 {
        return Err(vec![
            "weight intervals could not allocate a normalized weight vector".to_owned(),
        ]);
    }
    Ok(weights)
}

fn weighted_loss(
    losses: &BTreeMap<String, f64>,
    weights: &BTreeMap<String, f64>,
) -> Result<f64, Vec<String>> {
    let mut total = 0.0;
    for (model_id, weight) in weights {
        let Some(loss) = losses.get(model_id) else {
            return Err(vec![format!(
                "missing loss for model '{model_id}' in weight-sensitivity report"
            )]);
        };
        total += loss * weight;
    }
    if total.is_finite() {
        Ok(total)
    } else {
        Err(vec![
            "weight-sensitivity calculation produced non-finite expected loss".to_owned(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_models() -> Vec<ModelOutcome> {
        let all_actions = ["graze", "retreat", "wait"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let second_actions = ["retreat", "wait"].into_iter().map(str::to_owned).collect();

        vec![
            ModelOutcome {
                model_id: "predator_coupled".to_owned(),
                weight: 0.6,
                action_losses: BTreeMap::from([
                    ("graze".to_owned(), 2.0),
                    ("retreat".to_owned(), 5.0),
                    ("wait".to_owned(), 3.0),
                ]),
                admissible_actions: all_actions,
            },
            ModelOutcome {
                model_id: "predator_weak".to_owned(),
                weight: 0.4,
                action_losses: BTreeMap::from([
                    ("graze".to_owned(), 9.0),
                    ("retreat".to_owned(), 4.0),
                    ("wait".to_owned(), 3.0),
                ]),
                admissible_actions: second_actions,
            },
        ]
    }

    #[test]
    fn reports_expected_worst_case_and_tail_loss() {
        let scores = score_model_ensemble(&example_models(), 0.9).expect("valid models");
        let graze = scores
            .iter()
            .find(|score| score.action == "graze")
            .expect("graze score");
        assert!((graze.expected_loss - 4.8).abs() < 1e-9);
        assert_eq!(graze.worst_case_loss, 9.0);
        assert_eq!(graze.cvar_loss, 9.0);
        assert!(!graze.admissible_across_all_models);
        assert_eq!(graze.max_regret, None);
    }

    #[test]
    fn safe_action_has_complete_regret_and_lower_tail_risk() {
        let scores = score_model_ensemble(&example_models(), 0.9).expect("valid models");
        let wait = scores
            .iter()
            .find(|score| score.action == "wait")
            .expect("wait score");
        assert!(wait.admissible_across_all_models);
        assert_eq!(wait.expected_loss, 3.0);
        assert_eq!(wait.worst_case_loss, 3.0);
        assert_eq!(wait.cvar_loss, 3.0);
        assert_eq!(wait.max_regret, Some(1.0));
    }

    #[test]
    fn incomplete_action_set_is_rejected() {
        let mut models = example_models();
        models[1].action_losses.remove("wait");
        let errors =
            score_model_ensemble(&models, 0.9).expect_err("missing model/action must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("action set differs"))
        );
    }

    #[test]
    fn weights_must_sum_to_one() {
        let mut models = example_models();
        models[0].weight = 0.4;
        let errors = score_model_ensemble(&models, 0.9).expect_err("bad weights must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("weights must sum to one"))
        );
    }

    #[test]
    fn duplicate_model_ids_are_rejected() {
        let mut models = example_models();
        models[1].model_id = models[0].model_id.clone();
        let errors = score_model_ensemble(&models, 0.9).expect_err("duplicate model IDs must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate model id"))
        );
    }

    #[test]
    fn non_finite_losses_are_rejected() {
        let mut models = example_models();
        models[0].action_losses.insert("wait".to_owned(), f64::NAN);
        let errors = score_model_ensemble(&models, 0.9).expect_err("NaN loss must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("finite and non-negative"))
        );
    }

    #[test]
    fn cvar_alpha_must_be_strictly_between_zero_and_one() {
        let errors = score_model_ensemble(&example_models(), 1.0).expect_err("alpha 1 must fail");
        assert!(errors.iter().any(|error| error.contains("cvar_alpha")));
    }

    #[test]
    fn cvar_near_one_keeps_small_tail_mass() {
        let scores = score_model_ensemble(&example_models(), 1.0 - 1e-12)
            .expect("valid high-confidence tail");
        let graze = scores
            .iter()
            .find(|score| score.action == "graze")
            .expect("graze score");
        assert_eq!(graze.cvar_loss, 9.0);
    }

    #[test]
    fn cvar_alpha_zero_is_rejected() {
        let errors =
            score_model_ensemble(&example_models(), 0.0).expect_err("alpha zero must fail");
        assert!(errors.iter().any(|error| error.contains("cvar_alpha")));
    }

    #[test]
    fn no_action_is_called_globally_admissible_without_unanimity() {
        let mut models = example_models();
        models[0].admissible_actions = ["graze"].into_iter().map(str::to_owned).collect();
        models[1].admissible_actions = ["retreat"].into_iter().map(str::to_owned).collect();
        let scores = score_model_ensemble(&models, 0.9).expect("valid outcomes");
        assert!(
            scores
                .iter()
                .all(|score| !score.admissible_across_all_models)
        );
    }

    #[test]
    fn weight_sensitivity_bounds_decision_loss_under_model_weight_ambiguity() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.2,
                upper: 0.6,
            },
        ];
        let result =
            score_weight_sensitivity(&models, "retreat", &intervals).expect("feasible intervals");
        assert!((result.minimum_expected_loss - 4.4).abs() < 1e-9);
        assert!((result.maximum_expected_loss - 4.8).abs() < 1e-9);
        assert!((result.minimizing_weights["predator_coupled"] - 0.4).abs() < 1e-9);
        assert!((result.minimizing_weights["predator_weak"] - 0.6).abs() < 1e-9);
        assert!((result.maximizing_weights["predator_coupled"] - 0.8).abs() < 1e-9);
        assert!((result.maximizing_weights["predator_weak"] - 0.2).abs() < 1e-9);
    }

    #[test]
    fn weight_sensitivity_allocates_small_residual_mass_for_large_losses() {
        let mut models = example_models();
        models[0].weight = 0.5;
        models[1].weight = 0.5;
        models[0].action_losses.insert("retreat".to_owned(), 1.0);
        models[1]
            .action_losses
            .insert("retreat".to_owned(), 1_000_000_000_000.0);
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.499_999_999_7,
                upper: 0.500_000_000_3,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.499_999_999_7,
                upper: 0.500_000_000_3,
            },
        ];
        let result = score_weight_sensitivity(&models, "retreat", &intervals)
            .expect("narrow but feasible intervals");
        assert!(
            (result.minimum_expected_loss - 499_999_999_700.5).abs() < 0.01,
            "minimum loss should use the full feasible mass: {}",
            result.minimum_expected_loss
        );
        assert!(
            (result.maximum_expected_loss - 500_000_000_300.5).abs() < 0.01,
            "maximum loss should allocate the small residual to the high-loss model: {}",
            result.maximum_expected_loss
        );
        assert!((result.maximizing_weights["predator_coupled"] - 0.499_999_999_7).abs() < 1e-12);
        assert!((result.maximizing_weights["predator_weak"] - 0.500_000_000_3).abs() < 1e-12);
    }

    #[test]
    fn weight_sensitivity_rejects_infeasible_simplex() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.7,
                upper: 0.9,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
        ];
        let errors = score_weight_sensitivity(&models, "retreat", &intervals)
            .expect_err("lower bounds sum to more than one");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("lower bounds sum"))
        );
    }

    #[test]
    fn weight_sensitivity_rejects_nominal_weight_outside_interval() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.1,
                upper: 0.5,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.5,
                upper: 0.9,
            },
        ];
        let errors = score_weight_sensitivity(&models, "retreat", &intervals)
            .expect_err("nominal values must belong to intervals");
        assert!(errors.iter().any(|error| error.contains("falls outside")));
    }

    #[test]
    fn weight_sensitivity_requires_exact_model_set_and_action_losses() {
        let models = example_models();
        let intervals = vec![ModelWeightInterval {
            model_id: "predator_coupled".to_owned(),
            lower: 0.4,
            upper: 0.8,
        }];
        let errors = score_weight_sensitivity(&models, "retreat", &intervals)
            .expect_err("missing model interval must fail closed");
        assert!(errors.iter().any(|error| error.contains("exactly match")));
        let errors = score_weight_sensitivity(
            &models,
            "fly_away",
            &[
                ModelWeightInterval {
                    model_id: "predator_coupled".to_owned(),
                    lower: 0.4,
                    upper: 0.8,
                },
                ModelWeightInterval {
                    model_id: "predator_weak".to_owned(),
                    lower: 0.2,
                    upper: 0.6,
                },
            ],
        )
        .expect_err("missing action loss must fail closed");
        assert!(errors.iter().any(|error| error.contains("missing loss")));
    }
    #[test]
    fn action_ranking_reports_when_preference_flips_across_shared_weights() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.2,
                upper: 0.6,
            },
        ];
        let result =
            compare_actions_under_weight_uncertainty(&models, "graze", "retreat", &intervals, 0.0)
                .expect("feasible pairwise comparison");
        assert!((result.minimum_loss_difference + 1.4).abs() < 1e-9);
        assert!((result.maximum_loss_difference - 1.8).abs() < 1e-9);
        assert_eq!(result.stability, PreferenceStability::CanFlip);
        assert!(!result.first_admissible_across_all_models);
        assert!(result.second_admissible_across_all_models);
    }

    #[test]
    fn action_ranking_can_prove_one_action_strictly_better() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.2,
                upper: 0.6,
            },
        ];
        let result =
            compare_actions_under_weight_uncertainty(&models, "retreat", "wait", &intervals, 0.0)
                .expect("feasible pairwise comparison");
        assert!((result.minimum_loss_difference - 1.4).abs() < 1e-9);
        assert!((result.maximum_loss_difference - 1.8).abs() < 1e-9);
        assert!((result.minimizing_weights["predator_coupled"] - 0.4).abs() < 1e-9);
        assert!((result.maximizing_weights["predator_coupled"] - 0.8).abs() < 1e-9);
        assert_eq!(result.stability, PreferenceStability::SecondStrictlyBetter);
    }

    #[test]
    fn small_differences_are_equivalent_inside_preregistered_margin() {
        let mut models = example_models();
        models[0].action_losses.insert("wait".to_owned(), 3.0001);
        models[1].action_losses.insert("wait".to_owned(), 3.0002);
        models[0]
            .action_losses
            .insert("retreat".to_owned(), 3.00015);
        models[1]
            .action_losses
            .insert("retreat".to_owned(), 3.00019);
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.2,
                upper: 0.6,
            },
        ];
        let result = compare_actions_under_weight_uncertainty(
            &models, "wait", "retreat", &intervals, 0.0001,
        )
        .expect("valid margin and model set");
        assert_eq!(result.stability, PreferenceStability::Equivalent);
    }

    #[test]
    fn negative_or_nonfinite_practical_margin_is_rejected() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.2,
                upper: 0.6,
            },
        ];
        let errors = compare_actions_under_weight_uncertainty(
            &models,
            "graze",
            "retreat",
            &intervals,
            f64::NAN,
        )
        .expect_err("non-finite margin must be rejected");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("margin must be finite and non-negative"))
        );
    }

    #[test]
    fn action_ranking_rejects_identical_actions() {
        let models = example_models();
        let intervals = vec![
            ModelWeightInterval {
                model_id: "predator_coupled".to_owned(),
                lower: 0.4,
                upper: 0.8,
            },
            ModelWeightInterval {
                model_id: "predator_weak".to_owned(),
                lower: 0.2,
                upper: 0.6,
            },
        ];
        let errors =
            compare_actions_under_weight_uncertainty(&models, "wait", "wait", &intervals, 0.0)
                .expect_err("an action cannot be compared with itself");
        assert!(errors.iter().any(|error| error.contains("distinct")));
    }
}
